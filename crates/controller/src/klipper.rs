//! Klipper through Moonraker: a [`Controller`] of the suite that speaks the JSON-RPC of Moonraker (the same messages go over a WebSocket or
//! over HTTP), written against a [`JsonRpcTransport`] that carries one JSON text out and one JSON text in. This crate has no network
//! transport yet (see `docs/adr/0001-klipper-transport.md`): the controller is tried against the simulated Klipper of [`crate::klippersim`]
//! behind a cable that misbehaves on purpose, and a transport that opens a real socket only has to implement the two calls of the trait.
//!
//! What Klipper is not: it has no state report on the line (the state is asked for with `printer.objects.query`), no hold that can be
//! resumed from outside the queue of G-code, and no quick stop that keeps the machine ready: the only order that reaches the machine ahead of
//! the queue is `printer.emergency_stop`, after which Klipper is shut down until `printer.firmware_restart`. So every way this controller has
//! of stopping something that moves is the emergency stop, and it fails closed: a move that gets no answer, and no answer to a liveness probe
//! either, is stopped that way and the controller refuses everything that is not a stop, an unlock or a question until the operator unlocks it.
//! Klipper says its errors in words; this module gives each kind a number of its own, 201 and up, so that they are told apart and cannot be
//! mistaken for the numbers of GRBL (1 to 53) or of Marlin (101 to 108). Nothing here has run on a real Klipper: the evidence is the simulator.

use std::io;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::grbl::{Timeouts, MAX_BUSY_WAIT};
use crate::urgent::{Urgent, UrgentChannel};
use crate::{Capabilities, Command, Controller, Error, Family, State, Status};

/// The numbers of the kinds of error of Klipper.
pub const E_UNKNOWN: u32 = 201;
pub const E_HOME_FIRST: u32 = 202;
pub const E_RANGE: u32 = 203;
pub const E_SHUTDOWN: u32 = 204;
pub const E_THERMAL: u32 = 205;
pub const E_NOT_READY: u32 = 206;
pub const E_MALFORMED: u32 = 207;
pub const E_OTHER: u32 = 208;

/// The most liveness probes one line may send while the machine is busy with it (a guard against a link that answers the probes at once and
/// never the line; the real limit is the busy limit, in time).
pub const MAX_PROBES: u32 = 4096;

/// What a line that was cut short by an urgent stop says (as the GRBL and Marlin controllers of the suite say it).
const INTERRUPTED: &str = "interrupted by an urgent stop";

/// Why every order is refused after the machine was stopped.
const STOPPED: &str = "the machine was stopped and Klipper is shut down until it is unlocked";

/// The link of this controller: one JSON text out, and JSON texts in with a time limit. A WebSocket sends the text as it is; an HTTP link
/// posts it and hands back the body. Texts that arrive and are not an answer (notifications of Moonraker, noise) are the transport's to pass
/// on: the controller reads what it needs and ignores the rest.
pub trait JsonRpcTransport {
    /// Sends one JSON-RPC message.
    fn send(&mut self, text: &str) -> io::Result<()>;
    /// The next text that came in, or `None` when nothing came in the time given.
    fn recv(&mut self, timeout: Duration) -> io::Result<Option<String>>;
}

/// The kind of error a message of Klipper is, with the number of the kind and whether it is an alarm (the machine is shut down, not ready or
/// in danger) or the refusal of one line.
pub fn classify(message: &str) -> (u32, bool) {
    let m = message.to_ascii_lowercase();
    if m.contains("unknown command") {
        (E_UNKNOWN, false)
    } else if m.contains("must home") {
        (E_HOME_FIRST, false)
    } else if m.contains("out of range") {
        (E_RANGE, false)
    } else if m.contains("mintemp")
        || m.contains("maxtemp")
        || m.contains("thermal runaway")
        || m.contains("adc out of range")
        || m.contains("heater")
    {
        (E_THERMAL, true)
    } else if m.contains("shutdown")
        || m.contains("shut down")
        || m.contains("lost communication")
        || m.contains("timer too close")
        || m.contains("disconnected")
        || m.contains("firmware_restart")
    {
        (E_SHUTDOWN, true)
    } else if m.contains("not ready") || m.contains("startup") {
        (E_NOT_READY, true)
    } else if m.contains("malformed") || m.contains("parse error") || m.contains("invalid request")
    {
        (E_MALFORMED, false)
    } else {
        (E_OTHER, false)
    }
}

