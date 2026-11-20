//! Named workspaces: several layouts of the same window («Prepare», «Review», «Compact»), the one in use, and the factory ones a program ships.
//! They are saved in the settings file and read back with care: a layout that is damaged or from another version is replaced by the factory
//! one and the reason is given as a named warning, never as a panic.

use crate::model::{DockError, Layout};
use crate::serial::FORMAT_VERSION;
use crate::texts::{tr, DockText};
use eh3d_view::texts::Lang;

/// The most workspaces a program keeps.
pub const MAX_WORKSPACES: usize = 16;
/// The longest name of a workspace.
pub const MAX_NAME: usize = 32;

/// Whether `name` can name a workspace: 1 to 32 characters, no control character, no space at either end.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME
        && name.trim() == name
        && !name.chars().any(|c| c.is_control())
}

/// A layout with a name.
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    /// The name the person sees.
    pub name: String,
    /// Its arrangement.
    pub layout: Layout,
}

impl Workspace {
    /// A workspace with this name and layout.
    pub fn new(name: &str, layout: Layout) -> Workspace {
        Workspace {
            name: name.to_string(),
            layout,
        }
    }
}

/// What went wrong while the saved layouts were read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarnKind {
    /// A layout was damaged and was replaced by the factory one.
    Corrupt,
    /// The settings were written by another version of the frame; the factory layouts are used.
    OtherVersion,
    /// A layout had problems (a panel that does not exist, a share out of range) and was repaired.
    Repaired,
    /// A shortcut in the settings could not be read and was ignored.
    BadShortcut,
}

/// A warning about the saved layouts, to show once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadWarning {
    /// What kind of trouble.
    pub kind: WarnKind,
    /// Which workspace or key it is about (empty when it is about all of them).
    pub detail: String,
}

impl LoadWarning {
    /// The word of the warning, for logs and tests.
    pub fn name(&self) -> &'static str {
        match self.kind {
            WarnKind::Corrupt => "layout_corrupt",
            WarnKind::OtherVersion => "layout_other_version",
            WarnKind::Repaired => "layout_repaired",
            WarnKind::BadShortcut => "shortcut_unreadable",
        }
    }

    /// The warning in a language, with the name of the workspace when there is one.
    pub fn text(&self, lang: Lang) -> String {
        let t = match self.kind {
            WarnKind::Corrupt => DockText::WarnCorrupt,
            WarnKind::OtherVersion => DockText::WarnVersion,
            WarnKind::Repaired => DockText::WarnRepaired,
            WarnKind::BadShortcut => DockText::WarnKeys,
        };
        if self.detail.is_empty() {
            tr(lang, t).to_string()
        } else {
            format!("{} ({})", tr(lang, t), self.detail)
        }
    }
}

/// The workspaces of a window.
#[derive(Debug, Clone, PartialEq)]
pub struct Workspaces {
    items: Vec<Workspace>,
    current: usize,
    factory: Vec<Workspace>,
}

impl Workspaces {
    /// The factory workspaces, with the first one in use. An empty list gets one empty workspace called `Default`.
    pub fn new(factory: Vec<Workspace>) -> Workspaces {
        let mut factory: Vec<Workspace> = factory
            .into_iter()
            .filter(|w| valid_name(&w.name))
            .take(MAX_WORKSPACES)
            .collect();
        if factory.is_empty() {
            factory.push(Workspace::new("Default", Layout::new()));
        }
        Workspaces {
            items: factory.clone(),
            current: 0,
            factory,
        }
    }

    /// The names, in order.
    pub fn names(&self) -> Vec<&str> {
        self.items.iter().map(|w| w.name.as_str()).collect()
    }

    /// The name of the one in use.
    pub fn current_name(&self) -> &str {
        &self.items[self.current].name
    }

    /// The layout in use.
    pub fn layout(&self) -> &Layout {
        &self.items[self.current].layout
    }

    /// The layout in use, to change it.
    pub fn layout_mut(&mut self) -> &mut Layout {
        &mut self.items[self.current].layout
    }

