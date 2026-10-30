//! The state of an Open and Save window made like the one of any Windows program: the places on the left (the examples, the desktop, the
//! documents, the downloads and the drives), the path above, the list of the folder with its name, date, type and size (a click on a
//! heading sorts by it), the file name and the type of file below. There is no interface here: a program draws the window with its own
//! toolkit over a [`Browser`] and asks it what to do with each click, so that every program of the suite opens and saves the same way.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// What the window is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Open,
    Save,
}

/// A line of the folder list.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// A type of file the list can be limited to: the extensions it takes (lower case, without the dot); none means every file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub extensions: Vec<String>,
}

impl Filter {
    pub fn new(extensions: &[&str]) -> Filter {
        Filter {
            extensions: extensions.iter().map(|e| e.to_ascii_lowercase()).collect(),
        }
    }

    pub fn any() -> Filter {
        Filter {
            extensions: Vec::new(),
        }
    }

    pub fn shows(&self, name: &str) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        name.rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .is_some_and(|e| self.extensions.contains(&e))
    }
}

/// What a place on the left is, so that the program can name it in the language of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    Examples,
    Desktop,
    Documents,
    Downloads,
    Drive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub kind: PlaceKind,
    /// The name of a drive (`C:`); empty for the others.
    pub name: String,
    pub path: PathBuf,
}

/// What a double click or Enter on a line of the list did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Nothing,
    /// A folder was entered: the list shows it now.
    Entered,
    /// A file was chosen.
    Chosen(PathBuf),
}

/// Why the file name typed cannot be taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// No name was typed (or none is chosen).
    NoName,
    /// To open: there is no such file.
    Missing(PathBuf),
    /// The name is a folder (to open: go into it).
    IsFolder(PathBuf),
    /// There is no folder open (the list of drives).
    NoFolder,
}

/// The answer to Open or Save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chosen {
    /// The file to open, or the new file to save.
    File(PathBuf),
    /// To save: the file is there already, and the window has to ask whether to replace it.
    Overwrite(PathBuf),
}

/// The folder being looked at and what is chosen in it.
#[derive(Debug, Clone)]
pub struct Browser {
    pub mode: Mode,
    /// `None` is the list of drives.
    pub dir: Option<PathBuf>,
    pub entries: Vec<Entry>,
    pub sel: Option<usize>,
    /// The column sorted by (0 name, 1 date, 2 type, 3 size) and whether it goes up.
    pub sort: (usize, bool),
    pub filters: Vec<Filter>,
    pub filter: usize,
    pub history: Vec<Option<PathBuf>>,
    /// The path as typed above the list.
    pub path_text: String,
    /// The file name as typed below it.
    pub name_text: String,
    pub places: Vec<Place>,
    /// The extension a name that has none is given when saving (`nc`).
    pub default_ext: Option<String>,
}

impl Browser {
    /// A window that starts in `start`, with the examples folder of the program as the first place when it has one.
    pub fn new(
        mode: Mode,
        start: &Path,
        examples_base: &Path,
        filters: Vec<Filter>,
        default_ext: Option<&str>,
    ) -> Browser {
        let mut b = Browser {
            mode,
            dir: Some(start.to_path_buf()),
            entries: Vec::new(),
            sel: None,
            sort: (0, true),
            filters: if filters.is_empty() {
                vec![Filter::any()]
            } else {
                filters
            },
            filter: 0,
            history: Vec::new(),
            path_text: String::new(),
            name_text: String::new(),
            places: places(examples_base, start),
            default_ext: default_ext.map(|e| e.trim_start_matches('.').to_string()),
        };
        b.refresh();
        b
    }

