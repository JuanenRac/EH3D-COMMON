//! The text and the tables of office documents, without their formatting.
//!
//! `docx`, `xlsx`, `pptx`, `odt`, `ods` and `odp` are ZIP archives of XML files; only the parts named
//! here are read, each within a limit and all together within another, so an archive that unpacks to
//! far more than its size is stopped. Nothing is written to disk, nothing outside the archive is read
//! (external relationships are not followed), document type declarations are refused and macros are
//! never looked at. `rtf` is read as text. The binary formats before 2007 have no preview.

use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};

use crate::doc::{Block, Builder, Preview, MAX_COLUMNS};
use crate::error::ViewerError;
use crate::kind::OfficeKind;
use crate::limits::ViewerLimits;
use crate::xml::{attribute, Piece, Reader};

/// The longest text gathered for one paragraph or cell before it is clipped.
const MAX_GATHERED: usize = 100_000;
/// The most times a row with values is repeated.
const MAX_REPEATED_ROWS: usize = 64;

struct Package<'a> {
    archive: zip::ZipArchive<Cursor<&'a [u8]>>,
    part_limit: u64,
    total_left: u64,
}

impl<'a> Package<'a> {
    fn open(data: &'a [u8], limits: &ViewerLimits) -> Result<Self, ViewerError> {
        let archive = zip::ZipArchive::new(Cursor::new(data))
            .map_err(|e| ViewerError::Damaged(format!("ZIP: {e}")))?;
        if archive.len() as u64 > limits.max_office_entries {
            return Err(ViewerError::TooLarge {
                limit: limits.max_office_entries,
            });
        }
        Ok(Package {
            archive,
            part_limit: limits.max_office_part_bytes,
            total_left: limits.max_office_total_bytes,
        })
    }

    /// A part as text, or `None` when the archive has no such entry. The size announced is not
    /// trusted: the reading stops one byte over the limit.
    fn part(&mut self, name: &str) -> Result<Option<String>, ViewerError> {
        let file = match self.archive.by_name(name) {
            Ok(file) => file,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(ViewerError::Damaged(format!("ZIP: {e}"))),
        };
        let limit = self.part_limit.min(self.total_left);
        let mut data = Vec::new();
        file.take(limit.saturating_add(1))
            .read_to_end(&mut data)
            .map_err(|e| ViewerError::Damaged(format!("ZIP: {e}")))?;
        if data.len() as u64 > limit {
            return Err(ViewerError::TooLarge { limit });
        }
        self.total_left -= data.len() as u64;
        Ok(Some(crate::plain::decode(&data)))
    }

    fn required(&mut self, name: &str) -> Result<String, ViewerError> {
        self.part(name)?
            .ok_or_else(|| ViewerError::Damaged(format!("{name} is not there")))
    }

    fn names(&self) -> Vec<String> {
        self.archive
            .file_names()
            .filter_map(Result::ok)
            .map(|n| n.into_owned())
            .collect()
    }
}

fn gather(into: &mut String, text: &str) {
    if into.len() < MAX_GATHERED {
        into.push_str(text);
    }
}

fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, n)| n)
}

/// The identifiers and targets of the relationships of a part that stay inside the archive.
fn relationships(
    package: &mut Package,
    rels: &str,
    base: &str,
) -> Result<HashMap<String, String>, ViewerError> {
    let mut out = HashMap::new();
    let Some(text) = package.part(rels)? else {
        return Ok(out);
    };
    let mut reader = Reader::new(&text);
    while let Some(piece) = reader.next_piece()? {
        if let Piece::Start {
            name, attributes, ..
        } = piece
        {
            if local(&name) != "Relationship"
                || attribute(&attributes, "TargetMode") == Some("External")
            {
                continue;
            }
            if let (Some(id), Some(target)) = (
                attribute(&attributes, "Id"),
                attribute(&attributes, "Target"),
            ) {
                let path = match target.strip_prefix('/') {
                    Some(absolute) => absolute.to_string(),
                    None => format!("{base}{target}"),
                };
                out.insert(id.to_string(), path);
            }
        }
    }
    Ok(out)
}

/// The number at the end of `.../slide12.xml`.
fn part_number(name: &str, prefix: &str) -> Option<u32> {
    name.strip_prefix(prefix)?
        .strip_suffix(".xml")?
        .parse()
        .ok()
}

