//! A secret signature key kept under a password: the text of the key file and how it is opened.
//!
//! ```text
//! eh3d-ed25519-locked-v1
//! argon2id-v19 m=65536 t=3 p=1
//! public <64 lowercase hex digits>
//! salt <32 lowercase hex digits>
//! nonce <48 lowercase hex digits>
//! sealed <96 lowercase hex digits>
//! ```
//!
//! The password is turned into a 32 byte key with **Argon2id** (RFC 9106, version 0x13) over the
//! salt, with the memory (KiB), passes and lanes of the second line. The 32 byte seed of the
//! signature key is sealed with **XChaCha20-Poly1305** under that key and the nonce; `sealed` is
//! the ciphertext and then its 16 byte tag. The five lines above it, each ended by a line feed,
//! are authenticated with it, so none of them can be changed. The public key is in clear (it is
//! no secret): a program can say which key it is before it asks for the password.
//!
//! The password, the derived key and the seed are wiped from memory, nothing of them is printed
//! and no error says what they hold.

use std::fmt;

use argon2::{Algorithm, Argon2, Block, Params, Version};
use chacha20poly1305::{aead::AeadInOut, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use zeroize::{Zeroize, Zeroizing};

use crate::records::{from_hex, to_hex};

/// The first line of a secret key file kept under a password.
pub const LOCKED_KEY_HEADER: &str = "eh3d-ed25519-locked-v1";
/// The fewest characters of a new password.
pub const MIN_PASSWORD_CHARS: usize = 8;
/// The most bytes of a password.
pub const MAX_PASSWORD_BYTES: usize = 1024;
/// The most memory a key file may ask for, in KiB (1 GiB).
const MAX_MEMORY_KIB: u32 = 1024 * 1024;
/// The most passes and lanes a key file may ask for.
const MAX_PASSES: u32 = 16;
const MAX_LANES: u32 = 16;
/// The most memory times passes a key file may ask for, in KiB (4 GiB).
const MAX_WORK_KIB: u64 = 4 * 1024 * 1024;

/// A password of a key file. It is wiped from memory when it is dropped and `Debug` shows
/// nothing of it.
pub struct Password(Zeroizing<String>);

impl Password {
    /// Takes the text of a password; it is not copied.
    pub fn new(text: String) -> Password {
        Password(Zeroizing::new(text))
    }

    /// Whether it may protect a new key file: at least [`MIN_PASSWORD_CHARS`] characters and at
    /// most [`MAX_PASSWORD_BYTES`] bytes.
    pub fn is_long_enough(&self) -> bool {
        self.0.chars().count() >= MIN_PASSWORD_CHARS && self.0.len() <= MAX_PASSWORD_BYTES
    }

    /// Whether two passwords are the same, for a program that asks for it twice.
    pub fn same_as(&self, other: &Password) -> bool {
        self.0.as_str() == other.0.as_str()
    }

    fn bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(..)")
    }
}

/// The cost of Argon2id: memory in KiB, passes and lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    /// The memory, in KiB.
    pub memory_kib: u32,
    /// The passes over it.
    pub passes: u32,
    /// The lanes.
    pub lanes: u32,
}

impl Default for KdfParams {
    /// 64 MiB, 3 passes, 1 lane: above the least that is recommended for passwords today.
    fn default() -> Self {
        KdfParams {
            memory_kib: 64 * 1024,
            passes: 3,
            lanes: 1,
        }
    }
}

impl KdfParams {
    /// Whether a key file may ask for this cost: within what Argon2 allows and not so large that
    /// a made-up file could exhaust the computer.
    pub fn is_allowed(&self) -> bool {
        (1..=MAX_LANES).contains(&self.lanes)
            && (1..=MAX_PASSES).contains(&self.passes)
            && (8 * self.lanes..=MAX_MEMORY_KIB).contains(&self.memory_kib)
            && u64::from(self.memory_kib) * u64::from(self.passes) <= MAX_WORK_KIB
    }
}

/// What a locked key file holds, none of it secret by itself.
pub(crate) struct Locked {
    pub(crate) params: KdfParams,
    pub(crate) public: [u8; 32],
    salt: [u8; 16],
    nonce: [u8; 24],
    sealed: [u8; 48],
}

