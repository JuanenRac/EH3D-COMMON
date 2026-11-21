//! A simulated Klipper behind Moonraker, and the cable to it: [`KlipperDevice`] answers the JSON-RPC messages of Moonraker the way the
//! firmware stack does (a script of G-code, a query of objects, the emergency stop, the firmware restart), with the refusals in the words
//! Klipper uses, and [`RpcLink`] puts it behind the transport of the controller and spoils the messages by a schedule that a fixed seed makes
//! the same on every computer: noise between the answers (text that is no JSON, a notification, the late answer to a message that is over), a
//! flipped bit in what the program sends, and a silence for a while. No network and no time are in it: a read that finds nothing says so at
//! once. This is not Klipper: it answers what the controller asks and nothing else, and evidence from it is `simulator`.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::time::Duration;

use serde_json::{json, Value};

use crate::klipper::JsonRpcTransport;
use crate::wire::{Impair, Seed};

/// The travel of the simulated machine in mm (X, Y and Z go from 0 to this).
pub const TRAVEL_MM: [f64; 3] = [300.0, 200.0, 100.0];

/// What Moonraker puts on the line when the other end speaks to it.
pub trait RpcDevice {
    /// The messages that answer one message (an answer, and perhaps notifications); none when the message needs no answer.
    fn handle(&mut self, request: &str) -> Vec<String>;
}

/// The commands that are taken and change nothing the simulation keeps.
const QUIET: [&str; 14] = [
    "G21", "G4", "M400", "M18", "M84", "M82", "M83", "M104", "M109", "M140", "M190", "M106",
    "M107", "M114",
];

/// The simulated Klipper.
#[derive(Debug)]
pub struct KlipperDevice {
    machine: [f64; 3],
    origin: [f64; 3],
    homed: [bool; 3],
    absolute: bool,
    speed_mm_s: f64,
    shutdown: Option<String>,
    paused: bool,
    busy: u8,
    scripts: usize,
    /// A print is in progress (the one thing `printer.print.cancel` can stop).
    pub printing: bool,
    /// The script number N (from 1) fails with this message instead of running: a refusal in the words of Klipper.
    pub fail: BTreeMap<usize, String>,
    /// The travel of each axis, from 0 to this many mm.
    pub travel: [f64; 3],
    /// The commands that were executed, in order.
    pub executed: Vec<String>,
    /// The methods that were asked, in order.
    pub methods: Vec<String>,
    /// How many times the emergency stop was given, and how many times the firmware was restarted.
    pub emergency_stops: usize,
    pub restarts: usize,
}

impl Default for KlipperDevice {
    fn default() -> Self {
        KlipperDevice::new()
    }
}

fn error_frame(id: &Value, code: i64, message: &str) -> String {
    json!({"jsonrpc": "2.0", "error": {"code": code, "message": message}, "id": id}).to_string()
}

fn result_frame(id: &Value, result: Value) -> String {
    json!({"jsonrpc": "2.0", "result": result, "id": id}).to_string()
}

impl KlipperDevice {
    pub fn new() -> KlipperDevice {
        KlipperDevice {
            machine: [0.0; 3],
            origin: [0.0; 3],
            homed: [false; 3],
            absolute: true,
            speed_mm_s: 0.0,
            shutdown: None,
            paused: false,
            busy: 0,
            scripts: 0,
            printing: false,
            fail: BTreeMap::new(),
            travel: TRAVEL_MM,
            executed: Vec::new(),
            methods: Vec::new(),
            emergency_stops: 0,
            restarts: 0,
        }
    }

    pub fn position(&self) -> [f64; 3] {
        self.machine
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.is_some()
    }

    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    pub fn is_homed(&self) -> [bool; 3] {
        self.homed
    }

    fn work(&self) -> [f64; 3] {
        [
            self.machine[0] - self.origin[0],
            self.machine[1] - self.origin[1],
            self.machine[2] - self.origin[2],
        ]
    }

    fn words(command: &str) -> Vec<(char, f64)> {
        command
            .split_whitespace()
            .skip(1)
            .filter_map(|w| {
                let mut c = w.chars();
                let letter = c.next()?.to_ascii_uppercase();
                let value = c.as_str().parse::<f64>().ok()?;
                Some((letter, value))
            })
            .collect()
    }

    fn mv(&mut self, command: &str) -> Result<(), String> {
        let words = Self::words(command);
        let mut target = self.work();
        let mut moved = [false; 3];
        for (letter, v) in &words {
            let slot = match letter {
                'X' => 0,
                'Y' => 1,
                'Z' => 2,
                'F' => {
                    if *v > 0.0 {
                        self.speed_mm_s = v / 60.0;
                    }
                    continue;
                }
                _ => continue,
            };
            target[slot] = if self.absolute { *v } else { target[slot] + v };
            moved[slot] = true;
        }
        let fmt = |p: [f64; 3]| format!("{:.3} {:.3} {:.3} [0.000]", p[0], p[1], p[2]);
        if (0..3).any(|a| moved[a] && !self.homed[a]) {
            return Err(format!("Must home axis first: {}", fmt(target)));
        }
        let machine = [
            target[0] + self.origin[0],
            target[1] + self.origin[1],
            target[2] + self.origin[2],
        ];
        if (0..3).any(|a| moved[a] && (machine[a] < 0.0 || machine[a] > self.travel[a])) {
            return Err(format!("Move out of range: {}", fmt(target)));
        }
        self.machine = machine;
        self.busy = 1;
        Ok(())
    }