fn numbered(package: &Package, prefix: &str) -> Vec<String> {
    let mut found: BTreeMap<u32, String> = BTreeMap::new();
    for name in package.names() {
        if let Some(n) = part_number(&name, prefix) {
            found.insert(n, name);
        }
    }
    found.into_values().collect()
}

/// Text with paragraphs and tables, as `docx` (prefix `w`) and the slides of `pptx` (prefix `a`)
/// write them: `p`, `t`, `tab`, `br`, `tbl`, `tr`, `tc`.
fn word_like(xml: &str, builder: &mut Builder) -> Result<(), ViewerError> {
    let mut reader = Reader::new(xml);
    let mut paragraph = String::new();
    let mut heading = 0u8;
    let mut item = false;
    let mut in_text = false;
    // tables inside tables are flattened into the cell of the outer one
    let mut depth = 0usize;
    let mut table: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    while let Some(piece) = reader.next_piece()? {
        if builder.full() {
            builder.truncate();
            break;
        }
        match piece {
            Piece::Start {
                name,
                attributes,
                empty,
            } => match local(&name) {
                "t" => in_text = !empty,
                "tab" => gather(&mut paragraph, "\t"),
                "br" | "cr" => gather(&mut paragraph, "\n"),
                "numPr" => item = true,
                "pStyle" => {
                    let style = attribute(&attributes, "w:val").unwrap_or("");
                    if style == "Title" {
                        heading = 1;
                    } else if let Some(level) = style.strip_prefix("Heading") {
                        heading = level.parse().unwrap_or(2);
                    }
                }
                "tbl" if !empty => depth += 1,
                _ => {}
            },
            Piece::Text(text) if in_text => gather(&mut paragraph, &text),
            Piece::Text(_) => {}
            Piece::End(name) => match local(&name) {
                "t" => in_text = false,
                "p" => {
                    let done = std::mem::take(&mut paragraph);
                    if depth > 0 {
                        if !cell.is_empty() && !done.is_empty() {
                            gather(&mut cell, "\n");
                        }
                        gather(&mut cell, &done);
                    } else if !done.trim().is_empty() {
                        builder.push(if heading > 0 {
                            Block::Heading(heading, done)
                        } else if item {
                            Block::Item(0, done)
                        } else {
                            Block::Paragraph(done)
                        });
                    }
                    heading = 0;
                    item = false;
                }
                "tc" if depth == 1 => {
                    if row.len() <= MAX_COLUMNS {
                        row.push(std::mem::take(&mut cell));
                    }
                }
                "tr" if depth == 1 => builder.row(&mut table, std::mem::take(&mut row)),
                "tbl" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        builder.table(std::mem::take(&mut table));
                    }
                }
                _ => {}
            },
        }
    }
    builder.table(table);
    Ok(())
}

fn docx(package: &mut Package, builder: &mut Builder) -> Result<(), ViewerError> {
    let xml = package.required("word/document.xml")?;
    word_like(&xml, builder)
}

fn pptx(package: &mut Package, builder: &mut Builder) -> Result<(), ViewerError> {
    let presentation = package.required("ppt/presentation.xml")?;
    let targets = relationships(package, "ppt/_rels/presentation.xml.rels", "ppt/")?;
    let mut slides = Vec::new();
    let mut reader = Reader::new(&presentation);
    while let Some(piece) = reader.next_piece()? {
        if let Piece::Start {
            name, attributes, ..
        } = piece
        {
            if local(&name) == "sldId" && slides.len() < 2000 {
                if let Some(path) = attribute(&attributes, "r:id").and_then(|id| targets.get(id)) {
                    slides.push(path.clone());
                }
            }
        }
    }
    if slides.is_empty() {
        slides = numbered(package, "ppt/slides/slide");
        slides.truncate(2000);
    }
    for (at, slide) in slides.iter().enumerate() {
        if builder.full() {
            builder.truncate();
            break;
        }
        builder.push(Block::Heading(2, format!("{}", at + 1)));
        if let Some(xml) = package.part(slide)? {
            word_like(&xml, builder)?;
        }
    }
    Ok(())
}

/// The column, from 0, of a reference such as `BC12`.
fn column_of(reference: &str) -> Option<usize> {
    let letters = reference
        .bytes()
        .take_while(u8::is_ascii_alphabetic)
        .take(4)
        .collect::<Vec<_>>();
    if letters.is_empty() {
        return None;
    }
    let number = letters.iter().fold(0usize, |n, c| {
        n * 26 + usize::from(c.to_ascii_uppercase() - b'A') + 1
    });
    Some(number - 1)
}

