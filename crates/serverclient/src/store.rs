//! Helpers for the project store of the server (`/api/v1/apps/{app}/projects/{id}/files/content`): the tag of a version (`ETag`), a write made from a
//! version (`If-Match`) and the two refusals that rule has, `428` (the write names no version) and `412` (the file changed since it was read), as errors of
//! their own with the text in eight languages.
//!
//! A program reads a file with [`Client::get_with_etag`], edits it, and writes it back with [`Client::put_if_match`] naming the tag it read. If somebody
//! else wrote in between, the answer is [`StoreError::PreconditionFailed`]: the program reads again and decides (merge, ask, give up). A write is sent
//! once only (it is not repeated after a time-out), because the first try may have been applied and a second one would then fail on its own tag.

use crate::client::Client;
use crate::errors::{text_for, ClientError, Lang};
use crate::json::Json;
use crate::transport::{path_segment, FullReply, RequestParts};
use std::fmt;

/// The longest tag the client sends.
pub const MAX_TAG_LEN: usize = 128;

/// The value of an `If-Match` or `If-None-Match` header for a tag: the tag between quotes. A tag that is already quoted, or has the weak mark (`W/`), is
/// taken without them; `*` stays as it is. A tag that is empty, too long or has a quote, a space or anything that is not printable ASCII is refused.
pub fn etag_header(tag: &str) -> Result<String, String> {
    let tag = parse_etag(tag);
    if tag == "*" {
        return Ok(tag);
    }
    if tag.is_empty() || tag.len() > MAX_TAG_LEN {
        return Err("a tag is empty or too long".to_string());
    }
    if !tag.bytes().all(|b| (0x21..0x7f).contains(&b) && b != b'"') {
        return Err("a tag has a character that cannot be sent".to_string());
    }
    Ok(format!("\"{tag}\""))
}

/// The tag inside the value of an `ETag` header: without the weak mark and without the quotes.
pub fn parse_etag(value: &str) -> String {
    let v = value.trim();
    let v = v.strip_prefix("W/").unwrap_or(v);
    let v = v.strip_prefix('"').unwrap_or(v);
    let v = v.strip_suffix('"').unwrap_or(v);
    v.to_string()
}

/// `/api/v1/apps/<app>/projects/<project>/files/content?name=<name>`, with every part made safe. The name goes in the query because it may have
/// slashes (`layers/top.gbr`); append `&version=2` to read an older version or `&comment=...` (use [`path_segment`]) to write a note.
pub fn project_file_path(app: &str, project: &str, name: &str) -> Result<String, String> {
    if app.is_empty()
        || app.len() > 32
        || !app
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("the application is not lower case letters, digits and hyphens".to_string());
    }
    if project.is_empty() || project.len() > 128 || project == "." || project == ".." {
        return Err("the project id is empty, too long or a dot segment".to_string());
    }
    if name.is_empty() || name.len() > 240 {
        return Err("the name of the file is empty or too long".to_string());
    }
    Ok(format!(
        "/api/v1/apps/{app}/projects/{}/files/content?name={}",
        path_segment(project),
        path_segment(name)
    ))
}

/// A file read from the store: its bytes and the tag of the version (the `ETag`), which the next write names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Versioned {
    pub status: u16,
    /// The tag of this version, when the server sent one.
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub bytes: Vec<u8>,
}

impl Versioned {
    fn from_reply(r: FullReply) -> Versioned {
        Versioned {
            status: r.status,
            etag: r.etag(),
            content_type: r.header("content-type").map(str::to_string),
            bytes: r.body,
        }
    }

    /// The bytes as text (a byte that is not UTF-8 is replaced).
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).to_string()
    }
}

/// What the store answered to a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// 201 for a new version, 200 when the bytes were the same as the current version.
    pub status: u16,
    /// The tag of the version now current (to name in the next write), when the server sent one.
    pub etag: Option<String>,
    /// True when the bytes were already the current version and no version was made.
    pub unchanged: bool,
    /// The number of the version, when the answer says it.
    pub version: Option<u64>,
    /// The body of the answer.
    pub body: String,
}

