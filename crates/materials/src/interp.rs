//! Dk and Df between the frequencies a datasheet publishes.
//!
//! The interpolation is linear in the logarithm of the frequency (a loss tangent and a dielectric
//! constant change slowly with the decade, not with the hertz). It never extrapolates: a
//! frequency below the lowest or above the highest published point is an error, and so is a
//! list with no frequencies. When the same frequency is published more than once (another
//! resin content or another method) the first one listed is used.

use crate::error::MatError;
use crate::model::{DkPoint, Material};

/// The published points with a frequency, one per frequency (the first listed), by frequency.
fn ordered(list: &[DkPoint]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for p in list {
        if let Some(f) = p.freq_ghz {
            if f.is_finite() && f > 0.0 && !out.iter().any(|q| q.0 == f) {
                out.push((f, p.value));
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The value at `freq_ghz`, or the reason it cannot be given.
pub(crate) fn interpolate_raw(list: &[DkPoint], freq_ghz: f64) -> Result<f64, String> {
    if !freq_ghz.is_finite() || freq_ghz <= 0.0 {
        return Err(format!("{freq_ghz} GHz is not a frequency"));
    }
    let pts = ordered(list);
    let (first, last) = match (pts.first(), pts.last()) {
        (Some(a), Some(b)) => (*a, *b),
        _ => return Err("no value is published with a frequency".to_owned()),
    };
    if freq_ghz < first.0 || freq_ghz > last.0 {
        return Err(format!(
            "{freq_ghz} GHz is outside the published {} to {} GHz",
            first.0, last.0
        ));
    }
    if let Some(p) = pts.iter().find(|p| p.0 == freq_ghz) {
        return Ok(p.1);
    }
    match pts
        .windows(2)
        .find(|w| freq_ghz > w[0].0 && freq_ghz < w[1].0)
    {
        Some(w) => {
            let t = (freq_ghz.ln() - w[0].0.ln()) / (w[1].0.ln() - w[0].0.ln());
            Ok(w[0].1 + t * (w[1].1 - w[0].1))
        }
        None => Err("no pair of published points encloses the frequency".to_owned()),
    }
}

/// The value of a list of points at `freq_ghz`, for the material `id` (named in the error).
pub fn interpolate(id: &str, list: &[DkPoint], freq_ghz: f64) -> Result<f64, MatError> {
    interpolate_raw(list, freq_ghz).map_err(|reason| MatError::Interpolation {
        id: id.to_owned(),
        reason,
    })
}

/// The lowest and highest frequency of a list, in GHz, when it has any.
pub fn published_range(list: &[DkPoint]) -> Option<(f64, f64)> {
    let pts = ordered(list);
    match (pts.first(), pts.last()) {
        (Some(a), Some(b)) => Some((a.0, b.0)),
        _ => None,
    }
}

impl Material {
    /// Dk at `freq_ghz`, interpolated between the published points.
    pub fn dk_at(&self, freq_ghz: f64) -> Result<f64, MatError> {
        interpolate(&self.id, &self.dk, freq_ghz)
    }

    /// Df at `freq_ghz`, interpolated between the published points.
    pub fn df_at(&self, freq_ghz: f64) -> Result<f64, MatError> {
        interpolate(&self.id, &self.df, freq_ghz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn p(freq: Option<f64>, value: f64) -> DkPoint {
        DkPoint {
            freq_ghz: freq,
            value,
            method: None,
            resin_content_pct: None,
            tolerance: None,
        }
    }

    #[test]
    fn a_published_frequency_gives_its_value() {
        let list = [p(Some(10.0), 3.5), p(Some(1.0), 3.6)];
        assert_eq!(interpolate_raw(&list, 10.0), Ok(3.5));
        assert_eq!(interpolate_raw(&list, 1.0), Ok(3.6));
    }

    #[test]
    fn between_two_points_the_value_is_linear_in_the_decade() {
        let list = [p(Some(1.0), 4.0), p(Some(100.0), 3.0)];
        // 10 GHz is the middle of the two decades
        let v = interpolate_raw(&list, 10.0).unwrap();
        assert!((v - 3.5).abs() < 1e-12);
    }

    #[test]
    fn it_never_extrapolates() {
        let list = [p(Some(1.0), 4.0), p(Some(10.0), 3.0)];
        assert!(interpolate_raw(&list, 0.5).is_err());
        assert!(interpolate_raw(&list, 11.0).is_err());
        assert!(interpolate_raw(&list, 0.0).is_err());
        assert!(interpolate_raw(&list, f64::NAN).is_err());
    }

    #[test]
    fn a_list_with_no_frequency_cannot_be_interpolated() {
        assert!(interpolate_raw(&[], 1.0).is_err());
        assert!(interpolate_raw(&[p(None, 3.0)], 1.0).is_err());
        assert_eq!(published_range(&[p(None, 3.0)]), None);
    }

    #[test]
    fn the_first_value_listed_for_a_frequency_wins() {
        let list = [p(Some(2.0), 3.0), p(Some(2.0), 9.0), p(Some(4.0), 5.0)];
        assert_eq!(interpolate_raw(&list, 2.0), Ok(3.0));
        assert_eq!(published_range(&list), Some((2.0, 4.0)));
    }

    #[test]
    fn the_error_names_the_material() {
        let mut m = Material::new("a/b", "A", "B", Kind::Laminate, "b.pdf");
        m.dk = vec![p(Some(1.0), 4.0)];
        assert_eq!(m.dk_at(1.0), Ok(4.0));
        let e = m.dk_at(2.0).unwrap_err();
        assert_eq!(e.code(), crate::error::MatCode::Interpolation);
        assert!(e.to_string().contains("a/b"));
        assert!(m.df_at(1.0).is_err());
    }
}
