//! The client of EH3D-NODE-BUNGARD-AUX, the node on the drill head: what it says about itself (`GET /api/status`), the measure of the table under the head
//! (`POST /api/measure`) and the camera as MJPEG (`GET /stream`). The words are the ones the node builds in `core/protocol.hpp`. This crate reads them into
//! types, builds the requests, cuts the MJPEG stream into pictures and has a small blocking HTTP client on the standard library, so every program that talks to the
//! node does it the same way. Reaching the node through EH3D-SERVER uses the same types: the server hands over the same JSON.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::Value;

/// How hot the spindle is, as the node judges it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heat {
    Unknown,
    Ok,
    Warm,
    Hot,
}

impl Heat {
    fn from_word(w: &str) -> Heat {
        match w {
            "ok" => Heat::Ok,
            "warm" => Heat::Warm,
            "hot" => Heat::Hot,
            _ => Heat::Unknown,
        }
    }
}

/// What the node says about itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub node: String,
    pub version: String,
    pub camera: bool,
    /// What the camera sensor says it is (`ov2640`, `ov5640`); empty when the node does not say or has no camera.
    pub camera_model: String,
    pub laser: bool,
    pub rpm_sensor: bool,
    pub rpm: f64,
    pub temp_c: Option<f64>,
    pub heat: Heat,
    /// The protect output is open: the spindle must stop.
    pub protect_open: bool,
    pub uptime_s: u64,
}

impl Status {
    pub fn from_json(text: &str) -> Result<Status, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let flag = |k: &str| {
            v.get(k)
                .and_then(Value::as_bool)
                .ok_or_else(|| format!("no `{k}` in the status"))
        };
        Ok(Status {
            node: v
                .get("node")
                .and_then(Value::as_str)
                .ok_or("no `node` in the status")?
                .to_string(),
            version: v
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            camera: flag("camera")?,
            camera_model: v
                .get("camera_model")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            laser: flag("laser")?,
            rpm_sensor: flag("rpm_sensor")?,
            rpm: v.get("rpm").and_then(Value::as_f64).unwrap_or(0.0),
            temp_c: v.get("temp_c").and_then(Value::as_f64),
            heat: Heat::from_word(v.get("heat").and_then(Value::as_str).unwrap_or("")),
            protect_open: flag("protect_open")?,
            uptime_s: v.get("uptime_s").and_then(Value::as_u64).unwrap_or(0),
        })
    }
}

/// One measure of the height of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    pub ok: bool,
    /// `ok`, or why the node refused: `no_readings`, `out_of_range`, `too_few`, `too_noisy`.
    pub why: String,
    pub distance_mm: f64,
    pub sigma_mm: f64,
    pub asked: u32,
    pub used: u32,
    pub rejected: u32,
}

impl Measure {
    pub fn from_json(text: &str) -> Result<Measure, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let num = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
        Ok(Measure {
            ok: v
                .get("ok")
                .and_then(Value::as_bool)
                .ok_or("no `ok` in the measure")?,
            why: v
                .get("why")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            distance_mm: num("distance_mm"),
            sigma_mm: num("sigma_mm"),
            asked: num("asked") as u32,
            used: num("used") as u32,
            rejected: num("rejected") as u32,
        })
    }

    /// The height of the table that this measure says (up is positive): the table is lower when it is farther. `None` when the node refused the measure.
    pub fn height_mm(&self) -> Option<f64> {
        self.ok.then_some(-self.distance_mm)
    }
}

/// The request for `method` and `path`, in plain HTTP/1.0 (the node closes the connection after the answer), with the token when there is one.
pub fn request_text(method: &str, host: &str, path: &str, token: &str) -> String {
    let mut r = format!("{method} {path} HTTP/1.0\r\nHost: {host}\r\n");
    if !token.is_empty() {
        r.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    r.push_str("Connection: close\r\nContent-Length: 0\r\n\r\n");
    r
}

/// The code and the body of an HTTP answer.
pub fn parse_response(bytes: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let end = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("the answer has no end of headers")?;
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| "the headers are not text")?;
    let code = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or("no status code in the answer")?;
    Ok((code, bytes[end + 4..].to_vec()))
}

