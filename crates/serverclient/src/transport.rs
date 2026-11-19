//! How a request reaches the server. The client talks to a [`Transport`], so the tests give it a server that lives in memory and a program may give it
//! its own (a tunnel, a proxy, a platform HTTP stack); [`TcpTransport`] is the plain one every program can use.

use crate::errors::TransportError;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// One request, already complete: the path starts with `/`, the token (when there is one) is in `bearer`.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: Option<String>,
    /// The text of the token for the `Authorization: Bearer` line. It is part of the request on the wire and of nothing else: `Request` has no
    /// `Debug` output of it (nor has the body, which may hold a password).
    pub(crate) bearer: Option<String>,
    pub timeout: Duration,
    pub max_answer_bytes: u64,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("body", &self.body.as_ref().map(|_| "..."))
            .field("bearer", &self.bearer.as_ref().map(|_| "***"))
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Request {
    /// The token that goes in the `Authorization` line, for a transport that writes the request out. It must never be logged.
    pub fn bearer(&self) -> Option<&str> {
        self.bearer.as_deref()
    }
}

/// What the server answered: the status and the body as text (the body may be empty or not JSON).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    pub fn json(&self) -> Option<crate::json::Json> {
        crate::json::Json::parse(&self.body).ok()
    }
}

/// Sends a request and gives back the answer, or what went wrong on the network. A status of 4xx or 5xx is an answer, not an error here.
pub trait Transport: Send {
    fn send(&self, request: &Request) -> Result<Reply, TransportError>;
}

/// Plain HTTP/1.1, one request per connection, as the server speaks it. The token travels in the clear: use it on a network that can be trusted or
/// through a tunnel (SSH, VPN); [`plain_token_warning`] says when an address deserves a warning.
pub struct TcpTransport {
    address: String,
}

impl TcpTransport {
    pub fn new(address: impl Into<String>) -> TcpTransport {
        TcpTransport {
            address: address.into(),
        }
    }
}

/// A part of a request line or header refuses a space, a control character or anything that is not ASCII (a line break would end the line and let the
/// rest pass for another header).
pub fn check_request_part(what: &str, text: &str) -> Result<(), String> {
    if text.bytes().any(|b| b <= b' ' || b == 0x7f || b >= 0x80) {
        return Err(format!(
            "{what} has a space, a control character or a letter that cannot be sent"
        ));
    }
    Ok(())
}

/// A name (a machine, a job) as one segment of a path: everything but letters, digits and `-._~` is written as `%XX`, so a slash, a question mark, a
/// space or a line break in a name cannot change where the request goes.
pub fn path_segment(name: &str) -> String {
    let mut out = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

impl Transport for TcpTransport {
    fn send(&self, request: &Request) -> Result<Reply, TransportError> {
        let bad = |m: String| TransportError::Connect(m);
        check_request_part("the path", &request.path).map_err(bad)?;
        check_request_part("the address of the server", &self.address).map_err(bad)?;
        if let Some(t) = &request.bearer {
            check_request_part("the token", t).map_err(bad)?;
        }
        let authority = self
            .address
            .strip_prefix("http://")
            .unwrap_or(&self.address)
            .trim_end_matches('/')
            .to_string();
        let target = authority
            .to_socket_addrs()
            .map_err(|e| TransportError::Connect(format!("{authority}: {e}")))?
            .next()
            .ok_or_else(|| TransportError::Connect(format!("{authority}: no address")))?;
        let connect_limit = request.timeout.min(Duration::from_secs(5));
        let mut stream = TcpStream::connect_timeout(&target, connect_limit)
            .map_err(|e| TransportError::Connect(format!("{authority}: {e}")))?;
        let io = |e: std::io::Error| match e.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
                TransportError::Timeout
            }
            _ => TransportError::Io(e.to_string()),
        };
        stream.set_read_timeout(Some(request.timeout)).map_err(io)?;
        stream
            .set_write_timeout(Some(request.timeout))
            .map_err(io)?;
        let payload = request.body.clone().unwrap_or_default();
        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: application/json\r\n",
            request.method, request.path, authority
        );
        if let Some(t) = &request.bearer {
            head.push_str(&format!("Authorization: Bearer {t}\r\n"));
        }
        if request.body.is_some() {
            head.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                payload.len()
            ));
        } else if request.method == "POST" || request.method == "PUT" {
            head.push_str("Content-Length: 0\r\n");
        }
        head.push_str("\r\n");
        head.push_str(&payload);
        stream.write_all(head.as_bytes()).map_err(io)?;
        let limit = request.max_answer_bytes;
        let mut raw = Vec::new();
        (&mut stream)
            .take(limit.saturating_add(1))
            .read_to_end(&mut raw)
            .map_err(io)?;
        if raw.len() as u64 > limit {
            return Err(TransportError::TooLarge);
        }
        parse_response(&raw)
    }
}

/// The status and the body of an HTTP response (a body sent in chunks is put together).
pub fn parse_response(raw: &[u8]) -> Result<Reply, TransportError> {
    let cut = |m: &str| TransportError::Io(m.to_string());
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| cut("the answer of the server is incomplete"))?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| cut("the answer of the server has no status"))?;
    let body = &raw[split + 4..];
    let chunked = head.lines().any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    });
    let body = if chunked {
        unchunk(body)?
    } else {
        body.to_vec()
    };
    Ok(Reply {
        status,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn unchunk(mut rest: &[u8]) -> Result<Vec<u8>, TransportError> {
    let cut = || TransportError::Io("a chunk of the answer is cut".to_string());
    let mut out = Vec::new();
    loop {
        let eol = rest.windows(2).position(|w| w == b"\r\n").ok_or_else(cut)?;
        let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..eol]).trim(), 16)
            .map_err(|_| TransportError::Io("a chunk of the answer has a bad size".to_string()))?;
        rest = &rest[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        if rest.len() < 2 || size > rest.len() - 2 {
            return Err(cut());
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

/// True when the token would travel in the clear over a network that is not the user's own: an address that is not this computer (`127.x`, `::1`,
/// `localhost`) or a private network (`10.x`, `172.16` to `172.31`, `192.168.x`, link-local, unique-local, `.local` names). An address with `https://`
/// (a tunnel or a proxy in front of the server) and an empty address give no warning.
pub fn plain_token_warning(address: &str) -> bool {
    let text = address.trim();
    if text.is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return false;
    }
    let rest = lower.strip_prefix("http://").unwrap_or(&lower);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if let Some(inner) = authority.strip_prefix('[') {
        inner.split(']').next().unwrap_or("")
    } else if authority.matches(':').count() > 1 {
        authority
    } else {
        authority.split(':').next().unwrap_or("")
    };
    let host = host.trim_end_matches('.');
    if host.is_empty() {
        return false;
    }
    if host == "localhost" || host.ends_with(".local") || host.ends_with(".localhost") {
        return false;
    }
    if let Ok(v4) = host.parse::<std::net::Ipv4Addr>() {
        return !(v4.is_loopback() || v4.is_private() || v4.is_link_local());
    }
    if let Ok(v6) = host.parse::<std::net::Ipv6Addr>() {
        let first = v6.segments()[0];
        let unique_local = first & 0xfe00 == 0xfc00;
        let link_local = first & 0xffc0 == 0xfe80;
        return !(v6.is_loopback() || unique_local || link_local);
    }
    true
}
