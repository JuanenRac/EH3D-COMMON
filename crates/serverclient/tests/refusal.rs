//! The `why` of a refusal, the masked piece of its body and the token that a `401` does not take away: against the server in memory.

use eh3d_serverclient::sim::{Fault, SimServer};
use eh3d_serverclient::{body_excerpt, Client, ClientError, Link, Settings, MAX_WHY_CHARS};

const TOKEN: &str = "program-token";

fn settings(extra: &[(&str, &str)]) -> Settings {
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    s.set("server_machine", "head").unwrap();
    s.set("server_token", TOKEN).unwrap();
    for (k, v) in extra {
        s.set(k, v).unwrap();
    }
    s
}

fn client_for(sim: &SimServer, s: Settings) -> Client {
    Client::new(s, Box::new(sim.clone()))
        .with_sleeper(Box::new(sim.sleeper()))
        .with_clock(Box::new(sim.clock()))
}

#[test]
fn a_fixed_token_is_kept_after_a_401_and_asked_again() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[("server_token_fixed", "on")]));
    assert!(c.is_token_fixed());
    sim.script(vec![Fault::Status(401, "unauthorized")]);
    let e = c.get("/api/v1/echo").unwrap_err();
    assert!(e.needs_login());
    assert!(c.has_token(), "a fixed token is not forgotten");
    assert_eq!(
        c.link(),
        Link::LoggedOut,
        "the refusal still shows on the link"
    );
    // the next question really goes to the server, and its answer brings the link back
    let before = sim.request_count();
    assert_eq!(c.get("/api/v1/echo").unwrap().status, 200);
    assert_eq!(sim.request_count(), before + 1);
    assert_eq!(c.link(), Link::Connected);
    // three refusals in a row, still asked every time
    sim.script(vec![
        Fault::Status(401, "unauthorized"),
        Fault::Status(401, "unauthorized"),
        Fault::Status(401, "unauthorized"),
    ]);
    for _ in 0..3 {
        assert!(c.get("/api/v1/echo").is_err());
        assert!(c.has_token());
    }
    assert_eq!(c.get("/api/v1/echo").unwrap().status, 200);
}

#[test]
fn without_a_fixed_token_a_401_forgets_it_as_before() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[]));
    assert!(!c.is_token_fixed(), "off by default");
    sim.script(vec![Fault::Status(401, "unauthorized")]);
    assert!(c.get("/api/v1/echo").unwrap_err().needs_login());
    assert!(!c.has_token());
    assert_eq!(c.link(), Link::LoggedOut);
    // nothing is sent until there is a token again
    let before = sim.request_count();
    assert!(c.get("/api/v1/echo").unwrap_err().needs_login());
    assert_eq!(sim.request_count(), before);
}

#[test]
fn the_builder_overrides_the_setting_both_ways() {
    let sim = SimServer::new();
    let c = client_for(&sim, settings(&[("server_token_fixed", "on")])).with_fixed_token(false);
    assert!(!c.is_token_fixed());
    let c = client_for(&sim, settings(&[])).with_fixed_token(true);
    assert!(c.is_token_fixed());
}

#[test]
fn a_409_says_why_and_the_error_keeps_its_three_fields() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[]));
    sim.script(vec![Fault::Body(
        409,
        r#"{"ok":false,"why":"busy"}"#.to_string(),
    )]);
    let e = c.post("/api/v1/echo", Some("{}")).unwrap_err();
    assert_eq!(
        e,
        ClientError::Server {
            status: 409,
            code: None,
            detail: String::new()
        }
    );
    let r = c.last_refusal().expect("a refusal was recorded");
    assert_eq!(r.status, 409);
    assert_eq!(r.why.as_deref(), Some("busy"));
    assert!(r.belongs_to(&e));
    assert_eq!(c.why_of(&e), Some("busy"));
    assert_eq!(r.body_excerpt, None, "no body unless it was asked for");
}

#[test]
fn a_409_with_a_detail_and_no_why_has_no_why() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[]));
    sim.script(vec![Fault::Body(
        409,
        r#"{"error":"conflict","detail":"the head holds a part"}"#.to_string(),
    )]);
    let e = c.post("/api/v1/echo", Some("{}")).unwrap_err();
    match &e {
        ClientError::Server { code, detail, .. } => {
            assert_eq!(code.as_deref(), Some("conflict"));
            assert_eq!(detail, "the head holds a part");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(c.why_of(&e), None);
    assert_eq!(c.last_refusal().unwrap().code.as_deref(), Some("conflict"));
}

#[test]
fn a_why_of_another_refusal_is_not_given_for_this_error() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[]));
    sim.script(vec![Fault::Body(409, r#"{"why":"busy"}"#.to_string())]);
    let first = c.post("/api/v1/echo", Some("{}")).unwrap_err();
    let other = ClientError::Server {
        status: 412,
        code: None,
        detail: String::new(),
    };
    assert_eq!(c.why_of(&first), Some("busy"));
    assert_eq!(c.why_of(&other), None);
}

#[test]
fn the_why_is_cut_and_is_forgotten_after_a_good_request() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[]));
    let long = "w".repeat(MAX_WHY_CHARS * 4);
    sim.script(vec![Fault::Body(409, format!(r#"{{"why":"{long}"}}"#))]);
    assert!(c.post("/api/v1/echo", Some("{}")).is_err());
    assert_eq!(
        c.last_refusal()
            .unwrap()
            .why
            .as_ref()
            .unwrap()
            .chars()
            .count(),
        MAX_WHY_CHARS
    );
    c.get("/api/v1/echo").unwrap();
    assert!(c.last_refusal().is_none(), "a good request clears it");
    sim.script(vec![Fault::Body(409, r#"{"why":"busy"}"#.to_string())]);
    assert!(c.post("/api/v1/echo", Some("{}")).is_err());
    sim.script(vec![
        Fault::NoConnection,
        Fault::NoConnection,
        Fault::NoConnection,
    ]);
    assert!(c.get("/api/v1/echo").is_err());
    assert!(c.last_refusal().is_none(), "no answer, no refusal");
}

#[test]
fn a_giant_body_is_cut_to_the_bytes_asked_for() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[("server_error_body_bytes", "300")]));
    let giant = format!("{{\"detail\":\"{}\"}}", "x".repeat(3 * 1024 * 1024));
    sim.script(vec![Fault::Body(500, giant)]);
    assert!(c.get("/api/v1/echo").is_err());
    let excerpt = c.last_refusal().unwrap().body_excerpt.clone().unwrap();
    assert_eq!(excerpt.len(), 300);
    assert!(excerpt.starts_with("{\"detail\":\"xxx"));
}

#[test]
fn the_default_keeps_no_body_at_all() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[]));
    sim.script(vec![Fault::Body(500, "something broke".to_string())]);
    assert!(c.get("/api/v1/echo").is_err());
    assert_eq!(c.last_refusal().unwrap().body_excerpt, None);
}

