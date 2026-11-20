//! The database in memory, and what it can be asked.

use crate::data;
use crate::error::PmError;
use crate::model::{Filament, FilamentFile, Fill, Material, Resin, ResinFile, ResinKind};
use crate::validate::{check_filament_file, check_resin_file, check_unique};

/// What a search of filaments asks for. A field left as `None` asks nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilamentCriteria {
    /// The polymer.
    pub material: Option<Material>,
    /// The manufacturer (upper and lower case are the same).
    pub manufacturer: Option<String>,
    /// Whether it needs a hardened nozzle (`Some(false)` asks for the ones that do not).
    pub needs_hardened_nozzle: Option<bool>,
    /// A nozzle temperature the filament has to accept; a filament without a range does not.
    pub nozzle_c: Option<f64>,
    /// A bed temperature the filament has to accept; a filament without a range does not.
    pub bed_c: Option<f64>,
    /// A filler the filament has.
    pub fill: Option<Fill>,
    /// A diameter in mm the filament comes in.
    pub diameter_mm: Option<f64>,
    /// A printer identifier that the maker names as compatible.
    pub printer: Option<String>,
}

impl FilamentCriteria {
    /// Whether the filament meets every ask.
    pub fn matches(&self, f: &Filament) -> bool {
        if self.material.is_some_and(|m| m != f.material) {
            return false;
        }
        if let Some(name) = &self.manufacturer {
            if !f.manufacturer.eq_ignore_ascii_case(name) {
                return false;
            }
        }
        if self
            .needs_hardened_nozzle
            .is_some_and(|n| n != f.needs_hardened_nozzle())
        {
            return false;
        }
        if let Some(t) = self.nozzle_c {
            if f.accepts_nozzle_c(t) != Some(true) {
                return false;
            }
        }
        if let Some(t) = self.bed_c {
            match f.bed_range() {
                Some((lo, hi)) if (lo..=hi).contains(&t) => {}
                _ => return false,
            }
        }
        if self.fill.is_some_and(|x| !f.fill.contains(&x)) {
            return false;
        }
        if let Some(d) = self.diameter_mm {
            if !f.diameters_mm.iter().any(|x| (x - d).abs() < 1e-9) {
                return false;
            }
        }
        if let Some(p) = &self.printer {
            let listed = f
                .compatibility
                .as_ref()
                .is_some_and(|c| c.printers.iter().any(|x| x == p));
            if !listed {
                return false;
            }
        }
        true
    }
}

/// What a search of resins asks for. A field left as `None` asks nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResinCriteria {
    /// The kind.
    pub kind: Option<ResinKind>,
    /// The manufacturer (upper and lower case are the same).
    pub manufacturer: Option<String>,
    /// A printer the resin has settings for.
    pub printer: Option<String>,
    /// The wavelength in nm the resin cures at.
    pub wavelength_nm: Option<f64>,
    /// A layer height in mm the resin has an exposure for on the printer (needs `printer`); the
    /// height is answered by [`Resin::exposure_for`], so a height between two known ones counts.
    pub layer_mm: Option<f64>,
}

impl ResinCriteria {
    /// Whether the resin meets every ask.
    pub fn matches(&self, r: &Resin) -> bool {
        if self.kind.is_some_and(|k| k != r.kind) {
            return false;
        }
        if let Some(name) = &self.manufacturer {
            if !r.manufacturer.eq_ignore_ascii_case(name) {
                return false;
            }
        }
        if let Some(p) = &self.printer {
            if !r.exposures.iter().any(|e| &e.printer == p) {
                return false;
            }
            if let Some(l) = self.layer_mm {
                if r.exposure_for(p, l).is_err() {
                    return false;
                }
            }
        }
        if let Some(w) = self.wavelength_nm {
            if !r.wavelength_nm.is_some_and(|x| (x - w).abs() < 1e-9) {
                return false;
            }
        }
        true
    }
}

/// Parses a data file of filaments (the name is for the error messages).
pub fn parse_filament_file(name: &str, text: &str) -> Result<FilamentFile, PmError> {
    serde_json::from_str(text).map_err(|e| PmError::Parse {
        file: name.to_owned(),
        message: e.to_string(),
    })
}

/// Parses a data file of resins (the name is for the error messages).
pub fn parse_resin_file(name: &str, text: &str) -> Result<ResinFile, PmError> {
    serde_json::from_str(text).map_err(|e| PmError::Parse {
        file: name.to_owned(),
        message: e.to_string(),
    })
}

