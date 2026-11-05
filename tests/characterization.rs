//! What the `.eh3dproj` container does today, recorded as tests and as a corpus of reference packages.
//!
//! These tests describe the behaviour of format 1.0.0 as it is, not as it should be: the order of
//! the entries, the exact bytes of `project.json` and `integrity.json`, the ZIP flags, the compression
//! method, the time stamps, the absence of ZIP64, and the verdict of the reader on a set of damaged
//! packages. A change of the container that alters any of them must show up here as a failure and be
//! a conscious decision.
//!
//! The reference packages live in `tests/golden/`. The writer must reproduce the valid ones byte for
//! byte, the reader must read them back, and the reader must give the recorded verdict on the damaged
//! ones in `tests/golden/rejected/`. To rebuild the files after a deliberate change run
//! `cargo test --test characterization regenerate_golden -- --ignored`.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use eh3d_common::container::{Integrity, IntegrityEntry, INTEGRITY_ENTRY, PROJECT_ENTRY};
use eh3d_common::project::read_metadata;
use eh3d_common::{Error, Package};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

const PROJECT_JSON: &str = r#"{
  "format_version": "1.0.0",
  "id": "5f3c1c5e-5b0a-4f0e-9a43-0d6d3f6b8c11",
  "name": "Reference board",
  "created": "2026-01-02T03:04:05Z",
  "modified": "2026-01-02T03:04:06Z",
  "units": "millimetres",
  "zero_piece": { "x": 0.0, "y": 0.0, "z": 0.0 },
  "layers": [{ "path": "layers/top.gbr", "kind": "copper", "name": "Top copper" }],
  "tools": [{ "id": "v30", "name": "V-bit 30 degrees", "diameter_mm": 0.1 }],
  "toolpaths": [{ "path": "toolpaths/iso.nc", "dialect": "grbl", "tool_id": "v30" }],
  "fiducials": [{ "name": "F1", "x": 2.0, "y": 2.5 }],
  "notes": null
}"#;

const MINIMAL_JSON: &str = r#"{
  "format_version": "1.0.0",
  "id": "00000000-0000-4000-8000-000000000001",
  "name": "Empty",
  "created": "2026-01-02T03:04:05Z",
  "modified": "2026-01-02T03:04:05Z"
}"#;

fn typical() -> Package {
    let mut package = Package::new(read_metadata(PROJECT_JSON).unwrap());
    package
        .add_entry("vision/fiducials.json", b"[]".to_vec())
        .unwrap();
    package
        .add_entry("toolpaths/iso.nc", b"G21\nG0 X0 Y0\n".to_vec())
        .unwrap();
    package
        .add_entry("layers/top.gbr", b"G04 reference*\nM02*\n".to_vec())
        .unwrap();
    package
}

fn minimal() -> Package {
    Package::new(read_metadata(MINIMAL_JSON).unwrap())
}

fn unicode_names() -> Package {
    let mut metadata = read_metadata(MINIMAL_JSON).unwrap();
    metadata.name = "Names beyond ASCII".into();
    let mut package = Package::new(metadata);
    package
        .add_entry("layers/ca\u{f1}\u{f3}n.gbr", b"M02*\n".to_vec())
        .unwrap();
    package
        .add_entry("vision/\u{4e2d}\u{6587}.json", b"[]".to_vec())
        .unwrap();
    package
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
}

// ---------------------------------------------------------------------------------------------
// reading the raw ZIP structure, independently of the zip crate
// ---------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Central {
    name: String,
    made_by: u16,
    needed: u16,
    flags: u16,
    method: u16,
    time: u16,
    date: u16,
    external_attributes: u32,
    extra_len: u16,
    comment_len: u16,
}

