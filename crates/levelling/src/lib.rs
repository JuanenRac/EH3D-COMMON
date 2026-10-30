//! The height map of a machine table. A probe (a distance sensor beside the spindle, or a touch probe) measures the height of the table in the points of a grid;
//! the surface between the points is worked out by bilinear interpolation, and the Z of a job is corrected so that the tool keeps the same depth into the board
//! wherever the table is high or low. Everything here is plain arithmetic with no window and no machine, so it is tested on its own.

use std::fmt;

/// The grid of points that is measured: `cols` by `rows` points, from one corner of the area to the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub cols: usize,
    pub rows: usize,
}

impl Grid {
    /// A grid over the area, with at least 2 points each way.
    pub fn new(area: (f64, f64, f64, f64), cols: usize, rows: usize) -> Grid {
        Grid {
            x0: area.0.min(area.2),
            y0: area.1.min(area.3),
            x1: area.0.max(area.2),
            y1: area.1.max(area.3),
            cols: cols.max(2),
            rows: rows.max(2),
        }
    }

    pub fn len(&self) -> usize {
        self.cols * self.rows
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// The distance between two points of a row and of a column.
    pub fn step(&self) -> (f64, f64) {
        (
            (self.x1 - self.x0) / (self.cols - 1) as f64,
            (self.y1 - self.y0) / (self.rows - 1) as f64,
        )
    }

    /// Where the point `i` is: the points go along a row and then along the next one (`i = row * cols + col`).
    pub fn point(&self, i: usize) -> (f64, f64) {
        let (sx, sy) = self.step();
        (
            self.x0 + (i % self.cols) as f64 * sx,
            self.y0 + (i / self.cols) as f64 * sy,
        )
    }

    /// The order the probe visits the points: along the first row, back along the second, and so on, so that it never travels across the table.
    pub fn route(&self) -> Vec<usize> {
        let mut v = Vec::with_capacity(self.len());
        for r in 0..self.rows {
            if r % 2 == 0 {
                v.extend((0..self.cols).map(|c| r * self.cols + c));
            } else {
                v.extend((0..self.cols).rev().map(|c| r * self.cols + c));
            }
        }
        v
    }
}

/// What is wrong with a map or with a reading.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The point is not in the grid.
    NoSuchPoint(usize),
    /// The reading is not a number, or is not a line of the probe.
    BadReading(String),
    /// The map has points that were not measured yet.
    Incomplete { measured: usize, of: usize },
    /// The text of a saved map is not one.
    BadMap(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoSuchPoint(i) => write!(f, "point {i} is not in the grid"),
            Error::BadReading(s) => write!(f, "not a reading of the probe: {s}"),
            Error::Incomplete { measured, of } => {
                write!(f, "{measured} of {of} points measured")
            }
            Error::BadMap(s) => write!(f, "not a height map: {s}"),
        }
    }
}

impl std::error::Error for Error {}

/// The heights measured in the points of a grid, in mm (up is positive).
#[derive(Debug, Clone, PartialEq)]
pub struct HeightMap {
    pub grid: Grid,
    z: Vec<Option<f64>>,
}

