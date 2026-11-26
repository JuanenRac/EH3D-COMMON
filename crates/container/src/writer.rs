//! The generic container writer: streaming, deterministic and atomic.
//!
//! Each entry is compressed once into a temporary single-entry archive next to the target while its
//! SHA-256 is computed, so the writer never holds an entry (or the archive) in memory. When the
//! package is written, the compressed entries are copied as they are into the final archive in the
//! order of the spec and `integrity.json` is finished last. The caller may add entries in any
//! order: the result does not depend on it.
//!
//! An entry that an open package already has, and that does not change, is not compressed again:
//! [`ContainerWriter::keep`] takes its compressed bytes and its CRC as they are stored, and
//! [`ContainerWriter::write_to_path_from`] checks every entry of the new file against its SHA-256
//! before the old file is replaced.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::atomic::{write_atomic, AtomicOptions};
use crate::canonical::canonical_file_bytes;
use crate::codes::PkgCode;
use crate::error::{ContainerError, Violation};
use crate::integrity::{Integrity, IntegrityEntry, ALGORITHM};
use crate::policy::{check_name, collision_key, Policy};
use crate::reader::Container;
use crate::spec::{ContainerSpec, EntryOrder, JsonForm, INTEGRITY_ENTRY, PROJECT_ENTRY};

static SPOOL_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Spooled {
    path: PathBuf,
    size: u64,
    sha256: String,
}

/// Where the compressed bytes of an entry come from when the package is written.
enum Source {
    /// Compressed by this writer into a temporary file.
    Spooled(Spooled),
    /// Taken as they are stored from the package given to the write, which records this of them.
    Stored(IntegrityEntry),
}

impl Source {
    fn record(&self) -> IntegrityEntry {
        match self {
            Source::Spooled(s) => IntegrityEntry {
                sha256: s.sha256.clone(),
                size: s.size,
            },
            Source::Stored(e) => e.clone(),
        }
    }
}

impl Drop for Spooled {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// A writer for one package. See the module documentation.
pub struct ContainerWriter {
    spec: ContainerSpec,
    spool_dir: PathBuf,
    entries: BTreeMap<String, Source>,
    keys: BTreeMap<String, String>,
    total: u64,
}

fn zip_error(e: zip::result::ZipError) -> ContainerError {
    ContainerError::Io(io::Error::other(e.to_string()))
}

/// The options that reproduce `.eh3dproj` 1.0.0 byte for byte: deflate, the fixed DOS time of
/// 1980-01-01, no extra fields, the default permissions, written by a Unix host on every platform.
fn entry_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .system(zip::System::Unix)
        .last_modified_time(zip::DateTime::default())
}

/// The external attributes this writer gives an entry, as the ZIP library works them out from
/// [`entry_options`].
fn written_attributes() -> Result<u32, ContainerError> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("a", entry_options()).map_err(zip_error)?;
    let mut archive = ZipArchive::new(zip.finish().map_err(zip_error)?).map_err(zip_error)?;
    let entry = archive.by_index_raw(0).map_err(zip_error)?;
    Ok(entry.external_attributes())
}

impl ContainerWriter {
    /// A writer that spools in `spool_dir`. Use the folder of the target: a full disk then fails
    /// before the target is touched.
    pub fn new(spec: &ContainerSpec, spool_dir: &Path) -> ContainerWriter {
        ContainerWriter {
            spec: spec.clone(),
            spool_dir: spool_dir.to_owned(),
            entries: BTreeMap::new(),
            keys: BTreeMap::new(),
            total: 0,
        }
    }

    /// Adds an entry from a stream, in any order. Refused: a bad name (001), a duplicate (002), a
    /// name outside the roots or `integrity.json` itself (009), or a size over a limit (011).
    pub fn add_stream(&mut self, name: &str, mut data: impl Read) -> Result<(), ContainerError> {
        let key = self.admit(name)?;
        let path = self.spool_dir.join(format!(
            ".{}.{}.spool{}",
            std::process::id(),
            SPOOL_COUNTER.fetch_add(1, Ordering::Relaxed),
            crate::atomic::TEMP_SUFFIX
        ));
        // From here on the guard removes the file, whatever happens.
        let mut guard = Spooled {
            path,
            size: 0,
            sha256: String::new(),
        };
        let entry_limit = self.spec.limits.max_entry_bytes_for(name);
        let file = File::create(&guard.path)?;
        let mut zip = ZipWriter::new(file);
        zip.start_file(name, entry_options()).map_err(zip_error)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut size: u64 = 0;
        loop {
            let n = data.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            size += n as u64;
            self.check_size(name, size, entry_limit)?;
            hasher.update(&buffer[..n]);
            zip.write_all(&buffer[..n])?;
        }
        zip.finish().map_err(zip_error)?.sync_all()?;
        guard.size = size;
        guard.sha256 = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        self.total += size;
        self.keys.insert(key, name.to_owned());
        self.entries.insert(name.to_owned(), Source::Spooled(guard));
        Ok(())
    }

