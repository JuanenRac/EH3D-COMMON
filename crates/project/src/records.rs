//! The records of a project: `project.json`, `branch.json` and the pieces they hold.
//!
//! Every record keeps the fields it does not know (`extra`), so a program of another version
//! that rewrites a record does not lose what a newer program wrote there.

use std::collections::BTreeMap;

use eh3d_container::{canonical_string, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::spec::{BRANCH_FILE, RECORD_SCHEMA};

fn schema_one() -> u32 {
    RECORD_SCHEMA
}

/// The hash of another branch that a branch was derived from: "I took the outline from
/// `pcb` as it was when its content hash was this one".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    /// The branch it was taken from (always a branch of the same file).
    pub branch: String,
    /// The content hash of that branch at the time (see [`branch_content_hash`]).
    pub hash: String,
    /// The format version (`major.minor`) of that branch at the time.
    pub format: String,
    /// The kind of artifact that was taken, for example `outline2d` or `mesh3d`.
    pub artifact: String,
    /// The version of the schema of that kind of artifact.
    pub artifact_version: u32,
    /// The names inside this branch that were derived from it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<String>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A read-only seal: the branch (or the project) is closed for delivery and nobody writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seal {
    /// Who sealed it (a name or an identifier of a program).
    pub by: String,
    /// When, as the caller wrote it (this crate never reads the clock).
    pub at: String,
    /// Why, in a few words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A signature slot. The key that made it is never in the file: `key_id` only names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureRecord {
    /// The algorithm, for example `ed25519`.
    pub algorithm: String,
    /// The identifier of the public key (a fingerprint or a name the verifier knows).
    pub key_id: String,
    /// The signature, in lowercase hexadecimal.
    pub value: String,
}

/// An encryption slot of a branch: the entries are ciphertext and the key is outside the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionRecord {
    /// The algorithm, for example `xchacha20-poly1305`.
    pub algorithm: String,
    /// The identifier of the key that opens it.
    pub key_id: String,
    /// What the algorithm needs besides the key (nonces, salts); never a secret.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub parameters: Value,
}

/// What `branch.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchRecord {
    /// The schema number of this record.
    #[serde(default = "schema_one")]
    pub schema: u32,
    /// The branch name; it must be the name of the folder.
    pub branch: String,
    /// The program that wrote the branch.
    pub app: String,
    /// The format version of the branch, `major.minor`, as its program defines it.
    pub format: String,
    /// The version of the program that wrote it.
    pub app_version: String,
    /// The content hash of the branch (see [`branch_content_hash`]).
    pub content_hash: String,
    /// The branches it was derived from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<Dependency>,
    /// Its seal, when it is sealed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seal: Option<Seal>,
    /// Its signature, when it is signed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignatureRecord>,
    /// Its encryption, when it is encrypted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EncryptionRecord>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One line of the index of branches that `project.json` keeps, so a program can list the
/// branches without opening each one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchIndexEntry {
    /// The branch name.
    pub id: String,
    /// The program that wrote it (empty when the branch has no valid record).
    pub app: String,
    /// The format version of the branch.
    pub format: String,
    /// The version of the program that wrote it.
    pub app_version: String,
    /// The content hash of the branch.
    pub content_hash: String,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// What `project.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    /// The schema number of this record.
    #[serde(default = "schema_one")]
    pub schema: u32,
    /// The version of the project format, `major.minor.patch`.
    pub format_version: String,
    /// The version of the rules of the container, `major.minor`.
    pub container_version: String,
    /// The identifier of the project; it survives renames and copies of the file.
    pub id: String,
    /// The name of the project.
    pub name: String,
    /// The unit of the lengths of the project (`millimetres`).
    pub units: String,
    /// When it was created, as the caller wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// When it was last written, as the caller wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// The index of the branches.
    #[serde(default)]
    pub branches: Vec<BranchIndexEntry>,
    /// The seal of the whole project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seal: Option<Seal>,
    /// The signature of the whole project (covers the hash of every branch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignatureRecord>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// The content hash of a branch: the SHA-256, in lowercase hexadecimal, of the canonical JSON
