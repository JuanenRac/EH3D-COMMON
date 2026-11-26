//! What to start from on a printer: the generic filaments of `eh3d-printmaterials` it can take, and
//! processes computed from the diameter of the nozzle. The ratios of the processes are data
//! (`data/processes.json`), written for this suite and not copied from the profiles of any program.
//! They are a place to start: none has been tried on a machine.

use eh3d_printmaterials::{Database, Filament};
use serde::{Deserialize, Serialize};

use crate::error::{PrnCode, PrnError};
use crate::model::{Enclosure, Printer, SCHEMA_VERSION};
use crate::validate::NOZZLE_MM_RANGE;

/// The manufacturer whose records are the starting points of each family of plastic.
pub const GENERIC_MAKER: &str = "Generic";

/// The embedded text of the processes.
pub const PROCESS_DATA: &str = include_str!("../data/processes.json");

/// How fine or strong a print is meant to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Thick layers, quick.
    Draft,
    /// The middle.
    Standard,
    /// Thin layers.
    Fine,
    /// More walls and infill.
    Strong,
}

impl Level {
    /// Every level.
    pub const ALL: [Level; 4] = [Level::Draft, Level::Standard, Level::Fine, Level::Strong];

    /// The name the level has in the data and on a command line.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Draft => "draft",
            Level::Standard => "standard",
            Level::Fine => "fine",
            Level::Strong => "strong",
        }
    }

    /// The level with this name, in any case.
    pub fn from_name(name: &str) -> Option<Level> {
        Level::ALL
            .into_iter()
            .find(|l| l.as_str().eq_ignore_ascii_case(name.trim()))
    }
}

/// The ratios of one level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelRule {
    /// The level.
    pub level: Level,
    /// The layer height as a part of the nozzle diameter.
    pub layer_ratio: f64,
    /// The height of the first layer as a part of the nozzle diameter.
    pub first_layer_ratio: f64,
    /// The width of a line as a part of the nozzle diameter.
    pub line_width_ratio: f64,
    /// The perimeters.
    pub walls: u32,
    /// The solid layers on top.
    pub top_layers: u32,
    /// The solid layers at the bottom.
    pub bottom_layers: u32,
    /// The infill in percent.
    pub infill_pct: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessFile {
    schema_version: u32,
    levels: Vec<LevelRule>,
}

/// A process for one nozzle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Process {
    /// The level.
    pub level: Level,
    /// The nozzle in mm.
    pub nozzle_mm: f64,
    /// The layer height in mm.
    pub layer_mm: f64,
    /// The height of the first layer in mm.
    pub first_layer_mm: f64,
    /// The width of a line in mm.
    pub line_width_mm: f64,
    /// The perimeters.
    pub walls: u32,
    /// The solid layers on top.
    pub top_layers: u32,
    /// The solid layers at the bottom.
    pub bottom_layers: u32,
    /// The infill in percent.
    pub infill_pct: f64,
}

/// The rules of the processes.
#[derive(Debug, Clone, PartialEq)]
pub struct Processes {
    rules: Vec<LevelRule>,
}

fn rounded(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

impl Processes {
    /// The rules embedded in the crate.
    pub fn embedded() -> Result<Processes, PrnError> {
        Processes::from_text(PROCESS_DATA)
    }

    /// Rules from a text; every level is there once and every ratio is believable.
    pub fn from_text(text: &str) -> Result<Processes, PrnError> {
        let at = "processes.json";
        let file: ProcessFile = serde_json::from_str(text)
            .map_err(|e| PrnError::new(PrnCode::Parse, at, e.to_string()))?;
        if file.schema_version != SCHEMA_VERSION {
            return Err(PrnError::new(PrnCode::SchemaVersion, at, "not version 1"));
        }
        for level in Level::ALL {
            if file.levels.iter().filter(|r| r.level == level).count() != 1 {
                return Err(PrnError::new(
                    PrnCode::Inconsistent,
                    at,
                    format!("the level {} is not there once", level.as_str()),
                ));
            }
        }
        for r in &file.levels {
            let sound = (0.1..=0.75).contains(&r.layer_ratio)
                && (0.1..=0.75).contains(&r.first_layer_ratio)
                && (0.9..=1.5).contains(&r.line_width_ratio)
                && (1..=10).contains(&r.walls)
                && r.top_layers <= 50
                && r.bottom_layers <= 50
                && (0.0..=100.0).contains(&r.infill_pct);
            if !sound {
                return Err(PrnError::new(
                    PrnCode::OutOfRange,
                    at,
                    format!("the level {} has a ratio out of range", r.level.as_str()),
                ));
            }
        }
        Ok(Processes { rules: file.levels })
    }

    /// The process of a level for a nozzle of 0.1 to 2 mm.
    pub fn for_nozzle(&self, level: Level, nozzle_mm: f64) -> Result<Process, PrnError> {
        let bad = |reason: &str| PrnError::new(PrnCode::OutOfRange, level.as_str(), reason);
        if !(NOZZLE_MM_RANGE.0..=NOZZLE_MM_RANGE.1).contains(&nozzle_mm) {
            return Err(bad("the nozzle is not between 0.1 and 2 mm"));
        }
        let r = self
            .rules
            .iter()
            .find(|r| r.level == level)
            .ok_or_else(|| bad("no rule for the level"))?;
        Ok(Process {
            level,
            nozzle_mm,
            layer_mm: rounded(nozzle_mm * r.layer_ratio),
            first_layer_mm: rounded(nozzle_mm * r.first_layer_ratio),
            line_width_mm: rounded(nozzle_mm * r.line_width_ratio),
            walls: r.walls,
            top_layers: r.top_layers,
            bottom_layers: r.bottom_layers,
            infill_pct: r.infill_pct,
        })
    }
}

/// A reason a filament cannot be printed on a printer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Block {
    /// Its lowest nozzle temperature is above what the nozzle reaches.
    NozzleTemperature,
    /// Its lowest bed temperature is above what the bed reaches.
    BedTemperature,
    /// The nozzle is narrower than the filament goes through.
    NozzleTooSmall,
    /// It does not come in the diameter of the printer.
    Diameter,
}

