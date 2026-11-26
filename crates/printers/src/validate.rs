//! The rules the data follows, checked when it is loaded and by the tests over every file.
//!
//! * the identifier agrees with the maker and the model, and the maker with the file;
//! * every figure is inside the range it can have (a NaN fails too);
//! * texts have no control character and nothing that would break a line-based settings file;
//! * every claim of a record names a public `https` page that says it;
//! * no record says it was verified on a machine.

use eh3d_printmaterials::validate::make_id;

use crate::error::{PrnCode, PrnError};
use crate::model::{Adapter, Enclosure, Firmware, MulticolorStatus, Printer, PrinterFile};

/// The believable length of an axis of the build volume, in mm.
pub const AXIS_RANGE: (f64, f64) = (10.0, 2000.0);
/// The believable nozzle diameter, in mm.
pub const NOZZLE_MM_RANGE: (f64, f64) = (0.1, 2.0);
/// The believable highest temperature of a nozzle.
pub const NOZZLE_C_RANGE: (f64, f64) = (150.0, 500.0);
/// The believable highest temperature of a bed.
pub const BED_C_RANGE: (f64, f64) = (30.0, 200.0);
/// The believable highest temperature of a chamber.
pub const CHAMBER_C_RANGE: (f64, f64) = (20.0, 150.0);
/// The longest name.
pub const MAX_TEXT: usize = 80;
/// The longest address.
pub const MAX_URL: usize = 300;
/// The most pages of a record.
pub const MAX_SOURCES: usize = 16;

/// The names of the fields a page may cover.
pub const FIELDS: [&str; 12] = [
    "volume_mm",
    "filament_diameter_mm",
    "nozzles_mm",
    "enclosure",
    "heated_chamber",
    "nozzle_max_c",
    "bed_max_c",
    "chamber_max_c",
    "multicolor",
    "firmware",
    "adapter",
    "send_formats",
];

struct Check<'a> {
    at: &'a str,
    errors: Vec<PrnError>,
}

impl Check<'_> {
    fn add(&mut self, code: PrnCode, reason: impl Into<String>) {
        self.errors.push(PrnError::new(code, self.at, reason));
    }

    fn within(&mut self, field: &str, value: f64, range: (f64, f64)) {
        // `contains` is false for a NaN, so it fails too
        if !(range.0..=range.1).contains(&value) {
            self.add(
                PrnCode::OutOfRange,
                format!(
                    "{field} = {value} is not between {} and {}",
                    range.0, range.1
                ),
            );
        }
    }

    fn text(&mut self, field: &str, value: &str) {
        if value.trim().is_empty() || value.trim() != value {
            self.add(PrnCode::Text, format!("{field} is empty or has blank ends"));
        } else if value.chars().count() > MAX_TEXT {
            self.add(PrnCode::Text, format!("{field} is too long"));
        } else if value
            .chars()
            .any(|c| c.is_control() || matches!(c, '|' | '#' | '=' | '\u{2028}' | '\u{2029}'))
        {
            self.add(
                PrnCode::Text,
                format!("{field} has a character that is refused"),
            );
        }
    }
}

