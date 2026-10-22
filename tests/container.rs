//! The package must give back exactly what was put in, and refuse everything that was changed.

use std::io::{Cursor, Write};

use eh3d_common::container::{check_entry_name, Integrity, INTEGRITY_ENTRY, PROJECT_ENTRY};
use eh3d_common::project::{LayerKind, LayerRef};
use eh3d_common::{Error, Package, ProjectMetadata};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn sample() -> Package {
    let mut metadata = ProjectMetadata::new("Test board");
    metadata.layers.push(LayerRef {
        path: "layers/top.gbr".into(),
        kind: LayerKind::Copper,
        name: "Top".into(),
    });
    let mut package = Package::new(metadata);
    package
        .add_entry("layers/top.gbr", b"G04 test*\nM02*\n".to_vec())
        .unwrap();
    package
        .add_entry("toolpaths/iso.nc", b"G21\nG0 X0 Y0\n".to_vec())
        .unwrap();
    package
        .add_entry("vision/fiducials.json", b"[]".to_vec())
        .unwrap();
    package
}

/// Rebuilds a package after `change` edited its entries (a manipulated copy of a valid package).
fn tamper(bytes: &[u8], change: impl Fn(&str, &mut Vec<u8>) -> bool) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let name = file.name().to_owned();
        let mut content = Vec::new();
        std::io::copy(&mut file, &mut content).unwrap();
        if change(&name, &mut content) {
            out.start_file(name.as_str(), SimpleFileOptions::default())
                .unwrap();
            out.write_all(&content).unwrap();
        }
    }
    out.finish().unwrap().into_inner()
}

#[test]
fn a_package_round_trips_through_memory() {
    let package = sample();
    let bytes = package.to_bytes().unwrap();
    let back = Package::read(Cursor::new(bytes)).unwrap();
    assert_eq!(back, package);
    assert_eq!(back.entry("layers/top.gbr").unwrap(), b"G04 test*\nM02*\n");
    assert_eq!(
        back.entry_names().collect::<Vec<_>>(),
        [
            "layers/top.gbr",
            "toolpaths/iso.nc",
            "vision/fiducials.json"
        ]
    );
}

#[test]
fn a_package_round_trips_through_a_file_and_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.eh3dproj");
    let package = sample();
    package.write_to_path(&path).unwrap();
    assert_eq!(Package::read_from_path(&path).unwrap(), package);
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
}

#[test]
fn the_same_content_gives_the_same_bytes() {
    let package = sample();
    assert_eq!(package.to_bytes().unwrap(), package.to_bytes().unwrap());
}

#[test]
fn the_integrity_record_lists_every_entry_with_its_hash() {
    let package = sample();
    let integrity: Integrity = package.integrity().unwrap();
    assert_eq!(integrity.algorithm, "sha256");
    assert_eq!(integrity.files.len(), 4);
    assert!(integrity.files.contains_key(PROJECT_ENTRY));
    let record = &integrity.files["vision/fiducials.json"];
    assert_eq!(record.size, 2);
    // SHA-256 of "[]"
    assert_eq!(
        record.sha256,
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
    );
}

#[test]
fn a_changed_entry_is_detected() {
    let bytes = sample().to_bytes().unwrap();
    let damaged = tamper(&bytes, |name, content| {
        if name == "layers/top.gbr" {
            content[0] ^= 0x01;
        }
        true
    });
    match Package::read(Cursor::new(damaged)) {
        Err(Error::Integrity { path, .. }) => assert_eq!(path, "layers/top.gbr"),
        other => panic!("expected an integrity error, got {other:?}"),
    }
}

#[test]
fn a_changed_size_is_detected() {
    let bytes = sample().to_bytes().unwrap();
    let damaged = tamper(&bytes, |name, content| {
        if name == "toolpaths/iso.nc" {
            content.push(b'\n');
        }
        true
    });
    assert!(matches!(
        Package::read(Cursor::new(damaged)),
        Err(Error::Integrity { .. })
    ));
}

