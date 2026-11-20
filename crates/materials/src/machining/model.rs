//! The records of the machining family, as they are written in the JSON files.
//!
//! A record says what a workpiece is, whether a laser may touch it (and why not), and the
//! starting points known for a CNC router and for a laser. Every starting point carries its
//! source and says whether anybody tried it on a machine; the seed of the crate is never
//! verified. A field the schema does not know is refused when a file is read.

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

/// The schema version of the machining files that this crate reads.
pub const SCHEMA_VERSION: u32 = 1;

/// One data file of the machining family.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachiningFile {
    /// The version of the schema the file is written in.
    pub schema_version: u32,
    /// The workpieces.
    pub materials: Vec<Workpiece>,
}

/// The family a workpiece belongs to (the first part of its identifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Solid wood, plywood, MDF, bamboo and the like.
    Wood,
    /// Acrylic, polycarbonate, PVC, ABS and the other plastics.
    Plastic,
    /// Aluminium, brass, copper and other metals.
    Metal,
    /// Paper and cardboard.
    Paper,
    /// Woven and felted cloth.
    Fabric,
    /// Leather and imitation leather.
    Leather,
    /// Rubber and elastomers.
    Rubber,
    /// Foams.
    Foam,
    /// Fibre composites (carbon fibre, glass fibre sheet).
    Composite,
    /// Glass, ceramics and stone, and other things that are not in the other families.
    Mineral,
    /// Board stock: copper clad or bare laminate that is milled or engraved.
    PcbStock,
}

impl Family {
    /// Every family, in the order of the schema.
    pub const ALL: [Family; 11] = [
        Family::Wood,
        Family::Plastic,
        Family::Metal,
        Family::Paper,
        Family::Fabric,
        Family::Leather,
        Family::Rubber,
        Family::Foam,
        Family::Composite,
        Family::Mineral,
        Family::PcbStock,
    ];

    /// The name of the family in the JSON files and in the identifiers.
    pub fn as_str(self) -> &'static str {
        match self {
            Family::Wood => "wood",
            Family::Plastic => "plastic",
            Family::Metal => "metal",
            Family::Paper => "paper",
            Family::Fabric => "fabric",
            Family::Leather => "leather",
            Family::Rubber => "rubber",
            Family::Foam => "foam",
            Family::Composite => "composite",
            Family::Mineral => "mineral",
            Family::PcbStock => "pcb_stock",
        }
    }
}

/// The name of a workpiece in the eight languages of the suite (all eight are required).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Names {
    /// English.
    pub en: String,
    /// Spanish.
    pub es: String,
    /// German.
    pub de: String,
    /// French.
    pub fr: String,
    /// Italian.
    pub it: String,
    /// Japanese.
    pub ja: String,
    /// Chinese (simplified).
    pub zh: String,
    /// Portuguese of Portugal.
    pub pt: String,
}

impl Names {
    /// The name in a language.
    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::En => &self.en,
            Lang::Es => &self.es,
            Lang::De => &self.de,
            Lang::Fr => &self.fr,
            Lang::It => &self.it,
            Lang::Ja => &self.ja,
            Lang::Zh => &self.zh,
            Lang::Pt => &self.pt,
        }
    }
}

/// Why a laser should not touch a material, or what to be careful with. The texts, in the eight
/// languages, are in [`Hazard::text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hazard {
    /// Releases chlorine or hydrochloric acid when burnt (PVC, vinyl, neoprene): it corrodes the
    /// machine and poisons the room.
    Chlorine,
    /// Releases hydrogen cyanide or other toxic gases when burnt (ABS, polyurethane).
    Cyanide,
    /// Releases formaldehyde when burnt (acetal).
    Formaldehyde,
    /// Releases fluorine compounds when burnt (PTFE).
    Fluorine,
    /// Contains chromium salts (chrome tanned leather).
    Chromium,
    /// Fibres and fine dust that are harmful to breathe and, for carbon, conduct electricity.
    FibreDust,
    /// Toxic fumes of a burnt plastic or resin in general.
    ToxicFumes,
    /// Reflects the beam (bare metal): dangerous for the eyes and for the optics.
    Reflective,
    /// Burns easily or keeps burning (some foams, thin plastics, dry fine wood).
    Flammable,
    /// Melts and drips, or chars instead of cutting cleanly.
    MeltsOrChars,
    /// Fine dust when it is cut or milled with a bit: extraction and a mask are needed.
    FineDust,
    /// Does not absorb the beam (glass, clear plastics for a diode laser): nothing happens or the
    /// beam goes through.
    Transparent,
}

