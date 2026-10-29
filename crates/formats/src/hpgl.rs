//! HP-GL subset used by the plotter files of the CCD software: `IN`, `SP`, `PU`, `PD`, `PA`, `PR`.
//! One plotter unit is 0.025 mm and the origin is the lower left corner (HP 7475A convention).

pub const UNIT_MM: f64 = 0.025;

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

fn numbers(args: &str) -> Vec<i32> {
    args.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .map(|v| v.round() as i32)
        .collect()
}

pub fn parse(text: &str) -> Plot {
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
        let n = numbers(args);
        match op.as_str() {
            "IN" => {
                finish(&mut cur, &mut plot);
                down = false;
            }
            "SP" => {
                finish(&mut cur, &mut plot);
                pen = n.first().copied().unwrap_or(0).clamp(0, 255) as u8;
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
                for pair in n.as_chunks::<2>().0 {
                    let to = if rel {
                        (pos.0 + pair[0], pos.1 + pair[1])
                    } else {
                        (pair[0], pair[1])
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
                if down && n.is_empty() && cur.is_none() {
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