fn le16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn central_directory(bytes: &[u8]) -> Vec<Central> {
    let eocd = bytes
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .expect("end of central directory");
    let count = le16(bytes, eocd + 10) as usize;
    let mut at = le32(bytes, eocd + 16) as usize;
    let mut out = Vec::new();
    for _ in 0..count {
        assert_eq!(&bytes[at..at + 4], b"PK\x01\x02");
        let name_len = le16(bytes, at + 28) as usize;
        let extra_len = le16(bytes, at + 30);
        let comment_len = le16(bytes, at + 32);
        out.push(Central {
            name: String::from_utf8_lossy(&bytes[at + 46..at + 46 + name_len]).into_owned(),
            made_by: le16(bytes, at + 4),
            needed: le16(bytes, at + 6),
            flags: le16(bytes, at + 8),
            method: le16(bytes, at + 10),
            time: le16(bytes, at + 12),
            date: le16(bytes, at + 14),
            external_attributes: le32(bytes, at + 38),
            extra_len,
            comment_len,
        });
        at += 46 + name_len + extra_len as usize + comment_len as usize;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// the damaged packages
// ---------------------------------------------------------------------------------------------

fn raw_entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            std::io::copy(&mut file, &mut content).unwrap();
            (file.name().unwrap().into_owned(), content)
        })
        .collect()
}