#[test]
fn the_token_and_the_keys_are_masked_in_the_body_kept() {
    let sim = SimServer::new();
    let mut c = client_for(&sim, settings(&[("server_error_body_bytes", "4096")]));
    let body = format!(
        r#"{{"error":"bad","detail":"sent {TOKEN} by mistake","password":"hunter2","api_key":"k-123","note":"fine","Authorization":"Bearer abc.def"}}"#
    );
    sim.script(vec![Fault::Body(422, body)]);
    assert!(c.post("/api/v1/echo", Some("{}")).is_err());
    let r = c.last_refusal().unwrap();
    let excerpt = r.body_excerpt.clone().unwrap();
    for secret in [TOKEN, "hunter2", "k-123", "abc.def"] {
        assert!(!excerpt.contains(secret), "{secret} leaked: {excerpt}");
    }
    assert!(excerpt.contains(r#""note":"fine""#), "{excerpt}");
    assert!(excerpt.contains(r#""password":"***""#), "{excerpt}");
    // nothing printable of the refusal holds the token either
    assert!(!format!("{r:?}").contains(TOKEN));
}

#[test]
fn the_secret_forms_of_a_plain_text_body_are_masked() {
    let cases = [
        ("Authorization: Bearer s3cr3t-value", "s3cr3t-value"),
        ("token=abc123&user=ann", "abc123"),
        ("key = long-key-text; mode=x", "long-key-text"),
        ("{\"password\": \"p@ss\"}", "p@ss"),
        ("Cookie: session=zzz", "zzz"),
    ];
    for (body, secret) in cases {
        let shown = body_excerpt(body, 4096, None).unwrap();
        assert!(!shown.contains(secret), "{body} -> {shown}");
        assert!(shown.contains("***"), "{body} -> {shown}");
    }
    let plain = body_excerpt("nothing to hide here: 7 holes", 4096, None).unwrap();
    assert_eq!(plain, "nothing to hide here: 7 holes");
    assert_eq!(body_excerpt("anything", 0, None), None);
    assert_eq!(body_excerpt("", 10, None), None);
}

#[test]
fn the_cut_never_splits_a_character() {
    let body = "\u{e9}\u{4e2d}\u{1f600}".repeat(50);
    for max in 1..40 {
        let shown = body_excerpt(&body, max, None).unwrap();
        assert!(shown.len() <= max);
        assert!(body.starts_with(&shown));
    }
}

/// xorshift64*, with a fixed seed: the same bodies on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

#[test]
fn no_body_made_by_a_seed_shows_the_token_or_a_secret_value() {
    let pieces = [
        "{", "}", "\"", ":", ",", " ", "token", "password", "key", "why", "busy", "\n", "=", "&",
        "Bearer ", "\u{e9}", "ok", "detail",
    ];
    let mut rng = Rng(0x00C0_FFEE_5EED);
    for _ in 0..2000 {
        let mut body = String::new();
        for _ in 0..(rng.next() % 40) {
            body.push_str(pieces[(rng.next() % pieces.len() as u64) as usize]);
        }
        // the token is put in at a place made by the seed, whole
        let at = (rng.next() as usize) % (body.len() + 1);
        let at = (0..=at).rev().find(|i| body.is_char_boundary(*i)).unwrap();
        body.insert_str(at, TOKEN);
        let max = 1 + (rng.next() % 4096) as usize;
        let shown = body_excerpt(&body, max, Some(TOKEN)).unwrap();
        assert!(!shown.contains(TOKEN), "{body} -> {shown}");
        assert!(shown.len() <= max);
    }
}

#[test]
fn a_secret_value_under_a_secret_name_never_shows_whatever_the_cut() {
    let body = r#"{"a":1,"password":"correct-horse-battery","b":2}"#;
    for max in 1..=body.len() {
        let shown = body_excerpt(body, max, None).unwrap();
        assert!(!shown.contains("correct"), "{max}: {shown}");
        assert!(!shown.contains("horse"), "{max}: {shown}");
    }
}
