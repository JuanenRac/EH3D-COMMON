//! The frame drawn with egui 0.37: the tab bars with dragging, the drop zones with a preview, the dividers, the floating windows, the folded tabs
//! at the edges, the menu bar with the shortcuts, and the small windows (about, shortcuts, name of a workspace, warnings).
//!
//! The drawing only reads the state of [`DockCore`] and collects what the person did as a list of events; the events are applied to the model
//! after the frame is drawn, so every rule stays in the model, where it is tested without a window.

use crate::about::{self, AboutBrand};
use crate::arrange::{arrange, min_size, pick_drop, ratio_from_pointer, DropSlot};
use crate::core::{AboutInfo, DockCore, Outcome, PanelDef};
use crate::geom::{Axis, Rect, Side};
use crate::menu::{Chosen, DockAction, Kind, MenuBar, Resolved};
use crate::model::{DockError, DropTarget, Layout, Node, PanelState};
use crate::shortcuts::{DockKey, Shortcut};
use crate::stdmenus::shortcut_table;
use crate::texts::{panel_label, tr, DockText};
use crate::workspaces::{valid_name, LoadWarning, Workspace};
use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Order, Pos2, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2,
};
use eh3d_view::texts::Lang;

/// The height of a tab bar, in points.
pub const BAR_H: f32 = 24.0;
/// The width of a divider, in points.
pub const GAP: f32 = 5.0;
/// The thickness of the strip of folded tabs along an edge.
pub const STRIP: f32 = 24.0;
/// How close to an edge of the window a drop is a drop on the edge.
pub const EDGE_MARGIN: f32 = 28.0;

/// What draws the inside of a panel.
pub type Body = Box<dyn FnMut(&mut Ui)>;

fn to_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(Pos2::new(r.x, r.y), Vec2::new(r.w, r.h))
}

fn from_egui(r: egui::Rect) -> Rect {
    Rect::new(r.min.x, r.min.y, r.width(), r.height())
}

/// The colours of the frame, taken from the visuals of the window so that it follows the theme (light or dark).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    bg: Color32,
    bar: Color32,
    tab: Color32,
    tab_active: Color32,
    tab_hover: Color32,
    text: Color32,
    dim: Color32,
    divider: Color32,
    accent: Color32,
    border: Color32,
}

impl Palette {
    /// The palette for some visuals.
    pub fn from_visuals(v: &egui::Visuals) -> Palette {
        Palette {
            bg: v.panel_fill,
            bar: v.widgets.noninteractive.bg_fill,
            tab: v.widgets.inactive.weak_bg_fill,
            tab_active: v.selection.bg_fill,
            tab_hover: v.widgets.hovered.weak_bg_fill,
            text: v.text_color(),
            dim: v.weak_text_color(),
            divider: v.widgets.noninteractive.bg_stroke.color,
            accent: v.selection.stroke.color,
            border: v.widgets.noninteractive.bg_stroke.color,
        }
    }
}

enum Ev {
    Focus(String),
    Action(DockAction),
    Ratio(Vec<bool>, f32),
    BeginDrag(String),
    FloatRect(usize, Rect),
    ToggleOverlay(String),
    CloseOverlay,
}

struct View<'a> {
    defs: &'a [PanelDef],
    lang: Lang,
    layout: &'a Layout,
    pal: Palette,
    locked: bool,
}

