//! The state of the frame without a window: the panels a program has, its workspaces, the shortcuts, the recent files and the warnings, and the
//! one function that does what a menu entry or a shortcut of the frame asks. The drawing code only reads this and calls `apply`.

use crate::config;
use crate::menu::{DockAction, MenuContext, PanelEntry};
use crate::model::{valid_id, DockError, Layout, PanelState};
use crate::recent::RecentFiles;
use crate::serial::ParseError;
use crate::shortcuts::{DockKey, DockKeys, Shortcut};
use crate::texts::Text8;
use crate::workspaces::{LoadWarning, WarnKind, Workspace, Workspaces};
use eh3d_view::texts::Lang;

/// A panel a program has.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelDef {
    /// The stable name (see [`valid_id`]).
    pub id: String,
    /// The title in eight languages.
    pub title: Text8,
    /// A short sign for the tab (one or two characters), or empty.
    pub icon: &'static str,
    /// The smallest width the panel needs, in points.
    pub min_w: f32,
    /// The smallest height the panel needs, in points.
    pub min_h: f32,
    /// A shortcut that shows or hides it, unless the settings say otherwise.
    pub shortcut: Option<Shortcut>,
}

impl PanelDef {
    /// A panel with a name and a title, a small minimum size and no icon or shortcut.
    pub fn new(id: &str, title: Text8) -> PanelDef {
        PanelDef {
            id: id.to_string(),
            title,
            icon: "",
            min_w: 120.0,
            min_h: 80.0,
            shortcut: None,
        }
    }

    /// The same panel with a minimum size.
    pub fn min(mut self, w: f32, h: f32) -> PanelDef {
        self.min_w = w;
        self.min_h = h;
        self
    }

    /// The same panel with an icon.
    pub fn icon(mut self, icon: &'static str) -> PanelDef {
        self.icon = icon;
        self
    }

    /// The same panel with a shortcut that shows or hides it.
    pub fn key(mut self, shortcut: Shortcut) -> PanelDef {
        self.shortcut = Some(shortcut);
        self
    }
}

/// What the window «About» shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AboutInfo {
    /// The name of the program.
    pub app_name: String,
    /// Its version.
    pub version: String,
    /// Its license, in a few words.
    pub license: String,
    /// More lines (the address of the project, the credits).
    pub extra: Vec<String>,
}

/// What the drawing code must do after an action, beyond the change already made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing more.
    Done,
    /// Ask for a name for the layout, then apply `SaveWorkspaceAs`.
    NeedsName,
    /// Show the window about the program.
    ShowAbout,
    /// Show the window with the shortcuts.
    ShowShortcuts,
}

/// The frame without a window.
#[derive(Debug, Clone)]
pub struct DockCore {
    defs: Vec<PanelDef>,
    workspaces: Workspaces,
    /// The language of the window.
    pub lang: Lang,
    keys: DockKeys,
    recent: RecentFiles,
    warnings: Vec<LoadWarning>,
    focus: Option<String>,
    /// What the window about the program shows.
    pub about: AboutInfo,
}

impl DockCore {
    /// The frame for these panels and these factory workspaces. A factory layout that names a panel the program does not have, or leaves one out,
    /// is repaired (an omitted panel is hidden), so a mistake in the factory data never reaches the person.
    pub fn new(defs: Vec<PanelDef>, factory: Vec<Workspace>) -> DockCore {
        let defs: Vec<PanelDef> = defs.into_iter().filter(|d| valid_id(&d.id)).collect();
        let known: Vec<String> = defs.iter().map(|d| d.id.clone()).collect();
        let known_refs: Vec<&str> = known.iter().map(String::as_str).collect();
        let factory: Vec<Workspace> = factory
            .into_iter()
            .map(|mut w| {
                w.layout.repair(&known_refs);
                w
            })
            .collect();
        let mut keys = DockKeys::defaults();
        for d in &defs {
            if let Some(s) = &d.shortcut {
                keys.set(DockKey::Panel(d.id.clone()), Some(s.clone()));
            }
        }
        DockCore {
            defs,
            workspaces: Workspaces::new(factory),
            lang: Lang::En,
            keys,
            recent: RecentFiles::default(),
            warnings: Vec::new(),
            focus: None,
            about: AboutInfo::default(),
        }
    }

