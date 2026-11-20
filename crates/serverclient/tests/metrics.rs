//! The metrics of the link against a server that lives in memory: what a request, a retry, a loss and a recovery do to the counters.

use eh3d_serverclient::sim::{Fault, SimServer};
use eh3d_serverclient::{Client, Link, Settings};

fn settings(retries: u32) -> Settings {
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    s.set("server_retries", &retries.to_string()).unwrap();
    s
}

fn logged_in(sim: &SimServer, retries: u32) -> Client {
    let mut c = Client::new(settings(retries), Box::new(sim.clone()))
        .with_sleeper(Box::new(sim.sleeper()))
        .with_clock(Box::new(sim.clock()));
    c.login("admin", "secret-password", "phoenix-cnc").unwrap();
    c.reset_metrics();
    c
}

#[test]
fn a_request_that_works_counts_one_request_one_try_and_a_latency() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim, 2);
    c.get("/api/v1/echo").unwrap();
    let m = c.metrics();
    assert_eq!(m.requests(), 1);
    assert_eq!(m.attempts(), 1);
    assert_eq!(m.retries(), 0);
    assert_eq!(m.replies(), 1);
    assert_eq!(m.failures(), 0);
    assert!(m.last_latency().is_some());
    assert!(m.percentile(95) >= m.percentile(50));
}

#[test]
fn the_tries_after_a_network_failure_are_retries() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim, 2);
    sim.script(vec![Fault::NoConnection, Fault::Timeout]);
    c.get("/api/v1/echo").unwrap();
    let m = c.metrics();
    assert_eq!(m.requests(), 1);
    assert_eq!(m.attempts(), 3);
    assert_eq!(m.retries(), 2);
    assert_eq!(m.replies(), 1);
    assert_eq!(m.failures(), 0);
    assert_eq!(m.disconnects(), 0);
}

#[test]
fn a_503_that_is_tried_again_counts_as_a_server_error_and_a_retry() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim, 2);
    sim.script(vec![Fault::Status(503, "unavailable")]);
    c.get("/api/v1/echo").unwrap();
    let m = c.metrics();
    assert_eq!(m.server_errors(), 1);
    assert_eq!(m.retries(), 1);
    assert_eq!(m.replies(), 2);
}

#[test]
fn losing_the_server_is_one_disconnect_until_it_comes_back() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim, 1);
    sim.script(vec![Fault::NoConnection; 4]);
    assert!(c.get("/api/v1/echo").is_err());
    assert_eq!(c.link(), Link::Disconnected);
    assert!(c.get("/api/v1/echo").is_err());
    assert_eq!(c.metrics().failures(), 2);
    assert_eq!(c.metrics().disconnects(), 1);
    assert_eq!(c.metrics().attempts(), 4);

    c.get("/api/v1/echo").unwrap();
    assert_eq!(c.link(), Link::Connected);
    sim.script(vec![Fault::NoConnection; 2]);
    assert!(c.get("/api/v1/echo").is_err());
    assert_eq!(c.metrics().failures(), 3);
    assert_eq!(c.metrics().disconnects(), 2);
    assert!((c.metrics().failure_rate() - 0.75).abs() < 1e-9);
}

#[test]
fn a_post_that_timed_out_is_one_attempt_and_one_failure() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim, 5);
    sim.script(vec![Fault::Timeout]);
    assert!(c.post("/api/v1/echo", Some("{}")).is_err());
    assert_eq!(c.metrics().attempts(), 1);
    assert_eq!(c.metrics().retries(), 0);
    assert_eq!(c.metrics().failures(), 1);
}

#[test]
fn the_text_lists_every_counter_and_a_reset_empties_it() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim, 2);
    c.get("/api/v1/echo").unwrap();
    let text = c.metrics().to_text();
    for key in [
        "requests = 1",
        "attempts = 1",
        "retries = 0",
        "replies = 1",
        "server_errors = 0",
        "failures = 0",
        "disconnects = 0",
        "latency_p50_us = ",
        "latency_p95_us = ",
    ] {
        assert!(text.contains(key), "{key} in {text}");
    }
    assert!(!text.contains("secret"));
    c.reset_metrics();
    assert_eq!(c.metrics().requests(), 0);
    assert_eq!(c.metrics().last_latency(), None);
}
