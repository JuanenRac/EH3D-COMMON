//! The errors of the link: what the server refuses (by its code, with the text in eight languages) and what the network does.

use crate::errors_table::TABLE;
use std::fmt;

/// The eight languages of the suite, in the order the rows of the table have them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    It,
    De,
    Fr,
    /// European Portuguese.
    Pt,
    Zh,
    Ja,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::It,
        Lang::De,
        Lang::Fr,
        Lang::Pt,
        Lang::Zh,
        Lang::Ja,
    ];

    fn index(self) -> usize {
        self as usize
    }

    /// The language of a tag such as `es`, `pt-PT`, `zh_CN` or `ja`; anything the suite does not have is `None`.
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let lower = tag.trim().to_ascii_lowercase();
        let base = lower.split(['-', '_']).next().unwrap_or("");
        Some(match base {
            "en" => Lang::En,
            "es" => Lang::Es,
            "it" => Lang::It,
            "de" => Lang::De,
            "fr" => Lang::Fr,
            "pt" => Lang::Pt,
            "zh" => Lang::Zh,
            "ja" => Lang::Ja,
            _ => return None,
        })
    }
}

/// One code of the catalog of the server with its status and its eight texts.
#[derive(Debug, Clone, Copy)]
pub struct ErrorEntry {
    pub code: &'static str,
    pub status: u16,
    pub(crate) texts: [&'static str; 8],
}

impl ErrorEntry {
    pub fn text(&self, lang: Lang) -> &'static str {
        self.texts[lang.index()]
    }
}

/// The entry of a code of the server, `None` for a code that is not in the catalog this crate was built with (a newer server).
pub fn lookup(code: &str) -> Option<&'static ErrorEntry> {
    TABLE
        .binary_search_by(|e| e.code.cmp(code))
        .ok()
        .map(|i| &TABLE[i])
}

/// Every code of the catalog, in order.
pub fn all_codes() -> impl Iterator<Item = &'static ErrorEntry> {
    TABLE.iter()
}

/// The text of a code in a language; a code the table does not know gives the code itself, so that a newer server is never hidden.
pub fn text_for(code: &str, lang: Lang) -> String {
    match lookup(code) {
        Some(e) => e.text(lang).to_string(),
        None => code.to_string(),
    }
}

/// What went wrong on the network, before any answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The connection could not be made: nothing was sent, so any request may be tried again.
    Connect(String),
    /// The time ran out after the request may have been sent.
    Timeout,
    /// The connection broke or the answer cannot be read after the request may have been sent.
    Io(String),
    /// The answer is larger than `server_max_answer_mib`.
    TooLarge,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Connect(m) => write!(f, "cannot connect: {m}"),
            TransportError::Timeout => f.write_str("timed out"),
            TransportError::Io(m) => write!(f, "network error: {m}"),
            TransportError::TooLarge => f.write_str("the answer is too large"),
        }
    }
}

/// What a call of the client can end with. None of them ever holds the token or the password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The settings do not allow the call (no address, no token, mDNS off...).
    Config(String),
    /// The server could not be reached, after `attempts` tries.
    Unreachable {
        error: TransportError,
        attempts: u32,
    },
    /// 401: there is no login or it expired. The token was forgotten; log in again.
    Unauthorized { code: Option<String> },
    /// 403: the role or the scope does not allow it.
    Forbidden { code: Option<String> },
    /// Any other refusal of the server, by its code.
    Server {
        status: u16,
        code: Option<String>,
        detail: String,
    },
    /// The server answered something this crate cannot read.
    BadReply(String),
}

