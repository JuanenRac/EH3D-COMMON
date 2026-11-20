//! The settings of the server, the state of the link and the login window as a model with no window in it: the same logic for every machine program, whatever
//! draws it (`server_gui` draws it with egui when the feature `gui` is on).
//!
//! * [`ServerForm`]: the six keys the person sees (`server_address`, `server_token` (never shown), `server_machine`, `server_timeout_s`, `server_retries`,
//!   `server_mdns`) as text in boxes, checked against the same ranges as `eh3d_serverclient::Settings`, plus the shape of an address.
//! * [`LinkTracker`] and [`LinkState`]: connected, disconnected, session expired, no session, not checked yet, no server set.
//! * [`LoginDialog`]: the name and password (the password is wiped as soon as the call is made), the application the program is, the error of the last try.
//! * [`error_text`]: any failure of the client in the language of the person, with the text of the code of the server when there is one.
//! * [`LinkBadge`]: the small indicator of the link (dot, word, hint) that any window can put in its corner.
//! * [`SessionsView`]: the open logins of the person, to list and to close one by one (the one in use asks for a "yes" first).
//! * [`ServerPanel`]: the parts above and the [`Action`]s a window sends it; it keeps the last [`Notice`].

use crate::server_texts::{range_text, tr, ServerText};
use crate::texts::Lang;
use eh3d_serverclient as sc;
use sc::{Client, ClientError, Health, Link, Session, SessionInfo, Settings};

/// The limits of the numbers of the form (the same as the ones of `Settings`).
pub const TIMEOUT_RANGE: (u64, u64) = (1, 120);
pub const RETRIES_RANGE: (u64, u64) = (0, 5);

/// A box of the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Address,
    Token,
    Machine,
    TimeoutS,
    Retries,
    Mdns,
}

impl Field {
    pub const ALL: [Field; 6] = [
        Field::Address,
        Field::Token,
        Field::Machine,
        Field::TimeoutS,
        Field::Retries,
        Field::Mdns,
    ];

    /// The key of the machine file.
    pub fn key(self) -> &'static str {
        match self {
            Field::Address => "server_address",
            Field::Token => "server_token",
            Field::Machine => "server_machine",
            Field::TimeoutS => "server_timeout_s",
            Field::Retries => "server_retries",
            Field::Mdns => "server_mdns",
        }
    }

    pub fn label(self) -> ServerText {
        match self {
            Field::Address => ServerText::FieldAddress,
            Field::Token => ServerText::FieldToken,
            Field::Machine => ServerText::FieldMachine,
            Field::TimeoutS => ServerText::FieldTimeout,
            Field::Retries => ServerText::FieldRetries,
            Field::Mdns => ServerText::FieldMdns,
        }
    }
}

/// What is wrong with a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// A space or a control character where there must be none.
    Spaces,
    /// Not `host:port`.
    BadAddress,
    /// Not a whole number inside the range.
    Range { lo: u64, hi: u64 },
}

/// A box and what is wrong with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Issue {
    pub field: Field,
    pub problem: Problem,
}

impl Issue {
    /// `Label: what is wrong`, in the language of the person.
    pub fn text(&self, lang: Lang) -> String {
        let what = match self.problem {
            Problem::Spaces => tr(lang, ServerText::ProblemSpaces).to_string(),
            Problem::BadAddress => tr(lang, ServerText::ProblemAddress).to_string(),
            Problem::Range { lo, hi } => range_text(lang, lo, hi),
        };
        format!("{}: {}", tr(lang, self.field.label()), what)
    }
}

/// What the form does about the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    /// There is none and nothing was typed.
    Empty,
    /// There is one, kept as it is (it is never shown).
    Kept,
    /// A new one was typed.
    Replaced,
    /// The saved one will be removed on apply.
    WillRemove,
}

fn has_blank_or_control(text: &str) -> bool {
    text.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// `host:port`, `[v6]:port`, with or without `http://`; the port 1 to 65535.
fn address_ok(address: &str) -> bool {
    if address.is_empty() {
        return true;
    }
    let rest = address
        .strip_prefix("http://")
        .unwrap_or(address)
        .trim_end_matches('/');
    let (host, port) = if let Some(v6) = rest.strip_prefix('[') {
        match v6.split_once("]:") {
            Some((h, p)) => (h, p),
            None => return false,
        }
    } else {
        match rest.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') => (h, p),
            _ => return false,
        }
    };
    !host.is_empty() && matches!(port.parse::<u16>(), Ok(p) if p != 0)
}

fn in_range(text: &str, (lo, hi): (u64, u64)) -> bool {
    matches!(text.trim().parse::<u64>(), Ok(v) if (lo..=hi).contains(&v))
}

/// The six boxes of the settings of the server.
#[derive(Clone)]
pub struct ServerForm {
    pub address: String,
    /// What the person types for a new token. It starts empty: the saved token is never copied here.
    pub token_input: String,
    pub machine: String,
    pub timeout_s: String,
    pub retries: String,
    pub mdns: bool,
    token_was_set: bool,
    remove_token: bool,
    original: Settings,
}

impl std::fmt::Debug for ServerForm {
    // the token the person is typing is not written out
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerForm")
            .field("address", &self.address)
            .field("token_state", &self.token_state())
            .field("machine", &self.machine)
            .field("timeout_s", &self.timeout_s)
            .field("retries", &self.retries)
            .field("mdns", &self.mdns)
            .finish()
    }
}

impl ServerForm {
    pub fn from_settings(settings: &Settings) -> ServerForm {
        ServerForm {
            address: settings.server_address.clone(),
            token_input: String::new(),
            machine: settings.server_machine.clone(),
            timeout_s: settings.server_timeout_s.to_string(),
            retries: settings.server_retries.to_string(),
            mdns: settings.server_mdns,
            token_was_set: !settings.server_token.is_empty(),
            remove_token: false,
            original: settings.clone(),
        }
    }

    /// Throws the changes away.
    pub fn reset(&mut self) {
        *self = ServerForm::from_settings(&self.original.clone());
    }

    pub fn token_state(&self) -> TokenState {
        if !self.token_input.is_empty() {
            TokenState::Replaced
        } else if self.remove_token {
            TokenState::WillRemove
        } else if self.token_was_set {
            TokenState::Kept
        } else {
            TokenState::Empty
        }
    }

    /// Marks the saved token to be removed on apply (typing a new one cancels it).
    pub fn remove_token(&mut self) {
        self.token_input.clear();
        self.remove_token = self.token_was_set;
    }

