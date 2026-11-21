//! GRBL 1.1 and grblHAL as a [`Device`]: the lines and the bytes of the protocol answered the way the firmware answers them, so that
//! the controller of the suite is tried on its dialect and not on canned lines. The machine in it is a point that jumps to its target; what
//! it keeps is the position, the offset of the work, the spindle, the hold and the alarm. grblHAL differs where it is seen on the line: its
//! greeting, its build report with the extra options, the pins and the accessories in the status report, and the alarm that asks for homing
//! at the start. A line can be made to fail by its number, with the code the firmware would give. This is not the firmware: it answers
//! what the program asks and nothing else, and evidence from it is `simulator`.

use std::collections::BTreeMap;

use crate::wire::Device;

/// Which firmware answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Grbl,
    GrblHal,
}

/// The travel of the simulated machine in mm (X, Y, Z from 0 to the first, 0 to the second, minus the third to 0).
pub const TRAVEL_MM: [f64; 3] = [300.0, 200.0, 50.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Idle,
    Hold,
    Alarm(u32),
}

/// The simulated GRBL or grblHAL.
#[derive(Debug)]
pub struct GrblDevice {
    flavor: Flavor,
    mode: Mode,
    machine: [f64; 3],
    wco: [f64; 3],
    feed: f64,
    spindle: f64,
    relative: bool,
    rx: Vec<u8>,
    /// How many lines of G-code (not the `$` ones) were taken, from 1.
    lines: usize,
    /// The line of G-code number N (from 1) fails with `error:CODE`.
    pub fail: BTreeMap<usize, u32>,
    /// A status report of a move says `Run` this many more times before it says `Idle`.
    busy: u8,
    reports: usize,
    /// What the program asked, line by line.
    pub log: Vec<String>,
    /// How many times the machine was reset.
    pub resets: usize,
}

impl GrblDevice {
    pub fn new(flavor: Flavor) -> GrblDevice {
        GrblDevice {
            flavor,
            // grblHAL with homing on starts locked until it is homed or unlocked
            mode: Mode::Idle,
            machine: [0.0; 3],
            wco: [0.0; 3],
            feed: 0.0,
            spindle: 0.0,
            relative: false,
            rx: Vec::new(),
            lines: 0,
            fail: BTreeMap::new(),
            busy: 0,
            reports: 0,
            log: Vec::new(),
            resets: 0,
        }
    }

    /// Starts in the alarm that asks for homing (`ALARM:11` in grblHAL, `ALARM:` is the lock of GRBL).
    pub fn locked(mut self) -> GrblDevice {
        self.mode = Mode::Alarm(11);
        self
    }

    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    pub fn position(&self) -> [f64; 3] {
        self.machine
    }

    pub fn alarm(&self) -> Option<u32> {
        match self.mode {
            Mode::Alarm(n) => Some(n),
            _ => None,
        }
    }

    pub fn spindle_rpm(&self) -> f64 {
        self.spindle
    }

    fn greeting(&self) -> String {
        match self.flavor {
            Flavor::Grbl => "Grbl 1.1h ['$' for help]".to_string(),
            Flavor::GrblHal => "GrblHAL 1.1f ['$' for help]".to_string(),
        }
    }

    fn report(&mut self) -> String {
        self.reports += 1;
        let head = match self.mode {
            Mode::Alarm(_) => "Alarm".to_string(),
            Mode::Hold => "Hold:0".to_string(),
            Mode::Idle if self.busy > 0 => {
                self.busy -= 1;
                "Run".to_string()
            }
            Mode::Idle => "Idle".to_string(),
        };
        let m = self.machine;
        let mut s = format!(
            "<{head}|MPos:{:.3},{:.3},{:.3}|FS:{:.0},{:.0}",
            m[0], m[1], m[2], self.feed, self.spindle
        );
        if self.flavor == Flavor::GrblHal {
            s.push_str("|Pn:");
            if self.spindle > 0.0 {
                s.push_str("|A:S");
            }
        }
        if self.reports % 3 == 1 {
            s.push_str(&format!(
                "|WCO:{:.3},{:.3},{:.3}",
                self.wco[0], self.wco[1], self.wco[2]
            ));
        }
        s.push_str(">\n");
        s
    }

