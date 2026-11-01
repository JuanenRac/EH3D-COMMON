//! Machine configuration (`DRILLPRO.DCF`, `ROUTEPRO.RCF`). Each data line is `value   * comment`;
//! a line whose first non-blank character is `*` followed by blank is a comment
//! (`*.NCD` is a value, not a comment).

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub serial_number: String,
    pub version: String,
    pub directory: String,
    pub default_extension: String,
    /// Remaining fields in file order (scale, offsets, speeds, ..., interface, baud selector,
    /// then the tool changer table when the machine has one).
    pub values: Vec<String>,
}

/// Removes the trailing `* comment`, which starts at a `*` preceded by whitespace.
fn strip_comment(line: &str) -> &str {
    let mut prev_blank = true;
    for (i, c) in line.char_indices() {
        if c == '*' && prev_blank {
            let next_blank = line[i + 1..].chars().next().is_none_or(char::is_whitespace);
            if next_blank {
                return &line[..i];
            }
        }
        prev_blank = c.is_whitespace();
    }
    line
}

pub fn parse(text: &str) -> Option<Config> {
    let data: Vec<String> = text
        .lines()
        .map(|l| strip_comment(l).trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let mut it = data.into_iter();
    Some(Config {
        serial_number: it.next()?,
        version: it.next()?,
        directory: it.next()?,
        default_extension: it.next()?,
        values: it.collect(),
    })
}

/// Unit of the coordinates of the drill or plot files (the `Coordinates unit` line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordUnit {
    TenthMil,
    Mil,
    /// 0.025 mm, the HP-GL plotter step.
    PlotterStep,
    HundredthMm,
}

impl CoordUnit {
    pub fn from_index(i: u32) -> Option<Self> {
        Some(match i {
            0 => Self::TenthMil,
            1 => Self::Mil,
            2 => Self::PlotterStep,
            3 => Self::HundredthMm,
            _ => return None,
        })
    }

    pub fn index(self) -> u32 {
        match self {
            Self::TenthMil => 0,
            Self::Mil => 1,
            Self::PlotterStep => 2,
            Self::HundredthMm => 3,
        }
    }

    pub fn mm(self) -> f64 {
        match self {
            Self::TenthMil => 0.00254,
            Self::Mil => 0.0254,
            Self::PlotterStep => 0.025,
            Self::HundredthMm => 0.01,
        }
    }
}

/// How the drill holes are ordered before drilling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    None,
    XFirst,
    YFirst,
    Special,
}

impl SortMode {
    pub fn index(self) -> u32 {
        match self {
            Self::None => 0,
            Self::XFirst => 1,
            Self::YFirst => 2,
            Self::Special => 3,
        }
    }
}

/// One position of the automatic tool changer (millimetres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtcSlot {
    pub index: u8,
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
}

/// What the tool changer needs to know about the tools and the bar that holds them, in the machine file as lines of a key and a value
/// (`RACK_TOOL_LENGTH 38.00`) after the places. A file without them gives the defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RackSetup {
    /// Length of a tool, in mm.
    pub tool_length_mm: f64,
    /// From the ring of the tool to its tip, in mm.
    pub ring_to_tip_mm: f64,
    /// How far above the tops of the tools the chuck travels from one place to another, in mm.
    pub clearance_mm: f64,
    /// How far from a place along X the chuck comes in from the free side of the bar, in mm (negative: from the lower X, towards the
    /// table; zero: straight down onto the place).
    pub approach_mm: f64,
    /// How far past the edge of the table along X the bar may be, in mm: the bar stands at the end of the machine, outside the work area.
    pub reach_x_mm: f64,
}

impl Default for RackSetup {
    fn default() -> Self {
        RackSetup {
            tool_length_mm: 38.0,
            ring_to_tip_mm: 21.0,
            clearance_mm: 8.0,
            approach_mm: -8.0,
            reach_x_mm: 150.0,
        }
    }
}

/// The keys of [`RackSetup`] in the file, in the order they are written.
pub const RACK_KEYS: [&str; 5] = [
    "RACK_TOOL_LENGTH",
    "RACK_RING_TO_TIP",
    "RACK_CLEARANCE",
    "RACK_APPROACH",
    "RACK_REACH_X",
];

