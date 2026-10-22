//! The state a machine reports while it runs: axes, spindle, machine state and alarms.
//!
//! The names follow what GRBL-family controllers report, because that is what most hobby and
//! shop machines speak; other controllers map onto the same types.

use serde::{Deserialize, Serialize};

/// An axis of the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    /// X.
    X,
    /// Y.
    Y,
    /// Z.
    Z,
    /// A rotary axis around X.
    A,
    /// A rotary axis around Y.
    B,
    /// A rotary axis around Z.
    C,
}

/// The state of one axis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisState {
    /// Which axis.
    pub axis: Axis,
    /// The position in the workpiece coordinates, in millimetres (or degrees for rotary axes).
    pub position: f64,
    /// The position in the machine coordinates.
    pub machine_position: f64,
    /// The speed along the axis.
    pub velocity: f64,
    /// Whether the axis has been homed since power-up.
    pub homed: bool,
    /// Whether its limit switch is active now.
    pub limit_triggered: bool,
}

/// The direction the spindle turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpindleDirection {
    /// Clockwise.
    Clockwise,
    /// Counter-clockwise.
    CounterClockwise,
}

/// The state of the spindle (or the laser).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpindleState {
    /// Whether it is on.
    pub running: bool,
    /// The speed (or the laser power) the controller reports.
    pub rpm: f64,
    /// The direction.
    pub direction: SpindleDirection,
    /// The load in percent, when the driver reports it.
    pub load_percent: Option<f64>,
}

/// What the machine is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MachineState {
    /// Waiting.
    Idle,
    /// Running a program.
    Run,
    /// Paused by a feed hold.
    Hold,
    /// Jogging.
    Jog,
    /// Locked after an alarm.
    Alarm,
    /// The safety door is open.
    Door,
    /// Checking a program without moving.
    Check,
    /// Homing.
    Home,
    /// Asleep.
    Sleep,
}

impl MachineState {
    /// Reads the word a GRBL-family controller puts in its status report (`Idle`, `Run`,
    /// `Hold:0`, `Door:1`...). The part after a colon is ignored.
    pub fn from_report(word: &str) -> Option<Self> {
        match word.split(':').next()?.trim() {
            "Idle" => Some(Self::Idle),
            "Run" => Some(Self::Run),
            "Hold" => Some(Self::Hold),
            "Jog" => Some(Self::Jog),
            "Alarm" => Some(Self::Alarm),
            "Door" => Some(Self::Door),
            "Check" => Some(Self::Check),
            "Home" => Some(Self::Home),
            "Sleep" => Some(Self::Sleep),
            _ => None,
        }
    }
}

/// How serious an alarm is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The machine stopped but nothing was lost.
    Warning,
    /// The position may be lost: home again before cutting.
    Critical,
}

/// An alarm code of a GRBL-family controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlarmCode {
    /// 1: a hard limit switch was hit.
    HardLimit,
    /// 2: a movement would leave the machine's soft limits.
    SoftLimit,
    /// 3: the cycle was aborted while moving.
    AbortCycle,
    /// 4: the probe was not where it should be at the start of the cycle.
    ProbeFailInitial,
    /// 5: the probe never touched anything.
    ProbeFailContact,
    /// 6: homing was reset before it finished.
    HomingFailReset,
    /// 7: the safety door opened while homing.
    HomingFailDoor,
    /// 8: the cycle could not clear the limit switch.
    HomingFailPullOff,
    /// 9: the limit switch was not found.
    HomingFailApproach,
    /// A code this crate does not know.
    Unknown(u16),
}

impl AlarmCode {
    /// The alarm a controller reports with the number `code`.
    pub fn from_code(code: u16) -> Self {
        match code {
            1 => Self::HardLimit,
            2 => Self::SoftLimit,
            3 => Self::AbortCycle,
            4 => Self::ProbeFailInitial,
            5 => Self::ProbeFailContact,
            6 => Self::HomingFailReset,
            7 => Self::HomingFailDoor,
            8 => Self::HomingFailPullOff,
            9 => Self::HomingFailApproach,
            other => Self::Unknown(other),
        }
    }

    /// The number the controller uses.
    pub fn code(self) -> u16 {
        match self {
            Self::HardLimit => 1,
            Self::SoftLimit => 2,
            Self::AbortCycle => 3,
            Self::ProbeFailInitial => 4,
            Self::ProbeFailContact => 5,
            Self::HomingFailReset => 6,
            Self::HomingFailDoor => 7,
            Self::HomingFailPullOff => 8,
            Self::HomingFailApproach => 9,
            Self::Unknown(code) => code,
        }
    }

    /// How serious it is. An unknown code is treated as critical: it is safer to ask for homing.
    pub fn severity(self) -> Severity {
        match self {
            Self::SoftLimit | Self::ProbeFailInitial | Self::ProbeFailContact => Severity::Warning,
            _ => Severity::Critical,
        }
    }

    /// A short English description.
    pub fn description(self) -> &'static str {
        match self {
            Self::HardLimit => "A hard limit switch was hit",
            Self::SoftLimit => "The movement would leave the soft limits",
            Self::AbortCycle => "The cycle was aborted while moving",
            Self::ProbeFailInitial => "The probe was not in its initial state",
            Self::ProbeFailContact => "The probe did not touch anything",
            Self::HomingFailReset => "Homing was reset before it finished",
            Self::HomingFailDoor => "The door opened while homing",
            Self::HomingFailPullOff => "Homing could not clear the limit switch",
            Self::HomingFailApproach => "Homing did not find the limit switch",
            Self::Unknown(_) => "Unknown alarm",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alarm_codes_round_trip_and_unknown_codes_are_kept() {
        for code in 1..=9u16 {
            assert_eq!(AlarmCode::from_code(code).code(), code);
        }
        assert_eq!(AlarmCode::from_code(77), AlarmCode::Unknown(77));
        assert_eq!(AlarmCode::Unknown(77).code(), 77);
    }

    #[test]
    fn an_unknown_alarm_is_critical_and_a_probe_failure_is_not() {
        assert_eq!(AlarmCode::Unknown(40).severity(), Severity::Critical);
        assert_eq!(AlarmCode::ProbeFailContact.severity(), Severity::Warning);
        assert_eq!(AlarmCode::HardLimit.severity(), Severity::Critical);
        assert!(Severity::Warning < Severity::Critical);
    }

    #[test]
    fn machine_states_are_read_from_status_words() {
        assert_eq!(MachineState::from_report("Idle"), Some(MachineState::Idle));
        assert_eq!(
            MachineState::from_report("Hold:0"),
            Some(MachineState::Hold)
        );
        assert_eq!(
            MachineState::from_report("Door:1"),
            Some(MachineState::Door)
        );
        assert_eq!(MachineState::from_report("Banana"), None);
        assert_eq!(MachineState::from_report(""), None);
    }

    #[test]
    fn telemetry_serialises_with_readable_names() {
        let axis = AxisState {
            axis: Axis::X,
            position: 1.5,
            machine_position: -3.0,
            velocity: 0.0,
            homed: true,
            limit_triggered: false,
        };
        let json = serde_json::to_string(&axis).unwrap();
        assert!(json.contains("\"axis\":\"x\""));
        assert_eq!(serde_json::from_str::<AxisState>(&json).unwrap(), axis);
        let alarm = serde_json::to_string(&AlarmCode::HomingFailDoor).unwrap();
        assert_eq!(alarm, "\"homing_fail_door\"");
    }
}
