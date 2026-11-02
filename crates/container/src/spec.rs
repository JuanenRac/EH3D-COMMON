//! The parameters that tell one container format from another, and the entry-name rule.
//!
//! A spec describes what a format allows; it does not read or write anything. The preset
//! [`ContainerSpec::eh3dproj_v1`] records `.eh3dproj` 1.0.0 as it behaves today, so a test can
//! check that the code and the description agree.

use std::collections::BTreeMap;

use thiserror::Error;

/// Longest entry name in bytes that the 1.0.0 rule accepts.
pub const MAX_NAME_BYTES: usize = 240;
/// The metadata entry.
pub const PROJECT_ENTRY: &str = "project.json";
/// The integrity entry.
pub const INTEGRITY_ENTRY: &str = "integrity.json";

/// A name that cannot be used as an entry of a container.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NameError {
    /// Empty, absolute, too long, or made of parts that could leave the container folder.
    #[error("unsafe entry name '{0}'")]
    Unsafe(String),
}

/// Checks that `name` is a portable path inside a container: relative, forward slashes only, no
/// `.`/`..` parts, no empty parts, no drive letters, no control characters, at most 240 bytes.
/// This is the rule of `.eh3dproj` 1.0.0; a stricter rule for new presets is not decided yet.
pub fn check_entry_name(name: &str) -> Result<(), NameError> {
    let unsafe_name = || Err(NameError::Unsafe(name.to_owned()));
    if name.is_empty()
        || name.len() > MAX_NAME_BYTES
        || name.starts_with('/')
        || name.ends_with('/')
    {
        return unsafe_name();
    }
    if name.contains('\\') || name.contains(':') || name.chars().any(char::is_control) {
        return unsafe_name();
    }
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return unsafe_name();
    }
    Ok(())
}

/// A compression method a format allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    /// No compression.
    Stored,
    /// Deflate.
    Deflate,
}

/// How the JSON entries of a format are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonForm {
    /// Whatever `.eh3dproj` 1.0.0 writes today (pretty-printed by the old serialiser). Fixed:
    /// changing it would break the reference packages.
    LegacyV1,
    /// Canonical JSON, RFC 8785, plus one final line feed.
    Jcs,
}

/// The order of the entries inside the archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryOrder {
    /// `project.json`, `integrity.json`, then the data entries by name.
    LegacyV1,
    /// The data entries sorted by the bytes of their names, then `project.json`, then
    /// `integrity.json` last, so a streaming writer can finish it after every hash is known.
    SortedThenProjectThenIntegrity,
}

/// What a reader does with a version whose minor number is newer than the one it knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewerMinor {
    /// Read it and ignore what is not understood (what `.eh3dproj` 1.0.0 does today).
    Accept,
    /// Refuse it with a message about the version (the policy for new presets).
    Refuse,
}

/// The size limits of a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// The most entries.
    pub max_entries: usize,
    /// The longest entry name in bytes.
    pub max_name_bytes: usize,
    /// The most bytes in all entries once decompressed.
    pub max_total_bytes: u64,
    /// The most bytes of one entry once decompressed, unless its root has its own value.
    pub default_max_entry_bytes: u64,
    /// Entry limits per root folder (for example `"results/"`), overriding the default.
    pub per_root_max_entry_bytes: BTreeMap<String, u64>,
}

impl Limits {
    /// The entry limit that applies to `name`: the one of its root folder, or the default.
    pub fn max_entry_bytes_for(&self, name: &str) -> u64 {
        self.per_root_max_entry_bytes
            .iter()
            .filter(|(root, _)| name.starts_with(root.as_str()))
            .max_by_key(|(root, _)| root.len())
            .map_or(self.default_max_entry_bytes, |(_, limit)| *limit)
    }
}

/// What one container format allows. The time and memory budget of a decompression is not a
/// parameter yet: its values are still to be set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerSpec {
    /// The short name shown in messages.
    pub format_id: String,
    /// Entries that must exist.
    pub required_entries: Vec<String>,
    /// Top-level folders (with the trailing slash) where data entries may live.
    pub allowed_roots: Vec<String>,
    /// Prefixes of folders that are preserved and never interpreted (for example `x-`).
    pub extension_roots: Vec<String>,
    /// The size limits.
    pub limits: Limits,
    /// Whether ZIP64 records are accepted and written.
    pub allow_zip64: bool,
    /// The compression methods allowed.
    pub methods: Vec<Method>,
    /// How JSON entries are written.
    pub json_form: JsonForm,
    /// The order of the entries.
    pub entry_order: EntryOrder,
    /// The supported major version of the format.
    pub major_version: u64,
    /// The newest minor version of that major that this reader knows.
    pub known_minor: u64,
    /// What to do with a newer minor version.
    pub newer_minor: NewerMinor,
    /// The identifier of the deterministic writing profile.
    pub writer_profile: String,
}

