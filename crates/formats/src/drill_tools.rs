//! Tool table (`DRILLPRO.DTD`): `nn stroke feed spindle` per line, `.` decimal separator.

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub number: u8,
    /// Additional stroke length in mm.
    pub extra_stroke_mm: f64,
    /// Z feed in mm/min.
    pub feed_mm_min: u32,
    /// Spindle speed in rpm.
    pub spindle_rpm: u32,
}

/// Lines 1-19 are comments; data starts on line 20.
pub fn parse(text: &str) -> Vec<Tool> {
    text.lines()
        .skip(19)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some(Tool {
                number: it.next()?.parse().ok()?,
                extra_stroke_mm: it.next()?.parse().ok()?,
                feed_mm_min: it.next()?.parse().ok()?,
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
    // the data starts on line 20: a file with fewer comment lines (or none) is filled up to it, or the tools would be read back as comments
    for _ in out.lines().count()..19 {
        out.push_str(";\r\n");
    }
    for t in tools {
        // one decimal as in the file's own format, two when the stroke needs them
        let stroke = if ((t.extra_stroke_mm * 10.0).round() - t.extra_stroke_mm * 10.0).abs() < 1e-6
        {
            format!("{:.1}", t.extra_stroke_mm)
        } else {
            format!("{:.2}", t.extra_stroke_mm)
        };
        out.push_str(&format!(
            "{:02} {stroke} {:04} {:05}\r\n",
            t.number, t.feed_mm_min, t.spindle_rpm
        ));
    }
    out
}

#[cfg(test)]
mod table_tests {
    use super::*;

    #[test]
    fn a_table_saved_without_an_original_is_read_back() {
        let tools = vec![Tool {
            number: 1,
            extra_stroke_mm: 0.25,
            feed_mm_min: 600,
            spindle_rpm: 60000,
        }];
        let text = rewrite("", &tools);
        assert_eq!(parse(&text), tools);
    }
}
