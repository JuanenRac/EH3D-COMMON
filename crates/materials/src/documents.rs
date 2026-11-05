//! The catalogue of the reference documents: one record per file, with what the file is and
//! whether it serves the machine program (a CNC that drills and mills boards) or the design tool.
//!
//! Only metadata is kept: no local paths and no text copied from the documents.
//!
//! Categories (`category`): `laminate`, `prepreg`, `bonding_film`, `flex`, `metal_core`,
//! `ptfe_rf`, `copper_foil`, `solder_mask`, `photoresist`, `thermal_interface`, `foam_gasket`,
//! `adhesive_encapsulant`, `process_guide`, `msds`, `certificate`, `brochure`, `standard`,
//! `equipment`, `other`.
//!
//! Relevance, for each consumer: `yes` (it serves), `reference` (supporting documentation only),
//! `no` (it does not serve). Status: `auto` (classified by keywords, not read) or `checked`
//! (read by a person; only these are reliable).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::MatError;
use crate::model::SCHEMA_VERSION;

/// What the document is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// A rigid laminate.
    Laminate,
    /// A prepreg.
    Prepreg,
    /// A bonding film.
    BondingFilm,
    /// A flexible laminate or coverlay.
    Flex,
    /// A metal core laminate.
    MetalCore,
    /// A PTFE or RF material.
    PtfeRf,
    /// A copper foil.
    CopperFoil,
    /// A solder mask.
    SolderMask,
    /// A photoresist.
    Photoresist,
    /// A thermal interface material.
    ThermalInterface,
    /// A foam or a gasket.
    FoamGasket,
    /// An adhesive or an encapsulant.
    AdhesiveEncapsulant,
    /// A processing guide.
    ProcessGuide,
    /// A safety data sheet.
    Msds,
    /// A certificate.
    Certificate,
    /// A brochure.
    Brochure,
    /// A standard.
    Standard,
    /// Equipment.
    Equipment,
    /// Anything else.
    Other,
}

/// What kind of document it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    /// A datasheet.
    Datasheet,
    /// A processing guide.
    ProcessingGuide,
    /// A safety data sheet.
    Msds,
    /// A certificate.
    Certificate,
    /// A brochure.
    Brochure,
    /// A comparison table.
    Comparison,
    /// A standard.
    Standard,
    /// An image.
    Image,
    /// Not known.
    Unknown,
}

/// How much a document serves a consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relevance {
    /// It serves.
    Yes,
    /// Supporting documentation only.
    Reference,
    /// It does not serve.
    No,
}

/// Who a document may serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Consumer {
    /// The machine program of a CNC that drills and mills boards.
    Cnc,
    /// The design tool (RF simulation, impedance, stack-ups, thermal work).
    Phoenix,
}

/// Whether the classification was read by a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Classified by keywords.
    Auto,
    /// Read and confirmed.
    Checked,
}

/// The relevance of a document for each consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocRelevance {
    /// For a CNC that drills and mills boards.
    pub cnc_machining: Relevance,
    /// For the design tool.
    pub pcb_design: Relevance,
}

/// One document of the reference folders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    /// The file name, with no path.
    pub file: String,
    /// The folder of the reference set it is in (empty for the top one).
    #[serde(default)]
    pub folder: String,
    /// The number of pages.
    pub pages: Option<u32>,
    /// The manufacturer.
    pub manufacturer: Option<String>,
    /// What it is about.
    pub category: Category,
    /// What kind of document it is.
    pub doc_type: DocType,
    /// The language suffix of the file name, when it has one.
    pub language_suffix: Option<String>,
    /// Whether the file has a text layer.
    pub has_text: Option<bool>,
    /// What it serves.
    pub relevance: DocRelevance,
    /// Whether the classification was read by a person.
    pub status: Status,
    /// The materials of the database that the document defines.
    #[serde(default)]
    pub material_ids: Vec<String>,
    /// A short note.
    pub note: Option<String>,
}

/// The file of documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentFile {
    /// The schema version.
    pub schema_version: u32,
    /// The documents.
    pub documents: Vec<Document>,
}

