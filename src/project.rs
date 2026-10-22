//! The metadata of a project: what `project.json` inside a `.eh3dproj` package says.
//!
//! The file declares its own `format_version` (a semantic version). A reader accepts every file of
//! the same major version; a newer major version is refused with a clear error instead of being
//! half understood. Migrations of older majors would be registered in [`MIGRATIONS`]: there is
//! none yet because format 1.0.0 is the first one.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{Error, Result};

/// The format version written by this crate.
pub const FORMAT_VERSION: Version = Version::new(1, 0, 0);

/// The folders of a package that hold data, besides `project.json` and `integrity.json`.
pub const DATA_FOLDERS: [&str; 3] = ["layers/", "toolpaths/", "vision/"];

/// A step that rewrites the JSON of an older major version into the next one.
type Migration = fn(Value) -> Result<Value>;

/// Migrations by the major version they read. Empty: the first format has nothing to migrate from.
const MIGRATIONS: &[(u64, Migration)] = &[];

/// The units the coordinates of a project are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Units {
    /// Millimetres (the default).
    #[default]
    Millimetres,
    /// Inches.
    Inches,
}

/// A point of the workpiece or of the machine.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate.
    pub z: f64,
}

impl Point3 {
    fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

/// What a layer file is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    /// A copper layer (Gerber).
    Copper,
    /// The solder mask.
    SolderMask,
    /// The silkscreen legend.
    Silkscreen,
    /// The solder paste (stencil) layer.
    Paste,
    /// The board outline.
    Outline,
    /// A drill file (Excellon).
    Drill,
    /// Anything else.
    Other,
}

/// A layer file stored under `layers/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerRef {
    /// The path inside the package, for example `layers/top_copper.gbr`.
    pub path: String,
    /// What the layer is.
    pub kind: LayerKind,
    /// A name for people.
    pub name: String,
}

/// A tool of the tool store the project refers to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRef {
    /// A stable identifier inside the project.
    pub id: String,
    /// A name for people.
    pub name: String,
    /// The cutting diameter in millimetres, when it has one.
    pub diameter_mm: Option<f64>,
}

/// The dialect a toolpath file is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dialect {
    /// G-code for GRBL.
    Grbl,
    /// G-code for Marlin.
    Marlin,
    /// G-code for PlanetCNC.
    PlanetCnc,
    /// HP-GL as some plotters and older milling machines take it.
    Hpgl,
}

/// A precalculated toolpath stored under `toolpaths/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolpathRef {
    /// The path inside the package, for example `toolpaths/isolation.nc`.
    pub path: String,
    /// The dialect of the file.
    pub dialect: Dialect,
    /// The tool it was calculated for, when it is one of [`ProjectMetadata::tools`].
    pub tool_id: Option<String>,
}

/// An alignment mark the vision system finds on the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fiducial {
    /// A name for people.
    pub name: String,
    /// The expected position in the project's units.
    pub x: f64,
    /// The expected position in the project's units.
    pub y: f64,
}

/// What `project.json` holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// The version of the format this file follows.
    pub format_version: Version,
    /// A stable identity that survives renames and copies of the file.
    pub id: Uuid,
    /// The name of the project.
    pub name: String,
    /// When the project was created.
    pub created: DateTime<Utc>,
    /// When it was last changed.
    pub modified: DateTime<Utc>,
    /// The units of every coordinate in the project.
    #[serde(default)]
    pub units: Units,
    /// Where the zero of the workpiece is, when it is set.
    #[serde(default)]
    pub zero_piece: Option<Point3>,
    /// The layer files of the package.
    #[serde(default)]
    pub layers: Vec<LayerRef>,
    /// The tools the project refers to.
    #[serde(default)]
    pub tools: Vec<ToolRef>,
    /// The precalculated toolpaths.
    #[serde(default)]
    pub toolpaths: Vec<ToolpathRef>,
    /// The alignment marks for the vision system.
    #[serde(default)]
    pub fiducials: Vec<Fiducial>,
    /// Free notes.
    #[serde(default)]
    pub notes: Option<String>,
}

impl ProjectMetadata {
    /// A new, empty project named `name`, created now, in the current format.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            format_version: FORMAT_VERSION,
            id: Uuid::new_v4(),
            name: name.into(),
            created: now,
            modified: now,
            units: Units::default(),
            zero_piece: None,
            layers: Vec::new(),
            tools: Vec::new(),
            toolpaths: Vec::new(),
            fiducials: Vec::new(),
            notes: None,
        }
    }

    /// Checks what the types cannot say: a name, safe paths in the right folders, unique tool
    /// ids, toolpaths that name a tool that exists, and finite numbers.
    pub fn validate(&self) -> Result<()> {
        let bad = |message: String| Err(Error::InvalidMetadata(message));
        if self.name.trim().is_empty() {
            return bad("the project name is empty".into());
        }
        if self.modified < self.created {
            return bad("the project was modified before it was created".into());
        }
        if let Some(zero) = &self.zero_piece {
            if !zero.is_finite() {
                return bad("zero_piece has a coordinate that is not a finite number".into());
            }
        }
        for layer in &self.layers {
            crate::container::check_entry_name(&layer.path)?;
            if !layer.path.starts_with("layers/") {
                return bad(format!("layer '{}' is not under layers/", layer.path));
            }
        }
        for path in self.toolpaths.iter().map(|t| &t.path) {
            crate::container::check_entry_name(path)?;
            if !path.starts_with("toolpaths/") {
                return bad(format!("toolpath '{path}' is not under toolpaths/"));
            }
        }
        let mut ids = HashSet::new();
        for tool in &self.tools {
            if tool.id.trim().is_empty() {
                return bad("a tool has an empty id".into());
            }
            if !ids.insert(tool.id.as_str()) {
                return bad(format!("the tool id '{}' appears twice", tool.id));
            }
            if let Some(d) = tool.diameter_mm {
                if !d.is_finite() || d <= 0.0 {
                    return bad(format!(
                        "tool '{}' has a diameter that is not a positive number",
                        tool.id
                    ));
                }
            }
        }
        for toolpath in &self.toolpaths {
            if let Some(id) = &toolpath.tool_id {
                if !ids.contains(id.as_str()) {
                    return bad(format!(
                        "toolpath '{}' names the unknown tool '{id}'",
                        toolpath.path
                    ));
                }
            }
        }
        for mark in &self.fiducials {
            if !mark.x.is_finite() || !mark.y.is_finite() {
                return bad(format!(
                    "fiducial '{}' has a coordinate that is not a finite number",
                    mark.name
                ));
            }
        }
        Ok(())
    }

    /// The JSON of the project, indented so that a person (and a diff) can read it.
    pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Reads `project.json`: finds the declared format version, migrates an older major version,
