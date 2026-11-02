//! GRBL 1.1 over a line transport (a serial port in practice): the status report, the orders and the lines of G-code, as the free firmware
//! defines them. grblHAL and FluidNC speak the same protocol, so the same controller drives them (each reports itself as its own family).
//!
//! The lines are sent one at a time and each one waits for its `ok` (or `error:N`): simple and safe, not the fastest way to stream.

use std::io;
use std::time::{Duration, Instant};

use crate::{Capabilities, Command, Controller, Error, Family, State, Status, AXES};

/// What the controller needs from the link: bytes out, and lines in with a time limit.
pub trait LineTransport {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>;
    /// The next line, or `None` when nothing came in the time given.
    fn read_line(&mut self, timeout: Duration) -> io::Result<Option<String>>;
}

/// The control bytes of the protocol.
const STATUS: u8 = b'?';
const HOLD: u8 = b'!';
const RESUME: u8 = b'~';
const RESET: u8 = 0x18;
const JOG_CANCEL: u8 = 0x85;

/// How long the controller waits for the end of a line, by the kind of line. Most lines are taken at once; a homing cycle (`$H`) and a
/// jog (`$J=...`) answer only when the machine has finished moving, which can take a long time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// Any line that is not one of the two below, the status report and the settings.
    pub line: Duration,
    /// The homing cycle (`$H`).
    pub homing: Duration,
    /// A jog (`$J=...`).
    pub jog: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Timeouts {
            line: Duration::from_secs(5),
            homing: Duration::from_secs(120),
            jog: Duration::from_secs(60),
        }
    }
}

/// The most a line may wait for its answer while the machine says it is working.
const MAX_BUSY_WAIT: Duration = Duration::from_secs(4 * 3600);

enum Probe {
    Answered(Result<(), Error>),
    Busy,
    Silent,
}

/// The kinds of line that wait for different times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Plain,
    Homing,
    Jog,
}

impl Kind {
    fn of(text: &str) -> Kind {
        let t = text.trim().to_ascii_uppercase();
        if t.starts_with("$J=") {
            Kind::Jog
        } else if t.starts_with("$H") {
            Kind::Homing
        } else {
            Kind::Plain
        }
    }
}

pub struct GrblController<T: LineTransport> {
    transport: T,
    family: Family,
    timeouts: Timeouts,
    /// The offset of the work from the machine, which GRBL reports from time to time (`WCO:`).
    wco: [f64; AXES],
}

impl<T: LineTransport> GrblController<T> {
    pub fn new(transport: T) -> Self {
        GrblController {
            transport,
            family: Family::Grbl,
            timeouts: Timeouts::default(),
            wco: [0.0; AXES],
        }
    }

    /// The same protocol reported as another family (grblHAL, FluidNC).
    pub fn as_family(mut self, family: Family) -> Self {
        self.family = family;
        self
    }