impl View<'_> {
    fn title(&self, id: &str) -> String {
        self.defs
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.title.get(self.lang).to_string())
            .unwrap_or_else(|| id.to_string())
    }

    fn icon(&self, id: &str) -> &'static str {
        self.defs
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.icon)
            .unwrap_or("")
    }

    fn mins(&self, id: &str) -> (f32, f32) {
        self.defs
            .iter()
            .find(|d| d.id == id)
            .map(|d| (d.min_w, d.min_h + BAR_H))
            .unwrap_or((0.0, 0.0))
    }

    fn state_word(&self, id: &str) -> DockText {
        match self.layout.state_of(id) {
            Some(PanelState::Floating) => DockText::StateFloating,
            Some(PanelState::Hidden) => DockText::StateHidden,
            Some(PanelState::AutoHidden(_)) => DockText::StateAutoHidden,
            _ => DockText::StateDocked,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Glyph {
    Maximize,
    Restore,
    Dock,
    Close,
}

fn glyph_button(
    ui: &mut Ui,
    v: &View,
    rect: egui::Rect,
    id: Id,
    glyph: Glyph,
    tip: &str,
    enabled: bool,
) -> bool {
    let resp = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let p = ui.painter();
    if resp.hovered() && enabled {
        p.rect_filled(rect, CornerRadius::same(3), v.pal.tab_hover);
    }
    let c = if enabled { v.pal.text } else { v.pal.dim };
    let s = Stroke::new(1.3, c);
    let r = rect.shrink(5.0);
    match glyph {
        Glyph::Maximize => {
            p.rect_stroke(r, CornerRadius::ZERO, s, StrokeKind::Inside);
        }
        Glyph::Restore => {
            let a =
                egui::Rect::from_min_max(r.min + Vec2::new(2.0, 0.0), r.max - Vec2::new(0.0, 2.0));
            let b =
                egui::Rect::from_min_max(r.min + Vec2::new(0.0, 2.0), r.max - Vec2::new(2.0, 0.0));
            p.rect_stroke(a, CornerRadius::ZERO, s, StrokeKind::Inside);
            p.rect_stroke(b, CornerRadius::ZERO, s, StrokeKind::Inside);
        }
        Glyph::Dock => {
            p.rect_stroke(r, CornerRadius::ZERO, s, StrokeKind::Inside);
            p.line_segment(
                [
                    r.left_top() + Vec2::new(0.0, 3.0),
                    r.right_top() + Vec2::new(0.0, 3.0),
                ],
                s,
            );
        }
        Glyph::Close => {
            p.line_segment([r.left_top(), r.right_bottom()], s);
            p.line_segment([r.right_top(), r.left_bottom()], s);
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, tip));
    let clicked = resp.clicked() && enabled;
    resp.on_hover_text(tip);
    clicked
}

fn tab_menu(ui: &mut Ui, v: &View, id: &str, ev: &mut Vec<Ev>) {
    let state = v.layout.state_of(id);
    let free = !v.locked;
    fn push(ui: &mut Ui, ev: &mut Vec<Ev>, a: DockAction) {
        ev.push(Ev::Action(a));
        ui.close();
    }
    if state == Some(PanelState::Floating) {
        if ui
            .add_enabled(free, egui::Button::new(tr(v.lang, DockText::Dock)))
            .clicked()
        {
            push(ui, ev, DockAction::DockPanel(id.to_string()));
        }
    } else if ui
        .add_enabled(free, egui::Button::new(tr(v.lang, DockText::Float)))
        .clicked()
    {
        push(ui, ev, DockAction::FloatPanel(id.to_string()));
    }
    ui.add_enabled_ui(free, |ui| {
        ui.menu_button(tr(v.lang, DockText::AutoHide), |ui| {
            for s in Side::ALL {
                if ui.button(side_word(v.lang, s)).clicked() {
                    push(ui, ev, DockAction::AutoHidePanel(id.to_string(), s));
                }
            }
        });
        ui.menu_button(tr(v.lang, DockText::MoveTo), |ui| {
            for s in Side::ALL {
                if ui.button(side_word(v.lang, s)).clicked() {
                    push(ui, ev, DockAction::MovePanel(id.to_string(), s));
                }
            }
        });
    });
    if state == Some(PanelState::Docked) {
        let word = if v.layout.maximized.as_deref() == Some(id) {
            DockText::RestoreSize
        } else {
            DockText::Maximize
        };
        if ui.button(tr(v.lang, word)).clicked() {
            ev.push(Ev::Focus(id.to_string()));
            push(ui, ev, DockAction::ToggleMaximize);
        }
    }
    ui.separator();
    if ui
        .add_enabled(free, egui::Button::new(tr(v.lang, DockText::Hide)))
        .clicked()
    {
        push(ui, ev, DockAction::HidePanel(id.to_string()));
    }
}

fn side_word(lang: Lang, s: Side) -> &'static str {
    tr(
        lang,
        match s {
            Side::Left => DockText::Left,
            Side::Right => DockText::Right,
            Side::Top => DockText::Top,
            Side::Bottom => DockText::Bottom,
        },
    )
}

/// Draws a row of tabs in `bar` and gives the rectangle of each tab that was drawn (in the order of `ids`).
#[allow(clippy::too_many_arguments)]
fn tab_bar(
    ui: &mut Ui,
    v: &View,
    bar: egui::Rect,
    ids: &[String],
    active: usize,
    floating: bool,
    maximized: bool,
    ev: &mut Vec<Ev>,
) -> Vec<Rect> {
    let p = ui.painter().clone();
    p.rect_filled(bar, CornerRadius::ZERO, v.pal.bar);
    // the buttons on the right: maximise or restore (dock for a floating window), and hide
    let b = BAR_H - 6.0;
    let mut right = bar.right() - 3.0;
    let act = ids.get(active).cloned().unwrap_or_default();
    let hide = egui::Rect::from_min_size(Pos2::new(right - b, bar.top() + 3.0), Vec2::splat(b));
    right -= b + 2.0;
    if glyph_button(
        ui,
        v,
        hide,
        Id::unique(("eh3d_dock_hide", &act)),
        Glyph::Close,
        tr(v.lang, DockText::Hide),
        !v.locked,
    ) {
        ev.push(Ev::Action(DockAction::HidePanel(act.clone())));
    }
    let mid = egui::Rect::from_min_size(Pos2::new(right - b, bar.top() + 3.0), Vec2::splat(b));
    right -= b + 2.0;
    if floating {
        if glyph_button(
            ui,
            v,
            mid,
            Id::unique(("eh3d_dock_dockbtn", &act)),
            Glyph::Dock,
            tr(v.lang, DockText::Dock),
            !v.locked,
        ) {
            ev.push(Ev::Action(DockAction::DockPanel(act.clone())));
        }
    } else {
        let (g, word) = if maximized {
            (Glyph::Restore, DockText::RestoreSize)
        } else {
            (Glyph::Maximize, DockText::Maximize)
        };
        if glyph_button(
            ui,
            v,
            mid,
            Id::unique(("eh3d_dock_maxbtn", &act)),
            g,
            tr(v.lang, word),
            true,
        ) {
            ev.push(Ev::Focus(act.clone()));
            ev.push(Ev::Action(DockAction::ToggleMaximize));
        }
    }
    // measure the tabs and keep the one on show in view
    let font = FontId::proportional(13.0);
    let limit = right - bar.left() - 6.0;
    let widths: Vec<f32> = ids
        .iter()
        .map(|id| {
            p.layout_no_wrap(v.title(id), font.clone(), v.pal.text)
                .size()
                .x
                + 22.0
        })
        .collect();
    let mut start = active.min(widths.len().saturating_sub(1));
    let mut used = widths.get(start).copied().unwrap_or(0.0);
    while start > 0 && used + widths[start - 1] <= limit {
        start -= 1;
        used += widths[start];
    }
    let mut rects = vec![Rect::new(0.0, 0.0, 0.0, 0.0); ids.len()];
    let mut x = bar.left() + 4.0;
    for (i, id) in ids.iter().enumerate().skip(start) {
        let w = widths[i].min((bar.left() + limit + 4.0 - x).max(0.0));
        if w < 24.0 {
            break;
        }
        let rect =
            egui::Rect::from_min_size(Pos2::new(x, bar.top() + 2.0), Vec2::new(w, BAR_H - 2.0));
        rects[i] = from_egui(rect);
        let resp = ui.interact(
            rect,
            Id::unique(("eh3d_dock_tab", id)),
            Sense::click_and_drag(),
        );
        let fill = if i == active {
            v.pal.tab_active
        } else if resp.hovered() {
            v.pal.tab_hover
        } else {
            v.pal.tab
        };
        p.rect_filled(
            rect,
            CornerRadius {
                nw: 4,
                ne: 4,
                sw: 0,
                se: 0,
            },
            fill,
        );
        let label = if w < 60.0 && !v.icon(id).is_empty() {
            v.icon(id).to_string()
        } else {
            v.title(id)
        };
        let clipped = p.with_clip_rect(rect.shrink2(Vec2::new(4.0, 0.0)));
        clipped.text(
            Pos2::new(rect.left() + 11.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            font.clone(),
            if i == active { v.pal.text } else { v.pal.dim },
        );
        let summary = panel_label(v.lang, &v.title(id), v.state_word(id));
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &summary));
        if resp.clicked() || resp.drag_started() {
            ev.push(Ev::Focus(id.clone()));
        }
        if resp.drag_started() && !v.locked {
            ev.push(Ev::BeginDrag(id.clone()));
        }
        resp.context_menu(|ui| tab_menu(ui, v, id, ev));
        x += w + 2.0;
    }
    rects
}

/// Draws one group of tabs and its content, and notes where it is for the drop zones.
#[allow(clippy::too_many_arguments)]
fn draw_group(
    ui: &mut Ui,
    v: &View,
    rect: egui::Rect,
    ids: &[String],
    active: usize,
    maximized: bool,
    bodies: &mut [(String, Body)],
    ev: &mut Vec<Ev>,
    slots: &mut Vec<DropSlot>,
) {
    let p = ui.painter().clone();
    p.rect_filled(rect, CornerRadius::same(3), v.pal.bg);
    let bar = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), BAR_H));
    let tabs = tab_bar(ui, v, bar, ids, active, false, maximized, ev);
    p.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, v.pal.border),
        StrokeKind::Inside,
    );
    let content = egui::Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, bar.bottom()),
        rect.max - Vec2::splat(1.0),
    );
    if let Some(id) = ids.get(active) {
        run_body(ui, content, id, bodies, v, ev);
    }
    slots.push(DropSlot {
        panels: ids.to_vec(),
        rect: from_egui(rect),
        tabs,
        bar_h: BAR_H,
        floating: false,
    });
}

