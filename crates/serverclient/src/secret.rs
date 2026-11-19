//! The token of a login: a value that cannot be printed by mistake, and the places it may be kept.

use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

/// A secret text (a token). `Debug` and `Display` write `***`, never the text; the bytes are overwritten when the value is dropped. The text is
/// read only with [`Secret::expose`], which is the one place a program has to name on purpose.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(text: impl Into<String>) -> Secret {
        Secret(text.into())
    }

    /// The text, for the one line that sends it (`Authorization: Bearer`). Never log what this returns.
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // best effort: overwrite the bytes in place before the memory goes back
        let mut bytes = std::mem::take(&mut self.0).into_bytes();
        bytes.fill(0);
        std::hint::black_box(&bytes);
    }
}

/// Where a program keeps the token between runs. A program with a safer place of its own (the keychain, the Keystore, an encrypted file) writes
/// its own implementation; the client only asks for these three things.
pub trait TokenStore: Send {
    fn load(&self) -> Option<Secret>;
    fn save(&self, token: &Secret) -> Result<(), String>;
    fn clear(&self);
}

/// Keeps the token in memory only: nothing is written anywhere (the default).
#[derive(Default)]
pub struct MemoryStore(Mutex<Option<Secret>>);

impl TokenStore for MemoryStore {
    fn load(&self) -> Option<Secret> {
        self.0.lock().ok().and_then(|g| g.clone())
    }

    fn save(&self, token: &Secret) -> Result<(), String> {
        if let Ok(mut g) = self.0.lock() {
            *g = Some(token.clone());
        }
        Ok(())
    }

    fn clear(&self) {
        if let Ok(mut g) = self.0.lock() {
            *g = None;
        }
    }
}

/// Keeps the token in a file that only its owner may read: on Unix it is created with mode 0600 and a file that is open to the group or to others is
/// refused when read; on Windows the file lives in the user's own settings folder and its inheritance is the one of that folder (the program that needs
/// more uses the system keychain through its own [`TokenStore`]).
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub fn new(path: impl Into<PathBuf>) -> FileStore {
        FileStore { path: path.into() }
    }
}

impl TokenStore for FileStore {
    fn load(&self) -> Option<Secret> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&self.path).ok()?.permissions().mode();
            if mode & 0o077 != 0 {
                return None;
            }
        }
        let text = std::fs::read_to_string(&self.path).ok()?;
        let text = text.trim();
        if text.is_empty() {
            None
        } else {
            Some(Secret::new(text))
        }
    }

    fn save(&self, token: &Secret) -> Result<(), String> {
        use std::io::Write;
        if let Some(dir) = self.path.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir).map_err(|e| format!("token file: {e}"))?;
            }
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&self.path)
            .map_err(|e| format!("token file: {e}"))?;
        file.write_all(token.expose().as_bytes())
            .map_err(|e| format!("token file: {e}"))
    }

    fn clear(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
