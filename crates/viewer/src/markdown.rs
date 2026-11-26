//! Markdown, reduced to the blocks of a [`Preview`]. Nothing is fetched and no link is followed:
//! a link shows its text and its address, a picture shows its description, and HTML stays text.

use crate::doc::{Block, Builder, Preview};
use crate::limits::ViewerLimits;
use crate::plain;

/// The text of a line without the marks of emphasis, with links written out.
fn inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '[' || rest.starts_with("![") {
            let picture = c == '!';
            let body = &rest[if picture { 2 } else { 1 }..];
            if let Some((label, after)) = body.split_once("](") {
                if !label.contains('[') {
                    if let Some((address, tail)) = after.split_once(')') {
                        if picture {
                            out.push_str(&format!("[{label}]"));
                        } else if label == address || address.starts_with('#') {
                            out.push_str(label);
                        } else {
                            out.push_str(&format!("{label} ({address})"));
                        }
                        rest = tail;
                        continue;
                    }
                }
            }
        }
        if c == '`' {
            if let Some((code, tail)) = rest[1..].split_once('`') {
                out.push_str(code);
                rest = tail;
                continue;
            }
        }
        if rest.starts_with("**") || rest.starts_with("__") || rest.starts_with("~~") {
            rest = &rest[2..];
            continue;
        }
        if c == '*' && !rest[1..].starts_with(' ') {
            rest = &rest[1..];
            continue;
        }
        if c == '*' && !out.ends_with(' ') && !out.is_empty() {
            rest = &rest[1..];
            continue;
        }
        if c == '\\' && rest[1..].starts_with(|n: char| n.is_ascii_punctuation()) {
            out.extend(rest[1..].chars().next());
            rest = &rest[2..];
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

fn cells(line: &str) -> Vec<String> {
    let line = line.trim().trim_start_matches('|').trim_end_matches('|');
    line.split('|').map(|c| inline(c.trim())).collect()
}

fn is_separator_row(line: &str) -> bool {
    line.contains('-') && line.chars().all(|c| "|-: \t".contains(c))
}

fn item(line: &str) -> Option<(u8, &str)> {
    let indent = line.len() - line.trim_start().len();
    let body = line.trim_start();
    let depth = u8::try_from(indent / 2).unwrap_or(u8::MAX);
    for mark in ["- ", "* ", "+ "] {
        if let Some(rest) = body.strip_prefix(mark) {
            return Some((depth, rest));
        }
    }
    let digits = body.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && digits < 10 && body[digits..].starts_with(". ") {
        // the number stays in the text of the item
        return Some((depth, body));
    }
    None
}

/// The preview of a Markdown file.
pub fn preview(data: &[u8], limits: &ViewerLimits) -> Preview {
    let (content, cut) = plain::text(data, limits);
    let mut builder = Builder::new(limits);
    if cut {
        builder.truncate();
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut paragraph = String::new();
    let mut at = 0;
    let flush = |paragraph: &mut String, builder: &mut Builder| {
        if !paragraph.is_empty() {
            builder.push(Block::Paragraph(inline(paragraph)));
            paragraph.clear();
        }
    };
    while at < lines.len() && !builder.full() {
        let line = lines[at];
        let body = line.trim();
        at += 1;
        if body.starts_with("```") || body.starts_with("~~~") {
            flush(&mut paragraph, &mut builder);
            let fence = &body[..3];
            let mut code = Vec::new();
            while at < lines.len() && !lines[at].trim_start().starts_with(fence) {
                code.push(lines[at]);
                at += 1;
            }
            at += 1;
            builder.push(Block::Code(code.join("\n")));
            continue;
        }
        let hashes = body.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && body[hashes..].starts_with(' ') {
            flush(&mut paragraph, &mut builder);
            let title = body[hashes..].trim().trim_end_matches('#').trim_end();
            builder.push(Block::Heading(hashes as u8, inline(title)));
        } else if body.is_empty() {
            flush(&mut paragraph, &mut builder);
        } else if body.len() >= 3 && ["-", "*", "_"].iter().any(|m| body == m.repeat(body.len())) {
            flush(&mut paragraph, &mut builder);
            builder.push(Block::Rule);
        } else if body.starts_with('|') && lines.get(at).is_some_and(|l| is_separator_row(l)) {
            flush(&mut paragraph, &mut builder);
            let mut rows = Vec::new();
            builder.row(&mut rows, cells(body));
            at += 1;
            while at < lines.len() && lines[at].trim_start().starts_with('|') {
                builder.row(&mut rows, cells(lines[at]));
                at += 1;
            }
            builder.table(rows);
        } else if let Some(quoted) = body.strip_prefix('>') {
            flush(&mut paragraph, &mut builder);
            builder.push(Block::Quote(inline(quoted.trim_start())));
        } else if let Some((depth, rest)) = item(line) {
            flush(&mut paragraph, &mut builder);
            builder.push(Block::Item(depth, inline(rest)));
        } else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(body);
        }
    }
    if at < lines.len() {
        builder.truncate();
    }
    flush(&mut paragraph, &mut builder);
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usual_blocks() {
        let source = "# Title #\n\nA **bold** word, a `code` and a [page](https://example.org).\nSame paragraph.\n\n- one\n  - two\n3. three\n> said\n\n---\n```\n  kept **as is**\n```\n| a | b |\n|---|:-:|\n| 1 | *2* |\n";
        let preview = preview(source.as_bytes(), &ViewerLimits::default());
        assert_eq!(
            preview.blocks,
            vec![
                Block::Heading(1, "Title".into()),
                Block::Paragraph(
                    "A bold word, a code and a page (https://example.org). Same paragraph.".into()
                ),
                Block::Item(0, "one".into()),
                Block::Item(1, "two".into()),
                Block::Item(0, "3. three".into()),
                Block::Quote("said".into()),
                Block::Rule,
                Block::Code("  kept **as is**".into()),
                Block::Table(vec![
                    vec!["a".into(), "b".into()],
                    vec!["1".into(), "2".into()]
                ]),
            ]
        );
        assert!(!preview.truncated);
    }

    #[test]
    fn pictures_are_not_fetched_and_html_stays_text() {
        assert_eq!(
            inline("![a board](http://example.org/x.png) <script>x()</script> 2 * 3 \\* \\_"),
            "[a board] <script>x()</script> 2 * 3 * _"
        );
    }

    #[test]
    fn odd_input_never_panics() {
        let source = "[ ![ ]( ` ** \\ é](\n```\nopen fence\n#\n|\n|--|\n日本 [a](b";
        for (at, _) in source.char_indices() {
            let _ = preview(&source.as_bytes()[..at], &ViewerLimits::default());
        }
    }
}
