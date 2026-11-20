//! The records of the database, as they are written in the JSON files.
//!
//! Every field is optional except the identity of the record (identifier, manufacturer, name and
//! kind), its source and the flag `verified`: a value the datasheet does not publish is `null`
//! (or absent), never a guess. Units are in the name of the field. A field the schema does not
//! know is refused when a file is read. Nothing here carries a price: the price is the user's.

use serde::{Deserialize, Serialize};

/// The schema version that this crate reads.
pub const SCHEMA_VERSION: u32 = 1;

/// One data file of filaments: the filaments of one manufacturer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilamentFile {
    /// The version of the schema the file is written in.
    pub schema_version: u32,
    /// The manufacturer of every filament of the file.
    pub manufacturer: String,
    /// The filaments.
    pub filaments: Vec<Filament>,
}

/// One data file of resins: the resins of one manufacturer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResinFile {
    /// The version of the schema the file is written in.
    pub schema_version: u32,
    /// The manufacturer of every resin of the file.
    pub manufacturer: String,
    /// The resins.
    pub resins: Vec<Resin>,
}

/// The polymer of a filament.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    /// Polylactic acid (the variants such as PLA+ go in `grade`).
    Pla,
    /// Glycol modified PET.
    Petg,
    /// Acrylonitrile butadiene styrene.
    Abs,
    /// Acrylonitrile styrene acrylate.
    Asa,
    /// Thermoplastic polyurethane (give the Shore hardness in `shore`).
    Tpu,
    /// Other thermoplastic elastomers.
    Tpe,
    /// Polyamide (nylon; PA6, PA12... go in `grade`).
    Pa,
    /// Polycarbonate.
    Pc,
    /// Polypropylene.
    Pp,
    /// Polyvinyl alcohol (water soluble support).
    Pva,
    /// Butenediol vinyl alcohol (water soluble support).
    Bvoh,
    /// High impact polystyrene (dissolves in limonene).
    Hips,
    /// Polyethylene terephthalate.
    Pet,
    /// Polyphthalamide.
    Ppa,
    /// Polyphenylene sulfide.
    Pps,
    /// Polyether ether ketone.
    Peek,
    /// Polyetherimide.
    Pei,
    /// Anything else.
    Other,
}

impl Material {
    /// Every material, in the order of the schema.
    pub const ALL: [Material; 18] = [
        Material::Pla,
        Material::Petg,
        Material::Abs,
        Material::Asa,
        Material::Tpu,
        Material::Tpe,
        Material::Pa,
        Material::Pc,
        Material::Pp,
        Material::Pva,
        Material::Bvoh,
        Material::Hips,
        Material::Pet,
        Material::Ppa,
        Material::Pps,
        Material::Peek,
        Material::Pei,
        Material::Other,
    ];

    /// The name the material has in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            Material::Pla => "pla",
            Material::Petg => "petg",
            Material::Abs => "abs",
            Material::Asa => "asa",
            Material::Tpu => "tpu",
            Material::Tpe => "tpe",
            Material::Pa => "pa",
            Material::Pc => "pc",
            Material::Pp => "pp",
            Material::Pva => "pva",
            Material::Bvoh => "bvoh",
            Material::Hips => "hips",
            Material::Pet => "pet",
            Material::Ppa => "ppa",
            Material::Pps => "pps",
            Material::Peek => "peek",
            Material::Pei => "pei",
            Material::Other => "other",
        }
    }

    /// The material that has this name (upper and lower case are the same), if any.
    pub fn from_name(name: &str) -> Option<Material> {
        let want = name.trim().to_ascii_lowercase();
        Material::ALL.into_iter().find(|m| m.as_str() == want)
    }
}

/// What a filament is filled with (the filled ones wear a brass nozzle, see
/// [`Filament::needs_hardened_nozzle`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fill {
    /// Chopped carbon fibre.
    CarbonFiber,
    /// Chopped glass fibre.
    GlassFiber,
    /// Wood flour.
    Wood,
    /// Metal powder.
    Metal,
    /// Glitter, sparkle or mica.
    Glitter,
    /// Phosphorescent pigment (glow in the dark).
    Glow,
    /// Stone, marble or ceramic powder.
    Mineral,
    /// Any other filler.
    Other,
}

impl Fill {
    /// Whether the filler normally wears a brass nozzle.
    pub fn abrasive(self) -> bool {
        matches!(
            self,
            Fill::CarbonFiber | Fill::GlassFiber | Fill::Metal | Fill::Glow | Fill::Mineral
        )
    }
}

