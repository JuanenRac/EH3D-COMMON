//! The check of the milling after a routing job. The router leaves channels of bare board between the copper of one track and the next. The copper is told from the board by a
//! threshold, the distance of every board pixel to the nearest copper is worked out exactly, and the middle of each channel is where that distance is greatest: twice the
//! distance there is the width of the channel. From the widths along all the channels come their middle value, how uneven they are and where they pinch (a pinch is where a
//! short is likely).

use crate::blobs::{blobs, Mask};
use crate::edt::distance_to_off;
use crate::image::{otsu, Gray};
use crate::profile::{Polarity, Profile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MillingIssue {
    /// No channel could be found: nothing was milled, or the threshold does not split copper from board.
    NoChannels,
    /// The channels are narrower than the tool should have made.
    TooNarrow,
    /// The channels are wider than the tool should have made.
    TooWide,
    /// A channel pinches below the least width: copper may touch.
    Pinched,
    /// The widths vary too much along the channels.
    Uneven,
}

impl MillingIssue {
    pub fn code(self) -> &'static str {
        match self {
            MillingIssue::NoChannels => "mill_no_channels",
            MillingIssue::TooNarrow => "mill_too_narrow",
            MillingIssue::TooWide => "mill_too_wide",
            MillingIssue::Pinched => "mill_pinched",
            MillingIssue::Uneven => "mill_uneven",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MillingResult {
    /// The middle width of the channels, mm.
    pub width_mm: f64,
    /// The narrow and the wide end of the widths (10th and 90th percentile), mm.
    pub width_p10_mm: f64,
    pub width_p90_mm: f64,
    /// The gap between those two as a share of the middle width.
    pub spread: f64,
    /// The places where a channel is narrower than the least width.
    pub pinches: usize,
    /// The share of the picture that is copper.
    pub copper_share: f64,
    /// The points of the middle lines of the channels that were measured.
    pub samples: usize,
    pub issues: Vec<MillingIssue>,
}

impl MillingResult {
    pub fn ok(&self) -> bool {
        self.issues.is_empty()
    }
}

const MIN_SAMPLES: usize = 20;

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[i]
}

/// The channels of the picture, judged by the profile.
pub fn check_milling(img: &Gray, p: &Profile) -> MillingResult {
    let ppm = p.px_per_mm;
    let none = |share: f64| MillingResult {
        width_mm: 0.0,
        width_p10_mm: 0.0,
        width_p90_mm: 0.0,
        spread: 0.0,
        pinches: 0,
        copper_share: share,
        samples: 0,
        issues: vec![MillingIssue::NoChannels],
    };
    if ppm <= 0.0 || img.w < 5 || img.h < 5 {
        return none(0.0);
    }
    let level = if p.mill_threshold > 0 {
        p.mill_threshold
    } else {
        otsu(&img.px)
    };
    let copper_bright = p.mill_copper != Polarity::Dark;
    let mut copper = Mask::new(img.w, img.h);
    for (i, &v) in img.px.iter().enumerate() {
        copper.on[i] = if copper_bright { v >= level } else { v < level };
    }
    let share = copper.count() as f64 / (img.w * img.h) as f64;
    // the board is what is not copper; its distance to the copper is what is measured
    let mut board = Mask::new(img.w, img.h);
    for (b, c) in board.on.iter_mut().zip(copper.on.iter()) {
        *b = !*c;
    }
    if copper.count() == 0 {
        return none(share);
    }
    let dist = distance_to_off(&board);
    let (w, h) = (img.w, img.h);
    // the pixel is half a pixel wide on each side of its centre, and the nearest copper pixel is also a pixel: the gap between the edges is 2 d - 1
    let cap_px = p.mill_max_channel_mm * ppm;
    let border = 3usize;
    let mut ridge = Mask::new(w, h);
    let mut widths: Vec<f64> = Vec::new();
    for y in border..h.saturating_sub(border) {
        for x in border..w.saturating_sub(border) {
            let d = dist[y * w + x];
            if d < 1.0 || 2.0 * d - 1.0 > cap_px {
                continue;
            }
            // a ridge: no neighbour is farther from the copper
            let mut top = true;
            'n: for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if (dx != 0 || dy != 0)
                        && dist[(y as i32 + dy) as usize * w + (x as i32 + dx) as usize] > d + 1e-9
                    {
                        top = false;
                        break 'n;
                    }
                }
            }
            if top {
                ridge.set(x, y, true);
                widths.push(((2.0 * d - 1.0) / ppm).max(0.0));
            }
        }
    }
    if widths.len() < MIN_SAMPLES {
        return none(share);
    }
    widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (p10, med, p90) = (
        percentile(&widths, 0.10),
        percentile(&widths, 0.5),
        percentile(&widths, 0.90),
    );
    let spread = if med > 0.0 { (p90 - p10) / med } else { 0.0 };
    // the pinches: the patches of the middle line where the width is under the least
    let mut narrow = Mask::new(w, h);
    for y in 0..h {
        for x in 0..w {
            if ridge.get(x, y) && (2.0 * dist[y * w + x] - 1.0) / ppm < p.mill_min_width_mm {
                narrow.set(x, y, true);
            }
        }
    }
    let pinches = blobs(&narrow, 2).len();

    let mut issues = Vec::new();
    if med < p.mill_width_mm * (1.0 - p.mill_width_tol) {
        issues.push(MillingIssue::TooNarrow);
    }
    if med > p.mill_width_mm * (1.0 + p.mill_width_tol) {
        issues.push(MillingIssue::TooWide);
    }
    if pinches > 0 {
        issues.push(MillingIssue::Pinched);
    }
    if spread > p.mill_max_spread {
        issues.push(MillingIssue::Uneven);
    }
    MillingResult {
        width_mm: med,
        width_p10_mm: p10,
        width_p90_mm: p90,
        spread,
        pinches,
        copper_share: share,
        samples: widths.len(),
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PPM: f64 = 50.0;

    /// Copper (bright) with horizontal channels of bare board: (y_mm centre, width_mm).
    fn plane(channels: &[(f64, f64)]) -> Gray {
        let (w, h) = (300usize, 200usize);
        let mut g = Gray::new(w, h, 200);
        for &(yc, wd) in channels {
            let cy = h as f64 / 2.0 - yc * PPM;
            for y in 0..h {
                if (y as f64 - cy).abs() <= wd / 2.0 * PPM {
                    for x in 0..w {
                        g.set(x, y, 60);
                    }
                }
            }
        }
        g
    }

    fn profile(width: f64) -> Profile {
        Profile {
            px_per_mm: PPM,
            mill_width_mm: width,
            ..Profile::default()
        }
    }

    #[test]
    fn channels_of_the_right_width_pass() {
        let img = plane(&[(-0.8, 0.2), (0.0, 0.2), (0.8, 0.2)]);
        let r = check_milling(&img, &profile(0.2));
        assert!(r.ok(), "{r:?}");
        assert!((r.width_mm - 0.2).abs() < 0.03, "{}", r.width_mm);
        assert!(r.pinches == 0 && r.spread < 0.3 && r.samples > 100);
        assert!(r.copper_share > 0.5);
    }

    #[test]
    fn channels_that_are_too_narrow_or_too_wide_are_told() {
        let thin = check_milling(&plane(&[(0.0, 0.1), (0.8, 0.1)]), &profile(0.2));
        assert!(thin.issues.contains(&MillingIssue::TooNarrow), "{thin:?}");
        let wide = check_milling(&plane(&[(0.0, 0.4), (0.9, 0.4)]), &profile(0.2));
        assert!(wide.issues.contains(&MillingIssue::TooWide), "{wide:?}");
        // the tolerance belongs to the profile
        let loose = Profile {
            mill_width_tol: 1.5,
            ..profile(0.2)
        };
        assert!(!check_milling(&plane(&[(0.0, 0.4), (0.9, 0.4)]), &loose)
            .issues
            .contains(&MillingIssue::TooWide));
    }

    #[test]
    fn a_channel_that_pinches_is_a_danger_of_a_short() {
        let mut img = plane(&[(0.0, 0.2), (0.9, 0.2)]);
        // copper creeps in from the top edge of the first channel for 12 pixels
        for x in 140..152 {
            for y in 93..103 {
                img.set(x, y, 200);
            }
        }
        let r = check_milling(&img, &profile(0.2));
        assert!(r.issues.contains(&MillingIssue::Pinched), "{r:?}");
        assert!(r.pinches >= 1);
    }

    #[test]
    fn an_uneven_channel_is_told() {
        let mut img = plane(&[(0.0, 0.2), (0.9, 0.2)]);
        // half of the first channel is twice as wide
        for x in 0..150 {
            for y in 90..110 {
                img.set(x, y, 60);
            }
        }
        let r = check_milling(&img, &profile(0.2));
        assert!(r.issues.contains(&MillingIssue::Uneven), "{r:?}");
        assert!(r.width_p90_mm > r.width_p10_mm * 1.3);
    }

    #[test]
    fn nothing_milled_or_nothing_known_gives_no_channels() {
        let flat = Gray::new(300, 200, 200);
        assert_eq!(
            check_milling(&flat, &profile(0.2)).issues,
            vec![MillingIssue::NoChannels]
        );
        let uncalibrated = Profile::default();
        assert_eq!(
            check_milling(&plane(&[(0.0, 0.2)]), &uncalibrated).issues,
            vec![MillingIssue::NoChannels]
        );
        assert_eq!(MillingIssue::Pinched.code(), "mill_pinched");
    }

    #[test]
    fn dark_copper_is_handled_by_the_profile() {
        let mut img = plane(&[(0.0, 0.2), (0.8, 0.2), (-0.8, 0.2)]);
        for v in img.px.iter_mut() {
            *v = 255 - *v;
        }
        let dark = Profile {
            mill_copper: Polarity::Dark,
            ..profile(0.2)
        };
        assert!(check_milling(&img, &dark).ok());
    }

    #[test]
    fn a_threshold_of_the_profile_beats_the_automatic_one() {
        let img = plane(&[(0.0, 0.2), (0.8, 0.2)]);
        let fixed = Profile {
            mill_threshold: 130,
            ..profile(0.2)
        };
        assert!(
            (check_milling(&img, &fixed).width_mm - check_milling(&img, &profile(0.2)).width_mm)
                .abs()
                < 0.02
        );
    }
}
