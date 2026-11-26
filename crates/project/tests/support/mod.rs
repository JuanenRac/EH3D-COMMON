//! Shared helpers of the tests: a synthetic project (made up for the tests, with no data of
//! anyone), a way to build files with chosen faults, and a toy signature scheme.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use eh3d_container::{canonical_file_bytes, sha256_hex, AtomicOptions, ContainerWriter};
use eh3d_project::meta::{
    ConversionReport, Converter, Counted, DocIndex, DocKind, Document, Metadata, SourceFile,
};
use eh3d_project::records::from_hex;
use eh3d_project::spec::eh3d;
use eh3d_project::{
    manufacturing_update, BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Signer,
    Verifier, VerifyOutcome, Writer,
};
use serde_json::{json, Value};

pub const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden");

pub fn canon(value: &Value) -> Vec<u8> {
    canonical_file_bytes(value).unwrap()
}

pub fn serialised<T: serde::Serialize>(value: &T) -> Vec<u8> {
    canon(&serde_json::to_value(value).unwrap())
}

pub fn text(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

pub fn options_at(at: &str) -> SaveOptions<'static> {
    SaveOptions {
        modified: Some(at.to_owned()),
        ..SaveOptions::default()
    }
}

/// An empty project in `dir`.
pub fn new_project(dir: &Path) -> Project {
    Project::create(
        &dir.join("synthetic.eh3d"),
        NewProject {
            id: "00000000-0000-4000-8000-000000000001".into(),
            name: "Synthetic test project".into(),
            units: "millimetres".into(),
            created: Some("2000-01-01T00:00:00Z".into()),
        },
        &OpenOptions::default(),
        &AtomicOptions::default(),
    )
    .unwrap()
}

/// Saves a branch as the program `app` (which owns the branch of the same name).
pub fn save(project: Project, app: &str, update: BranchUpdate, at: &str) -> Project {
    project
        .save_branch(&Writer::for_app(app), update, &options_at(at))
        .unwrap()
}

pub fn pcb_update(outline: &str) -> BranchUpdate {
    let report = ConversionReport {
        schema: 1,
        source: SourceFile {
            format: "example-board".into(),
            name: "board.example".into(),
            sha256: sha256_hex(b"original board file"),
            size: b"original board file".len() as u64,
        },
        converter: Converter {
            name: "synthetic".into(),
            version: "1".into(),
        },
        original: Some("origin/board.example".into()),
        read: vec![Counted {
            item: "outline".into(),
            count: 1,
            reason: None,
        }],
        not_read: vec![Counted {
            item: "notes".into(),
            count: 2,
            reason: Some("not supported".into()),
        }],
        export_losses: vec![],
        extra: BTreeMap::new(),
    };
    BranchUpdate::new("pcb", "1.0", "0.1.0")
        .with_entry(
            "board.json",
            &canon(&json!({"layers": 2, "name": "synthetic board"})),
        )
        .with_entry("outline.json", &canon(&json!({"outline": outline})))
        .with_entry("origin/board.example", b"original board file")
        .with_entry("origin/report.json", &serialised(&report))
}

pub fn meta_update() -> BranchUpdate {
    let meta = Metadata {
        title: text(&[
            ("en", "Synthetic"),
            ("es", "Sintetico"),
            ("pt-PT", "Sintetico"),
        ]),
        license: Some("GPL-3.0-or-later".into()),
        tags: vec!["test".into()],
        ..Metadata::default()
    };
    BranchUpdate::new("common", "1.0", "0.1.0")
        .with_entry("meta/meta.json", &serialised(&meta))
        .with_entry("passport.json", &canon(&json!({"made_for": "tests"})))
}

