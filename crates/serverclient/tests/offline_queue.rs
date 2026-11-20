//! The queue of requests that wait for the server, against a server that goes away and comes back.

use eh3d_serverclient::sim::{Fault, SimServer};
use eh3d_serverclient::{
    Client, ClientError, Link, OfflineQueue, Outcome, QueueLimits, Rejected, Settings,
};

fn settings() -> Settings {
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    s
}

fn logged_in(sim: &SimServer) -> Client {
    let mut c = Client::new(settings(), Box::new(sim.clone()))
        .with_sleeper(Box::new(sim.sleeper()))
        .with_clock(Box::new(sim.clock()));
    c.login("admin", "secret-password", "phoenix-cnc").unwrap();
    c
}

fn down(sim: &SimServer, tries: usize) {
    sim.script(vec![Fault::NoConnection; tries]);
}

#[test]
fn a_put_that_cannot_be_sent_waits_and_goes_out_when_the_server_is_back() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::default();
    down(&sim, 3);
    let out = c
        .send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("one"))
        .unwrap();
    assert_eq!(out, Outcome::Queued);
    assert_eq!(c.link(), Link::Disconnected);
    assert_eq!(q.len(), 1);
    assert_eq!(q.total_bytes(), 3);

    let report = c.flush(&mut q);
    assert_eq!(report.sent, 1);
    assert_eq!(report.remaining, 0);
    assert!(report.stopped.is_none() && report.refused.is_empty());
    assert!(q.is_empty());
    assert_eq!(c.link(), Link::Connected);
    let last = sim.seen().pop().unwrap();
    assert_eq!(
        (last.method.as_str(), last.path.as_str()),
        ("PUT", "/api/v1/echo")
    );
}

#[test]
fn a_post_is_never_queued() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::default();
    down(&sim, 3);
    let err = c
        .send_or_queue(&mut q, "POST", "/api/v1/echo", Some("{}"))
        .unwrap_err();
    assert!(matches!(err, ClientError::Unreachable { .. }));
    assert!(q.is_empty());
}

#[test]
fn a_server_that_answers_is_not_a_reason_to_queue() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::default();
    // a refusal comes back as the error it is, and a good answer as the answer
    let err = c
        .send_or_queue(&mut q, "PUT", "/api/v1/nowhere", Some("x"))
        .unwrap_err();
    assert!(matches!(err, ClientError::Server { status: 404, .. }));
    let ok = c
        .send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("x"))
        .unwrap();
    assert!(matches!(ok, Outcome::Done(_)));
    assert!(q.is_empty());
}

#[test]
fn the_queue_refuses_what_it_cannot_hold_and_says_why() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::new(QueueLimits {
        max_items: 1,
        max_body_bytes: 4,
        max_total_bytes: 100,
        max_age_s: 100,
    });
    down(&sim, 9);
    assert_eq!(
        c.send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("toolarge"))
            .unwrap(),
        Outcome::NotQueued(Rejected::BodyTooLarge)
    );
    assert_eq!(
        c.send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("ok"))
            .unwrap(),
        Outcome::Queued
    );
    assert_eq!(
        c.send_or_queue(&mut q, "DELETE", "/api/v1/other", None)
            .unwrap(),
        Outcome::NotQueued(Rejected::Full)
    );
    assert_eq!(q.len(), 1);
    assert_eq!(q.refused(), 2);
}

#[test]
fn a_flush_while_the_server_is_still_away_keeps_everything_in_order() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::default();
    down(&sim, 6);
    c.send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("a"))
        .unwrap();
    c.send_or_queue(&mut q, "DELETE", "/api/v1/auth/sessions/gone", None)
        .unwrap();
    assert_eq!(q.len(), 2);
    down(&sim, 3);
    let report = c.flush(&mut q);
    assert_eq!(report.sent, 0);
    assert_eq!(report.remaining, 2);
    assert!(matches!(
        report.stopped,
        Some(ClientError::Unreachable { .. })
    ));
    let first = q.iter().next().unwrap();
    assert_eq!(first.path, "/api/v1/echo");
}

#[test]
fn a_flush_goes_on_after_a_refusal_and_reports_it() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::default();
    down(&sim, 6);
    c.send_or_queue(&mut q, "DELETE", "/api/v1/auth/sessions/gone", None)
        .unwrap();
    c.send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("b"))
        .unwrap();
    let report = c.flush(&mut q);
    assert_eq!(report.sent, 1);
    assert_eq!(report.refused.len(), 1);
    assert_eq!(report.refused[0].0.path, "/api/v1/auth/sessions/gone");
    assert!(matches!(
        report.refused[0].1,
        ClientError::Server { status: 404, .. }
    ));
    assert_eq!(report.remaining, 0);
    let paths: Vec<String> = sim.seen().iter().map(|s| s.path.clone()).collect();
    let gone = paths.iter().rposition(|p| p.ends_with("/gone")).unwrap();
    let echo = paths.iter().rposition(|p| p == "/api/v1/echo").unwrap();
    assert!(gone < echo, "oldest first: {paths:?}");
}

#[test]
fn a_flush_that_finds_the_login_gone_stops_and_keeps_the_rest() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::default();
    down(&sim, 3);
    c.send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("c"))
        .unwrap();
    sim.end_all_logins();
    let report = c.flush(&mut q);
    assert_eq!(report.sent, 0);
    assert_eq!(report.remaining, 1);
    assert!(matches!(
        report.stopped,
        Some(ClientError::Unauthorized { .. })
    ));
    assert_eq!(c.link(), Link::LoggedOut);
    // after a new login the same request goes out
    c.login("admin", "secret-password", "phoenix-cnc").unwrap();
    let again = c.flush(&mut q);
    assert_eq!(again.sent, 1);
    assert_eq!(again.remaining, 0);
}

#[test]
fn what_waited_longer_than_the_limit_is_dropped_instead_of_sent() {
    let sim = SimServer::new();
    let mut c = logged_in(&sim);
    let mut q = OfflineQueue::new(QueueLimits {
        max_age_s: 60,
        ..QueueLimits::default()
    });
    down(&sim, 3);
    c.send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("old"))
        .unwrap();
    sim.advance(61);
    let before = sim.request_count();
    let report = c.flush(&mut q);
    assert_eq!(report.expired, 1);
    assert_eq!(report.sent, 0);
    assert_eq!(report.remaining, 0);
    assert_eq!(sim.request_count(), before, "nothing was sent");
}

#[test]
fn without_a_server_address_nothing_is_queued() {
    let sim = SimServer::new();
    let mut c = Client::new(Settings::default(), Box::new(sim.clone()));
    let mut q = OfflineQueue::default();
    let err = c
        .send_or_queue(&mut q, "PUT", "/api/v1/echo", Some("x"))
        .unwrap_err();
    assert!(matches!(err, ClientError::Config(_)));
    assert!(q.is_empty());
}