    /// Reads the folder again with the filter and the sort of now.
    pub fn refresh(&mut self) {
        self.sel = None;
        let mut all = Vec::new();
        match &self.dir {
            None => {
                for l in b'A'..=b'Z' {
                    let d = format!("{}:\\", l as char);
                    if Path::new(&d).exists() {
                        all.push(Entry {
                            name: d,
                            is_dir: true,
                            size: 0,
                            modified: None,
                        });
                    }
                }
            }
            Some(d) => {
                if let Ok(rd) = std::fs::read_dir(d) {
                    let filter = self
                        .filters
                        .get(self.filter)
                        .cloned()
                        .unwrap_or_else(Filter::any);
                    for e in rd.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        let meta = e.metadata().ok();
                        let is_dir = e.path().is_dir();
                        if is_dir && name.starts_with('.') {
                            continue;
                        }
                        if !is_dir && !filter.shows(&name) {
                            continue;
                        }
                        all.push(Entry {
                            name,
                            is_dir,
                            size: meta.as_ref().map_or(0, |m| m.len()),
                            modified: meta.and_then(|m| m.modified().ok()),
                        });
                    }
                }
            }
        }
        let (col, up) = self.sort;
        all.sort_by(|a, b| {
            // the folders come first, whatever the sort
            b.is_dir.cmp(&a.is_dir).then_with(|| {
                let o = match col {
                    1 => a.modified.cmp(&b.modified),
                    2 => ext_of(&a.name).cmp(&ext_of(&b.name)),
                    3 => a.size.cmp(&b.size),
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                };
                if up {
                    o
                } else {
                    o.reverse()
                }
            })
        });
        self.entries = all;
        self.path_text = self
            .dir
            .as_ref()
            .map_or_else(String::new, |d| d.display().to_string());
    }

    /// Goes to a folder (`None` is the list of drives); a path that is not a folder is refused.
    pub fn go(&mut self, dir: Option<PathBuf>) -> bool {
        if let Some(d) = &dir {
            if !d.is_dir() {
                return false;
            }
        }
        self.history.push(self.dir.clone());
        self.dir = dir;
        self.refresh();
        true
    }

    /// The folder above (on Windows, the top of a drive goes to the list of drives).
    pub fn up(&mut self) -> bool {
        let Some(d) = self.dir.clone() else {
            return false;
        };
        match d.parent() {
            Some(p) if !p.as_os_str().is_empty() => self.go(Some(p.to_path_buf())),
            _ if cfg!(windows) => self.go(None),
            _ => false,
        }
    }

    pub fn back(&mut self) -> bool {
        match self.history.pop() {
            Some(prev) => {
                self.dir = prev;
                self.refresh();
                true
            }
            None => false,
        }
    }

    pub fn go_place(&mut self, i: usize) -> bool {
        match self.places.get(i).map(|p| p.path.clone()) {
            Some(p) => self.go(Some(p)),
            None => false,
        }
    }

    /// Sorts by a column; the same column again turns the order round.
    pub fn sort_by(&mut self, col: usize) {
        self.sort = if self.sort.0 == col {
            (col, !self.sort.1)
        } else {
            (col, true)
        };
        self.refresh();
    }

    pub fn set_filter(&mut self, i: usize) {
        if i < self.filters.len() {
            self.filter = i;
            self.refresh();
        }
    }

    /// A click on a line: a file puts its name in the box of the name.
    pub fn select(&mut self, i: usize) {
        if let Some(e) = self.entries.get(i) {
            self.sel = Some(i);
            if !e.is_dir {
                self.name_text = e.name.clone();
            }
        }
    }

    /// A double click or Enter on a line: a folder is entered, a file is chosen.
    pub fn activate(&mut self, i: usize) -> Action {
        let Some(e) = self.entries.get(i).cloned() else {
            return Action::Nothing;
        };
        if e.is_dir {
            let target = match &self.dir {
                Some(d) => d.join(&e.name),
                None => PathBuf::from(&e.name),
            };
            return if self.go(Some(target)) {
                Action::Entered
            } else {
                Action::Nothing
            };
        }
        match &self.dir {
            Some(d) => Action::Chosen(d.join(&e.name)),
            None => Action::Nothing,
        }
    }

    /// The path typed above the list, taken as a folder (or as a file: its folder is entered and its name typed below).
    pub fn go_typed_path(&mut self) -> bool {
        let p = PathBuf::from(self.path_text.trim());
        if p.is_dir() {
            return self.go(Some(p));
        }
        if p.is_file() {
            if let (Some(parent), Some(name)) = (p.parent(), p.file_name()) {
                let name = name.to_string_lossy().to_string();
                if self.go(Some(parent.to_path_buf())) {
                    self.name_text = name;
                    return true;
                }
            }
        }
        // what could not be taken is not left in the box
        self.path_text = self
            .dir
            .as_ref()
            .map_or_else(String::new, |d| d.display().to_string());
        false
    }

    /// The Open or Save button: what the name typed below means.
    pub fn confirm(&self) -> Result<Chosen, Problem> {
        let name = self.name_text.trim();
        if name.is_empty() {
            return Err(Problem::NoName);
        }
        let Some(dir) = &self.dir else {
            return Err(Problem::NoFolder);
        };
        // a whole path typed in the box is taken as it is
        let typed = PathBuf::from(name);
        let mut path = if typed.is_absolute() {
            typed
        } else {
            dir.join(name)
        };
        if path.is_dir() {
            return Err(Problem::IsFolder(path));
        }
        match self.mode {
            Mode::Open => {
                if path.is_file() {
                    Ok(Chosen::File(path))
                } else {
                    Err(Problem::Missing(path))
                }
            }
            Mode::Save => {
                if let Some(ext) = &self.default_ext {
                    if path.extension().is_none() {
                        path.set_extension(ext);
                    }
                }
                if path.is_file() {
                    Ok(Chosen::Overwrite(path))
                } else {
                    Ok(Chosen::File(path))
                }
            }
        }
    }
}

