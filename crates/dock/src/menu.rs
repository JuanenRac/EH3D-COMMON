//! The menu bar as data. A program declares its menus (a stable id, a title in eight languages, entries with a shortcut, a check mark, a separator,
//! a submenu, enabled or not, and the action each one gives back as a value) and the frame draws them. The View, Window and Open recent menus are
//! generated from what the frame knows: the panels, the workspaces, the files opened last. Everything here is plain data and plain functions:
//! building a menu from its declaration, finding the action of a shortcut, finding two entries that share one, and the rule that an entry which is
//! turned off never gives an action back.

use crate::geom::Side;
use crate::shortcuts::{find_conflicts, Conflict, DockKey, DockKeys, Shortcut};
use crate::texts::{tr, DockText, Text8};
use eh3d_view::texts::Lang;

/// A text of a menu: one of the frame's own words, or a text the program gives in eight languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// A word of the frame.
    Std(DockText),
    /// A text of the program.
    Own(Text8),
}

impl Label {
    /// The text in a language.
    pub fn get(&self, lang: Lang) -> &'static str {
        match self {
            Label::Std(t) => tr(lang, *t),
            Label::Own(t) => t.get(lang),
        }
    }
}

/// The actions every program has, which the standard menus give back; the program decides what each does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdAction {
    /// New file.
    New,
    /// Open a file.
    Open,
    /// Save.
    Save,
    /// Save under another name.
    SaveAs,
    /// Export.
    Export,
    /// Leave the program.
    Quit,
    /// Undo.
    Undo,
    /// Redo.
    Redo,
    /// Cut.
    Cut,
    /// Copy.
    Copy,
    /// Paste.
    Paste,
    /// Open the preferences.
    Preferences,
    /// Open the online documentation.
    OnlineHelp,
    /// Look for a newer version.
    CheckUpdates,
}

/// What the frame does by itself when its entries are chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockAction {
    /// Show the panel if it is hidden, hide it if it is shown.
    TogglePanel(String),
    /// Show the panel.
    ShowPanel(String),
    /// Hide the panel.
    HidePanel(String),
    /// Make the panel float.
    FloatPanel(String),
    /// Bring the panel back to the tree.
    DockPanel(String),
    /// Fold the panel into a side of the window.
    AutoHidePanel(String, Side),
    /// Move the panel one step towards a side.
    MovePanel(String, Side),
    /// Maximise or restore the panel with the focus.
    ToggleMaximize,
    /// The next tab of the group with the focus.
    NextTab,
    /// The previous tab of the group with the focus.
    PrevTab,
    /// Give the workspace its factory layout.
    RestoreLayout,
    /// Lock or unlock the layout.
    ToggleLock,
    /// Switch to the workspace with this name.
    SwitchWorkspace(String),
    /// Ask for a name and save the layout under it.
    AskSaveWorkspace,
    /// Save the layout under this name.
    SaveWorkspaceAs(String),
    /// Delete the workspace in use.
    DeleteWorkspace,
    /// Show the window about the program.
    ShowAbout,
    /// Show the window with the shortcuts.
    ShowShortcuts,
    /// Empty the list of recent files.
    ClearRecent,
}

/// What a chosen entry gives back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chosen<A> {
    /// An action of the program.
    App(A),
    /// An action the frame has done already (the program may ignore it, or look at it).
    Dock(DockAction),
    /// A recent file the person picked.
    Recent(String),
}

/// The action an entry stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act<A> {
    /// A program's action, given back as it is.
    App(A),
    /// A frame's action.
    Dock(DockAction),
}

/// An entry of a menu.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry<A> {
    /// A stable name, unique in the bar; the program uses it to turn the entry off or on and to tick it.
    pub id: String,
    /// What it says.
    pub label: Label,
    /// Its shortcut.
    pub shortcut: Option<Shortcut>,
    /// A check mark: `Some(true)` ticked, `Some(false)` empty, `None` no mark.
    pub checked: Option<bool>,
    /// Whether it can be chosen.
    pub enabled: bool,
    /// What it gives back.
    pub action: Act<A>,
}