fn build_zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::default());
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in entries {
        out.start_file(name.as_str(), options).unwrap();
        out.write_all(content).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// Rewrites a package after `change` edited its entries, leaving `integrity.json` as it was.
fn edit(bytes: &[u8], change: impl Fn(&mut Vec<(String, Vec<u8>)>)) -> Vec<u8> {
    let mut entries = raw_entries(bytes);
    change(&mut entries);
    build_zip(&entries)
}

/// Rewrites a package and recomputes `integrity.json` so that only the change itself is wrong.
fn edit_consistent(bytes: &[u8], change: impl Fn(&mut Vec<(String, Vec<u8>)>)) -> Vec<u8> {
    let mut entries = raw_entries(bytes);
    change(&mut entries);
    let files: BTreeMap<String, IntegrityEntry> = entries
        .iter()
        .filter(|(name, _)| name != INTEGRITY_ENTRY)
        .map(|(name, content)| {
            (
                name.clone(),
                IntegrityEntry {
                    sha256: sha256_hex(content),
                    size: content.len() as u64,
                },
            )
        })
        .collect();
    let integrity = Integrity {
        algorithm: "sha256".into(),
        files,
    };
    let text = serde_json::to_string_pretty(&integrity).unwrap();
    for (name, content) in entries.iter_mut() {
        if name == INTEGRITY_ENTRY {
            *content = text.clone().into_bytes();
        }
    }
    build_zip(&entries)
}

fn replace_bytes(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let mut out = bytes.to_vec();
    let mut at = 0;
    while let Some(p) = out[at..].windows(from.len()).position(|w| w == from) {
        out[at + p..at + p + from.len()].copy_from_slice(to);
        at += p + from.len();
    }
    out
}

fn set_entry(entries: &mut [(String, Vec<u8>)], name: &str, content: Vec<u8>) {
    entries.iter_mut().find(|(n, _)| n == name).unwrap().1 = content;
}

/// The damaged packages: file name, bytes. Every one is built from the typical package.
fn rejected_cases() -> Vec<(&'static str, Vec<u8>)> {
    let good = typical().to_bytes().unwrap();
    let mut cases: Vec<(&'static str, Vec<u8>)> = vec![
        ("not_a_zip", b"this is not a zip".to_vec()),
        ("empty_file", Vec::new()),
        ("truncated", good[..good.len() - 10].to_vec()),
        (
            "no_integrity",
            edit(&good, |e| e.retain(|(n, _)| n != INTEGRITY_ENTRY)),
        ),
        (
            "no_project",
            edit(&good, |e| e.retain(|(n, _)| n != PROJECT_ENTRY)),
        ),
        (
            "changed_data",
            edit(&good, |e| {
                set_entry(e, "layers/top.gbr", b"changed".to_vec())
            }),
        ),
        (
            "unlisted_entry",
            edit(&good, |e| {
                e.push(("vision/extra.json".into(), b"{}".to_vec()))
            }),
        ),
        (
            "listed_entry_missing",
            edit(&good, |e| e.retain(|(n, _)| n != "vision/fiducials.json")),
        ),
        (
            "unknown_algorithm",
            edit(&good, |e| {
                let text = String::from_utf8(
                    e.iter()
                        .find(|(n, _)| n == INTEGRITY_ENTRY)
                        .unwrap()
                        .1
                        .clone(),
                )
                .unwrap()
                .replacen("\"sha256\"", "\"md5\"", 1);
                set_entry(e, INTEGRITY_ENTRY, text.into_bytes());
            }),
        ),
        (
            "metadata_not_utf8",
            edit_consistent(&good, |e| {
                set_entry(e, PROJECT_ENTRY, vec![0xff, 0xfe, 0x7b])
            }),
        ),
        (
            "metadata_newer_major",
            edit_consistent(&good, |e| {
                let text = String::from_utf8(
                    e.iter()
                        .find(|(n, _)| n == PROJECT_ENTRY)
                        .unwrap()
                        .1
                        .clone(),
                )
                .unwrap()
                .replace("\"1.0.0\"", "\"2.0.0\"");
                set_entry(e, PROJECT_ENTRY, text.into_bytes());
            }),
        ),
        (
            "metadata_reference_missing",
            edit_consistent(&good, |e| e.retain(|(n, _)| n != "toolpaths/iso.nc")),
        ),
        (
            "hostile_name_parent",
            edit_consistent(&good, |e| e.push(("../../evil.txt".into(), b"x".to_vec()))),
        ),
        (
            "hostile_name_absolute",
            edit_consistent(&good, |e| e.push(("/etc/evil.txt".into(), b"x".to_vec()))),
        ),
        (
            "name_with_backslash",
            replace_bytes(&good, b"layers/top.gbr", b"layers\\top.gbr"),
        ),
        (
            "outside_data_folders",
            edit_consistent(&good, |e| {
                e.push(("notes/readme.txt".into(), b"x".to_vec()))
            }),
        ),
        ("duplicate_name", {
            let base = edit_consistent(&good, |e| {
                e.push(("layers/aaa.gbr".into(), b"a".to_vec()));
                e.push(("layers/bbb.gbr".into(), b"b".to_vec()));
            });
            replace_bytes(&base, b"layers/bbb.gbr", b"layers/aaa.gbr")
        }),
        (
            "trailing_bytes",
            [good.clone(), b"trailing garbage".to_vec()].concat(),
        ),
    ];
    // a name that differs only in letter case and a duplicate that the writer itself refuses
    cases.push((
        "case_only_duplicate",
        edit_consistent(&good, |e| {
            e.push(("layers/TOP.gbr".into(), b"G04 other*\nM02*\n".to_vec()))
        }),
    ));
    cases
}

fn verdict(bytes: &[u8]) -> String {
    match Package::read(Cursor::new(bytes.to_vec())) {
        Ok(package) => format!("accepted with {} entries", package.entry_names().count()),
        Err(Error::Io(_)) => "Io".into(),
        Err(Error::Json(_)) => "Json".into(),
        Err(Error::Zip(_)) => "Zip".into(),
        Err(Error::UnsupportedFormat { .. }) => "UnsupportedFormat".into(),
        Err(Error::MissingEntry(_)) => "MissingEntry".into(),
        Err(Error::Integrity { .. }) => "Integrity".into(),
        Err(Error::UnsafePath(_)) => "UnsafePath".into(),
        Err(Error::DuplicateEntry(_)) => "DuplicateEntry".into(),
        Err(Error::TooLarge(_)) => "TooLarge".into(),
        Err(Error::InvalidMetadata(_)) => "InvalidMetadata".into(),
    }
}

// ---------------------------------------------------------------------------------------------
// the recorded behaviour
// ---------------------------------------------------------------------------------------------

#[test]
fn entries_are_written_project_then_integrity_then_data_by_name() {
    let bytes = typical().to_bytes().unwrap();
    let names: Vec<_> = central_directory(&bytes)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(
        names,
        [
            "project.json",
            "integrity.json",
            "layers/top.gbr",
            "toolpaths/iso.nc",
            "vision/fiducials.json"
        ]
    );
    // the order in which entries were added does not matter
    let mut reversed = Package::new(read_metadata(PROJECT_JSON).unwrap());
    for name in [
        "layers/top.gbr",
        "toolpaths/iso.nc",
        "vision/fiducials.json",
    ] {
        let content = typical().entry(name).unwrap().to_vec();
        reversed.add_entry(name, content).unwrap();
    }
    assert_eq!(reversed.to_bytes().unwrap(), bytes);
}

#[test]
fn every_entry_is_deflated_with_the_fixed_dos_time_and_no_extras() {
    let bytes = typical().to_bytes().unwrap();
    let entries = central_directory(&bytes);
    assert_eq!(entries.len(), 5);
    for entry in &entries {
        assert_eq!(entry.method, 8, "{}: deflate", entry.name);
        // 1980-01-01 00:00:00 in DOS form
        assert_eq!((entry.time, entry.date), (0, 0x0021), "{}", entry.name);
        assert_eq!(entry.extra_len, 0, "{}: no extra field", entry.name);
        assert_eq!(entry.comment_len, 0, "{}: no comment", entry.name);
        assert_eq!(entry.flags, 0, "{}: no flags for ASCII names", entry.name);
    }
    let archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert!(archive.comment().is_empty());
}

#[test]
fn the_attributes_and_versions_of_the_entries_are_recorded() {
    let bytes = typical().to_bytes().unwrap();
    let first = &central_directory(&bytes)[0];
    // as characterised on the zip crate in Cargo.lock; a new zip release that changes these must be a conscious choice
    assert_eq!(
        (first.made_by, first.needed, first.external_attributes),
        (OBSERVED_MADE_BY, OBSERVED_NEEDED, OBSERVED_EXTERNAL)
    );
}

/// Version made by: 0x0314 is ZIP specification 2.0 on a Unix host, whatever the system that wrote the file.
const OBSERVED_MADE_BY: u16 = 0x0314;
/// Version needed to extract: 2.0 (deflate).
const OBSERVED_NEEDED: u16 = 20;
/// Unix mode 0o100644 (a plain file, rw-r--r--) in the high 16 bits.
const OBSERVED_EXTERNAL: u32 = 0o100644 << 16;

#[test]
fn small_packages_never_use_zip64() {
    for package in [minimal(), typical(), unicode_names()] {
        let bytes = package.to_bytes().unwrap();
        assert!(
            !bytes
                .windows(4)
                .any(|w| w == b"PK\x06\x06" || w == b"PK\x06\x07"),
            "no ZIP64 end records"
        );
        assert!(central_directory(&bytes).iter().all(|c| c.extra_len == 0));
    }
}

#[test]
fn names_beyond_ascii_set_the_utf8_flag_and_survive() {
    let bytes = unicode_names().to_bytes().unwrap();
    let entries = central_directory(&bytes);
    let flagged: Vec<_> = entries.iter().map(|c| (c.name.as_str(), c.flags)).collect();
    assert_eq!(flagged[0], ("project.json", 0));
    assert_eq!(flagged[1], ("integrity.json", 0));
    assert_eq!(flagged[2].1, 0x0800, "bit 11 for a non-ASCII name");
    assert_eq!(flagged[3].1, 0x0800);
    let back = Package::read(Cursor::new(bytes)).unwrap();
    assert!(back.entry("layers/ca\u{f1}\u{f3}n.gbr").is_some());
    assert!(back.entry("vision/\u{4e2d}\u{6587}.json").is_some());
}

#[test]
fn project_json_and_integrity_json_have_exactly_this_form() {
    let bytes = typical().to_bytes().unwrap();
    let entries = raw_entries(&bytes);
    let project = String::from_utf8(entries[0].1.clone()).unwrap();
    let integrity = String::from_utf8(entries[1].1.clone()).unwrap();
    assert!(project.starts_with("{\n  \"format_version\": \"1.0.0\",\n  \"id\": \""));
    assert!(!project.ends_with('\n'), "no trailing newline");
    assert!(integrity.starts_with("{\n  \"algorithm\": \"sha256\",\n  \"files\": {\n    \"layers/top.gbr\": {\n      \"sha256\": \""));
    assert!(!integrity.ends_with('\n'));
    // the integrity record lists project.json and every data entry in byte order of the names, with the hash before the size
    let parsed: Integrity = serde_json::from_str(&integrity).unwrap();
    let listed: Vec<_> = parsed.files.keys().map(String::as_str).collect();
    assert_eq!(
        listed,
        [
            "layers/top.gbr",
            "project.json",
            "toolpaths/iso.nc",
            "vision/fiducials.json"
        ]
    );
    assert!(
        integrity.contains("\"sha256\": \"")
            && integrity.find("\"sha256\"") < integrity.find("\"size\"")
    );
    assert_eq!(
        parsed.files["project.json"].sha256,
        sha256_hex(project.as_bytes())
    );
    // the metadata is written in the order of its fields, with the optional ones as null
    let order = [
        "format_version",
        "id",
        "name",
        "created",
        "modified",
        "units",
        "zero_piece",
        "layers",
        "tools",
        "toolpaths",
        "fiducials",
        "notes",
    ];
    let positions: Vec<_> = order
        .iter()
        .map(|key| project.find(&format!("\"{key}\"")).unwrap())
        .collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
    assert!(project.contains("\"notes\": null"));
}

#[test]
fn the_same_content_gives_the_same_bytes_in_any_run() {
    let first = typical().to_bytes().unwrap();
    for _ in 0..3 {
        assert_eq!(typical().to_bytes().unwrap(), first);
    }
}

#[test]
fn a_valid_package_survives_read_then_write_byte_for_byte() {
    for package in [minimal(), typical(), unicode_names()] {
        let bytes = package.to_bytes().unwrap();
        let back = Package::read(Cursor::new(bytes.clone())).unwrap();
        assert_eq!(back.to_bytes().unwrap(), bytes);
    }
}

// ---------------------------------------------------------------------------------------------
// the reference corpus
// ---------------------------------------------------------------------------------------------

fn valid_cases() -> Vec<(&'static str, Package)> {
    vec![
        ("minimal", minimal()),
        ("typical", typical()),
        ("unicode_names", unicode_names()),
    ]
}

#[test]
fn the_writer_reproduces_the_valid_reference_packages_byte_for_byte() {
    for (name, package) in valid_cases() {
        let path = golden_dir().join(format!("{name}.eh3dproj"));
        let golden = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(package.to_bytes().unwrap(), golden, "{name}");
    }
}

#[test]
fn the_reader_reads_the_valid_reference_packages_back() {
    for (name, package) in valid_cases() {
        let path = golden_dir().join(format!("{name}.eh3dproj"));
        let read = Package::read_from_path(&path).unwrap();
        assert_eq!(read, package, "{name}");
    }
}

#[test]
fn the_reader_gives_the_recorded_verdict_on_every_damaged_package() {
    for (name, expected) in RECORDED_VERDICTS {
        let path = golden_dir()
            .join("rejected")
            .join(format!("{name}.eh3dproj"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(&verdict(&bytes), expected, "{name}");
    }
    let on_disk = std::fs::read_dir(golden_dir().join("rejected"))
        .unwrap()
        .count();
    assert_eq!(
        on_disk,
        RECORDED_VERDICTS.len(),
        "a case without a verdict, or the reverse"
    );
}

/// The verdict of the reader of format 1.0.0 on each damaged package, as characterised.
///
/// Notes on the verdicts that are not obvious:
/// * `duplicate_name` ends in `Integrity` and not in `DuplicateEntry`: with the ZIP library of this
///   version a second entry of the same name is not reported as a duplicate. The new container must
///   state which behaviour it keeps.
/// * `trailing_bytes` (data after the end of the archive) and `case_only_duplicate` (two names that
///   differ only in letter case) are accepted today. The container draft rejects both for new formats.
const RECORDED_VERDICTS: &[(&str, &str)] = &[
    ("not_a_zip", "Zip"),
    ("empty_file", "Zip"),
    ("truncated", "Zip"),
    ("no_integrity", "MissingEntry"),
    ("no_project", "MissingEntry"),
    ("changed_data", "Integrity"),
    ("unlisted_entry", "Integrity"),
    ("listed_entry_missing", "MissingEntry"),
    ("unknown_algorithm", "Integrity"),
    ("metadata_not_utf8", "InvalidMetadata"),
    ("metadata_newer_major", "UnsupportedFormat"),
    ("metadata_reference_missing", "MissingEntry"),
    ("hostile_name_parent", "UnsafePath"),
    ("hostile_name_absolute", "UnsafePath"),
    ("name_with_backslash", "UnsafePath"),
    ("outside_data_folders", "UnsafePath"),
    ("duplicate_name", "Integrity"),
    ("trailing_bytes", "accepted with 3 entries"),
    ("case_only_duplicate", "accepted with 4 entries"),
];

#[test]
fn the_damaged_packages_on_disk_are_the_ones_the_tests_build() {
    for (name, bytes) in rejected_cases() {
        let path = golden_dir()
            .join("rejected")
            .join(format!("{name}.eh3dproj"));
        let golden = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(bytes, golden, "{name}");
    }
}

#[test]
fn the_manifest_of_the_corpus_matches_the_files() {
    let text = std::fs::read_to_string(golden_dir().join("SHA256SUMS")).unwrap();
    let mut listed = 0;
    for line in text.lines() {
        let (hash, path) = line.split_once("  ").unwrap();
        let bytes = std::fs::read(golden_dir().join(path)).unwrap();
        assert_eq!(sha256_hex(&bytes), hash, "{path}");
        listed += 1;
    }
    assert_eq!(listed, valid_cases().len() + RECORDED_VERDICTS.len());
}

#[test]
fn a_package_with_too_many_entries_is_refused_before_reading_them() {
    let mut entries: Vec<(String, Vec<u8>)> = vec![
        (PROJECT_ENTRY.into(), Vec::new()),
        (INTEGRITY_ENTRY.into(), Vec::new()),
    ];
    for n in 0..10_000 {
        entries.push((format!("layers/{n}"), Vec::new()));
    }
    assert_eq!(verdict(&build_zip(&entries)), "TooLarge");
}

/// Writes the reference corpus. Run it only after a deliberate change of the container.
#[test]
#[ignore]
fn regenerate_golden() {
    let dir = golden_dir();
    std::fs::create_dir_all(dir.join("rejected")).unwrap();
    let mut sums = Vec::new();
    for (name, package) in valid_cases() {
        let bytes = package.to_bytes().unwrap();
        std::fs::write(dir.join(format!("{name}.eh3dproj")), &bytes).unwrap();
        sums.push(format!("{}  {name}.eh3dproj", sha256_hex(&bytes)));
    }
    let mut verdicts = Vec::new();
    for (name, bytes) in rejected_cases() {
        std::fs::write(
            dir.join("rejected").join(format!("{name}.eh3dproj")),
            &bytes,
        )
        .unwrap();
        sums.push(format!("{}  rejected/{name}.eh3dproj", sha256_hex(&bytes)));
        verdicts.push(format!("    (\"{name}\", \"{}\"),", verdict(&bytes)));
    }
    std::fs::write(dir.join("SHA256SUMS"), sums.join("\n") + "\n").unwrap();
    println!("RECORDED_VERDICTS\n{}", verdicts.join("\n"));
    let first = central_directory(&typical().to_bytes().unwrap()).remove(0);
    println!(
        "OBSERVED made_by={} needed={} external={}",
        first.made_by, first.needed, first.external_attributes
    );
}