pub fn docs_update(app: &str) -> BranchUpdate {
    let doc = |path: &str, kind, lang: &str, of: Option<&str>| Document {
        path: path.into(),
        kind,
        title: text(&[("en", "Guide")]),
        lang: Some(lang.into()),
        translation_of: of.map(Into::into),
        extra: BTreeMap::new(),
    };
    let index = DocIndex {
        schema: 1,
        documents: vec![
            doc("guide.en.md", DocKind::Markdown, "en", None),
            doc("guide.es.md", DocKind::Markdown, "es", Some("guide.en.md")),
        ],
        extra: BTreeMap::new(),
    };
    BranchUpdate::new(&format!("docs/{app}"), "1.0", "0.1.0")
        .with_entry("index.json", &serialised(&index))
        .with_entry("guide.en.md", b"# Guide\n")
        .with_entry("guide.es.md", b"# Guia\n")
}

/// The whole synthetic project: a board, a case derived from it, a print, a logo, a metal part,
/// a manufacturing package, the documentation of the board and the metadata.
pub fn synthetic(dir: &Path) -> Project {
    let mut p = new_project(dir);
    p = save(p, "any", meta_update(), "2000-01-01T00:00:01Z");
    p = save(p, "pcb", pcb_update("rect 40x30"), "2000-01-01T00:00:02Z");
    let dep = p
        .dependency_on("pcb", "outline2d", 1, vec!["case.json".into()])
        .unwrap();
    let mut case = BranchUpdate::new("cad", "1.0", "0.1.0")
        .with_entry("case.json", &canon(&json!({"inner": "rect 40x30"})));
    case.depends.push(dep);
    p = save(p, "cad", case, "2000-01-01T00:00:03Z");
    let dep = p
        .dependency_on("cad", "mesh3d", 1, vec!["print.json".into()])
        .unwrap();
    let mut print = BranchUpdate::new("3dp", "1.0", "0.1.0")
        .with_entry("print.json", &canon(&json!({"layer_height": 0.2})));
    print.depends.push(dep);
    p = save(p, "3dp", print, "2000-01-01T00:00:04Z");
    p = save(
        p,
        "laser",
        BranchUpdate::new("laser", "1.0", "0.1.0")
            .with_entry("logo.json", &canon(&json!({"power": 30}))),
        "2000-01-01T00:00:05Z",
    );
    p = save(
        p,
        "cnc",
        BranchUpdate::new("cnc", "1.0", "0.1.0")
            .with_entry("part.json", &canon(&json!({"material": "aluminium"}))),
        "2000-01-01T00:00:06Z",
    );
    let package = PathBuf::from(GOLDEN).join("typical.eh3dproj");
    p = save(
        p,
        "manufacturing",
        manufacturing_update(&package, "0.1.0").unwrap(),
        "2000-01-01T00:00:07Z",
    );
    save(p, "pcb", docs_update("pcb"), "2000-01-01T00:00:08Z")
}

/// The compressed bytes and the CRC of every entry of a ZIP file, by name.
pub fn raw_entries(path: &Path) -> BTreeMap<String, (u32, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut out = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index_raw(i).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        out.insert(
            entry.name().expect("entry name").into_owned(),
            (entry.crc32(), bytes),
        );
    }
    out
}

/// The text of an entry of a ZIP file (for the audit log, which is not part of a branch).
pub fn zip_text(path: &Path, name: &str) -> String {
    let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut out = String::new();
    entry.read_to_string(&mut out).unwrap();
    out
}

/// Flips one bit in the stored (compressed) data of an entry, leaving the rest of the file.
pub fn flip_in_entry(path: &Path, name: &str) {
    let start = {
        let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
        let entry = archive.by_name_raw(name.as_bytes()).unwrap();
        entry.header_start() as usize
    };
    let mut bytes = std::fs::read(path).unwrap();
    // the local header is 30 bytes, then the name and the extra field, then the data
    let name_len = usize::from(u16::from_le_bytes([bytes[start + 26], bytes[start + 27]]));
    let extra_len = usize::from(u16::from_le_bytes([bytes[start + 28], bytes[start + 29]]));
    bytes[start + 30 + name_len + extra_len] ^= 0x01;
    std::fs::write(path, bytes).unwrap();
}

