//! The drawing of a board in a DXF file (the text form, as every drawing program writes it) turned into the routing paths of a plot: the
//! lines (`LINE`), the polylines of the light kind (`LWPOLYLINE`, with their arcs given as bulges), the arcs (`ARC`) and the circles
//! (`CIRCLE`). Strokes that meet end to end are joined into one path so that the router does not lift between them. The layers, the colours
//! and the text mean nothing for a cut and are skipped.

use crate::hpgl::{arc_points, Plot, Polyline, SEGMENTS_PER_CIRCLE, UNIT_MM};

/// The file extension of a drawing.
pub const EXTENSION: &str = "dxf";

/// How close the end of a stroke has to be to the start of the next one (mm) to join them.
const JOIN_MM: f64 = 0.005;

/// What the entity being read has collected.
#[derive(Default)]
struct Entity {
    kind: String,
    closed: bool,
    x: f64,
    y: f64,
    x2: f64,
    y2: f64,
    radius: f64,
    start: f64,
    end: f64,
    /// The vertices of a light polyline with the bulge of the stroke that leaves each one.
    vertices: Vec<(f64, f64, f64)>,
}

/// The millimetres of a drawing unit (`$INSUNITS`); a drawing that does not say is taken to be in millimetres.
fn unit_mm(code: i64) -> f64 {
    match code {
        1 => 25.4,
        2 => 304.8,
        4 => 1.0,
        5 => 10.0,
        6 => 1000.0,
        _ => 1.0,
    }
}

/// The strokes of an entity, each as a list of points in the drawing's own unit.
fn strokes(e: &Entity, chord: f64) -> Vec<Vec<(f64, f64)>> {
    match e.kind.as_str() {
        "LINE" => vec![vec![(e.x, e.y), (e.x2, e.y2)]],
        "CIRCLE" if e.radius > 0.0 => {
            let start = (e.x + e.radius, e.y);
            let mut p = vec![start];
            p.extend(arc_points(start, (e.x, e.y), 360.0, chord));
            vec![p]
        }
        "ARC" if e.radius > 0.0 => {
            let a0 = e.start.to_radians();
            let start = (e.x + e.radius * a0.cos(), e.y + e.radius * a0.sin());
            let mut sweep = (e.end - e.start).rem_euclid(360.0);
            if sweep == 0.0 {
                sweep = 360.0;
            }
            let mut p = vec![start];
            p.extend(arc_points(start, (e.x, e.y), sweep, chord));
            vec![p]
        }
        "LWPOLYLINE" if e.vertices.len() > 1 => {
            let mut v = e.vertices.clone();
            if e.closed {
                v.push(v[0]);
            }
            let mut pts = vec![(v[0].0, v[0].1)];
            for w in v.windows(2) {
                let (a, b) = ((w[0].0, w[0].1), (w[1].0, w[1].1));
                if w[0].2 == 0.0 {
                    pts.push(b);
                    continue;
                }
                // a bulge is the tangent of a quarter of the angle the stroke turns through (counterclockwise when it is positive)
                let theta = 4.0 * w[0].2.atan();
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let c = dx.hypot(dy);
                if c == 0.0 {
                    continue;
                }
                let d = (c / 2.0) / (theta / 2.0).tan();
                let center = (
                    (a.0 + b.0) / 2.0 - dy / c * d,
                    (a.1 + b.1) / 2.0 + dx / c * d,
                );
                let mut arc = arc_points(a, center, theta.to_degrees(), chord);
                if let Some(last) = arc.last_mut() {
                    *last = b;
                }
                pts.extend(arc);
            }
            vec![pts]
        }
        _ => Vec::new(),
    }
}

