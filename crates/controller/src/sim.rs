//! A controller that is not there: a machine in memory that takes the orders and the lines of G-code of a real one and answers as one
//! would. It lets a machine-control program be tried (jog, zero, a program run, the alarms) with nothing connected.
//!
//! The G-code it understands is the basic one: `G0`/`G1` (with `X Y Z A` and `F`), `G90`/`G91`, `G20`/`G21`, `G92` (the work zero),
//! `M3`/`M4`/`M5` with `S`, `M7`/`M8`/`M9`. A line that is anything else is refused with the controller's own kind of message.

use crate::{Capabilities, Command, Controller, Error, Family, State, Status, AXES};

/// The simulated machine.
#[derive(Debug, Clone)]
pub struct SimController {
    /// Where the machine is, in mm (the axes X, Y, Z, A, B, C).
    machine: [f64; AXES],
    /// The machine position of the work zero: work = machine - zero.
    zero: [f64; AXES],
    absolute: bool,
    inches: bool,
    feed: f64,
    spindle_rpm: f64,
    spindle_on: bool,
    coolant: bool,
    state: State,
    alarm: Option<String>,
    /// The limits of the travel, in mm, per axis (min, max).
    pub travel: [(f64, f64); AXES],
    /// Every line taken, for the tests and the log of the program.
    pub log: Vec<String>,
}

impl Default for SimController {
    fn default() -> Self {
        let mut travel = [(0.0, 0.0); AXES];
        travel[0] = (0.0, 500.0);
        travel[1] = (0.0, 300.0);
        travel[2] = (-100.0, 0.0);
        SimController {
            machine: [0.0; AXES],
            zero: [0.0; AXES],
            absolute: true,
            inches: false,
            feed: 500.0,
            spindle_rpm: 0.0,
            spindle_on: false,
            coolant: false,
            state: State::Idle,
            alarm: None,
            travel,
            log: Vec::new(),
        }
    }
}

impl SimController {
    pub fn new() -> Self {
        Self::default()
    }

    fn axis_of(c: char) -> Option<usize> {
        match c {
            'X' => Some(0),
            'Y' => Some(1),
            'Z' => Some(2),
            'A' => Some(3),
            'B' => Some(4),
            'C' => Some(5),
            _ => None,
        }
    }

    /// Moves to the machine position `to` (mm), refusing what is outside the travel: that is an alarm, as a soft limit is.
    fn go_to(&mut self, to: [f64; AXES]) -> Result<(), Error> {
        for (i, &v) in to.iter().enumerate() {
            let (lo, hi) = self.travel[i];
            if hi > lo && (v < lo - 1e-9 || v > hi + 1e-9) {
                let msg = format!("soft limit on axis {}", ["X", "Y", "Z", "A", "B", "C"][i]);
                self.alarm = Some(msg.clone());
                self.state = State::Alarm;
                return Err(Error::Alarm(msg));
            }
        }
        self.machine = to;
        Ok(())
    }

    fn check_free(&self) -> Result<(), Error> {
        match &self.alarm {
            Some(a) => Err(Error::Alarm(a.clone())),
            None => Ok(()),
        }
    }

    /// The words of a line of G-code: the letter and its number.
    fn words(line: &str) -> Result<Vec<(char, f64)>, Error> {
        let clean = line.split(';').next().unwrap_or("");
        let mut out = Vec::new();
        let mut it = clean.chars().filter(|c| !c.is_whitespace()).peekable();
        while let Some(c) = it.next() {
            if c == '(' {
                for d in it.by_ref() {
                    if d == ')' {
                        break;
                    }
                }
                continue;
            }
            let letter = c.to_ascii_uppercase();
            if !letter.is_ascii_alphabetic() {
                return Err(Error::Refused(format!("error:1 (bad character '{c}')")));
            }
            let mut num = String::new();
            while let Some(&n) = it.peek() {
                if n.is_ascii_digit() || n == '.' || n == '-' || n == '+' {
                    num.push(n);
                    it.next();
                } else {
                    break;
                }
            }
            let v = num
                .parse::<f64>()
                .map_err(|_| Error::Refused(format!("error:2 (no number after {letter})")))?;
            out.push((letter, v));
        }
        Ok(out)
    }
}

