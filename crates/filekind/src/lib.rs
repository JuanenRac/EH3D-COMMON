//! What a file attached to a project is: the one classification of the suite, used by the project container (`eh3d-project`), the
//! viewer (`eh3d-viewer`) and the attachment panel.
//!
//! The name is cleaned first, then the extension, the first bytes and, for office documents, what is inside are looked at. The
//! dangerous answer wins over every other. The rule is careful on purpose: programs, scripts, documents with macros, compressed
//! archives, pages that can run code (HTML, SVG) **and kinds the suite does not know** need the agreement of a person before they
//! are taken out or opened ([`Classification::needs_consent`]). Programs and scripts are never launched. Nothing here runs or
//! opens anything.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::io::{self, Read, Seek, SeekFrom};

/// The longest name that is kept, in bytes.
pub const MAX_FILE_NAME_BYTES: usize = 150;
/// The bytes of the start of a file that say what it is.
pub const HEAD_BYTES: usize = 4096;

/// What a file is for the record of an attachment, by its name.
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
            _ if has(PICTURES) || has(OTHER_PICTURES) || extension == "svg" => {
                AttachmentKind::Image
            }
            _ if has(TEXTS) || extension == "htm" || extension == "html" => AttachmentKind::Text,
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

/// The office formats that have a preview of their text and tables, and those that have none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeKind {
    /// Word, Office Open XML.
    Docx,
    /// Excel, Office Open XML.
    Xlsx,
    /// PowerPoint, Office Open XML.
    Pptx,
    /// OpenDocument text.
    Odt,
    /// OpenDocument spreadsheet.
    Ods,
    /// OpenDocument presentation.
    Odp,
    /// Rich text.
    Rtf,
    /// The binary formats before 2007 (`doc`, `xls`, `ppt`): no preview.
    Legacy,
}

/// How a file is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// PDF pages.
    Pdf,
    /// A picture drawn inside.
    Picture,
    /// Plain text.
    Text,
    /// Markdown.
    Markdown,
    /// Comma or tab separated values, shown as a table.
    Table,
    /// An office document.
    Office(OfficeKind),
    /// Video and audio: opened outside.
    Media,
    /// Design files: opened outside.
    Cad,
    /// Anything else: opened outside.
    Other,
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

/// The answer for one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    /// The name after cleaning; a copy gets this one.
    pub name: String,
    /// The extension in lowercase, empty when there is none.
    pub extension: String,
    /// How it is shown.
    pub kind: Kind,
    /// Why it is dangerous, when it is.
    pub danger: Option<Danger>,
    /// False for a kind the suite does not know: taking it out or opening it asks first.
    pub known: bool,
}

impl Classification {
    /// True when taking it out or opening it needs a person to agree.
    pub fn needs_consent(&self) -> bool {
        self.danger.is_some() || !self.known
    }

    /// False for a program or a script.
    pub fn may_launch(&self) -> bool {
        !self.danger.is_some_and(Danger::never_launched)
    }

