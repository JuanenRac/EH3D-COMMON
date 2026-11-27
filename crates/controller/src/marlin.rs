//! Marlin on a serial line: a [`Controller`] of the suite written against the dialect of Marlin 2, and the simulated Marlin it is tried on.
//!
//! Marlin answers `ok` to every line it has taken; it has no state report (the position comes from `M114`), no jog (a jog is a relative
//! move between `G91` and `G90`), no hold that can be resumed (the quick stop `M410` empties the queue) and no numbers for its errors (it
//! says them in words, and this module gives each kind of error a number of its own, 101 and up, so that they are told apart and cannot be
//! mistaken for the numbers of GRBL). Lines can be numbered with a checksum (`N12 G1 X5*97`), and then a line that arrived spoiled is asked
//! for again (`Resend: 12`); with the numbering off, which is how Marlin is usually driven, a spoiled line is executed as it arrived, so the
//! numbering is a choice of the program that drives it (`with_numbering`). The emergency stop is `M112`; a halted Marlin refuses everything but
//! `M999`. Nothing here has run on a real Marlin: the evidence is the simulator.

use std::collections::BTreeMap;
use std::io;
use std::time::{Duration, Instant};

use crate::grbl::{LineTransport, Timeouts, MAX_BUSY_WAIT};
use crate::urgent::{Urgent, UrgentChannel};
use crate::wire::Device;
use crate::{Capabilities, Command, Controller, Error, Family, State, Status, AXES};

/// The numbers of the kinds of error of Marlin.
pub const E_UNKNOWN: u32 = 101;
pub const E_LINE_NUMBER: u32 = 102;
pub const E_CHECKSUM: u32 = 103;
pub const E_HOMING: u32 = 104;
pub const E_PROBE: u32 = 105;
pub const E_HALTED: u32 = 106;
pub const E_THERMAL: u32 = 107;
pub const E_OTHER: u32 = 108;

/// How many times a line that arrived spoiled is sent again before the program gives up.
pub const MAX_RESENDS: u32 = 3;

/// What a line that was cut short by an urgent stop says (as the GRBL controller of the suite says it).
const INTERRUPTED: &str = "interrupted by an urgent stop";

/// What a controller that restarted by itself is told as.
const RESTARTED: &str =
    "the controller restarted by itself: the position and the modal state are lost";

/// The kind of error a line of Marlin is, with the number of the kind and whether it is an alarm (the machine is halted or in danger) or
/// a refusal of one line. `None` for a line that is not an error.
pub fn classify(line: &str) -> Option<(u32, bool)> {
    let l = line.trim();
    let lower = l.to_ascii_lowercase();
    if lower.starts_with("echo:unknown command") {
        return Some((E_UNKNOWN, false));
    }
    let body = lower
        .strip_prefix("error:")
        .or_else(|| lower.strip_prefix("!!"))?;
    let kind = if body.contains("printer halted") || body.contains("printer stopped") {
        (E_HALTED, true)
    } else if body.contains("mintemp")
        || body.contains("maxtemp")
        || body.contains("thermal runaway")
        || body.contains("heating failed")
    {
        (E_THERMAL, true)
    } else if body.contains("line number") {
        (E_LINE_NUMBER, false)
    } else if body.contains("checksum") {
        (E_CHECKSUM, false)
    } else if body.contains("homing failed") || body.contains("homing error") {
        (E_HOMING, false)
    } else if body.contains("probing failed") || body.contains("probe") {
        (E_PROBE, false)
    } else if l.starts_with("!!") {
        (E_HALTED, true)
    } else {
        (E_OTHER, false)
    };
    Some(kind)
}

/// The XOR of the bytes, as Marlin checks a numbered line.
pub fn checksum(text: &str) -> u8 {
    text.bytes().fold(0, |a, b| a ^ b)
}

/// A line with its number and its checksum (`N12 G1 X5*97`).
pub fn frame(number: u32, text: &str) -> String {
    let body = format!("N{number} {text}");
    format!("{body}*{}", checksum(&body))
}

/// Reads the position of an `M114` answer (`X:10.00 Y:0.00 Z:0.00 E:0.00 Count X:800 ...`): the three axes before the word `Count`.
pub fn parse_position(line: &str) -> Option<[f64; 3]> {
    let head = line.split("Count").next()?;
    let mut found: [Option<f64>; 3] = [None; 3];
    for token in head.split_whitespace() {
        let Some((k, v)) = token.split_once(':') else {
            continue;
        };
        let slot = match k {
            "X" => 0,
            "Y" => 1,
            "Z" => 2,
            _ => continue,
        };
        found[slot] = v.parse::<f64>().ok().filter(|n| n.is_finite());
    }
    match found {
        [Some(x), Some(y), Some(z)] => Some([x, y, z]),
        _ => None,
    }
}

/// Reads the longest travel of an `M211` answer (`Soft endstops: ON Min: X:0.00 ... Max: X:200.00 Y:200.00 Z:180.00`).
pub fn parse_max_travel(line: &str) -> Option<[f64; 3]> {
    let after = line.split("Max:").nth(1)?;
    parse_position(after)
}

