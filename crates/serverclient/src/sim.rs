//! A server that lives in memory, for the tests of this crate and of every program that uses it: it logs in, renews, refuses with the codes of the catalog,
//! and fails on demand (no connection, a time-out, a 503). It speaks the same words as EH3D-SERVER for the routes the client uses; it is a double, not the server.

use crate::client::{Clock, Sleeper};
use crate::errors::TransportError;
use crate::json::{quote, Json};
use crate::transport::{Reply, Request, Transport};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A failure the next request meets, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The connection cannot be made.
    NoConnection,
    /// The connection is made and the answer never comes.
    Timeout,
    /// The server answers this status with `{"error": code}`.
    Status(u16, &'static str),
}

/// One request the server saw. The token is never kept: only whether one was sent and whether it was a valid one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub method: String,
    pub path: String,
    pub had_token: bool,
    pub token_valid: bool,
}

struct Account {
    password: String,
    role: String,
}

struct Login {
    name: String,
    role: String,
    app: String,
    session_id: String,
    expires_at: u64,
    first_at: u64,
}

struct State {
    now_s: u64,
    accounts: HashMap<String, Account>,
    logins: HashMap<String, Login>,
    program_tokens: Vec<String>,
    faults: Vec<Fault>,
    seen: Vec<Seen>,
    next: u64,
    login_s: u64,
    refresh_max_s: u64,
    slept: Vec<Duration>,
}

/// The server in memory. Clones share the same state, so a test keeps one to look at and gives the other to the client.
#[derive(Clone)]
pub struct SimServer(Arc<Mutex<State>>);

impl Default for SimServer {
    fn default() -> SimServer {
        SimServer::new()
    }
}

fn error(status: u16, code: &str) -> Reply {
    Reply {
        status,
        body: format!("{{\"error\":{},\"detail\":\"\"}}", quote(code)),
    }
}

fn ok(body: String) -> Reply {
    Reply { status: 200, body }
}

