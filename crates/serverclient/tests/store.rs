//! Headers, bodies of bytes and the rule of the tag of the project store: the helpers, the request parts, the client against the server in memory and the
//! plain transport against a real socket.

use eh3d_serverclient::sim::{Fault, SimServer};
use eh3d_serverclient::{
    check_headers, etag_header, parse_etag, project_file_path, Client, ClientError, FullReply,
    Lang, Reply, Request, RequestParts, Settings, StoreError, TcpTransport, Transport,
    TransportError,
};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn settings() -> Settings {
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    s
}

fn client_as(sim: &SimServer, name: &str, password: &str) -> Client {
    let mut c = Client::new(settings(), Box::new(sim.clone()))
        .with_sleeper(Box::new(sim.sleeper()))
        .with_clock(Box::new(sim.clock()));
    c.login(name, password, "phoenix-cad").unwrap();
    c
}

fn admin(sim: &SimServer) -> Client {
    client_as(sim, "admin", "secret-password")
}

fn file_path() -> String {
    project_file_path("phoenix-cad", "p-1", "layers/top.gbr").unwrap()
}

#[test]
fn a_tag_becomes_the_value_of_a_header_and_back() {
    assert_eq!(etag_header("abc123").unwrap(), "\"abc123\"");
    assert_eq!(etag_header("\"abc123\"").unwrap(), "\"abc123\"");
    assert_eq!(etag_header("W/\"abc123\"").unwrap(), "\"abc123\"");
    assert_eq!(etag_header("*").unwrap(), "*");
    assert_eq!(parse_etag("W/\"abc\""), "abc");
    assert_eq!(parse_etag("  \"abc\"  "), "abc");
    assert_eq!(parse_etag("abc"), "abc");
    for bad in [
        "",
        "\"\"",
        "a b",
        "a\"b",
        "a\r\nX: 1",
        "\u{e9}tag",
        &"x".repeat(129),
    ] {
        assert!(etag_header(bad).is_err(), "{bad:?}");
    }
    assert!(etag_header(&"x".repeat(128)).is_ok());
}

#[test]
fn the_path_of_a_file_is_made_safe() {
    assert_eq!(
        file_path(),
        "/api/v1/apps/phoenix-cad/projects/p-1/files/content?name=layers%2Ftop.gbr"
    );
    let p = project_file_path("studio", "p x/../y", "a b&c=d#e").unwrap();
    assert!(!p.contains(' ') && !p.contains('#'));
    // the project is one segment of the path: its slashes are escaped
    assert_eq!(p.matches('/').count(), 8);
    assert_eq!(p.matches('?').count(), 1);
    assert_eq!(p.matches('&').count(), 0);
    for (app, project, name) in [
        ("", "p", "f"),
        ("Studio", "p", "f"),
        ("a/b", "p", "f"),
        ("studio", "", "f"),
        ("studio", "..", "f"),
        ("studio", ".", "f"),
        ("studio", "p", ""),
    ] {
        assert!(project_file_path(app, project, name).is_err());
    }
    assert!(project_file_path("studio", "p", &"n".repeat(241)).is_err());
}

fn pair(n: &str, v: &str) -> (String, String) {
    (n.to_string(), v.to_string())
}

#[test]
fn only_the_allowed_headers_pass_and_a_line_break_never_does() {
    let ok = vec![
        pair("If-Match", "\"abc\""),
        pair("if-none-match", "*"),
        pair("Content-Type", "application/octet-stream"),
        pair("Accept", "*/*"),
        pair("X-Api-Key", "k-123"),
        pair("X-Request-Id", "r-1"),
        pair("Accept-Language", "es"),
    ];
    assert!(check_headers(&ok).is_ok());
    for bad in [
        pair("Authorization", "Bearer x"),
        pair("Host", "evil"),
        pair("Content-Length", "0"),
        pair("Transfer-Encoding", "chunked"),
        pair("Connection", "keep-alive"),
        pair("Cookie", "a=b"),
        pair("If-Match", "\"a\"\r\nX-Evil: 1"),
        pair("If-Match", "\"a\"\nX-Evil: 1"),
        pair("If-Match", "a\0b"),
        pair("If-Match", "\u{e9}"),
        pair("If-Match", ""),
        pair("If-Match", " lead"),
        pair("If-Match", "trail "),
        pair("If\r\nMatch", "x"),
        pair("If Match", "x"),
        pair("", "x"),
        pair("Accept", &"a".repeat(513)),
    ] {
        assert!(
            check_headers(std::slice::from_ref(&bad)).is_err(),
            "{bad:?}"
        );
    }
    assert!(check_headers(&[pair("Accept", &"a".repeat(512))]).is_ok());
    // twice, whatever the letter case
    assert!(check_headers(&[pair("Accept", "a"), pair("accept", "b")]).is_err());
    // too many
    let many: Vec<_> = (0..17).map(|_| pair("Accept", "a")).collect();
    assert!(check_headers(&many).is_err());
    // the message does not repeat the value
    let e = check_headers(&[pair("X-Api-Key", "secret\r\nvalue")]).unwrap_err();
    assert!(!e.contains("secret"), "{e}");
}