/// Every filament and resin, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Database {
    filaments: Vec<Filament>,
    resins: Vec<Resin>,
}

impl Database {
    /// The database embedded in the crate.
    pub fn embedded() -> Result<Database, Vec<PmError>> {
        Database::from_texts(data::FILAMENT_FILES, data::RESIN_FILES)
    }

    /// A database from the text of data files, as `(file name, JSON text)`. Every file is parsed
    /// and checked; the errors of all of them come back together.
    pub fn from_texts(
        filament_files: &[(&str, &str)],
        resin_files: &[(&str, &str)],
    ) -> Result<Database, Vec<PmError>> {
        let mut errors = Vec::new();
        let mut filaments = Vec::new();
        let mut resins = Vec::new();
        for (name, text) in filament_files {
            match parse_filament_file(name, text) {
                Ok(file) => {
                    errors.extend(check_filament_file(name, &file));
                    filaments.extend(file.filaments);
                }
                Err(e) => errors.push(e),
            }
        }
        for (name, text) in resin_files {
            match parse_resin_file(name, text) {
                Ok(file) => {
                    errors.extend(check_resin_file(name, &file));
                    resins.extend(file.resins);
                }
                Err(e) => errors.push(e),
            }
        }
        errors.extend(check_unique(filaments.iter().map(|f| f.id.as_str())));
        errors.extend(check_unique(resins.iter().map(|r| r.id.as_str())));
        if errors.is_empty() {
            Ok(Database { filaments, resins })
        } else {
            Err(errors)
        }
    }

    /// Every filament, in the order of the files.
    pub fn filaments(&self) -> &[Filament] {
        &self.filaments
    }

    /// Every resin, in the order of the files.
    pub fn resins(&self) -> &[Resin] {
        &self.resins
    }

    /// The filament that has this identifier.
    pub fn filament(&self, id: &str) -> Option<&Filament> {
        self.filaments.iter().find(|f| f.id == id)
    }

    /// The resin that has this identifier.
    pub fn resin(&self, id: &str) -> Option<&Resin> {
        self.resins.iter().find(|r| r.id == id)
    }

    /// The filaments that meet the criteria.
    pub fn search_filaments(&self, criteria: &FilamentCriteria) -> Vec<&Filament> {
        self.filaments
            .iter()
            .filter(|f| criteria.matches(f))
            .collect()
    }

    /// The resins that meet the criteria.
    pub fn search_resins(&self, criteria: &ResinCriteria) -> Vec<&Resin> {
        self.resins.iter().filter(|r| criteria.matches(r)).collect()
    }

    /// The manufacturers of filaments, sorted, without repeats.
    pub fn filament_manufacturers(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .filaments
            .iter()
            .map(|f| f.manufacturer.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// The manufacturers of resins, sorted, without repeats.
    pub fn resin_manufacturers(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .resins
            .iter()
            .map(|r| r.manufacturer.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PmCode;

    const F: &str = r#"{"schema_version":1,"manufacturer":"Acme","filaments":[
      {"id":"acme/pla","manufacturer":"Acme","name":"PLA","material":"pla","diameters_mm":[1.75],
       "nozzle_c":{"min":190,"max":230},"bed_c":{"min":0,"max":60},"source":"s","verified":false,
       "compatibility":{"printers":["bambu-lab-a1"]}},
      {"id":"acme/pa-cf","manufacturer":"Acme","name":"PA CF","material":"pa","fill":["carbon_fiber"],
       "nozzle_c":{"min":260,"max":300},"source":"s","verified":false}]}"#;
    const R: &str = r#"{"schema_version":1,"manufacturer":"Acme","resins":[
      {"id":"acme/tough","manufacturer":"Acme","name":"Tough","kind":"abs_like","wavelength_nm":405,
       "source":"s","verified":false,"exposures":[
        {"printer":"p","layer_mm":0.05,"normal_s":2.5,"verified":false},
        {"printer":"p","layer_mm":0.1,"normal_s":4.0,"verified":false}]}]}"#;

    fn db() -> Database {
        Database::from_texts(&[("f.json", F)], &[("r.json", R)]).unwrap()
    }

    #[test]
    fn the_lookups_find_by_identifier() {
        let db = db();
        assert_eq!(db.filaments().len(), 2);
        assert!(db.filament("acme/pla").is_some());
        assert!(db.filament("acme/none").is_none());
        assert!(db.resin("acme/tough").is_some());
        assert_eq!(db.filament_manufacturers(), ["Acme"]);
        assert_eq!(db.resin_manufacturers(), ["Acme"]);
    }