    fn inside(target: [f64; 3]) -> bool {
        target[0] >= 0.0
            && target[0] <= TRAVEL_MM[0]
            && target[1] >= 0.0
            && target[1] <= TRAVEL_MM[1]
            && target[2] <= 0.0
            && target[2] >= -TRAVEL_MM[2]
    }

    /// The words of a line: letter and number, in order. `None` when a letter has no number or the number is not one.
    fn words(text: &str) -> Result<Vec<(char, f64)>, u32> {
        let mut out = Vec::new();
        let b: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        let mut i = 0;
        while i < b.len() {
            let letter = b[i];
            if !letter.is_ascii_alphabetic() {
                return Err(1);
            }
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_digit() || matches!(b[j], '.' | '-' | '+')) {
                j += 1;
            }
            if j == i + 1 {
                return Err(2);
            }
            let n: f64 = b[i + 1..j]
                .iter()
                .collect::<String>()
                .parse()
                .map_err(|_| 2u32)?;
            out.push((letter.to_ascii_uppercase(), n));
            i = j;
        }
        Ok(out)
    }

    fn gcode(&mut self, text: &str, jog: bool) -> String {
        self.lines += usize::from(!jog);
        if let Some(&code) = self.fail.get(&self.lines) {
            if !jog {
                return format!("error:{code}\n");
            }
        }
        if let Mode::Alarm(_) = self.mode {
            return "error:9\n".to_string();
        }
        let upper = text.to_ascii_uppercase();
        let words = match Self::words(&upper) {
            Ok(w) => w,
            Err(code) => return format!("error:{code}\n"),
        };
        let mut target = self.machine;
        let mut feed = None;
        let mut moved = false;
        let mut relative = self.relative;
        let mut spindle = None;
        for (letter, n) in &words {
            match (*letter, *n) {
                ('G', g) if g == 0.0 || g == 1.0 => moved = true,
                ('G', g) if g == 2.0 || g == 3.0 => return "error:20\n".to_string(),
                ('G', 90.0) => relative = false,
                ('G', 91.0) => relative = true,
                ('G', g)
                    if [4.0, 17.0, 18.0, 19.0, 20.0, 21.0, 54.0, 92.0, 94.0, 10.0].contains(&g) => {
                }
                ('G', _) => return "error:20\n".to_string(),
                ('M', m) if m == 3.0 || m == 4.0 || m == 5.0 => {
                    spindle = Some(m);
                }
                ('M', m) if [0.0, 2.0, 7.0, 8.0, 9.0, 30.0].contains(&m) => {}
                ('M', m) if m == 6.0 && self.flavor == Flavor::GrblHal => {}
                ('M', _) => return "error:20\n".to_string(),
                ('F', f) => feed = Some(f),
                ('S', s) => {
                    if spindle.is_none() {
                        spindle = Some(3.0);
                    }
                    self.spindle = s;
                }
                ('X' | 'Y' | 'Z' | 'P' | 'L' | 'N' | 'T', _) => {}
                _ => return "error:20\n".to_string(),
            }
        }
        if moved || jog {
            let has_feed = feed.is_some() || self.feed > 0.0;
            for (letter, n) in &words {
                let ax = match letter {
                    'X' => 0,
                    'Y' => 1,
                    'Z' => 2,
                    _ => continue,
                };
                target[ax] = if relative || jog && text.contains("G91") {
                    self.machine[ax] + n
                } else {
                    *n + self.wco[ax]
                };
            }
            if !jog && words.iter().any(|(l, n)| *l == 'G' && *n == 1.0) && !has_feed {
                return "error:22\n".to_string();
            }
            if !Self::inside(target) {
                if jog {
                    return "error:15\n".to_string();
                }
                self.mode = Mode::Alarm(2);
                return "ALARM:2\n".to_string();
            }
            self.machine = target;
            self.busy = 1;
        } else if words.iter().any(|(l, n)| *l == 'G' && *n == 10.0) {
            // G10 L20 P1 X0 Y0: the work position here is what the line says
            for (letter, n) in &words {
                let ax = match letter {
                    'X' => 0,
                    'Y' => 1,
                    'Z' => 2,
                    _ => continue,
                };
                self.wco[ax] = self.machine[ax] - n;
            }
        }
        if !jog {
            self.relative = relative;
        }
        if let Some(f) = feed {
            self.feed = f;
        }
        match spindle {
            Some(5.0) => self.spindle = 0.0,
            Some(_) => {}
            None => {}
        }
        "ok\n".to_string()
    }

    fn line(&mut self, raw: &str) -> String {
        let text = raw.trim();
        if text.is_empty() {
            return String::new();
        }
        self.log.push(text.to_string());
        let up = text.to_ascii_uppercase();
        if up == "$I" {
            let mut s = String::from("[VER:1.1h.20190825:]\n[OPT:V,15,128]\n");
            if self.flavor == Flavor::GrblHal {
                s = String::from(
                    "[VER:1.1f.20240401:]\n[OPT:V,15,128]\n[NEWOPT:ENC,RT+,TMC]\n[FIRMWARE:grblHAL]\n",
                );
            }
            s.push_str("ok\n");
            return s;
        }
        if up == "$$" {
            return format!(
                "$100=250.000\n$130={:.3}\n$131={:.3}\n$132={:.3}\nok\n",
                TRAVEL_MM[0], TRAVEL_MM[1], TRAVEL_MM[2]
            );
        }
        if up == "$X" {
            self.mode = Mode::Idle;
            return "[MSG:Caution: Unlocked]\nok\n".to_string();
        }
        if up == "$H" {
            if matches!(self.mode, Mode::Hold) {
                return "error:8\n".to_string();
            }
            self.mode = Mode::Idle;
            self.machine = [0.0; 3];
            return "ok\n".to_string();
        }
        if let Some(rest) = up.strip_prefix("$J=") {
            if let Mode::Alarm(_) = self.mode {
                return "error:9\n".to_string();
            }
            let line = rest.to_string();
            return self.gcode(&line, true);
        }
        if up.starts_with('$') {
            return "ok\n".to_string();
        }
        self.gcode(text, false)
    }
}

