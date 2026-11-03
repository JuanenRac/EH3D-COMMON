//! Atomic replacement of a file, with bounded retries for the moments when another program holds it.
//!
//! The new content is written to a temporary file in the same folder, flushed and synced; then it
//! replaces the target with one rename (an atomic replace on every platform the suite supports; on
//! Windows the standard library asks for replace-existing). The old file stays in place until the
//! new one is complete. On Windows an antivirus scanner, the search indexer or a cloud-sync client
//! may hold a freshly written file for a moment, so a refused replace is retried with a short,
//! bounded back-off; when the time is up the target is left exactly as it was (`EH3D-PKG-014`).
//!
//! The guarantee is stated per failure: strong against the process being killed at any point (the
//! target is either the old file or the new one); against sudden power loss it is as strong as the
//! file system and the hardware allow, and durable only after the syncs this module performs.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::codes::PkgCode;
use crate::error::{ContainerError, Violation};

/// The suffix of every temporary file this module creates.
pub const TEMP_SUFFIX: &str = ".eh3dtmp";

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// How an atomic write behaves. Both values are settings meant to come from a configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtomicOptions {
    /// The most milliseconds to keep retrying a refused replace; 0 means one attempt.
    pub retry_millis: u64,
    /// Keep a copy of the file being replaced as `<name>.bak` (for migrations).
    pub keep_backup: bool,
}

impl Default for AtomicOptions {
    fn default() -> Self {
        AtomicOptions {
            retry_millis: 2_000,
            keep_backup: false,
        }
    }
}

fn temp_path(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    target.with_file_name(format!(".{name}.{}.{n}{TEMP_SUFFIX}", std::process::id()))
}

/// True for the errors a program holding the file produces: sharing violation, lock violation and
/// access denied (Windows reports a rename over an open file this way).
fn is_retryable(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(5 | 32 | 33))
        || error.kind() == io::ErrorKind::PermissionDenied
}

/// Writes `target` atomically: `write` receives the temporary file and fills it.
///
/// If `write` fails, the replace fails or the time for retries runs out, the temporary file is
/// removed and the target is untouched. A failure to replace is `EH3D-PKG-014`; a failure of
/// `write` is passed on as it came.
pub fn write_atomic<F>(
    target: &Path,
    options: &AtomicOptions,
    write: F,
) -> Result<(), ContainerError>
where
    F: FnOnce(&mut File) -> Result<(), ContainerError>,
{
    let temporary = temp_path(target);
    let outcome = write_and_replace(target, &temporary, options, write);
    if outcome.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    outcome
}

fn write_and_replace<F>(
    target: &Path,
    temporary: &Path,
    options: &AtomicOptions,
    write: F,
) -> Result<(), ContainerError>
where
    F: FnOnce(&mut File) -> Result<(), ContainerError>,
{
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    write(&mut file)?;
    file.flush()?;
    file.sync_all()?;
    drop(file);

    if options.keep_backup && target.is_file() {
        let mut backup = target.as_os_str().to_owned();
        backup.push(".bak");
        fs::copy(target, PathBuf::from(backup))?;
    }

    let mut waited = Duration::ZERO;
    let mut delay = Duration::from_millis(10);
    let limit = Duration::from_millis(options.retry_millis);
    loop {
        match fs::rename(temporary, target) {
            Ok(()) => break,
            Err(e) if is_retryable(&e) && waited < limit => {
                let pause = delay.min(limit - waited);
                thread::sleep(pause);
                waited += pause;
                delay = (delay * 2).min(Duration::from_millis(250));
            }
            Err(e) => {
                return Err(Violation::new(
                    PkgCode::AtomicReplace,
                    format!("the target could not be replaced: {e}"),
                )
                .into());
            }
        }
    }
    sync_folder(target);
    Ok(())
}