#[test]
fn the_parts_of_a_request_do_not_print_a_key_or_a_body() {
    let parts = RequestParts::new()
        .with_header("X-Api-Key", "super-secret-key")
        .with_bytes(b"password=hunter2".to_vec())
        .if_match("tag1");
    let shown = format!("{parts:?}");
    assert!(!shown.contains("super-secret-key"), "{shown}");
    assert!(!shown.contains("hunter2"), "{shown}");
    assert!(shown.contains("X-Api-Key"));
    assert_eq!(parts.bytes().unwrap().len(), 16);
    assert!(parts.text().is_none());
    // text replaces bytes and the other way round
    let t = parts.clone().with_text("{}");
    assert!(t.bytes().is_none() && t.text() == Some("{}"));
    let b = t.with_bytes(vec![1]);
    assert!(b.text().is_none() && b.bytes() == Some(&[1u8][..]));
    // a bad tag makes the parts invalid, it is not sent as it is
    assert!(RequestParts::new().if_match("a b").validate().is_err());
    assert!(RequestParts::new().if_match("ok").validate().is_ok());
}

#[test]
fn a_write_of_new_bytes_a_read_and_a_write_from_the_tag_work_and_keep_every_byte() {
    let sim = SimServer::new();
    let mut c = admin(&sim);
    let all: Vec<u8> = (0..=255u8)
        .chain(b"\r\n\r\nHTTP/1.1 200 OK\r\n".iter().copied())
        .collect();
    let first = c
        .put_if_match(&file_path(), None, &all, Some("application/octet-stream"))
        .unwrap();
    assert_eq!(first.status, 201);
    assert!(!first.unchanged);
    assert_eq!(first.version, Some(1));
    let tag1 = first.etag.clone().unwrap();
    let read = c.get_with_etag(&file_path()).unwrap();
    assert_eq!(read.bytes, all);
    assert_eq!(read.etag.as_deref(), Some(tag1.as_str()));
    assert_eq!(
        read.content_type.as_deref(),
        Some("application/octet-stream")
    );
    let second = c
        .put_if_match(&file_path(), Some(&tag1), b"second", None)
        .unwrap();
    assert_eq!(second.status, 201);
    assert_ne!(second.etag, first.etag);
    assert_eq!(second.version, Some(2));
    assert_eq!(
        sim.file("phoenix-cad", "p-1", "layers/top.gbr").unwrap(),
        b"second"
    );
    assert_eq!(sim.file_versions("phoenix-cad", "p-1", "layers/top.gbr"), 2);
}

#[test]
fn the_same_bytes_make_no_new_version() {
    let sim = SimServer::new();
    let mut c = admin(&sim);
    let w = c.put_if_match(&file_path(), None, b"same", None).unwrap();
    let again = c
        .put_if_match(&file_path(), w.etag.as_deref(), b"same", None)
        .unwrap();
    assert_eq!(again.status, 200);
    assert!(again.unchanged);
    assert_eq!(again.etag, w.etag);
    assert_eq!(sim.file_versions("phoenix-cad", "p-1", "layers/top.gbr"), 1);
}

#[test]
fn a_write_without_a_tag_over_a_file_that_exists_is_428_with_its_text_in_eight_languages() {
    let sim = SimServer::new();
    let mut c = admin(&sim);
    c.put_if_match(&file_path(), None, b"v1", None).unwrap();
    let e = c.put_if_match(&file_path(), None, b"v2", None).unwrap_err();
    assert!(
        matches!(e, StoreError::PreconditionRequired { .. }),
        "{e:?}"
    );
    assert_eq!(e.code(), Some("precondition_required"));
    assert!(!e.is_conflict());
    let mut seen = Vec::new();
    for lang in Lang::ALL {
        let t = e.text(lang);
        assert!(!t.trim().is_empty());
        seen.push(t);
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 8, "eight different texts");
    assert_eq!(e.to_string(), e.text(Lang::En));
    // nothing was written
    assert_eq!(
        sim.file("phoenix-cad", "p-1", "layers/top.gbr").unwrap(),
        b"v1"
    );
}

