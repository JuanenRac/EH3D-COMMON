//! The database in memory, and what it can be asked.

use std::collections::HashSet;

use crate::data;
use crate::documents::{check_documents, parse_documents, Category, Consumer, Document, Relevance};
use crate::error::MatError;
use crate::interp::interpolate_raw;
use crate::model::{DkPoint, Kind, Material, MaterialFile};
use crate::summary::BungardSummary;
use crate::validate::{check_file, check_unique};

/// What a search asks for. A field left as `None` asks nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Criteria {
    /// The kind of material.
    pub kind: Option<Kind>,
    /// The manufacturer (upper and lower case are the same).
    pub manufacturer: Option<String>,
    /// The lowest glass transition in degrees Celsius (see [`Material::tg_c`]).
    pub min_tg_c: Option<f64>,
    /// The lowest Dk.
    pub min_dk: Option<f64>,
    /// The highest Dk.
    pub max_dk: Option<f64>,
    /// The lowest Df.
    pub min_df: Option<f64>,
    /// The highest Df.
    pub max_df: Option<f64>,
    /// The frequency Dk and Df are taken at, interpolated between the published points. Without
    /// it the first value listed is used.
    pub at_ghz: Option<f64>,
}

/// The value of a list of points as a search sees it.
fn value_for(list: &[DkPoint], at_ghz: Option<f64>) -> Option<f64> {
    match at_ghz {
        Some(f) => interpolate_raw(list, f).ok(),
        None => list.first().map(|p| p.value),
    }
}

/// Whether a value is between the bounds; with no bound anything goes, with a bound a missing
/// value does not.
fn within(value: Option<f64>, lowest: Option<f64>, highest: Option<f64>) -> bool {
    if lowest.is_none() && highest.is_none() {
        return true;
    }
    match value {
        None => false,
        Some(v) => lowest.is_none_or(|l| v >= l) && highest.is_none_or(|h| v <= h),
    }
}

impl Criteria {
    /// Whether the material meets every ask.
    pub fn matches(&self, m: &Material) -> bool {
        if self.kind.is_some_and(|k| k != m.kind) {
            return false;
        }
        if let Some(name) = &self.manufacturer {
            if !m.manufacturer.eq_ignore_ascii_case(name) {
                return false;
            }
        }
        if let Some(t) = self.min_tg_c {
            if !m.tg_c().is_some_and(|x| x >= t) {
                return false;
            }
        }
        within(value_for(&m.dk, self.at_ghz), self.min_dk, self.max_dk)
            && within(value_for(&m.df, self.at_ghz), self.min_df, self.max_df)
    }
}

/// Every material of the database, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Database {
    materials: Vec<Material>,
    documents: Vec<Document>,
}

/// Reads one data file.
pub fn parse_file(file_name: &str, json: &str) -> Result<MaterialFile, MatError> {
    serde_json::from_str(json).map_err(|e| MatError::Parse {
        file: file_name.to_owned(),
        message: e.to_string(),
    })
}

impl Database {
    /// The database of the data files embedded in the crate.
    pub fn embedded() -> Result<Database, MatError> {
        Database::load(data::FILES, Some(data::DOCUMENTS))
    }

    /// A database from `(file name, JSON text)` pairs. Every file is read and checked; the first
    /// error is returned.
    pub fn from_sources(files: &[(&str, &str)]) -> Result<Database, MatError> {
        Database::load(files, None)
    }

