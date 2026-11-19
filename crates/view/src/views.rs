//! The standard views of the machine: from above (a true plan, orthographic), from the front, from the side and the usual isometric one.
//!
//! Every view is given by a [`StandardView`] and drawn by an [`Ortho`]: a scale in pixels per millimetre and the pixel of the origin of
//! the table. Axes of the machine: X to the right along the front edge of the table, Y away from the operator, Z up. The home corner
//! `(0, 0, 0)` is the front left corner of the table, on the table.
//!
//! | View | Eye | On the screen |
//! |------|-----|---------------|
//! | `Top` | straight over the table, looking down | X to the right, Y up; Z does not move the point |
//! | `Front` | in front of the table (Y below zero), looking along +Y | X to the right, Z up; Y does not move the point |
//! | `Side` | at the right side (X over the table width), looking along -X | Y to the right, Z up; X does not move the point |
//! | `Iso` | the usual isometric view, the same as [`crate::scene::Iso`] with no turn and a tilt of 30 degrees | X goes right and down, Y goes left and down |
//!
//! `Top`, `Front` and `Side` are true orthographic views with the same scale on both screen axes. There is no tilt of 90 degrees to
//! divide by: the view from above does not go through the tilt formula of `Iso` (which at 90 degrees would turn the plan by 45 degrees).

use crate::scene::{Iso, Table};

/// One of the standard views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardView {
    Top,
    Front,
    Side,
    Iso,
}

impl StandardView {
    pub const ALL: [StandardView; 4] = [
        StandardView::Top,
        StandardView::Front,
        StandardView::Side,
        StandardView::Iso,
    ];

    /// The turn and the tilt, in degrees, that an `Iso`-based orbit (a turn around the vertical axis and a tilt of the eye) uses for the
    /// views that have one: `Front` is 315 and 0, `Side` is 225 and 0, `Iso` is 0 and 30. `Top` has none (its tilt is 90 degrees and is
    /// handled by [`Ortho`]); it answers 0 and 90.
    pub fn orbit_angles(self) -> (f64, f64) {
        match self {
            StandardView::Top => (0.0, 90.0),
            StandardView::Front => (315.0, 0.0),
            StandardView::Side => (225.0, 0.0),
            StandardView::Iso => (0.0, 30.0),
        }
    }
}

/// The projection of one standard view: a point of the machine in millimetres to a point of the screen in pixels (Y of the screen grows
/// downwards).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ortho {
    pub view: StandardView,
    /// Pixels per millimetre.
    pub k: f64,
    /// The pixel of the point `(0, 0, 0)`.
    pub origin: (f64, f64),
}

const COS30: f64 = 0.866_025_403_784_438_6;

/// The point at `k = 1` and no origin.
fn unit(view: StandardView, x: f64, y: f64, z: f64) -> (f64, f64) {
    match view {
        StandardView::Top => (x, -y),
        StandardView::Front => (x, -z),
        StandardView::Side => (y, -z),
        // the same numbers as `Iso::project` with no turn and a tilt of 30 degrees
        StandardView::Iso => ((x - y) * COS30, (x + y) * 0.5 - z),
    }
}

impl Ortho {
    pub fn project(&self, x_mm: f64, y_mm: f64, z_mm: f64) -> (f64, f64) {
        let (u, v) = unit(self.view, x_mm, y_mm, z_mm);
        (self.origin.0 + u * self.k, self.origin.1 + v * self.k)
    }

    /// The view from above gives back the point of the table under a pixel (the click). The other views cannot: a pixel is a line of
    /// points, so they answer `None`.
    pub fn to_table(&self, px: f64, py: f64) -> Option<(f64, f64)> {
        if self.view != StandardView::Top {
            return None;
        }
        Some((
            (px - self.origin.0) / self.k,
            -(py - self.origin.1) / self.k,
        ))
    }

    /// The view that holds the box from `min` to `max` (millimetres, `(x, y, z)`) centred in `width` x `height` pixels, with `margin`
    /// pixels free on every side. A box with no extent (or an area smaller than the margins) keeps a scale of 1 pixel per millimetre.
    pub fn fit_box(
        view: StandardView,
        min: (f64, f64, f64),
        max: (f64, f64, f64),
        width: f64,
        height: f64,
        margin: f64,
    ) -> Ortho {
        let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
        for x in [min.0, max.0] {
            for y in [min.1, max.1] {
                for z in [min.2, max.2] {
                    let (u, v) = unit(view, x, y, z);
                    lo = (lo.0.min(u), lo.1.min(v));
                    hi = (hi.0.max(u), hi.1.max(v));
                }
            }
        }
        let (w_mm, h_mm) = (hi.0 - lo.0, hi.1 - lo.1);
        let (room_w, room_h) = (width - 2.0 * margin, height - 2.0 * margin);
        let scale = |room: f64, mm: f64| {
            if mm > 1e-12 && room > 0.0 {
                room / mm
            } else {
                f64::MAX
            }
        };
        let k = scale(room_w, w_mm).min(scale(room_h, h_mm));
        let k = if k == f64::MAX { 1.0 } else { k };
        Ortho {
            view,
            k,
            origin: (
                (width - w_mm * k) / 2.0 - lo.0 * k,
                (height - h_mm * k) / 2.0 - lo.1 * k,
            ),
        }
    }

