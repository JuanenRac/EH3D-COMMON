//! The optional records of a project: its metadata (`common/meta/meta.json`), the index of the
//! documents of a program (`docs/<branch>/index.json`) and the report of a conversion
//! (`<branch>/origin/report.json`). All of them are optional; when one is there it is checked.

use std::collections::BTreeMap;

use eh3d_container::check_entry_name;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::spec::{is_sha256_hex, LANGUAGES};

/// The name of the metadata entry inside the `common` branch.
pub const META_FILE: &str = "meta/meta.json";
/// The name of the index of documents inside a documentation branch.
pub const DOC_INDEX_FILE: &str = "index.json";
/// The name of the conversion report inside a branch.
pub const REPORT_FILE: &str = "origin/report.json";
/// The folder inside a branch that keeps the original files of an import.
pub const ORIGIN_DIR: &str = "origin/";

/// A text in the languages of the project: the key is the language code, the value the text.
/// A text that is not empty must have an English version, which every program can fall back on.
pub type LocalText = BTreeMap<String, String>;

pub(crate) fn check_text(what: &str, text: &LocalText) -> Result<(), String> {
    for (lang, value) in text {
        if !LANGUAGES.contains(&lang.as_str()) {
            return Err(format!(
                "{what}: '{lang}' is not one of the eight languages"
            ));
        }
        if value.chars().any(|c| c.is_control() && c != '\n') {
            return Err(format!(
                "{what}: the text in '{lang}' has a control character"
            ));
        }
    }
    if !text.is_empty() && !text.contains_key("en") {
        return Err(format!("{what}: a text needs an English version"));
    }
    Ok(())
}

/// A person or an organisation that worked on the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    /// The name.
    pub name: String,
    /// How to reach them (an address or a link), if they want to say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

/// A link that goes with the project. It is data: nothing ever opens it by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// What it is.
    pub title: LocalText,
    /// The address, which starts with `https://` or `http://`.
    pub url: String,
}

/// What `common/meta/meta.json` holds. Every field is optional; the dates are written by the
/// caller (this crate never reads the clock).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// The schema number.
    #[serde(default = "one")]
    pub schema: u32,
    /// The name of the project in each language.
    #[serde(default)]
    pub title: LocalText,
    /// A description in each language.
    #[serde(default)]
    pub description: LocalText,
    /// The authors.
    #[serde(default)]
    pub authors: Vec<Author>,
    /// The licence as an SPDX identifier or expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Free labels.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The revision of the project (not the version of the file format).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// The customer or the order the project is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    /// The standards the project refers to.
    #[serde(default)]
    pub standards: Vec<String>,
    /// The materials and processes used.
    #[serde(default)]
    pub materials: Vec<String>,
    /// The links.
    #[serde(default)]
    pub links: Vec<Link>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn one() -> u32 {
    1
}

impl Metadata {
    /// Checks the metadata: the languages are the eight of the project, every text has an
    /// English version, the licence is a plausible SPDX expression and the links are web
    /// addresses.
    pub fn validate(&self) -> Result<(), String> {
        check_text("title", &self.title)?;
        check_text("description", &self.description)?;
        for author in &self.authors {
            if author.name.trim().is_empty() {
                return Err("an author has no name".into());
            }
        }
        if let Some(license) = &self.license {
            let plausible = !license.is_empty()
                && license.len() <= 128
                && license
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || " .-+()".contains(c));
            if !plausible {
                return Err("the licence is not an SPDX identifier or expression".into());
            }
        }
        for link in &self.links {
            check_text("link title", &link.title)?;
            let web = link.url.starts_with("https://") || link.url.starts_with("http://");
            if !web || link.url.chars().any(|c| c.is_control() || c == ' ') {
                return Err("a link is not a web address".into());
            }
        }
        Ok(())
    }
}

/// The kind of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    /// Markdown text (`.md`).
    Markdown,
    /// A PDF file (`.pdf`).
    Pdf,
    /// An image (`.png`, `.jpg`, `.jpeg`, `.webp`, `.svg`).
    Image,
    /// Plain text (`.txt`).
    Text,
    /// A machine-readable report (`.json`), for example a DRC or ERC result.
    Report,
}