fn ext_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// The places of the window: the examples next to the program (or the last folder), the folders of the user and the drives.
fn places(base: &Path, last: &Path) -> Vec<Place> {
    let mut v = Vec::new();
    let examples = base.join("examples");
    if examples.is_dir() {
        v.push(Place {
            kind: PlaceKind::Examples,
            name: String::new(),
            path: examples,
        });
    } else if last.is_dir() {
        v.push(Place {
            kind: PlaceKind::Examples,
            name: String::new(),
            path: last.to_path_buf(),
        });
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let home = PathBuf::from(home);
        for (kind, sub) in [
            (PlaceKind::Desktop, "Desktop"),
            (PlaceKind::Documents, "Documents"),
            (PlaceKind::Downloads, "Downloads"),
        ] {
            let p = home.join(sub);
            if p.is_dir() {
                v.push(Place {
                    kind,
                    name: String::new(),
                    path: p,
                });
            }
        }
    }
    for l in b'A'..=b'Z' {
        let d = format!("{}:\\", l as char);
        if Path::new(&d).exists() {
            v.push(Place {
                kind: PlaceKind::Drive,
                name: format!("{}:", l as char),
                path: PathBuf::from(d),
            });
        }
    }
    v
}

/// A date as Windows writes it, in universal time (the standard library has no time zones).
pub fn date_text(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // the civil date of a day count (the algorithm of Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60
    )
}