/// The scale of a Shore hardness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShoreScale {
    /// Shore A (soft materials).
    A,
    /// Shore D (hard materials).
    D,
}

/// A Shore hardness.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hardness {
    /// The value (1 to 100).
    pub value: f64,
    /// The scale.
    pub scale: ShoreScale,
}

/// A range with the value to start from. Every part is optional; when they are there,
/// `min <= recommended <= max`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Range {
    /// The lowest value.
    pub min: Option<f64>,
    /// The highest value.
    pub max: Option<f64>,
    /// The value to start from.
    pub recommended: Option<f64>,
}

/// A colour of a filament (the same filament comes in many).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorVariant {
    /// The name of the colour as the maker writes it.
    pub name: String,
    /// The colour as `#RRGGBB`, when the maker gives it.
    pub hex: Option<String>,
    /// The transmission distance of the colour in mm (the figure that colour-layering tools use), when somebody measured it.
    pub td: Option<f64>,
}

/// From which layer a fan speed holds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanSpan {
    /// The first layer (1 is the first one printed) of the span; it goes on until the next span.
    pub from_layer: u32,
    /// The speed of the part fan in percent.
    pub pct: f64,
}

/// How the extruder moves the filament.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drive {
    /// The motor sits on the hot end.
    Direct,
    /// The motor is away and a tube leads to the hot end.
    Bowden,
}

/// A retraction to start from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retraction {
    /// The distance in mm.
    pub distance_mm: Option<f64>,
    /// The speed in mm/s.
    pub speed_mm_s: Option<f64>,
    /// The kind of extruder the figures are for.
    pub drive: Option<Drive>,
}

/// How to dry a filament.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drying {
    /// The temperature in degrees Celsius.
    pub temp_c: f64,
    /// The time in hours.
    pub hours: f64,
}

/// How much a filament takes water from the air.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Moisture {
    /// It hardly does (PLA, PETG in most cases).
    Low,
    /// It does in some days.
    Medium,
    /// It does in hours (PA, PVA, BVOH, TPU).
    High,
}

/// A spool the filament is sold on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spool {
    /// The weight of the filament in grams.
    pub weight_g: f64,
    /// The outer diameter of the spool in mm.
    pub diameter_mm: Option<f64>,
    /// The width of the spool in mm.
    pub width_mm: Option<f64>,
    /// The diameter of the hole of the hub in mm.
    pub hub_mm: Option<f64>,
}

/// What the filament is known to work with.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    /// The identifiers of the printer profiles (the ones of the slicer of the suite) that the maker names.
    #[serde(default)]
    pub printers: Vec<String>,
    /// The nozzle diameters in mm that the maker names.
    #[serde(default)]
    pub nozzles_mm: Vec<f64>,
    /// A word on it.
    pub notes: Option<String>,
}