impl<A> Entry<A> {
    /// An entry of the program.
    pub fn app(id: &str, label: Label, action: A) -> Entry<A> {
        Entry {
            id: id.to_string(),
            label,
            shortcut: None,
            checked: None,
            enabled: true,
            action: Act::App(action),
        }
    }

    /// An entry of the frame.
    pub fn dock(id: &str, label: Label, action: DockAction) -> Entry<A> {
        Entry {
            id: id.to_string(),
            label,
            shortcut: None,
            checked: None,
            enabled: true,
            action: Act::Dock(action),
        }
    }

    /// The same entry with a shortcut.
    pub fn key(mut self, shortcut: Shortcut) -> Entry<A> {
        self.shortcut = Some(shortcut);
        self
    }

    /// The same entry with a check mark.
    pub fn check(mut self, on: bool) -> Entry<A> {
        self.checked = Some(on);
        self
    }

    /// The same entry turned off.
    pub fn disabled(mut self) -> Entry<A> {
        self.enabled = false;
        self
    }
}

/// The parts of a menu the frame fills in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gen {
    /// One checked entry per panel.
    Panels,
    /// Restore the layout and lock it.
    Layout,
    /// The workspaces, save as and delete.
    Workspaces,
    /// Next tab, previous tab, maximise.
    Tabs,
    /// A submenu with the files opened last.
    Recent,
}

/// An item of a menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Item<A> {
    /// An entry.
    Entry(Entry<A>),
    /// A line between groups.
    Separator,
    /// A menu inside the menu.
    Submenu(Menu<A>),
    /// Entries the frame fills in.
    Generated(Gen),
}

/// A menu.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu<A> {
    /// A stable name, unique in the bar.
    pub id: String,
    /// Its title.
    pub title: Label,
    /// What it holds.
    pub items: Vec<Item<A>>,
}

impl<A> Menu<A> {
    /// An empty menu.
    pub fn new(id: &str, title: Label) -> Menu<A> {
        Menu {
            id: id.to_string(),
            title,
            items: Vec::new(),
        }
    }

    /// The same menu with one more entry.
    pub fn entry(mut self, e: Entry<A>) -> Menu<A> {
        self.items.push(Item::Entry(e));
        self
    }

    /// The same menu with a separator.
    pub fn sep(mut self) -> Menu<A> {
        self.items.push(Item::Separator);
        self
    }

    /// The same menu with a submenu.
    pub fn submenu(mut self, m: Menu<A>) -> Menu<A> {
        self.items.push(Item::Submenu(m));
        self
    }

    /// The same menu with entries the frame fills in.
    pub fn generated(mut self, g: Gen) -> Menu<A> {
        self.items.push(Item::Generated(g));
        self
    }
}

/// A panel as the menus see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelEntry {
    /// The name of the panel.
    pub id: String,
    /// Its title in the language of the window.
    pub title: String,
    /// Whether it is shown.
    pub shown: bool,
}

/// What the frame and the program know when the menus are built.
#[derive(Debug, Clone)]
pub struct MenuContext {
    /// The language.
    pub lang: Lang,
    /// The panels, in the order the menu lists them.
    pub panels: Vec<PanelEntry>,
    /// The names of the workspaces.
    pub workspaces: Vec<String>,
    /// The workspace in use.
    pub current_workspace: String,
    /// Whether the layout is locked.
    pub locked: bool,
    /// Whether a panel is maximised.
    pub maximized: bool,
    /// The files opened last, the newest first.
    pub recent: Vec<String>,
    /// The shortcuts of the frame.
    pub keys: DockKeys,
    /// Entries the program turns off, by id.
    pub disabled: Vec<String>,
    /// Check marks the program sets, by id.
    pub checks: Vec<(String, bool)>,
}

