//! The interface every machine controller of the suite speaks, so that a machine-control program asks the same questions and gives the same
//! orders whatever is behind: a Bungard CCD through its node, a HYDRA-UMC controller, GRBL on a serial port, and the rest of the families
//! of the market (free software and closed controllers alike). A controller is a [`Controller`]; what it can do is its [`Capabilities`];
//! what it is doing is its [`Status`]; what it is told to do is a [`Command`].
//!
//! The families that are known are in [`Family`], with the way each one is reached; the ones that have a controller written here are in
//! [`grbl`]. A controller that has no implementation yet is still listed, so that the program can say that it is not driven yet rather
//! than pretend it is.

pub mod grbl;
pub mod mock;

use std::fmt;

/// The axes a controller can have, in the order of the programs.
pub const AXES: usize = 6;

/// A family of controllers. Each one has its own protocol; some are free software, some are closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// The Bungard CCD machines (the EH3D node, the serial port of the control unit).
    Bungard,
    /// The HYDRA-UMC controllers.
    HydraUmc,
    /// GRBL 1.1 and its relatives.
    Grbl,
    /// grblHAL.
    GrblHal,
    /// FluidNC.
    FluidNc,
    /// Marlin.
    Marlin,
    /// Smoothieware.
    Smoothie,
    /// LinuxCNC.
    LinuxCnc,
    /// Klipper (through Moonraker).
    Klipper,
    /// PlanetCNC (the Mk1 to Mk4 controllers).
    PlanetCnc,
    /// Mach3 and Mach4.
    Mach,
    /// UCCNC and the controllers of CNCdrive.
    Uccnc,
    /// Masso.
    Masso,
    /// The DDCS family of offline controllers.
    Ddcs,
}

/// How a family is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// A serial port (or a USB serial adapter).
    Serial,
    /// A network socket (TCP, WebSocket, an HTTP interface).
    Network,
    /// A USB device with its own protocol.
    Usb,
    /// A library of the maker, loaded by the program.
    Library,
}

impl Family {
    pub const ALL: [Family; 14] = [
        Family::Bungard,
        Family::HydraUmc,
        Family::Grbl,
        Family::GrblHal,
        Family::FluidNc,
        Family::Marlin,
        Family::Smoothie,
        Family::LinuxCnc,
        Family::Klipper,
        Family::PlanetCnc,
        Family::Mach,
        Family::Uccnc,
        Family::Masso,
        Family::Ddcs,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Family::Bungard => "Bungard CCD",
            Family::HydraUmc => "HYDRA-UMC",
            Family::Grbl => "GRBL",
            Family::GrblHal => "grblHAL",
            Family::FluidNc => "FluidNC",
            Family::Marlin => "Marlin",
            Family::Smoothie => "Smoothieware",
            Family::LinuxCnc => "LinuxCNC",
            Family::Klipper => "Klipper",
            Family::PlanetCnc => "PlanetCNC",
            Family::Mach => "Mach3 / Mach4",
            Family::Uccnc => "UCCNC",
            Family::Masso => "Masso",
            Family::Ddcs => "DDCS",
        }
    }

    /// The key of the configuration files (`bungard`, `grbl`...).
    pub fn key(self) -> &'static str {
        match self {
            Family::Bungard => "bungard",
            Family::HydraUmc => "hydra-umc",
            Family::Grbl => "grbl",
            Family::GrblHal => "grblhal",
            Family::FluidNc => "fluidnc",
            Family::Marlin => "marlin",
            Family::Smoothie => "smoothie",
            Family::LinuxCnc => "linuxcnc",
            Family::Klipper => "klipper",
            Family::PlanetCnc => "planetcnc",
            Family::Mach => "mach",
            Family::Uccnc => "uccnc",
            Family::Masso => "masso",
            Family::Ddcs => "ddcs",
        }
    }

    pub fn from_key(key: &str) -> Option<Family> {
        let k = key.trim().to_ascii_lowercase();
        Family::ALL.into_iter().find(|f| f.key() == k)
    }

    pub fn link(self) -> Link {
        match self {
            Family::Bungard => Link::Serial,
            Family::HydraUmc | Family::FluidNc | Family::Klipper | Family::Masso => Link::Network,
            Family::Grbl | Family::GrblHal | Family::Marlin | Family::Smoothie | Family::Ddcs => {
                Link::Serial
            }
            Family::LinuxCnc => Link::Network,
            Family::PlanetCnc => Link::Usb,
            Family::Mach | Family::Uccnc => Link::Library,
        }
    }

    /// Whether the family is free software (its firmware or its program can be read and rebuilt).
    pub fn is_free(self) -> bool {
        matches!(
            self,
            Family::Bungard
                | Family::HydraUmc
                | Family::Grbl
                | Family::GrblHal
                | Family::FluidNc
                | Family::Marlin
                | Family::Smoothie
                | Family::LinuxCnc
                | Family::Klipper
        )
    }

    /// Whether this crate has a controller for it.
    pub fn is_driven(self) -> bool {
        matches!(self, Family::Grbl | Family::GrblHal | Family::FluidNc)
    }
}