fn shared_strings(xml: &str, max: u64) -> Result<Vec<String>, ViewerError> {
    let mut reader = Reader::new(xml);
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_text = false;
    // the phonetic readings of East Asian text are not part of the value
    let mut phonetic = false;
    while let Some(piece) = reader.next_piece()? {
        if out.len() as u64 >= max {
            break;
        }
        match piece {
            Piece::Start { name, empty, .. } => match local(&name) {
                "t" => in_text = !empty && !phonetic,
                "rPh" => phonetic = !empty,
                "si" if empty => out.push(String::new()),
                _ => {}
            },
            Piece::Text(text) if in_text => gather(&mut current, &text),
            Piece::Text(_) => {}
            Piece::End(name) => match local(&name) {
                "t" => in_text = false,
                "rPh" => phonetic = false,
                "si" => out.push(std::mem::take(&mut current)),
                _ => {}
            },
        }
    }
    Ok(out)
}

/// Puts a value at a column of a row.
fn put(row: &mut Vec<String>, column: usize, value: String, builder: &mut Builder) {
    if column >= MAX_COLUMNS {
        if !value.is_empty() {
            builder.truncate();
        }
        return;
    }
    if row.len() <= column {
        row.resize(column + 1, String::new());
    }
    row[column] = value;
}

fn worksheet(xml: &str, strings: &[String], builder: &mut Builder) -> Result<(), ViewerError> {
    let mut reader = Reader::new(xml);
    let mut table = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut column = 0usize;
    let mut kind = String::new();
    let mut value = String::new();
    let mut in_value = false;
    while let Some(piece) = reader.next_piece()? {
        if builder.full() {
            builder.truncate();
            break;
        }
        match piece {
            Piece::Start {
                name,
                attributes,
                empty,
            } => match local(&name) {
                "c" => {
                    column = attribute(&attributes, "r")
                        .and_then(column_of)
                        .unwrap_or(row.len());
                    kind = attribute(&attributes, "t").unwrap_or("").to_string();
                    value.clear();
                }
                "v" | "t" => in_value = !empty,
                _ => {}
            },
            Piece::Text(text) if in_value => gather(&mut value, &text),
            Piece::Text(_) => {}
            Piece::End(name) => match local(&name) {
                "v" | "t" => in_value = false,
                "c" => {
                    let shown = match kind.as_str() {
                        "s" => value
                            .trim()
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| strings.get(i))
                            .cloned()
                            .unwrap_or_default(),
                        "b" => (if value.trim() == "1" { "TRUE" } else { "FALSE" }).to_string(),
                        _ => std::mem::take(&mut value),
                    };
                    put(&mut row, column, shown, builder);
                }
                "row" => builder.row(&mut table, std::mem::take(&mut row)),
                _ => {}
            },
        }
    }
    builder.table(table);
    Ok(())
}

fn xlsx(
    package: &mut Package,
    builder: &mut Builder,
    limits: &ViewerLimits,
) -> Result<(), ViewerError> {
    let workbook = package.required("xl/workbook.xml")?;
    let targets = relationships(package, "xl/_rels/workbook.xml.rels", "xl/")?;
    let strings = match package.part("xl/sharedStrings.xml")? {
        Some(xml) => shared_strings(&xml, limits.max_cells)?,
        None => Vec::new(),
    };
    let mut sheets = Vec::new();
    let mut reader = Reader::new(&workbook);
    while let Some(piece) = reader.next_piece()? {
        if let Piece::Start {
            name, attributes, ..
        } = piece
        {
            if local(&name) == "sheet" && sheets.len() < 256 {
                let title = attribute(&attributes, "name").unwrap_or("").to_string();
                let path = attribute(&attributes, "r:id").and_then(|id| targets.get(id));
                sheets.push((title, path.cloned()));
            }
        }
    }
    for (title, path) in sheets {
        if builder.full() {
            builder.truncate();
            break;
        }
        builder.push(Block::Heading(2, title));
        if let Some(xml) = match path {
            Some(path) => package.part(&path)?,
            None => None,
        } {
            worksheet(&xml, &strings, builder)?;
        }
    }
    Ok(())
}

/// How many times a column, a row or a space is repeated, never more than `max`.
fn repeat(attributes: &[(String, String)], name: &str, max: usize) -> usize {
    attribute(attributes, name)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, max)
}