#[test]
fn a_write_from_an_old_tag_is_412_and_reading_again_gives_the_new_tag() {
    let sim = SimServer::new();
    let mut c = admin(&sim);
    let w = c.put_if_match(&file_path(), None, b"v1", None).unwrap();
    let old = w.etag.unwrap();
    let theirs = sim.write_file("phoenix-cad", "p-1", "layers/top.gbr", b"theirs");
    let e = c
        .put_if_match(&file_path(), Some(&old), b"mine", None)
        .unwrap_err();
    assert!(e.is_conflict(), "{e:?}");
    assert_eq!(e.code(), Some("precondition_failed"));
    for lang in Lang::ALL {
        assert!(!e.text(lang).trim().is_empty());
    }
    assert_eq!(
        sim.file("phoenix-cad", "p-1", "layers/top.gbr").unwrap(),
        b"theirs"
    );
    let read = c.get_with_etag(&file_path()).unwrap();
    assert_eq!(read.etag.as_deref(), Some(theirs.as_str()));
    let ok = c
        .put_if_match(&file_path(), read.etag.as_deref(), b"merged", None)
        .unwrap();
    assert_eq!(ok.status, 201);
}

#[test]
fn the_rule_can_be_turned_off_like_on_the_server() {
    let sim = SimServer::new();
    sim.set_require_if_match(false);
    let mut c = admin(&sim);
    c.put_if_match(&file_path(), None, b"v1", None).unwrap();
    let w = c.put_if_match(&file_path(), None, b"v2", None).unwrap();
    assert_eq!(w.status, 201);
}

#[test]
fn a_viewer_cannot_write_and_other_failures_stay_as_they_are() {
    let sim = SimServer::new();
    let mut a = admin(&sim);
    a.put_if_match(&file_path(), None, b"v1", None).unwrap();
    let mut v = client_as(&sim, "viewer", "viewer-password");
    assert_eq!(v.get_with_etag(&file_path()).unwrap().bytes, b"v1");
    let e = v.put_if_match(&file_path(), None, b"x", None).unwrap_err();
    assert!(
        matches!(e, StoreError::Other(ClientError::Forbidden { .. })),
        "{e:?}"
    );
    // a file that is not there
    let missing = project_file_path("phoenix-cad", "p-1", "nope.txt").unwrap();
    let e = a.get_with_etag(&missing).unwrap_err();
    assert_eq!(e.code(), Some("unknown_file"));
    assert!(matches!(e, StoreError::Other(_)));
    // no login at all
    let mut anon = Client::new(settings(), Box::new(sim.clone()));
    let e = anon.get_with_etag(&file_path()).unwrap_err();
    assert!(matches!(
        e,
        StoreError::Other(ClientError::Unauthorized { .. })
    ));
}

#[test]
fn removing_a_file_needs_its_tag_and_keeps_the_versions() {
    let sim = SimServer::new();
    let mut c = admin(&sim);
    let w = c.put_if_match(&file_path(), None, b"v1", None).unwrap();
    let e = c.delete_if_match(&file_path(), "stale-tag").unwrap_err();
    assert!(e.is_conflict());
    c.delete_if_match(&file_path(), w.etag.as_deref().unwrap())
        .unwrap();
    assert!(sim.file("phoenix-cad", "p-1", "layers/top.gbr").is_none());
    assert_eq!(sim.file_versions("phoenix-cad", "p-1", "layers/top.gbr"), 1);
    // writing it again brings it back as a new file, with the next version
    let back = c.put_if_match(&file_path(), None, b"v2", None).unwrap();
    assert_eq!(back.version, Some(2));
    // an older version is read without a tag
    let old = format!("{}&version=1", file_path());
    let r = c.get_with_etag(&old).unwrap();
    assert_eq!(r.bytes, b"v1");
    assert!(r.etag.is_none());
}

#[test]
fn a_write_is_sent_once_even_if_the_answer_never_comes() {
    let sim = SimServer::new();
    let mut c = admin(&sim);
    c.put_if_match(&file_path(), None, b"v1", None).unwrap();
    let before = sim.request_count();
    sim.script(vec![Fault::Timeout, Fault::Timeout, Fault::Timeout]);
    let e = c
        .put_if_match(&file_path(), Some("whatever"), b"v2", None)
        .unwrap_err();
    match e {
        StoreError::Other(ClientError::Unreachable { attempts, .. }) => assert_eq!(attempts, 1),
        other => panic!("{other:?}"),
    }
    assert!(sim.pauses().is_empty());
    // a time-out is not a request that reached the server in the double, so the count did not move
    assert_eq!(sim.request_count(), before);
}