impl Controller for SimController {
    fn family(&self) -> Family {
        Family::Grbl
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            axes: 4,
            spindle: true,
            spindle_speed: true,
            coolant: true,
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
        let mut work = [0.0; AXES];
        for (w, (m, z)) in work.iter_mut().zip(self.machine.iter().zip(&self.zero)) {
            *w = m - z;
        }
        Ok(Status {
            state: self.state,
            machine_mm: self.machine,
            work_mm: work,
            feed_mm_min: self.feed,
            spindle_rpm: if self.spindle_on {
                self.spindle_rpm
            } else {
                0.0
            },
            buffer_percent: Some(0),
            pins: String::new(),
        })
    }

    fn command(&mut self, command: Command) -> Result<(), Error> {
        match command {
            Command::EmergencyStop => {
                self.spindle_on = false;
                self.coolant = false;
                self.alarm = Some("emergency stop".to_string());
                self.state = State::Alarm;
                Ok(())
            }
            Command::Unlock => {
                self.alarm = None;
                self.state = State::Idle;
                Ok(())
            }
            Command::Stop => {
                self.check_free()?;
                self.state = State::Idle;
                self.spindle_on = false;
                Ok(())
            }
            Command::Pause => {
                self.check_free()?;
                self.state = State::Hold;
                Ok(())
            }
            Command::Resume => {
                self.check_free()?;
                self.state = State::Idle;
                Ok(())
            }
            Command::JogStop => Ok(()),
            Command::Jog {
                axis, distance_mm, ..
            } => {
                self.check_free()?;
                if axis >= AXES {
                    return Err(Error::Unsupported("that axis"));
                }
                let mut to = self.machine;
                to[axis] += distance_mm;
                self.go_to(to)
            }
            Command::Home(axes) => {
                self.check_free()?;
                let all: Vec<usize> = if axes.is_empty() {
                    (0..3).collect()
                } else {
                    axes
                };
                let mut to = self.machine;
                for a in all {
                    if a < AXES {
                        // Z goes up to its top and X and Y to their zero, as the usual machine homes
                        to[a] = if a == 2 {
                            self.travel[2].1
                        } else {
                            self.travel[a].0
                        };
                    }
                }
                self.go_to(to)
            }
            Command::ZeroWork(axes) => {
                let all: Vec<usize> = if axes.is_empty() {
                    (0..AXES).collect()
                } else {
                    axes
                };
                for a in all {
                    if a < AXES {
                        self.zero[a] = self.machine[a];
                    }
                }
                Ok(())
            }
            Command::Spindle { rpm, clockwise: _ } => {
                self.check_free()?;
                self.spindle_rpm = rpm.max(0.0);
                self.spindle_on = rpm > 0.0;
                Ok(())
            }
            Command::Coolant(on) => {
                self.check_free()?;
                self.coolant = on;
                Ok(())
            }
        }
    }

