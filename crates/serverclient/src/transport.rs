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
    /// A body that is not text (the bytes of a file). At most one of `body` and `body_bytes` is set; a transport that only reads `body` does not see these
    /// bytes, so a program that sends bytes gives the client a transport that reads [`Request::payload`].
    pub body_bytes: Option<Vec<u8>>,
    /// The extra header lines (`If-Match`, `Content-Type`...), already checked by [`check_headers`]. The `Authorization` line is never here: it is `bearer`.
    pub headers: Vec<(String, String)>,
    /// True when the request must not be sent again after a time-out or a broken connection even if its method could be (a write made from a version).
    pub single_try: bool,
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
            .field("body_bytes", &self.body_bytes.as_ref().map(Vec::len))
            .field(
                "headers",
                &self
                    .headers
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>(),
            )
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

    /// The value of an extra header, by name (letter case does not matter).
    pub fn header(&self, name: &str) -> Option<&str> {
        header_value(&self.headers, name)
    }

    /// The bytes that go after the header lines: the bytes when there are, else the text.
    pub fn payload(&self) -> Vec<u8> {
        match (&self.body_bytes, &self.body) {
            (Some(b), _) => b.clone(),
            (None, Some(t)) => t.as_bytes().to_vec(),
            (None, None) => Vec::new(),
        }
    }

    /// Is there a body, empty or not?
    pub fn has_payload(&self) -> bool {
        self.body.is_some() || self.body_bytes.is_some()
    }
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// The header lines a program may add to a request. Anything else (`Host`, `Content-Length`, `Transfer-Encoding`, `Connection`, `Authorization`...) is
/// written by the transport or refused, so a header cannot change how the request is framed.
pub const ALLOWED_HEADERS: [&str; 7] = [
    "accept",
    "accept-language",
    "content-type",
    "if-match",
    "if-none-match",
    "x-api-key",
    "x-request-id",
];
/// The most extra headers in one request.
pub const MAX_HEADERS: usize = 16;
/// The longest name of a header, in bytes.
pub const MAX_HEADER_NAME: usize = 40;
/// The longest value of a header, in bytes.
pub const MAX_HEADER_VALUE: usize = 512;
/// The largest body the client sends (the server has its own, smaller, limit for a file).
pub const MAX_BODY_BYTES: usize = 256 * 1_048_576;

/// Checks the extra headers of a request: only the allowed names, each once, a name of letters, digits and `-`, a value of printable ASCII (no line
/// break, no control character, nothing that is not ASCII), and the sizes bounded. The error says what is wrong and never repeats a value (it may be a key).
pub fn check_headers(headers: &[(String, String)]) -> Result<(), String> {
    if headers.len() > MAX_HEADERS {
        return Err(format!("more than {MAX_HEADERS} headers"));
    }
    for (i, (name, value)) in headers.iter().enumerate() {
        if name.is_empty()
            || name.len() > MAX_HEADER_NAME
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("a header has a name that cannot be sent".to_string());
        }
        let lower = name.to_ascii_lowercase();
        if !ALLOWED_HEADERS.contains(&lower.as_str()) {
            return Err(format!("the header {lower} is not one a program may add"));
        }
        if headers[..i]
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            return Err(format!("the header {lower} is given twice"));
        }
        if value.is_empty() || value.len() > MAX_HEADER_VALUE {
            return Err(format!("the value of {lower} is empty or too long"));
        }
        if !value.bytes().all(|b| (0x20..0x7f).contains(&b)) {
            return Err(format!(
                "the value of {lower} has a control character or a letter that cannot be sent"
            ));
        }
        if value.starts_with(' ') || value.ends_with(' ') {
            return Err(format!("the value of {lower} starts or ends with a space"));
        }
    }
    Ok(())
}

/// The extra parts of a request: header lines and a body that may be bytes. Made with the builder methods, checked by [`RequestParts::validate`] (the
/// client does it before it sends anything).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RequestParts {
    headers: Vec<(String, String)>,
    text: Option<String>,
    bytes: Option<Vec<u8>>,
    single_try: bool,
}

