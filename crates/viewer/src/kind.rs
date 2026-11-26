//! What a file is and whether it is dangerous. The decision uses the cleaned name, the first bytes
//! and, for office files, what is inside; the dangerous answer wins over every other.

use std::io::Cursor;

use crate::limits::ViewerLimits;
use crate::outside::sanitize_name;
use crate::source::{read_all, read_head, Source};

/// The office formats that have a preview of their text and tables, and those that have none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeKind {
    Docx,
    Xlsx,
    Pptx,
    Odt,
    Ods,
    Odp,
    Rtf,
    /// The binary formats before 2007 (`doc`, `xls`, `ppt`): no preview.
    Legacy,
}

/// How a file is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pdf,
    Picture,
    Text,
    Markdown,
    /// Comma or tab separated values, shown as a table.
    Table,
    Office(OfficeKind),
    /// Video and audio: opened outside.
    Media,
    /// Design files of other programs: opened outside.
    Cad,
    /// Anything else: opened outside.
    Other,
}

/// Why a file is dangerous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Danger {
    /// A program, or a file the system runs when opened. Never launched.
    Executable,
    /// A script. Never launched.
    Script,
    /// An office document with macros.
    Macros,
    /// A compressed archive, which is not looked into.
    Archive,
    /// A page that may run code in a browser (HTML, SVG).
    ActiveContent,
}

impl Danger {
    /// A program or a script is never launched, whatever a person confirms.
    pub fn never_launched(self) -> bool {
        matches!(self, Danger::Executable | Danger::Script)
    }
}

/// The answer for one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    /// The name after cleaning; the temporary copy gets this one.
    pub name: String,
    /// The extension in lowercase, empty when there is none.
    pub extension: String,
    pub kind: Kind,
    pub danger: Option<Danger>,
    /// False for a kind the suite does not know: opening it outside asks first.
    pub known: bool,
}

impl Classification {
    /// True when opening it outside, or saving it out, needs a person to confirm.
    pub fn needs_consent(&self) -> bool {
        self.danger.is_some() || !self.known
    }

