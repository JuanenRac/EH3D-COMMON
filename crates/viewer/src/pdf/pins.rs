//! The one build of PDFium the viewer accepts (`pdfium.json`: release, version and the SHA-256 of
//! the library of every platform) and where it is looked for: the folder of the program, never the
//! current folder and never the search path of the system. A file that is not the pinned one is not
//! loaded.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::ViewerError;

const PINS: &str = include_str!("../../pdfium.json");

/// The library of one platform.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Pin {
    pub asset: String,
    pub archive_sha256: String,
    /// The path of the library inside the archive.
    pub member: String,
    /// The file name expected in the folder of the program.
    pub library: String,
    pub size: u64,
    pub library_sha256: String,
}

/// Every pin.
#[derive(Debug, Clone, Deserialize)]
pub struct Pins {
    pub project: String,
    pub release: String,
    pub version: String,
    /// False: the build accepted cannot run the scripts of a document.
    pub javascript: bool,
    pub xfa: bool,
    pub platforms: BTreeMap<String, Pin>,
}

impl Pins {
    /// The pins built into the program.
    pub fn built_in() -> Result<Pins, ViewerError> {
        serde_json::from_str(PINS).map_err(|e| ViewerError::EngineRejected(e.to_string()))
    }

    /// The pin of the system this program runs on, if there is one.
    pub fn here(&self) -> Option<&Pin> {
        self.platforms.get(&format!(
            "{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Checks that the file is exactly the pinned library.
pub fn verify(path: &Path, pin: &Pin) -> Result<(), ViewerError> {
    if !path.is_file() {
        return Err(ViewerError::EngineMissing);
    }
    let rejected = |why: &str| Err(ViewerError::EngineRejected(why.to_string()));
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() != pin.size {
        return rejected("it is not the pinned version (size)");
    }
    let mut data = Vec::new();
    file.take(pin.size + 1).read_to_end(&mut data)?;
    if hex(&Sha256::digest(&data)) != pin.library_sha256 {
        return rejected("it is not the pinned version (SHA-256)");
    }
    Ok(())
}

/// The folder of the program that runs.
pub fn program_folder() -> Result<PathBuf, ViewerError> {
    let program = std::env::current_exe()?;
    program
        .parent()
        .filter(|p| p.is_absolute())
        .map(Path::to_path_buf)
        .ok_or_else(|| ViewerError::EngineRejected("the program folder is unknown".into()))
}

/// The path of the library beside the program, once it has been checked.
pub fn library_beside_program() -> Result<PathBuf, ViewerError> {
    let pins = Pins::built_in()?;
    let pin = pins
        .here()
        .ok_or_else(|| ViewerError::EngineRejected("no pinned build for this system".into()))?;
    let path = program_folder()?.join(&pin.library);
    verify(&path, pin)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pins_are_complete_and_without_javascript() {
        let pins = Pins::built_in().unwrap();
        assert!(!pins.javascript && !pins.xfa);
        assert!(pins.release.starts_with("chromium/"));
        assert_eq!(pins.platforms.len(), 7);
        assert!(pins.here().is_some(), "the systems of the CI are pinned");
        for (name, pin) in &pins.platforms {
            assert!(!pin.asset.contains("v8"), "{name}");
            assert!(pin.member.ends_with(&pin.library), "{name}");
            for sum in [&pin.archive_sha256, &pin.library_sha256] {
                assert_eq!(sum.len(), 64, "{name}");
                assert!(sum.bytes().all(|b| b.is_ascii_hexdigit()), "{name}");
            }
        }
    }

    #[test]
    fn only_the_pinned_bytes_are_accepted() {
        let folder = std::env::temp_dir().join(format!("eh3d-pins-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("library");
        let pin = Pin {
            asset: "a".into(),
            archive_sha256: String::new(),
            member: "library".into(),
            library: "library".into(),
            size: 3,
            // SHA-256 of "abc"
            library_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .into(),
        };
        assert_eq!(verify(&path, &pin), Err(ViewerError::EngineMissing));
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(verify(&path, &pin), Ok(()));
        for other in [&b"abd"[..], b"abcd"] {
            std::fs::write(&path, other).unwrap();
            assert!(matches!(
                verify(&path, &pin),
                Err(ViewerError::EngineRejected(_))
            ));
        }
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