/// The node (or the server in front of it) at an address, with its token.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub host: String,
    pub port: u16,
    pub token: String,
    pub timeout: Duration,
    /// Through EH3D-SERVER: the id of the machine whose node it is. Empty: the node itself.
    pub machine: String,
}

impl Link {
    pub fn new(host: &str, port: u16, token: &str) -> Link {
        Link {
            host: host.to_string(),
            port,
            token: token.to_string(),
            timeout: Duration::from_millis(2500),
            machine: String::new(),
        }
    }

    /// The node of a machine through EH3D-SERVER: the address and the token are the server's (a login or the token of a program), and the server holds the node's own.
    pub fn via_server(host: &str, port: u16, token: &str, machine: &str) -> Link {
        let mut l = Link::new(host, port, token);
        l.machine = machine.to_string();
        l
    }

    /// The path of a request of the node (`/api/status`, `/api/measure`, `/stream`) as this link asks for it: the same on the node, under the machine on the server.
    pub fn path(&self, node_path: &str) -> String {
        if self.machine.is_empty() {
            return node_path.to_string();
        }
        let base = format!("/api/v1/machines/{}/head", self.machine);
        let (path, query) = match node_path.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (node_path, None),
        };
        let tail = match path {
            "/api/status" => "/status",
            "/api/measure" => "/measure",
            "/stream" => "/stream",
            other => other,
        };
        match query {
            Some(q) => format!("{base}{tail}?{q}"),
            None => format!("{base}{tail}"),
        }
    }

    fn connect(&self) -> Result<TcpStream, String> {
        let addr = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|e| format!("{}: {e}", self.host))?
            .next()
            .ok_or_else(|| format!("{}: no address", self.host))?;
        let s = TcpStream::connect_timeout(&addr, self.timeout)
            .map_err(|e| format!("{}: {e}", self.host))?;
        let _ = s.set_read_timeout(Some(self.timeout));
        let _ = s.set_write_timeout(Some(self.timeout));
        Ok(s)
    }

    fn call(&self, method: &str, path: &str) -> Result<(u16, Vec<u8>), String> {
        let mut s = self.connect()?;
        s.write_all(request_text(method, &self.host, &self.path(path), &self.token).as_bytes())
            .map_err(|e| e.to_string())?;
        let mut all = Vec::new();
        s.read_to_end(&mut all).map_err(|e| e.to_string())?;
        parse_response(&all)
    }

    pub fn status(&self) -> Result<Status, String> {
        let (code, body) = self.call("GET", "/api/status")?;
        if code != 200 {
            return Err(format!("the node answered {code}"));
        }
        Status::from_json(&String::from_utf8_lossy(&body))
    }

    /// One measure of the table under the head, from `samples` readings (0: the number the node has in its settings).
    pub fn measure(&self, samples: u32) -> Result<Measure, String> {
        let path = if samples == 0 {
            "/api/measure".to_string()
        } else {
            format!("/api/measure?samples={samples}")
        };
        let (code, body) = self.call("POST", &path)?;
        match code {
            200 => Measure::from_json(&String::from_utf8_lossy(&body)),
            401 => Err("the node refused the token".to_string()),
            c => Err(format!("the node answered {c}")),
        }
    }

    /// Opens the camera stream: the connection is returned after the headers, ready to be read with an [`MjpegSplitter`].
    pub fn open_stream(&self) -> Result<TcpStream, String> {
        let mut s = self.connect()?;
        s.write_all(request_text("GET", &self.host, &self.path("/stream"), &self.token).as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(s)
    }
}

/// Cuts an MJPEG stream into pictures: the bytes go in as they come, and each complete JPEG (from its start `FF D8` to its end `FF D9`) comes out. What comes before the
/// first picture (the headers, the boundary lines) is dropped.
#[derive(Debug, Default)]
pub struct MjpegSplitter {
    buf: Vec<u8>,
}

impl MjpegSplitter {
    pub fn new() -> MjpegSplitter {
        MjpegSplitter::default()
    }

