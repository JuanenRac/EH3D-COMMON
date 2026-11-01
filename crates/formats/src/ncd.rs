//! Excellon drill files (`.NCD`): imperial, integer coordinates in mils.
//!
//! Real-world files come in two shapes: a bare body (`%`, `T06`, `X..Y..`) and
//! an Excellon header (`M48 ... %`) with optional tool definitions (`T1C0.8`).

use std::fmt;

/// One mil is 0.001 inch.
pub const MIL_MM: f64 = 0.0254;

#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    pub tool: u8,
    pub x_mil: i32,
    pub y_mil: i32,
}

impl Hole {
    pub fn x_mm(&self) -> f64 {
        self.x_mil as f64 * MIL_MM
    }
    pub fn y_mm(&self) -> f64 {
        self.y_mil as f64 * MIL_MM
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct DrillFile {
    pub holes: Vec<Hole>,
    /// Tools in order of first use.
    pub tools: Vec<u8>,
    /// Diameters declared in the header (`T1C0.8`), as `(tool, value)` in the file's unit.
    pub diameters: Vec<(u8, f64)>,
    /// True when the header declares metric units.
    pub metric: bool,
    /// The lines that move the head in ways a drill file of holes does not hold (a slot written as `G00`/`G01`, `M15`/`M16` up and down) and are not part of the holes:
    /// they are named, not dropped without a word.
    pub ignored: Vec<Ignored>,
}

/// A line of the file that is not a hole and was left out: its number (from 1) and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct Ignored {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    BadLine { line: usize, text: String },
    HoleBeforeTool { line: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::BadLine { line, text } => write!(f, "line {line}: cannot parse {text:?}"),
            ParseError::HoleBeforeTool { line } => {
                write!(f, "line {line}: coordinate before any tool")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// How the coordinates of a file with a header are written: the unit, the digits and which zeros are left out.
#[derive(Debug, Clone, Copy, Default)]
struct Format {
    /// `Some(true)` for metric, `Some(false)` for inch; `None` when the header does not say: the numbers are then mils, as in the files of the machine.
    metric: Option<bool>,
    /// Digits before and after the point; `None` for the usual one of the unit (2.4 inch, 3.3 metric).
    digits: Option<(u32, u32)>,
    /// `LZ`: the leading zeros are written and the trailing ones left out; `TZ` (and the default): the other way.
    trailing_suppressed: bool,
    /// An `INCH` (or `M72`) header with nothing else: the files of the machine, and the ones this suite writes, say it and then give whole mils.
    plain_inch: bool,
}

impl Format {
    /// A coordinate in the unit of the file as a whole number of mils.
    fn mils(&self, text: &str) -> Option<i32> {
        let Some(metric) = self.metric else {
            return text.parse().ok();
        };
        let (negative, body) = match text.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if body.is_empty() || !body.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return None;
        }
        let value: f64 = if body.contains('.') {
            body.parse().ok()?
        } else {
            let (i, d) = self.digits.unwrap_or(if metric { (3, 3) } else { (2, 4) });
            let mut digits = body.to_string();
            if self.trailing_suppressed {
                while (digits.len() as u32) < i + d {
                    digits.push('0');
                }
            }
            digits.parse::<f64>().ok()? / 10f64.powi(d as i32)
        };
        let mils = if metric {
            value / MIL_MM
        } else {
            value * 1000.0
        };
        let mils = if negative { -mils } else { mils }.round();
        (mils.abs() < i32::MAX as f64).then_some(mils as i32)
    }

    /// Reads the options of a header line such as `INCH,LZ`, `METRIC,TZ,000.000` or `;FILE_FORMAT=2:4`.
    fn header(&mut self, line: &str) {
        if let Some(f) = line.strip_prefix(";FILE_FORMAT=") {
            if let Some((i, d)) = f.trim().split_once(':') {
                if let (Ok(i), Ok(d)) = (i.parse(), d.parse()) {
                    self.digits = Some((i, d));
                    if self.plain_inch {
                        // a format given in numbers: the digits of the file, not mils
                        self.metric = Some(false);
                    }
                }
            }
            return;
        }
        let mut parts = line.split(',');
        match parts.next().map(str::trim) {
            Some("METRIC") => self.metric = Some(true),
            Some("INCH") => {
                self.metric = Some(false);
                self.plain_inch = true;
            }
            Some("M71") => self.metric = Some(true),
            Some("M72") => {
                self.metric = Some(false);
                self.plain_inch = true;
            }
            _ => return,
        }
        let mut options = false;
        for part in parts {
            options = true;
            match part.trim() {
                "LZ" => self.trailing_suppressed = true,
                "TZ" => self.trailing_suppressed = false,
                other => {
                    if let Some((i, d)) = other.split_once('.') {
                        if other.chars().all(|c| c.is_ascii_digit() || c == '.') {
                            self.digits = Some((i.len() as u32, d.len() as u32));
                        }
                    }
                }
            }
        }
        if options {
            self.plain_inch = false;
        }
        if self.plain_inch {
            // no options: whole mils
            self.metric = None;
        }
    }
}

/// Parses `X..Y..` where either axis may be omitted (modal, keeps the previous value).
fn parse_coord(s: &str, fmt: &Format, x: &mut i32, y: &mut i32) -> Option<()> {
    let mut rest = s;
    let mut seen = false;
    while let Some(c) = rest.chars().next() {
        if c != 'X' && c != 'Y' {
            return None;
        }
        let body = &rest[1..];
        let end = body
            .find(|d: char| !(d.is_ascii_digit() || d == '-' || d == '+' || d == '.'))
            .unwrap_or(body.len());
        let v: i32 = fmt.mils(&body[..end])?;
        if c == 'X' {
            *x = v;
        } else {
            *y = v;
        }
        seen = true;
        rest = &body[end..];
    }
    seen.then_some(())
}

/// Whether a first line is the start of an Excellon file and not the free text of an old teach-in file.
fn looks_like_excellon(l: &str) -> bool {
    let mut c = l.chars();
    match c.next() {
        Some('T') => c.next().is_some_and(|d| d.is_ascii_digit()),
        Some('X') | Some('Y') => c
            .next()
            .is_some_and(|d| d.is_ascii_digit() || d == '-' || d == '+' || d == '.'),
        Some(';') => true,
        Some('M') => l == "M71" || l == "M72",
        _ => l.starts_with("METRIC") || l.starts_with("INCH"),
    }
}

pub fn parse(text: &str) -> Result<DrillFile, ParseError> {
    let mut out = DrillFile::default();
    let (mut tool, mut x, mut y) = (None::<u8>, 0, 0);
    let mut in_header = false;
    let mut fmt = Format::default();
    // Old teach-in files start with free text (title, unit note) that ends at the first `%`.
    // (a file whose first line is already Excellon - a tool, a coordinate, a comment, a unit - has no such text: a `%` at its end is not the end of a title)
    let free_text = text.lines().next().is_some_and(|l| {
        let l = l.trim();
        l != "M48" && !l.starts_with('%') && !looks_like_excellon(l)
    }) && text.lines().any(|l| l.trim().starts_with('%'))
        && !text.lines().any(|l| l.trim() == "M48");
    let mut skipping = free_text;
    for (n, raw) in text.lines().enumerate() {
        if skipping {
            skipping = !raw.trim().starts_with('%');
            continue;
        }
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let bad = || ParseError::BadLine {
            line: n + 1,
            text: raw.into(),
        };
        if line == "M48" {
            in_header = true;
            continue;
        }
        if line.starts_with('%') {
            in_header = false;
            continue;
        }
        if line.starts_with("METRIC")
            || line.starts_with("INCH")
            || line == "M71"
            || line == "M72"
            || line.starts_with(";FILE_FORMAT=")
        {
            // the unit, the digits and the zeros of the coordinates that follow
            fmt.header(line);
            if line.starts_with("METRIC") || line == "M71" {
                out.metric = true;
            }
            continue;
        }
        if line.starts_with('M') {
            // M30 end of program; M15/M16 put the tool down and up for a routed slot, which this format of holes does not hold: said, not lost
            if line == "M15" || line == "M16" {
                out.ignored.push(Ignored {
                    line: n + 1,
                    text: raw.trim().into(),
                });
            }
            continue;
        }
        if in_header {
            if let Some(def) = line.strip_prefix('T') {
                if let Some((num, dia)) = def.split_once('C') {
                    let t: u8 = num.parse().map_err(|_| bad())?;
                    // Trailing feed/speed words (F.., S..) may follow the diameter.
                    let end = dia
                        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                        .unwrap_or(dia.len());
                    out.diameters
                        .push((t, dia[..end].parse().map_err(|_| bad())?));
                }
            }
            continue;
        }
        if line.starts_with('G') || line.starts_with("INCH") || line.starts_with("METRIC") {
            // `G05` (drill mode) and `G90` (absolute) say nothing to do; a `G00`/`G01`/`G85` with a position is a slot, which is not read
            if line.len() > 3 && line[1..].chars().any(|c| c == 'X' || c == 'Y') {
                out.ignored.push(Ignored {
                    line: n + 1,
                    text: raw.trim().into(),
                });
            }
            continue;
        }
        if let Some(num) = line.strip_prefix('T') {
            let t: u8 = num.parse().map_err(|_| bad())?;
            if t == 0 {
                // T0: the tool is put away (KiCad and Altium write it before the end); it is no tool of the job
                tool = None;
                continue;
            }
            if !out.tools.contains(&t) {
                out.tools.push(t);
            }
            tool = Some(t);
            continue;
        }
        let t = tool.ok_or(ParseError::HoleBeforeTool { line: n + 1 })?;
        parse_coord(line, &fmt, &mut x, &mut y).ok_or_else(bad)?;
        out.holes.push(Hole {
            tool: t,
            x_mil: x,
            y_mil: y,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_with_inches_and_four_decimals_gives_the_right_position() {
        // TZ: the leading zeros are left out, 5850 is 0.5850 inch = 585 mils
        let f = parse(
            "M48
INCH,TZ
;FILE_FORMAT=2:4
T1C0.0315
%
T1
X5850Y10000
M30
",
        )
        .unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (585, 1000));
        assert!((f.holes[0].x_mm() - 14.859).abs() < 1e-6);
    }

    #[test]
    fn leading_zeros_present_means_the_trailing_ones_are_left_out() {
        // LZ with 2.4: 05 is 05.0000 inch
        let f = parse(
            "M48
INCH,LZ
T1C0.0315
%
T1
X05Y1
M30
",
        )
        .unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (5000, 10000));
    }

    #[test]
    fn metric_with_trailing_zeros_present() {
        let f = parse(
            "M48
METRIC,TZ,000.000
T1C0.8
%
T1
X12700Y0
M30
",
        )
        .unwrap();
        assert!(
            (f.holes[0].x_mm() - 12.7).abs() < 0.02,
            "{}",
            f.holes[0].x_mm()
        );
    }

    #[test]
    fn decimal_coordinates_and_t0() {
        let f = parse(
            "M48
METRIC
T1C0.8
%
T1
X12.7Y-25.4
T0
M30
",
        )
        .unwrap();
        assert!((f.holes[0].x_mm() - 12.7).abs() < 0.02);
        assert!((f.holes[0].y_mm() + 25.4).abs() < 0.02);
        assert_eq!(f.tools, vec![1]);
    }

    #[test]
    fn a_metric_file_is_read_in_millimetres_so_the_holes_are_where_they_were_drawn() {
        // 3.3 metric: X12345 is 12.345 mm, not 12345 mils (313.6 mm)
        let f = parse(
            "M48
METRIC
T1C0.8
%
T1
X12345Y20000
M30
",
        )
        .unwrap();
        assert!(
            (f.holes[0].x_mm() - 12.345).abs() < 0.02,
            "{}",
            f.holes[0].x_mm()
        );
        assert!((f.holes[0].y_mm() - 20.0).abs() < 0.02);
        assert!(f.metric);
    }

    #[test]
    fn slots_are_named_and_a_body_with_a_percent_at_its_end_keeps_its_holes() {
        let f = parse(
            "M48
INCH
T1C0.0315
%
T1
X1000Y1000
G00X2000Y2000
M15
G01X3000Y2000
M16
G05
M30
",
        )
        .unwrap();
        assert_eq!(f.holes.len(), 1);
        assert_eq!(
            f.ignored.iter().map(|i| i.line).collect::<Vec<_>>(),
            vec![7, 8, 9, 10]
        );
        assert_eq!(f.ignored[0].text, "G00X2000Y2000");
        // a file with no header whose first line is a tool is a file of holes, though a `%` closes it
        let g = parse(
            "T01
X1000Y1000
X2000Y2000
M30
%
",
        )
        .unwrap();
        assert_eq!(g.holes.len(), 2);
        // an old teach-in file with a title still has it skipped
        let h = parse(
            "TEACH IN FILE
%
T01
X1000Y1000
M30
",
        )
        .unwrap();
        assert_eq!(h.holes.len(), 1);
    }

    #[test]
    fn a_plain_inch_header_keeps_whole_mils_as_the_machine_files_have_them() {
        let f = parse("M48\nINCH\nT1C0.0315\n%\nT1\nX1000Y2000\nM30\n").unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (1000, 2000));
    }

    #[test]
    fn a_file_without_a_unit_keeps_its_mils() {
        let f = parse(
            "%
T06
X1000Y2000
",
        )
        .unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (1000, 2000));
    }
}