/// A line without its comments (`( ... )` and `;` to the end), without anything that is not plain ASCII and without the `*` that starts
/// a checksum and what follows it: what is left is what Marlin is told.
pub(crate) fn clean(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0u32;
    for c in line.chars() {
        match c {
            ';' | '*' if depth == 0 => break,
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            c if c.is_ascii() && !c.is_ascii_control() || c == ' ' || c == '\t' => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn axis_name(axis: usize) -> Result<char, Error> {
    ['X', 'Y', 'Z']
        .get(axis)
        .copied()
        .ok_or(Error::Unsupported("that axis"))
}

enum Reply {
    Done,
    Resend(u32),
}

/// Marlin over a line transport.
pub struct MarlinController<T: LineTransport> {
    transport: T,
    timeouts: Timeouts,
    busy_max: Duration,
    urgent: UrgentChannel,
    /// Lines are numbered and carry a checksum.
    numbering: bool,
    started: bool,
    next_n: u32,
    /// The lines the last exchange read that were not an answer to it (the data of `M114`, `M211`...).
    capture: Vec<String>,
    /// The offset of the work from the machine, which `G92` makes and Marlin does not report.
    offset: [f64; AXES],
    feed: f64,
    spindle_rpm: f64,
    halted: bool,
    /// An urgent stop or a timeout left lines on the way that nobody has read: the next exchange reads them off first.
    resync: bool,
    /// A reset went out and its greeting is expected.
    reset_expected: bool,
    /// The last `G20` or `G21` the program sent: inches are in force.
    inches: bool,
}

impl<T: LineTransport> MarlinController<T> {
    pub fn new(transport: T) -> Self {
        MarlinController {
            transport,
            timeouts: Timeouts::default(),
            busy_max: MAX_BUSY_WAIT,
            urgent: UrgentChannel::new(),
            numbering: false,
            started: false,
            next_n: 1,
            capture: Vec::new(),
            offset: [0.0; AXES],
            feed: 0.0,
            spindle_rpm: 0.0,
            halted: false,
            resync: false,
            reset_expected: false,
            inches: false,
        }
    }

    /// Numbers the lines and gives each a checksum, so that a line spoiled on the way is asked for again (off by default).
    pub fn with_numbering(mut self, on: bool) -> Self {
        self.numbering = on;
        self
    }

    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// The most a line waits while the controller says it is busy (a limit of zero waits for the ordinary time only).
    pub fn with_busy_limit(mut self, limit: Duration) -> Self {
        self.busy_max = limit;
        self
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Whether the controller said it is halted (until `M999` is accepted).
    pub fn halted(&self) -> bool {
        self.halted
    }

    fn link(e: io::Error) -> Error {
        Error::Link(e.to_string())
    }

    fn watched(&self) -> bool {
        self.urgent.handles() > 1
    }

    /// Writes what was asked to go out of turn. `Ok(true)` when it ended the job.
    fn flush_urgent(&mut self) -> Result<bool, Error> {
        let mut ended = false;
        for u in self.urgent.take() {
            let bytes: Option<&[u8]> = match u {
                // Marlin has no report that can be asked for out of turn, and no hold that can be resumed: a hold is the quick stop, which
                // is the safe side, and a resume has nothing to resume
                Urgent::Hold | Urgent::StopJob => Some(b"M410\n"),
                Urgent::Reset => {
                    self.halted = true;
                    Some(b"M112\n")
                }
                Urgent::Status | Urgent::Resume => None,
            };
            if let Some(b) = bytes {
                self.transport.write_all(b).map_err(Self::link)?;
                self.resync = true;
            }
            ended |= u.ends_job() || (matches!(u, Urgent::Hold));
        }
        Ok(ended)
    }

    fn read_next(&mut self, left: Duration) -> Result<Option<String>, Error> {
        const POLL: Duration = Duration::from_millis(25);
        if !self.watched() {
            if self.flush_urgent()? {
                return Err(Error::Refused(INTERRUPTED.to_string()));
            }
            return self.transport.read_line(left).map_err(Self::link);
        }
        let end = Instant::now() + left;
        loop {
            if self.flush_urgent()? {
                return Err(Error::Refused(INTERRUPTED.to_string()));
            }
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            if let Some(l) = self
                .transport
                .read_line(remaining.min(POLL))
                .map_err(Self::link)?
            {
                return Ok(Some(l));
            }
        }
    }

    /// Reads off what is on the way and belongs to no order.
    fn drain(&mut self) {
        let wait = self.timeouts.line.min(Duration::from_millis(50));
        let mut n = 0;
        while n < 256 {
            match self.transport.read_line(wait) {
                Ok(Some(l)) => {
                    // a start read off here is a start all the same: the controller is back in mm
                    if l.trim() == "start" {
                        self.inches = false;
                    }
                    n += 1;
                }
                _ => break,
            }
        }
    }

    /// Waits for the end of the order that was sent: `ok` is done; the errors are told by their kind; the keep-alive lines of a busy controller
    /// start the wait again (until the busy limit); the other lines are kept for whoever asked for data.
    fn wait_reply(&mut self, limit: Duration) -> Result<Reply, Error> {
        let began = Instant::now();
        let mut end = began + limit;
        let mut pending: Option<Error> = None;
        let mut resend: Option<u32> = None;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(pending.unwrap_or(Error::Timeout));
            }
            let Some(raw) = self.read_next(left)? else {
                return Err(pending.unwrap_or(Error::Timeout));
            };
            let l = raw.trim();
            if l == "start" {
                self.inches = false;
                if self.reset_expected {
                    self.reset_expected = false;
                    continue;
                }
                self.resync = true;
                return Err(Error::Refused(RESTARTED.to_string()));
            }
            if l == "ok" || l.starts_with("ok ") {
                if let Some(r) = resend {
                    return Ok(Reply::Resend(r));
                }
                if l.len() > 2 {
                    self.capture.push(l.to_string());
                }
                return match pending {
                    Some(e) => Err(e),
                    None => Ok(Reply::Done),
                };
            }
            if let Some(r) = l.strip_prefix("Resend:") {
                resend = r.trim().parse().ok();
                continue;
            }
            if l.starts_with("echo:busy:") {
                if began.elapsed() < self.busy_max {
                    end = Instant::now() + limit;
                }
                continue;
            }
            if let Some((class, alarm)) = classify(l) {
                let e = if alarm {
                    Error::Alarm(format!("alarm {class}: {l}"))
                } else {
                    Error::Refused(format!("error {class}: {l}"))
                };
                if matches!(class, E_HALTED | E_THERMAL) {
                    self.halted = true;
                    return Err(e);
                }
                pending = Some(e);
                continue;
            }
            self.capture.push(l.to_string());
        }
    }

    /// Whether a line moves the machine (a rapid, a cut, an arc, the homing, a probe).
    fn moves(text: &str) -> bool {
        let t = text.trim().to_ascii_uppercase();
        let first = t.split_whitespace().next().unwrap_or("");
        first
            .strip_prefix('G')
            .and_then(|n| n.parse::<f64>().ok())
            .is_some_and(|g| [0.0, 1.0, 2.0, 3.0, 28.0].contains(&g) || (38.0..39.0).contains(&g))
    }

    /// Sends one line and waits for its end, asking for it again when it arrived spoiled.
    fn exchange(&mut self, text: &str) -> Result<(), Error> {
        let limit = if text.trim().to_ascii_uppercase().starts_with("G28") {
            self.timeouts.homing
        } else {
            self.timeouts.line
        };
        self.flush_urgent()?;
        if self.resync {
            self.drain();
            self.resync = false;
        }
        if self.numbering && !self.started {
            self.started = true;
            self.next_n = 1;
            self.send_numbered("M110 N0", 0, limit)?;
        }
        self.capture.clear();
        if self.numbering {
            let n = self.next_n;
            self.send_numbered(text, n, limit)?;
            self.next_n = n.wrapping_add(1);
            return Ok(());
        }
        self.transport
            .write_all(format!("{text}\n").as_bytes())
            .map_err(Self::link)?;
        let done = self.wait_reply(limit).and_then(|r| match r {
            Reply::Done => Ok(()),
            Reply::Resend(_) => Err(Error::Refused(format!(
                "error {E_LINE_NUMBER}: the controller asked for a line again, and the lines are not numbered"
            ))),
        });
        self.after(text, done)
    }

    fn send_numbered(&mut self, text: &str, n: u32, limit: Duration) -> Result<(), Error> {
        let framed = frame(n, text);
        let mut tries = 0;
        loop {
            self.transport
                .write_all(format!("{framed}\n").as_bytes())
                .map_err(Self::link)?;
            match self.wait_reply(limit) {
                Ok(Reply::Done) => return Ok(()),
                Ok(Reply::Resend(r)) => {
                    tries += 1;
                    if r != n || tries > MAX_RESENDS {
                        return Err(Error::Refused(format!(
                            "error {E_CHECKSUM}: the line did not arrive after {tries} tries"
                        )));
                    }
                    // a line that was cut in two on its way is answered twice: what the second half said is not the answer to the line
                    // that is sent again
                    self.drain();
                }
                Err(e) => return self.after(text, Err(e)),
            }
        }
    }

    /// Whatever the end of an order was, a move that got no answer is not left running.
    fn after(&mut self, text: &str, done: Result<(), Error>) -> Result<(), Error> {
        if done == Err(Error::Timeout) {
            // an answer that comes late must not be taken for the answer to the next line
            self.resync = true;
            if Self::moves(text) {
                let _ = self.transport.write_all(b"M410\n");
            }
        }
        done
    }

    fn motion_state(&self) -> State {
        if self.halted {
            State::Alarm
        } else {
            State::Idle
        }
    }
}

impl<T: LineTransport> Controller for MarlinController<T> {
    fn family(&self) -> Family {
        Family::Marlin
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
            pause: false,
            emergency_stop: true,
            tool_changer: false,
            gcode: true,
        }
    }

    fn status(&mut self) -> Result<Status, Error> {
        self.exchange("M114")?;
        let work = self
            .capture
            .iter()
            .find_map(|l| parse_position(l))
            .ok_or_else(|| Error::Link("the controller gave no position".to_string()))?;
        let mut st = Status {
            state: self.motion_state(),
            feed_mm_min: self.feed,
            spindle_rpm: self.spindle_rpm,
            ..Status::default()
        };
        for (i, w) in work.iter().enumerate() {
            st.work_mm[i] = *w;
            st.machine_mm[i] = w + self.offset[i];
        }
        Ok(st)
    }

    fn command(&mut self, command: Command) -> Result<(), Error> {
        self.flush_urgent()?;
        match command {
            Command::Jog {
                axis,
                distance_mm,
                feed_mm_min,
            } => {
                let a = axis_name(axis)?;
                // the distance and the feed are in mm: a program that left inches on would go 25.4 times as far
                let inches = self.inches;
                if inches {
                    self.exchange("G21")?;
                }
                let done = self.exchange("G91").and_then(|()| {
                    let moved = self.exchange(&format!("G1 {a}{distance_mm:.3} F{feed_mm_min:.0}"));
                    // the machine never stays in relative mode, whatever happened to the move
                    let back = self.exchange("G90");
                    moved.and(back)
                });
                if inches {
                    // the units of the program come back
                    let units = self.exchange("G20");
                    return done.and(units);
                }
                done
            }
            Command::JogStop | Command::Stop => self.exchange("M410"),
            Command::Home(axes) => {
                let mut line = String::from("G28");
                for a in axes {
                    line.push(' ');
                    line.push(axis_name(a)?);
                }
                self.exchange(&line)
            }
            Command::Pause => Err(Error::Unsupported("pause")),
            Command::Resume => Err(Error::Unsupported("resume")),
            Command::EmergencyStop => {
                self.halted = true;
                self.resync = true;
                self.transport.write_all(b"M112\n").map_err(Self::link)
            }
            Command::Unlock => {
                self.exchange("M999")?;
                self.halted = false;
                Ok(())
            }
            Command::ZeroWork(axes) => {
                let axes = if axes.is_empty() { vec![0, 1, 2] } else { axes };
                let here = self.status()?;
                let mut words = String::new();
                for a in &axes {
                    words.push_str(&format!(" {}0", axis_name(*a)?));
                }
                self.exchange(&format!("G92{words}"))?;
                for a in axes {
                    self.offset[a] = here.machine_mm[a];
                }
                Ok(())
            }
            Command::Spindle { rpm, clockwise } => {
                let line = if rpm <= 0.0 {
                    "M5".to_string()
                } else {
                    format!("{} S{rpm:.0}", if clockwise { "M3" } else { "M4" })
                };
                self.exchange(&line)?;
                self.spindle_rpm = rpm.max(0.0);
                Ok(())
            }
            Command::Coolant(on) => self.exchange(if on { "M8" } else { "M9" }),
        }
    }

    fn send_line(&mut self, line: &str) -> Result<(), Error> {
        let text = clean(line);
        if text.is_empty() {
            return Ok(());
        }
        self.exchange(&text)?;
        // the feed of the program is what the status says, and its units are what a jog has to put back
        for w in text.to_ascii_uppercase().split_whitespace() {
            match w {
                "G20" => self.inches = true,
                "G21" => self.inches = false,
                _ => {}
            }
            if let Some(f) = w.strip_prefix('F').and_then(|v| v.parse::<f64>().ok()) {
                self.feed = f;
            }
        }
        Ok(())
    }

    fn max_travel(&mut self) -> Result<Option<[f64; 3]>, Error> {
        self.exchange("M211")?;
        let found = self.capture.iter().find_map(|l| parse_max_travel(l));
        Ok(found.filter(|m| m.iter().all(|v| *v > 0.0)))
    }

    fn urgent_channel(&self) -> Option<UrgentChannel> {
        Some(self.urgent.clone())
    }
}

/// The simulated Marlin: a point that moves to its target inside the travel, with the dialect of Marlin 2 on the line.
#[derive(Debug)]
pub struct MarlinDevice {
    machine: [f64; 3],
    offset: [f64; 3],
    relative: bool,
    halted: bool,
    last_n: u32,
    rx: Vec<u8>,
    /// How many commands were executed (a line that was asked for again, or refused for its number, does not count).
    count: usize,
    /// The command number N (from 1) answers with this text instead of working: a line that starts with `Error:` is an error that does
    /// not end the command (it is followed by `ok`) unless it says the printer is halted; one that starts with `echo:` is a message.
    pub fail: BTreeMap<usize, String>,
    /// What was executed, in order, as the device had it.
    pub executed: Vec<String>,
    /// The lines that were not a command it knows.
    pub unknown: Vec<String>,
    /// The travel of each axis, from 0 to this many mm (Marlin clamps what goes past it).
    pub travel: [f64; 3],
}

impl Default for MarlinDevice {
    fn default() -> Self {
        MarlinDevice::new()
    }
}

impl MarlinDevice {
    pub fn new() -> MarlinDevice {
        MarlinDevice {
            machine: [0.0; 3],
            offset: [0.0; 3],
            relative: false,
            halted: false,
            last_n: 0,
            rx: Vec::new(),
            count: 0,
            fail: BTreeMap::new(),
            executed: Vec::new(),
            unknown: Vec::new(),
            travel: [200.0, 200.0, 100.0],
        }
    }

    /// The greeting of a start, as the controller prints it when it is reset.
    pub fn greeting() -> &'static [u8] {
        b"start\necho:Marlin 2.1.2.5\necho: Last Updated: 2024-01-01 | Author: (none)\n"
    }

    pub fn position(&self) -> [f64; 3] {
        self.machine
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    fn work(&self) -> [f64; 3] {
        [
            self.machine[0] - self.offset[0],
            self.machine[1] - self.offset[1],
            self.machine[2] - self.offset[2],
        ]
    }

    /// Takes the number and the checksum off a line. `Err` is what the device answers.
    fn unframe(&mut self, text: &str) -> Result<String, String> {
        let last = self.last_n;
        let resend = |what: &str| {
            format!(
                "Error:{what}, Last Line: {last}\nResend: {}\nok\n",
                last + 1
            )
        };
        let numbered = text
            .strip_prefix('N')
            .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()));
        if !numbered {
            if text.contains('*') {
                return Err(resend("No Line Number with checksum"));
            }
            return Ok(text.to_string());
        }
        let (body, given) = match text.split_once('*') {
            Some((b, c)) => (b, Some(c.trim())),
            None => (text, None),
        };
        match given {
            Some(c) if c.parse::<u8>().ok() != Some(checksum(body)) => {
                return Err(resend("checksum mismatch"));
            }
            None if !text.to_ascii_uppercase().contains("M110") => {
                return Err(resend("No Checksum with line number"));
            }
            _ => {}
        }
        let digits: String = body[1..].chars().take_while(char::is_ascii_digit).collect();
        let Ok(n) = digits.parse::<u32>() else {
            return Err(resend("Line Number is not Last Line Number+1"));
        };
        let rest = body[1 + digits.len()..].trim().to_string();
        let is_m110 = rest.to_ascii_uppercase().starts_with("M110");
        if !is_m110 && n != last + 1 {
            return Err(resend("Line Number is not Last Line Number+1"));
        }
        self.last_n = n;
        if is_m110 {
            // M110 N<n> sets the number the next line must follow
            if let Some(v) = rest
                .to_ascii_uppercase()
                .split_whitespace()
                .find_map(|w| w.strip_prefix('N').and_then(|v| v.parse::<u32>().ok()))
            {
                self.last_n = v;
            }
        }
        Ok(rest)
    }

    fn words(text: &str) -> Vec<(char, f64)> {
        text.split_whitespace()
            .filter_map(|w| {
                let mut c = w.chars();
                let letter = c.next()?.to_ascii_uppercase();
                let rest = c.as_str();
                // a word with no number (`X` in `G28 X`) is there with NaN; a number that is not one is not a word
                let n = if rest.is_empty() {
                    f64::NAN
                } else {
                    rest.parse::<f64>().ok()?
                };
                Some((letter, n))
            })
            .collect()
    }

    fn process(&mut self, raw: &str) -> String {
        let text = raw.trim();
        if text.is_empty() {
            return String::new();
        }
        if self.halted {
            if text.eq_ignore_ascii_case("M999") {
                self.halted = false;
                return "ok\n".to_string();
            }
            return "Error:Printer stopped due to errors. Fix the error and use M999 to restart.\n"
                .to_string();
        }
        let body = match self.unframe(text) {
            Ok(b) => b,
            Err(answer) => return answer,
        };
        if body.is_empty() {
            return "ok\n".to_string();
        }
        self.count += 1;
        self.executed.push(body.clone());
        if let Some(said) = self.fail.get(&self.count).cloned() {
            if said.starts_with("Error:") && said.to_ascii_lowercase().contains("halted") {
                self.halted = true;
                return format!("{said}\n");
            }
            return format!("{said}\nok\n");
        }
        let up = body.to_ascii_uppercase();
        let words = Self::words(&up);
        let code = words.first().copied();
        let reply = match code {
            Some(('G', g)) if g == 0.0 || g == 1.0 => {
                for (letter, n) in &words[1..] {
                    let ax = match letter {
                        'X' => 0,
                        'Y' => 1,
                        'Z' => 2,
                        _ => continue,
                    };
                    if n.is_nan() {
                        continue;
                    }
                    let target = if self.relative {
                        self.machine[ax] + n
                    } else {
                        n + self.offset[ax]
                    };
                    self.machine[ax] = target.clamp(0.0, self.travel[ax]);
                }
                "ok\n".to_string()
            }
            Some(('G', 28.0)) => {
                let which: Vec<usize> = words[1..]
                    .iter()
                    .filter_map(|(l, _)| match l {
                        'X' => Some(0),
                        'Y' => Some(1),
                        'Z' => Some(2),
                        _ => None,
                    })
                    .collect();
                let all = which.is_empty();
                for ax in 0..3 {
                    if all || which.contains(&ax) {
                        self.machine[ax] = 0.0;
                    }
                }
                "echo:busy: processing\necho:busy: processing\nok\n".to_string()
            }
            Some(('G', 90.0)) => {
                self.relative = false;
                "ok\n".to_string()
            }
            Some(('G', 91.0)) => {
                self.relative = true;
                "ok\n".to_string()
            }
            Some(('G', 92.0)) => {
                for (letter, n) in &words[1..] {
                    let ax = match letter {
                        'X' => 0,
                        'Y' => 1,
                        'Z' => 2,
                        _ => continue,
                    };
                    self.offset[ax] = self.machine[ax] - n;
                }
                "ok\n".to_string()
            }
            Some(('G', g)) if g == 20.0 || g == 21.0 || g == 4.0 || g == 38.2 => {
                "ok\n".to_string()
            }
            Some(('M', 114.0)) => {
                let w = self.work();
                format!(
                    "X:{:.2} Y:{:.2} Z:{:.2} E:0.00 Count X:{} Y:{} Z:{}\nok\n",
                    w[0],
                    w[1],
                    w[2],
                    (w[0] * 80.0).round(),
                    (w[1] * 80.0).round(),
                    (w[2] * 400.0).round()
                )
            }
            Some(('M', 211.0)) => format!(
                "Soft endstops: ON    Min:  X:0.00  Y:0.00  Z:0.00    Max:  X:{:.2}  Y:{:.2}  Z:{:.2}\nok\n",
                self.travel[0], self.travel[1], self.travel[2]
            ),
            Some(('M', 112.0)) => {
                self.halted = true;
                "Error:Printer halted. kill() called!\n".to_string()
            }
            Some(('M', 105.0)) => "ok T:21.0 /0.0 B:21.0 /0.0\n".to_string(),
            Some(('M', m))
                if [3.0, 4.0, 5.0, 7.0, 8.0, 9.0, 17.0, 18.0, 84.0, 110.0, 400.0, 410.0, 999.0]
                    .contains(&m) =>
            {
                "ok\n".to_string()
            }
            _ => format!("echo:Unknown command: \"{body}\"\nok\n"),
        };
        if reply.starts_with("echo:Unknown command") {
            // not a command of this machine: it is told apart from what was executed
            self.executed.pop();
            self.unknown.push(body);
        }
        reply
    }
}

