//! Taking attachments out of the program: names, temporary copies, agreement and what is never run.

mod support;

use eh3d_viewer::outside::{sanitize_name, save_copy, Consent, Outbox};
use eh3d_viewer::{MemorySource, ViewerError, ViewerLimits};
use support::{Folder, Recorder};

fn source(name: &str, data: &[u8]) -> MemorySource {
    MemorySource::new(name, data.to_vec())
}

#[test]
fn names_are_cleaned() {
    for (given, cleaned) in [
        ("datasheet LM317.pdf", "datasheet LM317.pdf"),
        ("../../etc/passwd", "passwd"),
        ("C:\\Windows\\evil.dll", "evil.dll"),
        ("a<b>c:d\"e|f?g*h,i;j.txt", "a_b_c_d_e_f_g_h_i_j.txt"),
        ("manual\u{202e}fdp.exe", "manualfdp.exe"),
        ("line\nbreak\u{0}.md", "linebreak.md"),
        ("  .hidden. ", "hidden"),
        ("-rf.txt", "_rf.txt"),
        ("CON.txt", "_CON.txt"),
        ("com1", "_com1"),
        ("lpt9.log", "_lpt9.log"),
        ("console.txt", "console.txt"),
        ("", "attachment"),
        ("..", "attachment"),
        ("folder/", "attachment"),
    ] {
        assert_eq!(sanitize_name(given), cleaned, "{given:?}");
    }
    let long = sanitize_name(&format!("{}.pdf", "é".repeat(500)));
    assert!(long.len() <= 150 && long.ends_with(".pdf"), "{long}");
    let all_extension = sanitize_name(&format!("a.{}", "x".repeat(500)));
    assert!(all_extension.len() <= 150);
}

#[test]
fn a_safe_file_is_copied_under_its_name_and_handed_to_the_system() {
    let base = Folder::new("safe");
    let recorder = Recorder::default();
    let mut launcher = recorder.clone();
    let limits = ViewerLimits::default();
    let copy;
    {
        let mut outbox = Outbox::under(base.path());
        let file = source("../notes.txt", b"hello");
        copy = outbox
            .open(&file, Consent::NotAsked, &limits, &mut launcher)
            .unwrap();
        assert_eq!(copy.file_name().unwrap(), "notes.txt");
        assert!(copy.starts_with(base.path()));
        assert_eq!(std::fs::read(&copy).unwrap(), b"hello");
        assert_eq!(recorder.opened(), vec![copy.clone()]);
        // a second copy of the same name does not replace the first
        let second = outbox.extract(&file, Consent::NotAsked, &limits).unwrap();
        assert_ne!(second, copy);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&copy).unwrap().permissions().mode();
            assert_eq!(mode & 0o177, 0, "private and never executable: {mode:o}");
        }
    }
    assert!(!copy.exists(), "removed with the folder");
}

#[test]
fn a_program_or_a_script_is_never_launched() {
    let base = Folder::new("never");
    let recorder = Recorder::default();
    let mut launcher = recorder.clone();
    let mut outbox = Outbox::under(base.path());
    let limits = ViewerLimits::default();
    for file in [
        source("setup.exe", b"MZ\0"),
        source("run.bat", b"echo"),
        source("tool.sh", b"#!/bin/sh"),
        source("x.desktop", b"[Desktop Entry]"),
        source("looks.pdf", b"MZ\x90\0"),
        source("looks.txt", b"#!/usr/bin/env python"),
        source("trick.exe. ", b""),
    ] {
        for consent in [Consent::NotAsked, Consent::Given] {
            assert_eq!(
                outbox.open(&file, consent, &limits, &mut launcher),
                Err(ViewerError::Refused)
            );
        }
    }
    assert!(recorder.opened().is_empty());
    assert_eq!(
        std::fs::read_dir(base.path()).unwrap().count(),
        0,
        "nothing was written"
    );
}

#[test]
fn dangerous_and_unknown_files_need_agreement() {
    let base = Folder::new("consent");
    let recorder = Recorder::default();
    let mut launcher = recorder.clone();
    let mut outbox = Outbox::under(base.path());
    let limits = ViewerLimits::default();
    for file in [
        source("book.xlsm", b"PK\x03\x04"),
        source("pack.zip", b"PK\x03\x04"),
        source("page.html", b"<p>"),
        source("thing.weird", b"data"),
    ] {
        assert_eq!(
            outbox.open(&file, Consent::NotAsked, &limits, &mut launcher),
            Err(ViewerError::ConsentRequired)
        );
        assert_eq!(
            outbox.extract(&file, Consent::NotAsked, &limits),
            Err(ViewerError::ConsentRequired)
        );
        outbox
            .open(&file, Consent::Given, &limits, &mut launcher)
            .unwrap();
    }
    assert_eq!(recorder.opened().len(), 4);
}

#[test]
fn a_saved_copy_never_replaces_a_file_and_keeps_the_limit() {
    let folder = Folder::new("save");
    let limits = ViewerLimits::default();
    let to = folder.path().join("copy.txt");
    let file = source("a.txt", b"hello");
    save_copy(&file, &to, Consent::NotAsked, &limits).unwrap();
    assert!(matches!(
        save_copy(&file, &to, Consent::Given, &limits),
        Err(ViewerError::Io(_))
    ));
    assert_eq!(std::fs::read(&to).unwrap(), b"hello");
    // saving a harmless attachment under the name of a program asks first
    let renamed = folder.path().join("copy.exe");
    assert_eq!(
        save_copy(&file, &renamed, Consent::NotAsked, &limits),
        Err(ViewerError::ConsentRequired)
    );
    assert!(!renamed.exists());
    let small = ViewerLimits {
        max_attachment_bytes: 4,
        ..ViewerLimits::default()
    };
    let cut = folder.path().join("cut.txt");
    assert_eq!(
        save_copy(&file, &cut, Consent::NotAsked, &small),
        Err(ViewerError::TooLarge { limit: 4 })
    );
    assert!(!cut.exists(), "no partial file stays");
}