fn run_body(
    ui: &mut Ui,
    content: egui::Rect,
    id: &str,
    bodies: &mut [(String, Body)],
    v: &View,
    ev: &mut Vec<Ev>,
) {
    if content.width() < 4.0 || content.height() < 4.0 {
        return;
    }
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(content.shrink(3.0))
            .id_salt(("eh3d_dock_body", id)),
    );
    child.set_clip_rect(content);
    match bodies.iter_mut().find(|(n, _)| n == id) {
        Some((_, body)) => body(&mut child),
        None => {
            child.colored_label(v.pal.dim, v.title(id));
        }
    }
    if ui.input(|i| i.pointer.primary_pressed())
        && ui
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|pos| content.contains(pos))
    {
        ev.push(Ev::Focus(id.to_string()));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tree(
    ui: &mut Ui,
    v: &View,
    root: &Node,
    area: egui::Rect,
    bodies: &mut [(String, Body)],
    ev: &mut Vec<Ev>,
    slots: &mut Vec<DropSlot>,
) {
    let mins = |id: &str| v.mins(id);
    let arranged = arrange(root, from_egui(area), GAP, &mins);
    for leaf in &arranged.leaves {
        if let Some(Node::Tabs(g)) = root.get(&leaf.path) {
            draw_group(
                ui,
                v,
                to_egui(leaf.rect),
                &g.panels,
                g.active,
                false,
                bodies,
                ev,
                slots,
            );
        }
    }
    for d in &arranged.dividers {
        let hit = to_egui(d.rect).expand2(match d.axis {
            Axis::Row => Vec2::new(2.0, 0.0),
            Axis::Column => Vec2::new(0.0, 2.0),
        });
        let resp = ui.interact(
            hit,
            Id::unique(("eh3d_dock_div", &d.path)),
            if v.locked {
                Sense::hover()
            } else {
                Sense::drag()
            },
        );
        if !v.locked && (resp.hovered() || resp.dragged()) {
            ui.ctx().set_cursor_icon(match d.axis {
                Axis::Row => egui::CursorIcon::ResizeHorizontal,
                Axis::Column => egui::CursorIcon::ResizeVertical,
            });
        }
        let hot = resp.hovered() || resp.dragged();
        ui.painter().rect_filled(
            to_egui(d.rect),
            CornerRadius::ZERO,
            if hot {
                v.pal.accent
            } else {
                v.pal.divider.gamma_multiply(0.5)
            },
        );
        if resp.dragged() && !v.locked {
            if let (Some(pos), Some(Node::Split { first, second, .. })) =
                (resp.interact_pointer_pos(), root.get(&d.path))
            {
                let (mf, ms) = (min_size(first, &mins, GAP), min_size(second, &mins, GAP));
                let (pointer, a, b) = match d.axis {
                    Axis::Row => (pos.x, mf.0, ms.0),
                    Axis::Column => (pos.y, mf.1, ms.1),
                };
                ev.push(Ev::Ratio(
                    d.path.clone(),
                    ratio_from_pointer(d.parent, d.axis, pointer, GAP, a, b),
                ));
            }
        }
    }
}

/// Draws the strips of folded tabs and gives back the area that is left and the rectangles of the tabs.
fn draw_strips(
    ui: &mut Ui,
    v: &View,
    mut area: egui::Rect,
    overlay: Option<&str>,
    ev: &mut Vec<Ev>,
) -> (egui::Rect, Vec<egui::Rect>) {
    let mut taken = Vec::new();
    let font = FontId::proportional(12.0);
    for side in Side::ALL {
        let ids: Vec<&String> = v
            .layout
            .auto_hidden
            .iter()
            .filter(|(_, s)| *s == side)
            .map(|(p, _)| p)
            .collect();
        if ids.is_empty() {
            continue;
        }
        let strip = match side {
            Side::Left => egui::Rect::from_min_size(area.min, Vec2::new(STRIP, area.height())),
            Side::Right => egui::Rect::from_min_size(
                Pos2::new(area.right() - STRIP, area.top()),
                Vec2::new(STRIP, area.height()),
            ),
            Side::Top => egui::Rect::from_min_size(area.min, Vec2::new(area.width(), STRIP)),
            Side::Bottom => egui::Rect::from_min_size(
                Pos2::new(area.left(), area.bottom() - STRIP),
                Vec2::new(area.width(), STRIP),
            ),
        };
        ui.painter()
            .rect_filled(strip, CornerRadius::ZERO, v.pal.bar);
        let mut pos = 6.0;
        for id in ids {
            let galley = ui
                .painter()
                .layout_no_wrap(v.title(id), font.clone(), v.pal.text);
            let len = galley.size().x + 16.0;
            let rect = match side {
                Side::Left | Side::Right => egui::Rect::from_min_size(
                    Pos2::new(strip.left() + 2.0, strip.top() + pos),
                    Vec2::new(STRIP - 4.0, len),
                ),
                Side::Top | Side::Bottom => egui::Rect::from_min_size(
                    Pos2::new(strip.left() + pos, strip.top() + 2.0),
                    Vec2::new(len, STRIP - 4.0),
                ),
            };
            pos += len + 3.0;
            let resp = ui.interact(rect, Id::unique(("eh3d_dock_strip", id)), Sense::click());
            let open = overlay == Some(id.as_str());
            let fill = if open {
                v.pal.tab_active
            } else if resp.hovered() {
                v.pal.tab_hover
            } else {
                v.pal.tab
            };
            ui.painter().rect_filled(rect, CornerRadius::same(3), fill);
            match side {
                Side::Left | Side::Right => {
                    let shape = egui::epaint::TextShape::new(
                        Pos2::new(rect.left() + 5.0, rect.bottom() - 8.0),
                        galley,
                        v.pal.text,
                    )
                    .with_angle(-std::f32::consts::FRAC_PI_2);
                    ui.painter().add(shape);
                }
                _ => {
                    ui.painter().galley(
                        Pos2::new(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
                        galley,
                        v.pal.text,
                    );
                }
            }
            let summary = panel_label(v.lang, &v.title(id), DockText::StateAutoHidden);
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &summary));
            if resp.clicked() {
                ev.push(Ev::ToggleOverlay(id.clone()));
            }
            let idc = id.clone();
            resp.context_menu(|ui| {
                if ui
                    .add_enabled(!v.locked, egui::Button::new(tr(v.lang, DockText::Dock)))
                    .clicked()
                {
                    ev.push(Ev::Action(DockAction::DockPanel(idc.clone())));
                    ui.close();
                }
                if ui
                    .add_enabled(!v.locked, egui::Button::new(tr(v.lang, DockText::Hide)))
                    .clicked()
                {
                    ev.push(Ev::Action(DockAction::HidePanel(idc.clone())));
                    ui.close();
                }
            });
            taken.push(rect);
        }
        area = match side {
            Side::Left => {
                egui::Rect::from_min_max(Pos2::new(area.left() + STRIP, area.top()), area.max)
            }
            Side::Right => {
                egui::Rect::from_min_max(area.min, Pos2::new(area.right() - STRIP, area.bottom()))
            }
            Side::Top => {
                egui::Rect::from_min_max(Pos2::new(area.left(), area.top() + STRIP), area.max)
            }
            Side::Bottom => {
                egui::Rect::from_min_max(area.min, Pos2::new(area.right(), area.bottom() - STRIP))
            }
        };
    }
    (area, taken)
}

/// The frame: a core and the drawing of it.
pub struct Dock {
    core: DockCore,
    bodies: Vec<(String, Body)>,
    drag: Option<String>,
    overlay: Option<String>,
    generation: u32,
    ask_name: Option<String>,
    name_error: bool,
    show_about: bool,
    brand: Option<AboutBrand>,
    show_shortcuts: bool,
    banner: Vec<String>,
    shortcut_rows: Vec<(String, String, String)>,
    menu_disabled: Vec<String>,
    menu_checks: Vec<(String, bool)>,
    last_area: Rect,
    last_slots: Vec<DropSlot>,
}

impl Dock {
    /// A frame for these panels, with these factory workspaces (the first one is in use).
    pub fn new(defs: Vec<PanelDef>, factory: Vec<Workspace>) -> Dock {
        Dock {
            core: DockCore::new(defs, factory),
            bodies: Vec::new(),
            drag: None,
            overlay: None,
            generation: 0,
            ask_name: None,
            name_error: false,
            show_about: false,
            brand: None,
            show_shortcuts: false,
            banner: Vec::new(),
            shortcut_rows: Vec::new(),
            menu_disabled: Vec::new(),
            menu_checks: Vec::new(),
            last_area: Rect::new(0.0, 0.0, 0.0, 0.0),
            last_slots: Vec::new(),
        }
    }