impl Hazard {
    /// Every hazard, in the order of the schema.
    pub const ALL: [Hazard; 12] = [
        Hazard::Chlorine,
        Hazard::Cyanide,
        Hazard::Formaldehyde,
        Hazard::Fluorine,
        Hazard::Chromium,
        Hazard::FibreDust,
        Hazard::ToxicFumes,
        Hazard::Reflective,
        Hazard::Flammable,
        Hazard::MeltsOrChars,
        Hazard::FineDust,
        Hazard::Transparent,
    ];
}

/// What a laser may do with a material, and why not.
///
/// `laser_unsafe` means that the material must never be put under a laser (toxic or corrosive
/// fumes): it implies that neither cutting nor engraving is allowed. A material can also refuse
/// the laser without being unsafe (a bare metal reflects the beam, glass does not cut).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Safety {
    /// A CO2 or diode laser may cut it.
    pub laser_cut_allowed: bool,
    /// A CO2 or diode laser may engrave it.
    pub laser_engrave_allowed: bool,
    /// It must never be put under a laser (toxic or corrosive fumes).
    pub laser_unsafe: bool,
    /// What to know: the reason for a refusal, or what to be careful with.
    #[serde(default)]
    pub hazards: Vec<Hazard>,
}

/// What a CNC router does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CncOp {
    /// Cutting the outline through the material.
    Contour,
    /// Taking out an area to a depth.
    Pocket,
    /// A shallow engraving or a V-carving.
    Engrave,
    /// Drilling.
    Drill,
}

impl CncOp {
    /// The name in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            CncOp::Contour => "contour",
            CncOp::Pocket => "pocket",
            CncOp::Engrave => "engrave",
            CncOp::Drill => "drill",
        }
    }
}

/// The kind of cutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// A flat end mill with straight or upcut flutes.
    EndMill,
    /// A compression (up and down cut) end mill for laminated boards.
    Compression,
    /// A ball nose end mill.
    BallNose,
    /// A V-bit.
    VBit,
    /// A twist drill.
    Drill,
}

impl Tool {
    /// The name in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::EndMill => "end_mill",
            Tool::Compression => "compression",
            Tool::BallNose => "ball_nose",
            Tool::VBit => "v_bit",
            Tool::Drill => "drill",
        }
    }
}

/// How sure a starting point is. The seed of the crate never goes above `PublicReference`; a point
/// can only be `verified` when its evidence is a test of the maker or a test on a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    /// Common practice for small machines, written down by the maintainer of the data.
    GeneralPractice,
    /// A public table or calculator, with its licence in `source_info`.
    PublicReference,
    /// A test documented by the maker of the material, the cutter or the machine.
    ManufacturerTest,
    /// Tried on a machine, with the machine named in `machine`.
    TestedOnMachine,
}

impl Evidence {
    /// Whether the evidence is strong enough for `verified = true`.
    pub fn allows_verified(self) -> bool {
        matches!(self, Evidence::ManufacturerTest | Evidence::TestedOnMachine)
    }
}

/// Where a figure comes from, with the date it was taken and the licence it can be used under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInfo {
    /// The name of the table, site or person it comes from.
    pub origin: String,
    /// When it was taken (`YYYY-MM` or `YYYY-MM-DD`).
    pub date: String,
    /// The licence the figure can be used under.
    pub license: String,
    /// The address of the page, when there is one (`https://`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A lowest and a highest value: the range a table gives, around the single starting value.
