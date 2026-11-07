//! Which build of a program is running: its repository, version, commit and kind of build.
//!
//! Every program of the suite carries a small JSON text inside its binary, for example
//!
//! ```json
//! {"schema":1,"repo":"EH3D-DRILLPRO","version":"0.1.6","commit":"1a2b3c4","kind":"release"}
//! ```
//!
//! so that the About window, a bug report and the updater can say exactly what is running without
//! reading a file next to the program, the network or the clock. The text is made at compile time
//! from the variables of Cargo ([`embedded_build!`]) or written by hand ([`BuildInfo::from_json`]);
//! the parts of this module are plain functions over text, with no input or output.
//!
//! The reader is strict: one flat object, the five keys above in any order, no key twice, no
//! other key, no escapes in the text, at most [`MAX_JSON_BYTES`] bytes. A refusal carries a stable
//! code ([`BuildError::code`]) from `EH3D-BUILD-001` to `EH3D-BUILD-008`.

use std::fmt;

/// The version of the layout of the JSON text; a text with another number is refused.
pub const SCHEMA: u32 = 1;
/// The largest JSON text the reader looks at.
pub const MAX_JSON_BYTES: usize = 1024;
/// The longest name of a repository.
pub const MAX_REPO_LEN: usize = 64;
/// The shortest and the longest text of a commit (an abbreviated or a full SHA-1 in lower case hex).
pub const COMMIT_LEN: std::ops::RangeInclusive<usize> = 7..=40;

/// How the program was built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildKind {
    /// A published, signed release.
    Release,
    /// A build of the continuous integration that was not published.
    Ci,
    /// A build on a developer's computer (the default when nothing says otherwise).
    Dev,
}

impl BuildKind {
    /// The word in the JSON text.
    pub fn as_str(self) -> &'static str {
        match self {
            BuildKind::Release => "release",
            BuildKind::Ci => "ci",
            BuildKind::Dev => "dev",
        }
    }

    /// Reads the word of the JSON text (lower case only).
    pub fn parse(text: &str) -> Option<BuildKind> {
        match text {
            "release" => Some(BuildKind::Release),
            "ci" => Some(BuildKind::Ci),
            "dev" => Some(BuildKind::Dev),
            _ => None,
        }
    }
}

/// Why a build text was refused. Each reason has a stable code ([`BuildError::code`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// `EH3D-BUILD-001`: the text is longer than [`MAX_JSON_BYTES`].
    TooLong(usize),
    /// `EH3D-BUILD-002`: the text is not one flat JSON object of strings, whole numbers and `null`; the byte where it went wrong.
    Syntax(usize),
    /// `EH3D-BUILD-003`: a key appears twice.
    DuplicateKey(String),
    /// `EH3D-BUILD-004`: a key this layout does not have.
    UnknownKey(String),
    /// `EH3D-BUILD-005`: a key of the layout is missing, or its value has the wrong type.
    MissingKey(&'static str),
    /// `EH3D-BUILD-006`: the `schema` is not [`SCHEMA`]; the number found.
    Schema(u64),
    /// `EH3D-BUILD-007`: the name of the repository is empty, longer than [`MAX_REPO_LEN`] or has a character outside letters, digits, `.`, `_` and `-`.
    Repo,
    /// `EH3D-BUILD-008`: the version is not three or four numbers split by dots, the commit is not 7 to 40 lower case hex digits, or the kind is not a known word.
    Value(&'static str),
}

impl BuildError {
    /// The stable code of the refusal.
    pub fn code(&self) -> &'static str {
        match self {
            BuildError::TooLong(_) => "EH3D-BUILD-001",
            BuildError::Syntax(_) => "EH3D-BUILD-002",
            BuildError::DuplicateKey(_) => "EH3D-BUILD-003",
            BuildError::UnknownKey(_) => "EH3D-BUILD-004",
            BuildError::MissingKey(_) => "EH3D-BUILD-005",
            BuildError::Schema(_) => "EH3D-BUILD-006",
            BuildError::Repo => "EH3D-BUILD-007",
            BuildError::Value(_) => "EH3D-BUILD-008",
        }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        match self {
            BuildError::TooLong(n) => write!(
                f,
                "{code}: the build text has {n} bytes, the most is {MAX_JSON_BYTES}"
            ),
            BuildError::Syntax(at) => write!(
                f,
                "{code}: the build text is not a flat JSON object (byte {at})"
            ),
            BuildError::DuplicateKey(k) => write!(f, "{code}: the key \"{k}\" appears twice"),
            BuildError::UnknownKey(k) => {
                write!(f, "{code}: the key \"{k}\" is not part of the build text")
            }
            BuildError::MissingKey(k) => write!(
                f,
                "{code}: the key \"{k}\" is missing or has the wrong type"
            ),
            BuildError::Schema(n) => write!(f, "{code}: schema {n} is not {SCHEMA}"),
            BuildError::Repo => write!(f, "{code}: the name of the repository is not valid"),
            BuildError::Value(k) => write!(f, "{code}: the value of \"{k}\" is not valid"),
        }
    }
}

