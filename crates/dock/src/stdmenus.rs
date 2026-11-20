//! The menus every program can take as they are: File, Edit, View, Window and Help, with the usual shortcuts. A program takes the ones it wants,
//! adds its own entries beside them and its own menus (Tools, and so on), and gets back the actions as values of its own type.

use crate::menu::{DockAction, Entry, Gen, Label, Menu, MenuBar, ResolvedMenu, StdAction};
use crate::shortcuts::Shortcut;
use crate::texts::DockText;

/// What the Help menu offers besides the window about the program and the shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpOptions {
    /// An entry for the online documentation (F1).
    pub online_help: bool,
    /// An entry to look for a newer version (the program must handle `StdAction::CheckUpdates`).
    pub check_updates: bool,
}

impl Default for HelpOptions {
    fn default() -> Self {
        HelpOptions {
            online_help: true,
            check_updates: false,
        }
    }
}

fn std_entry<A: From<StdAction>>(
    id: &str,
    label: DockText,
    action: StdAction,
    key: Option<Shortcut>,
) -> Entry<A> {
    let e = Entry::app(id, Label::Std(label), A::from(action));
    match key {
        Some(k) => e.key(k),
        None => e,
    }
}

/// File: New, Open…, Open recent (when `recent`), Save, Save as…, Export…, Quit.
pub fn file_menu<A: From<StdAction>>(recent: bool) -> Menu<A> {
    let mut m = Menu::new("file", Label::Std(DockText::MenuFile))
        .entry(std_entry(
            "file.new",
            DockText::New,
            StdAction::New,
            Some(Shortcut::ctrl("N")),
        ))
        .entry(std_entry(
            "file.open",
            DockText::Open,
            StdAction::Open,
            Some(Shortcut::ctrl("O")),
        ));
    if recent {
        m = m.generated(Gen::Recent);
    }
    m.sep()
        .entry(std_entry(
            "file.save",
            DockText::Save,
            StdAction::Save,
            Some(Shortcut::ctrl("S")),
        ))
        .entry(std_entry(
            "file.save_as",
            DockText::SaveAs,
            StdAction::SaveAs,
            Some(Shortcut::ctrl_shift("S")),
        ))
        .entry(std_entry(
            "file.export",
            DockText::Export,
            StdAction::Export,
            Some(Shortcut::ctrl("E")),
        ))
        .sep()
        .entry(std_entry(
            "file.quit",
            DockText::Quit,
            StdAction::Quit,
            Some(Shortcut::ctrl("Q")),
        ))
}

/// Edit: Undo, Redo, Cut, Copy, Paste, Preferences….
pub fn edit_menu<A: From<StdAction>>() -> Menu<A> {
    Menu::new("edit", Label::Std(DockText::MenuEdit))
        .entry(std_entry(
            "edit.undo",
            DockText::Undo,
            StdAction::Undo,
            Some(Shortcut::ctrl("Z")),
        ))
        .entry(std_entry(
            "edit.redo",
            DockText::Redo,
            StdAction::Redo,
            Some(Shortcut::ctrl("Y")),
        ))
        .sep()
        .entry(std_entry(
            "edit.cut",
            DockText::Cut,
            StdAction::Cut,
            Some(Shortcut::ctrl("X")),
        ))
        .entry(std_entry(
            "edit.copy",
            DockText::Copy,
            StdAction::Copy,
            Some(Shortcut::ctrl("C")),
        ))
        .entry(std_entry(
            "edit.paste",
            DockText::Paste,
            StdAction::Paste,
            Some(Shortcut::ctrl("V")),
        ))
        .sep()
        .entry(std_entry(
            "edit.preferences",
            DockText::Preferences,
            StdAction::Preferences,
            Some(Shortcut::ctrl("Comma")),
        ))
}

/// View: one checked entry per panel, then restore and lock the layout.
pub fn view_menu<A>() -> Menu<A> {
    Menu::new("view", Label::Std(DockText::MenuView))
        .generated(Gen::Panels)
        .sep()
        .generated(Gen::Layout)
}

/// Window: the workspaces, then next and previous tab and maximise.
pub fn window_menu<A>() -> Menu<A> {
    Menu::new("window", Label::Std(DockText::MenuWindow))
        .generated(Gen::Workspaces)
        .sep()
        .generated(Gen::Tabs)
}

/// An empty Tools menu, for the program to fill.
pub fn tools_menu<A>() -> Menu<A> {
    Menu::new("tools", Label::Std(DockText::MenuTools))
}

/// Help: online help, the shortcuts, look for updates, about.
pub fn help_menu<A: From<StdAction>>(options: HelpOptions) -> Menu<A> {
    let mut m = Menu::new("help", Label::Std(DockText::MenuHelp));
    if options.online_help {
        m = m.entry(std_entry(
            "help.online",
            DockText::OnlineHelp,
            StdAction::OnlineHelp,
            Some(Shortcut::plain("F1")),
        ));
    }
    m = m.entry(Entry::dock(
        "help.shortcuts",
        Label::Std(DockText::Shortcuts),
        DockAction::ShowShortcuts,
    ));
    if options.check_updates {
        m = m.sep().entry(std_entry(
            "help.updates",
            DockText::CheckUpdates,
            StdAction::CheckUpdates,
            None,
        ));
    }
    m.sep().entry(Entry::dock(
        "help.about",
        Label::Std(DockText::About),
        DockAction::ShowAbout,
    ))
}