impl RackSetup {
    fn get(&self, key: usize) -> f64 {
        [
            self.tool_length_mm,
            self.ring_to_tip_mm,
            self.clearance_mm,
            self.approach_mm,
            self.reach_x_mm,
        ][key]
    }

    fn set(&mut self, key: usize, v: f64) {
        match key {
            0 => self.tool_length_mm = v.max(1.0),
            1 => self.ring_to_tip_mm = v.max(0.0),
            2 => self.clearance_mm = v.max(0.0),
            3 => self.approach_mm = v,
            _ => self.reach_x_mm = v.max(0.0),
        }
    }

    /// The setup from the lines of the file that are of the form `KEY value` (the places and the other lines are not).
    pub fn from_lines<'a>(lines: impl IntoIterator<Item = &'a String>) -> RackSetup {
        let mut s = RackSetup::default();
        for l in lines {
            let mut it = l.split_whitespace();
            let (Some(k), Some(v)) = (it.next(), it.next()) else {
                continue;
            };
            if let (Some(i), Ok(v)) = (
                RACK_KEYS.iter().position(|n| n.eq_ignore_ascii_case(k)),
                v.parse::<f64>(),
            ) {
                if v.is_finite() {
                    s.set(i, v);
                }
            }
        }
        s
    }
}

/// The places of a bar of `places` tools in a line: place 1 is `first`, place `places` is `last`, the ones between are evenly spaced, and
/// place 0 (the test tool) and place `places + 1` (the drill break detector) go on at the same spacing on each side. One tool only: all at `first`.
pub fn rack_line(first: (f64, f64, f64), last: (f64, f64, f64), places: usize) -> Vec<AtcSlot> {
    let places = places.max(1);
    let n = (places - 1).max(1) as f64;
    let step = (
        (last.0 - first.0) / n,
        (last.1 - first.1) / n,
        (last.2 - first.2) / n,
    );
    let step = if places == 1 { (0.0, 0.0, 0.0) } else { step };
    (0..places + 2)
        .map(|i| {
            let k = i as f64 - 1.0;
            AtcSlot {
                index: i as u8,
                x_mm: first.0 + step.0 * k,
                y_mm: first.1 + step.1 * k,
                z_mm: first.2 + step.2 * k,
            }
        })
        .collect()
}

/// The first and the last tool of a rack (X, Y, Z in mm) and how many tools it holds.
pub type RackEnds = ((f64, f64, f64), (f64, f64, f64), usize);

/// The first and the last tool of a rack and how many tools it holds (the places without the test tool and the break detector), when the
/// rack has the places 0 to `tools + 1`; `None` otherwise.
pub fn rack_ends(atc: &[AtcSlot]) -> Option<RackEnds> {
    let tools = atc.len().checked_sub(2).filter(|&n| n >= 1)?;
    let at = |i: usize| {
        atc.iter()
            .find(|s| s.index as usize == i)
            .map(|s| (s.x_mm, s.y_mm, s.z_mm))
    };
    Some((at(1)?, at(tools)?, tools))
}

/// Typed view of the configuration file.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineSettings {
    pub serial_number: String,
    pub scale: (f64, f64),
    pub offset_mm: (f64, f64),
    /// Largest XY travel in mm.
    pub clip_mm: (f64, f64),
    pub board_mm: f64,
    pub base_mm: f64,
    /// Speeds in mm/min: (normal, reference, ramp start) for XY and for Z.
    pub speed_xy: (u32, u32, u32),
    pub speed_z: (u32, u32, u32),
    pub accel: (u32, u32),
    /// Calibration coefficients of X, Y and Z.
    pub correction: (f64, f64, f64),
    pub z_max_depth_mm: f64,
    pub distance_tool_board_mm: f64,
    pub spindle_start_time: f64,
    pub unit: CoordUnit,
    pub sorting: SortMode,
    /// Chord angle of the routing program in degrees (the line where the drilling program keeps its sorting); 0 for drilling.
    pub chord_angle: i32,
    /// Serial interface index: 0..3 = COM1..COM4, 4 = demo.
    pub interface: u8,
    /// Baud selector: 0 = 9600 via DOS, 1 = 19200, 2 = 38400.
    pub baud: u8,
    /// The automatic tool changer is on (line "Tool change 1=Auto, 0=Man." of the ATC models' files; the manual models do not have it).
    pub auto_change: bool,
    /// The rack: position 0 is the test pin, 1 to 14 the tools and 15 the drill break detector.
    pub atc: Vec<AtcSlot>,
    /// The tools and the bar of the changer (the lengths, the height to travel at and the side the chuck comes in from).
    pub rack: RackSetup,
}

