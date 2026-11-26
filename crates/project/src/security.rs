//! The points where the optional security of a project plugs in.
//!
//! Integrity (the SHA-256 of every entry and of every branch) is always there. Signing is
//! optional: the file carries a signature record and the name of the key, never the key. This
//! module has the two traits of signing and checking and what is known about a signature; the
//! algorithm of the suite, Ed25519, is in [`crate::ed25519`]. A program may implement the traits
//! with other cryptography without changing the file format.
//!
//! Encryption of a branch and the sealing of a branch are designed in `docs/FORMAT_EH3D.md`.
//! A sealed branch is refused for writing (`EH3D-PKG-020`) and an encrypted branch is refused
//! for reading when no key is at hand (`EH3D-PKG-023`); the encryption itself is pending.

use std::collections::BTreeMap;
use std::fmt;

use crate::records::SignatureRecord;

/// What a verifier says about one signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// The signature is correct for that message and key.
    Valid,
    /// The signature is wrong.
    Invalid,
    /// The key is not one the verifier knows.
    UnknownKey,
    /// The algorithm is not one the verifier supports.
    UnsupportedAlgorithm,
}

/// Checks signatures. The public keys live outside the file, wherever the program keeps them.
pub trait Verifier {
    /// Checks `signature` over `message` made with the key called `key_id`.
    fn verify(
        &self,
        algorithm: &str,
        key_id: &str,
        message: &[u8],
        signature: &[u8],
    ) -> VerifyOutcome;
}

/// Makes signatures. The private key never enters the file nor this crate.
pub trait Signer {
    /// The algorithm name written in the record.
    fn algorithm(&self) -> &str;
    /// The identifier of the key written in the record.
    fn key_id(&self) -> &str;
    /// Signs `message`.
    fn sign(&self, message: &[u8]) -> Vec<u8>;
}

/// What is known about the signature of a branch or of the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureStatus {
    /// There is no signature.
    Unsigned,
    /// There is a signature and no verifier was given, so it was not checked.
    Unchecked,
    /// The signature is correct; the key that made it.
    Valid(String),
    /// The signature is wrong.
    Invalid,
    /// The key, or the algorithm, is not known to the verifier.
    UnknownKey,
}

impl fmt::Display for SignatureStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignatureStatus::Unsigned => f.write_str("unsigned"),
            SignatureStatus::Unchecked => f.write_str("signed, not checked"),
            SignatureStatus::Valid(key) => write!(f, "signed, valid (key {key})"),
            SignatureStatus::Invalid => f.write_str("INVALID signature"),
            SignatureStatus::UnknownKey => f.write_str("signed, unknown key"),
        }
    }
}

/// The state of every signature of a project, as [`crate::Project::signatures`] finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureReport {
    /// The signature of the project record.
    pub project: SignatureStatus,
    /// The signature of each branch (extension folders have no record and are not listed).
    pub branches: BTreeMap<String, SignatureStatus>,
}

impl SignatureReport {
    /// True when any signature is wrong.
    pub fn any_invalid(&self) -> bool {
        std::iter::once(&self.project)
            .chain(self.branches.values())
            .any(|s| *s == SignatureStatus::Invalid)
    }
}

/// Judges the signature record of a branch or of the project against `message`.
pub fn status_of(
    record: Option<&SignatureRecord>,
    message: &[u8],
    verifier: Option<&dyn Verifier>,
) -> SignatureStatus {
    let Some(sig) = record else {
        return SignatureStatus::Unsigned;
    };
    let Some(verifier) = verifier else {
        return SignatureStatus::Unchecked;
    };
    let Some(bytes) = crate::records::from_hex(&sig.value) else {
        return SignatureStatus::Invalid;
    };
    match verifier.verify(&sig.algorithm, &sig.key_id, message, &bytes) {
        VerifyOutcome::Valid => SignatureStatus::Valid(sig.key_id.clone()),
        VerifyOutcome::Invalid => SignatureStatus::Invalid,
        VerifyOutcome::UnknownKey | VerifyOutcome::UnsupportedAlgorithm => {
            SignatureStatus::UnknownKey
        }
    }
}

/// Makes the signature record of `message` with `signer`.
pub fn sign_with(signer: &dyn Signer, message: &[u8]) -> SignatureRecord {
    SignatureRecord {
        algorithm: signer.algorithm().to_owned(),
        key_id: signer.key_id().to_owned(),
        value: crate::records::to_hex(&signer.sign(message)),
    }
}