/// File, Edit, View, Window and Help, in that order. A program usually puts its own Tools menu between View and Window.
pub fn standard_bar<A: From<StdAction> + Clone>(help: HelpOptions) -> MenuBar<A> {
    MenuBar::new(vec![
        file_menu(true),
        edit_menu(),
        view_menu(),
        window_menu(),
        help_menu(help),
    ])
}

/// The rows of the shortcuts window: the title of the menu, the text of the entry and the shortcut, for every entry that has one.
pub fn shortcut_table<A>(menus: &[ResolvedMenu<A>]) -> Vec<(String, String, String)> {
    fn walk<A>(
        title: &str,
        items: &[crate::menu::Resolved<A>],
        out: &mut Vec<(String, String, String)>,
    ) {
        for r in items {
            match &r.kind {
                crate::menu::Kind::Submenu(sub) => walk(title, sub, out),
                _ => {
                    if let Some(s) = &r.shortcut {
                        out.push((title.to_string(), r.label.clone(), s.to_string()));
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for m in menus {
        walk(&m.title, &m.items, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::{Chosen, Kind, MenuContext, PanelEntry};
    use eh3d_view::texts::Lang;

    fn ctx(lang: Lang) -> MenuContext {
        let mut c = MenuContext::empty(lang);
        c.panels = vec![PanelEntry {
            id: "layers".into(),
            title: "Layers".into(),
            shown: true,
        }];
        c.workspaces = vec!["Prepare".into(), "Compact".into()];
        c.current_workspace = "Prepare".into();
        c
    }

    #[test]
    fn the_standard_bar_is_sound_and_has_no_clashes() {
        let bar = standard_bar::<StdAction>(HelpOptions {
            online_help: true,
            check_updates: true,
        });
        assert!(bar.validate().is_empty(), "{:?}", bar.validate());
        assert!(
            bar.conflicts(&ctx(Lang::En)).is_empty(),
            "{:?}",
            bar.conflicts(&ctx(Lang::En))
        );
        let built = bar.resolve(&ctx(Lang::En));
        let titles: Vec<&str> = built.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, ["File", "Edit", "View", "Window", "Help"]);
    }

    #[test]
    fn the_usual_shortcuts_give_the_usual_actions() {
        let bar = standard_bar::<StdAction>(HelpOptions::default());
        let c = ctx(Lang::En);
        for (key, action) in [
            (Shortcut::ctrl("N"), StdAction::New),
            (Shortcut::ctrl("O"), StdAction::Open),
            (Shortcut::ctrl("S"), StdAction::Save),
            (Shortcut::ctrl_shift("S"), StdAction::SaveAs),
            (Shortcut::ctrl("Q"), StdAction::Quit),
            (Shortcut::ctrl("Z"), StdAction::Undo),
            (Shortcut::ctrl("C"), StdAction::Copy),
            (Shortcut::plain("F1"), StdAction::OnlineHelp),
        ] {
            assert_eq!(
                bar.shortcut_action(&c, &key),
                Some(Chosen::App(action)),
                "{key}"
            );
        }
    }

    #[test]
    fn help_options_add_and_remove_entries() {
        let none = help_menu::<StdAction>(HelpOptions {
            online_help: false,
            check_updates: false,
        });
        let all = help_menu::<StdAction>(HelpOptions {
            online_help: true,
            check_updates: true,
        });
        let count = |m: &Menu<StdAction>| {
            m.items
                .iter()
                .filter(|i| matches!(i, crate::menu::Item::Entry(_)))
                .count()
        };
        assert_eq!(count(&none), 2, "shortcuts and about are always there");
        assert_eq!(count(&all), 4);
    }

    #[test]
    fn the_shortcut_table_lists_each_shortcut_once() {
        let bar = standard_bar::<StdAction>(HelpOptions::default());
        let built = bar.resolve(&ctx(Lang::Es));
        let rows = shortcut_table(&built);
        assert!(rows
            .iter()
            .any(|(m, l, s)| m == "Archivo" && l == "Guardar" && s == "Ctrl+S"));
        let mut keys: Vec<&String> = rows.iter().map(|r| &r.2).collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n);
    }

    #[test]
    fn the_standard_bar_speaks_every_language() {
        let bar = standard_bar::<StdAction>(HelpOptions {
            online_help: true,
            check_updates: true,
        });
        for lang in Lang::ALL {
            let built = bar.resolve(&ctx(lang));
            for m in &built {
                assert!(!m.title.is_empty());
                for i in &m.items {
                    if !matches!(i.kind, Kind::Separator) {
                        assert!(!i.label.is_empty(), "{lang:?} {}", i.id);
                    }
                }
            }
        }
    }
}
