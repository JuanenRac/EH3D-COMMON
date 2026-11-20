//! The client: login, renewal, requests with retries and time limits, and the state of the link.

use crate::contract::{Contract, ContractCache, ContractDiff};
use crate::errors::{ClientError, TransportError};
use crate::json::{quote, Json};
use crate::metrics::LinkMetrics;
use crate::queue::{is_queueable, OfflineQueue, Queued, Rejected};
use crate::secret::{MemoryStore, Secret, TokenStore};
use crate::settings::Settings;
use crate::transport::{
    path_segment, FullReply, Reply, Request, RequestParts, TcpTransport, Transport,
};
use std::time::{Duration, Instant};

/// Waits between two tries. A test gives one that only counts.
pub trait Sleeper: Send {
    fn sleep(&self, how_long: Duration);
}

/// Sleeps the thread.
pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&self, how_long: Duration) {
        std::thread::sleep(how_long);
    }
}

/// Seconds since a fixed point, to know when a login runs out.
pub trait Clock: Send {
    fn now_s(&self) -> u64;
}

/// The clock of the program (monotonic).
pub struct SystemClock(Instant);

impl Default for SystemClock {
    fn default() -> SystemClock {
        SystemClock(Instant::now())
    }
}

impl Clock for SystemClock {
    fn now_s(&self) -> u64 {
        self.0.elapsed().as_secs()
    }
}

/// Looks for servers on the local network (mDNS). The crate has no resolver of its own: a program that wants discovery gives one, and the client uses it
/// only when `server_mdns` is on.
pub trait Discover: Send {
    /// The `host:port` of every server that answered within the time.
    fn find(&self, within: Duration) -> Vec<String>;
}

/// The state of the link, for the corner of a window or a line of a status page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// Nothing has been tried yet.
    Unknown,
    /// The server answered the last request.
    Connected,
    /// The last request could not reach the server, after its retries.
    Disconnected,
    /// The server answered but there is no login (or it expired): the person has to log in.
    LoggedOut,
}

/// Who is logged in. The token is not here: it never leaves the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    pub role: String,
    pub app: String,
    pub session_id: Option<String>,
    /// The seconds the login lasted when it was made or last renewed.
    pub expires_s: u64,
}

/// What `GET /health` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    pub name: String,
    pub version: String,
    pub status: String,
    pub api_revision: u64,
}

/// One open login of the caller (`GET /auth/sessions`). The `id` is not the token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub app: String,
    pub ip: String,
    pub current: bool,
}

const MIB: u64 = 1_048_576;
const BACKOFF_FIRST_MS: u64 = 200;
const BACKOFF_MAX_MS: u64 = 2_000;

/// The pause before the try number `n` of a request (`n` is 1 for the first retry): 200 ms, 400 ms, 800 ms... up to 2 s.
pub fn backoff(n: u32) -> Duration {
    let ms = BACKOFF_FIRST_MS
        .saturating_mul(1u64 << n.saturating_sub(1).min(10))
        .min(BACKOFF_MAX_MS);
    Duration::from_millis(ms)
}

fn is_idempotent(method: &str) -> bool {
    matches!(method, "GET" | "HEAD" | "PUT" | "DELETE" | "OPTIONS")
}

/// The client of one server.
pub struct Client {
    settings: Settings,
    transport: Box<dyn Transport>,
    sleeper: Box<dyn Sleeper>,
    clock: Box<dyn Clock>,
    store: Box<dyn TokenStore>,
    discovery: Option<Box<dyn Discover>>,
    token: Option<Secret>,
    session: Option<Session>,
    expires_at: Option<u64>,
    link: Link,
    auto_refresh: bool,
    store_warning: Option<String>,
    metrics: LinkMetrics,
}

/// What `send_or_queue` did with a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The server answered.
    Done(Reply),
    /// The server could not be reached and the request waits in the queue.
    Queued,
    /// The server could not be reached and the queue refused the request, for this reason.
    NotQueued(Rejected),
}

