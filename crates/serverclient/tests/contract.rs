//! The contract of the server read through the client: a route that disappears from it is noticed.

use eh3d_serverclient::contract::DEFAULT_CONTRACT_PATH;
use eh3d_serverclient::sim::SimServer;
use eh3d_serverclient::{Client, ClientError, ContractCache, RouteStatus, Settings};

const V1: &str = r#"{
  "openapi": "3.1.0",
  "servers": [{"url": "/api/v1"}],
  "paths": {
    "/health": {"get": {}},
    "/jobs": {"get": {}, "post": {}},
    "/jobs/{id}": {"get": {}, "delete": {}},
    "/legacy/status": {"get": {}}
  }
}"#;

const V2: &str = r#"{
  "openapi": "3.1.0",
  "servers": [{"url": "/api/v1"}],
  "paths": {
    "/health": {"get": {}},
    "/jobs": {"get": {}, "post": {}},
    "/jobs/{id}": {"get": {}, "delete": {}},
    "/machines": {"get": {}}
  }
}"#;

fn client(sim: &SimServer) -> Client {
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    Client::new(s, Box::new(sim.clone()))
        .with_sleeper(Box::new(sim.sleeper()))
        .with_clock(Box::new(sim.clock()))
}

#[test]
fn the_first_contract_is_the_baseline_and_the_next_one_names_what_left() {
    let sim = SimServer::new();
    let mut c = client(&sim);
    let mut cache = ContractCache::new();

    sim.set_contract(V1);
    let first = c
        .refresh_contract(&mut cache, DEFAULT_CONTRACT_PATH)
        .unwrap();
    assert!(first.is_empty());
    assert_eq!(
        cache.status("GET", "/api/v1/legacy/status"),
        RouteStatus::Known
    );

    sim.set_contract(V2);
    let second = c
        .refresh_contract(&mut cache, DEFAULT_CONTRACT_PATH)
        .unwrap();
    assert_eq!(
        second.retired,
        vec!["GET /api/v1/legacy/status".to_string()]
    );
    assert_eq!(second.added, vec!["GET /api/v1/machines".to_string()]);
    assert_eq!(
        cache.status("GET", "/api/v1/legacy/status"),
        RouteStatus::Retired
    );
    assert_eq!(cache.status("GET", "/api/v1/machines"), RouteStatus::Known);
    assert_eq!(cache.status("GET", "/api/v1/jobs/7"), RouteStatus::Known);
    assert_eq!(cache.status("PUT", "/api/v1/jobs/7"), RouteStatus::Unknown);
}

#[test]
fn the_contract_is_read_with_the_token_when_there_is_one_and_without_it_when_not() {
    let sim = SimServer::new();
    sim.set_contract(V1);
    let mut c = client(&sim);
    c.fetch_contract(DEFAULT_CONTRACT_PATH).unwrap();
    c.login("admin", "secret-password", "studio").unwrap();
    c.fetch_contract(DEFAULT_CONTRACT_PATH).unwrap();
    let seen = sim.seen();
    let reads: Vec<bool> = seen
        .iter()
        .filter(|s| s.path == DEFAULT_CONTRACT_PATH)
        .map(|s| s.had_token)
        .collect();
    assert_eq!(reads, vec![false, true]);
}

#[test]
fn a_server_that_does_not_serve_its_contract_is_an_error_not_an_empty_contract() {
    let sim = SimServer::new();
    let mut c = client(&sim);
    let mut cache = ContractCache::new();
    let err = c
        .refresh_contract(&mut cache, DEFAULT_CONTRACT_PATH)
        .unwrap_err();
    assert!(matches!(err, ClientError::Server { status: 404, .. }));
    assert!(cache.current().is_empty());
}

#[test]
fn an_answer_that_is_not_a_contract_is_a_bad_reply_and_the_cache_is_not_touched() {
    let sim = SimServer::new();
    let mut c = client(&sim);
    let mut cache = ContractCache::new();
    sim.set_contract(V1);
    c.refresh_contract(&mut cache, DEFAULT_CONTRACT_PATH)
        .unwrap();
    let before = cache.clone();
    for body in ["<html>maintenance</html>", "{}", "{\"paths\":{}}"] {
        sim.set_contract(body);
        let err = c
            .refresh_contract(&mut cache, DEFAULT_CONTRACT_PATH)
            .unwrap_err();
        assert!(matches!(err, ClientError::BadReply(_)), "{body}");
        assert_eq!(cache, before);
    }
}

#[test]
fn the_cache_kept_in_a_file_remembers_what_was_retired_in_the_next_run() {
    let sim = SimServer::new();
    let mut c = client(&sim);
    let mut cache = ContractCache::new();
    sim.set_contract(V1);
    c.refresh_contract(&mut cache, DEFAULT_CONTRACT_PATH)
        .unwrap();
    sim.set_contract(V2);
    c.refresh_contract(&mut cache, DEFAULT_CONTRACT_PATH)
        .unwrap();

    let reloaded = ContractCache::from_text(&cache.to_text()).unwrap();
    assert_eq!(
        reloaded.status("GET", "/api/v1/legacy/status"),
        RouteStatus::Retired
    );
    assert_eq!(reloaded, cache);
}
