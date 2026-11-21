//! The real transport of the Klipper controller: JSON-RPC of Moonraker over plain HTTP/1.1 on a `std::net::TcpStream`, with no crate beyond
//! `serde_json` that the crate already has (option A of `docs/adr/0001-klipper-transport.md`). Every message the controller sends is one
//! `POST /server/jsonrpc` on a connection of its own (`Connection: close`), made by a short-lived thread, and every answer comes back
//! through a queue that [`HttpJsonRpc::recv`] reads with a time limit. That is what lets the controller ask the machine whether it is alive
//! (`printer.info`) while a line that moves is still waiting on its connection, and send `printer.emergency_stop` on a third one without
//! waiting for anything: the emergency stop is never refused for want of a free place and it abandons the requests that were waiting.
//!
//! What is checked, because the other end is a machine on a network and may be broken or hostile: the address (only loopback and the
//! local network unless [`HttpConfig::allow_non_local`] is on), the size of the head and of the body, the framing (`Content-Length` or a
//! simple chunked body, never both, never repeated), the end of the line of every header, the status (a redirection is refused and never
//! followed, a 4xx or 5xx is an error), that the body is a JSON object, and the time of every phase (connect, wait for the first byte, gap
//! between bytes, whole answer). Every failure is an `io::Error` that carries an [`HttpError`] with a number from 209 to 219 and a text
//! that never contains the credential, the body or the address of a redirection; the controller reports it as `Error::Link` and
//! `Error::code()` reads the number, which [`crate::dialtext`] says in eight languages.
//!
//! What this transport does not have: TLS (Moonraker listens on plain HTTP on a trusted network), the notifications that Moonraker pushes
//! over a WebSocket (a shutdown is noticed from the state in the status, which the controller reads), and the sockets kept open between
//! requests. Nothing here has run against a real Moonraker: the evidence is a fake one in the tests.

use std::fmt;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::grbl::{Timeouts, MAX_BUSY_WAIT};
use crate::klipper::{JsonRpcTransport, KlipperController};

/// Moonraker answered with a status that is an error (4xx or 5xx) other than the refusal of the credential.
pub const E_HTTP_STATUS: u32 = 209;
/// Moonraker refused the credential or asked for one (401 and 403).
pub const E_HTTP_AUTH: u32 = 210;
/// Moonraker answered with a redirection; it is never followed.
pub const E_HTTP_REDIRECT: u32 = 211;
/// The answer (or the request) is larger than the limit.
pub const E_HTTP_TOO_BIG: u32 = 212;
/// The body of the answer is not a JSON object.
pub const E_HTTP_NOT_JSON: u32 = 213;
/// The answer is not well-formed HTTP.
pub const E_HTTP_PROTOCOL: u32 = 214;
/// The address is refused.
pub const E_HTTP_HOST: u32 = 215;
/// Too many requests are waiting.
pub const E_HTTP_BUSY: u32 = 216;
/// Nothing listens at the address, or it cannot be reached.
pub const E_HTTP_UNREACHABLE: u32 = 217;
/// The connection closed or broke before the answer was complete.
pub const E_HTTP_CLOSED: u32 = 218;
/// A phase of the exchange took longer than its limit.
pub const E_HTTP_TIMEOUT: u32 = 219;

/// Every number of this transport, in order.
pub const HTTP_KINDS: [u32; 11] = [209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219];

/// The one route used: Moonraker takes a JSON-RPC request in the body and answers with the JSON-RPC response.
pub const PATH: &str = "/server/jsonrpc";

/// The most a request may weigh.
pub const MAX_REQUEST_BYTES: usize = 1 << 20;

/// How often a waiting thread looks at whether the transport was dropped or the requests were abandoned.
const POLL: Duration = Duration::from_millis(100);

/// The most headers an answer may have.
const MAX_HEADERS: usize = 100;

/// A request that can still be given to the emergency stop when every place is taken.
const STOP_EXTRA: usize = 2;

/// What went wrong with the transport: a number of [`HTTP_KINDS`] and a fixed text that holds neither a credential nor a byte of the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpError {
    pub code: u32,
    pub detail: String,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error {}: {}", self.code, self.detail)
    }
}

impl std::error::Error for HttpError {}

