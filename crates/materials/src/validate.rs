//! The rules the data has to follow, checked when it is loaded and by a test over every file.
//!
//! * the schema version is known;
//! * identifiers are unique and agree with the manufacturer and the name;
//! * Dk is between 1 and 100 and Df between 0 and 1, Tg and Td between 20 and 700 degrees;
//! * thicknesses, frequencies, copper and panel sizes are above zero;
//! * every material names its source, and the source is a plain file name.

use std::collections::HashSet;

use crate::error::MatError;
use crate::model::{DkPoint, Material, MaterialFile, SCHEMA_VERSION};

/// The lowest and highest Dk that is believable.
pub const DK_RANGE: (f64, f64) = (1.0, 100.0);
/// The lowest and highest Df that is believable.
pub const DF_RANGE: (f64, f64) = (0.0, 1.0);
/// The lowest and highest Tg or Td that is believable, in degrees Celsius.
pub const TEMPERATURE_RANGE: (f64, f64) = (20.0, 700.0);

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

/// The identifier a material has from its manufacturer and its name.
pub fn make_id(manufacturer: &str, name: &str) -> String {
    format!("{}/{}", slug(manufacturer), slug(name))
}

fn out_of_range(id: &str, field: &str, value: f64, range: (f64, f64)) -> MatError {
    MatError::OutOfRange {
        id: id.to_owned(),
        field: field.to_owned(),
        value,
        min: range.0,
        max: range.1,
    }
}

fn in_range(errors: &mut Vec<MatError>, id: &str, field: &str, value: Option<f64>, r: (f64, f64)) {
    if let Some(v) = value {
        if !(r.0..=r.1).contains(&v) {
            errors.push(out_of_range(id, field, v, r));
        }
    }
}

fn above_zero(errors: &mut Vec<MatError>, id: &str, field: &str, value: Option<f64>) {
    if let Some(v) = value {
        // not `v > 0.0`: a NaN must fail too
        if v.is_nan() || v <= 0.0 {
            errors.push(out_of_range(id, field, v, (f64::MIN_POSITIVE, f64::MAX)));
        }
    }
}

fn points(errors: &mut Vec<MatError>, id: &str, field: &str, list: &[DkPoint], r: (f64, f64)) {
    for (i, p) in list.iter().enumerate() {
        in_range(errors, id, &format!("{field}[{i}].value"), Some(p.value), r);
        above_zero(errors, id, &format!("{field}[{i}].freq_ghz"), p.freq_ghz);
        in_range(
            errors,
            id,
            &format!("{field}[{i}].resin_content_pct"),
            p.resin_content_pct,
            (0.0, 100.0),
        );
    }
}

fn plain_file_name(name: &str) -> bool {
    !name.trim().is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".."
}