pub type Span = [f64; 2];

/// The ranges that a table gives for a CNC starting point. The single value of the point is a
/// value inside every range given here (the middle of it in the seed).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CncRanges {
    /// The number of flutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flutes: Option<Span>,
    /// The spindle speed in revolutions per minute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rpm: Option<Span>,
    /// The feed in mm per minute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed_mm_min: Option<Span>,
    /// The plunge feed in mm per minute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plunge_mm_min: Option<Span>,
    /// The depth of a pass in mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_per_pass_mm: Option<Span>,
    /// The step over in percent of the diameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stepover_pct: Option<Span>,
}

impl CncRanges {
    /// Every span given, with the name of its field.
    pub fn spans(&self) -> Vec<(&'static str, Span)> {
        [
            ("flutes", self.flutes),
            ("rpm", self.rpm),
            ("feed_mm_min", self.feed_mm_min),
            ("plunge_mm_min", self.plunge_mm_min),
            ("depth_per_pass_mm", self.depth_per_pass_mm),
            ("stepover_pct", self.stepover_pct),
        ]
        .into_iter()
        .filter_map(|(n, s)| s.map(|s| (n, s)))
        .collect()
    }
}

/// The ranges that a table gives for a laser starting point (the single values of the point are
/// inside them).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaserRanges {
    /// The power setting in percent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_pct: Option<Span>,
    /// The speed in mm per minute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_mm_min: Option<Span>,
    /// The number of passes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passes: Option<Span>,
}

impl LaserRanges {
    /// Every span given, with the name of its field.
    pub fn spans(&self) -> Vec<(&'static str, Span)> {
        [
            ("power_pct", self.power_pct),
            ("speed_mm_min", self.speed_mm_min),
            ("passes", self.passes),
        ]
        .into_iter()
        .filter_map(|(n, s)| s.map(|s| (n, s)))
        .collect()
    }
}

/// The kind of flutes of a cutter, when the table says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CutterStyle {
    /// One flute, for plastics and soft metals.
    SingleFlute,
    /// One flute with a round bottom (O-flute).
    OFlute,
    /// A high helix flute.
    HighHelix,
    /// Flutes that lift the chips up.
    Upcut,
    /// Flutes that push the chips down.
    Downcut,
    /// A cutter that grinds the material away (corn cob, for boards).
    CornCob,
}

/// How the cut is cooled or cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Coolant {
    /// Nothing: dry cutting.
    None,
    /// A jet of air.
    Air,
    /// A lubricant (WD-40, isopropyl alcohol or cutting oil).
    Lubricant,
    /// Flood coolant.
    Flood,
}

/// The unit that a source gave the speed of a laser in (the file always holds mm per minute).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedUnit {
    /// Millimetres per minute.
    MmMin,
    /// Millimetres per second (multiplied by 60 into `speed_mm_min`).
    MmS,
}

impl SpeedUnit {
    /// How many mm per minute one unit of the source is.
    pub fn to_mm_min(self) -> f64 {
        match self {
            SpeedUnit::MmMin => 1.0,
            SpeedUnit::MmS => 60.0,
        }
    }
}

/// How strong the air assist of a laser is, as the sources of hobby lasers write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AirLevel {
    /// No air.
    Off,
    /// A light flow.
    Low,
    /// A medium flow.
    Medium,
    /// A strong flow.
    High,
    /// The most the machine gives.
    Max,
}