/// Syncs the folder so the rename itself is durable, where the platform lets a folder be synced.
fn sync_folder(target: &Path) {
    #[cfg(unix)]
    if let Some(folder) = target.parent() {
        let folder = if folder.as_os_str().is_empty() {
            Path::new(".")
        } else {
            folder
        };
        if let Ok(handle) = File::open(folder) {
            let _ = handle.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = target;
}

/// Removes the temporary files that an interrupted write left in `folder`: only files made by
/// this module (a name that starts with a dot and ends with [`TEMP_SUFFIX`]) and older than
/// `min_age`. Nothing else is touched, so recovery data of an open or recent project is never
/// removed. Returns how many files were removed.
pub fn clean_orphan_temporaries(folder: &Path, min_age: Duration) -> io::Result<usize> {
    let mut removed = 0;
    for item in fs::read_dir(folder)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if !(name.starts_with('.') && name.ends_with(TEMP_SUFFIX)) || !item.file_type()?.is_file() {
            continue;
        }
        let age = item
            .metadata()?
            .modified()
            .ok()
            .and_then(|m| SystemTime::now().duration_since(m).ok())
            .unwrap_or(Duration::ZERO);
        if age >= min_age && fs::remove_file(item.path()).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "eh3d_atomic_{tag}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn put(bytes: &'static [u8]) -> impl FnOnce(&mut File) -> Result<(), ContainerError> {
        move |f| f.write_all(bytes).map_err(Into::into)
    }

    fn leftovers(folder: &Path) -> Vec<String> {
        fs::read_dir(folder)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(TEMP_SUFFIX))
            .collect()
    }

    #[test]
    fn a_new_file_and_a_replacement_are_written_whole() {
        let dir = folder("write");
        let target = dir.join("p.bin");
        write_atomic(&target, &AtomicOptions::default(), put(b"one")).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"one");
        write_atomic(&target, &AtomicOptions::default(), put(b"two")).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"two");
        assert!(leftovers(&dir).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_interrupted_write_leaves_the_old_file_and_no_temporary() {
        let dir = folder("cut");
        let target = dir.join("p.bin");
        fs::write(&target, b"old").unwrap();
        let result = write_atomic(&target, &AtomicOptions::default(), |f| {
            f.write_all(b"half")?;
            Err(ContainerError::Io(io::Error::other("cut here")))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert!(leftovers(&dir).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_target_that_cannot_be_replaced_is_014_and_untouched() {
        let dir = folder("dir");
        let target = dir.join("p.bin");
        // A non-empty folder cannot be replaced by a file on any platform.
        fs::create_dir_all(target.join("inner")).unwrap();
        let options = AtomicOptions {
            retry_millis: 30,
            keep_backup: false,
        };
        let error = write_atomic(&target, &options, put(b"new")).unwrap_err();
        assert_eq!(error.code(), Some(PkgCode::AtomicReplace));
        assert!(target.join("inner").is_dir());
        assert!(leftovers(&dir).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_backup_keeps_the_previous_file() {
        let dir = folder("bak");
        let target = dir.join("p.bin");
        fs::write(&target, b"v1").unwrap();
        let options = AtomicOptions {
            keep_backup: true,
            ..AtomicOptions::default()
        };
        write_atomic(&target, &options, put(b"v2")).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"v2");
        assert_eq!(fs::read(dir.join("p.bin.bak")).unwrap(), b"v1");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_old_temporaries_of_this_module_are_cleaned() {
        let dir = folder("clean");
        fs::write(dir.join(".p.bin.1.0.eh3dtmp"), b"x").unwrap();
        fs::write(dir.join("p.bin"), b"keep").unwrap();
        fs::write(dir.join("notes.eh3dtmp"), b"keep too").unwrap();
        assert_eq!(
            clean_orphan_temporaries(&dir, Duration::from_secs(3600)).unwrap(),
            0,
            "a fresh temporary may belong to a write in progress"
        );
        assert_eq!(clean_orphan_temporaries(&dir, Duration::ZERO).unwrap(), 1);
        assert!(dir.join("p.bin").exists() && dir.join("notes.eh3dtmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_target_held_by_another_program_is_retried_then_released() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = folder("lock");
        let target = dir.join("p.bin");
        fs::write(&target, b"old").unwrap();
        // Held with no sharing at all, like a scanner that is reading the file.
        let held = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&target)
            .unwrap();
        let short = AtomicOptions {
            retry_millis: 60,
            keep_backup: false,
        };
        let error = write_atomic(&target, &short, put(b"new")).unwrap_err();
        assert_eq!(error.code(), Some(PkgCode::AtomicReplace));
        drop(held);
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert!(leftovers(&dir).is_empty());

        let held = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&target)
            .unwrap();
        let releaser = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            drop(held);
        });
        write_atomic(&target, &AtomicOptions::default(), put(b"new")).unwrap();
        releaser.join().unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        let _ = fs::remove_dir_all(&dir);
    }
}
