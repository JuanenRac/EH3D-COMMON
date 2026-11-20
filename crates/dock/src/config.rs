//! The part of a program's settings file that belongs to the frame. The file is plain `key=value` lines; everything the frame writes starts with
//! `dock.`, so it can be read out of the file and written back without touching a line of the program.

/// The prefix of every key the frame writes.
pub const PREFIX: &str = "dock.";

/// The `key=value` lines of a settings text (blank lines and lines that start with `#` or `;` are left out; the key and the value are trimmed).
pub fn parse_pairs(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with(';'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// Only the pairs that belong to the frame.
pub fn dock_pairs(text: &str) -> Vec<(String, String)> {
    parse_pairs(text)
        .into_iter()
        .filter(|(k, _)| k.starts_with(PREFIX))
        .collect()
}

/// A settings text with the lines of the frame replaced by `block`: every other line stays as it was, in the line ending the file uses.
pub fn replace_block(existing: &str, block: &[(String, String)]) -> String {
    let eol = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = String::new();
    for line in existing.lines() {
        let key = line.split_once('=').map(|(k, _)| k.trim()).unwrap_or("");
        if key.starts_with(PREFIX) {
            continue;
        }
        out.push_str(line);
        out.push_str(eol);
    }
    for (k, v) in block {
        // a value can never hold a line break, so a line of the block is always one line
        let v: String = v.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        out.push_str(&format!("{k}={v}{eol}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_skip_comments_and_blanks() {
        let p = parse_pairs("# a comment\n\nname = x \n; another\nbad line\n=novalue\nk=v=w\r\n");
        assert_eq!(
            p,
            vec![
                ("name".to_string(), "x".to_string()),
                ("k".to_string(), "v=w".to_string())
            ]
        );
    }

    #[test]
    fn the_block_replaces_only_its_own_lines() {
        let old = "language=es\ndock.version=1\ndock.ws.0.name=Old\ntheme=dark\n";
        let new = replace_block(
            old,
            &[
                ("dock.version".into(), "1".into()),
                ("dock.current".into(), "0".into()),
            ],
        );
        assert_eq!(
            new,
            "language=es\ntheme=dark\ndock.version=1\ndock.current=0\n"
        );
        assert_eq!(dock_pairs(&new).len(), 2);
        // writing twice gives the same file
        assert_eq!(replace_block(&new, &dock_pairs(&new)), new);
    }

    #[test]
    fn the_line_ending_of_the_file_is_kept() {
        let old = "a=1\r\ndock.x=2\r\n";
        let new = replace_block(old, &[("dock.y".into(), "3".into())]);
        assert_eq!(new, "a=1\r\ndock.y=3\r\n");
        assert!(!new.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn a_value_never_breaks_a_line() {
        let new = replace_block("", &[("dock.a".into(), "x\ny\r".into())]);
        assert_eq!(new, "dock.a=xy\n");
    }
}