/// Builds a file with exactly the entries given (a way to make the faults the API refuses to
/// make). `project_json` and the entries are written as they are; `integrity.json` is computed.
pub fn raw_project(path: &Path, project_json: &Value, entries: &[(&str, Vec<u8>)]) {
    let mut spec = eh3d();
    for (name, _) in entries {
        // a prefix of the name is enough for the writer to take it
        spec.allowed_roots.push((*name).to_owned());
    }
    let mut writer = ContainerWriter::new(&spec, path.parent().unwrap());
    writer.add("project.json", &canon(project_json)).unwrap();
    for (name, bytes) in entries {
        writer.add(name, bytes).unwrap();
    }
    writer
        .write_to_path(path, &AtomicOptions::default())
        .unwrap();
}

pub fn project_json(branches: Value) -> Value {
    json!({
        "schema": 1,
        "format_version": "1.0.0",
        "container_version": "1.0",
        "id": "raw",
        "name": "raw",
        "units": "millimetres",
        "branches": branches,
    })
}

pub fn branch_json(id: &str, entries: &[(&str, &[u8])], extra: Value) -> (Vec<u8>, String) {
    let shas: Vec<(String, u64, String)> = entries
        .iter()
        .map(|(n, b)| ((*n).to_owned(), b.len() as u64, sha256_hex(b)))
        .collect();
    let hash = eh3d_project::branch_content_hash(
        shas.iter().map(|(n, s, h)| (n.as_str(), *s, h.as_str())),
    );
    let mut record = json!({
        "schema": 1, "branch": id, "app": id, "format": "1.0", "app_version": "0.1.0",
        "content_hash": hash,
    });
    if let (Value::Object(r), Value::Object(e)) = (&mut record, extra) {
        r.extend(e);
    }
    (canon(&record), hash)
}

/// A toy signature: SHA-256 of the key and the message. It stands in for a real algorithm to
/// test the plumbing; it is not a security measure.
pub struct ToySigner(pub &'static str);

fn toy(key: &str, message: &[u8]) -> Vec<u8> {
    let mut input = key.as_bytes().to_vec();
    input.extend_from_slice(message);
    from_hex(&sha256_hex(&input)).unwrap()
}

impl Signer for ToySigner {
    fn algorithm(&self) -> &str {
        "toy-sha256"
    }
    fn key_id(&self) -> &str {
        self.0
    }
    fn sign(&self, message: &[u8]) -> Vec<u8> {
        toy(self.0, message)
    }
}

pub struct ToyVerifier(pub Vec<&'static str>);

impl Verifier for ToyVerifier {
    fn verify(
        &self,
        algorithm: &str,
        key_id: &str,
        message: &[u8],
        signature: &[u8],
    ) -> VerifyOutcome {
        if algorithm != "toy-sha256" {
            return VerifyOutcome::UnsupportedAlgorithm;
        }
        match self.0.iter().find(|k| **k == key_id) {
            None => VerifyOutcome::UnknownKey,
            Some(key) if toy(key, message) == signature => VerifyOutcome::Valid,
            Some(_) => VerifyOutcome::Invalid,
        }
    }
}

/// A small generator with a fixed seed (xorshift64*), so the tests make the same choices on
/// every computer.
pub struct Seed(pub u64);

impl Seed {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Writes the ZIP file again with the same entries in the same order and the same headers, but
/// compressed at `level`, so that its compressed bytes are not the ones the suite writes.
/// `change` gives other content to one entry (and nothing else, so the record is then wrong).
pub fn repack(path: &Path, level: i64, change: Option<(&str, &[u8])>) {
    let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(level))
        .system(zip::System::Unix)
        .last_modified_time(zip::DateTime::default());
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().expect("entry name").into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if let Some((_, other)) = change.filter(|(changed, _)| *changed == name) {
            bytes = other.to_vec();
        }
        writer.start_file(name.as_str(), options).unwrap();
        std::io::Write::write_all(&mut writer, &bytes).unwrap();
    }
    drop(archive);
    std::fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}