impl ClientError {
    /// The code of the server, when the error came from it.
    pub fn code(&self) -> Option<&str> {
        match self {
            ClientError::Unauthorized { code } | ClientError::Forbidden { code } => code.as_deref(),
            ClientError::Server { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    /// The HTTP status, when the error came from an answer.
    pub fn status(&self) -> Option<u16> {
        match self {
            ClientError::Unauthorized { .. } => Some(401),
            ClientError::Forbidden { .. } => Some(403),
            ClientError::Server { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// `428 precondition_required`: a write over a file that exists did not name the version it was made from.
    pub fn is_precondition_required(&self) -> bool {
        matches!(self, ClientError::Server { status: 428, .. })
            || self.code() == Some("precondition_required")
    }

    /// `412 precondition_failed`: the file changed since it was read.
    pub fn is_precondition_failed(&self) -> bool {
        matches!(self, ClientError::Server { status: 412, .. })
            || self.code() == Some("precondition_failed")
    }

    /// Is it worth showing a new login to the person?
    pub fn needs_login(&self) -> bool {
        matches!(self, ClientError::Unauthorized { .. })
    }

    /// The text for the person, in their language: the text of the code when the server gave one the table knows, else the detail of the server or
    /// a plain sentence for the kind of failure.
    pub fn text(&self, lang: Lang) -> String {
        if let Some(code) = self.code() {
            if let Some(e) = lookup(code) {
                return e.text(lang).to_string();
            }
        }
        match self {
            ClientError::Config(m) => m.clone(),
            ClientError::Unreachable { error, .. } => {
                format!("{}: {error}", plain(lang, Plain::Unreachable))
            }
            ClientError::Unauthorized { .. } => plain(lang, Plain::Unauthorized).to_string(),
            ClientError::Forbidden { .. } => plain(lang, Plain::Forbidden).to_string(),
            ClientError::Server {
                status,
                code,
                detail,
            } => match (code, detail.is_empty()) {
                (Some(c), true) => c.clone(),
                (Some(c), false) => format!("{c}: {detail}"),
                (None, _) => format!("{status} {detail}").trim().to_string(),
            },
            ClientError::BadReply(m) => m.clone(),
        }
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text(Lang::En))
    }
}

impl std::error::Error for ClientError {}

/// The longest `why` that is kept, in characters (as long as the `detail` of the server is read).
pub const MAX_WHY_CHARS: usize = 256;

/// What the last answer of 400 or more said, in more words than a [`ClientError`] holds. `ClientError::Server` keeps its three fields so that no
/// program that builds or takes it apart has to change; the rest is here, read with [`crate::Client::last_refusal`] right after the error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The HTTP status of the answer.
    pub status: u16,
    /// The `error` code of the answer, when it had one.
    pub code: Option<String>,
    /// The reason a node or the server gives for a refusal in the field `why` (`busy`, `not_holding`, `vacuum_not_reached`), cut to
    /// [`MAX_WHY_CHARS`] characters. A `409` on a command of a node says it there.
    pub why: Option<String>,
    /// The start of the body of the answer, cut to `server_error_body_bytes` bytes, with the token and every value under a key such as `token`,
    /// `password`, `secret`, `key` or `authorization` replaced by `***`. `None` when `server_error_body_bytes` is 0 (the default) or the body is empty.
    pub body_excerpt: Option<String>,
}

impl Refusal {
    /// Does this refusal belong to the error given (the same status and the same code)? Use it before trusting `why` when other calls may have run
    /// in between.
    pub fn belongs_to(&self, error: &ClientError) -> bool {
        error.status() == Some(self.status) && error.code() == self.code.as_deref()
    }
}

enum Plain {
    Unreachable,
    Unauthorized,
    Forbidden,
}

fn plain(lang: Lang, which: Plain) -> &'static str {
    // the same words the table uses for the codes of the same kind, for the cases that have no code
    let row: [&'static str; 8] = match which {
        Plain::Unreachable => [
            "The server cannot be reached",
            "No se puede llegar al servidor",
            "Il server non è raggiungibile",
            "Der Server ist nicht erreichbar",
            "Le serveur est injoignable",
            "Não é possível chegar ao servidor",
            "无法连接到服务器",
            "サーバーに接続できません",
        ],
        Plain::Unauthorized => [
            "You are not logged in or the login expired.",
            "No has iniciado sesión o la sesión caducó.",
            "Non hai effettuato l'accesso o la sessione è scaduta.",
            "Sie sind nicht angemeldet oder die Sitzung ist abgelaufen.",
            "Vous n'êtes pas connecté ou la session a expiré.",
            "Não iniciou sessão ou a sessão expirou.",
            "未登录或登录已过期。",
            "ログインしていないか、ログインの有効期限が切れています。",
        ],
        Plain::Forbidden => [
            "Your role does not allow this.",
            "Tu rol no permite esto.",
            "Il tuo ruolo non lo permette.",
            "Ihre Rolle erlaubt dies nicht.",
            "Votre rôle ne le permet pas.",
            "O seu perfil não permite isto.",
            "您的角色不允许此操作。",
            "あなたのロールでは実行できません。",
        ],
    };
    row[lang.index()]
}

/// How much of a body is looked through for secrets before it is cut: far more than the most `server_error_body_bytes` allows, so a cut never
/// falls inside a value that has not been looked at.
const MASK_WINDOW_BYTES: usize = 65_536;

/// Names (in lower case, as a part of a longer name too) whose value is never shown.
const SECRET_NAMES: [&str; 10] = [
    "token",
    "password",
    "passwd",
    "passphrase",
    "secret",
    "key",
    "authorization",
    "bearer",
    "cookie",
    "credential",
];

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
}

fn is_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_NAMES.iter().any(|s| lower.contains(s))
}

/// The start of `body`, at most `max_bytes` bytes (cut at a character), with the secrets replaced by `***`: the `token` given (wherever it appears) and
/// the value of every name such as `token`, `password`, `secret`, `key`, `authorization`, `cookie` or `credential`, in JSON (`"password":"x"`), in
/// lines (`key = x`, `Authorization: Bearer x`) and in a query (`token=x&...`). `None` when `max_bytes` is 0 or the body is empty. It looks through at
/// most 64 KiB of the body, so a giant body costs nothing more.
pub fn body_excerpt(body: &str, max_bytes: usize, token: Option<&str>) -> Option<String> {
    if max_bytes == 0 || body.is_empty() {
        return None;
    }
    let mut end = body.len().min(MASK_WINDOW_BYTES);
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    let mut text = body[..end].to_string();
    if let Some(t) = token.filter(|t| !t.is_empty()) {
        text = text.replace(t, "***");
    }
    let masked = mask_named_values(&text);
    let mut cut = masked.len().min(max_bytes);
    while !masked.is_char_boundary(cut) {
        cut -= 1;
    }
    Some(masked[..cut].to_string())
}

fn mask_named_values(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if !is_name_char(chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && is_name_char(chars[i]) {
            i += 1;
        }
        let name: String = chars[start..i].iter().collect();
        out.extend(&chars[start..i]);
        if !is_secret_name(&name) {
            continue;
        }
        // after the name: an optional closing quote, spaces, `:` or `=`, spaces, then the value
        let mut j = i;
        if j < chars.len() && (chars[j] == '"' || chars[j] == '\'') {
            j += 1;
        }
        while j < chars.len() && chars[j] == ' ' {
            j += 1;
        }
        if j >= chars.len() || (chars[j] != ':' && chars[j] != '=') {
            continue;
        }
        j += 1;
        while j < chars.len() && chars[j] == ' ' {
            j += 1;
        }
        // what was skipped (the quote, the separator, the spaces) is kept as it was
        out.extend(&chars[i..j]);
        if j >= chars.len() {
            i = j;
            continue;
        }
        let quote = chars[j];
        if quote == '"' || quote == '\'' {
            let mut k = j + 1;
            while k < chars.len() && chars[k] != quote {
                if chars[k] == '\\' {
                    k += 1;
                }
                k += 1;
            }
            out.push(quote);
            out.push_str("***");
            if k < chars.len() {
                out.push(quote);
            }
            i = (k + 1).min(chars.len());
        } else {
            let mut k = j;
            let stop = |c: char| c.is_whitespace() || matches!(c, ',' | '}' | ']' | '&' | ';');
            while k < chars.len() && !stop(chars[k]) {
                k += 1;
            }
            let word: String = chars[j..k].iter().collect();
            // `Authorization: Bearer abc`: the scheme is a word of its own and the secret is the next one
            if matches!(word.to_ascii_lowercase().as_str(), "bearer" | "basic") {
                out.push_str(&word);
                out.push(' ');
                let mut m = k;
                while m < chars.len() && chars[m] == ' ' {
                    m += 1;
                }
                let mut n = m;
                while n < chars.len() && !stop(chars[n]) {
                    n += 1;
                }
                out.push_str("***");
                i = n;
            } else {
                out.push_str("***");
                i = k;
            }
        }
    }
    out
}