fn axis_name(axis: usize) -> Result<char, Error> {
    ['X', 'Y', 'Z']
        .get(axis)
        .copied()
        .ok_or(Error::Unsupported("that axis"))
}

fn numbers(v: &Value, n: usize) -> Option<Vec<f64>> {
    let a = v.as_array()?;
    if a.len() < n {
        return None;
    }
    a.iter()
        .take(n)
        .map(|x| x.as_f64().filter(|f| f.is_finite()))
        .collect()
}

/// Klipper over a JSON-RPC transport.
pub struct KlipperController<T: JsonRpcTransport> {
    transport: T,
    timeouts: Timeouts,
    busy_max: Duration,
    urgent: UrgentChannel,
    next_id: u64,
    /// The machine was stopped (by the program, by a shutdown Klipper announced, or because it fell silent): everything that moves is
    /// refused until [`Command::Unlock`].
    halted: bool,
    /// A line got no answer or was cut short: what is still on its way belongs to no order.
    resync: bool,
    /// The liveness probes the last line sent.
    probes: u32,
    feed: f64,
}

impl<T: JsonRpcTransport> KlipperController<T> {
    pub fn new(transport: T) -> Self {
        KlipperController {
            transport,
            timeouts: Timeouts::default(),
            busy_max: MAX_BUSY_WAIT,
            urgent: UrgentChannel::new(),
            next_id: 1,
            halted: false,
            resync: false,
            probes: 0,
            feed: 0.0,
        }
    }

    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// The most a line that moves waits while the machine answers the liveness probes but not the line (zero waits for the ordinary time only).
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

    /// Whether the machine was stopped and waits for [`Command::Unlock`].
    pub fn halted(&self) -> bool {
        self.halted
    }

    /// How many liveness probes the last line that moved sent while it waited.
    pub fn probes_sent(&self) -> u32 {
        self.probes
    }

    fn link(e: io::Error) -> Error {
        Error::Link(e.to_string())
    }

    fn watched(&self) -> bool {
        self.urgent.handles() > 1
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        id
    }

    fn send(&mut self, method: &str, params: &Value, id: u64) -> Result<(), Error> {
        let msg = json!({"jsonrpc": "2.0", "method": method, "params": params, "id": id});
        self.transport.send(&msg.to_string()).map_err(Self::link)
    }

    /// Stops the machine at once, without waiting for an answer: the only order of Klipper that goes ahead of its queue.
    fn emergency(&mut self) -> Result<(), Error> {
        self.halted = true;
        self.resync = true;
        let id = self.take_id();
        self.send("printer.emergency_stop", &json!({}), id)
    }

    /// Writes what was asked to go out of turn. `Ok(true)` when it ended the job.
    fn flush_urgent(&mut self) -> Result<bool, Error> {
        let mut ended = false;
        for u in self.urgent.take() {
            match u {
                // nothing but the emergency stop passes the queue of Klipper: a hold and a stop of the job are that, the safe side
                Urgent::Hold | Urgent::StopJob | Urgent::Reset => self.emergency()?,
                Urgent::Status | Urgent::Resume => {}
            }
            ended |= u.ends_job() || matches!(u, Urgent::Hold);
        }
        Ok(ended)
    }

    fn read_next(&mut self, left: Duration) -> Result<Option<String>, Error> {
        const POLL: Duration = Duration::from_millis(25);
        if !self.watched() {
            if self.flush_urgent()? {
                return Err(Error::Refused(INTERRUPTED.to_string()));
            }
            return self.transport.recv(left).map_err(Self::link);
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
            if let Some(t) = self
                .transport
                .recv(remaining.min(POLL))
                .map_err(Self::link)?
            {
                return Ok(Some(t));
            }
        }
    }

