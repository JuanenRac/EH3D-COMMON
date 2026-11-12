//! The spindles a machine can be fitted with and the limits of each one: the slowest and the fastest turns, the power, the fastest feed along the cut (XY) and of the
//! plunge (Z), and the thickest bit. One table for the whole suite: the server judges a job against it and the interface of the machines offers the bits that fit it. The
//! starting values live in `spindles.cfg` (the file that comes with this crate); nothing is written into the code.

use serde::{Deserialize, Serialize};

/// The spindles, by the word the API and the settings files use.
pub const IDS: [&str; 2] = ["original", "vfd"];

/// The shipped file with the starting limits.
pub const DEFAULT_CONFIG: &str = include_str!("../spindles.cfg");

/// The limits of one spindle.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub min_rpm: u32,
    pub max_rpm: u32,
    pub power_w: u32,
    pub max_xy_mm_min: u32,
    pub max_z_mm_min: u32,
    pub max_bit_mm: f64,
}

impl Limits {
    /// Whether the numbers can be used: nothing zero, the slowest not above the fastest, nothing absurd.
    pub fn valid(&self) -> bool {
        (1..=200_000).contains(&self.max_rpm)
            && self.min_rpm <= self.max_rpm
            && (1..=20_000).contains(&self.max_xy_mm_min)
            && (1..=20_000).contains(&self.max_z_mm_min)
            && (1..=20_000).contains(&self.power_w)
            && self.max_bit_mm.is_finite()
            && self.max_bit_mm > 0.0
            && self.max_bit_mm <= 50.0
    }
}

/// The limits of spindle `name` read from the text of a table (`name = min_rpm, max_rpm, power_w, max_xy_mm_min, max_z_mm_min, max_bit_mm`);
/// `None` when the spindle is not in it or its line cannot be read.
pub fn from_table(table: &str, name: &str) -> Option<Limits> {
    table.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (key, values) = line.split_once('=')?;
        if key.trim() != name {
            return None;
        }
        let f: Vec<&str> = values.split(',').map(str::trim).collect();
        if f.len() != 6 {
            return None;
        }
        let whole = |i: usize| f[i].parse::<u32>().ok();
        Some(Limits {
            min_rpm: whole(0)?,
            max_rpm: whole(1)?,
            power_w: whole(2)?,
            max_xy_mm_min: whole(3)?,
            max_z_mm_min: whole(4)?,
            max_bit_mm: f[5].parse::<f64>().ok()?,
        })
    })
}

/// The starting limits of a spindle (from `spindles.cfg`); the tuning of the machine replaces them. `None` for a name that is not a spindle.
pub fn defaults(name: &str) -> Option<Limits> {
    from_table(DEFAULT_CONFIG, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_table_has_every_spindle_with_limits_that_can_be_used() {
        for id in IDS {
            let l = defaults(id).unwrap_or_else(|| panic!("{id} is not in spindles.cfg"));
            assert!(l.valid(), "{id}: {l:?}");
        }
        let (o, v) = (defaults("original").unwrap(), defaults("vfd").unwrap());
        assert_eq!((o.min_rpm, o.max_rpm, o.max_bit_mm), (5_000, 63_000, 3.2));
        assert_eq!(
            (v.max_rpm, v.power_w, v.max_z_mm_min),
            (24_000, 2_200, 1_200)
        );
        assert!(defaults("steam").is_none());
    }

    #[test]
    fn a_line_that_cannot_be_read_gives_nothing() {
        for bad in [
            "vfd = 1, 2, 3",
            "vfd = a, 2, 3, 4, 5, 6",
            "vfd 1, 2, 3, 4, 5, 6",
            "# vfd = 1, 2, 3, 4, 5, 6",
        ] {
            assert_eq!(from_table(bad, "vfd"), None, "{bad}");
        }
        let one = from_table("vfd = 100, 200, 300, 400, 500, 1.5\n", "vfd").unwrap();
        assert_eq!((one.min_rpm, one.max_rpm, one.max_bit_mm), (100, 200, 1.5));
    }

    #[test]
    fn limits_that_cannot_be_used_are_not_valid() {
        let good = defaults("vfd").unwrap();
        assert!(!Limits {
            min_rpm: good.max_rpm + 1,
            ..good
        }
        .valid());
        assert!(!Limits {
            max_z_mm_min: 0,
            ..good
        }
        .valid());
        assert!(!Limits {
            max_bit_mm: f64::NAN,
            ..good
        }
        .valid());
        assert!(!Limits {
            max_bit_mm: 60.0,
            ..good
        }
        .valid());
    }
}