    /// A frame with one factory layout (called `Default`).
    pub fn single(defs: Vec<PanelDef>, factory: Layout) -> Dock {
        Dock::new(defs, vec![Workspace::new("Default", factory)])
    }

    /// Gives a panel the function that draws its inside.
    pub fn register(&mut self, id: &str, body: impl FnMut(&mut Ui) + 'static) {
        self.bodies.retain(|(n, _)| n != id);
        self.bodies.push((id.to_string(), Box::new(body)));
    }

    /// The state of the frame.
    pub fn core(&self) -> &DockCore {
        &self.core
    }

    /// The state of the frame, to change it.
    pub fn core_mut(&mut self) -> &mut DockCore {
        self.generation = self.generation.wrapping_add(1);
        &mut self.core
    }

    /// Sets the language of the window.
    pub fn set_language(&mut self, lang: Lang) {
        self.core.lang = lang;
    }

    /// Sets what the window about the program shows.
    pub fn set_about(&mut self, about: AboutInfo) {
        self.core.about = about;
    }

    /// Gives the window about the program the look of its program of the PHOENIX family (see [`about_contents`]): the picture in a ring of the
    /// accent colour, the name in large letters, one sentence, the author and the links. Without it the window is the plain list of lines.
    pub fn set_brand(&mut self, brand: AboutBrand) {
        self.brand = Some(brand);
    }

    /// The program turns entries of the menus off and sets their check marks, by id; it can call this every frame.
    pub fn set_menu_state(&mut self, disabled: Vec<String>, checks: Vec<(String, bool)>) {
        self.menu_disabled = disabled;
        self.menu_checks = checks;
    }

    /// The layout in use as a text.
    pub fn layout_text(&self) -> String {
        self.core.layout_text()
    }

    /// Replaces the layout in use with a text (see [`DockCore::set_layout_text`]).
    pub fn set_layout_text(&mut self, text: &str) -> Vec<LoadWarning> {
        self.generation = self.generation.wrapping_add(1);
        self.core.set_layout_text(text)
    }

    /// Reads the lines of the frame out of a settings text.
    pub fn load_config(&mut self, text: &str) {
        self.generation = self.generation.wrapping_add(1);
        self.core.load_config(text);
    }

    /// The settings text with the lines of the frame brought up to date.
    pub fn config_text(&self, existing: &str) -> String {
        self.core.config_text(existing)
    }

    /// Does what a frame action asks and opens the window it needs (a name for a workspace, about, the shortcuts).
    pub fn apply(&mut self, action: DockAction) -> Result<(), DockError> {
        let restyle = matches!(
            action,
            DockAction::SwitchWorkspace(_)
                | DockAction::RestoreLayout
                | DockAction::SaveWorkspaceAs(_)
                | DockAction::DeleteWorkspace
        );
        let out = self.core.apply(&action)?;
        if restyle {
            self.generation = self.generation.wrapping_add(1);
            self.overlay = None;
        }
        match out {
            Outcome::Done => {}
            Outcome::NeedsName => {
                self.ask_name = Some(String::new());
                self.name_error = false;
            }
            Outcome::ShowAbout => self.show_about = true,
            Outcome::ShowShortcuts => self.show_shortcuts = true,
        }
        Ok(())
    }

    /// The groups of tabs drawn in the last frame, with their rectangles (what a test or a screen reader helper needs to find a tab).
    pub fn last_slots(&self) -> &[DropSlot] {
        &self.last_slots
    }

