//! The machining database in memory, and what it can be asked.

use super::data;
use super::model::{CncOp, Family, LaserProcess, MachiningFile, Tool, Workpiece};
use super::start::{CncStart, LaserStart};
use super::validate::{check_file, check_unique};
use crate::error::MatError;
use crate::query::Database;

/// Reads one machining data file.
pub fn parse_file(file_name: &str, json: &str) -> Result<MachiningFile, MatError> {
    serde_json::from_str(json).map_err(|e| MatError::Parse {
        file: file_name.to_owned(),
        message: e.to_string(),
    })
}

/// Every workpiece of the machining family, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct MachiningDatabase {
    items: Vec<Workpiece>,
}

impl MachiningDatabase {
    /// The database of the data files embedded in the crate.
    pub fn embedded() -> Result<MachiningDatabase, MatError> {
        MachiningDatabase::from_sources(data::FILES)
    }

    /// A database from `(file name, JSON text)` pairs. Every file is read and checked; the first
    /// error is returned.
    pub fn from_sources(files: &[(&str, &str)]) -> Result<MachiningDatabase, MatError> {
        let mut items = Vec::new();
        for (name, json) in files {
            let file = parse_file(name, json)?;
            if let Some(e) = check_file(name, &file).into_iter().next() {
                return Err(e);
            }
            items.extend(file.materials);
        }
        if let Some(e) = check_unique(&items).into_iter().next() {
            return Err(e);
        }
        Ok(MachiningDatabase { items })
    }

    /// Every workpiece.
    pub fn materials(&self) -> &[Workpiece] {
        &self.items
    }

    /// The number of workpieces.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The workpiece with an identifier.
    pub fn get(&self, id: &str) -> Option<&Workpiece> {
        self.items.iter().find(|w| w.id == id)
    }

    /// The workpieces of a family.
    pub fn by_family(&self, family: Family) -> impl Iterator<Item = &Workpiece> {
        self.items.iter().filter(move |w| w.family == family)
    }

    /// The workpieces a laser may cut.
    pub fn laser_cuttable(&self) -> impl Iterator<Item = &Workpiece> {
        self.items.iter().filter(|w| w.laser_cut_allowed())
    }

    /// The workpieces a laser must never touch.
    pub fn laser_unsafe(&self) -> impl Iterator<Item = &Workpiece> {
        self.items.iter().filter(|w| w.laser_unsafe())
    }

    /// Whether a laser may cut the material with this identifier (`None` when it is not known).
    pub fn laser_cut_allowed(&self, id: &str) -> Option<bool> {
        self.get(id).map(Workpiece::laser_cut_allowed)
    }

    /// A starting point for a CNC router (see [`Workpiece::cnc_start`]); an unknown identifier is
    /// an error of the code `EH3D-MAT-012`.
    pub fn cnc_start(
        &self,
        id: &str,
        op: CncOp,
        tool: Tool,
        diameter_mm: f64,
    ) -> Result<CncStart, MatError> {
        self.known(id)?.cnc_start_checked(op, tool, diameter_mm)
    }

    /// A starting point for a laser (see [`Workpiece::laser_start`]); an unknown identifier is an
    /// error of the code `EH3D-MAT-012`.
    pub fn laser_start(
        &self,
        id: &str,
        power_w: f64,
        thickness_mm: f64,
        process: LaserProcess,
    ) -> Result<LaserStart, MatError> {
        self.known(id)?
            .laser_start_checked(power_w, thickness_mm, process)
    }

    fn known(&self, id: &str) -> Result<&Workpiece, MatError> {
        self.get(id).ok_or_else(|| MatError::NoStartPoints {
            id: id.to_owned(),
            reason: "it is not in the machining database".to_owned(),
        })
    }

