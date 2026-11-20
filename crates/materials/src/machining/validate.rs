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
//! * every record names its source, which is not a path;
//! * the optional fields of a record are held to rules too: a range is ordered and holds the
//!   single value of its point, a speed given in another unit agrees with the one in mm per
//!   minute, the air assist level agrees with the flag, a source has its date and its licence,
//!   a second figure for the same case is marked as an alternative, and a point that says it
//!   is verified has evidence from the maker or from a machine (`EH3D-MAT-014`);
//! * a material with a hazard that poisons (chlorine, cyanide, fluorine, chromium, toxic fumes)
//!   is never cut or engraved by a laser.

use std::collections::HashSet;

use super::model::{
    CncPoint, Evidence, Hazard, LaserMode, LaserPoint, MachiningFile, SourceInfo, Span, Tool,
    Workpiece, SCHEMA_VERSION,
};
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

/// The range of the tip angle of a V-bit in degrees.
pub const TIP_ANGLE_RANGE: (f64, f64) = (1.0, 180.0);
/// The range of the number of flutes.
pub const FLUTES_RANGE: (f64, f64) = (1.0, 12.0);
/// The range of the power of a spindle in watts.
pub const SPINDLE_W_RANGE: (f64, f64) = (1.0, 50_000.0);
/// The hazards that poison the room: a material with one of them is never put under a laser.
pub const POISONING_HAZARDS: [Hazard; 5] = [
    Hazard::Chlorine,
    Hazard::Cyanide,
    Hazard::Fluorine,
    Hazard::Chromium,
    Hazard::ToxicFumes,
];

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

fn short_text(text: &str) -> bool {
    !text.trim().is_empty() && text.chars().count() <= 160 && !text.contains(['\n', '\r'])
}

fn valid_date(d: &str) -> bool {
    let b = d.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let num = |r: std::ops::Range<usize>| d[r].parse::<u32>().unwrap_or(0);
    match b.len() {
        7 => digits(0..4) && b[4] == b'-' && digits(5..7) && (1..=12).contains(&num(5..7)),
        10 => {
            digits(0..4)
                && b[4] == b'-'
                && digits(5..7)
                && b[7] == b'-'
                && digits(8..10)
                && (1..=12).contains(&num(5..7))
                && (1..=31).contains(&num(8..10))
        }
        _ => false,
    }
}

fn check_source_info(errors: &mut Vec<MatError>, id: &str, at: &str, info: &SourceInfo) {
    if !plain_file_text(&info.origin) || !short_text(&info.origin) {
        errors.push(MatError::Source {
            id: id.to_owned(),
            reason: format!("the origin of the source of {at} is empty or looks like a path"),
        });
    }
    if !valid_date(&info.date) {
        errors.push(MatError::Source {
            id: id.to_owned(),
            reason: format!("the date of the source of {at} is not YYYY-MM or YYYY-MM-DD"),
        });
    }
    if !short_text(&info.license) {
        errors.push(MatError::Source {
            id: id.to_owned(),
            reason: format!("the source of {at} does not name its licence"),
        });
    }
    if let Some(url) = &info.url {
        if !url.starts_with("https://") || url.contains(char::is_whitespace) {
            errors.push(MatError::Source {
                id: id.to_owned(),
                reason: format!("the address of the source of {at} is not an https address"),
            });
        }
    }
}

/// A range has its two ends inside the limits, in order, and holds the single value.
fn check_span(
    errors: &mut Vec<MatError>,
    id: &str,
    field: &str,
    span: Span,
    limits: (f64, f64),
    value: Option<f64>,
) {
    range(errors, id, &format!("{field}.min"), span[0], limits);
    range(errors, id, &format!("{field}.max"), span[1], limits);
    if span[0] > span[1] {
        errors.push(bad(id, format!("the range of {field} is reversed")));
        return;
    }
    if let Some(v) = value {
        if v.is_nan() || v < span[0] - 1e-9 || v > span[1] + 1e-9 {
            errors.push(bad(
                id,
                format!(
                    "{field} = {v} is outside its own range {} to {}",
                    span[0], span[1]
                ),
            ));
        }
    }
}

