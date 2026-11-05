//! The records of the database, as they are written in the JSON files.
//!
//! Every field is optional except the identity of the material: a value the datasheet does not
//! publish is `null` (or absent), never a guess. Units are in the name of the field and
//! frequencies are in GHz. A field the schema does not know is refused when a file is read.

use serde::{Deserialize, Serialize};

/// The schema version that this crate reads.
pub const SCHEMA_VERSION: u32 = 1;

/// One data file: the materials of one manufacturer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialFile {
    /// The version of the schema the file is written in.
    pub schema_version: u32,
    /// The manufacturer of every material of the file.
    pub manufacturer: String,
    /// The materials.
    pub materials: Vec<Material>,
}

/// What a material is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A copper clad or unclad rigid laminate.
    Laminate,
    /// A prepreg (bonding sheet of glass and partly cured resin).
    Prepreg,
    /// An unreinforced bonding film.
    BondingFilm,
    /// A flexible laminate (polyimide and the like).
    FlexLaminate,
    /// A coverlay of a flex circuit.
    Coverlay,
    /// A metal core laminate (aluminium, copper).
    MetalCore,
    /// A PTFE laminate.
    Ptfe,
    /// A ceramic filled laminate (hydrocarbon or PTFE).
    CeramicFilled,
    /// A copper foil.
    CopperFoil,
    /// Anything else.
    Other,
}

impl Kind {
    /// Every kind, in the order of the schema.
    pub const ALL: [Kind; 10] = [
        Kind::Laminate,
        Kind::Prepreg,
        Kind::BondingFilm,
        Kind::FlexLaminate,
        Kind::Coverlay,
        Kind::MetalCore,
        Kind::Ptfe,
        Kind::CeramicFilled,
        Kind::CopperFoil,
        Kind::Other,
    ];

    /// The name the kind has in the JSON files.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Laminate => "laminate",
            Kind::Prepreg => "prepreg",
            Kind::BondingFilm => "bonding_film",
            Kind::FlexLaminate => "flex_laminate",
            Kind::Coverlay => "coverlay",
            Kind::MetalCore => "metal_core",
            Kind::Ptfe => "ptfe",
            Kind::CeramicFilled => "ceramic_filled",
            Kind::CopperFoil => "copper_foil",
            Kind::Other => "other",
        }
    }
}

/// A Dk or a Df at one frequency (the same record serves both).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DkPoint {
    /// The frequency in GHz, when the sheet states it.
    pub freq_ghz: Option<f64>,
    /// The value as published.
    pub value: f64,
    /// The test method (IPC-650 2.5.5.x, SPDR, stripline...).
    pub method: Option<String>,
    /// The resin content the value is given for, when the sheet gives it.
    pub resin_content_pct: Option<f64>,
    /// The tolerance of the value, when the sheet gives it.
    pub tolerance: Option<f64>,
}

/// A value that a sheet may give per direction (lengthwise and crosswise) or without one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Directional {
    /// Lengthwise (warp, machine direction).
    pub lengthwise: Option<f64>,
    /// Crosswise (fill, transverse direction).
    pub crosswise: Option<f64>,
    /// The value when the sheet does not say the direction.
    pub value: Option<f64>,
}

/// A range of thicknesses when the sheet gives a range instead of a list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThicknessRange {
    /// The smallest nominal thickness in mm.
    pub min_mm: Option<f64>,
    /// The largest nominal thickness in mm.
    pub max_mm: Option<f64>,
    /// How the steps between them are given.
    pub steps_note: Option<String>,
}

/// A copper foil the material comes with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Copper {
    /// The weight in oz/ft2.
    pub weight_oz: Option<f64>,
    /// The thickness in micrometres.
    pub thickness_um: Option<f64>,
    /// The kind of foil (ED, RTF, VLP, HVLP, RA).
    pub foil_type: Option<String>,
}

/// A panel size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelSize {
    /// The width in mm.
    pub width_mm: f64,
    /// The height in mm.
    pub height_mm: f64,
    /// The size as the sheet writes it (for example in inches).
    pub label: Option<String>,
}

/// One glass style of a prepreg.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepregStyle {
    /// The glass style (1080, 2116, 7628...).
    pub style: Option<String>,
    /// The resin content in percent.
    pub resin_content_pct: Option<f64>,
    /// The thickness after pressing in micrometres.
    pub pressed_thickness_um: Option<f64>,
    /// The resin flow in percent.
    pub flow_pct: Option<f64>,
    /// The gel time in seconds.
    pub gel_time_s: Option<f64>,
    /// Dk of this construction, when the sheet gives one per style.
    pub dk: Option<f64>,
    /// Df of this construction, when the sheet gives one per style.
    pub df: Option<f64>,
    /// The frequency in GHz of `dk` and `df`.
    pub test_freq_ghz: Option<f64>,
}