    /// A database from material files and, when given, the catalogue of documents (also a
    /// `(file name, JSON text)` pair). The materials a document names have to exist.
    pub fn load(
        files: &[(&str, &str)],
        documents: Option<(&str, &str)>,
    ) -> Result<Database, MatError> {
        let mut materials = Vec::new();
        for (name, json) in files {
            let file = parse_file(name, json)?;
            if let Some(e) = check_file(name, &file).into_iter().next() {
                return Err(e);
            }
            materials.extend(file.materials);
        }
        if let Some(e) = check_unique(&materials).into_iter().next() {
            return Err(e);
        }
        let mut docs = Vec::new();
        if let Some((name, json)) = documents {
            let file = parse_documents(name, json)?;
            if let Some(e) = check_documents(name, &file).into_iter().next() {
                return Err(e);
            }
            let known: HashSet<&str> = materials.iter().map(|m| m.id.as_str()).collect();
            for d in &file.documents {
                if let Some(id) = d
                    .material_ids
                    .iter()
                    .find(|id| !known.contains(id.as_str()))
                {
                    return Err(MatError::Document {
                        file: d.file.clone(),
                        reason: format!("it names the material {id}, which is not in the database"),
                    });
                }
            }
            docs = file.documents;
        }
        Ok(Database {
            materials,
            documents: docs,
        })
    }

    /// Every document of the catalogue.
    pub fn documents(&self) -> &[Document] {
        &self.documents
    }

    /// The documents of one category.
    pub fn documents_by_category(&self, category: Category) -> impl Iterator<Item = &Document> {
        self.documents
            .iter()
            .filter(move |d| d.category == category)
    }

