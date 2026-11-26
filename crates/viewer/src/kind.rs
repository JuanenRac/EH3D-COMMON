//! What a file is and whether it is dangerous. The decision is the one of the whole suite (`eh3d-filekind`): the cleaned name,
//! the first bytes and, for office files, what is inside; the dangerous answer wins over every other, and a kind the suite does
//! not know asks like a dangerous one.

use std::io::Cursor;

pub use eh3d_filekind::{classify, Classification, Danger, Kind, OfficeKind};

use crate::limits::ViewerLimits;
use crate::source::{read_all, read_head, Source};

/// [`classify`], and for office files a look inside for macros. What the source says about itself (the record of a project) is
/// added: it only makes the answer more careful. An office file too large to look into asks before it leaves the program.
pub fn inspect(source: &dyn Source, limits: &ViewerLimits) -> Classification {
    let head = read_head(source, eh3d_filekind::HEAD_BYTES).unwrap_or_default();
    let mut found = classify(source.name(), &head);
    if found.danger.is_none() && matches!(found.kind, Kind::Office(_)) {
        let limit = limits.capped(limits.max_office_file_bytes);
        let looked = read_all(source, limit).ok().and_then(|data| {
            eh3d_filekind::classify_file(source.name(), &mut Cursor::new(data)).ok()
        });
        match looked {
            Some(whole) => found = whole,
            None => found.known = false,
        }
    }
    if let Some(danger) = source.declared_danger() {
        found.warn(danger);
        found.known = true;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MemorySource;
    use std::io::Write;

    fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

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