impl HttpError {
    fn io(code: u32, detail: impl Into<String>) -> io::Error {
        let kind = match code {
            E_HTTP_AUTH => io::ErrorKind::PermissionDenied,
            E_HTTP_BUSY => io::ErrorKind::WouldBlock,
            E_HTTP_UNREACHABLE => io::ErrorKind::ConnectionRefused,
            E_HTTP_CLOSED => io::ErrorKind::UnexpectedEof,
            E_HTTP_TIMEOUT => io::ErrorKind::TimedOut,
            E_HTTP_HOST => io::ErrorKind::InvalidInput,
            _ => io::ErrorKind::InvalidData,
        };
        io::Error::new(
            kind,
            HttpError {
                code,
                detail: detail.into(),
            },
        )
    }
}

/// The number of [`HTTP_KINDS`] an error of this transport carries; `None` for any other error.
pub fn http_error_code(e: &io::Error) -> Option<u32> {
    e.get_ref()
        .and_then(|r| r.downcast_ref::<HttpError>())
        .map(|h| h.code)
}

/// Whether a number is one of this transport.
pub fn is_http_code(code: u32) -> bool {
    HTTP_KINDS.contains(&code)
}

/// The range of a numeric setting of the configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setting {
    pub key: &'static str,
    pub min: u64,
    pub max: u64,
    pub default: u64,
}

/// The numeric settings of the configuration file, with their range and their default. The text settings are `klipper.host`
/// (default `127.0.0.1`), `klipper.allow_non_local` (`true` or `false`, default `false`) and `klipper.api_key` (none by default).
pub const SETTINGS: [Setting; 12] = [
    Setting {
        key: "klipper.port",
        min: 1,
        max: 65535,
        default: 7125,
    },
    Setting {
        key: "klipper.connect_timeout_ms",
        min: 100,
        max: 60_000,
        default: 3_000,
    },
    Setting {
        key: "klipper.read_timeout_ms",
        min: 100,
        max: 120_000,
        default: 5_000,
    },
    Setting {
        key: "klipper.answer_timeout_ms",
        min: 100,
        max: 600_000,
        default: 30_000,
    },
    Setting {
        key: "klipper.response_wait_ms",
        min: 1_000,
        max: 86_400_000,
        default: 14_460_000,
    },
    Setting {
        key: "klipper.max_response_bytes",
        min: 1_024,
        max: 16_777_216,
        default: 1_048_576,
    },
    Setting {
        key: "klipper.max_header_bytes",
        min: 512,
        max: 65_536,
        default: 16_384,
    },
    Setting {
        key: "klipper.max_in_flight",
        min: 1,
        max: 16,
        default: 4,
    },
    Setting {
        key: "klipper.line_timeout_ms",
        min: 100,
        max: 600_000,
        default: 5_000,
    },
    Setting {
        key: "klipper.homing_timeout_ms",
        min: 1_000,
        max: 3_600_000,
        default: 120_000,
    },
    Setting {
        key: "klipper.jog_timeout_ms",
        min: 1_000,
        max: 3_600_000,
        default: 60_000,
    },
    Setting {
        key: "klipper.busy_limit_s",
        min: 0,
        max: 14_400,
        default: 14_400,
    },
];

/// What is wrong with a configuration: the line (0 when it is about the whole file) and a fixed text. The text never holds the value of
/// a setting that was written, so a credential written on the wrong line is not repeated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for CfgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "settings: {}", self.message)
        } else {
            write!(f, "settings line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for CfgError {}

fn cfg_error(line: usize, message: impl Into<String>) -> CfgError {
    CfgError {
        line,
        message: message.into(),
    }
}

/// The settings of the transport and of the controller that goes over it.
#[derive(Clone)]
pub struct HttpConfig {
    /// An IP address or `localhost` (a name only when [`HttpConfig::allow_non_local`] is on).
    pub host: String,
    pub port: u16,
    pub connect_timeout: Duration,
    /// The longest gap between two bytes of an answer that has begun.
    pub read_timeout: Duration,
    /// The longest an answer may take to arrive entire once its first byte has come.
    pub answer_timeout: Duration,
    /// The longest a request for a script of G-code waits for the first byte of its answer (a move answers when it ends).
    pub response_wait: Duration,
    pub max_response_bytes: usize,
    pub max_header_bytes: usize,
    pub max_in_flight: usize,
    /// Accepts an address outside the local network. Off by default.
    pub allow_non_local: bool,
    /// The times the controller waits for a line, the homing and a jog.
    pub timeouts: Timeouts,
    /// The most a line that moves waits while the machine answers the liveness probes.
    pub busy_limit: Duration,
    api_key: Option<String>,
}

impl fmt::Debug for HttpConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("answer_timeout", &self.answer_timeout)
            .field("response_wait", &self.response_wait)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("max_header_bytes", &self.max_header_bytes)
            .field("max_in_flight", &self.max_in_flight)
            .field("allow_non_local", &self.allow_non_local)
            .field("timeouts", &self.timeouts)
            .field("busy_limit", &self.busy_limit)
            .field(
                "api_key",
                &if self.api_key.is_some() {
                    "set"
                } else {
                    "none"
                },
            )
            .finish()
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        let mut cfg = HttpConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            connect_timeout: Duration::ZERO,
            read_timeout: Duration::ZERO,
            answer_timeout: Duration::ZERO,
            response_wait: Duration::ZERO,
            max_response_bytes: 0,
            max_header_bytes: 0,
            max_in_flight: 0,
            allow_non_local: false,
            timeouts: Timeouts::default(),
            busy_limit: MAX_BUSY_WAIT,
            api_key: None,
        };
        for s in SETTINGS {
            cfg.put_number(s.key, s.default);
        }
        cfg
    }
}

