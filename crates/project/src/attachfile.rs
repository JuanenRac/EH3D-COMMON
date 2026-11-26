//! What a file that is attached to a project is: its kind, whether it is dangerous and the name it may safely have on a disk.
//!
//! Every kind of file may be attached; what is not recognised is [`AttachmentKind::Other`]. Programs, scripts, office documents
//! with macros, compressed archives and pages that can run code are **dangerous**: they are kept like any other file, marked, and
//! a person has to agree before one is taken out or opened. Nothing here runs or opens anything.

use std::io::{self, Read, Seek, SeekFrom};

/// The longest name that is kept, in bytes.
pub const MAX_FILE_NAME_BYTES: usize = 150;
/// The bytes of the start of a file that say what it is.
pub const HEAD_BYTES: usize = 4096;

/// What a file is, by its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// A PDF document.
    Pdf,
    /// Markdown text.
    Markdown,
    /// Plain text, source, settings, G-code and the like.
    Text,
    /// A picture.
    Image,
    /// A machine-readable report (`.json`).
    Report,
    /// An office document, a spreadsheet, a presentation or a table of values.
    Office,
    /// The project or library file of a design program (EAGLE, KiCad and the like, or a project of the suite).
    Project,
    /// A drawing or a model (STEP, STL, DXF, 3MF and the like).
    Cad,
    /// A video.
    Video,
    /// Sound.
    Audio,
    /// A compressed archive.
    Archive,
    /// Anything else.
    Other,
}

impl AttachmentKind {
    /// Every kind.
    pub const ALL: [AttachmentKind; 12] = [
        AttachmentKind::Pdf,
        AttachmentKind::Markdown,
        AttachmentKind::Text,
        AttachmentKind::Image,
        AttachmentKind::Report,
        AttachmentKind::Office,
        AttachmentKind::Project,
        AttachmentKind::Cad,
        AttachmentKind::Video,
        AttachmentKind::Audio,
        AttachmentKind::Archive,
        AttachmentKind::Other,
    ];

    /// The word of the record.
    pub fn name(self) -> &'static str {
        match self {
            AttachmentKind::Pdf => "pdf",
            AttachmentKind::Markdown => "markdown",
            AttachmentKind::Text => "text",
            AttachmentKind::Image => "image",
            AttachmentKind::Report => "report",
            AttachmentKind::Office => "office",
            AttachmentKind::Project => "project",
            AttachmentKind::Cad => "cad",
            AttachmentKind::Video => "video",
            AttachmentKind::Audio => "audio",
            AttachmentKind::Archive => "archive",
            AttachmentKind::Other => "other",
        }
    }

    /// The kind a word of the record names; a word of a newer version is [`AttachmentKind::Other`].
    pub fn parse(word: &str) -> AttachmentKind {
        AttachmentKind::ALL
            .into_iter()
            .find(|k| k.name() == word)
            .unwrap_or(AttachmentKind::Other)
    }

    /// The kind a file name belongs to, from its extension.
    pub fn of_name(name: &str) -> AttachmentKind {
        let extension = extension_of(&sanitize_file_name(name));
        let has = |list: &[&str]| list.contains(&extension.as_str());
        match extension.as_str() {
            "pdf" => AttachmentKind::Pdf,
            "md" | "markdown" => AttachmentKind::Markdown,
            "json" => AttachmentKind::Report,
            _ if has(PICTURES) => AttachmentKind::Image,
            _ if has(TEXTS) => AttachmentKind::Text,
            _ if has(OFFICE) || has(MACRO_OFFICE) => AttachmentKind::Office,
            _ if has(DESIGN) => AttachmentKind::Project,
            _ if has(MODELS) => AttachmentKind::Cad,
            _ if has(VIDEO) => AttachmentKind::Video,
            _ if has(AUDIO) => AttachmentKind::Audio,
            _ if has(ARCHIVES) => AttachmentKind::Archive,
            _ => AttachmentKind::Other,
        }
    }
}