/// What `flush` did with the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlushReport {
    /// Requests the server accepted.
    pub sent: usize,
    /// Requests the server refused (4xx or 5xx other than a login problem): they are dropped, with what it said.
    pub refused: Vec<(Queued, ClientError)>,
    /// Requests dropped for being too old before the flush.
    pub expired: usize,
    /// What stopped the flush (the server is unreachable again, or a login is needed); the rest of the queue is kept.
    pub stopped: Option<ClientError>,
    /// Requests still waiting.
    pub remaining: usize,
}

impl Client {
    /// A client that talks plain HTTP to `settings.server_address`.
    pub fn connect(settings: Settings) -> Client {
        let transport = TcpTransport::new(settings.server_address.clone());
        Client::new(settings, Box::new(transport))
    }

    /// A client over the transport given (a server in memory for the tests, a tunnel...). The token starts as `server_token` when the settings have
    /// one (the token of a program), else as nothing.
    pub fn new(settings: Settings, transport: Box<dyn Transport>) -> Client {
        let token = if settings.server_token.is_empty() {
            None
        } else {
            Some(settings.server_token.clone())
        };
        Client {
            settings,
            transport,
            sleeper: Box::new(ThreadSleeper),
            clock: Box::new(SystemClock::default()),
            store: Box::new(MemoryStore::default()),
            discovery: None,
            token,
            session: None,
            expires_at: None,
            link: Link::Unknown,
            auto_refresh: true,
            store_warning: None,
            metrics: LinkMetrics::default(),
        }
    }

    pub fn with_sleeper(mut self, sleeper: Box<dyn Sleeper>) -> Client {
        self.sleeper = sleeper;
        self
    }

    pub fn with_clock(mut self, clock: Box<dyn Clock>) -> Client {
        self.clock = clock;
        self
    }

    /// Where the token is kept between runs. A token the store already has is taken up, unless the settings gave one.
    pub fn with_store(mut self, store: Box<dyn TokenStore>) -> Client {
        if self.token.is_none() {
            self.token = store.load();
        }
        self.store = store;
        self
    }

    pub fn with_discovery(mut self, discovery: Box<dyn Discover>) -> Client {
        self.discovery = Some(discovery);
        self
    }

    /// The resolver of this crate for `_eh3d._tcp` (feature `mdns`). It still asks nothing of the network unless `server_mdns` is on.
    #[cfg(feature = "mdns")]
    pub fn with_mdns(self) -> Client {
        self.with_discovery(Box::new(crate::mdns::MdnsResolver::default()))
    }

    /// What the link has done so far: requests, tries, retries, latencies, losses.
    pub fn metrics(&self) -> &LinkMetrics {
        &self.metrics
    }

    /// Starts the metrics again from zero.
    pub fn reset_metrics(&mut self) {
        self.metrics.reset();
    }