    /// Everything wrong with the boxes, in the order of the form.
    pub fn issues(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        let address = self.address.trim();
        if has_blank_or_control(address) {
            out.push(Issue {
                field: Field::Address,
                problem: Problem::Spaces,
            });
        } else if !address_ok(address) {
            out.push(Issue {
                field: Field::Address,
                problem: Problem::BadAddress,
            });
        }
        if has_blank_or_control(self.token_input.trim()) {
            out.push(Issue {
                field: Field::Token,
                problem: Problem::Spaces,
            });
        }
        if !in_range(&self.timeout_s, TIMEOUT_RANGE) {
            out.push(Issue {
                field: Field::TimeoutS,
                problem: Problem::Range {
                    lo: TIMEOUT_RANGE.0,
                    hi: TIMEOUT_RANGE.1,
                },
            });
        }
        if !in_range(&self.retries, RETRIES_RANGE) {
            out.push(Issue {
                field: Field::Retries,
                problem: Problem::Range {
                    lo: RETRIES_RANGE.0,
                    hi: RETRIES_RANGE.1,
                },
            });
        }
        out
    }

    pub fn is_valid(&self) -> bool {
        self.issues().is_empty()
    }

    /// The problem of one box, if it has one.
    pub fn issue_of(&self, field: Field) -> Option<Issue> {
        self.issues().into_iter().find(|i| i.field == field)
    }

    /// The settings the boxes make: the saved ones with the boxes written over them (so the keys the form does not show, `server_max_answer_mib`, stay).
    /// The token stays as it was unless a new one was typed or its removal was asked.
    pub fn build(&self) -> Result<Settings, Vec<Issue>> {
        let issues = self.issues();
        if !issues.is_empty() {
            return Err(issues);
        }
        let mut s = self.original.clone();
        let mut set = |field: Field, value: &str| -> Result<(), Issue> {
            s.set(field.key(), value).map(|_| ()).map_err(|_| Issue {
                field,
                problem: Problem::Spaces,
            })
        };
        let mut fail: Vec<Issue> = Vec::new();
        let mut push = |r: Result<(), Issue>| {
            if let Err(i) = r {
                fail.push(i);
            }
        };
        push(set(Field::Address, self.address.trim()));
        push(set(Field::Machine, self.machine.trim()));
        push(set(Field::TimeoutS, self.timeout_s.trim()));
        push(set(Field::Retries, self.retries.trim()));
        push(set(Field::Mdns, if self.mdns { "on" } else { "off" }));
        match self.token_state() {
            TokenState::Replaced => push(set(Field::Token, self.token_input.trim())),
            TokenState::WillRemove => push(set(Field::Token, "")),
            TokenState::Kept | TokenState::Empty => {}
        }
        if fail.is_empty() {
            Ok(s)
        } else {
            Err(fail)
        }
    }

    /// Are there changes to apply (or boxes that cannot be applied)?
    pub fn is_dirty(&self) -> bool {
        match self.build() {
            Ok(s) => s != self.original,
            Err(_) => true,
        }
    }
}

/// How the link looks to the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    NotConfigured,
    Unknown,
    Connected,
    Disconnected,
    SessionExpired,
    NoSession,
}

/// How much attention a state asks for (a window picks its colours).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Neutral,
    Warning,
    Bad,
}

impl LinkState {
    /// Every state, in the order a legend would list them.
    pub const ALL: [LinkState; 6] = [
        LinkState::Connected,
        LinkState::Unknown,
        LinkState::NotConfigured,
        LinkState::NoSession,
        LinkState::SessionExpired,
        LinkState::Disconnected,
    ];

    pub fn text(self) -> ServerText {
        match self {
            LinkState::NotConfigured => ServerText::LinkNotSet,
            LinkState::Unknown => ServerText::LinkUnknown,
            LinkState::Connected => ServerText::LinkConnected,
            LinkState::Disconnected => ServerText::LinkDisconnected,
            LinkState::SessionExpired => ServerText::LinkSessionExpired,
            LinkState::NoSession => ServerText::LinkNoSession,
        }
    }

    pub fn tone(self) -> Tone {
        match self {
            LinkState::Connected => Tone::Good,
            LinkState::NotConfigured | LinkState::Unknown => Tone::Neutral,
            LinkState::SessionExpired | LinkState::NoSession => Tone::Warning,
            LinkState::Disconnected => Tone::Bad,
        }
    }

    /// Does the person have to log in?
    pub fn needs_login(self) -> bool {
        matches!(self, LinkState::SessionExpired | LinkState::NoSession)
    }
}

/// A small indicator of the link for the corner of any window: a coloured dot and a word, with a longer text for the hover. The same for every program,
/// whatever draws it (`server_gui::link_badge` draws it with egui when the feature `gui` is on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkBadge {
    state: LinkState,
}

impl LinkBadge {
    pub fn new(state: LinkState) -> LinkBadge {
        LinkBadge { state }
    }

    /// The badge of what the client says now (the tracker keeps the memory of a login).
    pub fn observe(tracker: &mut LinkTracker, client: &Client) -> LinkBadge {
        LinkBadge::new(tracker.observe(client))
    }

    /// One badge for each state, to show a legend or to test a drawing.
    pub fn all() -> [LinkBadge; 6] {
        LinkState::ALL.map(LinkBadge::new)
    }

    pub fn state(self) -> LinkState {
        self.state
    }

    pub fn tone(self) -> Tone {
        self.state.tone()
    }

    /// The word beside the dot.
    pub fn label(self, lang: Lang) -> &'static str {
        tr(lang, self.state.text())
    }

    /// What the person can do about it, when there is something: log in, set the address, test the connection.
    pub fn hint(self, lang: Lang) -> Option<&'static str> {
        match self.state {
            LinkState::SessionExpired | LinkState::NoSession => Some(tr(lang, ServerText::Login)),
            LinkState::NotConfigured => Some(tr(lang, ServerText::FieldAddress)),
            LinkState::Disconnected => Some(tr(lang, ServerText::TestConnection)),
            LinkState::Connected | LinkState::Unknown => None,
        }
    }

    /// `Link: Disconnected - Test the connection`, for the hover.
    pub fn tooltip(self, lang: Lang) -> String {
        let mut text = format!("{}: {}", tr(lang, ServerText::LinkLabel), self.label(lang));
        if let Some(hint) = self.hint(lang) {
            text.push_str(" - ");
            text.push_str(hint);
        }
        text
    }

    /// Does the state ask for the attention of the person (a warning or a failure)?
    pub fn is_alert(self) -> bool {
        matches!(self.tone(), Tone::Warning | Tone::Bad)
    }
}

