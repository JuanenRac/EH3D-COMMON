//! The board outline of a Gerber file (RS-274X), turned into the routing paths of a plot: the lines (`G01`) and the arcs (`G02` and
//! `G03`, with the centre given as `I` and `J`) drawn with the pen down (`D01`) between the moves (`D02`). The apertures, the polarity
//! and the flashes (`D03`) mean nothing for a route and are skipped. One path is made of the strokes that follow each other.

use crate::hpgl::{arc_points, Plot, Polyline, SEGMENTS_PER_CIRCLE, UNIT_MM};

/// The file extensions a Gerber outline usually has (the profile or mechanical layer of the usual board programs).
pub const OUTLINE_EXTENSIONS: [&str; 6] = ["gbr", "gko", "gm1", "gml", "gm2", "gm3"];

/// Whether a file is the outline of a board and not a layer of copper or of silk: the extensions made for it always are, and the generic
/// `.gbr` is when its attributes or its comments call it the profile or the edge of the board.
pub fn looks_like_outline(name: &str, text: &str) -> bool {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if ext != "gbr" {
        return OUTLINE_EXTENSIONS.contains(&ext.as_str());
    }
    let head: String = text
        .chars()
        .take(4000)
        .collect::<String>()
        .to_ascii_lowercase();
    ["profile", "edge", "outline"]
        .iter()
        .any(|k| head.contains(k))
}

/// What the format statement says: the digits of the integer and the decimal part of a coordinate.
#[derive(Debug, Clone, Copy)]
struct Format {
    integer: usize,
    decimal: usize,
    /// The zeros that are left out: the leading ones (`L`, the usual) or the trailing ones (`T`).
    trailing_omitted: bool,
}

/// Reads a coordinate (`-1234`) as the number it stands for.
fn coordinate(digits: &str, f: Format) -> Option<f64> {
    let (neg, d) = match digits.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, digits.strip_prefix('+').unwrap_or(digits)),
    };
    if d.is_empty() || !d.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let width = f.integer + f.decimal;
    let padded;
    let d = if f.trailing_omitted && d.len() < width {
        padded = format!("{d:0<width$}");
        padded.as_str()
    } else {
        d
    };
    let v = d.parse::<f64>().ok()? / 10f64.powi(f.decimal as i32);
    Some(if neg { -v } else { v })
}