    fn run(&mut self, command: &str) -> Result<(), String> {
        let head = command
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        match head.as_str() {
            "G0" | "G1" => self.mv(command)?,
            "G28" => {
                let named: Vec<usize> = command
                    .to_ascii_uppercase()
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|w| match w {
                        "X" => Some(0),
                        "Y" => Some(1),
                        "Z" => Some(2),
                        _ => None,
                    })
                    .collect();
                let axes = if named.is_empty() {
                    vec![0, 1, 2]
                } else {
                    named
                };
                for a in axes {
                    self.machine[a] = 0.0;
                    self.homed[a] = true;
                }
                self.busy = 1;
            }
            "G90" => self.absolute = true,
            "G91" => self.absolute = false,
            "G92" => {
                for (letter, v) in Self::words(command) {
                    let slot = match letter {
                        'X' => 0,
                        'Y' => 1,
                        'Z' => 2,
                        _ => continue,
                    };
                    self.origin[slot] = self.machine[slot] - v;
                }
            }
            "M112" => {
                self.shutdown = Some("Shutdown due to M112 command".to_string());
                self.emergency_stops += 1;
            }
            "PAUSE" => self.paused = true,
            "RESUME" => self.paused = false,
            "FIRMWARE_RESTART" | "RESTART" => self.restart(),
            h if QUIET.contains(&h) => {}
            _ => return Err(format!("Unknown command:\"{head}\"")),
        }
        self.executed.push(command.trim().to_string());
        Ok(())
    }

    fn restart(&mut self) {
        self.shutdown = None;
        self.homed = [false; 3];
        self.paused = false;
        self.absolute = true;
        self.busy = 0;
        self.restarts += 1;
    }

    fn status(&mut self, objects: &Value) -> Value {
        let mut st = serde_json::Map::new();
        let wanted = |name: &str| objects.get(name).is_some();
        if wanted("webhooks") {
            let (state, message) = match &self.shutdown {
                Some(m) => ("shutdown", m.as_str()),
                None => ("ready", "Printer is ready"),
            };
            st.insert(
                "webhooks".into(),
                json!({"state": state, "state_message": message}),
            );
        }
        if wanted("toolhead") {
            let homed: String = ["x", "y", "z"]
                .iter()
                .zip(self.homed)
                .filter(|(_, h)| *h)
                .map(|(n, _)| *n)
                .collect();
            st.insert(
                "toolhead".into(),
                json!({"position": [self.machine[0], self.machine[1], self.machine[2], 0.0], "homed_axes": homed}),
            );
        }
        if wanted("gcode_move") {
            let w = self.work();
            st.insert(
                "gcode_move".into(),
                json!({"gcode_position": [w[0], w[1], w[2], 0.0], "speed": self.speed_mm_s,
                       "homing_origin": [self.origin[0], self.origin[1], self.origin[2], 0.0]}),
            );
        }
        if wanted("idle_timeout") {
            let state = if self.busy > 0 { "Printing" } else { "Ready" };
            self.busy = self.busy.saturating_sub(1);
            st.insert("idle_timeout".into(), json!({ "state": state }));
        }
        if wanted("pause_resume") {
            st.insert("pause_resume".into(), json!({"is_paused": self.paused}));
        }
        if wanted("configfile") {
            st.insert(
                "configfile".into(),
                json!({"settings": {
                    "stepper_x": {"position_max": self.travel[0]},
                    "stepper_y": {"position_max": self.travel[1]},
                    "stepper_z": {"position_max": self.travel[2]},
                }}),
            );
        }
        json!({"eventtime": 1.0, "status": Value::Object(st)})
    }

    fn script(&mut self, id: &Value, script: &str) -> String {
        self.scripts += 1;
        if let Some(m) = &self.shutdown {
            return error_frame(id, 400, &format!("Printer is shutdown: {m}"));
        }
        if let Some(m) = self.fail.get(&self.scripts) {
            return error_frame(id, 400, m);
        }
        for line in script.lines().filter(|l| !l.trim().is_empty()) {
            if let Err(m) = self.run(line) {
                return error_frame(id, 400, &m);
            }
        }
        result_frame(id, json!("ok"))
    }
}