    /// Whether a request renews the login by itself when it is about to run out (on by default).
    pub fn with_auto_refresh(mut self, on: bool) -> Client {
        self.auto_refresh = on;
        self
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn link(&self) -> Link {
        self.link
    }

    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Is there a token to send (a login or a program token)?
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// A problem keeping the token in the store at the last login, if there was one. The login itself worked.
    pub fn store_warning(&self) -> Option<&str> {
        self.store_warning.as_deref()
    }

    /// Takes a token given by hand (the token of a program), forgetting any login.
    pub fn set_token(&mut self, token: Secret) {
        self.session = None;
        self.expires_at = None;
        self.token = Some(token);
        self.link = Link::Unknown;
    }

    fn forget(&mut self) {
        self.token = None;
        self.session = None;
        self.expires_at = None;
        self.store.clear();
        self.link = Link::LoggedOut;
    }

    fn need_address(&self) -> Result<(), ClientError> {
        if self.settings.is_configured() {
            Ok(())
        } else {
            Err(ClientError::Config(
                "server_address is empty: there is no server to talk to".to_string(),
            ))
        }
    }

    /// Looks for servers on the network with the resolver given. It does nothing, and asks nothing of the network, unless `server_mdns` is on.
    pub fn discover(&self) -> Result<Vec<String>, ClientError> {
        if !self.settings.server_mdns {
            return Err(ClientError::Config(
                "server_mdns is off: set it to on to look for the server on the network"
                    .to_string(),
            ));
        }
        match &self.discovery {
            Some(d) => Ok(d.find(Duration::from_secs(self.settings.server_timeout_s.min(10)))),
            None => Err(ClientError::Config(
                "no mDNS resolver was given to this program".to_string(),
            )),
        }
    }

    fn build(&self, method: &str, path: &str, body: Option<&str>, bearer: bool) -> Request {
        Request {
            method: method.to_string(),
            path: path.to_string(),
            body: body.map(str::to_string),
            body_bytes: None,
            headers: Vec::new(),
            single_try: false,
            bearer: if bearer {
                self.token.as_ref().map(|t| t.expose().to_string())
            } else {
                None
            },
            timeout: Duration::from_secs(self.settings.server_timeout_s),
            max_answer_bytes: self.settings.server_max_answer_mib.saturating_mul(MIB),
        }
    }

    /// Sends with the retries: a network failure is tried again up to `server_retries` times with a growing pause. A connection that could not be made
    /// is tried again for any method (nothing was sent); a time-out or a broken connection only for a method that can be repeated; so a `POST` is never sent
    /// twice. A 502, 503 or 504 is tried again for a method that can be repeated too.
    fn send_retrying(&mut self, request: &Request) -> Result<Reply, ClientError> {
        self.send_retrying_full(request).map(FullReply::into_reply)
    }

    /// The same as `send_retrying` with the headers and the body as bytes. A request marked `single_try` is not sent again after a time-out or a broken
    /// connection (nor after a 502, 503 or 504), whatever its method.
    fn send_retrying_full(&mut self, request: &Request) -> Result<FullReply, ClientError> {
        let repeatable = is_idempotent(&request.method) && !request.single_try;
        let tries = self.settings.server_retries + 1;
        let mut last = TransportError::Timeout;
        let link_before = self.link;
        self.metrics.begin_request();
        for attempt in 1..=tries {
            if attempt > 1 {
                self.sleeper.sleep(backoff(attempt - 1));
            }
            self.metrics.begin_attempt(attempt > 1);
            let started = Instant::now();
            match self.transport.send_full(request) {
                Ok(reply) => {
                    self.metrics.record_reply(started.elapsed(), reply.status);
                    self.link = Link::Connected;
                    if repeatable && matches!(reply.status, 502..=504) && attempt < tries {
                        continue;
                    }
                    return Ok(reply);
                }
                Err(e) => {
                    let again = match &e {
                        TransportError::Connect(_) => true,
                        TransportError::Timeout | TransportError::Io(_) => repeatable,
                        TransportError::TooLarge => false,
                    };
                    if !again {
                        self.link = Link::Disconnected;
                        self.metrics
                            .record_failure(link_before != Link::Disconnected);
                        return Err(ClientError::Unreachable {
                            error: e,
                            attempts: attempt,
                        });
                    }
                    last = e;
                }
            }
        }
        self.link = Link::Disconnected;
        self.metrics
            .record_failure(link_before != Link::Disconnected);
        Err(ClientError::Unreachable {
            error: last,
            attempts: tries,
        })
    }

    fn code_and_detail(reply: &Reply) -> (Option<String>, String) {
        match reply.json() {
            Some(j) => (
                j.get("error").and_then(Json::as_str).map(str::to_string),
                j.get("detail")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string(),
            ),
            None => (None, String::new()),
        }
    }

    /// An answer of 400 or more as the error it is; a 401 makes the client forget the token.
    fn check(&mut self, reply: Reply, forget_on_401: bool) -> Result<Reply, ClientError> {
        if reply.status < 400 {
            return Ok(reply);
        }
        let (code, detail) = Client::code_and_detail(&reply);
        match reply.status {
            401 => {
                if forget_on_401 {
                    self.forget();
                }
                Err(ClientError::Unauthorized { code })
            }
            403 => Err(ClientError::Forbidden { code }),
            status => Err(ClientError::Server {
                status,
                code,
                detail,
            }),
        }
    }

    /// The seconds left of the login, when this client made one.
    fn remaining_s(&self) -> Option<u64> {
        self.expires_at
            .map(|at| at.saturating_sub(self.clock.now_s()))
    }

    fn maybe_renew(&mut self) {
        if !self.auto_refresh {
            return;
        }
        let (Some(left), Some(session)) = (self.remaining_s(), self.session.as_ref()) else {
            return;
        };
        if left > 0 && left <= session.expires_s / 4 {
            // a renewal that does not work is not fatal here: the request goes on and the server says what it thinks of the login
            let _ = self.refresh();
        }
    }

    /// A request to a route that needs the login: the bearer token goes with it. `path` starts with `/api/v1/`.
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<Reply, ClientError> {
        self.need_address()?;
        if self.token.is_none() {
            self.link = Link::LoggedOut;
            return Err(ClientError::Unauthorized { code: None });
        }
        self.maybe_renew();
        let request = self.build(method, path, body, true);
        let reply = self.send_retrying(&request)?;
        self.check(reply, true)
    }

    /// A request with header lines and/or a body of bytes (`If-Match`, a file), to a route that needs the login. The parts are checked first
    /// ([`RequestParts::validate`]); a refusal of 400 or more is a `ClientError` as in `request`, and the answer comes with its headers.
    pub fn request_full(
        &mut self,
        method: &str,
        path: &str,
        parts: &RequestParts,
    ) -> Result<FullReply, ClientError> {
        parts.validate().map_err(ClientError::Config)?;
        self.need_address()?;
        if self.token.is_none() {
            self.link = Link::LoggedOut;
            return Err(ClientError::Unauthorized { code: None });
        }
        self.maybe_renew();
        let mut request = self.build(method, path, parts.text(), true);
        request.body_bytes = parts.bytes().map(<[u8]>::to_vec);
        request.headers = parts.headers().to_vec();
        request.single_try = parts.is_single_try();
        let reply = self.send_retrying_full(&request)?;
        if reply.status < 400 {
            return Ok(reply);
        }
        // `check` turns every status of 400 or more into the error it is (a 401 makes the client forget the token)
        let plain = reply.clone().into_reply();
        self.check(plain, true).map(|_| reply)
    }

    /// A request to a route that needs no login (health, set-up): no token is sent.
    pub fn request_open(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<Reply, ClientError> {
        self.need_address()?;
        let request = self.build(method, path, body, false);
        let reply = self.send_retrying(&request)?;
        self.check(reply, false)
    }

    pub fn get(&mut self, path: &str) -> Result<Reply, ClientError> {
        self.request("GET", path, None)
    }

    pub fn post(&mut self, path: &str, body: Option<&str>) -> Result<Reply, ClientError> {
        self.request("POST", path, body)
    }

    pub fn put(&mut self, path: &str, body: Option<&str>) -> Result<Reply, ClientError> {
        self.request("PUT", path, body)
    }

    pub fn delete(&mut self, path: &str) -> Result<Reply, ClientError> {
        self.request("DELETE", path, None)
    }

    /// `/api/v1/machines/<server_machine><suffix>`, with the name of the machine made safe for a path.
    pub fn machine_path(&self, suffix: &str) -> Result<String, ClientError> {
        if self.settings.server_machine.is_empty() {
            return Err(ClientError::Config(
                "server_machine is empty: no machine was chosen".to_string(),
            ));
        }
        Ok(format!(
            "/api/v1/machines/{}{}",
            path_segment(&self.settings.server_machine),
            suffix
        ))
    }

    /// `GET /api/v1/health`: whether the server is there and well. It needs no login.
    pub fn health(&mut self) -> Result<Health, ClientError> {
        let reply = self.request_open("GET", "/api/v1/health", None)?;
        let j = reply
            .json()
            .ok_or_else(|| ClientError::BadReply("the health answer is not JSON".to_string()))?;
        let text = |k: &str| j.get(k).and_then(Json::as_str).unwrap_or("").to_string();
        Ok(Health {
            name: text("name"),
            version: text("version"),
            status: text("status"),
            api_revision: j.get("api_revision").and_then(Json::as_u64).unwrap_or(0),
        })
    }

    /// `POST /api/v1/auth/login` for the application `app` (`studio`, `bungard-gui`, `hmi`...). The password is sent once and kept nowhere; the token is
    /// kept in the client (and in the store, when there is one) and never given back.
    pub fn login(&mut self, name: &str, password: &str, app: &str) -> Result<Session, ClientError> {
        self.need_address()?;
        let body = format!(
            "{{\"name\":{},\"password\":{},\"app\":{}}}",
            quote(name),
            quote(password),
            quote(app)
        );
        let request = self.build("POST", "/api/v1/auth/login", Some(&body), false);
        let reply = self.send_retrying(&request)?;
        let reply = self.check(reply, false)?;
        let j = reply
            .json()
            .ok_or_else(|| ClientError::BadReply("the login answer is not JSON".to_string()))?;
        let token = j
            .get("token")
            .and_then(Json::as_str)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| ClientError::BadReply("the login answer has no token".to_string()))?;
        let text = |k: &str| j.get(k).and_then(Json::as_str).unwrap_or("").to_string();
        let expires_s = j.get("expires_s").and_then(Json::as_u64).unwrap_or(0);
        let session = Session {
            name: text("name"),
            role: text("role"),
            app: text("app"),
            session_id: j
                .get("session_id")
                .and_then(Json::as_str)
                .map(str::to_string),
            expires_s,
        };
        let secret = Secret::new(token);
        self.store_warning = self.store.save(&secret).err();
        self.token = Some(secret);
        self.expires_at = Some(self.clock.now_s() + expires_s);
        self.session = Some(session.clone());
        self.link = Link::Connected;
        Ok(session)
    }

    /// `POST /api/v1/auth/refresh`: the login lasts again, with the same token. Gives the seconds left. A login the server no longer knows makes the
    /// client forget the token (`Unauthorized`).
    pub fn refresh(&mut self) -> Result<u64, ClientError> {
        self.need_address()?;
        if self.token.is_none() {
            self.link = Link::LoggedOut;
            return Err(ClientError::Unauthorized { code: None });
        }
        let request = self.build("POST", "/api/v1/auth/refresh", None, true);
        let reply = self.send_retrying(&request)?;
        if reply.status == 404 {
            let (code, _) = Client::code_and_detail(&reply);
            if code.as_deref() == Some("unknown_session") {
                self.forget();
                return Err(ClientError::Unauthorized { code });
            }
        }
        let reply = self.check(reply, true)?;
        let left = reply
            .json()
            .and_then(|j| j.get("expires_s").and_then(Json::as_u64))
            .ok_or_else(|| ClientError::BadReply("the renewal has no expires_s".to_string()))?;
        self.expires_at = Some(self.clock.now_s() + left);
        if let Some(s) = self.session.as_mut() {
            s.expires_s = left;
        }
        Ok(left)
    }

    /// `POST /api/v1/auth/logout`. The token is forgotten here whatever the server says.
    pub fn logout(&mut self) -> Result<(), ClientError> {
        self.need_address()?;
        let outcome = if self.token.is_some() {
            let request = self.build("POST", "/api/v1/auth/logout", None, true);
            self.send_retrying(&request)
                .and_then(|r| self.check(r, false))
                .map(|_| ())
        } else {
            Ok(())
        };
        let was = self.link;
        self.forget();
        if was == Link::Disconnected {
            self.link = Link::Disconnected;
        }
        outcome
    }

    /// `GET /api/v1/auth/sessions`: the open logins of the caller.
    pub fn sessions(&mut self) -> Result<Vec<SessionInfo>, ClientError> {
        let reply = self.get("/api/v1/auth/sessions")?;
        let j = reply
            .json()
            .ok_or_else(|| ClientError::BadReply("the sessions answer is not JSON".to_string()))?;
        let list = j
            .as_array()
            .or_else(|| j.get("sessions").and_then(Json::as_array))
            .ok_or_else(|| {
                ClientError::BadReply("the sessions answer is not a list".to_string())
            })?;
        Ok(list
            .iter()
            .map(|s| {
                let text = |k: &str| s.get(k).and_then(Json::as_str).unwrap_or("").to_string();
                SessionInfo {
                    id: text("id"),
                    name: text("name"),
                    role: text("role"),
                    app: text("app"),
                    ip: text("ip"),
                    current: s.get("current").and_then(Json::as_bool).unwrap_or(false),
                }
            })
            .collect())
    }

    /// `DELETE /api/v1/auth/sessions/<id>`: closes one login (the phone that was lost).
    pub fn close_session(&mut self, id: &str) -> Result<(), ClientError> {
        self.delete(&format!("/api/v1/auth/sessions/{}", path_segment(id)))
            .map(|_| ())
    }

    /// Sends a request; if the server cannot be reached and the request can wait (`PUT` and `DELETE`, see [`crate::queue`]), it goes to the queue
    /// instead of failing. Any other error (a refusal, a missing login, no address) is returned as it is, and a request that cannot wait is never queued.
    pub fn send_or_queue(
        &mut self,
        queue: &mut OfflineQueue,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<Outcome, ClientError> {
        match self.request(method, path, body) {
            Ok(reply) => Ok(Outcome::Done(reply)),
            Err(ClientError::Unreachable { .. }) if is_queueable(method) => {
                match queue.push(method, path, body, self.clock.now_s()) {
                    Ok(()) => Ok(Outcome::Queued),
                    Err(why) => Ok(Outcome::NotQueued(why)),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Sends what waits in the queue, oldest first, once the server is back. It stops at the first request the server cannot be reached for or that
    /// needs a login (that one and the rest stay in the queue); a request the server refuses is dropped and reported.
    pub fn flush(&mut self, queue: &mut OfflineQueue) -> FlushReport {
        let expired = queue.expire(self.clock.now_s());
        let mut report = FlushReport {
            sent: 0,
            refused: Vec::new(),
            expired,
            stopped: None,
            remaining: 0,
        };
        while let Some(item) = queue.pop_front() {
            match self.request(&item.method, &item.path, item.body.as_deref()) {
                Ok(_) => report.sent += 1,
                Err(e @ (ClientError::Unreachable { .. } | ClientError::Unauthorized { .. })) => {
                    queue.push_front(item);
                    report.stopped = Some(e);
                    break;
                }
                Err(e) => report.refused.push((item, e)),
            }
        }
        report.remaining = queue.len();
        report
    }

    /// Reads the contract of the server (`path` is [`crate::contract::DEFAULT_CONTRACT_PATH`] unless the server serves it elsewhere). It needs no login
    /// when the client has none.
    pub fn fetch_contract(&mut self, path: &str) -> Result<Contract, ClientError> {
        let reply = if self.token.is_some() {
            self.request("GET", path, None)?
        } else {
            self.request_open("GET", path, None)?
        };
        Contract::from_openapi(&reply.body).map_err(ClientError::BadReply)
    }

    /// Reads the contract and gives it to the cache: the routes the server has withdrawn since the last time become retired. Gives what changed.
    pub fn refresh_contract(
        &mut self,
        cache: &mut ContractCache,
        path: &str,
    ) -> Result<ContractDiff, ClientError> {
        let contract = self.fetch_contract(path)?;
        Ok(cache.update(contract))
    }
}