    /// Adds bytes and returns the pictures that are complete now.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        loop {
            let Some(start) = self.buf.windows(2).position(|w| w == [0xFF, 0xD8]) else {
                // no start in sight: keep at most the last byte (it may be the first half of a start)
                let keep = self.buf.last().copied().filter(|b| *b == 0xFF);
                self.buf.clear();
                self.buf.extend(keep);
                break;
            };
            let Some(len) = self.buf[start + 2..]
                .windows(2)
                .position(|w| w == [0xFF, 0xD9])
            else {
                self.buf.drain(..start);
                // a picture that never ends must not fill the memory
                if self.buf.len() > 8 * 1024 * 1024 {
                    self.buf.clear();
                }
                break;
            };
            let end = start + 2 + len + 2;
            out.push(self.buf[start..end].to_vec());
            self.buf.drain(..end);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    const STATUS: &str = r#"{"node":"head-1","version":"0.0.1","camera":true,"camera_model":"ov5640","laser":true,"rpm_sensor":true,"rpm":23998,"temp_c":41.3,"heat":"ok","protect_open":false,"uptime_s":90}"#;
    const MEASURE: &str = r#"{"ok":true,"why":"ok","distance_mm":100.012,"sigma_mm":0.021,"asked":7,"used":6,"rejected":1}"#;