    /// Draws the panels in the space left in `ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        let layout = self.core.layout().clone();
        let defs: Vec<PanelDef> = self.core.defs().to_vec();
        let v = View {
            defs: &defs,
            lang: self.core.lang,
            layout: &layout,
            pal: Palette::from_visuals(ui.visuals()),
            locked: layout.locked,
        };
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        self.last_area = from_egui(full);
        let mut ev: Vec<Ev> = Vec::new();
        let mut slots: Vec<DropSlot> = Vec::new();
        let (area, strips) = draw_strips(ui, &v, full, self.overlay.as_deref(), &mut ev);
        let area = area.shrink(2.0);
        match (&layout.root, layout.maximized.as_deref()) {
            (Some(root), Some(m)) => {
                if let Some(g) = layout.group_of(m) {
                    draw_group(
                        ui,
                        &v,
                        area,
                        &g.panels,
                        g.active,
                        true,
                        &mut self.bodies,
                        &mut ev,
                        &mut slots,
                    );
                } else {
                    draw_tree(ui, &v, root, area, &mut self.bodies, &mut ev, &mut slots);
                }
            }
            (Some(root), None) => {
                draw_tree(ui, &v, root, area, &mut self.bodies, &mut ev, &mut slots)
            }
            (None, _) => {
                ui.painter()
                    .rect_filled(area, CornerRadius::same(3), v.pal.bg);
                ui.painter().text(
                    area.center(),
                    Align2::CENTER_CENTER,
                    tr(v.lang, DockText::DropHere),
                    FontId::proportional(14.0),
                    v.pal.dim,
                );
            }
        }
        // floating windows, over the tree
        let ctx = ui.ctx().clone();
        for (i, f) in layout.floating.iter().enumerate() {
            let first = f.group.panels.first().cloned().unwrap_or_default();
            let active = f.group.active.min(f.group.panels.len().saturating_sub(1));
            let mut open = true;
            let shown = egui::Window::new(v.title(&f.group.panels[active]))
                .id(Id::unique(("eh3d_dock_float", self.generation, &first)))
                .open(&mut open)
                .default_rect(to_egui(f.rect))
                .resizable(!v.locked)
                .movable(!v.locked)
                .collapsible(false)
                .show(&ctx, |ui| {
                    let w = ui.available_width();
                    let (bar, _) = ui.allocate_exact_size(Vec2::new(w, BAR_H), Sense::hover());
                    let tabs = tab_bar(ui, &v, bar, &f.group.panels, active, true, false, &mut ev);
                    let rest = ui.available_rect_before_wrap();
                    run_body(
                        ui,
                        rest,
                        &f.group.panels[active],
                        &mut self.bodies,
                        &v,
                        &mut ev,
                    );
                    tabs
                });
            if !open {
                ev.push(Ev::Action(DockAction::HidePanel(
                    f.group.panels[active].clone(),
                )));
            }
            if let Some(r) = shown {
                if let Some(tabs) = r.inner {
                    let rect = from_egui(r.response.rect);
                    slots.push(DropSlot {
                        panels: f.group.panels.clone(),
                        rect,
                        tabs,
                        bar_h: BAR_H + 24.0,
                        floating: true,
                    });
                    if (rect.x - f.rect.x).abs() > 1.0
                        || (rect.y - f.rect.y).abs() > 1.0
                        || (rect.w - f.rect.w).abs() > 1.0
                        || (rect.h - f.rect.h).abs() > 1.0
                    {
                        ev.push(Ev::FloatRect(i, rect));
                    }
                }
            }
        }
        self.last_slots = slots.clone();
        // a folded panel that was opened
        self.draw_overlay(&ctx, &v, area, &strips, &mut ev);
        // dragging a tab: the zone under the pointer and the preview
        self.handle_drag(&ctx, &v, full, &slots, &mut ev);
        let drop = self.take_drop(&ctx, &slots);
        // the keyboard
        self.apply_events(ev);
        if let Some((id, target)) = drop {
            let _ = self.core.layout_mut().move_panel(&id, target);
            self.core.set_focus(&id);
        }
        self.show_windows(&ctx);
    }

    /// Draws the panels in the whole window (the root of the context).
    pub fn show_ctx(&mut self, ctx: &egui::Context) {
        ctx.root_ui(|ui| self.show(ui));
    }

    fn draw_overlay(
        &mut self,
        ctx: &egui::Context,
        v: &View,
        area: egui::Rect,
        strips: &[egui::Rect],
        ev: &mut Vec<Ev>,
    ) {
        let Some(id) = self.overlay.clone() else {
            return;
        };
        let Some((_, side)) = v.layout.auto_hidden.iter().find(|(p, _)| *p == id) else {
            self.overlay = None;
            return;
        };
        let (pos, size) = match side {
            Side::Left => (
                area.min,
                Vec2::new((area.width() * 0.45).min(360.0), area.height()),
            ),
            Side::Right => {
                let w = (area.width() * 0.45).min(360.0);
                (
                    Pos2::new(area.right() - w, area.top()),
                    Vec2::new(w, area.height()),
                )
            }
            Side::Top => (
                area.min,
                Vec2::new(area.width(), (area.height() * 0.45).min(280.0)),
            ),
            Side::Bottom => {
                let h = (area.height() * 0.45).min(280.0);
                (
                    Pos2::new(area.left(), area.bottom() - h),
                    Vec2::new(area.width(), h),
                )
            }
        };
        let shown = egui::Area::new(Id::unique("eh3d_dock_overlay"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_size(size - Vec2::splat(12.0));
                    ui.set_max_size(size - Vec2::splat(12.0));
                    let (bar, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), BAR_H),
                        Sense::hover(),
                    );
                    ui.painter().text(
                        bar.left_center(),
                        Align2::LEFT_CENTER,
                        v.title(&id),
                        FontId::proportional(13.0),
                        v.pal.text,
                    );
                    let b = BAR_H - 6.0;
                    let dock = egui::Rect::from_min_size(
                        Pos2::new(bar.right() - 2.0 * b - 4.0, bar.top() + 3.0),
                        Vec2::splat(b),
                    );
                    let close = egui::Rect::from_min_size(
                        Pos2::new(bar.right() - b - 1.0, bar.top() + 3.0),
                        Vec2::splat(b),
                    );
                    if glyph_button(
                        ui,
                        v,
                        dock,
                        Id::unique("eh3d_dock_overlay_dock"),
                        Glyph::Dock,
                        tr(v.lang, DockText::Dock),
                        !v.locked,
                    ) {
                        ev.push(Ev::Action(DockAction::DockPanel(id.clone())));
                    }
                    if glyph_button(
                        ui,
                        v,
                        close,
                        Id::unique("eh3d_dock_overlay_close"),
                        Glyph::Close,
                        tr(v.lang, DockText::Close),
                        true,
                    ) {
                        ev.push(Ev::CloseOverlay);
                    }
                    let rest = ui.available_rect_before_wrap();
                    run_body(ui, rest, &id, &mut self.bodies, v, ev);
                });
            });
        let inside = shown.response.rect;
        let (pressed, pos_now, esc) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.interact_pos(),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if esc
            || (pressed
                && pos_now
                    .is_some_and(|p| !inside.contains(p) && !strips.iter().any(|s| s.contains(p))))
        {
            ev.push(Ev::CloseOverlay);
        }
    }

    fn handle_drag(
        &mut self,
        ctx: &egui::Context,
        v: &View,
        full: egui::Rect,
        slots: &[DropSlot],
        ev: &mut Vec<Ev>,
    ) {
        let Some(id) = self.drag.clone() else { return };
        let (pos, esc) = ctx.input(|i| (i.pointer.latest_pos(), i.key_pressed(egui::Key::Escape)));
        if esc || v.locked {
            self.drag = None;
            return;
        }
        let Some(pos) = pos else { return };
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::unique("eh3d_dock_dropfx"),
        ));
        if let Some((target, preview)) =
            pick_drop(slots, from_egui(full), pos.x, pos.y, &id, EDGE_MARGIN)
        {
            let r = to_egui(preview);
            painter.rect_filled(r, CornerRadius::same(3), v.pal.accent.gamma_multiply(0.25));
            painter.rect_stroke(
                r,
                CornerRadius::same(3),
                Stroke::new(2.0, v.pal.accent),
                StrokeKind::Inside,
            );
            let word = match &target {
                DropTarget::OnGroup { zone, .. } if zone.side().is_none() => {
                    tr(v.lang, DockText::DropAsTab)
                }
                DropTarget::Float(_) => tr(v.lang, DockText::Float),
                _ => tr(v.lang, DockText::DropHere),
            };
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                word,
                FontId::proportional(13.0),
                v.pal.text,
            );
        }
        painter.text(
            pos + Vec2::new(14.0, 14.0),
            Align2::LEFT_TOP,
            v.title(&id),
            FontId::proportional(13.0),
            v.pal.text,
        );
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        let _ = ev;
    }

    fn take_drop(
        &mut self,
        ctx: &egui::Context,
        slots: &[DropSlot],
    ) -> Option<(String, DropTarget)> {
        let id = self.drag.clone()?;
        let (released, pos) = ctx.input(|i| {
            (
                i.pointer.any_released() || !i.pointer.any_down(),
                i.pointer.latest_pos(),
            )
        });
        if !released {
            return None;
        }
        self.drag = None;
        let pos = pos?;
        let (target, _) = pick_drop(slots, self.last_area, pos.x, pos.y, &id, EDGE_MARGIN)?;
        Some((id, target))
    }

    fn apply_events(&mut self, events: Vec<Ev>) {
        for e in events {
            match e {
                Ev::Focus(id) => {
                    let _ = self.core.layout_mut().set_active(&id);
                    self.core.set_focus(&id);
                }
                Ev::Action(a) => {
                    let _ = self.apply(a);
                }
                Ev::Ratio(path, r) => {
                    let _ = self.core.layout_mut().set_ratio(&path, r);
                }
                Ev::BeginDrag(id) => self.drag = Some(id),
                Ev::FloatRect(i, r) => {
                    if let Some(f) = self.core.layout_mut().floating.get_mut(i) {
                        f.rect = r;
                    }
                }
                Ev::ToggleOverlay(id) => {
                    self.overlay = if self.overlay.as_deref() == Some(id.as_str()) {
                        None
                    } else {
                        Some(id)
                    };
                }
                Ev::CloseOverlay => self.overlay = None,
            }
        }
    }

    // ---- the menu bar -----------------------------------------------------------------------------------------------------------------

    fn menu_ctx(&self) -> crate::menu::MenuContext {
        let mut c = self.core.menu_context();
        c.disabled = self.menu_disabled.clone();
        c.checks = self.menu_checks.clone();
        c
    }

    /// Looks for a shortcut pressed this frame (the ones of the menus first, then the ones of the frame) and gives what it stands for. The frame's
    /// own actions are done here. A key with no modifier is ignored while a text box has the keyboard.
    pub fn handle_shortcuts<A: Clone>(
        &mut self,
        ctx: &egui::Context,
        bar: &MenuBar<A>,
    ) -> Option<Chosen<A>> {
        let mctx = self.menu_ctx();
        let built = bar.resolve(&mctx);
        let mut candidates: Vec<(Shortcut, Option<Chosen<A>>)> = Vec::new();
        fn collect<A: Clone>(items: &[Resolved<A>], out: &mut Vec<(Shortcut, Option<Chosen<A>>)>) {
            for r in items {
                match &r.kind {
                    Kind::Submenu(s) => collect(s, out),
                    Kind::Action(c) => {
                        if let Some(s) = &r.shortcut {
                            out.push((s.clone(), Some(c.clone())));
                        }
                    }
                    _ => {}
                }
            }
        }
        for m in &built {
            collect(&m.items, &mut candidates);
        }
        for (_, s) in self.core.keys().named() {
            if !candidates.iter().any(|(c, _)| *c == s) {
                candidates.push((s, None));
            }
        }
        // the most specific first: Ctrl+Shift+S before Ctrl+S
        candidates
            .sort_by_key(|(s, _)| std::cmp::Reverse(s.ctrl as u8 + s.alt as u8 + s.shift as u8));
        let typing = ctx.text_edit_focused();
        for (s, chosen) in candidates {
            if typing && !s.has_modifier() {
                continue;
            }
            let Some((mods, key)) = egui_shortcut(&s) else {
                continue;
            };
            if ctx.input_mut(|i| i.consume_key(mods, key)) {
                let chosen = match chosen {
                    Some(c) => c,
                    None => Chosen::Dock(self.core.resolve_key(&s)?),
                };
                if let Chosen::Dock(a) = &chosen {
                    let _ = self.apply(a.clone());
                }
                return Some(chosen);
            }
        }
        None
    }

    /// Draws the menu bar in `ui` (put it in a top panel) and handles the shortcuts. Gives what the person chose with the mouse or the keyboard: an
    /// action of the program, a recent file, or a frame action that has already been done.
    pub fn menu_bar<A: Clone>(&mut self, ui: &mut Ui, bar: &MenuBar<A>) -> Option<Chosen<A>> {
        let ctx = ui.ctx().clone();
        let mut chosen = self.handle_shortcuts(&ctx, bar);
        let by_key = chosen.is_some();
        let mctx = self.menu_ctx();
        let built = bar.resolve(&mctx);
        let mut rows = shortcut_table(&built);
        let framed: Vec<(String, String, String)> = self
            .core
            .keys()
            .named()
            .into_iter()
            .map(|(name, s)| {
                (
                    tr(self.core.lang, DockText::MenuWindow).to_string(),
                    self.key_label(&name),
                    s.to_string(),
                )
            })
            .filter(|(_, _, s)| !rows.iter().any(|r| r.2 == *s))
            .collect();
        rows.extend(framed);
        self.shortcut_rows = rows;
        egui::MenuBar::new().ui(ui, |ui| {
            for m in &built {
                ui.menu_button(m.title.as_str(), |ui| draw_items(ui, &m.items, &mut chosen));
            }
        });
        if let (false, Some(Chosen::Dock(a))) = (by_key, &chosen) {
            // a shortcut did its action when it was found; a click has not been done yet
            let _ = self.apply(a.clone());
        }
        chosen
    }

    fn key_label(&self, name: &str) -> String {
        let lang = self.core.lang;
        match DockKey::from_name(name) {
            Some(DockKey::Maximize) => tr(lang, DockText::Maximize).to_string(),
            Some(DockKey::NextTab) => tr(lang, DockText::NextTab).to_string(),
            Some(DockKey::PrevTab) => tr(lang, DockText::PrevTab).to_string(),
            Some(DockKey::RestoreLayout) => tr(lang, DockText::RestoreLayout).to_string(),
            Some(DockKey::ToggleLock) => tr(lang, DockText::LockLayout).to_string(),
            Some(DockKey::Move(s)) => {
                format!("{} {}", tr(lang, DockText::MoveTo), side_word(lang, s))
            }
            Some(DockKey::Panel(id)) => self.core.title(&id).to_string(),
            None => name.to_string(),
        }
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let lang = self.core.lang;
        for w in self.core.take_warnings() {
            self.banner.push(w.text(lang));
        }
        if !self.banner.is_empty() {
            let mut close = false;
            egui::Area::new(Id::unique("eh3d_dock_banner"))
                .order(Order::Foreground)
                .anchor(Align2::CENTER_TOP, [0.0, 6.0])
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        for b in &self.banner {
                            ui.label(b.as_str());
                        }
                        if ui.button(tr(lang, DockText::Close)).clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                self.banner.clear();
            }
        }
        if self.show_about {
            let mut open = true;
            let about = self.core.about.clone();
            let brand = self.brand;
            egui::Window::new(tr(lang, DockText::About))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if let Some(brand) = &brand {
                        about_contents(ui, lang, &about, brand);
                        return;
                    }
                    ui.heading(about.app_name.as_str());
                    ui.label(format!(
                        "{}: {}",
                        tr(lang, DockText::Version),
                        about.version
                    ));
                    ui.label(format!(
                        "{}: {}",
                        tr(lang, DockText::License),
                        about.license
                    ));
                    for l in &about.extra {
                        ui.label(l.as_str());
                    }
                    ui.separator();
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(tr(lang, DockText::Trademarks))
                                .small()
                                .weak(),
                        )
                        .wrap(),
                    );
                });
            self.show_about = open;
        }
        if self.show_shortcuts {
            let mut open = true;
            egui::Window::new(tr(lang, DockText::Shortcuts))
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("eh3d_dock_shortcut_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                for (menu, label, key) in &self.shortcut_rows {
                                    ui.weak(menu.as_str());
                                    ui.label(label.as_str());
                                    ui.monospace(key.as_str());
                                    ui.end_row();
                                }
                            });
                    });
                });
            self.show_shortcuts = open;
        }
        if let Some(mut name) = self.ask_name.take() {
            let mut keep = true;
            let mut accepted = false;
            egui::Window::new(tr(lang, DockText::SaveWorkspace))
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(tr(lang, DockText::WorkspaceName));
                    let edit = ui.text_edit_singleline(&mut name);
                    if self.name_error {
                        ui.colored_label(ui.visuals().error_fg_color, tr(lang, DockText::WarnKeys));
                    }
                    ui.horizontal(|ui| {
                        let ok = valid_name(name.trim());
                        if ui
                            .add_enabled(ok, egui::Button::new(tr(lang, DockText::Accept)))
                            .clicked()
                            || (ok
                                && edit.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            accepted = true;
                        }
                        if ui.button(tr(lang, DockText::Cancel)).clicked() {
                            keep = false;
                        }
                    });
                });
            if accepted {
                let n = name.trim().to_string();
                match self.apply(DockAction::SaveWorkspaceAs(n)) {
                    Ok(()) => keep = false,
                    Err(_) => self.name_error = true,
                }
            }
            if keep && self.ask_name.is_none() {
                self.ask_name = Some(name);
            }
        }
    }
}