/// Every rule broken by one material (an empty list when it is sound).
pub fn check_material(m: &Material) -> Vec<MatError> {
    let mut errors = Vec::new();
    let id = m.id.as_str();

    // identity
    let mut parts = id.split('/');
    let (left, right, extra) = (parts.next(), parts.next(), parts.next());
    match (left, right, extra) {
        (Some(l), Some(r), None) => {
            if l != slug(&m.manufacturer) {
                errors.push(MatError::IdMismatch {
                    id: id.to_owned(),
                    reason: format!(
                        "the manufacturer {} gives {}",
                        m.manufacturer,
                        slug(&m.manufacturer)
                    ),
                });
            }
            if r != slug(&m.name) {
                errors.push(MatError::IdMismatch {
                    id: id.to_owned(),
                    reason: format!("the name {} gives {}", m.name, slug(&m.name)),
                });
            }
        }
        _ => errors.push(MatError::IdMismatch {
            id: id.to_owned(),
            reason: "it has to be <manufacturer>/<name>".to_owned(),
        }),
    }

    // temperatures
    in_range(&mut errors, id, "tg_dsc_c", m.tg_dsc_c, TEMPERATURE_RANGE);
    in_range(&mut errors, id, "tg_dma_c", m.tg_dma_c, TEMPERATURE_RANGE);
    in_range(&mut errors, id, "tg_tma_c", m.tg_tma_c, TEMPERATURE_RANGE);
    in_range(&mut errors, id, "td_c", m.td_c, TEMPERATURE_RANGE);

    // electrical
    points(&mut errors, id, "dk", &m.dk, DK_RANGE);
    points(&mut errors, id, "df", &m.df, DF_RANGE);

    // physical
    above_zero(
        &mut errors,
        id,
        "thermal_conductivity_w_mk",
        m.thermal_conductivity_w_mk,
    );
    above_zero(&mut errors, id, "density_g_cm3", m.density_g_cm3);
    in_range(
        &mut errors,
        id,
        "water_absorption_pct",
        m.water_absorption_pct,
        (0.0, 100.0),
    );

    // geometry
    for (i, t) in m.thicknesses_mm.iter().enumerate() {
        above_zero(&mut errors, id, &format!("thicknesses_mm[{i}]"), Some(*t));
    }
    if let Some(r) = &m.thickness_range {
        above_zero(&mut errors, id, "thickness_range.min_mm", r.min_mm);
        above_zero(&mut errors, id, "thickness_range.max_mm", r.max_mm);
        if let (Some(lo), Some(hi)) = (r.min_mm, r.max_mm) {
            if lo > hi {
                errors.push(out_of_range(
                    id,
                    "thickness_range.min_mm",
                    lo,
                    (f64::MIN_POSITIVE, hi),
                ));
            }
        }
    }
    for (i, c) in m.copper.iter().enumerate() {
        above_zero(
            &mut errors,
            id,
            &format!("copper[{i}].weight_oz"),
            c.weight_oz,
        );
        above_zero(
            &mut errors,
            id,
            &format!("copper[{i}].thickness_um"),
            c.thickness_um,
        );
    }
    for (i, p) in m.panel_sizes.iter().enumerate() {
        above_zero(
            &mut errors,
            id,
            &format!("panel_sizes[{i}].width_mm"),
            Some(p.width_mm),
        );
        above_zero(
            &mut errors,
            id,
            &format!("panel_sizes[{i}].height_mm"),
            Some(p.height_mm),
        );
    }
    for (i, p) in m.prepreg.iter().enumerate() {
        in_range(
            &mut errors,
            id,
            &format!("prepreg[{i}].resin_content_pct"),
            p.resin_content_pct,
            (0.0, 100.0),
        );
        above_zero(
            &mut errors,
            id,
            &format!("prepreg[{i}].pressed_thickness_um"),
            p.pressed_thickness_um,
        );
    }

    // sources
    if m.source.is_none() && m.sources.is_empty() {
        errors.push(MatError::Source {
            id: id.to_owned(),
            reason: "it names no source".to_owned(),
        });
    }
    for s in m.all_sources() {
        if !plain_file_name(&s.pdf) {
            errors.push(MatError::Source {
                id: id.to_owned(),
                reason: format!("the source {:?} is not a plain file name", s.pdf),
            });
        }
    }
    errors
}

