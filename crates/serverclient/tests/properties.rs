//! Properties of the pieces of the client, checked on many inputs made by a generator with a fixed seed, so a failure is the same on every run.

use eh3d_serverclient::sim::{Fault, SimServer};
use eh3d_serverclient::transport::path_segment;
use eh3d_serverclient::{
    backoff, Client, ContractCache, Json, OfflineQueue, QueueLimits, Settings, TransportError,
};
use std::collections::HashSet;
use std::time::Duration;

/// xorshift64*: small, fast and the same everywhere.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn text(&mut self, alphabet: &[char], max_len: usize) -> String {
        let len = self.below(max_len + 1);
        (0..len)
            .map(|_| alphabet[self.below(alphabet.len())])
            .collect()
    }
}

const JSON_ALPHABET: [char; 34] = [
    '{', '}', '[', ']', '"', ',', ':', '\\', ' ', '0', '1', '9', '.', '-', '+', 'e', 'E', 't', 'r',
    'u', 'f', 'a', 'l', 's', 'n', '\n', '\u{e9}', '\u{4e2d}', 'x', '/', 'b', '\t', '\r', '\u{0}',
];

#[test]
fn the_json_reader_never_panics_on_random_text() {
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..4000 {
        let text = rng.text(&JSON_ALPHABET, 60);
        let _ = Json::parse(&text);
    }
}

#[test]
fn the_json_reader_never_panics_on_a_valid_document_with_a_few_bytes_changed() {
    let valid = r#"{"token":"abc","name":"admin","expires_s":3600,"list":[1,2.5,-3e2,true,null,"x\né"],"o":{"a":{"b":[]}}}"#;
    assert!(Json::parse(valid).is_ok());
    let mut rng = Rng::new(0x5EED_0002);
    let chars: Vec<char> = valid.chars().collect();
    for _ in 0..4000 {
        let mut doc = chars.clone();
        for _ in 0..=rng.below(3) {
            let at = rng.below(doc.len());
            match rng.below(3) {
                0 => doc[at] = JSON_ALPHABET[rng.below(JSON_ALPHABET.len())],
                1 => {
                    doc.remove(at);
                }
                _ => doc.insert(at, JSON_ALPHABET[rng.below(JSON_ALPHABET.len())]),
            }
        }
        let text: String = doc.into_iter().collect();
        let _ = Json::parse(&text);
    }
}

#[test]
fn deep_nesting_is_refused_not_a_stack_overflow() {
    for depth in [10usize, 100, 10_000] {
        let text = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let r = Json::parse(&text);
        if depth > 64 {
            assert!(r.is_err(), "depth {depth}");
        }
    }
}

