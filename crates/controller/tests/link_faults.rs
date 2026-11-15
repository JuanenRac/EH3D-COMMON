//! Faults of the link with a GRBL board and the way back from them, against a twin of the board that lives in memory.
//!
//! The twin answers as a board does (`ok`, `error:N`, `ALARM:N`, the status report) and fails when the plan says so: a reply that never comes, a line of noise,
//! a refusal, an alarm, a link that is cut. The plan is fixed or comes from a seeded generator, so every run is the same. Nothing here has run on a real board.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::rc::Rc;
use std::time::Duration;

use eh3d_controller::grbl::{GrblController, LineTransport};
use eh3d_controller::{Command, Controller, Error, State};

/// What the twin does with a line of G-code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Answer `ok`.
    None,
    /// Say nothing.
    Silence,
    /// A line of noise, then `ok`.
    Noise,
    /// `error:20`.
    Refuse,
    /// `ALARM:1`, and the board is locked until `$X`.
    Alarm,
    /// The link is cut.
    Cut,
}

#[derive(Default)]
struct Board {
    plan: VecDeque<Fault>,
    out: VecDeque<String>,
    written: Vec<u8>,
    locked: bool,
    cut: bool,
    lines: Vec<String>,
}

#[derive(Clone)]
struct Link(Rc<RefCell<Board>>);

impl Link {
    fn new(plan: &[Fault]) -> Link {
        Link(Rc::new(RefCell::new(Board {
            plan: plan.iter().copied().collect(),
            ..Board::default()
        })))
    }

    fn mend(&self) {
        self.0.borrow_mut().cut = false;
    }

    fn written(&self) -> Vec<u8> {
        self.0.borrow().written.clone()
    }

    fn lines(&self) -> Vec<String> {
        self.0.borrow().lines.clone()
    }
}

impl LineTransport for Link {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut b = self.0.borrow_mut();
        if b.cut {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "link cut"));
        }
        b.written.extend_from_slice(bytes);
        if bytes == b"?" {
            let head = if b.locked { "Alarm" } else { "Idle" };
            b.out
                .push_back(format!("<{head}|MPos:1.000,2.000,3.000|FS:0,0>"));
            return Ok(());
        }
        if (bytes.len() == 1 && !bytes[0].is_ascii_graphic()) || bytes == b"!" || bytes == b"~" {
            return Ok(());
        }
        let line = String::from_utf8_lossy(bytes).trim().to_string();
        b.lines.push(line.clone());
        if line == "$X" {
            b.locked = false;
            b.out.push_back("ok".into());
            return Ok(());
        }
        if b.locked {
            b.out.push_back("error:9".into());
            return Ok(());
        }
        match b.plan.pop_front().unwrap_or(Fault::None) {
            Fault::None => b.out.push_back("ok".into()),
            Fault::Silence => {}
            Fault::Noise => {
                b.out.push_back("[MSG:Caution: Unlocked]".into());
                b.out.push_back("ok".into());
            }
            Fault::Refuse => b.out.push_back("error:20".into()),
            Fault::Alarm => {
                b.locked = true;
                b.out.push_back("ALARM:1".into());
            }
            Fault::Cut => {
                b.cut = true;
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "link cut"));
            }
        }
        Ok(())
    }

    fn read_line(&mut self, _timeout: Duration) -> io::Result<Option<String>> {
        let mut b = self.0.borrow_mut();
        if b.cut {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "link cut"));
        }
        Ok(b.out.pop_front())
    }
}

fn controller(plan: &[Fault]) -> (GrblController<Link>, Link) {
    let link = Link::new(plan);
    (GrblController::new(link.clone()), link)
}

/// A small deterministic generator (SplitMix64): the same seed always gives the same numbers.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[test]
fn a_line_of_noise_before_the_ok_is_skipped() {
    let (mut c, _) = controller(&[Fault::Noise]);
    assert_eq!(c.send_line("G0 X1"), Ok(()));
}

