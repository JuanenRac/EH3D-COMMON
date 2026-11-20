//! Keyboard shortcuts as data: a key with modifiers that is written and read as text («Ctrl+Shift+S»), a table from shortcuts to actions that
//! finds the ones that clash, and the shortcuts of the frame itself, which a person can change in the settings file.

use crate::geom::Side;
use std::fmt;

/// The keys that are not a letter, a digit or a function key.
const NAMED: [&str; 21] = [
    "Tab",
    "Enter",
    "Escape",
    "Space",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Up",
    "Down",
    "Left",
    "Right",
    "Plus",
    "Minus",
    "Comma",
    "Period",
    "Slash",
    "Backtick",
];

/// A key with the modifiers held. `ctrl` is the command key on a Mac.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Shortcut {
    /// The control (or command) key.
    pub ctrl: bool,
    /// The alt (or option) key.
    pub alt: bool,
    /// The shift key.
    pub shift: bool,
    /// The key: `A` to `Z`, `0` to `9`, `F1` to `F24` or one of the names of [`Shortcut::named_keys`].
    pub key: String,
}

/// Why the text of a shortcut was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutError(pub String);

impl fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a shortcut: {}", self.0)
    }
}

impl std::error::Error for ShortcutError {}

fn valid_key(key: &str) -> bool {
    let b = key.as_bytes();
    if b.len() == 1 {
        return b[0].is_ascii_uppercase() || b[0].is_ascii_digit();
    }
    if let Some(n) = key.strip_prefix('F') {
        if let Ok(v) = n.parse::<u8>() {
            return (1..=24).contains(&v) && v.to_string() == n;
        }
    }
    NAMED.contains(&key)
}

impl Shortcut {
    /// A shortcut from its parts (the key is put in the canonical spelling; `None` when it is not a key).
    pub fn new(ctrl: bool, alt: bool, shift: bool, key: &str) -> Option<Shortcut> {
        let key = canonical_key(key)?;
        Some(Shortcut {
            ctrl,
            alt,
            shift,
            key,
        })
    }

    /// Control (or command) and a key.
    pub fn ctrl(key: &str) -> Shortcut {
        Shortcut::new(true, false, false, key).expect("a key that exists")
    }

    /// Control, shift and a key.
    pub fn ctrl_shift(key: &str) -> Shortcut {
        Shortcut::new(true, false, true, key).expect("a key that exists")
    }

    /// A key alone.
    pub fn plain(key: &str) -> Shortcut {
        Shortcut::new(false, false, false, key).expect("a key that exists")
    }

    /// The names of the keys that are not letters, digits or function keys.
    pub fn named_keys() -> &'static [&'static str] {
        &NAMED
    }

    /// Reads «Ctrl+Shift+S» (modifiers in any order and case; `Cmd`, `Command` and `Control` are `Ctrl`; `Option` is `Alt`).
    pub fn parse(text: &str) -> Result<Shortcut, ShortcutError> {
        let bad = || ShortcutError(text.chars().take(40).collect());
        let mut s = Shortcut {
            ctrl: false,
            alt: false,
            shift: false,
            key: String::new(),
        };
        let parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let Some((key, mods)) = parts.split_last() else {
            return Err(bad());
        };
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "cmd" | "command" => s.ctrl = true,
                "alt" | "option" => s.alt = true,
                "shift" => s.shift = true,
                _ => return Err(bad()),
            }
        }
        s.key = canonical_key(key).ok_or_else(bad)?;
        Ok(s)
    }

    /// Whether this shortcut needs a modifier (a key alone, such as a letter, would be typed into a text box).
    pub fn has_modifier(&self) -> bool {
        self.ctrl || self.alt
    }
}