/// The plane that is closest to the table: `z = a * x + b * y + c`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl HeightMap {
    pub fn new(grid: Grid) -> HeightMap {
        HeightMap {
            grid,
            z: vec![None; grid.len()],
        }
    }

    /// A map of a table that is flat at `z`.
    pub fn flat(grid: Grid, z: f64) -> HeightMap {
        HeightMap {
            grid,
            z: vec![Some(z); grid.len()],
        }
    }

    pub fn set(&mut self, i: usize, z: f64) -> Result<(), Error> {
        if !z.is_finite() {
            return Err(Error::BadReading(z.to_string()));
        }
        *self.z.get_mut(i).ok_or(Error::NoSuchPoint(i))? = Some(z);
        Ok(())
    }

    pub fn get(&self, i: usize) -> Option<f64> {
        self.z.get(i).copied().flatten()
    }

    pub fn measured(&self) -> usize {
        self.z.iter().flatten().count()
    }

    pub fn is_complete(&self) -> bool {
        self.measured() == self.z.len()
    }

    /// The next point the probe has to measure, in the order of the route.
    pub fn next_point(&self) -> Option<usize> {
        self.grid.route().into_iter().find(|&i| self.z[i].is_none())
    }

    /// The lowest and the highest height, and how far apart they are (the spread is what a job has to make up for).
    pub fn spread(&self) -> Option<(f64, f64, f64)> {
        let v: Vec<f64> = self.z.iter().flatten().copied().collect();
        let lo = v.iter().copied().reduce(f64::min)?;
        let hi = v.iter().copied().reduce(f64::max)?;
        Some((lo, hi, hi - lo))
    }

    /// The height of the table at `(x, y)`, by bilinear interpolation between the four points around it; outside the grid it is the one of the nearest edge.
    /// `None` when the four points were not measured.
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        let g = &self.grid;
        let (sx, sy) = g.step();
        let fx = ((x - g.x0) / sx).clamp(0.0, (g.cols - 1) as f64);
        let fy = ((y - g.y0) / sy).clamp(0.0, (g.rows - 1) as f64);
        let (c0, r0) = (
            (fx.floor() as usize).min(g.cols - 2),
            (fy.floor() as usize).min(g.rows - 2),
        );
        let (tx, ty) = (fx - c0 as f64, fy - r0 as f64);
        let at = |c: usize, r: usize| self.get(r * g.cols + c);
        let (a, b, c, d) = (
            at(c0, r0)?,
            at(c0 + 1, r0)?,
            at(c0, r0 + 1)?,
            at(c0 + 1, r0 + 1)?,
        );
        Some(a * (1.0 - tx) * (1.0 - ty) + b * tx * (1.0 - ty) + c * (1.0 - tx) * ty + d * tx * ty)
    }

    /// The plane closest to the measured points, by least squares: how much the table is tilted each way.
    pub fn plane(&self) -> Option<Plane> {
        let pts: Vec<(f64, f64, f64)> = (0..self.grid.len())
            .filter_map(|i| {
                let (x, y) = self.grid.point(i);
                self.get(i).map(|z| (x, y, z))
            })
            .collect();
        if pts.len() < 3 {
            return None;
        }
        let n = pts.len() as f64;
        let (mx, my, mz) = (
            pts.iter().map(|p| p.0).sum::<f64>() / n,
            pts.iter().map(|p| p.1).sum::<f64>() / n,
            pts.iter().map(|p| p.2).sum::<f64>() / n,
        );
        let (mut sxx, mut syy, mut sxy, mut sxz, mut syz) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for &(x, y, z) in &pts {
            let (dx, dy, dz) = (x - mx, y - my, z - mz);
            sxx += dx * dx;
            syy += dy * dy;
            sxy += dx * dy;
            sxz += dx * dz;
            syz += dy * dz;
        }
        let det = sxx * syy - sxy * sxy;
        if det.abs() < 1e-12 {
            return None;
        }
        let a = (sxz * syy - syz * sxy) / det;
        let b = (syz * sxx - sxz * sxy) / det;
        Some(Plane {
            a,
            b,
            c: mz - a * mx - b * my,
        })
    }

    /// The tilt of the table, in mm of height for each metre of travel, along X and along Y.
    pub fn tilt_mm_per_m(&self) -> Option<(f64, f64)> {
        self.plane().map(|p| (p.a * 1000.0, p.b * 1000.0))
    }

    /// The Z that keeps the same depth into the board at `(x, y)`: the Z of the job plus how far the table is from the height it has at `reference`.
    pub fn corrected_z(&self, x: f64, y: f64, z: f64, reference: (f64, f64)) -> Option<f64> {
        Some(z + self.height_at(x, y)? - self.height_at(reference.0, reference.1)?)
    }

    /// A move from `a` to `b` cut into pieces no longer than `max_len` mm, each with its Z corrected: a long move across the table follows the surface instead
    /// of going straight between its ends. The points include both ends.
    pub fn corrected_path(
        &self,
        a: (f64, f64, f64),
        b: (f64, f64, f64),
        max_len: f64,
        reference: (f64, f64),
    ) -> Option<Vec<(f64, f64, f64)>> {
        let length = (b.0 - a.0).hypot(b.1 - a.1);
        let pieces = if max_len > 0.0 {
            (length / max_len).ceil().max(1.0) as usize
        } else {
            1
        };
        (0..=pieces)
            .map(|k| {
                let t = k as f64 / pieces as f64;
                let (x, y, z) = (
                    a.0 + (b.0 - a.0) * t,
                    a.1 + (b.1 - a.1) * t,
                    a.2 + (b.2 - a.2) * t,
                );
                Some((x, y, self.corrected_z(x, y, z, reference)?))
            })
            .collect()
    }

    /// The map as text, to keep it with the machine and to read it back.
    pub fn to_text(&self) -> String {
        let g = &self.grid;
        let mut t = format!(
            "levelling 1\ngrid {} {} {} {} {} {}\n",
            g.x0, g.y0, g.x1, g.y1, g.cols, g.rows
        );
        for r in 0..g.rows {
            let row: Vec<String> = (0..g.cols)
                .map(|c| match self.z[r * g.cols + c] {
                    Some(z) => format!("{z:.4}"),
                    None => "-".to_string(),
                })
                .collect();
            t.push_str(&row.join(" "));
            t.push('\n');
        }
        t
    }

    pub fn from_text(text: &str) -> Result<HeightMap, Error> {
        let bad = |s: &str| Error::BadMap(s.to_string());
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("levelling 1") {
            return Err(bad("the first line is not `levelling 1`"));
        }
        let head: Vec<&str> = lines
            .next()
            .ok_or_else(|| bad("no grid"))?
            .split_whitespace()
            .collect();
        if head.len() != 7 || head[0] != "grid" {
            return Err(bad("the second line is not a grid"));
        }
        let f = |s: &str| s.parse::<f64>().map_err(|_| bad(s));
        let u = |s: &str| s.parse::<usize>().map_err(|_| bad(s));
        let grid = Grid::new(
            (f(head[1])?, f(head[2])?, f(head[3])?, f(head[4])?),
            u(head[5])?,
            u(head[6])?,
        );
        let mut map = HeightMap::new(grid);
        let rows: Vec<&str> = lines.filter(|l| !l.trim().is_empty()).collect();
        if rows.len() != grid.rows {
            return Err(bad("the rows are not as many as the grid says"));
        }
        for (r, line) in rows.iter().enumerate() {
            let cells: Vec<&str> = line.split_whitespace().collect();
            if cells.len() != grid.cols {
                return Err(bad("a row has not as many points as the grid says"));
            }
            for (c, cell) in cells.iter().enumerate() {
                if *cell != "-" {
                    map.set(r * grid.cols + c, f(cell)?)?;
                }
            }
        }
        Ok(map)
    }

    /// Fails when a point is missing: a job is not corrected with half a map.
    pub fn require_complete(&self) -> Result<(), Error> {
        if self.is_complete() {
            Ok(())
        } else {
            Err(Error::Incomplete {
                measured: self.measured(),
                of: self.z.len(),
            })
        }
    }
}