    /// Adds a danger that something else says (the record of a project): it only ever makes the answer more careful.
    pub fn warn(&mut self, danger: Danger) {
        if !self.danger.is_some_and(Danger::never_launched) {
            self.danger = Some(danger);
            if danger.never_launched() {
                self.kind = Kind::Other;
            }
        }
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
/// The office formats that are a ZIP archive, and the old ones, which are looked into when they turn out to be one.
const ZIPPED_OFFICE: &[&str] = &[
    "doc", "dot", "docx", "dotx", "odp", "ods", "odt", "otp", "ots", "ott", "pot", "potx", "pps",
    "ppsx", "ppt", "pptx", "xls", "xlt", "xlsx", "xltx",
];
/// The design files whose normal form is a ZIP archive.
const ZIPPED_DESIGN: &[&str] = &["3mf", "eh3d", "eh3dproj", "f3d", "fcstd", "phoenixproj"];
const ARCHIVES: &[&str] = &[
    "7z", "ace", "arj", "bz2", "gz", "lz", "lzh", "lzma", "rar", "tar", "tbz", "tgz", "txz", "xz",
    "z", "zip", "zst",
];
const ACTIVE: &[&str] = &[
    "htm", "html", "mht", "mhtml", "shtml", "svg", "svgz", "xht", "xhtml",
];
/// The pictures drawn inside.
const PICTURES: &[&str] = &["bmp", "gif", "jpeg", "jpg", "png", "webp"];
/// The pictures that are known and opened outside.
const OTHER_PICTURES: &[&str] = &["heic", "ico", "psd", "tif", "tiff"];
const TEXTS: &[&str] = &[
    "asm", "c", "cc", "cfg", "cmake", "cnc", "conf", "cpp", "drl", "gbl", "gbr", "gbs", "gcode",
    "gko", "gtl", "gto", "gts", "h", "hex", "hpp", "ini", "ino", "log", "nc", "ngc", "plt", "rs",
    "tap", "toml", "txt", "xml", "yaml", "yml",
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
    "lbrn",
    "lbrn2",
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
pub fn extension_of(name: &str) -> String {
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
        // the cut may have left a dot at the end, which Windows drops and a second cleaning would take away
        let kept = cleaned
            .trim_end_matches(|c: char| c == '.' || c.is_whitespace())
            .len();
        cleaned.truncate(kept);
        // or have left nothing but a device name (`CON` and a long run of spaces)
        if reserved_on_windows(cleaned.split('.').next().unwrap_or("")) {
            cleaned.insert(0, '_');
        }
    }
    cleaned
}

fn danger_of_extension(extension: &str) -> Option<Danger> {
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

fn office_kind(extension: &str) -> Option<OfficeKind> {
    Some(match extension {
        "docx" | "dotx" | "docm" | "dotm" => OfficeKind::Docx,
        "xlsx" | "xltx" | "xlsm" | "xltm" => OfficeKind::Xlsx,
        "pptx" | "potx" | "ppsx" | "pptm" | "potm" | "ppsm" => OfficeKind::Pptx,
        "odt" | "ott" => OfficeKind::Odt,
        "ods" | "ots" => OfficeKind::Ods,
        "odp" | "otp" => OfficeKind::Odp,
        "rtf" => OfficeKind::Rtf,
        "doc" | "dot" | "xls" | "xlt" | "ppt" | "pps" | "pot" => OfficeKind::Legacy,
        _ => return None,
    })
}

/// How a file of this extension is shown, and whether the suite knows the extension.
fn view_of_extension(extension: &str) -> (Kind, bool) {
    let has = |list: &[&str]| list.contains(&extension);
    let kind = match extension {
        "pdf" => Kind::Pdf,
        "md" | "markdown" => Kind::Markdown,
        "csv" | "tsv" => Kind::Table,
        "json" => Kind::Text,
        _ if has(PICTURES) => Kind::Picture,
        _ if has(OTHER_PICTURES) => Kind::Other,
        _ if has(TEXTS) || has(ACTIVE) => Kind::Text,
        _ if has(VIDEO) || has(AUDIO) => Kind::Media,
        _ if has(DESIGN) || has(MODELS) => Kind::Cad,
        _ => match office_kind(extension) {
            Some(office) => Kind::Office(office),
            None => return (Kind::Other, danger_of_extension(extension).is_some()),
        },
    };
    (kind, true)
}

/// Whether the suite knows the kind of a file with this name. An unknown kind is asked about like a dangerous one.
pub fn is_known_name(name: &str) -> bool {
    view_of_extension(&extension_of(&sanitize_file_name(name))).1
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

/// What the name and the first bytes of a file ([`HEAD_BYTES`] are enough) say. What the bytes say wins when the name says
/// something harmless. [`classify_file`] also looks inside office documents.
pub fn classify(name: &str, head: &[u8]) -> Classification {
    let name = sanitize_file_name(name);
    let extension = extension_of(&name);
    let (mut kind, mut known) = view_of_extension(&extension);
    let mut danger = danger_of_extension(&extension);
    if let Some(found) = danger_of_program(head) {
        if !danger.is_some_and(Danger::never_launched) {
            danger = Some(found);
            kind = Kind::Other;
        }
    } else if danger.is_none() && is_other_archive(head) {
        danger = Some(Danger::Archive);
        kind = Kind::Other;
    } else if danger.is_none() && is_zip(head) {
        // a zip behind the name of an office document or of some design files is the normal form of that format
        let expected = ZIPPED_OFFICE.contains(&extension.as_str())
            || ZIPPED_DESIGN.contains(&extension.as_str());
        if !expected {
            danger = Some(Danger::Archive);
            kind = Kind::Other;
        }
    }
    known |= danger.is_some();
    Classification {
        name,
        extension,
        kind,
        danger,
        known,
    }
}

/// Why a name and the first bytes of a file say it is dangerous, when they do.
pub fn danger_of(name: &str, head: &[u8]) -> Option<Danger> {
    classify(name, head).danger
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
        // the end record is the one whose comment ends the file
        .filter(|&at| at + 22 <= tail.len() && at + 22 + le16(&tail, at + 20) == tail.len())
    else {
        return Ok(None);
    };
    let entries = le16(&tail, end + 10);
    let end_at = len - tail_len + end as u64;
    let size = le32(&tail, end + 12);
    let offset = le32(&tail, end + 16);
    // the list of entries lies just before the end record; otherwise the real one is another than it says
    if size > MAX_DIRECTORY_BYTES || offset.checked_add(size) != Some(end_at) {
        return Ok(None);
    }
    let mut directory = vec![0u8; size as usize];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut directory)?;
    let mut at = 0;
    let mut seen = 0;
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
            || name.starts_with("xl/macrosheets/")
        {
            return Ok(Some(true));
        }
        at = next;
        seen += 1;
    }
    // an office document with no entry, or with another number than it says, cannot be looked into
    Ok((seen > 0 && seen == entries).then_some(false))
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

/// [`classify`] for a whole file: office documents are also looked into for macros, whatever their extension says. A document
/// that cannot be looked into counts as one with macros. The file is read in blocks; nothing is decompressed or run.
pub fn classify_file(name: &str, file: &mut (impl Read + Seek)) -> io::Result<Classification> {
    file.seek(SeekFrom::Start(0))?;
    let mut head = Vec::new();
    (&mut *file)
        .take(HEAD_BYTES as u64)
        .read_to_end(&mut head)?;
    let mut found = classify(name, &head);
    if found.danger.is_some() || AttachmentKind::of_name(&found.name) != AttachmentKind::Office {
        return Ok(found);
    }
    let macros = if is_zip(&head) {
        zip_names_macros(file)? != Some(false)
    } else {
        is_ole(&head) && ole_names_macros(file)?
    };
    if macros {
        found.danger = Some(Danger::Macros);
        found.known = true;
    }
    Ok(found)
}

/// Why a whole file is dangerous, when it is ([`classify_file`]).
pub fn danger_of_file(name: &str, file: &mut (impl Read + Seek)) -> io::Result<Option<Danger>> {
    Ok(classify_file(name, file)?.danger)
}

#[cfg(test)]
mod tests;