    /// The factory workspaces.
    pub fn factory(&self) -> &[Workspace] {
        &self.factory
    }

    /// The factory layout of a name.
    pub fn factory_layout(&self, name: &str) -> Option<&Layout> {
        self.factory
            .iter()
            .find(|w| w.name == name)
            .map(|w| &w.layout)
    }

    /// Switches to the workspace with this name.
    pub fn switch(&mut self, name: &str) -> Result<(), DockError> {
        match self.items.iter().position(|w| w.name == name) {
            Some(i) => {
                self.current = i;
                Ok(())
            }
            None => Err(DockError::BadName),
        }
    }

    /// Saves the layout in use under a name (a workspace with that name is replaced) and switches to it.
    pub fn save_as(&mut self, name: &str) -> Result<(), DockError> {
        if !valid_name(name) {
            return Err(DockError::BadName);
        }
        let layout = self.layout().clone();
        match self.items.iter().position(|w| w.name == name) {
            Some(i) => {
                self.items[i].layout = layout;
                self.current = i;
            }
            None => {
                if self.items.len() >= MAX_WORKSPACES {
                    return Err(DockError::BadName);
                }
                self.items.push(Workspace::new(name, layout));
                self.current = self.items.len() - 1;
            }
        }
        Ok(())
    }

    /// Deletes the workspace in use (the last one cannot be deleted) and switches to the first.
    pub fn delete_current(&mut self) -> Result<(), DockError> {
        if self.items.len() <= 1 {
            return Err(DockError::BadName);
        }
        self.items.remove(self.current);
        self.current = 0;
        Ok(())
    }

    /// Gives the workspace in use its factory layout (the first factory layout when the name is not a factory one). The lock is kept as it is.
    pub fn restore_current(&mut self) {
        let locked = self.layout().locked;
        let fresh = self
            .factory_layout(self.current_name())
            .or_else(|| self.factory.first().map(|w| &w.layout))
            .cloned()
            .unwrap_or_default();
        *self.layout_mut() = fresh;
        self.layout_mut().locked = locked;
    }

    /// Puts back all the factory workspaces and switches to the first.
    pub fn restore_all(&mut self) {
        self.items = self.factory.clone();
        self.current = 0;
    }

    /// The pairs for the settings file: the version, the workspace in use and every workspace (`dock.ws.<n>.name` and its layout keys).
    pub fn to_pairs(&self) -> Vec<(String, String)> {
        let mut out = vec![
            ("dock.version".to_string(), FORMAT_VERSION.to_string()),
            ("dock.current".to_string(), self.current.to_string()),
        ];
        for (i, w) in self.items.iter().enumerate() {
            out.push((format!("dock.ws.{i}.name"), w.name.clone()));
            for (k, v) in w.layout.to_pairs() {
                out.push((format!("dock.ws.{i}.{k}"), v));
            }
        }
        out
    }

