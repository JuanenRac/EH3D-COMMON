//! The presets of the project container and the layout rules of its folders.
//!
//! A `.eh3d` file is a ZIP container (see `eh3d-container`) whose top-level folders are
//! branches: `pcb/`, `cad/`, `3dp/`, `laser/`, `cnc/` and so on, one for each program, plus
//! `common/` (what belongs to the project as a whole), `manufacturing/` (a `.eh3dproj` 1.0.0
//! package), `docs/<program>/` (documentation, one sub-branch for each program), `audit/` (the
//! optional log, not a branch) and `x-*/` (extensions of other people, kept and never read).
//!
//! Which branch names exist is not fixed: any folder whose name follows [`valid_segment`] is a
//! branch, so a program that appears in a later version writes a branch that an older program
//! keeps untouched. [`KNOWN_BRANCHES`] is the list this crate knows by name, for display.

use std::collections::BTreeMap;

use eh3d_container::{
    Budget, ContainerSpec, EntryOrder, JsonForm, Limits, Method, NewerMinor, Policy,
};

/// The file extension of a project.
pub const EXTENSION: &str = "eh3d";
/// The format version of the project record (`project.json`): major, minor, patch.
pub const FORMAT_VERSION: &str = "1.0.0";
/// The major number of [`FORMAT_VERSION`]: a reader refuses any other.
pub const FORMAT_MAJOR: u64 = 1;
/// The minor number of [`FORMAT_VERSION`]: a reader refuses a newer one.
pub const FORMAT_MINOR: u64 = 0;
/// The version of the rules of the container (names, ZIP structure, integrity), as `major.minor`.
pub const CONTAINER_VERSION: &str = "1.0";
/// The record every branch has, directly inside its folder.
pub const BRANCH_FILE: &str = "branch.json";
/// The schema number of `branch.json` and `project.json` that this crate writes and reads.
pub const RECORD_SCHEMA: u32 = 1;
/// The folder of the documentation branches.
pub const DOCS_ROOT: &str = "docs";
/// The folder of the audit log; it is not a branch and is left out of every content hash.
pub const AUDIT_ROOT: &str = "audit";
/// The branch of the project as a whole (metadata, passport, notes); any program may write it.
pub const COMMON_BRANCH: &str = "common";
/// The branch that holds a `.eh3dproj` 1.0.0 package.
pub const MANUFACTURING_BRANCH: &str = "manufacturing";
/// The documentation branch of the project as a whole; any program may write it.
pub const PROJECT_DOCS_BRANCH: &str = "docs/project";
/// The branch of the attachments of the project (data sheets, manuals and any other file); any program may write it, through
/// [`crate::attachments`].
pub const ATTACHMENTS_BRANCH: &str = "attachments";
/// The most characters of a branch name.
pub const MAX_SEGMENT: usize = 32;

/// The eight languages of the project, as the codes the files use.
pub const LANGUAGES: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt-PT"];

/// The branches this crate knows by name.
pub const KNOWN_BRANCHES: [&str; 15] = [
    "pcb",
    "cad",
    "3dp",
    "laser",
    "cnc",
    "pnp",
    "vision",
    "routepro",
    "drillpro",
    "stencilcam",
    "gerberviewer",
    COMMON_BRANCH,
    MANUFACTURING_BRANCH,
    DOCS_ROOT,
    ATTACHMENTS_BRANCH,
];

