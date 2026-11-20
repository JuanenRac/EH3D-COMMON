//! What a slicer asks of a record: the ranges, the weight of a metre, the drying, the nozzle it
//! needs and, for a resin, the exposure for a printer and a layer height.
//!
//! The price of a material is never in the data (it is the user's); the cost functions take it as
//! an argument. Nothing is extrapolated: where the data does not reach, the answer is `None` or a
//! warning.

use crate::error::PmError;
use crate::i18n::{ExposureNote, ExposureWarning};
use crate::model::{Drying, ExposurePoint, Filament, Moisture, Resin};

/// The weight in grams of one metre of filament: `density * pi * (d / 2)^2`, with the density
/// in g/cm3 and the diameter in mm (a square millimetre of a metre is one cubic centimetre).
pub fn grams_per_meter(density_g_cm3: f64, diameter_mm: f64) -> f64 {
    let r = diameter_mm / 2.0;
    density_g_cm3 * std::f64::consts::PI * r * r
}

/// The length in metres of `grams` of filament.
pub fn meters_for_grams(grams: f64, density_g_cm3: f64, diameter_mm: f64) -> f64 {
    grams / grams_per_meter(density_g_cm3, diameter_mm)
}

/// The weight in grams of a volume of material.
pub fn grams_for_volume(volume_cm3: f64, density_g_cm3: f64) -> f64 {
    volume_cm3 * density_g_cm3
}

/// The cost of a gram given the price paid for a spool and the grams it holds. The price is the
/// user's: it is never in the data.
pub fn cost_per_gram(spool_price: f64, spool_grams: f64) -> Option<f64> {
    (spool_grams > 0.0 && spool_price.is_finite()).then(|| spool_price / spool_grams)
}

/// The cost of a metre of filament given the price of a spool and the grams it holds.
pub fn cost_per_meter(
    spool_price: f64,
    spool_grams: f64,
    density_g_cm3: f64,
    diameter_mm: f64,
) -> Option<f64> {
    cost_per_gram(spool_price, spool_grams).map(|g| g * grams_per_meter(density_g_cm3, diameter_mm))
}

/// How to dry a filament, with whether it is worth it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DryingPlan {
    /// The temperature in degrees Celsius.
    pub temp_c: f64,
    /// The time in hours.
    pub hours: f64,
    /// How much the filament takes water, when the record says.
    pub moisture: Option<Moisture>,
}

fn pair(min: Option<f64>, max: Option<f64>) -> Option<(f64, f64)> {
    match (min, max) {
        (Some(a), Some(b)) if a <= b => Some((a, b)),
        _ => None,
    }
}

impl Filament {
    /// The lowest and highest temperature of the nozzle, when the record gives both.
    pub fn nozzle_range(&self) -> Option<(f64, f64)> {
        let r = self.nozzle_c?;
        pair(r.min, r.max)
    }

    /// The lowest and highest temperature of the bed, when the record gives both.
    pub fn bed_range(&self) -> Option<(f64, f64)> {
        let r = self.bed_c?;
        pair(r.min, r.max)
    }

    /// The highest volumetric flow in mm3/s, when the record gives it.
    pub fn mm3s_limit(&self) -> Option<f64> {
        self.max_flow_mm3_s
    }

    /// The diameter the calculations use: the one asked for, else 1.75 if the filament comes in
    /// it, else the first one listed.
    pub fn diameter_or_default(&self, asked: Option<f64>) -> Option<f64> {
        asked.or_else(|| {
            if self.diameters_mm.iter().any(|d| (d - 1.75).abs() < 1e-9) {
                Some(1.75)
            } else {
                self.diameters_mm.first().copied()
            }
        })
    }

    /// The weight of a metre in grams (the diameter as for [`Filament::diameter_or_default`]);
    /// `None` when the record has no density or no diameter is known.
    pub fn grams_per_meter(&self, diameter_mm: Option<f64>) -> Option<f64> {
        let d = self.diameter_or_default(diameter_mm)?;
        Some(grams_per_meter(self.density_g_cm3?, d))
    }

    /// How to dry it, when the record says.
    pub fn drying_plan(&self) -> Option<DryingPlan> {
        let Drying { temp_c, hours } = self.drying?;
        Some(DryingPlan {
            temp_c,
            hours,
            moisture: self.moisture,
        })
    }

    /// Whether it needs a hardened nozzle: what the record says, and when it says nothing, whether
    /// it is filled with something that wears brass.
    pub fn needs_hardened_nozzle(&self) -> bool {
        self.abrasive
            .unwrap_or_else(|| self.fill.iter().any(|f| f.abrasive()))
    }

