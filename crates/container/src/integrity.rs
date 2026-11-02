//! The integrity record of a container: the SHA-256 and the size of every entry other than
//! `integrity.json` itself.
//!
//! Integrity is not authenticity: whoever can change the entries can also rewrite the record.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::canonical::{canonical_string, CanonicalError};

/// The only algorithm the record may name.
pub const ALGORITHM: &str = "sha256";
/// Entries under this folder hold variable audit data and are left out of the content hash.
pub const AUDIT_ROOT: &str = "audit/";

/// The checksum record of one entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityEntry {
    /// The SHA-256 of the entry, in lowercase hexadecimal.
    pub sha256: String,
    /// The size of the entry in bytes.
    pub size: u64,
}

/// What `integrity.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integrity {
    /// Always `sha256`.
    pub algorithm: String,
    /// One record per entry other than `integrity.json` itself.
    pub files: BTreeMap<String, IntegrityEntry>,
}

/// Why an integrity check failed. The first problem found is reported, entries in name order.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IntegrityError {
    /// The record names an algorithm other than `sha256`.
    #[error("unknown algorithm '{0}'")]
    UnknownAlgorithm(String),
    /// The record lists an entry that the container does not have.
    #[error("the container has no entry '{0}'")]
    MissingEntry(String),
    /// The size of an entry differs from the record.
    #[error("the size of '{0}' differs from the record")]
    SizeDiffers(String),
    /// The SHA-256 of an entry differs from the record.
    #[error("the SHA-256 of '{0}' differs from the record")]
    HashDiffers(String),
    /// The container has an entry the record does not list.
    #[error("'{0}' is not listed in the integrity record")]
    NotListed(String),
}

/// The SHA-256 of `bytes` in lowercase hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