/// A filament.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filament {
    /// The identifier: `<manufacturer>/<name>` in lower case with hyphens.
    pub id: String,
    /// The manufacturer.
    pub manufacturer: String,
    /// The line and name of the filament as the maker writes them.
    pub name: String,
    /// The polymer.
    pub material: Material,
    /// The variant of the polymer (`PLA+`, `PA12`, `silk`...), as the maker writes it.
    pub grade: Option<String>,
    /// What it is filled with.
    #[serde(default)]
    pub fill: Vec<Fill>,
    /// The hardness, for the soft ones.
    pub shore: Option<Hardness>,
    /// The diameters it comes in, in mm (1.75 and 2.85 are the usual ones).
    #[serde(default)]
    pub diameters_mm: Vec<f64>,
    /// The tolerance of the diameter in mm.
    pub diameter_tolerance_mm: Option<f64>,
    /// The density in g/cm3.
    pub density_g_cm3: Option<f64>,
    /// The colours.
    #[serde(default)]
    pub colors: Vec<ColorVariant>,
    /// The temperature of the nozzle in degrees Celsius.
    pub nozzle_c: Option<Range>,
    /// The temperature of the bed in degrees Celsius.
    pub bed_c: Option<Range>,
    /// The temperature advised for the chamber in degrees Celsius.
    pub chamber_c: Option<f64>,
    /// The highest speed of the print in mm/s.
    pub max_speed_mm_s: Option<f64>,
    /// The highest volumetric flow in mm3/s.
    pub max_flow_mm3_s: Option<f64>,
    /// The part fan in percent.
    pub fan_pct: Option<Range>,
    /// The part fan by stretch of the print.
    #[serde(default)]
    pub fan_spans: Vec<FanSpan>,
    /// The retraction to start from.
    pub retraction: Option<Retraction>,
    /// How to dry it.
    pub drying: Option<Drying>,
    /// How much it takes water.
    pub moisture: Option<Moisture>,
    /// Whether it wears a brass nozzle; when absent the filler tells (see
    /// [`Filament::needs_hardened_nozzle`]).
    pub abrasive: Option<bool>,
    /// The smallest nozzle in mm that it goes through without trouble.
    pub min_nozzle_mm: Option<f64>,
    /// Whether it prints better in a closed, warm space.
    pub enclosure_advised: Option<bool>,
    /// Whether it goes best through a direct drive extruder.
    pub direct_drive_advised: Option<bool>,
    /// The spools.
    #[serde(default)]
    pub spools: Vec<Spool>,
    /// The glass transition in degrees Celsius.
    pub tg_c: Option<f64>,
    /// The heat deflection temperature in degrees Celsius.
    pub hdt_c: Option<f64>,
    /// The tensile strength in MPa.
    pub tensile_mpa: Option<f64>,
    /// The tensile (or flexural) modulus in MPa.
    pub modulus_mpa: Option<f64>,
    /// The transmission distance in mm of the filament in general (a colour may give its own).
    pub td: Option<f64>,
    /// What it is known to work with.
    pub compatibility: Option<Compatibility>,
    /// The pages of the maker (data sheet, safety sheet): `https` addresses.
    #[serde(default)]
    pub links: Vec<String>,
    /// A word on it.
    pub notes: Option<String>,
    /// Where the record comes from: `maker-datasheet` or the name of a database that allows reuse.
    pub source: String,
    /// The place in the source (a file name or a path of it), or the name of the data sheet.
    pub source_ref: Option<String>,
    /// Whether the values were tried on a machine (never claimed by the data of the suite).
    pub verified: bool,
}

/// What a resin is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResinKind {
    /// General purpose.
    Standard,
    /// Hard and stiff.
    Hard,
    /// Flexible or elastic.
    Flexible,
    /// Tough, behaves like ABS.
    AbsLike,
    /// Castable (burns out clean for jewellery).
    Castable,
    /// Dental or medical, with the approvals the maker states.
    Dental,
    /// Washes in water.
    WaterWashable,
    /// Keeps its shape at high temperature.
    HighTemperature,
    /// Filled with ceramic.
    Ceramic,
    /// Transparent.
    Clear,
    /// Made for fine detail (miniatures).
    HighDetail,
    /// Anything else.
    Other,
}

impl ResinKind {
    /// Every kind, in the order of the schema.
    pub const ALL: [ResinKind; 12] = [
        ResinKind::Standard,
        ResinKind::Hard,
        ResinKind::Flexible,
        ResinKind::AbsLike,
        ResinKind::Castable,
        ResinKind::Dental,
        ResinKind::WaterWashable,
        ResinKind::HighTemperature,
        ResinKind::Ceramic,
        ResinKind::Clear,
        ResinKind::HighDetail,
        ResinKind::Other,
    ];

    /// The name the kind has in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            ResinKind::Standard => "standard",
            ResinKind::Hard => "hard",
            ResinKind::Flexible => "flexible",
            ResinKind::AbsLike => "abs_like",
            ResinKind::Castable => "castable",
            ResinKind::Dental => "dental",
            ResinKind::WaterWashable => "water_washable",
            ResinKind::HighTemperature => "high_temperature",
            ResinKind::Ceramic => "ceramic",
            ResinKind::Clear => "clear",
            ResinKind::HighDetail => "high_detail",
            ResinKind::Other => "other",
        }
    }
}

/// What the part is washed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Solvent {
    /// Isopropyl alcohol.
    Ipa,
    /// Ethanol or any alcohol the maker does not name.
    Alcohol,
    /// Water.
    Water,
    /// Something else, named in the notes.
    Other,
}

/// How the part is washed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wash {
    /// The solvent.
    pub solvent: Solvent,
    /// The time in minutes.
    pub minutes: Option<f64>,
}

