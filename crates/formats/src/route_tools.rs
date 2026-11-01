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

/// A line of the table that is not a comment and cannot be read: its number (from 1) and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct BadLine {
    pub line: usize,
    pub text: String,
}

/// A number written with a decimal comma (`0,5`, as a Spanish or German Windows writes it) or a point.
fn number<T: std::str::FromStr>(word: &str) -> Option<T> {
    word.replace(',', ".").parse().ok()
}

/// Whether a line of the data part is a comment or empty (nothing to read).
fn is_blank(l: &str) -> bool {
    let t = l.trim();
    t.is_empty() || t.starts_with(';')
}

fn read_line(l: &str) -> Option<Tool> {
    let mut it = l.split_whitespace();
    let tool = Tool {
        number: number(it.next()?)?,
        depth_mm: number(it.next()?)?,
        iterations: number(it.next()?)?,
        feed_xy_mm_min: number(it.next()?)?,
        feed_z_mm_min: number(it.next()?)?,
        spindle_rpm: number(it.next()?)?,
    };
    it.next().is_none().then_some(tool)
}

/// The tools of the table and the lines of the data part that cannot be read: nothing is dropped without being named. Lines 1-19 are comments; the decimal may be a
/// point or a comma.
pub fn parse_report(text: &str) -> (Vec<Tool>, Vec<BadLine>) {
    let (mut tools, mut bad) = (Vec::new(), Vec::new());
    for (i, l) in text.lines().enumerate().skip(19) {
        if is_blank(l) {
            continue;
        }
        match read_line(l) {
            Some(t) => tools.push(t),
            None => bad.push(BadLine {
                line: i + 1,
                text: l.trim().to_string(),
            }),
        }
    }
    (tools, bad)
}

/// The tools of the table (see [`parse_report`] for the lines that could not be read).
pub fn parse(text: &str) -> Vec<Tool> {
    parse_report(text).0
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

#[cfg(test)]
mod table_tests {
    use super::*;

    #[test]
    fn a_decimal_comma_is_read_and_a_bad_line_is_named() {
        let text = format!(
            "{}05 1,7 2 900 300 24000
06 1.0 2 900 300
",
            ";
"
            .repeat(19)
        );
        let (tools, bad) = parse_report(&text);
        assert_eq!(tools.len(), 1);
        assert_eq!(
            (tools[0].number, tools[0].depth_mm, tools[0].iterations),
            (5, 1.7, 2)
        );
        assert_eq!(
            bad,
            vec![BadLine {
                line: 21,
                text: "06 1.0 2 900 300".into()
            }]
        );
    }

    #[test]
    fn a_table_saved_without_an_original_is_read_back() {
        let tools = vec![Tool {
            number: 1,
            depth_mm: 1.7,
            iterations: 2,
            feed_xy_mm_min: 900,
            feed_z_mm_min: 300,
            spindle_rpm: 40000,
        }];
        let text = rewrite("", &tools);
        assert_eq!(parse(&text), tools);
    }
}
