//! A job as it is seen on the table: the holes with their sizes, the paths of the routing, how far the machine has got, and the two
//! views of it (the plan from above, and the isometric view of the machine). Drilling and routing jobs go into the same scene, so
//! both can be shown on top of each other. Everything is in millimetres, with the origin at the home corner of the table.

/// The work table chosen in the configuration (the clipping of X and Y).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Table {
    pub width_mm: f64,
    pub depth_mm: f64,
}

impl Table {
    pub const fn new(width_mm: f64, depth_mm: f64) -> Self {
        Table { width_mm, depth_mm }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        (0.0..=self.width_mm).contains(&x) && (0.0..=self.depth_mm).contains(&y)
    }
}

/// One hole of a drilling job.
#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    pub x_mm: f64,
    pub y_mm: f64,
    pub diameter_mm: f64,
    /// The tool number the job drills it with.
    pub tool: u8,
    pub done: bool,
}

/// One path of a routing job: points in order, and how many of them the machine has already reached.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub tool: u8,
    pub points: Vec<(f64, f64)>,
    pub reached: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub table: Table,
    pub holes: Vec<Hole>,
    pub paths: Vec<Path>,
}

impl Scene {
    pub fn new(table: Table) -> Self {
        Scene {
            table,
            holes: Vec::new(),
            paths: Vec::new(),
        }
    }

    /// Puts the jobs of another scene on this one (a drilling job and a routing job in the same view).
    pub fn overlay(&mut self, other: &Scene) {
        self.holes.extend(other.holes.iter().cloned());
        self.paths.extend(other.paths.iter().cloned());
    }

    /// The diameters of the holes, each once, from the smallest, rounded to 0.01 mm (the legend of the sizes).
    pub fn sizes(&self) -> Vec<f64> {
        let mut v: Vec<i64> = self
            .holes
            .iter()
            .map(|h| (h.diameter_mm * 100.0).round() as i64)
            .collect();
        v.sort_unstable();
        v.dedup();
        v.into_iter().map(|d| d as f64 / 100.0).collect()
    }

    /// How many holes there are of a size (to 0.01 mm) and how many of them are done.
    pub fn size_count(&self, diameter_mm: f64) -> (usize, usize) {
        let key = (diameter_mm * 100.0).round() as i64;
        let of: Vec<&Hole> = self
            .holes
            .iter()
            .filter(|h| (h.diameter_mm * 100.0).round() as i64 == key)
            .collect();
        (of.len(), of.iter().filter(|h| h.done).count())
    }

    /// The part of the work done, from 0 to 1: the holes drilled and the points of the paths reached, together.
    pub fn progress(&self) -> f64 {
        let total = self.holes.len() + self.paths.iter().map(|p| p.points.len()).sum::<usize>();
        if total == 0 {
            return 0.0;
        }
        let done = self.holes.iter().filter(|h| h.done).count()
            + self
                .paths
                .iter()
                .map(|p| p.reached.min(p.points.len()))
                .sum::<usize>();
        done as f64 / total as f64
    }

    /// The box that holds every hole and every point: (min x, min y, max x, max y). `None` for an empty scene.
    pub fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let pts = self
            .holes
            .iter()
            .map(|h| (h.x_mm, h.y_mm))
            .chain(self.paths.iter().flat_map(|p| p.points.iter().copied()));
        let mut b: Option<(f64, f64, f64, f64)> = None;
        for (x, y) in pts {
            b = Some(match b {
                None => (x, y, x, y),
                Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)),
            });
        }
        b
    }

    /// What does not fit on the table: the indexes of the holes and of the paths with a point outside it. A job that does not fit is
    /// said so before it starts, not found out on the machine.
    pub fn outside_table(&self) -> (Vec<usize>, Vec<usize>) {
        let holes = (0..self.holes.len())
            .filter(|&i| !self.table.contains(self.holes[i].x_mm, self.holes[i].y_mm))
            .collect();
        let paths = (0..self.paths.len())
            .filter(|&i| {
                self.paths[i]
                    .points
                    .iter()
                    .any(|&(x, y)| !self.table.contains(x, y))
            })
            .collect();
        (holes, paths)
    }

    /// The plan from above fitted into a view of `width` x `height` pixels with a `margin`: the whole table, centred.
    pub fn view2d(&self, width: f64, height: f64, margin: f64) -> View2D {
        let sx = (width - 2.0 * margin) / self.table.width_mm;
        let sy = (height - 2.0 * margin) / self.table.depth_mm;
        let scale = sx.min(sy).max(f64::MIN_POSITIVE);
        View2D {
            scale,
            origin: (
                (width - self.table.width_mm * scale) / 2.0,
                (height - self.table.depth_mm * scale) / 2.0,
            ),
        }
    }
}

