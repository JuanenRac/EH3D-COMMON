//! GRBL 1.1 over a line transport (a serial port in practice): the status report, the orders and the lines of G-code, as the free firmware
//! defines them. grblHAL and FluidNC speak the same protocol, so the same controller drives them (each reports itself as its own family).
//!
//! The lines are sent one at a time and each one waits for its `ok` (or `error:N`): simple and safe, not the fastest way to stream.

use std::io;
use std::time::Duration;

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

pub struct GrblController<T: LineTransport> {
    transport: T,
    family: Family,
    timeout: Duration,
    /// The offset of the work from the machine, which GRBL reports from time to time (`WCO:`).
    wco: [f64; AXES],
}

impl<T: LineTransport> GrblController<T> {
    pub fn new(transport: T) -> Self {
        GrblController {
            transport,
            family: Family::Grbl,
            timeout: Duration::from_secs(5),
            wco: [0.0; AXES],
        }
    }

    /// The same protocol reported as another family (grblHAL, FluidNC).
    pub fn as_family(mut self, family: Family) -> Self {
        self.family = family;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    fn link(e: io::Error) -> Error {
        Error::Link(e.to_string())
    }

    /// Waits for the end of an order: `ok` is done; `error:N` and `ALARM:N` are refused; the other lines (messages, reports) are skipped.
    fn wait_ok(&mut self) -> Result<(), Error> {
        loop {
            match self.transport.read_line(self.timeout).map_err(Self::link)? {
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
        self.transport
            .write_all(format!("{text}\n").as_bytes())
            .map_err(Self::link)?;
        self.wait_ok()
    }

    fn byte(&mut self, b: u8) -> Result<(), Error> {
        self.transport.write_all(&[b]).map_err(Self::link)
    }
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
            match self.transport.read_line(self.timeout).map_err(Self::link)? {
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
                // hold first so that the machine stops moving, then reset to clear what is queued
                self.byte(HOLD)?;
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

    fn send_line(&mut self, line: &str) -> Result<(), Error> {
        let l = line.trim();
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
        assert_eq!(c.transport().written, vec![b'!', 0x18]);
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