    /// Reads off what is on the way and belongs to no order.
    fn drain(&mut self) {
        let wait = self.timeouts.line.min(Duration::from_millis(50));
        let mut n = 0;
        while n < 256 {
            match self.transport.recv(wait) {
                Ok(Some(_)) => n += 1,
                _ => break,
            }
        }
    }

    fn refusal(message: &str) -> (Error, bool) {
        let (kind, alarm) = classify(message);
        if alarm {
            (Error::Alarm(format!("alarm {kind}: {message}")), true)
        } else {
            (Error::Refused(format!("error {kind}: {message}")), false)
        }
    }

    fn answer(&mut self, msg: &Value) -> Result<Value, Error> {
        if let Some(e) = msg.get("error") {
            let text = e
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("no message");
            let (err, alarm) = Self::refusal(text);
            if alarm {
                self.halted = true;
            }
            return Err(err);
        }
        msg.get("result").cloned().ok_or_else(|| {
            Error::Refused(format!(
                "error {E_MALFORMED}: the answer has neither a result nor an error"
            ))
        })
    }

    /// Takes note of a message that is no answer: Moonraker tells when Klipper shuts down or goes away.
    fn notification(&mut self, msg: &Value) {
        if let Some(m) = msg.get("method").and_then(Value::as_str) {
            if m == "notify_klippy_shutdown" || m == "notify_klippy_disconnected" {
                self.halted = true;
            }
        }
    }

    /// One call and its answer. `probe` says that the call may take as long as the machine takes to do what it asks (a move): the wait then
    /// does not end at `limit` but with a liveness probe (`printer.info`, which Klipper answers out of the queue of G-code); while the probes
    /// are answered the machine is alive and busy, and the wait goes on up to the busy limit. A probe that is not answered is silence.
    fn call(
        &mut self,
        method: &str,
        params: &Value,
        limit: Duration,
        probe: bool,
    ) -> Result<Value, Error> {
        self.flush_urgent()?;
        if self.resync {
            self.drain();
            self.resync = false;
        }
        self.probes = 0;
        let id = self.take_id();
        self.send(method, params, id)?;
        let began = Instant::now();
        let mut end = began + limit;
        let mut probe_id: Option<u64> = None;
        loop {
            let now = Instant::now();
            let left = end.saturating_duration_since(now);
            let frame = if left.is_zero() {
                None
            } else {
                self.read_next(left)?
            };
            let Some(text) = frame else {
                // the wait is over with nothing: a line that moves asks whether the machine is alive before it gives up
                if probe
                    && probe_id.is_none()
                    && began.elapsed() < self.busy_max
                    && self.probes < MAX_PROBES
                {
                    let pid = self.take_id();
                    self.send("printer.info", &json!({}), pid)?;
                    self.probes += 1;
                    probe_id = Some(pid);
                    end = Instant::now() + self.timeouts.line;
                    continue;
                }
                self.resync = true;
                return Err(Error::Timeout);
            };
            let Ok(msg) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if !msg.is_object() {
                continue;
            }
            match msg.get("id") {
                Some(Value::Number(n)) => {
                    let got = n.as_u64();
                    if got == Some(id) {
                        return self.answer(&msg);
                    }
                    if got.is_some() && got == probe_id {
                        // alive and busy: wait for the line again
                        probe_id = None;
                        end = Instant::now() + limit;
                    }
                    // any other id is the late answer to something that is over
                }
                Some(Value::Null) if msg.get("error").is_some() => {
                    // the request was not understood well enough to be answered by its number: it can only be this one
                    return self.answer(&msg);
                }
                Some(_) => {}
                None => self.notification(&msg),
            }
        }
    }

    fn script(&mut self, script: &str, limit: Duration, probe: bool) -> Result<(), Error> {
        // a stop that was asked for from another thread comes before the line
        self.flush_urgent()?;
        self.refuse_if_halted()?;
        let moves = Self::moves(script);
        let r = self.call(
            "printer.gcode.script",
            &json!({ "script": script }),
            limit,
            probe && moves,
        );
        self.after(moves, r.map(|_| ()))
    }

