//! The plain HTTP transport against a real socket on this computer.

use eh3d_serverclient::{Client, ClientError, Link, Settings, TransportError};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

/// Answers one connection with `answer` and gives back what the client wrote.
fn one_shot(answer: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut got = Vec::new();
            let mut buf = [0u8; 1024];
            // the request is small: read until the end of the head and the short body arrived
            loop {
                let n = stream.read(&mut buf).unwrap_or(0);
                got.extend_from_slice(&buf[..n]);
                if n == 0 || got.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&got).to_string());
            let _ = stream.write_all(answer.as_bytes());
        }
    });
    (address, rx)
}

fn settings(address: &str) -> Settings {
    let mut s = Settings::default();
    s.set("server_address", address).unwrap();
    s.set("server_timeout_s", "5").unwrap();
    s.set("server_retries", "0").unwrap();
    s
}

#[test]
fn health_goes_over_a_real_socket_without_a_token() {
    let (address, seen) = one_shot("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"name\":\"eh3d-server\",\"version\":\"1.2.3\",\"api_revision\":1,\"status\":\"ok\"}");
    let mut c = Client::connect(settings(&address));
    let h = c.health().unwrap();
    assert_eq!(h.version, "1.2.3");
    assert_eq!(c.link(), Link::Connected);
    let request = seen.recv().unwrap();
    assert!(request.starts_with("GET /api/v1/health HTTP/1.1\r\n"));
    assert!(!request.to_ascii_lowercase().contains("authorization"));
}

#[test]
fn a_program_token_travels_as_a_bearer_and_a_401_is_unauthorized() {
    let (address, seen) = one_shot("HTTP/1.1 401 Unauthorized\r\nConnection: close\r\n\r\n{\"error\":\"unauthorized\",\"detail\":\"\"}");
    let mut s = settings(&address);
    s.set("server_token", "tok-xyz").unwrap();
    let mut c = Client::connect(s);
    let e = c.get("/api/v1/fleet").unwrap_err();
    assert!(e.needs_login());
    let request = seen.recv().unwrap();
    assert!(request.contains("Authorization: Bearer tok-xyz\r\n"));
    assert!(!c.has_token());
}

#[test]
fn nothing_listening_is_unreachable_and_disconnected() {
    // a port that was free a moment ago and has nobody now
    let address = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().to_string()
    };
    let mut s = settings(&address);
    s.set("server_token", "t").unwrap();
    let mut c = Client::connect(s);
    match c.get("/api/v1/fleet").unwrap_err() {
        ClientError::Unreachable { error, attempts } => {
            assert!(matches!(error, TransportError::Connect(_)));
            assert_eq!(attempts, 1);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(c.link(), Link::Disconnected);
}

#[test]
fn a_path_that_could_split_the_request_is_refused_before_it_is_sent() {
    let mut s = settings("127.0.0.1:1");
    s.set("server_token", "t").unwrap();
    let mut c = Client::connect(s);
    let e = c.get("/api/v1/x\r\nX-Evil: 1").unwrap_err();
    assert!(matches!(
        e,
        ClientError::Unreachable {
            error: TransportError::Connect(_),
            ..
        }
    ));
}
