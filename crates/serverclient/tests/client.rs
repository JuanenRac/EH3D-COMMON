//! The client against a server that lives in memory: login, renewal, refusals, a server that goes away and comes back, and the codes of the catalog.

use eh3d_serverclient::sim::{Fault, SimServer};
use eh3d_serverclient::{
    backoff, text_for, Client, ClientError, Link, MemoryStore, Secret, Settings, TokenStore,
    TransportError,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn settings() -> Settings {
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    s.set("server_machine", "bungard").unwrap();
    s
}

fn client_for(sim: &SimServer, s: Settings) -> Client {
    Client::new(s, Box::new(sim.clone()))
        .with_sleeper(Box::new(sim.sleeper()))
        .with_clock(Box::new(sim.clock()))
}

fn logged_in(sim: &SimServer) -> Client {
    let mut c = client_for(sim, settings());
    c.login("admin", "secret-password", "phoenix-cnc").unwrap();
    c
}

#[test]
fn login_gives_a_session_and_keeps_the_token_inside() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings());
    assert_eq!(c.link(), Link::Unknown);
    assert!(!c.has_token());
    let s = c.login("admin", "secret-password", "phoenix-cnc").unwrap();
    assert_eq!(s.name, "admin");
    assert_eq!(s.role, "admin");
    assert_eq!(s.app, "phoenix-cnc");
    assert_eq!(s.expires_s, 3600);
    assert!(s.session_id.is_some());
    assert!(c.has_token());
    assert_eq!(c.link(), Link::Connected);
    assert_eq!(sim.open_logins(), 1);
    // nothing printable of the client holds the token
    let shown = format!("{:?} {:?} {:?}", c.session(), c.settings(), c.link());
    assert!(!shown.contains("sim-token"));
}

#[test]
fn wrong_password_is_bad_credentials_and_leaves_no_token() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings());
    let e = c.login("admin", "nope", "phoenix-cnc").unwrap_err();
    assert_eq!(e.code(), Some("bad_credentials"));
    assert!(e.needs_login());
    assert!(!c.has_token());
    assert!(e.text(eh3d_serverclient::Lang::Es).contains("contraseña"));
    // the password is not in the error
    assert!(!format!("{e:?}").contains("nope"));
}

#[test]
fn login_without_a_good_app_is_refused_by_the_server() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings());
    let e = c.login("admin", "secret-password", "").unwrap_err();
    assert_eq!(e.code(), Some("bad_app"));
    assert_eq!(e.status(), Some(422));
}

#[test]
fn requests_carry_the_token_and_no_login_means_no_request() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings());
    let e = c.get("/api/v1/echo").unwrap_err();
    assert!(matches!(e, ClientError::Unauthorized { code: None }));
    assert_eq!(c.link(), Link::LoggedOut);
    assert_eq!(sim.request_count(), 0, "nothing is sent without a token");
    c.login("admin", "secret-password", "studio").unwrap();
    let r = c.get("/api/v1/echo").unwrap();
    assert_eq!(r.status, 200);
    let seen = sim.seen();
    let last = seen.last().unwrap();
    assert!(last.had_token && last.token_valid);
}

#[test]
fn a_401_makes_the_client_forget_the_token() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.end_all_logins();
    let e = c.get("/api/v1/echo").unwrap_err();
    assert!(e.needs_login());
    assert_eq!(e.code(), Some("unauthorized"));
    assert!(!c.has_token());
    assert!(c.session().is_none());
    assert_eq!(c.link(), Link::LoggedOut);
    // and it does not keep sending the dead token
    let before = sim.request_count();
    assert!(c.get("/api/v1/echo").is_err());
    assert_eq!(sim.request_count(), before);
}

#[test]
fn a_403_is_forbidden_and_keeps_the_login() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings());
    c.login("viewer", "viewer-password", "studio").unwrap();
    let e = c.get("/api/v1/admin/only").unwrap_err();
    assert!(matches!(e, ClientError::Forbidden { .. }));
    assert_eq!(e.code(), Some("forbidden"));
    assert!(!e.needs_login());
    assert!(c.has_token());
    assert_eq!(c.link(), Link::Connected);
}