#[test]
fn a_reply_that_never_comes_is_a_timeout_and_the_machine_is_held() {
    let (mut c, link) = controller(&[Fault::Silence]);
    assert_eq!(c.send_line("G1 X5 F100"), Err(Error::Timeout));
    assert!(
        link.written().contains(&b'!'),
        "the feed hold byte was not sent"
    );
    // the board is still there: the next line goes through
    assert_eq!(c.send_line("G0 X0"), Ok(()));
}

#[test]
fn a_refusal_carries_its_number_and_the_text_of_the_manual() {
    let (mut c, _) = controller(&[Fault::Refuse]);
    let error = c.send_line("G99").unwrap_err();
    assert!(matches!(error, Error::Refused(_)));
    assert_eq!(error.code(), Some(20));
    assert!(error.official_text().unwrap().contains("Unsupported"));
}

#[test]
fn an_alarm_locks_the_board_until_it_is_unlocked() {
    let (mut c, link) = controller(&[Fault::Alarm]);
    let error = c.send_line("G0 X1000").unwrap_err();
    assert!(matches!(error, Error::Alarm(_)));
    assert_eq!(c.status().unwrap().state, State::Alarm);
    // locked: a line is refused with the number the board gives for it
    assert_eq!(c.send_line("G0 X1").unwrap_err().code(), Some(9));
    // the way back
    assert_eq!(c.command(Command::Unlock), Ok(()));
    assert_eq!(c.status().unwrap().state, State::Idle);
    assert_eq!(c.send_line("G0 X1"), Ok(()));
    assert_eq!(link.lines().iter().filter(|l| *l == "$X").count(), 1);
}

#[test]
fn a_cut_link_is_a_link_error_and_a_mended_one_works_again() {
    let (mut c, link) = controller(&[Fault::None, Fault::Cut]);
    assert_eq!(c.send_line("G0 X1"), Ok(()));
    assert!(matches!(c.send_line("G0 X2"), Err(Error::Link(_))));
    // while it is cut, nothing works, and nothing panics
    assert!(matches!(c.status(), Err(Error::Link(_))));
    assert!(matches!(c.command(Command::Unlock), Err(Error::Link(_))));
    link.mend();
    assert_eq!(c.status().unwrap().state, State::Idle);
    assert_eq!(c.send_line("G0 X3"), Ok(()));
}

#[test]
fn the_emergency_stop_needs_no_answer() {
    let (mut c, link) = controller(&[]);
    assert_eq!(c.command(Command::EmergencyStop), Ok(()));
    assert_eq!(link.written(), vec![0x18]);
}

fn outcomes(seed: u64) -> Vec<String> {
    let mut rng = Seeded(seed);
    let plan: Vec<Fault> = (0..120)
        .map(|_| match rng.next() % 10 {
            0 => Fault::Silence,
            1 => Fault::Noise,
            2 => Fault::Refuse,
            3 => Fault::Alarm,
            _ => Fault::None,
        })
        .collect();
    let (mut c, _) = controller(&plan);
    let mut out = Vec::new();
    for n in 0..120 {
        let result = c.send_line(&format!("G1 X{n} F200"));
        out.push(format!("{result:?}"));
        // the way back from an alarm, as an operator does it
        if matches!(result, Err(Error::Alarm(_))) {
            out.push(format!("{:?}", c.command(Command::Unlock)));
        }
    }
    out
}

#[test]
fn a_seeded_run_of_every_fault_is_the_same_every_time_and_ends_in_a_working_board() {
    let first = outcomes(2025);
    assert_eq!(first, outcomes(2025));
    assert_ne!(first, outcomes(2026));
    assert!(first.iter().any(|o| o == "Ok(())"));
    assert!(first.iter().any(|o| o.starts_with("Err(Timeout)")));
    assert!(first.iter().any(|o| o.starts_with("Err(Refused")));
    assert!(first.iter().any(|o| o.starts_with("Err(Alarm")));
}