impl std::fmt::Debug for RequestParts {
    // the values of the headers (a key may be one) and the body are not written out
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestParts")
            .field(
                "headers",
                &self
                    .headers
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("text", &self.text.as_ref().map(String::len))
            .field("bytes", &self.bytes.as_ref().map(Vec::len))
            .field("single_try", &self.single_try)
            .finish()
    }
}

impl RequestParts {
    pub fn new() -> RequestParts {
        RequestParts::default()
    }

    /// Adds a header line (a second one of the same name is refused by `validate`).
    pub fn with_header(mut self, name: &str, value: &str) -> RequestParts {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// A text body (JSON unless `Content-Type` says otherwise). Replaces any bytes.
    pub fn with_text(mut self, body: &str) -> RequestParts {
        self.text = Some(body.to_string());
        self.bytes = None;
        self
    }

    /// A body of bytes (`application/octet-stream` unless `Content-Type` says otherwise). Replaces any text.
    pub fn with_bytes(mut self, body: Vec<u8>) -> RequestParts {
        self.bytes = Some(body);
        self.text = None;
        self
    }

    pub fn content_type(self, value: &str) -> RequestParts {
        self.with_header("Content-Type", value)
    }

    pub fn accept(self, value: &str) -> RequestParts {
        self.with_header("Accept", value)
    }

    /// `If-Match` with the tag of the version the write is made from (see [`crate::store::etag_header`]). A tag that cannot be sent leaves an empty
    /// value, which `validate` refuses.
    pub fn if_match(self, tag: &str) -> RequestParts {
        let value = crate::store::etag_header(tag).unwrap_or_default();
        self.with_header("If-Match", &value)
    }

    pub fn if_none_match(self, tag: &str) -> RequestParts {
        let value = crate::store::etag_header(tag).unwrap_or_default();
        self.with_header("If-None-Match", &value)
    }

    /// The request is not sent again after a time-out or a broken connection (the first try may have been applied).
    pub fn single_try(mut self) -> RequestParts {
        self.single_try = true;
        self
    }

    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }

    pub fn is_single_try(&self) -> bool {
        self.single_try
    }

    /// Are the headers and the body fit to be sent?
    pub fn validate(&self) -> Result<(), String> {
        check_headers(&self.headers)?;
        let size =
            self.bytes.as_ref().map_or(0, Vec::len) + self.text.as_ref().map_or(0, String::len);
        if size > MAX_BODY_BYTES {
            return Err(format!(
                "the body is larger than {} MiB",
                MAX_BODY_BYTES / 1_048_576
            ));
        }
        Ok(())
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

impl Reply {
    /// An answer with a status and a text body.
    pub fn new(status: u16, body: impl Into<String>) -> Reply {
        Reply {
            status,
            body: body.into(),
        }
    }
}

/// The whole answer: the status, the header lines (`ETag`...) and the body as bytes. A [`Reply`] is what is left when the headers are not needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullReply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl FullReply {
    /// The value of a header, by name (letter case does not matter).
    pub fn header(&self, name: &str) -> Option<&str> {
        header_value(&self.headers, name)
    }

    /// The tag of the version: the `ETag` header without its quotes and without the weak mark.
    pub fn etag(&self) -> Option<String> {
        self.header("etag").map(crate::store::parse_etag)
    }

    /// The body as text (a byte that is not UTF-8 is replaced).
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    /// The answer without its headers, with the body as text.
    pub fn into_reply(self) -> Reply {
        Reply {
            status: self.status,
            body: String::from_utf8_lossy(&self.body).into_owned(),
        }
    }
}

impl From<Reply> for FullReply {
    fn from(r: Reply) -> FullReply {
        FullReply {
            status: r.status,
            headers: Vec::new(),
            body: r.body.into_bytes(),
        }
    }
}

/// Sends a request and gives back the answer, or what went wrong on the network. A status of 4xx or 5xx is an answer, not an error here.
pub trait Transport: Send {
    fn send(&self, request: &Request) -> Result<Reply, TransportError>;