#[test]
fn a_request_with_bad_headers_is_refused_before_anything_is_sent() {
    let sim = SimServer::new();
    let mut c = admin(&sim);
    let before = sim.request_count();
    let bad = RequestParts::new().with_header("If-Match", "a\r\nX-Evil: 1");
    let e = c.request_full("PUT", &file_path(), &bad).unwrap_err();
    assert!(matches!(e, ClientError::Config(_)), "{e:?}");
    assert_eq!(sim.request_count(), before);
    // the 428 and 412 are also recognised on a plain ClientError
    let shown = ClientError::Server {
        status: 428,
        code: None,
        detail: String::new(),
    };
    assert!(shown.is_precondition_required() && !shown.is_precondition_failed());
    let other = ClientError::Server {
        status: 412,
        code: Some("precondition_failed".to_string()),
        detail: String::new(),
    };
    assert!(other.is_precondition_failed());
    assert!(StoreError::from(other).is_conflict());
    assert!(matches!(
        StoreError::from(ClientError::Config("x".to_string())),
        StoreError::Other(_)
    ));
}

/// A transport that keeps the last request it was given and answers `200 {}`.
struct Keep(std::sync::Arc<std::sync::Mutex<Option<Request>>>);

impl Transport for Keep {
    fn send(&self, r: &Request) -> Result<Reply, TransportError> {
        *self.0.lock().unwrap() = Some(r.clone());
        Ok(Reply::new(200, "{}"))
    }
}

/// The request the client builds for these parts, with a token.
fn caught(method: &str, parts: &RequestParts) -> Request {
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut s = settings();
    s.set("server_token", "t").unwrap();
    let mut c = Client::new(s, Box::new(Keep(slot.clone())));
    c.request_full(method, "/api/v1/x", parts).unwrap();
    let got = slot.lock().unwrap().clone().unwrap();
    got
}

#[test]
fn the_client_builds_the_request_with_the_headers_the_bytes_and_one_try() {
    let got = caught(
        "PUT",
        &RequestParts::new()
            .with_bytes(vec![1, 2, 3])
            .if_match("tag")
            .single_try(),
    );
    assert_eq!(got.payload(), vec![1, 2, 3]);
    assert!(got.body.is_none() && got.has_payload());
    assert_eq!(got.header("IF-MATCH"), Some("\"tag\""));
    assert!(got.single_try);
    assert_eq!(got.bearer(), Some("t"));
    // the token and the bytes are not in what is printed
    let shown = format!("{got:?}");
    assert!(shown.contains("***") && !shown.contains("\"t\""), "{shown}");
}

#[test]
fn a_transport_written_before_the_headers_still_works() {
    struct Old;
    impl Transport for Old {
        fn send(&self, _r: &Request) -> Result<Reply, TransportError> {
            Ok(Reply {
                status: 200,
                body: "{\"ok\":true}".to_string(),
            })
        }
    }
    let request = caught("GET", &RequestParts::new());
    let r = Old.send_full(&request).unwrap();
    assert_eq!(r.status, 200);
    assert!(r.headers.is_empty());
    assert_eq!(r.text(), "{\"ok\":true}");
    assert_eq!(r.etag(), None);
}

/// Serves one connection: reads the head and the body that `Content-Length` announces, gives back what arrived and answers with `answer`.
fn serve_once(answer: Vec<u8>) -> (String, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut got = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = stream.read(&mut buf).unwrap_or(0);
            got.extend_from_slice(&buf[..n]);
            if n == 0 {
                break;
            }
            if let Some(end) = got.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&got[..end]).to_ascii_lowercase();
                let want = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if got.len() >= end + 4 + want {
                    break;
                }
            }
        }
        let _ = stream.write_all(&answer);
        got
    });
    (address, handle)
}

fn real_client(address: &str) -> Client {
    let mut s = Settings::default();
    s.set("server_address", address).unwrap();
    s.set("server_token", "tok-1").unwrap();
    s.set("server_timeout_s", "5").unwrap();
    s.set("server_retries", "0").unwrap();
    Client::connect(s)
}