fn draw_items<A: Clone>(ui: &mut Ui, items: &[Resolved<A>], chosen: &mut Option<Chosen<A>>) {
    for r in items {
        match &r.kind {
            Kind::Separator => {
                ui.separator();
            }
            Kind::Note => {
                ui.add_enabled(false, egui::Label::new(r.label.as_str()));
            }
            Kind::Submenu(sub) => {
                ui.menu_button(r.label.as_str(), |ui| draw_items(ui, sub, chosen));
            }
            Kind::Action(_) | Kind::Inert => {
                let mut b = match r.checked {
                    Some(on) => egui::Button::selectable(on, r.label.as_str()),
                    None => egui::Button::new(r.label.as_str()),
                };
                if let Some(s) = &r.shortcut {
                    b = b.shortcut_text(s.to_string());
                }
                let enabled = matches!(r.kind, Kind::Action(_));
                if ui.add_enabled(enabled, b).clicked() {
                    if let Some(c) = r.activate() {
                        *chosen = Some(c.clone());
                        ui.close();
                    }
                }
            }
        }
    }
}

/// The modifiers and the key egui uses for a shortcut of the frame.
pub fn egui_shortcut(s: &Shortcut) -> Option<(egui::Modifiers, egui::Key)> {
    let key = egui::Key::from_name(&s.key)?;
    let mut m = egui::Modifiers::NONE;
    if s.ctrl {
        m = m.plus(egui::Modifiers::COMMAND);
    }
    if s.alt {
        m = m.plus(egui::Modifiers::ALT);
    }
    if s.shift {
        m = m.plus(egui::Modifiers::SHIFT);
    }
    Some((m, key))
}