    /// The view that holds the whole machine volume (the table and `height_mm` over it) in `width` x `height` pixels.
    pub fn fit(
        view: StandardView,
        table: &Table,
        height_mm: f64,
        width: f64,
        height: f64,
        margin: f64,
    ) -> Ortho {
        Ortho::fit_box(
            view,
            (0.0, 0.0, 0.0),
            (table.width_mm, table.depth_mm, height_mm.max(0.0)),
            width,
            height,
            margin,
        )
    }
}

impl Iso {
    /// The `Iso` of a standard view that goes through the tilt formula (`Front`, `Side` and `Iso`): the turn and the tilt of
    /// [`StandardView::orbit_angles`] around the middle of the table, fitted as [`Iso::fit`] does. `Top` answers `None` because its
    /// true plan is [`Ortho`], not a tilt of 90 degrees.
    pub fn standard(
        view: StandardView,
        table: &Table,
        height_mm: f64,
        width: f64,
        height: f64,
    ) -> Option<Iso> {
        if view == StandardView::Top {
            return None;
        }
        let (yaw_deg, pitch_deg) = view.orbit_angles();
        let mut iso = Iso::fit(table, height_mm.max(1.0), width, height);
        iso.yaw_deg = yaw_deg;
        iso.pitch_deg = pitch_deg;
        iso.pivot = (table.width_mm / 2.0, table.depth_mm / 2.0);
        Some(iso)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(view: StandardView) -> Ortho {
        Ortho {
            view,
            k: 2.0,
            origin: (100.0, 50.0),
        }
    }

    fn near(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn the_view_from_above_has_x_to_the_right_and_y_up_and_ignores_the_height() {
        let v = at(StandardView::Top);
        assert_eq!(v.project(0.0, 0.0, 0.0), (100.0, 50.0));
        assert_eq!(v.project(10.0, 0.0, 0.0), (120.0, 50.0));
        // Y up on the machine is up on the screen: a smaller row
        assert_eq!(v.project(0.0, 10.0, 0.0), (100.0, 30.0));
        assert_eq!(v.project(10.0, 5.0, 0.0), v.project(10.0, 5.0, 99.0));
        assert_eq!(v.project(10.0, 5.0, -3.0), (120.0, 40.0));
    }

    #[test]
    fn the_front_and_the_side_see_the_height_and_not_the_depth() {
        let f = at(StandardView::Front);
        assert_eq!(f.project(10.0, 0.0, 0.0), (120.0, 50.0));
        assert_eq!(f.project(10.0, 0.0, 5.0), (120.0, 40.0));
        assert_eq!(f.project(10.0, 77.0, 5.0), f.project(10.0, -4.0, 5.0));
        let s = at(StandardView::Side);
        assert_eq!(s.project(0.0, 10.0, 0.0), (120.0, 50.0));
        assert_eq!(s.project(0.0, 10.0, 5.0), (120.0, 40.0));
        assert_eq!(s.project(31.0, 10.0, 5.0), s.project(-8.0, 10.0, 5.0));
    }

    #[test]
    fn the_isometric_view_is_the_one_of_iso_with_no_turn() {
        let o = Ortho {
            view: StandardView::Iso,
            k: 1.7,
            origin: (13.0, 21.0),
        };
        let i = Iso {
            k: 1.7,
            origin: (13.0, 21.0),
            yaw_deg: 0.0,
            pivot: (0.0, 0.0),
            pitch_deg: 30.0,
        };
        for p in [(0.0, 0.0, 0.0), (10.0, 20.0, 5.0), (495.0, 325.0, 170.0)] {
            assert!(near(o.project(p.0, p.1, p.2), i.project(p.0, p.1, p.2)));
        }
    }

    #[test]
    fn a_click_on_the_plan_gives_back_the_point_and_the_other_views_cannot() {
        let v = at(StandardView::Top);
        let (px, py) = v.project(123.4, 56.7, 0.0);
        let (x, y) = v.to_table(px, py).unwrap();
        assert!((x - 123.4).abs() < 1e-9 && (y - 56.7).abs() < 1e-9);
        assert_eq!(at(StandardView::Front).to_table(0.0, 0.0), None);
        assert_eq!(at(StandardView::Iso).to_table(0.0, 0.0), None);
    }

    #[test]
    fn the_orbit_angles_of_the_views_are_the_ones_in_use() {
        assert_eq!(StandardView::Front.orbit_angles(), (315.0, 0.0));
        assert_eq!(StandardView::Side.orbit_angles(), (225.0, 0.0));
        assert_eq!(StandardView::Iso.orbit_angles(), (0.0, 30.0));
        assert_eq!(StandardView::Top.orbit_angles(), (0.0, 90.0));
    }

    #[test]
    fn the_standard_iso_of_the_tilted_views_is_the_fitted_one_with_their_angles() {
        let t = Table::new(495.0, 325.0);
        assert!(Iso::standard(StandardView::Top, &t, 170.0, 480.0, 540.0).is_none());
        let plain = Iso::fit(&t, 170.0, 480.0, 540.0);
        let iso = Iso::standard(StandardView::Iso, &t, 170.0, 480.0, 540.0).unwrap();
        assert_eq!(
            (iso.k, iso.origin, iso.yaw_deg, iso.pitch_deg),
            (plain.k, plain.origin, 0.0, 30.0)
        );
        let f = Iso::standard(StandardView::Front, &t, 170.0, 480.0, 540.0).unwrap();
        assert_eq!(
            (f.yaw_deg, f.pitch_deg, f.pivot),
            (315.0, 0.0, (247.5, 162.5))
        );
        let s = Iso::standard(StandardView::Side, &t, 170.0, 480.0, 540.0).unwrap();
        assert_eq!((s.yaw_deg, s.pitch_deg), (225.0, 0.0));
    }

    #[test]
    fn each_view_fits_the_machine_volume_centred_with_its_margin() {
        for (w, d) in [(495.0, 325.0), (325.0, 270.0), (600.0, 400.0)] {
            let t = Table::new(w, d);
            for view in StandardView::ALL {
                let o = Ortho::fit(view, &t, 170.0, 480.0, 540.0, 20.0);
                let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
                for x in [0.0, w] {
                    for y in [0.0, d] {
                        for z in [0.0, 170.0] {
                            let p = o.project(x, y, z);
                            lo = (lo.0.min(p.0), lo.1.min(p.1));
                            hi = (hi.0.max(p.0), hi.1.max(p.1));
                        }
                    }
                }
                assert!(
                    lo.0 >= 20.0 - 1e-6 && lo.1 >= 20.0 - 1e-6,
                    "{view:?} {w}x{d}"
                );
                assert!(
                    hi.0 <= 460.0 + 1e-6 && hi.1 <= 520.0 + 1e-6,
                    "{view:?} {w}x{d}"
                );
                // centred, and one direction touches its margin
                assert!(((lo.0 + hi.0) / 2.0 - 240.0).abs() < 1e-6, "{view:?}");
                assert!(((lo.1 + hi.1) / 2.0 - 270.0).abs() < 1e-6, "{view:?}");
                let touches = (lo.0 - 20.0).abs() < 1e-6 || (lo.1 - 20.0).abs() < 1e-6;
                assert!(touches, "{view:?} {w}x{d}");
            }
        }
    }

    #[test]
    fn the_plan_of_a_known_table_fits_to_exact_numbers() {
        // 400 x 200 mm in 440 x 240 pixels with 20 of margin: 1 pixel per millimetre, no free space
        let o = Ortho::fit(
            StandardView::Top,
            &Table::new(400.0, 200.0),
            50.0,
            440.0,
            240.0,
            20.0,
        );
        assert_eq!(o.k, 1.0);
        assert_eq!(o.project(0.0, 0.0, 0.0), (20.0, 220.0));
        assert_eq!(o.project(400.0, 200.0, 0.0), (420.0, 20.0));
        // the front of the same machine: 400 wide and 50 high, the width decides
        let f = Ortho::fit(
            StandardView::Front,
            &Table::new(400.0, 200.0),
            50.0,
            440.0,
            240.0,
            20.0,
        );
        assert_eq!(f.k, 1.0);
        assert_eq!(f.project(0.0, 0.0, 0.0), (20.0, 145.0));
        assert_eq!(f.project(400.0, 0.0, 50.0), (420.0, 95.0));
    }

    #[test]
    fn a_box_without_extent_or_a_tiny_area_keeps_a_sane_scale() {
        let o = Ortho::fit_box(
            StandardView::Top,
            (5.0, 5.0, 0.0),
            (5.0, 5.0, 0.0),
            100.0,
            100.0,
            10.0,
        );
        assert_eq!(o.k, 1.0);
        assert_eq!(o.project(5.0, 5.0, 0.0), (50.0, 50.0));
        let t = Ortho::fit_box(
            StandardView::Top,
            (0.0, 0.0, 0.0),
            (10.0, 10.0, 0.0),
            10.0,
            10.0,
            10.0,
        );
        assert_eq!(t.k, 1.0);
    }

    #[test]
    fn the_defaults_of_the_tilted_projection_do_not_change() {
        // golden values of `Iso::project` with the default view (no turn, 30 degrees)
        let i = Iso::fit(&Table::new(495.0, 325.0), 170.0, 480.0, 540.0);
        let p = i.project(100.0, 50.0, 20.0);
        let c = 30f64.to_radians().cos();
        let expect = (
            i.origin.0 + 50.0 * c * i.k,
            i.origin.1 + 150.0 * 0.5 * i.k - 20.0 * i.k,
        );
        assert!(near(p, expect));
        assert_eq!((i.yaw_deg, i.pitch_deg, i.pivot), (0.0, 30.0, (0.0, 0.0)));
    }
}
