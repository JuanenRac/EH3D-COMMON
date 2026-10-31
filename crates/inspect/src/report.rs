//! One run of the analyser: the picture is looked at, the checks the profile asks for are made, and the answer is a verdict (ok, not ok, or unknown when the picture was too poor
//! to say) with the findings that led to it. The findings carry codes, not sentences, so each program says them in the language of the person at the machine.

use serde_json::{json, Value};

use crate::holes::{check_holes, ExpectedHole, HoleResult};
use crate::image::Gray;
use crate::milling::{check_milling, MillingResult};
use crate::profile::Profile;
use crate::quality::{assess, ImageQuality};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    NotOk,
    /// The picture, or what was asked, did not allow a judgement.
    Unknown,
}

impl Verdict {
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::NotOk => "not_ok",
            Verdict::Unknown => "unknown",
        }
    }
}

/// One thing the analyser found.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// What it is: `hole_missing`, `hole_wrong_size`, `mill_pinched`, `image_too_dark`, `not_calibrated` ...
    pub code: &'static str,
    /// The hole it is about (its number in the list that was checked), when it is about one.
    pub hole: Option<usize>,
    /// What was found and the limit it broke, in the unit of the check (mm, a share), when there are such numbers.
    pub value: f64,
    pub limit: f64,
    /// It makes the verdict not ok (false: it only prevents a judgement or is a note).
    pub fails: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub verdict: Verdict,
    pub profile: String,
    pub quality: Option<ImageQuality>,
    pub holes: Vec<HoleResult>,
    pub milling: Option<MillingResult>,
    pub findings: Vec<Finding>,
    pub width_px: usize,
    pub height_px: usize,
}