impl Block {
    /// Every reason.
    pub const ALL: [Block; 4] = [
        Block::NozzleTemperature,
        Block::BedTemperature,
        Block::NozzleTooSmall,
        Block::Diameter,
    ];
}

/// Something to know before printing a filament on a printer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Caution {
    /// A closed printer is advised and this one is open.
    Enclosure,
    /// A closed printer is advised and the maker does not say whether this one is.
    EnclosureUnknown,
    /// It wears a brass nozzle.
    HardenedNozzle,
    /// The advised nozzle temperature is above what the nozzle reaches.
    NozzleLimit,
    /// The advised bed temperature is above what the bed reaches.
    BedLimit,
}

impl Caution {
    /// Every caution.
    pub const ALL: [Caution; 5] = [
        Caution::Enclosure,
        Caution::EnclosureUnknown,
        Caution::HardenedNozzle,
        Caution::NozzleLimit,
        Caution::BedLimit,
    ];
}

/// A generic filament seen against a printer.
#[derive(Debug, Clone, PartialEq)]
pub struct StartFilament<'a> {
    /// The filament.
    pub filament: &'a Filament,
    /// Why it cannot be printed; empty when it can.
    pub blocks: Vec<Block>,
    /// What to know.
    pub cautions: Vec<Caution>,
}

impl StartFilament<'_> {
    /// Whether nothing stops it. A caution does not.
    pub fn usable(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Every generic filament of the database seen against a printer with a nozzle, in the order of
/// the database. Nothing is left out: the ones that cannot be printed say why.
pub fn starting_filaments<'a>(
    printer: &Printer,
    database: &'a Database,
    nozzle_mm: f64,
) -> Vec<StartFilament<'a>> {
    database
        .filaments()
        .iter()
        .filter(|f| f.manufacturer == GENERIC_MAKER)
        .map(|f| see(printer, f, nozzle_mm))
        .collect()
}

fn see<'a>(printer: &Printer, f: &'a Filament, nozzle_mm: f64) -> StartFilament<'a> {
    let mut blocks = Vec::new();
    let mut cautions = Vec::new();
    if let Some((lo, _)) = f.nozzle_range() {
        if lo > printer.nozzle_max_c {
            blocks.push(Block::NozzleTemperature);
        }
    }
    if let Some((lo, _)) = f.bed_range() {
        if lo > printer.bed_max_c {
            blocks.push(Block::BedTemperature);
        }
    }
    if f.min_nozzle_mm.is_some_and(|m| nozzle_mm < m - 1e-9) {
        blocks.push(Block::NozzleTooSmall);
    }
    if let Some(d) = printer.filament_diameter_mm {
        if !f.diameters_mm.is_empty() && !f.diameters_mm.iter().any(|x| (x - d).abs() < 1e-9) {
            blocks.push(Block::Diameter);
        }
    }
    if f.enclosure_advised == Some(true) {
        match printer.enclosure {
            Enclosure::Open => cautions.push(Caution::Enclosure),
            Enclosure::Unknown => cautions.push(Caution::EnclosureUnknown),
            Enclosure::Enclosed => {}
        }
    }
    if f.needs_hardened_nozzle() {
        cautions.push(Caution::HardenedNozzle);
    }
    let advised = |r: Option<&eh3d_printmaterials::model::Range>| r.and_then(|r| r.recommended);
    if advised(f.nozzle_c.as_ref()).is_some_and(|t| t > printer.nozzle_max_c) {
        cautions.push(Caution::NozzleLimit);
    }
    if advised(f.bed_c.as_ref()).is_some_and(|t| t > printer.bed_max_c) {
        cautions.push(Caution::BedLimit);
    }
    StartFilament {
        filament: f,
        blocks,
        cautions,
    }
}
