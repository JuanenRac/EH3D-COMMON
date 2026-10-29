//! Tool table (`ROUTEPRO.RTD`): `nn depth iterations feed_xy feed_z spindle`, data from line 20.

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub number: u8,
    /// Total routing depth in mm.
    pub depth_mm: f64,
    /// Number of passes the depth is divided into.
    pub iterations: u32,
    pub feed_xy_mm_min: u32,
    pub feed_z_mm_min: u32,
    pub spindle_rpm: u32,
}

pub fn parse(text: &str) -> Vec<Tool> {
    text.lines()
        .skip(19)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some(Tool {
                number: it.next()?.parse().ok()?,
                depth_mm: it.next()?.parse().ok()?,
                iterations: it.next()?.parse().ok()?,
                feed_xy_mm_min: it.next()?.parse().ok()?,
                feed_z_mm_min: it.next()?.parse().ok()?,
                spindle_rpm: it.next()?.parse().ok()?,
            })
        })
        .collect()
}

/// The text of a tool table file: the 19 comment lines of `original` followed by one line per tool.
pub fn rewrite(original: &str, tools: &[Tool]) -> String {
    let mut out: String = original.split_inclusive('\n').take(19).collect();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    for t in tools {
        // one decimal as in the file's own format, two when the depth needs them
        let depth = if ((t.depth_mm * 10.0).round() - t.depth_mm * 10.0).abs() < 1e-6 {
            format!("{:.1}", t.depth_mm)
        } else {
            format!("{:.2}", t.depth_mm)
        };
        out.push_str(&format!(
            "{:02} {depth} {:02} {:04}  {:04} {:05}\r\n",
            t.number, t.iterations, t.feed_xy_mm_min, t.feed_z_mm_min, t.spindle_rpm
        ));
    }
    out
}