/// `verified` needs evidence from the maker or from a machine, and a source with its licence; a
/// test on a machine names the machine.
fn check_evidence(
    errors: &mut Vec<MatError>,
    id: &str,
    at: &str,
    verified: bool,
    evidence: Option<Evidence>,
    info: Option<&SourceInfo>,
    machine: Option<&String>,
) {
    if verified {
        let strong = evidence.is_some_and(Evidence::allows_verified);
        if !strong || info.is_none() {
            errors.push(bad(
                id,
                format!(
                    "{at} says it is verified and has no evidence from the maker or from a machine with its source"
                ),
            ));
        }
    }
    if evidence == Some(Evidence::TestedOnMachine) && machine.is_none() {
        errors.push(bad(
            id,
            format!("{at} was tried on a machine and does not name it"),
        ));
    }
    if let Some(i) = info {
        check_source_info(errors, id, at, i);
    }
    if let Some(m) = machine {
        if !short_text(m) {
            errors.push(bad(id, format!("the machine of {at} is empty or too long")));
        }
    }
}

fn check_cnc_extras(errors: &mut Vec<MatError>, id: &str, at: &str, p: &CncPoint) {
    if let Some(r) = &p.ranges {
        let flutes = p.flutes.map(f64::from);
        for (name, span) in r.spans() {
            let (limits, value) = match name {
                "flutes" => (FLUTES_RANGE, flutes),
                "rpm" => (RPM_RANGE, Some(p.rpm)),
                "feed_mm_min" => (FEED_RANGE, Some(p.feed_mm_min)),
                "plunge_mm_min" => (FEED_RANGE, p.plunge_mm_min),
                "depth_per_pass_mm" => (DEPTH_RANGE, Some(p.depth_per_pass_mm)),
                _ => (STEPOVER_RANGE, p.stepover_pct),
            };
            check_span(
                errors,
                id,
                &format!("{at}.ranges.{name}"),
                span,
                limits,
                value,
            );
        }
    }
    if let Some(angle) = p.tip_angle_deg {
        if p.tool != Tool::VBit {
            errors.push(bad(id, format!("{at} has a tip angle and is not a V-bit")));
        }
        check_span(
            errors,
            id,
            &format!("{at}.tip_angle_deg"),
            angle,
            TIP_ANGLE_RANGE,
            None,
        );
    }
    if let Some(c) = &p.coating {
        if !short_text(c) {
            errors.push(bad(id, format!("the coating of {at} is empty or too long")));
        }
    }
    range_opt(
        errors,
        id,
        &format!("{at}.spindle_w"),
        p.spindle_w,
        SPINDLE_W_RANGE,
    );
    check_evidence(
        errors,
        id,
        at,
        p.verified,
        p.evidence,
        p.source_info.as_ref(),
        p.machine.as_ref(),
    );
}

