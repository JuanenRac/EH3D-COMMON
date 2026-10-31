//! Is the picture good enough to judge anything? A check made on a dark, washed out or blurred picture says nothing, so the picture is looked at first: how bright it is, how
//! much contrast it has and how sharp the edges are. When it is not good the answer is "unknown" and a hint for the ring of LEDs: more light or less.

use crate::image::Gray;
use crate::profile::Profile;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageQuality {
    /// The mean grey, 0 to 255.
    pub mean: f64,
    /// The spread of the greys (standard deviation).
    pub contrast: f64,
    /// The variance of the Laplacian: high when the edges are sharp, low when the picture is out of focus.
    pub sharpness: f64,
    /// The share of pixels that are black / white to the limit (lost detail).
    pub clipped_dark: f64,
    pub clipped_bright: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityIssue {
    TooDark,
    TooBright,
    LowContrast,
    Blurry,
}

impl QualityIssue {
    pub fn code(self) -> &'static str {
        match self {
            QualityIssue::TooDark => "image_too_dark",
            QualityIssue::TooBright => "image_too_bright",
            QualityIssue::LowContrast => "image_low_contrast",
            QualityIssue::Blurry => "image_blurry",
        }
    }
}

/// The numbers of a picture.
pub fn measure(img: &Gray) -> ImageQuality {
    let n = (img.w * img.h).max(1) as f64;
    let mean = img.mean();
    let var = img
        .px
        .iter()
        .map(|&v| (v as f64 - mean).powi(2))
        .sum::<f64>()
        / n;
    let dark = img.px.iter().filter(|&&v| v <= 3).count() as f64 / n;
    let bright = img.px.iter().filter(|&&v| v >= 252).count() as f64 / n;
    // the Laplacian with the four neighbours, on the inside of the picture
    let (mut sum, mut sum2, mut count) = (0f64, 0f64, 0f64);
    if img.w >= 3 && img.h >= 3 {
        for y in 1..img.h - 1 {
            for x in 1..img.w - 1 {
                let l = 4.0 * img.at(x, y) as f64
                    - img.at(x - 1, y) as f64
                    - img.at(x + 1, y) as f64
                    - img.at(x, y - 1) as f64
                    - img.at(x, y + 1) as f64;
                sum += l;
                sum2 += l * l;
                count += 1.0;
            }
        }
    }
    let sharpness = if count > 0.0 {
        sum2 / count - (sum / count).powi(2)
    } else {
        0.0
    };
    ImageQuality {
        mean,
        contrast: var.sqrt(),
        sharpness,
        clipped_dark: dark,
        clipped_bright: bright,
    }
}

/// The numbers of the picture and what is wrong with it for this profile.
pub fn assess(img: &Gray, p: &Profile) -> (ImageQuality, Vec<QualityIssue>) {
    let q = measure(img);
    let mut issues = Vec::new();
    if q.mean < p.min_mean {
        issues.push(QualityIssue::TooDark);
    }
    if q.mean > p.max_mean || q.clipped_bright > 0.25 {
        issues.push(QualityIssue::TooBright);
    }
    if q.contrast < p.min_contrast {
        issues.push(QualityIssue::LowContrast);
    }
    if q.sharpness < p.min_sharpness {
        issues.push(QualityIssue::Blurry);
    }
    (q, issues)
}

/// The level of the ring of LEDs (0 to 100) that should bring the picture to a good brightness, from the level it had and the mean grey it gave, or `None` when it is fine as
/// it is. The camera answers roughly with the square of the level (the ring follows the eye), so the level is moved by the square root of the ratio.
pub fn suggest_light_level(
    current_level: u32,
    q: &ImageQuality,
    target_mean: f64,
    ceiling: u32,
) -> Option<u32> {
    if current_level == 0 || q.mean < 1.0 {
        return Some(ceiling.min(60));
    }
    if (q.mean - target_mean).abs() <= 25.0 {
        return None;
    }
    let ratio = (target_mean / q.mean).clamp(0.25, 4.0).sqrt();
    let wanted = ((current_level as f64) * ratio)
        .round()
        .clamp(1.0, ceiling as f64) as u32;
    (wanted != current_level).then_some(wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture with a checkerboard of two greys: `a` and `b`.
    fn checker(a: u8, b: u8, cell: usize) -> Gray {
        let mut g = Gray::new(64, 64, a);
        for y in 0..64 {
            for x in 0..64 {
                if (x / cell + y / cell) % 2 == 1 {
                    g.set(x, y, b);
                }
            }
        }
        g
    }

    #[test]
    fn a_good_picture_has_nothing_wrong() {
        let (q, issues) = assess(&checker(60, 190, 8), &Profile::default());
        assert!(issues.is_empty(), "{issues:?} {q:?}");
        assert!((q.mean - 125.0).abs() < 1.0 && q.contrast > 50.0 && q.sharpness > 100.0);
    }

    #[test]
    fn dark_bright_flat_and_blurred_pictures_are_named() {
        let p = Profile::default();
        assert!(assess(&Gray::new(32, 32, 10), &p)
            .1
            .contains(&QualityIssue::TooDark));
        assert!(assess(&Gray::new(32, 32, 250), &p)
            .1
            .contains(&QualityIssue::TooBright));
        assert!(assess(&Gray::new(32, 32, 120), &p)
            .1
            .contains(&QualityIssue::LowContrast));
        // a smooth ramp: contrast but no edges
        let mut ramp = Gray::new(64, 64, 0);
        for y in 0..64 {
            for x in 0..64 {
                ramp.set(x, y, (60 + x * 2) as u8);
            }
        }
        let (_, issues) = assess(&ramp, &p);
        assert!(
            issues.contains(&QualityIssue::Blurry) && !issues.contains(&QualityIssue::LowContrast),
            "{issues:?}"
        );
        // bright with a lot of burnt out pixels is too bright even when the mean is fine
        let mut burnt = Gray::new(40, 40, 100);
        for y in 0..40 {
            for x in 0..14 {
                burnt.set(x, y, 255);
            }
        }
        assert!(assess(&burnt, &p).1.contains(&QualityIssue::TooBright));
        assert_eq!(QualityIssue::Blurry.code(), "image_blurry");
    }

    #[test]
    fn the_light_is_moved_toward_a_good_brightness() {
        let dark = ImageQuality {
            mean: 30.0,
            contrast: 20.0,
            sharpness: 100.0,
            clipped_dark: 0.0,
            clipped_bright: 0.0,
        };
        let more = suggest_light_level(40, &dark, 125.0, 80).unwrap();
        assert!(more > 40 && more <= 80);
        let bright = ImageQuality {
            mean: 230.0,
            ..dark
        };
        let less = suggest_light_level(60, &bright, 125.0, 80).unwrap();
        assert!((1..60).contains(&less));
        let fine = ImageQuality {
            mean: 120.0,
            ..dark
        };
        assert_eq!(suggest_light_level(60, &fine, 125.0, 80), None);
        // with the light off, or nothing seen at all, it asks for a normal level, never over the ceiling
        assert_eq!(suggest_light_level(0, &fine, 125.0, 80), Some(60));
        assert_eq!(
            suggest_light_level(20, &ImageQuality { mean: 0.0, ..dark }, 125.0, 40),
            Some(40)
        );
    }
}
