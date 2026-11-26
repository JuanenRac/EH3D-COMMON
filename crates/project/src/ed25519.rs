//! Ed25519 (RFC 8032) signing and checking of branches and projects, with the crate
//! `ed25519-dalek`.
//!
//! **Keys never live in a project.** A key is a small text file whose path the person gives:
//!
//! ```text
//! eh3d-ed25519-secret-v1          eh3d-ed25519-public-v1
//! <64 lowercase hex digits>       <64 lowercase hex digits>
//! ```
//!
//! The secret file holds the 32 byte seed; the public file (the same path plus `.pub`) holds
//! the public key and is what is handed to those who check. A new secret file never replaces one
//! and is readable only by its owner (mode 600 on Unix; on Windows the inherited permissions are
//! removed and only the owner is granted, with the system tool `icacls`). No message of this
//! module carries what a key file holds, the secret key is wiped from memory when it is dropped
//! and `Debug` shows only the identifier of the key.
//!
//! The identifier of a key, written in a signature record, is `ed25519:` and the first 32 hex
//! digits of the SHA-256 of the public key.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use eh3d_container::sha256_hex;
use thiserror::Error;
use zeroize::Zeroizing;

use crate::security::{Signer, Verifier, VerifyOutcome};

/// The algorithm name written in a signature record.
pub const ALGORITHM: &str = "ed25519";
/// The first line of a secret key file.
pub const SECRET_KEY_HEADER: &str = "eh3d-ed25519-secret-v1";
/// The first line of a public key file.
pub const PUBLIC_KEY_HEADER: &str = "eh3d-ed25519-public-v1";
/// What is added to the path of a secret key file to name its public key file.
pub const PUBLIC_KEY_SUFFIX: &str = ".pub";
/// A key file is never larger than this.
const MAX_KEY_FILE: usize = 256;

/// Why a key could not be made, read or used. No variant carries what a key file holds.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum KeyError {
    /// The file system failed.
    #[error("{}: {error}", path.display())]
    Io {
        /// The file concerned.
        path: PathBuf,
        /// What the system said.
        error: io::Error,
    },
    /// The file is there already; a key is never replaced.
    #[error("{}: the file already exists and is not replaced", .0.display())]
    AlreadyExists(PathBuf),
    /// The file is not a key file of this format.
    #[error("{}: not an Ed25519 key file of this program", .0.display())]
    NotAKeyFile(PathBuf),
    /// A public key was given where the secret key is needed.
    #[error("{}: this is a public key; signing needs the secret key", .0.display())]
    PublicKeyGiven(PathBuf),
    /// A secret key was given where the public key is needed.
    #[error("{}: this is a secret key; give the public key (.pub) to check signatures", .0.display())]
    SecretKeyGiven(PathBuf),
    /// The 32 bytes are not a usable public key (not a point of the curve, or of small order).
    #[error("the public key is weak or not a point of the curve")]
    BadPublicKey,
    /// The name asked for the key is the name of a project or package.
    #[error("{}: a key file is never a project or a package", .0.display())]
    InsideProject(PathBuf),
    /// The system gave no random bytes.
    #[error("the system gave no random bytes")]
    Random,
    /// The access to the new secret key file could not be limited to its owner.
    #[error("{}: the access to the key file could not be limited to its owner", .0.display())]
    Permissions(PathBuf),
}

/// The files and the identifier of a key that was just made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPair {
    /// The identifier that signature records will carry.
    pub key_id: String,
    /// The secret key file.
    pub secret_path: PathBuf,
    /// The public key file.
    pub public_path: PathBuf,
}

/// The identifier of a public key.
pub fn key_id_of(public: &[u8; 32]) -> String {
    let digest = sha256_hex(public);
    format!("{ALGORITHM}:{}", digest.get(..32).unwrap_or(&digest))
}

/// The path of the public key file that goes with a secret key file.
pub fn public_key_path(secret: &Path) -> PathBuf {
    let mut name = secret.as_os_str().to_owned();
    name.push(PUBLIC_KEY_SUFFIX);
    PathBuf::from(name)
}

/// True when `bytes` start like a secret key file; such bytes are never put into a project.
pub fn looks_like_secret_key(bytes: &[u8]) -> bool {
    bytes
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap_or(bytes)
        .starts_with(SECRET_KEY_HEADER.as_bytes())
}

/// Signs with a secret key held in memory.
pub struct Ed25519Signer {
    key: SigningKey,
    key_id: String,
}

impl fmt::Debug for Ed25519Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ed25519Signer")
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

