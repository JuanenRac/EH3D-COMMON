//! The generic container writer: streaming, deterministic and atomic.
//!
//! Each entry is compressed once into a temporary single-entry archive next to the target while its
//! SHA-256 is computed, so the writer never holds an entry (or the archive) in memory. When the
//! package is written, the compressed entries are copied as they are into the final archive in the
//! order of the spec and `integrity.json` is finished last. The caller may add entries in any
//! order: the result does not depend on it.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
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
use crate::spec::{ContainerSpec, EntryOrder, JsonForm, INTEGRITY_ENTRY, PROJECT_ENTRY};

static SPOOL_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Spooled {
    path: PathBuf,
    size: u64,
    sha256: String,
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
    entries: BTreeMap<String, Spooled>,
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
        let limits = self.spec.limits.clone();
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
        let entry_limit = limits.max_entry_bytes_for(name);
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
            if size > entry_limit {
                return Err(Violation::new(
                    PkgCode::Limit,
                    format!("the entry is larger than its limit of {entry_limit} bytes"),
                )
                .at(name)
                .into());
            }
            if self.total + size > limits.max_total_bytes {
                return Err(Violation::new(
                    PkgCode::Limit,
                    format!(
                        "the entries together exceed {} bytes",
                        limits.max_total_bytes
                    ),
                )
                .into());
            }
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
        self.entries.insert(name.to_owned(), guard);
        Ok(())
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
                .map(|(name, s)| {
                    (
                        name.clone(),
                        IntegrityEntry {
                            sha256: s.sha256.clone(),
                            size: s.size,
                        },
                    )
                })
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
    /// are removed.
    pub fn write_to_path(self, path: &Path, options: &AtomicOptions) -> Result<(), ContainerError> {
        if !self.entries.contains_key(PROJECT_ENTRY) {
            return Err(
                Violation::new(PkgCode::OutsideRoots, "a required entry is missing")
                    .at(PROJECT_ENTRY)
                    .into(),
            );
        }
        let integrity = self.integrity_bytes()?;
        write_atomic(path, options, |file| {
            let mut zip = ZipWriter::new(&*file);
            let copy = |zip: &mut ZipWriter<&File>, name: &String| -> Result<(), ContainerError> {
                let spooled = &self.entries[name];
                let mut archive = ZipArchive::new(File::open(&spooled.path)?).map_err(zip_error)?;
                let entry = archive.by_index_raw(0).map_err(zip_error)?;
                zip.raw_copy_file(entry).map_err(zip_error)
            };
            let project = self
                .entries
                .get_key_value(PROJECT_ENTRY)
                .map(|(k, _)| k)
                .expect("checked above");
            let data: Vec<&String> = self
                .entries
                .keys()
                .filter(|k| k.as_str() != PROJECT_ENTRY)
                .collect();
            let finish_integrity = |zip: &mut ZipWriter<&File>| -> Result<(), ContainerError> {
                zip.start_file(INTEGRITY_ENTRY, entry_options())
                    .map_err(zip_error)?;
                zip.write_all(&integrity)?;
                Ok(())
            };
            match self.spec.entry_order {
                EntryOrder::LegacyV1 => {
                    copy(&mut zip, project)?;
                    finish_integrity(&mut zip)?;
                    for name in data {
                        copy(&mut zip, name)?;
                    }
                }
                EntryOrder::SortedThenProjectThenIntegrity => {
                    for name in data {
                        copy(&mut zip, name)?;
                    }
                    copy(&mut zip, project)?;
                    finish_integrity(&mut zip)?;
                }
            }
            zip.finish().map_err(zip_error)?;
            Ok(())
        })
    }
}