/// Why a file is dangerous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Danger {
    /// A program, or a file the system runs when it is opened.
    Executable,
    /// A script.
    Script,
    /// An office document with macros, or one that cannot be looked into.
    Macros,
    /// A compressed archive, which is not looked into.
    Archive,
    /// A page that may run code in a browser (HTML, SVG).
    ActiveContent,
}

impl Danger {
    /// Every danger.
    pub const ALL: [Danger; 5] = [
        Danger::Executable,
        Danger::Script,
        Danger::Macros,
        Danger::Archive,
        Danger::ActiveContent,
    ];

    /// The word of the record.
    pub fn name(self) -> &'static str {
        match self {
            Danger::Executable => "executable",
            Danger::Script => "script",
            Danger::Macros => "macros",
            Danger::Archive => "archive",
            Danger::ActiveContent => "active-content",
        }
    }

    /// The danger a word of the record names. A word of a newer version is taken as the worst one, so it is never lost.
    pub fn parse(word: &str) -> Danger {
        Danger::ALL
            .into_iter()
            .find(|d| d.name() == word)
            .unwrap_or(Danger::Executable)
    }

    /// A program or a script is never launched by a program of the suite, whatever a person confirms.
    pub fn never_launched(self) -> bool {
        matches!(self, Danger::Executable | Danger::Script)
    }
}

const EXECUTABLES: &[&str] = &[
    "app",
    "appimage",
    "appref-ms",
    "apk",
    "application",
    "bin",
    "cab",
    "chm",
    "com",
    "command",
    "cpl",
    "deb",
    "desktop",
    "dll",
    "dmg",
    "drv",
    "dylib",
    "elf",
    "exe",
    "gadget",
    "hlp",
    "img",
    "inf",
    "ins",
    "ipa",
    "iso",
    "isp",
    "jar",
    "jnlp",
    "lnk",
    "msc",
    "msi",
    "msix",
    "msp",
    "mst",
    "ocx",
    "pif",
    "pkg",
    "reg",
    "rpm",
    "run",
    "scf",
    "scr",
    "settingcontent-ms",
    "so",
    "sys",
    "url",
    "vhd",
    "vhdx",
    "webloc",
    "website",
    "xll",
];
const SCRIPTS: &[&str] = &[
    "action",
    "ahk",
    "applescript",
    "bash",
    "bat",
    "cmd",
    "csh",
    "fish",
    "hta",
    "js",
    "jse",
    "ksh",
    "lisp",
    "lsp",
    "mjs",
    "php",
    "pl",
    "ps1",
    "ps1xml",
    "ps2",
    "psc1",
    "psm1",
    "py",
    "pyc",
    "pyw",
    "pyz",
    "rb",
    "scpt",
    "sh",
    "tcl",
    "ulp",
    "vb",
    "vbe",
    "vbs",
    "workflow",
    "ws",
    "wsc",
    "wsf",
    "wsh",
    "zsh",
];
const MACRO_OFFICE: &[&str] = &[
    "docm", "dotm", "xlsm", "xltm", "xlsb", "xlam", "pptm", "potm", "ppsm", "ppam", "sldm",
    "accdb", "accde", "mdb", "mde",
];
const OFFICE: &[&str] = &[
    "csv", "doc", "dot", "docx", "dotx", "odp", "ods", "odt", "otp", "ots", "ott", "pot", "potx",
    "pps", "ppsx", "ppt", "pptx", "rtf", "tsv", "xls", "xlt", "xlsx", "xltx",
];
const ARCHIVES: &[&str] = &[
    "7z", "ace", "arj", "bz2", "gz", "lz", "lzh", "lzma", "rar", "tar", "tbz", "tgz", "txz", "xz",
    "z", "zip", "zst",
];
const ACTIVE: &[&str] = &[
    "htm", "html", "mht", "mhtml", "shtml", "svg", "svgz", "xht", "xhtml",
];
const PICTURES: &[&str] = &[
    "bmp", "gif", "heic", "ico", "jpeg", "jpg", "png", "psd", "svg", "tif", "tiff", "webp",
];
const TEXTS: &[&str] = &[
    "asm", "c", "cc", "cfg", "cmake", "cnc", "conf", "cpp", "drl", "gbl", "gbr", "gbs", "gcode",
    "gko", "gtl", "gto", "gts", "h", "hex", "hpp", "htm", "html", "ini", "ino", "log", "nc", "ngc",
    "plt", "rs", "tap", "toml", "txt", "xml", "yaml", "yml",
];
const DESIGN: &[&str] = &[
    "brd",
    "dsn",
    "eh3d",
    "eh3dproj",
    "f3d",
    "fcstd",
    "kicad_mod",
    "kicad_pcb",
    "kicad_pro",
    "kicad_sch",
    "kicad_sym",
    "lbr",
    "phoenixproj",
    "phoenixsketch",
    "sch",
    "sldasm",
    "sldprt",
];
const MODELS: &[&str] = &[
    "3mf", "dwg", "dxf", "iges", "igs", "obj", "ply", "step", "stl", "stp",
];
const VIDEO: &[&str] = &[
    "avi", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "ogv", "webm", "wmv",
];
const AUDIO: &[&str] = &[
    "aac", "flac", "m4a", "mp3", "oga", "ogg", "opus", "wav", "weba", "wma",
];

