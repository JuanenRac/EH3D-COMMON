//! The Klipper controller against the simulated Klipper behind a cable that misbehaves by a fixed seed: the orders and their words, the
//! refusals by number, the stop that fails closed when the machine falls silent, the noise and the spoiled messages. Nothing here has run on
//! a real Klipper.

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use eh3d_controller::dialtext;
use eh3d_controller::klipper::{
    self, JsonRpcTransport, KlipperController, E_HOME_FIRST, E_MALFORMED, E_RANGE, E_SHUTDOWN,
    E_UNKNOWN,
};
use eh3d_controller::klippersim::{KlipperDevice, RpcLink, TRAVEL_MM};
use eh3d_controller::urgent::Urgent;
use eh3d_controller::wire::Impair;
use eh3d_controller::{Command, Controller, Error, Family, State};

type Sim = KlipperController<RpcLink<KlipperDevice>>;

fn controller(impair: Impair) -> Sim {
    KlipperController::new(RpcLink::new(KlipperDevice::new(), impair))
}

/// A controller that does not wait for a machine that is busy: with a link that loses answers the test ends at once.
fn quick(c: Sim) -> Sim {
    c.with_busy_limit(Duration::ZERO)
}

fn homed(impair: Impair) -> Sim {
    let mut c = controller(Impair::clean());
    c.command(Command::Home(vec![])).unwrap();
    // the cable of the test starts to misbehave once the machine is homed
    let device = std::mem::take(c.transport_mut().device_mut());
    KlipperController::new(RpcLink::new(device, impair))
}

fn device(c: &Sim) -> &KlipperDevice {
    c.transport().device()
}

#[test]
fn it_is_klipper_and_says_what_it_can_do() {
    let c = controller(Impair::clean());
    assert_eq!(c.family(), Family::Klipper);
    let caps = c.capabilities();
    assert!(caps.gcode && caps.homing && caps.emergency_stop && caps.pause);
    assert!(!caps.spindle && !caps.coolant && caps.axes == 3);
    assert!(c.urgent_channel().is_some());
}

#[test]
fn homing_and_a_move_show_in_the_status_and_the_state_goes_from_run_to_idle() {
    let mut c = controller(Impair::clean());
    c.command(Command::Home(vec![])).unwrap();
    assert_eq!(device(&c).is_homed(), [true; 3]);
    c.send_line("G1 X10 Y5 F600").unwrap();
    let s = c.status().unwrap();
    assert_eq!(&s.machine_mm[..3], &[10.0, 5.0, 0.0]);
    assert_eq!(s.state, State::Run);
    assert_eq!(s.feed_mm_min, 600.0);
    assert_eq!(c.status().unwrap().state, State::Idle);
}

#[test]
fn a_jog_goes_back_to_absolute_mode_even_when_the_move_is_refused() {
    let mut c = controller(Impair::clean());
    // not homed: refused, by the number of its kind
    let e = c
        .command(Command::Jog {
            axis: 0,
            distance_mm: 5.0,
            feed_mm_min: 300.0,
        })
        .unwrap_err();
    assert_eq!(e.code(), Some(E_HOME_FIRST));
    assert!(device(&c).is_absolute());
    c.command(Command::Home(vec![0, 1, 2])).unwrap();
    c.command(Command::Jog {
        axis: 1,
        distance_mm: 7.5,
        feed_mm_min: 300.0,
    })
    .unwrap();
    assert_eq!(device(&c).position()[1], 7.5);
    assert!(device(&c).is_absolute());
    // past the travel
    let e = c
        .command(Command::Jog {
            axis: 0,
            distance_mm: 9_000.0,
            feed_mm_min: 300.0,
        })
        .unwrap_err();
    assert_eq!(e.code(), Some(E_RANGE));
    assert!(device(&c).is_absolute());
}

