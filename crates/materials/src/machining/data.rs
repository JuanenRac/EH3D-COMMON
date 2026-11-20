//! The data files of the machining family embedded in the crate (`data/machining/`).
//!
//! The list is written out so that a file is never embedded by accident; a test fails when a
//! file of the folder is not in it.

/// The embedded files as `(file name, JSON text)`.
pub const FILES: &[(&str, &str)] = &[
    (
        "board.json",
        include_str!("../../data/machining/board.json"),
    ),
    (
        "metal.json",
        include_str!("../../data/machining/metal.json"),
    ),
    (
        "mineral.json",
        include_str!("../../data/machining/mineral.json"),
    ),
    (
        "plastic.json",
        include_str!("../../data/machining/plastic.json"),
    ),
    (
        "sheet.json",
        include_str!("../../data/machining/sheet.json"),
    ),
    ("wood.json", include_str!("../../data/machining/wood.json")),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MatError;

    #[test]
    fn every_json_file_of_the_machining_folder_is_in_the_list() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data/machining");
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