impl std::error::Error for BuildError {}

/// What a program says about its own build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    /// The repository the program is built from, for example `EH3D-DRILLPRO`.
    pub repo: String,
    /// The version: three or four numbers split by dots.
    pub version: String,
    /// The commit as 7 to 40 lower case hex digits, or `None` when the build did not know it.
    pub commit: Option<String>,
    /// How it was built.
    pub kind: BuildKind,
}

/// Whether a repository name is valid (see [`BuildError::Repo`]).
pub fn valid_repo(repo: &str) -> bool {
    !repo.is_empty()
        && repo.len() <= MAX_REPO_LEN
        && repo
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Whether a version is three or four whole numbers split by dots, each of at most 9 digits (`0.1.9`, `0.8.0.1`).
pub fn valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    (parts.len() == 3 || parts.len() == 4)
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 9 && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether a commit is 7 to 40 lower case hex digits.
pub fn valid_commit(commit: &str) -> bool {
    COMMIT_LEN.contains(&commit.len())
        && commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl BuildInfo {
    /// Builds the record from its parts and checks each of them. A missing kind counts as [`BuildKind::Dev`]; a commit that is empty counts as unknown.
    pub fn from_parts(
        repo: &str,
        version: &str,
        commit: Option<&str>,
        kind: Option<&str>,
    ) -> Result<BuildInfo, BuildError> {
        if !valid_repo(repo) {
            return Err(BuildError::Repo);
        }
        if !valid_version(version) {
            return Err(BuildError::Value("version"));
        }
        let commit = match commit {
            None | Some("") => None,
            Some(c) if valid_commit(c) => Some(c.to_string()),
            Some(_) => return Err(BuildError::Value("commit")),
        };
        let kind = match kind {
            None | Some("") => BuildKind::Dev,
            Some(k) => BuildKind::parse(k).ok_or(BuildError::Value("kind"))?,
        };
        Ok(BuildInfo {
            repo: repo.to_string(),
            version: version.to_string(),
            commit,
            kind,
        })
    }

    /// The JSON text, with the keys in a fixed order and no spaces: the same record always gives the same bytes.
    pub fn to_json(&self) -> String {
        let commit = match &self.commit {
            Some(c) => format!("\"{c}\""),
            None => "null".to_string(),
        };
        format!(
            "{{\"schema\":{SCHEMA},\"repo\":\"{}\",\"version\":\"{}\",\"commit\":{commit},\"kind\":\"{}\"}}",
            self.repo,
            self.version,
            self.kind.as_str()
        )
    }

    /// Reads a JSON text (see the module for the rules).
    pub fn from_json(text: &str) -> Result<BuildInfo, BuildError> {
        if text.len() > MAX_JSON_BYTES {
            return Err(BuildError::TooLong(text.len()));
        }
        let fields = parse_flat(text)?;
        let mut schema = None;
        let mut repo = None;
        let mut version = None;
        let mut commit: Option<Option<String>> = None;
        let mut kind = None;
        for (key, value) in fields {
            let slot_taken = match key.as_str() {
                "schema" => schema.replace(value).is_some(),
                "repo" => repo.replace(value).is_some(),
                "version" => version.replace(value).is_some(),
                "kind" => kind.replace(value).is_some(),
                "commit" => commit
                    .replace(match value {
                        Value::Null => None,
                        Value::Str(s) => Some(s),
                        Value::Num(_) => return Err(BuildError::MissingKey("commit")),
                    })
                    .is_some(),
                _ => return Err(BuildError::UnknownKey(key)),
            };
            if slot_taken {
                return Err(BuildError::DuplicateKey(key));
            }
        }
        match schema {
            Some(Value::Num(n)) if n == u64::from(SCHEMA) => {}
            Some(Value::Num(n)) => return Err(BuildError::Schema(n)),
            _ => return Err(BuildError::MissingKey("schema")),
        }
        let text_of = |value: Option<Value>, key: &'static str| match value {
            Some(Value::Str(s)) => Ok(s),
            _ => Err(BuildError::MissingKey(key)),
        };
        let repo = text_of(repo, "repo")?;
        let version = text_of(version, "version")?;
        let kind = text_of(kind, "kind")?;
        let commit = commit.ok_or(BuildError::MissingKey("commit"))?;
        // a stored commit is either null or valid: an empty text is not a way to say "unknown" here
        if matches!(commit.as_deref(), Some("")) {
            return Err(BuildError::Value("commit"));
        }
        if BuildKind::parse(&kind).is_none() {
            return Err(BuildError::Value("kind"));
        }
        BuildInfo::from_parts(&repo, &version, commit.as_deref(), Some(&kind))
    }
}

impl fmt::Display for BuildInfo {
    /// `EH3D-DRILLPRO 0.1.6 (1a2b3c4, release)`; the commit shows at most its first 7 digits, `unknown` when there is none.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let commit = self
            .commit
            .as_deref()
            .map(|c| c.get(..7).unwrap_or(c))
            .unwrap_or("unknown");
        write!(
            f,
            "{} {} ({commit}, {})",
            self.repo,
            self.version,
            self.kind.as_str()
        )
    }
}