/// The open logins of the person (`GET /auth/sessions`) as a list that can be refreshed and closed one by one. The model has no window in it; the
/// window asks for [`Action`]s and the panel does them.
#[derive(Debug, Clone, Default)]
pub struct SessionsView {
    /// Is the list shown?
    pub open: bool,
    rows: Vec<SessionInfo>,
    error: Option<ClientError>,
    loaded: bool,
    pending: Option<String>,
    confirming: bool,
}

impl SessionsView {
    /// The open sessions, the one in use first.
    pub fn rows(&self) -> &[SessionInfo] {
        &self.rows
    }

    /// Has the list been read from the server (even when it is empty)?
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Why the last reading failed, if it did; the list shown is the last one that was read.
    pub fn error(&self) -> Option<&ClientError> {
        self.error.as_ref()
    }

    pub fn error_text(&self, lang: Lang) -> Option<String> {
        self.error.as_ref().map(|e| error_text(lang, e))
    }

    /// The session the person asked to close, if one waits.
    pub fn pending(&self) -> Option<&str> {
        self.pending.as_deref()
    }

    /// Does the one asked for need a "yes, close it" first (it is the session in use)?
    pub fn is_confirming(&self) -> bool {
        self.confirming
    }

    /// Reads the list again. A failure keeps the last list and the reason.
    pub fn load(&mut self, client: &mut Client) -> bool {
        match client.sessions() {
            Ok(mut rows) => {
                rows.sort_by_key(|r| !r.current);
                self.rows = rows;
                self.error = None;
                self.loaded = true;
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }

    /// Marks a session to close (the window then sends `Action::CloseSession`). An id the list does not have is ignored.
    pub fn ask_close(&mut self, id: &str) {
        if self.rows.iter().any(|r| r.id == id) {
            self.pending = Some(id.to_string());
            self.confirming = false;
        }
    }

    pub fn cancel_close(&mut self) {
        self.pending = None;
        self.confirming = false;
    }

    /// Forgets the list (the person logged out).
    pub fn clear(&mut self) {
        *self = SessionsView::default();
    }
}

/// What closing a session came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseOutcome {
    /// No session was waiting to be closed.
    Nothing,
    /// It is the session in use: the person has to say yes first.
    NeedsConfirm,
    /// Another session was closed.
    Closed,
    /// The session in use was closed: the person is logged out.
    ClosedOwn,
    /// The server did not do it; the notice says why.
    Failed,
}

/// Turns what the client says into what the person sees; it remembers whether there was a login, to tell an expired session from no session.
#[derive(Debug, Clone, Default)]
pub struct LinkTracker {
    had_login: bool,
}

impl LinkTracker {
    pub fn observe(&mut self, client: &Client) -> LinkState {
        if !client.settings().is_configured() {
            self.had_login = false;
            return LinkState::NotConfigured;
        }
        if client.has_token() {
            self.had_login = true;
        }
        let lost = |had: bool| {
            if had {
                LinkState::SessionExpired
            } else {
                LinkState::NoSession
            }
        };
        match client.link() {
            Link::Disconnected => LinkState::Disconnected,
            Link::LoggedOut => lost(self.had_login),
            Link::Connected if !client.has_token() => lost(self.had_login),
            Link::Connected => LinkState::Connected,
            Link::Unknown if !client.has_token() => LinkState::NoSession,
            Link::Unknown => LinkState::Unknown,
        }
    }

    /// The person ended the login on purpose: what follows is no session, not an expired one.
    pub fn forget_login(&mut self) {
        self.had_login = false;
    }

    /// A login just worked.
    pub fn note_login(&mut self) {
        self.had_login = true;
    }
}

/// The language of the table of errors of the client for a language of the interface (the variants go to their main language, the rest to English).
pub fn client_lang(lang: Lang) -> sc::Lang {
    sc::Lang::from_tag(lang.code()).unwrap_or(sc::Lang::En)
}

/// Any failure of the client in the language of the person.
pub fn error_text(lang: Lang, error: &ClientError) -> String {
    match error {
        ClientError::Config(message) => {
            let which = if message.starts_with("server_address") {
                Some(ServerText::ErrNoAddress)
            } else if message.starts_with("server_machine") {
                Some(ServerText::ErrNoMachine)
            } else if message.starts_with("server_mdns") {
                Some(ServerText::ErrMdnsOff)
            } else if message.starts_with("no mDNS") {
                Some(ServerText::ErrNoResolver)
            } else {
                None
            };
            which.map_or_else(|| message.clone(), |t| tr(lang, t).to_string())
        }
        ClientError::BadReply(_) => tr(lang, ServerText::ErrBadReply).to_string(),
        ClientError::Unreachable { attempts, .. } => {
            let full = error.text(client_lang(lang));
            // the text of the client is "<sentence>: <technical detail>"; the person gets the sentence in their language and the number of tries
            let sentence = full.split_once(": ").map_or(full.as_str(), |(s, _)| s);
            format!("{sentence} ({attempts} {})", tr(lang, ServerText::Attempts))
        }
        other => other.text(client_lang(lang)),
    }
}

/// The login window.
#[derive(Clone)]
pub struct LoginDialog {
    pub name: String,
    pub password: String,
    /// The application the program is (`phoenix-cnc`, `bungard-gui`...), sent with the login.
    pub app: String,
    pub open: bool,
    error: Option<ClientError>,
}

impl std::fmt::Debug for LoginDialog {
    // the password is not written out
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginDialog")
            .field("name", &self.name)
            .field("password", &"***")
            .field("app", &self.app)
            .field("open", &self.open)
            .field("error", &self.error)
            .finish()
    }
}

impl LoginDialog {
    pub fn new(app: &str) -> LoginDialog {
        LoginDialog {
            name: String::new(),
            password: String::new(),
            app: app.to_string(),
            open: false,
            error: None,
        }
    }

    pub fn can_submit(&self) -> bool {
        !self.name.trim().is_empty() && !self.password.is_empty() && !self.app.is_empty()
    }

    /// Overwrites the password in memory before it is dropped.
    pub fn wipe_password(&mut self) {
        let blank = "\0".repeat(self.password.len());
        self.password.replace_range(.., &blank);
        self.password.clear();
    }

    /// Logs in. The password is wiped whatever the outcome; a failure stays for the window to show.
    pub fn submit(&mut self, client: &mut Client) -> Option<Session> {
        if !self.can_submit() {
            return None;
        }
        let result = client.login(self.name.trim(), &self.password, &self.app);
        self.wipe_password();
        match result {
            Ok(session) => {
                self.error = None;
                Some(session)
            }
            Err(e) => {
                self.error = Some(e);
                None
            }
        }
    }

    pub fn error(&self) -> Option<&ClientError> {
        self.error.as_ref()
    }