impl AirLevel {
    /// Whether any air is used (the value that `air_assist` has to agree with).
    pub fn is_on(self) -> bool {
        self != AirLevel::Off
    }
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// One starting point for a CNC router.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CncPoint {
    /// What the router does.
    pub op: CncOp,
    /// The cutter.
    pub tool: Tool,
    /// The diameter of the cutter in mm.
    pub diameter_mm: f64,
    /// The number of flutes, when it matters.
    pub flutes: Option<u8>,
    /// The spindle speed in revolutions per minute.
    pub rpm: f64,
    /// The feed in the material in mm per minute.
    pub feed_mm_min: f64,
    /// The plunge feed in mm per minute.
    pub plunge_mm_min: Option<f64>,
    /// The depth of each pass in mm.
    pub depth_per_pass_mm: f64,
    /// The step over of a pocket as a percentage of the diameter.
    pub stepover_pct: Option<f64>,
    /// Whether somebody tried the point on a machine. The seed of the crate never says yes.
    pub verified: bool,
    /// Where the point comes from, in a few words.
    pub source: String,
    /// The ranges the table gives around the values above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranges: Option<CncRanges>,
    /// The tip angle of a V-bit in degrees (lowest and highest).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip_angle_deg: Option<Span>,
    /// The kind of flutes, when the table says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutter_style: Option<CutterStyle>,
    /// The coating or the material of the cutter, when the table says so (`diamond or PCD`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coating: Option<String>,
    /// How the cut is cooled or cleared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coolant: Option<Coolant>,
    /// The machine the point was given for (`router, 800 W spindle`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    /// The power of the spindle of that machine in watts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spindle_w: Option<f64>,
    /// How sure the point is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
    /// The source with its date and licence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_info: Option<SourceInfo>,
}

/// The kind of laser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaserKind {
    /// A diode laser (blue, 445 nm and the like).
    Diode,
    /// A CO2 tube laser (10.6 micrometres).
    Co2,
}

impl LaserKind {
    /// The name in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            LaserKind::Diode => "diode",
            LaserKind::Co2 => "co2",
        }
    }
}

/// What the laser does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaserMode {
    /// Cutting through the material.
    Cut,
    /// Engraving or marking the surface.
    Engrave,
}

impl LaserMode {
    /// The name in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            LaserMode::Cut => "cut",
            LaserMode::Engrave => "engrave",
        }
    }
}

/// A kind of laser doing one thing: what `laser_start` is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LaserProcess {
    /// The kind of laser.
    pub kind: LaserKind,
    /// Cutting or engraving.
    pub mode: LaserMode,
}

impl LaserProcess {
    /// A diode laser cutting.
    pub const DIODE_CUT: LaserProcess = LaserProcess {
        kind: LaserKind::Diode,
        mode: LaserMode::Cut,
    };
    /// A diode laser engraving.
    pub const DIODE_ENGRAVE: LaserProcess = LaserProcess {
        kind: LaserKind::Diode,
        mode: LaserMode::Engrave,
    };
    /// A CO2 laser cutting.
    pub const CO2_CUT: LaserProcess = LaserProcess {
        kind: LaserKind::Co2,
        mode: LaserMode::Cut,
    };
    /// A CO2 laser engraving.
    pub const CO2_ENGRAVE: LaserProcess = LaserProcess {
        kind: LaserKind::Co2,
        mode: LaserMode::Engrave,
    };
}

/// One starting point for a laser, for a nominal power of the laser and, when cutting, a thickness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaserPoint {
    /// The kind of laser.
    pub kind: LaserKind,
    /// Cutting or engraving.
    pub mode: LaserMode,
    /// The nominal (optical) power of the laser in watts the point is given for.
    pub power_w: f64,
    /// The thickness in mm (always given for a cut, absent for an engraving).
    pub thickness_mm: Option<f64>,
    /// The power setting in percent of the nominal power.
    pub power_pct: f64,
    /// The speed in mm per minute.
    pub speed_mm_min: f64,
    /// The number of passes.
    pub passes: u32,
    /// Whether air assist is used, when it matters.
    pub air_assist: Option<bool>,
    /// Whether somebody tried the point on a machine. The seed of the crate never says yes.
    pub verified: bool,
    /// Where the point comes from, in a few words.
    pub source: String,
    /// The ranges the table gives around the values above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranges: Option<LaserRanges>,
    /// How strong the air assist is, when the source says so; it agrees with `air_assist`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub air_assist_level: Option<AirLevel>,
    /// The unit the source gave the speed in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_unit: Option<SpeedUnit>,
    /// The speed as the source wrote it, in `source_unit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_speed: Option<f64>,
    /// The machine the point was given for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    /// A second figure from another source for the same case: it is listed next to the main one
    /// and never used for a starting point (`Workpiece::laser_alternatives`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub alternative: bool,
    /// How sure the point is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
    /// The source with its date and licence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_info: Option<SourceInfo>,
}