/// array of `[name, size, sha256]` for every file of the branch except `branch.json`, with the
/// names relative to the branch folder and sorted by their bytes. Two branches with the same
/// files have the same hash whatever their records say, and a record can state the hash of the
/// files it describes.
pub fn branch_content_hash<'a>(files: impl IntoIterator<Item = (&'a str, u64, &'a str)>) -> String {
    let mut rows: Vec<(&str, u64, &str)> = files
        .into_iter()
        .filter(|(name, _, _)| *name != BRANCH_FILE)
        .collect();
    rows.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let array = Value::Array(
        rows.into_iter()
            .map(|(name, size, sha)| {
                Value::Array(vec![Value::from(name), Value::from(size), Value::from(sha)])
            })
            .collect(),
    );
    // A value made of strings and integers is always representable.
    let text = canonical_string(&array).unwrap_or_default();
    sha256_hex(text.as_bytes())
}

/// The bytes that a signature of a record covers: a fixed label, then the canonical JSON of the
/// record without its `signature`.
pub fn signing_message(label: &str, record: &impl Serialize) -> Vec<u8> {
    try_signing_message(label, record).unwrap_or_else(|| format!("{label}\n").into_bytes())
}

/// [`signing_message`], or `None` for a record that has no canonical form (an integer that no
/// double equals, in a field of another program). Nothing is signed or accepted for such a
/// record: a message without it would be the same for every one of them.
pub fn try_signing_message(label: &str, record: &impl Serialize) -> Option<Vec<u8>> {
    let mut value = serde_json::to_value(record).ok()?;
    if let Value::Object(map) = &mut value {
        map.remove("signature");
    }
    let mut out = format!("{label}\n").into_bytes();
    out.extend_from_slice(canonical_string(&value).ok()?.as_bytes());
    Some(out)
}

/// The label of the message of a branch signature.
pub const BRANCH_SIGNATURE_LABEL: &str = "eh3d-branch-signature-v1";
/// The label of the message of a project signature.
pub const PROJECT_SIGNATURE_LABEL: &str = "eh3d-project-signature-v1";

/// Lowercase hexadecimal of `bytes`.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The bytes of a lowercase hexadecimal text, or `None` when it is not one.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    let digit = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    text.as_bytes()
        .chunks(2)
        .map(|pair| Some(digit(pair[0])? * 16 + digit(pair[1])?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_branch_hash_ignores_the_record_and_the_order() {
        let a = branch_content_hash([
            ("b.json", 1, "x"),
            ("a.json", 2, "y"),
            ("branch.json", 9, "z"),
        ]);
        let b = branch_content_hash([("a.json", 2, "y"), ("b.json", 1, "x")]);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        let c = branch_content_hash([("a.json", 2, "y"), ("b.json", 1, "w")]);
        assert_ne!(a, c);
        assert_eq!(branch_content_hash([]), sha256_hex(b"[]"));
    }

    #[test]
    fn hex_round_trips_and_refuses_what_is_not_hex() {
        assert_eq!(
            from_hex(&to_hex(&[0, 1, 254, 255])),
            Some(vec![0, 1, 254, 255])
        );
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("zz"), None);
        assert_eq!(from_hex("AB"), None);
    }

    #[test]
    fn unknown_fields_survive_a_round_trip() {
        let text = r#"{"branch":"pcb","app":"a","format":"1.0","app_version":"0.1.0","content_hash":"h","future":{"k":[1,2]}}"#;
        let record: BranchRecord = serde_json::from_str(text).unwrap();
        assert_eq!(record.schema, 1);
        assert!(record.extra.contains_key("future"));
        let back = serde_json::to_value(&record).unwrap();
        assert_eq!(back["future"], serde_json::json!({"k":[1,2]}));
    }

    #[test]
    fn the_signing_message_leaves_the_signature_out() {
        let mut record: BranchRecord = serde_json::from_str(
            r#"{"branch":"pcb","app":"a","format":"1.0","app_version":"0","content_hash":"h"}"#,
        )
        .unwrap();
        let unsigned = signing_message(BRANCH_SIGNATURE_LABEL, &record);
        record.signature = Some(SignatureRecord {
            algorithm: "x".into(),
            key_id: "k".into(),
            value: "00".into(),
        });
        assert_eq!(unsigned, signing_message(BRANCH_SIGNATURE_LABEL, &record));
        assert!(unsigned.starts_with(b"eh3d-branch-signature-v1\n"));
    }

    #[test]
    fn a_record_without_a_canonical_form_gives_no_message() {
        let record: BranchRecord = serde_json::from_str(
            r#"{"branch":"pcb","app":"a","format":"1.0","app_version":"0","content_hash":"h","n":18446744073709551615}"#,
        )
        .unwrap();
        assert_eq!(try_signing_message(BRANCH_SIGNATURE_LABEL, &record), None);
    }
}