impl Integrity {
    /// The record for the given entries (`integrity.json` itself must not be among them).
    pub fn for_entries<'a>(entries: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Integrity {
        let files = entries
            .into_iter()
            .map(|(name, bytes)| {
                (
                    name.to_owned(),
                    IntegrityEntry {
                        sha256: sha256_hex(bytes),
                        size: bytes.len() as u64,
                    },
                )
            })
            .collect();
        Integrity {
            algorithm: ALGORITHM.to_owned(),
            files,
        }
    }

    /// Checks the record against the entries that were read (everything except
    /// `integrity.json`). The order of the checks is fixed: the algorithm, then each listed entry
    /// in name order (present, size, hash), then the entries that are not listed.
    pub fn verify(&self, entries: &BTreeMap<String, Vec<u8>>) -> Result<(), IntegrityError> {
        if self.algorithm != ALGORITHM {
            return Err(IntegrityError::UnknownAlgorithm(self.algorithm.clone()));
        }
        let mut seen = HashSet::new();
        for (name, record) in &self.files {
            let bytes = entries
                .get(name)
                .ok_or_else(|| IntegrityError::MissingEntry(name.clone()))?;
            if bytes.len() as u64 != record.size {
                return Err(IntegrityError::SizeDiffers(name.clone()));
            }
            if sha256_hex(bytes) != record.sha256 {
                return Err(IntegrityError::HashDiffers(name.clone()));
            }
            seen.insert(name.as_str());
        }
        if let Some(extra) = entries.keys().find(|name| !seen.contains(name.as_str())) {
            return Err(IntegrityError::NotListed(extra.clone()));
        }
        Ok(())
    }

    /// The content hash: SHA-256, in lowercase hexadecimal, of the canonical JSON array of
    /// `[name, size, sha256]` for every entry except those under `audit/`, sorted by the bytes
    /// of the name. Two containers with the same content hash have the same content even when
    /// their audit entries, and therefore their ZIP bytes, differ.
    pub fn content_hash(&self) -> Result<String, CanonicalError> {
        // The map is sorted by the UTF-8 bytes of the name, as the rule asks.
        let rows: Vec<Value> = self
            .files
            .iter()
            .filter(|(name, _)| !name.starts_with(AUDIT_ROOT))
            .map(|(name, e)| {
                Value::Array(vec![
                    Value::from(name.as_str()),
                    Value::from(e.size),
                    Value::from(e.sha256.as_str()),
                ])
            })
            .collect();
        let text = canonical_string(&Value::Array(rows))?;
        Ok(sha256_hex(text.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(list: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
        list.iter()
            .map(|(n, c)| ((*n).to_owned(), c.as_bytes().to_vec()))
            .collect()
    }

    fn record(map: &BTreeMap<String, Vec<u8>>) -> Integrity {
        Integrity::for_entries(map.iter().map(|(n, b)| (n.as_str(), b.as_slice())))
    }

    #[test]
    fn sha256_matches_the_standard_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_matching_record_verifies() {
        let map = entries(&[("a.txt", "one"), ("b/c.txt", "two")]);
        assert_eq!(record(&map).verify(&map), Ok(()));
        let empty = BTreeMap::new();
        assert_eq!(record(&empty).verify(&empty), Ok(()));
    }

    #[test]
    fn every_kind_of_mismatch_is_named() {
        let map = entries(&[("a.txt", "one"), ("b.txt", "two")]);
        let good = record(&map);

        let mut other_algorithm = good.clone();
        other_algorithm.algorithm = "md5".into();
        assert_eq!(
            other_algorithm.verify(&map),
            Err(IntegrityError::UnknownAlgorithm("md5".into()))
        );

        let mut missing = map.clone();
        missing.remove("b.txt");
        assert_eq!(
            good.verify(&missing),
            Err(IntegrityError::MissingEntry("b.txt".into()))
        );

        let mut longer = map.clone();
        longer.insert("a.txt".into(), b"onee".to_vec());
        assert_eq!(
            good.verify(&longer),
            Err(IntegrityError::SizeDiffers("a.txt".into()))
        );

        let mut changed = map.clone();
        changed.insert("a.txt".into(), b"ONE".to_vec());
        assert_eq!(
            good.verify(&changed),
            Err(IntegrityError::HashDiffers("a.txt".into()))
        );

        let mut extra = map.clone();
        extra.insert("z.txt".into(), b"x".to_vec());
        assert_eq!(
            good.verify(&extra),
            Err(IntegrityError::NotListed("z.txt".into()))
        );
    }

    #[test]
    fn the_record_has_the_json_form_of_the_container() {
        let map = entries(&[("a.txt", "abc")]);
        let json = serde_json::to_string(&record(&map)).unwrap();
        assert_eq!(
            json,
            r#"{"algorithm":"sha256","files":{"a.txt":{"sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","size":3}}}"#
        );
        assert_eq!(
            serde_json::from_str::<Integrity>(&json).unwrap(),
            record(&map)
        );
    }

    #[test]
    fn the_content_hash_ignores_audit_entries_only() {
        let base = entries(&[("design/a.json", "1"), ("project.json", "p")]);
        let mut with_audit_one = base.clone();
        with_audit_one.insert("audit/producer.json".into(), b"v1".to_vec());
        let mut with_audit_two = base.clone();
        with_audit_two.insert("audit/producer.json".into(), b"v2 and more".to_vec());
        with_audit_two.insert("audit/host.json".into(), b"h".to_vec());

        let h0 = record(&base).content_hash().unwrap();
        assert_eq!(h0.len(), 64);
        assert_eq!(h0, record(&with_audit_one).content_hash().unwrap());
        assert_eq!(h0, record(&with_audit_two).content_hash().unwrap());
        // The records themselves differ, so the integrity check still sees audit changes.
        assert_ne!(record(&with_audit_one), record(&with_audit_two));

        let mut changed = base.clone();
        changed.insert("design/a.json".into(), b"2".to_vec());
        assert_ne!(h0, record(&changed).content_hash().unwrap());
        let mut renamed = base.clone();
        let v = renamed.remove("design/a.json").unwrap();
        renamed.insert("design/b.json".into(), v);
        assert_ne!(h0, record(&renamed).content_hash().unwrap());
        // A folder that merely starts like the audit folder is content.
        let mut lookalike = base.clone();
        lookalike.insert("auditor/x".into(), b"x".to_vec());
        assert_ne!(h0, record(&lookalike).content_hash().unwrap());
    }

    #[test]
    fn the_content_hash_is_the_hash_of_the_documented_text() {
        let map = entries(&[("b", "2"), ("a", "1")]);
        let r = record(&map);
        let text = format!(
            "[[\"a\",1,\"{}\"],[\"b\",1,\"{}\"]]",
            sha256_hex(b"1"),
            sha256_hex(b"2")
        );
        assert_eq!(r.content_hash().unwrap(), sha256_hex(text.as_bytes()));
        // No entries: the hash of the empty array.
        let empty = Integrity::for_entries(std::iter::empty());
        assert_eq!(empty.content_hash().unwrap(), sha256_hex(b"[]"));
    }
}