    /// The panels.
    pub fn defs(&self) -> &[PanelDef] {
        &self.defs
    }

    /// The definition of a panel.
    pub fn def(&self, id: &str) -> Option<&PanelDef> {
        self.defs.iter().find(|d| d.id == id)
    }

    /// The title of a panel in the language of the window (its name when the panel is not known).
    pub fn title<'a>(&'a self, id: &'a str) -> &'a str {
        self.def(id).map(|d| d.title.get(self.lang)).unwrap_or(id)
    }

    fn known(&self) -> Vec<&str> {
        self.defs.iter().map(|d| d.id.as_str()).collect()
    }

    /// The layout of the workspace in use.
    pub fn layout(&self) -> &Layout {
        self.workspaces.layout()
    }

    /// The layout of the workspace in use, to change it.
    pub fn layout_mut(&mut self) -> &mut Layout {
        self.workspaces.layout_mut()
    }

    /// The workspaces.
    pub fn workspaces(&self) -> &Workspaces {
        &self.workspaces
    }

    /// The shortcuts of the frame.
    pub fn keys(&self) -> &DockKeys {
        &self.keys
    }

    /// The shortcuts of the frame, to change them.
    pub fn keys_mut(&mut self) -> &mut DockKeys {
        &mut self.keys
    }

    /// The recent files.
    pub fn recent(&self) -> &RecentFiles {
        &self.recent
    }

    /// The recent files, to change them.
    pub fn recent_mut(&mut self) -> &mut RecentFiles {
        &mut self.recent
    }

    /// The panel with the focus.
    pub fn focus(&self) -> Option<&str> {
        self.focus.as_deref()
    }

    /// Notes which panel has the focus (a name that is not a panel is ignored).
    pub fn set_focus(&mut self, id: &str) {
        if self.def(id).is_some() {
            self.focus = Some(id.to_string());
        }
    }

    /// The panel the keyboard commands act on: the one with the focus when it is shown, else the first docked one.
    pub fn target_panel(&self) -> Option<String> {
        if let Some(f) = &self.focus {
            if self.layout().is_shown(f) {
                return Some(f.clone());
            }
        }
        self.layout()
            .root
            .as_ref()
            .and_then(|r| r.first_panel())
            .map(str::to_string)
    }

    /// The warnings found while reading the saved layouts, to show once.
    pub fn take_warnings(&mut self) -> Vec<LoadWarning> {
        std::mem::take(&mut self.warnings)
    }

    /// The warnings not yet taken.
    pub fn warnings(&self) -> &[LoadWarning] {
        &self.warnings
    }

    /// What the frame does when a shortcut is pressed (the shortcut of a menu entry of the program is not here: the bar answers for those).
    pub fn resolve_key(&self, s: &Shortcut) -> Option<DockAction> {
        Some(match self.keys.resolve(s)? {
            DockKey::Maximize => DockAction::ToggleMaximize,
            DockKey::NextTab => DockAction::NextTab,
            DockKey::PrevTab => DockAction::PrevTab,
            DockKey::RestoreLayout => DockAction::RestoreLayout,
            DockKey::ToggleLock => DockAction::ToggleLock,
            DockKey::Move(side) => DockAction::MovePanel(self.target_panel()?, *side),
            DockKey::Panel(id) => DockAction::TogglePanel(id.clone()),
        })
    }

    fn require_panel(&self, id: &str) -> Result<(), DockError> {
        if self.def(id).is_some() {
            Ok(())
        } else {
            Err(DockError::UnknownPanel)
        }
    }