    /// The checks of a name before an entry is taken; returns the key it is known by.
    fn admit(&self, name: &str) -> Result<String, ContainerError> {
        let limits = &self.spec.limits;
        check_name(self.spec.policy, name, limits.max_name_bytes)?;
        let key = if self.spec.policy == Policy::Strict {
            collision_key(name)
        } else {
            name.to_owned()
        };
        if let Some(first) = self.keys.get(&key) {
            return Err(Violation::new(
                PkgCode::DuplicateName,
                format!("same as '{first}' once case and Unicode form are ignored"),
            )
            .at(name)
            .into());
        }
        if name == INTEGRITY_ENTRY {
            return Err(Violation::new(
                PkgCode::OutsideRoots,
                "integrity.json is written by the container",
            )
            .at(name)
            .into());
        }
        let known = name == PROJECT_ENTRY
            || self.spec.is_data_entry(name)
            || self.spec.is_extension_entry(name);
        if !known {
            return Err(Violation::new(
                PkgCode::OutsideRoots,
                "the entry is not under a root this format allows",
            )
            .at(name)
            .into());
        }
        // + integrity.json
        if self.entries.len() + 2 > limits.max_entries {
            return Err(Violation::new(
                PkgCode::Limit,
                format!("more than {} entries", limits.max_entries),
            )
            .into());
        }
        Ok(key)
    }

    /// Refuses (011) an entry that is larger than its limit, or that passes the limit of all.
    fn check_size(&self, name: &str, size: u64, entry_limit: u64) -> Result<(), ContainerError> {
        if size > entry_limit {
            return Err(Violation::new(
                PkgCode::Limit,
                format!("the entry is larger than its limit of {entry_limit} bytes"),
            )
            .at(name)
            .into());
        }
        let total = self.spec.limits.max_total_bytes;
        if self.total + size > total {
            return Err(Violation::new(
                PkgCode::Limit,
                format!("the entries together exceed {total} bytes"),
            )
            .into());
        }
        Ok(())
    }

    /// Takes the entry `name` of the open package `from` unchanged; `recorded` is what the
    /// integrity record of that package says of it. When the entry is stored the way this writer
    /// stores one, nothing is compressed: its compressed bytes and its CRC are copied by
    /// [`ContainerWriter::write_to_path_from`], and the answer is true. An entry stored in another
    /// way (written by another tool) is read and compressed again, so that the new package keeps
    /// its one form, and the answer is false. The refusals are those of
    /// [`ContainerWriter::add_stream`], plus 010 for an entry that is not what `recorded` says.
    pub fn keep<R: Read + Seek>(
        &mut self,
        from: &mut Container<R>,
        name: &str,
        recorded: &IntegrityEntry,
    ) -> Result<bool, ContainerError> {
        let differs = || {
            ContainerError::from(
                Violation::new(PkgCode::Integrity, "the entry differs from the record").at(name),
            )
        };
        let attributes = written_attributes()?;
        let as_written = {
            let entry = from.raw_entry(name)?;
            let plain = entry.extra_data_fields().next().is_none();
            plain
                && entry.compression() == CompressionMethod::Deflated
                && entry.system() == zip::System::Unix
                && entry.external_attributes() == attributes
                && entry.last_modified() == Some(zip::DateTime::default())
                && entry.comment().is_empty()
                && !entry.encrypted()
        };
        if !as_written {
            let mut stream = from.read_stream(name)?;
            self.add_stream(name, &mut stream)?;
            if self.entries.get(name).map(Source::record).as_ref() != Some(recorded) {
                return Err(differs());
            }
            return Ok(false);
        }
        let key = self.admit(name)?;
        if from.entry(name).map(|e| e.size) != Some(recorded.size) {
            return Err(differs());
        }
        let entry_limit = self.spec.limits.max_entry_bytes_for(name);
        self.check_size(name, recorded.size, entry_limit)?;
        self.total += recorded.size;
        self.keys.insert(key, name.to_owned());
        self.entries
            .insert(name.to_owned(), Source::Stored(recorded.clone()));
        Ok(true)
    }

    /// Adds an entry from bytes.
    pub fn add(&mut self, name: &str, bytes: &[u8]) -> Result<(), ContainerError> {
        self.add_stream(name, bytes)
    }

    /// The record `integrity.json` will hold for what has been added.
    pub fn integrity(&self) -> Integrity {
        Integrity {
            algorithm: ALGORITHM.to_owned(),
            files: self
                .entries
                .iter()
                .map(|(name, source)| (name.clone(), source.record()))
                .collect(),
        }
    }