    #[test]
    fn the_search_of_filaments_by_each_ask() {
        let db = db();
        let ask = |c: FilamentCriteria| -> Vec<&str> {
            db.search_filaments(&c)
                .iter()
                .map(|f| f.id.as_str())
                .collect()
        };
        assert_eq!(ask(FilamentCriteria::default()).len(), 2);
        assert_eq!(
            ask(FilamentCriteria {
                material: Some(Material::Pa),
                ..Default::default()
            }),
            ["acme/pa-cf"]
        );
        assert_eq!(
            ask(FilamentCriteria {
                needs_hardened_nozzle: Some(true),
                ..Default::default()
            }),
            ["acme/pa-cf"]
        );
        assert_eq!(
            ask(FilamentCriteria {
                needs_hardened_nozzle: Some(false),
                ..Default::default()
            }),
            ["acme/pla"]
        );
        assert_eq!(
            ask(FilamentCriteria {
                nozzle_c: Some(210.0),
                ..Default::default()
            }),
            ["acme/pla"]
        );
        assert_eq!(
            ask(FilamentCriteria {
                nozzle_c: Some(280.0),
                ..Default::default()
            }),
            ["acme/pa-cf"]
        );
        // a filament without a bed range does not pass a bed ask
        assert_eq!(
            ask(FilamentCriteria {
                bed_c: Some(50.0),
                ..Default::default()
            }),
            ["acme/pla"]
        );
        assert_eq!(
            ask(FilamentCriteria {
                fill: Some(Fill::CarbonFiber),
                ..Default::default()
            }),
            ["acme/pa-cf"]
        );
        assert_eq!(
            ask(FilamentCriteria {
                diameter_mm: Some(1.75),
                ..Default::default()
            }),
            ["acme/pla"]
        );
        assert_eq!(
            ask(FilamentCriteria {
                printer: Some("bambu-lab-a1".into()),
                ..Default::default()
            }),
            ["acme/pla"]
        );
        assert!(ask(FilamentCriteria {
            manufacturer: Some("other".into()),
            ..Default::default()
        })
        .is_empty());
        assert_eq!(
            ask(FilamentCriteria {
                manufacturer: Some("ACME".into()),
                ..Default::default()
            })
            .len(),
            2
        );
    }

    #[test]
    fn the_search_of_resins_counts_a_layer_between_two_known_ones() {
        let db = db();
        let ask = |c: ResinCriteria| db.search_resins(&c).len();
        assert_eq!(ask(ResinCriteria::default()), 1);
        assert_eq!(
            ask(ResinCriteria {
                kind: Some(ResinKind::AbsLike),
                ..Default::default()
            }),
            1
        );
        assert_eq!(
            ask(ResinCriteria {
                kind: Some(ResinKind::Dental),
                ..Default::default()
            }),
            0
        );
        assert_eq!(
            ask(ResinCriteria {
                wavelength_nm: Some(405.0),
                ..Default::default()
            }),
            1
        );
        assert_eq!(
            ask(ResinCriteria {
                wavelength_nm: Some(385.0),
                ..Default::default()
            }),
            0
        );
        let on = |l: f64| ResinCriteria {
            printer: Some("p".into()),
            layer_mm: Some(l),
            ..Default::default()
        };
        assert_eq!(ask(on(0.075)), 1);
        assert_eq!(ask(on(0.2)), 0);
        assert_eq!(
            ask(ResinCriteria {
                printer: Some("q".into()),
                ..Default::default()
            }),
            0
        );
    }

    #[test]
    fn the_errors_of_all_the_files_come_back_together() {
        let errors = Database::from_texts(
            &[("bad.json", "{"), ("f.json", F), ("again.json", F)],
            &[("r.json", R)],
        )
        .unwrap_err();
        let codes: Vec<PmCode> = errors.iter().map(PmError::code).collect();
        assert_eq!(codes[0], PmCode::Parse);
        assert_eq!(
            codes.iter().filter(|c| **c == PmCode::DuplicateId).count(),
            2
        );
    }

    #[test]
    fn a_manufacturer_that_differs_from_its_file_is_refused() {
        let text = F.replace(
            r#""manufacturer":"Acme","filaments""#,
            r#""manufacturer":"Other","filaments""#,
        );
        let errors = Database::from_texts(&[("f.json", &text)], &[]).unwrap_err();
        assert!(errors.iter().all(|e| e.code() == PmCode::IdMismatch));
        assert_eq!(errors.len(), 2);
    }
}