impl MenuContext {
    /// A context with nothing in it, in a language.
    pub fn empty(lang: Lang) -> MenuContext {
        MenuContext {
            lang,
            panels: Vec::new(),
            workspaces: Vec::new(),
            current_workspace: String::new(),
            locked: false,
            maximized: false,
            recent: Vec::new(),
            keys: DockKeys::defaults(),
            disabled: Vec::new(),
            checks: Vec::new(),
        }
    }
}

/// What an item of a built menu is.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind<A> {
    /// An entry that can be chosen, with what it gives back. An entry that is turned off is never this.
    Action(Chosen<A>),
    /// An entry that is turned off.
    Inert,
    /// A line between groups.
    Separator,
    /// A line of text that is not an entry («(none)»).
    Note,
    /// A menu inside the menu.
    Submenu(Vec<Resolved<A>>),
}

/// An item of a menu built for one moment: the text is already in the language.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved<A> {
    /// The id of the entry (empty for separators and notes).
    pub id: String,
    /// The text.
    pub label: String,
    /// Its shortcut.
    pub shortcut: Option<Shortcut>,
    /// Its check mark.
    pub checked: Option<bool>,
    /// What it is.
    pub kind: Kind<A>,
}

impl<A> Resolved<A> {
    /// What choosing the item gives: `None` for an item that is turned off, a separator, a note or a submenu.
    pub fn activate(&self) -> Option<&Chosen<A>> {
        match &self.kind {
            Kind::Action(c) => Some(c),
            _ => None,
        }
    }

    /// Whether the item can be chosen.
    pub fn is_enabled(&self) -> bool {
        matches!(self.kind, Kind::Action(_) | Kind::Submenu(_))
    }
}

/// A menu built for one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedMenu<A> {
    /// The id of the menu.
    pub id: String,
    /// Its title in the language.
    pub title: String,
    /// Its items.
    pub items: Vec<Resolved<A>>,
}

/// Something wrong in the declaration of a menu bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuProblem {
    /// Two menus or two entries share an id.
    DuplicateId(String),
    /// An id that is empty.
    EmptyId,
    /// A menu with nothing in it.
    EmptyMenu(String),
}

/// A menu bar: the menus, left to right.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuBar<A> {
    /// The menus.
    pub menus: Vec<Menu<A>>,
}

fn shorten(path: &str) -> String {
    let n = path.chars().count();
    if n <= 60 {
        return path.to_string();
    }
    let head: String = path.chars().take(20).collect();
    let tail: String = path.chars().skip(n - 37).collect();
    format!("{head}…{tail}")
}

fn make_entry<A: Clone>(e: &Entry<A>, ctx: &MenuContext) -> Resolved<A> {
    let enabled = e.enabled && !ctx.disabled.contains(&e.id);
    let checked = ctx
        .checks
        .iter()
        .find(|(id, _)| *id == e.id)
        .map(|(_, c)| *c)
        .or(e.checked);
    let kind = if enabled {
        Kind::Action(match &e.action {
            Act::App(a) => Chosen::App(a.clone()),
            Act::Dock(d) => Chosen::Dock(d.clone()),
        })
    } else {
        Kind::Inert
    };
    Resolved {
        id: e.id.clone(),
        label: e.label.get(ctx.lang).to_string(),
        shortcut: e.shortcut.clone(),
        checked,
        kind,
    }
}

fn dock_item<A>(
    id: &str,
    label: DockText,
    action: DockAction,
    shortcut: Option<Shortcut>,
    checked: Option<bool>,
    enabled: bool,
    ctx: &MenuContext,
) -> Resolved<A> {
    Resolved {
        id: id.to_string(),
        label: tr(ctx.lang, label).to_string(),
        shortcut,
        checked,
        kind: if enabled {
            Kind::Action(Chosen::Dock(action))
        } else {
            Kind::Inert
        },
    }
}