/// The plan view: millimetres to pixels, with Y going up on the table and down on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View2D {
    pub scale: f64,
    pub origin: (f64, f64),
}

impl View2D {
    pub fn to_screen(&self, table: &Table, x_mm: f64, y_mm: f64) -> (f64, f64) {
        (
            self.origin.0 + x_mm * self.scale,
            self.origin.1 + (table.depth_mm - y_mm) * self.scale,
        )
    }

    /// The other way round: a click on the screen to a point of the table.
    pub fn to_table(&self, table: &Table, px: f64, py: f64) -> (f64, f64) {
        (
            (px - self.origin.0) / self.scale,
            table.depth_mm - (py - self.origin.1) / self.scale,
        )
    }

    /// The radius in pixels of a hole of `diameter_mm`; never smaller than `min_px` so that a small drill can still be seen.
    pub fn hole_radius(&self, diameter_mm: f64, min_px: f64) -> f64 {
        (diameter_mm * self.scale / 2.0).max(min_px)
    }
}

/// The isometric view of the machine: a point of the table and a height to a point of the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Iso {
    /// Pixels per millimetre.
    pub k: f64,
    pub origin: (f64, f64),
}

impl Iso {
    pub fn project(&self, x_mm: f64, y_mm: f64, z_mm: f64) -> (f64, f64) {
        let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
        (
            self.origin.0 + (x_mm - y_mm) * c * self.k,
            self.origin.1 + (x_mm + y_mm) * s * self.k - z_mm * self.k,
        )
    }

