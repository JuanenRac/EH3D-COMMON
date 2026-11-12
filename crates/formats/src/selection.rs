//! How a job picks what it works on, the same for the drilling and the routing and for any program that plans a job: the range of vectors, the copies of the board on a panel
//! and the test of the working area. Plain functions over numbers, so the programs share the rule and not a copy of it.

/// The first and the last vector (counted from 1) that a job takes: `from` is at least 1 and a `to` of 0 means up to the end.
pub fn vector_range(from: usize, to: usize) -> (usize, usize) {
    (from.max(1), if to == 0 { usize::MAX } else { to })
}

/// Where each copy of the board goes on a panel of `columns` by `rows` places, `pitch_mm` apart, as the shift of its corner in mm, row by row. A panel of one place (or of
/// none) is one copy with no shift.
pub fn panel_offsets(columns: u32, rows: u32, pitch_mm: (f64, f64)) -> Vec<(f64, f64)> {
    if columns.saturating_mul(rows) > 1 {
        (0..rows)
            .flat_map(|iy| {
                (0..columns).map(move |ix| (f64::from(ix) * pitch_mm.0, f64::from(iy) * pitch_mm.1))
            })
            .collect()
    } else {
        vec![(0.0, 0.0)]
    }
}

/// Whether every point (in mm from the zero of the file, before the offset of the machine file) falls inside the working area `clip_mm` once the offset is added; with no
/// working area everything is inside.
pub fn inside_work_area(
    clip_mm: Option<(f64, f64)>,
    offset_mm: (f64, f64),
    points: impl IntoIterator<Item = (f64, f64)>,
) -> bool {
    match clip_mm {
        None => true,
        Some((cx, cy)) => points.into_iter().all(|(x, y)| {
            let (x, y) = (x + offset_mm.0, y + offset_mm.1);
            x >= 0.0 && y >= 0.0 && x <= cx && y <= cy
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_starts_at_one_and_zero_means_to_the_end() {
        assert_eq!(vector_range(0, 0), (1, usize::MAX));
        assert_eq!(vector_range(3, 7), (3, 7));
    }

    #[test]
    fn a_panel_of_one_place_is_one_copy_and_a_grid_goes_row_by_row() {
        assert_eq!(panel_offsets(1, 1, (5.0, 5.0)), vec![(0.0, 0.0)]);
        assert_eq!(panel_offsets(0, 3, (5.0, 5.0)), vec![(0.0, 0.0)]);
        assert_eq!(
            panel_offsets(2, 2, (10.0, 20.0)),
            vec![(0.0, 0.0), (10.0, 0.0), (0.0, 20.0), (10.0, 20.0)]
        );
    }

    #[test]
    fn the_working_area_counts_the_offset_and_every_point() {
        let area = Some((100.0, 50.0));
        assert!(inside_work_area(None, (0.0, 0.0), [(1e9, 1e9)]));
        assert!(inside_work_area(
            area,
            (0.0, 0.0),
            [(0.0, 0.0), (100.0, 50.0)]
        ));
        assert!(!inside_work_area(area, (1.0, 0.0), [(100.0, 50.0)]));
        assert!(!inside_work_area(
            area,
            (0.0, 0.0),
            [(5.0, 5.0), (5.0, -0.1)]
        ));
        assert!(inside_work_area(area, (10.0, 5.0), [(90.0, 45.0)]));
    }
}