fn separator<A>() -> Resolved<A> {
    Resolved {
        id: String::new(),
        label: String::new(),
        shortcut: None,
        checked: None,
        kind: Kind::Separator,
    }
}

fn generate<A: Clone>(g: Gen, ctx: &MenuContext, out: &mut Vec<Resolved<A>>) {
    match g {
        Gen::Panels => {
            for p in &ctx.panels {
                out.push(Resolved {
                    id: format!("panel.{}", p.id),
                    label: p.title.clone(),
                    shortcut: ctx.keys.get(&DockKey::Panel(p.id.clone())).cloned(),
                    checked: Some(p.shown),
                    kind: if ctx.locked {
                        Kind::Inert
                    } else {
                        Kind::Action(Chosen::Dock(DockAction::TogglePanel(p.id.clone())))
                    },
                });
            }
        }
        Gen::Layout => {
            out.push(dock_item(
                "layout.restore",
                DockText::RestoreLayout,
                DockAction::RestoreLayout,
                ctx.keys.get(&DockKey::RestoreLayout).cloned(),
                None,
                true,
                ctx,
            ));
            out.push(dock_item(
                "layout.lock",
                DockText::LockLayout,
                DockAction::ToggleLock,
                ctx.keys.get(&DockKey::ToggleLock).cloned(),
                Some(ctx.locked),
                true,
                ctx,
            ));
        }
        Gen::Workspaces => {
            for w in &ctx.workspaces {
                out.push(Resolved {
                    id: format!("workspace.{w}"),
                    label: crate::sectors::workspace_title(w)
                        .map(|t| t.get(ctx.lang).to_string())
                        .unwrap_or_else(|| w.clone()),
                    shortcut: None,
                    checked: Some(*w == ctx.current_workspace),
                    kind: Kind::Action(Chosen::Dock(DockAction::SwitchWorkspace(w.clone()))),
                });
            }
            out.push(separator());
            out.push(dock_item(
                "workspace.save",
                DockText::SaveWorkspace,
                DockAction::AskSaveWorkspace,
                None,
                None,
                !ctx.locked,
                ctx,
            ));
            out.push(dock_item(
                "workspace.delete",
                DockText::DeleteWorkspace,
                DockAction::DeleteWorkspace,
                None,
                None,
                ctx.workspaces.len() > 1 && !ctx.locked,
                ctx,
            ));
        }
        Gen::Tabs => {
            out.push(dock_item(
                "tabs.next",
                DockText::NextTab,
                DockAction::NextTab,
                ctx.keys.get(&DockKey::NextTab).cloned(),
                None,
                true,
                ctx,
            ));
            out.push(dock_item(
                "tabs.prev",
                DockText::PrevTab,
                DockAction::PrevTab,
                ctx.keys.get(&DockKey::PrevTab).cloned(),
                None,
                true,
                ctx,
            ));
            out.push(dock_item(
                "tabs.maximize",
                DockText::Maximize,
                DockAction::ToggleMaximize,
                ctx.keys.get(&DockKey::Maximize).cloned(),
                Some(ctx.maximized),
                true,
                ctx,
            ));
        }
        Gen::Recent => {
            let mut sub: Vec<Resolved<A>> = Vec::new();
            if ctx.recent.is_empty() {
                sub.push(Resolved {
                    id: String::new(),
                    label: tr(ctx.lang, DockText::NoRecent).to_string(),
                    shortcut: None,
                    checked: None,
                    kind: Kind::Note,
                });
            } else {
                for (i, p) in ctx.recent.iter().enumerate() {
                    sub.push(Resolved {
                        id: format!("recent.{i}"),
                        label: shorten(p),
                        shortcut: None,
                        checked: None,
                        kind: Kind::Action(Chosen::Recent(p.clone())),
                    });
                }
                sub.push(separator());
                sub.push(dock_item(
                    "recent.clear",
                    DockText::ClearRecent,
                    DockAction::ClearRecent,
                    None,
                    None,
                    true,
                    ctx,
                ));
            }
            out.push(Resolved {
                id: "recent".to_string(),
                label: tr(ctx.lang, DockText::OpenRecent).to_string(),
                shortcut: None,
                checked: None,
                kind: Kind::Submenu(sub),
            });
        }
    }
}