impl Ed25519Signer {
    /// A signer from the 32 byte seed of a secret key (RFC 8032, "secret key").
    pub fn from_seed(seed: &[u8; 32]) -> Ed25519Signer {
        let key = SigningKey::from_bytes(seed);
        let key_id = key_id_of(&key.verifying_key().to_bytes());
        Ed25519Signer { key, key_id }
    }

    /// Reads the secret key file at `path`.
    pub fn read(path: &Path) -> Result<Ed25519Signer, KeyError> {
        match read_key_file(path)? {
            (KeyKind::Secret, seed) => Ok(Ed25519Signer::from_seed(&seed)),
            (KeyKind::Public, _) => Err(KeyError::PublicKeyGiven(path.to_owned())),
        }
    }

    /// The public key.
    pub fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
}

impl Signer for Ed25519Signer {
    fn algorithm(&self) -> &str {
        ALGORITHM
    }

    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, message: &[u8]) -> Vec<u8> {
        ed25519_dalek::Signer::sign(&self.key, message)
            .to_bytes()
            .to_vec()
    }
}

/// The public keys a program trusts, by identifier. Signatures are checked with the strict
/// rules (no small order points, no second encoding of a signature).
#[derive(Debug, Default, Clone)]
pub struct Ed25519Keyring {
    keys: BTreeMap<String, VerifyingKey>,
}

impl Ed25519Keyring {
    /// An empty keyring.
    pub fn new() -> Ed25519Keyring {
        Ed25519Keyring::default()
    }

    /// Trusts a public key and returns its identifier.
    pub fn add(&mut self, public: &[u8; 32]) -> Result<String, KeyError> {
        let key = VerifyingKey::from_bytes(public).map_err(|_| KeyError::BadPublicKey)?;
        if key.is_weak() {
            return Err(KeyError::BadPublicKey);
        }
        let id = key_id_of(public);
        self.keys.insert(id.clone(), key);
        Ok(id)
    }

    /// Trusts the public key of the file at `path` and returns its identifier.
    pub fn read_file(&mut self, path: &Path) -> Result<String, KeyError> {
        match read_key_file(path)? {
            (KeyKind::Public, bytes) => self.add(&bytes),
            (KeyKind::Secret, _) => Err(KeyError::SecretKeyGiven(path.to_owned())),
        }
    }

    /// How many keys are trusted.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// True when no key is trusted.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl Verifier for Ed25519Keyring {
    fn verify(
        &self,
        algorithm: &str,
        key_id: &str,
        message: &[u8],
        signature: &[u8],
    ) -> VerifyOutcome {
        if algorithm != ALGORITHM {
            return VerifyOutcome::UnsupportedAlgorithm;
        }
        let Some(key) = self.keys.get(key_id) else {
            return VerifyOutcome::UnknownKey;
        };
        let Ok(signature) = Signature::from_slice(signature) else {
            return VerifyOutcome::Invalid;
        };
        match key.verify_strict(message, &signature) {
            Ok(()) => VerifyOutcome::Valid,
            Err(_) => VerifyOutcome::Invalid,
        }
    }
}

/// Makes a new key from the random source of the system and writes the two files. Neither file
/// replaces one that is there.
pub fn generate_key_files(secret_path: &Path) -> Result<KeyPair, KeyError> {
    let extension = secret_path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    if matches!(extension.as_deref(), Some("eh3d" | "eh3dproj")) {
        return Err(KeyError::InsideProject(secret_path.to_owned()));
    }
    let public_path = public_key_path(secret_path);
    for path in [secret_path, public_path.as_path()] {
        // a link that points nowhere counts as a file that is there
        if path.symlink_metadata().is_ok() {
            return Err(KeyError::AlreadyExists(path.to_owned()));
        }
    }
    let mut seed = Zeroizing::new([0u8; 32]);
    getrandom::fill(seed.as_mut_slice()).map_err(|_| KeyError::Random)?;
    let signer = Ed25519Signer::from_seed(&seed);
    write_new(secret_path, &key_text(SECRET_KEY_HEADER, &seed), true)?;
    let public = key_text(PUBLIC_KEY_HEADER, &signer.public_key());
    if let Err(e) = write_new(&public_path, &public, false) {
        // the secret file is the one made a moment ago
        let _ = fs::remove_file(secret_path);
        return Err(e);
    }
    Ok(KeyPair {
        key_id: signer.key_id.clone(),
        secret_path: secret_path.to_owned(),
        public_path,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    Secret,
    Public,
}

/// The two lines of a key file, in one buffer that is wiped when it is dropped.
fn key_text(header: &str, bytes: &[u8; 32]) -> Zeroizing<Vec<u8>> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = Zeroizing::new(Vec::with_capacity(header.len() + 66));
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');
    for b in bytes {
        out.push(DIGITS[usize::from(b >> 4)]);
        out.push(DIGITS[usize::from(b & 0x0f)]);
    }
    out.push(b'\n');
    out
}

fn parse_key(bytes: &[u8]) -> Option<(KeyKind, Zeroizing<[u8; 32]>)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    let kind = match lines.next()? {
        SECRET_KEY_HEADER => KeyKind::Secret,
        PUBLIC_KEY_HEADER => KeyKind::Public,
        _ => return None,
    };
    let digits = lines.next()?.as_bytes();
    if digits.len() != 64 || lines.next().is_some() {
        return None;
    }
    let nibble = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    let mut out = Zeroizing::new([0u8; 32]);
    for (slot, pair) in out.iter_mut().zip(digits.as_chunks::<2>().0) {
        *slot = nibble(pair[0])? * 16 + nibble(pair[1])?;
    }
    Some((kind, out))
}