impl Document {
    /// How much the document serves a consumer.
    pub fn relevance_for(&self, consumer: Consumer) -> Relevance {
        match consumer {
            Consumer::Cnc => self.relevance.cnc_machining,
            Consumer::Phoenix => self.relevance.pcb_design,
        }
    }
}

fn bad(file: &str, reason: impl Into<String>) -> MatError {
    MatError::Document {
        file: file.to_owned(),
        reason: reason.into(),
    }
}

fn plain(name: &str) -> bool {
    !name.contains(['/', '\\', ':'])
}

/// Reads the file of documents.
pub fn parse_documents(file_name: &str, json: &str) -> Result<DocumentFile, MatError> {
    serde_json::from_str(json).map_err(|e| MatError::Parse {
        file: file_name.to_owned(),
        message: e.to_string(),
    })
}

/// Every rule broken by the file of documents: the schema version, plain names and one record
/// per file of a folder.
pub fn check_documents(file_name: &str, file: &DocumentFile) -> Vec<MatError> {
    let mut errors = Vec::new();
    if file.schema_version != SCHEMA_VERSION {
        errors.push(MatError::SchemaVersion {
            file: file_name.to_owned(),
            found: file.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    let mut seen = HashSet::new();
    for d in &file.documents {
        if d.file.trim().is_empty() || !plain(&d.file) {
            errors.push(bad(&d.file, "the file is not a plain file name"));
        }
        if !plain(&d.folder) {
            errors.push(bad(&d.file, "the folder is not a plain name"));
        }
        if !seen.insert((d.folder.as_str(), d.file.as_str())) {
            errors.push(bad(&d.file, "two records for the same file of a folder"));
        }
        if d.pages == Some(0) {
            errors.push(bad(&d.file, "a document with zero pages"));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(file: &str, folder: &str) -> Document {
        Document {
            file: file.to_owned(),
            folder: folder.to_owned(),
            pages: Some(2),
            manufacturer: None,
            category: Category::Laminate,
            doc_type: DocType::Datasheet,
            language_suffix: None,
            has_text: Some(true),
            relevance: DocRelevance {
                cnc_machining: Relevance::Yes,
                pcb_design: Relevance::Reference,
            },
            status: Status::Auto,
            material_ids: Vec::new(),
            note: None,
        }
    }

    #[test]
    fn a_record_with_only_the_required_fields_reads() {
        let json = r#"{"schema_version":1,"documents":[{"file":"a.jpg","category":"other","doc_type":"image","relevance":{"cnc_machining":"no","pcb_design":"no"},"status":"auto"}]}"#;
        let f = parse_documents("documents.json", json).unwrap();
        assert_eq!(f.documents[0].folder, "");
        assert_eq!(f.documents[0].pages, None);
        assert!(check_documents("documents.json", &f).is_empty());
    }

    #[test]
    fn an_unknown_category_or_field_is_refused() {
        let json = r#"{"schema_version":1,"documents":[{"file":"a.pdf","category":"soup","doc_type":"image","relevance":{"cnc_machining":"no","pcb_design":"no"},"status":"auto"}]}"#;
        assert!(parse_documents("d.json", json).is_err());
        let json = r#"{"schema_version":1,"documents":[],"extra":1}"#;
        assert!(parse_documents("d.json", json).is_err());
    }

    #[test]
    fn paths_and_repeated_files_are_refused() {
        let file = DocumentFile {
            schema_version: 1,
            documents: vec![doc("a.pdf", "x"), doc("a.pdf", "x"), doc("C:\\b.pdf", "x")],
        };
        let errors = check_documents("d.json", &file);
        assert_eq!(errors.len(), 2);
        // the same name in another folder is a different file
        let file = DocumentFile {
            schema_version: 1,
            documents: vec![doc("a.pdf", "x"), doc("a.pdf", "y")],
        };
        assert!(check_documents("d.json", &file).is_empty());
    }

    #[test]
    fn the_relevance_depends_on_the_consumer() {
        let d = doc("a.pdf", "");
        assert_eq!(d.relevance_for(Consumer::Cnc), Relevance::Yes);
        assert_eq!(d.relevance_for(Consumer::Phoenix), Relevance::Reference);
    }
}