#[test]
fn an_entry_that_is_not_listed_is_refused() {
    let bytes = sample().to_bytes().unwrap();
    let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let name = file.name().to_owned();
        out.start_file(name.as_str(), SimpleFileOptions::default())
            .unwrap();
        std::io::copy(&mut file, &mut out).unwrap();
    }
    out.start_file("layers/extra.gbr", SimpleFileOptions::default())
        .unwrap();
    out.write_all(b"smuggled").unwrap();
    let with_extra = out.finish().unwrap().into_inner();
    match Package::read(Cursor::new(with_extra)) {
        Err(Error::Integrity { path, .. }) => assert_eq!(path, "layers/extra.gbr"),
        other => panic!("expected an integrity error, got {other:?}"),
    }
}

#[test]
fn a_missing_entry_and_a_missing_integrity_file_are_refused() {
    let bytes = sample().to_bytes().unwrap();
    let without_layer = tamper(&bytes, |name, _| name != "layers/top.gbr");
    assert!(
        matches!(Package::read(Cursor::new(without_layer)), Err(Error::MissingEntry(n)) if n == "layers/top.gbr")
    );
    let without_integrity = tamper(&bytes, |name, _| name != INTEGRITY_ENTRY);
    assert!(
        matches!(Package::read(Cursor::new(without_integrity)), Err(Error::MissingEntry(n)) if n == INTEGRITY_ENTRY)
    );
}

#[test]
fn a_package_whose_metadata_was_edited_is_refused() {
    let bytes = sample().to_bytes().unwrap();
    let edited = tamper(&bytes, |name, content| {
        if name == PROJECT_ENTRY {
            let text = String::from_utf8(content.clone())
                .unwrap()
                .replace("Test board", "Other board");
            *content = text.into_bytes();
        }
        true
    });
    assert!(
        matches!(Package::read(Cursor::new(edited)), Err(Error::Integrity { path, .. }) if path == PROJECT_ENTRY)
    );
}

#[test]
fn a_zip_that_is_not_a_package_is_refused() {
    assert!(Package::read(Cursor::new(b"this is not a zip".to_vec())).is_err());
    assert!(Package::read(Cursor::new(Vec::new())).is_err());
}

#[test]
fn unsafe_entry_names_are_refused_everywhere() {
    for bad in [
        "",
        "/etc/passwd",
        "../x",
        "layers/../x",
        "layers//x",
        "layers/./x",
        "layers\\x",
        "C:/x",
        "layers/x/",
        "layers/a\nb",
    ] {
        assert!(
            matches!(check_entry_name(bad), Err(Error::UnsafePath(_))),
            "{bad:?}"
        );
    }
    for good in ["layers/top.gbr", "toolpaths/a b.nc", "vision/x"] {
        assert!(check_entry_name(good).is_ok(), "{good:?}");
    }
    let mut package = sample();
    assert!(package.add_entry("../escape", vec![1]).is_err());
    assert!(
        package.add_entry("notes.txt", vec![1]).is_err(),
        "only layers/, toolpaths/ and vision/ hold data"
    );
    assert!(package.add_entry("project.json", vec![1]).is_err());
}

#[test]
fn a_hostile_entry_name_inside_a_zip_is_refused_when_reading() {
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    out.start_file("../../evil.txt", SimpleFileOptions::default())
        .unwrap();
    out.write_all(b"x").unwrap();
    let bytes = out.finish().unwrap().into_inner();
    assert!(matches!(
        Package::read(Cursor::new(bytes)),
        Err(Error::UnsafePath(_))
    ));
}

#[test]
fn replacing_an_entry_keeps_one_copy() {
    let mut package = sample();
    package
        .add_entry("layers/top.gbr", b"new".to_vec())
        .unwrap();
    assert_eq!(package.entry("layers/top.gbr").unwrap(), b"new");
    assert_eq!(package.entry_names().count(), 3);
}

#[test]
fn an_invalid_project_cannot_be_written() {
    let mut package = sample();
    package.metadata.name = String::new();
    assert!(package.to_bytes().is_err());
}