fn read_key_file(path: &Path) -> Result<(KeyKind, Zeroizing<[u8; 32]>), KeyError> {
    let io_error = |error| KeyError::Io {
        path: path.to_owned(),
        error,
    };
    let file = File::open(path).map_err(io_error)?;
    // the buffer never grows, so no copy of the key is left behind
    let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_KEY_FILE + 1));
    file.take(MAX_KEY_FILE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > MAX_KEY_FILE {
        return Err(KeyError::NotAKeyFile(path.to_owned()));
    }
    parse_key(&bytes).ok_or_else(|| KeyError::NotAKeyFile(path.to_owned()))
}

/// Creates `path` (never over a file) and writes `bytes`; a `private` file is limited to its
/// owner before anything is written in it.
fn write_new(path: &Path, bytes: &[u8], private: bool) -> Result<(), KeyError> {
    let io_error = |error: io::Error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            KeyError::AlreadyExists(path.to_owned())
        } else {
            KeyError::Io {
                path: path.to_owned(),
                error,
            }
        }
    };
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if private {
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    }
    let mut file = options.open(path).map_err(io_error)?;
    let limited = if private {
        limit_to_owner(path)
    } else {
        Ok(())
    };
    let written = limited.and_then(|()| {
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(io_error)
    });
    if written.is_err() {
        drop(file);
        let _ = fs::remove_file(path);
    }
    written
}

#[cfg(unix)]
fn limit_to_owner(path: &Path) -> Result<(), KeyError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| KeyError::Permissions(path.to_owned()))
}

#[cfg(windows)]
fn limit_to_owner(path: &Path) -> Result<(), KeyError> {
    use std::os::windows::process::CommandExt;
    // no console window flashes when a program with a window makes a key
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let refused = || KeyError::Permissions(path.to_owned());
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let tool = PathBuf::from(root).join("System32").join("icacls.exe");
    // an absolute path cannot be taken for an option of the tool
    let full = std::path::absolute(path).map_err(|_| refused())?;
    // S-1-3-4 is "owner rights": what the owner of the file may do, whoever that is
    let output = std::process::Command::new(tool)
        .arg(full)
        .args(["/inheritance:r", "/grant:r", "*S-1-3-4:F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|_| refused())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(refused())
    }
}

#[cfg(not(any(unix, windows)))]
fn limit_to_owner(_path: &Path) -> Result<(), KeyError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_text_reads_back_with_either_line_ending() {
        let bytes: [u8; 32] = std::array::from_fn(|i| i as u8 * 7);
        let text = key_text(PUBLIC_KEY_HEADER, &bytes);
        let (kind, read) = parse_key(&text).unwrap();
        assert_eq!((kind, *read), (KeyKind::Public, bytes));
        let windows = String::from_utf8(text.to_vec())
            .unwrap()
            .replace('\n', "\r\n");
        assert_eq!(*parse_key(windows.as_bytes()).unwrap().1, bytes);
        let without_end = &text[..text.len() - 1];
        assert_eq!(*parse_key(without_end).unwrap().1, bytes);
        assert!(parse_key(&text[..text.len() - 2]).is_none());
        let mut more = text.to_vec();
        more.extend_from_slice(b"more\n");
        assert!(parse_key(&more).is_none());
    }

    #[test]
    fn the_start_of_a_secret_key_is_recognised() {
        assert!(looks_like_secret_key(b"eh3d-ed25519-secret-v1\nabc"));
        assert!(looks_like_secret_key(
            b"\xef\xbb\xbfeh3d-ed25519-secret-v1\r\n"
        ));
        assert!(!looks_like_secret_key(b"eh3d-ed25519-public-v1\n"));
        assert!(!looks_like_secret_key(
            b"# about eh3d-ed25519-secret-v1 files"
        ));
    }
}