fn unescape(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap();
            out.push(u8::from_str_radix(hex, 16).unwrap());
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

#[test]
fn a_name_in_a_path_cannot_change_where_the_request_goes_and_comes_back_whole() {
    let alphabet: Vec<char> = "ab Z09-._~/?#%&=\r\n\t\"'<>\\\u{e9}\u{4e2d}\u{1f600}:@"
        .chars()
        .collect();
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..3000 {
        let name = rng.text(&alphabet, 24);
        let seg = path_segment(&name);
        assert!(
            seg.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~%".contains(&b)),
            "{name:?} -> {seg}"
        );
        assert!(!seg.contains('/') && !seg.contains('?') && !seg.contains('#'));
        assert_eq!(unescape(&seg), name.as_bytes(), "{name:?}");
    }
}

#[test]
fn settings_written_and_read_again_are_the_same() {
    let word: Vec<char> = "abcxyz019-_.".chars().collect();
    let mut rng = Rng::new(0x5EED_0004);
    for _ in 0..500 {
        let mut s = Settings::default();
        let host = rng.text(&word, 12);
        let address = if host.is_empty() {
            String::new()
        } else {
            format!("{host}:{}", 1 + rng.below(65_535))
        };
        s.set("server_address", &address).unwrap();
        s.set("server_token", &rng.text(&word, 20)).unwrap();
        s.set("server_machine", &rng.text(&word, 10)).unwrap();
        s.set("server_timeout_s", &(1 + rng.below(120)).to_string())
            .unwrap();
        s.set("server_retries", &rng.below(6).to_string()).unwrap();
        s.set("server_mdns", if rng.below(2) == 0 { "on" } else { "off" })
            .unwrap();
        s.set("server_max_answer_mib", &(1 + rng.below(256)).to_string())
            .unwrap();
        let back = Settings::from_text(&s.to_text(true)).unwrap();
        assert_eq!(back, s);
        let shown = s.to_text(false);
        if s.server_token.is_empty() {
            assert!(shown.contains(
                "server_token = 
"
            ));
        } else {
            assert!(shown.contains(
                "server_token = ***
"
            ));
        }
    }
}

#[test]
fn a_value_out_of_its_range_is_refused_and_changes_nothing() {
    let mut rng = Rng::new(0x5EED_0005);
    for _ in 0..500 {
        let mut s = Settings::default();
        let before = s.clone();
        let too_big = 121 + rng.below(100_000);
        assert!(s.set("server_timeout_s", &too_big.to_string()).is_err());
        assert!(s
            .set("server_retries", &(6 + rng.below(1000)).to_string())
            .is_err());
        assert!(s.set("server_timeout_s", "0").is_err());
        assert_eq!(s, before);
    }
}

#[test]
fn the_pauses_between_tries_never_shrink_and_never_pass_two_seconds() {
    let mut last = Duration::ZERO;
    for n in 0..100u32 {
        let p = backoff(n);
        assert!(p >= last, "{n}");
        assert!(p >= Duration::from_millis(200) && p <= Duration::from_secs(2));
        last = p;
    }
}

#[test]
fn the_queue_keeps_its_limits_whatever_is_done_to_it() {
    let limits = QueueLimits {
        max_items: 5,
        max_body_bytes: 12,
        max_total_bytes: 40,
        max_age_s: 50,
    };
    let mut q = OfflineQueue::new(limits);
    let mut rng = Rng::new(0x5EED_0006);
    let body_chars: Vec<char> = "abcdefgh".chars().collect();
    let mut now = 0u64;
    for _ in 0..5000 {
        now += rng.below(8) as u64;
        match rng.below(10) {
            0..=5 => {
                let method = ["PUT", "DELETE", "POST", "GET"][rng.below(4)];
                let path = format!("/p{}", rng.below(8));
                let body = rng.text(&body_chars, 16);
                let body = if body.is_empty() { None } else { Some(body) };
                let _ = q.push(method, &path, body.as_deref(), now);
            }
            6 | 7 => {
                q.pop_front();
            }
            8 => {
                q.expire(now);
            }
            _ => {
                if rng.below(20) == 0 {
                    q.clear();
                }
            }
        }
        assert!(q.len() <= limits.max_items);
        assert!(q.total_bytes() <= limits.max_total_bytes);
        let sum: usize = q
            .iter()
            .map(|i| i.body.as_ref().map_or(0, String::len))
            .sum();
        assert_eq!(sum, q.total_bytes());
        let paths: HashSet<&str> = q.iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths.len(), q.len(), "a path twice");
        assert!(q.iter().all(|i| i.method == "PUT" || i.method == "DELETE"));
    }
}

#[test]
fn the_contract_cache_never_calls_retired_a_route_the_newest_contract_has() {
    let mut rng = Rng::new(0x5EED_0007);
    let methods = ["GET", "POST", "PUT", "DELETE"];
    for _ in 0..200 {
        let mut cache = ContractCache::new();
        for _ in 0..6 {
            let mut doc = String::from("{\"paths\":{");
            let mut first = true;
            for p in 0..6 {
                if rng.below(2) == 0 {
                    continue;
                }
                if !first {
                    doc.push(',');
                }
                first = false;
                doc.push_str(&format!("\"/r{p}/{{id}}\":{{"));
                doc.push_str(&format!(
                    "\"{}\":{{}}",
                    methods[rng.below(4)].to_lowercase()
                ));
                doc.push('}');
            }
            doc.push_str("}}");
            let Ok(contract) = eh3d_serverclient::Contract::from_openapi(&doc) else {
                continue; // no routes this time
            };
            cache.update(contract);
            let current = cache.current().routes();
            for r in cache.retired() {
                assert!(!current.contains(&r), "{r} is retired and current");
            }
            let again = ContractCache::from_text(&cache.to_text()).unwrap();
            assert_eq!(again, cache);
        }
    }
}

