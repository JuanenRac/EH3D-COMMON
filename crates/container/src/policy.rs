//! The name policy: what an entry name may be.
//!
//! `Legacy` is the rule of `.eh3dproj` 1.0.0 and keeps its behaviour. `Strict` is the policy for
//! new formats: a name must also be valid UTF-8 in Unicode normal form C, and two names that
//! differ only in letter case or Unicode form are the same name (they would collide on Windows
//! and macOS).

use unicode_normalization::{is_nfc, UnicodeNormalization};

use crate::codes::PkgCode;
use crate::error::Violation;

/// How strictly names and the ZIP structure are judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// The behaviour of `.eh3dproj` 1.0.0: names by the old rule, exact duplicates only, flags
    /// and data after the archive not examined.
    Legacy,
    /// The policy for new formats (codes 001 to 009 as written in the registry).
    Strict,
}

/// Checks `name` under `policy`. The length limit is `max_name_bytes`.
pub fn check_name(policy: Policy, name: &str, max_name_bytes: usize) -> Result<(), Violation> {
    let bad = |why: &str| Violation::new(PkgCode::InvalidName, why).at(name);
    if name.len() > max_name_bytes {
        return Err(bad("the name is longer than the limit"));
    }
    // The old rule (empty, absolute, trailing slash, backslash, colon, control characters, empty
    // parts, `.` and `..`) holds for both policies; its length limit is applied above.
    check_without_length(name)
        .map_err(|()| bad("the name is empty, absolute, not portable or leaves the folder"))?;
    if policy == Policy::Strict {
        if !is_nfc(name) {
            return Err(bad("the name is not in Unicode normal form C"));
        }
        for part in name.split('/') {
            if part.ends_with('.') || part.ends_with(' ') {
                return Err(bad("a part of the name ends with a dot or a space"));
            }
            if is_windows_reserved(part) {
                return Err(bad("a part of the name is a reserved device name on Windows"));
            }
        }
    }
    Ok(())
}

/// True for the names Windows treats as devices (`CON`, `NUL`, `COM1`, ...), with or without an
/// extension and in any letter case.
pub fn is_windows_reserved(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part).trim_end().to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" => true,
        _ => {
            let digits = stem.strip_prefix("COM").or_else(|| stem.strip_prefix("LPT"));
            matches!(digits, Some(d) if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() && d != "0")
        }
    }
}

/// The old rule without its fixed length limit.
fn check_without_length(name: &str) -> Result<(), ()> {
    if name.is_empty() || name.starts_with('/') || name.ends_with('/') {
        return Err(());
    }
    if name.contains('\\') || name.contains(':') || name.chars().any(char::is_control) {
        return Err(());
    }
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(());
    }
    Ok(())
}

/// The key under which two names are the same name for the strict policy: normal form C, then
/// lower case.
pub fn collision_key(name: &str) -> String {
    name.nfc().collect::<String>().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_refuses_what_the_old_rule_refuses_and_what_is_not_nfc() {
        for bad in ["", "/a", "a/", "a//b", "a/../b", "a\\b", "a:b", "a\u{0}b"] {
            for policy in [Policy::Legacy, Policy::Strict] {
                let e = check_name(policy, bad, 240).unwrap_err();
                assert_eq!(e.code, PkgCode::InvalidName, "{bad:?}");
            }
        }
        let nfd = "design/e\u{301}.json"; // e + combining acute
        let nfc = "design/\u{e9}.json";
        assert!(check_name(Policy::Legacy, nfd, 240).is_ok());
        assert_eq!(
            check_name(Policy::Strict, nfd, 240).unwrap_err().code,
            PkgCode::InvalidName
        );
        assert!(check_name(Policy::Strict, nfc, 240).is_ok());
    }

    #[test]
    fn strict_refuses_windows_device_names_and_trailing_dots_or_spaces() {
        for bad in [
            "design/CON", "design/nul.json", "design/Aux.txt", "design/com1", "design/LPT9.x",
            "com5/a", "design/a.", "design/a ", "design/dir./a", "design/CONIN$",
        ] {
            assert!(check_name(Policy::Strict, bad, 240).is_err(), "{bad}");
            assert!(check_name(Policy::Legacy, bad, 240).is_ok(), "{bad}: legacy keeps its rule");
        }
        for ok in ["design/console.json", "design/com10", "design/com0", "design/lpt", "design/a.b", "design/auxiliary"] {
            assert!(check_name(Policy::Strict, ok, 240).is_ok(), "{ok}");
        }
    }

    #[test]
    fn the_length_limit_comes_from_the_caller() {
        let name = format!("design/{}", "a".repeat(300));
        assert!(check_name(Policy::Strict, &name, 400).is_ok());
        assert!(check_name(Policy::Strict, &name, 240).is_err());
        let with_dots = format!("design/{}/../x", "a".repeat(300));
        assert!(check_name(Policy::Strict, &with_dots, 400).is_err());
    }

    #[test]
    fn names_that_differ_in_case_or_form_collide() {
        assert_eq!(
            collision_key("Design/A.json"),
            collision_key("design/a.json")
        );
        assert_eq!(collision_key("e\u{301}"), collision_key("\u{c9}"));
        assert_ne!(collision_key("design/a"), collision_key("design/b"));
    }
}
