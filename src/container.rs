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

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use eh3d_container::{IntegrityError, NameError};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{Error, Result};
use crate::project::{read_metadata, ProjectMetadata, DATA_FOLDERS};

// The integrity record and the name rule are shared with the other container formats; they live
// in `eh3d-container` and keep their shape and behaviour here.
pub use eh3d_container::{Integrity, IntegrityEntry};

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

/// Checks that `name` is a portable path inside the package: relative, forward slashes only, no
/// `.`/`..` parts, no empty parts, no drive letters, no control characters, a reasonable length.
pub fn check_entry_name(name: &str) -> Result<()> {
    eh3d_container::check_entry_name(name)
        .map_err(|NameError::Unsafe(name)| Error::UnsafePath(name))
}

/// The error a failed integrity check becomes in this crate.
fn integrity_error(error: IntegrityError) -> Error {
    let integrity = |path: String, reason: &str| Error::Integrity {
        path,
        reason: reason.into(),
    };
    match error {
        IntegrityError::UnknownAlgorithm(algorithm) => Error::Integrity {
            path: INTEGRITY_ENTRY.into(),
            reason: format!("unknown algorithm '{algorithm}'"),
        },
        IntegrityError::MissingEntry(name) => Error::MissingEntry(name),
        IntegrityError::SizeDiffers(name) => integrity(name, "the size differs from the record"),
        IntegrityError::HashDiffers(name) => integrity(name, "the SHA-256 differs from the record"),
        IntegrityError::NotListed(name) => {
            integrity(name, "the entry is not listed in integrity.json")
        }
    }
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

    /// Every layer and toolpath that the metadata names must be in the package.
    fn check_references(&self) -> Result<()> {
        let named = self
            .metadata
            .layers
            .iter()
            .map(|l| &l.path)
            .chain(self.metadata.toolpaths.iter().map(|p| &p.path));
        for path in named {
            if !self.entries.contains_key(path.as_str()) {
                return Err(Error::MissingEntry(path.clone()));
            }
        }
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
        let entries = std::iter::once((PROJECT_ENTRY, project.as_bytes())).chain(
            self.entries
                .iter()
                .map(|(name, bytes)| (name.as_str(), bytes.as_slice())),
        );
        Ok(Integrity::for_entries(entries))
    }

    /// Writes the package. The result does not depend on the clock: the same content gives the
    /// same bytes, so two copies of a project can be compared by hash.
    pub fn write<W: Write + Seek>(&self, writer: W) -> Result<()> {
        self.check_references()?;
        let project = self.metadata.to_json()?;
        let integrity = serde_json::to_string_pretty(&self.integrity()?)?;
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .system(zip::System::Unix)
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
            let name = file.name()?.into_owned();
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
        integrity.verify(&raw).map_err(integrity_error)?;
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
        package.check_references()?;
        Ok(package)
    }

    /// Reads and checks the package at `path`.
    pub fn read_from_path(path: &Path) -> Result<Self> {
        Self::read(File::open(path)?)
    }
}