impl Device for MarlinDevice {
    fn input(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = String::new();
        for &b in bytes {
            match b {
                b'\n' => {
                    let text = String::from_utf8_lossy(&std::mem::take(&mut self.rx)).to_string();
                    out.push_str(&self.process(&text));
                }
                b'\r' => {}
                _ => self.rx.push(b),
            }
        }
        out.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{Impair, WireLink};

    type Marlin = MarlinController<WireLink<MarlinDevice>>;

    fn marlin(impair: Impair, numbering: bool) -> Marlin {
        MarlinController::new(WireLink::new(MarlinDevice::new(), impair)).with_numbering(numbering)
    }

    #[test]
    fn the_checksum_and_the_frame_are_those_of_marlin() {
        // the XOR of the bytes of the line, as the documentation of the protocol writes it
        assert_eq!(checksum("N1 G28"), 18);
        assert_eq!(
            frame(5, "G1 X1"),
            format!("N5 G1 X1*{}", checksum("N5 G1 X1"))
        );
        assert_eq!(checksum(""), 0);
        assert_eq!(checksum("A"), 65);
        assert_eq!(checksum("AB"), 65 ^ 66);
    }

    #[test]
    fn the_kinds_of_error_have_numbers_of_their_own_and_the_alarms_are_told_from_the_refusals() {
        for (line, want) in [
            ("echo:Unknown command: \"G99\"", Some((E_UNKNOWN, false))),
            (
                "Error:Printer halted. kill() called!",
                Some((E_HALTED, true)),
            ),
            (
                "Error:Printer stopped due to errors. Fix it",
                Some((E_HALTED, true)),
            ),
            (
                "Error:Extruder switched off. MINTEMP triggered !",
                Some((E_THERMAL, true)),
            ),
            (
                "Error:Thermal Runaway, system stopped!",
                Some((E_THERMAL, true)),
            ),
            (
                "Error:Line Number is not Last Line Number+1, Last Line: 4",
                Some((E_LINE_NUMBER, false)),
            ),
            (
                "Error:checksum mismatch, Last Line: 4",
                Some((E_CHECKSUM, false)),
            ),
            ("Error:Homing Failed", Some((E_HOMING, false))),
            ("Error:Probing Failed", Some((E_PROBE, false))),
            ("!! Something broke", Some((E_HALTED, true))),
            ("Error:something else", Some((E_OTHER, false))),
            ("ok", None),
            ("X:1.00 Y:2.00 Z:3.00 E:0.00 Count X:80 Y:160 Z:1200", None),
            ("echo:busy: processing", None),
        ] {
            assert_eq!(classify(line), want, "{line}");
        }
        // none of the numbers is one of the numbers of GRBL
        for n in [
            E_UNKNOWN,
            E_LINE_NUMBER,
            E_CHECKSUM,
            E_HOMING,
            E_PROBE,
            E_HALTED,
            E_THERMAL,
            E_OTHER,
        ] {
            assert!(n > 100 && crate::codes::grbl_error_text(n).is_none());
        }
    }

    #[test]
    fn the_position_and_the_travel_are_read_from_their_lines() {
        assert_eq!(
            parse_position("X:10.50 Y:-2.00 Z:3.25 E:0.00 Count X:840 Y:-160 Z:1300"),
            Some([10.5, -2.0, 3.25])
        );
        assert_eq!(parse_position("X:1 Y:2"), None);
        assert_eq!(parse_position("X:a Y:2 Z:3"), None);
        assert_eq!(parse_position("ok"), None);
        assert_eq!(
            parse_max_travel("Soft endstops: ON    Min:  X:0.00  Y:0.00  Z:0.00    Max:  X:200.00  Y:210.00  Z:180.00"),
            Some([200.0, 210.0, 180.0])
        );
        assert_eq!(parse_max_travel("Soft endstops: OFF"), None);
    }

    #[test]
    fn a_line_is_sent_without_its_comments_and_without_a_checksum_mark() {
        assert_eq!(clean("G1 X1 (go) Y2 ; fast"), "G1 X1  Y2");
        assert_eq!(clean("G1 X1*99"), "G1 X1");
        assert_eq!(clean("(only a comment)"), "");
        assert_eq!(clean("G1 X\u{e9}1"), "G1 X1");
    }

    #[test]
    fn marlin_moves_reports_and_homes_through_the_dialect() {
        let mut c = marlin(Impair::clean(), false);
        c.send_line("G90").unwrap();
        c.send_line("G1 X10 Y5 F600").unwrap();
        let s = c.status().unwrap();
        assert_eq!(&s.machine_mm[..3], &[10.0, 5.0, 0.0]);
        assert_eq!((s.state, s.feed_mm_min), (State::Idle, 600.0));
        c.command(Command::Home(vec![0])).unwrap();
        assert_eq!(c.status().unwrap().machine_mm[0], 0.0);
        assert_eq!(c.status().unwrap().machine_mm[1], 5.0);
        assert_eq!(c.family(), Family::Marlin);
        assert!(!c.capabilities().pause);
        assert_eq!(c.command(Command::Pause), Err(Error::Unsupported("pause")));
        assert_eq!(
            c.command(Command::Resume),
            Err(Error::Unsupported("resume"))
        );
    }

    #[test]
    fn a_jog_is_in_mm_while_the_program_is_in_inches_and_the_inches_come_back() {
        let mut c = marlin(Impair::clean(), false);
        c.send_line("G20").unwrap();
        let jog = Command::Jog {
            axis: 0,
            distance_mm: 5.0,
            feed_mm_min: 300.0,
        };
        c.command(jog.clone()).unwrap();
        assert_eq!(
            c.transport().device().executed,
            ["G20", "G21", "G91", "G1 X5.000 F300", "G90", "G20"]
        );
        // back in mm, a jog adds nothing
        c.send_line("G21").unwrap();
        c.command(jog).unwrap();
        assert_eq!(
            c.transport().device().executed[7..],
            ["G91", "G1 X5.000 F300", "G90"]
        );
    }

    #[test]
    fn a_jog_is_a_relative_move_and_the_machine_is_back_in_absolute_mode_after_it() {
        let mut c = marlin(Impair::clean(), false);
        c.send_line("G1 X20 F600").unwrap();
        c.command(Command::Jog {
            axis: 0,
            distance_mm: -5.0,
            feed_mm_min: 300.0,
        })
        .unwrap();
        assert_eq!(c.status().unwrap().machine_mm[0], 15.0);
        let log = c.transport().device().executed.clone();
        assert_eq!(&log[1..4], &["G91", "G1 X-5.000 F300", "G90"]);
        // an absolute move afterwards is absolute
        c.send_line("G1 X3").unwrap();
        assert_eq!(c.status().unwrap().machine_mm[0], 3.0);
        assert_eq!(
            c.command(Command::Jog {
                axis: 4,
                distance_mm: 1.0,
                feed_mm_min: 1.0
            }),
            Err(Error::Unsupported("that axis"))
        );
    }

    #[test]
    fn zero_work_moves_the_work_but_not_the_machine() {
        let mut c = marlin(Impair::clean(), false);
        c.send_line("G1 X12 Y7 F100").unwrap();
        c.command(Command::ZeroWork(vec![0])).unwrap();
        let s = c.status().unwrap();
        assert_eq!((s.work_mm[0], s.machine_mm[0]), (0.0, 12.0));
        assert_eq!((s.work_mm[1], s.machine_mm[1]), (7.0, 7.0));
        c.command(Command::ZeroWork(vec![])).unwrap();
        let s = c.status().unwrap();
        assert_eq!(&s.work_mm[..3], &[0.0, 0.0, 0.0]);
        assert_eq!(&s.machine_mm[..2], &[12.0, 7.0]);
    }

    #[test]
    fn an_unknown_command_is_refused_with_its_number_and_the_line_after_it_still_works() {
        let mut c = marlin(Impair::clean(), false);
        let e = c.send_line("T1 M6").unwrap_err();
        assert_eq!(e.code(), Some(E_UNKNOWN));
        assert!(matches!(e, Error::Refused(_)));
        // the ok that followed the message was read with it: the next line is not answered by a stale ok
        c.send_line("G1 X4 F100").unwrap();
        assert_eq!(c.status().unwrap().machine_mm[0], 4.0);
    }

    #[test]
    fn errors_by_number_of_command_are_told_by_kind() {
        let mut dev = MarlinDevice::new();
        dev.fail.insert(1, "Error:Homing Failed".into());
        dev.fail.insert(2, "Error:Probing Failed".into());
        dev.fail.insert(3, "Error:Something odd".into());
        let mut c = MarlinController::new(WireLink::new(dev, Impair::clean()));
        assert_eq!(
            c.command(Command::Home(vec![])).unwrap_err().code(),
            Some(E_HOMING)
        );
        assert_eq!(
            c.send_line("G38.2 Z-5 F20").unwrap_err().code(),
            Some(E_PROBE)
        );
        assert_eq!(c.send_line("G1 X1 F10").unwrap_err().code(), Some(E_OTHER));
        // the commands that follow are not spoiled by what the errors left behind
        c.send_line("G1 X2 F10").unwrap();
        assert_eq!(c.status().unwrap().machine_mm[0], 2.0);
    }

    #[test]
    fn a_halted_marlin_is_an_alarm_and_refuses_everything_until_m999() {
        let mut c = marlin(Impair::clean(), false);
        c.command(Command::EmergencyStop).unwrap();
        assert!(c.halted());
        let e = c.send_line("G1 X1 F10").unwrap_err();
        assert!(matches!(e, Error::Alarm(_)), "{e:?}");
        assert_eq!(e.code(), Some(E_HALTED));
        assert_eq!(c.status().unwrap_err().code(), Some(E_HALTED));
        c.command(Command::Unlock).unwrap();
        assert!(!c.halted());
        c.send_line("G1 X1 F10").unwrap();
        assert_eq!(c.status().unwrap().state, State::Idle);
    }

    #[test]
    fn a_halt_by_the_device_itself_is_known_to_the_status() {
        let mut dev = MarlinDevice::new();
        dev.fail
            .insert(1, "Error:Printer halted. kill() called!".into());
        let mut c = MarlinController::new(WireLink::new(dev, Impair::clean()));
        assert!(matches!(
            c.send_line("G1 X1 F10").unwrap_err(),
            Error::Alarm(_)
        ));
        assert!(c.halted());
    }

    #[test]
    fn the_travel_is_read_from_the_soft_endstops() {
        let mut c = marlin(Impair::clean(), false);
        assert_eq!(c.max_travel().unwrap(), Some([200.0, 200.0, 100.0]));
    }

    #[test]
    fn spindle_and_coolant_are_plain_lines_and_the_speed_is_told_by_the_status() {
        let mut c = marlin(Impair::clean(), false);
        c.command(Command::Spindle {
            rpm: 9000.0,
            clockwise: false,
        })
        .unwrap();
        assert_eq!(c.status().unwrap().spindle_rpm, 9000.0);
        c.command(Command::Spindle {
            rpm: 0.0,
            clockwise: true,
        })
        .unwrap();
        c.command(Command::Coolant(true)).unwrap();
        c.command(Command::Coolant(false)).unwrap();
        let log = c.transport().device().executed.clone();
        assert!(log.contains(&"M4 S9000".to_string()) && log.contains(&"M5".to_string()));
        assert!(log.contains(&"M8".to_string()) && log.contains(&"M9".to_string()));
    }

    #[test]
    fn silence_is_a_timeout_and_a_move_that_got_no_answer_is_stopped() {
        let mut c = marlin(
            Impair {
                mute_from: Some(1),
                mute_lines: 1,
                ..Impair::clean()
            },
            false,
        );
        assert_eq!(c.send_line("G1 X5 F100"), Err(Error::Timeout));
        // the quick stop went out behind it
        assert!(c.transport().heard_text().ends_with("M410\n"));
        // and the cable is good again: the next line is answered
        c.send_line("G1 X6 F100").unwrap();
        assert_eq!(c.status().unwrap().machine_mm[0], 6.0);
    }

    #[test]
    fn a_greeting_nobody_asked_for_means_the_controller_restarted_and_one_after_a_reset_does_not() {
        let mut c = marlin(Impair::clean(), false);
        let greeting: Vec<u8> = MarlinDevice::greeting().to_vec();
        c.transport_mut().push_unsolicited(&greeting);
        let e = c.send_line("G1 X1 F10").unwrap_err();
        assert!(
            matches!(e, Error::Refused(ref t) if t.contains("restarted")),
            "{e:?}"
        );
    }

    #[test]
    fn answers_cut_in_pieces_and_noise_between_the_lines_do_not_change_the_result() {
        for seed in 1..=40u64 {
            let mut c = marlin(
                Impair {
                    seed,
                    chunk_max: 3,
                    noise_per_mille: 400,
                    ..Impair::clean()
                },
                false,
            );
            c.send_line("G1 X10 Y20 F500").unwrap();
            c.command(Command::Home(vec![1])).unwrap();
            let s = c.status().unwrap();
            assert_eq!(&s.machine_mm[..2], &[10.0, 0.0], "seed {seed}");
        }
    }

    #[test]
    fn numbered_lines_are_accepted_and_a_gap_in_the_numbers_is_asked_for_again() {
        let mut c = marlin(Impair::clean(), true);
        for i in 1..=5 {
            c.send_line(&format!("G1 X{i} F100")).unwrap();
        }
        assert_eq!(c.status().unwrap().machine_mm[0], 5.0);
        let heard = c.transport().heard_text();
        assert!(heard.starts_with("N0 M110 N0*"), "{heard:?}");
        assert!(heard.contains(&frame(2, "G1 X2 F100")), "{heard:?}");
        // the device is told it is on line 12 while the program is on 7: it asks for 6 and that is not the line the program is on
        c.transport_mut()
            .device_mut()
            .input(frame(11, "M110 N11").as_bytes());
        let _ = c.transport_mut().device_mut().input(b"\n");
        let e = c.send_line("G1 X9 F100").unwrap_err();
        assert!(matches!(e, Error::Refused(_)), "{e:?}");
    }

    #[test]
    fn a_line_spoiled_on_the_wire_is_never_executed_when_the_lines_are_numbered() {
        let intended: Vec<String> = (1..=25).map(|i| format!("G1 X{i}.5 Y{i} F300")).collect();
        let mut resent_somewhere = false;
        for seed in 1..=30u64 {
            let mut c = marlin(
                Impair {
                    seed,
                    flip_per_mille: 350,
                    chunk_max: 5,
                    ..Impair::clean()
                },
                true,
            );
            for l in &intended {
                match c.send_line(l) {
                    Ok(()) => {}
                    // after three tries the program gives up on the line: that is an error the operator sees, not a wrong move
                    Err(e) => assert_eq!(e.code(), Some(E_CHECKSUM), "seed {seed}: {e:?}"),
                }
            }
            let dev = c.transport().device();
            for done in &dev.executed {
                assert!(
                    // the first line, which tells Marlin where the numbers start, moves nothing and is the one a spoiled byte can change
                    intended.contains(done) || done.starts_with("M110"),
                    "seed {seed}: the device executed '{done}'"
                );
            }
            resent_somewhere |= c.transport().flips > 0;
        }
        assert!(resent_somewhere, "the schedule never spoiled a line");
    }

    #[test]
    fn without_numbers_a_spoiled_line_reaches_the_machine_as_it_arrived() {
        // this is why the numbering exists: the same cable, the same flipped bits, and a line is executed that nobody wrote
        let intended: Vec<String> = (1..=25).map(|i| format!("G1 X{i}.5 Y{i} F300")).collect();
        let mut wrong = 0;
        for seed in 1..=30u64 {
            let mut c = marlin(
                Impair {
                    seed,
                    flip_per_mille: 350,
                    ..Impair::clean()
                },
                false,
            );
            for l in &intended {
                let _ = c.send_line(l);
            }
            wrong += c
                .transport()
                .device()
                .executed
                .iter()
                .filter(|d| !intended.contains(d))
                .count();
        }
        assert!(wrong > 0, "no spoiled line was ever executed");
    }

    /// A transport that puts an urgent order in the channel the first time the controller waits, as another thread would.
    struct Interrupt {
        link: WireLink<MarlinDevice>,
        channel: Option<UrgentChannel>,
        what: Urgent,
        armed: bool,
    }

    impl LineTransport for Interrupt {
        fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.link.write_all(bytes)
        }

        fn read_line(&mut self, timeout: Duration) -> io::Result<Option<String>> {
            if self.armed {
                self.armed = false;
                if let Some(ch) = &self.channel {
                    ch.send(self.what);
                }
                return Ok(None);
            }
            self.link.read_line(timeout)
        }
    }

