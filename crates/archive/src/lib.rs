//! A job in a ZIP archive: the drilling files (`.ncd`, `.drl`...) and the routing files (`.plt`, `.hpgl`...) of a job together. The archive
//! is opened with care: a name that would leave the folder it is extracted to is not listed, an archive with too many entries is
//! refused, and no entry is read past a limit, whatever its header says.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// What a file of an archive is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A drilling file.
    Drill,
    /// A routing file (HP-GL, or the outline of a board in Gerber or DXF).
    Route,
    /// Anything else (notes, pictures, the Gerber layers of copper): listed, not loaded.
    Other,
}

/// What the extension of a file name says about it.
pub fn kind_of(name: &str) -> Kind {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "ncd" | "drl" | "exc" | "xln" | "drd" => Kind::Drill,
        "plt" | "hpgl" | "hpg" | "plot" => Kind::Route,
        // the outline of a board in Gerber: its edge is what a router cuts
        "gko" | "gm1" | "gml" | "gm2" | "gm3" | "gbr" => Kind::Route,
        // a drawing of the board: its lines, arcs and circles are what a router cuts
        "dxf" => Kind::Route,
        _ => Kind::Other,
    }
}

/// The name a file of an archive is given when it is extracted: only its last part, and never one that could point out of the folder.
pub fn safe_name(name: &str) -> Option<String> {
    let name = name.replace('\\', "/");
    if name.starts_with('/') || name.split('/').any(|p| p == "..") {
        return None;
    }
    let last = name.rsplit('/').next()?;
    if last.is_empty() || last == "." || last.contains(':') || last.contains('\0') {
        return None;
    }
    Some(last.to_string())
}

/// One file of the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The name inside the archive (with its folders).
    pub name: String,
    pub size: u64,
    pub kind: Kind,
}

/// The most entries an archive may have, and the most bytes read from one entry.
pub const MAX_ENTRIES: usize = 2000;
pub const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;

/// Why an archive could not be opened, read or extracted. Each case keeps what went wrong apart from the words, so a caller can act on it.
#[derive(Debug)]
pub enum ArchiveError {
    /// The file or folder could not be read or written.
    Io {
        path: String,
        source: std::io::Error,
    },
    /// The file is not a ZIP archive (or an entry of it is damaged).
    Zip { path: String, detail: String },
    /// The archive has more entries than [`MAX_ENTRIES`].
    TooManyEntries { path: String, count: usize },
    /// The name is not one of the listed entries.
    NotInArchive { name: String },
    /// The entry is bigger than the limit asked for.
    TooBig { name: String, limit: u64 },
    /// The name could point out of the folder it is extracted to.
    UnsafeName { name: String },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::Io { path, source } => write!(f, "{path}: {source}"),
            ArchiveError::Zip { path, detail } => {
                write!(f, "{path}: not a ZIP archive ({detail})")
            }
            ArchiveError::TooManyEntries { path, count } => {
                write!(f, "{path}: too many entries ({count})")
            }
            ArchiveError::NotInArchive { name } => write!(f, "{name}: not in the archive"),
            ArchiveError::TooBig { name, limit } => {
                write!(f, "{name}: bigger than {limit} bytes")
            }
            ArchiveError::UnsafeName { name } => write!(f, "{name}: not a safe name"),
        }
    }
}

impl std::error::Error for ArchiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ArchiveError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The words of the error, for callers that still carry their errors as text.
impl From<ArchiveError> for String {
    fn from(e: ArchiveError) -> String {
        e.to_string()
    }
}

pub struct Archive {
    zip: zip::ZipArchive<File>,
    /// The files that can be used: folders and names that are not safe are left out.
    pub entries: Vec<Entry>,
}