    #[test]
    fn the_status_and_the_measure_are_read() {
        let s = Status::from_json(STATUS).unwrap();
        assert_eq!((s.node.as_str(), s.version.as_str()), ("head-1", "0.0.1"));
        assert!(s.camera && s.laser && s.rpm_sensor && !s.protect_open);
        assert_eq!(s.camera_model, "ov5640");
        // an older node does not say which camera it has
        let old = STATUS.replace(r#""camera_model":"ov5640","#, "");
        assert_eq!(Status::from_json(&old).unwrap().camera_model, "");
        assert_eq!(
            (s.rpm, s.temp_c, s.heat, s.uptime_s),
            (23998.0, Some(41.3), Heat::Ok, 90)
        );
        let none = STATUS.replace("41.3", "null");
        assert_eq!(Status::from_json(&none).unwrap().temp_c, None);
        assert!(Status::from_json("{}").is_err() && Status::from_json("no json").is_err());

        let m = Measure::from_json(MEASURE).unwrap();
        assert!(m.ok && m.used == 6 && m.rejected == 1);
        assert_eq!(m.height_mm(), Some(-100.012));
        let refused = Measure::from_json(r#"{"ok":false,"why":"too_noisy","distance_mm":100.0,"sigma_mm":0.3,"asked":7,"used":7,"rejected":0}"#).unwrap();
        assert!(!refused.ok && refused.why == "too_noisy" && refused.height_mm().is_none());
    }

    #[test]
    fn a_request_carries_the_token_and_an_answer_gives_its_code_and_body() {
        let r = request_text("POST", "10.0.0.5", "/api/measure?samples=5", "abcdefgh12");
        assert!(r.starts_with("POST /api/measure?samples=5 HTTP/1.0\r\n"));
        assert!(r.contains("Authorization: Bearer abcdefgh12\r\n") && r.ends_with("\r\n\r\n"));
        assert!(!request_text("GET", "h", "/api/status", "").contains("Authorization"));
        let (code, body) =
            parse_response(b"HTTP/1.0 200 OK\r\nContent-Type: x\r\n\r\n{\"a\":1}").unwrap();
        assert_eq!((code, body), (200, b"{\"a\":1}".to_vec()));
        assert!(
            parse_response(b"nothing").is_err() && parse_response(b"HTTP/1.0\r\n\r\n").is_err()
        );
    }

    /// A node that answers one request with `answer` and closes; it gives back the request it got.
    fn fake_node(answer: String) -> (u16, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 512];
            while !got.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = s.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
            }
            s.write_all(answer.as_bytes()).unwrap();
            String::from_utf8_lossy(&got).to_string()
        });
        (port, handle)
    }

    #[test]
    fn the_link_asks_the_node_over_the_network() {
        let (port, node) = fake_node(format!("HTTP/1.0 200 OK\r\n\r\n{STATUS}"));
        let s = Link::new("127.0.0.1", port, "").status().unwrap();
        assert_eq!(s.node, "head-1");
        assert!(node.join().unwrap().starts_with("GET /api/status HTTP/1.0"));

        let (port, node) = fake_node(format!("HTTP/1.0 200 OK\r\n\r\n{MEASURE}"));
        let m = Link::new("127.0.0.1", port, "abcdefgh12")
            .measure(5)
            .unwrap();
        assert!(m.ok && (m.distance_mm - 100.012).abs() < 1e-9);
        let request = node.join().unwrap();
        assert!(
            request.starts_with("POST /api/measure?samples=5 HTTP/1.0")
                && request.contains("Bearer abcdefgh12")
        );

        let (port, node) =
            fake_node("HTTP/1.0 401 Unauthorized\r\n\r\n{\"error\":\"unauthorized\"}".to_string());
        assert!(Link::new("127.0.0.1", port, "wrong")
            .measure(0)
            .unwrap_err()
            .contains("token"));
        node.join().unwrap();

        // nothing listens there
        let free = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        assert!(Link::new("127.0.0.1", free, "").status().is_err());
    }

    #[test]
    fn through_the_server_the_paths_are_the_ones_of_the_machine() {
        let node = Link::new("h", 80, "t");
        assert_eq!(node.path("/api/status"), "/api/status");
        let via = Link::via_server("srv", 18200, "login", "bench-1");
        assert_eq!(
            via.path("/api/status"),
            "/api/v1/machines/bench-1/head/status"
        );
        assert_eq!(
            via.path("/api/measure?samples=5"),
            "/api/v1/machines/bench-1/head/measure?samples=5"
        );
        assert_eq!(via.path("/stream"), "/api/v1/machines/bench-1/head/stream");
    }

    #[test]
    fn the_link_asks_the_server_for_the_node_of_a_machine() {
        let (port, node) = fake_node(format!("HTTP/1.0 200 OK\r\n\r\n{MEASURE}"));
        let m = Link::via_server("127.0.0.1", port, "login-token", "bench-1")
            .measure(0)
            .unwrap();
        assert!(m.ok);
        let request = node.join().unwrap();
        assert!(
            request.starts_with("POST /api/v1/machines/bench-1/head/measure HTTP/1.0")
                && request.contains("Bearer login-token")
        );
    }

    #[test]
    fn the_stream_is_cut_into_pictures_however_the_bytes_arrive() {
        let jpeg = |n: u8| vec![0xFF, 0xD8, n, n, 0xFF, 0xD9];
        let mut stream = b"HTTP/1.0 200 OK\r\nContent-Type: multipart/x-mixed-replace;boundary=frame\r\n\r\n--frame\r\nContent-Type: image/jpeg\r\nContent-Length: 6\r\n\r\n".to_vec();
        stream.extend(jpeg(1));
        stream.extend(b"\r\n--frame\r\nContent-Type: image/jpeg\r\n\r\n");
        stream.extend(jpeg(2));
        stream.extend(b"\r\n--frame\r\n\r\n");
        stream.extend(jpeg(3));
        // all at once
        let mut a = MjpegSplitter::new();
        assert_eq!(a.push(&stream), vec![jpeg(1), jpeg(2), jpeg(3)]);
        // one byte at a time
        let mut b = MjpegSplitter::new();
        let got: Vec<Vec<u8>> = stream.iter().flat_map(|x| b.push(&[*x])).collect();
        assert_eq!(got, vec![jpeg(1), jpeg(2), jpeg(3)]);
        // a picture that has not ended waits for its end
        let mut c = MjpegSplitter::new();
        assert!(c.push(&[0xFF, 0xD8, 9, 9]).is_empty());
        assert_eq!(
            c.push(&[0xFF, 0xD9]),
            vec![vec![0xFF, 0xD8, 9, 9, 0xFF, 0xD9]]
        );
    }
}