impl DocKind {
    /// The kind a file name belongs to, from its extension.
    pub fn of_name(name: &str) -> Option<DocKind> {
        let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
        Some(match extension.as_str() {
            "md" => DocKind::Markdown,
            "pdf" => DocKind::Pdf,
            "png" | "jpg" | "jpeg" | "webp" | "svg" => DocKind::Image,
            "txt" => DocKind::Text,
            "json" => DocKind::Report,
            _ => return None,
        })
    }
}

/// One document of a program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// The name inside the documentation branch.
    pub path: String,
    /// What kind of document it is; it must agree with the extension.
    pub kind: DocKind,
    /// The title in each language.
    pub title: LocalText,
    /// The language the document is written in, if it is a text in one language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// The document this one is a translation of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_of: Option<String>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// What `docs/<branch>/index.json` holds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocIndex {
    /// The schema number.
    #[serde(default = "one")]
    pub schema: u32,
    /// The documents.
    #[serde(default)]
    pub documents: Vec<Document>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl DocIndex {
    /// Checks the index against the names of the files of its branch: every path is a safe name
    /// of a file that is there, the kind agrees with the extension, a language is one of the
    /// eight, a translation names a document of the index in another language, and a path is
    /// listed once.
    pub fn validate<'a>(&self, files: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
        let files: Vec<&str> = files.into_iter().collect();
        let mut seen: BTreeMap<&str, Option<&str>> = BTreeMap::new();
        for doc in &self.documents {
            if check_entry_name(&doc.path).is_err() {
                return Err(format!("'{}' is not a safe name", doc.path));
            }
            if !files.contains(&doc.path.as_str()) {
                return Err(format!("'{}' is listed but is not in the branch", doc.path));
            }
            if DocKind::of_name(&doc.path) != Some(doc.kind) {
                return Err(format!(
                    "'{}': the kind does not match the extension",
                    doc.path
                ));
            }
            if let Some(lang) = &doc.lang {
                if !LANGUAGES.contains(&lang.as_str()) {
                    return Err(format!(
                        "'{}': '{lang}' is not one of the eight languages",
                        doc.path
                    ));
                }
            }
            check_text(&doc.path, &doc.title)?;
            if seen.insert(&doc.path, doc.lang.as_deref()).is_some() {
                return Err(format!("'{}' is listed twice", doc.path));
            }
        }
        for doc in &self.documents {
            if let Some(original) = &doc.translation_of {
                let Some(original_lang) = seen.get(original.as_str()) else {
                    return Err(format!(
                        "'{}' is a translation of '{original}', which is not listed",
                        doc.path
                    ));
                };
                if original == &doc.path
                    || (doc.lang.is_some() && doc.lang.as_deref() == *original_lang)
                {
                    return Err(format!(
                        "'{}' is a translation into the language of its original",
                        doc.path
                    ));
                }
            }
        }
        Ok(())
    }
}

/// The file an import read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    /// The format, for example `eagle-brd` or `kicad-pcb`.
    pub format: String,
    /// The name of the file as it was.
    pub name: String,
    /// Its SHA-256.
    pub sha256: String,
    /// Its size in bytes.
    pub size: u64,
}

/// The program that converted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Converter {
    /// The name of the program or the library.
    pub name: String,
    /// Its version.
    pub version: String,
}

/// A count of something the converter handled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counted {
    /// What it is (`tracks`, `pads`, `text`...).
    pub item: String,
    /// How many.
    pub count: u64,
    /// Why it was not read or what was lost, when that is the case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// What `<branch>/origin/report.json` holds: what an import read and did not read, so the file