/// The lamination cycle the process sheet recommends.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lamination {
    /// The highest temperature in degrees Celsius.
    pub max_temp_c: Option<f64>,
    /// The pressure in kPa.
    pub pressure_kpa: Option<f64>,
    /// The time at temperature in minutes.
    pub time_at_temp_min: Option<f64>,
    /// The heating rate in degrees Celsius per minute.
    pub heat_rate_c_min: Option<f64>,
    /// Whatever else the sheet says about the cycle.
    pub notes: Option<String>,
}

/// The drilling advice of the process sheet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drilling {
    /// The surface speed in m/min.
    pub speed_surface_m_min: Option<f64>,
    /// The chip load in micrometres per revolution.
    pub chip_load_um: Option<f64>,
    /// The advice on the retract.
    pub retract_notes: Option<String>,
}

/// The process advice of the sheet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Processing {
    /// The lamination cycle.
    pub lamination: Option<Lamination>,
    /// The drilling advice.
    pub drilling: Option<Drilling>,
    /// The advice on desmear.
    pub desmear_note: Option<String>,
    /// The advice on baking.
    pub bake_note: Option<String>,
}

/// What the safety data sheet says.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Safety {
    /// The hazard statements (H phrases) of the sheet.
    #[serde(default)]
    pub hazard_statements: Vec<String>,
    /// What the sheet says about flammability.
    pub flammability_note: Option<String>,
    /// The revision of the sheet.
    pub msds_revision: Option<String>,
}

/// Where a value comes from: the name of a PDF and the place in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The file name of the PDF, with no path.
    pub pdf: String,
    /// The pages or the table.
    pub pages: Option<String>,
    /// The revision the sheet states.
    pub revision: Option<String>,
}

/// One material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    /// `<manufacturer>/<name>` in lower case with hyphens.
    pub id: String,
    /// The manufacturer.
    pub manufacturer: String,
    /// The product family.
    pub family: Option<String>,
    /// The name of the material.
    pub name: String,
    /// What it is.
    pub kind: Kind,

    /// The resin system.
    pub resin: Option<String>,
    /// The reinforcement (for example `E-glass woven`).
    pub reinforcement: Option<String>,
    /// The filler.
    pub filler: Option<String>,
    /// Halogen free.
    pub halogen_free: Option<bool>,
    /// Compatible with lead free assembly.
    pub lead_free_compatible: Option<bool>,
    /// The UL 94 class (`V-0`, `HB`...).
    pub ul94: Option<String>,
    /// The IPC-4101 slash sheet (`IPC-4101/126`).
    pub ipc_slash_sheet: Option<String>,
    /// The IEC 249 class.
    pub iec249: Option<String>,

    /// The glass transition by DSC in degrees Celsius.
    pub tg_dsc_c: Option<f64>,
    /// The glass transition by DMA in degrees Celsius.
    pub tg_dma_c: Option<f64>,
    /// The glass transition by TMA in degrees Celsius.
    pub tg_tma_c: Option<f64>,
    /// The decomposition temperature (5 % loss) in degrees Celsius.
    pub td_c: Option<f64>,
    /// The time to delamination at 260 C in minutes.
    pub t260_min: Option<f64>,
    /// The time to delamination at 288 C in minutes.
    pub t288_min: Option<f64>,
    /// The time to delamination at 300 C in minutes.
    pub t300_min: Option<f64>,

    /// The expansion along x in ppm per degree.
    pub cte_x_ppm: Option<f64>,
    /// The expansion along y in ppm per degree.
    pub cte_y_ppm: Option<f64>,
    /// The expansion along z below Tg in ppm per degree.
    pub cte_z_below_tg_ppm: Option<f64>,
    /// The expansion along z above Tg in ppm per degree.
    pub cte_z_above_tg_ppm: Option<f64>,
    /// The expansion along z between 50 and 260 C in percent.
    pub cte_z_50_260_pct: Option<f64>,

    /// The dielectric constant at the frequencies the sheet gives.
    #[serde(default)]
    pub dk: Vec<DkPoint>,
    /// The loss tangent at the frequencies the sheet gives.
    #[serde(default)]
    pub df: Vec<DkPoint>,
    /// The thermal coefficient of Dk in ppm per degree.
    pub tcdk_ppm_per_c: Option<f64>,

    /// The thermal conductivity in W/(m K).
    pub thermal_conductivity_w_mk: Option<f64>,
    /// The specific heat in J/(g K).
    pub specific_heat_j_gk: Option<f64>,
    /// The density in g/cm3.
    pub density_g_cm3: Option<f64>,

    /// The peel strength in N/mm.
    pub peel_strength_n_mm: Option<f64>,
    /// The condition the peel strength is given for.
    pub peel_condition: Option<String>,
    /// The water absorption in percent.
    pub water_absorption_pct: Option<f64>,
    /// What the sheet says about moisture when it gives no number.
    pub moisture_absorption_note: Option<String>,

    /// The volume resistivity in ohm cm.
    pub volume_resistivity_ohm_cm: Option<f64>,
    /// The surface resistivity in ohm.
    pub surface_resistivity_ohm: Option<f64>,
    /// The dielectric strength in kV/mm.
    pub dielectric_strength_kv_mm: Option<f64>,
    /// The arc resistance in seconds.
    pub arc_resistance_s: Option<f64>,
    /// The comparative tracking index in volts.
    pub cti_v: Option<f64>,
    /// The flexural strength in MPa.
    pub flexural_strength_mpa: Option<Directional>,
    /// The tensile strength in MPa.
    pub tensile_strength_mpa: Option<Directional>,
    /// Young's modulus in GPa.
    pub youngs_modulus_gpa: Option<Directional>,

    /// The nominal thicknesses of the laminate in mm.
    #[serde(default)]
    pub thicknesses_mm: Vec<f64>,
    /// The range of thicknesses, when the sheet gives a range.
    pub thickness_range: Option<ThicknessRange>,
    /// The copper foils.
    #[serde(default)]
    pub copper: Vec<Copper>,
    /// The panel sizes.
    #[serde(default)]
    pub panel_sizes: Vec<PanelSize>,
    /// The glass styles (prepregs and bonding films).
    #[serde(default)]
    pub prepreg: Vec<PrepregStyle>,
    /// The process advice.
    pub processing: Option<Processing>,
    /// The safety data.
    pub safety: Option<Safety>,

    /// A short note.
    pub notes: Option<String>,
    /// The material that replaces this one, when the sheet says so.
    pub superseded_by: Option<String>,
    /// The sheet the values come from.
    pub source: Option<Source>,
    /// The sheets the values come from, when there are several.
    #[serde(default)]
    pub sources: Vec<Source>,
}

