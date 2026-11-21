# The machine controllers

`eh3d-controller` is the one place where the suite speaks to a machine controller. A program (the CNC, the laser, the printer and the pick and place programs, the Bungard programs) asks the same questions and gives the same orders through the `Controller` trait whatever is behind it, and it never carries a copy of a controller or of a simulated machine of its own.

Nothing in this crate has run on a real machine. Every controller below was tried against a simulated device; the evidence is `simulator`, and the test on a real board or a real controller is still to be done.

## What is driven

| Family | Controller | Link | `Family::is_driven()` |
|---|---|---|---|
| GRBL, grblHAL, FluidNC | `grbl::GrblController` | lines over a `LineTransport` (the serial port with the feature `serial`, a socket in the program) | yes |
| Marlin 2 | `marlin::MarlinController` | the same `LineTransport` | yes |
| Klipper (through Moonraker) | `klipper::KlipperController` | `JsonRpcTransport`: one JSON-RPC text out, texts in | no, until a transport opens a socket (see [the decision](adr/0001-klipper-transport.md)) |
| The other families of `Family` | none yet | | no |

## What is there to try them with nothing connected

| Module | What it is |
|---|---|
| `sim` | a machine in memory that takes the orders and the basic G-code, with travel limits |
| `grblsim` | a GRBL 1.1 or grblHAL answering the way the firmware does (`GrblDevice`) |
| `marlin` | `MarlinDevice`, a Marlin 2 with its greeting, `M114`, `M211`, line numbers and checksums, and errors in words |
| `klippersim` | `KlipperDevice`, the JSON-RPC of Moonraker answered the way Klipper does, and `RpcLink`, its cable |
| `wire` | `WireLink`: a cable of bytes that cuts the answers in pieces, puts noise between the lines, flips a bit of what the program sends and falls silent for a while, all by a schedule that a fixed seed (`Impair::seed`) makes the same on every computer |
| `faulty` | `FaultyController`: wraps any controller and, by a schedule, makes it silent, busy, refuse a line or lie about the position |
| `mock` | a scripted transport for unit tests |

A cable with `Impair::clean()` is a good cable; a test names the seed and the faults it wants and gets the same run everywhere.

## The numbers of the errors

GRBL gives numbers (`error:20`, `ALARM:3`) and `Error::code()` reads them. Marlin and Klipper say their errors in words, so the controllers of this crate give each kind a number of its own, in a range that cannot be mistaken for another family:

| Range | Family | Kinds |
|---|---|---|
| 1 to 53 | GRBL and grblHAL | the numbers of the firmware |
| 101 to 108 | Marlin | unknown command, line number, checksum, homing, probe, halted, thermal, other |
| 201 to 208 | Klipper | unknown command, home first, out of range, shutdown, thermal, not ready, malformed, other |

`dialtext::explain(family, kind, code, given, lang)` gives the text of any of them in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal) and the text the controller gave when the number is in no table.

## Safe when a controller goes silent

The controllers of this crate stop what moves when the answers stop coming, and they do not send the order again.

* GRBL holds the machine by the kind of line (a homing is reset, a jog cancelled, anything else held).
* Marlin sends the quick stop `M410` after a move that got no answer.
* Klipper has no quick stop that keeps the machine ready: the only order that goes ahead of its queue is `printer.emergency_stop`. A move that gets no answer asks the machine whether it is alive (`printer.info`, answered out of the queue of G-code): while the probes are answered the machine is busy and the wait goes on up to the busy limit; a probe that is not answered is silence, the emergency stop is given, the controller is `halted()` and refuses everything that is not a stop, an unlock or a question until `Command::Unlock` restarts the firmware and Klipper says it is ready again. A hold and a stop of the job asked from another thread through the `UrgentChannel` are that same stop.

A program that wants more (a guard that trips on an implausible position, on two orders that move at once, on a jog that was not renewed) wraps the controller in its own guard; that guard is not in this crate because its settings belong to the program.

## Using a controller

```text
let link = RpcLink::new(KlipperDevice::new(), Impair { seed: 7, noise_per_mille: 300, ..Impair::clean() });
let mut klipper = KlipperController::new(link);
klipper.command(Command::Home(vec![]))?;
klipper.send_line("G1 X20 F600")?;
let status = klipper.status()?;
```

`Controller::max_travel()` reads the longest travel the controller keeps in its own settings (`$130` to `$132` for GRBL, `M211` for Marlin, `position_max` of the steppers for Klipper).

## What a program that had its own copy changes

A program that carried its own Marlin controller, simulated devices, cable or error texts depends on `eh3d-controller` for all of them and deletes its copy: `eh3d_controller::marlin`, `wire`, `grblsim`, `faulty` and `dialtext` have the same names and the same behaviour. What stays in the program is what belongs to the program: its dialect detection, its guard and its settings.