#[test]
fn a_command_klipper_does_not_know_is_refused_with_its_number_and_the_words_in_each_language() {
    let mut c = controller(Impair::clean());
    let e = c.send_line("FOO 1").unwrap_err();
    assert!(matches!(e, Error::Refused(_)));
    assert_eq!(e.code(), Some(E_UNKNOWN));
    for lang in dialtext::LANGS {
        let t = dialtext::explain(Family::Klipper, "error", e.code(), "raw", lang);
        assert_ne!(t, "raw", "{lang}");
    }
    // a comment and an empty line cost nothing
    c.send_line("; only a comment").unwrap();
    c.send_line("(and another)").unwrap();
    assert!(device(&c).executed.is_empty());
}

#[test]
fn zeroing_the_work_keeps_the_position_of_the_machine() {
    let mut c = controller(Impair::clean());
    c.command(Command::Home(vec![])).unwrap();
    c.send_line("G1 X20 Y10 F300").unwrap();
    c.command(Command::ZeroWork(vec![0])).unwrap();
    let s = c.status().unwrap();
    assert_eq!(s.work_mm[0], 0.0);
    assert_eq!(s.machine_mm[0], 20.0);
    assert_eq!(s.work_mm[1], 10.0);
}

#[test]
fn the_travel_is_read_from_the_configuration() {
    let mut c = controller(Impair::clean());
    assert_eq!(c.max_travel().unwrap(), Some(TRAVEL_MM));
}

#[test]
fn the_emergency_stop_shuts_klipper_down_and_everything_is_refused_until_it_is_unlocked() {
    let mut c = homed_clean();
    c.command(Command::EmergencyStop).unwrap();
    assert!(c.halted());
    assert!(device(&c).is_shutdown());
    assert_eq!(c.status().unwrap().state, State::Alarm);
    // refused here, not even sent
    let sent = c.transport().sent;
    let e = c.send_line("G1 X1 F100").unwrap_err();
    assert!(matches!(e, Error::Alarm(_)));
    assert_eq!(e.code(), Some(E_SHUTDOWN));
    assert_eq!(c.transport().sent, sent);
    assert!(c.command(Command::Home(vec![])).is_err());
    // a stop of a machine that is stopped has nothing to do
    c.command(Command::Stop).unwrap();
    c.command(Command::Unlock).unwrap();
    assert!(!c.halted());
    assert_eq!(c.status().unwrap().state, State::Idle);
    assert_eq!(device(&c).emergency_stops, 1);
    assert_eq!(device(&c).restarts, 1);
    // the machine must be homed again
    assert_eq!(
        c.send_line("G1 X1 F100").unwrap_err().code(),
        Some(E_HOME_FIRST)
    );
}

fn homed_clean() -> Sim {
    homed(Impair::clean())
}

#[test]
fn a_stop_with_no_print_to_cancel_is_the_emergency_stop_and_a_stop_of_a_print_is_a_cancel() {
    let mut c = homed_clean();
    c.transport_mut().device_mut().printing = true;
    c.command(Command::Stop).unwrap();
    assert!(!c.halted() && !device(&c).is_shutdown());
    // nothing left to cancel: the only thing that stops what is queued is the emergency stop
    c.command(Command::Stop).unwrap();
    assert!(c.halted() && device(&c).is_shutdown());
}

#[test]
fn a_move_that_gets_no_answer_and_no_answer_to_the_probe_is_stopped_and_the_machine_is_locked() {
    // messages are numbered from 1 by what the program sends: the home was sent before the cable was changed
    let mut c = homed(Impair {
        mute_from: Some(1),
        mute_lines: 2,
        ..Impair::clean()
    });
    let e = c.send_line("G1 X5 F100").unwrap_err();
    assert_eq!(e, Error::Timeout);
    // the order did reach the machine (that is what a silence is) and the stop reached it too
    assert_eq!(device(&c).position()[0], 5.0);
    assert!(device(&c).is_shutdown());
    assert_eq!(device(&c).emergency_stops, 1);
    assert!(c.halted());
    assert_eq!(c.probes_sent(), 1);
    assert!(c.send_line("G1 X0 F100").is_err());
    c.command(Command::Unlock).unwrap();
    assert!(!c.halted());
}

