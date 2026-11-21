//! The HTTP transport of the Klipper controller against a fake Moonraker in the test itself: a thread with a `TcpListener` on `127.0.0.1`
//! with a port the system picks, that answers the way Moonraker does and, when told, the way a broken or hostile server would: answers cut
//! into pieces by a fixed seed, slow, chunked, 4xx and 5xx, redirections, a body of any size, JSON that is not JSON, a connection that
//! closes at once, headers made to hurt. No test touches a real network or a real printer; nothing here has run against a real Moonraker.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eh3d_controller::dialtext;
use eh3d_controller::httprpc::{
    self, http_error_code, klipper_controller, HttpConfig, HttpJsonRpc, E_HTTP_AUTH, E_HTTP_BUSY,
    E_HTTP_CLOSED, E_HTTP_HOST, E_HTTP_NOT_JSON, E_HTTP_PROTOCOL, E_HTTP_REDIRECT, E_HTTP_STATUS,
    E_HTTP_TIMEOUT, E_HTTP_TOO_BIG, E_HTTP_UNREACHABLE, HTTP_KINDS, SETTINGS,
};
use eh3d_controller::klipper::{JsonRpcTransport, E_HOME_FIRST};
use eh3d_controller::urgent::Urgent;
use eh3d_controller::{Command, Controller, Error, Family, State};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------------------------------------------------------------
// the fake Moonraker
// ---------------------------------------------------------------------------------------------------------------------------------

/// A request the fake server received.
#[derive(Clone, Debug)]
struct Seen {
    /// The number of the connection, in the order they were accepted.
    n: usize,
    head: String,
    body: String,
}

impl Seen {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
    fn method(&self) -> String {
        self.json()["method"].as_str().unwrap_or("").to_string()
    }
    fn id(&self) -> Value {
        self.json()["id"].clone()
    }
}

/// What the fake server does with a request.
enum Act {
    /// Writes the bytes and closes.
    Raw(Vec<u8>),
    /// Writes the bytes in pieces of 1 to 7 bytes by a seed, and closes.
    Pieces(Vec<u8>, u64),
    /// Closes without a byte.
    Close,
    /// Waits until the flag is raised (at most ten seconds), then does the other thing.
    Hold(Arc<AtomicBool>, Box<Act>),
    /// Writes the bytes one by one with a pause between them.
    Drip(Vec<u8>, u64),
    /// Writes the bytes and goes on with the other thing, keeping the connection open.
    Then(Vec<u8>, Box<Act>),
    /// Writes the head and then bytes without end until the client closes (the flag says that it did).
    Endless(Vec<u8>, Arc<AtomicBool>),
    /// Reads until the client closes the connection (the flag says that it did).
    WatchClose(Arc<AtomicBool>),
}

fn run(act: Act, s: &mut TcpStream) {
    match act {
        Act::Raw(b) => {
            let _ = s.write_all(&b);
            let _ = s.flush();
        }
        Act::Pieces(b, seed) => {
            let mut rng = seed;
            let mut i = 0;
            while i < b.len() {
                let n = (1 + next(&mut rng) % 7) as usize;
                let end = (i + n).min(b.len());
                if s.write_all(&b[i..end]).is_err() {
                    return;
                }
                let _ = s.flush();
                i = end;
                if next(&mut rng) & 3 == 0 {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
        Act::Close => {}
        Act::Hold(flag, a) => {
            let until = Instant::now() + Duration::from_secs(10);
            while !flag.load(Ordering::SeqCst) && Instant::now() < until {
                thread::sleep(Duration::from_millis(5));
            }
            run(*a, s);
        }
        Act::Then(b, a) => {
            if s.write_all(&b).is_ok() {
                let _ = s.flush();
                run(*a, s);
            }
        }
        Act::Drip(b, ms) => {
            for byte in b {
                if s.write_all(&[byte]).is_err() {
                    return;
                }
                let _ = s.flush();
                thread::sleep(Duration::from_millis(ms));
            }
        }
        Act::Endless(head, closed) => {
            if s.write_all(&head).is_err() {
                closed.store(true, Ordering::SeqCst);
                return;
            }
            let block = [b'a'; 1000];
            for _ in 0..200_000 {
                if s.write_all(&block).is_err() {
                    closed.store(true, Ordering::SeqCst);
                    return;
                }
            }
        }
        Act::WatchClose(closed) => {
            let _ = s.set_read_timeout(Some(Duration::from_millis(50)));
            let until = Instant::now() + Duration::from_secs(5);
            let mut b = [0u8; 64];
            while Instant::now() < until {
                match s.read(&mut b) {
                    Ok(0) => {
                        closed.store(true, Ordering::SeqCst);
                        return;
                    }
                    Ok(_) => {}
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(_) => {
                        closed.store(true, Ordering::SeqCst);
                        return;
                    }
                }
            }
        }
    }
}

/// The generator of the pieces: the same seed, the same pieces on every computer.
fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn serve(
    mut s: TcpStream,
    n: usize,
    handler: &(dyn Fn(&Seen) -> Act + Send + Sync),
    seen: &Mutex<Vec<Seen>>,
) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let mut data: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(k) => data.extend_from_slice(&chunk[..k]),
        }
        if data.len() > 1 << 20 {
            return;
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).to_string();
    let length = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    while data.len() < head_end + length {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(k) => data.extend_from_slice(&chunk[..k]),
        }
    }
    let body = String::from_utf8_lossy(&data[head_end..head_end + length]).to_string();
    let request = Seen { n, head, body };
    seen.lock().unwrap().push(request.clone());
    let act = handler(&request);
    run(act, &mut s);
}

struct Fake {
    addr: SocketAddr,
    seen: Arc<Mutex<Vec<Seen>>>,
    done: Arc<AtomicBool>,
}

impl Fake {
    fn start(handler: impl Fn(&Seen) -> Act + Send + Sync + 'static) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let done = Arc::new(AtomicBool::new(false));
        let handler: Arc<dyn Fn(&Seen) -> Act + Send + Sync> = Arc::new(handler);
        let (seen2, done2) = (Arc::clone(&seen), Arc::clone(&done));
        thread::spawn(move || {
            let mut n = 0;
            for conn in listener.incoming() {
                if done2.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                let (h, sn, idx) = (Arc::clone(&handler), Arc::clone(&seen2), n);
                n += 1;
                thread::spawn(move || serve(stream, idx, &*h, &sn));
            }
        });
        Fake { addr, seen, done }
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }

    fn methods(&self) -> Vec<String> {
        self.seen().iter().map(Seen::method).collect()
    }

