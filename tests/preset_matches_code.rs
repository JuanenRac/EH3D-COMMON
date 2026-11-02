//! The `eh3dproj` preset of `eh3d-container` describes the constants of this crate. If one side
//! changes, this test fails, so the description cannot drift from the behaviour.

use eh3d_common::container::{
    check_entry_name, INTEGRITY_ENTRY, MAX_ENTRIES, MAX_ENTRY_BYTES, MAX_TOTAL_BYTES, PROJECT_ENTRY,
};
use eh3d_common::project::{read_metadata, DATA_FOLDERS};
use eh3d_container::{ContainerSpec, JsonForm, NewerMinor};

#[test]
fn the_limits_and_entries_of_the_preset_are_the_ones_of_the_code() {
    let spec = ContainerSpec::eh3dproj_v1();
    assert_eq!(spec.limits.max_entries, MAX_ENTRIES);
    assert_eq!(spec.limits.max_total_bytes, MAX_TOTAL_BYTES);
    assert_eq!(spec.limits.default_max_entry_bytes, MAX_ENTRY_BYTES);
    assert_eq!(spec.required_entries, [PROJECT_ENTRY, INTEGRITY_ENTRY]);
    let roots: Vec<&str> = spec.allowed_roots.iter().map(String::as_str).collect();
    assert_eq!(roots, DATA_FOLDERS);
    assert_eq!(spec.json_form, JsonForm::LegacyV1);
}

#[test]
fn the_name_rule_is_the_same_on_both_sides() {
    let names = [
        "layers/a.gbr",
        "",
        "/a",
        "a/",
        "a//b",
        "a/../b",
        r"a\b",
        "a:b",
        "a\u{1}b",
        "Layers/A",
        "é/ü",
        "toolpaths/./x",
    ];
    for name in names {
        assert_eq!(
            check_entry_name(name).is_ok(),
            eh3d_container::check_entry_name(name).is_ok(),
            "{name:?}"
        );
    }
    assert!(check_entry_name(&"a".repeat(241)).is_err());
    assert!(check_entry_name(&"a".repeat(240)).is_ok());
}

#[test]
fn the_version_policy_of_the_preset_is_what_the_reader_does() {
    let spec = ContainerSpec::eh3dproj_v1();
    assert_eq!(spec.newer_minor, NewerMinor::Accept);
    let json = |version: &str| {
        format!(
            r#"{{"format_version":"{version}","id":"00000000-0000-4000-8000-000000000001","name":"n","created":"2026-01-02T03:04:05Z","modified":"2026-01-02T03:04:05Z"}}"#
        )
    };
    for (major, minor) in [(1, 0), (1, 4), (2, 0)] {
        let read = read_metadata(&json(&format!("{major}.{minor}.0"))).is_ok();
        assert_eq!(spec.supports_version(major, minor), read, "{major}.{minor}");
    }
}