    /// The same with the headers and the body as bytes. The default calls [`Transport::send`] and has no headers, so a transport written before this
    /// existed keeps working; the transports of this crate (`TcpTransport`, `SimServer`) answer with the headers.
    fn send_full(&self, request: &Request) -> Result<FullReply, TransportError> {
        self.send(request).map(FullReply::from)
    }
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
        self.send_full(request).map(FullReply::into_reply)
    }

    fn send_full(&self, request: &Request) -> Result<FullReply, TransportError> {
        let bad = |m: String| TransportError::Connect(m);
        check_request_part("the path", &request.path).map_err(bad)?;
        check_request_part("the address of the server", &self.address).map_err(bad)?;
        if let Some(t) = &request.bearer {
            check_request_part("the token", t).map_err(bad)?;
        }
        check_headers(&request.headers).map_err(bad)?;
        if request.body.is_some() && request.body_bytes.is_some() {
            return Err(bad(
                "a request has a text body and a body of bytes".to_string()
            ));
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
        let payload = request.payload();
        let head = request_head(request, &authority, payload.len());
        let mut wire = head.into_bytes();
        wire.extend_from_slice(&payload);
        stream.write_all(&wire).map_err(io)?;
        let limit = request.max_answer_bytes;
        let mut raw = Vec::new();
        (&mut stream)
            .take(limit.saturating_add(1))
            .read_to_end(&mut raw)
            .map_err(io)?;
        if raw.len() as u64 > limit {
            return Err(TransportError::TooLarge);
        }
        parse_response_full(&raw)
    }
}

/// The request line and the header lines, ending with the empty line. The default `Accept` and `Content-Type` are written only when the program did not
/// give its own; `Content-Length` is always the transport's.
pub fn request_head(request: &Request, authority: &str, payload_len: usize) -> String {
    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        request.method, request.path, authority
    );
    if request.header("accept").is_none() {
        head.push_str("Accept: application/json\r\n");
    }
    if let Some(t) = &request.bearer {
        head.push_str(&format!("Authorization: Bearer {t}\r\n"));
    }
    for (name, value) in &request.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if request.has_payload() {
        if request.header("content-type").is_none() {
            let kind = if request.body_bytes.is_some() {
                "application/octet-stream"
            } else {
                "application/json"
            };
            head.push_str(&format!("Content-Type: {kind}\r\n"));
        }
        head.push_str(&format!("Content-Length: {payload_len}\r\n"));
    } else if request.method == "POST" || request.method == "PUT" {
        head.push_str("Content-Length: 0\r\n");
    }
    head.push_str("\r\n");
    head
}

/// The status and the body of an HTTP response (a body sent in chunks is put together).
pub fn parse_response(raw: &[u8]) -> Result<Reply, TransportError> {
    parse_response_full(raw).map(FullReply::into_reply)
}

/// The most header lines read from an answer; the rest are ignored.
const MAX_ANSWER_HEADERS: usize = 100;

/// The status, the header lines and the body of an HTTP response (a body sent in chunks is put together).
pub fn parse_response_full(raw: &[u8]) -> Result<FullReply, TransportError> {
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
    let headers: Vec<(String, String)> = head
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_string(), v.trim().to_string()))
        .filter(|(n, _)| !n.is_empty())
        .take(MAX_ANSWER_HEADERS)
        .collect();
    let body = &raw[split + 4..];
    let chunked = headers.iter().any(|(n, v)| {
        n.eq_ignore_ascii_case("transfer-encoding") && v.to_ascii_lowercase().contains("chunked")
    });
    let body = if chunked {
        unchunk(body)?
    } else {
        body.to_vec()
    };
    Ok(FullReply {
        status,
        headers,
        body,
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