    /// Does what an action asks. The change is made here; the outcome says what the window must show next.
    pub fn apply(&mut self, action: &DockAction) -> Result<Outcome, DockError> {
        match action {
            DockAction::TogglePanel(id) => {
                self.require_panel(id)?;
                if self.layout_mut().toggle(id)? {
                    self.focus = Some(id.clone());
                }
            }
            DockAction::ShowPanel(id) => {
                self.require_panel(id)?;
                self.layout_mut().show(id)?;
                self.focus = Some(id.clone());
            }
            DockAction::HidePanel(id) => {
                self.require_panel(id)?;
                self.layout_mut().hide(id)?;
            }
            DockAction::FloatPanel(id) => {
                self.require_panel(id)?;
                if self.layout().state_of(id) != Some(PanelState::Floating) {
                    self.layout_mut().float(id, None)?;
                }
            }
            DockAction::DockPanel(id) => {
                self.require_panel(id)?;
                self.layout_mut().dock(id)?;
            }
            DockAction::AutoHidePanel(id, side) => {
                self.require_panel(id)?;
                self.layout_mut().auto_hide(id, *side)?;
            }
            DockAction::MovePanel(id, side) => {
                self.require_panel(id)?;
                self.layout_mut().move_toward(id, *side)?;
            }
            DockAction::ToggleMaximize => {
                let id = self.target_panel().ok_or(DockError::UnknownPanel)?;
                self.layout_mut().toggle_maximize(&id)?;
            }
            DockAction::NextTab | DockAction::PrevTab => {
                let id = self.target_panel().ok_or(DockError::UnknownPanel)?;
                let delta = if matches!(action, DockAction::NextTab) {
                    1
                } else {
                    -1
                };
                let now = self.layout_mut().cycle(&id, delta)?;
                self.focus = Some(now);
            }
            DockAction::RestoreLayout => self.workspaces.restore_current(),
            DockAction::ToggleLock => {
                let now = !self.layout().locked;
                self.layout_mut().set_locked(now);
            }
            DockAction::SwitchWorkspace(name) => self.workspaces.switch(name)?,
            DockAction::AskSaveWorkspace => return Ok(Outcome::NeedsName),
            DockAction::SaveWorkspaceAs(name) => {
                if self.layout().locked {
                    return Err(DockError::Locked);
                }
                self.workspaces.save_as(name)?;
            }
            DockAction::DeleteWorkspace => {
                if self.layout().locked {
                    return Err(DockError::Locked);
                }
                self.workspaces.delete_current()?;
            }
            DockAction::ShowAbout => return Ok(Outcome::ShowAbout),
            DockAction::ShowShortcuts => return Ok(Outcome::ShowShortcuts),
            DockAction::ClearRecent => self.recent.clear(),
        }
        Ok(Outcome::Done)
    }

    /// What the menus need to know.
    pub fn menu_context(&self) -> MenuContext {
        let layout = self.layout();
        let mut ctx = MenuContext::empty(self.lang);
        ctx.panels = self
            .defs
            .iter()
            .map(|d| PanelEntry {
                id: d.id.clone(),
                title: d.title.get(self.lang).to_string(),
                shown: layout.is_shown(&d.id),
            })
            .collect();
        ctx.workspaces = self
            .workspaces
            .names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        ctx.current_workspace = self.workspaces.current_name().to_string();
        ctx.locked = layout.locked;
        ctx.maximized = layout.maximized.is_some();
        ctx.recent = self.recent.items().to_vec();
        ctx.keys = self.keys.clone();
        ctx
    }

    // ---- saving and reading ------------------------------------------------------------------------------------------------------------

    /// Everything the frame keeps in the settings file, as `key=value` pairs: the workspaces, the shortcuts that differ from the defaults and the
    /// recent files.
    pub fn config_pairs(&self) -> Vec<(String, String)> {
        let mut out = self.workspaces.to_pairs();
        // a panel's own shortcut is not a choice of the person: it is written only when the person changed it
        out.extend(self.keys.to_lines().into_iter().filter(|(k, v)| {
            let own = k
                .strip_prefix("dock.key.panel.")
                .and_then(|id| self.def(id))
                .and_then(|d| d.shortcut.as_ref());
            own.map(|s| s.to_string()) != Some(v.clone())
        }));
        out.extend(self.recent.to_pairs());
        out
    }

    /// The settings text with the lines of the frame brought up to date; every other line is as it was.
    pub fn config_text(&self, existing: &str) -> String {
        config::replace_block(existing, &self.config_pairs())
    }