fn client_with(sim: &SimServer, retries: u32) -> Client {
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    s.set("server_retries", &retries.to_string()).unwrap();
    Client::new(s, Box::new(sim.clone()))
        .with_sleeper(Box::new(sim.sleeper()))
        .with_clock(Box::new(sim.clock()))
}

#[test]
fn a_get_is_tried_once_more_for_every_retry_allowed_and_no_more() {
    let mut rng = Rng::new(0x5EED_0008);
    for _ in 0..300 {
        let sim = SimServer::new();
        let retries = rng.below(6) as u32;
        let mut c = client_with(&sim, retries);
        c.login("admin", "secret-password", "studio").unwrap();
        c.reset_metrics();
        let faults = rng.below(9);
        let plan: Vec<Fault> = (0..faults)
            .map(|_| {
                if rng.below(2) == 0 {
                    Fault::NoConnection
                } else {
                    Fault::Timeout
                }
            })
            .collect();
        sim.script(plan);
        let result = c.get("/api/v1/echo");
        let tries = retries as usize + 1;
        let expected = if faults < tries { faults + 1 } else { tries };
        assert_eq!(c.metrics().attempts(), expected as u64);
        assert_eq!(
            result.is_ok(),
            faults < tries,
            "{faults} faults, {retries} retries"
        );
        assert_eq!(c.metrics().retries(), c.metrics().attempts() - 1);
    }
}

#[test]
fn a_post_that_may_have_been_sent_is_never_sent_again() {
    let mut rng = Rng::new(0x5EED_0009);
    for _ in 0..200 {
        let sim = SimServer::new();
        let mut c = client_with(&sim, rng.below(6) as u32);
        c.login("admin", "secret-password", "studio").unwrap();
        c.reset_metrics();
        sim.script(vec![Fault::Timeout; 8]);
        assert!(c.post("/api/v1/echo", Some("{}")).is_err());
        assert_eq!(c.metrics().attempts(), 1);
    }
    // the error says how many tries were made
    let sim = SimServer::new();
    let mut c = client_with(&sim, 2);
    c.login("admin", "secret-password", "studio").unwrap();
    sim.script(vec![Fault::Timeout]);
    match c.post("/api/v1/echo", None) {
        Err(eh3d_serverclient::ClientError::Unreachable { error, attempts }) => {
            assert_eq!(error, TransportError::Timeout);
            assert_eq!(attempts, 1);
        }
        other => panic!("{other:?}"),
    }
}

// ---- headers, tags and bytes of the project store ----

use eh3d_serverclient::transport::{parse_response_full, request_head};
use eh3d_serverclient::{
    check_headers, etag_header, parse_etag, project_file_path, Reply, Request, RequestParts,
    StoreError, Transport, ALLOWED_HEADERS,
};
use std::sync::{Arc, Mutex};

struct Keep(Arc<Mutex<Option<Request>>>);

impl Transport for Keep {
    fn send(&self, r: &Request) -> Result<Reply, TransportError> {
        *self.0.lock().unwrap() = Some(r.clone());
        Ok(Reply::new(200, "{}"))
    }
}

fn request_for(parts: &RequestParts, method: &str) -> Request {
    let slot = Arc::new(Mutex::new(None));
    let mut s = Settings::default();
    s.set("server_address", "127.0.0.1:18200").unwrap();
    s.set("server_token", "tok").unwrap();
    let mut c = Client::new(s, Box::new(Keep(slot.clone())));
    c.request_full(method, "/api/v1/x", parts).unwrap();
    let got = slot.lock().unwrap().clone().unwrap();
    got
}

#[test]
fn random_headers_never_panic_and_the_ones_that_pass_have_no_line_break() {
    let name_alphabet: Vec<char> = "AaBbIiFf-MatchContentTypexyz0 :\r\n\0\u{e9}"
        .chars()
        .collect();
    let value_alphabet: Vec<char> = "ab\"*/ ,;=09\r\n\t\0\u{7f}\u{e9}\u{4e2d}W"
        .chars()
        .collect();
    let mut rng = Rng::new(0x5EED_0101);
    let mut passed = 0;
    for _ in 0..6000 {
        let n = rng.below(4);
        let headers: Vec<(String, String)> = (0..n)
            .map(|_| {
                let name = if rng.below(2) == 0 {
                    ALLOWED_HEADERS[rng.below(ALLOWED_HEADERS.len())].to_string()
                } else {
                    rng.text(&name_alphabet, 14)
                };
                (name, rng.text(&value_alphabet, 20))
            })
            .collect();
        if check_headers(&headers).is_ok() {
            passed += 1;
            for (name, value) in &headers {
                assert!(ALLOWED_HEADERS.contains(&name.to_ascii_lowercase().as_str()));
                assert!(
                    value.bytes().all(|b| (0x20..0x7f).contains(&b)),
                    "{value:?}"
                );
                assert!(!name.contains(['\r', '\n', ':', ' ']));
            }
        }
    }
    assert!(
        passed > 100,
        "the generator reaches the accepted case ({passed})"
    );
}