#[test]
fn a_move_that_the_machine_is_busy_with_is_waited_for_while_the_probes_are_answered_and_then_given_up(
) {
    // the line is lost but the probes are answered: the machine is alive and busy; with the busy limit of this controller the wait ends
    let mut c = KlipperController::new(RpcLink::new(
        KlipperDevice::new(),
        Impair {
            mute_from: Some(2),
            mute_lines: 1,
            ..Impair::clean()
        },
    ));
    c.command(Command::Home(vec![])).unwrap();
    let e = c.send_line("G1 X5 F100").unwrap_err();
    assert_eq!(e, Error::Timeout);
    assert_eq!(c.probes_sent(), klipper::MAX_PROBES);
    assert!(device(&c).is_shutdown());
}

#[test]
fn a_line_that_does_not_move_and_gets_no_answer_is_a_timeout_and_nothing_is_stopped() {
    let mut c = homed(Impair {
        mute_from: Some(1),
        mute_lines: 1,
        ..Impair::clean()
    });
    assert_eq!(c.send_line("M400").unwrap_err(), Error::Timeout);
    assert!(!c.halted() && !device(&c).is_shutdown());
    // the link works again
    c.send_line("G1 X1 F100").unwrap();
    assert_eq!(device(&c).position()[0], 1.0);
}

#[test]
fn an_urgent_stop_from_another_thread_is_the_emergency_stop() {
    let mut c = homed_clean();
    let channel = c.urgent_channel().unwrap();
    channel.send(Urgent::StopJob);
    // the line that was about to go is not sent: the stop went first
    let e = c.send_line("G1 X5 F100").unwrap_err();
    assert_eq!(e.code(), Some(E_SHUTDOWN));
    assert_eq!(device(&c).position()[0], 0.0);
    assert!(device(&c).is_shutdown());
    assert!(c.halted());
    assert_eq!(device(&c).emergency_stops, 1);
}