    pub fn error_text(&self, lang: Lang) -> Option<String> {
        self.error.as_ref().map(|e| error_text(lang, e))
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

/// What a window asks of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Apply,
    Undo,
    TestConnection,
    RemoveToken,
    OpenLogin,
    SubmitLogin,
    CancelLogin,
    Logout,
    /// Show the list of open sessions and read it.
    OpenSessions,
    /// Hide the list.
    HideSessions,
    /// Read the list again.
    RefreshSessions,
    /// Close the session marked with `SessionsView::ask_close` (the one in use asks for a "yes" first).
    CloseSession,
    /// The person said yes to closing the session in use.
    ConfirmCloseSession,
    /// The person changed their mind.
    CancelCloseSession,
}

/// The last thing worth telling the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    None,
    Applied,
    Answered(Health),
    Failed(ClientError),
    Invalid(Vec<Issue>),
    /// A session was closed.
    SessionClosed,
}

/// The panel: the form, the login window, the state of the link and the last notice.
#[derive(Debug, Clone)]
pub struct ServerPanel {
    pub form: ServerForm,
    pub login: LoginDialog,
    /// The list of open sessions.
    pub sessions: SessionsView,
    pub notice: Notice,
    tracker: LinkTracker,
    state: LinkState,
    who: Option<Session>,
}

impl ServerPanel {
    /// `app` is the application the program is, as the server knows it (`phoenix-cnc`, `drillpro`, `bungard-gui`...).
    pub fn new(settings: &Settings, app: &str) -> ServerPanel {
        ServerPanel {
            form: ServerForm::from_settings(settings),
            login: LoginDialog::new(app),
            sessions: SessionsView::default(),
            notice: Notice::None,
            tracker: LinkTracker::default(),
            state: LinkState::Unknown,
            who: None,
        }
    }

    pub fn state(&self) -> LinkState {
        self.state
    }

    /// Who is logged in, if the client has a login made here.
    pub fn who(&self) -> Option<&Session> {
        self.who.as_ref()
    }

    /// Reads the state of the client again (a window calls it every frame; it only reads).
    pub fn refresh(&mut self, client: &Client) -> LinkState {
        self.state = self.tracker.observe(client);
        if let Some(s) = client.session() {
            self.who = Some(s.clone());
        } else if !client.has_token() {
            self.who = None;
        }
        if !client.has_token() {
            // a list of logins is of a person who is logged in
            self.sessions.clear();
        }
        self.state
    }

    /// The badge of the link as it is now, for the corner of a window.
    pub fn badge(&self) -> LinkBadge {
        LinkBadge::new(self.state)
    }

    /// Closes the session marked with `sessions.ask_close`. The one in use asks for a "yes" first (`confirmed` false gives `NeedsConfirm` and the window
    /// shows the question); closing it logs the person out. Another session is closed on the server and the list is read again.
    pub fn close_pending_session(&mut self, client: &mut Client, confirmed: bool) -> CloseOutcome {
        let Some(id) = self.sessions.pending.clone() else {
            return CloseOutcome::Nothing;
        };
        let own = self.sessions.rows.iter().any(|r| r.id == id && r.current);
        if own && !confirmed {
            self.sessions.confirming = true;
            return CloseOutcome::NeedsConfirm;
        }
        self.sessions.cancel_close();
        if own {
            let result = client.logout();
            self.tracker.forget_login();
            self.who = None;
            self.sessions.clear();
            return match result {
                Ok(()) => {
                    self.notice = Notice::SessionClosed;
                    CloseOutcome::ClosedOwn
                }
                Err(e) => {
                    self.notice = Notice::Failed(e);
                    CloseOutcome::Failed
                }
            };
        }
        match client.close_session(&id) {
            Ok(()) => {
                self.notice = Notice::SessionClosed;
                self.sessions.load(client);
                CloseOutcome::Closed
            }
            Err(e) => {
                self.notice = Notice::Failed(e);
                CloseOutcome::Failed
            }
        }
    }

    /// Does what the window asked. Gives the new settings when the person applied them: the program saves them to its file and makes a new client.
    pub fn handle(&mut self, action: Action, client: &mut Client) -> Option<Settings> {
        let mut applied = None;
        match action {
            Action::Apply => match self.form.build() {
                Ok(settings) => {
                    self.form = ServerForm::from_settings(&settings);
                    self.notice = Notice::Applied;
                    applied = Some(settings);
                }
                Err(issues) => self.notice = Notice::Invalid(issues),
            },
            Action::Undo => {
                self.form.reset();
                self.notice = Notice::None;
            }
            Action::RemoveToken => self.form.remove_token(),
            Action::TestConnection => {
                self.notice = match client.health() {
                    Ok(h) => Notice::Answered(h),
                    Err(e) => Notice::Failed(e),
                };
            }
            Action::OpenLogin => {
                self.login.clear_error();
                self.login.open = true;
            }
            Action::CancelLogin => {
                self.login.wipe_password();
                self.login.clear_error();
                self.login.open = false;
            }
            Action::SubmitLogin => {
                if self.login.submit(client).is_some() {
                    self.login.open = false;
                    self.tracker.note_login();
                    self.notice = Notice::None;
                }
            }
            Action::Logout => {
                self.notice = match client.logout() {
                    Ok(()) => Notice::None,
                    Err(e) => Notice::Failed(e),
                };
                self.tracker.forget_login();
                self.who = None;
                self.sessions.clear();
            }
            Action::OpenSessions => {
                self.sessions.open = true;
                self.sessions.load(client);
            }
            Action::RefreshSessions => {
                self.sessions.load(client);
            }
            Action::HideSessions => {
                self.sessions.open = false;
                self.sessions.cancel_close();
            }
            Action::CloseSession => {
                self.close_pending_session(client, false);
            }
            Action::ConfirmCloseSession => {
                self.close_pending_session(client, true);
            }
            Action::CancelCloseSession => self.sessions.cancel_close(),
        }
        self.refresh(client);
        applied
    }