/// How the part is cured after the wash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostCure {
    /// The time in minutes.
    pub minutes: Option<f64>,
    /// The temperature in degrees Celsius.
    pub temp_c: Option<f64>,
    /// The equipment (UV chamber, lamp, oven).
    pub device: Option<String>,
}

/// The settings of one resin on one printer at one layer height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposurePoint {
    /// The identifier of the printer: the one of the printer profiles of the slicer of the suite
    /// when it is there, `generic-msla` for settings that fit a monochrome screen printer in general.
    pub printer: String,
    /// The layer height in mm.
    pub layer_mm: f64,
    /// The exposure of a normal layer in seconds.
    pub normal_s: f64,
    /// The lowest and highest normal exposure that worked, in seconds.
    pub normal_s_min: Option<f64>,
    /// See `normal_s_min`.
    pub normal_s_max: Option<f64>,
    /// The exposure of the base layers in seconds.
    pub base_s: Option<f64>,
    /// The lowest base exposure in seconds.
    pub base_s_min: Option<f64>,
    /// The highest base exposure in seconds.
    pub base_s_max: Option<f64>,
    /// The number of base layers.
    pub base_layers: Option<u32>,
    /// The speed of the lift in mm/min.
    pub lift_speed_mm_min: Option<f64>,
    /// The speed of the retraction in mm/min.
    pub retract_speed_mm_min: Option<f64>,
    /// The distance of the lift in mm.
    pub lift_mm: Option<f64>,
    /// The distance of the second stretch of the lift in mm, for printers that lift in two.
    pub lift2_mm: Option<f64>,
    /// The wait before the exposure in seconds (after the retraction).
    pub wait_before_s: Option<f64>,
    /// The wait after the exposure in seconds (before the lift).
    pub wait_after_s: Option<f64>,
    /// Whether the settings were tried on this printer (never claimed by the data of the suite).
    pub verified: bool,
}

/// A photopolymer resin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resin {
    /// The identifier: `<manufacturer>/<name>` in lower case with hyphens.
    pub id: String,
    /// The manufacturer.
    pub manufacturer: String,
    /// The name as the maker writes it.
    pub name: String,
    /// The kind.
    pub kind: ResinKind,
    /// The wavelength it cures at, in nm (385 and 405 are the usual ones).
    pub wavelength_nm: Option<f64>,
    /// The viscosity in mPa s.
    pub viscosity_mpa_s: Option<f64>,
    /// The density in g/cm3 (liquid).
    pub density_g_cm3: Option<f64>,
    /// The hardness Shore D after curing.
    pub shore_d: Option<f64>,
    /// The hardness Shore A after curing, for the flexible ones.
    pub shore_a: Option<f64>,
    /// The glass transition in degrees Celsius.
    pub tg_c: Option<f64>,
    /// The heat deflection temperature in degrees Celsius.
    pub hdt_c: Option<f64>,
    /// The shrinkage in percent.
    pub shrinkage_pct: Option<f64>,
    /// The tensile strength in MPa.
    pub tensile_mpa: Option<f64>,
    /// The flexural strength in MPa.
    pub flexural_mpa: Option<f64>,
    /// The impact strength in kJ/m2.
    pub impact_kj_m2: Option<f64>,
    /// The elongation at break in percent.
    pub elongation_pct: Option<f64>,
    /// The water absorption in percent.
    pub water_absorption_pct: Option<f64>,
    /// The colour(s) as the maker writes them.
    #[serde(default)]
    pub colors: Vec<ColorVariant>,
    /// Whether the cured part is transparent or translucent.
    pub translucent: Option<bool>,
    /// The file name of the safety data sheet (SDS).
    pub sds: Option<String>,
    /// The post-cure.
    pub post_cure: Option<PostCure>,
    /// The wash.
    pub wash: Option<Wash>,
    /// The layer heights in mm that the maker recommends.
    pub layer_mm: Option<Range>,
    /// The settings by printer and layer height.
    #[serde(default)]
    pub exposures: Vec<ExposurePoint>,
    /// The pages of the maker (data sheet, safety sheet): `https` addresses.
    #[serde(default)]
    pub links: Vec<String>,
    /// A word on it.
    pub notes: Option<String>,
    /// Where the record comes from: `maker-datasheet` or `generic-profile`.
    pub source: String,
    /// The place in the source (a file name), or the name of the data sheet.
    pub source_ref: Option<String>,
    /// Whether the values were tried on a machine (never claimed by the data of the suite).
    pub verified: bool,
}