fn build_items<A: Clone>(items: &[Item<A>], ctx: &MenuContext) -> Vec<Resolved<A>> {
    let mut out: Vec<Resolved<A>> = Vec::new();
    for it in items {
        match it {
            Item::Entry(e) => out.push(make_entry(e, ctx)),
            Item::Separator => {
                // no separator first, last or twice in a row
                if !out.is_empty() && !matches!(out.last().map(|r| &r.kind), Some(Kind::Separator))
                {
                    out.push(separator());
                }
            }
            Item::Submenu(m) => out.push(Resolved {
                id: m.id.clone(),
                label: m.title.get(ctx.lang).to_string(),
                shortcut: None,
                checked: None,
                kind: Kind::Submenu(build_items(&m.items, ctx)),
            }),
            Item::Generated(g) => generate(*g, ctx, &mut out),
        }
    }
    while matches!(out.last().map(|r| &r.kind), Some(Kind::Separator)) {
        out.pop();
    }
    out
}

fn collect_shortcuts<A>(items: &[Resolved<A>], path: &str, out: &mut Vec<(String, Shortcut)>) {
    for r in items {
        match &r.kind {
            Kind::Submenu(sub) => collect_shortcuts(sub, path, out),
            Kind::Action(_) | Kind::Inert => {
                if let Some(s) = &r.shortcut {
                    out.push((format!("{path}/{}", r.id), s.clone()));
                }
            }
            _ => {}
        }
    }
}

fn find_action<'a, A>(items: &'a [Resolved<A>], s: &Shortcut) -> Option<&'a Chosen<A>> {
    for r in items {
        match &r.kind {
            Kind::Submenu(sub) => {
                if let Some(c) = find_action(sub, s) {
                    return Some(c);
                }
            }
            Kind::Action(c) if r.shortcut.as_ref() == Some(s) => return Some(c),
            _ => {}
        }
    }
    None
}

fn ids_of<A>(items: &[Item<A>], out: &mut Vec<String>) {
    for it in items {
        match it {
            Item::Entry(e) => out.push(e.id.clone()),
            Item::Submenu(m) => {
                out.push(m.id.clone());
                ids_of(&m.items, out);
            }
            _ => {}
        }
    }
}

impl<A: Clone> MenuBar<A> {
    /// A bar of these menus.
    pub fn new(menus: Vec<Menu<A>>) -> MenuBar<A> {
        MenuBar { menus }
    }

    /// Builds the menus for one moment: the text in the language, the generated entries filled in, the entries the program turned off inert.
    /// A separator is never first, last or doubled, and a menu that ends up with nothing in it is left out.
    pub fn resolve(&self, ctx: &MenuContext) -> Vec<ResolvedMenu<A>> {
        self.menus
            .iter()
            .map(|m| ResolvedMenu {
                id: m.id.clone(),
                title: m.title.get(ctx.lang).to_string(),
                items: build_items(&m.items, ctx),
            })
            .filter(|m| !m.items.is_empty())
            .collect()
    }

    /// What a shortcut does: the action of the entry that has it, if that entry is on. Entries that are turned off never answer.
    pub fn shortcut_action(&self, ctx: &MenuContext, s: &Shortcut) -> Option<Chosen<A>> {
        let built = self.resolve(ctx);
        built.iter().find_map(|m| find_action(&m.items, s).cloned())
    }

