//! A controller that goes wrong on purpose, to try what the program does when the real one does: it wraps any controller and, by a
//! schedule, falls silent, says it is busy, refuses the line number N, or reports a position that is not true. The order that was given
//! while it was silent still reaches the inner controller (that is what a real silence is: the order went and the answer did not come
//! back), so a test can tell whether a program sent it again. Nothing here is a machine.

use crate::{Capabilities, Command, Controller, Error, Family, Status, AXES};

/// One thing that goes wrong. A call is counted from 0 over the three kinds of call (`status`, `command` and `send_line`).
#[derive(Debug, Clone, PartialEq)]
pub enum Fault {
    /// From the call `from`, for `calls` calls (for ever when `None`), the answer never comes: [`Error::Timeout`]. An order still reaches
    /// the inner controller, as it would on a real link that loses the answers.
    Silence { from: usize, calls: Option<usize> },
    /// From the call `from`, for `calls` calls, the controller says it is busy and takes nothing: the order does not reach the inner one.
    Busy { from: usize, calls: usize },
    /// The line number `line` (from 1) sent with `send_line` is refused with `error:CODE`.
    ErrorAtLine { line: usize, code: u32 },
    /// From the call `from`, the position of every status is moved by this much (mm).
    FalsePosition { from: usize, offset_mm: [f64; AXES] },
}

/// A controller with faults.
pub struct FaultyController<C: Controller> {
    inner: C,
    faults: Vec<Fault>,
    calls: usize,
    lines: usize,
    /// How many orders (commands and lines) reached the inner controller, and what they were, in order.
    pub delivered: Vec<String>,
}

fn within(call: usize, from: usize, calls: Option<usize>) -> bool {
    call >= from && calls.is_none_or(|n| call < from.saturating_add(n))
}

impl<C: Controller> FaultyController<C> {
    pub fn new(inner: C, faults: Vec<Fault>) -> Self {
        FaultyController {
            inner,
            faults,
            calls: 0,
            lines: 0,
            delivered: Vec::new(),
        }
    }

    pub fn inner(&self) -> &C {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    /// The calls made so far.
    pub fn calls(&self) -> usize {
        self.calls
    }

    fn silent(&self, call: usize) -> bool {
        self.faults
            .iter()
            .any(|f| matches!(f, Fault::Silence { from, calls } if within(call, *from, *calls)))
    }

    fn busy(&self, call: usize) -> bool {
        self.faults
            .iter()
            .any(|f| matches!(f, Fault::Busy { from, calls } if within(call, *from, Some(*calls))))
    }
}

impl<C: Controller> Controller for FaultyController<C> {
    fn family(&self) -> Family {
        self.inner.family()
    }

    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    fn status(&mut self) -> Result<Status, Error> {
        let call = self.calls;
        self.calls += 1;
        if self.silent(call) {
            return Err(Error::Timeout);
        }
        if self.busy(call) {
            return Err(Error::Refused("busy".into()));
        }
        let mut st = self.inner.status()?;
        for f in &self.faults {
            if let Fault::FalsePosition { from, offset_mm } = f {
                if call >= *from {
                    for (i, off) in offset_mm.iter().enumerate() {
                        st.machine_mm[i] += off;
                        st.work_mm[i] += off;
                    }
                }
            }
        }
        Ok(st)
    }

    fn command(&mut self, command: Command) -> Result<(), Error> {
        let call = self.calls;
        self.calls += 1;
        if self.busy(call) {
            return Err(Error::Refused("busy".into()));
        }
        self.delivered.push(format!("{command:?}"));
        let r = self.inner.command(command);
        if self.silent(call) {
            return Err(Error::Timeout);
        }
        r
    }

    fn send_line(&mut self, line: &str) -> Result<(), Error> {
        let call = self.calls;
        self.calls += 1;
        self.lines += 1;
        if self.busy(call) {
            return Err(Error::Refused("busy".into()));
        }
        for f in &self.faults {
            if let Fault::ErrorAtLine { line: n, code } = f {
                if *n == self.lines {
                    return Err(Error::Refused(format!("error:{code}")));
                }
            }
        }
        self.delivered.push(line.to_string());
        let r = self.inner.send_line(line);
        if self.silent(call) {
            return Err(Error::Timeout);
        }
        r
    }

    fn set_travel(&mut self, travel: [(f64, f64); AXES]) -> bool {
        self.inner.set_travel(travel)
    }

    fn max_travel(&mut self) -> Result<Option<[f64; 3]>, Error> {
        self.inner.max_travel()
    }

    fn urgent_channel(&self) -> Option<crate::urgent::UrgentChannel> {
        self.inner.urgent_channel()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::SimController;

    #[test]
    fn a_silence_loses_the_answer_but_the_order_still_reaches_the_machine() {
        let mut f = FaultyController::new(
            SimController::new(),
            vec![Fault::Silence {
                from: 1,
                calls: Some(1),
            }],
        );
        assert!(f.status().is_ok()); // call 0
        assert_eq!(f.send_line("G1 X5 F100"), Err(Error::Timeout)); // call 1: silent
        assert_eq!(f.delivered, vec!["G1 X5 F100".to_string()]);
        let st = f.status().unwrap(); // call 2: the silence is over
        assert_eq!(st.machine_mm[0], 5.0);
    }

    #[test]
    fn a_busy_controller_takes_nothing_and_the_error_comes_at_the_line_asked() {
        let mut f = FaultyController::new(
            SimController::new(),
            vec![
                Fault::Busy { from: 0, calls: 1 },
                Fault::ErrorAtLine { line: 3, code: 20 },
            ],
        );
        assert!(matches!(f.send_line("G1 X1 F100"), Err(Error::Refused(t)) if t == "busy"));
        assert!(f.delivered.is_empty());
        // the first line that went in is the second call of send_line, so line 2 here
        assert!(f.send_line("G1 X1 F100").is_ok());
        let e = f.send_line("G1 X2").unwrap_err();
        assert_eq!(e.code(), Some(20));
        assert_eq!(f.delivered.len(), 1);
    }

    #[test]
    fn a_false_position_moves_what_the_status_says_and_not_the_machine() {
        let mut off = [0.0; AXES];
        off[0] = 40.0;
        let mut f = FaultyController::new(
            SimController::new(),
            vec![Fault::FalsePosition {
                from: 1,
                offset_mm: off,
            }],
        );
        assert_eq!(f.status().unwrap().machine_mm[0], 0.0);
        assert_eq!(f.status().unwrap().machine_mm[0], 40.0);
        assert_eq!(f.inner_mut().status().unwrap().machine_mm[0], 0.0);
        assert_eq!(f.calls(), 2);
    }

    #[test]
    fn a_silence_for_ever_is_for_ever() {
        let mut f = FaultyController::new(
            SimController::new(),
            vec![Fault::Silence {
                from: 2,
                calls: None,
            }],
        );
        let _ = f.status();
        let _ = f.status();
        for _ in 0..50 {
            assert_eq!(f.status(), Err(Error::Timeout));
        }
    }
}