    fn send_line(&mut self, line: &str) -> Result<(), Error> {
        self.check_free()?;
        self.log.push(line.to_string());
        let words = Self::words(line)?;
        let mut target_present = [None::<f64>; AXES];
        for &(l, v) in &words {
            match l {
                'G' => match v as i32 {
                    0 | 1 => {}
                    90 => self.absolute = true,
                    91 => self.absolute = false,
                    20 => self.inches = true,
                    21 => self.inches = false,
                    92 => {}
                    _ => return Err(Error::Refused(format!("error:20 (G{v} is not supported)"))),
                },
                'M' => match v as i32 {
                    3 | 4 => self.spindle_on = true,
                    5 => self.spindle_on = false,
                    7 | 8 => self.coolant = true,
                    9 => self.coolant = false,
                    30 | 2 => self.state = State::Idle,
                    _ => return Err(Error::Refused(format!("error:20 (M{v} is not supported)"))),
                },
                'F' => self.feed = v * if self.inches { 25.4 } else { 1.0 },
                'S' => self.spindle_rpm = v,
                'N' | 'T' => {}
                other => {
                    if let Some(i) = Self::axis_of(other) {
                        target_present[i] = Some(v);
                    } else {
                        return Err(Error::Refused(format!("error:20 (unknown word {other})")));
                    }
                }
            }
        }
        // G92 sets the work zero where the machine is, to the values given
        if words.iter().any(|&(l, v)| l == 'G' && v as i32 == 92) {
            for (i, t) in target_present.iter().enumerate() {
                if let Some(v) = t {
                    let scale = if self.inches { 25.4 } else { 1.0 };
                    self.zero[i] = self.machine[i] - v * scale;
                }
            }
            return Ok(());
        }
        if target_present.iter().any(Option::is_some) {
            let scale = if self.inches { 25.4 } else { 1.0 };
            let mut to = self.machine;
            for (i, t) in target_present.iter().enumerate() {
                if let Some(v) = t {
                    to[i] = if self.absolute {
                        self.zero[i] + v * scale
                    } else {
                        self.machine[i] + v * scale
                    };
                }
            }
            self.go_to(to)?;
            self.state = State::Idle;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jog_moves_the_machine_and_the_work_zero_follows_it() {
        let mut s = SimController::new();
        s.command(Command::Jog {
            axis: 0,
            distance_mm: 10.0,
            feed_mm_min: 500.0,
        })
        .unwrap();
        s.command(Command::ZeroWork(vec![0])).unwrap();
        s.command(Command::Jog {
            axis: 0,
            distance_mm: 5.0,
            feed_mm_min: 500.0,
        })
        .unwrap();
        let st = s.status().unwrap();
        assert_eq!((st.machine_mm[0], st.work_mm[0]), (15.0, 5.0));
    }

    #[test]
    fn g_code_moves_in_absolute_and_relative_and_in_inches() {
        let mut s = SimController::new();
        s.send_line("G21 G90 G1 X10 Y20 F300").unwrap();
        let st = s.status().unwrap();
        assert_eq!(
            (st.machine_mm[0], st.machine_mm[1], st.feed_mm_min),
            (10.0, 20.0, 300.0)
        );
        s.send_line("G91 G0 X5").unwrap();
        assert_eq!(s.status().unwrap().machine_mm[0], 15.0);
        s.send_line("G20 G90 G0 X1").unwrap();
        assert!((s.status().unwrap().machine_mm[0] - 25.4).abs() < 1e-9);
        // G92 makes the work zero here
        s.send_line("G21 G92 X0 Y0").unwrap();
        let st = s.status().unwrap();
        assert_eq!((st.work_mm[0], st.work_mm[1]), (0.0, 0.0));
        s.send_line("G90 G1 X10").unwrap();
        assert!((s.status().unwrap().machine_mm[0] - 35.4).abs() < 1e-9);
    }

    #[test]
    fn the_spindle_and_the_coolant_answer_to_m_codes() {
        let mut s = SimController::new();
        s.send_line("M3 S12000").unwrap();
        assert_eq!(s.status().unwrap().spindle_rpm, 12000.0);
        s.send_line("M5").unwrap();
        assert_eq!(s.status().unwrap().spindle_rpm, 0.0);
        s.send_line("M8 (flood)").unwrap();
        s.send_line("M9 ; off").unwrap();
    }

    #[test]
    fn what_goes_out_of_the_travel_is_an_alarm_until_it_is_unlocked() {
        let mut s = SimController::new();
        let e = s.send_line("G0 X900").unwrap_err();
        assert!(matches!(e, Error::Alarm(_)));
        assert_eq!(s.status().unwrap().state, State::Alarm);
        // nothing is taken while it is in alarm
        assert!(matches!(s.send_line("G0 X1"), Err(Error::Alarm(_))));
        s.command(Command::Unlock).unwrap();
        s.send_line("G0 X1").unwrap();
    }

    #[test]
    fn the_emergency_stop_cuts_the_spindle_and_blocks_everything() {
        let mut s = SimController::new();
        s.send_line("M3 S5000").unwrap();
        s.command(Command::EmergencyStop).unwrap();
        let st = s.status().unwrap();
        assert_eq!((st.state, st.spindle_rpm), (State::Alarm, 0.0));
        assert!(s
            .command(Command::Jog {
                axis: 0,
                distance_mm: 1.0,
                feed_mm_min: 100.0
            })
            .is_err());
    }

    #[test]
    fn a_line_that_is_not_understood_is_refused_not_ignored() {
        let mut s = SimController::new();
        assert!(matches!(s.send_line("G38.2 Z-5"), Err(Error::Refused(_))));
        assert!(matches!(s.send_line("Q5"), Err(Error::Refused(_))));
        assert!(matches!(s.send_line("X"), Err(Error::Refused(_))));
        // a comment alone is fine
        s.send_line("(just a comment)").unwrap();
    }

    #[test]
    fn homing_goes_to_the_start_of_x_and_y_and_the_top_of_z() {
        let mut s = SimController::new();
        s.send_line("G0 X50 Y60 Z-20").unwrap();
        s.command(Command::Home(vec![])).unwrap();
        let st = s.status().unwrap();
        assert_eq!(
            (st.machine_mm[0], st.machine_mm[1], st.machine_mm[2]),
            (0.0, 0.0, 0.0)
        );
    }
}
