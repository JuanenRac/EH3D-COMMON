//! The rules the machining data has to follow, checked when it is loaded and by a test over every
//! file.
//!
//! * the schema version is known and every identifier is `<family>/<English name in a slug>`,
//!   unique over all the files;
//! * the name exists in the eight languages;
//! * density, thicknesses, diameters, speeds, feeds, powers and passes are inside the ranges
//!   they can have (`EH3D-MAT-010`);
//! * a material a laser must never touch is not cut or engraved, says why, and has no laser
//!   points; a point of a cut or of an engraving needs the material to allow it (`EH3D-MAT-011`);
//! * a cut point names its thickness and no two points are the same;
//! * every record names its source, which is not a path.

use std::collections::HashSet;

use super::model::{LaserMode, MachiningFile, Workpiece, SCHEMA_VERSION};
use crate::error::MatError;
use crate::i18n::Lang;
use crate::validate::slug;

/// The range of a density in g/cm3.
pub const DENSITY_RANGE: (f64, f64) = (0.01, 25.0);
/// The range of a thickness in mm.
pub const THICKNESS_RANGE: (f64, f64) = (0.01, 200.0);
/// The range of a cutter diameter in mm.
pub const DIAMETER_RANGE: (f64, f64) = (0.05, 50.0);
/// The range of a spindle speed in revolutions per minute.
pub const RPM_RANGE: (f64, f64) = (100.0, 100_000.0);
/// The range of a feed in mm per minute.
pub const FEED_RANGE: (f64, f64) = (1.0, 20_000.0);
/// The range of the depth of a pass in mm.
pub const DEPTH_RANGE: (f64, f64) = (0.01, 50.0);
/// The range of a step over in percent of the diameter.
pub const STEPOVER_RANGE: (f64, f64) = (1.0, 100.0);
/// The range of the nominal power of a laser in watts.
pub const POWER_W_RANGE: (f64, f64) = (0.1, 500.0);
/// The range of a power setting in percent.
pub const POWER_PCT_RANGE: (f64, f64) = (0.1, 100.0);
/// The range of the speed of a laser in mm per minute.
pub const LASER_SPEED_RANGE: (f64, f64) = (1.0, 100_000.0);
/// The range of the number of passes.
pub const PASSES_RANGE: (f64, f64) = (1.0, 50.0);

/// The identifier a workpiece has from its family and its English name.
pub fn make_id(w: &Workpiece) -> String {
    format!("{}/{}", w.family.as_str(), slug(&w.names.en))
}

fn range(errors: &mut Vec<MatError>, id: &str, field: &str, value: f64, r: (f64, f64)) {
    // not `r.0 <= value`: a NaN must fail too
    if value.is_nan() || value < r.0 || value > r.1 {
        errors.push(MatError::PointRange {
            id: id.to_owned(),
            field: field.to_owned(),
            value,
            min: r.0,
            max: r.1,
        });
    }
}

fn range_opt(errors: &mut Vec<MatError>, id: &str, field: &str, value: Option<f64>, r: (f64, f64)) {
    if let Some(v) = value {
        range(errors, id, field, v, r);
    }
}

fn bad(id: &str, reason: String) -> MatError {
    MatError::Workpiece {
        id: id.to_owned(),
        reason,
    }
}

fn plain_file_text(text: &str) -> bool {
    !text.trim().is_empty() && !text.contains(['/', '\\']) && !text.contains(':')
}

