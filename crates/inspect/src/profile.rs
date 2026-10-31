//! The analyser of a shop: which checks are on and how strict each one is. A profile is a name and plain numbers, kept as JSON, so a shop can have one for fine pitch work and
//! another for a quick prototype and the screen of the machine only has to say which to use. A value out of its range is reported and the default stays, so a typing mistake
//! never loosens a check without anyone noticing.

use serde_json::{json, Map, Value};

/// Which of two things is the dark one in the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Work it out from the picture (the inside of a hole against the board around it).
    Auto,
    Dark,
    Bright,
}

impl Polarity {
    pub fn word(self) -> &'static str {
        match self {
            Polarity::Auto => "auto",
            Polarity::Dark => "dark",
            Polarity::Bright => "bright",
        }
    }

    pub fn from_word(w: &str) -> Option<Polarity> {
        match w {
            "auto" => Some(Polarity::Auto),
            "dark" => Some(Polarity::Dark),
            "bright" => Some(Polarity::Bright),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    /// The scale of the picture: pixels to the millimetre at the height of the board. Zero means not calibrated, and nothing can be measured.
    pub px_per_mm: f64,

    pub check_quality: bool,
    pub check_holes: bool,
    pub check_milling: bool,
    /// A picture too poor to judge counts as not ok (when false it only says unknown).
    pub fail_on_unknown: bool,

    // the picture itself
    pub min_mean: f64,
    pub max_mean: f64,
    pub min_contrast: f64,
    pub min_sharpness: f64,

    // the holes
    /// Is the inside of a hole darker or lighter than the board around it? `Auto` looks at the picture.
    pub hole_polarity: Polarity,
    /// How far the width of a hole may be from the one of the job, in mm.
    pub hole_dia_tol_mm: f64,
    /// How far the centre of a hole may be from where the job puts it, in mm.
    pub hole_pos_tol_mm: f64,
    /// The ratio of the short to the long axis of the hole; below this it is oval or broken.
    pub hole_min_roundness: f64,
    /// The share of the ring around the hole that is as dark as the hole itself (burrs, dust).
    pub hole_max_debris: f64,

    // the milling
    /// Is the copper the bright or the dark part of the picture?
    pub mill_copper: Polarity,
    /// The grey that splits copper from board; zero finds it in the picture.
    pub mill_threshold: u8,
    /// The width the channels should have (the diameter of the tool at the depth of the cut), in mm.
    pub mill_width_mm: f64,
    /// How far the middle width of the channels may be from that, as a share of it.
    pub mill_width_tol: f64,
    /// A channel narrower than this anywhere is a danger of a short, in mm.
    pub mill_min_width_mm: f64,
    /// A gap wider than this is bare board, not a channel, in mm.
    pub mill_max_channel_mm: f64,
    /// How uneven the channels may be: the gap between the narrow and the wide ones as a share of the middle width.
    pub mill_max_spread: f64,
}

impl Default for Profile {
    fn default() -> Profile {
        Profile {
            name: "standard".to_string(),
            px_per_mm: 0.0,
            check_quality: true,
            check_holes: true,
            check_milling: true,
            fail_on_unknown: false,
            min_mean: 45.0,
            max_mean: 215.0,
            min_contrast: 18.0,
            min_sharpness: 40.0,
            hole_polarity: Polarity::Auto,
            hole_dia_tol_mm: 0.08,
            hole_pos_tol_mm: 0.15,
            hole_min_roundness: 0.85,
            hole_max_debris: 0.15,
            mill_copper: Polarity::Bright,
            mill_threshold: 0,
            mill_width_mm: 0.20,
            mill_width_tol: 0.30,
            mill_min_width_mm: 0.10,
            mill_max_channel_mm: 1.5,
            mill_max_spread: 0.40,
        }
    }
}

impl Profile {
    /// The presets a shop starts from: `standard`, `fine_pitch` (tight) and `prototype` (loose).
    pub fn preset(name: &str) -> Option<Profile> {
        let base = Profile::default();
        match name {
            "standard" => Some(base),
            "fine_pitch" => Some(Profile {
                name: name.to_string(),
                hole_dia_tol_mm: 0.04,
                hole_pos_tol_mm: 0.08,
                hole_min_roundness: 0.90,
                hole_max_debris: 0.08,
                mill_width_tol: 0.20,
                mill_min_width_mm: 0.12,
                mill_max_spread: 0.30,
                fail_on_unknown: true,
                ..base
            }),
            "prototype" => Some(Profile {
                name: name.to_string(),
                hole_dia_tol_mm: 0.15,
                hole_pos_tol_mm: 0.30,
                hole_min_roundness: 0.75,
                hole_max_debris: 0.30,
                mill_width_tol: 0.50,
                mill_min_width_mm: 0.08,
                mill_max_spread: 0.60,
                ..base
            }),
            _ => None,
        }
    }

    /// The names of the presets.
    pub fn presets() -> [&'static str; 3] {
        ["standard", "fine_pitch", "prototype"]
    }

    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "px_per_mm": self.px_per_mm,
            "check_quality": self.check_quality,
            "check_holes": self.check_holes,
            "check_milling": self.check_milling,
            "fail_on_unknown": self.fail_on_unknown,
            "min_mean": self.min_mean,
            "max_mean": self.max_mean,
            "min_contrast": self.min_contrast,
            "min_sharpness": self.min_sharpness,
            "hole_polarity": self.hole_polarity.word(),
            "hole_dia_tol_mm": self.hole_dia_tol_mm,
            "hole_pos_tol_mm": self.hole_pos_tol_mm,
            "hole_min_roundness": self.hole_min_roundness,
            "hole_max_debris": self.hole_max_debris,
            "mill_copper": self.mill_copper.word(),
            "mill_threshold": self.mill_threshold,
            "mill_width_mm": self.mill_width_mm,
            "mill_width_tol": self.mill_width_tol,
            "mill_min_width_mm": self.mill_min_width_mm,
            "mill_max_channel_mm": self.mill_max_channel_mm,
            "mill_max_spread": self.mill_max_spread,
        })
    }

    /// Reads a profile on top of `base`: what the document omits stays as it was; a member that cannot be used is named in the second part of the answer and changes nothing.
    pub fn from_json_over(base: &Profile, text: &str) -> Result<(Profile, Vec<String>), String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let obj: &Map<String, Value> = v.as_object().ok_or("a profile is a JSON object")?;
        let mut p = base.clone();
        let mut bad: Vec<String> = Vec::new();
        for (key, val) in obj {
            let num = |lo: f64, hi: f64| {
                val.as_f64()
                    .filter(|n| n.is_finite() && *n >= lo && *n <= hi)
            };
            let flag = || val.as_bool();
            let ok = match key.as_str() {
                "name" => val
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 40 && !s.contains(['\n', '\r']))
                    .map(|s| p.name = s.to_string())
                    .is_some(),
                "px_per_mm" => num(0.0, 2000.0).map(|n| p.px_per_mm = n).is_some(),
                "check_quality" => flag().map(|b| p.check_quality = b).is_some(),
                "check_holes" => flag().map(|b| p.check_holes = b).is_some(),
                "check_milling" => flag().map(|b| p.check_milling = b).is_some(),
                "fail_on_unknown" => flag().map(|b| p.fail_on_unknown = b).is_some(),
                "min_mean" => num(0.0, 255.0).map(|n| p.min_mean = n).is_some(),
                "max_mean" => num(0.0, 255.0).map(|n| p.max_mean = n).is_some(),
                "min_contrast" => num(0.0, 128.0).map(|n| p.min_contrast = n).is_some(),
                "min_sharpness" => num(0.0, 100000.0).map(|n| p.min_sharpness = n).is_some(),
                "hole_polarity" => val
                    .as_str()
                    .and_then(Polarity::from_word)
                    .map(|x| p.hole_polarity = x)
                    .is_some(),
                "hole_dia_tol_mm" => num(0.001, 2.0).map(|n| p.hole_dia_tol_mm = n).is_some(),
                "hole_pos_tol_mm" => num(0.001, 5.0).map(|n| p.hole_pos_tol_mm = n).is_some(),
                "hole_min_roundness" => num(0.1, 1.0).map(|n| p.hole_min_roundness = n).is_some(),
                "hole_max_debris" => num(0.0, 1.0).map(|n| p.hole_max_debris = n).is_some(),
                "mill_copper" => val
                    .as_str()
                    .and_then(Polarity::from_word)
                    .filter(|x| *x != Polarity::Auto)
                    .map(|x| p.mill_copper = x)
                    .is_some(),
                "mill_threshold" => num(0.0, 255.0)
                    .map(|n| p.mill_threshold = n as u8)
                    .is_some(),
                "mill_width_mm" => num(0.02, 5.0).map(|n| p.mill_width_mm = n).is_some(),
                "mill_width_tol" => num(0.01, 2.0).map(|n| p.mill_width_tol = n).is_some(),
                "mill_min_width_mm" => num(0.01, 5.0).map(|n| p.mill_min_width_mm = n).is_some(),
                "mill_max_channel_mm" => {
                    num(0.05, 20.0).map(|n| p.mill_max_channel_mm = n).is_some()
                }
                "mill_max_spread" => num(0.01, 5.0).map(|n| p.mill_max_spread = n).is_some(),
                _ => false,
            };
            if !ok {
                bad.push(key.clone());
            }
        }
        // the limits must make sense together
        if p.min_mean >= p.max_mean {
            bad.push("min_mean".to_string());
            p.min_mean = base.min_mean;
            p.max_mean = base.max_mean;
        }
        if p.mill_min_width_mm >= p.mill_max_channel_mm {
            bad.push("mill_min_width_mm".to_string());
            p.mill_min_width_mm = base.mill_min_width_mm;
            p.mill_max_channel_mm = base.mill_max_channel_mm;
        }
        Ok((p, bad))
    }

    pub fn from_json(text: &str) -> Result<(Profile, Vec<String>), String> {
        Profile::from_json_over(&Profile::default(), text)
    }

    /// Whether anything can be measured with this profile: it needs the scale of the picture.
    pub fn calibrated(&self) -> bool {
        self.px_per_mm > 0.0
    }

    /// The scale from a hole whose width is known: `measured_px` pixels across for a hole of `known_mm`.
    pub fn calibrate_from_hole(measured_px: f64, known_mm: f64) -> Option<f64> {
        (measured_px > 1.0 && known_mm > 0.0).then(|| measured_px / known_mm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_make_sense_and_the_presets_differ() {
        let d = Profile::default();
        assert!(!d.calibrated() && d.check_holes && d.check_milling && d.check_quality);
        let fine = Profile::preset("fine_pitch").unwrap();
        let proto = Profile::preset("prototype").unwrap();
        assert!(
            fine.hole_pos_tol_mm < d.hole_pos_tol_mm && d.hole_pos_tol_mm < proto.hole_pos_tol_mm
        );
        assert!(fine.fail_on_unknown && !proto.fail_on_unknown);
        assert_eq!(Profile::preset("standard").unwrap(), d);
        assert!(Profile::preset("nonsense").is_none());
        for n in Profile::presets() {
            assert_eq!(Profile::preset(n).unwrap().name, n);
        }
    }

    #[test]
    fn a_profile_survives_json() {
        let mut p = Profile::preset("fine_pitch").unwrap();
        p.px_per_mm = 85.5;
        p.hole_polarity = Polarity::Dark;
        let (back, bad) = Profile::from_json(&p.to_json().to_string()).unwrap();
        assert!(bad.is_empty());
        assert_eq!(back, p);
    }

    #[test]
    fn what_is_wrong_is_named_and_the_value_stays() {
        let text = r#"{"px_per_mm": 90, "hole_pos_tol_mm": -1, "hole_polarity": "purple", "mill_copper": "auto", "who": 1, "check_holes": "yes", "name": ""}"#;
        let (p, bad) = Profile::from_json(text).unwrap();
        assert_eq!(p.px_per_mm, 90.0);
        assert_eq!(p.hole_pos_tol_mm, Profile::default().hole_pos_tol_mm);
        assert_eq!(p.hole_polarity, Polarity::Auto);
        assert!(p.check_holes);
        for k in [
            "hole_pos_tol_mm",
            "hole_polarity",
            "mill_copper",
            "who",
            "check_holes",
            "name",
        ] {
            assert!(bad.contains(&k.to_string()), "{k} should be named: {bad:?}");
        }
        assert!(Profile::from_json("[1]").is_err() && Profile::from_json("not json").is_err());
    }

    #[test]
    fn limits_that_cross_are_refused_together() {
        let (p, bad) = Profile::from_json(r#"{"min_mean": 200, "max_mean": 100}"#).unwrap();
        assert!(bad.contains(&"min_mean".to_string()));
        assert_eq!((p.min_mean, p.max_mean), (45.0, 215.0));
        let (p, bad) =
            Profile::from_json(r#"{"mill_min_width_mm": 3.0, "mill_max_channel_mm": 1.0}"#)
                .unwrap();
        assert!(bad.contains(&"mill_min_width_mm".to_string()));
        assert_eq!(p.mill_min_width_mm, 0.10);
    }

    #[test]
    fn the_scale_comes_from_a_known_hole() {
        assert_eq!(Profile::calibrate_from_hole(60.0, 0.8), Some(75.0));
        assert!(Profile::calibrate_from_hole(0.5, 0.8).is_none());
        assert!(Profile::calibrate_from_hole(60.0, 0.0).is_none());
    }
}