#[test]
fn klipper_shutting_down_by_itself_is_noticed_from_the_notification_and_from_the_status() {
    let mut c = homed_clean();
    c.transport_mut()
        .push_unsolicited(r#"{"jsonrpc":"2.0","method":"notify_klippy_shutdown","params":[]}"#);
    c.status().unwrap();
    assert!(c.halted());
    let mut c = homed_clean();
    c.transport_mut()
        .device_mut()
        .fail
        .insert(2, "MCU 'mcu' shutdown: Timer too close".to_string());
    let e = c.send_line("G1 X1 F100").unwrap_err();
    assert!(matches!(e, Error::Alarm(_)), "{e:?}");
    assert_eq!(e.code(), Some(E_SHUTDOWN));
    assert!(c.halted());
}

#[test]
fn noise_between_the_answers_is_ignored_with_every_seed() {
    for seed in 1..=8u64 {
        let mut c = controller(Impair {
            seed,
            noise_per_mille: 500,
            ..Impair::clean()
        });
        c.command(Command::Home(vec![])).unwrap();
        for i in 1..=10 {
            c.send_line(&format!("G1 X{} Y{} F600", i * 5, i * 3))
                .unwrap();
            assert!(c.status().is_ok(), "seed {seed}");
        }
        assert_eq!(&c.status().unwrap().machine_mm[..2], &[50.0, 30.0]);
        assert!(c.transport().noise_frames > 0, "seed {seed}: no noise");
    }
}

#[test]
fn a_spoiled_message_never_hangs_the_controller_or_moves_the_machine_out_of_its_travel() {
    for seed in 1..=8u64 {
        let mut c = quick(homed(Impair {
            seed,
            flip_per_mille: 400,
            noise_per_mille: 200,
            ..Impair::clean()
        }));
        for i in 0..20 {
            let r = c.send_line(&format!("G1 X{} Y{} F600", (i * 7) % 250, (i * 3) % 150));
            if r.is_err() && c.halted() {
                c.command(Command::Unlock).ok();
                // after a stop the machine is homed again before it moves
                c.command(Command::Home(vec![])).ok();
            }
            let p = device(&c).position();
            for a in 0..3 {
                assert!(
                    (0.0..=TRAVEL_MM[a]).contains(&p[a]),
                    "seed {seed}: axis {a} at {}",
                    p[a]
                );
            }
        }
        assert!(c.transport().flips > 0, "seed {seed}: no bit was flipped");
    }
}

#[test]
fn the_same_seed_gives_the_same_run() {
    let run = |seed: u64| {
        let mut c = quick(homed(Impair {
            seed,
            flip_per_mille: 300,
            noise_per_mille: 300,
            ..Impair::clean()
        }));
        let mut outcome = Vec::new();
        for i in 0..12 {
            let r = c.send_line(&format!("G1 X{} F600", i * 9));
            outcome.push(format!("{r:?}"));
            if c.halted() {
                c.command(Command::Unlock).ok();
                c.command(Command::Home(vec![])).ok();
            }
        }
        (outcome, c.transport().heard.clone())
    };
    assert_eq!(run(5), run(5));
    assert_ne!(run(5).1, run(6).1);
}

/// A transport that gives a fixed list of texts after each message, to try what the controller does with answers that no device would give.
struct Script(VecDeque<String>);

impl JsonRpcTransport for Script {
    fn send(&mut self, _text: &str) -> io::Result<()> {
        Ok(())
    }
    fn recv(&mut self, _timeout: Duration) -> io::Result<Option<String>> {
        Ok(self.0.pop_front())
    }
}

fn scripted(texts: &[&str]) -> KlipperController<Script> {
    KlipperController::new(Script(texts.iter().map(|t| t.to_string()).collect()))
        .with_busy_limit(Duration::ZERO)
}

#[test]
fn an_error_with_no_number_can_only_be_for_the_message_in_flight() {
    let mut c = scripted(&[
        r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#,
    ]);
    let e = c.send_line("G4 P1").unwrap_err();
    assert_eq!(e.code(), Some(E_MALFORMED));
}

#[test]
fn an_answer_with_neither_result_nor_error_and_a_status_with_no_position_are_refused() {
    let mut c = scripted(&[r#"{"jsonrpc":"2.0","id":1}"#]);
    assert_eq!(c.send_line("G4 P1").unwrap_err().code(), Some(E_MALFORMED));
    let mut c = scripted(&[
        r#"{"jsonrpc":"2.0","result":{"status":{"webhooks":{"state":"ready"}}},"id":1}"#,
    ]);
    assert!(matches!(c.status().unwrap_err(), Error::Link(_)));
}

#[test]
fn a_position_that_is_not_a_number_is_not_believed() {
    let mut c = scripted(&[
        r#"{"jsonrpc":"2.0","result":{"status":{"toolhead":{"position":[1,"x",3,0]}}},"id":1}"#,
    ]);
    assert!(c.status().is_err());
}

#[test]
fn the_unsupported_orders_are_said_not_to_be_supported() {
    let mut c = controller(Impair::clean());
    assert!(matches!(
        c.command(Command::Spindle {
            rpm: 100.0,
            clockwise: true
        }),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        c.command(Command::Coolant(true)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        c.command(Command::Jog {
            axis: 5,
            distance_mm: 1.0,
            feed_mm_min: 1.0
        }),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn pause_and_resume_show_in_the_state() {
    let mut c = homed_clean();
    c.command(Command::Pause).unwrap();
    assert_eq!(c.status().unwrap().state, State::Hold);
    c.command(Command::Resume).unwrap();
    assert_eq!(c.status().unwrap().state, State::Idle);
}

#[test]
fn the_kinds_of_error_are_told_apart_by_their_words() {
    use klipper::classify;
    assert_eq!(
        classify("Unknown command:\"X\""),
        (klipper::E_UNKNOWN, false)
    );
    assert_eq!(
        classify("Must home axis first: 1 2 3 [0]"),
        (E_HOME_FIRST, false)
    );
    assert_eq!(classify("Move out of range: 1 2 3"), (E_RANGE, false));
    assert_eq!(
        classify("Lost communication with MCU 'mcu'"),
        (E_SHUTDOWN, true)
    );
    assert_eq!(
        classify("Heater extruder not heating at expected rate"),
        (klipper::E_THERMAL, true)
    );
    assert_eq!(
        classify("Printer is not ready"),
        (klipper::E_NOT_READY, true)
    );
    assert_eq!(classify("something else"), (klipper::E_OTHER, false));
}
