//! The check of the holes after a drill job. For each hole of the job the picture is looked at where the hole should be: is there a hole, how wide is it, how far is its centre
//! from the place of the job, is it round, and is the ring around it clean. The positions of the job come in millimetres from the centre of the picture (x to the right, y up).

use crate::blobs::{blobs, threshold};
use crate::image::{otsu, Gray};
use crate::profile::{Polarity, Profile};

/// A hole of the job, where the camera should see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpectedHole {
    /// From the centre of the picture, mm, x to the right and y up.
    pub x_mm: f64,
    pub y_mm: f64,
    /// The width of the hole in the job, mm.
    pub dia_mm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoleIssue {
    /// The place of the hole is not in the picture.
    OutOfView,
    /// No hole where it should be.
    Missing,
    WrongSize,
    OffCentre,
    NotRound,
    /// Burrs or dust around the hole.
    Debris,
}

impl HoleIssue {
    pub fn code(self) -> &'static str {
        match self {
            HoleIssue::OutOfView => "hole_out_of_view",
            HoleIssue::Missing => "hole_missing",
            HoleIssue::WrongSize => "hole_wrong_size",
            HoleIssue::OffCentre => "hole_off_centre",
            HoleIssue::NotRound => "hole_not_round",
            HoleIssue::Debris => "hole_debris",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleResult {
    pub index: usize,
    pub expected: ExpectedHole,
    pub found: bool,
    /// Where the hole was found, in pixels of the picture.
    pub cx_px: f64,
    pub cy_px: f64,
    /// The width found, mm.
    pub dia_mm: f64,
    /// How far from the place of the job, mm.
    pub offset_mm: f64,
    /// 1 is a circle.
    pub roundness: f64,
    /// The share of the ring around the hole that is as dark as the hole.
    pub debris: f64,
    pub issues: Vec<HoleIssue>,
}

impl HoleResult {
    pub fn ok(&self) -> bool {
        self.issues.is_empty()
    }
}

fn empty(index: usize, e: ExpectedHole, cx: f64, cy: f64, issue: HoleIssue) -> HoleResult {
    HoleResult {
        index,
        expected: e,
        found: false,
        cx_px: cx,
        cy_px: cy,
        dia_mm: 0.0,
        offset_mm: 0.0,
        roundness: 0.0,
        debris: 0.0,
        issues: vec![issue],
    }
}

/// The picture is judged at the place of each hole. With `Polarity::Auto` the inside of the hole is taken as the darker part when the middle of the place is darker than the
/// ring around it, and as the lighter when it is lighter (a hole over a lit table, or a backlit board).
pub fn check_holes(img: &Gray, expected: &[ExpectedHole], p: &Profile) -> Vec<HoleResult> {
    let ppm = p.px_per_mm;
    let (icx, icy) = (img.w as f64 / 2.0, img.h as f64 / 2.0);
    expected
        .iter()
        .enumerate()
        .map(|(index, e)| check_one(img, index, *e, icx + e.x_mm * ppm, icy - e.y_mm * ppm, p))
        .collect()
}

fn check_one(
    img: &Gray,
    index: usize,
    e: ExpectedHole,
    ex: f64,
    ey: f64,
    p: &Profile,
) -> HoleResult {
    let ppm = p.px_per_mm;
    let r_nom = (e.dia_mm / 2.0 * ppm).max(1.0);
    // the window to look in: the hole, the room its centre may be off by, and the ring around it for the debris
    let reach = (r_nom + p.hole_pos_tol_mm * ppm) * 1.7 + 4.0;
    let (x0, y0) = ((ex - reach).floor(), (ey - reach).floor());
    let (x1, y1) = ((ex + reach).ceil(), (ey + reach).ceil());
    if ppm <= 0.0 || x0 < 0.0 || y0 < 0.0 || x1 >= img.w as f64 || y1 >= img.h as f64 {
        return empty(index, e, ex, ey, HoleIssue::OutOfView);
    }
    let (x0, y0, x1, y1) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);
    let (ww, wh) = (x1 - x0 + 1, y1 - y0 + 1);
    let win = img.window(x0, y0, x1 + 1, y1 + 1);

    // which part is the hole: darker or lighter
    let mean_in = |lo: f64, hi: f64| {
        let (mut s, mut n) = (0f64, 0f64);
        for y in 0..wh {
            for x in 0..ww {
                let d = ((x0 + x) as f64 - ex).hypot((y0 + y) as f64 - ey);
                if d >= lo && d < hi {
                    s += win[y * ww + x] as f64;
                    n += 1.0;
                }
            }
        }
        if n > 0.0 {
            s / n
        } else {
            0.0
        }
    };
    let dark = match p.hole_polarity {
        Polarity::Dark => true,
        Polarity::Bright => false,
        Polarity::Auto => mean_in(0.0, r_nom * 0.6) <= mean_in(r_nom * 1.3, r_nom * 1.9),
    };
    // the grey that splits the hole from what is around it: halfway between the middle of the place and the ring around it when they differ enough (the window may hold
    // copper, bare board and the hole, three greys, and the threshold of Otsu would split the wrong two); otherwise the threshold of Otsu
    let (inside, ring) = (
        mean_in(0.0, r_nom * 0.5),
        mean_in(r_nom * 1.25, r_nom * 1.7),
    );
    let level = if (inside - ring).abs() >= 25.0 {
        ((inside + ring) / 2.0).round().clamp(1.0, 255.0) as u8
    } else {
        otsu(&win)
    };
    let mask = threshold(&win, ww, wh, level, dark);
    // the patch whose centre is nearest to the place of the hole, big enough to be a hole and not touching the window (that would be the whole board)
    let min_area = ((r_nom * 0.35).powi(2) * std::f64::consts::PI).max(3.0) as usize;
    let found = blobs(&mask, min_area)
        .into_iter()
        .filter(|b| !b.touches_edge && b.fill >= 0.55)
        .min_by(|a, b| {
            let da = (a.cx + x0 as f64 - ex).hypot(a.cy + y0 as f64 - ey);
            let db = (b.cx + x0 as f64 - ex).hypot(b.cy + y0 as f64 - ey);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
    let Some(b) = found else {
        return empty(index, e, ex, ey, HoleIssue::Missing);
    };
    let (cx, cy) = (b.cx + x0 as f64, b.cy + y0 as f64);
    let r_found = (b.area as f64 / std::f64::consts::PI).sqrt();
    let dia_mm = 2.0 * r_found / ppm;
    let offset_mm = (cx - ex).hypot(cy - ey) / ppm;

    // the ring around the hole: how much of it is as dark (or light) as the hole itself
    let (mut ring, mut dirty) = (0f64, 0f64);
    for y in 0..wh {
        for x in 0..ww {
            let d = ((x0 + x) as f64 - cx).hypot((y0 + y) as f64 - cy);
            if d >= r_found * 1.25 && d <= r_found * 1.7 {
                ring += 1.0;
                if mask.get(x, y) {
                    dirty += 1.0;
                }
            }
        }
    }
    let debris = if ring > 0.0 { dirty / ring } else { 0.0 };

    let mut issues = Vec::new();
    // a hole that is far off is more likely another one than this one: say it is off the place
    if (dia_mm - e.dia_mm).abs() > p.hole_dia_tol_mm {
        issues.push(HoleIssue::WrongSize);
    }
    if offset_mm > p.hole_pos_tol_mm {
        issues.push(HoleIssue::OffCentre);
    }
    if b.roundness < p.hole_min_roundness {
        issues.push(HoleIssue::NotRound);
    }
    if debris > p.hole_max_debris {
        issues.push(HoleIssue::Debris);
    }
    HoleResult {
        index,
        expected: e,
        found: true,
        cx_px: cx,
        cy_px: cy,
        dia_mm,
        offset_mm,
        roundness: b.roundness,
        debris,
        issues,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const PPM: f64 = 50.0;

    /// A board of grey `board` with discs of grey `hole`: (x_mm, y_mm, diameter_mm) from the centre of the picture.
    pub(crate) fn board(
        w: usize,
        h: usize,
        board: u8,
        hole: u8,
        discs: &[(f64, f64, f64)],
    ) -> Gray {
        let mut g = Gray::new(w, h, board);
        for y in 0..h {
            for x in 0..w {
                for &(mx, my, d) in discs {
                    let (cx, cy) = (w as f64 / 2.0 + mx * PPM, h as f64 / 2.0 - my * PPM);
                    if (x as f64 - cx).hypot(y as f64 - cy) <= d / 2.0 * PPM {
                        g.set(x, y, hole);
                    }
                }
            }
        }
        g
    }

    fn profile() -> Profile {
        Profile {
            px_per_mm: PPM,
            ..Profile::default()
        }
    }

    fn hole(x: f64, y: f64, d: f64) -> ExpectedHole {
        ExpectedHole {
            x_mm: x,
            y_mm: y,
            dia_mm: d,
        }
    }

    #[test]
    fn a_good_hole_passes_with_its_size_and_place() {
        let img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8), (1.5, 1.0, 1.0)]);
        let r = check_holes(
            &img,
            &[hole(0.0, 0.0, 0.8), hole(1.5, 1.0, 1.0)],
            &profile(),
        );
        assert!(r.iter().all(|h| h.ok() && h.found), "{r:?}");
        assert!((r[0].dia_mm - 0.8).abs() < 0.06 && r[0].offset_mm < 0.05 && r[0].roundness > 0.95);
        assert!((r[1].dia_mm - 1.0).abs() < 0.06);
    }

    #[test]
    fn a_hole_that_is_not_there_is_missing() {
        let img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8)]);
        let r = check_holes(
            &img,
            &[hole(0.0, 0.0, 0.8), hole(1.5, 1.0, 0.8)],
            &profile(),
        );
        assert!(r[0].ok());
        assert_eq!(r[1].issues, vec![HoleIssue::Missing]);
        assert!(!r[1].found);
    }

    #[test]
    fn the_wrong_size_and_the_wrong_place_are_told_apart() {
        // a 1.0 mm hole where 0.8 was asked; and a 0.8 hole 0.4 mm to the side
        let img = board(400, 240, 170, 25, &[(0.0, 0.0, 1.0), (1.5, 0.0, 0.8)]);
        let r = check_holes(
            &img,
            &[hole(0.0, 0.0, 0.8), hole(1.5, 0.4, 0.8)],
            &profile(),
        );
        assert_eq!(r[0].issues, vec![HoleIssue::WrongSize]);
        assert!((r[0].dia_mm - 1.0).abs() < 0.06);
        assert_eq!(r[1].issues, vec![HoleIssue::OffCentre], "{:?}", r[1]);
        assert!((r[1].offset_mm - 0.4).abs() < 0.06);
        // the tolerance is the profile's: loosened, both pass
        let loose = Profile {
            hole_dia_tol_mm: 0.3,
            hole_pos_tol_mm: 0.6,
            ..profile()
        };
        assert!(
            check_holes(&img, &[hole(0.0, 0.0, 0.8), hole(1.5, 0.4, 0.8)], &loose)
                .iter()
                .all(|h| h.ok())
        );
    }

    #[test]
    fn an_oval_or_a_slot_is_not_round() {
        let mut img = Gray::new(320, 240, 170);
        // a slot: 1.4 mm wide and 0.6 high
        for y in 0..240 {
            for x in 0..320 {
                let (dx, dy) = (
                    (x as f64 - 160.0) / (0.7 * PPM),
                    (y as f64 - 120.0) / (0.3 * PPM),
                );
                if dx * dx + dy * dy <= 1.0 {
                    img.set(x, y, 25);
                }
            }
        }
        let r = check_holes(&img, &[hole(0.0, 0.0, 0.9)], &profile());
        assert!(
            r[0].found && r[0].issues.contains(&HoleIssue::NotRound),
            "{:?}",
            r[0]
        );
    }

    #[test]
    fn dust_around_the_hole_is_debris() {
        let mut img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8)]);
        // dark flecks on the ring at 1.4 to 1.7 of the radius, a third of it
        let r_hole = 0.4 * PPM;
        for y in 0..240 {
            for x in 0..320 {
                let d = (x as f64 - 160.0).hypot(y as f64 - 120.0);
                let angle = (y as f64 - 120.0).atan2(x as f64 - 160.0);
                if d > r_hole * 1.3 && d < r_hole * 1.6 && angle.sin() > 0.5 {
                    img.set(x, y, 25);
                }
            }
        }
        let r = check_holes(&img, &[hole(0.0, 0.0, 0.8)], &profile());
        assert!(r[0].found, "{:?}", r[0]);
        assert!(
            r[0].issues.contains(&HoleIssue::Debris) || r[0].issues.contains(&HoleIssue::NotRound),
            "{:?}",
            r[0]
        );
    }

    #[test]
    fn a_light_hole_over_a_dark_board_is_found_too() {
        let img = board(320, 240, 40, 220, &[(0.0, 0.0, 0.8)]);
        assert!(check_holes(&img, &[hole(0.0, 0.0, 0.8)], &profile())[0].ok());
        // and the profile can say which it is
        let dark = Profile {
            hole_polarity: Polarity::Dark,
            ..profile()
        };
        assert_eq!(
            check_holes(&img, &[hole(0.0, 0.0, 0.8)], &dark)[0].issues,
            vec![HoleIssue::Missing]
        );
    }

    #[test]
    fn a_place_outside_the_picture_or_without_a_scale_cannot_be_judged() {
        let img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8)]);
        let r = check_holes(&img, &[hole(20.0, 0.0, 0.8)], &profile());
        assert_eq!(r[0].issues, vec![HoleIssue::OutOfView]);
        let uncalibrated = Profile::default();
        assert_eq!(
            check_holes(&img, &[hole(0.0, 0.0, 0.8)], &uncalibrated)[0].issues,
            vec![HoleIssue::OutOfView]
        );
        assert_eq!(HoleIssue::Debris.code(), "hole_debris");
    }

    /// Copper with a milled ring around a pad, and a hole in the pad (when `hole`): three greys in one window.
    fn pad_with_ring(hole: bool) -> Gray {
        let mut img = Gray::new(320, 240, 165);
        for y in 0..240 {
            for x in 0..320 {
                let d = (x as f64 - 160.0).hypot(y as f64 - 120.0);
                if d > 0.95 * PPM && d <= 1.15 * PPM {
                    img.set(x, y, 98); // the milled channel: bare board
                }
                if hole && d <= 0.4 * PPM {
                    img.set(x, y, 8);
                }
            }
        }
        img
    }

    #[test]
    fn a_hole_inside_a_milled_ring_is_measured_not_confused_with_it() {
        let r = check_holes(&pad_with_ring(true), &[hole(0.0, 0.0, 0.8)], &profile());
        assert!(r[0].ok(), "{:?}", r[0]);
        assert!((r[0].dia_mm - 0.8).abs() < 0.06);
        // without the hole the ring must not be taken for it
        let r = check_holes(&pad_with_ring(false), &[hole(0.0, 0.0, 0.8)], &profile());
        assert_eq!(r[0].issues, vec![HoleIssue::Missing], "{:?}", r[0]);
    }

    #[test]
    fn noise_does_not_break_a_good_hole() {
        let mut img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8)]);
        let mut seed = 12345u32;
        for v in img.px.iter_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let n = ((seed >> 24) as i32 % 21) - 10;
            *v = (*v as i32 + n).clamp(0, 255) as u8;
        }
        let r = check_holes(&img, &[hole(0.0, 0.0, 0.8)], &profile());
        assert!(r[0].ok(), "{:?}", r[0]);
    }
}