    /// Reads the lines of the frame out of a settings text. What cannot be read is replaced by the factory layout and noted as a warning
    /// (see [`DockCore::take_warnings`]); nothing here panics.
    pub fn load_config(&mut self, text: &str) {
        let pairs = config::dock_pairs(text);
        let factory = self.workspaces.factory().to_vec();
        let known: Vec<String> = self.defs.iter().map(|d| d.id.clone()).collect();
        let known_refs: Vec<&str> = known.iter().map(String::as_str).collect();
        let (w, mut warnings) = Workspaces::from_pairs(&pairs, factory, &known_refs);
        self.workspaces = w;
        let (mut keys, bad) = DockKeys::from_pairs(&pairs);
        for d in &self.defs {
            if let Some(s) = &d.shortcut {
                if !pairs
                    .iter()
                    .any(|(k, _)| *k == format!("dock.key.panel.{}", d.id))
                {
                    keys.set(DockKey::Panel(d.id.clone()), Some(s.clone()));
                }
            }
        }
        self.keys = keys;
        if bad > 0 {
            warnings.push(LoadWarning {
                kind: WarnKind::BadShortcut,
                detail: bad.to_string(),
            });
        }
        self.recent = RecentFiles::from_pairs(&pairs);
        self.warnings.extend(warnings);
    }

    /// The layout in use as a text (see [`Layout::to_text`]).
    pub fn layout_text(&self) -> String {
        self.layout().to_text()
    }

    /// Replaces the layout in use with a text. A text that is damaged or of another version gives the factory layout of the workspace instead,
    /// with a warning; a text with problems that can be put right is repaired, with a warning. The warnings are also kept for
    /// [`DockCore::take_warnings`].
    pub fn set_layout_text(&mut self, text: &str) -> Vec<LoadWarning> {
        let name = self.workspaces.current_name().to_string();
        let mut out = Vec::new();
        match Layout::from_text(text) {
            Ok(mut l) => {
                let known: Vec<String> = self.defs.iter().map(|d| d.id.clone()).collect();
                let refs: Vec<&str> = known.iter().map(String::as_str).collect();
                if !l.repair(&refs).is_empty() {
                    out.push(LoadWarning {
                        kind: WarnKind::Repaired,
                        detail: name,
                    });
                }
                *self.layout_mut() = l;
            }
            Err(e) => {
                self.workspaces.restore_current();
                out.push(LoadWarning {
                    kind: if matches!(e, ParseError::OtherVersion(_)) {
                        WarnKind::OtherVersion
                    } else {
                        WarnKind::Corrupt
                    },
                    detail: name,
                });
            }
        }
        self.warnings.extend(out.clone());
        out
    }