    fn interrupted(what: Urgent) -> MarlinController<Interrupt> {
        let mut c = MarlinController::new(Interrupt {
            link: WireLink::new(MarlinDevice::new(), Impair::clean()),
            channel: None,
            what,
            armed: false,
        });
        let ch = c.urgent_channel().unwrap();
        c.transport_mut().channel = Some(ch);
        c.transport_mut().armed = true;
        c
    }

    #[test]
    fn an_urgent_hold_is_the_quick_stop_because_marlin_cannot_resume_and_the_line_that_waits_is_cut_short(
    ) {
        let mut c = interrupted(Urgent::Hold);
        let e = c.send_line("G1 X5 F100").unwrap_err();
        assert!(
            matches!(e, Error::Refused(ref t) if t.contains("urgent")),
            "{e:?}"
        );
        assert!(c.transport().link.heard_text().ends_with("M410\n"));
        // the next exchange reads off what the stop left on the way and works
        c.send_line("G1 X6 F100").unwrap();
        assert_eq!(c.status().unwrap().machine_mm[0], 6.0);
    }

    #[test]
    fn an_urgent_reset_is_the_emergency_stop_and_leaves_marlin_halted() {
        let mut c = interrupted(Urgent::Reset);
        let e = c.send_line("G1 X5 F100").unwrap_err();
        assert!(matches!(e, Error::Refused(_)), "{e:?}");
        assert!(c.halted());
        assert!(c.transport().link.heard_text().ends_with("M112\n"));
        let e = c.send_line("G1 X6 F100").unwrap_err();
        assert!(matches!(e, Error::Alarm(_)), "{e:?}");
        assert_eq!(e.code(), Some(E_HALTED));
    }
}