/// The extension of a name in lower case, or nothing.
fn extension_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => extension.to_ascii_lowercase(),
        _ => String::new(),
    }
}

fn hidden_or_reordering(c: char) -> bool {
    // marks that change the order the letters are shown in, or are not shown at all
    matches!(c, '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2069}' | '\u{feff}')
}

fn reserved_on_windows(stem: &str) -> bool {
    let stem = stem.trim_end().to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|p| {
        stem.strip_prefix(p).is_some_and(|n| {
            matches!(
                n,
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    })
}

/// A file name that is safe on every system: only the last part of a path, no separators, controls, invisible or reordering
/// marks, no leading or trailing dots and spaces, no leading dash, no reserved device name and not too long. The extension is
/// kept. A name from a project is never used as a path before it has been through here.
pub fn sanitize_file_name(name: &str) -> String {
    let last = name.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = last
        .chars()
        .filter(|c| !c.is_control() && !hidden_or_reordering(*c))
        .map(|c| if "<>:\"|?*,;".contains(c) { '_' } else { c })
        .collect();
    let mut cleaned = cleaned
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .to_string();
    // a name that starts with a dash could be read as an option by the program that opens it
    if cleaned.starts_with('-') {
        cleaned.replace_range(..1, "_");
    }
    if cleaned.is_empty() {
        return "attachment".to_string();
    }
    if reserved_on_windows(cleaned.split('.').next().unwrap_or("")) {
        cleaned.insert(0, '_');
    }
    if cleaned.len() > MAX_FILE_NAME_BYTES {
        let (stem, extension) = match cleaned.rsplit_once('.') {
            Some((s, e)) if e.len() <= 24 => (s, format!(".{e}")),
            _ => (cleaned.as_str(), String::new()),
        };
        let mut end = (MAX_FILE_NAME_BYTES - extension.len()).min(stem.len());
        while !stem.is_char_boundary(end) {
            end -= 1;
        }
        cleaned = format!("{}{extension}", stem[..end].trim_end());
    }
    cleaned
}

fn danger_of_name(extension: &str) -> Option<Danger> {
    let has = |list: &[&str]| list.contains(&extension);
    if has(EXECUTABLES) {
        Some(Danger::Executable)
    } else if has(SCRIPTS) {
        Some(Danger::Script)
    } else if has(MACRO_OFFICE) {
        Some(Danger::Macros)
    } else if has(ARCHIVES) {
        Some(Danger::Archive)
    } else if has(ACTIVE) {
        Some(Danger::ActiveContent)
    } else {
        None
    }
}

/// The danger that the first bytes alone show, whatever the name says.
fn danger_of_program(head: &[u8]) -> Option<Danger> {
    // two letters are a weak sign: a text that happens to start with them has no zero byte
    if head.starts_with(b"MZ") {
        return head.contains(&0).then_some(Danger::Executable);
    }
    // ELF, then the forms of Mach-O
    let magics: &[&[u8]] = &[
        b"\x7fELF",
        b"\xfe\xed\xfa\xce",
        b"\xfe\xed\xfa\xcf",
        b"\xce\xfa\xed\xfe",
        b"\xcf\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe",
    ];
    if magics.iter().any(|m| head.starts_with(m)) {
        return Some(Danger::Executable);
    }
    head.starts_with(b"#!").then_some(Danger::Script)
}

fn is_zip(head: &[u8]) -> bool {
    head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06")
}

fn is_other_archive(head: &[u8]) -> bool {
    head.starts_with(b"7z\xbc\xaf\x27\x1c")
        || head.starts_with(b"Rar!\x1a\x07")
        || head.starts_with(b"\x1f\x8b")
        || head.starts_with(b"\xfd7zXZ\x00")
        || head.starts_with(b"\x28\xb5\x2f\xfd")
}

fn is_ole(head: &[u8]) -> bool {
    head.starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1")
}

/// Whether a name and the first bytes of a file ([`HEAD_BYTES`] are enough) say it is dangerous. What the bytes say wins when the
/// name says something harmless. [`danger_of_file`] also looks inside office documents.
pub fn danger_of(name: &str, head: &[u8]) -> Option<Danger> {
    let extension = extension_of(&sanitize_file_name(name));
    let mut danger = danger_of_name(&extension);
    if let Some(found) = danger_of_program(head) {
        if !danger.is_some_and(Danger::never_launched) {
            danger = Some(found);
        }
    } else if danger.is_none() && is_other_archive(head) {
        danger = Some(Danger::Archive);
    } else if danger.is_none() && is_zip(head) {
        // a zip behind the name of an office document or of some design files is the normal form of that format
        let expected = (OFFICE.contains(&extension.as_str()) && extension != "rtf")
            || matches!(
                extension.as_str(),
                "3mf" | "f3d" | "fcstd" | "eh3d" | "eh3dproj"
            );
        if !expected {
            danger = Some(Danger::Archive);
        }
    }
    danger
}

/// The most bytes of the list of entries of an office archive that are read.
const MAX_DIRECTORY_BYTES: u64 = 8 * 1024 * 1024;

fn le16(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
}

fn le32(bytes: &[u8], at: usize) -> u64 {
    u64::from(u32::from_le_bytes([
        bytes[at],
        bytes[at + 1],
        bytes[at + 2],
        bytes[at + 3],
    ]))
}

/// Whether the entries of a ZIP archive name macros; `None` when the list of entries cannot be read. Nothing is decompressed.
fn zip_names_macros(file: &mut (impl Read + Seek)) -> io::Result<Option<bool>> {
    let len = file.seek(SeekFrom::End(0))?;
    let tail_len = len.min(22 + u64::from(u16::MAX));
    let mut tail = vec![0u8; tail_len as usize];
    file.seek(SeekFrom::Start(len - tail_len))?;
    file.read_exact(&mut tail)?;
    let Some(end) = tail
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .filter(|at| at + 22 <= tail.len())
    else {
        return Ok(None);
    };
    let size = le32(&tail, end + 12);
    let offset = le32(&tail, end + 16);
    if size > MAX_DIRECTORY_BYTES || offset.checked_add(size).is_none_or(|e| e > len) {
        return Ok(None);
    }
    let mut directory = vec![0u8; size as usize];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut directory)?;
    let mut at = 0;
    while at < directory.len() {
        if at + 46 > directory.len() || directory[at..at + 4] != *b"PK\x01\x02" {
            return Ok(None);
        }
        let name_len = le16(&directory, at + 28);
        let next = at + 46 + name_len + le16(&directory, at + 30) + le16(&directory, at + 32);
        if next > directory.len() {
            return Ok(None);
        }
        let name = String::from_utf8_lossy(&directory[at + 46..at + 46 + name_len])
            .to_ascii_lowercase()
            .replace('\\', "/");
        if name.ends_with("vbaproject.bin")
            || name.ends_with("vbadata.xml")
            || name.starts_with("basic/")
            || name.starts_with("scripts/")
        {
            return Ok(Some(true));
        }
        at = next;
    }
    Ok(Some(false))
}

/// Whether the bytes of an old binary office document name macros, reading it in blocks.
fn ole_names_macros(file: &mut (impl Read + Seek)) -> io::Result<bool> {
    let wide = |text: &str| -> Vec<u8> { text.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    let needles = [wide("_VBA_PROJECT"), wide("Macros")];
    let keep = needles.iter().map(Vec::len).max().unwrap_or(1) - 1;
    file.seek(SeekFrom::Start(0))?;
    let mut block = vec![0u8; 64 * 1024];
    let mut held = 0;
    loop {
        let n = file.read(&mut block[held..])?;
        if n == 0 {
            return Ok(false);
        }
        let filled = held + n;
        let found = needles
            .iter()
            .any(|needle| block[..filled].windows(needle.len()).any(|w| w == needle));
        if found {
            return Ok(true);
        }
        held = filled.min(keep);
        block.copy_within(filled - held..filled, 0);
    }
}

/// [`danger_of`] for a whole file: office documents are also looked into for macros, whatever their extension says. A document
/// that cannot be looked into counts as dangerous. The file is read in blocks; nothing is decompressed or run.
pub fn danger_of_file(name: &str, file: &mut (impl Read + Seek)) -> io::Result<Option<Danger>> {
    file.seek(SeekFrom::Start(0))?;
    let mut head = Vec::new();
    (&mut *file)
        .take(HEAD_BYTES as u64)
        .read_to_end(&mut head)?;
    if let Some(danger) = danger_of(name, &head) {
        return Ok(Some(danger));
    }
    if AttachmentKind::of_name(name) != AttachmentKind::Office {
        return Ok(None);
    }
    if is_zip(&head) {
        return Ok(match zip_names_macros(file)? {
            Some(false) => None,
            Some(true) | None => Some(Danger::Macros),
        });
    }
    if is_ole(&head) && ole_names_macros(file)? {
        return Ok(Some(Danger::Macros));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn zip_of(names: &[&str]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in names {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn of_bytes(name: &str, bytes: &[u8]) -> Option<Danger> {
        danger_of_file(name, &mut Cursor::new(bytes)).unwrap()
    }

    #[test]
    fn each_family_of_files_has_its_kind_and_the_rest_is_other() {
        use AttachmentKind as K;
        for (name, kind) in [
            ("Data.PDF", K::Pdf),
            ("read.md", K::Markdown),
            ("job.gcode", K::Text),
            ("photo.JPG", K::Image),
            ("drc.json", K::Report),
            ("a.doc", K::Office),
            ("a.docx", K::Office),
            ("a.odt", K::Office),
            ("a.rtf", K::Office),
            ("a.xls", K::Office),
            ("a.xlsx", K::Office),
            ("a.ods", K::Office),
            ("a.csv", K::Office),
            ("a.ppt", K::Office),
            ("a.pptx", K::Office),
            ("a.odp", K::Office),
            ("a.xlsm", K::Office),
            ("board.brd", K::Project),
            ("board.kicad_pcb", K::Project),
            ("case.eh3d", K::Project),
            ("case.step", K::Cad),
            ("case.stl", K::Cad),
            ("case.dxf", K::Cad),
            ("clip.mp4", K::Video),
            ("clip.webm", K::Video),
            ("talk.mp3", K::Audio),
            ("files.zip", K::Archive),
            ("files.7z", K::Archive),
            ("thing.abc123", K::Other),
            ("noextension", K::Other),
            ("setup.exe", K::Other),
        ] {
            assert_eq!(AttachmentKind::of_name(name), kind, "{name}");
        }
        for kind in AttachmentKind::ALL {
            assert_eq!(AttachmentKind::parse(kind.name()), kind);
        }
        assert_eq!(AttachmentKind::parse("hologram"), AttachmentKind::Other);
    }

    #[test]
    fn programs_scripts_macros_archives_and_pages_are_dangerous() {
        for (name, danger) in [
            ("setup.exe", Danger::Executable),
            ("a.LNK", Danger::Executable),
            ("eagle.scr", Danger::Executable),
            ("run.ps1", Danger::Script),
            ("tool.py", Danger::Script),
            ("library.ulp", Danger::Script),
            ("report.docm", Danger::Macros),
            ("report.xlsm", Danger::Macros),
            ("talk.pptm", Danger::Macros),
            ("files.zip", Danger::Archive),
            ("files.7z", Danger::Archive),
            ("page.html", Danger::ActiveContent),
            ("logo.svg", Danger::ActiveContent),
            ("manual.pdf.exe", Danger::Executable),
            ("run.exe. . ", Danger::Executable),
            ("a\u{202e}fdp.exe", Danger::Executable),
        ] {
            assert_eq!(danger_of(name, b""), Some(danger), "{name}");
        }
        for name in ["a.pdf", "a.step", "a.mp4", "a.docx", "thing.abc123"] {
            assert_eq!(danger_of(name, b""), None, "{name}");
        }
        for danger in Danger::ALL {
            assert_eq!(Danger::parse(danger.name()), danger);
        }
        assert_eq!(Danger::parse("new-danger"), Danger::Executable);
    }

    #[test]
    fn the_bytes_win_over_a_harmless_name() {
        assert_eq!(
            danger_of("datasheet.pdf", b"MZ\x90\x00\x03"),
            Some(Danger::Executable)
        );
        assert_eq!(danger_of("notes.txt", b"#!/bin/sh\n"), Some(Danger::Script));
        assert_eq!(
            danger_of("photo.png", b"\x7fELF\x02"),
            Some(Danger::Executable)
        );
        assert_eq!(
            danger_of("manual.pdf", b"PK\x03\x04"),
            Some(Danger::Archive)
        );
        assert_eq!(danger_of("part.3mf", b"PK\x03\x04"), None);
        assert_eq!(danger_of("notes.txt", b"MZ is a code"), None);
    }

    #[test]
    fn macros_inside_a_document_with_a_harmless_extension_are_found() {
        assert_eq!(of_bytes("a.docx", &zip_of(&["word/document.xml"])), None);
        assert_eq!(
            of_bytes("a.docx", &zip_of(&["word/vbaProject.bin"])),
            Some(Danger::Macros)
        );
        assert_eq!(
            of_bytes("a.odt", &zip_of(&["content.xml", "Basic/script.xml"])),
            Some(Danger::Macros)
        );
        let mut old = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1".to_vec();
        old.resize(200_000, 0);
        assert_eq!(of_bytes("a.xls", &old), None);
        // across the edge of a block of the reader
        let needle: Vec<u8> = "_VBA_PROJECT"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        old[65_530..65_530 + needle.len()].copy_from_slice(&needle);
        assert_eq!(of_bytes("a.xls", &old), Some(Danger::Macros));
        assert_eq!(
            of_bytes("a.docx", b"PK\x03\x04broken"),
            Some(Danger::Macros),
            "what cannot be looked into is asked about"
        );
    }

    #[test]
    fn a_name_never_leaves_its_folder() {
        for (given, safe) in [
            ("../../etc/passwd", "passwd"),
            ("..\\..\\windows\\evil.exe", "evil.exe"),
            ("C:\\a\\b.pdf", "b.pdf"),
            ("..", "attachment"),
            ("", "attachment"),
            ("  .hidden  ", "hidden"),
            ("-rf", "_rf"),
            ("CON", "_CON"),
            ("com1.txt", "_com1.txt"),
            ("a<b>c:d|e?.pdf", "a_b_c_d_e_.pdf"),
            ("line\nbreak\u{0}.txt", "linebreak.txt"),
            ("Manual NE555.pdf", "Manual NE555.pdf"),
        ] {
            assert_eq!(sanitize_file_name(given), safe, "{given:?}");
        }
        let long = format!("{}.pdf", "é".repeat(400));
        let safe = sanitize_file_name(&long);
        assert!(safe.len() <= MAX_FILE_NAME_BYTES && safe.ends_with(".pdf"));
    }
}