fn check_laser_extras(errors: &mut Vec<MatError>, id: &str, at: &str, p: &LaserPoint) {
    if let Some(r) = &p.ranges {
        for (name, span) in r.spans() {
            let (limits, value) = match name {
                "power_pct" => (POWER_PCT_RANGE, Some(p.power_pct)),
                "speed_mm_min" => (LASER_SPEED_RANGE, Some(p.speed_mm_min)),
                _ => (PASSES_RANGE, Some(f64::from(p.passes))),
            };
            check_span(
                errors,
                id,
                &format!("{at}.ranges.{name}"),
                span,
                limits,
                value,
            );
        }
    }
    if let (Some(level), Some(flag)) = (p.air_assist_level, p.air_assist) {
        if level.is_on() != flag {
            errors.push(bad(
                id,
                format!("{at}: the air assist level and the air assist flag disagree"),
            ));
        }
    }
    match (p.source_unit, p.source_speed) {
        (None, None) => {}
        (Some(unit), Some(speed)) => {
            range(
                errors,
                id,
                &format!("{at}.source_speed"),
                speed,
                (0.001, 100_000.0),
            );
            if (speed * unit.to_mm_min() - p.speed_mm_min).abs() > 0.01 {
                errors.push(bad(
                    id,
                    format!("{at}: the speed of the source does not give speed_mm_min"),
                ));
            }
        }
        _ => errors.push(bad(
            id,
            format!("{at}: the unit and the value of the speed of the source go together"),
        )),
    }
    if p.alternative && (p.source_info.is_none() || p.evidence.is_none()) {
        errors.push(bad(
            id,
            format!("{at} is an alternative and does not name its source and its evidence"),
        ));
    }
    check_evidence(
        errors,
        id,
        at,
        p.verified,
        p.evidence,
        p.source_info.as_ref(),
        p.machine.as_ref(),
    );
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
    if s.hazards.iter().any(|h| POISONING_HAZARDS.contains(h))
        && (s.laser_cut_allowed || s.laser_engrave_allowed)
    {
        errors.push(bad(
            id,
            "a material that gives off poisons or corrosive fumes cannot allow a cut or an engraving"
                .to_owned(),
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
        check_cnc_extras(&mut errors, id, &at, p);
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
        check_laser_extras(&mut errors, id, &at, p);
        if w.laser[..i].iter().any(|q| {
            q.alternative == p.alternative
                && (!p.alternative || q.source == p.source)
                && q.kind == p.kind
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

    // a second figure sits next to a main one of the same kind and mode
    for (i, p) in w.laser.iter().enumerate() {
        if p.alternative
            && !w
                .laser
                .iter()
                .any(|q| !q.alternative && q.kind == p.kind && q.mode == p.mode)
        {
            errors.push(bad(
                id,
                format!("laser[{i}] is an alternative and has no main point of its kind and mode"),
            ));
        }
    }

    if !plain_file_text(&w.source) {
        errors.push(MatError::Source {
            id: id.to_owned(),
            reason: "the source is empty or looks like a path".to_owned(),
        });
    }
    if let Some(info) = &w.source_info {
        check_source_info(&mut errors, id, "the record", info);
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
                ranges: None,
                tip_angle_deg: None,
                cutter_style: None,
                coating: None,
                coolant: None,
                machine: None,
                spindle_w: None,
                evidence: None,
                source_info: None,
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
                ranges: None,
                air_assist_level: None,
                source_unit: None,
                source_speed: None,
                machine: None,
                alternative: false,
                evidence: None,
                source_info: None,
            }],
            pcb_material_id: None,
            notes: None,
            source: "general reference".into(),
            source_info: None,
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
    fn info() -> SourceInfo {
        SourceInfo {
            origin: "A public table".into(),
            date: "2026-11".into(),
            license: "MIT".into(),
            url: Some("https://example.org/table".into()),
        }
    }

    #[test]
    fn a_range_is_ordered_inside_its_limits_and_holds_the_single_value() {
        use crate::machining::model::{CncRanges, LaserRanges};
        let mut w = good();
        w.cnc[0].ranges = Some(CncRanges {
            rpm: Some([16000.0, 20000.0]),
            ..CncRanges::default()
        });
        w.laser[0].ranges = Some(LaserRanges {
            power_pct: Some([70.0, 90.0]),
            passes: Some([1.0, 2.0]),
            ..LaserRanges::default()
        });
        assert!(check_workpiece(&w).is_empty());
        // reversed
        w.cnc[0].ranges.as_mut().unwrap().rpm = Some([20000.0, 16000.0]);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        // the single value is outside
        w.cnc[0].ranges.as_mut().unwrap().rpm = Some([1000.0, 2000.0]);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        // an end outside the limits of the field
        w.cnc[0].ranges.as_mut().unwrap().rpm = Some([18000.0, 900_000.0]);
        assert_eq!(codes(&w), [MatCode::PointRange]);
        w.cnc[0].ranges = None;
        w.laser[0].ranges.as_mut().unwrap().passes = Some([3.0, 4.0]);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
    }

    #[test]
    fn a_tip_angle_belongs_to_a_v_bit() {
        let mut w = good();
        w.cnc[0].tip_angle_deg = Some([30.0, 60.0]);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.cnc[0].tool = Tool::VBit;
        assert!(check_workpiece(&w).is_empty());
        w.cnc[0].tip_angle_deg = Some([30.0, 400.0]);
        assert_eq!(codes(&w), [MatCode::PointRange]);
    }

    #[test]
    fn verified_needs_evidence_from_the_maker_or_a_machine_and_a_source() {
        use crate::machining::model::Evidence;
        let mut w = good();
        w.cnc[0].verified = true;
        w.laser[0].verified = true;
        assert_eq!(codes(&w), [MatCode::Workpiece, MatCode::Workpiece]);
        // a public reference is not enough, nor is evidence without its source
        w.cnc[0].evidence = Some(Evidence::PublicReference);
        w.cnc[0].source_info = Some(info());
        w.laser[0].evidence = Some(Evidence::ManufacturerTest);
        assert_eq!(codes(&w), [MatCode::Workpiece, MatCode::Workpiece]);
        w.cnc[0].evidence = Some(Evidence::ManufacturerTest);
        w.laser[0].source_info = Some(info());
        assert!(check_workpiece(&w).is_empty());
        // a test on a machine names the machine
        w.cnc[0].evidence = Some(Evidence::TestedOnMachine);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.cnc[0].machine = Some("router, 800 W spindle".into());
        assert!(check_workpiece(&w).is_empty());
    }

    #[test]
    fn a_source_has_a_date_a_licence_and_an_https_address() {
        let mut w = good();
        w.laser[0].source_info = Some(info());
        assert!(check_workpiece(&w).is_empty());
        for date in [
            "2026",
            "2026-13",
            "2026-11-32",
            "26-11",
            "2026/11",
            "abcd-ef",
        ] {
            let mut i = info();
            i.date = date.into();
            w.laser[0].source_info = Some(i);
            assert_eq!(codes(&w), [MatCode::Source], "{date}");
        }
        let mut i = info();
        i.date = "2026-11-20".into();
        i.license = " ".into();
        w.laser[0].source_info = Some(i);
        assert_eq!(codes(&w), [MatCode::Source]);
        let mut i = info();
        i.url = Some("http://example.org".into());
        i.origin = "C:\\tables".into();
        w.laser[0].source_info = Some(i);
        assert_eq!(codes(&w), [MatCode::Source, MatCode::Source]);
        w.laser[0].source_info = None;
        w.source_info = Some(SourceInfo {
            date: "yesterday".into(),
            ..info()
        });
        assert_eq!(codes(&w), [MatCode::Source]);
    }

    #[test]
    fn the_air_assist_level_and_the_speed_unit_agree_with_the_plain_fields() {
        use crate::machining::model::{AirLevel, SpeedUnit};
        let mut w = good();
        w.laser[0].air_assist_level = Some(AirLevel::Max);
        assert!(check_workpiece(&w).is_empty());
        w.laser[0].air_assist_level = Some(AirLevel::Off);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.laser[0].air_assist_level = None;
        // 10 mm/s are 600 mm/min
        w.laser[0].source_unit = Some(SpeedUnit::MmS);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.laser[0].source_speed = Some(11.0);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.laser[0].source_speed = Some(10.0);
        w.laser[0].speed_mm_min = 600.0;
        assert!(check_workpiece(&w).is_empty());
        w.laser[0].source_unit = Some(SpeedUnit::MmMin);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
    }

    #[test]
    fn a_second_figure_names_its_source_and_has_a_main_figure_beside_it() {
        use crate::machining::model::Evidence;
        let mut w = good();
        let mut alt = w.laser[0].clone();
        alt.alternative = true;
        alt.power_pct = 60.0;
        alt.source = "another table".into();
        w.laser.push(alt);
        // no source and no evidence
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.laser[1].source_info = Some(info());
        w.laser[1].evidence = Some(Evidence::PublicReference);
        assert!(check_workpiece(&w).is_empty());
        // the same source twice for the same case is a repetition
        let again = w.laser[1].clone();
        w.laser.push(again);
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        w.laser.pop();
        // no main figure of its kind and mode
        w.laser[0].mode = LaserMode::Engrave;
        w.laser[0].thickness_mm = None;
        assert_eq!(codes(&w), [MatCode::Workpiece]);
        // a laser that must not touch the material has no second figure either
        let mut w = good();
        let mut alt = w.laser[0].clone();
        alt.alternative = true;
        alt.source_info = Some(info());
        alt.evidence = Some(Evidence::PublicReference);
        w.laser.push(alt);
        w.laser.remove(0);
        w.safety.laser_cut_allowed = false;
        w.safety.laser_engrave_allowed = false;
        w.safety.hazards = vec![Hazard::Reflective];
        assert!(codes(&w).contains(&MatCode::LaserRefused));
    }

    #[test]
    fn a_hazard_that_poisons_forbids_the_laser() {
        for h in [
            Hazard::Chlorine,
            Hazard::Cyanide,
            Hazard::Fluorine,
            Hazard::Chromium,
            Hazard::ToxicFumes,
        ] {
            let mut w = good();
            w.safety.hazards = vec![h];
            assert_eq!(codes(&w), [MatCode::Workpiece], "{h:?}");
            w.safety.laser_cut_allowed = false;
            w.laser.clear();
            assert_eq!(codes(&w), [MatCode::Workpiece], "{h:?} still engraves");
            w.safety.laser_engrave_allowed = false;
            assert!(check_workpiece(&w).is_empty(), "{h:?}");
        }
        // a hazard that only asks for care does not forbid it
        let mut w = good();
        w.safety.hazards = vec![Hazard::Flammable, Hazard::FineDust, Hazard::Formaldehyde];
        assert!(check_workpiece(&w).is_empty());
    }
}