impl Report {
    pub fn holes_ok(&self) -> usize {
        self.holes.iter().filter(|h| h.ok()).count()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "verdict": self.verdict.word(),
            "profile": self.profile,
            "width_px": self.width_px,
            "height_px": self.height_px,
            "quality": self.quality.map(|q| json!({
                "mean": q.mean, "contrast": q.contrast, "sharpness": q.sharpness,
                "clipped_dark": q.clipped_dark, "clipped_bright": q.clipped_bright,
            })),
            "holes": self.holes.iter().map(|h| json!({
                "index": h.index, "x_mm": h.expected.x_mm, "y_mm": h.expected.y_mm, "expected_dia_mm": h.expected.dia_mm,
                "found": h.found, "cx_px": h.cx_px, "cy_px": h.cy_px, "dia_mm": h.dia_mm, "offset_mm": h.offset_mm,
                "roundness": h.roundness, "debris": h.debris,
                "ok": h.ok(), "issues": h.issues.iter().map(|i| i.code()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "milling": self.milling.as_ref().map(|m| json!({
                "width_mm": m.width_mm, "width_p10_mm": m.width_p10_mm, "width_p90_mm": m.width_p90_mm,
                "spread": m.spread, "pinches": m.pinches, "copper_share": m.copper_share, "samples": m.samples,
                "ok": m.ok(), "issues": m.issues.iter().map(|i| i.code()).collect::<Vec<_>>(),
            })),
            "findings": self.findings.iter().map(|f| json!({
                "code": f.code, "hole": f.hole, "value": f.value, "limit": f.limit, "fails": f.fails,
            })).collect::<Vec<_>>(),
        })
    }
}

fn finding(
    code: &'static str,
    hole: Option<usize>,
    value: f64,
    limit: f64,
    fails: bool,
) -> Finding {
    Finding {
        code,
        hole,
        value,
        limit,
        fails,
    }
}

/// Runs what the profile asks on a picture. `expected` are the holes of the job that the camera should see (empty: the holes are not checked, and that is said).
pub fn inspect(img: &Gray, expected: &[ExpectedHole], p: &Profile) -> Report {
    let mut findings: Vec<Finding> = Vec::new();
    let mut report = Report {
        verdict: Verdict::Unknown,
        profile: p.name.clone(),
        quality: None,
        holes: Vec::new(),
        milling: None,
        findings: Vec::new(),
        width_px: img.w,
        height_px: img.h,
    };
    let wants_holes = p.check_holes;
    let wants_milling = p.check_milling;
    if !wants_holes && !wants_milling {
        findings.push(finding("nothing_to_check", None, 0.0, 0.0, false));
        report.findings = findings;
        return report;
    }
    if !p.calibrated() {
        // without the scale of the picture nothing can be measured
        findings.push(finding("not_calibrated", None, 0.0, 0.0, false));
        report.findings = findings;
        return report;
    }
    let mut blocked = false;
    if p.check_quality {
        let (q, issues) = assess(img, p);
        report.quality = Some(q);
        for i in issues {
            let (value, limit) = match i {
                crate::quality::QualityIssue::TooDark => (q.mean, p.min_mean),
                crate::quality::QualityIssue::TooBright => (q.mean, p.max_mean),
                crate::quality::QualityIssue::LowContrast => (q.contrast, p.min_contrast),
                crate::quality::QualityIssue::Blurry => (q.sharpness, p.min_sharpness),
            };
            findings.push(finding(i.code(), None, value, limit, false));
            blocked = true;
        }
    }
    if blocked {
        // a poor picture says nothing; the verdict is unknown (or not ok, when the profile does not accept doubt)
        report.verdict = if p.fail_on_unknown {
            Verdict::NotOk
        } else {
            Verdict::Unknown
        };
        report.findings = findings;
        return report;
    }
    let mut failed = false;
    let mut judged = false;
    if wants_holes {
        if expected.is_empty() {
            findings.push(finding("no_expected_holes", None, 0.0, 0.0, false));
        } else {
            judged = true;
            report.holes = check_holes(img, expected, p);
            for h in &report.holes {
                for i in &h.issues {
                    let (value, limit) = match i {
                        crate::holes::HoleIssue::WrongSize => (h.dia_mm, h.expected.dia_mm),
                        crate::holes::HoleIssue::OffCentre => (h.offset_mm, p.hole_pos_tol_mm),
                        crate::holes::HoleIssue::NotRound => (h.roundness, p.hole_min_roundness),
                        crate::holes::HoleIssue::Debris => (h.debris, p.hole_max_debris),
                        _ => (0.0, 0.0),
                    };
                    // a place the camera could not see is not a failure of the work, only a hole that was not judged
                    let fails = *i != crate::holes::HoleIssue::OutOfView;
                    findings.push(finding(i.code(), Some(h.index), value, limit, fails));
                    failed |= fails;
                }
            }
            if report
                .holes
                .iter()
                .all(|h| h.issues == [crate::holes::HoleIssue::OutOfView])
            {
                judged = false;
            }
        }
    }
    if wants_milling {
        let m = check_milling(img, p);
        for i in &m.issues {
            let (value, limit) = match i {
                crate::milling::MillingIssue::TooNarrow | crate::milling::MillingIssue::TooWide => {
                    (m.width_mm, p.mill_width_mm)
                }
                crate::milling::MillingIssue::Pinched => (m.width_p10_mm, p.mill_min_width_mm),
                crate::milling::MillingIssue::Uneven => (m.spread, p.mill_max_spread),
                crate::milling::MillingIssue::NoChannels => (0.0, 0.0),
            };
            // no channel at all is not a failure of the milling: the camera may be over a place without any
            let fails = *i != crate::milling::MillingIssue::NoChannels;
            findings.push(finding(i.code(), None, value, limit, fails));
            failed |= fails;
        }
        if m.samples > 0 {
            judged = true;
        }
        report.milling = Some(m);
    }
    report.verdict = if failed {
        Verdict::NotOk
    } else if judged {
        Verdict::Ok
    } else if p.fail_on_unknown {
        Verdict::NotOk
    } else {
        Verdict::Unknown
    };
    report.findings = findings;
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holes::tests::{board, PPM};

    fn profile() -> Profile {
        Profile {
            px_per_mm: PPM,
            check_milling: false,
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
    fn a_good_board_is_ok() {
        let img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8), (1.5, 1.0, 1.0)]);
        let r = inspect(
            &img,
            &[hole(0.0, 0.0, 0.8), hole(1.5, 1.0, 1.0)],
            &profile(),
        );
        assert_eq!(r.verdict, Verdict::Ok, "{:?}", r.findings);
        assert_eq!(r.holes_ok(), 2);
        assert!(r.quality.is_some() && r.findings.is_empty());
        let j = r.to_json();
        assert_eq!(j["verdict"], "ok");
        assert_eq!(j["holes"].as_array().unwrap().len(), 2);
        assert_eq!(j["holes"][0]["ok"], true);
        assert!(j["milling"].is_null());
    }

    #[test]
    fn a_missing_hole_makes_it_not_ok_and_says_which() {
        let img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8)]);
        let r = inspect(
            &img,
            &[hole(0.0, 0.0, 0.8), hole(1.5, 1.0, 0.8)],
            &profile(),
        );
        assert_eq!(r.verdict, Verdict::NotOk);
        let f = r
            .findings
            .iter()
            .find(|f| f.code == "hole_missing")
            .unwrap();
        assert_eq!(f.hole, Some(1));
        assert!(f.fails);
        assert_eq!(r.to_json()["findings"][0]["code"], "hole_missing");
    }

    #[test]
    fn a_dark_picture_is_unknown_not_a_pass_or_a_fail() {
        let img = board(320, 240, 20, 5, &[(0.0, 0.0, 0.8)]);
        let r = inspect(&img, &[hole(0.0, 0.0, 0.8)], &profile());
        assert_eq!(r.verdict, Verdict::Unknown);
        assert!(r
            .findings
            .iter()
            .any(|f| f.code == "image_too_dark" && !f.fails));
        assert!(
            r.holes.is_empty(),
            "nothing is judged on a picture like that"
        );
        // a strict shop does not accept doubt
        let strict = Profile {
            fail_on_unknown: true,
            ..profile()
        };
        assert_eq!(
            inspect(&img, &[hole(0.0, 0.0, 0.8)], &strict).verdict,
            Verdict::NotOk
        );
        // and the check of the picture can be turned off
        let blind = Profile {
            check_quality: false,
            ..profile()
        };
        assert!(inspect(&img, &[hole(0.0, 0.0, 0.8)], &blind)
            .quality
            .is_none());
    }

    #[test]
    fn without_a_scale_or_without_checks_nothing_is_judged() {
        let img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8)]);
        let r = inspect(&img, &[hole(0.0, 0.0, 0.8)], &Profile::default());
        assert_eq!(r.verdict, Verdict::Unknown);
        assert_eq!(r.findings[0].code, "not_calibrated");
        let off = Profile {
            check_holes: false,
            check_milling: false,
            ..profile()
        };
        assert_eq!(
            inspect(&img, &[], &off).findings[0].code,
            "nothing_to_check"
        );
        // holes asked for and none given: said, and nothing is judged
        let r = inspect(&img, &[], &profile());
        assert_eq!(r.verdict, Verdict::Unknown);
        assert_eq!(r.findings[0].code, "no_expected_holes");
    }

    #[test]
    fn holes_out_of_view_are_not_a_failure_of_the_work() {
        let img = board(320, 240, 170, 25, &[(0.0, 0.0, 0.8)]);
        let r = inspect(
            &img,
            &[hole(0.0, 0.0, 0.8), hole(30.0, 0.0, 0.8)],
            &profile(),
        );
        assert_eq!(r.verdict, Verdict::Ok);
        assert!(r
            .findings
            .iter()
            .any(|f| f.code == "hole_out_of_view" && !f.fails));
        // all of them out of view: nothing was judged
        let r = inspect(&img, &[hole(30.0, 0.0, 0.8)], &profile());
        assert_eq!(r.verdict, Verdict::Unknown);
    }

    #[test]
    fn the_milling_is_checked_with_the_holes_when_both_are_on() {
        // copper with two channels of the right width and a good hole in one of the copper areas
        let mut img = Gray::new(320, 240, 200);
        for &yc in &[-0.9, 0.9] {
            let cy = 120.0 - yc * PPM;
            for y in 0..240 {
                if (y as f64 - cy).abs() <= 0.1 * PPM {
                    for x in 0..320 {
                        img.set(x, y, 60);
                    }
                }
            }
        }
        for y in 0..240 {
            for x in 0..320 {
                if (x as f64 - 160.0).hypot(y as f64 - 120.0) <= 0.4 * PPM {
                    img.set(x, y, 25);
                }
            }
        }
        let p = Profile {
            px_per_mm: PPM,
            hole_polarity: crate::profile::Polarity::Dark,
            ..Profile::default()
        };
        let r = inspect(&img, &[hole(0.0, 0.0, 0.8)], &p);
        assert_eq!(r.verdict, Verdict::Ok, "{:?}", r.findings);
        assert!(r.milling.is_some() && r.holes.len() == 1);
        let j = r.to_json();
        assert_eq!(j["milling"]["ok"], true);
        assert_eq!(Verdict::NotOk.word(), "not_ok");
    }
}
