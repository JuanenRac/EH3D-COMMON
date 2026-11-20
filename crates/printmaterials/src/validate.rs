//! The rules the data has to follow, checked when it is loaded and by a test over every file.
//!
//! * the schema version is known;
//! * identifiers are unique and agree with the manufacturer and the name;
//! * every value is inside the range it can have (a nozzle at 2000 degrees is a typing error);
//! * a range keeps `min <= recommended <= max`;
//! * every record names its source, a reference without a path that climbs, and links are `https`;
//! * no record says it was verified on a machine (the data of the suite never claims it).

use std::collections::HashSet;

use crate::error::PmError;
use crate::model::{
    ColorVariant, ExposurePoint, Filament, FilamentFile, Range, Resin, ResinFile, SCHEMA_VERSION,
};

/// The nozzle temperature that is believable, in degrees Celsius.
pub const NOZZLE_RANGE: (f64, f64) = (100.0, 500.0);
/// The bed temperature that is believable, in degrees Celsius.
pub const BED_RANGE: (f64, f64) = (0.0, 200.0);
/// The chamber temperature that is believable, in degrees Celsius.
pub const CHAMBER_RANGE: (f64, f64) = (0.0, 150.0);
/// The density that is believable for a filament or a liquid resin, in g/cm3.
pub const DENSITY_RANGE: (f64, f64) = (0.5, 6.0);
/// The layer height that is believable for a resin printer, in mm.
pub const RESIN_LAYER_RANGE: (f64, f64) = (0.005, 0.5);

/// The name of a thing as it is written in an identifier: lower case, every run of characters
/// that are not letters or digits becomes one hyphen, none at the ends.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut pending = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if pending && !out.is_empty() {
                out.push('-');
            }
            pending = false;
            out.extend(c.to_lowercase());
        } else {
            pending = true;
        }
    }
    out
}

/// The identifier a record has from its manufacturer and its name.
pub fn make_id(manufacturer: &str, name: &str) -> String {
    format!("{}/{}", slug(manufacturer), slug(name))
}

/// The collector of the rules a record breaks.
struct Check<'a> {
    id: &'a str,
    errors: Vec<PmError>,
}

impl<'a> Check<'a> {
    fn new(id: &'a str) -> Self {
        Check {
            id,
            errors: Vec::new(),
        }
    }

    fn bad(&mut self, field: &str, value: f64, r: (f64, f64)) {
        self.errors.push(PmError::OutOfRange {
            id: self.id.to_owned(),
            field: field.to_owned(),
            value,
            min: r.0,
            max: r.1,
        });
    }

    fn within(&mut self, field: &str, value: Option<f64>, r: (f64, f64)) {
        if let Some(v) = value {
            // not `!(a <= v <= b)` spelled with comparisons: a NaN must fail too
            if !(r.0..=r.1).contains(&v) {
                self.bad(field, v, r);
            }
        }
    }

    fn within_value(&mut self, field: &str, value: f64, r: (f64, f64)) {
        self.within(field, Some(value), r);
    }

    fn range(&mut self, field: &str, range: Option<&Range>, r: (f64, f64)) {
        let Some(range) = range else { return };
        self.within(&format!("{field}.min"), range.min, r);
        self.within(&format!("{field}.max"), range.max, r);
        self.within(&format!("{field}.recommended"), range.recommended, r);
        let parts = [range.min, range.recommended, range.max];
        let given: Vec<f64> = parts.iter().flatten().copied().collect();
        if given.windows(2).any(|w| w[0] > w[1]) {
            self.errors.push(PmError::Inconsistent {
                id: self.id.to_owned(),
                reason: format!("{field}: the order min <= recommended <= max does not hold"),
            });
        }
    }

    fn source(&mut self, source: &str, source_ref: Option<&str>, links: &[String]) {
        let mut wrong = |reason: String| {
            self.errors.push(PmError::Source {
                id: self.id.to_owned(),
                reason,
            });
        };
        if source.trim().is_empty() {
            wrong("the source is empty".to_owned());
        }
        if let Some(r) = source_ref {
            if r.trim().is_empty()
                || r.starts_with('/')
                || r.contains(['\\', ':'])
                || r.split('/').any(|p| p == "..")
            {
                wrong(format!(
                    "the reference {r} is not a plain file name or path"
                ));
            }
        }
        for link in links {
            if !link.starts_with("https://") || link.contains(char::is_whitespace) {
                wrong(format!("the link {link} is not an https address"));
            }
        }
    }