    /// Reads the pairs of the settings file. `known` is every panel the program has. A file with no lines of the frame is a first run (the factory
    /// layouts, no warning); anything unreadable gives the factory layout and a warning.
    pub fn from_pairs(
        pairs: &[(String, String)],
        factory: Vec<Workspace>,
        known: &[&str],
    ) -> (Workspaces, Vec<LoadWarning>) {
        let mut w = Workspaces::new(factory);
        let mut warnings = Vec::new();
        let get = |k: &str| {
            pairs
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.as_str())
        };
        let any = pairs.iter().any(|(k, _)| {
            k.starts_with("dock.version") || k.starts_with("dock.ws.") || k == "dock.current"
        });
        if !any {
            return (w, warnings);
        }
        if get("dock.version").map(str::trim) != Some(&FORMAT_VERSION.to_string()) {
            warnings.push(LoadWarning {
                kind: WarnKind::OtherVersion,
                detail: String::new(),
            });
            return (w, warnings);
        }
        let mut items: Vec<Workspace> = Vec::new();
        for i in 0..MAX_WORKSPACES {
            let prefix = format!("dock.ws.{i}.");
            let Some(name) = get(&format!("{prefix}name")) else {
                continue;
            };
            if !valid_name(name) || items.iter().any(|x| x.name == name) {
                warnings.push(LoadWarning {
                    kind: WarnKind::Corrupt,
                    detail: format!("#{i}"),
                });
                continue;
            }
            let own: Vec<(String, String)> = pairs
                .iter()
                .filter_map(|(k, v)| k.strip_prefix(&prefix).map(|r| (r.to_string(), v.clone())))
                .collect();
            match Layout::from_pairs(&own) {
                Ok(mut layout) => {
                    if !layout.repair(known).is_empty() {
                        warnings.push(LoadWarning {
                            kind: WarnKind::Repaired,
                            detail: name.to_string(),
                        });
                    }
                    items.push(Workspace::new(name, layout));
                }
                Err(_) => {
                    warnings.push(LoadWarning {
                        kind: WarnKind::Corrupt,
                        detail: name.to_string(),
                    });
                    if let Some(f) = w.factory_layout(name) {
                        items.push(Workspace::new(name, f.clone()));
                    }
                }
            }
        }
        if items.is_empty() {
            if !warnings.iter().any(|x| x.kind == WarnKind::Corrupt) {
                warnings.push(LoadWarning {
                    kind: WarnKind::Corrupt,
                    detail: String::new(),
                });
            }
            return (w, warnings);
        }
        let current = get("dock.current")
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);
        w.items = items;
        w.current = if current < w.items.len() { current } else { 0 };
        (w, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Node;

    const KNOWN: [&str; 4] = ["a", "b", "c", "d"];

    fn factory() -> Vec<Workspace> {
        vec![
            Workspace::new(
                "Prepare",
                Layout::with_root(Node::row(0.3, Node::tabs(&["a", "b"]), Node::tabs(&["c"])))
                    .with_hidden(&["d"]),
            ),
            Workspace::new(
                "Review",
                Layout::with_root(Node::tabs(&["a", "b", "c"])).with_hidden(&["d"]),
            ),
            Workspace::new(
                "Compact",
                Layout::with_root(Node::tabs(&["a"])).with_hidden(&["b", "c", "d"]),
            ),
        ]
    }

    #[test]
    fn the_first_run_uses_the_factory_without_warning() {
        let (w, warn) = Workspaces::from_pairs(&[], factory(), &KNOWN);
        assert!(warn.is_empty());
        assert_eq!(w.names(), ["Prepare", "Review", "Compact"]);
        assert_eq!(w.current_name(), "Prepare");
        let (w2, warn2) =
            Workspaces::from_pairs(&[("language".into(), "es".into())], factory(), &KNOWN);
        assert!(warn2.is_empty());
        assert_eq!(w2, w);
    }

    #[test]
    fn workspaces_round_trip_through_the_pairs() {
        let mut w = Workspaces::new(factory());
        w.layout_mut().hide("a").unwrap();
        w.save_as("Mine").unwrap();
        w.switch("Review").unwrap();
        let (back, warn) = Workspaces::from_pairs(&w.to_pairs(), factory(), &KNOWN);
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(back, w);
        assert_eq!(back.current_name(), "Review");
        assert!(back.names().contains(&"Mine"));
    }

    #[test]
    fn another_version_gives_the_factory_and_a_named_warning() {
        let mut pairs = Workspaces::new(factory()).to_pairs();
        pairs[0].1 = "9".into();
        let (w, warn) = Workspaces::from_pairs(&pairs, factory(), &KNOWN);
        assert_eq!(warn.len(), 1);
        assert_eq!(warn[0].name(), "layout_other_version");
        assert_eq!(w, Workspaces::new(factory()));
    }

    #[test]
    fn a_damaged_workspace_goes_back_to_its_factory_layout_and_the_others_stay() {
        let mut w = Workspaces::new(factory());
        w.layout_mut().hide("a").unwrap();
        let mut pairs = w.to_pairs();
        for (k, v) in pairs.iter_mut() {
            if k == "dock.ws.1.tree" {
                *v = "T0(((".into();
            }
        }
        let (back, warn) = Workspaces::from_pairs(&pairs, factory(), &KNOWN);
        assert!(warn
            .iter()
            .any(|x| x.name() == "layout_corrupt" && x.detail == "Review"));
        assert_eq!(back.names(), ["Prepare", "Review", "Compact"]);
        assert_eq!(back.factory_layout("Review"), Some(&back.items[1].layout));
        assert!(
            !back.items[0].layout.is_shown("a"),
            "the healthy one keeps the person's change"
        );
    }

    #[test]
    fn all_damaged_gives_the_factory() {
        let pairs: Vec<(String, String)> = vec![
            ("dock.version".into(), "1".into()),
            ("dock.ws.0.name".into(), "Mine".into()),
            ("dock.ws.0.tree".into(), "garbage".into()),
        ];
        let (w, warn) = Workspaces::from_pairs(&pairs, factory(), &KNOWN);
        assert!(!warn.is_empty());
        assert_eq!(w, Workspaces::new(factory()));
    }

    #[test]
    fn a_layout_with_a_panel_that_does_not_exist_is_repaired() {
        let mut pairs = Workspaces::new(factory()).to_pairs();
        pairs.push(("dock.ws.0.hidden".into(), "zzz".into()));
        let (w, warn) = Workspaces::from_pairs(&pairs, factory(), &KNOWN);
        assert!(warn.iter().any(|x| x.name() == "layout_repaired"));
        assert!(!w.layout().panels().contains(&"zzz".to_string()));
    }

    #[test]
    fn saving_deleting_and_the_limits() {
        let mut w = Workspaces::new(factory());
        assert_eq!(w.save_as(""), Err(DockError::BadName));
        assert_eq!(w.save_as(" x"), Err(DockError::BadName));
        assert_eq!(w.save_as("a\nb"), Err(DockError::BadName));
        assert_eq!(
            w.save_as(&"n".repeat(MAX_NAME + 1)),
            Err(DockError::BadName)
        );
        w.save_as("Review").unwrap();
        assert_eq!(w.names().len(), 3, "the same name replaces");
        for i in 0..MAX_WORKSPACES {
            let _ = w.save_as(&format!("w{i}"));
        }
        assert_eq!(w.names().len(), MAX_WORKSPACES);
        while w.delete_current().is_ok() {}
        assert_eq!(w.names().len(), 1);
        assert_eq!(w.delete_current(), Err(DockError::BadName));
        assert_eq!(w.switch("zzz"), Err(DockError::BadName));
    }

    #[test]
    fn restoring_the_factory_keeps_the_lock_and_restore_all_brings_back_the_set() {
        let mut w = Workspaces::new(factory());
        w.layout_mut().hide("a").unwrap();
        w.layout_mut().set_locked(true);
        w.restore_current();
        assert!(w.layout().is_shown("a"));
        assert!(w.layout().locked);
        w.save_as("Mine").unwrap();
        w.restore_current();
        assert_eq!(
            w.layout().panels().len(),
            4,
            "a name that is not a factory one gets the first factory layout"
        );
        w.restore_all();
        assert_eq!(w, Workspaces::new(factory()));
    }

    #[test]
    fn warnings_speak_every_language() {
        let w = LoadWarning {
            kind: WarnKind::Corrupt,
            detail: "Review".into(),
        };
        assert!(w.text(Lang::Es).contains("dañada"));
        assert!(w.text(Lang::Ja).contains("Review"));
        for kind in [
            WarnKind::Corrupt,
            WarnKind::OtherVersion,
            WarnKind::Repaired,
            WarnKind::BadShortcut,
        ] {
            let w = LoadWarning {
                kind,
                detail: String::new(),
            };
            for lang in Lang::ALL {
                assert!(!w.text(lang).is_empty());
            }
        }
    }
}