    /// The links of the board stock to the board database: every `pcb_material_id` has to name a
    /// material of it (`EH3D-MAT-015`). Returns every broken link.
    pub fn check_links(&self, boards: &Database) -> Vec<MatError> {
        self.items
            .iter()
            .filter_map(|w| {
                let link = w.pcb_material_id.as_ref()?;
                boards.get(link).is_none().then(|| MatError::BrokenLink {
                    id: w.id.clone(),
                    link: link.clone(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MatCode;
    use crate::machining::model::Hazard;

    fn db() -> MachiningDatabase {
        MachiningDatabase::embedded().unwrap()
    }

    #[test]
    fn the_embedded_seed_is_sound_and_covers_every_family() {
        let db = db();
        assert!(db.len() >= 40, "{}", db.len());
        for family in Family::ALL {
            if family == Family::Composite || family == Family::Mineral {
                assert!(db.by_family(family).count() >= 1, "{family:?}");
            } else {
                assert!(db.by_family(family).count() >= 2, "{family:?}");
            }
        }
        for w in db.materials() {
            assert!(
                crate::machining::validate::check_workpiece(w).is_empty(),
                "{}",
                w.id
            );
        }
    }

    #[test]
    fn no_point_of_the_seed_says_it_was_tried_on_a_machine() {
        for w in db().materials() {
            for p in &w.cnc {
                assert!(!p.verified, "{}", w.id);
            }
            for p in &w.laser {
                assert!(!p.verified, "{}", w.id);
            }
        }
    }

    #[test]
    fn the_materials_a_laser_must_never_touch_are_the_ones_that_give_off_poisons() {
        let db = db();
        for id in [
            "plastic/rigid-pvc",
            "plastic/foam-pvc-board",
            "plastic/abs",
            "plastic/polycarbonate-pc",
            "plastic/adhesive-vinyl",
            "plastic/acetal-pom",
            "plastic/ptfe",
            "composite/carbon-fibre-sheet",
            "pcb_stock/fr-4-copper-clad-board",
            "pcb_stock/fr-4-unclad-sheet",
            "rubber/neoprene",
            "leather/faux-leather-pu-or-pvc",
            "foam/polyurethane-foam",
        ] {
            let w = db.get(id).unwrap_or_else(|| panic!("{id}"));
            assert!(
                w.laser_unsafe() && !w.laser_cut_allowed() && !w.laser_engrave_allowed(),
                "{id}"
            );
            assert!(!w.safety.hazards.is_empty(), "{id}");
            assert!(w.laser.is_empty(), "{id}");
        }
        assert_eq!(
            db.laser_cut_allowed("plastic/polycarbonate-pc"),
            Some(false)
        );
        assert_eq!(db.laser_cut_allowed("wood/birch-plywood"), Some(true));
        assert_eq!(db.laser_cut_allowed("wood/unknown"), None);
        // PVC and vinyl say chlorine
        let pvc = db.get("plastic/rigid-pvc").unwrap();
        assert!(pvc.safety.hazards.contains(&Hazard::Chlorine));
        // a bare metal is refused for the beam, not for the fumes
        let al = db.get("metal/aluminium-6061").unwrap();
        assert!(!al.laser_cut_allowed() && !al.laser_unsafe());
        assert_eq!(al.safety.hazards, [Hazard::Reflective]);
        assert!(db.laser_unsafe().count() >= 13);
        assert!(db.laser_cuttable().count() >= 20);
        assert!(db.laser_cuttable().all(|w| !w.laser_unsafe()));
    }

    #[test]
    fn the_queries_by_identifier_give_points_and_typed_errors() {
        let db = db();
        let exact = db
            .cnc_start("wood/birch-plywood", CncOp::Contour, Tool::EndMill, 3.0)
            .unwrap();
        assert_eq!(exact.rpm, 18000.0);
        let between = db
            .cnc_start("wood/birch-plywood", CncOp::Contour, Tool::EndMill, 4.5)
            .unwrap();
        assert!(between.interpolated && between.rpm < 18000.0 && between.rpm > 16000.0);
        let e = db
            .cnc_start("wood/birch-plywood", CncOp::Contour, Tool::EndMill, 12.0)
            .unwrap_err();
        assert_eq!(e.code(), MatCode::OutsideKnown);
        let e = db
            .cnc_start("wood/none", CncOp::Contour, Tool::EndMill, 3.0)
            .unwrap_err();
        assert_eq!(e.code(), MatCode::NoStartPoints);

        let ok = db
            .laser_start("wood/birch-plywood", 40.0, 4.5, LaserProcess::CO2_CUT)
            .unwrap();
        assert!(ok.interpolated && ok.speed_mm_min < 600.0 && ok.speed_mm_min > 240.0);
        let e = db
            .laser_start("wood/birch-plywood", 40.0, 12.0, LaserProcess::CO2_CUT)
            .unwrap_err();
        assert_eq!(e.code(), MatCode::OutsideKnown);
        let e = db
            .laser_start("wood/birch-plywood", 200.0, 3.0, LaserProcess::CO2_CUT)
            .unwrap_err();
        assert_eq!(e.code(), MatCode::OutsideKnown);
        let e = db
            .laser_start("plastic/rigid-pvc", 40.0, 3.0, LaserProcess::CO2_CUT)
            .unwrap_err();
        assert_eq!(e.code(), MatCode::LaserRefused);
        let e = db
            .laser_start("metal/copper-cu-etp", 40.0, 1.0, LaserProcess::CO2_CUT)
            .unwrap_err();
        assert_eq!(e.code(), MatCode::LaserRefused);
        // acrylic is cut by a CO2 laser; the seed has no diode point for it
        let e = db
            .laser_start(
                "plastic/cast-acrylic-pmma",
                10.0,
                3.0,
                LaserProcess::DIODE_CUT,
            )
            .unwrap_err();
        assert_eq!(e.code(), MatCode::NoStartPoints);
    }

    #[test]
    fn every_point_of_the_seed_can_be_asked_for_by_its_own_values() {
        for w in db().materials() {
            for p in &w.cnc {
                let s = w.cnc_start(p.op, p.tool, p.diameter_mm).unwrap();
                assert!(
                    !s.interpolated && s.rpm == p.rpm && s.feed_mm_min == p.feed_mm_min,
                    "{}",
                    w.id
                );
            }
            for p in &w.laser {
                let process = LaserProcess {
                    kind: p.kind,
                    mode: p.mode,
                };
                let s = w
                    .laser_start(p.power_w, p.thickness_mm.unwrap_or(0.0), process)
                    .unwrap();
                assert!(
                    !s.interpolated
                        && s.power_pct == p.power_pct
                        && s.speed_mm_min == p.speed_mm_min,
                    "{}",
                    w.id
                );
            }
        }
    }

    #[test]
    fn the_board_stock_names_laminates_that_exist_in_the_board_database() {
        let boards = Database::embedded().unwrap();
        let db = db();
        assert!(db.check_links(&boards).is_empty());
        let stock: Vec<_> = db.by_family(Family::PcbStock).collect();
        assert!(stock.len() >= 4);
        assert!(stock.iter().all(|w| w.pcb_material_id.is_some()));
        // a link that does not exist is found
        let broken = MachiningDatabase::from_sources(&[(
            "x.json",
            include_str!("../../data/machining/board.json"),
        )])
        .unwrap();
        let mut moved = broken.clone();
        moved.items[0].pcb_material_id = Some("bungard/none".into());
        let errors = moved.check_links(&boards);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), MatCode::BrokenLink);
    }

    #[test]
    fn a_bad_machining_file_is_refused_with_its_code() {
        let e = MachiningDatabase::from_sources(&[("x.json", "{ not json")]).unwrap_err();
        assert_eq!(e.code(), MatCode::Parse);
        let e = MachiningDatabase::from_sources(&[(
            "x.json",
            r#"{"schema_version":9,"materials":[]}"#,
        )])
        .unwrap_err();
        assert_eq!(e.code(), MatCode::SchemaVersion);
        let e = MachiningDatabase::from_sources(&[(
            "x.json",
            r#"{"schema_version":1,"materials":[],"price":3}"#,
        )])
        .unwrap_err();
        assert_eq!(e.code(), MatCode::Parse);
        let one = include_str!("../../data/machining/metal.json");
        let e = MachiningDatabase::from_sources(&[("a.json", one), ("b.json", one)]).unwrap_err();
        assert_eq!(e.code(), MatCode::DuplicateId);
    }
}
