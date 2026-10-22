//! The `.eh3dproj` container: a standard ZIP archive that carries one project.
//!
//! ```text
//! project.json      the metadata (see [`crate::project`])
//! integrity.json    the SHA-256 and the size of every other entry
//! layers/...        the original Gerber / Excellon / outline files
//! toolpaths/...     precalculated G-code or HP-GL
//! vision/...        fiducials and inspection templates
//! ```
//!
//! A package refuses to open when an entry does not match its recorded checksum, when an entry is
//! not recorded at all, when an entry name could leave the package folder, or when it is larger
//! than the limits below (they stop a decompression bomb).

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{Error, Result};
use crate::project::{read_metadata, ProjectMetadata, DATA_FOLDERS};

/// The file extension of a project package, without the dot.
pub const EXTENSION: &str = "eh3dproj";
/// The name of the metadata entry.
pub const PROJECT_ENTRY: &str = "project.json";
/// The name of the integrity entry.
pub const INTEGRITY_ENTRY: &str = "integrity.json";
/// The largest an entry may be once decompressed.
pub const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
/// The largest a whole package may be once decompressed.
pub const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
/// The most entries a package may have.
pub const MAX_ENTRIES: usize = 10_000;

/// The checksum record of one entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityEntry {
    /// The SHA-256 of the entry, in lowercase hexadecimal.
    pub sha256: String,
    /// The size of the entry in bytes.
    pub size: u64,
}

/// What `integrity.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integrity {
    /// Always `sha256`.
    pub algorithm: String,
    /// One record per entry other than `integrity.json` itself.
    pub files: BTreeMap<String, IntegrityEntry>,
}

/// Checks that `name` is a portable path inside the package: relative, forward slashes only, no
/// `.`/`..` parts, no empty parts, no drive letters, no control characters, a reasonable length.
pub fn check_entry_name(name: &str) -> Result<()> {
    let unsafe_name = || Err(Error::UnsafePath(name.to_owned()));
    if name.is_empty() || name.len() > 240 || name.starts_with('/') || name.ends_with('/') {
        return unsafe_name();
    }
    if name.contains('\\') || name.contains(':') || name.chars().any(char::is_control) {
        return unsafe_name();
    }
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return unsafe_name();
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// A project in memory: its metadata and the bytes of every data entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    /// The metadata (`project.json`).
    pub metadata: ProjectMetadata,
    entries: BTreeMap<String, Vec<u8>>,
}

impl Package {
    /// An empty package for `metadata`.
    pub fn new(metadata: ProjectMetadata) -> Self {
        Self {
            metadata,
            entries: BTreeMap::new(),
        }
    }

    /// Adds a data entry. The name must be safe and under `layers/`, `toolpaths/` or `vision/`.
    /// Adding the same name twice replaces the earlier bytes.
    pub fn add_entry(&mut self, name: &str, bytes: Vec<u8>) -> Result<()> {
        check_entry_name(name)?;
        if !DATA_FOLDERS.iter().any(|folder| name.starts_with(folder)) {
            return Err(Error::UnsafePath(name.to_owned()));
        }
        if bytes.len() as u64 > MAX_ENTRY_BYTES {
            return Err(Error::TooLarge(format!(
                "'{name}' is larger than {MAX_ENTRY_BYTES} bytes"
            )));
        }
        if !self.entries.contains_key(name) && self.entries.len() + 2 >= MAX_ENTRIES {
            return Err(Error::TooLarge(format!("more than {MAX_ENTRIES} entries")));
        }
        self.entries.insert(name.to_owned(), bytes);
        Ok(())
    }

    /// The bytes of a data entry.
    pub fn entry(&self, name: &str) -> Option<&[u8]> {
        self.entries.get(name).map(Vec::as_slice)
    }

    /// The names of the data entries, in alphabetical order.
    pub fn entry_names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// The integrity record that [`Package::write`] stores for the current content.
    pub fn integrity(&self) -> Result<Integrity> {
        let project = self.metadata.to_json()?;
        let mut files = BTreeMap::new();
        files.insert(
            PROJECT_ENTRY.to_owned(),
            IntegrityEntry {
                sha256: sha256_hex(project.as_bytes()),
                size: project.len() as u64,
            },
        );
        for (name, bytes) in &self.entries {
            files.insert(
                name.clone(),
                IntegrityEntry {
                    sha256: sha256_hex(bytes),
                    size: bytes.len() as u64,
                },
            );
        }
        Ok(Integrity {
            algorithm: "sha256".into(),
            files,
        })
    }