    fn identity(&mut self, manufacturer: &str, name: &str, file_manufacturer: Option<&str>) {
        if let Some(fm) = file_manufacturer {
            if fm != manufacturer {
                self.errors.push(PmError::IdMismatch {
                    id: self.id.to_owned(),
                    reason: format!("the file is of {fm} and the record of {manufacturer}"),
                });
            }
        }
        if self.id != make_id(manufacturer, name) {
            self.errors.push(PmError::IdMismatch {
                id: self.id.to_owned(),
                reason: format!(
                    "{manufacturer} and {name} give {}",
                    make_id(manufacturer, name)
                ),
            });
        }
    }

    fn colors(&mut self, colors: &[ColorVariant]) {
        for (i, c) in colors.iter().enumerate() {
            if c.name.trim().is_empty() {
                self.errors.push(PmError::Inconsistent {
                    id: self.id.to_owned(),
                    reason: format!("colors[{i}] has no name"),
                });
            }
            if let Some(h) = &c.hex {
                let ok = h.len() == 7
                    && h.starts_with('#')
                    && h[1..].chars().all(|d| d.is_ascii_hexdigit());
                if !ok {
                    self.errors.push(PmError::Inconsistent {
                        id: self.id.to_owned(),
                        reason: format!("colors[{i}].hex {h} is not #RRGGBB"),
                    });
                }
            }
            self.within(&format!("colors[{i}].td"), c.td, (0.0, 100.0));
        }
    }
}

/// Every rule broken by one filament (an empty list when it is sound).
pub fn check_filament(f: &Filament, file_manufacturer: Option<&str>) -> Vec<PmError> {
    let mut c = Check::new(&f.id);
    c.identity(&f.manufacturer, &f.name, file_manufacturer);
    c.source(&f.source, f.source_ref.as_deref(), &f.links);
    if f.verified {
        c.errors.push(PmError::Inconsistent {
            id: f.id.clone(),
            reason: "the data of the suite is never marked verified".to_owned(),
        });
    }
    c.range("nozzle_c", f.nozzle_c.as_ref(), NOZZLE_RANGE);
    c.range("bed_c", f.bed_c.as_ref(), BED_RANGE);
    c.range("fan_pct", f.fan_pct.as_ref(), (0.0, 100.0));
    c.within("chamber_c", f.chamber_c, CHAMBER_RANGE);
    c.within("density_g_cm3", f.density_g_cm3, DENSITY_RANGE);
    c.within("diameter_tolerance_mm", f.diameter_tolerance_mm, (0.0, 0.5));
    for (i, d) in f.diameters_mm.iter().enumerate() {
        c.within_value(&format!("diameters_mm[{i}]"), *d, (1.0, 4.0));
    }
    c.within("max_speed_mm_s", f.max_speed_mm_s, (1.0, 2000.0));
    c.within("max_flow_mm3_s", f.max_flow_mm3_s, (0.1, 150.0));
    c.within("min_nozzle_mm", f.min_nozzle_mm, (0.1, 2.0));
    c.within("tg_c", f.tg_c, (-100.0, 400.0));
    c.within("hdt_c", f.hdt_c, (20.0, 400.0));
    c.within("tensile_mpa", f.tensile_mpa, (1.0, 500.0));
    c.within("modulus_mpa", f.modulus_mpa, (1.0, 100_000.0));
    c.within("td", f.td, (0.0, 100.0));
    if let Some(s) = &f.shore {
        c.within_value("shore.value", s.value, (1.0, 100.0));
    }
    for (i, s) in f.fan_spans.iter().enumerate() {
        c.within_value(&format!("fan_spans[{i}].pct"), s.pct, (0.0, 100.0));
        c.within_value(
            &format!("fan_spans[{i}].from_layer"),
            f64::from(s.from_layer),
            (1.0, 1_000_000.0),
        );
    }
    if f.fan_spans
        .windows(2)
        .any(|w| w[0].from_layer >= w[1].from_layer)
    {
        c.errors.push(PmError::Inconsistent {
            id: f.id.clone(),
            reason: "fan_spans are not in order of layer".to_owned(),
        });
    }
    if let Some(r) = &f.retraction {
        c.within("retraction.distance_mm", r.distance_mm, (0.0, 30.0));
        c.within("retraction.speed_mm_s", r.speed_mm_s, (0.0, 200.0));
    }
    if let Some(d) = &f.drying {
        c.within_value("drying.temp_c", d.temp_c, (20.0, 150.0));
        c.within_value("drying.hours", d.hours, (0.1, 48.0));
    }
    for (i, s) in f.spools.iter().enumerate() {
        c.within_value(
            &format!("spools[{i}].weight_g"),
            s.weight_g,
            (10.0, 20_000.0),
        );
        c.within(
            &format!("spools[{i}].diameter_mm"),
            s.diameter_mm,
            (50.0, 500.0),
        );
        c.within(&format!("spools[{i}].width_mm"), s.width_mm, (10.0, 250.0));
        c.within(&format!("spools[{i}].hub_mm"), s.hub_mm, (10.0, 120.0));
    }
    if let Some(comp) = &f.compatibility {
        for (i, n) in comp.nozzles_mm.iter().enumerate() {
            c.within_value(&format!("compatibility.nozzles_mm[{i}]"), *n, (0.1, 2.0));
        }
    }
    c.colors(&f.colors);
    c.errors
}

