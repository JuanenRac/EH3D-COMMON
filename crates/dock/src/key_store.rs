//! Where the **path** of the signing key a person used last is kept, so that a program offers it again: a file of its own in the settings
//! folder of the user (the folder of [`crate::recent_store`]), one for each program of the PHOENIX family. It holds the path and nothing
//! else: nothing of this module reads the key file, and no key and no password is ever written here. Only an absolute path of one line
//! is kept, never an address (anything with `://`, which could carry a password). A file that cannot be used is left aside without an
//! error, and it is written in one step.
//!
//! As in [`crate::recent_store`], nothing looks at the environment by itself: [`KeyStore::for_user`] is what a program calls, and a
//! test gives a folder of its own with [`KeyStore::in_dir`].

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::about::Phoenix;
use crate::recent_store::user_dir;

/// The first line of the file.
const HEADER: &str = "eh3d.signing-key=1";
/// What comes before the path.
const PREFIX: &str = "key.path=";
/// The largest file that is read.
const MAX_FILE: u64 = 8 * 1024;

/// The name of the file of a program.
fn file_name(program: Phoenix) -> String {
    let name = program.name().to_ascii_lowercase();
    format!("signing-key-{}.txt", name.trim_start_matches("eh3d-"))
}

/// The text of a path that may be kept.
fn usable(path: &Path) -> Option<&str> {
    path.to_str().filter(|text| {
        path.is_absolute() && !text.contains("://") && !text.chars().any(char::is_control)
    })
}

/// The path of the signing key of one program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyStore {
    path: PathBuf,
}

impl KeyStore {
    /// The one of a program in a folder.
    pub fn in_dir(dir: &Path, program: Phoenix) -> KeyStore {
        KeyStore {
            path: dir.join(file_name(program)),
        }
    }

    /// The one of a program in the settings folder of the user; None when the environment names no folder.
    pub fn for_user(program: Phoenix) -> Option<KeyStore> {
        user_dir(&|name| std::env::var(name).ok()).map(|dir| KeyStore::in_dir(&dir, program))
    }

    /// The file where the path is kept.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path of the key used last. It may no longer be there: the program looks before it uses it.
    pub fn load(&self) -> Option<PathBuf> {
        let file = fs::File::open(&self.path).ok()?;
        let mut text = String::new();
        file.take(MAX_FILE + 1).read_to_string(&mut text).ok()?;
        let mut lines = text.lines();
        if text.len() as u64 > MAX_FILE || lines.next() != Some(HEADER) {
            return None;
        }
        let key = PathBuf::from(lines.next()?.strip_prefix(PREFIX)?);
        usable(&key).is_some().then_some(key)
    }

    /// Keeps the path of the key that was used. A relative path, an address or a path with a control character is refused.
    pub fn remember(&self, key: &Path) -> io::Result<()> {
        static CALLS: AtomicU32 = AtomicU32::new(0);
        let key = usable(key).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "only the absolute path of a key file is kept",
            )
        })?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let call = CALLS.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .path
            .with_extension(format!("{}-{call}.tmp", std::process::id()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            // where a person keeps a key is for that person
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let written = options.open(&temporary).and_then(|mut file| {
            file.write_all(format!("{HEADER}\n{PREFIX}{key}\n").as_bytes())?;
            file.sync_all()
        });
        let moved = written.and_then(|()| fs::rename(&temporary, &self.path));
        if moved.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        moved
    }

    /// Forgets the path. Nothing happens when none is kept.
    pub fn forget(&self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own for one test, removed at the end.
    struct Folder(PathBuf);

    impl Folder {
        fn new() -> Folder {
            static CALLS: AtomicU32 = AtomicU32::new(0);
            let call = CALLS.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("eh3d-key-store-{}-{call}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Folder(dir)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_path_is_kept_read_back_and_forgotten_and_the_key_is_never_read() {
        let folder = Folder::new();
        let store = KeyStore::in_dir(&folder.0.join("EH3D"), Phoenix::Cad);
        assert_eq!(store.load(), None);
        store.forget().unwrap();
        // the key file is not there: only its path matters here
        let key = folder.0.join("my keys").join("ñ 鍵.key");
        store.remember(&key).unwrap();
        assert_eq!(store.load(), Some(key.clone()));
        assert_eq!(
            fs::read_to_string(store.path()).unwrap(),
            format!("{HEADER}\n{PREFIX}{}\n", key.display())
        );
        let other = folder.0.join("other.key");
        store.remember(&other).unwrap();
        assert_eq!(store.load(), Some(other));
        assert_eq!(fs::read_dir(folder.0.join("EH3D")).unwrap().count(), 1);
        store.forget().unwrap();
        assert_eq!(store.load(), None);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            store.remember(&key).unwrap();
            let mode = fs::metadata(store.path()).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn what_is_not_an_absolute_path_of_one_line_is_neither_written_nor_read() {
        let folder = Folder::new();
        let store = KeyStore::in_dir(&folder.0, Phoenix::Laser);
        let root = std::env::temp_dir();
        for bad in [
            PathBuf::from("relative.key"),
            PathBuf::new(),
            root.join("a\nb.key"),
            PathBuf::from(format!("https://{}@example.invalid/k", "someone")),
        ] {
            assert!(store.remember(&bad).is_err(), "{bad:?}");
        }
        assert!(!store.path().exists());
        fs::create_dir_all(&folder.0).unwrap();
        for damaged in [
            String::new(),
            format!("{PREFIX}{}\n", root.join("k").display()),
            format!("{HEADER}\n{PREFIX}relative\n"),
            format!("{HEADER}\nkey.text=x\n"),
            format!("{HEADER}\n{}", "#\n".repeat(MAX_FILE as usize)),
        ] {
            fs::write(store.path(), damaged).unwrap();
            assert_eq!(store.load(), None);
        }
    }

    #[test]
    fn each_program_has_its_own_file_apart_from_the_recent_projects() {
        let names: Vec<String> = Phoenix::ALL.into_iter().map(file_name).collect();
        assert_eq!(names[0], "signing-key-phoenix-pcb.txt");
        for (i, name) in names.iter().enumerate() {
            assert!(!names[..i].contains(name), "{name}");
        }
    }
}
