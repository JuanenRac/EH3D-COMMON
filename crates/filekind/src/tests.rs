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

#[test]
fn a_kind_the_suite_does_not_know_asks_like_a_dangerous_one() {
    for name in ["thing.abc123", "noextension", "data.s19x"] {
        let found = classify(name, b"");
        assert_eq!(found.danger, None, "{name}");
        assert!(
            !found.known && found.needs_consent() && found.may_launch(),
            "{name}"
        );
        assert!(!is_known_name(name), "{name}");
    }
    for name in [
        "a.pdf",
        "a.md",
        "a.txt",
        "a.json",
        "a.csv",
        "a.png",
        "a.tiff",
        "a.docx",
        "a.xls",
        "a.mp4",
        "a.mp3",
        "a.step",
        "a.brd",
        "a.eh3d",
        "a.phoenixproj",
        "a.lbrn2",
    ] {
        let found = classify(name, b"");
        assert!(found.known && !found.needs_consent(), "{name}");
        assert!(is_known_name(name), "{name}");
    }
}

#[test]
fn a_zip_is_expected_only_behind_the_formats_that_are_one() {
    for name in [
        "a.docx",
        "a.ods",
        "a.xls",
        "a.3mf",
        "a.eh3d",
        "a.eh3dproj",
        "a.fcstd",
        "a.phoenixproj",
    ] {
        assert_eq!(danger_of(name, b"PK"), None, "{name}");
    }
    for name in ["a.pdf", "a.csv", "a.rtf", "a.txt", "a.brd"] {
        assert_eq!(danger_of(name, b"PK"), Some(Danger::Archive), "{name}");
    }
}

#[test]
fn each_name_has_one_view() {
    for (name, kind) in [
        ("Data.PDF", Kind::Pdf),
        ("photo.JPG", Kind::Picture),
        ("scan.tiff", Kind::Other),
        ("read.md", Kind::Markdown),
        ("bom.csv", Kind::Table),
        ("drc.json", Kind::Text),
        ("job.gcode", Kind::Text),
        ("logo.svg", Kind::Text),
        ("a.docx", Kind::Office(OfficeKind::Docx)),
        ("a.ods", Kind::Office(OfficeKind::Ods)),
        ("a.xls", Kind::Office(OfficeKind::Legacy)),
        ("clip.mp4", Kind::Media),
        ("case.step", Kind::Cad),
        ("case.eh3d", Kind::Cad),
        ("thing.abc123", Kind::Other),
    ] {
        assert_eq!(classify(name, b"").kind, kind, "{name}");
    }
}

#[test]
fn a_warning_from_a_record_only_adds_care() {
    let mut found = classify("manual.pdf", b"");
    found.warn(Danger::Executable);
    assert_eq!(
        (found.danger, found.kind),
        (Some(Danger::Executable), Kind::Other)
    );
    found.warn(Danger::Archive);
    assert_eq!(found.danger, Some(Danger::Executable), "never less careful");
    assert!(!found.may_launch() && found.needs_consent());
}

/// An end record of a list of entries that has nothing in it.
const EMPTY_END: &[u8; 22] = b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

#[test]
fn a_second_end_record_in_the_comment_does_not_hide_macros() {
    let plain = zip_of(&["word/document.xml"]);
    assert_eq!(of_bytes("a.docx", &plain), None);
    let mut macros = zip_of(&["word/document.xml", "word/vbaProject.bin"]);
    // the comment of the real end record is a second one that lists no entry
    let comment_at = macros.len() - 2;
    macros[comment_at..].copy_from_slice(&22u16.to_le_bytes());
    macros.extend_from_slice(EMPTY_END);
    assert_eq!(of_bytes("a.docx", &macros), Some(Danger::Macros));
    // and neither an empty archive nor one with bytes after its end can be looked into
    let mut empty = b"PK\x03\x04".to_vec();
    empty.extend_from_slice(EMPTY_END);
    assert_eq!(of_bytes("a.docx", &empty), Some(Danger::Macros));
    let mut trailing = plain.clone();
    trailing.extend_from_slice(b"more");
    assert_eq!(of_bytes("a.docx", &trailing), Some(Danger::Macros));
    assert_eq!(
        of_bytes("a.xlsx", &zip_of(&["xl/macrosheets/sheet1.xml"])),
        Some(Danger::Macros)
    );
}

#[test]
fn an_office_archive_cut_or_changed_is_judged_without_a_panic() {
    let whole = zip_of(&["word/document.xml", "word/vbaProject.bin"]);
    for cut in 4..whole.len() {
        assert_eq!(
            of_bytes("a.docx", &whole[..cut]),
            Some(Danger::Macros),
            "cut at {cut}"
        );
    }
    for at in 0..whole.len() {
        for bit in [0x01, 0x80, 0xff] {
            let mut bytes = whole.clone();
            bytes[at] ^= bit;
            let _ = of_bytes("a.docx", &bytes);
        }
    }
}

#[test]
fn a_cleaned_name_cleans_to_itself() {
    let pieces = [
        "a", "Z", " ", ".", "..", "-", "_", "/", "\\", ":", "*", "é", "日", "\u{202e}", "\u{0}",
        "CON", "nul", "COM1", "exe", "pdf",
    ];
    let mut state = 0x5EED_0002u64;
    let mut below = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    let mut names: Vec<String> = vec![
        format!("{}.{}", "a".repeat(149), "b".repeat(30)),
        format!("CON{}x", " ".repeat(200)),
        format!("com1{}x.pdf", " ".repeat(200)),
    ];
    for _ in 0..5_000 {
        let mut name = String::new();
        for _ in 0..1 + below(8) {
            let piece = pieces[below(pieces.len())];
            let times = if below(6) == 0 { 1 + below(80) } else { 1 };
            name.push_str(&piece.repeat(times));
        }
        names.push(name);
    }
    for name in names {
        let once = sanitize_file_name(&name);
        assert_eq!(sanitize_file_name(&once), once, "{name:?}");
        assert!(
            !once.is_empty() && once.len() <= MAX_FILE_NAME_BYTES,
            "{name:?}"
        );
        assert!(
            !once.ends_with('.')
                && !once.starts_with(['.', '-'])
                && !once.contains(['/', '\\', ':']),
            "{name:?} {once:?}"
        );
    }
}