#[test]
fn over_a_real_socket_the_bytes_the_if_match_and_the_etag_travel() {
    let answer = b"HTTP/1.1 201 Created\r\nETag: \"new-tag\"\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"unchanged\":false,\"version\":3}".to_vec();
    let (address, handle) = serve_once(answer);
    let mut c = real_client(&address);
    let body: Vec<u8> = (0..=255u8).collect();
    let w = c
        .put_if_match(&file_path(), Some("old-tag"), &body, None)
        .unwrap();
    assert_eq!(w.status, 201);
    assert_eq!(w.etag.as_deref(), Some("new-tag"));
    assert_eq!(w.version, Some(3));
    let got = handle.join().unwrap();
    let end = got.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&got[..end]).to_string();
    assert!(head.starts_with("PUT /api/v1/apps/phoenix-cad/projects/p-1/files/content?name=layers%2Ftop.gbr HTTP/1.1\r\n"));
    assert!(head.contains("\r\nIf-Match: \"old-tag\""));
    assert!(head.contains("\r\nContent-Type: application/octet-stream"));
    assert!(head.contains("\r\nContent-Length: 256"));
    assert!(head.contains("\r\nAuthorization: Bearer tok-1"));
    assert_eq!(head.matches("Content-Type").count(), 1);
    assert_eq!(&got[end + 4..], &body[..]);
}

#[test]
fn over_a_real_socket_a_428_and_a_412_are_the_errors_of_their_own() {
    for (status_line, code, conflict) in [
        ("428 Precondition Required", "precondition_required", false),
        ("412 Precondition Failed", "precondition_failed", true),
    ] {
        let answer = format!(
            "HTTP/1.1 {status_line}\r\nConnection: close\r\n\r\n{{\"error\":\"{code}\",\"detail\":\"d\"}}"
        )
        .into_bytes();
        let (address, handle) = serve_once(answer);
        let mut c = real_client(&address);
        let e = c
            .put_if_match(&file_path(), Some("t"), b"x", None)
            .unwrap_err();
        assert_eq!(e.is_conflict(), conflict, "{e:?}");
        assert_eq!(e.code(), Some(code));
        handle.join().unwrap();
    }
}

#[test]
fn over_a_real_socket_a_read_gives_the_headers_and_a_chunked_body_is_put_together() {
    let answer = b"HTTP/1.1 200 OK\r\nEtag: W/\"abc\"\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n".to_vec();
    let (address, handle) = serve_once(answer);
    let mut c = real_client(&address);
    let r = c.get_with_etag(&file_path()).unwrap();
    assert_eq!(r.bytes, b"abcde");
    assert_eq!(r.etag.as_deref(), Some("abc"));
    let got = String::from_utf8_lossy(&handle.join().unwrap()).to_string();
    // the caller's Accept replaces the default one, it is not written twice
    assert_eq!(got.matches("Accept:").count(), 1, "{got}");
    assert!(got.contains("Accept: */*\r\n"));
}

#[test]
fn the_transport_refuses_a_header_that_could_split_the_request() {
    let t = TcpTransport::new("127.0.0.1:1");
    let mut request = caught("GET", &RequestParts::new().accept("*/*"));
    // port 1 has nobody: the request itself was fine, the connection is what fails
    assert!(matches!(t.send(&request), Err(TransportError::Connect(m)) if !m.contains("header")));
    request.headers.push(pair("Content-Length", "0"));
    match t.send(&request) {
        Err(TransportError::Connect(m)) => assert!(m.contains("not one a program may add"), "{m}"),
        other => panic!("{other:?}"),
    }
    request.headers.pop();
    request.headers.push(pair("Accept-Language", "es\r\nX: 1"));
    assert!(t.send_full(&request).is_err());
    // a request with a text body and a body of bytes is refused too
    request.headers.pop();
    request.body = Some("{}".to_string());
    request.body_bytes = Some(vec![1]);
    assert!(t.send(&request).is_err());
}

#[test]
fn a_full_reply_reads_its_headers_without_caring_for_the_letter_case() {
    let r = FullReply {
        status: 200,
        headers: vec![pair("eTaG", "W/\"q\""), pair("X-Other", "1")],
        body: b"\xff\xfe".to_vec(),
    };
    assert_eq!(r.etag().as_deref(), Some("q"));
    assert_eq!(r.header("x-other"), Some("1"));
    assert_eq!(r.header("missing"), None);
    // the text of bytes that are not UTF-8 does not fail
    assert_eq!(r.text().chars().count(), 2);
    assert_eq!(r.clone().into_reply().status, 200);
    assert_eq!(FullReply::from(Reply::new(404, "no")).body, b"no");
}