    /// The part fan in percent at a layer (1 is the first one): the span that holds there, else
    /// the recommended value of the fan.
    pub fn fan_at_layer(&self, layer: u32) -> Option<f64> {
        self.fan_spans
            .iter()
            .rev()
            .find(|s| s.from_layer <= layer)
            .map(|s| s.pct)
            .or_else(|| self.fan_pct.and_then(|r| r.recommended))
    }

    /// Whether the nozzle temperature is inside the range of the filament; `None` when the
    /// record gives no range.
    pub fn accepts_nozzle_c(&self, temp_c: f64) -> Option<bool> {
        self.nozzle_range()
            .map(|(lo, hi)| (lo..=hi).contains(&temp_c))
    }
}

/// The settings of a resin for a printer and a layer height.
#[derive(Debug, Clone, PartialEq)]
pub struct Exposure {
    /// The printer the settings are for.
    pub printer: String,
    /// The layer height in mm.
    pub layer_mm: f64,
    /// The exposure of a normal layer in seconds.
    pub normal_s: f64,
    /// The exposure of the base layers in seconds.
    pub base_s: Option<f64>,
    /// The number of base layers.
    pub base_layers: Option<u32>,
    /// The speed of the lift in mm/min.
    pub lift_speed_mm_min: Option<f64>,
    /// The speed of the retraction in mm/min.
    pub retract_speed_mm_min: Option<f64>,
    /// The distance of the lift in mm.
    pub lift_mm: Option<f64>,
    /// The second stretch of the lift in mm.
    pub lift2_mm: Option<f64>,
    /// The wait before the exposure in seconds.
    pub wait_before_s: Option<f64>,
    /// The wait after the exposure in seconds.
    pub wait_after_s: Option<f64>,
    /// Whether the values come from a point between two known ones.
    pub interpolated: bool,
    /// Whether every point they come from was tried on a machine.
    pub verified: bool,
}

