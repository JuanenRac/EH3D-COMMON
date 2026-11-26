//! The catalog in memory: the data files embedded in the crate, one per maker, and what it can be
//! asked.

use std::collections::HashSet;

use crate::error::{PrnCode, PrnError};
use crate::model::{Adapter, Printer, PrinterFile, SCHEMA_VERSION};
use crate::validate::check_file;

/// The embedded files of printers as `(file name, JSON text)`. The list is written out so that a
/// file is never embedded by accident; a test fails when a file of the folder is not in it.
pub const PRINTER_FILES: &[(&str, &str)] = &[
    (
        "artillery.json",
        include_str!("../data/printers/artillery.json"),
    ),
    (
        "bambu-lab.json",
        include_str!("../data/printers/bambu-lab.json"),
    ),
    (
        "creality.json",
        include_str!("../data/printers/creality.json"),
    ),
];

/// The largest data file that is read, in bytes.
pub const MAX_FILE_BYTES: usize = 1 << 20;
/// The most printers of one file.
pub const MAX_PRINTERS_PER_FILE: usize = 500;

/// Reads one data file (the name is for the error messages). The size, the schema version and the
/// number of records are checked; the rules of the records are [`check_file`].
pub fn parse_printer_file(name: &str, text: &str) -> Result<PrinterFile, PrnError> {
    if text.len() > MAX_FILE_BYTES {
        return Err(PrnError::new(
            PrnCode::TooLarge,
            name,
            "the file is larger than 1 MiB",
        ));
    }
    let file: PrinterFile = serde_json::from_str(text)
        .map_err(|e| PrnError::new(PrnCode::Parse, name, e.to_string()))?;
    if file.schema_version != SCHEMA_VERSION {
        return Err(PrnError::new(
            PrnCode::SchemaVersion,
            name,
            format!(
                "version {}, this crate reads {SCHEMA_VERSION}",
                file.schema_version
            ),
        ));
    }
    if file.printers.len() > MAX_PRINTERS_PER_FILE {
        return Err(PrnError::new(
            PrnCode::TooLarge,
            name,
            "too many printers in one file",
        ));
    }
    Ok(file)
}

/// Every printer, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Catalog {
    printers: Vec<Printer>,
}

impl Catalog {
    /// The catalog embedded in the crate.
    pub fn embedded() -> Result<Catalog, Vec<PrnError>> {
        Catalog::from_texts(PRINTER_FILES)
    }

    /// A catalog from the text of data files, as `(file name, JSON text)`. Every file is parsed
    /// and checked; the errors of all of them come back together.
    pub fn from_texts(files: &[(&str, &str)]) -> Result<Catalog, Vec<PrnError>> {
        let mut errors = Vec::new();
        let mut printers: Vec<Printer> = Vec::new();
        for (name, text) in files {
            match parse_printer_file(name, text) {
                Ok(file) => {
                    errors.extend(check_file(name, &file));
                    printers.extend(file.printers);
                }
                Err(e) => errors.push(e),
            }
        }
        let mut seen = HashSet::new();
        for p in &printers {
            if !seen.insert(p.flat_id()) {
                errors.push(PrnError::new(
                    PrnCode::DuplicateId,
                    p.id.clone(),
                    "the identifier appears more than once",
                ));
            }
        }
        if errors.is_empty() {
            Ok(Catalog { printers })
        } else {
            Err(errors)
        }
    }

    /// Every printer, in the order of the files.
    pub fn printers(&self) -> &[Printer] {
        &self.printers
    }

    /// The printer with an identifier, written as `maker/model` or `maker-model`, in any case.
    pub fn find(&self, id: &str) -> Option<&Printer> {
        let want = id.trim().to_lowercase().replace('/', "-");
        self.printers.iter().find(|p| p.flat_id() == want)
    }

    /// The printers whose identifier, maker or model contain a text, in any case.
    pub fn search(&self, query: &str) -> Vec<&Printer> {
        let q = query.trim().to_lowercase();
        self.printers
            .iter()
            .filter(|p| {
                p.id.contains(&q)
                    || p.flat_id().contains(&q)
                    || p.title().to_lowercase().contains(&q)
            })
            .collect()
    }

    /// The printers a family of adapter would talk to.
    pub fn by_adapter(&self, adapter: Adapter) -> Vec<&Printer> {
        self.printers
            .iter()
            .filter(|p| p.adapter == adapter)
            .collect()
    }
}