    /// Waits (at most three seconds) until a request of this method has arrived.
    fn wait_for(&self, method: &str) -> bool {
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until {
            if self.methods().iter().any(|m| m == method) {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
    }
}

// the answers

fn http(status: &str, headers: &[&str], body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status}\r\n").into_bytes();
    for h in headers {
        out.extend_from_slice(h.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    if !headers
        .iter()
        .any(|h| h.to_ascii_lowercase().starts_with("content-length"))
    {
        out.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    out.extend_from_slice(b"Connection: close\r\n\r\n");
    out.extend_from_slice(body);
    out
}

fn ok(body: &str) -> Vec<u8> {
    http(
        "200 OK",
        &["Content-Type: application/json"],
        body.as_bytes(),
    )
}

fn result(id: &Value, r: Value) -> String {
    json!({"jsonrpc": "2.0", "result": r, "id": id}).to_string()
}

/// The body in chunks of 1 to 9 bytes by a seed, one of them with an extension, and a trailer.
fn chunked(body: &str, seed: u64) -> Vec<u8> {
    let mut rng = seed;
    let mut out = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut first = true;
    while i < bytes.len() {
        let n = ((1 + next(&mut rng) % 9) as usize).min(bytes.len() - i);
        let ext = if first { ";name=value" } else { "" };
        first = false;
        out.extend_from_slice(format!("{n:X}{ext}\r\n").as_bytes());
        out.extend_from_slice(&bytes[i..i + n]);
        out.extend_from_slice(b"\r\n");
        i += n;
    }
    out.extend_from_slice(b"0\r\nX-Trailer: 1\r\n\r\n");
    out
}

/// A Moonraker that answers the methods the controller uses; `stopped` says that Klipper was shut down by the emergency stop.
fn moonraker(stopped: Arc<AtomicBool>) -> impl Fn(&Seen) -> Act + Send + Sync + 'static {
    move |s| {
        let id = s.id();
        let state = if stopped.load(Ordering::SeqCst) {
            "shutdown"
        } else {
            "ready"
        };
        let text = match s.method().as_str() {
            "printer.info" => result(&id, json!({"state": state})),
            "printer.objects.query" => {
                if s.json()["params"]["objects"].get("configfile").is_some() {
                    result(
                        &id,
                        json!({"status": {"configfile": {"settings": {
                            "stepper_x": {"position_max": 200.0},
                            "stepper_y": {"position_max": 200.0},
                            "stepper_z": {"position_max": 50.0}}}}}),
                    )
                } else {
                    result(
                        &id,
                        json!({"status": {
                            "webhooks": {"state": state, "state_message": ""},
                            "toolhead": {"position": [10.0, 20.0, 5.0, 0.0], "homed_axes": "xyz"},
                            "gcode_move": {"gcode_position": [10.0, 20.0, 5.0, 0.0], "speed": 0.0},
                            "idle_timeout": {"state": "Ready"},
                            "pause_resume": {"is_paused": false}}}),
                    )
                }
            }
            "printer.gcode.script" => result(&id, json!("ok")),
            "printer.emergency_stop" => {
                stopped.store(true, Ordering::SeqCst);
                result(&id, json!("ok"))
            }
            "printer.firmware_restart" => {
                stopped.store(false, Ordering::SeqCst);
                result(&id, json!("ok"))
            }
            _ => json!({"jsonrpc": "2.0", "error": {"code": -32601, "message": "Method not found"}, "id": id})
                .to_string(),
        };
        Act::Raw(ok(&text))
    }
}

/// The settings of a test: short times, a place for the fake server, no wait for a machine that is busy.
fn cfg(f: &Fake) -> HttpConfig {
    let mut c = HttpConfig::new("127.0.0.1", f.addr.port());
    for (k, v) in [
        ("klipper.connect_timeout_ms", 1000),
        ("klipper.read_timeout_ms", 500),
        ("klipper.answer_timeout_ms", 2000),
        ("klipper.response_wait_ms", 5000),
        ("klipper.line_timeout_ms", 2000),
        ("klipper.homing_timeout_ms", 1000),
        ("klipper.jog_timeout_ms", 1000),
        ("klipper.busy_limit_s", 0),
    ] {
        c.set_number(k, v).unwrap();
    }
    c
}

const INFO: &str = r#"{"jsonrpc":"2.0","method":"printer.info","params":{},"id":7}"#;

/// Sends the liveness question and waits for what comes back.
fn ask(t: &mut HttpJsonRpc) -> std::io::Result<Option<String>> {
    t.send(INFO)?;
    t.recv(Duration::from_secs(5))
}

fn transport(f: &Fake) -> HttpJsonRpc {
    HttpJsonRpc::new(cfg(f)).unwrap()
}

fn code_of(r: std::io::Result<Option<String>>) -> Option<u32> {
    r.err().and_then(|e| http_error_code(&e))
}

// ---------------------------------------------------------------------------------------------------------------------------------
// the seven tests the decision lists, then the rest
// ---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_request_is_answered_and_a_request_with_no_answer_in_the_time_gives_none_not_an_error() {
    let release = Arc::new(AtomicBool::new(false));
    let r2 = Arc::clone(&release);
    let f = Fake::start(move |s| {
        if s.n == 0 {
            Act::Raw(ok(&result(&s.id(), json!({"state": "ready"}))))
        } else {
            Act::Hold(
                Arc::clone(&r2),
                Box::new(Act::Raw(ok(&result(&s.id(), json!({}))))),
            )
        }
    });
    let mut t = transport(&f);
    let answer = ask(&mut t).unwrap().unwrap();
    assert!(answer.contains(r#""state":"ready""#));
    // the request as it went out: one POST on the one route, the body as the controller wrote it, the connection closed after it
    let seen = f.seen();
    assert!(seen[0]
        .head
        .starts_with("POST /server/jsonrpc HTTP/1.1\r\n"));
    assert!(seen[0].head.contains("Connection: close\r\n"));
    assert!(seen[0].head.contains("Content-Type: application/json\r\n"));
    assert!(seen[0]
        .head
        .contains(&format!("Host: 127.0.0.1:{}\r\n", f.addr.port())));
    assert_eq!(seen[0].body, INFO);
    assert!(!seen[0].head.to_ascii_lowercase().contains("x-api-key"));
    // the second one is held: nothing comes in the time given, and that is not an error
    t.send(INFO).unwrap();
    assert_eq!(t.recv(Duration::from_millis(150)).unwrap(), None);
    release.store(true, Ordering::SeqCst);
    assert!(t.recv(Duration::from_secs(3)).unwrap().is_some());
}

#[test]
fn the_answer_to_a_probe_arrives_while_the_answer_to_the_line_is_missing_on_a_second_connection() {
    let release = Arc::new(AtomicBool::new(false));
    let r2 = Arc::clone(&release);
    let f = Fake::start(move |s| {
        let reply = Act::Raw(ok(&result(&s.id(), json!("ok"))));
        if s.method() == "printer.gcode.script" {
            Act::Hold(Arc::clone(&r2), Box::new(reply))
        } else {
            reply
        }
    });
    let mut c = {
        let mut conf = cfg(&f);
        conf.set_number("klipper.busy_limit_s", 4).unwrap();
        conf.set_number("klipper.line_timeout_ms", 300).unwrap();
        klipper_controller(conf).unwrap()
    };
    let r3 = Arc::clone(&release);
    let opener = thread::spawn(move || {
        thread::sleep(Duration::from_millis(900));
        r3.store(true, Ordering::SeqCst);
    });
    // the line waits 300 ms, asks whether the machine is alive, gets the answer on another connection and goes on waiting
    c.send_line("G1 X5 F600").unwrap();
    opener.join().unwrap();
    assert!(c.probes_sent() >= 1, "no probe was sent");
    let seen = f.seen();
    let script = seen
        .iter()
        .position(|s| s.method() == "printer.gcode.script")
        .unwrap();
    let probe = seen
        .iter()
        .position(|s| s.method() == "printer.info")
        .unwrap();
    assert!(script < probe);
    assert_ne!(seen[script].n, seen[probe].n, "the probe shared the line");
    assert!(!c.halted());
}

#[test]
fn a_closed_connection_is_a_link_error_with_its_number_and_a_move_that_lost_its_link_is_stopped() {
    let f = Fake::start(|s| match s.method().as_str() {
        "printer.emergency_stop" => Act::Raw(ok(&result(&s.id(), json!("ok")))),
        _ => Act::Close,
    });
    let mut c = klipper_controller(cfg(&f)).unwrap();
    // a question: the connection closes at once
    let e = c.status().unwrap_err();
    assert!(matches!(e, Error::Link(_)), "{e:?}");
    assert_eq!(e.code(), Some(E_HTTP_CLOSED));
    assert!(
        !c.halted(),
        "a question that failed does not shut the machine down"
    );
    // the words of the number, in each language
    for lang in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
        let text = dialtext::explain(Family::Klipper, "error", e.code(), "raw", lang);
        assert_ne!(text, "raw");
    }
    // a line that moves: nobody knows whether the machine still moves, so it is stopped
    let e = c.send_line("G1 X5 F600").unwrap_err();
    assert_eq!(e.code(), Some(E_HTTP_CLOSED));
    assert!(c.halted());
    assert!(
        f.wait_for("printer.emergency_stop"),
        "the stop never went out"
    );
    // every number of this transport is read from a link error, and no other link text is taken for one
    assert_eq!(Error::Link("error 219: x".into()).code(), Some(219));
    assert_eq!(Error::Link("error 5: x".into()).code(), None);
    assert_eq!(Error::Link("error 2x9: x".into()).code(), None);
    assert_eq!(Error::Link("broken pipe".into()).code(), None);
    assert_eq!(Error::Link("error 218: x".into()).official_text(), None);
}

#[test]
fn a_connection_that_closes_during_the_firmware_restart_is_reopened_and_the_next_question_is_answered(
) {
    let cut = Arc::new(AtomicBool::new(false));
    let c2 = Arc::clone(&cut);
    let f = Fake::start(move |s| {
        if s.method() == "printer.firmware_restart" && !c2.swap(true, Ordering::SeqCst) {
            Act::Close
        } else {
            Act::Raw(ok(&result(&s.id(), json!({"state": "ready"}))))
        }
    });
    let mut t = transport(&f);
    t.send(r#"{"jsonrpc":"2.0","method":"printer.firmware_restart","params":{},"id":1}"#)
        .unwrap();
    let e = t.recv(Duration::from_secs(3)).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_CLOSED));
    // Klipper is up again: the next question opens a new connection and is answered
    assert!(ask(&mut t).unwrap().unwrap().contains("ready"));
    // and the restart asked again is answered
    t.send(r#"{"jsonrpc":"2.0","method":"printer.firmware_restart","params":{},"id":2}"#)
        .unwrap();
    assert!(t.recv(Duration::from_secs(3)).unwrap().is_some());
}

#[test]
fn the_credential_is_sent_in_the_header_and_never_appears_in_an_error_a_debug_text_or_a_setting_error(
) {
    // the value is made at run time and has no name that a scanner takes for a stored secret
    let sample_key: String = ["K", "7", "q", "Z", "x", "9", "Pw", "3", "Lm", "2"].concat();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("401", http("401 Unauthorized", &[], b"{}")),
        (
            "500",
            http("500 Internal Server Error", &[], sample_key.as_bytes()),
        ),
        (
            "redirect",
            http(
                "302 Found",
                &[&format!("Location: http://127.0.0.1:1/{sample_key}")],
                b"",
            ),
        ),
        ("html", ok(&format!("<html>{sample_key}</html>"))),
        ("garbage", format!("{sample_key}\r\n\r\n").into_bytes()),
        (
            "bad length",
            http("200 OK", &[&format!("Content-Length: {sample_key}")], b"{}"),
        ),
        (
            "cut",
            b"HTTP/1.1 200 OK\r\nContent-Length: 50\r\n\r\n{".to_vec(),
        ),
    ];
    let n = cases.len();
    let table = Arc::new(cases);
    let t2 = Arc::clone(&table);
    let f = Fake::start(move |s| Act::Raw(t2[s.n].1.clone()));
    let conf = cfg(&f).with_api_key(&sample_key).unwrap();
    assert!(conf.has_api_key());
    let shown = format!("{conf:?}");
    assert!(!shown.contains(&sample_key) && shown.contains("api_key"));
    let mut t = HttpJsonRpc::new(conf.clone()).unwrap();
    assert!(!format!("{t:?}").contains(&sample_key));
    let mut texts = Vec::new();
    for (name, _) in table.iter() {
        let e = ask(&mut t).unwrap_err();
        texts.push((name, format!("{e} | {e:?}")));
    }
    assert_eq!(texts.len(), n);
    for (name, text) in &texts {
        assert!(!text.contains(&sample_key), "{name}: {text}");
    }
    // the header went out exactly once on every request, with the value the program was given and nothing else on its line
    let seen = f.seen();
    assert_eq!(seen.len(), n);
    for s in &seen {
        let lines: Vec<&str> = s
            .head
            .lines()
            .filter(|l| l.to_ascii_lowercase().starts_with("x-api-key"))
            .collect();
        assert_eq!(lines, vec![format!("X-Api-Key: {sample_key}").as_str()]);
    }
    // through the controller the error is a link error that does not hold the value either
    let f2 = Fake::start(|_| Act::Raw(http("401 Unauthorized", &[], b"{}")));
    let mut c = klipper_controller(cfg(&f2).with_api_key(&sample_key).unwrap()).unwrap();
    let e = c.status().unwrap_err();
    assert_eq!(e.code(), Some(E_HTTP_AUTH));
    assert!(!format!("{e} {e:?} {c:?}", c = c.transport()).contains(&sample_key));
    // a key written on a wrong line of the configuration file is not repeated by the error
    let e = HttpConfig::from_cfg(&format!("klipper.api_key = {sample_key} extra")).unwrap_err();
    assert!(!e.to_string().contains(&sample_key), "{e}");
    let e = HttpConfig::from_cfg(&sample_key).unwrap_err();
    assert!(!e.to_string().contains(&sample_key), "{e}");
    let e = HttpConfig::from_cfg(&format!("{sample_key} = 1")).unwrap_err();
    assert!(!e.to_string().contains(&sample_key), "{e}");
    // a key that could split the header is refused
    for bad in [
        "",
        "a b",
        "a\r\nX-Evil: 1",
        "tab\there",
        "é",
        &"k".repeat(257),
    ] {
        assert!(HttpConfig::default().with_api_key(bad).is_err(), "{bad:?}");
    }
    assert!(HttpConfig::default().with_api_key("a-b_C.9").is_ok());
}

#[test]
fn a_message_cut_into_pieces_is_read_whole_with_a_length_with_chunks_and_one_byte_at_a_time() {
    let body = result(
        &json!(7),
        json!({"state": "ready", "text": "x".repeat(300), "n": [1, 2, 3]}),
    );
    let variants: Vec<(&str, Vec<u8>)> = vec![
        ("length", ok(&body)),
        ("chunked", chunked(&body, 11)),
        ("lowercase names", {
            let mut v = format!(
                "HTTP/1.0 200\r\ncontent-length: {}\r\nx-other:  spaced \t\r\n\r\n",
                body.len()
            )
            .into_bytes();
            v.extend_from_slice(body.as_bytes());
            v
        }),
        ("until close", {
            let mut v = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
            v.extend_from_slice(body.as_bytes());
            v
        }),
        ("Chunked in capitals", {
            let mut v = chunked(&body, 3);
            let text = String::from_utf8(v.clone())
                .unwrap()
                .replace("chunked", "Chunked");
            v = text.into_bytes();
            v
        }),
    ];
    let handles: Vec<_> = [1u64, 2, 3, 5, 8, 13]
        .into_iter()
        .map(|seed| {
            let table: Arc<Vec<(&str, Vec<u8>)>> = Arc::new(variants.clone());
            let body = body.clone();
            thread::spawn(move || {
                let t2 = Arc::clone(&table);
                let f = Fake::start(move |s| Act::Pieces(t2[s.n].1.clone(), seed));
                let mut t = transport(&f);
                for (name, _) in table.iter() {
                    let got = ask(&mut t).unwrap_or_else(|e| panic!("seed {seed} {name}: {e}"));
                    assert_eq!(got.as_deref(), Some(body.as_str()), "seed {seed} {name}");
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    // one byte at a time, each inside the gap that is allowed
    let small = r#"{"jsonrpc":"2.0","result":1,"id":7}"#;
    let f = Fake::start(move |_| Act::Drip(ok(small), 1));
    let mut t = transport(&f);
    assert_eq!(ask(&mut t).unwrap().as_deref(), Some(small));
    // the generator of the pieces is the same on every run and differs by seed
    let (mut a, mut b, mut c) = (5u64, 5u64, 6u64);
    let ra: Vec<u64> = (0..16).map(|_| next(&mut a) % 7).collect();
    let rb: Vec<u64> = (0..16).map(|_| next(&mut b) % 7).collect();
    let rc: Vec<u64> = (0..16).map(|_| next(&mut c) % 7).collect();
    assert_eq!(ra, rb);
    assert_ne!(ra, rc);
}

#[test]
fn a_notification_that_is_passed_on_halts_the_controller_and_so_does_a_status_that_says_shutdown() {
    // the notification arrives where an answer was expected (over HTTP the transport only has what a request brings back)
    let f = Fake::start(|_| {
        Act::Raw(ok(
            r#"{"jsonrpc":"2.0","method":"notify_klippy_shutdown","params":[]}"#,
        ))
    });
    let mut c = klipper_controller(cfg(&f)).unwrap();
    assert!(c.status().is_err());
    assert!(c.halted(), "the notification did not halt the controller");
    // the status itself says that Klipper shut down
    let down = Arc::new(AtomicBool::new(true));
    let f = Fake::start(moonraker(Arc::clone(&down)));
    let mut c = klipper_controller(cfg(&f)).unwrap();
    assert_eq!(c.status().unwrap().state, State::Alarm);
    assert!(c.halted());
    assert!(c.send_line("G1 X1 F600").is_err());
    // and the pushed notifications of a WebSocket are not here: this is the part that stays pending
}

#[test]
fn the_emergency_stop_goes_out_while_another_thread_is_blocked_in_a_read() {
    let release = Arc::new(AtomicBool::new(false));
    let r2 = Arc::clone(&release);
    let f = Fake::start(move |s| {
        let reply = Act::Raw(ok(&result(&s.id(), json!("ok"))));
        if s.method() == "printer.gcode.script" {
            Act::Hold(Arc::clone(&r2), Box::new(reply))
        } else {
            reply
        }
    });
    let mut c = {
        let mut conf = cfg(&f);
        conf.set_number("klipper.busy_limit_s", 4).unwrap();
        conf.set_number("klipper.line_timeout_ms", 300).unwrap();
        klipper_controller(conf).unwrap()
    };
    let channel = c.urgent_channel().unwrap();
    let worker = thread::spawn(move || {
        let r = c.send_line("G1 X5 F600");
        (r, c)
    });
    assert!(f.wait_for("printer.gcode.script"));
    // the thread of the line is blocked in a read; the stop is asked for from this one
    channel.send(Urgent::Hold);
    assert!(
        f.wait_for("printer.emergency_stop"),
        "the stop did not go out"
    );
    assert!(
        !release.load(Ordering::SeqCst),
        "the line was still waiting when the stop arrived"
    );
    let (r, c) = worker.join().unwrap();
    assert!(matches!(r, Err(Error::Refused(_))), "{r:?}");
    assert!(c.halted());
    release.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// the other ways a server can be wrong
// ---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_redirection_is_refused_and_never_followed() {
    let trap = TcpListener::bind("127.0.0.1:0").unwrap();
    trap.set_nonblocking(true).unwrap();
    let trap_port = trap.local_addr().unwrap().port();
    let statuses = [
        "301 Moved Permanently",
        "302 Found",
        "303 See Other",
        "307 Temporary Redirect",
        "308 Permanent Redirect",
    ];
    let f = Fake::start(move |s| {
        Act::Raw(http(
            statuses[s.n % statuses.len()],
            &[&format!(
                "Location: http://127.0.0.1:{trap_port}/printer/info"
            )],
            b"",
        ))
    });
    let mut t = transport(&f);
    for _ in 0..statuses.len() {
        let r = ask(&mut t);
        assert_eq!(code_of(r), Some(E_HTTP_REDIRECT));
    }
    thread::sleep(Duration::from_millis(100));
    assert_eq!(f.seen().len(), statuses.len());
    assert!(trap.accept().is_err(), "the redirection was followed");
    // the text does not hold the address it pointed at
    let e = ask(&mut t).unwrap_err();
    assert!(!e.to_string().contains(&trap_port.to_string()));
}

#[test]
fn a_status_of_error_is_an_error_with_a_text_that_does_not_depend_on_the_body() {
    let statuses = [
        ("400 Bad Request", E_HTTP_STATUS),
        ("404 Not Found", E_HTTP_STATUS),
        ("500 Internal Server Error", E_HTTP_STATUS),
        ("503 Service Unavailable", E_HTTP_STATUS),
        ("401 Unauthorized", E_HTTP_AUTH),
        ("403 Forbidden", E_HTTP_AUTH),
        ("100 Continue", E_HTTP_PROTOCOL),
    ];
    let f = Fake::start(move |s| {
        let body = if s.n % 2 == 0 {
            r#"{"error":{"message":"ignored"}}"#
        } else {
            "<html>ignored</html>"
        };
        Act::Raw(http(
            statuses[s.n / 2 % statuses.len()].0,
            &[],
            body.as_bytes(),
        ))
    });
    let mut t = transport(&f);
    let mut texts = Vec::new();
    for (status, code) in statuses {
        // the same status twice, with a JSON body and with an HTML body
        let first = ask(&mut t).unwrap_err();
        let second = ask(&mut t).unwrap_err();
        assert_eq!(http_error_code(&first), Some(code), "{status}");
        assert_eq!(first.to_string(), second.to_string(), "{status}");
        assert!(!first.to_string().contains("ignored"));
        texts.push(first.to_string());
    }
    assert_eq!(texts[0], "error 209: HTTP status 400");
    assert_eq!(texts[3], "error 209: HTTP status 503");
    // a JSON-RPC error inside a good HTTP answer is the controller's to read: it is a refusal of the line, not a failure of the link
    let f = Fake::start(|s| {
        Act::Raw(ok(&json!({"jsonrpc":"2.0","error":{"code":400,"message":"Must home axis first: 0.000"},"id":s.id()}).to_string()))
    });
    let mut c = klipper_controller(cfg(&f)).unwrap();
    let e = c.send_line("G1 X5 F600").unwrap_err();
    assert!(matches!(e, Error::Refused(_)));
    assert_eq!(e.code(), Some(E_HOME_FIRST));
}

#[test]
fn a_body_of_any_size_is_cut_by_the_limit_without_being_read_to_the_end() {
    let conf_for = |f: &Fake| {
        let mut c = cfg(f);
        c.set_number("klipper.max_response_bytes", 1024).unwrap();
        c
    };
    let cases: Vec<(&str, Vec<u8>)> = vec![
        // the length it announces is already too much: nothing of the body is read
        (
            "announced",
            b"HTTP/1.1 200 OK\r\nContent-Length: 999999999999\r\n\r\n".to_vec(),
        ),
        (
            "announced exactly over",
            b"HTTP/1.1 200 OK\r\nContent-Length: 1025\r\n\r\n".to_vec(),
        ),
        ("sent", http("200 OK", &[], "x".repeat(5000).as_bytes())),
        ("one chunk", {
            let mut v = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7D0\r\n".to_vec();
            v.extend_from_slice(&[b'a'; 2000]);
            v.extend_from_slice(b"\r\n0\r\n\r\n");
            v
        }),
        ("many chunks", {
            let mut v = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
            for _ in 0..40 {
                v.extend_from_slice(b"64\r\n");
                v.extend_from_slice(&[b'a'; 100]);
                v.extend_from_slice(b"\r\n");
            }
            v.extend_from_slice(b"0\r\n\r\n");
            v
        }),
        (
            "size of 2^64-1",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nFFFFFFFFFFFFFFFF\r\n".to_vec(),
        ),
    ];
    let table = Arc::new(cases);
    let t2 = Arc::clone(&table);
    let f = Fake::start(move |s| Act::Raw(t2[s.n].1.clone()));
    let mut t = HttpJsonRpc::new(conf_for(&f)).unwrap();
    for (name, _) in table.iter() {
        let r = ask(&mut t);
        assert_eq!(code_of(r), Some(E_HTTP_TOO_BIG), "{name}");
    }
    // a body of exactly the limit is taken
    let exactly = format!("{{\"a\":\"{}\"}}", "x".repeat(1024 - 8));
    assert_eq!(exactly.len(), 1024);
    let e2 = exactly.clone();
    let f = Fake::start(move |_| Act::Raw(ok(&e2)));
    let mut t = HttpJsonRpc::new(conf_for(&f)).unwrap();
    assert_eq!(ask(&mut t).unwrap().as_deref(), Some(exactly.as_str()));
    // a body without end: the connection is closed on it and the server finds out
    let closed = Arc::new(AtomicBool::new(false));
    let c2 = Arc::clone(&closed);
    let f = Fake::start(move |_| {
        Act::Endless(
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec(),
            Arc::clone(&c2),
        )
    });
    let mut t = HttpJsonRpc::new(conf_for(&f)).unwrap();
    assert_eq!(code_of(ask(&mut t)), Some(E_HTTP_TOO_BIG));
    let until = Instant::now() + Duration::from_secs(3);
    while !closed.load(Ordering::SeqCst) && Instant::now() < until {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        closed.load(Ordering::SeqCst),
        "the connection was not closed"
    );
    // a request that is too large never leaves
    let big = format!(
        r#"{{"jsonrpc":"2.0","method":"printer.gcode.script","params":{{"script":"{}"}},"id":1}}"#,
        "G".repeat(httprpc::MAX_REQUEST_BYTES)
    );
    let e = t.send(&big).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_TOO_BIG));
}

#[test]
fn an_answer_that_is_not_json_is_the_same_error_whatever_it_is() {
    let mut cases: Vec<Vec<u8>> = vec![
        ok("<html><body>502</body></html>"),
        ok(""),
        ok("[1,2]"),
        ok("42"),
        ok("null"),
        ok("\"text\""),
        ok("{\"a\":"),
        ok("{\"a\":1}{\"b\":2}"),
        ok("{'a':1}"),
        http("200 OK", &[], b"\xff\xfe{}"),
        http("200 OK", &[], b"{\"a\":\"\xc3\x28\"}"),
        http("200 OK", &[], b""),
    ];
    // nesting deep enough to overflow a recursive reader that does not count
    cases.push(ok(&format!(
        "{}{}",
        "[".repeat(100_000),
        "]".repeat(100_000)
    )));
    cases.push(ok(&format!(
        "{{\"a\":{}1{}}}",
        "[".repeat(5000),
        "]".repeat(5000)
    )));
    let n = cases.len();
    let table = Arc::new(cases);
    let t2 = Arc::clone(&table);
    let f = Fake::start(move |s| Act::Raw(t2[s.n].clone()));
    let mut t = transport(&f);
    for i in 0..n {
        let e = ask(&mut t).unwrap_err();
        assert_eq!(http_error_code(&e), Some(E_HTTP_NOT_JSON), "case {i}");
        assert_eq!(e.to_string(), "error 213: the answer is not a JSON object");
    }
    // a JSON object with surrounding blanks is an answer
    let f = Fake::start(|_| Act::Raw(ok(" \r\n{\"jsonrpc\":\"2.0\",\"result\":1,\"id\":7}\n ")));
    let mut t = transport(&f);
    assert!(ask(&mut t).unwrap().is_some());
}

#[test]
fn hostile_headers_and_framing_are_refused_with_the_same_number_and_no_harm() {
    let long = "a".repeat(3000);
    let mut many = String::from("HTTP/1.1 200 OK\r\n");
    for i in 0..101 {
        many.push_str(&format!("X-{i}: 1\r\n"));
    }
    many.push_str("Content-Length: 2\r\n\r\n{}");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("head over the budget", format!("HTTP/1.1 200 OK\r\nX: {long}\r\nContent-Length: 2\r\n\r\n{{}}").into_bytes()),
        ("status line over the budget", format!("HTTP/1.1 200 {long}\r\n\r\n").into_bytes()),
        ("101 headers", many.into_bytes()),
        ("bare line feeds", b"HTTP/1.1 200 OK\nContent-Length: 2\n\n{}".to_vec()),
        ("bare line feed in a header", b"HTTP/1.1 200 OK\r\nX: a\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("bare carriage return", b"HTTP/1.1 200 OK\r\nX: a\rb\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("folded header", b"HTTP/1.1 200 OK\r\nX: a\r\n b\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("length and chunked", b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n".to_vec()),
        ("length twice, equal", b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("length twice, different", b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 9\r\n\r\n{}".to_vec()),
        ("negative length", b"HTTP/1.1 200 OK\r\nContent-Length: -1\r\n\r\n{}".to_vec()),
        ("signed length", b"HTTP/1.1 200 OK\r\nContent-Length: +2\r\n\r\n{}".to_vec()),
        ("hexadecimal length", b"HTTP/1.1 200 OK\r\nContent-Length: 0x2\r\n\r\n{}".to_vec()),
        ("length with a space inside", b"HTTP/1.1 200 OK\r\nContent-Length: 2 2\r\n\r\n{}".to_vec()),
        ("empty length", b"HTTP/1.1 200 OK\r\nContent-Length:\r\n\r\n{}".to_vec()),
        ("length of 19 digits", b"HTTP/1.1 200 OK\r\nContent-Length: 1000000000000000000\r\n\r\n{}".to_vec()),
        ("coding gzip", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\n{}".to_vec()),
        ("coding chunked and gzip", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked, gzip\r\n\r\n0\r\n\r\n".to_vec()),
        ("coding twice", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n".to_vec()),
        ("header without colon", b"HTTP/1.1 200 OK\r\nNo colon here\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("header with no name", b"HTTP/1.1 200 OK\r\n: x\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("space in the name", b"HTTP/1.1 200 OK\r\nContent-Length : 2\r\n\r\n{}".to_vec()),
        ("control byte in a value", b"HTTP/1.1 200 OK\r\nX: a\x01b\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("byte over 127 in a value", b"HTTP/1.1 200 OK\r\nX: a\xc3\xa9\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("HTTP/2", b"HTTP/2 200\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("status of two digits", b"HTTP/1.1 20 OK\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("status of four digits", b"HTTP/1.1 2000 OK\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("no space after the status", b"HTTP/1.1 200OK\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("not HTTP at all", b"SSH-2.0-OpenSSH_9.9\r\n\r\n".to_vec()),
        ("control byte in the reason", b"HTTP/1.1 200 O\x00K\r\nContent-Length: 2\r\n\r\n{}".to_vec()),
        ("chunk size that is not hexadecimal", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n{}\r\n0\r\n\r\n".to_vec()),
        ("empty chunk size", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\r\n{}\r\n0\r\n\r\n".to_vec()),
        ("chunk size of 17 digits", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n00000000000000002\r\n{}\r\n0\r\n\r\n".to_vec()),
        ("negative chunk size", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n-2\r\n{}\r\n0\r\n\r\n".to_vec()),
        ("no CRLF after a chunk", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}XX0\r\n\r\n".to_vec()),
        ("bare line feed after a chunk size", b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\n{}\r\n0\r\n\r\n".to_vec()),
        ("endless trailer", {
            let mut v = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n".to_vec();
            for _ in 0..400 {
                v.extend_from_slice(b"X-T: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n");
            }
            v
        }),
    ];
    let n = cases.len();
    let table = Arc::new(cases);
    let t2 = Arc::clone(&table);
    let f = Fake::start(move |s| Act::Raw(t2[s.n].1.clone()));
    let mut conf = cfg(&f);
    conf.set_number("klipper.max_header_bytes", 512).unwrap();
    let mut t = HttpJsonRpc::new(conf).unwrap();
    for (name, _) in table.iter() {
        let r = ask(&mut t);
        assert_eq!(code_of(r), Some(E_HTTP_PROTOCOL), "{name}");
    }
    assert_eq!(f.seen().len(), n);
    // a head that never ends is cut by the budget and the connection closed on it
    let closed = Arc::new(AtomicBool::new(false));
    let c2 = Arc::clone(&closed);
    let f = Fake::start(move |_| Act::Endless(b"HTTP/1.1 200 OK\r\nX: ".to_vec(), Arc::clone(&c2)));
    let mut t = transport(&f);
    assert_eq!(code_of(ask(&mut t)), Some(E_HTTP_PROTOCOL));
    let until = Instant::now() + Duration::from_secs(3);
    while !closed.load(Ordering::SeqCst) && Instant::now() < until {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(closed.load(Ordering::SeqCst));
    // what is cut short is not a protocol error but a closed connection
    let cut: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"HTTP/1.1 200 OK\r\nContent-Le".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 50\r\n\r\n{\"a\"".to_vec(),
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n{\"a\"".to_vec(),
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n".to_vec(),
    ];
    let table = Arc::new(cut);
    let t2 = Arc::clone(&table);
    let f = Fake::start(move |s| Act::Raw(t2[s.n].clone()));
    let mut t = transport(&f);
    for i in 0..table.len() {
        assert_eq!(code_of(ask(&mut t)), Some(E_HTTP_CLOSED), "cut {i}");
    }
}

#[test]
fn a_slow_answer_is_cut_by_its_times_and_a_silent_one_by_the_wait() {
    let release = Arc::new(AtomicBool::new(false));
    let r2 = Arc::clone(&release);
    let full = ok(&result(&json!(7), json!({"x": "y".repeat(100)})));
    let f = Fake::start(move |s| match s.n {
        // bytes keep coming, each inside the gap, for longer than the whole answer may take
        0 => Act::Drip(full.clone(), 40),
        // the head begins and then nothing comes
        1 => Act::Then(
            b"HTTP/1.1 200 OK\r\n".to_vec(),
            Box::new(Act::Hold(Arc::clone(&r2), Box::new(Act::Close))),
        ),
        // nothing comes at all
        _ => Act::Hold(Arc::clone(&r2), Box::new(Act::Close)),
    });
    let mut conf = cfg(&f);
    conf.set_number("klipper.answer_timeout_ms", 400).unwrap();
    conf.set_number("klipper.read_timeout_ms", 300).unwrap();
    conf.set_number("klipper.line_timeout_ms", 1000).unwrap();
    let mut t = HttpJsonRpc::new(conf).unwrap();
    // the words of the error say which time cut it (the text, not the clock, so a busy computer cannot fool the test)
    let slow = "the answer stopped or was too slow";
    let silent = "no answer came in the time given";
    let e = ask(&mut t).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_TIMEOUT));
    assert!(e.to_string().contains(slow), "{e}");
    // the head that began and stopped: the gap between bytes is what cuts it
    let e = ask(&mut t).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_TIMEOUT));
    assert!(e.to_string().contains(slow), "{e}");
    // nothing at all, for longer than the wait of an ordinary order (one second at least)
    let began = Instant::now();
    let e = ask(&mut t).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_TIMEOUT));
    assert!(e.to_string().contains(silent), "{e}");
    assert!(began.elapsed() >= Duration::from_millis(900));
    release.store(true, Ordering::SeqCst);
}

#[test]
fn nothing_listening_is_unreachable_with_its_number() {
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let mut c = klipper_controller(HttpConfig::new("127.0.0.1", port)).unwrap();
    let e = c.status().unwrap_err();
    assert!(matches!(e, Error::Link(_)));
    assert_eq!(e.code(), Some(E_HTTP_UNREACHABLE));
    assert!(e.to_string().contains("error 217"));
}

#[test]
fn addresses_are_checked_strictly_and_other_networks_need_their_switch() {
    let ok_local = [
        "127.0.0.1",
        "127.9.9.9",
        "localhost",
        "LOCALHOST",
        "::1",
        "10.1.2.3",
        "172.16.0.1",
        "172.31.255.254",
        "192.168.0.180",
        "169.254.1.1",
        "fe80::1",
        "fd00::1",
        "::ffff:192.168.0.1",
    ];
    for h in ok_local {
        assert!(httprpc::resolve(h, 7125, false).is_ok(), "{h}");
    }
    let outside = [
        "8.8.8.8",
        "172.32.0.1",
        "172.15.0.1",
        "100.64.0.1",
        "2001:db8::1",
        "::ffff:8.8.8.8",
        "1.1.1.1",
    ];
    for h in outside {
        let e = httprpc::resolve(h, 7125, false).unwrap_err();
        assert_eq!(e.code, E_HTTP_HOST, "{h}");
        // with the switch on they are taken
        assert!(
            httprpc::resolve(h, 7125, true).is_ok(),
            "{h} with the switch"
        );
    }
    let malformed = [
        "",
        " 127.0.0.1",
        "127.0.0.1 ",
        "127.0.0.1:7125",
        "http://127.0.0.1",
        "127.0.0.1/x",
        "user@127.0.0.1",
        "[::1]",
        "127.1",
        "0x7f.1",
        "2130706433",
        "fe80::1%eth0",
        "a\r\nHost: x",
        "127.0.0.1\0",
        "ünï.local",
        "example.com",
        "printer.local",
        "a b",
        "-a.com",
        "a..b",
        "a-.com",
        &"a".repeat(300),
    ];
    for h in malformed {
        let e = httprpc::resolve(h, 7125, false).unwrap_err();
        assert_eq!(e.code, E_HTTP_HOST, "{h:?}");
    }
    // a name that is not well formed is refused even with the switch on (and nothing is resolved, so no network is needed)
    for h in [
        "a b",
        "-a.com",
        "a..b",
        "a-.com",
        "127.1",
        "0x7f.1",
        "2130706433",
        "ünï.local",
        "a\r\nHost: x",
        "",
    ] {
        assert_eq!(
            httprpc::resolve(h, 7125, true).unwrap_err().code,
            E_HTTP_HOST,
            "{h:?}"
        );
    }
    // never a machine, switch or not
    for h in [
        "0.0.0.0",
        "::",
        "224.0.0.1",
        "239.255.255.250",
        "255.255.255.255",
        "ff02::1",
        "::ffff:0.0.0.0",
    ] {
        assert!(httprpc::resolve(h, 7125, false).is_err(), "{h}");
        assert!(
            httprpc::resolve(h, 7125, true).is_err(),
            "{h} with the switch"
        );
    }
    // the port
    assert!(httprpc::resolve("127.0.0.1", 0, true).is_err());
    assert!(httprpc::resolve("127.0.0.1", 1, false).is_ok());
    assert!(httprpc::resolve("127.0.0.1", 65535, false).is_ok());
    // the transport and the file say it too, with the number
    let e = HttpJsonRpc::new(HttpConfig::new("8.8.8.8", 7125)).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_HOST));
    assert!(HttpConfig::from_cfg("klipper.host = 8.8.8.8").is_err());
    // the switch written after the host is the same as before it
    assert!(HttpConfig::from_cfg("klipper.host = 8.8.8.8\nklipper.allow_non_local = true").is_ok());
    assert!(HttpConfig::from_cfg("klipper.allow_non_local = true\nklipper.host = 8.8.8.8").is_ok());
    let mut on = HttpConfig::new("8.8.8.8", 7125);
    on.allow_non_local = true;
    assert!(HttpJsonRpc::new(on).is_ok());
    // the Host header of an address of IPv6 has brackets
    if let Ok(l) = TcpListener::bind("[::1]:0") {
        let port = l.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(String::new()));
        let s2 = Arc::clone(&seen);
        let h = thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut b = vec![0u8; 2048];
            let n = s.read(&mut b).unwrap();
            *s2.lock().unwrap() = String::from_utf8_lossy(&b[..n]).to_string();
            let _ = s.write_all(&ok("{}"));
        });
        let mut t = HttpJsonRpc::new(HttpConfig::new("::1", port)).unwrap();
        assert!(ask(&mut t).unwrap().is_some());
        h.join().unwrap();
        assert!(seen
            .lock()
            .unwrap()
            .contains(&format!("Host: [::1]:{port}\r\n")));
    }
}

#[test]
fn the_settings_have_their_ranges_and_defaults_and_the_file_refuses_what_is_wrong() {
    let d = HttpConfig::default();
    for s in SETTINGS {
        assert_eq!(d.number(s.key), Some(s.default), "{}", s.key);
        assert!(s.min <= s.default && s.default <= s.max, "{}", s.key);
        let mut c = HttpConfig::default();
        assert!(c.set_number(s.key, s.min).is_ok());
        assert_eq!(c.number(s.key), Some(s.min));
        assert!(c.set_number(s.key, s.max).is_ok());
        assert_eq!(c.number(s.key), Some(s.max));
        assert!(c.set_number(s.key, s.max + 1).is_err(), "{} over", s.key);
        if s.min > 0 {
            assert!(c.set_number(s.key, s.min - 1).is_err(), "{} under", s.key);
        }
    }
    assert_eq!(d.host, "127.0.0.1");
    assert!(!d.allow_non_local && !d.has_api_key());
    assert_eq!(d.timeouts, eh3d_controller::grbl::Timeouts::default());
    assert_eq!(d.busy_limit, eh3d_controller::grbl::MAX_BUSY_WAIT);
    assert!(d.check().is_ok());
    assert_eq!(d.number("klipper.nothing"), None);
    assert!(HttpConfig::default()
        .set_number("klipper.nothing", 1)
        .is_err());
    // a file
    let text = "# the printer\n\n  klipper.host = 192.168.0.50  \nklipper.port=7126\nklipper.read_timeout_ms = 2500\nklipper.busy_limit_s = 600\nklipper.response_wait_ms = 700000\nklipper.api_key = abc123\n";
    let c = HttpConfig::from_cfg(text).unwrap();
    assert_eq!((c.host.as_str(), c.port), ("192.168.0.50", 7126));
    assert_eq!(c.read_timeout, Duration::from_millis(2500));
    assert_eq!(c.busy_limit, Duration::from_secs(600));
    assert!(c.has_api_key());
    let bad: [(&str, usize); 14] = [
        ("klipper.nothing = 1", 1),
        ("klipper.port = 7125\nklipper.port = 7126", 2),
        ("klipper.port 7125", 1),
        ("klipper.port = 12x", 1),
        ("klipper.port = -1", 1),
        ("klipper.port = 0", 1),
        ("klipper.port = 65536", 1),
        ("klipper.max_in_flight = 0", 1),
        ("klipper.max_response_bytes = 10", 1),
        ("\n\nklipper.allow_non_local = yes", 3),
        ("klipper.busy_limit_s = 14401", 1),
        ("klipper.connect_timeout_ms = 99", 1),
        ("klipper.host = a b", 0),
        (
            "klipper.busy_limit_s = 14400\nklipper.response_wait_ms = 1000",
            0,
        ),
    ];
    for (text, line) in bad {
        let e = HttpConfig::from_cfg(text).unwrap_err();
        assert_eq!(e.line, line, "{text:?}: {e}");
    }
    // a script is never cut off before the controller itself gives up
    let mut short = HttpConfig::new("127.0.0.1", 7125);
    short.response_wait = Duration::from_secs(1);
    let e = HttpJsonRpc::new(short).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_TIMEOUT));
    // the guide names every key and every number
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CONTROLLERS.md");
    if let Ok(guide) = std::fs::read_to_string(path) {
        for s in SETTINGS {
            assert!(guide.contains(s.key), "{} is not in the guide", s.key);
        }
        for key in ["klipper.host", "klipper.allow_non_local", "klipper.api_key"] {
            assert!(guide.contains(key), "{key} is not in the guide");
        }
        for code in HTTP_KINDS {
            assert!(guide.contains(&code.to_string()) || guide.contains("209 to 219"));
        }
    }
}

#[test]
fn requests_that_wait_are_abandoned_when_the_places_run_out_and_the_stop_always_gets_one() {
    let release = Arc::new(AtomicBool::new(false));
    let r2 = Arc::clone(&release);
    let f = Fake::start(move |s| {
        let reply = Act::Raw(ok(&result(&s.id(), json!("ok"))));
        if s.method() == "printer.info" && s.n < 2 {
            Act::Hold(Arc::clone(&r2), Box::new(reply))
        } else {
            reply
        }
    });
    let mut conf = cfg(&f);
    conf.set_number("klipper.max_in_flight", 2).unwrap();
    let mut t = HttpJsonRpc::new(conf).unwrap();
    t.send(INFO).unwrap();
    t.send(INFO).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    while t.in_flight() < 2 && Instant::now() < until {
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(t.in_flight(), 2);
    // a third one finds no place: the two that wait are abandoned and it goes out
    t.send(INFO).unwrap();
    let third = t.recv(Duration::from_secs(3)).unwrap().unwrap();
    assert!(third.contains("result"));
    // the two that were abandoned never bring an answer, not even when the server answers them
    release.store(true, Ordering::SeqCst);
    assert_eq!(t.recv(Duration::from_millis(300)).unwrap(), None);
    let until = Instant::now() + Duration::from_secs(3);
    while t.in_flight() > 0 && Instant::now() < until {
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(t.in_flight(), 0);
    // the stop goes out with every place taken
    release.store(false, Ordering::SeqCst);
    let f2 = Fake::start(|s| {
        if s.method() == "printer.emergency_stop" {
            Act::Raw(ok(&result(&s.id(), json!("ok"))))
        } else {
            Act::Hold(Arc::new(AtomicBool::new(false)), Box::new(Act::Close))
        }
    });
    let mut conf = cfg(&f2);
    conf.set_number("klipper.max_in_flight", 2).unwrap();
    let mut t = HttpJsonRpc::new(conf).unwrap();
    t.send(INFO).unwrap();
    t.send(INFO).unwrap();
    t.send(r#"{"jsonrpc":"2.0","method":"printer.emergency_stop","params":{},"id":3}"#)
        .unwrap();
    assert!(f2.wait_for("printer.emergency_stop"));
    assert!(t
        .recv(Duration::from_secs(3))
        .unwrap()
        .unwrap()
        .contains("result"));
    // the number of a refusal for want of a place has its text
    assert!(dialtext::klipper(E_HTTP_BUSY, "pt").is_some());
}

#[test]
fn dropping_the_transport_closes_the_connections_that_wait() {
    let closed = Arc::new(AtomicBool::new(false));
    let c2 = Arc::clone(&closed);
    let f = Fake::start(move |_| Act::WatchClose(Arc::clone(&c2)));
    let mut t = transport(&f);
    t.send(INFO).unwrap();
    assert!(f.wait_for("printer.info"));
    drop(t);
    let until = Instant::now() + Duration::from_secs(3);
    while !closed.load(Ordering::SeqCst) && Instant::now() < until {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(closed.load(Ordering::SeqCst), "the connection stayed open");
}

#[test]
fn a_request_that_is_not_json_never_leaves_and_the_route_and_the_body_are_the_ones_of_moonraker() {
    let f = Fake::start(|s| Act::Raw(ok(&result(&s.id(), json!("ok")))));
    let mut t = transport(&f);
    for bad in ["", "not json", "{"] {
        let e = t.send(bad).unwrap_err();
        assert!(http_error_code(&e).is_some(), "{bad:?}");
    }
    assert_eq!(f.seen().len(), 0);
    let text =
        r#"{"jsonrpc":"2.0","method":"printer.gcode.script","params":{"script":"G28"},"id":9}"#;
    t.send(text).unwrap();
    assert!(t.recv(Duration::from_secs(3)).unwrap().is_some());
    let seen = f.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].body, text);
    assert!(seen[0]
        .head
        .contains(&format!("Content-Length: {}\r\n", text.len())));
    assert_eq!(httprpc::PATH, "/server/jsonrpc");
}

#[test]
fn klipper_is_driven_and_a_controller_over_the_socket_does_what_it_is_told() {
    assert!(Family::Klipper.is_driven());
    let stopped = Arc::new(AtomicBool::new(false));
    let f = Fake::start(moonraker(Arc::clone(&stopped)));
    let mut c = klipper_controller(cfg(&f)).unwrap();
    assert_eq!(c.family(), Family::Klipper);
    c.command(Command::Home(vec![])).unwrap();
    c.send_line("G1 X20 F600").unwrap();
    c.command(Command::Jog {
        axis: 0,
        distance_mm: 1.0,
        feed_mm_min: 300.0,
    })
    .unwrap();
    let st = c.status().unwrap();
    assert_eq!(st.state, State::Idle);
    assert_eq!(st.machine_mm[..3], [10.0, 20.0, 5.0]);
    assert_eq!(c.max_travel().unwrap(), Some([200.0, 200.0, 50.0]));
    c.command(Command::Pause).unwrap();
    c.command(Command::Resume).unwrap();
    let scripts: Vec<String> = f
        .seen()
        .iter()
        .filter(|s| s.method() == "printer.gcode.script")
        .map(|s| s.json()["params"]["script"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        scripts,
        [
            "G28",
            "G1 X20 F600",
            "G91",
            "G1 X1.000 F300",
            "G90",
            "PAUSE",
            "RESUME"
        ]
    );
    // the emergency stop shuts Klipper down; the unlock restarts it and is believed only when it says it is ready
    c.command(Command::EmergencyStop).unwrap();
    assert!(c.halted());
    assert!(f.wait_for("printer.emergency_stop"));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(c.status().unwrap().state, State::Alarm);
    assert!(matches!(c.send_line("G1 X1 F600"), Err(Error::Alarm(_))));
    c.command(Command::Unlock).unwrap();
    assert!(!c.halted());
    assert_eq!(c.status().unwrap().state, State::Idle);
    assert!(HTTP_KINDS
        .iter()
        .all(|k| dialtext::klipper(*k, "pt").is_some()));
}

#[test]
fn every_failure_of_the_transport_reads_back_as_its_number_and_has_its_words_in_eight_languages() {
    let count = Arc::new(AtomicUsize::new(0));
    let c2 = Arc::clone(&count);
    let f = Fake::start(move |_| {
        c2.fetch_add(1, Ordering::SeqCst);
        Act::Close
    });
    let mut t = transport(&f);
    let e = ask(&mut t).unwrap_err();
    assert_eq!(http_error_code(&e), Some(E_HTTP_CLOSED));
    assert!(http_error_code(&std::io::Error::other("x")).is_none());
    for code in HTTP_KINDS {
        assert!(httprpc::is_http_code(code));
        for lang in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
            assert!(dialtext::klipper(code, lang).is_some(), "{code} {lang}");
        }
    }
    assert!(!httprpc::is_http_code(208) && !httprpc::is_http_code(220));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