/// The width of the contents of the window about a program of the family.
const ABOUT_WIDTH: f32 = 440.0;
/// The side of the square that holds the ring and the picture.
const ABOUT_RING: f32 = 160.0;
/// The largest side of a picture that is read, in pixels.
const ABOUT_LOGO_MAX: u32 = 1024;

/// The pixels of a PNG, or nothing when it cannot be read or is larger than [`ABOUT_LOGO_MAX`].
fn decode_logo(bytes: &[u8]) -> Option<egui::ColorImage> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (width, height) = (reader.info().width, reader.info().height);
    if width == 0 || height == 0 || width > ABOUT_LOGO_MAX || height > ABOUT_LOGO_MAX {
        return None;
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let data = buffer.get(..frame.buffer_size())?;
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => return None,
    };
    let size = [width as usize, height as usize];
    (rgba.len() == size[0] * size[1] * 4)
        .then(|| egui::ColorImage::from_rgba_unmultiplied(size, &rgba))
}

/// The texture of the picture of a program, made once and kept with the window.
fn logo_texture(ctx: &egui::Context, brand: &AboutBrand) -> Option<egui::TextureHandle> {
    let id = Id::unique(("eh3d_dock_about_logo", brand.program));
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let image = decode_logo(brand.logo_png)?;
    let texture = ctx.load_texture(brand.program.name(), image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    Some(texture)
}

/// Draws what the window about a program of the PHOENIX family holds, into `ui`: the picture inside a ring of the accent colour, the name of
/// the program in large letters, one sentence about it, the version, the author, the e-mail, the page of the author, the repository and the
/// licence (the addresses can be clicked), the notices of data of others when the program has them and the notice about trademarks. A program
/// that has its own window calls this inside it; [`Dock::set_brand`] does it for the window of the frame.
pub fn about_contents(ui: &mut Ui, lang: Lang, info: &AboutInfo, brand: &AboutBrand) {
    let [r, g, b] = brand.program.accent();
    let accent = Color32::from_rgb(r, g, b);
    ui.set_width(ABOUT_WIDTH);
    ui.vertical_centered(|ui| {
        let (square, _) = ui.allocate_exact_size(Vec2::splat(ABOUT_RING), Sense::hover());
        let centre = square.center();
        let painter = ui.painter();
        painter.circle_filled(centre, 78.0, Color32::from_rgba_unmultiplied(r, g, b, 30));
        painter.circle_stroke(centre, 76.0, Stroke::new(4.0, accent));
        painter.circle_stroke(
            centre,
            70.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 130)),
        );
        if let Some(texture) = logo_texture(ui.ctx(), brand) {
            let side = if brand.program.round_logo() {
                134.0
            } else {
                96.0
            };
            egui::Image::new(&texture)
                .paint_at(ui, egui::Rect::from_center_size(centre, Vec2::splat(side)));
        }
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(info.app_name.as_str())
                .size(28.0)
                .strong()
                .color(accent),
        );
        ui.add_space(4.0);
        ui.add(egui::Label::new(brand.program.summary().get(lang)).wrap());
        for line in &info.extra {
            ui.add(egui::Label::new(egui::RichText::new(line.as_str()).weak()).wrap());
        }
    });
    ui.add_space(6.0);
    ui.separator();
    egui::Grid::new("eh3d_dock_about_rows")
        .num_columns(2)
        .spacing([14.0, 4.0])
        .show(ui, |ui| {
            for row in about::rows(lang, brand.program, info) {
                ui.label(egui::RichText::new(row.label).strong());
                match &row.url {
                    Some(url) => {
                        ui.hyperlink_to(row.value.as_str(), url);
                    }
                    None => {
                        ui.label(row.value.as_str());
                    }
                }
                ui.end_row();
            }
        });
    let notices = brand.program.notices();
    if !notices.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(tr(lang, DockText::ThirdParty)).strong());
        for notice in notices {
            ui.add(egui::Label::new(egui::RichText::new(*notice).small()).wrap());
        }
    }
    ui.separator();
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr(lang, DockText::Trademarks))
                .small()
                .weak(),
        )
        .wrap(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::about::Phoenix;
    use crate::menu::StdAction;
    use crate::sectors::Sector;
    use crate::stdmenus::{standard_bar, HelpOptions};
    use std::cell::Cell;
    use std::rc::Rc;

    struct Harness {
        ctx: egui::Context,
        dock: Dock,
        t: f64,
        calls: Rc<Cell<u32>>,
        chosen: Vec<Chosen<StdAction>>,
    }

    impl Harness {
        fn new(sector: Sector) -> Harness {
            let mut dock = Dock::new(sector.panels(), sector.workspaces());
            let calls = Rc::new(Cell::new(0));
            for d in sector.panels() {
                let c = calls.clone();
                dock.register(&d.id, move |ui| {
                    c.set(c.get() + 1);
                    ui.label("body");
                });
            }
            Harness {
                ctx: egui::Context::default(),
                dock,
                t: 0.0,
                calls,
                chosen: Vec::new(),
            }
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            self.t += 0.05;
            let bar = standard_bar::<StdAction>(HelpOptions::default());
            let mut got = None;
            let dock = &mut self.dock;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1400.0, 900.0),
                )),
                time: Some(self.t),
                events,
                ..Default::default()
            };
            let out = self.ctx.run_ui(input, |ui| {
                egui::Panel::top("menu").show(ui, |ui| {
                    if let Some(c) = dock.menu_bar(ui, &bar) {
                        got = Some(c);
                    }
                });
                dock.show(ui);
            });
            out.drop_without_applying_deltas();
            self.chosen.extend(got);
        }

        fn frames(&mut self, n: usize) {
            for _ in 0..n {
                self.frame(Vec::new());
            }
        }

        fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
            self.frame(vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers,
            }]);
        }

        fn pointer(&mut self, pos: Pos2, pressed: Option<bool>) {
            let mut ev = vec![egui::Event::PointerMoved(pos)];
            if let Some(p) = pressed {
                ev.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: p,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            self.frame(ev);
        }

        fn drag(&mut self, from: Pos2, to: Pos2) {
            self.pointer(from, None);
            self.pointer(from, Some(true));
            self.pointer(from + Vec2::new(12.0, 6.0), None);
            for step in 1..=4 {
                self.pointer(from + (to - from) * (step as f32 / 4.0), None);
            }
            self.pointer(to, None);
            self.pointer(to, Some(false));
            self.frames(2);
        }
    }

    #[test]
    fn every_sector_draws_its_panels_without_changing_the_layout() {
        for s in Sector::ALL {
            let mut h = Harness::new(s);
            let before = h.dock.layout_text();
            h.frames(3);
            assert!(
                h.calls.get() >= 3,
                "{}: the bodies of the panels on show were drawn",
                s.name()
            );
            assert_eq!(h.dock.layout_text(), before, "{}", s.name());
            assert!(!h.dock.last_slots().is_empty());
        }
    }

    #[test]
    fn a_menu_shortcut_gives_its_action_and_a_disabled_one_does_not() {
        let mut h = Harness::new(Sector::Pcb);
        h.frames(2);
        h.key(egui::Key::S, egui::Modifiers::COMMAND);
        assert_eq!(h.chosen, vec![Chosen::App(StdAction::Save)]);
        h.key(
            egui::Key::S,
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
        );
        assert_eq!(h.chosen[1], Chosen::App(StdAction::SaveAs));
        h.dock.set_menu_state(vec!["file.save".into()], Vec::new());
        h.frames(1);
        h.key(egui::Key::S, egui::Modifiers::COMMAND);
        assert_eq!(h.chosen.len(), 2, "a disabled entry gives nothing");
    }

    #[test]
    fn the_keys_of_the_frame_act_on_the_layout() {
        let mut h = Harness::new(Sector::Cnc);
        h.frames(2);
        h.key(egui::Key::M, egui::Modifiers::COMMAND);
        assert!(h.dock.core().layout().maximized.is_some());
        h.frames(2);
        assert!(h.calls.get() > 0);
        h.key(egui::Key::M, egui::Modifiers::COMMAND);
        assert!(h.dock.core().layout().maximized.is_none());
        let before = h.dock.layout_text();
        h.key(egui::Key::PageDown, egui::Modifiers::COMMAND);
        h.frames(1);
        assert!(h.dock.core().layout_is_sound());
        assert_eq!(before.lines().count(), h.dock.layout_text().lines().count());
    }

    #[test]
    fn floating_folded_and_opened_panels_draw() {
        let mut h = Harness::new(Sector::Pnp);
        h.frames(2);
        h.dock
            .apply(DockAction::FloatPanel("camera".into()))
            .unwrap();
        h.dock
            .apply(DockAction::AutoHidePanel("console".into(), Side::Bottom))
            .unwrap();
        h.dock
            .apply(DockAction::AutoHidePanel("parts".into(), Side::Left))
            .unwrap();
        h.frames(3);
        assert!(h.dock.core().layout().floating.len() == 1);
        h.dock.overlay = Some("console".into());
        let n = h.calls.get();
        h.frames(2);
        assert!(h.calls.get() > n);
        h.dock
            .apply(DockAction::SwitchWorkspace("Run".into()))
            .unwrap();
        h.frames(2);
        assert!(h.dock.core().layout_is_sound());
    }

    #[test]
    fn the_dialogs_open_and_draw() {
        let mut h = Harness::new(Sector::Laser);
        h.dock.set_about(AboutInfo {
            app_name: "Test".into(),
            version: "1.0".into(),
            license: "GPL".into(),
            extra: vec!["x".into()],
        });
        h.frames(2);
        h.dock.apply(DockAction::ShowAbout).unwrap();
        h.dock.apply(DockAction::ShowShortcuts).unwrap();
        h.dock.apply(DockAction::AskSaveWorkspace).unwrap();
        h.dock.load_config("dock.version=9\n");
        h.frames(3);
        assert!(h.dock.show_about && h.dock.show_shortcuts && h.dock.ask_name.is_some());
        assert!(!h.dock.shortcut_rows.is_empty());
        assert!(h.dock.banner.is_empty() || !h.dock.banner[0].is_empty());
    }

    fn small_png(color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 2, 1);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
        writer.finish().unwrap();
        out
    }

    #[test]
    fn the_picture_of_a_program_is_read_and_a_broken_one_is_left_out() {
        let rgba = small_png(png::ColorType::Rgba, &[1, 2, 3, 255, 4, 5, 6, 255]);
        let image = decode_logo(&rgba).expect("a picture");
        assert_eq!(image.size, [2, 1]);
        assert_eq!(image.pixels[1], Color32::from_rgb(4, 5, 6));
        let rgb = small_png(png::ColorType::Rgb, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(decode_logo(&rgb).expect("a picture").size, [2, 1]);
        let grey = small_png(png::ColorType::Grayscale, &[7, 9]);
        assert_eq!(
            decode_logo(&grey).expect("a picture").pixels[1],
            Color32::from_rgb(9, 9, 9)
        );
        assert!(decode_logo(b"not a picture").is_none());
        assert!(decode_logo(&rgba[..rgba.len() / 2]).is_none());
        assert!(decode_logo(&[]).is_none());
    }

    #[test]
    fn the_window_about_each_program_draws_in_every_language_with_and_without_its_picture() {
        let picture: &'static [u8] = Box::leak(
            small_png(png::ColorType::Rgba, &[1, 2, 3, 255, 4, 5, 6, 255]).into_boxed_slice(),
        );
        for program in Phoenix::ALL {
            for logo_png in [picture, &b"broken"[..]] {
                let mut h = Harness::new(Sector::Laser);
                h.dock.set_about(program.info("1.2.3"));
                h.dock.set_brand(AboutBrand { program, logo_png });
                h.dock.apply(DockAction::ShowAbout).unwrap();
                for lang in Lang::ALL {
                    h.dock.set_language(lang);
                    h.frames(2);
                }
                assert!(h.dock.show_about);
                let id = Id::unique(("eh3d_dock_about_logo", program));
                let kept = h
                    .ctx
                    .data(|d| d.get_temp::<egui::TextureHandle>(id))
                    .is_some();
                assert_eq!(kept, logo_png == picture, "{program:?}");
            }
        }
    }

    #[test]
    fn a_tab_dragged_to_the_side_of_another_group_moves() {
        let mut h = Harness::new(Sector::Pcb);
        h.frames(3);
        let slots = h.dock.last_slots().to_vec();
        let from = &slots[0];
        let tab = from.tabs[0];
        let to = slots
            .iter()
            .find(|s| s.panels != from.panels && s.rect.w > 200.0 && s.rect.h > 200.0)
            .expect("a second big group");
        let before = h.dock.layout_text();
        let start = Pos2::new(tab.x + tab.w / 2.0, tab.y + tab.h / 2.0);
        let end = Pos2::new(to.rect.x + 14.0, to.rect.y + to.rect.h / 2.0);
        h.drag(start, end);
        assert_ne!(h.dock.layout_text(), before, "the layout changed");
        assert!(h.dock.core().layout_is_sound());
        assert!(h.dock.drag.is_none());
    }

    #[test]
    fn a_locked_layout_cannot_be_rearranged_by_dragging() {
        let mut h = Harness::new(Sector::Pcb);
        h.frames(3);
        h.dock.apply(DockAction::ToggleLock).unwrap();
        h.frames(2);
        let slots = h.dock.last_slots().to_vec();
        let tab = slots[0].tabs[0];
        let to = &slots[slots.len() - 1];
        let before = h.dock.layout_text();
        h.drag(
            Pos2::new(tab.x + tab.w / 2.0, tab.y + tab.h / 2.0),
            Pos2::new(to.rect.x + 14.0, to.rect.y + to.rect.h / 2.0),
        );
        assert_eq!(h.dock.layout_text(), before);
    }

    #[test]
    fn a_divider_can_be_dragged() {
        let mut h = Harness::new(Sector::Pcb);
        h.frames(3);
        let slots = h.dock.last_slots().to_vec();
        let a = &slots[0];
        let before = h.dock.layout_text();
        let y = a.rect.y + a.rect.h / 2.0;
        let x = a.rect.x + a.rect.w + GAP / 2.0;
        h.drag(Pos2::new(x, y), Pos2::new(x + 80.0, y));
        assert_ne!(
            h.dock.layout_text(),
            before,
            "the share of the division changed"
        );
        assert!(h.dock.core().layout_is_sound());
    }

    #[test]
    fn the_shortcut_conversion_covers_the_keys_of_the_frame() {
        for (_, s) in crate::shortcuts::DockKeys::defaults().named() {
            assert!(egui_shortcut(&s).is_some(), "{s}");
        }
        for t in [
            "Ctrl+S",
            "Ctrl+Shift+S",
            "F1",
            "Ctrl+Comma",
            "Alt+Shift+Left",
            "Ctrl+PageDown",
            "Ctrl+0",
        ] {
            assert!(egui_shortcut(&Shortcut::parse(t).unwrap()).is_some(), "{t}");
        }
    }
}
