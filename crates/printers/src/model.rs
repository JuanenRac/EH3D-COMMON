//! The records of the catalog, as they are written in the JSON files.
//!
//! A figure the maker does not publish is absent (or the word `unknown`), never a guess. Units are
//! in the name of the field. A field the schema does not know is refused when a file is read.

use serde::{Deserialize, Serialize};

/// The schema version that this crate reads.
pub const SCHEMA_VERSION: u32 = 1;

/// One data file: the printers of one maker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrinterFile {
    /// The version of the schema the file is written in.
    pub schema_version: u32,
    /// The maker of every printer of the file.
    pub manufacturer: String,
    /// The printers.
    pub printers: Vec<Printer>,
}

/// Whether the build space is closed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enclosure {
    /// Open frame.
    Open,
    /// Closed on every side.
    Enclosed,
    /// The maker does not say.
    #[default]
    Unknown,
}

impl Enclosure {
    /// Every value.
    pub const ALL: [Enclosure; 3] = [Enclosure::Open, Enclosure::Enclosed, Enclosure::Unknown];
}

/// The firmware the maker names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Firmware {
    /// The firmware of Bambu Lab.
    Bambu,
    /// Klipper, or a system built on it.
    Klipper,
    /// Marlin, or a system built on it.
    Marlin,
    /// The maker does not say.
    #[default]
    Unknown,
}

impl Firmware {
    /// Every value.
    pub const ALL: [Firmware; 4] = [
        Firmware::Bambu,
        Firmware::Klipper,
        Firmware::Marlin,
        Firmware::Unknown,
    ];
}

/// The family of the adapter that would talk to the printer: one per protocol, not per make.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Adapter {
    /// The LAN mode of Bambu Lab (MQTT and FTPS).
    BambuLan,
    /// Klipper behind Moonraker.
    KlipperMoonraker,
    /// Marlin over a serial line.
    MarlinSerial,
    /// OctoPrint.
    Octoprint,
    /// PrusaLink.
    Prusalink,
    /// A protocol of the maker alone.
    VendorProtocol,
    /// Not known.
    #[default]
    Unknown,
}

impl Adapter {
    /// Every value.
    pub const ALL: [Adapter; 7] = [
        Adapter::BambuLan,
        Adapter::KlipperMoonraker,
        Adapter::MarlinSerial,
        Adapter::Octoprint,
        Adapter::Prusalink,
        Adapter::VendorProtocol,
        Adapter::Unknown,
    ];

    /// The name the value has in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            Adapter::BambuLan => "bambu_lan",
            Adapter::KlipperMoonraker => "klipper_moonraker",
            Adapter::MarlinSerial => "marlin_serial",
            Adapter::Octoprint => "octoprint",
            Adapter::Prusalink => "prusalink",
            Adapter::VendorProtocol => "vendor_protocol",
            Adapter::Unknown => "unknown",
        }
    }
}

/// Whether the printer can print several colours.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MulticolorStatus {
    /// The system comes with the printer.
    Included,
    /// The system is bought apart.
    Optional,
    /// The maker says there is none.
    None,
    /// The maker does not say.
    #[default]
    Unknown,
}

impl MulticolorStatus {
    /// Every value.
    pub const ALL: [MulticolorStatus; 4] = [
        MulticolorStatus::Included,
        MulticolorStatus::Optional,
        MulticolorStatus::None,
        MulticolorStatus::Unknown,
    ];
}

/// The multi-colour system.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Multicolor {
    /// Whether there is one.
    pub status: MulticolorStatus,
    /// Its name as the maker writes it.
    pub system: Option<String>,
    /// The most colours the maker states.
    pub max_colors: Option<u32>,
}

/// A kind of file the printer takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SendFormat {
    /// Plain G-code.
    #[serde(rename = "gcode")]
    Gcode,
    /// A 3MF package that carries G-code.
    #[serde(rename = "gcode_3mf")]
    Gcode3mf,
    /// Binary G-code.
    #[serde(rename = "bgcode")]
    Bgcode,
}

impl SendFormat {
    /// The ending of the name of such a file.
    pub fn extension(self) -> &'static str {
        match self {
            SendFormat::Gcode => ".gcode",
            SendFormat::Gcode3mf => ".gcode.3mf",
            SendFormat::Bgcode => ".bgcode",
        }
    }
}

/// A public page of the maker and the fields it says.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// An `https` address.
    pub url: String,
    /// The names of the fields of the record that the page says.
    pub covers: Vec<String>,
}

/// A printer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Printer {
    /// The identifier: `<maker>/<model>` in lower case with hyphens.
    pub id: String,
    /// The maker.
    pub manufacturer: String,
    /// The model as the maker writes it.
    pub model: String,
    /// The build volume: x, y and z in mm.
    pub volume_mm: [f64; 3],
    /// The diameter of the filament in mm.
    pub filament_diameter_mm: Option<f64>,
    /// The nozzle that comes fitted, in mm.
    pub nozzle_default_mm: Option<f64>,
    /// The nozzle diameters the maker names, in mm, from small to large.
    #[serde(default)]
    pub nozzles_mm: Vec<f64>,
    /// Whether the build space is closed.
    #[serde(default)]
    pub enclosure: Enclosure,
    /// Whether the chamber is heated on purpose.
    pub heated_chamber: Option<bool>,
    /// The highest temperature of the nozzle in degrees Celsius.
    pub nozzle_max_c: f64,
    /// The highest temperature of the bed in degrees Celsius.
    pub bed_max_c: f64,
    /// The highest temperature of the chamber in degrees Celsius.
    pub chamber_max_c: Option<f64>,
    /// The multi-colour system.
    #[serde(default)]
    pub multicolor: Multicolor,
    /// The firmware.
    #[serde(default)]
    pub firmware: Firmware,
    /// The family of the adapter that would talk to it.
    #[serde(default)]
    pub adapter: Adapter,
    /// Whether the maker publishes how another program talks to it; when `false` the family
    /// comes from the firmware alone and has to be checked on the machine.
    pub adapter_documented: bool,
    /// The files it takes; empty when the maker does not say.
    #[serde(default)]
    pub send_formats: Vec<SendFormat>,
    /// The pages of the maker.
    pub sources: Vec<Source>,
    /// Whether the figures were tried on a machine (never claimed by the data of the suite).
    pub verified: bool,
}

impl Printer {
    /// The identifier with a hyphen in place of the slash, the form the profiles of a slicer use.
    pub fn flat_id(&self) -> String {
        self.id.replace('/', "-")
    }

    /// The maker and the model together.
    pub fn title(&self) -> String {
        format!("{} {}", self.manufacturer, self.model)
    }

    /// Whether a page of the maker covers a field.
    pub fn covers(&self, field: &str) -> bool {
        self.sources
            .iter()
            .any(|s| s.covers.iter().any(|c| c == field))
    }
}