/// Every rule broken by one workpiece (an empty list when it is sound).
pub fn check_workpiece(w: &Workpiece) -> Vec<MatError> {
    let mut errors = Vec::new();
    let id = w.id.as_str();

    if id != make_id(w) {
        errors.push(MatError::IdMismatch {
            id: id.to_owned(),
            reason: format!("the family and the English name give {}", make_id(w)),
        });
    }
    for lang in Lang::ALL {
        if w.names.get(lang).trim().is_empty() {
            errors.push(bad(id, format!("the name in {} is empty", lang.code())));
        }
    }

    range_opt(
        &mut errors,
        id,
        "density_g_cm3",
        w.density_g_cm3,
        DENSITY_RANGE,
    );
    for (i, t) in w.thicknesses_mm.iter().enumerate() {
        range(
            &mut errors,
            id,
            &format!("thicknesses_mm[{i}]"),
            *t,
            THICKNESS_RANGE,
        );
    }

    // safety
    let s = &w.safety;
    if s.laser_unsafe && (s.laser_cut_allowed || s.laser_engrave_allowed) {
        errors.push(bad(
            id,
            "a material a laser must never touch cannot allow a cut or an engraving".to_owned(),
        ));
    }
    if !s.laser_cut_allowed && !s.laser_engrave_allowed && s.hazards.is_empty() {
        errors.push(bad(
            id,
            "a material the laser refuses has to say why (hazards)".to_owned(),
        ));
    }
    if s.laser_unsafe && s.hazards.is_empty() {
        errors.push(bad(
            id,
            "a material a laser must never touch has to say why (hazards)".to_owned(),
        ));
    }
    let mut seen = HashSet::new();
    for h in &s.hazards {
        if !seen.insert(*h) {
            errors.push(bad(id, format!("the hazard {h:?} is listed twice")));
        }
    }

    // CNC points
    for (i, p) in w.cnc.iter().enumerate() {
        let at = format!("cnc[{i}]");
        range(
            &mut errors,
            id,
            &format!("{at}.diameter_mm"),
            p.diameter_mm,
            DIAMETER_RANGE,
        );
        range(&mut errors, id, &format!("{at}.rpm"), p.rpm, RPM_RANGE);
        range(
            &mut errors,
            id,
            &format!("{at}.feed_mm_min"),
            p.feed_mm_min,
            FEED_RANGE,
        );
        range_opt(
            &mut errors,
            id,
            &format!("{at}.plunge_mm_min"),
            p.plunge_mm_min,
            FEED_RANGE,
        );
        range(
            &mut errors,
            id,
            &format!("{at}.depth_per_pass_mm"),
            p.depth_per_pass_mm,
            DEPTH_RANGE,
        );
        range_opt(
            &mut errors,
            id,
            &format!("{at}.stepover_pct"),
            p.stepover_pct,
            STEPOVER_RANGE,
        );
        if !plain_file_text(&p.source) {
            errors.push(MatError::Source {
                id: id.to_owned(),
                reason: format!("the source of {at} is empty or looks like a path"),
            });
        }
        if w.cnc[..i].iter().any(|q| {
            q.op == p.op && q.tool == p.tool && (q.diameter_mm - p.diameter_mm).abs() < 1e-9
        }) {
            errors.push(bad(id, format!("{at} repeats an earlier point")));
        }
    }

    // laser points
    for (i, p) in w.laser.iter().enumerate() {
        let at = format!("laser[{i}]");
        let allowed = match p.mode {
            LaserMode::Cut => s.laser_cut_allowed && !s.laser_unsafe,
            LaserMode::Engrave => s.laser_engrave_allowed && !s.laser_unsafe,
        };
        if !allowed {
            errors.push(MatError::LaserRefused {
                id: id.to_owned(),
                reason: format!(
                    "{at} is a {} point and the material does not allow it",
                    p.mode.as_str()
                ),
            });
        }
        range(
            &mut errors,
            id,
            &format!("{at}.power_w"),
            p.power_w,
            POWER_W_RANGE,
        );
        range(
            &mut errors,
            id,
            &format!("{at}.power_pct"),
            p.power_pct,
            POWER_PCT_RANGE,
        );
        range(
            &mut errors,
            id,
            &format!("{at}.speed_mm_min"),
            p.speed_mm_min,
            LASER_SPEED_RANGE,
        );
        range(
            &mut errors,
            id,
            &format!("{at}.passes"),
            f64::from(p.passes),
            PASSES_RANGE,
        );
        match (p.mode, p.thickness_mm) {
            (LaserMode::Cut, None) => {
                errors.push(bad(id, format!("{at} is a cut and names no thickness")))
            }
            (LaserMode::Engrave, Some(_)) => errors.push(bad(
                id,
                format!("{at} is an engraving and names a thickness"),
            )),
            (_, t) => range_opt(
                &mut errors,
                id,
                &format!("{at}.thickness_mm"),
                t,
                THICKNESS_RANGE,
            ),
        }
        if !plain_file_text(&p.source) {
            errors.push(MatError::Source {
                id: id.to_owned(),
                reason: format!("the source of {at} is empty or looks like a path"),
            });
        }
        if w.laser[..i].iter().any(|q| {
            q.kind == p.kind
                && q.mode == p.mode
                && (q.power_w - p.power_w).abs() < 1e-9
                && match (q.thickness_mm, p.thickness_mm) {
                    (Some(a), Some(b)) => (a - b).abs() < 1e-9,
                    (None, None) => true,
                    _ => false,
                }
        }) {
            errors.push(bad(id, format!("{at} repeats an earlier point")));
        }
    }

    if !plain_file_text(&w.source) {
        errors.push(MatError::Source {
            id: id.to_owned(),
            reason: "the source is empty or looks like a path".to_owned(),
        });
    }
    errors
}