impl RpcDevice for KlipperDevice {
    fn handle(&mut self, request: &str) -> Vec<String> {
        let Ok(v) = serde_json::from_str::<Value>(request) else {
            return vec![error_frame(&Value::Null, -32700, "Parse error")];
        };
        let Some(method) = v.get("method").and_then(Value::as_str) else {
            return vec![error_frame(&Value::Null, -32600, "Invalid Request")];
        };
        self.methods.push(method.to_string());
        let Some(id) = v.get("id").cloned() else {
            return Vec::new();
        };
        let params = v.get("params").cloned().unwrap_or(Value::Null);
        let frame = match method {
            "printer.gcode.script" => match params.get("script").and_then(Value::as_str) {
                Some(s) => self.script(&id, s),
                None => error_frame(&id, -32602, "Invalid params: script"),
            },
            "printer.objects.query" => {
                let objects = params.get("objects").cloned().unwrap_or(Value::Null);
                result_frame(&id, self.status(&objects))
            }
            "printer.emergency_stop" => {
                if self.shutdown.is_none() {
                    self.shutdown = Some("Shutdown due to M112 command".to_string());
                }
                self.emergency_stops += 1;
                result_frame(&id, json!("ok"))
            }
            "printer.firmware_restart" | "printer.restart" => {
                self.restart();
                result_frame(&id, json!("ok"))
            }
            "printer.info" | "server.info" => {
                let (state, message) = match &self.shutdown {
                    Some(m) => ("shutdown", m.as_str()),
                    None => ("ready", "Printer is ready"),
                };
                result_frame(
                    &id,
                    json!({"state": state, "state_message": message, "software_version": "v0.12.0-simulated"}),
                )
            }
            "printer.print.cancel" | "printer.print.pause" | "printer.print.resume" => {
                if self.printing {
                    if method == "printer.print.cancel" {
                        self.printing = false;
                    }
                    result_frame(&id, json!("ok"))
                } else {
                    error_frame(&id, 400, "No print job is active")
                }
            }
            _ => error_frame(&id, -32601, "Method not found"),
        };
        vec![frame]
    }
}

/// A [`RpcDevice`] behind a cable that misbehaves on purpose. The fields of [`Impair`] that mean something for messages that arrive whole
/// are the seed, `noise_per_mille` (before an answer, a message that is not one: text that is no JSON, a notification of Moonraker, or the
/// late answer to a message that is over), `flip_per_mille` (one bit of one byte of a message the program sends) and the silence
/// (`mute_from`, `mute_lines`: the device hears the messages from the number `mute_from`, from 1, and its answers to `mute_lines` of them
/// are lost). `chunk_max` means nothing here: a message is not cut.
#[derive(Debug)]
pub struct RpcLink<D: RpcDevice> {
    device: D,
    impair: Impair,
    seed: Seed,
    pending: VecDeque<String>,
    /// How many messages the program has sent.
    pub sent: usize,
    /// Every message the device heard, as it heard it (after the flipped bits).
    pub heard: Vec<String>,
    /// How many messages of noise were put in, and how many bits were flipped.
    pub noise_frames: usize,
    pub flips: usize,
}

impl<D: RpcDevice> RpcLink<D> {
    pub fn new(device: D, impair: Impair) -> RpcLink<D> {
        RpcLink {
            device,
            seed: Seed::new(impair.seed),
            impair,
            pending: VecDeque::new(),
            sent: 0,
            heard: Vec::new(),
            noise_frames: 0,
            flips: 0,
        }
    }

    pub fn device(&self) -> &D {
        &self.device
    }

    pub fn device_mut(&mut self) -> &mut D {
        &mut self.device
    }

    /// A message the device sends without being asked (a notification of Moonraker).
    pub fn push_unsolicited(&mut self, text: &str) {
        self.pending.push_back(text.to_string());
    }

    fn muted(&self) -> bool {
        match self.impair.mute_from {
            Some(from) => {
                self.sent >= from && self.sent < from.saturating_add(self.impair.mute_lines)
            }
            None => false,
        }
    }

    fn noise(&mut self) -> String {
        match self.seed.below(4) {
            0 => "{\"broken".to_string(),
            1 => "Klippy is talking to itself".to_string(),
            2 => json!({"jsonrpc": "2.0", "method": "notify_proc_stat_update", "params": [{}]})
                .to_string(),
            // the late answer to a message of long ago: its number is one this program never used
            _ => json!({"jsonrpc": "2.0", "result": "ok", "id": 999_999}).to_string(),
        }
    }
}

impl<D: RpcDevice> JsonRpcTransport for RpcLink<D> {
    fn send(&mut self, text: &str) -> io::Result<()> {
        let mut bytes = text.as_bytes().to_vec();
        self.sent += 1;
        if !bytes.is_empty() && self.seed.chance(self.impair.flip_per_mille) {
            let at = self.seed.below(bytes.len());
            bytes[at] ^= 1 << self.seed.below(7);
            self.flips += 1;
        }
        // a flipped bit of an ASCII byte stays ASCII; the message is text in any case
        let heard = String::from_utf8_lossy(&bytes).into_owned();
        self.heard.push(heard.clone());
        let answers = self.device.handle(&heard);
        if !self.muted() {
            self.pending.extend(answers);
        }
        Ok(())
    }

    fn recv(&mut self, _timeout: Duration) -> io::Result<Option<String>> {
        if self.pending.is_empty() {
            return Ok(None);
        }
        if self.seed.chance(self.impair.noise_per_mille) {
            self.noise_frames += 1;
            return Ok(Some(self.noise()));
        }
        Ok(self.pending.pop_front())
    }
}