/// Why a locked key could not be made or opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockError {
    /// Not enough memory, or a cost that is not allowed.
    Derive,
    /// The password is not the one, or the file was changed.
    Open,
}

fn derive(
    password: &Password,
    salt: &[u8; 16],
    params: KdfParams,
) -> Result<Zeroizing<[u8; 32]>, LockError> {
    if !params.is_allowed() {
        return Err(LockError::Derive);
    }
    let cost = Params::new(params.memory_kib, params.passes, params.lanes, Some(32))
        .map_err(|_| LockError::Derive)?;
    let count = cost.block_count();
    let mut blocks: Vec<Block> = Vec::new();
    blocks
        .try_reserve_exact(count)
        .map_err(|_| LockError::Derive)?;
    blocks.resize(count, Block::new());
    let mut key = Zeroizing::new([0u8; 32]);
    let made = Argon2::new(Algorithm::Argon2id, Version::V0x13, cost)
        .hash_password_into_with_memory(
            password.bytes(),
            salt,
            key.as_mut_slice(),
            blocks.as_mut_slice(),
        );
    blocks.iter_mut().for_each(Zeroize::zeroize);
    made.map_err(|_| LockError::Derive)?;
    Ok(key)
}

impl Locked {
    /// The lines that are authenticated with the sealed seed.
    fn authenticated(&self) -> String {
        format!(
            "{LOCKED_KEY_HEADER}\nargon2id-v19 m={} t={} p={}\npublic {}\nsalt {}\nnonce {}\n",
            self.params.memory_kib,
            self.params.passes,
            self.params.lanes,
            to_hex(&self.public),
            to_hex(&self.salt),
            to_hex(&self.nonce),
        )
    }

    /// The whole text of the file.
    pub(crate) fn text(&self) -> String {
        format!("{}sealed {}\n", self.authenticated(), to_hex(&self.sealed))
    }

    /// Reads the text of a locked key file; `None` when it is not one, or asks for a cost that
    /// is not allowed. Either line ending is read.
    pub(crate) fn parse(bytes: &[u8]) -> Option<Locked> {
        let mut lines = std::str::from_utf8(bytes).ok()?.lines();
        if lines.next()? != LOCKED_KEY_HEADER {
            return None;
        }
        let params = parse_cost(lines.next()?)?;
        let mut field = |name: &str| {
            let text = lines.next()?.strip_prefix(name)?.strip_prefix(' ')?;
            if text.bytes().any(|b| b.is_ascii_uppercase()) {
                return None;
            }
            from_hex(text)
        };
        let public = field("public")?.try_into().ok()?;
        let salt = field("salt")?.try_into().ok()?;
        let nonce = field("nonce")?.try_into().ok()?;
        let sealed = field("sealed")?.try_into().ok()?;
        if lines.next().is_some() {
            return None;
        }
        Some(Locked {
            params,
            public,
            salt,
            nonce,
            sealed,
        })
    }

    /// Seals the seed of the public key `public` under a password, with the salt and the nonce
    /// given (both random for a real file).
    pub(crate) fn seal(
        seed: &[u8; 32],
        public: [u8; 32],
        password: &Password,
        params: KdfParams,
        salt: [u8; 16],
        nonce: [u8; 24],
    ) -> Result<Locked, LockError> {
        let key = derive(password, &salt, params)?;
        let mut locked = Locked {
            params,
            public,
            salt,
            nonce,
            sealed: [0; 48],
        };
        let aad = locked.authenticated();
        let cipher = XChaCha20Poly1305::new((&*key).into());
        let mut buffer = Zeroizing::new(*seed);
        let tag = cipher
            .encrypt_inout_detached(
                <&XNonce>::from(&locked.nonce),
                aad.as_bytes(),
                buffer.as_mut_slice().into(),
            )
            .map_err(|_| LockError::Derive)?;
        locked.sealed[..32].copy_from_slice(buffer.as_slice());
        locked.sealed[32..].copy_from_slice(tag.as_slice());
        Ok(locked)
    }