#[test]
fn other_refusals_carry_status_and_code() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let e = c.get("/api/v1/nothing/here").unwrap_err();
    match e {
        ClientError::Server { status, code, .. } => {
            assert_eq!(status, 404);
            assert_eq!(code.as_deref(), Some("not_found"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn refresh_renews_the_login_with_the_same_token() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.advance(3000);
    let left = c.refresh().unwrap();
    assert_eq!(left, 3600);
    // 3000 + 3600 would pass the longest login (7200 from the start): later renewals are cut short
    sim.advance(3500);
    let left = c.refresh().unwrap();
    assert_eq!(left, 7200 - 6500);
    assert!(c.get("/api/v1/echo").is_ok());
}

#[test]
fn a_login_the_server_lost_makes_refresh_unauthorized() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.end_all_logins();
    let e = c.refresh().unwrap_err();
    assert!(e.needs_login());
    assert_eq!(e.code(), Some("unknown_session"));
    assert!(!c.has_token());
}

#[test]
fn a_request_renews_a_login_that_is_about_to_run_out() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.advance(3000); // 600 s left, a quarter of 3600 is 900
    c.get("/api/v1/echo").unwrap();
    let paths: Vec<String> = sim.seen().iter().map(|s| s.path.clone()).collect();
    assert!(paths.contains(&"/api/v1/auth/refresh".to_string()));
    sim.advance(1000); // would have been past the old end
    assert!(c.get("/api/v1/echo").is_ok());
}

#[test]
fn auto_refresh_can_be_turned_off() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings()).with_auto_refresh(false);
    c.login("admin", "secret-password", "studio").unwrap();
    sim.advance(3500);
    c.get("/api/v1/echo").unwrap();
    assert!(!sim.seen().iter().any(|s| s.path == "/api/v1/auth/refresh"));
}

#[test]
fn logout_forgets_the_token_even_when_the_server_is_gone() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.script(vec![Fault::NoConnection; 3]);
    let e = c.logout().unwrap_err();
    assert!(matches!(e, ClientError::Unreachable { .. }));
    assert!(!c.has_token());
    assert_eq!(c.link(), Link::Disconnected);
    // a normal logout ends the login on the server
    let mut c2 = logged_in(&sim);
    let open = sim.open_logins();
    c2.logout().unwrap();
    assert_eq!(sim.open_logins(), open - 1);
}

#[test]
fn sessions_are_listed_and_one_can_be_closed() {
    let sim = SimServer::new();
    let mut phone = client_for(&sim, settings());
    phone.login("admin", "secret-password", "android").unwrap();
    let mut c = logged_in(&sim);
    let list = c.sessions().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list.iter().filter(|s| s.current).count(), 1);
    let other = list.iter().find(|s| s.app == "android").unwrap();
    c.close_session(&other.id).unwrap();
    assert_eq!(c.sessions().unwrap().len(), 1);
    // the closed login stops working at once
    let e = phone.get("/api/v1/echo").unwrap_err();
    assert!(e.needs_login());
    // an id that never existed
    let e = c.close_session("sess-999").unwrap_err();
    assert_eq!(e.code(), Some("unknown_session"));
}

#[test]
fn a_server_that_is_down_is_retried_with_growing_pauses_then_disconnected() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.script(vec![Fault::NoConnection; 3]);
    let e = c.get("/api/v1/echo").unwrap_err();
    match &e {
        ClientError::Unreachable { error, attempts } => {
            assert!(matches!(error, TransportError::Connect(_)));
            assert_eq!(*attempts, 3, "one try and two retries by default");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(c.link(), Link::Disconnected);
    assert_eq!(
        sim.pauses(),
        vec![Duration::from_millis(200), Duration::from_millis(400)]
    );
    // the token is kept: the server was not reached, so the login was not refused
    assert!(c.has_token());
}

#[test]
fn a_server_that_comes_back_is_reached_again() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.script(vec![Fault::NoConnection, Fault::Timeout]);
    let r = c.get("/api/v1/echo").unwrap();
    assert_eq!(r.status, 200);
    assert_eq!(c.link(), Link::Connected);
    assert_eq!(sim.pauses().len(), 2);
}

#[test]
fn retries_follow_the_setting_and_zero_means_one_try() {
    let sim = SimServer::new();
    let mut s = settings();
    s.set("server_retries", "0").unwrap();
    let mut c = client_for(&sim, s);
    c.login("admin", "secret-password", "studio").unwrap();
    sim.script(vec![Fault::Timeout]);
    let e = c.get("/api/v1/echo").unwrap_err();
    assert!(matches!(
        e,
        ClientError::Unreachable {
            attempts: 1,
            error: TransportError::Timeout
        }
    ));
    assert!(sim.pauses().is_empty());
}