fn check_exposure(c: &mut Check<'_>, i: usize, p: &ExposurePoint) {
    let at = format!("exposures[{i}]");
    if p.printer.trim().is_empty() || p.printer != slug(&p.printer) {
        c.errors.push(PmError::Inconsistent {
            id: c.id.to_owned(),
            reason: format!(
                "{at}.printer {} is not an identifier in lower case with hyphens",
                p.printer
            ),
        });
    }
    c.within_value(&format!("{at}.layer_mm"), p.layer_mm, RESIN_LAYER_RANGE);
    c.within_value(&format!("{at}.normal_s"), p.normal_s, (0.1, 300.0));
    c.within(&format!("{at}.normal_s_min"), p.normal_s_min, (0.1, 300.0));
    c.within(&format!("{at}.normal_s_max"), p.normal_s_max, (0.1, 300.0));
    c.within(&format!("{at}.base_s"), p.base_s, (0.1, 1000.0));
    c.within(&format!("{at}.base_s_min"), p.base_s_min, (0.1, 1000.0));
    c.within(&format!("{at}.base_s_max"), p.base_s_max, (0.1, 1000.0));
    c.within(
        &format!("{at}.base_layers"),
        p.base_layers.map(f64::from),
        (0.0, 50.0),
    );
    c.within(
        &format!("{at}.lift_speed_mm_min"),
        p.lift_speed_mm_min,
        (1.0, 3000.0),
    );
    c.within(
        &format!("{at}.retract_speed_mm_min"),
        p.retract_speed_mm_min,
        (1.0, 3000.0),
    );
    c.within(&format!("{at}.lift_mm"), p.lift_mm, (0.5, 40.0));
    c.within(&format!("{at}.lift2_mm"), p.lift2_mm, (0.0, 40.0));
    c.within(&format!("{at}.wait_before_s"), p.wait_before_s, (0.0, 60.0));
    c.within(&format!("{at}.wait_after_s"), p.wait_after_s, (0.0, 60.0));
    let pairs = [
        ("normal_s", p.normal_s_min, Some(p.normal_s), p.normal_s_max),
        ("base_s", p.base_s_min, p.base_s, p.base_s_max),
    ];
    for (name, lo, mid, hi) in pairs {
        let given: Vec<f64> = [lo, mid, hi].iter().flatten().copied().collect();
        if given.windows(2).any(|w| w[0] > w[1]) {
            c.errors.push(PmError::Inconsistent {
                id: c.id.to_owned(),
                reason: format!("{at}.{name}: the order min <= value <= max does not hold"),
            });
        }
    }
    if p.verified {
        c.errors.push(PmError::Inconsistent {
            id: c.id.to_owned(),
            reason: format!("{at} is marked verified: the data of the suite never is"),
        });
    }
}