impl Config {
    fn f(&self, i: usize) -> Option<f64> {
        self.values.get(i)?.split_whitespace().next()?.parse().ok()
    }

    fn u(&self, i: usize) -> Option<u32> {
        self.values.get(i)?.split_whitespace().next()?.parse().ok()
    }

    /// Parses the numeric fields. `None` when the file does not have the expected layout.
    pub fn settings(&self) -> Option<MachineSettings> {
        self.parse_settings(false)
    }

    /// The same for the routing program, whose line 23 is the HP-GL chord angle instead of the sorting.
    pub fn route_settings(&self) -> Option<MachineSettings> {
        self.parse_settings(true)
    }

    fn parse_settings(&self, route: bool) -> Option<MachineSettings> {
        let unit = CoordUnit::from_index(self.u(22)?)?;
        let (sorting, chord_angle) = if route {
            let angle: i32 = self
                .values
                .get(23)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()?;
            (SortMode::None, angle)
        } else {
            let sorting = match self.u(23)? {
                0 => SortMode::None,
                1 => SortMode::XFirst,
                2 => SortMode::YFirst,
                3 => SortMode::Special,
                _ => return None,
            };
            (sorting, 0)
        };
        let atc = self
            .values
            .iter()
            .skip(27)
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                Some(AtcSlot {
                    index: it.next()?.parse().ok()?,
                    x_mm: it.next()?.parse().ok()?,
                    y_mm: it.next()?.parse().ok()?,
                    z_mm: it.next()?.parse().ok()?,
                })
            })
            .collect();
        Some(MachineSettings {
            serial_number: self.serial_number.clone(),
            scale: (self.f(0)?, self.f(1)?),
            offset_mm: (self.f(2)?, self.f(3)?),
            clip_mm: (self.f(4)?, self.f(5)?),
            board_mm: self.f(6)?,
            base_mm: self.f(7)?,
            speed_xy: (self.u(8)?, self.u(9)?, self.u(10)?),
            speed_z: (self.u(11)?, self.u(12)?, self.u(13)?),
            accel: (self.u(14)?, self.u(15)?),
            correction: (self.f(16)?, self.f(17)?, self.f(18)?),
            z_max_depth_mm: self.f(19)?,
            distance_tool_board_mm: self.f(20)?,
            spindle_start_time: self.f(21)?,
            unit,
            sorting,
            chord_angle,
            interface: self.u(25)? as u8,
            baud: self.u(26)? as u8,
            auto_change: self.u(27) == Some(1),
            atc,
            rack: RackSetup::from_lines(self.values.iter().skip(27)),
        })
    }
}