#[test]
fn the_head_of_a_request_has_exactly_the_lines_it_should_whatever_the_headers() {
    let value_alphabet: Vec<char> = "abc012-*/\"=;, ".chars().collect();
    let mut rng = Rng::new(0x5EED_0102);
    for _ in 0..600 {
        let mut parts = RequestParts::new();
        let mut asked: Vec<&str> = Vec::new();
        for name in ["Accept", "Content-Type", "If-Match", "X-Request-Id"] {
            if rng.below(2) == 0 {
                let v = rng.text(&value_alphabet, 16);
                let v = v.trim().to_string();
                if !v.is_empty() {
                    parts = parts.with_header(name, &v);
                    asked.push(name);
                }
            }
        }
        let bytes = rng.below(3) == 0;
        if bytes {
            let n = rng.below(40);
            parts = parts.with_bytes((0..n).map(|_| rng.below(256) as u8).collect());
        }
        let method = ["GET", "PUT", "POST", "DELETE"][rng.below(4)];
        let request = request_for(&parts, method);
        let head = request_head(&request, "127.0.0.1:18200", request.payload().len());
        assert!(head.ends_with("\r\n\r\n"));
        let lines: Vec<&str> = head.trim_end_matches("\r\n\r\n").split("\r\n").collect();
        assert_eq!(lines[0], format!("{method} /api/v1/x HTTP/1.1"));
        // Host, Connection and Authorization, and what the program asked for
        let mut expected = 3 + asked.len();
        if !asked.contains(&"Accept") {
            expected += 1;
        }
        if bytes {
            expected += 1;
            if !asked.contains(&"Content-Type") {
                expected += 1;
            }
        } else if method == "POST" || method == "PUT" {
            expected += 1;
        }
        assert_eq!(lines.len() - 1, expected, "{head:?}");
        let known = [
            "host",
            "connection",
            "authorization",
            "accept",
            "content-type",
            "content-length",
            "if-match",
            "x-request-id",
        ];
        for line in &lines[1..] {
            let (name, _) = line.split_once(": ").unwrap_or_else(|| panic!("{line:?}"));
            assert!(
                known.contains(&name.to_ascii_lowercase().as_str()),
                "{line:?}"
            );
        }
        let mut names: Vec<String> = lines[1..]
            .iter()
            .map(|l| l.split(": ").next().unwrap().to_ascii_lowercase())
            .collect();
        names.sort();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "{head:?}");
    }
}

#[test]
fn a_tag_comes_back_as_it_went_and_never_carries_a_line_break() {
    let alphabet: Vec<char> = "abcXYZ0189-_.:+=/~\r\n \"\u{e9}W".chars().collect();
    let mut rng = Rng::new(0x5EED_0103);
    let mut good = 0;
    for _ in 0..6000 {
        let tag = rng.text(&alphabet, 140);
        match etag_header(&tag) {
            Ok(value) => {
                assert!(!value.contains(['\r', '\n', ' ']), "{value:?}");
                assert!(value.len() <= 130);
                if !tag.starts_with("W/")
                    && tag != "*"
                    && tag == tag.trim()
                    && !tag.starts_with('"')
                    && !tag.ends_with('"')
                {
                    good += 1;
                    assert_eq!(parse_etag(&value), tag);
                }
            }
            Err(_) => {
                // refused: it was empty, long, or had a character that cannot be sent
                let clean = !tag.is_empty()
                    && tag.len() <= 128
                    && tag.bytes().all(|b| (0x21..0x7f).contains(&b) && b != b'"');
                let weak = tag.starts_with("W/") || tag.starts_with('"') || tag.ends_with('"');
                assert!(!clean || weak, "{tag:?}");
            }
        }
    }
    assert!(good > 100, "{good}");
}