/// The outline drawn in the file as the paths of a plot, in plotter units. `segments_per_circle` is how many chords a whole circle is
/// cut into. An error says what cannot be read.
pub fn outline(text: &str, segments_per_circle: u32) -> Result<Plot, String> {
    let chord = 360.0 / f64::from(segments_per_circle.clamp(4, 3600));
    let lines: Vec<&str> = text.lines().collect();
    let mut pairs: Vec<(i64, &str)> = Vec::with_capacity(lines.len() / 2);
    for c in lines.chunks(2) {
        if let [code, value] = c {
            let code = code
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("not a DXF file (group code '{}')", code.trim()))?;
            pairs.push((code, value.trim()));
        }
    }
    if pairs.is_empty() {
        return Err("empty file".to_string());
    }
    let mut mm = 1.0;
    let mut in_entities = false;
    let mut section_open = false;
    let mut cur: Option<Entity> = None;
    let mut done: Vec<Entity> = Vec::new();
    let mut i = 0;
    while i < pairs.len() {
        let (code, value) = pairs[i];
        i += 1;
        if code == 9 && value.eq_ignore_ascii_case("$INSUNITS") {
            if let Some((70, v)) = pairs.get(i) {
                mm = unit_mm(v.parse::<i64>().unwrap_or(4));
            }
            continue;
        }
        if code == 0 {
            if let Some(e) = cur.take() {
                done.push(e);
            }
            match value {
                "SECTION" => section_open = true,
                "ENDSEC" => {
                    in_entities = false;
                    section_open = false;
                }
                "EOF" => break,
                other if in_entities => {
                    cur = Some(Entity {
                        kind: other.to_string(),
                        ..Default::default()
                    })
                }
                _ => {}
            }
            continue;
        }
        if code == 2 && section_open && value.eq_ignore_ascii_case("ENTITIES") {
            in_entities = true;
            continue;
        }
        let Some(e) = cur.as_mut() else { continue };
        let num = value.parse::<f64>().unwrap_or(0.0);
        match code {
            10 => {
                if e.kind == "LWPOLYLINE" {
                    e.vertices.push((num, 0.0, 0.0));
                } else {
                    e.x = num;
                }
            }
            20 => {
                if e.kind == "LWPOLYLINE" {
                    if let Some(v) = e.vertices.last_mut() {
                        v.1 = num;
                    }
                } else {
                    e.y = num;
                }
            }
            42 => {
                if let Some(v) = e.vertices.last_mut() {
                    v.2 = num;
                }
            }
            11 => e.x2 = num,
            21 => e.y2 = num,
            40 => e.radius = num,
            50 => e.start = num,
            51 => e.end = num,
            70 => e.closed = (num as i64) & 1 == 1,
            _ => {}
        }
    }
    if let Some(e) = cur.take() {
        done.push(e);
    }

    // the strokes, joined where one ends where the next begins
    let mut paths: Vec<Vec<(f64, f64)>> = Vec::new();
    for e in &done {
        for s in strokes(e, chord) {
            let s: Vec<(f64, f64)> = s.into_iter().map(|(x, y)| (x * mm, y * mm)).collect();
            let (Some(first), Some(_)) = (s.first().copied(), s.last()) else {
                continue;
            };
            match paths.last_mut() {
                Some(p)
                    if p.last()
                        .is_some_and(|l| (l.0 - first.0).hypot(l.1 - first.1) <= JOIN_MM) =>
                {
                    p.extend(s.into_iter().skip(1));
                }
                _ => paths.push(s),
            }
        }
    }
    if paths.is_empty() {
        return Err("no lines, arcs or circles in the drawing".to_string());
    }
    let units = |v: f64| (v / UNIT_MM).round() as i32;
    Ok(Plot {
        paths: paths
            .into_iter()
            .map(|p| Polyline {
                pen: 1,
                points: p.into_iter().map(|(x, y)| (units(x), units(y))).collect(),
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

    fn dxf(units: Option<i64>, entities: &str) -> String {
        let header = units.map_or(String::new(), |u| {
            format!("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n{u}\n0\nENDSEC\n")
        });
        format!("{header}0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n")
    }

    fn line(x: f64, y: f64, x2: f64, y2: f64) -> String {
        format!("0\nLINE\n8\n0\n10\n{x}\n20\n{y}\n11\n{x2}\n21\n{y2}\n")
    }

    #[test]
    fn lines_that_meet_end_to_end_are_one_path_in_plotter_units() {
        let e = [
            line(0.0, 0.0, 50.0, 0.0),
            line(50.0, 0.0, 50.0, 30.0),
            line(50.0, 30.0, 0.0, 30.0),
            line(0.0, 30.0, 0.0, 0.0),
        ]
        .concat();
        let p = outline_default(&dxf(Some(4), &e)).unwrap();
        assert_eq!(p.paths.len(), 1);
        assert_eq!(
            p.paths[0].points,
            vec![(0, 0), (2000, 0), (2000, 1200), (0, 1200), (0, 0)]
        );
        // a line that does not meet the others is a path of its own
        let e2 = [line(0.0, 0.0, 10.0, 0.0), line(20.0, 0.0, 30.0, 0.0)].concat();
        assert_eq!(outline_default(&dxf(None, &e2)).unwrap().paths.len(), 2);
    }

    #[test]
    fn inches_are_taken_from_the_header() {
        let p = outline_default(&dxf(Some(1), &line(0.0, 0.0, 1.0, 0.0))).unwrap();
        assert_eq!(p.paths[0].points.last(), Some(&(1016, 0)));
    }

    #[test]
    fn arcs_and_circles_are_cut_into_chords() {
        let arc = "0\nARC\n10\n0\n20\n0\n40\n10\n50\n0\n51\n90\n";
        let p = outline(&dxf(Some(4), arc), 72).unwrap();
        let pts = &p.paths[0].points;
        assert_eq!(pts.first(), Some(&(400, 0)));
        assert_eq!(pts.last(), Some(&(0, 400)));
        assert_eq!(pts.len(), 1 + 18);
        let circle = "0\nCIRCLE\n10\n5\n20\n5\n40\n2\n";
        let c = outline(&dxf(Some(4), circle), 36).unwrap();
        assert_eq!(c.paths[0].points.len(), 37);
        assert_eq!(c.paths[0].points.first(), c.paths[0].points.last());
    }

    #[test]
    fn a_light_polyline_with_a_bulge_has_its_arc() {
        // a half circle of radius 5 from (0,0) to (10,0), then the line back: a closed D shape
        let shape = |bulge: i32| {
            format!(
                "0
LWPOLYLINE
90
2
70
1
10
0
20
0
42
{bulge}
10
10
20
0
"
            )
        };
        // clockwise (a negative bulge) from the left end to the right one goes over the top
        let p = outline(&dxf(Some(4), &shape(-1)), 72).unwrap();
        let b = p.bounds_mm().unwrap();
        assert!(
            (b.0 - 0.0).abs() < 0.01 && (b.2 - 10.0).abs() < 0.01,
            "{b:?}"
        );
        assert!(b.1.abs() < 0.01 && (b.3 - 5.0).abs() < 0.05, "{b:?}");
        assert_eq!(p.paths[0].points.first(), p.paths[0].points.last());
        // counterclockwise (a positive one) goes under it
        let q = outline(&dxf(Some(4), &shape(1)), 72).unwrap();
        let b = q.bounds_mm().unwrap();
        assert!((b.1 + 5.0).abs() < 0.05 && b.3.abs() < 0.01, "{b:?}");
    }

    #[test]
    fn what_is_not_a_drawing_is_refused() {
        assert!(outline_default("").is_err());
        assert!(outline_default("hello\nworld\n").is_err());
        assert!(outline_default(&dxf(None, "0\nTEXT\n1\nhi\n")).is_err());
    }
}