/// refuses a newer one, then checks the result.
pub fn read_metadata(json: &str) -> Result<ProjectMetadata> {
    let mut value: Value = serde_json::from_str(json)?;
    let declared = value
        .get("format_version")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::InvalidMetadata("format_version is missing".into()))?
        .to_owned();
    let found = Version::parse(&declared).map_err(|_| {
        Error::InvalidMetadata(format!(
            "format_version '{declared}' is not a semantic version"
        ))
    })?;
    let supported = format!("{}.x", FORMAT_VERSION.major);
    if found.major > FORMAT_VERSION.major {
        return Err(Error::UnsupportedFormat {
            found: declared,
            supported,
        });
    }
    let mut major = found.major;
    while major < FORMAT_VERSION.major {
        let step = MIGRATIONS
            .iter()
            .find(|(from, _)| *from == major)
            .map(|(_, f)| *f)
            .ok_or_else(|| Error::UnsupportedFormat {
                found: declared.clone(),
                supported: supported.clone(),
            })?;
        value = step(value)?;
        major += 1;
    }
    let metadata: ProjectMetadata = serde_json::from_value(value)?;
    metadata.validate()?;
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProjectMetadata {
        let mut m = ProjectMetadata::new("Test board");
        m.layers.push(LayerRef {
            path: "layers/top.gbr".into(),
            kind: LayerKind::Copper,
            name: "Top".into(),
        });
        m.tools.push(ToolRef {
            id: "v30".into(),
            name: "V-bit 30".into(),
            diameter_mm: Some(0.1),
        });
        m.toolpaths.push(ToolpathRef {
            path: "toolpaths/iso.nc".into(),
            dialect: Dialect::Grbl,
            tool_id: Some("v30".into()),
        });
        m
    }

    #[test]
    fn a_new_project_is_valid_and_round_trips() {
        let m = sample();
        let json = m.to_json().unwrap();
        assert_eq!(read_metadata(&json).unwrap(), m);
    }

    #[test]
    fn an_empty_name_is_refused() {
        let mut m = sample();
        m.name = "  ".into();
        assert!(matches!(m.validate(), Err(Error::InvalidMetadata(_))));
    }

    #[test]
    fn a_layer_outside_layers_is_refused() {
        let mut m = sample();
        m.layers[0].path = "toolpaths/top.gbr".into();
        assert!(m.validate().is_err());
    }

    #[test]
    fn repeated_tool_ids_and_unknown_tools_are_refused() {
        let mut m = sample();
        m.tools.push(m.tools[0].clone());
        assert!(m.validate().is_err());
        let mut m = sample();
        m.toolpaths[0].tool_id = Some("nope".into());
        assert!(m.validate().is_err());
    }

    #[test]
    fn numbers_must_be_finite_and_diameters_positive() {
        let mut m = sample();
        m.zero_piece = Some(Point3 {
            x: f64::NAN,
            y: 0.0,
            z: 0.0,
        });
        assert!(m.validate().is_err());
        let mut m = sample();
        m.tools[0].diameter_mm = Some(0.0);
        assert!(m.validate().is_err());
        let mut m = sample();
        m.fiducials.push(Fiducial {
            name: "F1".into(),
            x: f64::INFINITY,
            y: 1.0,
        });
        assert!(m.validate().is_err());
    }

    #[test]
    fn a_newer_major_version_is_refused_and_a_newer_minor_is_read() {
        let json = sample().to_json().unwrap();
        let newer_major = json.replace("\"1.0.0\"", "\"2.0.0\"");
        assert!(matches!(
            read_metadata(&newer_major),
            Err(Error::UnsupportedFormat { .. })
        ));
        let newer_minor = json.replace("\"1.0.0\"", "\"1.4.2\"");
        assert!(read_metadata(&newer_minor).is_ok());
    }

    #[test]
    fn an_older_major_without_a_migration_is_refused() {
        let json = sample()
            .to_json()
            .unwrap()
            .replace("\"1.0.0\"", "\"0.9.0\"");
        assert!(matches!(
            read_metadata(&json),
            Err(Error::UnsupportedFormat { .. })
        ));
    }

    #[test]
    fn a_missing_or_bad_format_version_is_refused() {
        assert!(matches!(
            read_metadata("{}"),
            Err(Error::InvalidMetadata(_))
        ));
        assert!(matches!(
            read_metadata("{\"format_version\":\"one\"}"),
            Err(Error::InvalidMetadata(_))
        ));
        assert!(read_metadata("not json").is_err());
    }
}