/// `content.xml` of the formats of LibreOffice.
fn open_document(xml: &str, builder: &mut Builder) -> Result<(), ViewerError> {
    let mut reader = Reader::new(xml);
    let mut text = String::new();
    let mut heading = 0u8;
    // how many paragraphs are open: text outside them is not part of the document
    let mut open = 0usize;
    let mut depth = 0usize;
    let mut table: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut columns = 1usize;
    let mut rows = 1usize;
    let mut slide = 0usize;
    while let Some(piece) = reader.next_piece()? {
        if builder.full() {
            builder.truncate();
            break;
        }
        match piece {
            Piece::Start {
                name,
                attributes,
                empty,
            } => match name.as_str() {
                "text:p" if !empty => open += 1,
                "text:h" if !empty => {
                    open += 1;
                    heading = attribute(&attributes, "text:outline-level")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(1);
                }
                "text:s" => gather(&mut text, &" ".repeat(repeat(&attributes, "text:c", 200))),
                "text:tab" => gather(&mut text, "\t"),
                "text:line-break" => gather(&mut text, "\n"),
                "draw:page" => {
                    slide += 1;
                    builder.push(Block::Heading(2, slide.to_string()));
                }
                "table:table" if !empty => {
                    depth += 1;
                    if depth == 1 {
                        if let Some(title) = attribute(&attributes, "table:name") {
                            builder.push(Block::Heading(2, title.to_string()));
                        }
                    }
                }
                "table:table-row" => {
                    rows = repeat(&attributes, "table:number-rows-repeated", usize::MAX);
                }
                "table:table-cell" | "table:covered-table-cell" if depth == 1 => {
                    columns = repeat(&attributes, "table:number-columns-repeated", usize::MAX);
                    if empty {
                        let room = MAX_COLUMNS.saturating_sub(row.len());
                        row.extend(std::iter::repeat_n(String::new(), columns.min(room)));
                    }
                }
                _ => {}
            },
            Piece::Text(piece) if open > 0 => gather(&mut text, &piece),
            Piece::Text(_) => {}
            Piece::End(name) => match name.as_str() {
                "text:p" | "text:h" => {
                    open = open.saturating_sub(1);
                    let done = std::mem::take(&mut text);
                    if depth > 0 {
                        if !cell.is_empty() && !done.is_empty() {
                            gather(&mut cell, "\n");
                        }
                        gather(&mut cell, &done);
                    } else if !done.trim().is_empty() {
                        builder.push(if heading > 0 {
                            Block::Heading(heading, done)
                        } else {
                            Block::Paragraph(done)
                        });
                    }
                    heading = 0;
                }
                "table:table-cell" | "table:covered-table-cell" if depth == 1 => {
                    let value = std::mem::take(&mut cell);
                    // an empty cell repeated to the end of the sheet only needs to be kept short
                    let room = MAX_COLUMNS.saturating_sub(row.len());
                    if columns > room && !value.is_empty() {
                        builder.truncate();
                    }
                    row.extend(std::iter::repeat_n(value, columns.min(room)));
                    columns = 1;
                }
                "table:table-row" if depth == 1 => {
                    let mut done = std::mem::take(&mut row);
                    while done.last().is_some_and(String::is_empty) {
                        done.pop();
                    }
                    // empty rows are kept once, full rows as often as they are repeated
                    let times = if done.is_empty() {
                        1
                    } else {
                        if rows > MAX_REPEATED_ROWS {
                            builder.truncate();
                        }
                        rows.min(MAX_REPEATED_ROWS)
                    };
                    for _ in 0..times {
                        builder.row(&mut table, done.clone());
                    }
                    rows = 1;
                }
                "table:table" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        builder.table(std::mem::take(&mut table));
                    }
                }
                _ => {}
            },
        }
    }
    builder.table(table);
    Ok(())
}

/// The destinations of RTF that are not text of the document.
const RTF_SKIPPED: &[&str] = &[
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "object",
    "header",
    "footer",
    "themedata",
    "datastore",
    "latentstyles",
    "listtable",
    "listoverridetable",
    "rsidtbl",
    "generator",
    "fldinst",
    "xmlnstbl",
];