impl ContainerSpec {
    /// `.eh3dproj` 1.0.0 as the code behaves today. Every value here is checked against the
    /// reference packages by the tests of `eh3d-common`.
    pub fn eh3dproj_v1() -> ContainerSpec {
        ContainerSpec {
            format_id: "eh3dproj".to_owned(),
            required_entries: vec![PROJECT_ENTRY.to_owned(), INTEGRITY_ENTRY.to_owned()],
            allowed_roots: vec![
                "layers/".to_owned(),
                "toolpaths/".to_owned(),
                "vision/".to_owned(),
            ],
            extension_roots: Vec::new(),
            limits: Limits {
                max_entries: 10_000,
                max_name_bytes: MAX_NAME_BYTES,
                max_total_bytes: 1024 * 1024 * 1024,
                default_max_entry_bytes: 256 * 1024 * 1024,
                per_root_max_entry_bytes: BTreeMap::new(),
            },
            allow_zip64: false,
            methods: vec![Method::Deflate],
            json_form: JsonForm::LegacyV1,
            entry_order: EntryOrder::LegacyV1,
            major_version: 1,
            known_minor: 0,
            newer_minor: NewerMinor::Accept,
            writer_profile: "eh3dproj-1.0.0".to_owned(),
        }
    }

    /// True when `name` is under one of the allowed roots.
    pub fn is_data_entry(&self, name: &str) -> bool {
        self.allowed_roots
            .iter()
            .any(|root| name.starts_with(root.as_str()))
    }

    /// True when `name` is under an extension folder: the first path part starts with one of the
    /// extension prefixes.
    pub fn is_extension_entry(&self, name: &str) -> bool {
        let first = name.split('/').next().unwrap_or("");
        name.contains('/')
            && self
                .extension_roots
                .iter()
                .any(|p| first.starts_with(p.as_str()))
    }

    /// True when a package of format version `major.minor` can be read by this spec: the same
    /// major and either a minor that is not newer than the known one or a spec that accepts
    /// newer minors. A version that is not supported must be refused with a message about the
    /// version, before any check of the roots.
    pub fn supports_version(&self, major: u64, minor: u64) -> bool {
        major == self.major_version
            && (minor <= self.known_minor || self.newer_minor == NewerMinor::Accept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_rule_of_version_1_accepts_and_refuses() {
        for ok in ["layers/top.gbr", "a", "vision/a b/c.json", "toolpaths/é.nc"] {
            assert_eq!(check_entry_name(ok), Ok(()), "{ok}");
        }
        let long = format!("layers/{}", "a".repeat(233));
        assert_eq!(long.len(), 240);
        assert!(check_entry_name(&long).is_ok());
        assert!(check_entry_name(&format!("{long}a")).is_err());
        for bad in [
            "", "/abs", "dir/", "a//b", "./a", "a/./b", "../a", "a/..", "a\\b", "c:x", "a\u{0}b",
            "a\nb", "a\u{7f}b",
        ] {
            assert_eq!(
                check_entry_name(bad),
                Err(NameError::Unsafe(bad.to_owned())),
                "{bad:?}"
            );
        }
        // Today's rule does not look at letter case or Unicode form.
        assert!(check_entry_name("Layers/A").is_ok());
    }

    #[test]
    fn the_preset_holds_the_constants_of_the_old_format() {
        let spec = ContainerSpec::eh3dproj_v1();
        assert_eq!(spec.required_entries, ["project.json", "integrity.json"]);
        assert_eq!(spec.limits.max_entries, 10_000);
        assert_eq!(spec.limits.max_name_bytes, 240);
        assert_eq!(spec.limits.max_total_bytes, 1 << 30);
        assert_eq!(spec.limits.max_entry_bytes_for("layers/x"), 256 << 20);
        assert!(!spec.allow_zip64);
        assert_eq!(spec.methods, [Method::Deflate]);
        assert_eq!(
            (spec.json_form, spec.entry_order),
            (JsonForm::LegacyV1, EntryOrder::LegacyV1)
        );
    }

    #[test]
    fn roots_and_extension_folders() {
        let mut spec = ContainerSpec::eh3dproj_v1();
        assert!(spec.is_data_entry("layers/top.gbr"));
        assert!(!spec.is_data_entry("layer/top.gbr"));
        assert!(!spec.is_data_entry("project.json"));
        assert!(!spec.is_extension_entry("x-notes/a.json"));
        spec.extension_roots.push("x-".to_owned());
        assert!(spec.is_extension_entry("x-notes/a.json"));
        assert!(
            !spec.is_extension_entry("x-file"),
            "a bare file is not a folder"
        );
        assert!(!spec.is_extension_entry("layers/x-a"));
    }

    #[test]
    fn a_root_limit_overrides_the_default_and_the_longest_root_wins() {
        let mut limits = ContainerSpec::eh3dproj_v1().limits;
        limits
            .per_root_max_entry_bytes
            .insert("results/".into(), 10);
        limits
            .per_root_max_entry_bytes
            .insert("results/big/".into(), 99);
        assert_eq!(limits.max_entry_bytes_for("results/a"), 10);
        assert_eq!(limits.max_entry_bytes_for("results/big/a"), 99);
        assert_eq!(limits.max_entry_bytes_for("layers/a"), 256 << 20);
    }

    #[test]
    fn versions_follow_the_major_and_the_minor_policy() {
        let mut spec = ContainerSpec::eh3dproj_v1();
        assert!(spec.supports_version(1, 0));
        assert!(
            spec.supports_version(1, 4),
            "version 1.0.0 reads a newer minor today"
        );
        assert!(!spec.supports_version(2, 0));
        assert!(!spec.supports_version(0, 0));
        spec.newer_minor = NewerMinor::Refuse;
        assert!(spec.supports_version(1, 0));
        assert!(!spec.supports_version(1, 1), "a newer minor is refused");
        assert!(!spec.supports_version(2, 0));
    }
}