    /// The notice in words (empty for none).
    pub fn notice_text(&self, lang: Lang) -> String {
        match &self.notice {
            Notice::None => String::new(),
            Notice::Applied => tr(lang, ServerText::Applied).to_string(),
            Notice::Answered(h) => {
                let mut text = tr(lang, ServerText::HealthOk).to_string();
                if !h.name.is_empty() || !h.version.is_empty() {
                    text.push_str(format!(": {} {}", h.name, h.version).trim_end());
                }
                text
            }
            Notice::Failed(e) => error_text(lang, e),
            Notice::SessionClosed => tr(lang, ServerText::SessionClosed).to_string(),
            Notice::Invalid(issues) => issues
                .iter()
                .map(|i| i.text(lang))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::sim::{Fault, SimServer};

    fn settings() -> Settings {
        let mut s = Settings::default();
        s.set("server_address", "192.168.0.180:18200").unwrap();
        s.set("server_machine", "bungard").unwrap();
        s
    }

    fn client(sim: &SimServer) -> Client {
        Client::new(settings(), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()))
            .with_clock(Box::new(sim.clock()))
    }

    fn typed(panel: &mut ServerPanel, name: &str, password: &str) {
        panel.login.name = name.to_string();
        panel.login.password = password.to_string();
    }

    #[test]
    fn the_form_starts_as_the_settings_and_is_not_dirty() {
        let f = ServerForm::from_settings(&settings());
        assert_eq!(f.address, "192.168.0.180:18200");
        assert_eq!(f.machine, "bungard");
        assert_eq!(f.timeout_s, "15");
        assert_eq!(f.retries, "2");
        assert!(!f.mdns);
        assert!(f.is_valid() && !f.is_dirty());
        assert_eq!(f.build().unwrap(), settings());
    }

    #[test]
    fn every_number_is_checked_against_its_range() {
        let mut f = ServerForm::from_settings(&settings());
        for (value, good) in [
            ("0", false),
            ("1", true),
            ("120", true),
            ("121", false),
            ("-3", false),
            ("abc", false),
            ("", false),
            (" 15 ", true),
            ("1.5", false),
        ] {
            f.timeout_s = value.to_string();
            assert_eq!(f.issue_of(Field::TimeoutS).is_none(), good, "{value:?}");
        }
        f.timeout_s = "15".to_string();
        for (value, good) in [("0", true), ("5", true), ("6", false), ("x", false)] {
            f.retries = value.to_string();
            assert_eq!(f.issue_of(Field::Retries).is_none(), good, "{value:?}");
        }
    }

    #[test]
    fn the_ranges_of_the_form_are_the_ranges_of_the_settings() {
        let mut s = Settings::default();
        for (key, range) in [
            ("server_timeout_s", TIMEOUT_RANGE),
            ("server_retries", RETRIES_RANGE),
        ] {
            assert!(s.set(key, &range.0.to_string()).is_ok(), "{key}");
            assert!(s.set(key, &range.1.to_string()).is_ok(), "{key}");
            assert!(s.set(key, &(range.1 + 1).to_string()).is_err(), "{key}");
            if range.0 > 0 {
                assert!(s.set(key, &(range.0 - 1).to_string()).is_err(), "{key}");
            }
        }
    }

    #[test]
    fn an_address_must_be_host_and_port() {
        for (a, good) in [
            ("", true),
            ("192.168.0.180:18200", true),
            ("cm5.local:18200", true),
            ("http://cm5.local:18200/", true),
            ("[::1]:18200", true),
            ("192.168.0.180", false),
            (":18200", false),
            ("host:0", false),
            ("host:65536", false),
            ("host:port", false),
            ("::1:18200", false),
            ("[::1]", false),
            ("host name:1", false),
            ("host\n:1", false),
        ] {
            let mut f = ServerForm::from_settings(&Settings::default());
            f.address = a.to_string();
            assert_eq!(f.issue_of(Field::Address).is_none(), good, "{a:?}");
        }
    }

    #[test]
    fn a_blank_in_the_token_is_refused() {
        let mut f = ServerForm::from_settings(&settings());
        f.token_input = "abc def".to_string();
        assert_eq!(
            f.issue_of(Field::Token).map(|i| i.problem),
            Some(Problem::Spaces)
        );
        f.token_input = "abcdef".to_string();
        assert!(f.issue_of(Field::Token).is_none());
    }

    #[test]
    fn the_saved_token_is_never_copied_into_the_form_and_is_kept_unless_changed() {
        let mut s = settings();
        s.set("server_token", "very-secret-token").unwrap();
        let mut f = ServerForm::from_settings(&s);
        assert_eq!(f.token_input, "");
        assert_eq!(f.token_state(), TokenState::Kept);
        assert!(
            format!("{f:?}").contains("***") || !format!("{f:?}").contains("very-secret-token")
        );
        f.machine = "other".to_string();
        let built = f.build().unwrap();
        assert_eq!(built.server_token.expose(), "very-secret-token");
        assert_eq!(built.server_machine, "other");

        f.token_input = "new-token".to_string();
        assert_eq!(f.token_state(), TokenState::Replaced);
        assert_eq!(f.build().unwrap().server_token.expose(), "new-token");

        f.token_input.clear();
        f.remove_token();
        assert_eq!(f.token_state(), TokenState::WillRemove);
        assert!(f.build().unwrap().server_token.is_empty());
        assert!(f.is_dirty());

        f.reset();
        assert_eq!(f.token_state(), TokenState::Kept);
        assert!(!f.is_dirty());
    }

    #[test]
    fn removing_a_token_that_is_not_there_changes_nothing() {
        let mut f = ServerForm::from_settings(&settings());
        f.remove_token();
        assert_eq!(f.token_state(), TokenState::Empty);
        assert!(!f.is_dirty());
    }

    #[test]
    fn the_keys_the_form_does_not_show_are_kept() {
        let mut s = settings();
        s.set("server_max_answer_mib", "64").unwrap();
        let mut f = ServerForm::from_settings(&s);
        f.mdns = true;
        let built = f.build().unwrap();
        assert_eq!(built.server_max_answer_mib, 64);
        assert!(built.server_mdns);
    }

    #[test]
    fn the_problems_are_written_in_each_language_with_the_name_of_the_box() {
        let mut f = ServerForm::from_settings(&settings());
        f.timeout_s = "500".to_string();
        let issue = f.issue_of(Field::TimeoutS).unwrap();
        for l in Lang::ALL.into_iter().take(8) {
            let t = issue.text(l);
            assert!(t.contains("120") && t.contains('1'), "{l:?}: {t}");
            assert!(t.starts_with(tr(l, ServerText::FieldTimeout)), "{l:?}");
            assert!(!t.contains('{'));
        }
    }

    #[test]
    fn a_server_that_is_there_and_a_login_that_works_show_connected() {
        let sim = SimServer::new();
        let mut c = client(&sim);
        let mut p = ServerPanel::new(&settings(), "phoenix-cnc");
        assert_eq!(p.refresh(&c), LinkState::NoSession);
        p.handle(Action::TestConnection, &mut c);
        assert!(matches!(p.notice, Notice::Answered(_)));
        assert!(p
            .notice_text(Lang::En)
            .starts_with("The server answered: eh3d-server"));
        assert_eq!(p.state(), LinkState::NoSession);

        p.handle(Action::OpenLogin, &mut c);
        assert!(p.login.open);
        typed(&mut p, "admin", "secret-password");
        p.handle(Action::SubmitLogin, &mut c);
        assert!(!p.login.open);
        assert_eq!(p.login.password, "");
        assert_eq!(p.state(), LinkState::Connected);
        assert_eq!(p.who().map(|s| s.name.as_str()), Some("admin"));
        assert!(!p.state().needs_login());
    }

    #[test]
    fn a_wrong_password_shows_the_error_in_the_language_and_wipes_the_password() {
        let sim = SimServer::new();
        let mut c = client(&sim);
        let mut p = ServerPanel::new(&settings(), "phoenix-cnc");
        p.handle(Action::OpenLogin, &mut c);
        typed(&mut p, "admin", "wrong");
        p.handle(Action::SubmitLogin, &mut c);
        assert!(p.login.open, "the window stays open for another try");
        assert_eq!(p.login.password, "");
        let es = p.login.error_text(Lang::Es).unwrap();
        let en = p.login.error_text(Lang::En).unwrap();
        let de = p.login.error_text(Lang::De).unwrap();
        assert_ne!(es, en);
        assert_ne!(de, en);
        assert!(!en.contains("wrong"));
        assert_eq!(p.state(), LinkState::NoSession);
        p.handle(Action::CancelLogin, &mut c);
        assert!(!p.login.open && p.login.error().is_none());
    }

    #[test]
    fn a_login_that_runs_out_is_an_expired_session_and_logging_out_is_no_session() {
        let sim = SimServer::new();
        let mut c = client(&sim);
        let mut p = ServerPanel::new(&settings(), "phoenix-cnc");
        typed(&mut p, "admin", "secret-password");
        p.handle(Action::SubmitLogin, &mut c);
        assert_eq!(p.state(), LinkState::Connected);

        sim.advance(4000);
        let _ = c.get("/api/v1/echo");
        assert_eq!(p.refresh(&c), LinkState::SessionExpired);
        assert!(p.state().needs_login());
        assert_eq!(p.state().tone(), Tone::Warning);

        typed(&mut p, "admin", "secret-password");
        p.handle(Action::SubmitLogin, &mut c);
        assert_eq!(p.state(), LinkState::Connected);
        p.handle(Action::Logout, &mut c);
        assert_eq!(p.state(), LinkState::NoSession);
        assert!(p.who().is_none());
    }

    #[test]
    fn a_server_that_cannot_be_reached_shows_disconnected_and_the_tries() {
        let sim = SimServer::new();
        let mut c = client(&sim);
        let mut p = ServerPanel::new(&settings(), "phoenix-cnc");
        sim.script(vec![Fault::NoConnection; 3]);
        p.handle(Action::TestConnection, &mut c);
        assert_eq!(p.state(), LinkState::Disconnected);
        assert_eq!(p.state().tone(), Tone::Bad);
        assert_eq!(
            p.notice_text(Lang::En),
            "The server cannot be reached (3 tries)"
        );
        assert_eq!(
            p.notice_text(Lang::Es),
            "No se puede llegar al servidor (3 intentos)"
        );
        assert!(p.notice_text(Lang::Ja).contains("3 回試行"));
        // it comes back
        p.handle(Action::TestConnection, &mut c);
        assert_ne!(p.state(), LinkState::Disconnected);
    }

    #[test]
    fn without_an_address_the_state_is_no_server_and_the_error_says_so_in_each_language() {
        let sim = SimServer::new();
        let mut c = Client::new(Settings::default(), Box::new(sim.clone()));
        let mut p = ServerPanel::new(&Settings::default(), "studio");
        p.handle(Action::TestConnection, &mut c);
        assert_eq!(p.state(), LinkState::NotConfigured);
        for l in Lang::ALL.into_iter().take(8) {
            assert_eq!(p.notice_text(l), tr(l, ServerText::ErrNoAddress), "{l:?}");
        }
    }

    #[test]
    fn the_errors_of_the_server_are_shown_by_code_in_the_eight_languages() {
        let e = ClientError::Forbidden {
            code: Some("forbidden".to_string()),
        };
        let mut seen = std::collections::HashSet::new();
        for l in Lang::ALL.into_iter().take(8) {
            let t = error_text(l, &e);
            assert!(!t.is_empty());
            seen.insert(t);
        }
        assert!(seen.len() >= 7, "{seen:?}");
        // a variant speaks its main language
        assert_eq!(error_text(Lang::PtBr, &e), error_text(Lang::Pt, &e));
        assert_eq!(error_text(Lang::Bg, &e), error_text(Lang::En, &e));
        // a code of a newer server is shown as itself
        let newer = ClientError::Server {
            status: 409,
            code: Some("brand_new_code".to_string()),
            detail: String::new(),
        };
        assert_eq!(error_text(Lang::De, &newer), "brand_new_code");
    }

    #[test]
    fn the_messages_of_the_client_about_its_settings_have_their_own_words() {
        for (m, t) in [
            ("server_address is empty: x", ServerText::ErrNoAddress),
            ("server_machine is empty: x", ServerText::ErrNoMachine),
            ("server_mdns is off: x", ServerText::ErrMdnsOff),
            ("no mDNS resolver was given", ServerText::ErrNoResolver),
        ] {
            assert_eq!(
                error_text(Lang::Fr, &ClientError::Config(m.to_string())),
                tr(Lang::Fr, t)
            );
        }
        assert_eq!(
            error_text(Lang::Fr, &ClientError::Config("other".to_string())),
            "other"
        );
        assert_eq!(
            error_text(Lang::It, &ClientError::BadReply("x".to_string())),
            tr(Lang::It, ServerText::ErrBadReply)
        );
    }

    #[test]
    fn applying_gives_the_settings_once_and_a_bad_box_gives_the_problems() {
        let sim = SimServer::new();
        let mut c = client(&sim);
        let mut p = ServerPanel::new(&settings(), "phoenix-cnc");
        p.form.retries = "9".to_string();
        p.form.address = "nonsense".to_string();
        assert!(p.handle(Action::Apply, &mut c).is_none());
        match &p.notice {
            Notice::Invalid(issues) => assert_eq!(issues.len(), 2),
            other => panic!("{other:?}"),
        }
        assert_eq!(p.notice_text(Lang::En).lines().count(), 2);

        p.form.retries = "3".to_string();
        p.form.address = "10.0.0.5:18200".to_string();
        p.form.mdns = true;
        let applied = p.handle(Action::Apply, &mut c).unwrap();
        assert_eq!(applied.server_retries, 3);
        assert_eq!(applied.server_address, "10.0.0.5:18200");
        assert!(applied.server_mdns);
        assert_eq!(p.notice, Notice::Applied);
        assert!(!p.form.is_dirty(), "the form is the applied settings now");

        p.form.machine = "x".to_string();
        p.handle(Action::Undo, &mut c);
        assert_eq!(p.form.machine, "bungard");
    }

    #[test]
    fn the_settings_screen_text_never_shows_the_token() {
        let mut s = settings();
        s.set("server_token", "very-secret-token").unwrap();
        let panel = ServerPanel::new(&s, "studio");
        let shown = format!("{panel:?}");
        assert!(!shown.contains("very-secret-token"), "{shown}");
        assert!(!s.to_text(false).contains("very-secret-token"));
    }

    #[test]
    fn every_state_has_a_text_and_a_tone_in_the_eight_languages() {
        for st in [
            LinkState::NotConfigured,
            LinkState::Unknown,
            LinkState::Connected,
            LinkState::Disconnected,
            LinkState::SessionExpired,
            LinkState::NoSession,
        ] {
            let mut words = std::collections::HashSet::new();
            for l in Lang::ALL.into_iter().take(8) {
                words.insert(tr(l, st.text()));
            }
            assert!(words.len() >= 6, "{st:?}");
        }
    }

    #[test]
    fn the_login_button_needs_a_name_and_a_password() {
        let mut d = LoginDialog::new("studio");
        assert!(!d.can_submit());
        d.name = "  ".to_string();
        d.password = "x".to_string();
        assert!(!d.can_submit());
        d.name = "admin".to_string();
        assert!(d.can_submit());
        d.password.clear();
        assert!(!d.can_submit());
    }

    fn core_langs() -> impl Iterator<Item = Lang> {
        Lang::ALL.into_iter().take(8)
    }

    #[test]
    fn the_link_badge_has_a_word_a_tone_a_hint_and_a_tooltip_in_every_language() {
        let badges = LinkBadge::all();
        assert_eq!(badges.len(), LinkState::ALL.len());
        let mut states: Vec<String> = badges.iter().map(|b| format!("{:?}", b.state())).collect();
        states.sort();
        states.dedup();
        assert_eq!(states.len(), 6, "one badge for each state");
        for b in badges {
            assert_eq!(b.tone(), b.state().tone());
            for l in core_langs() {
                assert!(!b.label(l).is_empty());
                let tip = b.tooltip(l);
                assert!(tip.starts_with(tr(l, ServerText::LinkLabel)), "{tip}");
                assert!(tip.contains(b.label(l)), "{tip}");
                assert_eq!(tip.contains(" - "), b.hint(l).is_some(), "{tip}");
            }
        }
        let hint_of = |s| LinkBadge::new(s).hint(Lang::En);
        assert_eq!(hint_of(LinkState::Connected), None);
        assert_eq!(hint_of(LinkState::Unknown), None);
        assert_eq!(hint_of(LinkState::NoSession), Some("Log in"));
        assert_eq!(hint_of(LinkState::SessionExpired), Some("Log in"));
        assert_eq!(
            hint_of(LinkState::Disconnected),
            Some("Test the connection")
        );
        assert_eq!(hint_of(LinkState::NotConfigured), Some("Server address"));
        let alerts: Vec<LinkState> = LinkState::ALL
            .into_iter()
            .filter(|s| LinkBadge::new(*s).is_alert())
            .collect();
        assert_eq!(
            alerts,
            vec![
                LinkState::NoSession,
                LinkState::SessionExpired,
                LinkState::Disconnected
            ]
        );
    }

    #[test]
    fn the_link_badge_follows_the_client_through_its_life() {
        let sim = SimServer::new();
        let mut tracker = LinkTracker::default();
        let nothing = Client::new(Settings::default(), Box::new(sim.clone()));
        assert_eq!(
            LinkBadge::observe(&mut tracker, &nothing).state(),
            LinkState::NotConfigured
        );
        let mut c = client(&sim);
        assert_eq!(
            LinkBadge::observe(&mut tracker, &c).state(),
            LinkState::NoSession
        );
        c.login("admin", "secret-password", "phoenix-cnc").unwrap();
        assert_eq!(
            LinkBadge::observe(&mut tracker, &c).state(),
            LinkState::Connected
        );
        sim.script(vec![Fault::NoConnection; 3]);
        let _ = c.get("/api/v1/echo");
        let b = LinkBadge::observe(&mut tracker, &c);
        assert_eq!(b.state(), LinkState::Disconnected);
        assert!(b.is_alert());
        sim.end_all_logins();
        let _ = c.get("/api/v1/echo");
        assert_eq!(
            LinkBadge::observe(&mut tracker, &c).state(),
            LinkState::SessionExpired
        );
    }

    #[test]
    fn the_panel_gives_the_badge_of_its_state() {
        let sim = SimServer::new();
        let mut c = client(&sim);
        let mut panel = ServerPanel::new(&settings(), "phoenix-cnc");
        panel.refresh(&c);
        assert_eq!(panel.badge().state(), LinkState::NoSession);
        typed(&mut panel, "admin", "secret-password");
        panel.handle(Action::SubmitLogin, &mut c);
        assert_eq!(panel.badge().state(), LinkState::Connected);
    }

    fn logged_panel(sim: &SimServer) -> (ServerPanel, Client) {
        let mut c = client(sim);
        let mut panel = ServerPanel::new(&settings(), "phoenix-cnc");
        typed(&mut panel, "admin", "secret-password");
        panel.handle(Action::SubmitLogin, &mut c);
        assert!(panel.who().is_some());
        (panel, c)
    }

    fn other_device(sim: &SimServer, app: &str) -> Client {
        let mut phone = client(sim);
        phone.login("admin", "secret-password", app).unwrap();
        phone
    }

    #[test]
    fn the_list_of_sessions_shows_the_one_in_use_first_and_closes_another() {
        let sim = SimServer::new();
        let (mut panel, mut c) = logged_panel(&sim);
        let mut phone = other_device(&sim, "android");
        assert!(!panel.sessions.open && !panel.sessions.is_loaded());
        panel.handle(Action::OpenSessions, &mut c);
        assert!(panel.sessions.open && panel.sessions.is_loaded());
        assert!(panel.sessions.error().is_none());
        let rows = panel.sessions.rows();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].current && !rows[1].current);
        assert_eq!(rows[0].app, "phoenix-cnc");
        assert_eq!(rows[1].app, "android");
        // close the phone
        let phone_id = rows[1].id.clone();
        panel.sessions.ask_close(&phone_id);
        assert_eq!(panel.sessions.pending(), Some(phone_id.as_str()));
        panel.handle(Action::CloseSession, &mut c);
        assert_eq!(panel.notice, Notice::SessionClosed);
        assert_eq!(panel.sessions.rows().len(), 1);
        assert!(panel.sessions.pending().is_none());
        assert_eq!(sim.open_logins(), 1);
        assert!(panel.who().is_some() && panel.state() == LinkState::Connected);
        for l in core_langs() {
            assert_eq!(panel.notice_text(l), tr(l, ServerText::SessionClosed));
        }
        // the phone found out: its next request is refused
        assert!(phone.get("/api/v1/echo").unwrap_err().needs_login());
    }

    #[test]
    fn closing_the_session_in_use_asks_first_and_then_logs_out() {
        let sim = SimServer::new();
        let (mut panel, mut c) = logged_panel(&sim);
        panel.handle(Action::OpenSessions, &mut c);
        let own = panel.sessions.rows()[0].id.clone();
        panel.sessions.ask_close(&own);
        panel.handle(Action::CloseSession, &mut c);
        // nothing happened yet: the question is asked
        assert!(panel.sessions.is_confirming());
        assert_eq!(sim.open_logins(), 1);
        assert!(c.has_token());
        // changing the mind
        panel.handle(Action::CancelCloseSession, &mut c);
        assert!(!panel.sessions.is_confirming() && panel.sessions.pending().is_none());
        assert!(c.has_token());
        // saying yes
        panel.sessions.ask_close(&own);
        panel.handle(Action::CloseSession, &mut c);
        panel.handle(Action::ConfirmCloseSession, &mut c);
        assert_eq!(sim.open_logins(), 0);
        assert!(!c.has_token());
        assert!(panel.who().is_none());
        assert_eq!(panel.state(), LinkState::NoSession);
        assert!(!panel.sessions.open && panel.sessions.rows().is_empty());
        assert_eq!(panel.notice, Notice::SessionClosed);
    }

    #[test]
    fn a_session_the_list_does_not_have_is_ignored_and_nothing_pending_does_nothing() {
        let sim = SimServer::new();
        let (mut panel, mut c) = logged_panel(&sim);
        panel.handle(Action::OpenSessions, &mut c);
        panel.sessions.ask_close("sess-unknown");
        assert!(panel.sessions.pending().is_none());
        assert_eq!(
            panel.close_pending_session(&mut c, true),
            CloseOutcome::Nothing
        );
        assert_eq!(sim.open_logins(), 1);
    }

    #[test]
    fn a_failed_reading_keeps_the_last_list_and_says_why_in_every_language() {
        let sim = SimServer::new();
        let (mut panel, mut c) = logged_panel(&sim);
        let _phone = other_device(&sim, "ios");
        panel.handle(Action::OpenSessions, &mut c);
        assert_eq!(panel.sessions.rows().len(), 2);
        sim.script(vec![Fault::NoConnection; 3]);
        panel.handle(Action::RefreshSessions, &mut c);
        assert_eq!(panel.sessions.rows().len(), 2, "the last list stays");
        assert!(matches!(
            panel.sessions.error(),
            Some(ClientError::Unreachable { .. })
        ));
        for l in core_langs() {
            assert!(!panel.sessions.error_text(l).unwrap().is_empty());
        }
        assert_eq!(panel.state(), LinkState::Disconnected);
        // the server is back
        panel.handle(Action::RefreshSessions, &mut c);
        assert!(panel.sessions.error().is_none());
        assert_eq!(panel.state(), LinkState::Connected);
    }

    #[test]
    fn a_session_that_cannot_be_closed_leaves_the_list_and_says_why() {
        let sim = SimServer::new();
        let (mut panel, mut c) = logged_panel(&sim);
        let phone = other_device(&sim, "ios");
        panel.handle(Action::OpenSessions, &mut c);
        let id = panel.sessions.rows()[1].id.clone();
        panel.sessions.ask_close(&id);
        // the phone logs out before the person clicks: the server does not know the session any more
        let mut phone = phone;
        phone.logout().unwrap();
        panel.handle(Action::CloseSession, &mut c);
        match &panel.notice {
            Notice::Failed(e) => assert_eq!(e.code(), Some("unknown_session")),
            other => panic!("{other:?}"),
        }
        assert!(!panel.notice_text(Lang::En).is_empty());
        assert!(panel.who().is_some());
    }

    #[test]
    fn without_a_login_the_list_cannot_be_read_and_logging_out_hides_it() {
        let sim = SimServer::new();
        let mut c = client(&sim);
        let mut panel = ServerPanel::new(&settings(), "phoenix-cnc");
        panel.handle(Action::OpenSessions, &mut c);
        // there is nothing to list without a login: the list does not stay open
        assert!(!panel.sessions.open && !panel.sessions.is_loaded());
        assert!(panel.sessions.rows().is_empty());
        let mut direct = SessionsView::default();
        assert!(!direct.load(&mut c));
        assert!(direct.error().unwrap().needs_login());
        // logged in, open, then log out: the list goes
        typed(&mut panel, "admin", "secret-password");
        panel.handle(Action::SubmitLogin, &mut c);
        panel.handle(Action::OpenSessions, &mut c);
        assert!(panel.sessions.is_loaded() && panel.sessions.open);
        panel.handle(Action::Logout, &mut c);
        assert!(!panel.sessions.open && !panel.sessions.is_loaded());
        assert!(panel.sessions.rows().is_empty());
        // hiding keeps what was read but drops a pending question
        let (mut p2, mut c2) = logged_panel(&sim);
        p2.handle(Action::OpenSessions, &mut c2);
        let id = p2.sessions.rows()[0].id.clone();
        p2.sessions.ask_close(&id);
        p2.handle(Action::HideSessions, &mut c2);
        assert!(!p2.sessions.open && p2.sessions.pending().is_none());
        assert!(p2.sessions.is_loaded());
    }

    #[test]
    fn the_texts_of_the_sessions_say_the_same_things_in_every_language() {
        // the warning about the session in use names closing and leaving; the question has a yes
        for l in core_langs() {
            for t in [
                ServerText::Sessions,
                ServerText::SessionsRefresh,
                ServerText::SessionsHide,
                ServerText::SessionsNone,
                ServerText::SessionClose,
                ServerText::SessionThis,
                ServerText::SessionCloseOwnWarning,
                ServerText::SessionConfirm,
                ServerText::SessionApp,
                ServerText::SessionAddress,
                ServerText::SessionRole,
                ServerText::LinkLabel,
            ] {
                assert!(!tr(l, t).trim().is_empty(), "{t:?} {l:?}");
            }
        }
        assert!(tr(Lang::En, ServerText::SessionCloseOwnWarning).contains("logs you out"));
        assert!(tr(Lang::Pt, ServerText::SessionThis).contains("sessão"));
    }
}
