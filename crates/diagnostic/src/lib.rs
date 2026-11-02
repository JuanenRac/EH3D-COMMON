//! The common diagnostic of the suite.
//!
//! A reader that cannot understand something does not panic and does not guess: it returns a
//! [`Diagnostic`]. A diagnostic has a stable code (an opaque string such as `EH3D-PKG-010` or
//! `PHOENIX-IO-002`), a severity, an English message, and where it happened. This crate knows no
//! registry of codes: each prefix has its own registry elsewhere. Codes and field names are never
//! translated; a user interface translates by the key `diag.<code>` and fills in [`Diagnostic::args`].
//!
//! The JSON form is stable and is what the server API carries: lower-case enum values, optional
//! fields left out when absent, `args` as an object.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::borrow::Cow;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Severity {
    /// Worth knowing, changes nothing.
    Info,
    /// Something was repaired, skipped or approximated.
    Warning,
    /// The input cannot be trusted as it is.
    Error,
}

/// What kind of problem it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Category {
    /// An electrical problem.
    Electrical,
    /// A geometric problem.
    Geometric,
    /// A problem for manufacturing.
    Manufacturing,
    /// A safety problem.
    Safety,
    /// A problem with the format of a file.
    Format,
    /// A fault of the program itself.
    Internal,
}

/// Whether the problem can be put right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Recoverability {
    /// The reader went on and the rest can be used.
    Recoverable,
    /// The reader had to stop.
    Unrecoverable,
    /// The reader could not tell: the check is inconclusive.
    Inconclusive,
}

/// Where a diagnostic applies, when a line number is not enough.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Location {
    /// A column of the line.
    Column(u32),
    /// An entry of a ZIP container, by name.
    Entry(String),
    /// A design object, by id.
    Object(String),
    /// A byte offset in the file.
    Offset(u64),
}

#[cfg(feature = "serde")]
mod args_form {
    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(args: &[(String, String)], s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(args.len()))?;
        for (key, value) in args {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }

    struct Pairs;

    impl<'de> Visitor<'de> for Pairs {
        type Value = Vec<(String, String)>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("an object of text values")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                out.push((key, value));
            }
            Ok(out)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(String, String)>, D::Error> {
        d.deserialize_map(Pairs)
    }
}

/// One finding of a reader or a check.
///
/// The struct is `#[non_exhaustive]`: build it with [`Diagnostic::new`] and the `with_` methods.
/// Adding a field later is a change of the major version of this crate.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Diagnostic {
    /// The stable code, such as `EH3D-PKG-010`; `None` for readers that have none yet.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub code: Option<Cow<'static, str>>,
    /// How serious it is.
    pub severity: Severity,
    /// The English text. A user interface translates by the key `diag.<code>`.
    pub message: String,
    /// The line it applies to; 0 means no line.
    pub line: u32,
    /// Where it applies, when a line is not enough.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub location: Option<Location>,
    /// What kind of problem it is.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub category: Option<Category>,
    /// Whether it can be put right.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub recoverability: Option<Recoverability>,
    /// The values the translatable message needs, in order.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty", with = "args_form")
    )]
    pub args: Vec<(String, String)>,
}

impl Diagnostic {
    /// A diagnostic with a line, a severity and a message, and nothing else.
    pub fn new(line: u32, severity: Severity, message: impl Into<String>) -> Self {
        Diagnostic {
            code: None,
            severity,
            message: message.into(),
            line,
            location: None,
            category: None,
            recoverability: None,
            args: Vec::new(),
        }
    }

    /// Set the stable code.
    pub fn with_code(mut self, code: impl Into<Cow<'static, str>>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set where it applies.
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: Category) -> Self {
        self.category = Some(category);
        self
    }

    /// Set the recoverability.
    pub fn with_recoverability(mut self, recoverability: Recoverability) -> Self {
        self.recoverability = Some(recoverability);
        self
    }

    /// Add a value for the translatable message.
    pub fn with_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((key.into(), value.into()));
        self
    }

    /// The form for runs over a private corpus: it keeps the code, severity, category and
    /// recoverability and drops the message, the arguments, the line and the location, which may
    /// hold names or values taken from the files.
    pub fn redacted(&self) -> Diagnostic {
        Diagnostic {
            code: self.code.clone(),
            severity: self.severity,
            message: String::new(),
            line: 0,
            location: None,
            category: self.category,
            recoverability: self.recoverability,
            args: Vec::new(),
        }
    }
}

/// True when no diagnostic is an error.
pub fn is_clean(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().all(|d| d.severity != Severity::Error)
}

