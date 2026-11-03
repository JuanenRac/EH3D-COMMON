//! The generic reader and writer: every rule of the policy table, the determinism of the writer,
//! the budget, and the reference packages of `.eh3dproj` 1.0.0.

use std::io::{Cursor, Read};
use std::path::PathBuf;

use eh3d_container::{
    sha256_hex, AtomicOptions, Container, ContainerError, ContainerSpec, ContainerWriter,
    Integrity, Method, PkgCode, Policy,
};

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// One entry of a hand-made archive; every field can be bent.
#[derive(Clone)]
struct Raw {
    name: Vec<u8>,
    data: Vec<u8>,
    flags: u16,
    method: u16,
    made_by: u16,
    external: u32,
    extra: Vec<u8>,
    crc: Option<u32>,
}

impl Raw {
    fn file(name: &str, data: &[u8]) -> Raw {
        Raw {
            name: name.as_bytes().to_vec(),
            data: data.to_vec(),
            flags: if name.is_ascii() { 0 } else { 0x0800 },
            method: 0,
            made_by: 0x031E,
            external: 0o100_644 << 16,
            extra: Vec::new(),
            crc: None,
        }
    }
}

fn raw_zip(entries: &[Raw]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for e in entries {
        let offset = out.len() as u32;
        let crc = e.crc.unwrap_or_else(|| crc32(&e.data));
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&e.flags.to_le_bytes());
        out.extend_from_slice(&e.method.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0x21, 0]); // 1980-01-01
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&e.name);
        out.extend_from_slice(&e.data);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&e.made_by.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&e.flags.to_le_bytes());
        central.extend_from_slice(&e.method.to_le_bytes());
        central.extend_from_slice(&[0, 0, 0x21, 0]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        central.extend_from_slice(&(e.extra.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&e.external.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(&e.name);
        central.extend_from_slice(&e.extra);
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// The spec used with hand-made archives: stored entries are allowed so no deflater is needed.
fn spec() -> ContainerSpec {
    let mut spec = ContainerSpec::phoenixproj(0, 1);
    spec.methods = vec![Method::Stored, Method::Deflate];
    spec
}

const PROJECT: &[u8] = br#"{"format_version":"0.1.0","name":"t"}"#;

/// The raw entries of a valid package for `spec()`, with a correct integrity record.
fn valid_entries(data: &[(&str, &[u8])]) -> Vec<Raw> {
    let mut all: Vec<(&str, &[u8])> = vec![("project.json", PROJECT)];
    all.extend_from_slice(data);
    let record = Integrity::for_entries(all.iter().map(|(n, b)| (*n, *b)));
    let mut raws: Vec<Raw> = all.iter().map(|(n, b)| Raw::file(n, b)).collect();
    raws.push(Raw::file(
        "integrity.json",
        &serde_json::to_vec(&record).unwrap(),
    ));
    raws
}

fn open(
    bytes: Vec<u8>,
    spec: &ContainerSpec,
) -> Result<Container<Cursor<Vec<u8>>>, ContainerError> {
    Container::open(Cursor::new(bytes), spec)
}

fn code_of<T>(result: Result<T, ContainerError>) -> PkgCode {
    match result {
        Ok(_) => panic!("expected a rejection"),
        Err(e) => e.code().unwrap_or_else(|| panic!("no code: {e}")),
    }
}

fn checked(
    bytes: Vec<u8>,
    spec: &ContainerSpec,
) -> Result<Container<Cursor<Vec<u8>>>, ContainerError> {
    Container::open_checked(Cursor::new(bytes), spec, version_of)
}

fn version_of(project: &[u8]) -> Option<(u64, u64)> {
    let value: serde_json::Value = serde_json::from_slice(project).ok()?;
    let text = value.get("format_version")?.as_str()?;
    let mut parts = text.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

#[test]
fn a_valid_package_opens_checks_and_reads() {
    let spec = spec();
    let bytes = raw_zip(&valid_entries(&[
        ("design/a.json", b"{}"),
        ("x-notes/n.txt", b"hi"),
    ]));
    let mut c = checked(bytes, &spec).unwrap();
    assert_eq!(c.entries().count(), 4);
    assert_eq!(c.read_limited("design/a.json", 1024).unwrap(), b"{}");
    let mut text = String::new();
    c.read_stream("x-notes/n.txt")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "hi");
    assert_eq!(c.content_hash().unwrap().len(), 64);
}

#[test]
fn rule_001_refuses_bad_names() {
    let spec = spec();
    let cases: Vec<(&str, Raw)> = vec![
        ("backslash", Raw::file("design\\a.json", b"x")),
        ("colon", Raw::file("design/a:b", b"x")),
        ("parent", Raw::file("design/../a", b"x")),
        ("absolute", Raw::file("/design/a", b"x")),
        ("empty part", Raw::file("design//a", b"x")),
        ("control", Raw::file("design/a\u{1}b", b"x")),
        ("not NFC", Raw::file("design/e\u{301}.json", b"x")),
        ("no utf8 flag", {
            let mut r = Raw::file("design/\u{e9}.json", b"x");
            r.flags = 0;
            r
        }),
        ("not utf8", {
            let mut r = Raw::file("design/a", b"x");
            r.name = vec![b'd', b'/', 0xFF, 0xFE];
            r
        }),
        (
            "too long",
            Raw::file(&format!("design/{}", "a".repeat(300)), b"x"),
        ),
    ];
    for (label, raw) in cases {
        assert_eq!(
            code_of(open(raw_zip(&[raw]), &spec)),
            PkgCode::InvalidName,
            "{label}"
        );
    }
}

#[test]
fn rule_002_refuses_names_that_collide_on_windows() {
    let spec = spec();
    let bytes = raw_zip(&[
        Raw::file("design/A.json", b"1"),
        Raw::file("design/a.json", b"2"),
    ]);
    assert_eq!(code_of(open(bytes, &spec)), PkgCode::DuplicateName);
    let same = raw_zip(&[
        Raw::file("design/a.json", b"1"),
        Raw::file("design/a.json", b"2"),
    ]);
    assert_eq!(code_of(open(same, &spec)), PkgCode::DuplicateName);
}

#[test]
fn the_legacy_policy_keeps_the_old_tolerance_for_case() {
    let mut legacy = ContainerSpec::eh3dproj_v1();
    legacy.methods = vec![Method::Stored, Method::Deflate];
    let bytes = raw_zip(&[
        Raw::file("layers/A.gbr", b"1"),
        Raw::file("layers/a.gbr", b"2"),
    ]);
    assert!(open(bytes, &legacy).is_ok());
    let nfd = raw_zip(&[Raw::file("layers/e\u{301}.gbr", b"1")]);
    assert!(open(nfd, &legacy).is_ok());
}

#[test]
fn rule_003_refuses_links_and_special_entries() {
    let spec = spec();
    for kind in [0o120_000u32, 0o060_000, 0o020_000, 0o010_000, 0o140_000] {
        let mut raw = Raw::file("design/l", b"target");
        raw.external = (kind | 0o777) << 16;
        assert_eq!(
            code_of(open(raw_zip(&[raw]), &spec)),
            PkgCode::SpecialEntry,
            "{kind:o}"
        );
    }
}

#[test]
fn rule_004_refuses_directory_entries() {
    let spec = spec();
    assert_eq!(
        code_of(open(raw_zip(&[Raw::file("design/", b"")]), &spec)),
        PkgCode::DirectoryEntry
    );
    let mut dos = Raw::file("design/sub", b"");
    dos.made_by = 0x0014;
    dos.external = 0x10;
    assert_eq!(
        code_of(open(raw_zip(&[dos]), &spec)),
        PkgCode::DirectoryEntry
    );
}

#[test]
fn rule_005_refuses_zip64_when_it_is_not_allowed() {
    let spec = spec();
    let mut bytes = raw_zip(&valid_entries(&[]));
    let eocd = bytes.len() - 22;
    bytes[eocd + 10] = 0xFF;
    bytes[eocd + 11] = 0xFF;
    assert_eq!(code_of(open(bytes, &spec)), PkgCode::Zip64);
    let mut with_extra = Raw::file("design/a", b"x");
    with_extra.extra = vec![1, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(code_of(open(raw_zip(&[with_extra]), &spec)), PkgCode::Zip64);
}

#[test]
fn rule_006_refuses_encryption_and_methods_outside_the_spec() {
    let spec = spec();
    let mut encrypted = Raw::file("design/a", b"x");
    encrypted.flags = 0x0001;
    assert_eq!(code_of(open(raw_zip(&[encrypted]), &spec)), PkgCode::Method);
    let mut strong = Raw::file("design/a", b"x");
    strong.flags = 0x0040;
    assert_eq!(code_of(open(raw_zip(&[strong]), &spec)), PkgCode::Method);
    let mut bzip = Raw::file("design/a", b"x");
    bzip.method = 12;
    assert_eq!(code_of(open(raw_zip(&[bzip]), &spec)), PkgCode::Method);
    let deflate_only = ContainerSpec::phoenixproj(0, 1);
    assert_eq!(
        code_of(open(raw_zip(&[Raw::file("design/a", b"x")]), &deflate_only)),
        PkgCode::Method,
        "stored is not allowed by the preset"
    );
}

#[test]
fn rule_007_refuses_damage() {
    let spec = spec();
    let good = raw_zip(&valid_entries(&[("design/a.json", b"{}")]));
    // truncated
    assert_eq!(
        code_of(open(good[..good.len() - 30].to_vec(), &spec)),
        PkgCode::DamagedArchive
    );
    assert_eq!(code_of(open(vec![1, 2, 3], &spec)), PkgCode::DamagedArchive);
    assert_eq!(code_of(open(Vec::new(), &spec)), PkgCode::DamagedArchive);
    // data after the end
    let mut trailing = good.clone();
    trailing.extend_from_slice(b"extra bytes after the archive");
    assert_eq!(code_of(open(trailing, &spec)), PkgCode::DamagedArchive);
    // a wrong CRC is found when the entry is read
    let mut wrong = Raw::file("design/a.json", b"{}");
    wrong.crc = Some(1);
    let mut c = open(raw_zip(&[wrong]), &spec).unwrap();
    assert_eq!(
        code_of(c.read_limited("design/a.json", 100)),
        PkgCode::DamagedArchive
    );
}

#[test]
fn rule_008_refuses_other_flags_but_allows_the_descriptor_bit() {
    let spec = spec();
    for flags in [0x0002u16, 0x0004, 0x0080, 0x2000] {
        let mut raw = Raw::file("design/a", b"x");
        raw.flags = flags;
        assert_eq!(
            code_of(open(raw_zip(&[raw]), &spec)),
            PkgCode::Flags,
            "{flags:#x}"
        );
    }
    let mut descriptor = Raw::file("design/a", b"x");
    descriptor.flags = 0x0008;
    assert!(open(raw_zip(&[descriptor]), &spec).is_ok());
}

#[test]
fn rule_009_refuses_entries_outside_the_roots_and_missing_required_entries() {
    let spec = spec();
    let outside = raw_zip(&valid_entries(&[("secret/a", b"x")]));
    assert_eq!(code_of(checked(outside, &spec)), PkgCode::OutsideRoots);
    let mut no_project = valid_entries(&[]);
    no_project.remove(0);
    let c = open(raw_zip(&no_project), &spec).unwrap();
    assert_eq!(code_of(c.check_layout()), PkgCode::OutsideRoots);
    // x-<name> folders are preserved, a bare file called x-a is not a folder
    assert!(checked(raw_zip(&valid_entries(&[("x-a/b", b"1")])), &spec).is_ok());
    assert_eq!(
        code_of(checked(raw_zip(&valid_entries(&[("x-a", b"1")])), &spec)),
        PkgCode::OutsideRoots
    );
}

#[test]
fn rule_010_refuses_a_missing_incomplete_or_different_integrity_record() {
    let spec = spec();
    let mut missing = valid_entries(&[("design/a.json", b"{}")]);
    missing.pop();
    assert_eq!(
        code_of(checked(raw_zip(&missing), &spec)),
        PkgCode::Integrity
    );

    let mut unlisted = valid_entries(&[]);
    unlisted.insert(1, Raw::file("design/extra.json", b"{}"));
    assert_eq!(
        code_of(checked(raw_zip(&unlisted), &spec)),
        PkgCode::Integrity
    );

    let mut changed = valid_entries(&[("design/a.json", b"{}")]);
    changed[1] = Raw::file("design/a.json", b"[]");
    assert_eq!(
        code_of(checked(raw_zip(&changed), &spec)),
        PkgCode::Integrity
    );

    // same size, other bytes
    let mut same_size = valid_entries(&[("design/a.json", b"{}")]);
    same_size[1] = Raw::file("design/a.json", b"[]");
    let ok_size = same_size;
    assert_eq!(
        code_of(checked(raw_zip(&ok_size), &spec)),
        PkgCode::Integrity
    );

    let mut bad_record = valid_entries(&[]);
    *bad_record.last_mut().unwrap() = Raw::file("integrity.json", b"not json");
    assert_eq!(
        code_of(checked(raw_zip(&bad_record), &spec)),
        PkgCode::Integrity
    );

    let mut listed_missing = valid_entries(&[("design/a.json", b"{}")]);
    listed_missing.remove(1);
    assert_eq!(
        code_of(checked(raw_zip(&listed_missing), &spec)),
        PkgCode::Integrity
    );
    let _ = sha256_hex(b"");
}

#[test]
fn rule_011_refuses_what_exceeds_the_limits_before_reading_it() {
    let mut small = spec();
    small.limits.max_entries = 3;
    let many = raw_zip(&valid_entries(&[("design/a", b"1"), ("design/b", b"2")]));
    assert_eq!(code_of(open(many, &small)), PkgCode::Limit);

    let mut tiny = spec();
    tiny.limits.default_max_entry_bytes = 10;
    let big = raw_zip(&[Raw::file("design/a", &[7u8; 11])]);
    assert_eq!(code_of(open(big, &tiny)), PkgCode::Limit);

    let mut total = spec();
    total.limits.max_total_bytes = 15;
    let two = raw_zip(&[
        Raw::file("design/a", &[1u8; 10]),
        Raw::file("design/b", &[1u8; 10]),
    ]);
    assert_eq!(code_of(open(two, &total)), PkgCode::Limit);

    // a root can have its own limit
    let mut root = spec();
    root.limits
        .per_root_max_entry_bytes
        .insert("results/".into(), 4);
    assert_eq!(
        code_of(open(raw_zip(&[Raw::file("results/r", &[0u8; 5])]), &root)),
        PkgCode::Limit
    );
    assert!(open(raw_zip(&[Raw::file("design/r", &[0u8; 5])]), &root).is_ok());

    // a read that keeps bytes in memory has its own ceiling
    let mut c = open(raw_zip(&[Raw::file("design/a", &[1u8; 100])]), &spec()).unwrap();
    assert_eq!(code_of(c.read_limited("design/a", 50)), PkgCode::Limit);
    assert_eq!(c.read_limited("design/a", 100).unwrap().len(), 100);
}

#[test]
fn rule_012_refuses_what_exceeds_the_memory_or_time_budget() {
    let mut low_memory = spec();
    low_memory.budget.max_memory_bytes = 50;
    let mut c = open(raw_zip(&[Raw::file("design/a", &[1u8; 100])]), &low_memory).unwrap();
    assert_eq!(code_of(c.read_limited("design/a", 1000)), PkgCode::Budget);

    // a package that is big once decompressed and a budget of one millisecond
    let dir = std::env::temp_dir().join(format!("eh3d_budget_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut slow = ContainerSpec::phoenixproj(0, 1);
    slow.limits.default_max_entry_bytes = 1 << 30;
    let mut writer = ContainerWriter::new(&slow, &dir);
    writer.add("project.json", PROJECT).unwrap();
    writer
        .add_stream(
            "design/zeros.bin",
            std::io::repeat(0).take(16 * 1024 * 1024),
        )
        .unwrap();
    let path = dir.join("big.phoenixproj");
    writer
        .write_to_path(&path, &AtomicOptions::default())
        .unwrap();
    slow.budget.max_millis = 1;
    let mut c = Container::open_path(&path, &slow).unwrap();
    let mut stream = c.read_stream("design/zeros.bin").unwrap();
    let mut sink = vec![0u8; 64 * 1024];
    let error = loop {
        match stream.read(&mut sink) {
            Ok(0) => panic!("the budget did not stop the read"),
            Ok(_) => {}
            Err(e) => break e,
        }
    };
    assert_eq!(
        eh3d_container::violation_in(&error).unwrap().code,
        PkgCode::Budget
    );
    drop(stream);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rule_013_checks_the_version_before_the_roots() {
    let spec = spec(); // knows 0.1 and refuses newer minors
    let newer = |version: &str| {
        let project = format!(r#"{{"format_version":"{version}"}}"#);
        let mut raws = valid_entries(&[("secret/unknown-root", b"x")]);
        raws[0] = Raw::file("project.json", project.as_bytes());
        raw_zip(&raws)
    };
    for version in ["0.2.0", "1.0.0"] {
        let error = checked(newer(version), &spec).err().unwrap();
        let violation = match error {
            ContainerError::Rejected(v) => v,
            other => panic!("{other}"),
        };
        assert_eq!(violation.code, PkgCode::NewerVersion, "{version}");
        assert!(violation.detail.contains("newer"), "{}", violation.detail);
    }
    // an older or equal version gets to the root check
    assert_eq!(
        code_of(checked(newer("0.1.0"), &spec)),
        PkgCode::OutsideRoots
    );
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_container_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn leftovers(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".eh3dtmp"))
        .collect()
}

fn write_phoenix(order: &[usize], audit: &str, dir: &std::path::Path, name: &str) -> Vec<u8> {
    let spec = ContainerSpec::phoenixproj(0, 1);
    let parts: [(&str, &[u8]); 4] = [
        ("design/b.json", b"{\"b\":2}"),
        ("design/a.json", b"{\"a\":1}"),
        ("x-note/n.txt", b"hello"),
        ("project.json", PROJECT),
    ];
    let mut writer = ContainerWriter::new(&spec, dir);
    for &i in order {
        writer.add(parts[i].0, parts[i].1).unwrap();
    }
    writer.add("audit/run.json", audit.as_bytes()).unwrap();
    let path = dir.join(name);
    writer
        .write_to_path(&path, &AtomicOptions::default())
        .unwrap();
    std::fs::read(path).unwrap()
}

#[test]
fn the_writer_gives_the_same_bytes_in_any_insertion_order() {
    let dir = temp_dir("order");
    let one = write_phoenix(&[0, 1, 2, 3], "run-1", &dir, "one.phoenixproj");
    let two = write_phoenix(&[3, 2, 1, 0], "run-1", &dir, "two.phoenixproj");
    assert_eq!(one, two);
    assert!(leftovers(&dir).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_writer_orders_sorted_data_then_project_then_integrity() {
    let dir = temp_dir("layout");
    let bytes = write_phoenix(&[0, 1, 2, 3], "r", &dir, "p.phoenixproj");
    let c = Container::open(Cursor::new(bytes), &ContainerSpec::phoenixproj(0, 1)).unwrap();
    let names: Vec<&str> = c.entries().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "audit/run.json",
            "design/a.json",
            "design/b.json",
            "x-note/n.txt",
            "project.json",
            "integrity.json"
        ]
    );
    // every entry is deflated, with the fixed DOS time and flags 0 for ASCII names
    assert!(c.entries().all(|e| e.method == Method::Deflate));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn what_the_writer_writes_the_reader_accepts_and_the_content_hash_ignores_audit() {
    let dir = temp_dir("hash");
    let spec = ContainerSpec::phoenixproj(0, 1);
    let a = write_phoenix(&[0, 1, 2, 3], "first run", &dir, "a.phoenixproj");
    let b = write_phoenix(&[0, 1, 2, 3], "second run, longer", &dir, "b.phoenixproj");
    assert_ne!(a, b, "the audit entry differs, so the ZIP bytes differ");
    let mut ca = checked(a, &spec).unwrap();
    let mut cb = checked(b, &spec).unwrap();
    assert_eq!(ca.content_hash().unwrap(), cb.content_hash().unwrap());
    assert_eq!(ca.read_limited("design/a.json", 100).unwrap(), b"{\"a\":1}");
    // the record is canonical JSON: sorted keys, no spaces, one final line feed
    let record = ca.read_limited("integrity.json", 1 << 20).unwrap();
    assert!(record.starts_with(b"{\"algorithm\":\"sha256\",\"files\":{"));
    assert_eq!(record.last(), Some(&b'\n'));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_writer_refuses_what_the_reader_would_refuse() {
    let dir = temp_dir("refuse");
    let spec = ContainerSpec::phoenixproj(0, 1);
    let mut w = ContainerWriter::new(&spec, &dir);
    let refused = |r: Result<(), ContainerError>| r.err().and_then(|e| e.code());
    assert_eq!(
        refused(w.add("design/e\u{301}", b"x")),
        Some(PkgCode::InvalidName)
    );
    assert_eq!(
        refused(w.add("design\\a", b"x")),
        Some(PkgCode::InvalidName)
    );
    assert_eq!(
        refused(w.add("outside/a", b"x")),
        Some(PkgCode::OutsideRoots)
    );
    assert_eq!(
        refused(w.add("integrity.json", b"x")),
        Some(PkgCode::OutsideRoots)
    );
    w.add("design/A.json", b"x").unwrap();
    assert_eq!(
        refused(w.add("design/a.json", b"y")),
        Some(PkgCode::DuplicateName)
    );
    let path = dir.join("p.phoenixproj");
    assert_eq!(
        w.write_to_path(&path, &AtomicOptions::default())
            .err()
            .and_then(|e| e.code()),
        Some(PkgCode::OutsideRoots),
        "project.json is required"
    );
    assert!(!path.exists());

    let mut limited = spec.clone();
    limited.limits.default_max_entry_bytes = 4;
    let mut w = ContainerWriter::new(&limited, &dir);
    assert_eq!(refused(w.add("design/big", b"12345")), Some(PkgCode::Limit));
    drop(w);
    assert!(
        leftovers(&dir).is_empty(),
        "spool files are removed on every path"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_stream_that_fails_adds_nothing_and_leaves_no_spool_file() {
    let dir = temp_dir("keep");
    let path = dir.join("p.phoenixproj");
    let spec = ContainerSpec::phoenixproj(0, 1);
    let mut w = ContainerWriter::new(&spec, &dir);
    w.add("project.json", PROJECT).unwrap();
    struct Failing(usize);
    impl Read for Failing {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0 == 0 {
                return Err(std::io::Error::other("disk gone"));
            }
            self.0 -= 1;
            buf[0] = 1;
            Ok(1)
        }
    }
    assert!(w.add_stream("design/a", Failing(10)).is_err());
    assert_eq!(
        leftovers(&dir).len(),
        1,
        "only the spool of project.json is left, until the write"
    );
    w.write_to_path(&path, &AtomicOptions::default()).unwrap();
    let mut c = checked(std::fs::read(&path).unwrap(), &spec).unwrap();
    assert!(c.entry("design/a").is_none());
    assert!(c.read_limited("project.json", 1000).is_ok());
    assert!(leftovers(&dir).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

#[test]
fn the_legacy_writer_profile_reproduces_the_reference_packages_byte_for_byte() {
    let spec = ContainerSpec::eh3dproj_v1();
    let dir = temp_dir("golden");
    for name in ["minimal", "typical", "unicode_names"] {
        let golden_path = golden_dir().join(format!("{name}.eh3dproj"));
        let golden = std::fs::read(&golden_path).unwrap();
        let mut reader = Container::open(Cursor::new(golden.clone()), &spec).unwrap();
        reader.check_layout().unwrap();
        reader.verify_integrity().unwrap();
        // rewrite every entry except integrity.json, in the reverse order
        let names: Vec<String> = reader
            .entries()
            .map(|e| e.name.clone())
            .filter(|n| n != "integrity.json")
            .collect();
        let mut writer = ContainerWriter::new(&spec, &dir);
        for entry in names.iter().rev() {
            let bytes = reader.read_limited(entry, 1 << 20).unwrap();
            writer.add(entry, &bytes).unwrap();
        }
        let out = dir.join(format!("{name}.eh3dproj"));
        writer
            .write_to_path(&out, &AtomicOptions::default())
            .unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), golden, "{name}");
    }
    assert!(leftovers(&dir).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_new_reader_reads_the_reference_packages_with_the_legacy_preset() {
    let spec = ContainerSpec::eh3dproj_v1();
    for name in ["minimal", "typical", "unicode_names"] {
        let bytes = std::fs::read(golden_dir().join(format!("{name}.eh3dproj"))).unwrap();
        let mut c = checked(bytes, &spec).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(c.content_hash().unwrap().len(), 64);
    }
}

#[test]
fn the_presets_are_distinct_and_the_budget_and_limits_survive_a_config_file() {
    let phoenix = ContainerSpec::phoenixproj(0, 1);
    let v11 = ContainerSpec::eh3dproj_v1_1();
    let v10 = ContainerSpec::eh3dproj_v1();
    assert_eq!(phoenix.policy, Policy::Strict);
    assert_eq!(v10.policy, Policy::Legacy);
    assert!(v11.is_data_entry("assembly/assembly.json"));
    assert!(!v10.is_data_entry("assembly/assembly.json"));
    assert!(v11.is_data_entry("heightmaps/a.json"));
    assert!(!v10.is_data_entry("heightmaps/a.json"));
    assert!(phoenix.is_extension_entry("x-vendor/data"));
    assert!(v11.supports_version(1, 1) && !v11.supports_version(1, 2));
    assert!(
        v10.supports_version(1, 5),
        "1.0.0 readers accept newer minors"
    );
    let json = serde_json::to_string(&phoenix.limits).unwrap();
    let back: eh3d_container::Limits = serde_json::from_str(&json).unwrap();
    assert_eq!(back, phoenix.limits);
    let budget: eh3d_container::Budget =
        serde_json::from_str(r#"{"max_millis":5,"max_memory_bytes":6}"#).unwrap();
    assert_eq!((budget.max_millis, budget.max_memory_bytes), (5, 6));
}

#[test]
fn the_legacy_name_rule_is_the_same_as_the_old_function() {
    for name in [
        "layers/a",
        "",
        "/a",
        "a/",
        "a//b",
        "../a",
        "a\\b",
        "a:b",
        "a\u{1}b",
        "Layers/A",
        "toolpaths/./x",
        "layers/e\u{301}",
    ] {
        assert_eq!(
            eh3d_container::check_name(Policy::Legacy, name, 240).is_ok(),
            eh3d_container::check_entry_name(name).is_ok(),
            "{name:?}"
        );
    }
}

#[test]
fn the_codes_of_the_registry_in_the_documentation_match_when_it_is_next_to_this_checkout() {
    let registry =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../EH3D-DOCS/registry/EH3D-PKG.md");
    let Ok(text) = std::fs::read_to_string(registry) else {
        return; // CI checks out this repository alone
    };
    for code in PkgCode::ALL {
        let row = text
            .lines()
            .find(|l| l.starts_with(&format!("| {} |", code.code())))
            .unwrap_or_else(|| panic!("{} is not in the registry", code.code()));
        assert!(
            row.contains(code.title()),
            "{}: title differs: {row}",
            code.code()
        );
    }
    let listed = text
        .lines()
        .filter(|l| l.starts_with("| EH3D-PKG-"))
        .count();
    assert_eq!(listed, PkgCode::ALL.len());
}
