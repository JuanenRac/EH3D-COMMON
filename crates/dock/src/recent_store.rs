//! Where the list of recent projects of a program is kept: always a file of its own in the settings folder of the user (`%APPDATA%\EH3D` on
//! Windows, `$XDG_CONFIG_HOME/eh3d` or `~/.config/eh3d` elsewhere), the same for every program of the PHOENIX family and whatever settings file
//! the program was started with. It holds paths and nothing else; an address (anything with `://`, which could carry a password) is never
//! written. A file that cannot be used is left aside without an error, and it is written in one step, so a half written list is never read.
//!
//! Nothing here looks at the environment by itself: [`RecentStore::for_user`] is what a program calls when it opens its window, and a test gives
//! a folder of its own with [`RecentStore::in_dir`].

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::about::Phoenix;
use crate::config;
use crate::recent::RecentFiles;

/// The first line of the file.
const HEADER: &str = "eh3d.recent=1";
/// The largest file that is read.
const MAX_FILE: u64 = 64 * 1024;

/// The settings folder of the suite for the user, from the environment: `APPDATA`, else `XDG_CONFIG_HOME`, else `.config` in `HOME`. An empty
/// value, or one that is not an absolute path, counts as missing. `get` reads one variable, so the choice is tested without the real environment.
pub fn user_dir(get: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let value = |name: &str| {
        get(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    if let Some(base) = value("APPDATA") {
        return Some(base.join("EH3D"));
    }
    if let Some(base) = value("XDG_CONFIG_HOME") {
        return Some(base.join("eh3d"));
    }
    value("HOME").map(|home| home.join(".config").join("eh3d"))
}

/// The name of the file of a program.
fn file_name(program: Phoenix) -> String {
    let name = program.name().to_ascii_lowercase();
    format!("recent-{}.txt", name.trim_start_matches("eh3d-"))
}

/// The file of the recent projects of one program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentStore {
    path: PathBuf,
}

impl RecentStore {
    /// The list of a program in a folder.
    pub fn in_dir(dir: &Path, program: Phoenix) -> RecentStore {
        RecentStore {
            path: dir.join(file_name(program)),
        }
    }

    /// The list of a program in the settings folder of the user; None when the environment names no folder.
    pub fn for_user(program: Phoenix) -> Option<RecentStore> {
        user_dir(&|name| std::env::var(name).ok()).map(|dir| RecentStore::in_dir(&dir, program))
    }

    /// The file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The list that was kept, with at most `limit` entries. None when there is no file, or when what is there cannot be used (it is not this
    /// kind of file, it is not text or it is too large).
    pub fn load(&self, limit: usize) -> Option<RecentFiles> {
        let file = fs::File::open(&self.path).ok()?;
        let mut text = String::new();
        file.take(MAX_FILE + 1).read_to_string(&mut text).ok()?;
        if text.len() as u64 > MAX_FILE || text.lines().next().map(str::trim) != Some(HEADER) {
            return None;
        }
        let mut pairs = config::dock_pairs(&text);
        pairs.retain(|(key, value)| key != "dock.recent_limit" && !value.contains("://"));
        pairs.push(("dock.recent_limit".to_string(), limit.to_string()));
        Some(RecentFiles::from_pairs(&pairs))
    }

    /// Writes the list: into a new file of the same folder first, which then takes the place of the old one.
    pub fn save(&self, recent: &RecentFiles) -> io::Result<()> {
        static CALLS: AtomicU32 = AtomicU32::new(0);
        let mut text = format!("{HEADER}\n");
        for (i, path) in recent
            .items()
            .iter()
            .filter(|path| !path.contains("://"))
            .enumerate()
        {
            text.push_str(&format!("dock.recent.{i}={path}\n"));
        }
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let call = CALLS.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .path
            .with_extension(format!("{}-{call}.tmp", std::process::id()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            // the names of the files of a person are for that person
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let written = options.open(&temporary).and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        });
        let moved = written.and_then(|()| fs::rename(&temporary, &self.path));
        if moved.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own for one test, removed at the end.
    struct Folder(PathBuf);

    impl Folder {
        fn new() -> Folder {
            static CALLS: AtomicU32 = AtomicU32::new(0);
            let call = CALLS.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir()
                .join(format!("eh3d-recent-store-{}-{call}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Folder(dir)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn list(paths: &[&str]) -> RecentFiles {
        let mut r = RecentFiles::default();
        for p in paths.iter().rev() {
            r.push(p);
        }
        r
    }

    #[test]
    fn the_folder_of_the_user_comes_from_the_environment() {
        let root = std::env::temp_dir();
        let (a, x, h) = (root.join("a"), root.join("x"), root.join("h"));
        let env = |pairs: &[(&str, &Path)]| {
            let pairs: Vec<(String, String)> = pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.display().to_string()))
                .collect();
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| k == name)
                    .map(|(_, v)| v.clone())
            }
        };
        let all = env(&[("APPDATA", &a), ("XDG_CONFIG_HOME", &x), ("HOME", &h)]);
        assert_eq!(user_dir(&all), Some(a.join("EH3D")));
        let xdg = env(&[("XDG_CONFIG_HOME", &x), ("HOME", &h)]);
        assert_eq!(user_dir(&xdg), Some(x.join("eh3d")));
        let home = env(&[("HOME", &h)]);
        assert_eq!(user_dir(&home), Some(h.join(".config").join("eh3d")));
        assert_eq!(user_dir(&env(&[])), None);
        let unusable = env(&[("APPDATA", Path::new("")), ("HOME", Path::new("relative"))]);
        assert_eq!(user_dir(&unusable), None);
    }

    #[test]
    fn each_program_has_its_own_file() {
        let names: Vec<String> = Phoenix::ALL.into_iter().map(file_name).collect();
        assert_eq!(names[0], "recent-phoenix-pcb.txt");
        assert_eq!(names[5], "recent-phoenix-3dp.txt");
        for (i, name) in names.iter().enumerate() {
            assert!(!names[..i].contains(name), "{name}");
        }
    }

    #[test]
    fn what_is_saved_is_read_back_and_no_temporary_file_is_left() {
        let folder = Folder::new();
        let store = RecentStore::in_dir(&folder.0.join("EH3D"), Phoenix::Cad);
        assert_eq!(store.load(10), None);
        let first = list(&[
            "C:\\jobs\\two.eh3d",
            "/home/me/a=b.eh3d",
            "/home/me/ñ 板.eh3d",
        ]);
        store.save(&first).unwrap();
        assert_eq!(store.load(10), Some(first.clone()));
        let second = list(&["/other.eh3d"]);
        store.save(&second).unwrap();
        assert_eq!(store.load(10), Some(second));
        assert_eq!(fs::read_dir(folder.0.join("EH3D")).unwrap().count(), 1);
        // the limit of the settings of the program is the one that counts
        store.save(&first).unwrap();
        assert_eq!(store.load(2).unwrap().items(), &first.items()[..2]);
        store.save(&RecentFiles::default()).unwrap();
        assert!(store.load(10).unwrap().items().is_empty());
    }

    #[test]
    fn an_address_is_never_written_nor_read() {
        let folder = Folder::new();
        let store = RecentStore::in_dir(&folder.0, Phoenix::Pnp);
        let address = format!("https://{}@example.invalid/job.eh3d", "someone");
        store.save(&list(&[&address, "/a.eh3d"])).unwrap();
        assert!(!fs::read_to_string(store.path()).unwrap().contains("://"));
        assert_eq!(store.load(10).unwrap().items(), ["/a.eh3d"]);
        fs::write(store.path(), format!("{HEADER}\ndock.recent.0={address}\n")).unwrap();
        assert!(store.load(10).unwrap().items().is_empty());
    }

    #[test]
    fn a_damaged_file_is_left_aside_and_replaced_by_the_next_list() {
        let folder = Folder::new();
        fs::create_dir_all(&folder.0).unwrap();
        let store = RecentStore::in_dir(&folder.0, Phoenix::Laser);
        let too_large = format!("{HEADER}\n{}", "#\n".repeat(MAX_FILE as usize));
        for damaged in [
            Vec::new(),
            vec![0xff, 0xfe, 0x00, 0x41],
            b"dock.recent.0=/a\n".to_vec(),
            b"eh3d.recent=9\ndock.recent.0=/a\n".to_vec(),
            too_large.into_bytes(),
        ] {
            fs::write(store.path(), &damaged).unwrap();
            assert_eq!(store.load(10), None);
        }
        store.save(&list(&["/b.eh3d"])).unwrap();
        assert_eq!(store.load(10).unwrap().items(), ["/b.eh3d"]);
    }

    #[test]
    fn a_folder_that_cannot_be_made_gives_an_error_and_no_panic() {
        let folder = Folder::new();
        fs::create_dir_all(&folder.0).unwrap();
        let file = folder.0.join("a-file");
        fs::write(&file, "x").unwrap();
        let store = RecentStore::in_dir(&file, Phoenix::Cnc);
        assert!(store.save(&list(&["/a"])).is_err());
        assert_eq!(store.load(10), None);
    }
}
