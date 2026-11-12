//! The limits of the readers: how big a file may be before a reader refuses it, and from where it warns. The values live in `reader_limits.cfg`
//! (the file that comes with this crate, which a program can replace with its own copy through [`ReaderLimits::from_config`]); nothing is
//! written into the code. Refusing early keeps a huge or hostile file from using the time and the memory of the program.

use std::fmt;

/// The shipped file with the conservative values.
pub const DEFAULT_CONFIG: &str = include_str!("../reader_limits.cfg");

/// How big a text file may be for a reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReaderLimits {
    /// A file with more bytes is refused.
    pub max_bytes: u64,
    /// A file with more lines is refused.
    pub max_lines: u64,
    /// A file with more bytes is read, with a warning.
    pub warn_bytes: u64,
    /// A file with more lines is read, with a warning.
    pub warn_lines: u64,
}

/// Which limit a file went past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    Bytes,
    Lines,
}

/// What the check of a file found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitVerdict {
    /// Inside the limits.
    Ok,
    /// Readable, but big: a program should say so.
    Warn(LimitKind),
}

/// A file past a hard limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitError {
    pub kind: LimitKind,
    pub limit: u64,
    pub found: u64,
}

impl fmt::Display for LimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            LimitKind::Bytes => "bytes",
            LimitKind::Lines => "lines",
        };
        write!(
            f,
            "the file has {} {what}, more than the limit of {}",
            self.found, self.limit
        )
    }
}

impl std::error::Error for LimitError {}

/// A line of the configuration that cannot be used (`line` is 0 for a fault of the whole file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub line: usize,
    pub text: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.text)
    }
}

impl std::error::Error for ConfigError {}

const KEYS: [&str; 4] = ["max_bytes", "max_lines", "warn_bytes", "warn_lines"];

impl Default for ReaderLimits {
    /// The values of the shipped file.
    fn default() -> ReaderLimits {
        ReaderLimits::from_config(DEFAULT_CONFIG).expect("the shipped reader_limits.cfg is valid")
    }
}

impl ReaderLimits {
    /// Reads the limits from the text of a configuration file. Every key must be there once, a value must be a whole number above zero, and a
    /// warning limit may not be above its hard limit.
    pub fn from_config(text: &str) -> Result<ReaderLimits, ConfigError> {
        let mut found: [Option<u64>; 4] = [None; 4];
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let bad = |why: &str| ConfigError {
                line: n + 1,
                text: why.to_string(),
            };
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| bad("expected key = value"))?;
            let key = key.trim();
            let slot = KEYS
                .iter()
                .position(|k| *k == key)
                .ok_or_else(|| bad("unknown key"))?;
            if found[slot].is_some() {
                return Err(bad("key given twice"));
            }
            let v: u64 = value
                .trim()
                .parse()
                .map_err(|_| bad("not a whole number"))?;
            if v == 0 {
                return Err(bad("must be above zero"));
            }
            found[slot] = Some(v);
        }
        let get = |i: usize| {
            found[i].ok_or_else(|| ConfigError {
                line: 0,
                text: format!("missing key {}", KEYS[i]),
            })
        };
        let limits = ReaderLimits {
            max_bytes: get(0)?,
            max_lines: get(1)?,
            warn_bytes: get(2)?,
            warn_lines: get(3)?,
        };
        if limits.warn_bytes > limits.max_bytes || limits.warn_lines > limits.max_lines {
            return Err(ConfigError {
                line: 0,
                text: "a warning limit is above its hard limit".into(),
            });
        }
        Ok(limits)
    }

    /// Checks the text of a file before it is read.
    pub fn check(&self, text: &str) -> Result<LimitVerdict, LimitError> {
        let bytes = text.len() as u64;
        if bytes > self.max_bytes {
            return Err(LimitError {
                kind: LimitKind::Bytes,
                limit: self.max_bytes,
                found: bytes,
            });
        }
        let lines = text.bytes().filter(|&b| b == b'\n').count() as u64 + 1;
        if lines > self.max_lines {
            return Err(LimitError {
                kind: LimitKind::Lines,
                limit: self.max_lines,
                found: lines,
            });
        }
        if bytes > self.warn_bytes {
            return Ok(LimitVerdict::Warn(LimitKind::Bytes));
        }
        if lines > self.warn_lines {
            return Ok(LimitVerdict::Warn(LimitKind::Lines));
        }
        Ok(LimitVerdict::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = "max_bytes = 100\nmax_lines = 5\nwarn_bytes = 50\nwarn_lines = 3\n";

    #[test]
    fn the_shipped_file_gives_real_limits() {
        let l = ReaderLimits::default();
        assert!(l.max_bytes > 0 && l.warn_bytes <= l.max_bytes);
        assert!(l.max_lines > 0 && l.warn_lines <= l.max_lines);
        assert_eq!(ReaderLimits::from_config(DEFAULT_CONFIG), Ok(l));
    }

    #[test]
    fn a_file_is_ok_warned_or_refused_by_its_size_and_its_lines() {
        let l = ReaderLimits::from_config(SMALL).unwrap();
        assert_eq!(l.check("a\nb\n"), Ok(LimitVerdict::Ok));
        assert_eq!(
            l.check("a\nb\nc\nd\n"),
            Ok(LimitVerdict::Warn(LimitKind::Lines))
        );
        assert_eq!(
            l.check(&"x".repeat(60)),
            Ok(LimitVerdict::Warn(LimitKind::Bytes))
        );
        assert_eq!(
            l.check(&"x".repeat(101)).unwrap_err().kind,
            LimitKind::Bytes
        );
        assert_eq!(
            l.check("1\n2\n3\n4\n5\n6\n").unwrap_err().kind,
            LimitKind::Lines
        );
    }

    #[test]
    fn a_bad_configuration_is_refused_with_its_line() {
        for (text, line) in [
            ("max_bytes\n", 1),
            ("max_bytes = x\n", 1),
            ("max_bytes = 0\n", 1),
            ("nope = 1\n", 1),
            ("max_bytes = 1\nmax_bytes = 2\n", 2),
        ] {
            assert_eq!(
                ReaderLimits::from_config(text).unwrap_err().line,
                line,
                "{text}"
            );
        }
        assert!(ReaderLimits::from_config("max_bytes = 5\n").is_err());
        let over = "max_bytes = 5\nmax_lines = 5\nwarn_bytes = 6\nwarn_lines = 1\n";
        assert!(ReaderLimits::from_config(over).is_err());
    }
}
