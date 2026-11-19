//! The settings of the link to the server, read from the keys of the machine file every program already has.
//!
//! ```text
//! server_address = 192.168.0.180:18200   # host:port of EH3D-SERVER; empty = no server
//! server_token = ...                      # the token of a program (write-only: never shown, never logged)
//! server_machine = bungard                # the machine the jobs are for
//! server_timeout_s = 15                   # 1 to 120: how long a request may take before it is given up
//! server_retries = 2                      # 0 to 5: how many more times a request that failed on the network is tried
//! server_mdns = off                       # on | off: look for the server on the local network (off by default)
//! server_max_answer_mib = 8               # 1 to 256: the most MiB read from one answer
//! ```

use crate::secret::Secret;
use std::fmt;

pub const DEFAULT_TIMEOUT_S: u64 = 15;
pub const DEFAULT_RETRIES: u32 = 2;
pub const DEFAULT_MAX_ANSWER_MIB: u64 = 8;

/// One key: its name, its range or values and its default, as the docs and the screens of settings list them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySpec {
    pub key: &'static str,
    /// `text`, `secret`, `1..120`, `on|off`...
    pub range: &'static str,
    pub default: &'static str,
}

/// Every key of the link, in the order the docs list them.
pub const KEYS: [KeySpec; 7] = [
    KeySpec {
        key: "server_address",
        range: "host:port, or empty",
        default: "",
    },
    KeySpec {
        key: "server_token",
        range: "secret text",
        default: "",
    },
    KeySpec {
        key: "server_machine",
        range: "text",
        default: "",
    },
    KeySpec {
        key: "server_timeout_s",
        range: "1..120",
        default: "15",
    },
    KeySpec {
        key: "server_retries",
        range: "0..5",
        default: "2",
    },
    KeySpec {
        key: "server_mdns",
        range: "on|off",
        default: "off",
    },
    KeySpec {
        key: "server_max_answer_mib",
        range: "1..256",
        default: "8",
    },
];

/// Why a value was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingError {
    pub key: String,
    pub message: String,
}

impl fmt::Display for SettingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.message)
    }
}

impl std::error::Error for SettingError {}

/// The settings of the link. `Debug` never shows the token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub server_address: String,
    pub server_token: Secret,
    pub server_machine: String,
    pub server_timeout_s: u64,
    pub server_retries: u32,
    /// Look for the server with mDNS. Off by default: the program asks nothing of the network unless the owner turns it on.
    pub server_mdns: bool,
    pub server_max_answer_mib: u64,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            server_address: String::new(),
            server_token: Secret::new(""),
            server_machine: String::new(),
            server_timeout_s: DEFAULT_TIMEOUT_S,
            server_retries: DEFAULT_RETRIES,
            server_mdns: false,
            server_max_answer_mib: DEFAULT_MAX_ANSWER_MIB,
        }
    }
}

fn err(key: &str, message: &str) -> SettingError {
    SettingError {
        key: key.to_string(),
        message: message.to_string(),
    }
}

fn number(key: &str, value: &str, lo: u64, hi: u64) -> Result<u64, SettingError> {
    match value.trim().parse::<u64>() {
        Ok(v) if (lo..=hi).contains(&v) => Ok(v),
        _ => Err(err(
            key,
            &format!("must be a whole number from {lo} to {hi}"),
        )),
    }
}

fn switch(key: &str, value: &str) -> Result<bool, SettingError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        _ => Err(err(key, "must be on or off")),
    }
}

/// True when the key is one of the link.
pub fn is_server_key(key: &str) -> bool {
    KEYS.iter().any(|k| k.key == key)
}

impl Settings {
    /// Sets one key. An unknown key is `Ok(false)` so that a program can offer the rest of its own keys to its own reader; a value out of its range is an
    /// error and changes nothing.
    pub fn set(&mut self, key: &str, value: &str) -> Result<bool, SettingError> {
        let key_l = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key_l.as_str() {
            "server_address" => {
                if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
                    return Err(err(
                        "server_address",
                        "must not hold a space or a control character",
                    ));
                }
                self.server_address = value.to_string();
            }
            "server_token" => {
                if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
                    return Err(err(
                        "server_token",
                        "must not hold a space or a control character",
                    ));
                }
                self.server_token = Secret::new(value);
            }
            "server_machine" => self.server_machine = value.to_string(),
            "server_timeout_s" => self.server_timeout_s = number(&key_l, value, 1, 120)?,
            "server_retries" => self.server_retries = number(&key_l, value, 0, 5)? as u32,
            "server_mdns" => self.server_mdns = switch(&key_l, value)?,
            "server_max_answer_mib" => self.server_max_answer_mib = number(&key_l, value, 1, 256)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Reads the text of a machine file: `key = value` lines, blank lines and lines that start with `#` or `;` ignored, a `#` after a value starts a
    /// comment. Keys that are not of the link are skipped (the program reads them); a bad value of a key of the link is an error with its line.
    pub fn from_text(text: &str) -> Result<Settings, String> {
        let mut s = Settings::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.split(" #").next().unwrap_or("");
            s.set(key, value)
                .map_err(|e| format!("line {}: {}", n + 1, e))?;
        }
        Ok(s)
    }

    /// Is there a server to talk to?
    pub fn is_configured(&self) -> bool {
        !self.server_address.is_empty()
    }

    /// The lines of the file for the screen of settings and for saving, with the token as `***` unless `with_secret` is true (only the program that
    /// writes its own file asks for it).
    pub fn to_text(&self, with_secret: bool) -> String {
        let token = if with_secret {
            self.server_token.expose().to_string()
        } else if self.server_token.is_empty() {
            String::new()
        } else {
            "***".to_string()
        };
        format!(
            "server_address = {}\nserver_token = {}\nserver_machine = {}\nserver_timeout_s = {}\nserver_retries = {}\nserver_mdns = {}\nserver_max_answer_mib = {}\n",
            self.server_address,
            token,
            self.server_machine,
            self.server_timeout_s,
            self.server_retries,
            if self.server_mdns { "on" } else { "off" },
            self.server_max_answer_mib
        )
    }
}
