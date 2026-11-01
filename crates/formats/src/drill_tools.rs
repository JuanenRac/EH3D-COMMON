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
        extra_stroke_mm: number(it.next()?)?,
        feed_mm_min: number(it.next()?)?,
        spindle_rpm: number(it.next()?)?,
    };
    // a fifth word is not part of the line of a tool
    it.next().is_none().then_some(tool)
}

/// The tools of the table and the lines of the data part that cannot be read (a number with a letter, a missing field): nothing is dropped without being named.
/// Lines 1-19 are comments; data starts on line 20. The decimal may be a point or a comma.
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

    fn table(lines: &str) -> String {
        format!(
            "{}{lines}",
            ";
"
            .repeat(19)
        )
    }

    #[test]
    fn a_decimal_comma_is_read_and_a_bad_line_is_named() {
        let (tools, bad) = parse_report(&table(
            "05 0,5 1200 24000
06 0.25 800 20000
07 0.5 abc 24000
08 0.5 900
; a note

",
        ));
        assert_eq!(tools.len(), 2);
        assert_eq!(
            (
                tools[0].number,
                tools[0].extra_stroke_mm,
                tools[0].feed_mm_min,
                tools[0].spindle_rpm
            ),
            (5, 0.5, 1200, 24000)
        );
        assert_eq!(bad.iter().map(|b| b.line).collect::<Vec<_>>(), vec![22, 23]);
        assert_eq!(bad[0].text, "07 0.5 abc 24000");
        // the comments of the first nineteen lines are never bad lines
        assert!(parse_report(
            "anything
at all
"
        )
        .1
        .is_empty());
    }

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