    /// The shortcuts that two or more entries share, counting the shortcuts of the frame that are not in a menu.
    pub fn conflicts(&self, ctx: &MenuContext) -> Vec<Conflict> {
        let mut all: Vec<(String, Shortcut)> = Vec::new();
        for m in self.resolve(ctx) {
            collect_shortcuts(&m.items, &m.id, &mut all);
        }
        for (name, s) in ctx.keys.named() {
            // a shortcut of the frame that a generated entry already shows is the same entry, not a second one
            if !all
                .iter()
                .any(|(n, sc)| *sc == s && n.ends_with(&frame_entry_id(&name)))
            {
                all.push((format!("dock.key.{name}"), s));
            }
        }
        find_conflicts(&all)
    }

    /// What is wrong with the declaration: ids that repeat or are empty, menus with nothing in them.
    pub fn validate(&self) -> Vec<MenuProblem> {
        let mut problems = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for m in &self.menus {
            let mut ids = vec![m.id.clone()];
            ids_of(&m.items, &mut ids);
            if m.items.is_empty() {
                problems.push(MenuProblem::EmptyMenu(m.id.clone()));
            }
            for id in ids {
                if id.is_empty() {
                    problems.push(MenuProblem::EmptyId);
                } else if seen.contains(&id) {
                    problems.push(MenuProblem::DuplicateId(id));
                } else {
                    seen.push(id);
                }
            }
        }
        problems
    }
}