    /// An isometric view that holds the whole machine (the table of this size and a gantry `height_mm` high) in `width` x `height`
    /// pixels: the table changes with the configuration, the view follows.
    pub fn fit(table: &Table, height_mm: f64, width: f64, height: f64) -> Iso {
        let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
        let w_mm = (table.width_mm + table.depth_mm) * c;
        let h_mm = (table.width_mm + table.depth_mm) * s + height_mm;
        let k = (width / w_mm).min(height / h_mm);
        // the left-most point is the corner (0, depth); the highest is the top of the gantry above the corner (0, 0)
        let left = -table.depth_mm * c * k;
        let origin = (
            (width - w_mm * k) / 2.0 - left,
            (height - h_mm * k) / 2.0 + height_mm * k,
        );
        Iso { k, origin }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hole(x: f64, y: f64, d: f64, done: bool) -> Hole {
        Hole {
            x_mm: x,
            y_mm: y,
            diameter_mm: d,
            tool: 1,
            done,
        }
    }

    fn scene() -> Scene {
        let mut s = Scene::new(Table::new(495.0, 325.0));
        s.holes = vec![
            hole(10.0, 10.0, 0.8, true),
            hole(20.0, 10.0, 0.8, true),
            hole(30.0, 10.0, 1.0, false),
            hole(40.0, 10.0, 3.2, false),
        ];
        s
    }

    #[test]
    fn the_legend_has_each_size_once_from_the_smallest() {
        let mut s = scene();
        s.holes.push(hole(1.0, 1.0, 0.801, false));
        assert_eq!(s.sizes(), vec![0.8, 1.0, 3.2]);
        assert_eq!(s.size_count(0.8), (3, 2));
        assert_eq!(s.size_count(3.2), (1, 0));
        assert_eq!(s.size_count(9.9), (0, 0));
    }

    #[test]
    fn the_progress_counts_holes_and_points_together() {
        let mut s = scene();
        assert_eq!(s.progress(), 0.5);
        s.paths.push(Path {
            tool: 2,
            points: vec![(0.0, 0.0); 4],
            reached: 3,
        });
        // 2 holes + 3 points done out of 4 + 4
        assert!((s.progress() - 5.0 / 8.0).abs() < 1e-12);
        assert_eq!(Scene::new(Table::new(1.0, 1.0)).progress(), 0.0);
    }

    #[test]
    fn a_drilling_job_and_a_routing_job_share_one_scene() {
        let mut drill = scene();
        let mut route = Scene::new(Table::new(495.0, 325.0));
        route.paths.push(Path {
            tool: 1,
            points: vec![(5.0, 5.0), (100.0, 5.0), (100.0, 80.0)],
            reached: 0,
        });
        drill.overlay(&route);
        assert_eq!((drill.holes.len(), drill.paths.len()), (4, 1));
        assert_eq!(drill.bounds(), Some((5.0, 5.0, 100.0, 80.0)));
        assert_eq!(Scene::new(Table::new(1.0, 1.0)).bounds(), None);
    }

    #[test]
    fn what_does_not_fit_on_the_table_is_found_before_the_job_starts() {
        let mut s = scene();
        s.holes.push(hole(500.0, 10.0, 1.0, false));
        s.holes.push(hole(10.0, -0.5, 1.0, false));
        s.paths.push(Path {
            tool: 1,
            points: vec![(0.0, 0.0), (495.0, 325.0)],
            reached: 0,
        });
        s.paths.push(Path {
            tool: 1,
            points: vec![(0.0, 0.0), (495.1, 10.0)],
            reached: 0,
        });
        assert_eq!(s.outside_table(), (vec![4, 5], vec![1]));
        // a bigger table, chosen in the configuration, takes them all
        s.table = Table::new(600.0, 400.0);
        assert_eq!(s.outside_table().1, Vec::<usize>::new());
    }

    #[test]
    fn the_plan_view_fits_the_table_and_the_clicks_come_back() {
        let s = scene();
        let v = s.view2d(1000.0, 600.0, 20.0);
        // the whole table is inside the margin and centred
        let (x0, y0) = v.to_screen(&s.table, 0.0, 325.0);
        let (x1, y1) = v.to_screen(&s.table, 495.0, 0.0);
        assert!(x0 >= 20.0 - 1e-9 && y0 >= 20.0 - 1e-9 && x1 <= 980.0 + 1e-9 && y1 <= 580.0 + 1e-9);
        assert!(((x0 + x1) / 2.0 - 500.0).abs() < 1e-9 && ((y0 + y1) / 2.0 - 300.0).abs() < 1e-9);
        let (px, py) = v.to_screen(&s.table, 123.4, 56.7);
        let (x, y) = v.to_table(&s.table, px, py);
        assert!((x - 123.4).abs() < 1e-9 && (y - 56.7).abs() < 1e-9);
        // Y goes up on the table: a higher point is higher on the screen (a smaller pixel row)
        assert!(v.to_screen(&s.table, 0.0, 100.0).1 < v.to_screen(&s.table, 0.0, 10.0).1);
        // a small drill is still seen
        assert_eq!(v.hole_radius(0.1, 2.2), 2.2);
        assert!(v.hole_radius(3.2, 2.2) > 2.2);
    }

    #[test]
    fn the_machine_view_follows_the_size_of_the_table() {
        for (w, d) in [(495.0, 325.0), (325.0, 270.0), (600.0, 400.0)] {
            let t = Table::new(w, d);
            let iso = Iso::fit(&t, 170.0, 480.0, 540.0);
            // the four corners of the table and the top of the gantry are inside the view
            let pts = [
                iso.project(0.0, 0.0, 0.0),
                iso.project(w, 0.0, 0.0),
                iso.project(0.0, d, 0.0),
                iso.project(w, d, 0.0),
                iso.project(0.0, 0.0, 170.0),
            ];
            for (x, y) in pts {
                assert!((-1e-6..=480.0 + 1e-6).contains(&x), "{w}x{d}: x {x}");
                assert!((-1e-6..=540.0 + 1e-6).contains(&y), "{w}x{d}: y {y}");
            }
        }
        // a bigger table is drawn smaller in the same space
        let a = Iso::fit(&Table::new(325.0, 270.0), 170.0, 480.0, 540.0);
        let b = Iso::fit(&Table::new(600.0, 400.0), 170.0, 480.0, 540.0);
        assert!(b.k < a.k);
    }
}