impl Material {
    /// A material with only its identity filled in; the rest is unknown.
    pub fn new(id: &str, manufacturer: &str, name: &str, kind: Kind, pdf: &str) -> Material {
        Material {
            id: id.to_owned(),
            manufacturer: manufacturer.to_owned(),
            family: None,
            name: name.to_owned(),
            kind,
            resin: None,
            reinforcement: None,
            filler: None,
            halogen_free: None,
            lead_free_compatible: None,
            ul94: None,
            ipc_slash_sheet: None,
            iec249: None,
            tg_dsc_c: None,
            tg_dma_c: None,
            tg_tma_c: None,
            td_c: None,
            t260_min: None,
            t288_min: None,
            t300_min: None,
            cte_x_ppm: None,
            cte_y_ppm: None,
            cte_z_below_tg_ppm: None,
            cte_z_above_tg_ppm: None,
            cte_z_50_260_pct: None,
            dk: Vec::new(),
            df: Vec::new(),
            tcdk_ppm_per_c: None,
            thermal_conductivity_w_mk: None,
            specific_heat_j_gk: None,
            density_g_cm3: None,
            peel_strength_n_mm: None,
            peel_condition: None,
            water_absorption_pct: None,
            moisture_absorption_note: None,
            volume_resistivity_ohm_cm: None,
            surface_resistivity_ohm: None,
            dielectric_strength_kv_mm: None,
            arc_resistance_s: None,
            cti_v: None,
            flexural_strength_mpa: None,
            tensile_strength_mpa: None,
            youngs_modulus_gpa: None,
            thicknesses_mm: Vec::new(),
            thickness_range: None,
            copper: Vec::new(),
            panel_sizes: Vec::new(),
            prepreg: Vec::new(),
            processing: None,
            safety: None,
            notes: None,
            superseded_by: None,
            source: Some(Source {
                pdf: pdf.to_owned(),
                pages: None,
                revision: None,
            }),
            sources: Vec::new(),
        }
    }

    /// Every sheet the material cites: `source` first, then `sources`.
    pub fn all_sources(&self) -> impl Iterator<Item = &Source> {
        self.source.iter().chain(self.sources.iter())
    }

    /// The glass transition to compare materials by: DSC if published, else DMA, else TMA.
    /// The methods give different numbers; this is a way to order materials, not a measurement.
    pub fn tg_c(&self) -> Option<f64> {
        self.tg_dsc_c.or(self.tg_dma_c).or(self.tg_tma_c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind_names_of_the_json_are_the_ones_of_as_str() {
        for kind in Kind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
            assert_eq!(serde_json::from_str::<Kind>(&json).unwrap(), kind);
        }
    }

    #[test]
    fn a_material_with_only_its_identity_reads_and_writes_back() {
        let json = r#"{"id":"a/b","manufacturer":"A","name":"B","kind":"prepreg","source":{"pdf":"b.pdf"}}"#;
        let m: Material = serde_json::from_str(json).unwrap();
        assert_eq!(m, Material::new("a/b", "A", "B", Kind::Prepreg, "b.pdf"));
        let again: Material = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(again, m);
    }

    #[test]
    fn the_glass_transition_to_compare_prefers_dsc() {
        let mut m = Material::new("a/b", "A", "B", Kind::Laminate, "b.pdf");
        assert_eq!(m.tg_c(), None);
        m.tg_tma_c = Some(150.0);
        assert_eq!(m.tg_c(), Some(150.0));
        m.tg_dma_c = Some(170.0);
        assert_eq!(m.tg_c(), Some(170.0));
        m.tg_dsc_c = Some(160.0);
        assert_eq!(m.tg_c(), Some(160.0));
    }
}