    /// Writes the package. The result does not depend on the clock: the same content gives the
    /// same bytes, so two copies of a project can be compared by hash.
    pub fn write<W: Write + Seek>(&self, writer: W) -> Result<()> {
        let project = self.metadata.to_json()?;
        let integrity = serde_json::to_string_pretty(&self.integrity()?)?;
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default());
        let mut zip = ZipWriter::new(writer);
        zip.start_file(PROJECT_ENTRY, options)?;
        zip.write_all(project.as_bytes())?;
        zip.start_file(INTEGRITY_ENTRY, options)?;
        zip.write_all(integrity.as_bytes())?;
        for (name, bytes) in &self.entries {
            zip.start_file(name.as_str(), options)?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
        Ok(())
    }

    /// The whole package as bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());
        self.write(&mut cursor)?;
        Ok(cursor.into_inner())
    }

    /// Writes the package to `path` through a temporary file, so a crash never leaves half a project.
    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        let temporary = path.with_extension(format!("{EXTENSION}.tmp"));
        {
            let file = File::create(&temporary)?;
            self.write(&file)?;
            file.sync_all()?;
        }
        fs::rename(&temporary, path)?;
        Ok(())
    }

    /// Reads and checks a package.
    pub fn read<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut archive = ZipArchive::new(reader)?;
        if archive.len() > MAX_ENTRIES {
            return Err(Error::TooLarge(format!("more than {MAX_ENTRIES} entries")));
        }
        let mut raw: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut total: u64 = 0;
        for index in 0..archive.len() {
            let file = archive.by_index(index)?;
            if file.is_dir() {
                continue;
            }
            let name = file.name().to_owned();
            check_entry_name(&name)?;
            if raw.contains_key(&name) {
                return Err(Error::DuplicateEntry(name));
            }
            let mut bytes = Vec::new();
            file.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_ENTRY_BYTES {
                return Err(Error::TooLarge(format!(
                    "'{name}' is larger than {MAX_ENTRY_BYTES} bytes"
                )));
            }
            total += bytes.len() as u64;
            if total > MAX_TOTAL_BYTES {
                return Err(Error::TooLarge(format!(
                    "the package is larger than {MAX_TOTAL_BYTES} bytes"
                )));
            }
            raw.insert(name, bytes);
        }
        let integrity_bytes = raw
            .remove(INTEGRITY_ENTRY)
            .ok_or_else(|| Error::MissingEntry(INTEGRITY_ENTRY.into()))?;
        let integrity: Integrity = serde_json::from_slice(&integrity_bytes)?;
        if integrity.algorithm != "sha256" {
            return Err(Error::Integrity {
                path: INTEGRITY_ENTRY.into(),
                reason: format!("unknown algorithm '{}'", integrity.algorithm),
            });
        }
        let mut seen = HashSet::new();
        for (name, record) in &integrity.files {
            let bytes = raw
                .get(name)
                .ok_or_else(|| Error::MissingEntry(name.clone()))?;
            if bytes.len() as u64 != record.size {
                return Err(Error::Integrity {
                    path: name.clone(),
                    reason: "the size differs from the record".into(),
                });
            }
            if sha256_hex(bytes) != record.sha256 {
                return Err(Error::Integrity {
                    path: name.clone(),
                    reason: "the SHA-256 differs from the record".into(),
                });
            }
            seen.insert(name.as_str());
        }
        if let Some(extra) = raw.keys().find(|name| !seen.contains(name.as_str())) {
            return Err(Error::Integrity {
                path: extra.clone(),
                reason: "the entry is not listed in integrity.json".into(),
            });
        }
        let project = raw
            .remove(PROJECT_ENTRY)
            .ok_or_else(|| Error::MissingEntry(PROJECT_ENTRY.into()))?;
        let metadata = read_metadata(
            std::str::from_utf8(&project)
                .map_err(|_| Error::InvalidMetadata("project.json is not UTF-8".into()))?,
        )?;
        let mut package = Self::new(metadata);
        for (name, bytes) in raw {
            package.add_entry(&name, bytes)?;
        }
        Ok(package)
    }

    /// Reads and checks the package at `path`.
    pub fn read_from_path(path: &Path) -> Result<Self> {
        Self::read(File::open(path)?)
    }
}