/// False when any diagnostic is inconclusive: the result may be incomplete even if it is clean.
pub fn is_conclusive(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .all(|d| d.recoverability != Some(Recoverability::Inconclusive))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Diagnostic {
        Diagnostic::new(12, Severity::Error, "clearance below the minimum")
            .with_code("PHOENIX-DRC-001")
            .with_location(Location::Object(
                "018f0000-0000-7000-8000-000000000001".into(),
            ))
            .with_category(Category::Manufacturing)
            .with_recoverability(Recoverability::Recoverable)
            .with_arg("min_nm", "150000")
            .with_arg("found_nm", "90000")
    }

    #[test]
    fn a_plain_diagnostic_has_nothing_optional() {
        let d = Diagnostic::new(0, Severity::Info, "hello");
        assert!(d.code.is_none() && d.location.is_none() && d.category.is_none());
        assert!(d.recoverability.is_none() && d.args.is_empty());
    }

    #[test]
    fn clean_means_no_error_and_conclusive_means_nothing_inconclusive() {
        let warn = Diagnostic::new(1, Severity::Warning, "w");
        let err = Diagnostic::new(1, Severity::Error, "e");
        let unsure = Diagnostic::new(1, Severity::Info, "i")
            .with_recoverability(Recoverability::Inconclusive);
        assert!(is_clean(&[]) && is_clean(std::slice::from_ref(&warn)));
        assert!(!is_clean(&[warn.clone(), err]));
        assert!(
            is_clean(std::slice::from_ref(&unsure)),
            "inconclusive is not an error"
        );
        assert!(!is_conclusive(&[warn.clone(), unsure]));
        assert!(is_conclusive(&[warn]));
    }

    #[test]
    fn redaction_keeps_only_what_cannot_name_anything() {
        let d = full().with_arg("net", "SECRET_NET").redacted();
        assert_eq!(d.code.as_deref(), Some("PHOENIX-DRC-001"));
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.category, Some(Category::Manufacturing));
        assert_eq!(d.recoverability, Some(Recoverability::Recoverable));
        let shown = format!("{d:?}");
        for needle in ["SECRET_NET", "clearance", "018f", "150000"] {
            assert!(!shown.contains(needle), "{needle}");
        }
        assert_eq!(
            (d.line, d.location.is_none(), d.args.is_empty()),
            (0, true, true)
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn the_json_form_is_the_stable_one() {
        let json = serde_json::to_string(&full()).unwrap();
        assert_eq!(
            json,
            r#"{"code":"PHOENIX-DRC-001","severity":"error","message":"clearance below the minimum","line":12,"location":{"object":"018f0000-0000-7000-8000-000000000001"},"category":"manufacturing","recoverability":"recoverable","args":{"min_nm":"150000","found_nm":"90000"}}"#
        );
        // a bare diagnostic leaves every optional field out
        let bare = serde_json::to_string(&Diagnostic::new(3, Severity::Warning, "x")).unwrap();
        assert_eq!(bare, r#"{"severity":"warning","message":"x","line":3}"#);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn json_round_trips_and_owned_codes_deserialize() {
        for d in [
            full(),
            Diagnostic::new(0, Severity::Info, "m"),
            full().redacted(),
        ] {
            let back: Diagnostic =
                serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
            assert_eq!(back, d);
        }
        let from_text: Diagnostic = serde_json::from_str(
            r#"{"code":"EH3D-PKG-010","severity":"info","message":"m","line":0}"#,
        )
        .unwrap();
        assert_eq!(from_text.code.as_deref(), Some("EH3D-PKG-010"));
        for loc in [
            Location::Column(4),
            Location::Entry("layers/a".into()),
            Location::Offset(99),
        ] {
            let d = Diagnostic::new(0, Severity::Info, "m").with_location(loc.clone());
            let back: Diagnostic =
                serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
            assert_eq!(back.location, Some(loc));
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn bad_json_is_an_error_not_a_panic() {
        for text in [
            r#"{"severity":"fatal","message":"x","line":0}"#,
            r#"{"severity":"error","message":"x"}"#,
            r#"{"severity":"error","message":"x","line":-1}"#,
            r#"{"severity":"error","message":"x","line":0,"args":["a"]}"#,
            r#"{"severity":"error","message":"x","line":0,"args":{"a":1}}"#,
            r#"{"severity":"error","message":"x","line":0,"location":{"nowhere":1}}"#,
            "[]",
        ] {
            assert!(serde_json::from_str::<Diagnostic>(text).is_err(), "{text}");
        }
    }
}