impl Exposure {
    /// What a person should know about the values.
    pub fn notes(&self) -> Vec<ExposureNote> {
        let mut notes = Vec::new();
        if self.interpolated {
            notes.push(ExposureNote::Interpolated);
        }
        if !self.verified {
            notes.push(ExposureNote::NotVerified);
        }
        notes
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp_opt(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    Some(lerp(a?, b?, t))
}

fn exact(p: &ExposurePoint) -> Exposure {
    Exposure {
        printer: p.printer.clone(),
        layer_mm: p.layer_mm,
        normal_s: p.normal_s,
        base_s: p.base_s,
        base_layers: p.base_layers,
        lift_speed_mm_min: p.lift_speed_mm_min,
        retract_speed_mm_min: p.retract_speed_mm_min,
        lift_mm: p.lift_mm,
        lift2_mm: p.lift2_mm,
        wait_before_s: p.wait_before_s,
        wait_after_s: p.wait_after_s,
        interpolated: false,
        verified: p.verified,
    }
}

fn between(a: &ExposurePoint, b: &ExposurePoint, layer_mm: f64) -> Exposure {
    let t = (layer_mm - a.layer_mm) / (b.layer_mm - a.layer_mm);
    Exposure {
        printer: a.printer.clone(),
        layer_mm,
        normal_s: lerp(a.normal_s, b.normal_s, t),
        base_s: lerp_opt(a.base_s, b.base_s, t),
        base_layers: match (a.base_layers, b.base_layers) {
            (Some(x), Some(y)) => Some(lerp(f64::from(x), f64::from(y), t).round() as u32),
            _ => None,
        },
        lift_speed_mm_min: lerp_opt(a.lift_speed_mm_min, b.lift_speed_mm_min, t),
        retract_speed_mm_min: lerp_opt(a.retract_speed_mm_min, b.retract_speed_mm_min, t),
        lift_mm: lerp_opt(a.lift_mm, b.lift_mm, t),
        lift2_mm: lerp_opt(a.lift2_mm, b.lift2_mm, t),
        wait_before_s: lerp_opt(a.wait_before_s, b.wait_before_s, t),
        wait_after_s: lerp_opt(a.wait_after_s, b.wait_after_s, t),
        interpolated: true,
        verified: a.verified && b.verified,
    }
}

impl Resin {
    /// The printers the resin has settings for, in order of appearance, without repeats.
    pub fn printers(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for p in &self.exposures {
            if !out.contains(&p.printer.as_str()) {
                out.push(&p.printer);
            }
        }
        out
    }

    /// The settings for a printer and a layer height in mm: the point itself when there is one,
    /// else a linear interpolation between the two nearest known layer heights of that printer.
    /// Outside the known heights the answer is a warning, never an extrapolation.
    pub fn exposure_for(&self, printer: &str, layer_mm: f64) -> Result<Exposure, ExposureWarning> {
        if self.exposures.is_empty() {
            return Err(ExposureWarning::NoPoints);
        }
        let mut points: Vec<&ExposurePoint> = self
            .exposures
            .iter()
            .filter(|p| p.printer == printer)
            .collect();
        if points.is_empty() {
            return Err(ExposureWarning::NoPrinter {
                printer: printer.to_owned(),
            });
        }
        points.sort_by(|a, b| a.layer_mm.total_cmp(&b.layer_mm));
        let lowest = points[0].layer_mm;
        let highest = points[points.len() - 1].layer_mm;
        let tol = 1e-9;
        if layer_mm.is_nan() || layer_mm < lowest - tol || layer_mm > highest + tol {
            return Err(ExposureWarning::OutOfRange {
                layer_mm,
                lowest_mm: lowest,
                highest_mm: highest,
            });
        }
        if let Some(p) = points.iter().find(|p| (p.layer_mm - layer_mm).abs() <= tol) {
            return Ok(exact(p));
        }
        let upper = points
            .iter()
            .position(|p| p.layer_mm > layer_mm)
            .unwrap_or(points.len() - 1);
        Ok(between(points[upper - 1], points[upper], layer_mm))
    }

    /// Like [`Resin::exposure_for`] with the warning as a typed error (code `EH3D-PMAT-008`),
    /// for the code that reports by codes.
    pub fn exposure_checked(&self, printer: &str, layer_mm: f64) -> Result<Exposure, PmError> {
        self.exposure_for(printer, layer_mm)
            .map_err(|w| PmError::Exposure {
                id: self.id.clone(),
                reason: w.text(crate::i18n::Lang::En),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FanSpan, Fill, Range};

    fn filament(json: &str) -> Filament {
        serde_json::from_str(&format!(
            r#"{{"id":"a/b","manufacturer":"A","name":"B","material":"pla","source":"s","verified":false{}}}"#,
            if json.is_empty() { String::new() } else { format!(",{json}") }
        ))
        .unwrap()
    }

    fn point(layer: f64, normal: f64, lift: Option<f64>, verified: bool) -> ExposurePoint {
        ExposurePoint {
            printer: "p".into(),
            layer_mm: layer,
            normal_s: normal,
            normal_s_min: None,
            normal_s_max: None,
            base_s: Some(normal * 10.0),
            base_s_min: None,
            base_s_max: None,
            base_layers: Some(4),
            lift_speed_mm_min: lift,
            retract_speed_mm_min: None,
            lift_mm: Some(6.0),
            lift2_mm: None,
            wait_before_s: None,
            wait_after_s: Some(0.5),
            verified,
        }
    }

    fn resin(points: Vec<ExposurePoint>) -> Resin {
        let mut r: Resin = serde_json::from_str(
            r#"{"id":"a/r","manufacturer":"A","name":"R","kind":"standard","source":"s","verified":false}"#,
        )
        .unwrap();
        r.exposures = points;
        r
    }

    #[test]
    fn a_metre_of_pla_weighs_about_three_grams() {
        let g = grams_per_meter(1.24, 1.75);
        assert!((g - 2.9824).abs() < 1e-3, "{g}");
        assert!((meters_for_grams(1000.0, 1.24, 1.75) - 335.3).abs() < 0.5);
        assert!((grams_for_volume(10.0, 1.24) - 12.4).abs() < 1e-9);
        let f = filament(r#""density_g_cm3":1.24,"diameters_mm":[2.85,1.75]"#);
        assert_eq!(f.diameter_or_default(None), Some(1.75));
        assert!((f.grams_per_meter(None).unwrap() - g).abs() < 1e-12);
        assert!(f.grams_per_meter(Some(2.85)).unwrap() > 7.0);
        assert_eq!(filament("").grams_per_meter(None), None);
        assert_eq!(
            filament(r#""density_g_cm3":1.2"#).grams_per_meter(None),
            None
        );
    }

    #[test]
    fn the_cost_comes_from_the_price_the_user_gives() {
        assert_eq!(cost_per_gram(20.0, 1000.0), Some(0.02));
        assert_eq!(cost_per_gram(20.0, 0.0), None);
        assert_eq!(cost_per_gram(f64::NAN, 1000.0), None);
        let per_m = cost_per_meter(20.0, 1000.0, 1.24, 1.75).unwrap();
        assert!((per_m - 0.02 * grams_per_meter(1.24, 1.75)).abs() < 1e-12);
    }

    #[test]
    fn the_ranges_need_both_ends() {
        let f = filament(
            r#""nozzle_c":{"min":190,"max":230},"bed_c":{"min":50},"max_flow_mm3_s":15.5"#,
        );
        assert_eq!(f.nozzle_range(), Some((190.0, 230.0)));
        assert_eq!(f.bed_range(), None);
        assert_eq!(f.mm3s_limit(), Some(15.5));
        assert_eq!(f.accepts_nozzle_c(200.0), Some(true));
        assert_eq!(f.accepts_nozzle_c(250.0), Some(false));
        assert_eq!(filament("").accepts_nozzle_c(200.0), None);
    }

    #[test]
    fn the_drying_plan_is_what_the_record_says() {
        let f = filament(r#""drying":{"temp_c":55,"hours":8},"moisture":"high""#);
        let plan = f.drying_plan().unwrap();
        assert_eq!(
            (plan.temp_c, plan.hours, plan.moisture),
            (55.0, 8.0, Some(Moisture::High))
        );
        assert_eq!(filament("").drying_plan(), None);
    }

    #[test]
    fn a_filled_filament_needs_a_hardened_nozzle_unless_the_record_says_otherwise() {
        let mut f = filament("");
        assert!(!f.needs_hardened_nozzle());
        f.fill = vec![Fill::CarbonFiber];
        assert!(f.needs_hardened_nozzle());
        f.fill = vec![Fill::Wood];
        assert!(!f.needs_hardened_nozzle());
        f.abrasive = Some(true);
        assert!(f.needs_hardened_nozzle());
        f.fill = vec![Fill::GlassFiber];
        f.abrasive = Some(false);
        assert!(!f.needs_hardened_nozzle());
    }

    #[test]
    fn the_fan_follows_its_spans_and_falls_back_to_the_recommended_value() {
        let mut f = filament("");
        assert_eq!(f.fan_at_layer(1), None);
        f.fan_pct = Some(Range {
            min: None,
            max: None,
            recommended: Some(80.0),
        });
        assert_eq!(f.fan_at_layer(5), Some(80.0));
        f.fan_spans = vec![
            FanSpan {
                from_layer: 1,
                pct: 0.0,
            },
            FanSpan {
                from_layer: 4,
                pct: 100.0,
            },
        ];
        assert_eq!(f.fan_at_layer(1), Some(0.0));
        assert_eq!(f.fan_at_layer(3), Some(0.0));
        assert_eq!(f.fan_at_layer(4), Some(100.0));
        assert_eq!(f.fan_at_layer(400), Some(100.0));
    }

    #[test]
    fn an_exposure_is_exact_interpolated_or_a_warning() {
        let r = resin(vec![
            point(0.1, 4.0, Some(100.0), false),
            point(0.05, 2.0, Some(200.0), true),
        ]);
        let e = r.exposure_for("p", 0.05).unwrap();
        assert!(!e.interpolated && e.verified && e.normal_s == 2.0);
        let e = r.exposure_for("p", 0.075).unwrap();
        assert!(e.interpolated && !e.verified);
        assert!((e.normal_s - 3.0).abs() < 1e-9);
        assert!((e.lift_speed_mm_min.unwrap() - 150.0).abs() < 1e-9);
        assert_eq!(e.base_layers, Some(4));
        assert_eq!(e.retract_speed_mm_min, None);
        assert_eq!(
            e.notes(),
            [ExposureNote::Interpolated, ExposureNote::NotVerified]
        );
        assert_eq!(r.printers(), ["p"]);
    }

    #[test]
    fn an_exposure_is_never_extrapolated() {
        let r = resin(vec![
            point(0.05, 2.0, None, false),
            point(0.1, 4.0, None, false),
        ]);
        for layer in [0.025, 0.2, f64::NAN] {
            assert!(matches!(
                r.exposure_for("p", layer),
                Err(ExposureWarning::OutOfRange { .. })
            ));
        }
        assert!(matches!(
            r.exposure_for("other", 0.05),
            Err(ExposureWarning::NoPrinter { .. })
        ));
        assert_eq!(
            resin(vec![]).exposure_for("p", 0.05),
            Err(ExposureWarning::NoPoints)
        );
        // a value one side lacks is not invented
        let e = r.exposure_for("p", 0.07).unwrap();
        assert_eq!(e.lift_speed_mm_min, None);
        let err = r.exposure_checked("p", 0.5).unwrap_err();
        assert_eq!(err.code(), crate::error::PmCode::Exposure);
        assert!(err.to_string().starts_with("EH3D-PMAT-008"));
    }

    #[test]
    fn a_single_point_answers_only_at_its_own_layer() {
        let r = resin(vec![point(0.05, 2.0, None, false)]);
        assert!(r.exposure_for("p", 0.05).is_ok());
        assert!(r.exposure_for("p", 0.0500001).is_err());
    }
}