/// The record of the crate that uses the macro, made from what Cargo and the build give at compile time:
/// the repository from `EH3D_BUILD_REPO` (the name of the crate when it is not set), the version of the crate,
/// the commit from `EH3D_BUILD_COMMIT` and the kind from `EH3D_BUILD_KIND`. It gives a `Result<BuildInfo, BuildError>`.
///
/// A `build.rs` of the program sets the optional ones with `println!("cargo:rustc-env=EH3D_BUILD_COMMIT=...")`
/// (for example from `git rev-parse --short HEAD`); nothing here reads the clock or the network.
#[macro_export]
macro_rules! embedded_build {
    () => {
        $crate::build::BuildInfo::from_parts(
            match option_env!("EH3D_BUILD_REPO") {
                Some(repo) => repo,
                None => env!("CARGO_PKG_NAME"),
            },
            env!("CARGO_PKG_VERSION"),
            option_env!("EH3D_BUILD_COMMIT"),
            option_env!("EH3D_BUILD_KIND"),
        )
    };
}

enum Value {
    Str(String),
    Num(u64),
    Null,
}

/// Reads `{"key":value,...}` with strings without escapes, whole numbers and `null`. Duplicate keys are kept so the caller can name them.
fn parse_flat(text: &str) -> Result<Vec<(String, Value)>, BuildError> {
    let b = text.as_bytes();
    let mut i = 0;
    let skip = |i: &mut usize| {
        while *i < b.len() && matches!(b[*i], b' ' | b'\t' | b'\r' | b'\n') {
            *i += 1;
        }
    };
    let string = |i: &mut usize| -> Result<String, BuildError> {
        if b.get(*i) != Some(&b'"') {
            return Err(BuildError::Syntax(*i));
        }
        let start = *i + 1;
        let mut j = start;
        while j < b.len() && b[j] != b'"' {
            if b[j] == b'\\' || b[j] < 0x20 {
                return Err(BuildError::Syntax(j));
            }
            j += 1;
        }
        if j >= b.len() {
            return Err(BuildError::Syntax(j));
        }
        *i = j + 1;
        Ok(text[start..j].to_string())
    };
    skip(&mut i);
    if b.get(i) != Some(&b'{') {
        return Err(BuildError::Syntax(i));
    }
    i += 1;
    let mut out = Vec::new();
    skip(&mut i);
    if b.get(i) == Some(&b'}') {
        i += 1;
    } else {
        loop {
            skip(&mut i);
            let key = string(&mut i)?;
            skip(&mut i);
            if b.get(i) != Some(&b':') {
                return Err(BuildError::Syntax(i));
            }
            i += 1;
            skip(&mut i);
            let value = match b.get(i) {
                Some(b'"') => Value::Str(string(&mut i)?),
                Some(b'n') if text[i..].starts_with("null") => {
                    i += 4;
                    Value::Null
                }
                Some(d) if d.is_ascii_digit() => {
                    let start = i;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i - start > 9 || (i - start > 1 && b[start] == b'0') {
                        return Err(BuildError::Syntax(start));
                    }
                    Value::Num(
                        text[start..i]
                            .parse()
                            .map_err(|_| BuildError::Syntax(start))?,
                    )
                }
                _ => return Err(BuildError::Syntax(i)),
            };
            out.push((key, value));
            skip(&mut i);
            match b.get(i) {
                Some(b',') => i += 1,
                Some(b'}') => {
                    i += 1;
                    break;
                }
                _ => return Err(BuildError::Syntax(i)),
            }
        }
    }
    skip(&mut i);
    if i != b.len() {
        return Err(BuildError::Syntax(i));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"schema":1,"repo":"EH3D-DRILLPRO","version":"0.1.6","commit":"1a2b3c4","kind":"release"}"#;

    #[test]
    fn reads_the_sample_by_hand() {
        let info = BuildInfo::from_json(SAMPLE).unwrap();
        assert_eq!(info.repo, "EH3D-DRILLPRO");
        assert_eq!(info.version, "0.1.6");
        assert_eq!(info.commit.as_deref(), Some("1a2b3c4"));
        assert_eq!(info.kind, BuildKind::Release);
    }

    #[test]
    fn writes_the_exact_bytes_and_reads_them_back() {
        let info = BuildInfo::from_parts(
            "EH3D-NODE-BUNGARD",
            "0.8.0.1",
            Some("0123456789abcdef0123456789abcdef01234567"),
            Some("ci"),
        )
        .unwrap();
        let json = info.to_json();
        assert_eq!(
            json,
            r#"{"schema":1,"repo":"EH3D-NODE-BUNGARD","version":"0.8.0.1","commit":"0123456789abcdef0123456789abcdef01234567","kind":"ci"}"#
        );
        assert_eq!(BuildInfo::from_json(&json).unwrap(), info);
        assert_eq!(BuildInfo::from_json(SAMPLE).unwrap().to_json(), SAMPLE);
    }

    #[test]
    fn a_build_without_commit_writes_null_and_defaults_to_dev() {
        let info = BuildInfo::from_parts("EH3D-COMMON", "0.1.10", None, None).unwrap();
        assert_eq!(info.kind, BuildKind::Dev);
        assert_eq!(
            info.to_json(),
            r#"{"schema":1,"repo":"EH3D-COMMON","version":"0.1.10","commit":null,"kind":"dev"}"#
        );
        assert_eq!(BuildInfo::from_json(&info.to_json()).unwrap(), info);
        assert_eq!(
            BuildInfo::from_parts("A", "1.2.3", Some(""), Some(""))
                .unwrap()
                .commit,
            None
        );
    }

    #[test]
    fn keys_may_come_in_any_order_and_with_spaces() {
        let text = "{ \"kind\" : \"dev\",\n \"commit\": null, \"version\": \"1.2.3\", \"repo\": \"X\", \"schema\": 1 }\n";
        let info = BuildInfo::from_json(text).unwrap();
        assert_eq!(info.to_string(), "X 1.2.3 (unknown, dev)");
    }

    #[test]
    fn the_display_shows_seven_digits_of_the_commit() {
        let info = BuildInfo::from_parts(
            "EH3D-SERVER",
            "0.1.3",
            Some("abcdef0123456789"),
            Some("release"),
        )
        .unwrap();
        assert_eq!(info.to_string(), "EH3D-SERVER 0.1.3 (abcdef0, release)");
    }

    #[test]
    fn the_codes_of_the_refusals() {
        let code = |t: &str| BuildInfo::from_json(t).unwrap_err().code();
        assert_eq!(code(&" ".repeat(MAX_JSON_BYTES + 1)), "EH3D-BUILD-001");
        assert_eq!(code("[]"), "EH3D-BUILD-002");
        assert_eq!(code(""), "EH3D-BUILD-002");
        assert_eq!(code(&format!("{SAMPLE} x")), "EH3D-BUILD-002");
        assert_eq!(code(r#"{"schema":1,"schema":1}"#), "EH3D-BUILD-003");
        assert_eq!(code(r#"{"schema":1,"extra":"x"}"#), "EH3D-BUILD-004");
        assert_eq!(code(r#"{"schema":1,"repo":"X"}"#), "EH3D-BUILD-005");
        assert_eq!(
            code(r#"{"repo":"X","version":"1.2.3","commit":null,"kind":"dev"}"#),
            "EH3D-BUILD-005"
        );
        assert_eq!(
            code(r#"{"schema":2,"repo":"X","version":"1.2.3","commit":null,"kind":"dev"}"#),
            "EH3D-BUILD-006"
        );
        assert_eq!(
            code(r#"{"schema":1,"repo":"a b","version":"1.2.3","commit":null,"kind":"dev"}"#),
            "EH3D-BUILD-007"
        );
        assert_eq!(
            code(r#"{"schema":1,"repo":"X","version":"1.2","commit":null,"kind":"dev"}"#),
            "EH3D-BUILD-008"
        );
        assert_eq!(
            code(r#"{"schema":1,"repo":"X","version":"1.2.3","commit":"ABCDEF0","kind":"dev"}"#),
            "EH3D-BUILD-008"
        );
        assert_eq!(
            code(r#"{"schema":1,"repo":"X","version":"1.2.3","commit":"abc","kind":"dev"}"#),
            "EH3D-BUILD-008"
        );
        assert_eq!(
            code(r#"{"schema":1,"repo":"X","version":"1.2.3","commit":"","kind":"dev"}"#),
            "EH3D-BUILD-008"
        );
        assert_eq!(
            code(r#"{"schema":1,"repo":"X","version":"1.2.3","commit":null,"kind":"Release"}"#),
            "EH3D-BUILD-008"
        );
    }

    #[test]
    fn a_wrong_type_is_a_missing_key() {
        let e = BuildInfo::from_json(
            r#"{"schema":1,"repo":3,"version":"1.2.3","commit":null,"kind":"dev"}"#,
        )
        .unwrap_err();
        assert_eq!(e, BuildError::MissingKey("repo"));
        let e = BuildInfo::from_json(
            r#"{"schema":"1","repo":"X","version":"1.2.3","commit":null,"kind":"dev"}"#,
        )
        .unwrap_err();
        assert_eq!(e, BuildError::MissingKey("schema"));
        let e = BuildInfo::from_json(
            r#"{"schema":1,"repo":"X","version":"1.2.3","commit":7,"kind":"dev"}"#,
        )
        .unwrap_err();
        assert_eq!(e, BuildError::MissingKey("commit"));
    }

    #[test]
    fn escapes_and_odd_numbers_are_refused() {
        assert_eq!(
            BuildInfo::from_json(r#"{"schema":1,"repo":"a\"b"}"#)
                .unwrap_err()
                .code(),
            "EH3D-BUILD-002"
        );
        assert_eq!(
            BuildInfo::from_json(r#"{"schema":01}"#).unwrap_err().code(),
            "EH3D-BUILD-002"
        );
        assert_eq!(
            BuildInfo::from_json(r#"{"schema":1.0}"#)
                .unwrap_err()
                .code(),
            "EH3D-BUILD-002"
        );
        assert_eq!(
            BuildInfo::from_json(r#"{"schema":-1}"#).unwrap_err().code(),
            "EH3D-BUILD-002"
        );
        assert_eq!(
            BuildInfo::from_json(r#"{"schema":1,}"#).unwrap_err().code(),
            "EH3D-BUILD-002"
        );
    }

    #[test]
    fn the_rules_of_each_part() {
        assert!(valid_version("0.1.9") && valid_version("0.8.0.1") && valid_version("10.20.30"));
        assert!(
            !valid_version("0.1")
                && !valid_version("0.1.2.3.4")
                && !valid_version("0..1")
                && !valid_version("v0.1.2")
                && !valid_version("0.1.2-rc")
        );
        assert!(valid_repo("EH3D-NODE-BUNGARD-AUX") && valid_repo("a.b_c-1"));
        assert!(
            !valid_repo("")
                && !valid_repo("a/b")
                && !valid_repo(&"x".repeat(MAX_REPO_LEN + 1))
                && valid_repo(&"x".repeat(MAX_REPO_LEN))
        );
        assert!(valid_commit("abcdef0") && valid_commit(&"a".repeat(40)));
        assert!(
            !valid_commit("abcdef")
                && !valid_commit(&"a".repeat(41))
                && !valid_commit("abcdefg")
                && !valid_commit("ABCDEF0")
        );
    }

    #[test]
    fn the_macro_reads_the_crate_it_is_used_in() {
        let info = crate::embedded_build!().unwrap();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(valid_repo(&info.repo));
        assert_eq!(BuildInfo::from_json(&info.to_json()).unwrap(), info);
    }
}