/// Every rule broken by one data file, without the rule between files (see [`check_unique`]).
pub fn check_file(file_name: &str, file: &MaterialFile) -> Vec<MatError> {
    let mut errors = Vec::new();
    if file.schema_version != SCHEMA_VERSION {
        errors.push(MatError::SchemaVersion {
            file: file_name.to_owned(),
            found: file.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    for m in &file.materials {
        if m.manufacturer != file.manufacturer {
            errors.push(MatError::IdMismatch {
                id: m.id.clone(),
                reason: format!(
                    "the material says {} but the file {} holds {}",
                    m.manufacturer, file_name, file.manufacturer
                ),
            });
        }
        errors.extend(check_material(m));
    }
    errors
}

/// The identifiers that appear more than once in a list of materials.
pub fn check_unique<'a>(materials: impl IntoIterator<Item = &'a Material>) -> Vec<MatError> {
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    for m in materials {
        if !seen.insert(m.id.as_str()) {
            errors.push(MatError::DuplicateId { id: m.id.clone() });
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn good() -> Material {
        Material::new("isola/370hr", "Isola", "370HR", Kind::Laminate, "370hr.pdf")
    }

    fn point(freq: f64, value: f64) -> DkPoint {
        DkPoint {
            freq_ghz: Some(freq),
            value,
            method: None,
            resin_content_pct: None,
            tolerance: None,
        }
    }

    #[test]
    fn the_slug_is_lower_case_with_single_hyphens() {
        assert_eq!(slug("S1000-2M"), "s1000-2m");
        assert_eq!(slug("  RO4350B "), "ro4350b");
        assert_eq!(slug("Pyralux AP / 9100"), "pyralux-ap-9100");
        assert_eq!(make_id("Shengyi", "S1000-2M"), "shengyi/s1000-2m");
    }

    #[test]
    fn a_sound_material_has_no_error() {
        assert!(check_material(&good()).is_empty());
    }

    #[test]
    fn the_identifier_has_to_agree_with_the_manufacturer_and_the_name() {
        let mut m = good();
        m.id = "isola/is410".into();
        assert!(matches!(check_material(&m)[0], MatError::IdMismatch { .. }));
        m.id = "370hr".into();
        assert!(matches!(check_material(&m)[0], MatError::IdMismatch { .. }));
        m.id = "other/370hr".into();
        assert!(matches!(check_material(&m)[0], MatError::IdMismatch { .. }));
    }

    #[test]
    fn dk_df_and_temperatures_are_held_to_their_ranges() {
        let mut m = good();
        m.dk = vec![point(1.0, 4.2), point(10.0, 0.5)];
        m.df = vec![point(1.0, 0.02), point(2.0, 1.5)];
        m.tg_dsc_c = Some(10.0);
        m.td_c = Some(800.0);
        let fields: Vec<String> = check_material(&m)
            .into_iter()
            .filter_map(|e| match e {
                MatError::OutOfRange { field, .. } => Some(field),
                _ => None,
            })
            .collect();
        assert_eq!(fields, ["tg_dsc_c", "td_c", "dk[1].value", "df[1].value"]);
    }

    #[test]
    fn a_frequency_or_a_thickness_of_zero_is_refused() {
        let mut m = good();
        m.dk = vec![point(0.0, 4.2)];
        m.thicknesses_mm = vec![0.1, 0.0];
        assert_eq!(check_material(&m).len(), 2);
    }

    #[test]
    fn the_source_is_required_and_has_no_path() {
        let mut m = good();
        m.source = None;
        assert!(matches!(check_material(&m)[0], MatError::Source { .. }));
        let mut m = good();
        if let Some(s) = m.source.as_mut() {
            s.pdf = "C:\\x\\a.pdf".to_owned();
        }
        assert!(matches!(check_material(&m)[0], MatError::Source { .. }));
        let mut m = good();
        if let Some(s) = m.source.as_mut() {
            s.pdf = "dir/a.pdf".to_owned();
        }
        assert!(!check_material(&m).is_empty());
    }

    #[test]
    fn the_file_checks_the_version_and_the_manufacturer() {
        let file = MaterialFile {
            schema_version: 2,
            manufacturer: "Rogers".into(),
            materials: vec![good()],
        };
        let codes: Vec<_> = check_file("x.json", &file)
            .iter()
            .map(MatError::code)
            .collect();
        assert_eq!(
            codes,
            [
                crate::error::MatCode::SchemaVersion,
                crate::error::MatCode::IdMismatch
            ]
        );
    }

    #[test]
    fn a_repeated_identifier_is_found() {
        let a = good();
        let b = good();
        let errors = check_unique([&a, &b]);
        assert_eq!(
            errors,
            vec![MatError::DuplicateId {
                id: "isola/370hr".into()
            }]
        );
    }
}