#[test]
fn the_names_of_files_make_one_safe_query_and_the_project_one_segment() {
    let alphabet: Vec<char> = "ab Z09-._~/?#%&=\r\n\"\\\u{e9}\u{4e2d}".chars().collect();
    let mut rng = Rng::new(0x5EED_0104);
    for _ in 0..3000 {
        let project = rng.text(&alphabet, 20);
        let name = rng.text(&alphabet, 30);
        match project_file_path("studio", &project, &name) {
            Ok(p) => {
                let (path, query) = p.split_once('?').expect("one query");
                assert!(!query.contains('?') && !query.contains('&') && !query.contains('#'));
                assert!(query.starts_with("name="));
                assert!(!p.contains(['\r', '\n', ' ']));
                assert_eq!(path.matches('/').count(), 8);
            }
            Err(_) => assert!(
                project.is_empty()
                    || project.len() > 128
                    || project == "."
                    || project == ".."
                    || name.is_empty()
                    || name.len() > 240
            ),
        }
    }
}

#[test]
fn bytes_written_with_the_right_tag_come_back_whole_and_a_stale_tag_always_conflicts() {
    let mut rng = Rng::new(0x5EED_0105);
    let sim = SimServer::new();
    let mut c = client_with(&sim, 0);
    c.login("admin", "secret-password", "studio").unwrap();
    let path = project_file_path("studio", "p-seed", "data/file.bin").unwrap();
    let mut tag: Option<String> = None;
    let mut stale: Vec<String> = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    for round in 0..300 {
        let n = rng.below(300);
        let bytes: Vec<u8> = (0..n).map(|_| rng.below(256) as u8).collect();
        match rng.below(4) {
            0 if !stale.is_empty() => {
                let old = &stale[rng.below(stale.len())];
                let e = c.put_if_match(&path, Some(old), &bytes, None).unwrap_err();
                assert!(
                    matches!(e, StoreError::PreconditionFailed { .. }),
                    "{round}"
                );
            }
            1 if tag.is_some() => {
                let e = c.put_if_match(&path, None, &bytes, None).unwrap_err();
                assert!(
                    matches!(e, StoreError::PreconditionRequired { .. }),
                    "{round}"
                );
            }
            _ => {
                let w = c.put_if_match(&path, tag.as_deref(), &bytes, None).unwrap();
                if let Some(t) = tag.take() {
                    if w.etag.as_deref() != Some(t.as_str()) {
                        stale.push(t);
                    }
                }
                tag = w.etag;
                current = bytes;
            }
        }
        let read = c.get_with_etag(&path).unwrap();
        assert_eq!(read.bytes, current, "{round}");
        assert_eq!(read.etag, tag, "{round}");
    }
    assert!(stale.len() > 10);
}

#[test]
fn the_reader_of_answers_never_panics_and_finds_the_headers_of_a_good_one() {
    let mut rng = Rng::new(0x5EED_0106);
    for _ in 0..4000 {
        let n = rng.below(200);
        let raw: Vec<u8> = (0..n)
            .map(|_| match rng.below(6) {
                0 => b'\r',
                1 => b'\n',
                2 => b':',
                _ => rng.below(256) as u8,
            })
            .collect();
        let _ = parse_response_full(&raw);
    }
    let good = b"HTTP/1.1 201 Created\r\nETag: \"t-1\"\r\nX-A: b: c\r\nContent-Length: 2\r\n\r\nhi";
    let r = parse_response_full(good).unwrap();
    assert_eq!(r.status, 201);
    assert_eq!(r.etag().as_deref(), Some("t-1"));
    assert_eq!(r.header("x-a"), Some("b: c"));
    assert_eq!(r.body, b"hi");
    // cut anywhere before the end of the head: an error, not a panic
    for cut in 0..good.len() - 2 {
        let _ = parse_response_full(&good[..cut]);
    }
    // more headers than the reader keeps are ignored, the body stays whole
    let mut many = b"HTTP/1.1 200 OK\r\n".to_vec();
    for i in 0..300 {
        many.extend_from_slice(format!("X-{i}: v\r\n").as_bytes());
    }
    many.extend_from_slice(b"\r\nbody");
    let r = parse_response_full(&many).unwrap();
    assert_eq!(r.headers.len(), 100);
    assert_eq!(r.body, b"body");
}