/// The preset of the project container: the strict name and ZIP policy, canonical JSON, the
/// data entries sorted and `project.json` and `integrity.json` last, a newer minor version
/// refused. The folders are advisory here: the reader accepts any branch name that
/// [`valid_segment`] allows (the writer takes the roots of what it writes), and `x-` folders are
/// extension folders.
pub fn eh3d() -> ContainerSpec {
    let mut per_root = BTreeMap::new();
    for root in ["cad/", "3dp/", "cnc/", "pnp/", "laser/", "manufacturing/"] {
        per_root.insert(root.to_owned(), 1024 * 1024 * 1024);
    }
    let roots = [
        "pcb/",
        "cad/",
        "3dp/",
        "laser/",
        "cnc/",
        "pnp/",
        "vision/",
        "common/",
        "manufacturing/",
        "docs/",
        "attachments/",
        "audit/",
    ];
    ContainerSpec {
        format_id: EXTENSION.to_owned(),
        required_entries: vec!["project.json".to_owned(), "integrity.json".to_owned()],
        allowed_roots: roots.iter().map(|r| (*r).to_owned()).collect(),
        extension_roots: vec!["x-".to_owned()],
        limits: Limits {
            max_entries: 50_000,
            max_name_bytes: 240,
            max_total_bytes: 4 * 1024 * 1024 * 1024 - 1,
            default_max_entry_bytes: 256 * 1024 * 1024,
            per_root_max_entry_bytes: per_root,
        },
        allow_zip64: false,
        methods: vec![Method::Deflate],
        json_form: JsonForm::Jcs,
        entry_order: EntryOrder::SortedThenProjectThenIntegrity,
        major_version: FORMAT_MAJOR,
        known_minor: FORMAT_MINOR,
        newer_minor: NewerMinor::Refuse,
        writer_profile: "eh3d-jcs-1".to_owned(),
        policy: Policy::Strict,
        budget: Budget::default(),
    }
}

/// The preset of the `manufacturing` branch: exactly `.eh3dproj` 1.0.0, the format that the
/// CAM programs, the server and the Studio read. A branch written with this preset and taken out
/// of the container is, byte for byte, the package it was made from.
pub fn manufacturing() -> ContainerSpec {
    ContainerSpec::eh3dproj_v1()
}

/// True when `text` can be one part of a branch name: 1 to 32 characters, lowercase ASCII
/// letters, digits and hyphens, starting and ending with a letter or a digit.
pub fn valid_segment(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_SEGMENT {
        return false;
    }
    let edge = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    bytes.iter().all(|&b| edge(b) || b == b'-') && edge(bytes[0]) && edge(bytes[bytes.len() - 1])
}

/// True when `id` names a branch: a valid part that is neither `docs` nor `audit`, or `docs/`
/// and a valid part (the documentation of one program).
pub fn valid_branch_id(id: &str) -> bool {
    match id.split_once('/') {
        None => valid_segment(id) && id != DOCS_ROOT && id != AUDIT_ROOT,
        Some((DOCS_ROOT, rest)) => valid_segment(rest),
        Some(_) => false,
    }
}

/// What a name inside the container is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryClass<'a> {
    /// `project.json`.
    Project,
    /// `integrity.json`.
    Integrity,
    /// A file of the audit log.
    Audit,
    /// A file of a branch: the branch name and the name inside it.
    Branch {
        /// The branch name, for example `pcb` or `docs/pcb`.
        id: &'a str,
        /// The name inside the branch, for example `board.json`.
        rel: &'a str,
    },
}

/// Tells what a name is, or `None` when it has no place in the layout (a file at the top level
/// other than the two records, a folder that is not a valid branch name, a file directly under
/// `docs/`).
pub fn classify(name: &str) -> Option<EntryClass<'_>> {
    match name {
        "project.json" => return Some(EntryClass::Project),
        "integrity.json" => return Some(EntryClass::Integrity),
        _ => {}
    }
    let (root, rest) = name.split_once('/')?;
    if rest.is_empty() {
        return None;
    }
    if root == AUDIT_ROOT {
        return Some(EntryClass::Audit);
    }
    if root == DOCS_ROOT {
        let (sub, rel) = rest.split_once('/')?;
        if !valid_segment(sub) || rel.is_empty() {
            return None;
        }
        let end = root.len() + 1 + sub.len();
        return Some(EntryClass::Branch {
            id: &name[..end],
            rel,
        });
    }
    if valid_branch_id(root) {
        Some(EntryClass::Branch {
            id: root,
            rel: rest,
        })
    } else {
        None
    }
}

/// The folder of a branch, with its trailing slash.
pub fn branch_dir(id: &str) -> String {
    format!("{id}/")
}

/// The name of an entry of a branch inside the container.
pub fn entry_name(id: &str, rel: &str) -> String {
    format!("{id}/{rel}")
}