/// Every rule broken by one machining data file, without the rule between files.
pub fn check_file(file_name: &str, file: &MachiningFile) -> Vec<MatError> {
    let mut errors = Vec::new();
    if file.schema_version != SCHEMA_VERSION {
        errors.push(MatError::SchemaVersion {
            file: file_name.to_owned(),
            found: file.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    for w in &file.materials {
        errors.extend(check_workpiece(w));
    }
    errors
}

/// The identifiers that appear more than once in a list of workpieces.
pub fn check_unique<'a>(items: impl IntoIterator<Item = &'a Workpiece>) -> Vec<MatError> {
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    for w in items {
        if !seen.insert(w.id.as_str()) {
            errors.push(MatError::DuplicateId { id: w.id.clone() });
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MatCode;
    use crate::machining::model::{
        CncOp, CncPoint, Family, Hazard, LaserKind, LaserPoint, Names, Safety, Tool,
    };

    fn good() -> Workpiece {
        let n = |s: &str| s.to_owned();
        Workpiece {
            id: "wood/birch-plywood".into(),
            family: Family::Wood,
            names: Names {
                en: n("Birch plywood"),
                es: n("Contrachapado de abedul"),
                de: n("Birkensperrholz"),
                fr: n("Contreplaqué de bouleau"),
                it: n("Compensato di betulla"),
                ja: n("シナ合板"),
                zh: n("桦木胶合板"),
                pt: n("Contraplacado de bétula"),
            },
            density_g_cm3: Some(0.68),
            thicknesses_mm: vec![3.0, 6.0],
            safety: Safety {
                laser_cut_allowed: true,
                laser_engrave_allowed: true,
                laser_unsafe: false,
                hazards: vec![Hazard::FineDust],
            },
            cnc: vec![CncPoint {
                op: CncOp::Contour,
                tool: Tool::EndMill,
                diameter_mm: 3.0,
                flutes: Some(2),
                rpm: 18000.0,
                feed_mm_min: 1200.0,
                plunge_mm_min: Some(300.0),
                depth_per_pass_mm: 1.5,
                stepover_pct: None,
                verified: false,
                source: "general practice".into(),
            }],
            laser: vec![LaserPoint {
                kind: LaserKind::Co2,
                mode: LaserMode::Cut,
                power_w: 40.0,
                thickness_mm: Some(3.0),
                power_pct: 80.0,
                speed_mm_min: 600.0,
                passes: 1,
                air_assist: Some(true),
                verified: false,
                source: "general practice".into(),
            }],
            pcb_material_id: None,
            notes: None,
            source: "general reference".into(),
        }
    }

    fn codes(w: &Workpiece) -> Vec<MatCode> {
        check_workpiece(w).iter().map(MatError::code).collect()
    }

    #[test]
    fn a_sound_workpiece_has_no_error() {
        assert!(check_workpiece(&good()).is_empty());
        assert_eq!(make_id(&good()), "wood/birch-plywood");
    }

    #[test]
    fn the_identifier_follows_the_family_and_the_english_name() {
        let mut w = good();
        w.id = "wood/plywood".into();
        assert_eq!(codes(&w), [MatCode::IdMismatch]);
        let mut w = good();
        w.family = Family::Plastic;
        assert_eq!(codes(&w), [MatCode::IdMismatch]);
    }

    #[test]
    fn a_name_has_to_exist_in_the_eight_languages() {
        let mut w = good();
        w.names.pt = "  ".into();
        w.names.ja.clear();
        assert_eq!(codes(&w), [MatCode::Workpiece, MatCode::Workpiece]);
    }

    #[test]
    fn values_are_held_to_their_ranges_and_a_nan_fails() {
        let mut w = good();
        w.density_g_cm3 = Some(40.0);
        w.thicknesses_mm = vec![0.0];
        w.cnc[0].rpm = 5.0;
        w.cnc[0].feed_mm_min = f64::NAN;
        w.cnc[0].stepover_pct = Some(250.0);
        w.laser[0].power_pct = 150.0;
        w.laser[0].passes = 0;
        let fields: Vec<String> = check_workpiece(&w)
            .into_iter()
            .filter_map(|e| match e {
                MatError::PointRange { field, .. } => Some(field),
                _ => None,
            })
            .collect();
        assert_eq!(fields.len(), 7, "{fields:?}");
        assert!(fields.contains(&"cnc[0].feed_mm_min".to_owned()));
    }

    #[test]
    fn a_material_a_laser_must_never_touch_cannot_allow_it_nor_have_laser_points() {
        let mut w = good();
        w.safety.laser_unsafe = true;
        assert!(codes(&w).contains(&MatCode::Workpiece));
        w.safety.laser_cut_allowed = false;
        w.safety.laser_engrave_allowed = false;
        // points of a refused cut are an error of their own
        assert_eq!(codes(&w), [MatCode::LaserRefused]);
        w.laser.clear();
        assert!(check_workpiece(&w).is_empty());
        w.safety.hazards.clear();
        assert_eq!(codes(&w), [MatCode::Workpiece, MatCode::Workpiece]);
    }

    #[test]
    fn a_refusal_has_to_say_why_and_a_hazard_is_listed_once() {
        let mut w = good();
        w.laser.clear();
        w.safety.laser_cut_allowed = false;
        w.safety.laser_engrave_allowed = false;
        w.safety.hazards = vec![Hazard::Reflective, Hazard::Reflective];
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.safety.hazards.clear();
        assert_eq!(codes(&w), [MatCode::Workpiece]);
    }

    #[test]
    fn a_cut_names_its_thickness_an_engraving_does_not_and_a_point_is_not_repeated() {
        let mut w = good();
        w.laser[0].thickness_mm = None;
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        let mut w = good();
        w.laser[0].mode = LaserMode::Engrave;
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        let mut w = good();
        w.laser.push(w.laser[0].clone());
        w.cnc.push(w.cnc[0].clone());
        assert_eq!(codes(&w), [MatCode::Workpiece, MatCode::Workpiece]);
    }

    #[test]
    fn the_sources_are_text_and_not_paths() {
        let mut w = good();
        w.source = "C:\\data\\x".into();
        w.cnc[0].source = String::new();
        w.laser[0].source = "dir/file".into();
        assert_eq!(
            codes(&w),
            [MatCode::Source, MatCode::Source, MatCode::Source]
        );
    }

    #[test]
    fn the_file_checks_the_version_and_the_identifiers_between_files() {
        let file = MachiningFile {
            schema_version: 7,
            materials: vec![good(), good()],
        };
        let found: Vec<MatCode> = check_file("x.json", &file)
            .iter()
            .map(MatError::code)
            .collect();
        assert_eq!(found, [MatCode::SchemaVersion]);
        let dup = check_unique(&file.materials);
        assert_eq!(dup.len(), 1);
        assert_eq!(dup[0].code(), MatCode::DuplicateId);
    }
}