fn canonical_key(key: &str) -> Option<String> {
    let k = key.trim();
    if k.is_empty() {
        return None;
    }
    let up = k.to_ascii_uppercase();
    if valid_key(&up) {
        return Some(up);
    }
    let lower = k.to_ascii_lowercase();
    let alias = match lower.as_str() {
        "esc" => "Escape",
        "return" => "Enter",
        "del" => "Delete",
        "ins" => "Insert",
        "pgup" | "pageup" => "PageUp",
        "pgdn" | "pagedown" => "PageDown",
        "arrowup" => "Up",
        "arrowdown" => "Down",
        "arrowleft" => "Left",
        "arrowright" => "Right",
        "+" => "Plus",
        "-" => "Minus",
        "," => "Comma",
        "." => "Period",
        "/" => "Slash",
        "`" => "Backtick",
        _ => "",
    };
    if !alias.is_empty() {
        return Some(alias.to_string());
    }
    NAMED
        .iter()
        .find(|n| n.eq_ignore_ascii_case(k))
        .map(|n| n.to_string())
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        f.write_str(&self.key)
    }
}

/// Two or more entries of a table that use the same shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The shortcut.
    pub shortcut: Shortcut,
    /// The names given to the entries that use it.
    pub names: Vec<String>,
}

/// Finds the shortcuts used by more than one of `entries` (a name and a shortcut each), in the order they first appear.
pub fn find_conflicts(entries: &[(String, Shortcut)]) -> Vec<Conflict> {
    let mut out: Vec<Conflict> = Vec::new();
    for (name, sc) in entries {
        match out.iter_mut().find(|c| &c.shortcut == sc) {
            Some(c) => c.names.push(name.clone()),
            None => out.push(Conflict {
                shortcut: sc.clone(),
                names: vec![name.clone()],
            }),
        }
    }
    out.retain(|c| c.names.len() > 1);
    out
}

/// The actions of the frame that have a shortcut a person can change.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DockKey {
    /// Maximise or restore the panel with the focus.
    Maximize,
    /// The next tab of the group with the focus.
    NextTab,
    /// The previous tab of the group with the focus.
    PrevTab,
    /// Restore the layout of the workspace.
    RestoreLayout,
    /// Lock or unlock the layout.
    ToggleLock,
    /// Move the panel with the focus one step towards a side.
    Move(Side),
    /// Show or hide the panel with this name.
    Panel(String),
}

impl DockKey {
    /// The name used in the settings file: `maximize`, `next_tab`, `prev_tab`, `restore_layout`, `toggle_lock` or `panel.<name>`.
    pub fn name(&self) -> String {
        match self {
            DockKey::Maximize => "maximize".into(),
            DockKey::NextTab => "next_tab".into(),
            DockKey::PrevTab => "prev_tab".into(),
            DockKey::RestoreLayout => "restore_layout".into(),
            DockKey::ToggleLock => "toggle_lock".into(),
            DockKey::Move(s) => format!("move_{}", s.name()),
            DockKey::Panel(id) => format!("panel.{id}"),
        }
    }

    /// The key with that name.
    pub fn from_name(name: &str) -> Option<DockKey> {
        Some(match name {
            "maximize" => DockKey::Maximize,
            "next_tab" => DockKey::NextTab,
            "prev_tab" => DockKey::PrevTab,
            "restore_layout" => DockKey::RestoreLayout,
            "toggle_lock" => DockKey::ToggleLock,
            "move_left" => DockKey::Move(Side::Left),
            "move_right" => DockKey::Move(Side::Right),
            "move_up" | "move_top" => DockKey::Move(Side::Top),
            "move_down" | "move_bottom" => DockKey::Move(Side::Bottom),
            _ => {
                let id = name.strip_prefix("panel.")?;
                if !crate::model::valid_id(id) {
                    return None;
                }
                DockKey::Panel(id.to_string())
            }
        })
    }
}

/// The shortcuts of the frame: what is set, and what a person changed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DockKeys {
    /// The entries; `None` is a shortcut a person turned off.
    entries: Vec<(DockKey, Option<Shortcut>)>,
}