/// Writes `settings` back into the text of a configuration file, keeping every comment, the blank lines, the reserved
/// lines and the layout: only the first value of each data line that the program edits is replaced (and padded to the
/// width of the old one, so the comments stay in their column). `route` selects the routing layout (chord angle on the
/// line where the drilling program keeps its sorting).
pub fn rewrite(
    original: &str,
    settings: &MachineSettings,
    directory: &str,
    pattern: &str,
    route: bool,
) -> String {
    let chord = if settings.chord_angle >= 0 {
        format!("{:02}", settings.chord_angle)
    } else {
        format!("{:03}", settings.chord_angle)
    };
    let value = |j: usize| -> Option<String> {
        let s = settings;
        Some(match j {
            0 => format!("{:.4}", s.scale.0),
            1 => format!("{:.4}", s.scale.1),
            2 => format!("{:05.2}", s.offset_mm.0),
            3 => format!("{:05.2}", s.offset_mm.1),
            4 => format!("{:06.2}", s.clip_mm.0),
            5 => format!("{:06.2}", s.clip_mm.1),
            6 => format!("{:.2}", s.board_mm),
            7 => format!("{:.2}", s.base_mm),
            8 => s.speed_xy.0.to_string(),
            9 => s.speed_xy.1.to_string(),
            10 => s.speed_xy.2.to_string(),
            11 => s.speed_z.0.to_string(),
            12 => s.speed_z.1.to_string(),
            13 => s.speed_z.2.to_string(),
            14 => s.accel.0.to_string(),
            15 => s.accel.1.to_string(),
            16 => format!("{:.4}", s.correction.0),
            17 => format!("{:.4}", s.correction.1),
            18 => format!("{:.4}", s.correction.2),
            19 => format!("{:.2}", s.z_max_depth_mm),
            20 => format!("{:.2}", s.distance_tool_board_mm),
            21 => format!("{:.2}", s.spindle_start_time),
            22 => s.unit.index().to_string(),
            23 => {
                if route {
                    chord.clone()
                } else {
                    s.sorting.index().to_string()
                }
            }
            // only the ATC models' files have this line; the rewrite only replaces lines that exist
            27 => if s.auto_change { "1" } else { "0" }.to_string(),
            _ => return None,
        })
    };
    let eol = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    // the places of the changer and the lines of its setup are written again only when they changed
    let before = if route {
        parse(original).and_then(|c| c.route_settings())
    } else {
        parse(original).and_then(|c| c.settings())
    };
    let rack_block = !route
        && (settings.auto_change || !settings.atc.is_empty())
        && before
            .as_ref()
            .is_none_or(|b| b.atc != settings.atc || b.rack != settings.rack);
    let block = |out: &mut String| {
        for s in &settings.atc {
            out.push_str(&format!(
                "{:<3} {:>8.2} {:>8.2} {:>8.2}   * Place {}{eol}",
                s.index, s.x_mm, s.y_mm, s.z_mm, s.index
            ));
        }
        if !settings.atc.is_empty() {
            for (i, key) in RACK_KEYS.iter().enumerate() {
                out.push_str(&format!(
                    "{key} {:.2}   * Setup of the tool bar{eol}",
                    settings.rack.get(i)
                ));
            }
        }
    };
    let mut out = String::with_capacity(original.len() + 64);
    let mut data_index = 0usize;
    let mut block_done = false;
    for line in original.split_inclusive('\n') {
        let is_data = !strip_comment(line).trim().is_empty();
        if !is_data {
            out.push_str(line);
            continue;
        }
        if rack_block && data_index >= 32 {
            let data = strip_comment(line);
            let first = data.split_whitespace().next().unwrap_or("");
            let numeric = data.split_whitespace().count() >= 4
                && data
                    .split_whitespace()
                    .take(4)
                    .all(|t| t.parse::<f64>().is_ok());
            if numeric || RACK_KEYS.iter().any(|k| k.eq_ignore_ascii_case(first)) {
                // a place or a line of the setup: they are all written where the first one was
                data_index += 1;
                if !block_done {
                    block(&mut out);
                    block_done = true;
                }
                continue;
            }
        }
        let new = match data_index {
            2 => Some(directory.to_string()),
            3 => Some(pattern.to_string()),
            k if k >= 4 => value(k - 4),
            _ => None,
        };
        data_index += 1;
        let Some(new) = new else {
            out.push_str(line);
            continue;
        };
        let start = line.len() - line.trim_start().len();
        let token_len = line[start..]
            .find(char::is_whitespace)
            .unwrap_or(line.len() - start);
        let pad = " ".repeat(token_len.saturating_sub(new.len()));
        out.push_str(&line[..start]);
        out.push_str(&new);
        out.push_str(&pad);
        out.push_str(&line[start + token_len..]);
    }
    if rack_block && !block_done {
        // a file of the manual layout (or one with no places yet): the line of the changer, then the places
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(eol);
        }
        if data_index < 32 {
            out.push_str(&format!(
                "{}   * Tool change 1=Auto, 0=Man.{eol}",
                u8::from(settings.auto_change)
            ));
        }
        block(&mut out);
    }
    out
}