impl HttpConfig {
    /// The default settings for a machine at this address and port.
    pub fn new(host: &str, port: u16) -> HttpConfig {
        HttpConfig {
            host: host.to_string(),
            port,
            ..HttpConfig::default()
        }
    }

    /// Sets the credential sent in the `X-Api-Key` header. It must be 1 to 256 printable characters of ASCII; it is never shown by
    /// `Debug` or by an error.
    pub fn with_api_key(mut self, key: &str) -> Result<HttpConfig, CfgError> {
        self.set_api_key(key)?;
        Ok(self)
    }

    fn set_api_key(&mut self, key: &str) -> Result<(), CfgError> {
        let ok =
            !key.is_empty() && key.len() <= 256 && key.bytes().all(|b| (0x21..=0x7e).contains(&b));
        if !ok {
            return Err(cfg_error(
                0,
                "klipper.api_key must be 1 to 256 printable characters without spaces",
            ));
        }
        self.api_key = Some(key.to_string());
        Ok(())
    }

    /// Whether a credential is set.
    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    fn put_number(&mut self, key: &str, v: u64) {
        let ms = Duration::from_millis;
        match key {
            "klipper.port" => self.port = v as u16,
            "klipper.connect_timeout_ms" => self.connect_timeout = ms(v),
            "klipper.read_timeout_ms" => self.read_timeout = ms(v),
            "klipper.answer_timeout_ms" => self.answer_timeout = ms(v),
            "klipper.response_wait_ms" => self.response_wait = ms(v),
            "klipper.max_response_bytes" => self.max_response_bytes = v as usize,
            "klipper.max_header_bytes" => self.max_header_bytes = v as usize,
            "klipper.max_in_flight" => self.max_in_flight = v as usize,
            "klipper.line_timeout_ms" => self.timeouts.line = ms(v),
            "klipper.homing_timeout_ms" => self.timeouts.homing = ms(v),
            "klipper.jog_timeout_ms" => self.timeouts.jog = ms(v),
            "klipper.busy_limit_s" => self.busy_limit = Duration::from_secs(v),
            _ => {}
        }
    }

    /// The value of a numeric setting (`None` for a key that is not one), in the unit of its name.
    pub fn number(&self, key: &str) -> Option<u64> {
        let ms = |d: Duration| d.as_millis() as u64;
        Some(match key {
            "klipper.port" => u64::from(self.port),
            "klipper.connect_timeout_ms" => ms(self.connect_timeout),
            "klipper.read_timeout_ms" => ms(self.read_timeout),
            "klipper.answer_timeout_ms" => ms(self.answer_timeout),
            "klipper.response_wait_ms" => ms(self.response_wait),
            "klipper.max_response_bytes" => self.max_response_bytes as u64,
            "klipper.max_header_bytes" => self.max_header_bytes as u64,
            "klipper.max_in_flight" => self.max_in_flight as u64,
            "klipper.line_timeout_ms" => ms(self.timeouts.line),
            "klipper.homing_timeout_ms" => ms(self.timeouts.homing),
            "klipper.jog_timeout_ms" => ms(self.timeouts.jog),
            "klipper.busy_limit_s" => self.busy_limit.as_secs(),
            _ => return None,
        })
    }

    /// Sets a numeric setting by its key, refusing a value outside its range.
    pub fn set_number(&mut self, key: &str, value: u64) -> Result<(), CfgError> {
        let Some(s) = SETTINGS.iter().find(|s| s.key == key) else {
            return Err(cfg_error(0, "unknown setting"));
        };
        if value < s.min || value > s.max {
            return Err(cfg_error(
                0,
                format!("{} must be between {} and {}", s.key, s.min, s.max),
            ));
        }
        self.put_number(key, value);
        Ok(())
    }