    /// The time to wait for an ordinary line (the others keep theirs).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeouts.line = timeout;
        self
    }

    /// The times to wait for each kind of line.
    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    fn link(e: io::Error) -> Error {
        Error::Link(e.to_string())
    }

    /// Asks for the state until the hold is complete (`Hold:0`, or idle or alarm), at most `limit`.
    fn wait_hold_done(&mut self, limit: Duration) {
        let end = Instant::now() + limit;
        while Instant::now() < end {
            if self.byte(STATUS).is_err() {
                return;
            }
            let ask_until = Instant::now() + Duration::from_millis(250);
            while Instant::now() < ask_until {
                let left = ask_until.saturating_duration_since(Instant::now());
                match self.transport.read_line(left) {
                    Ok(Some(l)) => {
                        let l = l.trim();
                        if l.starts_with("<Hold:0")
                            || l.starts_with("<Idle")
                            || l.starts_with("<Alarm")
                        {
                            return;
                        }
                        if l.starts_with('<') {
                            break;
                        }
                    }
                    Ok(None) | Err(_) => return,
                }
            }
        }
    }

    /// Waits for the end of an order, at most `limit` in all: `ok` is done; `error:N` and `ALARM:N` are refused; the other lines
    /// (messages, reports) are skipped.
    fn wait_ok(&mut self, limit: Duration) -> Result<(), Error> {
        let end = Instant::now() + limit;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(Error::Timeout);
            }
            match self.transport.read_line(left).map_err(Self::link)? {
                None => return Err(Error::Timeout),
                Some(l) => {
                    let l = l.trim();
                    if l == "ok" {
                        return Ok(());
                    }
                    if let Some(code) = l.strip_prefix("error:") {
                        return Err(Error::Refused(format!("error {code}")));
                    }
                    if let Some(code) = l.strip_prefix("ALARM:") {
                        return Err(Error::Alarm(format!("alarm {code}")));
                    }
                }
            }
        }
    }

    fn line(&mut self, text: &str) -> Result<(), Error> {
        let kind = Kind::of(text);
        let limit = match kind {
            Kind::Plain => self.timeouts.line,
            Kind::Homing => self.timeouts.homing,
            Kind::Jog => self.timeouts.jog,
        };
        self.transport
            .write_all(format!("{text}\n").as_bytes())
            .map_err(Self::link)?;
        let mut done = self.wait_ok(limit);
        // GRBL answers `ok` when the line enters its buffer, which with the buffer full is when an older block ends: a slow move, a dwell or
        // a spindle change can keep it from answering far longer than the limit while the machine works. As long as it says it is busy, wait on.
        let mut waited = limit;
        while kind == Kind::Plain && done == Err(Error::Timeout) && waited < MAX_BUSY_WAIT {
            match self.probe_busy() {
                Probe::Answered(r) => {
                    done = r;
                    break;
                }
                Probe::Busy => {
                    done = self.wait_ok(limit);
                    waited += limit;
                }
                Probe::Silent => break,
            }
        }
        if done == Err(Error::Timeout) {
            // never leave the machine moving: a homing cycle ignores a hold, so it is reset; a jog is cancelled; the rest is held
            let _ = self.byte(match kind {
                Kind::Homing => RESET,
                Kind::Jog => JOG_CANCEL,
                Kind::Plain => HOLD,
            });
        }
        done
    }

    /// Asks for the state while an order has not been answered: tells whether the answer came meanwhile, the machine is working or it is silent.
    fn probe_busy(&mut self) -> Probe {
        if self.byte(STATUS).is_err() {
            return Probe::Silent;
        }
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Probe::Silent;
            }
            match self.transport.read_line(left) {
                Ok(Some(l)) => {
                    let l = l.trim();
                    if l == "ok" {
                        return Probe::Answered(Ok(()));
                    }
                    if let Some(code) = l.strip_prefix("error:") {
                        return Probe::Answered(Err(Error::Refused(format!("error {code}"))));
                    }
                    if let Some(code) = l.strip_prefix("ALARM:") {
                        return Probe::Answered(Err(Error::Alarm(format!("alarm {code}"))));
                    }
                    if l.starts_with("<Run")
                        || l.starts_with("<Hold")
                        || l.starts_with("<Jog")
                        || l.starts_with("<Home")
                    {
                        return Probe::Busy;
                    }
                    if l.starts_with('<') {
                        return Probe::Silent;
                    }
                }
                _ => return Probe::Silent,
            }
        }
    }

    fn byte(&mut self, b: u8) -> Result<(), Error> {
        self.transport.write_all(&[b]).map_err(Self::link)
    }
}

/// A line without its comments (`( ... )` and `;` to the end) and without the characters that GRBL takes as real-time commands wherever they
/// are in the stream (`!`, `~`, `?`, the reset and everything that is not ASCII): a comment such as `(tool change!)` would otherwise hold the feed.
fn without_comments_and_realtime(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0u32;
    for c in line.chars() {
        match c {
            ';' if depth == 0 => break,
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            '!' | '~' | '?' | '\u{18}' => {}
            c if c.is_ascii() && !c.is_ascii_control() || c == ' ' || c == '\t' => out.push(c),
            _ => {}
        }
    }
    out
}

/// Reads a line of the settings (`$130=500.000`): the number and the value. `None` when the line is not one.
fn parse_setting(line: &str) -> Option<(u32, f64)> {
    let (key, value) = line.trim().strip_prefix('$')?.split_once('=')?;
    let value: f64 = value.split_whitespace().next()?.parse().ok()?;
    value
        .is_finite()
        .then_some((key.trim().parse().ok()?, value))
}

fn axis_name(axis: usize) -> Result<char, Error> {
    ['X', 'Y', 'Z', 'A', 'B', 'C']
        .get(axis)
        .copied()
        .ok_or(Error::Unsupported("that axis"))
}

