//! HP-GL subset used by the plotter files of the CCD software: `IN`, `SP`, `PU`, `PD`, `PA`, `PR`, the arcs and circles (`AA`, `AR`,
//! `CI`, cut into chords) and the scaling (`SC` with `IP`).
//! One plotter unit is 0.025 mm and the origin is the lower left corner (HP 7475A convention).

pub const UNIT_MM: f64 = 0.025;

/// The default number of chords of a whole circle (one every 5 degrees).
pub const SEGMENTS_PER_CIRCLE: u32 = 72;

#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    /// Pen number, which selects the routing tool.
    pub pen: u8,
    /// Points in plotter units.
    pub points: Vec<(i32, i32)>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Plot {
    pub paths: Vec<Polyline>,
    /// Number of commands that were ignored because they are outside the supported subset.
    pub ignored: usize,
}

impl Plot {
    /// Bounding box in mm: (min_x, min_y, max_x, max_y).
    pub fn bounds_mm(&self) -> Option<(f64, f64, f64, f64)> {
        let mut it = self.paths.iter().flat_map(|p| p.points.iter());
        let first = it.next()?;
        let mut b = (first.0, first.1, first.0, first.1);
        for &(x, y) in it {
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
        Some((
            b.0 as f64 * UNIT_MM,
            b.1 as f64 * UNIT_MM,
            b.2 as f64 * UNIT_MM,
            b.3 as f64 * UNIT_MM,
        ))
    }

    /// Total pen-down length in mm.
    pub fn cut_length_mm(&self) -> f64 {
        self.paths
            .iter()
            .map(|p| p.points.windows(2).map(|w| dist(w[0], w[1])).sum::<f64>())
            .sum::<f64>()
            * UNIT_MM
    }

    /// The plot as the text of a plotter file: every path with its pen, the pen lifted to the first point and put down along the rest.
    pub fn to_hpgl(&self) -> String {
        let mut out = String::from("IN;\n");
        for p in &self.paths {
            let Some(first) = p.points.first() else {
                continue;
            };
            out.push_str(&format!("SP{};PU{},{};PD", p.pen, first.0, first.1));
            let rest: Vec<String> = p.points[1..]
                .iter()
                .map(|(x, y)| format!("{x},{y}"))
                .collect();
            out.push_str(&rest.join(","));
            out.push_str(";\n");
        }
        out.push_str("PU;SP0;\n");
        out
    }

    pub fn pens(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        for p in &self.paths {
            if !v.contains(&p.pen) {
                v.push(p.pen);
            }
        }
        v
    }
}

fn dist(a: (i32, i32), b: (i32, i32)) -> f64 {
    (((a.0 - b.0) as f64).powi(2) + ((a.1 - b.1) as f64).powi(2)).sqrt()
}

fn floats(args: &str) -> Vec<f64> {
    args.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

fn round(p: (f64, f64)) -> (i32, i32) {
    (p.0.round() as i32, p.1.round() as i32)
}

/// The points of an arc round `center` from the point `from`, `sweep` degrees counterclockwise, in chords of at most `chord` degrees
/// (the first point, `from`, is not in the list).
pub(crate) fn arc_points(
    from: (f64, f64),
    center: (f64, f64),
    sweep: f64,
    chord: f64,
) -> Vec<(f64, f64)> {
    let (dx, dy) = (from.0 - center.0, from.1 - center.1);
    let r = dx.hypot(dy);
    if r == 0.0 || sweep == 0.0 {
        return Vec::new();
    }
    let a0 = dy.atan2(dx);
    let chord = chord.abs().clamp(0.5, 90.0);
    let n = (sweep.abs() / chord).ceil().max(1.0) as usize;
    (1..=n)
        .map(|k| {
            let a = a0 + sweep.to_radians() * k as f64 / n as f64;
            (center.0 + r * a.cos(), center.1 + r * a.sin())
        })
        .collect()
}

/// The chords of a whole circle that a chord angle (in degrees, the line of the routing configuration; a minus sign only says which way the
/// circle is cut, as in the original program) asks for. A zero gives the usual number.
pub fn segments_for_chord(angle_deg: i32) -> u32 {
    if angle_deg == 0 {
        return SEGMENTS_PER_CIRCLE;
    }
    (360.0 / f64::from(angle_deg.abs()))
        .round()
        .clamp(6.0, 720.0) as u32
}

/// Reads a plotter file; the arcs and the circles are cut into `SEGMENTS_PER_CIRCLE` chords to a whole circle.
pub fn parse(text: &str) -> Plot {
    parse_with(text, SEGMENTS_PER_CIRCLE)
}

/// Like [`parse`], with the number of chords of a whole circle that the configuration of the machine asks for.
pub fn parse_with(text: &str, segments_per_circle: u32) -> Plot {
    let default_chord = 360.0 / segments_per_circle.clamp(4, 3600) as f64;
    // the window of the user units (`SC`) and the two plotter points it is laid on (`IP`)
    let (mut sc, mut ip): (Option<[f64; 4]>, Option<[f64; 4]>) = (None, None);
    let mut plot = Plot::default();
    let (mut pen, mut down, mut pos) = (1u8, false, (0i32, 0i32));
    let mut cur: Option<Polyline> = None;

    let finish = |cur: &mut Option<Polyline>, plot: &mut Plot| {
        if let Some(p) = cur.take() {
            if p.points.len() > 1 {
                plot.paths.push(p);
            }
        }
    };

    for cmd in text.split([';', '\n', '\r']) {
        let cmd = cmd.trim();
        if cmd.len() < 2 || !cmd.is_char_boundary(2) {
            continue;
        }
        let (op, args) = cmd.split_at(2);
        let op = op.to_ascii_uppercase();
        let f = floats(args);
        // what brings the user units to plotter units: factor x, factor y, offset x, offset y (none while there is no scaling)
        let map = match (sc, ip) {
            (Some(s), Some(i)) if s[1] != s[0] && s[3] != s[2] => {
                let (kx, ky) = ((i[2] - i[0]) / (s[1] - s[0]), (i[3] - i[1]) / (s[3] - s[2]));
                Some((kx, ky, i[0] - s[0] * kx, i[1] - s[2] * ky))
            }
            _ => None,
        };
        match op.as_str() {
            "IN" => {
                finish(&mut cur, &mut plot);
                down = false;
                sc = None;
                ip = None;
            }
            "SP" => {
                finish(&mut cur, &mut plot);
                pen = f.first().copied().unwrap_or(0.0).round().clamp(0.0, 255.0) as u8;
            }
            "SC" => sc = (f.len() >= 4).then(|| [f[0], f[1], f[2], f[3]]),
            "IP" => ip = (f.len() >= 4).then(|| [f[0], f[1], f[2], f[3]]),
            "CI" => {
                // a circle round the pen, whatever the pen does: its own path, and the pen stays where it was
                finish(&mut cur, &mut plot);
                let r = f.first().copied().unwrap_or(0.0) * map.map_or(1.0, |m| m.0.abs());
                let chord = f.get(1).copied().unwrap_or(default_chord);
                if r > 0.0 {
                    let c = (pos.0 as f64, pos.1 as f64);
                    let start = (c.0 + r, c.1);
                    let mut points = vec![round(start)];
                    points.extend(arc_points(start, c, 360.0, chord).into_iter().map(round));
                    plot.paths.push(Polyline { pen, points });
                }
            }
            "AA" | "AR" => {
                let (cx, cy) = (
                    f.first().copied().unwrap_or(0.0),
                    f.get(1).copied().unwrap_or(0.0),
                );
                let sweep = f.get(2).copied().unwrap_or(0.0);
                let chord = f.get(3).copied().unwrap_or(default_chord);
                let from = (pos.0 as f64, pos.1 as f64);
                let center = if op == "AR" {
                    let (kx, ky) = map.map_or((1.0, 1.0), |m| (m.0, m.1));
                    (from.0 + cx * kx, from.1 + cy * ky)
                } else {
                    map.map_or((cx, cy), |m| (cx * m.0 + m.2, cy * m.1 + m.3))
                };
                for p in arc_points(from, center, sweep, chord) {
                    let to = round(p);
                    if down {
                        cur.get_or_insert_with(|| Polyline {
                            pen,
                            points: vec![pos],
                        })
                        .points
                        .push(to);
                    }
                    pos = to;
                }
                if !down {
                    finish(&mut cur, &mut plot);
                }
            }
            "PU" | "PD" | "PA" | "PR" => {
                let new_down = match op.as_str() {
                    "PU" => false,
                    "PD" => true,
                    _ => down,
                };
                if new_down != down {
                    finish(&mut cur, &mut plot);
                }
                down = new_down;
                let rel = op == "PR";
                for pair in f.as_chunks::<2>().0 {
                    let to = match (rel, map) {
                        (true, Some(m)) => {
                            round((pos.0 as f64 + pair[0] * m.0, pos.1 as f64 + pair[1] * m.1))
                        }
                        (true, None) => round((pos.0 as f64 + pair[0], pos.1 as f64 + pair[1])),
                        (false, Some(m)) => round((pair[0] * m.0 + m.2, pair[1] * m.1 + m.3)),
                        (false, None) => round((pair[0], pair[1])),
                    };
                    if down {
                        cur.get_or_insert_with(|| Polyline {
                            pen,
                            points: vec![pos],
                        })
                        .points
                        .push(to);
                    } else {
                        finish(&mut cur, &mut plot);
                    }
                    pos = to;
                }
                if down && f.is_empty() && cur.is_none() {
                    cur = Some(Polyline {
                        pen,
                        points: vec![pos],
                    });
                }
            }
            _ => plot.ignored += 1,
        }
    }
    finish(&mut cur, &mut plot);
    plot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(p: &Plot) -> (i32, i32, i32, i32) {
        let pts: Vec<_> = p
            .paths
            .iter()
            .flat_map(|q| q.points.iter().copied())
            .collect();
        (
            pts.iter().map(|p| p.0).min().unwrap(),
            pts.iter().map(|p| p.1).min().unwrap(),
            pts.iter().map(|p| p.0).max().unwrap(),
            pts.iter().map(|p| p.1).max().unwrap(),
        )
    }

    #[test]
    fn a_circle_is_cut_into_chords_round_the_pen() {
        let p = parse("SP1;PU1000,1000;CI200;");
        assert_eq!(p.paths.len(), 1);
        assert_eq!(p.paths[0].points.len() as u32, SEGMENTS_PER_CIRCLE + 1);
        assert_eq!(bounds(&p), (800, 800, 1200, 1200));
        // closed: it ends where it began
        assert_eq!(p.paths[0].points.first(), p.paths[0].points.last());
        // the chords of the configuration of the machine
        assert_eq!(parse_with("CI200;", 36).paths[0].points.len(), 37);
        // a chord angle in the command is the one that counts
        assert_eq!(parse("CI200,10;").paths[0].points.len(), 37);
    }

    #[test]
    fn an_arc_goes_round_its_centre_from_where_the_pen_is() {
        // from (100,0) a quarter turn counterclockwise round the origin ends at (0,100)
        let p = parse("PU100,0;PD;AA0,0,90;");
        assert_eq!(p.paths.len(), 1);
        assert_eq!(p.paths[0].points.first(), Some(&(100, 0)));
        assert_eq!(p.paths[0].points.last(), Some(&(0, 100)));
        // clockwise
        assert_eq!(
            parse("PU100,0;PD;AA0,0,-90;").paths[0].points.last(),
            Some(&(0, -100))
        );
        // the centre given relative to the pen
        assert_eq!(
            parse("PU100,50;PD;AR-100,-50,90;").paths[0].points.last(),
            Some(&(-50, 100))
        );
        // with the pen up an arc only moves it
        let s = parse("PU100,0;AA0,0,90;PD;PA10,10;");
        assert_eq!(s.paths[0].points.first(), Some(&(0, 100)));
    }

    #[test]
    fn the_scaling_lays_the_user_units_on_the_plotter() {
        // 0..100 user units on the plotter points 0..4000 (40 points each)
        let p = parse("IP0,0,4000,4000;SC0,100,0,100;PU0,0;PD50,50,100,0;");
        assert_eq!(bounds(&p), (0, 0, 4000, 2000));
        // a circle of 10 user units is 400 plotter points
        let c = parse("IP0,0,4000,4000;SC0,100,0,100;PU50,50;CI10;");
        assert_eq!(bounds(&c), (1600, 1600, 2400, 2400));
        // a scaling without the points to lay it on is left alone
        assert_eq!(
            bounds(&parse("SC0,100,0,100;PU0,0;PD50,50;")),
            (0, 0, 50, 50)
        );
    }

    #[test]
    fn the_old_commands_read_as_they_did() {
        let p = parse("IN;SP1;PU0,0;PD400,0,400,800;SP2;PU40,40;PD80,80;");
        assert_eq!(p.paths.len(), 2);
        assert_eq!(p.paths[0].points, vec![(0, 0), (400, 0), (400, 800)]);
        assert_eq!(p.paths[1].pen, 2);
        assert_eq!(p.ignored, 0);
        assert_eq!(parse("LT2;").ignored, 1);
    }

    #[test]
    fn the_chord_angle_of_the_configuration_gives_the_chords_of_a_circle() {
        assert_eq!(segments_for_chord(5), 72);
        assert_eq!(segments_for_chord(-5), 72);
        assert_eq!(segments_for_chord(10), 36);
        assert_eq!(segments_for_chord(0), SEGMENTS_PER_CIRCLE);
        assert_eq!(segments_for_chord(60), 6);
        assert_eq!(segments_for_chord(1), 360);
    }

    #[test]
    fn a_plot_written_as_hpgl_reads_back_the_same() {
        let p = parse("SP1;PU0,0;PD400,0,400,800;SP2;PU40,40;PD80,80,120,40;");
        let back = parse(&p.to_hpgl());
        assert_eq!(back, p);
    }
}
