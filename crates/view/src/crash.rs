//! The record of a crash. A window program on Windows has no console to show a panic, so where it happened is also appended to a file in
//! the configuration folder of the user, with the program and its version; the next start of the program (or the person who reports the
//! fault) can read what happened.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The file of the crash records: `eh3d-crash.log` in the configuration folder of the user (`%APPDATA%\EH3D`, `~/.config/eh3d`).
pub fn log_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(|a| PathBuf::from(a).join("EH3D"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("eh3d")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("eh3d-crash.log")
}

/// The words of a panic, whatever it was raised with.
pub fn text_of(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown".to_string()
    }
}

/// One line of the record: seconds since 1970, the program, its version, the words and the place.
pub fn line(secs: u64, program: &str, version: &str, words: &str, place: &str) -> String {
    // a line break in the words would make the record look like two
    let words = words.replace(['\r', '\n'], " ");
    format!("[{secs}] {program} {version}: {words} at {place}")
}

/// Appends a line to the record at `path` (the folder is made when it is not there). Nothing is reported when it cannot be written: a program
/// that is already failing must not fail again on its way out.
pub fn append(path: &Path, text: &str) -> bool {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut file) => writeln!(file, "{text}").is_ok(),
        Err(_) => false,
    }
}

/// From now on a panic is also appended to the crash log before the usual message is given.
pub fn install(program: &'static str, version: &'static str) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        append(
            &log_path(),
            &line(secs, program, version, &text_of(info.payload()), &place),
        );
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_says_its_words_whatever_it_was_raised_with() {
        assert_eq!(text_of(&"plain"), "plain");
        assert_eq!(text_of(&String::from("formatted")), "formatted");
        assert_eq!(text_of(&42u32), "unknown");
    }

    #[test]
    fn a_record_is_one_line_with_the_program_the_version_and_the_place() {
        let l = line(7, "motioncnc", "0.1.0", "boom\nsecond", "src/a.rs:3");
        assert_eq!(l, "[7] motioncnc 0.1.0: boom second at src/a.rs:3");
        assert_eq!(l.lines().count(), 1);
    }

    #[test]
    fn records_are_appended_and_the_folder_is_made() {
        let dir = std::env::temp_dir().join(format!("eh3d-crash-{}", std::process::id()));
        let file = dir.join("deeper").join("crash.log");
        assert!(append(&file, "one"));
        assert!(append(&file, "two"));
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(text.lines().collect::<Vec<_>>(), ["one", "two"]);
        // a place that cannot be written is not an error: here the "file" is a folder
        assert!(!append(&dir.join("deeper"), "x"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