/// One workpiece: a material that a CNC router or a laser works.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workpiece {
    /// `<family>/<name>` in lower case with hyphens, the name being the English one.
    pub id: String,
    /// The family.
    pub family: Family,
    /// The name in the eight languages.
    pub names: Names,
    /// The density in g/cm3.
    pub density_g_cm3: Option<f64>,
    /// The commercial thicknesses in mm.
    #[serde(default)]
    pub thicknesses_mm: Vec<f64>,
    /// What a laser may do with it.
    pub safety: Safety,
    /// The starting points for a CNC router.
    #[serde(default)]
    pub cnc: Vec<CncPoint>,
    /// The starting points for a laser.
    #[serde(default)]
    pub laser: Vec<LaserPoint>,
    /// The record of the board database (`eh3d-materials` laminates) this material is, when it is
    /// board stock.
    pub pcb_material_id: Option<String>,
    /// A short note in English.
    pub notes: Option<String>,
    /// Where the physical figures (density, thicknesses) come from, in a few words.
    pub source: String,
    /// The source of the physical figures with its date and licence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_info: Option<SourceInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_json_are_the_ones_of_as_str() {
        for f in Family::ALL {
            let json = serde_json::to_string(&f).unwrap();
            assert_eq!(json, format!("\"{}\"", f.as_str()));
            assert_eq!(serde_json::from_str::<Family>(&json).unwrap(), f);
        }
        for (v, s) in [
            (CncOp::Contour, "contour"),
            (CncOp::Pocket, "pocket"),
            (CncOp::Engrave, "engrave"),
            (CncOp::Drill, "drill"),
        ] {
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{s}\""));
            assert_eq!(v.as_str(), s);
        }
        for (v, s) in [
            (Tool::EndMill, "end_mill"),
            (Tool::Compression, "compression"),
            (Tool::BallNose, "ball_nose"),
            (Tool::VBit, "v_bit"),
            (Tool::Drill, "drill"),
        ] {
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{s}\""));
            assert_eq!(v.as_str(), s);
        }
        assert_eq!(LaserKind::Co2.as_str(), "co2");
        assert_eq!(LaserKind::Diode.as_str(), "diode");
        assert_eq!(LaserMode::Cut.as_str(), "cut");
        assert_eq!(LaserMode::Engrave.as_str(), "engrave");
        assert_eq!(serde_json::to_string(&LaserKind::Co2).unwrap(), "\"co2\"");
    }

    #[test]
    fn a_record_reads_with_its_defaults_and_refuses_an_unknown_field() {
        let json = r#"{"id":"wood/x","family":"wood",
            "names":{"en":"X","es":"X","de":"X","fr":"X","it":"X","ja":"X","zh":"X","pt":"X"},
            "safety":{"laser_cut_allowed":true,"laser_engrave_allowed":true,"laser_unsafe":false},
            "source":"s"}"#;
        let w: Workpiece = serde_json::from_str(json).unwrap();
        assert!(w.cnc.is_empty() && w.laser.is_empty() && w.thicknesses_mm.is_empty());
        assert!(w.safety.hazards.is_empty());
        assert_eq!(w.names.get(Lang::Pt), "X");
        let again: Workpiece = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(again, w);
        let bad = json.replace("\"source\":\"s\"", "\"source\":\"s\",\"price\":1");
        assert!(serde_json::from_str::<Workpiece>(&bad).is_err());
    }
}