fn frame_entry_id(key_name: &str) -> String {
    match key_name {
        "maximize" => "tabs.maximize".into(),
        "next_tab" => "tabs.next".into(),
        "prev_tab" => "tabs.prev".into(),
        "restore_layout" => "layout.restore".into(),
        "toggle_lock" => "layout.lock".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Cmd {
        Open,
        Save,
        Slice,
        Grid,
    }

    const SLICE: Label = Label::Own(Text8([
        "Slice",
        "Laminar",
        "Elabora",
        "Slicen",
        "Trancher",
        "Fatiar",
        "切片",
        "スライス",
    ]));

    fn bar() -> MenuBar<Cmd> {
        MenuBar::new(vec![
            Menu::new("file", Label::Std(DockText::MenuFile))
                .entry(
                    Entry::app("open", Label::Std(DockText::Open), Cmd::Open)
                        .key(Shortcut::ctrl("O")),
                )
                .generated(Gen::Recent)
                .sep()
                .entry(
                    Entry::app("save", Label::Std(DockText::Save), Cmd::Save)
                        .key(Shortcut::ctrl("S"))
                        .disabled(),
                ),
            Menu::new("tools", Label::Std(DockText::MenuTools))
                .entry(Entry::app("slice", SLICE, Cmd::Slice).key(Shortcut::plain("F5")))
                .entry(Entry::app("grid", Label::Own(Text8::same("Grid")), Cmd::Grid).check(true)),
            Menu::new("view", Label::Std(DockText::MenuView))
                .generated(Gen::Panels)
                .sep()
                .generated(Gen::Layout),
            Menu::new("window", Label::Std(DockText::MenuWindow))
                .generated(Gen::Workspaces)
                .sep()
                .generated(Gen::Tabs),
        ])
    }

    fn ctx() -> MenuContext {
        let mut c = MenuContext::empty(Lang::Es);
        c.panels = vec![
            PanelEntry {
                id: "layers".into(),
                title: "Capas".into(),
                shown: true,
            },
            PanelEntry {
                id: "console".into(),
                title: "Consola".into(),
                shown: false,
            },
        ];
        c.workspaces = vec!["Preparar".into(), "Revisar".into()];
        c.current_workspace = "Revisar".into();
        c.recent = vec!["/jobs/uno.eh3dproj".into()];
        c
    }

    fn find<'a>(items: &'a [Resolved<Cmd>], id: &str) -> Option<&'a Resolved<Cmd>> {
        for r in items {
            if r.id == id {
                return Some(r);
            }
            if let Kind::Submenu(s) = &r.kind {
                if let Some(x) = find(s, id) {
                    return Some(x);
                }
            }
        }
        None
    }

    #[test]
    fn the_menus_are_built_in_the_language_with_the_generated_parts() {
        let built = bar().resolve(&ctx());
        let titles: Vec<&str> = built.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, ["Archivo", "Herramientas", "Ver", "Ventana"]);
        let view = &built[2];
        assert_eq!(view.items[0].label, "Capas");
        assert_eq!(view.items[0].checked, Some(true));
        assert_eq!(view.items[1].checked, Some(false));
        assert_eq!(
            find(&view.items, "layout.lock").unwrap().checked,
            Some(false)
        );
        let win = &built[3];
        assert_eq!(
            find(&win.items, "workspace.Revisar").unwrap().checked,
            Some(true)
        );
        assert_eq!(
            find(&win.items, "workspace.Preparar").unwrap().checked,
            Some(false)
        );
        let recent = find(&built[0].items, "recent").unwrap();
        assert!(
            matches!(&recent.kind, Kind::Submenu(s) if s[0].activate() == Some(&Chosen::Recent("/jobs/uno.eh3dproj".into())))
        );
    }

    #[test]
    fn an_entry_that_is_turned_off_never_gives_an_action() {
        let b = bar();
        let mut c = ctx();
        let built = b.resolve(&c);
        let save = find(&built[0].items, "save").unwrap();
        assert!(save.activate().is_none() && !save.is_enabled());
        assert_eq!(b.shortcut_action(&c, &Shortcut::ctrl("S")), None);
        // the program turns another one off for a moment
        assert_eq!(
            b.shortcut_action(&c, &Shortcut::ctrl("O")),
            Some(Chosen::App(Cmd::Open))
        );
        c.disabled.push("open".into());
        assert_eq!(b.shortcut_action(&c, &Shortcut::ctrl("O")), None);
        assert!(find(&b.resolve(&c)[0].items, "open")
            .unwrap()
            .activate()
            .is_none());
    }

    #[test]
    fn the_program_sets_check_marks_by_id() {
        let mut c = ctx();
        c.checks.push(("grid".into(), false));
        let built = bar().resolve(&c);
        assert_eq!(find(&built[1].items, "grid").unwrap().checked, Some(false));
    }

    #[test]
    fn a_locked_layout_turns_off_the_panel_toggles_and_workspace_changes() {
        let mut c = ctx();
        c.locked = true;
        let built = bar().resolve(&c);
        assert!(built[2].items[0].activate().is_none());
        assert!(find(&built[3].items, "workspace.save")
            .unwrap()
            .activate()
            .is_none());
        assert!(find(&built[3].items, "workspace.delete")
            .unwrap()
            .activate()
            .is_none());
        assert!(
            find(&built[2].items, "layout.lock")
                .unwrap()
                .activate()
                .is_some(),
            "the lock itself can always be undone"
        );
        assert_eq!(
            find(&built[2].items, "layout.lock").unwrap().checked,
            Some(true)
        );
    }

    #[test]
    fn the_last_workspace_cannot_be_deleted() {
        let mut c = ctx();
        c.workspaces = vec!["Solo".into()];
        let built = bar().resolve(&c);
        assert!(find(&built[3].items, "workspace.delete")
            .unwrap()
            .activate()
            .is_none());
    }

    #[test]
    fn shortcuts_find_their_action_and_clashes_are_reported() {
        let b = bar();
        let c = ctx();
        assert_eq!(
            b.shortcut_action(&c, &Shortcut::plain("F5")),
            Some(Chosen::App(Cmd::Slice))
        );
        assert_eq!(
            b.shortcut_action(&c, &Shortcut::ctrl("PageDown")),
            Some(Chosen::Dock(DockAction::NextTab))
        );
        assert_eq!(b.shortcut_action(&c, &Shortcut::ctrl("Q")), None);
        assert!(b.conflicts(&c).is_empty(), "{:?}", b.conflicts(&c));
        let mut clash = bar();
        clash.menus[1] = Menu::new("tools", Label::Std(DockText::MenuTools))
            .entry(Entry::app("slice", SLICE, Cmd::Slice).key(Shortcut::ctrl("O")))
            .entry(Entry::app("grid", SLICE, Cmd::Grid).key(Shortcut::ctrl("M")));
        let conflicts = clash.conflicts(&c);
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts
            .iter()
            .any(|x| x.shortcut == Shortcut::ctrl("O") && x.names.len() == 2));
        assert!(
            conflicts.iter().any(|x| x.shortcut == Shortcut::ctrl("M")),
            "a frame shortcut counts too"
        );
    }

    #[test]
    fn separators_never_start_end_or_double() {
        let m = Menu::new("m", Label::Std(DockText::MenuFile))
            .sep()
            .entry(Entry::app("a", Label::Std(DockText::New), Cmd::Open))
            .sep()
            .sep()
            .generated(Gen::Panels)
            .sep();
        let built = MenuBar::new(vec![m]).resolve(&ctx());
        let kinds: Vec<bool> = built[0]
            .items
            .iter()
            .map(|r| matches!(r.kind, Kind::Separator))
            .collect();
        assert_eq!(kinds, [false, true, false, false]);
    }

    #[test]
    fn a_menu_with_nothing_in_it_is_left_out_and_the_declaration_is_checked() {
        let b = MenuBar::new(vec![
            Menu::<Cmd>::new("empty", Label::Std(DockText::MenuTools)),
            Menu::new("dup", Label::Std(DockText::MenuFile)).entry(Entry::app(
                "x",
                Label::Std(DockText::New),
                Cmd::Open,
            )),
            Menu::new("dup", Label::Std(DockText::MenuFile)).entry(Entry::app(
                "x",
                Label::Std(DockText::New),
                Cmd::Open,
            )),
            Menu::new("", Label::Std(DockText::MenuFile)).entry(Entry::app(
                "",
                Label::Std(DockText::New),
                Cmd::Open,
            )),
        ]);
        assert_eq!(b.resolve(&ctx()).len(), 3);
        let p = b.validate();
        assert!(p.contains(&MenuProblem::EmptyMenu("empty".into())));
        assert!(p.contains(&MenuProblem::DuplicateId("dup".into())));
        assert!(p.contains(&MenuProblem::DuplicateId("x".into())));
        assert!(p.contains(&MenuProblem::EmptyId));
        assert!(bar().validate().is_empty());
    }

    #[test]
    fn long_paths_are_shortened_in_the_middle() {
        let long = format!("/{}/file.eh3dproj", "d".repeat(100));
        let s = shorten(&long);
        assert_eq!(s.chars().count(), 58);
        assert!(s.contains('…') && s.ends_with("file.eh3dproj"));
        assert_eq!(shorten("/short"), "/short");
    }

    #[test]
    fn every_language_builds_every_menu() {
        for lang in Lang::ALL {
            let mut c = ctx();
            c.lang = lang;
            let built = bar().resolve(&c);
            assert!(built.iter().all(|m| !m.title.is_empty()
                && m.items
                    .iter()
                    .all(|i| matches!(i.kind, Kind::Separator) || !i.label.is_empty())));
        }
    }

    #[test]
    fn no_recent_files_gives_a_note() {
        let mut c = ctx();
        c.recent.clear();
        let built = bar().resolve(&c);
        let recent = find(&built[0].items, "recent").unwrap();
        match &recent.kind {
            Kind::Submenu(s) => {
                assert_eq!(s.len(), 1);
                assert_eq!(s[0].kind, Kind::Note);
                assert_eq!(s[0].label, "(ninguno)");
                assert!(s[0].activate().is_none());
            }
            _ => panic!(),
        }
    }
}
