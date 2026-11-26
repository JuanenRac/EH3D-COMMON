//! Plain text and tables of separated values.

use crate::doc::{Builder, Preview};
use crate::limits::ViewerLimits;

/// Decodes a text: a byte order mark says UTF-8 or UTF-16; otherwise UTF-8, and a file that is not
/// valid UTF-8 is read byte by byte as Latin-1, so nothing is ever refused for its encoding.
pub fn decode(data: &[u8]) -> String {
    if let Some(rest) = data.strip_prefix(b"\xef\xbb\xbf") {
        return String::from_utf8_lossy(rest).into_owned();
    }
    for (mark, big) in [(b"\xff\xfe", false), (b"\xfe\xff", true)] {
        if let Some(rest) = data.strip_prefix(mark) {
            let units = rest.as_chunks::<2>().0.iter().map(|&pair| {
                if big {
                    u16::from_be_bytes(pair)
                } else {
                    u16::from_le_bytes(pair)
                }
            });
            return char::decode_utf16(units)
                .map(|c| c.unwrap_or('\u{fffd}'))
                .collect();
        }
    }
    match std::str::from_utf8(data) {
        Ok(text) => text.to_string(),
        // a cut in the middle of the last character is still UTF-8
        Err(e) if e.error_len().is_none() => String::from_utf8_lossy(data).into_owned(),
        Err(_) => data.iter().map(|b| char::from(*b)).collect(),
    }
}

/// The text to show: at most the limit of bytes, with zero bytes and other controls replaced.
pub fn text(data: &[u8], limits: &ViewerLimits) -> (String, bool) {
    let limit = usize::try_from(limits.max_text_bytes).unwrap_or(usize::MAX);
    let truncated = data.len() > limit;
    let shown = decode(&data[..data.len().min(limit)])
        .replace("\r\n", "\n")
        .chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                '\u{fffd}'
            } else {
                c
            }
        })
        .collect();
    (shown, truncated)
}

fn delimiter(first_line: &str, tabs: bool) -> char {
    if tabs {
        return '\t';
    }
    let count = |d: char| first_line.matches(d).count();
    let mut best = ',';
    for d in [';', '\t'] {
        if count(d) > count(best) {
            best = d;
        }
    }
    best
}

/// A table of comma, semicolon or tab separated values, with quoted fields.
pub fn table(data: &[u8], tabs: bool, limits: &ViewerLimits) -> Preview {
    let (content, cut) = text(data, limits);
    let mut builder = Builder::new(limits);
    if cut {
        builder.truncate();
    }
    let separator = delimiter(content.lines().next().unwrap_or(""), tabs);
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if builder.full() {
            builder.truncate();
            break;
        }
        match c {
            '"' if quoted => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            }
            '"' if field.is_empty() => quoted = true,
            c if c == separator && !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                builder.row(&mut rows, std::mem::take(&mut row));
            }
            c => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        builder.row(&mut rows, row);
    }
    builder.table(rows);
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Block;

    #[test]
    fn the_encodings_of_text_files() {
        assert_eq!(decode(b"\xef\xbb\xbfhola"), "hola");
        assert_eq!(decode(b"\xff\xfeh\x00i\x00"), "hi");
        assert_eq!(decode(b"\xfe\xff\x00h\x00i"), "hi");
        assert_eq!(decode("año 日".as_bytes()), "año 日");
        assert_eq!(decode(b"a\xf1o"), "año");
        let (shown, cut) = text(b"a\r\nb\x00c\x1b[2J", &ViewerLimits::default());
        assert_eq!(shown, "a\nb\u{fffd}c\u{fffd}[2J");
        assert!(!cut);
        let small = ViewerLimits {
            max_text_bytes: 3,
            ..ViewerLimits::default()
        };
        assert_eq!(text(b"abcdef", &small), ("abc".to_string(), true));
    }

    #[test]
    fn separated_values_with_quotes() {
        let preview = table(
            b"ref;value\nR1;\"4,7 k\"\nC1;\"say \"\"hi\"\"\nsecond line\"",
            false,
            &ViewerLimits::default(),
        );
        assert_eq!(
            preview.blocks,
            vec![Block::Table(vec![
                vec!["ref".into(), "value".into()],
                vec!["R1".into(), "4,7 k".into()],
                vec!["C1".into(), "say \"hi\"\nsecond line".into()],
            ])]
        );
        assert!(!preview.truncated);
    }
}
