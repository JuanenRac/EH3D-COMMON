//! The data files embedded in the crate: one JSON file per manufacturer, filaments and resins
//! apart.
//!
//! The list of the resin files is written out here so that a file is never embedded by accident;
//! the list of the filament files is written by `tools/import_open_filament_db.py` next to them
//! (`filament_files.rs`). A test fails when a file of a folder is not in its list.

#[path = "filament_files.rs"]
mod filament_files;

/// The embedded files of filaments as `(file name, JSON text)`.
pub use self::filament_files::FILES as FILAMENT_FILES;

/// The embedded files of resins as `(file name, JSON text)`.
pub const RESIN_FILES: &[(&str, &str)] =
    &[("generic.json", include_str!("../data/resins/generic.json"))];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PmError;

    fn json_names(folder: &str) -> Vec<String> {
        let dir = format!("{}/data/{folder}", env!("CARGO_MANIFEST_DIR"));
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".json"))
            .collect();
        names.sort();
        names
    }

    fn check_folder(folder: &str, listed: &[(&str, &str)]) {
        let names: Vec<&str> = listed.iter().map(|f| f.0).collect();
        let on_disk = json_names(folder);
        for name in &on_disk {
            assert!(
                names.contains(&name.as_str()),
                "{}",
                PmError::FileList {
                    file: format!("{folder}/{name}")
                }
            );
        }
        assert_eq!(on_disk.len(), listed.len());
    }

    #[test]
    fn every_json_file_of_the_data_folders_is_in_its_list() {
        check_folder("filaments", FILAMENT_FILES);
        check_folder("resins", RESIN_FILES);
    }

    #[test]
    fn the_names_of_the_lists_are_plain_and_unique() {
        for list in [FILAMENT_FILES, RESIN_FILES] {
            let mut names: Vec<&str> = list.iter().map(|f| f.0).collect();
            names.sort_unstable();
            let before = names.len();
            names.dedup();
            assert_eq!(before, names.len());
            for n in names {
                assert!(n.ends_with(".json") && !n.contains('/'));
            }
        }
    }
}