    /// Whether every panel of the layout in use is a panel the program has and each is in one place.
    pub fn layout_is_sound(&self) -> bool {
        self.layout().validate(&self.known()).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Node;

    fn defs() -> Vec<PanelDef> {
        let t = |s: &'static str| Text8::same(s);
        vec![
            PanelDef::new("files", t("Files")).min(150.0, 100.0),
            PanelDef::new("layers", t("Layers")).key(Shortcut::ctrl_shift("L")),
            PanelDef::new("view", t("View")),
            PanelDef::new("console", t("Console")),
        ]
    }

    fn factory() -> Vec<Workspace> {
        vec![
            Workspace::new(
                "Prepare",
                Layout::with_root(Node::row(
                    0.25,
                    Node::tabs(&["files", "layers"]),
                    Node::column(0.8, Node::tabs(&["view"]), Node::tabs(&["console"])),
                )),
            ),
            Workspace::new(
                "Compact",
                Layout::with_root(Node::tabs(&["view"]))
                    .with_hidden(&["files", "layers", "console"]),
            ),
        ]
    }

    fn core() -> DockCore {
        DockCore::new(defs(), factory())
    }

    #[test]
    fn a_mistake_in_the_factory_data_is_repaired() {
        let bad = vec![Workspace::new(
            "X",
            Layout::with_root(Node::tabs(&["files", "ghost", "files"])),
        )];
        let c = DockCore::new(defs(), bad);
        assert!(c.layout_is_sound());
        assert!(!c.layout().panels().contains(&"ghost".to_string()));
        assert_eq!(c.layout().panels().len(), 4);
    }

    #[test]
    fn actions_change_the_layout() {
        let mut c = core();
        assert_eq!(
            c.apply(&DockAction::TogglePanel("console".into())),
            Ok(Outcome::Done)
        );
        assert!(!c.layout().is_shown("console"));
        c.apply(&DockAction::TogglePanel("console".into())).unwrap();
        assert!(c.layout().is_shown("console"));
        assert_eq!(c.focus(), Some("console"));
        c.apply(&DockAction::FloatPanel("layers".into())).unwrap();
        assert_eq!(c.layout().state_of("layers"), Some(PanelState::Floating));
        c.apply(&DockAction::DockPanel("layers".into())).unwrap();
        assert_eq!(c.layout().state_of("layers"), Some(PanelState::Docked));
        assert_eq!(
            c.apply(&DockAction::HidePanel("zzz".into())),
            Err(DockError::UnknownPanel)
        );
        assert!(c.layout_is_sound());
    }

    #[test]
    fn maximise_and_tabs_act_on_the_focused_panel() {
        let mut c = core();
        c.set_focus("layers");
        c.apply(&DockAction::NextTab).unwrap();
        assert_eq!(c.focus(), Some("files"));
        c.apply(&DockAction::ToggleMaximize).unwrap();
        assert_eq!(c.layout().maximized.as_deref(), Some("files"));
        c.apply(&DockAction::ToggleMaximize).unwrap();
        assert_eq!(c.layout().maximized, None);
        c.set_focus("nope");
        assert_eq!(c.focus(), Some("files"));
    }

    #[test]
    fn the_lock_blocks_changes_but_workspaces_can_still_be_switched() {
        let mut c = core();
        c.apply(&DockAction::ToggleLock).unwrap();
        assert_eq!(
            c.apply(&DockAction::HidePanel("view".into())),
            Err(DockError::Locked)
        );
        assert_eq!(
            c.apply(&DockAction::SaveWorkspaceAs("N".into())),
            Err(DockError::Locked)
        );
        assert_eq!(
            c.apply(&DockAction::DeleteWorkspace),
            Err(DockError::Locked)
        );
        c.apply(&DockAction::SwitchWorkspace("Compact".into()))
            .unwrap();
        assert!(!c.layout().locked, "each workspace has its own lock");
        c.apply(&DockAction::RestoreLayout).unwrap();
    }

    #[test]
    fn workspaces_are_saved_switched_and_deleted() {
        let mut c = core();
        c.apply(&DockAction::HidePanel("console".into())).unwrap();
        assert_eq!(
            c.apply(&DockAction::AskSaveWorkspace),
            Ok(Outcome::NeedsName)
        );
        c.apply(&DockAction::SaveWorkspaceAs("Mine".into()))
            .unwrap();
        assert_eq!(c.workspaces().current_name(), "Mine");
        c.apply(&DockAction::SwitchWorkspace("Prepare".into()))
            .unwrap();
        c.apply(&DockAction::RestoreLayout).unwrap();
        assert!(c.layout().is_shown("console"));
        c.apply(&DockAction::SwitchWorkspace("Mine".into()))
            .unwrap();
        assert!(!c.layout().is_shown("console"));
        c.apply(&DockAction::DeleteWorkspace).unwrap();
        assert_eq!(c.workspaces().names(), ["Prepare", "Compact"]);
        assert_eq!(
            c.apply(&DockAction::SwitchWorkspace("Mine".into())),
            Err(DockError::BadName)
        );
        assert_eq!(c.apply(&DockAction::ShowAbout), Ok(Outcome::ShowAbout));
        assert_eq!(
            c.apply(&DockAction::ShowShortcuts),
            Ok(Outcome::ShowShortcuts)
        );
    }

    #[test]
    fn restore_gives_back_the_factory_layout() {
        let mut c = core();
        c.apply(&DockAction::HidePanel("view".into())).unwrap();
        c.apply(&DockAction::RestoreLayout).unwrap();
        assert_eq!(
            c.layout(),
            c.workspaces().factory_layout("Prepare").unwrap()
        );
    }

    #[test]
    fn the_settings_file_round_trips_everything_and_keeps_other_lines() {
        let mut c = core();
        c.apply(&DockAction::HidePanel("console".into())).unwrap();
        c.apply(&DockAction::SaveWorkspaceAs("Mine".into()))
            .unwrap();
        c.keys_mut()
            .set(DockKey::Maximize, Some(Shortcut::plain("F11")));
        c.recent_mut().push("/jobs/a.eh3dproj");
        let text = c.config_text("language=es\ntheme=dark\n");
        assert!(text.starts_with("language=es\ntheme=dark\n"));
        let mut d = core();
        d.load_config(&text);
        assert!(d.warnings().is_empty(), "{:?}", d.warnings());
        assert_eq!(d.workspaces(), c.workspaces());
        assert_eq!(d.keys(), c.keys());
        assert_eq!(d.recent(), c.recent());
        assert_eq!(
            d.config_text(&text),
            text,
            "writing what was read changes nothing"
        );
    }

    #[test]
    fn a_settings_file_with_no_lines_of_the_frame_is_a_first_run() {
        let mut c = core();
        c.load_config("language=es\n");
        assert!(c.warnings().is_empty());
        assert_eq!(c.workspaces().current_name(), "Prepare");
    }

    #[test]
    fn a_damaged_or_foreign_settings_file_gives_the_factory_and_a_warning() {
        let mut c = core();
        c.load_config("dock.version=7\ndock.ws.0.name=Prepare\n");
        let w = c.take_warnings();
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].name(), "layout_other_version");
        assert!(c.warnings().is_empty());
        c.load_config("dock.version=1\ndock.ws.0.name=Prepare\ndock.ws.0.tree=T0((\ndock.key.maximize=Hyper+Q\n");
        let names: Vec<&str> = c.warnings().iter().map(|w| w.name()).collect();
        assert!(
            names.contains(&"layout_corrupt") && names.contains(&"shortcut_unreadable"),
            "{names:?}"
        );
        assert_eq!(
            c.layout(),
            c.workspaces().factory_layout("Prepare").unwrap()
        );
        assert!(c.layout_is_sound());
    }

    #[test]
    fn the_layout_text_api() {
        let mut c = core();
        c.apply(&DockAction::HidePanel("console".into())).unwrap();
        let text = c.layout_text();
        let mut d = core();
        assert!(d.set_layout_text(&text).is_empty());
        assert_eq!(d.layout(), c.layout());
        let w = d.set_layout_text("rubbish");
        assert_eq!(w[0].name(), "layout_corrupt");
        assert_eq!(
            d.layout(),
            d.workspaces().factory_layout("Prepare").unwrap()
        );
        let w = d.set_layout_text("eh3d-dock-layout 99\n");
        assert_eq!(w[0].name(), "layout_other_version");
        let w = d.set_layout_text("eh3d-dock-layout 1\ntree=T0(files,ghost)\nhidden=layers\n");
        assert_eq!(w[0].name(), "layout_repaired");
        assert!(d.layout_is_sound());
    }

    #[test]
    fn keys_resolve_to_actions_and_a_panel_can_bring_its_own() {
        let c = core();
        assert_eq!(
            c.resolve_key(&Shortcut::ctrl("M")),
            Some(DockAction::ToggleMaximize)
        );
        assert_eq!(
            c.resolve_key(&Shortcut::ctrl_shift("L")),
            Some(DockAction::TogglePanel("layers".into()))
        );
        assert_eq!(c.resolve_key(&Shortcut::ctrl("Q")), None);
        let ctx = c.menu_context();
        assert_eq!(ctx.panels.len(), 4);
        assert!(ctx.panels.iter().all(|p| p.shown));
        assert_eq!(ctx.current_workspace, "Prepare");
        // a person's setting beats the panel's own shortcut
        let mut d = core();
        d.load_config("dock.key.panel.layers=Ctrl+Alt+L\n");
        assert_eq!(d.resolve_key(&Shortcut::ctrl_shift("L")), None);
        assert_eq!(
            d.keys()
                .get(&DockKey::Panel("layers".into()))
                .unwrap()
                .to_string(),
            "Ctrl+Alt+L"
        );
    }

    #[test]
    fn titles_follow_the_language() {
        let mut c = core();
        assert_eq!(c.title("files"), "Files");
        assert_eq!(c.title("zzz"), "zzz");
        c.lang = Lang::Es;
        assert_eq!(c.menu_context().lang, Lang::Es);
    }
}