impl Written {
    fn from_reply(r: FullReply) -> Written {
        let etag = r.etag();
        let body = r.text();
        let json = Json::parse(&body).ok();
        let unchanged = json
            .as_ref()
            .and_then(|j| j.get("unchanged").and_then(Json::as_bool))
            .unwrap_or(false);
        let version = json
            .as_ref()
            .and_then(|j| j.get("version").and_then(Json::as_u64));
        Written {
            status: r.status,
            etag,
            unchanged,
            version,
            body,
        }
    }
}

/// What can go wrong with a write made from a version: the two refusals of the rule of the tag, and any other failure of the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// `428 precondition_required`: the write did not name the version it was made from (a file that exists needs `If-Match`).
    PreconditionRequired { detail: String },
    /// `412 precondition_failed`: somebody changed the file since it was read; read it again before writing.
    PreconditionFailed { detail: String },
    /// Any other failure: no login, no connection, a lock (`423`), a quota...
    Other(ClientError),
}

impl StoreError {
    /// Sorts an error of the client: a `428` or `412` (or their codes) become the errors of their own.
    pub fn from_client(error: ClientError) -> StoreError {
        if let ClientError::Server {
            status,
            code,
            detail,
        } = &error
        {
            let code = code.as_deref();
            if *status == 428 || code == Some("precondition_required") {
                return StoreError::PreconditionRequired {
                    detail: detail.clone(),
                };
            }
            if *status == 412 || code == Some("precondition_failed") {
                return StoreError::PreconditionFailed {
                    detail: detail.clone(),
                };
            }
        }
        StoreError::Other(error)
    }

    /// The code of the server, when the error came from it.
    pub fn code(&self) -> Option<&str> {
        match self {
            StoreError::PreconditionRequired { .. } => Some("precondition_required"),
            StoreError::PreconditionFailed { .. } => Some("precondition_failed"),
            StoreError::Other(e) => e.code(),
        }
    }

    /// Must the program read the file again (somebody else changed it)?
    pub fn is_conflict(&self) -> bool {
        matches!(self, StoreError::PreconditionFailed { .. })
    }

    /// The text for the person, in their language.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            StoreError::PreconditionRequired { .. } => text_for("precondition_required", lang),
            StoreError::PreconditionFailed { .. } => text_for("precondition_failed", lang),
            StoreError::Other(e) => e.text(lang),
        }
    }
}

impl From<ClientError> for StoreError {
    fn from(e: ClientError) -> StoreError {
        StoreError::from_client(e)
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text(Lang::En))
    }
}

impl std::error::Error for StoreError {}

impl Client {
    /// Reads a file of the store with the tag of its version. `path` is made by [`project_file_path`].
    pub fn get_with_etag(&mut self, path: &str) -> Result<Versioned, StoreError> {
        let parts = RequestParts::new().accept("*/*");
        let reply = self.request_full("GET", path, &parts)?;
        Ok(Versioned::from_reply(reply))
    }

    /// Writes a file of the store from the version named by `tag`: `If-Match` goes with the bytes. `None` is for a file that does not exist yet (no
    /// tag). The write is sent once. A `428` (no tag for a file that exists) and a `412` (the file changed) are [`StoreError`]s of their own.
    pub fn put_if_match(
        &mut self,
        path: &str,
        tag: Option<&str>,
        bytes: &[u8],
        content_type: Option<&str>,
    ) -> Result<Written, StoreError> {
        let mut parts = RequestParts::new().with_bytes(bytes.to_vec()).single_try();
        if let Some(t) = tag {
            parts = parts.if_match(t);
        }
        if let Some(c) = content_type {
            parts = parts.content_type(c);
        }
        let reply = self.request_full("PUT", path, &parts)?;
        Ok(Written::from_reply(reply))
    }

    /// Removes a file of the store from the version named by `tag` (its versions stay on the server).
    pub fn delete_if_match(&mut self, path: &str, tag: &str) -> Result<Written, StoreError> {
        let parts = RequestParts::new().if_match(tag).single_try();
        let reply = self.request_full("DELETE", path, &parts)?;
        Ok(Written::from_reply(reply))
    }
}