    /// Reads the settings from the text of a configuration file: `key = value` lines, `#` comments and blank lines. Unknown keys, a key
    /// written twice, a value that is not a number or is outside its range, and a line with no `=` are refused, each with its line.
    pub fn from_cfg(text: &str) -> Result<HttpConfig, CfgError> {
        let mut cfg = HttpConfig::default();
        let mut seen: Vec<String> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let n = i + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                return Err(cfg_error(n, "a line must be key = value"));
            };
            let (k, v) = (k.trim(), v.trim());
            let known = SETTINGS.iter().any(|s| s.key == k)
                || ["klipper.host", "klipper.allow_non_local", "klipper.api_key"].contains(&k);
            if !known {
                return Err(cfg_error(n, "unknown setting"));
            }
            if seen.iter().any(|s| s == k) {
                return Err(cfg_error(n, "a setting is written twice"));
            }
            seen.push(k.to_string());
            match k {
                "klipper.host" => cfg.host = v.to_string(),
                "klipper.allow_non_local" => {
                    cfg.allow_non_local = match v {
                        "true" => true,
                        "false" => false,
                        _ => {
                            return Err(cfg_error(
                                n,
                                "klipper.allow_non_local must be true or false",
                            ))
                        }
                    }
                }
                "klipper.api_key" => cfg.set_api_key(v).map_err(|e| cfg_error(n, e.message))?,
                _ => {
                    let Ok(value) = v.parse::<u64>() else {
                        return Err(cfg_error(n, format!("{k} must be a whole number")));
                    };
                    cfg.set_number(k, value)
                        .map_err(|e| cfg_error(n, e.message))?;
                }
            }
        }
        cfg.check()?;
        Ok(cfg)
    }

    /// Checks what no single key can: the address, and that a script is not cut off before the controller itself would give up.
    pub fn check(&self) -> Result<(), CfgError> {
        resolve(&self.host, self.port, self.allow_non_local).map_err(|e| cfg_error(0, e.detail))?;
        if self.response_wait < self.busy_limit {
            return Err(cfg_error(
                0,
                "klipper.response_wait_ms must not be shorter than klipper.busy_limit_s, or a move would be cut off",
            ));
        }
        Ok(())
    }
}

/// Whether an address is on the local network: loopback, private, link-local (and the unique-local of IPv6).
fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => a.is_loopback() || a.is_private() || a.is_link_local(),
        IpAddr::V6(a) => {
            if let Some(v4) = a.to_ipv4_mapped() {
                return is_local(IpAddr::V4(v4));
            }
            let first = a.segments()[0];
            a.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    }
}

fn is_name(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    let labels: Vec<&str> = host.split('.').collect();
    let well_formed = labels.iter().all(|l| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    });
    // a last label of digits only is an old spelling of an address (`127.1`, `0x7f.1`), not a name
    let last = labels.last().copied().unwrap_or("");
    well_formed && !last.bytes().all(|b| b.is_ascii_digit())
}

