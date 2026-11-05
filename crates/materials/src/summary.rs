//! The small view of a material for the machine program: what its catalogue needs and no more.

use serde::Serialize;

use crate::model::{Copper, DkPoint, Kind, Material, PanelSize, ThicknessRange};

/// The part of a material that the machine program shows. The design tool reads the full record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BungardSummary {
    /// The identifier of the material.
    pub id: String,
    /// The name.
    pub name: String,
    /// The manufacturer.
    pub manufacturer: String,
    /// What it is.
    pub kind: Kind,
    /// The nominal thicknesses in mm.
    pub thicknesses_mm: Vec<f64>,
    /// The range of thicknesses, when the sheet gives a range.
    pub thickness_range: Option<ThicknessRange>,
    /// The copper foils.
    pub copper: Vec<Copper>,
    /// The panel sizes.
    pub panel_sizes: Vec<PanelSize>,
    /// The glass transition (DSC, else DMA, else TMA) in degrees Celsius.
    pub tg_c: Option<f64>,
    /// Dk at the published frequency nearest to 1 GHz.
    pub dk: Option<f64>,
    /// The frequency of `dk` in GHz.
    pub dk_freq_ghz: Option<f64>,
    /// Df at the published frequency nearest to 1 GHz.
    pub df: Option<f64>,
    /// The frequency of `df` in GHz.
    pub df_freq_ghz: Option<f64>,
    /// The thermal conductivity in W/(m K) (metal cores).
    pub thermal_conductivity_w_mk: Option<f64>,
    /// A short note.
    pub notes: Option<String>,
}

/// How far a point is from 1 GHz in decades; a point with no frequency is infinitely far.
fn distance(p: &DkPoint) -> f64 {
    match p.freq_ghz {
        Some(f) if f > 0.0 => f.ln().abs(),
        _ => f64::INFINITY,
    }
}

/// The point of a list nearest to 1 GHz (the first listed when two are equally near).
pub fn representative(list: &[DkPoint]) -> Option<&DkPoint> {
    list.iter()
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
}

impl Material {
    /// The view of the material for the machine program.
    pub fn bungard_summary(&self) -> BungardSummary {
        let dk = representative(&self.dk);
        let df = representative(&self.df);
        BungardSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            manufacturer: self.manufacturer.clone(),
            kind: self.kind,
            thicknesses_mm: self.thicknesses_mm.clone(),
            thickness_range: self.thickness_range.clone(),
            copper: self.copper.clone(),
            panel_sizes: self.panel_sizes.clone(),
            tg_c: self.tg_c(),
            dk: dk.map(|p| p.value),
            dk_freq_ghz: dk.and_then(|p| p.freq_ghz),
            df: df.map(|p| p.value),
            df_freq_ghz: df.and_then(|p| p.freq_ghz),
            thermal_conductivity_w_mk: self.thermal_conductivity_w_mk,
            notes: self.notes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(freq: Option<f64>, value: f64) -> DkPoint {
        DkPoint {
            freq_ghz: freq,
            value,
            method: None,
            resin_content_pct: None,
            tolerance: None,
        }
    }

    #[test]
    fn the_representative_point_is_the_nearest_to_one_gigahertz() {
        let list = [
            p(Some(10.0), 3.4),
            p(Some(2.0), 3.5),
            p(Some(0.4), 3.6),
            p(None, 9.0),
        ];
        assert_eq!(representative(&list).map(|x| x.value), Some(3.5));
        assert_eq!(representative(&[p(None, 9.0)]).map(|x| x.value), Some(9.0));
        assert!(representative(&[]).is_none());
    }

    #[test]
    fn the_summary_keeps_what_the_machine_program_needs() {
        let mut m = Material::new("a/b", "A", "B", Kind::MetalCore, "b.pdf");
        m.tg_tma_c = Some(130.0);
        m.dk = vec![p(Some(1.0), 4.5)];
        m.thicknesses_mm = vec![1.5];
        m.thermal_conductivity_w_mk = Some(2.0);
        let s = m.bungard_summary();
        assert_eq!(s.id, "a/b");
        assert_eq!(s.tg_c, Some(130.0));
        assert_eq!(s.dk, Some(4.5));
        assert_eq!(s.dk_freq_ghz, Some(1.0));
        assert_eq!(s.df, None);
        assert_eq!(s.thicknesses_mm, [1.5]);
        assert_eq!(s.thermal_conductivity_w_mk, Some(2.0));
    }
}