/// Reads a status report (`<Idle|MPos:1.000,2.000,3.000|FS:0,0|...>`); `wco` is the work offset known so far and is updated when the
/// report brings one. `None` when the line is not a report.
pub fn parse_status(line: &str, wco: &mut [f64; AXES]) -> Option<Status> {
    let body = line.trim().strip_prefix('<')?.strip_suffix('>')?;
    let mut parts = body.split('|');
    let head = parts.next()?;
    let state = match head.split(':').next()? {
        "Idle" => State::Idle,
        "Run" => State::Run,
        "Hold" => State::Hold,
        "Jog" => State::Jog,
        "Home" => State::Home,
        "Alarm" => State::Alarm,
        "Door" => State::Door,
        "Check" => State::Check,
        "Sleep" => State::Sleep,
        _ => State::Unknown,
    };
    let mut st = Status {
        state,
        ..Status::default()
    };
    let nums =
        |v: &str| -> Vec<f64> { v.split(',').filter_map(|n| n.trim().parse().ok()).collect() };
    let mut mpos: Option<Vec<f64>> = None;
    let mut wpos: Option<Vec<f64>> = None;
    for p in parts {
        let Some((k, v)) = p.split_once(':') else {
            continue;
        };
        match k {
            "MPos" => mpos = Some(nums(v)),
            "WPos" => wpos = Some(nums(v)),
            "WCO" => {
                for (i, n) in nums(v).into_iter().take(AXES).enumerate() {
                    wco[i] = n;
                }
            }
            "FS" => {
                let n = nums(v);
                st.feed_mm_min = n.first().copied().unwrap_or(0.0);
                st.spindle_rpm = n.get(1).copied().unwrap_or(0.0);
            }
            "F" => st.feed_mm_min = nums(v).first().copied().unwrap_or(0.0),
            "Bf" => {
                // free blocks of the planner (of 15 here) and free bytes of the serial buffer
                let n = nums(v);
                if let Some(&blocks) = n.first() {
                    st.buffer_percent =
                        Some((100.0 - blocks.min(15.0) / 15.0 * 100.0).round() as u8);
                }
            }
            "Pn" => st.pins = v.to_string(),
            _ => {}
        }
    }
    match (mpos, wpos) {
        (Some(m), w) => {
            for (i, n) in m.into_iter().take(AXES).enumerate() {
                st.machine_mm[i] = n;
                st.work_mm[i] = n - wco[i];
            }
            if let Some(w) = w {
                for (i, n) in w.into_iter().take(AXES).enumerate() {
                    st.work_mm[i] = n;
                }
            }
        }
        (None, Some(w)) => {
            for (i, n) in w.into_iter().take(AXES).enumerate() {
                st.work_mm[i] = n;
                st.machine_mm[i] = n + wco[i];
            }
        }
        (None, None) => {}
    }
    Some(st)
}