impl SimServer {
    /// A server with one administrator `admin` / `secret-password`, one viewer `viewer` / `viewer-password`, logins that last 3600 s, renewals that
    /// never reach past 7200 s after the login, and one token of a program, `program-token`.
    pub fn new() -> SimServer {
        let mut accounts = HashMap::new();
        accounts.insert(
            "admin".to_string(),
            Account {
                password: "secret-password".to_string(),
                role: "admin".to_string(),
            },
        );
        accounts.insert(
            "viewer".to_string(),
            Account {
                password: "viewer-password".to_string(),
                role: "viewer".to_string(),
            },
        );
        SimServer(Arc::new(Mutex::new(State {
            now_s: 0,
            accounts,
            logins: HashMap::new(),
            program_tokens: vec!["program-token".to_string()],
            faults: Vec::new(),
            seen: Vec::new(),
            next: 1,
            login_s: 3600,
            refresh_max_s: 7200,
            slept: Vec::new(),
        })))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The next requests meet these failures, one each, before the server answers normally.
    pub fn script(&self, faults: Vec<Fault>) {
        self.state().faults = faults;
    }

    /// Time passes on the server (and on a [`SimClock`] made from it).
    pub fn advance(&self, seconds: u64) {
        self.state().now_s += seconds;
    }

    /// Every login ends on the server, as if it expired or the server lost them.
    pub fn end_all_logins(&self) {
        self.state().logins.clear();
    }

    /// How many logins are open.
    pub fn open_logins(&self) -> usize {
        self.state().logins.len()
    }

    /// Every request that reached the server, in order.
    pub fn seen(&self) -> Vec<Seen> {
        self.state().seen.clone()
    }

    /// How many requests reached the server (a refused connection is not one).
    pub fn request_count(&self) -> usize {
        self.state().seen.len()
    }

    pub fn clock(&self) -> SimClock {
        SimClock(self.clone())
    }

    /// A sleeper that does not wait: it only writes down the pauses the client asked for.
    pub fn sleeper(&self) -> SimSleeper {
        SimSleeper(self.clone())
    }

    /// The pauses the client asked for, in order.
    pub fn pauses(&self) -> Vec<Duration> {
        self.state().slept.clone()
    }

    fn handle(&self, req: &Request) -> Result<Reply, TransportError> {
        let mut st = self.state();
        if !st.faults.is_empty() {
            let fault = st.faults.remove(0);
            match fault {
                Fault::NoConnection => {
                    return Err(TransportError::Connect("connection refused".to_string()))
                }
                Fault::Timeout => return Err(TransportError::Timeout),
                Fault::Status(status, code) => {
                    st.seen.push(Seen {
                        method: req.method.clone(),
                        path: req.path.clone(),
                        had_token: req.bearer().is_some(),
                        token_valid: false,
                    });
                    return Ok(error(status, code));
                }
            }
        }
        let now = st.now_s;
        let bearer = req.bearer().map(str::to_string);
        let who = bearer.as_ref().and_then(|t| {
            if st.program_tokens.contains(t) {
                Some(("program".to_string(), "operator".to_string(), None))
            } else {
                st.logins
                    .get(t)
                    .filter(|l| l.expires_at > now)
                    .map(|l| (l.name.clone(), l.role.clone(), Some(t.clone())))
            }
        });
        st.seen.push(Seen {
            method: req.method.clone(),
            path: req.path.clone(),
            had_token: bearer.is_some(),
            token_valid: who.is_some(),
        });
        let route = format!("{} {}", req.method, req.path);
        match route.as_str() {
            "GET /api/v1/health" => Ok(ok(
                "{\"name\":\"eh3d-server\",\"version\":\"0.0.0-sim\",\"api_revision\":1,\"status\":\"ok\"}".to_string(),
            )),
            "POST /api/v1/auth/login" => {
                let Some(body) = req.body.as_deref().and_then(|b| Json::parse(b).ok()) else {
                    return Ok(error(400, "bad_request"));
                };
                let field = |k: &str| {
                    body.get(k)
                        .and_then(Json::as_str)
                        .unwrap_or("")
                        .to_string()
                };
                let (name, password, app) = (field("name"), field("password"), field("app"));
                if app.is_empty() || app.len() > 32 {
                    return Ok(error(422, "bad_app"));
                }
                let Some(account) = st.accounts.get(&name) else {
                    return Ok(error(401, "bad_credentials"));
                };
                if account.password != password {
                    return Ok(error(401, "bad_credentials"));
                }
                let role = account.role.clone();
                let n = st.next;
                st.next += 1;
                let token = format!("sim-token-{n}");
                let session_id = format!("sess-{n}");
                let expires_at = now + st.login_s;
                let login_s = st.login_s;
                st.logins.insert(
                    token.clone(),
                    Login {
                        name: name.clone(),
                        role: role.clone(),
                        app: app.clone(),
                        session_id: session_id.clone(),
                        expires_at,
                        first_at: now,
                    },
                );
                Ok(ok(format!(
                    "{{\"token\":{},\"name\":{},\"role\":{},\"app\":{},\"session_id\":{},\"expires_s\":{login_s}}}",
                    quote(&token),
                    quote(&name),
                    quote(&role),
                    quote(&app),
                    quote(&session_id)
                )))
            }
            "POST /api/v1/auth/refresh" => {
                let Some(token) = bearer else {
                    return Ok(error(401, "unauthorized"));
                };
                let (login_s, refresh_max_s) = (st.login_s, st.refresh_max_s);
                match st.logins.get_mut(&token) {
                    Some(l) if l.expires_at > now => {
                        let renewed = (now + login_s).min(l.first_at + refresh_max_s);
                        l.expires_at = l.expires_at.max(renewed);
                        let left = l.expires_at - now;
                        Ok(ok(format!("{{\"expires_s\":{left}}}")))
                    }
                    _ => Ok(error(404, "unknown_session")),
                }
            }
            "POST /api/v1/auth/logout" => {
                if who.is_none() {
                    return Ok(error(401, "unauthorized"));
                }
                if let Some(t) = bearer {
                    st.logins.remove(&t);
                }
                Ok(ok("{}".to_string()))
            }
            "GET /api/v1/auth/sessions" => {
                let Some((name, _, current)) = who else {
                    return Ok(error(401, "unauthorized"));
                };
                let mut rows: Vec<String> = st
                    .logins
                    .iter()
                    .filter(|(_, l)| l.name == name && l.expires_at > now)
                    .map(|(t, l)| {
                        format!(
                            "{{\"id\":{},\"name\":{},\"role\":{},\"app\":{},\"ip\":\"127.0.0.1\",\"current\":{}}}",
                            quote(&l.session_id),
                            quote(&l.name),
                            quote(&l.role),
                            quote(&l.app),
                            current.as_deref() == Some(t.as_str())
                        )
                    })
                    .collect();
                rows.sort();
                Ok(ok(format!("{{\"sessions\":[{}]}}", rows.join(","))))
            }
            _ if req.method == "DELETE" && req.path.starts_with("/api/v1/auth/sessions/") => {
                let Some((name, _, _)) = who else {
                    return Ok(error(401, "unauthorized"));
                };
                let id = &req.path["/api/v1/auth/sessions/".len()..];
                let key = st
                    .logins
                    .iter()
                    .find(|(_, l)| l.session_id == id && l.name == name)
                    .map(|(t, _)| t.clone());
                match key {
                    Some(t) => {
                        st.logins.remove(&t);
                        Ok(ok("{}".to_string()))
                    }
                    None => Ok(error(404, "unknown_session")),
                }
            }
            "GET /api/v1/echo" | "PUT /api/v1/echo" | "POST /api/v1/echo" => match who {
                None => Ok(error(401, "unauthorized")),
                Some(_) => Ok(ok(format!(
                    "{{\"ok\":true,\"body\":{}}}",
                    quote(req.body.as_deref().unwrap_or(""))
                ))),
            },
            "GET /api/v1/admin/only" => match who {
                None => Ok(error(401, "unauthorized")),
                Some((_, role, _)) if role != "admin" => Ok(error(403, "forbidden")),
                Some(_) => Ok(ok("{\"ok\":true}".to_string())),
            },
            _ => Ok(error(404, "not_found")),
        }
    }
}

impl Transport for SimServer {
    fn send(&self, request: &Request) -> Result<Reply, TransportError> {
        self.handle(request)
    }
}

/// The clock of a [`SimServer`]: it moves only when the test calls `advance`.
pub struct SimClock(SimServer);

impl Clock for SimClock {
    fn now_s(&self) -> u64 {
        self.0.state().now_s
    }
}

/// A sleeper that writes the pause down and does not wait.
pub struct SimSleeper(SimServer);

impl Sleeper for SimSleeper {
    fn sleep(&self, how_long: Duration) {
        self.0.state().slept.push(how_long);
    }
}
