//! Starting points for a CNC router and for a laser, for a diameter, a thickness or a power that is
//! asked for.
//!
//! The point itself is given when there is one, else a straight line between the two nearest
//! known ones. Outside what is known the answer is a warning, never an extrapolation. A result
//! says whether it was interpolated and whether every point it comes from was tried on a machine;
//! the seed of the crate never was.

use super::model::{CncOp, CncPoint, LaserMode, LaserPoint, LaserProcess, Tool, Workpiece};
use crate::error::MatError;
use crate::i18n::Lang;

const TOLERANCE: f64 = 1e-9;

/// What was asked for and was outside what is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quantity {
    /// The diameter of the cutter.
    Diameter,
    /// The thickness of the material.
    Thickness,
    /// The nominal power of the laser.
    Power,
}

/// Why a starting point is not given.
#[derive(Debug, Clone, PartialEq)]
pub enum StartWarning {
    /// The material has no starting points at all.
    NoPoints,
    /// The material has points, but none for this process.
    NoSuchProcess,
    /// A laser must not be used on the material.
    LaserForbidden,
    /// The request is outside the known range (the range is the one that was known).
    OutOfRange {
        /// What was asked for.
        quantity: Quantity,
        /// The value asked for.
        requested: f64,
        /// The lowest value that is known.
        lowest: f64,
        /// The highest value that is known.
        highest: f64,
    },
}

/// What a person should know about a starting point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartNote {
    /// The values come from a point between two known ones.
    Interpolated,
    /// A point they come from was not tried on a machine.
    NotVerified,
}

/// A starting point for a CNC router.
#[derive(Debug, Clone, PartialEq)]
pub struct CncStart {
    /// What the router does.
    pub op: CncOp,
    /// The cutter.
    pub tool: Tool,
    /// The diameter in mm.
    pub diameter_mm: f64,
    /// The number of flutes, when the two points it comes from agree.
    pub flutes: Option<u8>,
    /// The spindle speed in revolutions per minute.
    pub rpm: f64,
    /// The feed in mm per minute.
    pub feed_mm_min: f64,
    /// The plunge feed in mm per minute.
    pub plunge_mm_min: Option<f64>,
    /// The depth of each pass in mm.
    pub depth_per_pass_mm: f64,
    /// The step over as a percentage of the diameter.
    pub stepover_pct: Option<f64>,
    /// Whether the values come from a point between two known ones.
    pub interpolated: bool,
    /// Whether every point they come from was tried on a machine.
    pub verified: bool,
}

/// A starting point for a laser.
#[derive(Debug, Clone, PartialEq)]
pub struct LaserStart {
    /// The kind of laser and what it does.
    pub process: LaserProcess,
    /// The nominal power of the laser in watts.
    pub power_w: f64,
    /// The thickness in mm (absent for an engraving).
    pub thickness_mm: Option<f64>,
    /// The power setting in percent of the nominal power.
    pub power_pct: f64,
    /// The speed in mm per minute.
    pub speed_mm_min: f64,
    /// The number of passes.
    pub passes: u32,
    /// Whether air assist is used, when the points it comes from agree.
    pub air_assist: Option<bool>,
    /// Whether the values come from a point between two known ones.
    pub interpolated: bool,
    /// Whether every point they come from was tried on a machine.
    pub verified: bool,
}

impl CncStart {
    /// What a person should know about the values.
    pub fn notes(&self) -> Vec<StartNote> {
        notes(self.interpolated, self.verified)
    }
}

impl LaserStart {
    /// What a person should know about the values.
    pub fn notes(&self) -> Vec<StartNote> {
        notes(self.interpolated, self.verified)
    }
}