pub fn size_text(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn folder(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eh3d-filedialog-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("b.nc"), "G0 X1").unwrap();
        std::fs::write(d.join("a.txt"), "hello world").unwrap();
        std::fs::write(d.join("C.NC"), "G0").unwrap();
        d
    }

    fn browser(d: &Path, mode: Mode) -> Browser {
        Browser::new(
            mode,
            d,
            d,
            vec![Filter::new(&["nc", "gcode"]), Filter::any()],
            Some("nc"),
        )
    }

    #[test]
    fn the_list_has_the_folders_first_and_only_the_files_of_the_filter() {
        let d = folder("list");
        let mut b = browser(&d, Mode::Open);
        let names: Vec<_> = b.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["sub", "b.nc", "C.NC"]);
        b.set_filter(1);
        assert_eq!(b.entries.len(), 4);
        // the folders stay first when the order is turned round
        b.sort_by(0);
        assert_eq!(b.entries[0].name, "sub");
        assert_eq!(b.entries.last().unwrap().name, "a.txt");
        // by size: the smallest first, then the other way
        b.sort_by(3);
        assert_eq!(b.entries[1].name, "C.NC");
        b.sort_by(3);
        assert_eq!(b.entries[1].name, "a.txt");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_folder_is_entered_a_file_is_chosen_and_up_and_back_return() {
        let d = folder("walk");
        let mut b = browser(&d, Mode::Open);
        assert_eq!(b.activate(0), Action::Entered);
        assert_eq!(b.dir.as_deref(), Some(d.join("sub").as_path()));
        assert!(b.up());
        assert_eq!(b.dir.as_deref(), Some(d.as_path()));
        assert!(b.back());
        assert_eq!(b.dir.as_deref(), Some(d.join("sub").as_path()));
        assert!(b.back());
        assert!(!b.back(), "nothing before the start");
        assert_eq!(b.activate(1), Action::Chosen(d.join("b.nc")));
        // a click on a file puts its name below
        b.select(1);
        assert_eq!(b.name_text, "b.nc");
        assert!(!b.go(Some(d.join("nowhere"))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn open_wants_a_file_that_is_there() {
        let d = folder("open");
        let mut b = browser(&d, Mode::Open);
        assert_eq!(b.confirm(), Err(Problem::NoName));
        b.name_text = "b.nc".into();
        assert_eq!(b.confirm(), Ok(Chosen::File(d.join("b.nc"))));
        b.name_text = "none.nc".into();
        assert_eq!(b.confirm(), Err(Problem::Missing(d.join("none.nc"))));
        b.name_text = "sub".into();
        assert_eq!(b.confirm(), Err(Problem::IsFolder(d.join("sub"))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn save_gives_the_extension_and_asks_before_it_replaces() {
        let d = folder("save");
        let mut b = browser(&d, Mode::Save);
        b.name_text = "new".into();
        assert_eq!(b.confirm(), Ok(Chosen::File(d.join("new.nc"))));
        b.name_text = "new.gcode".into();
        assert_eq!(b.confirm(), Ok(Chosen::File(d.join("new.gcode"))));
        b.name_text = "b".into();
        assert_eq!(b.confirm(), Ok(Chosen::Overwrite(d.join("b.nc"))));
        b.name_text = "b.nc".into();
        assert_eq!(b.confirm(), Ok(Chosen::Overwrite(d.join("b.nc"))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_path_typed_above_the_list_is_gone_to_and_a_wrong_one_is_not_kept() {
        let d = folder("typed");
        let mut b = browser(&d, Mode::Open);
        b.path_text = d.join("sub").display().to_string();
        assert!(b.go_typed_path());
        assert_eq!(b.dir.as_deref(), Some(d.join("sub").as_path()));
        b.path_text = d.join("b.nc").display().to_string();
        assert!(b.go_typed_path());
        assert_eq!(
            (b.dir.as_deref(), b.name_text.as_str()),
            (Some(d.as_path()), "b.nc")
        );
        b.path_text = "/no/such/place".into();
        assert!(!b.go_typed_path());
        assert_eq!(b.path_text, d.display().to_string());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_places_start_with_the_examples_and_the_filters_read_the_extension() {
        let d = folder("places");
        let b = browser(&d, Mode::Open);
        assert_eq!(b.places[0].kind, PlaceKind::Examples);
        assert!(Filter::new(&["NC"]).shows("job.Nc"));
        assert!(!Filter::new(&["nc"]).shows("job"));
        assert!(Filter::any().shows("job"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn dates_and_sizes_are_written_as_windows_writes_them() {
        assert_eq!(
            date_text(UNIX_EPOCH + Duration::from_secs(0)),
            "1970-01-01 00:00"
        );
        // 2026-10-30 08:09 UTC
        assert_eq!(
            date_text(UNIX_EPOCH + Duration::from_secs(1_793_347_740)),
            "2026-10-30 08:09"
        );
        assert_eq!(size_text(900), "900 B");
        assert_eq!(size_text(2048), "2.0 KB");
        assert_eq!(size_text(3 * 1_048_576), "3.0 MB");
    }
}