/// The socket address for a host and a port, or the reason it is refused. An IP address is used as it is; `localhost` is the loopback; any
/// other name is resolved only when `allow_non_local` is on. The unspecified, the multicast and the broadcast addresses are never taken.
pub fn resolve(host: &str, port: u16, allow_non_local: bool) -> Result<SocketAddr, HttpError> {
    let refuse = |why: &str| HttpError {
        code: E_HTTP_HOST,
        detail: why.to_string(),
    };
    if port == 0 {
        return Err(refuse("the port must be between 1 and 65535"));
    }
    if host.is_empty() || host.len() > 253 || !host.is_ascii() {
        return Err(refuse("the host is not an address"));
    }
    let ip = if let Ok(ip) = host.parse::<IpAddr>() {
        ip
    } else if host.eq_ignore_ascii_case("localhost") {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else if !is_name(host) {
        return Err(refuse("the host is not an address"));
    } else if !allow_non_local {
        return Err(refuse(
            "only an address of the local network is allowed; a name needs the switch for other networks",
        ));
    } else {
        let found: Vec<SocketAddr> = (host, port)
            .to_socket_addrs()
            .map_err(|_| refuse("the name does not resolve"))?
            .collect();
        let pick = found
            .iter()
            .find(|a| a.is_ipv4())
            .or_else(|| found.first())
            .ok_or_else(|| refuse("the name does not resolve"))?;
        pick.ip()
    };
    let plain = match ip {
        IpAddr::V6(a) => a.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        v4 => v4,
    };
    let bad = match plain {
        IpAddr::V4(a) => a.is_unspecified() || a.is_multicast() || a.is_broadcast(),
        IpAddr::V6(a) => a.is_unspecified() || a.is_multicast(),
    };
    if bad {
        return Err(refuse("that address cannot be a machine"));
    }
    if !allow_non_local && !is_local(plain) {
        return Err(refuse(
            "only an address of the local network is allowed unless the switch for other networks is on",
        ));
    }
    Ok(SocketAddr::new(plain, port))
}

/// What a worker leaves in the queue.
enum Event {
    Body(String),
    Failed(io::Error),
}

/// The state the transport shares with its threads.
struct Shared {
    closed: AtomicBool,
    /// Raised by the emergency stop: the requests of an older epoch are abandoned.
    epoch: AtomicU64,
    in_flight: AtomicUsize,
}

/// Gives a place back when its thread ends, however it ends.
struct Place(Arc<Shared>);

impl Drop for Place {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The error of a thread that stops because the transport was dropped or the request was abandoned: nobody is waiting for it.
#[derive(Debug)]
struct Abandoned;

impl fmt::Display for Abandoned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("abandoned")
    }
}

impl std::error::Error for Abandoned {}

struct Job {
    cfg: Arc<HttpConfig>,
    addr: SocketAddr,
    host_header: String,
    body: String,
    wait: Duration,
    epoch: u64,
}

/// An answer being read: the socket, what was read and not used yet, and the clocks of its phases.
struct Reader<'a> {
    stream: TcpStream,
    buf: Vec<u8>,
    pos: usize,
    shared: &'a Shared,
    epoch: u64,
    began: Instant,
    wait: Duration,
    read: Duration,
    answer: Duration,
    first: Option<Instant>,
    last: Instant,
}

fn closed_early() -> io::Error {
    HttpError::io(
        E_HTTP_CLOSED,
        "the connection closed before the answer was complete",
    )
}

fn malformed(why: &str) -> io::Error {
    HttpError::io(E_HTTP_PROTOCOL, why)
}

fn too_big() -> io::Error {
    HttpError::io(E_HTTP_TOO_BIG, "the answer is larger than the limit")
}