fn url_is_sound(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    url.len() <= MAX_URL
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
        && host.contains('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// The fields of a record that make a claim, and so need a page.
pub fn claimed_fields(p: &Printer) -> Vec<&'static str> {
    let mut out = vec!["volume_mm", "nozzle_max_c", "bed_max_c"];
    let claims = [
        ("filament_diameter_mm", p.filament_diameter_mm.is_some()),
        ("nozzles_mm", !p.nozzles_mm.is_empty()),
        ("enclosure", p.enclosure != Enclosure::Unknown),
        ("heated_chamber", p.heated_chamber.is_some()),
        ("chamber_max_c", p.chamber_max_c.is_some()),
        (
            "multicolor",
            p.multicolor.status != MulticolorStatus::Unknown,
        ),
        ("firmware", p.firmware != Firmware::Unknown),
        // an adapter that is not documented is an inference and claims nothing of the maker
        ("adapter", p.adapter_documented),
        ("send_formats", !p.send_formats.is_empty()),
    ];
    out.extend(claims.iter().filter(|c| c.1).map(|c| c.0));
    out
}

/// Every rule broken by one printer (an empty list when it is sound).
pub fn check_printer(p: &Printer, file_manufacturer: Option<&str>) -> Vec<PrnError> {
    let mut c = Check {
        at: &p.id,
        errors: Vec::new(),
    };
    c.text("manufacturer", &p.manufacturer);
    c.text("model", &p.model);
    if p.id != make_id(&p.manufacturer, &p.model) {
        c.add(
            PrnCode::IdMismatch,
            format!(
                "the maker and the model give {}",
                make_id(&p.manufacturer, &p.model)
            ),
        );
    }
    if file_manufacturer.is_some_and(|m| m != p.manufacturer) {
        c.add(
            PrnCode::IdMismatch,
            "the file is of another maker than the record",
        );
    }
    for (axis, v) in ["x", "y", "z"].iter().zip(p.volume_mm) {
        c.within(&format!("volume_mm.{axis}"), v, AXIS_RANGE);
    }
    if let Some(d) = p.filament_diameter_mm {
        c.within("filament_diameter_mm", d, (1.0, 4.0));
    }
    for (i, n) in p.nozzles_mm.iter().enumerate() {
        c.within(&format!("nozzles_mm[{i}]"), *n, NOZZLE_MM_RANGE);
    }
    if p.nozzles_mm.windows(2).any(|w| w[0] >= w[1]) {
        c.add(
            PrnCode::Inconsistent,
            "nozzles_mm are not from small to large without repeats",
        );
    }
    if let Some(d) = p.nozzle_default_mm {
        if !p.nozzles_mm.iter().any(|n| (n - d).abs() < 1e-9) {
            c.add(
                PrnCode::Inconsistent,
                "nozzle_default_mm is not one of nozzles_mm",
            );
        }
    }
    c.within("nozzle_max_c", p.nozzle_max_c, NOZZLE_C_RANGE);
    c.within("bed_max_c", p.bed_max_c, BED_C_RANGE);
    if let Some(t) = p.chamber_max_c {
        c.within("chamber_max_c", t, CHAMBER_C_RANGE);
        if p.heated_chamber == Some(false) {
            c.add(
                PrnCode::Inconsistent,
                "a chamber that is not heated has no highest temperature",
            );
        }
    }
    if p.heated_chamber == Some(true) && p.enclosure == Enclosure::Open {
        c.add(PrnCode::Inconsistent, "an open frame has no heated chamber");
    }
    let m = &p.multicolor;
    if let Some(system) = &m.system {
        c.text("multicolor.system", system);
    }
    if let Some(n) = m.max_colors {
        c.within("multicolor.max_colors", f64::from(n), (2.0, 64.0));
    }
    if matches!(m.status, MulticolorStatus::None | MulticolorStatus::Unknown)
        && (m.system.is_some() || m.max_colors.is_some())
    {
        c.add(
            PrnCode::Inconsistent,
            "a multicolor system is named with the status none or unknown",
        );
    }
    if p.adapter_documented && p.adapter == Adapter::Unknown {
        c.add(
            PrnCode::Inconsistent,
            "an unknown adapter cannot be documented",
        );
    }
    let mut formats = p.send_formats.clone();
    formats.sort_by_key(|f| f.extension());
    formats.dedup();
    if formats.len() != p.send_formats.len() {
        c.add(PrnCode::Inconsistent, "send_formats repeats a format");
    }
    if p.verified {
        c.add(
            PrnCode::Inconsistent,
            "the data of the suite is never marked verified",
        );
    }
    if p.sources.is_empty() || p.sources.len() > MAX_SOURCES {
        c.add(PrnCode::Source, "a record names 1 to 16 pages");
    }
    for s in &p.sources {
        if !url_is_sound(&s.url) {
            c.add(PrnCode::Source, "a source is not a plain https address");
        }
        if s.covers.is_empty() {
            c.add(PrnCode::Source, "a source covers no field");
        }
        for f in &s.covers {
            if !FIELDS.contains(&f.as_str()) {
                c.add(PrnCode::Source, "a source covers a field that is not one");
            }
        }
    }
    for field in claimed_fields(p) {
        if !p.covers(field) {
            c.add(PrnCode::Source, format!("no source says {field}"));
        }
    }
    c.errors
}

/// Every rule broken by one file.
pub fn check_file(name: &str, file: &PrinterFile) -> Vec<PrnError> {
    let mut errors = Vec::new();
    if file.printers.is_empty() {
        errors.push(PrnError::new(
            PrnCode::Inconsistent,
            name,
            "the file has no printer",
        ));
    }
    for p in &file.printers {
        errors.extend(check_printer(p, Some(&file.manufacturer)));
    }
    errors
}
