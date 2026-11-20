//! A layout as text and back. The form is a handful of `key=value` lines that any settings file can hold: the tree on one line as
//! `H200(T0(files,layers),V700(T0(view),T1(console,log)))` (`H` side by side, `V` one above the other, the number is the share of the first part in
//! thousandths; `T<n>(…)` is a group of tabs with tab `n` on show), the floating windows, the folded panels, the hidden ones, the maximised one
//! and the lock. Reading never panics: a text that cannot be read is an error the caller turns into the factory layout.

use crate::geom::{Axis, Rect, Side};
use crate::model::{valid_id, FloatWin, Group, Layout, Node, MAX_DEPTH, MAX_FLOATING, MAX_PANELS};

/// The version of the form.
pub const FORMAT_VERSION: u32 = 1;
/// The first line of a layout kept as a text of its own.
pub const HEADER: &str = "eh3d-dock-layout";
/// The longest text of a tree that is read.
pub const MAX_TEXT: usize = 65_536;

/// Why a layout could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The text is damaged; the word says where.
    Corrupt(&'static str),
    /// The text says it is of another version of the form.
    OtherVersion(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Corrupt(w) => write!(f, "layout damaged: {w}"),
            ParseError::OtherVersion(v) => write!(f, "layout of version {v}"),
        }
    }
}

impl std::error::Error for ParseError {}

fn write_node(n: &Node, out: &mut String) {
    match n {
        Node::Tabs(g) => {
            out.push_str(&format!("T{}(", g.active));
            out.push_str(&g.panels.join(","));
            out.push(')');
        }
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let c = if *axis == Axis::Row { 'H' } else { 'V' };
            out.push_str(&format!("{c}{}(", (ratio * 1000.0).round() as i32));
            write_node(first, out);
            out.push(',');
            write_node(second, out);
            out.push(')');
        }
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
    panels: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn expect(&mut self, c: u8, why: &'static str) -> Result<(), ParseError> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(ParseError::Corrupt(why))
        }
    }

    fn number(&mut self, why: &'static str) -> Result<u32, ParseError> {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
            if self.i - start > 6 {
                return Err(ParseError::Corrupt(why));
            }
        }
        std::str::from_utf8(&self.b[start..self.i])
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(ParseError::Corrupt(why))
    }

    fn id(&mut self) -> Result<String, ParseError> {
        let start = self.i;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'.')
        {
            self.i += 1;
        }
        let s =
            std::str::from_utf8(&self.b[start..self.i]).map_err(|_| ParseError::Corrupt("name"))?;
        if !valid_id(s) {
            return Err(ParseError::Corrupt("name"));
        }
        self.panels += 1;
        if self.panels > MAX_PANELS {
            return Err(ParseError::Corrupt("too many panels"));
        }
        Ok(s.to_string())
    }

    fn node(&mut self, depth: usize) -> Result<Node, ParseError> {
        if depth > MAX_DEPTH {
            return Err(ParseError::Corrupt("too deep"));
        }
        let kind = self.peek().ok_or(ParseError::Corrupt("end"))?;
        self.i += 1;
        match kind {
            b'T' => {
                let active = self.number("active tab")? as usize;
                self.expect(b'(', "open")?;
                let mut panels = vec![self.id()?];
                while self.peek() == Some(b',') {
                    self.i += 1;
                    panels.push(self.id()?);
                }
                self.expect(b')', "close")?;
                Ok(Node::Tabs(Group { panels, active }))
            }
            b'H' | b'V' => {
                let ratio = self.number("share")? as f32 / 1000.0;
                self.expect(b'(', "open")?;
                let first = self.node(depth + 1)?;
                self.expect(b',', "comma")?;
                let second = self.node(depth + 1)?;
                self.expect(b')', "close")?;
                Ok(Node::Split {
                    axis: if kind == b'H' {
                        Axis::Row
                    } else {
                        Axis::Column
                    },
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                })
            }
            _ => Err(ParseError::Corrupt("node")),
        }
    }
}