/// Every rule broken by one resin (an empty list when it is sound).
pub fn check_resin(r: &Resin, file_manufacturer: Option<&str>) -> Vec<PmError> {
    let mut c = Check::new(&r.id);
    c.identity(&r.manufacturer, &r.name, file_manufacturer);
    c.source(&r.source, r.source_ref.as_deref(), &r.links);
    if r.verified {
        c.errors.push(PmError::Inconsistent {
            id: r.id.clone(),
            reason: "the data of the suite is never marked verified".to_owned(),
        });
    }
    c.within("wavelength_nm", r.wavelength_nm, (350.0, 450.0));
    c.within("viscosity_mpa_s", r.viscosity_mpa_s, (1.0, 100_000.0));
    c.within("density_g_cm3", r.density_g_cm3, DENSITY_RANGE);
    c.within("shore_d", r.shore_d, (1.0, 100.0));
    c.within("shore_a", r.shore_a, (1.0, 100.0));
    c.within("tg_c", r.tg_c, (-100.0, 400.0));
    c.within("hdt_c", r.hdt_c, (20.0, 400.0));
    c.within("shrinkage_pct", r.shrinkage_pct, (0.0, 20.0));
    c.within("tensile_mpa", r.tensile_mpa, (0.1, 500.0));
    c.within("flexural_mpa", r.flexural_mpa, (0.1, 500.0));
    c.within("impact_kj_m2", r.impact_kj_m2, (0.0, 1000.0));
    c.within("elongation_pct", r.elongation_pct, (0.0, 1000.0));
    c.within("water_absorption_pct", r.water_absorption_pct, (0.0, 20.0));
    c.range("layer_mm", r.layer_mm.as_ref(), RESIN_LAYER_RANGE);
    if let Some(s) = &r.sds {
        if s.trim().is_empty() || s.contains(['/', '\\', ':']) {
            c.errors.push(PmError::Source {
                id: r.id.clone(),
                reason: format!("the safety data sheet {s} is not a plain file name"),
            });
        }
    }
    if let Some(p) = &r.post_cure {
        c.within("post_cure.minutes", p.minutes, (0.0, 600.0));
        c.within("post_cure.temp_c", p.temp_c, (20.0, 120.0));
    }
    if let Some(w) = &r.wash {
        c.within("wash.minutes", w.minutes, (0.0, 120.0));
    }
    for (i, p) in r.exposures.iter().enumerate() {
        check_exposure(&mut c, i, p);
    }
    let mut keys: Vec<(String, u64)> = r
        .exposures
        .iter()
        .map(|p| (p.printer.clone(), (p.layer_mm * 1_000_000.0).round() as u64))
        .collect();
    keys.sort();
    let before = keys.len();
    keys.dedup();
    if keys.len() != before {
        c.errors.push(PmError::Inconsistent {
            id: r.id.clone(),
            reason: "two exposures are for the same printer and layer height".to_owned(),
        });
    }
    c.colors(&r.colors);
    c.errors
}

fn check_version(file: &str, found: u32) -> Option<PmError> {
    (found != SCHEMA_VERSION).then(|| PmError::SchemaVersion {
        file: file.to_owned(),
        found,
        supported: SCHEMA_VERSION,
    })
}

/// Every rule broken by a file of filaments.
pub fn check_filament_file(name: &str, file: &FilamentFile) -> Vec<PmError> {
    if let Some(e) = check_version(name, file.schema_version) {
        return vec![e];
    }
    file.filaments
        .iter()
        .flat_map(|f| check_filament(f, Some(&file.manufacturer)))
        .collect()
}

/// Every rule broken by a file of resins.
pub fn check_resin_file(name: &str, file: &ResinFile) -> Vec<PmError> {
    if let Some(e) = check_version(name, file.schema_version) {
        return vec![e];
    }
    file.resins
        .iter()
        .flat_map(|r| check_resin(r, Some(&file.manufacturer)))
        .collect()
}