impl Reader<'_> {
    /// Reads more from the socket. `Ok(0)` is the end of the stream.
    fn fill(&mut self) -> io::Result<usize> {
        if self.pos > 0 && self.pos == self.buf.len() {
            self.buf.clear();
            self.pos = 0;
        }
        let mut chunk = [0u8; 4096];
        loop {
            if self.shared.closed.load(Ordering::SeqCst)
                || self.shared.epoch.load(Ordering::SeqCst) != self.epoch
            {
                return Err(io::Error::other(Abandoned));
            }
            let now = Instant::now();
            let limit = match self.first {
                None => self.began + self.wait,
                Some(f) => (self.last + self.read).min(f + self.answer),
            };
            if now >= limit {
                return Err(HttpError::io(
                    E_HTTP_TIMEOUT,
                    if self.first.is_none() {
                        "no answer came in the time given"
                    } else {
                        "the answer stopped or was too slow"
                    },
                ));
            }
            let slice = (limit - now).min(POLL).max(Duration::from_millis(1));
            self.stream.set_read_timeout(Some(slice))?;
            match self.stream.read(&mut chunk) {
                Ok(0) => return Ok(0),
                Ok(n) => {
                    let t = Instant::now();
                    self.first.get_or_insert(t);
                    self.last = t;
                    self.buf.extend_from_slice(&chunk[..n]);
                    return Ok(n);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock
                            | io::ErrorKind::TimedOut
                            | io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => return Err(closed_early()),
            }
        }
    }

    /// One line that ends in CRLF, without the CRLF. A line feed with no carriage return before it, or a carriage return inside, is refused,
    /// and so is a line that would take more than the budget of the head.
    fn line(&mut self, budget: &mut usize) -> io::Result<Vec<u8>> {
        loop {
            let avail = &self.buf[self.pos..];
            if let Some(i) = avail.iter().position(|&b| b == b'\n') {
                if i == 0 || avail[i - 1] != b'\r' {
                    return Err(malformed("a line does not end in CRLF"));
                }
                let line = avail[..i - 1].to_vec();
                if line.contains(&b'\r') {
                    return Err(malformed("a line holds a bare carriage return"));
                }
                if i + 1 > *budget {
                    return Err(malformed("the head of the answer is too large"));
                }
                *budget -= i + 1;
                self.pos += i + 1;
                return Ok(line);
            }
            if avail.len() > *budget {
                return Err(malformed("the head of the answer is too large"));
            }
            if self.fill()? == 0 {
                return Err(closed_early());
            }
        }
    }

    /// Appends exactly `n` bytes of body to `out`.
    fn exact(&mut self, n: usize, out: &mut Vec<u8>) -> io::Result<()> {
        let mut left = n;
        while left > 0 {
            if self.pos == self.buf.len() && self.fill()? == 0 {
                return Err(closed_early());
            }
            let take = left.min(self.buf.len() - self.pos);
            out.extend_from_slice(&self.buf[self.pos..self.pos + take]);
            self.pos += take;
            left -= take;
        }
        Ok(())
    }

    /// Everything up to the end of the stream, refusing more than `cap` bytes.
    fn until_close(&mut self, cap: usize, out: &mut Vec<u8>) -> io::Result<()> {
        loop {
            out.extend_from_slice(&self.buf[self.pos..]);
            self.pos = self.buf.len();
            if out.len() > cap {
                return Err(too_big());
            }
            if self.fill()? == 0 {
                return Ok(());
            }
        }
    }

    /// A chunked body: sizes in hexadecimal (extensions are ignored), each chunk closed by CRLF, a last chunk of zero and the trailers,
    /// which are read within the budget of the head and thrown away.
    fn chunked(&mut self, cap: usize, head_budget: usize, out: &mut Vec<u8>) -> io::Result<()> {
        loop {
            let mut small = 256usize;
            let line = self.line(&mut small)?;
            let size_text = line.split(|&b| b == b';').next().unwrap_or(&[]);
            if size_text.is_empty()
                || size_text.len() > 16
                || !size_text.iter().all(u8::is_ascii_hexdigit)
            {
                return Err(malformed("a chunk has no valid size"));
            }
            let size = std::str::from_utf8(size_text)
                .ok()
                .and_then(|t| u64::from_str_radix(t, 16).ok())
                .ok_or_else(|| malformed("a chunk has no valid size"))?;
            if size == 0 {
                let mut budget = head_budget;
                loop {
                    if self.line(&mut budget)?.is_empty() {
                        return Ok(());
                    }
                }
            }
            if (out.len() as u64).saturating_add(size) > cap as u64 {
                return Err(too_big());
            }
            self.exact(size as usize, out)?;
            let mut two = 2usize;
            if !self.line(&mut two)?.is_empty() {
                return Err(malformed("a chunk does not end in CRLF"));
            }
        }
    }
}

fn parse_status(line: &[u8]) -> io::Result<u16> {
    let ok = line.len() >= 12
        && (line.starts_with(b"HTTP/1.1 ") || line.starts_with(b"HTTP/1.0 "))
        && line[9..12].iter().all(u8::is_ascii_digit)
        && (line.len() == 12 || line[12] == b' ')
        && line.iter().all(|&b| (0x20..=0x7e).contains(&b));
    if !ok {
        return Err(malformed("the status line is not HTTP/1"));
    }
    Ok(std::str::from_utf8(&line[9..12])
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(0))
}