impl<T: LineTransport> Controller for GrblController<T> {
    fn family(&self) -> Family {
        self.family
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            axes: 3,
            spindle: true,
            spindle_speed: true,
            coolant: true,
            homing: true,
            probe: true,
            jog: true,
            pause: true,
            emergency_stop: true,
            tool_changer: false,
            gcode: true,
        }
    }

    fn status(&mut self) -> Result<Status, Error> {
        self.byte(STATUS)?;
        loop {
            match self
                .transport
                .read_line(self.timeouts.line)
                .map_err(Self::link)?
            {
                None => return Err(Error::Timeout),
                Some(l) => {
                    if let Some(st) = parse_status(&l, &mut self.wco) {
                        return Ok(st);
                    }
                }
            }
        }
    }

    fn command(&mut self, command: Command) -> Result<(), Error> {
        match command {
            Command::Jog {
                axis,
                distance_mm,
                feed_mm_min,
            } => {
                let a = axis_name(axis)?;
                self.line(&format!("$J=G91G21{a}{distance_mm:.3}F{feed_mm_min:.0}"))
            }
            Command::JogStop => self.byte(JOG_CANCEL),
            Command::Home(axes) => {
                if axes.is_empty() {
                    self.line("$H")
                } else {
                    // GRBL 1.1 homes all the axes with one command
                    Err(Error::Unsupported("homing some of the axes"))
                }
            }
            Command::Pause => self.byte(HOLD),
            Command::Resume => self.byte(RESUME),
            Command::Stop => {
                // hold first so that the machine stops moving; the reset waits until it has (a reset while it still moves loses steps
                // and raises ALARM:3); when it does not say so in time the reset goes anyway and the position must be taken as lost
                self.byte(HOLD)?;
                self.wait_hold_done(Duration::from_secs(3));
                self.byte(RESET)
            }
            Command::EmergencyStop => self.byte(RESET),
            Command::Unlock => self.line("$X"),
            Command::ZeroWork(axes) => {
                let mut words = String::new();
                let axes = if axes.is_empty() { vec![0, 1, 2] } else { axes };
                for a in axes {
                    words.push_str(&format!("{}0", axis_name(a)?));
                }
                self.line(&format!("G10L20P1{words}"))
            }
            Command::Spindle { rpm, clockwise } => {
                if rpm <= 0.0 {
                    self.line("M5")
                } else {
                    self.line(&format!("{}S{rpm:.0}", if clockwise { "M3" } else { "M4" }))
                }
            }
            Command::Coolant(on) => self.line(if on { "M8" } else { "M9" }),
        }
    }

    fn max_travel(&mut self) -> Result<Option<[f64; 3]>, Error> {
        self.transport.write_all(b"$$\n").map_err(Self::link)?;
        let end = Instant::now() + self.timeouts.line;
        let mut found: [Option<f64>; 3] = [None; 3];
        loop {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(Error::Timeout);
            }
            match self.transport.read_line(left).map_err(Self::link)? {
                None => return Err(Error::Timeout),
                Some(l) => {
                    let l = l.trim();
                    if l == "ok" {
                        break;
                    }
                    if let Some(code) = l.strip_prefix("error:") {
                        return Err(Error::Refused(format!("error {code}")));
                    }
                    if let Some((n, v)) = parse_setting(l) {
                        if (130..=132).contains(&n) {
                            found[(n - 130) as usize] = Some(v);
                        }
                    }
                }
            }
        }
        Ok(match found {
            [Some(x), Some(y), Some(z)] if x > 0.0 && y > 0.0 && z > 0.0 => Some([x, y, z]),
            _ => None,
        })
    }

    fn send_line(&mut self, line: &str) -> Result<(), Error> {
        let clean = without_comments_and_realtime(line);
        let l = clean.trim();
        if l.is_empty() {
            return Ok(());
        }
        self.line(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockTransport;

    #[test]
    fn the_longest_travel_is_read_from_the_settings_of_the_controller() {
        let mock = MockTransport::with_replies(&[
            "$100=250.000",
            "$130=300.000",
            "$131=200.500",
            "$132=45.000",
            "ok",
        ]);
        let mut g = GrblController::new(mock);
        assert_eq!(g.max_travel(), Ok(Some([300.0, 200.5, 45.0])));
        assert_eq!(g.transport().text(), "$$\n");
    }

    #[test]
    fn a_controller_that_does_not_give_all_three_travels_or_does_not_answer_says_so() {
        // one of the three is missing, or it is not a positive number: nothing is claimed
        let mut g = GrblController::new(MockTransport::with_replies(&[
            "$130=300.000",
            "$131=200.000",
            "ok",
        ]));
        assert_eq!(g.max_travel(), Ok(None));
        let mut g = GrblController::new(MockTransport::with_replies(&[
            "$130=300.000",
            "$131=0.000",
            "$132=45.000",
            "ok",
        ]));
        assert_eq!(g.max_travel(), Ok(None));
        // silence is a timeout; a refusal is a refusal
        let mut g = GrblController::new(MockTransport::with_replies(&["$130=300.000"]));
        assert_eq!(g.max_travel(), Err(Error::Timeout));
        let mut g = GrblController::new(MockTransport::with_replies(&["error:3"]));
        assert!(matches!(g.max_travel(), Err(Error::Refused(_))));
        assert_eq!(parse_setting("$130=300.000"), Some((130, 300.0)));
        assert_eq!(parse_setting("<Idle>"), None);
        assert_eq!(parse_setting("$130=abc"), None);
    }

    #[test]
    fn a_status_report_is_read_with_the_work_offset() {
        let mut wco = [0.0; AXES];
        let s = parse_status(
            "<Idle|MPos:10.000,20.000,-1.500|FS:0,0|WCO:2.000,3.000,0.500>",
            &mut wco,
        )
        .unwrap();
        assert_eq!(s.state, State::Idle);
        assert_eq!(&s.machine_mm[..3], &[10.0, 20.0, -1.5]);
        assert_eq!(&s.work_mm[..3], &[8.0, 17.0, -2.0]);
        // a later report that does not bring the offset keeps the one known
        let s = parse_status(
            "<Run|MPos:11.000,20.000,-1.500|FS:600,12000|Bf:10,100|Pn:XZ>",
            &mut wco,
        )
        .unwrap();
        assert_eq!(s.state, State::Run);
        assert_eq!(s.work_mm[0], 9.0);
        assert_eq!((s.feed_mm_min, s.spindle_rpm), (600.0, 12000.0));
        assert_eq!(s.pins, "XZ");
        assert_eq!(s.buffer_percent, Some(33));
        // work position reports
        let s = parse_status("<Hold:0|WPos:1.000,2.000,3.000>", &mut wco).unwrap();
        assert_eq!(s.state, State::Hold);
        assert_eq!(&s.work_mm[..3], &[1.0, 2.0, 3.0]);
        assert_eq!(&s.machine_mm[..3], &[3.0, 5.0, 3.5]);
        // not a report
        assert!(parse_status("ok", &mut wco).is_none());
        assert!(parse_status("[MSG:Caution]", &mut wco).is_none());
        assert_eq!(
            parse_status("<Weird|MPos:0,0,0>", &mut wco).unwrap().state,
            State::Unknown
        );
    }

    #[test]
    fn the_status_is_asked_with_a_question_mark_and_skips_what_is_not_a_report() {
        let t =
            MockTransport::with_replies(&["[MSG:Hello]", "<Idle|MPos:0.000,0.000,0.000|FS:0,0>"]);
        let mut c = GrblController::new(t);
        assert_eq!(c.status().unwrap().state, State::Idle);
        assert_eq!(c.transport().text(), "?");
        let mut c =
            GrblController::new(MockTransport::default()).with_timeout(Duration::from_millis(1));
        assert_eq!(c.status(), Err(Error::Timeout));
    }

    #[test]
    fn the_orders_are_the_lines_and_the_bytes_of_the_protocol() {
        let t = MockTransport::with_replies(&["ok"; 12]);
        let mut c = GrblController::new(t);
        c.command(Command::Jog {
            axis: 0,
            distance_mm: -2.5,
            feed_mm_min: 300.0,
        })
        .unwrap();
        c.command(Command::Home(vec![])).unwrap();
        c.command(Command::Unlock).unwrap();
        c.command(Command::ZeroWork(vec![0, 1])).unwrap();
        c.command(Command::Spindle {
            rpm: 12000.0,
            clockwise: true,
        })
        .unwrap();
        c.command(Command::Spindle {
            rpm: 0.0,
            clockwise: true,
        })
        .unwrap();
        c.command(Command::Coolant(true)).unwrap();
        c.command(Command::Pause).unwrap();
        c.command(Command::Resume).unwrap();
        c.command(Command::JogStop).unwrap();
        c.command(Command::EmergencyStop).unwrap();
        assert_eq!(
            c.transport().text(),
            "$J=G91G21X-2.500F300\n$H\n$X\nG10L20P1X0Y0\nM3S12000\nM5\nM8\n!~\u{85}\u{18}"
        );
        // Stop holds and then resets
        let mut c = GrblController::new(MockTransport::default());
        c.command(Command::Stop).unwrap();
        assert_eq!(c.transport().written, vec![b'!', b'?', 0x18]);
    }

    #[test]
    fn a_line_keeps_waiting_while_the_machine_is_working() {
        let quick = Timeouts {
            line: Duration::from_millis(1),
            homing: Duration::from_millis(1),
            jog: Duration::from_millis(1),
        };
        // no answer within the limit, the machine says it runs, then the ok comes
        struct Script(std::collections::VecDeque<Option<&'static str>>, Vec<u8>);
        impl LineTransport for Script {
            fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
                self.1.extend_from_slice(bytes);
                Ok(())
            }
            fn read_line(&mut self, _timeout: Duration) -> io::Result<Option<String>> {
                Ok(self.0.pop_front().flatten().map(String::from))
            }
        }
        let t = Script(
            [None, Some("<Run|MPos:0.000,0.000,0.000>"), Some("ok")]
                .into_iter()
                .collect(),
            Vec::new(),
        );
        let mut c = GrblController::new(t).with_timeouts(quick);
        assert_eq!(c.send_line("G1 X100 F60"), Ok(()));
        assert!(
            !c.transport().1.contains(&HOLD),
            "the machine was working: no feed hold"
        );
    }

    #[test]
    fn comments_and_real_time_characters_do_not_reach_the_controller() {
        let mut c = GrblController::new(MockTransport::with_replies(&["ok", "ok", "ok"]));
        c.send_line("G1 X1 (tool change!) F100 ; what?").unwrap();
        c.send_line("(only a comment!)").unwrap();
        c.send_line("M3 S1000 ?!~é").unwrap();
        assert_eq!(c.transport().text(), "G1 X1  F100\nM3 S1000\n");
    }

    #[test]
    fn stop_resets_only_after_the_hold_is_complete() {
        let t = MockTransport::with_replies(&[
            "<Hold:1|MPos:0.000,0.000,0.000>",
            "<Hold:0|MPos:0.000,0.000,0.000>",
        ]);
        let mut c = GrblController::new(t);
        c.command(Command::Stop).unwrap();
        assert_eq!(c.transport().written, vec![b'!', b'?', b'?', 0x18]);
    }

    #[test]
    fn a_line_waits_for_its_ok_and_an_error_or_an_alarm_is_said() {
        let t = MockTransport::with_replies(&["ok", "error:20", "ALARM:1", "[MSG:x]", "ok"]);
        let mut c = GrblController::new(t);
        assert_eq!(c.send_line("G0 X1"), Ok(()));
        assert_eq!(c.send_line("G99"), Err(Error::Refused("error 20".into())));
        assert!(matches!(c.send_line("G1 X5"), Err(Error::Alarm(_))));
        assert_eq!(c.send_line("   "), Ok(()));
        assert_eq!(c.send_line("G0 X0"), Ok(()));
        assert_eq!(c.transport().text(), "G0 X1\nG99\nG1 X5\nG0 X0\n");
    }

    #[test]
    fn each_kind_of_line_waits_its_own_time() {
        let mut c =
            GrblController::new(MockTransport::with_replies(&["ok"; 4])).with_timeouts(Timeouts {
                line: Duration::from_secs(2),
                homing: Duration::from_secs(90),
                jog: Duration::from_secs(30),
            });
        c.send_line("G0 X1").unwrap();
        c.send_line("$H").unwrap();
        c.send_line("$J=G91X5F100").unwrap();
        c.send_line("$G").unwrap();
        let secs: Vec<u64> = c.transport().timeouts.iter().map(|t| t.as_secs()).collect();
        // each wait is at most the time of its kind (a little less: the time already spent is taken off)
        assert!(secs[0] <= 2 && secs[1] <= 90 && secs[1] >= 89);
        assert!(secs[2] <= 30 && secs[2] >= 29 && secs[3] <= 2);
        assert_eq!(Timeouts::default().homing, Duration::from_secs(120));
    }

    #[test]
    fn a_line_that_does_not_end_stops_the_machine_by_its_kind() {
        let quick = Timeouts {
            line: Duration::from_millis(1),
            homing: Duration::from_millis(1),
            jog: Duration::from_millis(1),
        };
        let mut c = GrblController::new(MockTransport::default()).with_timeouts(quick);
        assert_eq!(c.send_line("$H"), Err(Error::Timeout));
        assert_eq!(c.transport().written.last(), Some(&0x18));
        let mut c = GrblController::new(MockTransport::default()).with_timeouts(quick);
        assert_eq!(c.send_line("$J=G91X5F100"), Err(Error::Timeout));
        assert_eq!(c.transport().written.last(), Some(&0x85));
        let mut c = GrblController::new(MockTransport::default()).with_timeouts(quick);
        assert_eq!(c.send_line("G1 X5 F100"), Err(Error::Timeout));
        assert_eq!(c.transport().written.last(), Some(&b'!'));
    }

    #[test]
    fn what_the_controller_cannot_do_is_said_and_the_family_can_be_another_of_the_same_protocol() {
        let mut c = GrblController::new(MockTransport::default()).as_family(Family::FluidNc);
        assert_eq!(c.family(), Family::FluidNc);
        assert_eq!(
            c.command(Command::Home(vec![0])),
            Err(Error::Unsupported("homing some of the axes"))
        );
        assert_eq!(
            c.command(Command::Jog {
                axis: 9,
                distance_mm: 1.0,
                feed_mm_min: 100.0
            }),
            Err(Error::Unsupported("that axis"))
        );
        let caps = c.capabilities();
        assert!(caps.gcode && caps.emergency_stop && !caps.tool_changer);
    }
}