/// Reads the text of a tree.
pub fn parse_tree(text: &str) -> Result<Node, ParseError> {
    if text.len() > MAX_TEXT {
        return Err(ParseError::Corrupt("too long"));
    }
    let mut r = Reader {
        b: text.trim().as_bytes(),
        i: 0,
        panels: 0,
    };
    let n = r.node(0)?;
    if r.i != r.b.len() {
        return Err(ParseError::Corrupt("trailing text"));
    }
    Ok(n)
}

/// Writes the text of a tree.
pub fn tree_text(n: &Node) -> String {
    let mut s = String::new();
    write_node(n, &mut s);
    s
}

fn parse_ids(value: &str) -> Result<Vec<String>, ParseError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for p in value.split(',') {
        let p = p.trim();
        if !valid_id(p) {
            return Err(ParseError::Corrupt("name"));
        }
        out.push(p.to_string());
    }
    if out.len() > MAX_PANELS {
        return Err(ParseError::Corrupt("too many panels"));
    }
    Ok(out)
}

fn parse_float(value: &str) -> Result<FloatWin, ParseError> {
    let bad = ParseError::Corrupt("floating window");
    let parts: Vec<&str> = value.split(';').collect();
    if parts.len() != 3 {
        return Err(bad);
    }
    let nums: Vec<f32> = parts[0]
        .split(',')
        .map(|n| n.trim().parse::<i32>().map(|v| v as f32))
        .collect::<Result<_, _>>()
        .map_err(|_| ParseError::Corrupt("floating window"))?;
    if nums.len() != 4 {
        return Err(bad);
    }
    let active: usize = parts[1]
        .trim()
        .parse()
        .map_err(|_| ParseError::Corrupt("floating window"))?;
    let panels = parse_ids(parts[2])?;
    if panels.is_empty() {
        return Err(bad);
    }
    Ok(FloatWin {
        group: Group { panels, active },
        rect: Rect::new(nums[0], nums[1], nums[2], nums[3]),
    })
}

