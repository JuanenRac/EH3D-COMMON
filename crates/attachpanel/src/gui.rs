//! The attachment panel drawn with egui. A program keeps one [`AttachPanel`] for as long as it runs and calls
//! [`AttachPanel::show`] inside its dock panel; the panel has its own window to choose files and folders, so there is nothing else
//! to wire.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{RichText, ScrollArea, Ui};
use eh3d_filedialog::{size_text, Action, Browser, Chosen, Filter, Mode, PlaceKind};
use eh3d_project::attachments;
use eh3d_viewer::gui::ViewerPanel;
use eh3d_viewer::{Viewer, ViewerEvent, ViewerLimits};

use crate::model::{component_link, link_label, Item, LinkForm, Message, Model, Program, Request};
use crate::source::ProjectSource;
use crate::texts::{fill, lang_of, tr, Lang, Text};

/// How often the file of the project is looked at, for what another program wrote there.
const CHECK_EVERY: Duration = Duration::from_secs(1);
/// From this width the viewer is beside the list; narrower, it is under it.
const WIDE: f32 = 760.0;

/// What the window that chooses is open for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Purpose {
    Attach,
    OpenProject,
    HashOf,
    ExtractTo(String),
    SaveCopy,
}

struct Chooser {
    browser: Browser,
    purpose: Purpose,
}

#[derive(Default)]
struct LinkDraft {
    url: String,
    sha256: String,
    size: Option<u64>,
}

#[derive(Default)]
struct ComponentDraft {
    form: LinkForm,
    text: String,
    maker: String,
}

/// What was asked while the panel was drawn; it is done when the drawing is over.
enum Asked {
    Do(Request),
    Select(String),
    Choose(Purpose),
    Link(bool),
    ShowAll,
    ReadAgain,
    Close,
    Confirm,
    Cancel,
}

/// The panel «Attachments».
pub struct AttachPanel {
    /// What the panel holds and does.
    pub model: Model,
    viewer: ViewerPanel,
    viewer_limits: ViewerLimits,
    /// The project and the attachment the viewer shows.
    shown: Option<(PathBuf, String)>,
    chooser: Option<Chooser>,
    link: Option<LinkDraft>,
    component: ComponentDraft,
    checked: Option<Instant>,
    /// The folder the last file was chosen in.
    folder: Option<PathBuf>,
    /// Whether the panel opens projects itself, for a program with no project menu.
    opens: bool,
    /// The project the panel opened itself.
    own: Option<PathBuf>,
}

fn kind_of_form(form: LinkForm) -> Text {
    match form {
        LinkForm::Designator => Text::Designator,
        LinkForm::Value => Text::Value,
        LinkForm::PartNumber => Text::PartNumber,
    }
}

fn row_text(item: &Item, lang: Lang) -> String {
    let size = item.size.map_or_else(|| "?".to_owned(), size_text);
    let place = tr(
        lang,
        if item.is_link {
            Text::Link
        } else {
            Text::Inside
        },
    );
    let mark = if item.needs_confirmation { "⚠ " } else { "" };
    format!("{mark}{}  ({size}, {place})", item.name)
}

impl AttachPanel {
    /// A panel of a program.
    pub fn new(program: Program) -> AttachPanel {
        let viewer_limits = ViewerLimits::default();
        AttachPanel {
            model: Model::new(program),
            viewer: ViewerPanel::new(viewer_limits.clone()),
            viewer_limits,
            shown: None,
            chooser: None,
            link: None,
            component: ComponentDraft::default(),
            checked: None,
            folder: None,
            opens: false,
            own: None,
        }
    }

    /// The same panel for a program whose window has no project of the suite: the panel has its own button to open one. A
    /// project the program gives to [`AttachPanel::show`] still comes first.
    #[must_use]
    pub fn opens_projects(mut self) -> AttachPanel {
        self.opens = true;
        self
    }

    /// Asks for a project to open (an entry of a menu): the window to choose it is drawn with the panel.
    pub fn choose_project(&mut self) {
        self.open_chooser(Purpose::OpenProject);
    }