/// The identifiers that appear twice among the ones given.
pub fn check_unique<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<PmError> {
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    for id in ids {
        if !seen.insert(id) {
            errors.push(PmError::DuplicateId { id: id.to_owned() });
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PmCode;

    fn filament() -> Filament {
        serde_json::from_str(
            r#"{"id":"acme/pla-basic","manufacturer":"Acme","name":"PLA Basic","material":"pla",
            "nozzle_c":{"min":190,"max":230,"recommended":210},"source":"maker-datasheet","verified":false}"#,
        )
        .unwrap()
    }

    fn resin() -> Resin {
        serde_json::from_str(
            r#"{"id":"acme/tough","manufacturer":"Acme","name":"Tough","kind":"abs_like",
            "source":"maker-datasheet","verified":false}"#,
        )
        .unwrap()
    }

    fn codes(errors: &[PmError]) -> Vec<PmCode> {
        errors.iter().map(PmError::code).collect()
    }

    #[test]
    fn a_sound_record_has_no_errors() {
        assert!(check_filament(&filament(), Some("Acme")).is_empty());
        assert!(check_resin(&resin(), Some("Acme")).is_empty());
    }

    #[test]
    fn the_identifier_must_agree_with_the_manufacturer_and_the_name() {
        let mut f = filament();
        f.id = "acme/other".into();
        assert_eq!(codes(&check_filament(&f, None)), [PmCode::IdMismatch]);
        let f = filament();
        assert_eq!(
            codes(&check_filament(&f, Some("Other"))),
            [PmCode::IdMismatch]
        );
        assert_eq!(slug("  PLA+ / Silk  "), "pla-silk");
    }

    #[test]
    fn a_temperature_out_of_range_or_out_of_order_is_refused() {
        let mut f = filament();
        f.nozzle_c = Some(Range {
            min: Some(190.0),
            max: Some(2000.0),
            recommended: Some(210.0),
        });
        assert_eq!(codes(&check_filament(&f, None)), [PmCode::OutOfRange]);
        f.nozzle_c = Some(Range {
            min: Some(230.0),
            max: Some(190.0),
            recommended: None,
        });
        assert_eq!(codes(&check_filament(&f, None)), [PmCode::Inconsistent]);
        f.nozzle_c = Some(Range {
            min: Some(f64::NAN),
            max: None,
            recommended: None,
        });
        assert_eq!(codes(&check_filament(&f, None)), [PmCode::OutOfRange]);
    }

    #[test]
    fn the_source_and_the_links_are_checked() {
        let mut f = filament();
        f.source = " ".into();
        f.source_ref = Some("../etc/passwd".into());
        f.links = vec!["http://insecure.example/x".into()];
        assert_eq!(check_filament(&f, None).len(), 3);
        f.source = "maker-datasheet".into();
        f.source_ref = Some("acme/pla/basic".into());
        f.links = vec!["https://acme.example/pla.pdf".into()];
        assert!(check_filament(&f, None).is_empty());
    }

    #[test]
    fn nothing_is_marked_verified() {
        let mut f = filament();
        f.verified = true;
        assert_eq!(codes(&check_filament(&f, None)), [PmCode::Inconsistent]);
        let mut r = resin();
        r.verified = true;
        assert_eq!(codes(&check_resin(&r, None)), [PmCode::Inconsistent]);
    }

    #[test]
    fn colours_and_fan_spans_are_checked() {
        let mut f = filament();
        f.colors = vec![ColorVariant {
            name: "Red".into(),
            hex: Some("#GG0000".into()),
            td: None,
        }];
        assert_eq!(codes(&check_filament(&f, None)), [PmCode::Inconsistent]);
        f.colors.clear();
        f.fan_spans = vec![
            crate::model::FanSpan {
                from_layer: 3,
                pct: 50.0,
            },
            crate::model::FanSpan {
                from_layer: 1,
                pct: 100.0,
            },
        ];
        assert_eq!(codes(&check_filament(&f, None)), [PmCode::Inconsistent]);
    }

    #[test]
    fn exposures_are_checked_and_not_repeated() {
        let mut r = resin();
        let p: ExposurePoint = serde_json::from_str(
            r#"{"printer":"generic-msla","layer_mm":0.05,"normal_s":2.5,"verified":false}"#,
        )
        .unwrap();
        r.exposures = vec![p.clone(), p.clone()];
        assert_eq!(codes(&check_resin(&r, None)), [PmCode::Inconsistent]);
        let mut bad = p;
        bad.normal_s = 9000.0;
        r.exposures = vec![bad];
        assert_eq!(codes(&check_resin(&r, None)), [PmCode::OutOfRange]);
    }

    #[test]
    fn a_wrong_schema_version_and_a_repeated_identifier_are_reported() {
        let file = FilamentFile {
            schema_version: 9,
            manufacturer: "Acme".into(),
            filaments: vec![filament()],
        };
        assert_eq!(
            codes(&check_filament_file("a.json", &file)),
            [PmCode::SchemaVersion]
        );
        assert_eq!(
            codes(&check_unique(["a/b", "a/c", "a/b"])),
            [PmCode::DuplicateId]
        );
    }

    #[test]
    fn a_field_the_schema_does_not_know_is_refused() {
        let text = r#"{"id":"acme/x","manufacturer":"Acme","name":"X","material":"pla",
            "source":"s","verified":false,"price":12}"#;
        assert!(serde_json::from_str::<Filament>(text).is_err());
    }
}