    fn refuse_if_halted(&self) -> Result<(), Error> {
        if self.halted {
            Err(Error::Alarm(format!("alarm {E_SHUTDOWN}: {STOPPED}")))
        } else {
            Ok(())
        }
    }

    /// Whatever the end of an order was, a move that got no answer (and no answer to the probes) is not left running.
    fn after(&mut self, moves: bool, done: Result<(), Error>) -> Result<(), Error> {
        if done == Err(Error::Timeout) && moves {
            let _ = self.emergency();
        }
        done
    }

    /// Whether a script moves the machine (a rapid, a cut, an arc, the homing, a probe).
    fn moves(script: &str) -> bool {
        script.lines().any(|l| {
            let t = l.trim().to_ascii_uppercase();
            let first = t.split_whitespace().next().unwrap_or("");
            first
                .strip_prefix('G')
                .and_then(|n| n.parse::<f64>().ok())
                .is_some_and(|g| {
                    [0.0, 1.0, 2.0, 3.0, 28.0].contains(&g) || (38.0..39.0).contains(&g)
                })
        })
    }

    fn limit_of(&self, script: &str) -> Duration {
        if script.trim().to_ascii_uppercase().starts_with("G28") {
            self.timeouts.homing
        } else {
            self.timeouts.line
        }
    }

    fn query(&mut self, objects: Value) -> Result<Value, Error> {
        let limit = self.timeouts.line;
        let r = self.call(
            "printer.objects.query",
            &json!({ "objects": objects }),
            limit,
            false,
        )?;
        r.get("status")
            .cloned()
            .ok_or_else(|| Error::Link("the controller gave no status".to_string()))
    }
}