    /// The bytes of `integrity.json` in the JSON form of the spec.
    fn integrity_bytes(&self) -> Result<Vec<u8>, ContainerError> {
        let record = self.integrity();
        match self.spec.json_form {
            JsonForm::LegacyV1 => serde_json::to_string_pretty(&record)
                .map(String::into_bytes)
                .map_err(|e| ContainerError::Io(io::Error::other(e.to_string()))),
            JsonForm::Jcs => {
                let value = serde_json::to_value(&record)
                    .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))?;
                canonical_file_bytes(&value)
                    .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))
            }
        }
    }

    /// Writes the package to `path` through [`write_atomic`]: assembled in the order of the spec,
    /// `integrity.json` last, replaced atomically. The writer is consumed; its spooled entries
    /// are removed. Entries taken with [`ContainerWriter::keep`] need
    /// [`ContainerWriter::write_to_path_from`].
    pub fn write_to_path(self, path: &Path, options: &AtomicOptions) -> Result<(), ContainerError> {
        let integrity = self.finished_record()?;
        write_atomic(path, options, |file| {
            self.assemble::<File>(file, &integrity, None)
        })
    }

    /// Like [`ContainerWriter::write_to_path`], with the package `from` that the entries taken by
    /// [`ContainerWriter::keep`] are copied from (it may be the file at `path`; it is closed before
    /// the replacement). Before the target is replaced, the new file is read back and every entry
    /// is checked against its CRC and its SHA-256, so a damaged entry never replaces a file: the
    /// write is refused and the target stays as it was. Returns the record that was written.
    pub fn write_to_path_from<R: Read + Seek>(
        self,
        path: &Path,
        options: &AtomicOptions,
        from: Container<R>,
    ) -> Result<Integrity, ContainerError> {
        let integrity = self.finished_record()?;
        let mut written = None;
        write_atomic(path, options, |file| {
            // `from` moves in here so that it is closed before the rename.
            let mut from = from;
            self.assemble(file, &integrity, Some(&mut from))?;
            drop(from);
            file.flush()?;
            written = Some(Container::open(&*file, &self.spec)?.verify_integrity()?);
            Ok(())
        })?;
        Ok(written.expect("set when the write succeeds"))
    }

    /// The bytes of `integrity.json` of a package that has everything it needs.
    fn finished_record(&self) -> Result<Vec<u8>, ContainerError> {
        if !self.entries.contains_key(PROJECT_ENTRY) {
            return Err(
                Violation::new(PkgCode::OutsideRoots, "a required entry is missing")
                    .at(PROJECT_ENTRY)
                    .into(),
            );
        }
        let record = self.integrity_bytes()?;
        // the reader counts integrity.json with the entries: a package it would refuse is not written
        self.check_size(
            INTEGRITY_ENTRY,
            record.len() as u64,
            self.spec.limits.max_entry_bytes_for(INTEGRITY_ENTRY),
        )?;
        Ok(record)
    }

    /// Writes the archive into `file`.
    fn assemble<R: Read + Seek>(
        &self,
        file: &File,
        integrity: &[u8],
        mut from: Option<&mut Container<R>>,
    ) -> Result<(), ContainerError> {
        let mut zip = ZipWriter::new(file);
        let mut copy = |zip: &mut ZipWriter<&File>, name: &str| -> Result<(), ContainerError> {
            match &self.entries[name] {
                Source::Spooled(spooled) => {
                    let mut archive =
                        ZipArchive::new(File::open(&spooled.path)?).map_err(zip_error)?;
                    let entry = archive.by_index_raw(0).map_err(zip_error)?;
                    zip.raw_copy_file(entry).map_err(zip_error)
                }
                Source::Stored(_) => {
                    let from = from.as_deref_mut().ok_or_else(|| {
                        ContainerError::Io(io::Error::other(
                            "a kept entry needs the package it was taken from",
                        ))
                    })?;
                    let entry = from.raw_entry(name)?;
                    zip.raw_copy_file(entry).map_err(zip_error)
                }
            }
        };
        let finish_integrity = |zip: &mut ZipWriter<&File>| -> Result<(), ContainerError> {
            zip.start_file(INTEGRITY_ENTRY, entry_options())
                .map_err(zip_error)?;
            zip.write_all(integrity)?;
            Ok(())
        };
        let data = self.entries.keys().filter(|k| k.as_str() != PROJECT_ENTRY);
        match self.spec.entry_order {
            EntryOrder::LegacyV1 => {
                copy(&mut zip, PROJECT_ENTRY)?;
                finish_integrity(&mut zip)?;
                for name in data {
                    copy(&mut zip, name)?;
                }
            }
            EntryOrder::SortedThenProjectThenIntegrity => {
                for name in data {
                    copy(&mut zip, name)?;
                }
                copy(&mut zip, PROJECT_ENTRY)?;
                finish_integrity(&mut zip)?;
            }
        }
        zip.finish().map_err(zip_error)?;
        Ok(())
    }
}