fn notes(interpolated: bool, verified: bool) -> Vec<StartNote> {
    let mut out = Vec::new();
    if interpolated {
        out.push(StartNote::Interpolated);
    }
    if !verified {
        out.push(StartNote::NotVerified);
    }
    out
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp_opt(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    Some(lerp(a?, b?, t))
}

fn same<T: PartialEq>(a: Option<T>, b: Option<T>) -> Option<T> {
    if a == b {
        a
    } else {
        None
    }
}

fn out_of_range(quantity: Quantity, requested: f64, lowest: f64, highest: f64) -> StartWarning {
    StartWarning::OutOfRange {
        quantity,
        requested,
        lowest,
        highest,
    }
}

/// The text of a warning as a typed error (the code of the error follows the warning).
fn typed(id: &str, w: &StartWarning) -> MatError {
    let reason = w.text(Lang::En);
    match w {
        StartWarning::NoPoints | StartWarning::NoSuchProcess => MatError::NoStartPoints {
            id: id.to_owned(),
            reason,
        },
        StartWarning::LaserForbidden => MatError::LaserRefused {
            id: id.to_owned(),
            reason,
        },
        StartWarning::OutOfRange { .. } => MatError::OutsideKnown {
            id: id.to_owned(),
            reason,
        },
    }
}

fn cnc_exact(p: &CncPoint) -> CncStart {
    CncStart {
        op: p.op,
        tool: p.tool,
        diameter_mm: p.diameter_mm,
        flutes: p.flutes,
        rpm: p.rpm,
        feed_mm_min: p.feed_mm_min,
        plunge_mm_min: p.plunge_mm_min,
        depth_per_pass_mm: p.depth_per_pass_mm,
        stepover_pct: p.stepover_pct,
        interpolated: false,
        verified: p.verified,
    }
}

fn cnc_between(a: &CncPoint, b: &CncPoint, diameter_mm: f64) -> CncStart {
    let t = (diameter_mm - a.diameter_mm) / (b.diameter_mm - a.diameter_mm);
    CncStart {
        op: a.op,
        tool: a.tool,
        diameter_mm,
        flutes: same(a.flutes, b.flutes),
        rpm: lerp(a.rpm, b.rpm, t),
        feed_mm_min: lerp(a.feed_mm_min, b.feed_mm_min, t),
        plunge_mm_min: lerp_opt(a.plunge_mm_min, b.plunge_mm_min, t),
        depth_per_pass_mm: lerp(a.depth_per_pass_mm, b.depth_per_pass_mm, t),
        stepover_pct: lerp_opt(a.stepover_pct, b.stepover_pct, t),
        interpolated: true,
        verified: a.verified && b.verified,
    }
}

/// Where a value falls in a sorted list of known values: the point itself, or the two around it.
enum Place {
    At(usize),
    Between(usize, usize),
}

/// Places `value` among `known` (sorted, not empty), or says what the known range is.
fn place(known: &[f64], value: f64) -> Result<Place, (f64, f64)> {
    let lowest = known[0];
    let highest = known[known.len() - 1];
    if value.is_nan() || value < lowest - TOLERANCE || value > highest + TOLERANCE {
        return Err((lowest, highest));
    }
    if let Some(i) = known.iter().position(|k| (k - value).abs() <= TOLERANCE) {
        return Ok(Place::At(i));
    }
    let upper = known
        .iter()
        .position(|k| *k > value)
        .unwrap_or(known.len() - 1);
    Ok(Place::Between(upper - 1, upper))
}

impl Workpiece {
    /// Whether a CO2 or diode laser may cut the material.
    pub fn laser_cut_allowed(&self) -> bool {
        self.safety.laser_cut_allowed && !self.safety.laser_unsafe
    }

    /// Whether a CO2 or diode laser may engrave the material.
    pub fn laser_engrave_allowed(&self) -> bool {
        self.safety.laser_engrave_allowed && !self.safety.laser_unsafe
    }

    /// Whether the material must never be put under a laser (toxic or corrosive fumes).
    pub fn laser_unsafe(&self) -> bool {
        self.safety.laser_unsafe
    }

    /// The reasons a laser is refused or needs care, in a language (empty when there are none).
    pub fn laser_reasons(&self, lang: Lang) -> Vec<&'static str> {
        self.safety.hazards.iter().map(|h| h.text(lang)).collect()
    }

    /// A starting point for a CNC router: the operation, the kind of cutter and its diameter in
    /// mm. The point itself when there is one, else a straight line between the two nearest known
    /// diameters of that operation and cutter; outside the known diameters a warning.
    pub fn cnc_start(
        &self,
        op: CncOp,
        tool: Tool,
        diameter_mm: f64,
    ) -> Result<CncStart, StartWarning> {
        if self.cnc.is_empty() {
            return Err(StartWarning::NoPoints);
        }
        let mut points: Vec<&CncPoint> = self
            .cnc
            .iter()
            .filter(|p| p.op == op && p.tool == tool)
            .collect();
        if points.is_empty() {
            return Err(StartWarning::NoSuchProcess);
        }
        points.sort_by(|a, b| a.diameter_mm.total_cmp(&b.diameter_mm));
        let known: Vec<f64> = points.iter().map(|p| p.diameter_mm).collect();
        match place(&known, diameter_mm) {
            Err((lo, hi)) => Err(out_of_range(Quantity::Diameter, diameter_mm, lo, hi)),
            Ok(Place::At(i)) => Ok(cnc_exact(points[i])),
            Ok(Place::Between(a, b)) => Ok(cnc_between(points[a], points[b], diameter_mm)),
        }
    }

    /// Like [`Workpiece::cnc_start`] with the warning as a typed error (`EH3D-MAT-012` and
    /// `EH3D-MAT-013`).
    pub fn cnc_start_checked(
        &self,
        op: CncOp,
        tool: Tool,
        diameter_mm: f64,
    ) -> Result<CncStart, MatError> {
        self.cnc_start(op, tool, diameter_mm)
            .map_err(|w| typed(&self.id, &w))
    }

    /// A starting point for a laser of a nominal power in watts, for a thickness in mm (ignored
    /// when engraving) and a process. A laser that must not be used on the material is refused.
    /// The point itself when there is one; else a straight line between the two nearest known
    /// thicknesses and, when the power is between two known powers, between the two powers;
    /// outside what is known a warning, never an extrapolation.
    pub fn laser_start(
        &self,
        power_w: f64,
        thickness_mm: f64,
        process: LaserProcess,
    ) -> Result<LaserStart, StartWarning> {
        let allowed = match process.mode {
            LaserMode::Cut => self.laser_cut_allowed(),
            LaserMode::Engrave => self.laser_engrave_allowed(),
        };
        if !allowed {
            return Err(StartWarning::LaserForbidden);
        }
        if self.laser.is_empty() {
            return Err(StartWarning::NoPoints);
        }
        let points: Vec<&LaserPoint> = self
            .laser
            .iter()
            .filter(|p| p.kind == process.kind && p.mode == process.mode)
            .collect();
        if points.is_empty() {
            return Err(StartWarning::NoSuchProcess);
        }
        let mut powers: Vec<f64> = points.iter().map(|p| p.power_w).collect();
        powers.sort_by(f64::total_cmp);
        powers.dedup_by(|a, b| (*a - *b).abs() <= TOLERANCE);
        let at_power = |w: f64| -> Vec<&LaserPoint> {
            points
                .iter()
                .copied()
                .filter(|p| (p.power_w - w).abs() <= TOLERANCE)
                .collect()
        };
        match place(&powers, power_w) {
            Err((lo, hi)) => Err(out_of_range(Quantity::Power, power_w, lo, hi)),
            Ok(Place::At(i)) => curve(&at_power(powers[i]), thickness_mm, process),
            Ok(Place::Between(a, b)) => {
                let low = at_power(powers[a]);
                let high = at_power(powers[b]);
                // both curves have to cover the thickness: the range that is known is the one they share
                if process.mode == LaserMode::Cut {
                    let (l0, h0) = thickness_span(&low);
                    let (l1, h1) = thickness_span(&high);
                    let (lo, hi) = (l0.max(l1), h0.min(h1));
                    if lo > hi + TOLERANCE
                        || thickness_mm.is_nan()
                        || thickness_mm < lo - TOLERANCE
                        || thickness_mm > hi + TOLERANCE
                    {
                        return Err(out_of_range(Quantity::Thickness, thickness_mm, lo, hi));
                    }
                }
                let first = curve(&low, thickness_mm, process)?;
                let second = curve(&high, thickness_mm, process)?;
                let t = (power_w - powers[a]) / (powers[b] - powers[a]);
                Ok(LaserStart {
                    process,
                    power_w,
                    thickness_mm: first.thickness_mm,
                    power_pct: lerp(first.power_pct, second.power_pct, t),
                    speed_mm_min: lerp(first.speed_mm_min, second.speed_mm_min, t),
                    passes: lerp(f64::from(first.passes), f64::from(second.passes), t)
                        .round()
                        .max(1.0) as u32,
                    air_assist: same(first.air_assist, second.air_assist),
                    interpolated: true,
                    verified: first.verified && second.verified,
                })
            }
        }
    }

    /// Like [`Workpiece::laser_start`] with the warning as a typed error (`EH3D-MAT-011`,
    /// `EH3D-MAT-012` and `EH3D-MAT-013`).
    pub fn laser_start_checked(
        &self,
        power_w: f64,
        thickness_mm: f64,
        process: LaserProcess,
    ) -> Result<LaserStart, MatError> {
        self.laser_start(power_w, thickness_mm, process)
            .map_err(|w| typed(&self.id, &w))
    }
}