impl Device for GrblDevice {
    fn input(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = String::new();
        for &b in bytes {
            match b {
                b'?' => out.push_str(&self.report()),
                b'!' => {
                    if self.mode == Mode::Idle {
                        self.mode = Mode::Hold;
                    }
                }
                b'~' => {
                    if self.mode == Mode::Hold {
                        self.mode = Mode::Idle;
                    }
                }
                0x18 => {
                    self.resets += 1;
                    self.rx.clear();
                    self.spindle = 0.0;
                    self.relative = false;
                    // a reset in the middle of a move loses the position: the firmware says so with an alarm
                    self.mode = if self.busy > 0 {
                        Mode::Alarm(3)
                    } else if self.mode == Mode::Hold {
                        Mode::Idle
                    } else {
                        self.mode
                    };
                    out.push_str(&format!("\r\n{}\r\n", self.greeting()));
                    if let Mode::Alarm(n) = self.mode {
                        out.push_str(&format!("ALARM:{n}\n"));
                    }
                }
                0x85 => {
                    self.busy = 0;
                }
                b'\n' => {
                    let text = String::from_utf8_lossy(&std::mem::take(&mut self.rx)).to_string();
                    out.push_str(&self.line(&text));
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
    use crate::grbl::GrblController;
    use crate::wire::{Impair, WireLink};
    use crate::{Command, Controller, Error, Family, State};

    fn controller(flavor: Flavor, impair: Impair) -> GrblController<WireLink<GrblDevice>> {
        let family = match flavor {
            Flavor::Grbl => Family::Grbl,
            Flavor::GrblHal => Family::GrblHal,
        };
        GrblController::new(WireLink::new(GrblDevice::new(flavor), impair)).as_family(family)
    }

    #[test]
    fn a_line_of_g_code_moves_the_simulated_machine_and_the_status_tells_where() {
        let mut c = controller(Flavor::Grbl, Impair::clean());
        c.send_line("G21 G90").unwrap();
        c.send_line("G1 X10 Y5 F300").unwrap();
        let s = c.status().unwrap();
        assert_eq!(&s.machine_mm[..3], &[10.0, 5.0, 0.0]);
        assert_eq!(s.state, State::Run);
        assert_eq!(c.status().unwrap().state, State::Idle);
        assert_eq!(s.feed_mm_min, 300.0);
    }

    #[test]
    fn a_comment_with_a_character_the_protocol_takes_is_not_sent() {
        let mut c = controller(Flavor::Grbl, Impair::clean());
        c.send_line("G1 X1 F100 (tool change!)").unwrap();
        c.send_line("; only a comment ?").unwrap();
        let heard = c.transport().heard_text();
        assert!(!heard.contains('!') && !heard.contains('?'), "{heard:?}");
    }

    #[test]
    fn a_move_past_the_travel_is_an_alarm_and_the_lock_is_cleared_with_unlock() {
        let mut c = controller(Flavor::Grbl, Impair::clean());
        let e = c.send_line("G0 X900").unwrap_err();
        assert!(matches!(e, Error::Alarm(_)), "{e:?}");
        assert_eq!(e.code(), Some(2));
        assert_eq!(c.status().unwrap().state, State::Alarm);
        assert_eq!(c.send_line("G0 X1").unwrap_err().code(), Some(9));
        c.command(Command::Unlock).unwrap();
        c.send_line("G0 X1").unwrap();
    }

    #[test]
    fn a_line_fails_with_the_code_it_was_given_by_number() {
        let mut link = WireLink::new(GrblDevice::new(Flavor::Grbl), Impair::clean());
        link.device_mut().fail.insert(2, 22);
        link.device_mut().fail.insert(4, 33);
        let mut c = GrblController::new(link);
        assert!(c.send_line("G21").is_ok());
        let e = c.send_line("G1 X1").unwrap_err();
        assert_eq!(e, Error::Refused("error 22".into()));
        assert_eq!(e.code(), Some(22));
        assert!(e.official_text().is_some_and(|t| !t.is_empty()));
        assert!(c.send_line("G90").is_ok());
        assert_eq!(c.send_line("G1 X2 F5").unwrap_err().code(), Some(33));
    }

    #[test]
    fn grblhal_says_so_in_its_build_and_starts_locked_until_it_is_unlocked() {
        let link = WireLink::new(GrblDevice::new(Flavor::GrblHal).locked(), Impair::clean());
        let mut c = GrblController::new(link).as_family(Family::GrblHal);
        assert_eq!(c.family(), Family::GrblHal);
        assert_eq!(c.status().unwrap().state, State::Alarm);
        assert_eq!(c.send_line("G0 X1").unwrap_err().code(), Some(9));
        c.command(Command::Unlock).unwrap();
        c.send_line("G0 X1").unwrap();
    }

    #[test]
    fn grblhal_status_has_the_pins_and_the_accessories_and_the_reader_ignores_what_it_does_not_know(
    ) {
        let mut c = controller(Flavor::GrblHal, Impair::clean());
        c.command(Command::Spindle {
            rpm: 8000.0,
            clockwise: true,
        })
        .unwrap();
        let s = c.status().unwrap();
        assert_eq!(s.spindle_rpm, 8000.0);
        assert_eq!(s.state, State::Idle);
    }

    #[test]
    fn the_travel_comes_from_the_settings_of_the_device() {
        let mut c = controller(Flavor::GrblHal, Impair::clean());
        assert_eq!(c.max_travel().unwrap(), Some(TRAVEL_MM));
    }

    #[test]
    fn m6_is_refused_by_grbl_and_taken_by_grblhal() {
        let mut g = controller(Flavor::Grbl, Impair::clean());
        assert_eq!(g.send_line("T1 M6").unwrap_err().code(), Some(20));
        let mut h = controller(Flavor::GrblHal, Impair::clean());
        assert!(h.send_line("T1 M6").is_ok());
    }

    #[test]
    fn a_reset_while_the_machine_moves_raises_the_alarm_of_a_lost_position() {
        let mut c = controller(Flavor::Grbl, Impair::clean());
        c.send_line("G1 X20 F100").unwrap();
        c.command(Command::EmergencyStop).unwrap();
        // the greeting is expected after the reset this program sent, and the alarm follows
        let s = c.status().unwrap();
        assert_eq!(s.state, State::Alarm);
        assert_eq!(c.transport().device().resets, 1);
    }
}