impl Layout {
    /// The layout as `key=value` pairs: `tree`, `float.<n>`, `auto`, `hidden`, `max` and `locked` (a part that is empty is left out).
    pub fn to_pairs(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            out.push(("tree".to_string(), tree_text(root)));
        }
        for (i, f) in self.floating.iter().enumerate() {
            out.push((
                format!("float.{i}"),
                format!(
                    "{},{},{},{};{};{}",
                    f.rect.x.round() as i32,
                    f.rect.y.round() as i32,
                    f.rect.w.round() as i32,
                    f.rect.h.round() as i32,
                    f.group.active,
                    f.group.panels.join(",")
                ),
            ));
        }
        if !self.auto_hidden.is_empty() {
            let v: Vec<String> = self
                .auto_hidden
                .iter()
                .map(|(p, s)| format!("{}:{p}", s.name()))
                .collect();
            out.push(("auto".to_string(), v.join(",")));
        }
        if !self.hidden.is_empty() {
            out.push(("hidden".to_string(), self.hidden.join(",")));
        }
        if let Some(m) = &self.maximized {
            out.push(("max".to_string(), m.clone()));
        }
        if self.locked {
            out.push(("locked".to_string(), "1".to_string()));
        }
        out
    }

    /// Reads the pairs written by [`Layout::to_pairs`]. Keys it does not know are ignored; a value it cannot read is an error.
    pub fn from_pairs(pairs: &[(String, String)]) -> Result<Layout, ParseError> {
        let mut l = Layout::new();
        let mut floats: Vec<(usize, FloatWin)> = Vec::new();
        for (k, v) in pairs {
            match k.as_str() {
                "tree" => l.root = Some(parse_tree(v)?),
                "auto" => {
                    for item in v.split(',').filter(|s| !s.trim().is_empty()) {
                        let (side, id) = item
                            .trim()
                            .split_once(':')
                            .ok_or(ParseError::Corrupt("folded panel"))?;
                        let side =
                            Side::from_name(side).ok_or(ParseError::Corrupt("folded panel"))?;
                        if !valid_id(id) {
                            return Err(ParseError::Corrupt("name"));
                        }
                        l.auto_hidden.push((id.to_string(), side));
                    }
                }
                "hidden" => l.hidden = parse_ids(v)?,
                "max" => {
                    if !valid_id(v.trim()) {
                        return Err(ParseError::Corrupt("name"));
                    }
                    l.maximized = Some(v.trim().to_string());
                }
                "locked" => l.locked = v.trim() == "1",
                other => {
                    if let Some(n) = other.strip_prefix("float.") {
                        let n: usize = n
                            .parse()
                            .map_err(|_| ParseError::Corrupt("floating window"))?;
                        floats.push((n, parse_float(v)?));
                        if floats.len() > MAX_FLOATING {
                            return Err(ParseError::Corrupt("too many windows"));
                        }
                    }
                }
            }
        }
        floats.sort_by_key(|(n, _)| *n);
        l.floating = floats.into_iter().map(|(_, f)| f).collect();
        Ok(l)
    }

    /// The layout as a text of its own: a header line and the pairs.
    pub fn to_text(&self) -> String {
        let mut s = format!("{HEADER} {FORMAT_VERSION}\n");
        for (k, v) in self.to_pairs() {
            s.push_str(&format!("{k}={v}\n"));
        }
        s
    }

    /// Reads the text written by [`Layout::to_text`].
    pub fn from_text(text: &str) -> Result<Layout, ParseError> {
        if text.len() > 4 * MAX_TEXT {
            return Err(ParseError::Corrupt("too long"));
        }
        let mut lines = text.lines();
        let head = lines.next().unwrap_or("").trim();
        let mut words = head.split_whitespace();
        if words.next() != Some(HEADER) {
            return Err(ParseError::Corrupt("header"));
        }
        match words.next() {
            Some(v) if v == FORMAT_VERSION.to_string() => {}
            Some(v) => return Err(ParseError::OtherVersion(v.chars().take(12).collect())),
            None => return Err(ParseError::Corrupt("header")),
        }
        let pairs: Vec<(String, String)> = lines
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect();
        Layout::from_pairs(&pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PanelState;

    fn sample() -> Layout {
        let mut l = Layout::with_root(Node::row(
            0.2,
            Node::tabs(&["files", "layers"]),
            Node::column(
                0.7,
                Node::tabs(&["view"]),
                Node::Tabs(Group {
                    panels: vec!["console".into(), "log".into()],
                    active: 1,
                }),
            ),
        ))
        .with_hidden(&["help"])
        .with_floating(&["props", "extra"], Rect::new(40.0, 60.0, 360.0, 260.0))
        .with_auto_hidden("notes", Side::Right);
        l.maximized = Some("view".into());
        l.locked = true;
        l
    }

    #[test]
    fn a_layout_survives_the_text() {
        let l = sample();
        let text = l.to_text();
        let back = Layout::from_text(&text).unwrap();
        assert_eq!(back, l);
        assert_eq!(back.to_text(), text);
        assert_eq!(back.state_of("props"), Some(PanelState::Floating));
        assert_eq!(
            back.state_of("notes"),
            Some(PanelState::AutoHidden(Side::Right))
        );
    }

    #[test]
    fn the_tree_text_is_the_documented_one() {
        let l = sample();
        assert_eq!(
            tree_text(l.root.as_ref().unwrap()),
            "H200(T0(files,layers),V700(T0(view),T1(console,log)))"
        );
        assert_eq!(parse_tree("T0(a)").unwrap(), Node::tabs(&["a"]));
    }

    #[test]
    fn damaged_text_is_an_error_never_a_panic() {
        for t in [
            "",
            "T",
            "T0",
            "T0(",
            "T0()",
            "T0(a",
            "T0(a,)",
            "H500(T0(a))",
            "H500(T0(a),T0(b)",
            "H(T0(a),T0(b))",
            "X1(a)",
            "T0(a)junk",
            "T0(a b)",
            "T9999999(a)",
            "H5000000000(T0(a),T0(b))",
            "T0(é)",
        ] {
            assert!(parse_tree(t).is_err(), "{t:?}");
        }
        assert!(Layout::from_text("nothing").is_err());
        assert_eq!(
            Layout::from_text("eh3d-dock-layout 2\n"),
            Err(ParseError::OtherVersion("2".into()))
        );
        assert!(Layout::from_text("eh3d-dock-layout 1\ntree=T0(\n").is_err());
        assert!(Layout::from_text("eh3d-dock-layout 1\nfloat.0=1,2,3;0;a\n").is_err());
        assert!(Layout::from_text("eh3d-dock-layout 1\nauto=up:a\n").is_err());
        assert!(Layout::from_text("eh3d-dock-layout 1\nhidden=a,b c\n").is_err());
    }

    #[test]
    fn a_tree_that_is_too_deep_or_too_long_is_refused() {
        let mut deep = String::from("T0(a)");
        for _ in 0..(MAX_DEPTH + 5) {
            deep = format!("H500({deep},T0(b))");
        }
        assert!(parse_tree(&deep).is_err());
        let long = format!("T0({})", vec!["a"; MAX_TEXT].join(","));
        assert!(parse_tree(&long).is_err());
        let many = format!(
            "T0({})",
            (0..=MAX_PANELS)
                .map(|i| format!("p{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(parse_tree(&many).is_err());
    }

    #[test]
    fn unknown_keys_are_ignored_and_ratios_out_of_range_are_left_for_the_repair() {
        let l =
            Layout::from_text("eh3d-dock-layout 1\nfuture=1\ntree=H5000(T0(a),T0(b))\n").unwrap();
        let mut l = l;
        let p = l.repair(&["a", "b"]);
        assert!(p.contains(&crate::model::Problem::BadRatio));
    }

    #[test]
    fn random_text_never_panics() {
        // a fixed seed: the same ten thousand strings on every run
        let mut s = 0x2545_F491_4F6C_DD1Du64;
        let alphabet = b"HVT0123456789(),;:=abc.- \n\r\xC3\xA9";
        for _ in 0..10_000 {
            let mut t = Vec::new();
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let n = (s % 60) as usize;
            for _ in 0..n {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                t.push(alphabet[(s % alphabet.len() as u64) as usize]);
            }
            if let Ok(text) = String::from_utf8(t) {
                let _ = parse_tree(&text);
                let _ = Layout::from_text(&text);
                let _ = Layout::from_text(&format!(
                    "{HEADER} 1\ntree={text}\nfloat.0={text}\nauto={text}\nhidden={text}\n"
                ));
            }
        }
    }

    #[test]
    fn many_random_layouts_round_trip() {
        let mut s = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let ids = ["a", "b", "c", "d", "e", "f", "g"];
        for _ in 0..200 {
            let mut l = Layout::with_root(Node::tabs(&[ids[0]]));
            for id in &ids[1..] {
                let r = next() % 5;
                let anchor = ids[(next() % 7) as usize];
                l.hidden.push(id.to_string());
                let _ = match r {
                    0 => l.float(id, None),
                    1 => l.hide(id),
                    2 => l.auto_hide(id, Side::ALL[(next() % 4) as usize]),
                    _ => l.move_panel(
                        id,
                        crate::model::DropTarget::OnGroup {
                            anchor: anchor.into(),
                            zone: crate::geom::Zone::of_side(Side::ALL[(next() % 4) as usize]),
                            index: None,
                        },
                    ),
                };
            }
            let back = Layout::from_text(&l.to_text()).unwrap();
            assert_eq!(back, l);
        }
    }
}