impl Archive {
    pub fn open(path: &Path) -> Result<Archive, ArchiveError> {
        let shown = path.display().to_string();
        let file = File::open(path).map_err(|source| ArchiveError::Io {
            path: shown.clone(),
            source,
        })?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| ArchiveError::Zip {
            path: shown.clone(),
            detail: e.to_string(),
        })?;
        if zip.len() > MAX_ENTRIES {
            return Err(ArchiveError::TooManyEntries {
                path: shown,
                count: zip.len(),
            });
        }
        let mut entries = Vec::new();
        for i in 0..zip.len() {
            let f = zip.by_index(i).map_err(|e| ArchiveError::Zip {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?;
            let Ok(name) = f.name().map(|n| n.into_owned()) else {
                continue;
            };
            if f.is_dir() || safe_name(&name).is_none() {
                continue;
            }
            entries.push(Entry {
                size: f.size(),
                kind: kind_of(&name),
                name,
            });
        }
        Ok(Archive { zip, entries })
    }

    /// The first file of a kind, in the order of the archive.
    pub fn first(&self, kind: Kind) -> Option<&Entry> {
        self.entries.iter().find(|e| e.kind == kind)
    }

    /// The bytes of a file, at most `limit` of them (more is an error, whatever the header of the archive says).
    pub fn read_limited(&mut self, name: &str, limit: u64) -> Result<Vec<u8>, ArchiveError> {
        if !self.entries.iter().any(|e| e.name == name) {
            return Err(ArchiveError::NotInArchive {
                name: name.to_string(),
            });
        }
        let f = self.zip.by_name(name).map_err(|e| ArchiveError::Zip {
            path: name.to_string(),
            detail: e.to_string(),
        })?;
        let mut out = Vec::new();
        f.take(limit + 1)
            .read_to_end(&mut out)
            .map_err(|source| ArchiveError::Io {
                path: name.to_string(),
                source,
            })?;
        if out.len() as u64 > limit {
            return Err(ArchiveError::TooBig {
                name: name.to_string(),
                limit,
            });
        }
        Ok(out)
    }

    pub fn read(&mut self, name: &str) -> Result<Vec<u8>, ArchiveError> {
        self.read_limited(name, MAX_ENTRY_BYTES)
    }

    /// Writes a file of the archive into `dir` (which is made when it does not exist) under its safe name; says where it went.
    pub fn extract(&mut self, name: &str, dir: &Path) -> Result<PathBuf, ArchiveError> {
        let mut target = safe_name(name).ok_or_else(|| ArchiveError::UnsafeName {
            name: name.to_string(),
        })?;
        // two files of different folders with the same last part must not land on each other: the second one keeps its folders in its name
        let same_last_part = self
            .entries
            .iter()
            .filter(|e| e.name != name && safe_name(&e.name).as_deref() == Some(target.as_str()))
            .count();
        if same_last_part > 0 {
            let flat = name.replace('\\', "/").replace('/', "__");
            if let Some(unique) = safe_name(&flat) {
                target = unique;
            }
        }
        let bytes = self.read(name)?;
        std::fs::create_dir_all(dir).map_err(|source| ArchiveError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = dir.join(target);
        std::fs::write(&path, bytes).map_err(|source| ArchiveError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eh3d-archive-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A ZIP with the given (name, bytes) entries.
    fn make(path: &Path, files: &[(&str, &[u8])]) {
        let mut w = zip::ZipWriter::new(File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn the_extension_tells_what_a_file_is() {
        assert_eq!(kind_of("board.NCD"), Kind::Drill);
        assert_eq!(kind_of("a/b/holes.drl"), Kind::Drill);
        assert_eq!(kind_of("outline.PLT"), Kind::Route);
        assert_eq!(kind_of("cut.hpgl"), Kind::Route);
        assert_eq!(kind_of("notes.txt"), Kind::Other);
        assert_eq!(kind_of("noext"), Kind::Other);
    }

    #[test]
    fn only_safe_names_are_kept_and_only_their_last_part() {
        assert_eq!(safe_name("job/board.ncd").as_deref(), Some("board.ncd"));
        assert_eq!(safe_name("job\\board.ncd").as_deref(), Some("board.ncd"));
        for bad in [
            "../evil.ncd",
            "a/../../evil.ncd",
            "/etc/passwd",
            "c:evil.ncd",
            "dir/",
            ".",
            "",
        ] {
            assert_eq!(safe_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn files_with_the_same_name_in_different_folders_do_not_overwrite_each_other() {
        let d = scratch("same");
        let z = d.join("job.zip");
        make(
            &z,
            &[
                ("top/job.ncd", b"top"),
                ("bottom/job.ncd", b"bottom"),
                ("only/one.ncd", b"one"),
            ],
        );
        let mut a = Archive::open(&z).unwrap();
        let out = d.join("out");
        let top = a.extract("top/job.ncd", &out).unwrap();
        let bottom = a.extract("bottom/job.ncd", &out).unwrap();
        let one = a.extract("only/one.ncd", &out).unwrap();
        assert_ne!(top, bottom);
        assert_eq!(std::fs::read(&top).unwrap(), b"top");
        assert_eq!(std::fs::read(&bottom).unwrap(), b"bottom");
        assert_eq!(one.file_name().unwrap(), "one.ncd");
    }

    #[test]
    fn a_job_archive_lists_its_drilling_and_routing_files() {
        let d = scratch("list");
        let z = d.join("job.zip");
        make(
            &z,
            &[
                ("job/board.NCD", b"%\nT10\nX1000Y1000\nM30\n"),
                ("job/outline.PLT", b"IN;SP1;PU0,0;PD400,0;"),
                ("job/readme.txt", b"hello"),
                ("../evil.ncd", b"x"),
            ],
        );
        let mut a = Archive::open(&z).unwrap();
        // the hostile name is not listed
        assert_eq!(a.entries.len(), 3);
        assert_eq!(a.first(Kind::Drill).unwrap().name, "job/board.NCD");
        assert_eq!(a.first(Kind::Route).unwrap().name, "job/outline.PLT");
        assert_eq!(a.first(Kind::Other).unwrap().name, "job/readme.txt");
        assert_eq!(a.read("job/readme.txt").unwrap(), b"hello");
        assert!(a
            .read("../evil.ncd")
            .unwrap_err()
            .to_string()
            .contains("not in the archive"));
    }

    #[test]
    fn a_file_is_extracted_under_its_safe_name() {
        let d = scratch("extract");
        let z = d.join("job.zip");
        make(&z, &[("deep/er/board.ncd", b"%\nM30\n")]);
        let mut a = Archive::open(&z).unwrap();
        let out = d.join("out");
        let p = a.extract("deep/er/board.ncd", &out).unwrap();
        assert_eq!(p, out.join("board.ncd"));
        assert_eq!(std::fs::read(&p).unwrap(), b"%\nM30\n");
    }

    #[test]
    fn an_entry_is_never_read_past_the_limit() {
        let d = scratch("limit");
        let z = d.join("job.zip");
        make(&z, &[("big.ncd", &vec![b'x'; 5000])]);
        let mut a = Archive::open(&z).unwrap();
        assert_eq!(a.read_limited("big.ncd", 5000).unwrap().len(), 5000);
        assert!(a
            .read_limited("big.ncd", 4999)
            .unwrap_err()
            .to_string()
            .contains("bigger than"));
    }

    #[test]
    fn what_is_not_a_zip_or_does_not_exist_is_said() {
        let d = scratch("bad");
        let f = d.join("job.zip");
        std::fs::write(&f, b"this is not a zip").unwrap();
        assert!(Archive::open(&f)
            .err()
            .unwrap()
            .to_string()
            .contains("not a ZIP archive"));
        assert!(Archive::open(&d.join("missing.zip")).is_err());
    }

    #[test]
    fn errors_keep_their_kind_and_still_turn_into_text() {
        let d = scratch("kinds");
        let z = d.join("job.zip");
        make(&z, &[("a.ncd", b"x")]);
        let mut a = Archive::open(&z).unwrap();
        assert!(matches!(
            a.read("b.ncd"),
            Err(ArchiveError::NotInArchive { .. })
        ));
        assert!(matches!(
            a.extract("../x.ncd", &d),
            Err(ArchiveError::UnsafeName { .. })
        ));
        assert!(matches!(
            Archive::open(&d.join("missing.zip")),
            Err(ArchiveError::Io { .. })
        ));
        let text: String = a.read("b.ncd").unwrap_err().into();
        assert!(text.contains("not in the archive"));
    }
}