    /// Opens a project in a panel that opens them itself.
    pub fn open_project(&mut self, path: PathBuf) {
        self.own = Some(path);
        self.link = None;
    }

    /// The same panel with another viewer (a test gives one that starts no program).
    #[must_use]
    pub fn with_viewer(mut self, viewer: Viewer) -> AttachPanel {
        self.viewer = ViewerPanel::with(viewer);
        self.shown = None;
        self
    }

    /// Takes one line of a settings file (`attachments.max_mb`, `viewer.max_pdf_mb`...). Call it before the first frame; false
    /// for a key that is not of the panel.
    pub fn apply_setting(&mut self, key: &str, value: &str) -> bool {
        if self.model.apply_setting(key, value) {
            return true;
        }
        if !self.viewer_limits.apply(key, value) {
            return false;
        }
        self.viewer = ViewerPanel::new(self.viewer_limits.clone());
        self.shown = None;
        true
    }

    /// Draws the panel for the project of the program (nothing when it has none) in the language of a code such as `es`. True
    /// once after the project was written: the program reads its own state of the project again.
    pub fn show(&mut self, ui: &mut Ui, project: Option<&Path>, code: &str) -> bool {
        let lang = lang_of(code);
        let own = self.own.clone();
        let project = project.or(own.as_deref());
        if self.model.project() != project {
            // the window that chooses the next project stays
            if self
                .chooser
                .as_ref()
                .is_some_and(|c| c.purpose != Purpose::OpenProject)
            {
                self.chooser = None;
            }
            self.link = None;
        }
        self.model.set_project(project);
        self.model.poll();
        if self.checked.is_none_or(|at| at.elapsed() >= CHECK_EVERY) {
            self.checked = Some(Instant::now());
            self.model.check_file();
        }
        self.follow_selection();
        let mut asked = Vec::new();
        if self.opens {
            self.project_line(ui, project, lang, &mut asked);
        }
        if project.is_none() {
            ui.label(tr(lang, Text::NoProject));
            for what in asked {
                self.act(what);
            }
            self.chooser_window(ui, lang);
            return self.model.take_wrote();
        }
        self.head(ui, lang, &mut asked);
        let mut event = None;
        if ui.available_width() >= WIDE {
            ui.columns(2, |columns| {
                ScrollArea::vertical()
                    .id_salt("eh3d_attach_list")
                    .auto_shrink([false, false])
                    .show(&mut columns[0], |ui| self.list(ui, lang, &mut asked));
                event = self.viewer.show(&mut columns[1], lang);
            });
        } else {
            let height = (ui.available_height() * 0.45).max(140.0);
            ScrollArea::vertical()
                .id_salt("eh3d_attach_list")
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| self.list(ui, lang, &mut asked));
            ui.separator();
            event = self.viewer.show(ui, lang);
        }
        if let Some(ViewerEvent::ChooseSavePath { suggested }) = event {
            self.open_chooser(Purpose::SaveCopy);
            if let Some(chooser) = &mut self.chooser {
                chooser.browser.name_text = suggested;
            }
        }
        for what in asked {
            self.act(what);
        }
        self.chooser_window(ui, lang);
        if self.model.busy() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        self.model.take_wrote()
    }

    /// The project of a panel that opens them itself, with the buttons to open and close it.
    fn project_line(
        &mut self,
        ui: &mut Ui,
        project: Option<&Path>,
        lang: Lang,
        asked: &mut Vec<Asked>,
    ) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr(lang, Text::OpenProject)).clicked() {
                asked.push(Asked::Choose(Purpose::OpenProject));
            }
            if let Some(file) = project {
                let name = eh3d_project::suiteview::name_of(file);
                ui.label(fill(tr(lang, Text::ProjectName), &[&name]));
                if self.own.is_some() && ui.button(tr(lang, Text::CloseProject)).clicked() {
                    asked.push(Asked::Close);
                }
            }
        });
    }

    /// The viewer shows the attachment that is chosen, and nothing for a link or for none.
    fn follow_selection(&mut self) {
        let wanted = match (self.model.project(), self.model.selected()) {
            (Some(file), Some(item)) if !item.is_link => {
                Some((file.to_path_buf(), item.id.clone()))
            }
            _ => None,
        };
        if wanted == self.shown {
            return;
        }
        self.viewer.viewer.close();
        if let Some((_, id)) = &wanted {
            if let Some(Ok(source)) = self.model.source(id) {
                self.viewer.viewer.open(Arc::new(ProjectSource(source)));
            }
        }
        self.shown = wanted;
    }

    fn head(&mut self, ui: &mut Ui, lang: Lang, asked: &mut Vec<Asked>) {
        let free = !self.model.busy();
        ui.horizontal_wrapped(|ui| {
            let file = egui::Button::new(tr(lang, Text::AttachFile));
            if ui.add_enabled(free, file).clicked() {
                asked.push(Asked::Choose(Purpose::Attach));
            }
            let link = egui::Button::new(tr(lang, Text::AttachLink));
            if ui.add_enabled(free, link).clicked() {
                asked.push(Asked::Link(true));
            }
            let again = egui::Button::new(tr(lang, Text::ReadAgain));
            if ui.add_enabled(free, again).clicked() {
                asked.push(Asked::ReadAgain);
            }
            if !free {
                ui.spinner();
                ui.label(tr(lang, Text::Working));
            }
        });
        if let Some(message) = self.model.message() {
            say(ui, message, lang);
        }
        if let Some(error) = self.model.error() {
            let color = ui.visuals().warn_fg_color;
            ui.colored_label(color, fill(tr(lang, Text::Failed), &[error]));
        }
        if let Some(question) = self.model.pending() {
            ui.group(|ui| {
                for line in &question.lines {
                    let color = ui.visuals().warn_fg_color;
                    ui.colored_label(color, line.text(lang));
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr(lang, Text::Continue)).clicked() {
                        asked.push(Asked::Confirm);
                    }
                    if ui.button(tr(lang, Text::Cancel)).clicked() {
                        asked.push(Asked::Cancel);
                    }
                });
            });
        }
        if let Some(draft) = &mut self.link {
            ui.group(|ui| {
                ui.label(tr(lang, Text::Address));
                ui.add(egui::TextEdit::singleline(&mut draft.url).desired_width(f32::INFINITY));
                ui.label(tr(lang, Text::Hash));
                let hash = ui.add(
                    egui::TextEdit::singleline(&mut draft.sha256).desired_width(f32::INFINITY),
                );
                if hash.changed() {
                    // the size was the one of the file the hash was taken from
                    draft.size = None;
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr(lang, Text::HashFromFile)).clicked() {
                        asked.push(Asked::Choose(Purpose::HashOf));
                    }
                    let ready = free && !draft.url.trim().is_empty() && !draft.sha256.is_empty();
                    let add = egui::Button::new(tr(lang, Text::Add));
                    if ui.add_enabled(ready, add).clicked() {
                        asked.push(Asked::Do(Request::AttachLink {
                            url: draft.url.trim().to_owned(),
                            sha256: draft.sha256.trim().to_owned(),
                            size: draft.size,
                        }));
                        asked.push(Asked::Link(false));
                    }
                    if ui.button(tr(lang, Text::Cancel)).clicked() {
                        asked.push(Asked::Link(false));
                    }
                });
            });
        }
        if let Some(piece) = self.model.filter() {
            ui.horizontal_wrapped(|ui| {
                ui.strong(fill(tr(lang, Text::FilterOf), &[&piece.label()]));
                if ui.button(tr(lang, Text::ShowAll)).clicked() {
                    asked.push(Asked::ShowAll);
                }
            });
        }
        ui.separator();
    }

    fn list(&mut self, ui: &mut Ui, lang: Lang, asked: &mut Vec<Asked>) {
        let chosen = self.model.selected().cloned();
        let items = self.model.visible();
        if items.is_empty() {
            ui.label(tr(lang, Text::NoAttachments));
        }
        for item in items {
            let selected = chosen.as_ref().is_some_and(|c| c.id == item.id);
            if ui
                .selectable_label(selected, row_text(item, lang))
                .clicked()
            {
                asked.push(Asked::Select(item.id.clone()));
            }
        }
        if let Some(item) = chosen {
            ui.separator();
            self.details(ui, &item, lang, asked);
        }
    }

    fn details(&mut self, ui: &mut Ui, item: &Item, lang: Lang, asked: &mut Vec<Asked>) {
        let free = !self.model.busy();
        ui.strong(&item.name);
        if item.needs_confirmation {
            let word = attachments::doubt_word(item.danger).tr(lang.code());
            let color = ui.visuals().warn_fg_color;
            ui.colored_label(color, format!("⚠ {word}: {}", tr(lang, Text::NeedsCare)));
        }
        if item.is_link {
            ui.label(tr(lang, Text::LinkNote));
        }
        if let Some(url) = &item.url {
            ui.label(url);
            if ui.button(tr(lang, Text::CopyAddress)).clicked() {
                ui.ctx().copy_text(url.clone());
            }
        }
        ui.horizontal_wrapped(|ui| {
            let extract = egui::Button::new(tr(lang, Text::Extract));
            if ui.add_enabled(free && !item.is_link, extract).clicked() {
                asked.push(Asked::Choose(Purpose::ExtractTo(item.id.clone())));
            }
            let remove = egui::Button::new(tr(lang, Text::Remove));
            if ui.add_enabled(free, remove).clicked() {
                asked.push(Asked::Do(Request::Remove(item.id.clone())));
            }
        });
        ui.label(tr(lang, Text::Components));
        for link in &item.components {
            ui.horizontal_wrapped(|ui| {
                ui.label(link_label(link));
                let unlink = egui::Button::new(tr(lang, Text::Unlink)).small();
                if ui.add_enabled(free, unlink).clicked() {
                    let request = Request::RemoveComponent(item.id.clone(), link.clone());
                    asked.push(Asked::Do(request));
                }
            });
        }
        let draft = &mut self.component;
        egui::ComboBox::from_id_salt("eh3d_attach_form")
            .selected_text(tr(lang, kind_of_form(draft.form)))
            .show_ui(ui, |ui| {
                for form in [LinkForm::Designator, LinkForm::Value, LinkForm::PartNumber] {
                    ui.selectable_value(&mut draft.form, form, tr(lang, kind_of_form(form)));
                }
            });
        ui.add(egui::TextEdit::singleline(&mut draft.text).desired_width(180.0));
        if draft.form == LinkForm::PartNumber {
            ui.label(tr(lang, Text::Maker));
            ui.add(egui::TextEdit::singleline(&mut draft.maker).desired_width(180.0));
        }
        let link = component_link(draft.form, &draft.text, &draft.maker);
        let button = egui::Button::new(tr(lang, Text::LinkComponent));
        if ui.add_enabled(free && link.is_some(), button).clicked() {
            if let Some(link) = link {
                asked.push(Asked::Do(Request::AddComponent(item.id.clone(), link)));
                draft.text.clear();
                draft.maker.clear();
            }
        }
    }

    fn act(&mut self, what: Asked) {
        match what {
            Asked::Do(request) => self.model.request(request),
            Asked::Select(id) => self.model.select(Some(&id)),
            Asked::Choose(purpose) => self.open_chooser(purpose),
            Asked::Link(open) => self.link = open.then(LinkDraft::default),
            Asked::ShowAll => self.model.set_filter(None),
            Asked::ReadAgain => self.model.reload(),
            Asked::Close => self.own = None,
            Asked::Confirm => self.model.confirm(),
            Asked::Cancel => self.model.cancel(),
        }
    }

    fn open_chooser(&mut self, purpose: Purpose) {
        let home = self
            .model
            .project()
            .and_then(Path::parent)
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let start = self
            .folder
            .clone()
            .filter(|f| f.is_dir())
            .unwrap_or_else(|| home.clone());
        let mode = if purpose == Purpose::SaveCopy {
            Mode::Save
        } else {
            Mode::Open
        };
        let filter = if purpose == Purpose::OpenProject {
            Filter::new(&[eh3d_project::appbranch::EXTENSION])
        } else {
            Filter::any()
        };
        let browser = Browser::new(mode, &start, &home, vec![filter], None);
        self.chooser = Some(Chooser { browser, purpose });
    }

    /// What was chosen in the window.
    fn chosen(&mut self, purpose: &Purpose, path: PathBuf) {
        self.folder = match purpose {
            Purpose::ExtractTo(_) => Some(path.clone()),
            _ => path.parent().map(Path::to_path_buf),
        };
        match purpose {
            Purpose::Attach => self.model.request(Request::AttachFile(path)),
            Purpose::OpenProject => self.open_project(path),
            Purpose::HashOf => {
                if let (Some(draft), Ok((sha256, size))) =
                    (&mut self.link, attachments::hash_file(&path))
                {
                    draft.sha256 = sha256;
                    draft.size = Some(size);
                }
            }
            Purpose::ExtractTo(id) => self.model.request(Request::Extract(id.clone(), path)),
            Purpose::SaveCopy => {
                // the viewer says in its own panel how it ended
                self.viewer.viewer.save_to(&path);
            }
        }
    }

    fn chooser_window(&mut self, ui: &mut Ui, lang: Lang) {
        let Some(mut chooser) = self.chooser.take() else {
            return;
        };
        let folder = matches!(chooser.purpose, Purpose::ExtractTo(_));
        let title = match chooser.purpose {
            Purpose::ExtractTo(_) => Text::ChooseFolder,
            Purpose::SaveCopy => Text::SaveAs,
            _ => Text::ChooseFile,
        };
        let accept = match chooser.purpose {
            Purpose::ExtractTo(_) => Text::ExtractHere,
            Purpose::SaveCopy => Text::Save,
            _ => Text::Open,
        };
        let mut open = true;
        let mut cancelled = false;
        let mut picked = None;
        egui::Window::new(tr(lang, title))
            .id(egui::Id::unique("eh3d_attach_chooser"))
            .open(&mut open)
            .collapsible(false)
            .default_size([560.0, 420.0])
            .show(ui.ctx(), |ui| {
                let browser = &mut chooser.browser;
                let places = browser.places.clone();
                ui.horizontal_wrapped(|ui| {
                    for (i, place) in places.iter().enumerate() {
                        let name = match place.kind {
                            PlaceKind::Examples => tr(lang, Text::ProjectFolder).to_owned(),
                            PlaceKind::Desktop => tr(lang, Text::Desktop).to_owned(),
                            PlaceKind::Documents => tr(lang, Text::Documents).to_owned(),
                            PlaceKind::Downloads => tr(lang, Text::Downloads).to_owned(),
                            PlaceKind::Drive => place.name.clone(),
                        };
                        if ui.button(name).clicked() {
                            browser.go_place(i);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button(tr(lang, Text::Up)).clicked() {
                        browser.up();
                    }
                    let edit = egui::TextEdit::singleline(&mut browser.path_text)
                        .desired_width(ui.available_width());
                    let typed = ui.add(edit);
                    if typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        browser.go_typed_path();
                    }
                });
                let mut clicked = None;
                let mut activated = None;
                ScrollArea::vertical()
                    .id_salt("eh3d_attach_folder")
                    .max_height(260.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, entry) in browser.entries.iter().enumerate() {
                            let text = if entry.is_dir {
                                RichText::new(format!("▸ {}", entry.name)).strong()
                            } else {
                                RichText::new(format!("{}   {}", entry.name, size_text(entry.size)))
                            };
                            let line = ui.selectable_label(browser.sel == Some(i), text);
                            if line.double_clicked() {
                                activated = Some(i);
                            } else if line.clicked() {
                                clicked = Some(i);
                            }
                        }
                    });
                if let Some(i) = clicked {
                    browser.select(i);
                }
                if let Some(i) = activated {
                    if let Action::Chosen(path) = browser.activate(i) {
                        // a double click on a file is the button of the window, except where a folder is asked for
                        if !folder {
                            picked = Some(path);
                        }
                    }
                }
                if !folder {
                    ui.add(
                        egui::TextEdit::singleline(&mut browser.name_text)
                            .desired_width(f32::INFINITY),
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button(tr(lang, accept)).clicked() {
                        picked = if folder {
                            browser.dir.clone()
                        } else {
                            match browser.confirm() {
                                // the viewer never replaces a file: it says so
                                Ok(Chosen::File(path) | Chosen::Overwrite(path)) => Some(path),
                                Err(_) => None,
                            }
                        };
                    }
                    if ui.button(tr(lang, Text::Cancel)).clicked() {
                        cancelled = true;
                    }
                });
            });
        match picked {
            Some(path) => self.chosen(&chooser.purpose, path),
            None if open && !cancelled => self.chooser = Some(chooser),
            None => {}
        }
    }
}