#[test]
fn a_post_is_never_sent_twice_after_a_timeout() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.script(vec![Fault::Timeout, Fault::Timeout, Fault::Timeout]);
    let e = c.post("/api/v1/echo", Some("{}")).unwrap_err();
    assert!(matches!(
        e,
        ClientError::Unreachable {
            attempts: 1,
            error: TransportError::Timeout
        }
    ));
    // a refused connection sent nothing, so a POST may try again
    sim.script(vec![Fault::NoConnection, Fault::NoConnection]);
    let r = c.post("/api/v1/echo", Some("{}")).unwrap();
    assert_eq!(r.status, 200);
}

#[test]
fn a_gateway_error_is_retried_for_a_get_and_not_for_a_post() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    sim.script(vec![Fault::Status(503, "unavailable")]);
    assert_eq!(c.get("/api/v1/echo").unwrap().status, 200);
    sim.script(vec![Fault::Status(503, "unavailable")]);
    let e = c.post("/api/v1/echo", Some("{}")).unwrap_err();
    assert_eq!(e.status(), Some(503));
    // a 503 that does not stop comes out as the refusal it is, after the retries
    sim.script(vec![Fault::Status(503, "unavailable"); 3]);
    let e = c.get("/api/v1/echo").unwrap_err();
    assert_eq!(e.status(), Some(503));
    assert_eq!(c.link(), Link::Connected, "the server answered");
}

#[test]
fn the_backoff_grows_and_stops_at_two_seconds() {
    let ms: Vec<u128> = (1..=6).map(|n| backoff(n).as_millis()).collect();
    assert_eq!(ms, vec![200, 400, 800, 1600, 2000, 2000]);
    assert_eq!(backoff(0).as_millis(), 200);
    assert_eq!(backoff(u32::MAX).as_millis(), 2000);
}

#[test]
fn health_needs_no_login_and_sends_no_token() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings());
    let h = c.health().unwrap();
    assert_eq!(h.name, "eh3d-server");
    assert_eq!(h.status, "ok");
    assert_eq!(h.api_revision, 1);
    assert!(!sim.seen()[0].had_token);
    assert_eq!(c.link(), Link::Connected);
}

#[test]
fn an_empty_address_is_a_config_error_and_sends_nothing() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, Settings::default());
    assert!(matches!(c.health().unwrap_err(), ClientError::Config(_)));
    assert!(matches!(
        c.login("a", "b", "studio").unwrap_err(),
        ClientError::Config(_)
    ));
    assert_eq!(sim.request_count(), 0);
}

#[test]
fn a_program_token_in_the_settings_is_used_without_a_login() {
    let sim = SimServer::new();
    let mut s = settings();
    s.set("server_token", "program-token").unwrap();
    let mut c = client_for(&sim, s);
    assert!(c.has_token());
    assert_eq!(c.get("/api/v1/echo").unwrap().status, 200);
    assert!(c.session().is_none());
    // a program token has no login to renew
    sim.advance(100_000);
    assert_eq!(c.get("/api/v1/echo").unwrap().status, 200);
}

#[test]
fn a_wrong_program_token_is_unauthorized() {
    let sim = SimServer::new();
    let mut s = settings();
    s.set("server_token", "not-the-token").unwrap();
    let mut c = client_for(&sim, s);
    assert!(c.get("/api/v1/echo").unwrap_err().needs_login());
    assert!(!c.has_token());
}

/// A store that writes what it was told to a shared place, to look at it.
#[derive(Clone, Default)]
struct SpyStore(Arc<Mutex<Option<String>>>);

impl TokenStore for SpyStore {
    fn load(&self) -> Option<Secret> {
        self.0.lock().unwrap().clone().map(Secret::new)
    }
    fn save(&self, token: &Secret) -> Result<(), String> {
        *self.0.lock().unwrap() = Some(token.expose().to_string());
        Ok(())
    }
    fn clear(&self) {
        *self.0.lock().unwrap() = None;
    }
}