fn rtf(data: &[u8], builder: &mut Builder) -> Result<(), ViewerError> {
    if !data.starts_with(b"{\\rtf") {
        return Err(ViewerError::Damaged("RTF: no header".into()));
    }
    // one flag per open group: whether its text is skipped
    let mut groups: Vec<bool> = Vec::new();
    let mut skip = false;
    let mut text = String::new();
    let mut at = 0;
    // characters to leave out after \u, written for programs that do not read it
    let mut fallback = 0usize;
    while at < data.len() && !builder.full() {
        let byte = data[at];
        at += 1;
        let mut shown = None;
        match byte {
            b'{' => {
                if groups.len() >= 256 {
                    return Err(ViewerError::Damaged("RTF: too many groups".into()));
                }
                groups.push(skip);
            }
            b'}' => skip = groups.pop().unwrap_or(false),
            b'\r' | b'\n' => {}
            b'\\' => {
                let Some(&next) = data.get(at) else { break };
                if next.is_ascii_alphabetic() {
                    let start = at;
                    while data.get(at).is_some_and(u8::is_ascii_alphabetic) {
                        at += 1;
                    }
                    let word = std::str::from_utf8(&data[start..at]).unwrap_or("");
                    let digits = at;
                    if data.get(at) == Some(&b'-') {
                        at += 1;
                    }
                    while data.get(at).is_some_and(u8::is_ascii_digit) {
                        at += 1;
                    }
                    let number: Option<i32> = std::str::from_utf8(&data[digits..at])
                        .ok()
                        .and_then(|n| n.parse().ok());
                    if data.get(at) == Some(&b' ') {
                        at += 1;
                    }
                    match word {
                        "par" | "line" | "row" if !skip => {
                            let done = std::mem::take(&mut text);
                            if !done.trim().is_empty() {
                                builder.push(Block::Paragraph(done));
                            }
                        }
                        "tab" | "cell" => shown = Some('\t'),
                        "u" => {
                            // the number is signed: values above 32767 are written negative
                            shown = number
                                .map(|n| char::from_u32(u32::from(n as u16)).unwrap_or('\u{fffd}'));
                            fallback = 1;
                        }
                        "bin" => {
                            // raw bytes that must not be read as text or groups
                            let count = number.and_then(|n| usize::try_from(n).ok()).unwrap_or(0);
                            at = at.saturating_add(count).min(data.len());
                        }
                        word if RTF_SKIPPED.contains(&word) => skip = true,
                        _ => {}
                    }
                } else {
                    at += 1;
                    match next {
                        b'*' => skip = true,
                        b'\'' => {
                            let hex = data
                                .get(at..at + 2)
                                .and_then(|h| std::str::from_utf8(h).ok())
                                .and_then(|h| u8::from_str_radix(h, 16).ok());
                            at += 2;
                            if fallback > 0 {
                                fallback -= 1;
                            } else {
                                shown = hex.map(char::from);
                            }
                        }
                        b'\\' | b'{' | b'}' => shown = Some(char::from(next)),
                        b'~' => shown = Some(' '),
                        _ => {}
                    }
                }
            }
            other => {
                if fallback > 0 {
                    fallback -= 1;
                } else if other >= 0x20 || other == b'\t' {
                    shown = Some(char::from(other));
                }
            }
        }
        if let Some(c) = shown {
            if !skip && text.len() < MAX_GATHERED {
                text.push(c);
            }
        }
    }
    if at < data.len() {
        builder.truncate();
    }
    if !text.trim().is_empty() {
        builder.push(Block::Paragraph(text));
    }
    Ok(())
}

/// The preview of an office document held in `data`.
pub fn preview(
    kind: OfficeKind,
    data: &[u8],
    limits: &ViewerLimits,
) -> Result<Preview, ViewerError> {
    let mut builder = Builder::new(limits);
    match kind {
        OfficeKind::Legacy => return Err(ViewerError::Unsupported),
        OfficeKind::Rtf => rtf(data, &mut builder)?,
        OfficeKind::Docx => docx(&mut Package::open(data, limits)?, &mut builder)?,
        OfficeKind::Xlsx => xlsx(&mut Package::open(data, limits)?, &mut builder, limits)?,
        OfficeKind::Pptx => pptx(&mut Package::open(data, limits)?, &mut builder)?,
        OfficeKind::Odt | OfficeKind::Ods | OfficeKind::Odp => {
            let xml = Package::open(data, limits)?.required("content.xml")?;
            open_document(&xml, &mut builder)?;
        }
    }
    Ok(builder.finish())
}