fn exchange(job: &Job, shared: &Shared) -> io::Result<String> {
    let cfg = &job.cfg;
    let stream = TcpStream::connect_timeout(&job.addr, cfg.connect_timeout).map_err(|e| {
        HttpError::io(
            E_HTTP_UNREACHABLE,
            format!("cannot connect ({:?})", e.kind()),
        )
    })?;
    let _ = stream.set_nodelay(true);
    let _ = stream.set_write_timeout(Some(cfg.read_timeout));
    let mut head = format!(
        "POST {PATH} HTTP/1.1\r\nHost: {}\r\nUser-Agent: eh3d-controller\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        job.host_header,
        job.body.len()
    );
    if let Some(key) = &cfg.api_key {
        head.push_str("X-Api-Key: ");
        head.push_str(key);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    head.push_str(&job.body);
    {
        let mut out = &stream;
        out.write_all(head.as_bytes())
            .and_then(|()| out.flush())
            .map_err(|_| closed_early())?;
    }
    drop(head);
    let mut r = Reader {
        stream,
        buf: Vec::new(),
        pos: 0,
        shared,
        epoch: job.epoch,
        began: Instant::now(),
        wait: job.wait,
        read: cfg.read_timeout,
        answer: cfg.answer_timeout,
        first: None,
        last: Instant::now(),
    };
    let mut budget = cfg.max_header_bytes;
    let status_line = r.line(&mut budget)?;
    let status = parse_status(&status_line)?;
    match status {
        200..=299 => {}
        100..=199 => return Err(malformed("an interim status is not expected")),
        300..=399 => {
            return Err(HttpError::io(
                E_HTTP_REDIRECT,
                format!("HTTP status {status}: a redirection is not followed"),
            ))
        }
        401 | 403 => {
            return Err(HttpError::io(
                E_HTTP_AUTH,
                format!("HTTP status {status}: the credential was refused or is missing"),
            ))
        }
        _ => {
            return Err(HttpError::io(
                E_HTTP_STATUS,
                format!("HTTP status {status}"),
            ))
        }
    }
    let mut length: Option<u64> = None;
    let mut chunked = false;
    let mut count = 0usize;
    loop {
        let line = r.line(&mut budget)?;
        if line.is_empty() {
            break;
        }
        count += 1;
        if count > MAX_HEADERS {
            return Err(malformed("the answer has too many headers"));
        }
        if line[0] == b' ' || line[0] == b'\t' {
            return Err(malformed("a folded header is not accepted"));
        }
        if !line
            .iter()
            .all(|&b| b == b'\t' || (0x20..=0x7e).contains(&b))
        {
            return Err(malformed("a header has bytes that are not text"));
        }
        let Some(colon) = line.iter().position(|&b| b == b':') else {
            return Err(malformed("a header has no colon"));
        };
        let name = &line[..colon];
        if name.is_empty()
            || !name
                .iter()
                .all(|&b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        {
            return Err(malformed("a header has an invalid name"));
        }
        let value = std::str::from_utf8(&line[colon + 1..])
            .map_err(|_| malformed("a header has bytes that are not text"))?
            .trim_matches(|c| c == ' ' || c == '\t');
        if name.eq_ignore_ascii_case(b"content-length") {
            if length.is_some() {
                return Err(malformed("Content-Length is repeated"));
            }
            if value.is_empty() || value.len() > 18 || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(malformed("Content-Length is not a number"));
            }
            length = value.parse().ok();
        } else if name.eq_ignore_ascii_case(b"transfer-encoding") {
            if chunked || !value.eq_ignore_ascii_case("chunked") {
                return Err(malformed("only a single chunked coding is accepted"));
            }
            chunked = true;
        }
    }
    if chunked && length.is_some() {
        return Err(malformed("Content-Length and chunked together"));
    }
    let cap = cfg.max_response_bytes;
    let mut body = Vec::new();
    if chunked {
        r.chunked(cap, cfg.max_header_bytes, &mut body)?;
    } else if let Some(n) = length {
        if n > cap as u64 {
            return Err(too_big());
        }
        r.exact(n as usize, &mut body)?;
    } else {
        r.until_close(cap, &mut body)?;
    }
    let not_json = || HttpError::io(E_HTTP_NOT_JSON, "the answer is not a JSON object");
    let text = String::from_utf8(body).map_err(|_| not_json())?;
    match serde_json::from_str::<Value>(&text) {
        Ok(v) if v.is_object() => Ok(text),
        _ => Err(not_json()),
    }
}

fn work(job: Job, shared: &Shared, tx: &Sender<Event>) {
    let result = exchange(&job, shared);
    let stale =
        shared.closed.load(Ordering::SeqCst) || shared.epoch.load(Ordering::SeqCst) != job.epoch;
    let event = match result {
        Ok(text) if !stale => Event::Body(text),
        Err(e) if !stale && !e.get_ref().is_some_and(|r| r.is::<Abandoned>()) => Event::Failed(e),
        _ => return,
    };
    let _ = tx.send(event);
}

/// The transport of the Klipper controller over HTTP. See the module for what it does and what it does not.
pub struct HttpJsonRpc {
    cfg: Arc<HttpConfig>,
    addr: SocketAddr,
    host_header: String,
    shared: Arc<Shared>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl fmt::Debug for HttpJsonRpc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpJsonRpc")
            .field("addr", &self.addr)
            .field("cfg", &self.cfg)
            .finish()
    }
}