/// What a controller can do.
#[derive(Debug, Clone, PartialEq)]
pub struct Capabilities {
    pub axes: usize,
    pub spindle: bool,
    pub spindle_speed: bool,
    pub coolant: bool,
    pub homing: bool,
    pub probe: bool,
    pub jog: bool,
    pub pause: bool,
    pub emergency_stop: bool,
    pub tool_changer: bool,
    /// The controller takes G-code lines; the others take moves.
    pub gcode: bool,
}

/// What the machine is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Run,
    Hold,
    Jog,
    Home,
    Alarm,
    Door,
    Check,
    Sleep,
    /// The controller said something that is not known.
    Unknown,
}

/// The answer to "how are you": the state, where the machine is, how fast it goes and how full its buffer is.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub state: State,
    /// The position of the machine and the position of the work, in mm, for the axes the controller has.
    pub machine_mm: [f64; AXES],
    pub work_mm: [f64; AXES],
    pub feed_mm_min: f64,
    pub spindle_rpm: f64,
    /// How much of the buffer of the controller is in use, in percent (`None` when it does not say).
    pub buffer_percent: Option<u8>,
    /// The inputs that are on, as a text of the pins the controller reports (`XYZPDHRS` for GRBL).
    pub pins: String,
}

impl Default for Status {
    fn default() -> Self {
        Status {
            state: State::Unknown,
            machine_mm: [0.0; AXES],
            work_mm: [0.0; AXES],
            feed_mm_min: 0.0,
            spindle_rpm: 0.0,
            buffer_percent: None,
            pins: String::new(),
        }
    }
}

/// An order to the machine.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Moves an axis by this distance (mm) at this feed (mm/min), until it has gone or until [`Command::JogStop`].
    Jog {
        axis: usize,
        distance_mm: f64,
        feed_mm_min: f64,
    },
    JogStop,
    /// Finds the home of every axis (or of the axes given).
    Home(Vec<usize>),
    /// Holds the job where it is, and goes on.
    Pause,
    Resume,
    /// Stops the job and clears what is queued; the machine stays where it is.
    Stop,
    /// Stops everything at once and puts the controller out of use until it is reset.
    EmergencyStop,
    /// Clears an alarm.
    Unlock,
    /// Makes the work position here zero for these axes.
    ZeroWork(Vec<usize>),
    /// Spindle on at this speed (0 is off).
    Spindle {
        rpm: f64,
        clockwise: bool,
    },
    Coolant(bool),
}

/// What went wrong with a controller.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The link failed or has not been opened.
    Link(String),
    /// The controller did not answer in time.
    Timeout,
    /// The controller refused a line or an order (its own text or number).
    Refused(String),
    /// The controller cannot do this (`what` it was asked).
    Unsupported(&'static str),
    /// The controller is in an alarm and must be unlocked first.
    Alarm(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Link(e) => write!(f, "link: {e}"),
            Error::Timeout => write!(f, "the controller did not answer"),
            Error::Refused(e) => write!(f, "refused: {e}"),
            Error::Unsupported(w) => write!(f, "not supported: {w}"),
            Error::Alarm(e) => write!(f, "alarm: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// A machine controller, whatever family it is.
pub trait Controller {
    fn family(&self) -> Family;
    fn capabilities(&self) -> Capabilities;
    /// Asks the controller how it is (and so tells whether it is there).
    fn status(&mut self) -> Result<Status, Error>;
    /// Gives an order.
    fn command(&mut self, command: Command) -> Result<(), Error>;
    /// Sends a line of G-code and waits until the controller has taken it.
    fn send_line(&mut self, line: &str) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_has_a_name_a_key_and_a_way_to_be_reached() {
        for f in Family::ALL {
            assert!(!f.name().is_empty());
            assert_eq!(Family::from_key(f.key()), Some(f));
        }
        assert_eq!(Family::from_key(" GRBL "), Some(Family::Grbl));
        assert_eq!(Family::from_key("nothing"), None);
        // the keys are all different
        let mut keys: Vec<&str> = Family::ALL.iter().map(|f| f.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), Family::ALL.len());
    }

    #[test]
    fn the_free_and_the_closed_families_are_told_apart_and_only_what_is_written_is_said_to_be_driven(
    ) {
        assert!(Family::Grbl.is_free() && Family::HydraUmc.is_free() && Family::Bungard.is_free());
        assert!(!Family::PlanetCnc.is_free() && !Family::Mach.is_free());
        assert!(
            Family::Grbl.is_driven() && !Family::Mach.is_driven() && !Family::Marlin.is_driven()
        );
        assert_eq!(Family::PlanetCnc.link(), Link::Usb);
    }
}