/// The folder the files of the jobs live in: the `Directory` line of the configuration (DOS style, `.\RACK3000`)
/// under `data_dir`, when that folder exists; `data_dir` itself otherwise.
pub fn work_dir(data_dir: &std::path::Path, cfg: Option<&Config>) -> std::path::PathBuf {
    let Some(c) = cfg else {
        return data_dir.to_path_buf();
    };
    let rel = c.directory.trim().replace('\\', "/");
    let rel = rel.trim_start_matches("./").trim_start_matches('/');
    if rel.is_empty() || rel == "." {
        return data_dir.to_path_buf();
    }
    let dir = data_dir.join(rel);
    if dir.is_dir() {
        dir
    } else {
        data_dir.to_path_buf()
    }
}

/// Bytes of a file read as text (the files are DOS text: one character per byte).
pub fn bytes_to_text(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// The text written back as bytes (characters above 255 become `?`).
pub fn text_to_bytes(t: &str) -> Vec<u8> {
    t.chars()
        .map(|c| u8::try_from(c as u32).unwrap_or(b'?'))
        .collect()
}

/// Writes `text` to `path`, keeping the old file as `<name>.BAK` next to it.
pub fn write_with_backup(path: &std::path::Path, text: &str) -> Result<(), String> {
    // the files are DOS text, one byte for each character: a character the file cannot hold is refused instead of becoming a question mark
    if let Some(c) = text.chars().find(|&c| u32::from(c) > 255) {
        return Err(format!(
            "the character '{c}' cannot be written in the file {}: it is a DOS text file, one byte for each character",
            path.display()
        ));
    }
    if path.exists() {
        let mut bak = path.as_os_str().to_owned();
        bak.push(".BAK");
        std::fs::copy(path, std::path::PathBuf::from(bak)).map_err(|e| e.to_string())?;
    }
    // through a temporary file and a rename, so that a cut in the middle never leaves the file half written
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".TMP");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, text_to_bytes(text)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// Where the program looks for its files when no folder is given: the current folder, the folder of the program, and
/// the user's configuration folder (`%APPDATA%\EH3D`, `~/.config/eh3d`); the first one that holds `file` (for example
/// `DRILLPRO.DCF`) wins, and the current folder is the answer when none does.
pub fn find_data_dir(file: &str) -> std::path::PathBuf {
    let mut candidates = vec![std::path::PathBuf::from(".")];
    if let Some(exe) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
    {
        candidates.push(exe);
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        candidates.push(std::path::PathBuf::from(appdata).join("EH3D"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(std::path::PathBuf::from(home).join(".config").join("eh3d"));
    }
    candidates
        .into_iter()
        .find(|d| d.join(file).is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

#[cfg(test)]
mod write_tests {
    use super::*;

    #[test]
    fn a_text_is_written_through_a_temporary_file_and_a_character_that_does_not_fit_is_refused() {
        let dir = std::env::temp_dir().join(format!("eh3d-config-write-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("MACHINE.DCF");
        write_with_backup(&file, "one\r\n").unwrap();
        write_with_backup(&file, "two\r\n").unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"two\r\n");
        assert_eq!(
            std::fs::read(dir.join("MACHINE.DCF.BAK")).unwrap(),
            b"one\r\n"
        );
        assert!(!dir.join("MACHINE.DCF.TMP").exists());
        assert!(write_with_backup(&file, "Łukasz").is_err());
        assert_eq!(
            std::fs::read(&file).unwrap(),
            b"two\r\n",
            "the file is not touched when the text is refused"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod rack_tests {
    use super::*;

    fn shipped() -> String {
        // serial, version, directory, pattern, then the 27 values
        let mut t =
            String::from("0000 * serial\n1.00 * version\n./JOBS * directory\n*.NCD * pattern\n");
        for v in [
            "1.0000", "1.0000", "00.00", "00.00", "300.00", "250.00", "1.60", "6.00", "1000",
            "500", "100", "300", "200", "100", "10", "20", "1.0000", "1.0000", "1.0000", "37.50",
            "3.00", "2.00", "1", "3", "0", "4", "2",
        ] {
            t.push_str(&format!("{v} * value\n"));
        }
        t
    }

    #[test]
    fn a_line_of_places_goes_from_the_first_to_the_last_with_the_test_tool_before_and_the_detector_after(
    ) {
        let line = rack_line((540.0, 20.0, 37.4), (540.0, 300.0, 37.4), 15);
        assert_eq!(line.len(), 17);
        assert_eq!(line[1].y_mm, 20.0);
        assert!((line[15].y_mm - 300.0).abs() < 1e-9);
        assert!(
            (line[0].y_mm - 0.0).abs() < 1e-9,
            "the test tool is one step before the first"
        );
        assert!(
            (line[16].y_mm - 320.0).abs() < 1e-9,
            "the detector is one step after the last"
        );
        let step = line[2].y_mm - line[1].y_mm;
        assert!(line
            .windows(2)
            .all(|w| (w[1].y_mm - w[0].y_mm - step).abs() < 1e-9));
        assert!(line.iter().enumerate().all(|(i, s)| s.index as usize == i));
        let (first, last, n) = rack_ends(&line).unwrap();
        assert_eq!((first.1, last.1, n), (20.0, 300.0, 15));
        // one tool only: every place at the same point
        let one = rack_line((1.0, 2.0, 3.0), (9.0, 9.0, 9.0), 1);
        assert!(one
            .iter()
            .all(|s| (s.x_mm, s.y_mm, s.z_mm) == (1.0, 2.0, 3.0)));
    }

    #[test]
    fn the_places_and_the_setup_are_written_in_the_file_and_read_back() {
        let text = shipped();
        let mut s = parse(&text).unwrap().settings().unwrap();
        assert!(!s.auto_change && s.atc.is_empty());
        assert_eq!(s.rack, RackSetup::default());
        // nothing is added to a file of the manual layout while the changer is off
        assert_eq!(rewrite(&text, &s, "./JOBS", "*.NCD", false), text);
        s.auto_change = true;
        s.atc = rack_line((540.0, 20.0, 37.4), (540.0, 160.0, 37.4), 8);
        s.rack.clearance_mm = 12.5;
        s.rack.approach_mm = 6.0;
        let out = rewrite(&text, &s, "./JOBS", "*.NCD", false);
        let back = parse(&out).unwrap().settings().unwrap();
        assert!(back.auto_change);
        assert_eq!(back.atc.len(), 10);
        assert_eq!(
            back.atc,
            s.atc
                .iter()
                .map(|a| AtcSlot {
                    index: a.index,
                    x_mm: (a.x_mm * 100.0).round() / 100.0,
                    y_mm: (a.y_mm * 100.0).round() / 100.0,
                    z_mm: (a.z_mm * 100.0).round() / 100.0
                })
                .collect::<Vec<_>>()
        );
        assert_eq!((back.rack.clearance_mm, back.rack.approach_mm), (12.5, 6.0));
        assert_eq!(back.rack.tool_length_mm, 38.0);
        // the other lines are as they were
        assert_eq!(back.clip_mm, s.clip_mm);
        // writing the same again changes nothing, and a change of one place changes that place only
        assert_eq!(rewrite(&out, &back, "./JOBS", "*.NCD", false), out);
        let mut moved = back.clone();
        moved.atc[3].y_mm += 1.0;
        let out2 = rewrite(&out, &moved, "./JOBS", "*.NCD", false);
        let again = parse(&out2).unwrap().settings().unwrap();
        assert_eq!(again.atc[3].y_mm, moved.atc[3].y_mm);
        assert_eq!(again.atc.len(), 10);
        assert_eq!(out2.matches("RACK_CLEARANCE").count(), 1);
    }

    #[test]
    fn a_file_with_the_places_but_no_setup_gets_the_defaults_and_keeps_its_comments() {
        let mut t = shipped();
        t.push_str("1 * Tool change 1=Auto, 0=Man.\n");
        t.push_str("0 540.0 20.0 37.4 * test tool\n1 530.0 40.0 37.4 * first\n2 540.0 60.0 37.4 * last\n3 530.0 80.0 37.4\n");
        let s = parse(&t).unwrap().settings().unwrap();
        assert_eq!(s.atc.len(), 4);
        assert_eq!(s.rack, RackSetup::default());
        // unchanged: the text is exactly the same, comments and all
        assert_eq!(rewrite(&t, &s, "./JOBS", "*.NCD", false), t);
    }
}