/// The number that follows the letter `axis` in a block (`X1000Y-20D01`), if there is one.
fn word(block: &str, axis: char) -> Option<&str> {
    let i = block.find(axis)?;
    let rest = &block[i + 1..];
    let end = rest
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// The outline of the board: every closed or open run of strokes as a path of the plot, in plotter units. `segments_per_circle` is how
/// many chords a whole circle is cut into. An error says what in the file cannot be read.
pub fn outline(text: &str, segments_per_circle: u32) -> Result<Plot, String> {
    let chord = 360.0 / f64::from(segments_per_circle.clamp(4, 3600));
    let mut format = Format {
        integer: 2,
        decimal: 4,
        trailing_omitted: false,
    };
    let mut mm_per_unit = 25.4;
    let mut saw_format = false;
    let (mut x, mut y) = (0.0f64, 0.0f64);
    let mut mode = 1u8; // 1 line, 2 clockwise arc, 3 counterclockwise arc
    let mut run: Vec<(f64, f64)> = Vec::new();
    let mut runs: Vec<Vec<(f64, f64)>> = Vec::new();

    // the statements end at `*`; the ones between two `%` are parameters
    let mut statements: Vec<(bool, String)> = Vec::new();
    let (mut param, mut cur) = (false, String::new());
    for c in text.chars() {
        match c {
            '%' => {
                if !cur.is_empty() {
                    statements.push((param, std::mem::take(&mut cur)));
                }
                param = !param;
            }
            '*' => statements.push((param, std::mem::take(&mut cur))),
            c if c.is_whitespace() => {}
            c => cur.push(c),
        }
    }
    for (is_param, stmt) in &statements {
        let stmt = stmt.as_str();
        if stmt.is_empty() {
            continue;
        }
        if *is_param {
            if stmt.starts_with("FS") {
                // FSLAX24Y24: the zeros that are left out, the absolute coordinates and the digits of X
                let t = stmt.trim_start_matches("FS");
                let trailing = t.starts_with('T');
                let digits = t.split('X').nth(1).unwrap_or("");
                let mut d = digits.chars().filter(|c| c.is_ascii_digit());
                if let (Some(i), Some(f)) = (d.next(), d.next()) {
                    format = Format {
                        integer: i.to_digit(10).unwrap_or(2) as usize,
                        decimal: f.to_digit(10).unwrap_or(4) as usize,
                        trailing_omitted: trailing,
                    };
                    saw_format = true;
                }
            } else if stmt.starts_with("MOMM") {
                mm_per_unit = 1.0;
            } else if stmt.starts_with("MOIN") {
                mm_per_unit = 25.4;
            }
            continue;
        }

        let block = stmt.to_ascii_uppercase();
        if block.starts_with("G04") || block.starts_with("M02") {
            continue;
        }
        for (code, m) in [("G01", 1u8), ("G02", 2), ("G03", 3)] {
            if block.contains(code) {
                mode = m;
            }
        }
        let has_move = ["X", "Y", "I", "J"].iter().any(|a| block.contains(a));
        let op = ["D01", "D02", "D03", "D1", "D2", "D3"]
            .iter()
            .find(|d| block.ends_with(**d))
            .map(|d| d.trim_start_matches('D').trim_start_matches('0'));
        if !has_move && op.is_none() {
            continue;
        }
        let read = |axis: char, now: f64| -> Result<f64, String> {
            match word(&block, axis) {
                Some(w) => coordinate(w, format)
                    .map(|v| v * mm_per_unit)
                    .ok_or_else(|| format!("{axis}{w}")),
                None => Ok(now),
            }
        };
        let (nx, ny) = (read('X', x)?, read('Y', y)?);
        match op {
            Some("2") => {
                if run.len() > 1 {
                    runs.push(std::mem::take(&mut run));
                } else {
                    run.clear();
                }
                run.push((nx, ny));
            }
            Some("1") => {
                if run.is_empty() {
                    run.push((x, y));
                }
                if mode == 1 {
                    run.push((nx, ny));
                } else {
                    let (i, j) = (read('I', 0.0)?, read('J', 0.0)?);
                    // a signed centre, `G75`: the arc goes the way the mode says, a whole circle when it ends where it began
                    let center = (x + i, y + j);
                    let a0 = (y - center.1).atan2(x - center.0);
                    let a1 = (ny - center.1).atan2(nx - center.0);
                    let tau = std::f64::consts::TAU;
                    let mut sweep = (a1 - a0).rem_euclid(tau);
                    if mode == 2 {
                        sweep = if sweep == 0.0 { -tau } else { sweep - tau };
                    } else if sweep == 0.0 {
                        sweep = tau;
                    }
                    let mut pts = arc_points((x, y), center, sweep.to_degrees(), chord);
                    if let Some(last) = pts.last_mut() {
                        *last = (nx, ny);
                    }
                    run.extend(pts);
                }
            }
            _ => {}
        }
        x = nx;
        y = ny;
    }
    if run.len() > 1 {
        runs.push(run);
    }
    if !saw_format {
        return Err("no format statement (FS)".to_string());
    }
    if runs.is_empty() {
        return Err("no outline in the file".to_string());
    }
    let to_units = |v: f64| (v / UNIT_MM).round() as i32;
    Ok(Plot {
        paths: runs
            .into_iter()
            .map(|r| Polyline {
                pen: 1,
                points: r
                    .into_iter()
                    .map(|(a, b)| (to_units(a), to_units(b)))
                    .collect(),
            })
            .collect(),
        ignored: 0,
    })
}

/// [`outline`] with the usual number of chords of a circle.
pub fn outline_default(text: &str) -> Result<Plot, String> {
    outline(text, SEGMENTS_PER_CIRCLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG01*\nX0Y0D02*\nX500000Y0D01*\nX500000Y300000D01*\nX0Y300000D01*\nX0Y0D01*\nM02*\n";

    #[test]
    fn a_square_outline_is_one_closed_path_in_plotter_units() {
        let p = outline_default(SQUARE).unwrap();
        assert_eq!(p.paths.len(), 1);
        // 50 mm and 30 mm at 0.025 mm a unit
        assert_eq!(
            p.paths[0].points,
            vec![(0, 0), (2000, 0), (2000, 1200), (0, 1200), (0, 0)]
        );
        assert_eq!(p.bounds_mm(), Some((0.0, 0.0, 50.0, 30.0)));
    }

    /// A square from the origin of this side, to the thousandth of a millimetre.
    fn near(b: (f64, f64, f64, f64), side: f64) -> bool {
        b.0.abs() < 1e-9
            && b.1.abs() < 1e-9
            && (b.2 - side).abs() < 0.001
            && (b.3 - side).abs() < 0.001
    }

    #[test]
    fn inches_and_the_zeros_that_are_left_out_are_read() {
        // 1 inch square, 2.4 format with the leading zeros left out (`L`): 10000 is 1.0000
        let g =
            "%FSLAX24Y24*%\n%MOIN*%\nG01*\nX0Y0D02*\nX10000D01*\nY10000D01*\nX0D01*\nY0D01*\nM02*";
        let p = outline_default(g).unwrap();
        assert!(near(p.bounds_mm().unwrap(), 25.4));
        // the same with the trailing zeros left out (`T`): 01 is 1.0000
        let t = "%FSTAX24Y24*%\n%MOIN*%\nX0Y0D02*\nX01D01*\nY01D01*\nM02*";
        assert!(near(outline_default(t).unwrap().bounds_mm().unwrap(), 25.4));
    }

    #[test]
    fn an_arc_and_a_whole_circle_are_cut_into_chords() {
        // quarter turn counterclockwise from (10,0) to (0,10) round the origin, in mm with 3.3 digits
        let q = "%FSLAX33Y33*%\n%MOMM*%\nG01*\nX10000Y0D02*\nG03*\nX0Y10000I-10000J0D01*\nM02*";
        let p = outline(q, 72).unwrap();
        let pts = &p.paths[0].points;
        assert_eq!(pts.first(), Some(&(400, 0)));
        assert_eq!(pts.last(), Some(&(0, 400)));
        assert_eq!(pts.len(), 1 + 18);
        // clockwise to the other side
        let c = "%FSLAX33Y33*%\n%MOMM*%\nX10000Y0D02*\nG02*\nX0Y-10000I-10000J0D01*\nM02*";
        assert_eq!(
            outline(c, 72).unwrap().paths[0].points.last(),
            Some(&(0, -400))
        );
        // the same start and end is a whole circle
        let w = "%FSLAX33Y33*%\n%MOMM*%\nX10000Y0D02*\nG03*\nX10000Y0I-10000J0D01*\nM02*";
        let p = outline(w, 36).unwrap();
        assert_eq!(p.paths[0].points.len(), 37);
        assert_eq!(p.paths[0].points.first(), p.paths[0].points.last());
    }

    #[test]
    fn only_the_outline_of_a_board_is_taken_for_a_route() {
        assert!(looks_like_outline("board.GKO", SQUARE));
        assert!(looks_like_outline("board.gm1", ""));
        assert!(!looks_like_outline("board.gtl", SQUARE));
        assert!(!looks_like_outline("copper.gbr", SQUARE));
        assert!(looks_like_outline(
            "b.gbr",
            "%TF.FileFunction,Profile,NP*%
%FSLAX24Y24*%"
        ));
        assert!(looks_like_outline("b-Edge_Cuts.gbr", "G04 Edge.Cuts*"));
    }

    #[test]
    fn a_moved_pen_starts_another_path_and_a_file_without_outline_is_refused() {
        let g =
            "%FSLAX33Y33*%\n%MOMM*%\nX0Y0D02*\nX1000Y0D01*\nX5000Y5000D02*\nX6000Y5000D01*\nM02*";
        assert_eq!(outline_default(g).unwrap().paths.len(), 2);
        assert!(outline_default("M02*").is_err());
        assert!(outline_default("%FSLAX24Y24*%\n%MOMM*%\nM02*").is_err());
        // a bad coordinate is said, not skipped
        assert!(outline_default("%FSLAX24Y24*%\nX1.5Y0D01*").is_err());
    }
}