#[test]
fn the_token_is_stored_at_login_taken_up_next_run_and_cleared_on_logout() {
    let sim = SimServer::new();
    let spy = SpyStore::default();
    let mut c = client_for(&sim, settings()).with_store(Box::new(spy.clone()));
    c.login("admin", "secret-password", "studio").unwrap();
    assert!(spy.0.lock().unwrap().is_some());
    // a new run of the program takes the stored token up and needs no login
    let mut next = client_for(&sim, settings()).with_store(Box::new(spy.clone()));
    assert!(next.has_token());
    assert_eq!(next.get("/api/v1/echo").unwrap().status, 200);
    next.logout().unwrap();
    assert!(spy.0.lock().unwrap().is_none());
}

#[test]
fn a_401_clears_the_stored_token_too() {
    let sim = SimServer::new();
    let spy = SpyStore::default();
    let mut c = client_for(&sim, settings()).with_store(Box::new(spy.clone()));
    c.login("admin", "secret-password", "studio").unwrap();
    sim.end_all_logins();
    assert!(c.get("/api/v1/echo").is_err());
    assert!(spy.0.lock().unwrap().is_none());
}

struct BrokenStore;

impl TokenStore for BrokenStore {
    fn load(&self) -> Option<Secret> {
        None
    }
    fn save(&self, _: &Secret) -> Result<(), String> {
        Err("disk full".to_string())
    }
    fn clear(&self) {}
}

#[test]
fn a_store_that_fails_does_not_fail_the_login() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings()).with_store(Box::new(BrokenStore));
    c.login("admin", "secret-password", "studio").unwrap();
    assert_eq!(c.store_warning(), Some("disk full"));
    assert!(c.has_token());
}

#[test]
fn the_memory_store_round_trips() {
    let m = MemoryStore::default();
    assert!(m.load().is_none());
    m.save(&Secret::new("x")).unwrap();
    assert_eq!(m.load().unwrap().expose(), "x");
    m.clear();
    assert!(m.load().is_none());
}

#[test]
fn the_machine_path_is_made_safe() {
    let sim = SimServer::new();
    let mut s = settings();
    s.set("server_machine", "my machine/1?x").unwrap();
    let c = client_for(&sim, s);
    assert_eq!(
        c.machine_path("/jobs").unwrap(),
        "/api/v1/machines/my%20machine%2F1%3Fx/jobs"
    );
    let c = client_for(&sim, Settings::default());
    assert!(matches!(
        c.machine_path("/jobs").unwrap_err(),
        ClientError::Config(_)
    ));
}

#[test]
fn error_text_comes_in_eight_languages_and_falls_back_to_the_code() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings());
    let e = c.login("admin", "bad", "studio").unwrap_err();
    let mut seen = std::collections::HashSet::new();
    for lang in eh3d_serverclient::Lang::ALL {
        let t = e.text(lang);
        assert!(!t.is_empty());
        seen.insert(t);
    }
    assert_eq!(seen.len(), 8, "eight different texts");
    assert_eq!(
        text_for("a_code_of_a_newer_server", eh3d_serverclient::Lang::De),
        "a_code_of_a_newer_server"
    );
    // a failure with no code of the table speaks plain words in the language
    let e = ClientError::Unreachable {
        error: TransportError::Timeout,
        attempts: 3,
    };
    assert!(e.text(eh3d_serverclient::Lang::Pt).contains("servidor"));
    assert!(e.to_string().contains("cannot be reached"));
}

#[test]
fn discovery_is_off_by_default_and_asks_nothing_of_the_network() {
    struct Boom;
    impl eh3d_serverclient::Discover for Boom {
        fn find(&self, _: Duration) -> Vec<String> {
            panic!("the network must not be asked while server_mdns is off");
        }
    }
    let sim = SimServer::new();
    let c = client_for(&sim, settings()).with_discovery(Box::new(Boom));
    assert!(!c.settings().server_mdns);
    assert!(matches!(c.discover().unwrap_err(), ClientError::Config(_)));

    struct One;
    impl eh3d_serverclient::Discover for One {
        fn find(&self, _: Duration) -> Vec<String> {
            vec!["192.168.0.180:18200".to_string()]
        }
    }
    let mut s = settings();
    s.set("server_mdns", "on").unwrap();
    let c = client_for(&sim, s).with_discovery(Box::new(One));
    assert_eq!(c.discover().unwrap(), vec!["192.168.0.180:18200"]);
    // on, but with no resolver: a clear error, not a silent nothing
    let mut s = settings();
    s.set("server_mdns", "on").unwrap();
    let c = client_for(&sim, s);
    assert!(matches!(c.discover().unwrap_err(), ClientError::Config(_)));
}