    /// The documents of one manufacturer (upper and lower case are the same).
    pub fn documents_by_manufacturer<'a>(
        &'a self,
        manufacturer: &'a str,
    ) -> impl Iterator<Item = &'a Document> {
        self.documents.iter().filter(move |d| {
            d.manufacturer
                .as_deref()
                .is_some_and(|m| m.eq_ignore_ascii_case(manufacturer))
        })
    }

    /// The documents with this relevance for a consumer.
    pub fn documents_by_relevance(
        &self,
        consumer: Consumer,
        relevance: Relevance,
    ) -> impl Iterator<Item = &Document> {
        self.documents
            .iter()
            .filter(move |d| d.relevance_for(consumer) == relevance)
    }

    /// The documents that serve a CNC that drills and mills boards.
    pub fn documents_useful_for_cnc(&self) -> Vec<&Document> {
        self.documents_by_relevance(Consumer::Cnc, Relevance::Yes)
            .collect()
    }

    /// The documents that serve the design tool.
    pub fn documents_useful_for_phoenix(&self) -> Vec<&Document> {
        self.documents_by_relevance(Consumer::Phoenix, Relevance::Yes)
            .collect()
    }

    /// The materials defined by a document that serves the consumer: those the document names
    /// and those that cite the document as their source.
    pub fn materials_useful_for(&self, consumer: Consumer) -> Vec<&Material> {
        let mut files: HashSet<&str> = HashSet::new();
        let mut ids: HashSet<&str> = HashSet::new();
        for d in self.documents_by_relevance(consumer, Relevance::Yes) {
            files.insert(d.file.as_str());
            ids.extend(d.material_ids.iter().map(String::as_str));
        }
        self.materials
            .iter()
            .filter(|m| {
                ids.contains(m.id.as_str())
                    || m.all_sources().any(|s| files.contains(s.pdf.as_str()))
            })
            .collect()
    }

    /// The materials useful for a CNC that drills and mills boards.
    pub fn materials_useful_for_cnc(&self) -> Vec<&Material> {
        self.materials_useful_for(Consumer::Cnc)
    }

    /// The materials useful for the design tool.
    pub fn materials_useful_for_phoenix(&self) -> Vec<&Material> {
        self.materials_useful_for(Consumer::Phoenix)
    }

    /// Every material, in the order of the files.
    pub fn materials(&self) -> &[Material] {
        &self.materials
    }

    /// How many materials there are.
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Whether there are no materials.
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    /// The material with this identifier.
    pub fn get(&self, id: &str) -> Option<&Material> {
        self.materials.iter().find(|m| m.id == id)
    }

    /// The manufacturers, in alphabetical order, each once.
    pub fn manufacturers(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .materials
            .iter()
            .map(|m| m.manufacturer.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// The materials of one manufacturer (upper and lower case are the same).
    pub fn by_manufacturer<'a>(
        &'a self,
        manufacturer: &'a str,
    ) -> impl Iterator<Item = &'a Material> {
        self.materials
            .iter()
            .filter(move |m| m.manufacturer.eq_ignore_ascii_case(manufacturer))
    }

    /// The materials of one kind.
    pub fn by_kind(&self, kind: Kind) -> impl Iterator<Item = &Material> {
        self.materials.iter().filter(move |m| m.kind == kind)
    }

    /// The materials that meet every ask of `criteria`.
    pub fn search(&self, criteria: &Criteria) -> Vec<&Material> {
        self.materials
            .iter()
            .filter(|m| criteria.matches(m))
            .collect()
    }

    /// The view of every material for the machine program.
    pub fn bungard_summaries(&self) -> Vec<BungardSummary> {
        self.materials
            .iter()
            .map(Material::bungard_summary)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MatCode;

    const SAMPLE: &str = r#"{
        "schema_version": 1,
        "manufacturer": "Acme",
        "materials": [
            {"id": "acme/fr4-hi", "manufacturer": "Acme", "name": "FR4 Hi", "kind": "laminate",
             "tg_dsc_c": 180.0,
             "dk": [{"freq_ghz": 1.0, "value": 4.4}, {"freq_ghz": 10.0, "value": 4.2}],
             "df": [{"freq_ghz": 1.0, "value": 0.016}],
             "source": {"pdf": "fr4.pdf", "pages": "1"}},
            {"id": "acme/rf-3", "manufacturer": "Acme", "name": "RF 3", "kind": "ptfe",
             "tg_tma_c": 280.0,
             "dk": [{"freq_ghz": 10.0, "value": 3.0}],
             "df": [{"freq_ghz": 10.0, "value": 0.002}],
             "source": {"pdf": "rf3.pdf"}}
        ]
    }"#;

    const OTHER: &str = r#"{"schema_version": 1, "manufacturer": "Beta", "materials": [
        {"id": "beta/core", "manufacturer": "Beta", "name": "Core", "kind": "metal_core",
         "thermal_conductivity_w_mk": 2.0, "source": {"pdf": "core.pdf"}}]}"#;

    fn db() -> Database {
        Database::from_sources(&[("acme.json", SAMPLE), ("beta.json", OTHER)]).unwrap()
    }

    #[test]
    fn the_files_are_loaded_and_found_by_id() {
        let db = db();
        assert_eq!(db.len(), 3);
        assert!(!db.is_empty());
        assert_eq!(db.get("beta/core").map(|m| m.kind), Some(Kind::MetalCore));
        assert!(db.get("beta/none").is_none());
        assert_eq!(db.manufacturers(), ["Acme", "Beta"]);
    }

    #[test]
    fn the_queries_by_manufacturer_and_kind() {
        let db = db();
        assert_eq!(db.by_manufacturer("acme").count(), 2);
        assert_eq!(db.by_kind(Kind::Ptfe).count(), 1);
        assert_eq!(db.by_kind(Kind::Prepreg).count(), 0);
    }

    #[test]
    fn the_search_by_tg_dk_and_df() {
        let db = db();
        let high_tg = Criteria {
            min_tg_c: Some(200.0),
            ..Criteria::default()
        };
        assert_eq!(db.search(&high_tg).len(), 1);
        let low_dk = Criteria {
            max_dk: Some(3.5),
            ..Criteria::default()
        };
        let found = db.search(&low_dk);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "acme/rf-3");
        // at 5 GHz the first material interpolates between 4.4 and 4.2 and the second is out of range
        let at = Criteria {
            max_dk: Some(4.4),
            at_ghz: Some(5.0),
            ..Criteria::default()
        };
        assert_eq!(db.search(&at).len(), 1);
        let low_df = Criteria {
            max_df: Some(0.005),
            ..Criteria::default()
        };
        assert_eq!(db.search(&low_df)[0].id, "acme/rf-3");
        let by_name = Criteria {
            manufacturer: Some("BETA".into()),
            kind: Some(Kind::MetalCore),
            ..Criteria::default()
        };
        assert_eq!(db.search(&by_name).len(), 1);
        assert_eq!(db.search(&Criteria::default()).len(), 3);
    }

    #[test]
    fn a_bad_file_is_refused_with_its_code() {
        let e = Database::from_sources(&[("a.json", "{not json")]).unwrap_err();
        assert_eq!(e.code(), MatCode::Parse);
        let unknown = r#"{"schema_version":1,"manufacturer":"A","materials":[],"extra":1}"#;
        let e = Database::from_sources(&[("a.json", unknown)]).unwrap_err();
        assert_eq!(e.code(), MatCode::Parse);
        let twice = Database::from_sources(&[("a.json", SAMPLE), ("b.json", SAMPLE)]).unwrap_err();
        assert_eq!(twice.code(), MatCode::DuplicateId);
        let newer = SAMPLE.replace("\"schema_version\": 1", "\"schema_version\": 2");
        let e = Database::from_sources(&[("a.json", newer.as_str())]).unwrap_err();
        assert_eq!(e.code(), MatCode::SchemaVersion);
    }

    #[test]
    fn a_field_the_schema_does_not_know_is_refused_in_a_material() {
        let bad = OTHER.replace("\"kind\"", "\"colour\": \"red\", \"kind\"");
        let e = Database::from_sources(&[("b.json", bad.as_str())]).unwrap_err();
        assert_eq!(e.code(), MatCode::Parse);
    }

    #[test]
    fn the_summaries_follow_the_materials() {
        let db = db();
        let all = db.bungard_summaries();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].dk, Some(4.4));
        assert_eq!(all[1].tg_c, Some(280.0));
    }

    #[test]
    fn the_documents_are_queried_and_name_real_materials() {
        let docs = r#"{"schema_version":1,"documents":[
            {"file":"fr4.pdf","folder":"d","manufacturer":"Acme","category":"laminate","doc_type":"datasheet",
             "relevance":{"cnc_machining":"yes","pcb_design":"yes"},"status":"auto"},
            {"file":"foam.pdf","folder":"d","manufacturer":"Foamco","category":"foam_gasket","doc_type":"datasheet",
             "relevance":{"cnc_machining":"no","pcb_design":"reference"},"status":"auto"},
            {"file":"core.pdf","folder":"d","category":"metal_core","doc_type":"datasheet",
             "relevance":{"cnc_machining":"reference","pcb_design":"yes"},"status":"checked","material_ids":["beta/core"]}]}"#;
        let db = Database::load(
            &[("acme.json", SAMPLE), ("beta.json", OTHER)],
            Some(("documents.json", docs)),
        )
        .unwrap();
        assert_eq!(db.documents().len(), 3);
        assert_eq!(db.documents_by_category(Category::FoamGasket).count(), 1);
        assert_eq!(db.documents_by_manufacturer("ACME").count(), 1);
        assert_eq!(db.documents_useful_for_cnc().len(), 1);
        assert_eq!(db.documents_useful_for_phoenix().len(), 2);
        // fr4.pdf is the source of both Acme materials? only of the first
        let cnc: Vec<&str> = db
            .materials_useful_for_cnc()
            .iter()
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(cnc, ["acme/fr4-hi"]);
        let phoenix = db.materials_useful_for_phoenix();
        assert_eq!(phoenix.len(), 2);
        let unknown = docs.replace("beta/core", "beta/none");
        let e = Database::load(
            &[("acme.json", SAMPLE), ("beta.json", OTHER)],
            Some(("documents.json", unknown.as_str())),
        )
        .unwrap_err();
        assert_eq!(e.code(), MatCode::Document);
    }

    #[test]
    fn the_embedded_database_is_sound() {
        let db = Database::embedded().unwrap();
        assert!(!db.is_empty());
        assert!(!db.documents().is_empty());
        for m in db.materials() {
            assert!(crate::validate::check_material(m).is_empty(), "{}", m.id);
        }
    }
}