/// can be exported again without losses nobody knows about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionReport {
    /// The schema number.
    #[serde(default = "one")]
    pub schema: u32,
    /// The file that was read.
    pub source: SourceFile,
    /// The converter.
    pub converter: Converter,
    /// The name inside the branch of the kept copy of the original file (under `origin/`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
    /// What was read.
    #[serde(default)]
    pub read: Vec<Counted>,
    /// What was not read.
    #[serde(default)]
    pub not_read: Vec<Counted>,
    /// What an export to the same format cannot say (losses of the way back).
    #[serde(default)]
    pub export_losses: Vec<Counted>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ConversionReport {
    /// Checks the report against the names of the files of its branch: the hash is a SHA-256 and
    /// the kept original is under `origin/` and is there with the size and the name announced.
    pub fn validate<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, u64, &'a str)>,
    ) -> Result<(), String> {
        if !is_sha256_hex(&self.source.sha256) {
            return Err("the hash of the source is not a SHA-256".into());
        }
        if self.source.name.is_empty() || self.converter.name.is_empty() {
            return Err("the source or the converter has no name".into());
        }
        if let Some(original) = &self.original {
            if !original.starts_with(ORIGIN_DIR) || check_entry_name(original).is_err() {
                return Err(format!("'{original}' is not a name under origin/"));
            }
            let found = files.into_iter().find(|(name, _, _)| name == original);
            match found {
                None => return Err(format!("the original '{original}' is not in the branch")),
                Some((_, size, sha)) if size != self.source.size || sha != self.source.sha256 => {
                    return Err(format!(
                        "the original '{original}' is not the file the report describes"
                    ));
                }
                Some(_) => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(pairs: &[(&str, &str)]) -> LocalText {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn metadata_needs_english_and_the_eight_languages_only() {
        let mut meta = Metadata {
            title: text(&[("en", "Box"), ("es", "Caja"), ("pt-PT", "Caixa")]),
            license: Some("GPL-3.0-or-later".into()),
            ..Metadata::default()
        };
        assert_eq!(meta.validate(), Ok(()));
        meta.title.insert("nl".into(), "Doos".into());
        assert!(meta.validate().is_err());
        meta.title = text(&[("es", "Caja")]);
        assert!(meta.validate().is_err());
        meta.title = LocalText::new();
        meta.license = Some("not a licence!".into());
        assert!(meta.validate().is_err());
        meta.license = Some("MIT OR Apache-2.0".into());
        meta.links.push(Link {
            title: text(&[("en", "x")]),
            url: "ftp://x".into(),
        });
        assert!(meta.validate().is_err());
    }

    #[test]
    fn a_document_index_is_checked_against_its_files() {
        let doc = |path: &str, kind, lang: Option<&str>, of: Option<&str>| Document {
            path: path.into(),
            kind,
            title: text(&[("en", "t")]),
            lang: lang.map(Into::into),
            translation_of: of.map(Into::into),
            extra: BTreeMap::new(),
        };
        let files = ["a.en.md", "a.es.md", "scheme.png"];
        let mut index = DocIndex {
            schema: 1,
            documents: vec![
                doc("a.en.md", DocKind::Markdown, Some("en"), None),
                doc("a.es.md", DocKind::Markdown, Some("es"), Some("a.en.md")),
                doc("scheme.png", DocKind::Image, None, None),
            ],
            extra: BTreeMap::new(),
        };
        assert_eq!(index.validate(files), Ok(()));
        index.documents[2].kind = DocKind::Pdf;
        assert!(index.validate(files).is_err());
        index.documents[2].kind = DocKind::Image;
        index.documents[1].lang = Some("en".into());
        assert!(
            index.validate(files).is_err(),
            "a translation into its own language"
        );
        index.documents[1].lang = Some("xx".into());
        assert!(index.validate(files).is_err());
        index.documents[1].lang = Some("es".into());
        index.documents[1].translation_of = Some("missing.md".into());
        assert!(index.validate(files).is_err());
        index.documents[1].translation_of = None;
        index
            .documents
            .push(doc("a.en.md", DocKind::Markdown, None, None));
        assert!(index.validate(files).is_err(), "listed twice");
        assert!(
            index.validate(["a.en.md"]).is_err(),
            "a listed file that is missing"
        );
    }

    #[test]
    fn a_report_must_describe_the_kept_original() {
        let sha = "a".repeat(64);
        let report = ConversionReport {
            schema: 1,
            source: SourceFile {
                format: "eagle-brd".into(),
                name: "b.brd".into(),
                sha256: sha.clone(),
                size: 5,
            },
            converter: Converter {
                name: "t".into(),
                version: "1".into(),
            },
            original: Some("origin/b.brd".into()),
            read: vec![],
            not_read: vec![],
            export_losses: vec![],
            extra: BTreeMap::new(),
        };
        assert_eq!(report.validate([("origin/b.brd", 5, sha.as_str())]), Ok(()));
        assert!(report
            .validate([("origin/b.brd", 6, sha.as_str())])
            .is_err());
        assert!(report.validate([]).is_err());
        let mut outside = report.clone();
        outside.original = Some("b.brd".into());
        assert!(outside.validate([("b.brd", 5, sha.as_str())]).is_err());
        let mut bad_hash = report;
        bad_hash.source.sha256 = "zz".into();
        assert!(bad_hash.validate([]).is_err());
    }
}
