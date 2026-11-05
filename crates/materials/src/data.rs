//! The data files embedded in the crate: one JSON file per manufacturer.
//!
//! The list is written out so that a file is never embedded by accident; a test fails when a
//! file of the `data` folder is not in it.

/// The embedded files as `(file name, JSON text)`.
pub const FILES: &[(&str, &str)] = &[
    ("bungard.json", include_str!("../data/bungard.json")),
    ("isola.json", include_str!("../data/isola.json")),
    ("panasonic.json", include_str!("../data/panasonic.json")),
    ("shengyi.json", include_str!("../data/shengyi.json")),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MatError;

    #[test]
    fn every_json_file_of_the_data_folder_is_in_the_list() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        let listed: Vec<&str> = FILES.iter().map(|f| f.0).collect();
        let mut seen = 0;
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") {
                seen += 1;
                assert!(
                    listed.contains(&name.as_str()),
                    "{}",
                    MatError::FileList { file: name }
                );
            }
        }
        assert_eq!(seen, FILES.len());
    }

    #[test]
    fn the_names_of_the_list_are_plain_and_unique() {
        let mut names: Vec<&str> = FILES.iter().map(|f| f.0).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
        for n in names {
            assert!(n.ends_with(".json") && !n.contains('/'));
        }
    }
}