impl DockKeys {
    /// The shortcuts that come with the frame: maximise `Ctrl+M`, next tab `Ctrl+PageDown`, previous tab `Ctrl+PageUp`.
    pub fn defaults() -> DockKeys {
        DockKeys {
            entries: vec![
                (DockKey::Maximize, Some(Shortcut::ctrl("M"))),
                (DockKey::NextTab, Some(Shortcut::ctrl("PageDown"))),
                (DockKey::PrevTab, Some(Shortcut::ctrl("PageUp"))),
                (DockKey::RestoreLayout, None),
                (DockKey::ToggleLock, None),
                (
                    DockKey::Move(Side::Left),
                    Shortcut::new(false, true, true, "Left"),
                ),
                (
                    DockKey::Move(Side::Right),
                    Shortcut::new(false, true, true, "Right"),
                ),
                (
                    DockKey::Move(Side::Top),
                    Shortcut::new(false, true, true, "Up"),
                ),
                (
                    DockKey::Move(Side::Bottom),
                    Shortcut::new(false, true, true, "Down"),
                ),
            ],
        }
    }

    /// The shortcut of an action.
    pub fn get(&self, key: &DockKey) -> Option<&Shortcut> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, s)| s.as_ref())
    }

    /// Sets (or, with `None`, turns off) the shortcut of an action.
    pub fn set(&mut self, key: DockKey, shortcut: Option<Shortcut>) {
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 = shortcut,
            None => self.entries.push((key, shortcut)),
        }
    }

    /// The action whose shortcut this is.
    pub fn resolve(&self, shortcut: &Shortcut) -> Option<&DockKey> {
        self.entries
            .iter()
            .find(|(_, s)| s.as_ref() == Some(shortcut))
            .map(|(k, _)| k)
    }

    /// Every action with a shortcut, with the name of the action.
    pub fn named(&self) -> Vec<(String, Shortcut)> {
        self.entries
            .iter()
            .filter_map(|(k, s)| s.clone().map(|s| (k.name(), s)))
            .collect()
    }

    /// The lines of the settings file: `dock.key.<name>=<shortcut>`, or `none`, for what differs from the defaults and for each panel.
    pub fn to_lines(&self) -> Vec<(String, String)> {
        let defaults = DockKeys::defaults();
        let mut out = Vec::new();
        for (k, s) in &self.entries {
            let default = defaults
                .entries
                .iter()
                .find(|(d, _)| d == k)
                .map(|(_, s)| s.clone());
            if default.as_ref() != Some(s) {
                let value = s
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "none".into());
                out.push((format!("dock.key.{}", k.name()), value));
            }
        }
        out
    }

    /// Reads the `dock.key.*` lines of a settings file on top of the defaults; the number of lines that could not be read is given too.
    pub fn from_pairs(pairs: &[(String, String)]) -> (DockKeys, usize) {
        let mut keys = DockKeys::defaults();
        let mut bad = 0;
        for (k, v) in pairs {
            let Some(name) = k.strip_prefix("dock.key.") else {
                continue;
            };
            let Some(action) = DockKey::from_name(name) else {
                bad += 1;
                continue;
            };
            if v.trim().eq_ignore_ascii_case("none") {
                keys.set(action, None);
            } else {
                match Shortcut::parse(v) {
                    Ok(s) => keys.set(action, Some(s)),
                    Err(_) => bad += 1,
                }
            }
        }
        (keys, bad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips() {
        for t in [
            "Ctrl+S",
            "Ctrl+Shift+S",
            "Alt+F4",
            "Ctrl+Alt+Shift+PageDown",
            "F11",
            "Ctrl+Plus",
            "Escape",
        ] {
            let s = Shortcut::parse(t).unwrap();
            assert_eq!(s.to_string(), t);
            assert_eq!(Shortcut::parse(&s.to_string()).unwrap(), s);
        }
    }

    #[test]
    fn spelling_variants_give_the_same_shortcut() {
        assert_eq!(
            Shortcut::parse("cmd+shift+s").unwrap(),
            Shortcut::ctrl_shift("S")
        );
        assert_eq!(Shortcut::parse("Control + s").unwrap(), Shortcut::ctrl("S"));
        assert_eq!(
            Shortcut::parse("shift+ctrl+S").unwrap(),
            Shortcut::ctrl_shift("S")
        );
        assert_eq!(
            Shortcut::parse("Ctrl+pgdn").unwrap(),
            Shortcut::ctrl("PageDown")
        );
        assert_eq!(Shortcut::parse("Ctrl+-").unwrap(), Shortcut::ctrl("Minus"));
        assert_eq!(
            Shortcut::parse("Option+esc").unwrap().to_string(),
            "Alt+Escape"
        );
    }

    #[test]
    fn nonsense_is_refused() {
        for t in [
            "",
            "Ctrl+",
            "Ctrl+Hyper",
            "Banana+S",
            "F25",
            "F0",
            "Ctrl+SS",
            "+",
            "Ctrl+Shift",
            "Ctrl+Ç",
        ] {
            assert!(Shortcut::parse(t).is_err(), "{t}");
        }
        assert!(Shortcut::new(false, false, false, "ab").is_none());
    }

    #[test]
    fn function_keys_up_to_24() {
        assert!(Shortcut::parse("F1").is_ok());
        assert!(Shortcut::parse("Ctrl+F24").is_ok());
        assert!(Shortcut::parse("F024").is_err());
    }

    #[test]
    fn conflicts_are_found_with_every_name() {
        let e = vec![
            ("open".to_string(), Shortcut::ctrl("O")),
            ("save".to_string(), Shortcut::ctrl("S")),
            ("also_open".to_string(), Shortcut::ctrl("O")),
            ("third".to_string(), Shortcut::ctrl("O")),
        ];
        let c = find_conflicts(&e);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].names, ["open", "also_open", "third"]);
        assert!(find_conflicts(&e[..2]).is_empty());
    }

    #[test]
    fn the_frame_keys_have_defaults_and_can_be_changed() {
        let mut k = DockKeys::defaults();
        assert_eq!(k.resolve(&Shortcut::ctrl("M")), Some(&DockKey::Maximize));
        assert_eq!(k.get(&DockKey::NextTab), Some(&Shortcut::ctrl("PageDown")));
        assert!(k.to_lines().is_empty(), "the defaults write nothing");
        k.set(DockKey::Maximize, Some(Shortcut::plain("F11")));
        k.set(DockKey::NextTab, None);
        k.set(DockKey::Panel("layers".into()), Some(Shortcut::ctrl("L")));
        let lines = k.to_lines();
        assert_eq!(lines.len(), 3);
        let (back, bad) = DockKeys::from_pairs(&lines);
        assert_eq!(bad, 0);
        assert_eq!(back, k);
        assert_eq!(
            back.resolve(&Shortcut::plain("F11")),
            Some(&DockKey::Maximize)
        );
        assert_eq!(back.resolve(&Shortcut::ctrl("PageDown")), None);
    }

    #[test]
    fn bad_lines_are_counted_and_ignored() {
        let pairs = vec![
            ("dock.key.maximize".to_string(), "Hyper+Q".to_string()),
            ("dock.key.nonsense".to_string(), "Ctrl+Q".to_string()),
            ("dock.key.panel.bad id".to_string(), "Ctrl+Q".to_string()),
            (
                "dock.key.prev_tab".to_string(),
                "Ctrl+Shift+Tab".to_string(),
            ),
            ("other".to_string(), "x".to_string()),
        ];
        let (k, bad) = DockKeys::from_pairs(&pairs);
        assert_eq!(bad, 3);
        assert_eq!(k.get(&DockKey::Maximize), Some(&Shortcut::ctrl("M")));
        assert_eq!(
            k.get(&DockKey::PrevTab).unwrap().to_string(),
            "Ctrl+Shift+Tab"
        );
    }

    #[test]
    fn names_of_the_keys() {
        for n in [
            "maximize",
            "next_tab",
            "prev_tab",
            "restore_layout",
            "toggle_lock",
            "move_left",
            "move_top",
            "panel.layers",
        ] {
            assert_eq!(DockKey::from_name(n).unwrap().name(), n);
        }
        assert_eq!(DockKey::from_name("panel."), None);
    }
}