fn thickness_span(points: &[&LaserPoint]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for t in points.iter().filter_map(|p| p.thickness_mm) {
        lo = lo.min(t);
        hi = hi.max(t);
    }
    (lo, hi)
}

fn laser_exact(p: &LaserPoint, process: LaserProcess) -> LaserStart {
    LaserStart {
        process,
        power_w: p.power_w,
        thickness_mm: p.thickness_mm,
        power_pct: p.power_pct,
        speed_mm_min: p.speed_mm_min,
        passes: p.passes,
        air_assist: p.air_assist,
        interpolated: false,
        verified: p.verified,
    }
}

/// The settings along one power: for an engraving the point itself, for a cut the point at the
/// thickness or a straight line between the two nearest.
fn curve(
    points: &[&LaserPoint],
    thickness_mm: f64,
    process: LaserProcess,
) -> Result<LaserStart, StartWarning> {
    if process.mode == LaserMode::Engrave {
        return points
            .first()
            .map(|p| laser_exact(p, process))
            .ok_or(StartWarning::NoSuchProcess);
    }
    let mut cuts: Vec<&LaserPoint> = points
        .iter()
        .copied()
        .filter(|p| p.thickness_mm.is_some())
        .collect();
    if cuts.is_empty() {
        return Err(StartWarning::NoSuchProcess);
    }
    cuts.sort_by(|a, b| {
        a.thickness_mm
            .unwrap_or(0.0)
            .total_cmp(&b.thickness_mm.unwrap_or(0.0))
    });
    let known: Vec<f64> = cuts.iter().map(|p| p.thickness_mm.unwrap_or(0.0)).collect();
    match place(&known, thickness_mm) {
        Err((lo, hi)) => Err(out_of_range(Quantity::Thickness, thickness_mm, lo, hi)),
        Ok(Place::At(i)) => Ok(laser_exact(cuts[i], process)),
        Ok(Place::Between(a, b)) => {
            let (p, q) = (cuts[a], cuts[b]);
            let t = (thickness_mm - known[a]) / (known[b] - known[a]);
            Ok(LaserStart {
                process,
                power_w: p.power_w,
                thickness_mm: Some(thickness_mm),
                power_pct: lerp(p.power_pct, q.power_pct, t),
                speed_mm_min: lerp(p.speed_mm_min, q.speed_mm_min, t),
                passes: lerp(f64::from(p.passes), f64::from(q.passes), t)
                    .round()
                    .max(1.0) as u32,
                air_assist: same(p.air_assist, q.air_assist),
                interpolated: true,
                verified: p.verified && q.verified,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machining::model::{Family, Names, Safety};

    fn names() -> Names {
        let n = |s: &str| s.to_owned();
        Names {
            en: n("X"),
            es: n("X"),
            de: n("X"),
            fr: n("X"),
            it: n("X"),
            ja: n("X"),
            zh: n("X"),
            pt: n("X"),
        }
    }

    fn cnc(d: f64, rpm: f64, feed: f64, flutes: u8) -> CncPoint {
        CncPoint {
            op: CncOp::Contour,
            tool: Tool::EndMill,
            diameter_mm: d,
            flutes: Some(flutes),
            rpm,
            feed_mm_min: feed,
            plunge_mm_min: Some(feed / 4.0),
            depth_per_pass_mm: d / 2.0,
            stepover_pct: None,
            verified: false,
            source: "test".into(),
        }
    }

    fn laser(power: f64, thickness: Option<f64>, pct: f64, speed: f64, passes: u32) -> LaserPoint {
        LaserPoint {
            kind: LaserKind::Co2,
            mode: if thickness.is_some() {
                LaserMode::Cut
            } else {
                LaserMode::Engrave
            },
            power_w: power,
            thickness_mm: thickness,
            power_pct: pct,
            speed_mm_min: speed,
            passes,
            air_assist: Some(true),
            verified: false,
            source: "test".into(),
        }
    }

    use crate::machining::model::LaserKind;

    fn piece() -> Workpiece {
        Workpiece {
            id: "wood/x".into(),
            family: Family::Wood,
            names: names(),
            density_g_cm3: Some(0.6),
            thicknesses_mm: vec![3.0],
            safety: Safety {
                laser_cut_allowed: true,
                laser_engrave_allowed: true,
                laser_unsafe: false,
                hazards: vec![],
            },
            cnc: vec![cnc(3.0, 18000.0, 1200.0, 2), cnc(6.0, 16000.0, 1800.0, 2)],
            laser: vec![
                laser(40.0, Some(3.0), 80.0, 600.0, 1),
                laser(40.0, Some(6.0), 90.0, 300.0, 2),
                laser(60.0, Some(3.0), 60.0, 900.0, 1),
                laser(60.0, Some(6.0), 70.0, 500.0, 1),
                laser(60.0, Some(9.0), 90.0, 300.0, 2),
                laser(40.0, None, 20.0, 3000.0, 1),
                laser(60.0, None, 15.0, 4000.0, 1),
            ],
            pcb_material_id: None,
            notes: None,
            source: "test".into(),
        }
    }

    #[test]
    fn the_cnc_point_itself_is_given_as_it_is() {
        let s = piece()
            .cnc_start(CncOp::Contour, Tool::EndMill, 3.0)
            .unwrap();
        assert_eq!((s.rpm, s.feed_mm_min, s.flutes), (18000.0, 1200.0, Some(2)));
        assert!(!s.interpolated && !s.verified);
        assert_eq!(s.notes(), vec![StartNote::NotVerified]);
    }

    #[test]
    fn a_cnc_diameter_between_two_points_is_a_straight_line() {
        let s = piece()
            .cnc_start(CncOp::Contour, Tool::EndMill, 4.5)
            .unwrap();
        assert!((s.rpm - 17000.0).abs() < 1e-9);
        assert!((s.feed_mm_min - 1500.0).abs() < 1e-9);
        assert!((s.depth_per_pass_mm - 2.25).abs() < 1e-9);
        assert_eq!(s.flutes, Some(2));
        assert_eq!(
            s.notes(),
            vec![StartNote::Interpolated, StartNote::NotVerified]
        );
    }

    #[test]
    fn a_cnc_request_is_never_extrapolated() {
        let p = piece();
        for d in [2.0, 8.0, f64::NAN] {
            assert!(matches!(
                p.cnc_start(CncOp::Contour, Tool::EndMill, d),
                Err(StartWarning::OutOfRange {
                    quantity: Quantity::Diameter,
                    ..
                })
            ));
        }
        assert_eq!(
            p.cnc_start(CncOp::Pocket, Tool::EndMill, 3.0),
            Err(StartWarning::NoSuchProcess)
        );
        let mut empty = piece();
        empty.cnc.clear();
        assert_eq!(
            empty.cnc_start(CncOp::Contour, Tool::EndMill, 3.0),
            Err(StartWarning::NoPoints)
        );
        let e = p
            .cnc_start_checked(CncOp::Contour, Tool::EndMill, 9.0)
            .unwrap_err();
        assert_eq!(e.code(), crate::MatCode::OutsideKnown);
        let e = empty
            .cnc_start_checked(CncOp::Contour, Tool::EndMill, 9.0)
            .unwrap_err();
        assert_eq!(e.code(), crate::MatCode::NoStartPoints);
    }

    #[test]
    fn the_laser_point_at_a_power_and_a_thickness_is_given_as_it_is() {
        let s = piece()
            .laser_start(40.0, 6.0, LaserProcess::CO2_CUT)
            .unwrap();
        assert_eq!((s.power_pct, s.speed_mm_min, s.passes), (90.0, 300.0, 2));
        assert!(!s.interpolated);
    }

    #[test]
    fn the_laser_thickness_between_two_points_is_a_straight_line() {
        let s = piece()
            .laser_start(40.0, 4.5, LaserProcess::CO2_CUT)
            .unwrap();
        assert!((s.power_pct - 85.0).abs() < 1e-9);
        assert!((s.speed_mm_min - 450.0).abs() < 1e-9);
        assert!(s.interpolated);
        assert_eq!(s.thickness_mm, Some(4.5));
    }

    #[test]
    fn a_power_between_two_powers_blends_the_two_curves_where_both_cover_the_thickness() {
        let p = piece();
        // 50 W is half way between 40 W and 60 W, and 4.5 mm is covered by both
        let s = p.laser_start(50.0, 4.5, LaserProcess::CO2_CUT).unwrap();
        let low = p.laser_start(40.0, 4.5, LaserProcess::CO2_CUT).unwrap();
        let high = p.laser_start(60.0, 4.5, LaserProcess::CO2_CUT).unwrap();
        assert!((s.speed_mm_min - (low.speed_mm_min + high.speed_mm_min) / 2.0).abs() < 1e-9);
        assert!((s.power_pct - (low.power_pct + high.power_pct) / 2.0).abs() < 1e-9);
        assert!(s.interpolated);
        // 9 mm exists only at 60 W: between the two powers it is not known
        let w = p.laser_start(50.0, 9.0, LaserProcess::CO2_CUT).unwrap_err();
        assert_eq!(
            w,
            StartWarning::OutOfRange {
                quantity: Quantity::Thickness,
                requested: 9.0,
                lowest: 3.0,
                highest: 6.0
            }
        );
        // and at 60 W alone it is known
        assert!(p.laser_start(60.0, 9.0, LaserProcess::CO2_CUT).is_ok());
    }

    #[test]
    fn a_laser_request_is_never_extrapolated() {
        let p = piece();
        assert!(matches!(
            p.laser_start(80.0, 3.0, LaserProcess::CO2_CUT),
            Err(StartWarning::OutOfRange {
                quantity: Quantity::Power,
                ..
            })
        ));
        assert!(matches!(
            p.laser_start(10.0, 3.0, LaserProcess::CO2_CUT),
            Err(StartWarning::OutOfRange {
                quantity: Quantity::Power,
                ..
            })
        ));
        assert!(matches!(
            p.laser_start(40.0, 7.0, LaserProcess::CO2_CUT),
            Err(StartWarning::OutOfRange {
                quantity: Quantity::Thickness,
                ..
            })
        ));
        assert_eq!(
            p.laser_start(5.0, 3.0, LaserProcess::DIODE_CUT),
            Err(StartWarning::NoSuchProcess)
        );
        let e = p
            .laser_start_checked(80.0, 3.0, LaserProcess::CO2_CUT)
            .unwrap_err();
        assert_eq!(e.code(), crate::MatCode::OutsideKnown);
    }

    #[test]
    fn an_engraving_ignores_the_thickness_and_still_blends_the_powers() {
        let p = piece();
        let s = p
            .laser_start(40.0, 99.0, LaserProcess::CO2_ENGRAVE)
            .unwrap();
        assert_eq!((s.power_pct, s.thickness_mm), (20.0, None));
        let m = p.laser_start(50.0, 0.0, LaserProcess::CO2_ENGRAVE).unwrap();
        assert!((m.speed_mm_min - 3500.0).abs() < 1e-9);
    }

    #[test]
    fn a_material_a_laser_must_not_touch_is_refused_before_anything_else() {
        let mut p = piece();
        p.safety = Safety {
            laser_cut_allowed: false,
            laser_engrave_allowed: false,
            laser_unsafe: true,
            hazards: vec![crate::machining::model::Hazard::Chlorine],
        };
        assert!(!p.laser_cut_allowed() && !p.laser_engrave_allowed() && p.laser_unsafe());
        assert_eq!(
            p.laser_start(40.0, 3.0, LaserProcess::CO2_CUT),
            Err(StartWarning::LaserForbidden)
        );
        let e = p
            .laser_start_checked(40.0, 3.0, LaserProcess::CO2_ENGRAVE)
            .unwrap_err();
        assert_eq!(e.code(), crate::MatCode::LaserRefused);
        assert_eq!(p.laser_reasons(Lang::En).len(), 1);
        // the flag wins over the two switches
        p.safety.laser_cut_allowed = true;
        assert!(!p.laser_cut_allowed());
    }

    #[test]
    fn every_warning_reads_in_every_language() {
        let ws = [
            StartWarning::NoPoints,
            StartWarning::NoSuchProcess,
            StartWarning::LaserForbidden,
            StartWarning::OutOfRange {
                quantity: Quantity::Power,
                requested: 5.0,
                lowest: 10.0,
                highest: 60.0,
            },
        ];
        for w in &ws {
            for lang in Lang::ALL {
                assert!(!w.text(lang).is_empty());
            }
        }
    }
}