    /// False for a program or a script.
    pub fn may_launch(&self) -> bool {
        !self.danger.is_some_and(Danger::never_launched)
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
const ARCHIVES: &[&str] = &[
    "7z", "ace", "arj", "bz2", "gz", "lz", "lzh", "lzma", "rar", "tar", "tbz", "tgz", "txz", "xz",
    "z", "zip", "zst",
];
const ACTIVE: &[&str] = &[
    "htm", "html", "mht", "mhtml", "shtml", "svg", "svgz", "xht", "xhtml",
];
const PICTURES: &[&str] = &["bmp", "gif", "jpeg", "jpg", "png", "webp"];
const OTHER_PICTURES: &[&str] = &["heic", "ico", "psd", "tif", "tiff"];
const MARKDOWN: &[&str] = &["markdown", "md"];
const TABLES: &[&str] = &["csv", "tsv"];
const TEXTS: &[&str] = &[
    "asm", "c", "cc", "cfg", "cmake", "cnc", "conf", "cpp", "drl", "gbl", "gbr", "gbs", "gcode",
    "gko", "gtl", "gto", "gts", "h", "hex", "hpp", "ini", "ino", "json", "log", "nc", "ngc", "plt",
    "rs", "tap", "toml", "txt", "xml", "yaml", "yml",
];
const MEDIA: &[&str] = &[
    "aac", "avi", "flac", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "oga", "ogg",
    "ogv", "opus", "wav", "weba", "webm", "wma", "wmv",
];
const CAD: &[&str] = &[
    "3mf",
    "brd",
    "dsn",
    "dwg",
    "dxf",
    "f3d",
    "fcstd",
    "iges",
    "igs",
    "kicad_mod",
    "kicad_pcb",
    "kicad_pro",
    "kicad_sch",
    "kicad_sym",
    "lbr",
    "obj",
    "ply",
    "sch",
    "sldasm",
    "sldprt",
    "step",
    "stl",
    "stp",
];

fn extension_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// The danger that the first bytes alone show, whatever the name says.
fn danger_of_head(head: &[u8]) -> Option<Danger> {
    // ELF, then the forms of Mach-O
    let magics: &[&[u8]] = &[
        b"\x7fELF",
        b"\xfe\xed\xfa\xce",
        b"\xfe\xed\xfa\xcf",
        b"\xce\xfa\xed\xfe",
        b"\xcf\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe",
    ];
    // two letters are a weak sign: a text that happens to start with them has no zero byte
    if head.starts_with(b"MZ") {
        return head.contains(&0).then_some(Danger::Executable);
    }
    if magics.iter().any(|m| head.starts_with(m)) {
        return Some(Danger::Executable);
    }
    if head.starts_with(b"#!") {
        return Some(Danger::Script);
    }
    None
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

/// What the name and the first bytes say. [`inspect`] also looks inside office files.
pub fn classify(name: &str, head: &[u8]) -> Classification {
    let name = sanitize_name(name);
    let extension = extension_of(&name);
    let ext = extension.as_str();
    let has = |list: &[&str]| list.contains(&ext);
    let mut known = true;
    let mut danger = None;
    let mut kind = if ext == "pdf" {
        Kind::Pdf
    } else if has(PICTURES) {
        Kind::Picture
    } else if has(OTHER_PICTURES) {
        Kind::Other
    } else if has(MARKDOWN) {
        Kind::Markdown
    } else if has(TABLES) {
        Kind::Table
    } else if has(TEXTS) {
        Kind::Text
    } else if has(MEDIA) {
        Kind::Media
    } else if has(CAD) {
        Kind::Cad
    } else {
        match ext {
            "docx" | "dotx" | "docm" | "dotm" => Kind::Office(OfficeKind::Docx),
            "xlsx" | "xltx" | "xlsm" | "xltm" => Kind::Office(OfficeKind::Xlsx),
            "pptx" | "potx" | "ppsx" | "pptm" | "potm" | "ppsm" => Kind::Office(OfficeKind::Pptx),
            "odt" | "ott" => Kind::Office(OfficeKind::Odt),
            "ods" | "ots" => Kind::Office(OfficeKind::Ods),
            "odp" | "otp" => Kind::Office(OfficeKind::Odp),
            "rtf" => Kind::Office(OfficeKind::Rtf),
            "doc" | "dot" | "xls" | "xlt" | "ppt" | "pps" | "pot" => {
                Kind::Office(OfficeKind::Legacy)
            }
            _ => {
                known = false;
                Kind::Other
            }
        }
    };

    if has(EXECUTABLES) {
        danger = Some(Danger::Executable);
        known = true;
    } else if has(SCRIPTS) {
        danger = Some(Danger::Script);
        known = true;
    } else if has(MACRO_OFFICE) {
        danger = Some(Danger::Macros);
        known = true;
    } else if has(ARCHIVES) {
        danger = Some(Danger::Archive);
        known = true;
    } else if has(ACTIVE) {
        danger = Some(Danger::ActiveContent);
        known = true;
        kind = Kind::Text;
    }

    // what the bytes say wins when the name says something harmless
    if let Some(found) = danger_of_head(head) {
        if !danger.is_some_and(Danger::never_launched) {
            danger = Some(found);
            kind = Kind::Other;
        }
    } else if danger.is_none() && is_other_archive(head) {
        danger = Some(Danger::Archive);
        kind = Kind::Other;
    } else if danger.is_none() && is_zip(head) {
        // a zip behind the name of a design file is the normal form of that format
        let expected = matches!(kind, Kind::Office(k) if k != OfficeKind::Rtf)
            || matches!(ext, "3mf" | "f3d" | "fcstd");
        if !expected {
            danger = Some(Danger::Archive);
            kind = Kind::Other;
        }
    }

    Classification {
        name,
        extension,
        kind,
        danger,
        known,
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// True when an office archive carries macros, whatever its extension says.
fn zip_has_macros(data: &[u8], max_entries: u64) -> Option<bool> {
    let archive = zip::ZipArchive::new(Cursor::new(data)).ok()?;
    if archive.len() as u64 > max_entries {
        return None;
    }
    let found = archive.file_names().filter_map(Result::ok).any(|n| {
        let n = n.to_ascii_lowercase();
        n.ends_with("vbaproject.bin")
            || n.ends_with("vbadata.xml")
            || n.starts_with("basic/")
            || n.starts_with("scripts/")
    });
    Some(found)
}

/// [`classify`], and for office files a look inside for macros. A file that cannot be looked into
/// counts as dangerous.
pub fn inspect(source: &dyn Source, limits: &ViewerLimits) -> Classification {
    let head = read_head(source, 4096).unwrap_or_default();
    let mut found = classify(source.name(), &head);
    if found.danger.is_some() || !matches!(found.kind, Kind::Office(_)) {
        return found;
    }
    let limit = limits.capped(limits.max_office_file_bytes);
    let Ok(data) = read_all(source, limit) else {
        // too large to look into: the person is asked before it leaves the program
        found.known = false;
        return found;
    };
    if is_zip(&head) {
        match zip_has_macros(&data, limits.max_office_entries) {
            Some(false) => {}
            Some(true) => found.danger = Some(Danger::Macros),
            None => found.known = false,
        }
    } else if is_ole(&head)
        && (contains(&data, &utf16("_VBA_PROJECT")) || contains(&data, &utf16("Macros")))
    {
        found.danger = Some(Danger::Macros);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MemorySource;
    use std::io::Write;

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

    #[test]
    fn each_family_of_files_gets_its_view() {
        for (name, kind) in [
            ("Data.PDF", Kind::Pdf),
            ("photo.JPG", Kind::Picture),
            ("read.md", Kind::Markdown),
            ("bom.csv", Kind::Table),
            ("job.gcode", Kind::Text),
            ("a.docx", Kind::Office(OfficeKind::Docx)),
            ("a.ods", Kind::Office(OfficeKind::Ods)),
            ("a.xls", Kind::Office(OfficeKind::Legacy)),
            ("clip.mp4", Kind::Media),
            ("case.step", Kind::Cad),
            ("thing.abc123", Kind::Other),
        ] {
            let c = classify(name, b"");
            assert_eq!(c.kind, kind, "{name}");
            assert_eq!(c.danger, None, "{name}");
        }
        assert!(!classify("thing.abc123", b"").known);
        assert!(classify("thing.abc123", b"").needs_consent());
        assert!(classify("noextension", b"").needs_consent());
    }

    #[test]
    fn programs_scripts_macros_archives_and_pages_are_dangerous() {
        for (name, danger) in [
            ("setup.exe", Danger::Executable),
            ("a.LNK", Danger::Executable),
            ("eagle.scr", Danger::Executable),
            ("run.ps1", Danger::Script),
            ("tool.py", Danger::Script),
            ("report.xlsm", Danger::Macros),
            ("files.zip", Danger::Archive),
            ("files.7z", Danger::Archive),
            ("page.html", Danger::ActiveContent),
            ("logo.svg", Danger::ActiveContent),
        ] {
            let c = classify(name, b"");
            assert_eq!(c.danger, Some(danger), "{name}");
            assert!(c.needs_consent(), "{name}");
        }
        assert!(!classify("setup.exe", b"").may_launch());
        assert!(!classify("run.ps1", b"").may_launch());
        assert!(classify("report.xlsm", b"").may_launch());
    }

    #[test]
    fn the_bytes_win_over_a_harmless_name() {
        let c = classify("datasheet.pdf", b"MZ\x90\x00\x03");
        assert_eq!(c.danger, Some(Danger::Executable));
        assert_eq!(c.kind, Kind::Other);
        assert_eq!(
            classify("notes.txt", b"#!/bin/sh\n").danger,
            Some(Danger::Script)
        );
        assert_eq!(
            classify("photo.png", b"\x7fELF\x02").danger,
            Some(Danger::Executable)
        );
        assert_eq!(
            classify("manual.pdf", b"PK\x03\x04").danger,
            Some(Danger::Archive)
        );
        assert_eq!(classify("part.3mf", b"PK\x03\x04").danger, None);
        // a text that only happens to start with those two letters
        assert_eq!(classify("notes.txt", b"MZ is a code").danger, None);
    }

    #[test]
    fn a_name_that_hides_its_real_extension_is_judged_by_the_real_one() {
        assert_eq!(
            classify("manual.pdf.exe", b"").danger,
            Some(Danger::Executable)
        );
        assert_eq!(
            classify("run.exe. . ", b"").danger,
            Some(Danger::Executable)
        );
        assert_eq!(
            classify("a\u{202e}fdp.exe", b"").danger,
            Some(Danger::Executable)
        );
    }

    #[test]
    fn macros_inside_a_document_with_a_harmless_extension_are_found() {
        let limits = ViewerLimits::default();
        let clean = MemorySource::new("a.docx", zip_of(&["word/document.xml"]));
        assert_eq!(inspect(&clean, &limits).danger, None);
        let hidden = MemorySource::new("a.docx", zip_of(&["word/vbaProject.bin"]));
        assert_eq!(inspect(&hidden, &limits).danger, Some(Danger::Macros));
        let basic = MemorySource::new("a.odt", zip_of(&["Basic/script.xml"]));
        assert_eq!(inspect(&basic, &limits).danger, Some(Danger::Macros));
        let mut ole = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1".to_vec();
        ole.extend(utf16("_VBA_PROJECT"));
        let old = MemorySource::new("a.xls", ole);
        assert_eq!(inspect(&old, &limits).danger, Some(Danger::Macros));
        let broken = MemorySource::new("a.docx", b"PK\x03\x04broken".to_vec());
        assert!(inspect(&broken, &limits).needs_consent());
    }
}