impl<T: JsonRpcTransport> Controller for KlipperController<T> {
    fn family(&self) -> Family {
        Family::Klipper
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            axes: 3,
            spindle: false,
            spindle_speed: false,
            coolant: false,
            homing: true,
            probe: false,
            jog: true,
            pause: true,
            emergency_stop: true,
            tool_changer: false,
            gcode: true,
        }
    }

    fn status(&mut self) -> Result<Status, Error> {
        let st = self.query(json!({
            "webhooks": ["state", "state_message"],
            "toolhead": ["position", "homed_axes"],
            "gcode_move": ["gcode_position", "speed"],
            "idle_timeout": ["state"],
            "pause_resume": ["is_paused"],
        }))?;
        let machine = st
            .get("toolhead")
            .and_then(|t| t.get("position"))
            .and_then(|p| numbers(p, 3))
            .ok_or_else(|| Error::Link("the controller gave no position".to_string()))?;
        let work = st
            .get("gcode_move")
            .and_then(|g| g.get("gcode_position"))
            .and_then(|p| numbers(p, 3))
            .unwrap_or_else(|| machine.clone());
        let speed = st
            .get("gcode_move")
            .and_then(|g| g.get("speed"))
            .and_then(Value::as_f64)
            .filter(|s| s.is_finite() && *s >= 0.0)
            .unwrap_or(0.0);
        let web = st
            .get("webhooks")
            .and_then(|w| w.get("state"))
            .and_then(Value::as_str)
            .unwrap_or("ready");
        let idle = st
            .get("idle_timeout")
            .and_then(|i| i.get("state"))
            .and_then(Value::as_str)
            .unwrap_or("Ready");
        let paused = st
            .get("pause_resume")
            .and_then(|p| p.get("is_paused"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let state = if web == "shutdown" || web == "error" {
            self.halted = true;
            State::Alarm
        } else if self.halted {
            State::Alarm
        } else if web == "startup" {
            State::Unknown
        } else if paused {
            State::Hold
        } else if idle == "Printing" {
            State::Run
        } else {
            State::Idle
        };
        let mut out = Status {
            state,
            feed_mm_min: speed * 60.0,
            ..Status::default()
        };
        for (i, (m, w)) in machine.iter().zip(&work).enumerate() {
            out.machine_mm[i] = *m;
            out.work_mm[i] = *w;
        }
        self.feed = out.feed_mm_min;
        Ok(out)
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
                let limit = self.timeouts.jog;
                self.script("G91", self.timeouts.line, false)?;
                let moved = self.script(
                    &format!("G1 {a}{distance_mm:.3} F{feed_mm_min:.0}"),
                    limit,
                    true,
                );
                // the machine never stays in relative mode, whatever happened to the move (a machine that was stopped is shut down: there
                // is nothing to put back)
                let back = if self.halted {
                    Ok(())
                } else {
                    self.script("G90", self.timeouts.line, false)
                };
                moved.and(back)
            }
            Command::JogStop | Command::Stop => {
                if self.halted {
                    return Ok(());
                }
                // a print that Moonraker knows is cancelled; anything else only the emergency stop can stop
                match self.call(
                    "printer.print.cancel",
                    &json!({}),
                    self.timeouts.line,
                    false,
                ) {
                    Ok(_) => Ok(()),
                    Err(Error::Link(e)) => Err(Error::Link(e)),
                    Err(_) => self.emergency(),
                }
            }
            Command::Home(axes) => {
                let mut line = String::from("G28");
                for a in axes {
                    line.push(' ');
                    line.push(axis_name(a)?);
                }
                let limit = self.limit_of(&line);
                self.script(&line, limit, true)
            }
            Command::Pause => self.script("PAUSE", self.timeouts.line, false),
            Command::Resume => self.script("RESUME", self.timeouts.line, false),
            Command::EmergencyStop => self.emergency(),
            Command::Unlock => {
                self.call(
                    "printer.firmware_restart",
                    &json!({}),
                    self.timeouts.homing,
                    false,
                )?;
                self.halted = false;
                self.resync = true;
                // the restart is not trusted until Klipper says it is ready
                if self.status()?.state == State::Alarm {
                    self.halted = true;
                    return Err(Error::Alarm(format!(
                        "alarm {E_NOT_READY}: Klipper is not ready after the restart"
                    )));
                }
                Ok(())
            }
            Command::ZeroWork(axes) => {
                self.refuse_if_halted()?;
                let axes = if axes.is_empty() { vec![0, 1, 2] } else { axes };
                let mut words = String::new();
                for a in &axes {
                    words.push_str(&format!(" {}0", axis_name(*a)?));
                }
                self.script(&format!("G92{words}"), self.timeouts.line, false)
            }
            Command::Spindle { .. } => Err(Error::Unsupported("spindle")),
            Command::Coolant(_) => Err(Error::Unsupported("coolant")),
        }
    }

    fn send_line(&mut self, line: &str) -> Result<(), Error> {
        let text = crate::marlin::clean(line);
        if text.is_empty() {
            return Ok(());
        }
        let limit = self.limit_of(&text);
        self.script(&text, limit, true)?;
        // the feed of the program is what the status says
        for w in text.to_ascii_uppercase().split_whitespace() {
            if let Some(f) = w.strip_prefix('F').and_then(|v| v.parse::<f64>().ok()) {
                self.feed = f;
            }
        }
        Ok(())
    }

    fn max_travel(&mut self) -> Result<Option<[f64; 3]>, Error> {
        let st = self.query(json!({ "configfile": ["settings"] }))?;
        let settings = st.get("configfile").and_then(|c| c.get("settings"));
        let mut found = [0.0; 3];
        for (i, name) in ["stepper_x", "stepper_y", "stepper_z"].iter().enumerate() {
            let max = settings
                .and_then(|s| s.get(*name))
                .and_then(|s| s.get("position_max"))
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite() && *v > 0.0);
            match max {
                Some(v) => found[i] = v,
                None => return Ok(None),
            }
        }
        Ok(Some(found))
    }

    fn urgent_channel(&self) -> Option<UrgentChannel> {
        Some(self.urgent.clone())
    }
}