/// The top-level folder (with its slash) of a name, for the roots a writer must allow.
pub fn root_of(name: &str) -> Option<String> {
    name.split_once('/').map(|(root, _)| format!("{root}/"))
}

/// True for the branches that belong to the project as a whole and may be written by any
/// program: `common`, `docs/project` and `attachments`.
pub fn is_shared_branch(id: &str) -> bool {
    id == COMMON_BRANCH || id == PROJECT_DOCS_BRANCH || id == ATTACHMENTS_BRANCH
}

/// Reads `major.minor` (digits only, at most nine each) into numbers.
pub fn parse_major_minor(text: &str) -> Option<(u64, u64)> {
    let (a, b) = text.split_once('.')?;
    let number = |s: &str| {
        (!s.is_empty() && s.len() <= 9 && s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse::<u64>().ok())
            .flatten()
    };
    Some((number(a)?, number(b)?))
}

/// Reads `major.minor.patch` into numbers.
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let (head, patch) = text.rsplit_once('.')?;
    let (major, minor) = parse_major_minor(head)?;
    let patch =
        (!patch.is_empty() && patch.len() <= 9 && patch.bytes().all(|c| c.is_ascii_digit()))
            .then(|| patch.parse::<u64>().ok())
            .flatten()?;
    Some((major, minor, patch))
}

/// True for a SHA-256 in lowercase hexadecimal.
pub fn is_sha256_hex(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_and_branch_ids_follow_the_rule() {
        for ok in ["pcb", "3dp", "x-lbrn", "a", "a-b-1"] {
            assert!(valid_segment(ok), "{ok}");
            assert!(valid_branch_id(ok), "{ok}");
        }
        for bad in ["", "-a", "a-", "A", "a_b", "a b", "é", &"a".repeat(33)] {
            assert!(!valid_segment(bad), "{bad:?}");
        }
        assert!(valid_branch_id("docs/pcb"));
        for bad in [
            "docs", "audit", "docs/", "docs/Pcb", "docs/a/b", "pcb/x", "audit/x",
        ] {
            assert!(!valid_branch_id(bad), "{bad:?}");
        }
    }

    #[test]
    fn names_are_classified() {
        use EntryClass::*;
        assert_eq!(classify("project.json"), Some(Project));
        assert_eq!(classify("integrity.json"), Some(Integrity));
        assert_eq!(classify("audit/log.jsonl"), Some(Audit));
        assert_eq!(
            classify("pcb/branch.json"),
            Some(Branch {
                id: "pcb",
                rel: "branch.json"
            })
        );
        assert_eq!(
            classify("docs/pcb/guide/a.md"),
            Some(Branch {
                id: "docs/pcb",
                rel: "guide/a.md"
            })
        );
        assert_eq!(
            classify("x-vendor/data.bin"),
            Some(Branch {
                id: "x-vendor",
                rel: "data.bin"
            })
        );
        for none in [
            "readme.txt",
            "docs/a.md",
            "Pcb/a",
            "docs/Bad/a",
            "a_b/c",
            "audit",
        ] {
            assert_eq!(classify(none), None, "{none}");
        }
    }

    #[test]
    fn versions_are_read_strictly() {
        assert_eq!(parse_major_minor("1.0"), Some((1, 0)));
        assert_eq!(parse_major_minor("1"), None);
        assert_eq!(parse_major_minor("1.0.0"), None);
        assert_eq!(parse_major_minor("+1.0"), None);
        assert_eq!(parse_major_minor("1.9999999999"), None);
        assert_eq!(parse_version("1.0.3"), Some((1, 0, 3)));
        assert_eq!(parse_version("1.0"), None);
        assert!(is_sha256_hex(&"a".repeat(64)));
        assert!(!is_sha256_hex(&"A".repeat(64)));
    }

    #[test]
    fn the_manufacturing_preset_is_the_old_format() {
        assert_eq!(manufacturing(), ContainerSpec::eh3dproj_v1());
        let spec = eh3d();
        assert_eq!(spec.format_id, "eh3d");
        assert_eq!(spec.policy, Policy::Strict);
        assert!(spec.is_extension_entry("x-a/b"));
        assert!(!spec.allow_zip64);
    }
}