impl HttpJsonRpc {
    /// Checks the settings and the address (nothing is connected yet: each request opens its own connection).
    pub fn new(cfg: HttpConfig) -> io::Result<HttpJsonRpc> {
        let addr = resolve(&cfg.host, cfg.port, cfg.allow_non_local)
            .map_err(|e| HttpError::io(e.code, e.detail))?;
        if cfg.response_wait < cfg.busy_limit {
            return Err(HttpError::io(
                E_HTTP_TIMEOUT,
                "the wait for a script is shorter than the busy limit, a move would be cut off",
            ));
        }
        let host_header = match addr.ip() {
            IpAddr::V6(a) if cfg.host.parse::<IpAddr>().is_ok() => format!("[{a}]:{}", addr.port()),
            ip if cfg.host.parse::<IpAddr>().is_ok() => format!("{ip}:{}", addr.port()),
            _ => format!("{}:{}", cfg.host, addr.port()),
        };
        let (tx, rx) = channel();
        Ok(HttpJsonRpc {
            cfg: Arc::new(cfg),
            addr,
            host_header,
            shared: Arc::new(Shared {
                closed: AtomicBool::new(false),
                epoch: AtomicU64::new(0),
                in_flight: AtomicUsize::new(0),
            }),
            tx,
            rx,
        })
    }

    /// The requests that wait for their answer now.
    pub fn in_flight(&self) -> usize {
        self.shared.in_flight.load(Ordering::SeqCst)
    }

    /// Takes a place for a request, or says that none is free. The emergency stop has two more than the others.
    fn take_place(&self, stop: bool) -> io::Result<Place> {
        let s = &self.shared;
        let limit = self.cfg.max_in_flight + if stop { STOP_EXTRA } else { 0 };
        let mut tried_to_free = false;
        let until = Instant::now() + Duration::from_millis(400);
        loop {
            if s.in_flight.fetch_add(1, Ordering::SeqCst) < limit {
                return Ok(Place(Arc::clone(s)));
            }
            s.in_flight.fetch_sub(1, Ordering::SeqCst);
            if !tried_to_free {
                // the requests that wait are abandoned (their threads end within a poll); what they were waiting for is lost
                s.epoch.fetch_add(1, Ordering::SeqCst);
                tried_to_free = true;
            }
            if Instant::now() >= until {
                return Err(HttpError::io(
                    E_HTTP_BUSY,
                    "too many requests are waiting for Moonraker",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for HttpJsonRpc {
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::SeqCst);
    }
}

impl JsonRpcTransport for HttpJsonRpc {
    fn send(&mut self, text: &str) -> io::Result<()> {
        if text.is_empty() || text.len() > MAX_REQUEST_BYTES {
            return Err(HttpError::io(E_HTTP_TOO_BIG, "the request is too large"));
        }
        let msg: Value = serde_json::from_str(text)
            .map_err(|_| HttpError::io(E_HTTP_NOT_JSON, "the request is not JSON"))?;
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let stop = method == "printer.emergency_stop";
        // a script may take as long as the machine takes to do it; anything else gets the time of the longest ordinary order
        let wait = if method == "printer.gcode.script" {
            self.cfg.response_wait
        } else {
            self.cfg.timeouts.homing.max(self.cfg.timeouts.line)
        };
        if stop {
            // whatever waits is over: the machine is being stopped
            self.shared.epoch.fetch_add(1, Ordering::SeqCst);
        }
        let place = self.take_place(stop)?;
        let job = Job {
            cfg: Arc::clone(&self.cfg),
            addr: self.addr,
            host_header: self.host_header.clone(),
            body: text.to_string(),
            wait,
            epoch: self.shared.epoch.load(Ordering::SeqCst),
        };
        let shared = Arc::clone(&self.shared);
        let tx = self.tx.clone();
        thread::Builder::new()
            .name("eh3d-http-rpc".to_string())
            .spawn(move || {
                let _place = place;
                work(job, &shared, &tx);
            })
            .map(|_| ())
            .map_err(|_| HttpError::io(E_HTTP_BUSY, "a thread for the request could not start"))
    }

    fn recv(&mut self, timeout: Duration) -> io::Result<Option<String>> {
        match self.rx.recv_timeout(timeout) {
            Ok(Event::Body(t)) => Ok(Some(t)),
            Ok(Event::Failed(e)) => Err(e),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(closed_early()),
        }
    }
}

/// The Klipper controller over HTTP with the times of the settings.
pub fn klipper_controller(cfg: HttpConfig) -> io::Result<KlipperController<HttpJsonRpc>> {
    let timeouts = cfg.timeouts;
    let busy = cfg.busy_limit;
    Ok(KlipperController::new(HttpJsonRpc::new(cfg)?)
        .with_timeouts(timeouts)
        .with_busy_limit(busy))
}