/// What a probe node says when it has measured a point: `Z <x> <y> <height>` (mm), one reading to a line.
pub fn parse_reading(line: &str) -> Result<(f64, f64, f64), Error> {
    let bad = || Error::BadReading(line.trim().to_string());
    let mut it = line.split_whitespace();
    if it.next() != Some("Z") {
        return Err(bad());
    }
    let mut n = || -> Result<f64, Error> {
        let v: f64 = it.next().ok_or_else(bad)?.parse().map_err(|_| bad())?;
        v.is_finite().then_some(v).ok_or_else(bad)
    };
    let r = (n()?, n()?, n()?);
    if it.next().is_some() {
        return Err(bad());
    }
    Ok(r)
}

/// A table that is tilted and has a bump, as a probe would measure it: for the tests and for the simulation of the probe.
pub fn simulated_table(x: f64, y: f64) -> f64 {
    0.0004 * x + 0.0002 * y + 0.08 * (-((x - 120.0).powi(2) + (y - 90.0).powi(2)) / 2500.0).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid::new((0.0, 0.0, 200.0, 100.0), 5, 3)
    }

    fn measured() -> HeightMap {
        let mut m = HeightMap::new(grid());
        for i in 0..grid().len() {
            let (x, y) = grid().point(i);
            m.set(i, 0.001 * x + 0.002 * y).unwrap();
        }
        m
    }

    #[test]
    fn the_points_are_in_a_grid_and_the_route_never_crosses_the_table() {
        let g = grid();
        assert_eq!(g.len(), 15);
        assert_eq!(g.point(0), (0.0, 0.0));
        assert_eq!(g.point(4), (200.0, 0.0));
        assert_eq!(g.point(14), (200.0, 100.0));
        let route = g.route();
        assert_eq!(route.len(), 15);
        assert_eq!(&route[..6], &[0, 1, 2, 3, 4, 9]);
        let mut seen = route.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..15).collect::<Vec<_>>());
        // two points that follow each other are neighbours
        for w in route.windows(2) {
            let (a, b) = (g.point(w[0]), g.point(w[1]));
            assert!((a.0 - b.0).hypot(a.1 - b.1) <= 50.0 + 1e-9);
        }
    }

    #[test]
    fn the_surface_between_the_points_is_interpolated() {
        let m = measured();
        assert_eq!(m.height_at(0.0, 0.0), Some(0.0));
        assert!((m.height_at(200.0, 100.0).unwrap() - 0.4).abs() < 1e-9);
        // a plane is rebuilt exactly anywhere
        assert!((m.height_at(73.0, 41.0).unwrap() - (0.073 + 0.082)).abs() < 1e-9);
        // outside the area, the nearest edge
        assert_eq!(m.height_at(-50.0, 0.0), m.height_at(0.0, 0.0));
        // a point that was not measured leaves the map without an answer
        let mut half = HeightMap::new(grid());
        half.set(0, 0.0).unwrap();
        assert_eq!(half.height_at(10.0, 10.0), None);
    }

    #[test]
    fn the_tilt_and_the_spread_of_the_table_are_found() {
        let m = measured();
        let p = m.plane().unwrap();
        assert!((p.a - 0.001).abs() < 1e-9 && (p.b - 0.002).abs() < 1e-9 && p.c.abs() < 1e-9);
        let (tx, ty) = m.tilt_mm_per_m().unwrap();
        assert!((tx - 1.0).abs() < 1e-6 && (ty - 2.0).abs() < 1e-6);
        let (lo, hi, spread) = m.spread().unwrap();
        assert_eq!((lo, hi), (0.0, 0.4));
        assert!((spread - 0.4).abs() < 1e-12);
        assert!(HeightMap::new(grid()).plane().is_none());
    }

    #[test]
    fn the_z_of_a_job_follows_the_table() {
        let m = measured();
        // where the table is 0.2 mm higher than at the reference, the tool goes 0.2 mm higher
        let z = m.corrected_z(100.0, 0.0, -0.1, (0.0, 0.0)).unwrap();
        assert!((z - (-0.1 + 0.1)).abs() < 1e-9);
        // a long move is cut into pieces that follow the surface
        let path = m
            .corrected_path((0.0, 0.0, -0.1), (200.0, 0.0, -0.1), 25.0, (0.0, 0.0))
            .unwrap();
        assert_eq!(path.len(), 9);
        assert!((path[8].2 - (-0.1 + 0.2)).abs() < 1e-9);
        assert!(path.windows(2).all(|w| w[1].2 >= w[0].2));
        // a flat table changes nothing
        let flat = HeightMap::flat(grid(), 0.0);
        assert_eq!(flat.corrected_z(50.0, 50.0, -0.3, (0.0, 0.0)), Some(-0.3));
    }

    #[test]
    fn the_probe_is_followed_point_by_point() {
        let mut m = HeightMap::new(grid());
        assert_eq!(m.next_point(), Some(0));
        m.set(0, 0.0).unwrap();
        m.set(1, 0.0).unwrap();
        assert_eq!(m.next_point(), Some(2));
        assert_eq!(m.measured(), 2);
        assert_eq!(
            m.require_complete(),
            Err(Error::Incomplete {
                measured: 2,
                of: 15
            })
        );
        assert_eq!(m.set(99, 0.0), Err(Error::NoSuchPoint(99)));
        assert!(m.set(3, f64::NAN).is_err());
    }

    #[test]
    fn a_reading_of_the_probe_is_read_or_refused() {
        assert_eq!(
            parse_reading("Z 12.5 40 -0.0312\n"),
            Ok((12.5, 40.0, -0.0312))
        );
        for bad in ["", "Z 1 2", "Z a b c", "X 1 2 3", "Z 1 2 3 4", "Z 1 2 nan"] {
            assert!(parse_reading(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_map_is_kept_as_text_and_comes_back() {
        let m = measured();
        // the heights are kept to a tenth of a micron
        let back = HeightMap::from_text(&m.to_text()).unwrap();
        assert_eq!(back.to_text(), m.to_text());
        assert!((0..15).all(|i| (back.get(i).unwrap() - m.get(i).unwrap()).abs() < 1e-4));
        let mut half = HeightMap::new(grid());
        half.set(7, 0.123).unwrap();
        assert_eq!(HeightMap::from_text(&half.to_text()), Ok(half));
        assert!(HeightMap::from_text("").is_err());
        assert!(HeightMap::from_text("levelling 1\ngrid 0 0 1 1 2 2\n1 2\n").is_err());
        assert!(HeightMap::from_text("levelling 1\ngrid 0 0 1 1 2 2\n1 2\n3\n").is_err());
    }

    #[test]
    fn the_simulated_table_is_tilted_and_has_a_bump() {
        let g = Grid::new((0.0, 0.0, 240.0, 180.0), 9, 7);
        let mut m = HeightMap::new(g);
        for i in 0..g.len() {
            let (x, y) = g.point(i);
            m.set(i, simulated_table(x, y)).unwrap();
        }
        let (_, _, spread) = m.spread().unwrap();
        assert!(spread > 0.1 && spread < 0.3, "{spread}");
    }
}