    /// The seed, with the password it was sealed under.
    pub(crate) fn open(&self, password: &Password) -> Result<Zeroizing<[u8; 32]>, LockError> {
        let key = derive(password, &self.salt, self.params)?;
        let cipher = XChaCha20Poly1305::new((&*key).into());
        let mut seed = Zeroizing::new([0u8; 32]);
        seed.copy_from_slice(&self.sealed[..32]);
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&self.sealed[32..]);
        cipher
            .decrypt_inout_detached(
                <&XNonce>::from(&self.nonce),
                self.authenticated().as_bytes(),
                seed.as_mut_slice().into(),
                <&Tag>::from(&tag),
            )
            .map_err(|_| LockError::Open)?;
        Ok(seed)
    }
}

fn parse_cost(line: &str) -> Option<KdfParams> {
    let mut words = line.split(' ');
    if words.next()? != "argon2id-v19" {
        return None;
    }
    let mut number = |name: &str| -> Option<u32> {
        let digits = words.next()?.strip_prefix(name)?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok()
    };
    let params = KdfParams {
        memory_kib: number("m=")?,
        passes: number("t=")?,
        lanes: number("p=")?,
    };
    (words.next().is_none() && params.is_allowed()).then_some(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cheap() -> KdfParams {
        KdfParams {
            memory_kib: 64,
            passes: 1,
            lanes: 1,
        }
    }

    fn words(n: usize) -> Password {
        Password::new(format!("made up words {n}"))
    }

    #[test]
    fn a_sealed_seed_opens_with_its_password_and_with_no_other() {
        let seed: [u8; 32] = std::array::from_fn(|i| i as u8);
        let public = [7u8; 32];
        let locked = Locked::seal(&seed, public, &words(1), cheap(), [1; 16], [2; 24]).unwrap();
        let text = locked.text();
        assert!(!text.contains(&to_hex(&seed)));
        assert_eq!(text.lines().count(), 6);
        for read in [text.clone(), text.replace('\n', "\r\n")] {
            let back = Locked::parse(read.as_bytes()).unwrap();
            assert_eq!((back.params, back.public), (cheap(), public));
            assert_eq!(*back.open(&words(1)).unwrap(), seed);
            assert_eq!(back.open(&words(2)).unwrap_err(), LockError::Open);
        }
    }

    #[test]
    fn no_line_of_the_file_can_be_changed() {
        let seed = [9u8; 32];
        let locked = Locked::seal(&seed, [7; 32], &words(1), cheap(), [1; 16], [2; 24]).unwrap();
        let text = locked.text();
        for (from, to) in [
            ("m=64", "m=72"),
            ("t=1", "t=2"),
            ("public 07", "public 08"),
            ("salt 01", "salt 02"),
            ("nonce 02", "nonce 03"),
        ] {
            let changed = text.replacen(from, to, 1);
            assert_ne!(changed, text, "{from}");
            let read = Locked::parse(changed.as_bytes()).unwrap();
            assert_eq!(read.open(&words(1)).unwrap_err(), LockError::Open, "{from}");
        }
    }

    #[test]
    fn a_damaged_or_greedy_file_is_not_read() {
        let locked = Locked::seal(&[9; 32], [7; 32], &words(1), cheap(), [1; 16], [2; 24]).unwrap();
        let text = locked.text();
        for bad in [
            text.replace("m=64", "m=4294967295"),
            text.replace("m=64", "m=1"),
            text.replace("t=1", "t=0"),
            text.replace("t=1", "t=999"),
            text.replace("p=1", "p=+1"),
            text.replace("argon2id-v19", "argon2i-v19"),
            text.replace("salt ", "salt  "),
            format!("{text}more\n"),
            text[..text.len() - 4].to_owned(),
            text.replace("locked-v1", "locked-v2"),
        ] {
            assert!(Locked::parse(bad.as_bytes()).is_none(), "{bad}");
        }
    }

    #[test]
    fn a_password_shows_nothing_and_is_long_enough_from_eight_characters() {
        assert_eq!(format!("{:?}", words(3)), "Password(..)");
        assert!(!Password::new("seven77".into()).is_long_enough());
        assert!(Password::new("eight888".into()).is_long_enough());
        assert!(Password::new("ñññññññ8".into()).is_long_enough());
        assert!(!Password::new("a".repeat(MAX_PASSWORD_BYTES + 1)).is_long_enough());
        assert!(words(1).same_as(&words(1)) && !words(1).same_as(&words(2)));
        assert!(KdfParams::default().is_allowed());
    }
}