fn say(ui: &mut Ui, message: &Message, lang: Lang) {
    let text = message.text(lang);
    if message.is_warning() {
        let color = ui.visuals().warn_fg_color;
        ui.colored_label(color, text);
    } else {
        ui.label(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect, Vec2};
    use eh3d_container::AtomicOptions;
    use eh3d_project::{NewProject, OpenOptions, Project};

    fn frame(
        ctx: &egui::Context,
        panel: &mut AttachPanel,
        file: Option<&Path>,
        code: &str,
        w: f32,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, 700.0))),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            panel.show(ui, file, code);
        });
        out.drop_without_applying_deltas();
    }

    fn wait(panel: &mut AttachPanel) {
        let until = Instant::now() + Duration::from_secs(60);
        while panel.model.busy() {
            assert!(Instant::now() < until, "the work did not end");
            std::thread::sleep(Duration::from_millis(5));
            panel.model.poll();
        }
    }

    #[test]
    fn every_state_is_drawn_in_every_language_wide_and_narrow() {
        let dir = std::env::temp_dir().join(format!("eh3d-attachpanel-gui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.eh3d");
        Project::create(
            &file,
            NewProject {
                id: "0f6d3c1a-9b1e-4c53-9b60-3a8f1d2c7e11".to_owned(),
                name: "made up".to_owned(),
                units: "mm".to_owned(),
                created: None,
            },
            &OpenOptions::default(),
            &AtomicOptions::default(),
        )
        .unwrap();
        let script = dir.join("tool.py");
        std::fs::write(&script, "made up").unwrap();

        let ctx = egui::Context::default();
        let mut panel = AttachPanel::new(Program::new("cnc", "0.1.0"));
        assert!(panel.apply_setting("viewer.max_pdf_mb", "8"));
        assert!(panel.apply_setting("attachments.max_mb", "8"));
        assert!(!panel.apply_setting("other", "8"));
        for code in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
            frame(&ctx, &mut panel, None, code, 400.0);
            frame(&ctx, &mut panel, Some(&file), code, 400.0);
        }
        panel.model.request(Request::AttachFile(script));
        wait(&mut panel);
        let id = panel.model.items()[0].id.clone();
        panel.model.select(Some(&id));
        panel.link = Some(LinkDraft::default());
        panel.component.form = LinkForm::PartNumber;
        panel.model.set_filter(None);
        for code in ["en", "ja"] {
            for width in [400.0, 1200.0] {
                frame(&ctx, &mut panel, Some(&file), code, width);
            }
        }
        panel.model.request(Request::Remove(id));
        assert!(panel.model.pending().is_some());
        for purpose in [
            Purpose::Attach,
            Purpose::SaveCopy,
            Purpose::ExtractTo("x".to_owned()),
        ] {
            panel.open_chooser(purpose);
            frame(&ctx, &mut panel, Some(&file), "es", 900.0);
            assert!(panel.chooser.is_some());
        }
        assert!(panel.shown.is_some(), "the viewer follows the list");

        // a panel of a program with no project menu opens one itself
        let mut alone = AttachPanel::new(Program::new("pnp", "0.1.0")).opens_projects();
        frame(&ctx, &mut alone, None, "fr", 500.0);
        alone.choose_project();
        frame(&ctx, &mut alone, None, "fr", 500.0);
        assert!(alone.chooser.is_some());
        alone.open_project(file.clone());
        frame(&ctx, &mut alone, None, "zh", 900.0);
        assert_eq!(alone.model.project(), Some(file.as_path()));
        assert_eq!(alone.model.items().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
