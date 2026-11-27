//! The panel of the settings of the server drawn with egui (feature `gui`). All the logic is in [`crate::server_panel`]; this only draws the boxes and
//! gives back the [`Action`] the person chose, which the program hands to [`ServerPanel::handle`] together with its client.
//!
//! ```ignore
//! panel.refresh(&client);
//! if let Some(action) = eh3d_view::server_gui::show(ui, &mut panel, lang) {
//!     if let Some(settings) = panel.handle(action, &mut client) {
//!         // save `settings` to the machine file and make a new client with them
//!     }
//! }
//! ```

use crate::server_panel::{Action, Field, LinkBadge, ServerPanel, TokenState, Tone};
use crate::server_texts::{tr, ServerText};
use crate::texts::Lang;

fn tone_color(tone: Tone) -> egui::Color32 {
    match tone {
        Tone::Good => egui::Color32::from_rgb(60, 170, 90),
        Tone::Neutral => egui::Color32::from_rgb(150, 150, 150),
        Tone::Warning => egui::Color32::from_rgb(220, 160, 40),
        Tone::Bad => egui::Color32::from_rgb(210, 70, 60),
    }
}

const PROBLEM: egui::Color32 = egui::Color32::from_rgb(210, 70, 60);
const WARNING: egui::Color32 = egui::Color32::from_rgb(220, 160, 40);

/// The width of the boxes of the form, of the short boxes and of the boxes of the login window.
const FIELD: f32 = 260.0;
const SHORT_FIELD: f32 = 60.0;
const LOGIN_FIELD: f32 = 160.0;
/// What a box takes beyond its text (its margins).
const BOX_EDGE: f32 = 8.0;
/// The narrowest column the panel is laid out for.
const NARROWEST: f32 = 80.0;

/// The width of a text written on one line.
fn text_width(ui: &egui::Ui, text: &str, style: egui::TextStyle) -> f32 {
    let font = style.resolve(ui.style());
    ui.painter()
        .layout_no_wrap(text.to_string(), font, egui::Color32::WHITE)
        .size()
        .x
}

fn button_width(ui: &egui::Ui, text: &str) -> f32 {
    text_width(ui, text, egui::TextStyle::Button) + 2.0 * ui.spacing().button_padding.x
}

/// A button of a row that wraps: it goes whole to the next row, and only a button wider than the column writes its words on two lines.
fn button(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    let mode = if button_width(ui, text) <= ui.max_rect().width() {
        egui::TextWrapMode::Extend
    } else {
        egui::TextWrapMode::Wrap
    };
    ui.add_enabled(enabled, egui::Button::new(text).wrap_mode(mode))
}

/// A text box with its label: beside it where both fit, under it in a narrow column.
fn labelled_box(ui: &mut egui::Ui, label: &str, value: &mut String, secret: bool, wide: f32) {
    let beside = text_width(ui, label, egui::TextStyle::Body)
        + ui.spacing().item_spacing.x
        + wide
        + BOX_EDGE
        <= ui.available_width();
    let draw = |ui: &mut egui::Ui| {
        ui.label(label);
        ui.add(
            egui::TextEdit::singleline(value)
                .password(secret)
                .desired_width(wide),
        );
    };
    if beside {
        ui.horizontal(draw);
    } else {
        let mut draw = draw;
        draw(ui);
    }
}

/// The box of the token with what is said about the saved one.
fn token_box(ui: &mut egui::Ui, panel: &mut ServerPanel, lang: Lang, action: &mut Option<Action>) {
    ui.add(
        egui::TextEdit::singleline(&mut panel.form.token_input)
            .password(true)
            .desired_width(FIELD),
    );
    let note = match panel.form.token_state() {
        TokenState::Kept => Some(ServerText::TokenKept),
        TokenState::WillRemove => Some(ServerText::TokenWillRemove),
        TokenState::Empty => Some(ServerText::TokenNone),
        TokenState::Replaced => None,
    };
    if let Some(note) = note {
        ui.label(tr(lang, note));
    }
    if panel.form.token_state() == TokenState::Kept
        && ui.button(tr(lang, ServerText::TokenRemove)).clicked()
    {
        *action = Some(Action::RemoveToken);
    }
}

/// A box of the form, as wide as the form wants or as the column allows.
fn text_box(ui: &mut egui::Ui, value: &mut String, wide: f32) {
    ui.add(egui::TextEdit::singleline(value).desired_width(wide));
}

/// The boxes of the settings: two columns (labels and boxes) where they fit, else each label over its box.
fn form(ui: &mut egui::Ui, panel: &mut ServerPanel, lang: Lang, action: &mut Option<Action>) {
    let labels = [
        Field::Address,
        Field::Token,
        Field::Machine,
        Field::TimeoutS,
        Field::Retries,
    ]
    .into_iter()
    .map(|f| text_width(ui, tr(lang, f.label()), egui::TextStyle::Body))
    .fold(0.0, f32::max);
    let spacing = 8.0;
    if labels + spacing + FIELD + BOX_EDGE <= ui.available_width() {
        egui::Grid::new(ui.scope_id().with("eh3d_server_form"))
            .num_columns(2)
            .spacing([spacing, 6.0])
            .show(ui, |ui| {
                ui.label(tr(lang, Field::Address.label()));
                text_box(ui, &mut panel.form.address, FIELD);
                ui.end_row();

                ui.label(tr(lang, Field::Token.label()));
                // the words under a box wrap inside the width of the boxes
                ui.vertical(|ui| {
                    ui.set_max_width(FIELD + BOX_EDGE);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    token_box(ui, panel, lang, action);
                });
                ui.end_row();

                ui.label(tr(lang, Field::Machine.label()));
                text_box(ui, &mut panel.form.machine, FIELD);
                ui.end_row();

                ui.label(tr(lang, Field::TimeoutS.label()));
                text_box(ui, &mut panel.form.timeout_s, SHORT_FIELD);
                ui.end_row();

                ui.label(tr(lang, Field::Retries.label()));
                text_box(ui, &mut panel.form.retries, SHORT_FIELD);
                ui.end_row();

                ui.label("");
                ui.vertical(|ui| {
                    ui.set_max_width(FIELD + BOX_EDGE);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    ui.checkbox(&mut panel.form.mdns, tr(lang, Field::Mdns.label()));
                });
                ui.end_row();
            });
    } else {
        ui.label(tr(lang, Field::Address.label()));
        text_box(ui, &mut panel.form.address, FIELD);
        ui.label(tr(lang, Field::Token.label()));
        token_box(ui, panel, lang, action);
        ui.label(tr(lang, Field::Machine.label()));
        text_box(ui, &mut panel.form.machine, FIELD);
        let timeout = tr(lang, Field::TimeoutS.label());
        labelled_box(ui, timeout, &mut panel.form.timeout_s, false, SHORT_FIELD);
        let retries = tr(lang, Field::Retries.label());
        labelled_box(ui, retries, &mut panel.form.retries, false, SHORT_FIELD);
        ui.checkbox(&mut panel.form.mdns, tr(lang, Field::Mdns.label()));
    }
}

/// The indicator of the link for the corner of any window: a dot of the colour of the state and its word, with the longer text (and what the person can
/// do about it) on hover. It reads nothing and sends nothing; give it the badge of the panel (`panel.badge()`) or of a tracker.
pub fn link_badge(ui: &mut egui::Ui, badge: LinkBadge, lang: Lang) -> egui::Response {
    let inner = ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 4.5, tone_color(badge.tone()));
        ui.label(badge.label(lang));
    });
    inner.response.on_hover_text(badge.tooltip(lang))
}

/// The width the table of the sessions needs.
fn sessions_width(ui: &egui::Ui, panel: &ServerPanel, lang: Lang) -> f32 {
    let wide = |text: &str| text_width(ui, text, egui::TextStyle::Body);
    let mut columns = [
        ServerText::LoginUser,
        ServerText::SessionApp,
        ServerText::SessionRole,
        ServerText::SessionAddress,
    ]
    .map(|head| wide(tr(lang, head)));
    let mut last: f32 = 0.0;
    for row in panel.sessions.rows() {
        for (column, text) in columns
            .iter_mut()
            .zip([&row.name, &row.app, &row.role, &row.ip])
        {
            *column = column.max(wide(text));
        }
        let mut end = button_width(ui, tr(lang, ServerText::SessionClose));
        if row.current {
            end += wide(tr(lang, ServerText::SessionThis)) + ui.spacing().item_spacing.x;
        }
        last = last.max(end);
    }
    columns.iter().sum::<f32>() + last + 4.0 * 8.0
}

/// The table of the open sessions.
fn sessions_table(ui: &mut egui::Ui, panel: &ServerPanel, lang: Lang, ask: &mut Option<String>) {
    egui::Grid::new(ui.scope_id().with("eh3d_server_sessions"))
        .num_columns(5)
        .striped(true)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.strong(tr(lang, ServerText::LoginUser));
            ui.strong(tr(lang, ServerText::SessionApp));
            ui.strong(tr(lang, ServerText::SessionRole));
            ui.strong(tr(lang, ServerText::SessionAddress));
            ui.label("");
            ui.end_row();
            for row in panel.sessions.rows() {
                ui.label(row.name.as_str());
                ui.label(row.app.as_str());
                ui.label(row.role.as_str());
                ui.label(row.ip.as_str());
                ui.horizontal(|ui| {
                    if row.current {
                        ui.strong(tr(lang, ServerText::SessionThis));
                    }
                    if ui.button(tr(lang, ServerText::SessionClose)).clicked() {
                        *ask = Some(row.id.clone());
                    }
                });
                ui.end_row();
            }
        });
}

/// The same sessions in a narrow column: each one in rows of its own.
fn sessions_list(ui: &mut egui::Ui, panel: &ServerPanel, lang: Lang, ask: &mut Option<String>) {
    for row in panel.sessions.rows() {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.strong(row.name.as_str());
            ui.label(row.app.as_str());
            ui.label(row.role.as_str());
            ui.label(row.ip.as_str());
            if row.current {
                ui.strong(tr(lang, ServerText::SessionThis));
            }
            if button(ui, true, tr(lang, ServerText::SessionClose)).clicked() {
                *ask = Some(row.id.clone());
            }
        });
    }
}

/// The list of open sessions with a button to close each one; a click on the session in use asks "are you sure" first. Gives the action through
/// `action`; the close button marks the session in the panel and sends `Action::CloseSession`.
fn show_sessions(
    ui: &mut egui::Ui,
    panel: &mut ServerPanel,
    lang: Lang,
    action: &mut Option<Action>,
) {
    let mut ask: Option<String> = None;
    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong(tr(lang, ServerText::Sessions));
            if button(ui, true, tr(lang, ServerText::SessionsRefresh)).clicked() {
                *action = Some(Action::RefreshSessions);
            }
            if button(ui, true, tr(lang, ServerText::SessionsHide)).clicked() {
                *action = Some(Action::HideSessions);
            }
        });
        if let Some(error) = panel.sessions.error_text(lang) {
            ui.colored_label(PROBLEM, error);
        }
        if panel.sessions.is_loaded() && panel.sessions.rows().is_empty() {
            ui.label(tr(lang, ServerText::SessionsNone));
        }
        if !panel.sessions.rows().is_empty() {
            if sessions_width(ui, panel, lang) <= ui.available_width() {
                sessions_table(ui, panel, lang, &mut ask);
            } else {
                sessions_list(ui, panel, lang, &mut ask);
            }
        }
        if panel.sessions.is_confirming() {
            ui.colored_label(WARNING, tr(lang, ServerText::SessionCloseOwnWarning));
            ui.horizontal_wrapped(|ui| {
                if button(ui, true, tr(lang, ServerText::SessionConfirm)).clicked() {
                    *action = Some(Action::ConfirmCloseSession);
                }
                if button(ui, true, tr(lang, ServerText::LoginCancel)).clicked() {
                    *action = Some(Action::CancelCloseSession);
                }
            });
        }
    });
    if let Some(id) = ask {
        panel.sessions.ask_close(&id);
        *action = Some(Action::CloseSession);
    }
}

/// Draws the panel and gives the action the person chose this frame, if any. It keeps to the width of its column: what does not fit beside goes
/// to the next row.
pub fn show(ui: &mut egui::Ui, panel: &mut ServerPanel, lang: Lang) -> Option<Action> {
    // what is seen of the column: something drawn above may have made it wider than the window
    let seen = ui.clip_rect().right() - ui.cursor().left();
    let width = ui.available_width().min(seen).max(NARROWEST);
    ui.vertical(|ui| {
        ui.set_max_width(width);
        contents(ui, panel, lang)
    })
    .inner
}

fn contents(ui: &mut egui::Ui, panel: &mut ServerPanel, lang: Lang) -> Option<Action> {
    let mut action = None;
    ui.heading(tr(lang, ServerText::Title));

    // the state of the link and who is logged in
    let state = panel.state();
    ui.horizontal_wrapped(|ui| {
        link_badge(ui, LinkBadge::new(state), lang);
        if let Some(who) = panel.who() {
            ui.label(format!("{} {}", tr(lang, ServerText::LoggedInAs), who.name));
        }
    });
    ui.add_space(4.0);

    // the boxes
    form(ui, panel, lang, &mut action);

    // what is wrong with the boxes
    for issue in panel.form.issues() {
        ui.colored_label(PROBLEM, issue.text(lang));
    }
    if panel.form.is_dirty() {
        ui.label(tr(lang, ServerText::NotApplied));
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        let can_apply = panel.form.is_valid() && panel.form.is_dirty();
        if button(ui, can_apply, tr(lang, ServerText::Apply)).clicked() {
            action = Some(Action::Apply);
        }
        if button(ui, panel.form.is_dirty(), tr(lang, ServerText::Undo)).clicked() {
            action = Some(Action::Undo);
        }
        if button(ui, true, tr(lang, ServerText::TestConnection)).clicked() {
            action = Some(Action::TestConnection);
        }
        if panel.who().is_some() {
            let (key, next) = if panel.sessions.open {
                (ServerText::SessionsHide, Action::HideSessions)
            } else {
                (ServerText::Sessions, Action::OpenSessions)
            };
            if button(ui, true, tr(lang, key)).clicked() {
                action = Some(next);
            }
            if button(ui, true, tr(lang, ServerText::Logout)).clicked() {
                action = Some(Action::Logout);
            }
        } else if button(ui, true, tr(lang, ServerText::Login)).clicked() {
            action = Some(Action::OpenLogin);
        }
    });

    let notice = panel.notice_text(lang);
    if !notice.is_empty() {
        ui.label(notice);
    }

    // the open sessions, for a person who is logged in
    if panel.sessions.open && panel.who().is_some() {
        ui.add_space(6.0);
        show_sessions(ui, panel, lang, &mut action);
    }

    // the login window, in the same panel
    if panel.login.open {
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.label(tr(lang, ServerText::Login));
            let user = tr(lang, ServerText::LoginUser);
            labelled_box(ui, user, &mut panel.login.name, false, LOGIN_FIELD);
            let secret = tr(lang, ServerText::LoginPassword);
            labelled_box(ui, secret, &mut panel.login.password, true, LOGIN_FIELD);
            if let Some(error) = panel.login.error_text(lang) {
                ui.colored_label(PROBLEM, error);
            }
            ui.horizontal_wrapped(|ui| {
                let ready = panel.login.can_submit();
                if button(ui, ready, tr(lang, ServerText::Login)).clicked() {
                    action = Some(Action::SubmitLogin);
                }
                if button(ui, true, tr(lang, ServerText::LoginCancel)).clicked() {
                    action = Some(Action::CancelLogin);
                }
            });
        });
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_serverclient as sc;
    use sc::sim::SimServer;
    use sc::{Client, Settings};

    const SCREEN: f32 = 1400.0;

    fn settings() -> Settings {
        let mut s = Settings::default();
        s.set("server_address", "192.168.0.180:18200").unwrap();
        s.set("server_machine", "bungard").unwrap();
        s.set("server_token", "kept-value").unwrap();
        s
    }

    fn client(sim: &SimServer) -> Client {
        Client::new(settings(), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()))
            .with_clock(Box::new(sim.clock()))
    }

    /// A panel with everything on show: a saved token, a changed form with something wrong, the login window.
    fn full_panel() -> ServerPanel {
        let mut panel = ServerPanel::new(&settings(), "phoenix-pnp");
        panel.form.retries = "99".to_string();
        panel.login.open = true;
        panel
    }

    /// A logged in panel with two sessions and the question about closing the one in use.
    fn sessions_panel(sim: &SimServer) -> ServerPanel {
        let mut c = client(sim);
        let mut other = client(sim);
        other.login("admin", "secret-password", "android").unwrap();
        let mut panel = ServerPanel::new(&settings(), "phoenix-pnp");
        panel.login.name = "admin".to_string();
        panel.login.password = "secret-password".to_string();
        panel.handle(Action::SubmitLogin, &mut c);
        panel.handle(Action::OpenSessions, &mut c);
        let own = panel.sessions.rows()[0].id.clone();
        panel.sessions.ask_close(&own);
        panel.handle(Action::CloseSession, &mut c);
        assert!(panel.sessions.is_confirming());
        panel
    }

    /// Every text of a shape with the place where it is seen: what a text box holds scrolls inside the box, whose clip cuts it.
    fn texts_of(shape: &egui::Shape, clip: egui::Rect, out: &mut Vec<(String, egui::Rect)>) {
        match shape {
            // an empty label has no place of its own
            egui::Shape::Text(t) if !t.galley.text().is_empty() => {
                let seen = shape.visual_bounding_rect().intersect(clip);
                out.push((t.galley.text().to_string(), seen));
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| texts_of(s, clip, out)),
            _ => {}
        }
    }

    /// Draws the panel in a column `width` wide of a wider window: what was written and where, and how far right anything was drawn.
    fn draw(panel: &mut ServerPanel, lang: Lang, width: f32) -> (Vec<(String, egui::Rect)>, f32) {
        let ctx = egui::Context::default();
        let column = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 3000.0));
        let mut texts = Vec::new();
        let mut right: f32 = 0.0;
        for frame in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN, 3000.0),
                )),
                time: Some(f64::from(frame) * 0.05),
                ..Default::default()
            };
            let mut used = 0.0;
            let out = ctx.run_ui(input, |ui| {
                let inner = ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                    show(ui, panel, lang);
                });
                used = inner.response.rect.right();
            });
            texts.clear();
            right = used;
            for clipped in &out.shapes {
                texts_of(&clipped.shape, clipped.clip_rect, &mut texts);
                // the background of the window is as wide as the screen and is not part of the panel
                let r = clipped
                    .shape
                    .visual_bounding_rect()
                    .intersect(clipped.clip_rect);
                if r.is_finite() && r.is_positive() && r.width() < SCREEN - 1.0 {
                    right = right.max(r.right());
                }
            }
            out.drop_without_applying_deltas();
        }
        (texts, right)
    }

    fn has(texts: &[(String, egui::Rect)], wanted: &str) -> bool {
        texts.iter().any(|(t, _)| t == wanted)
    }

    fn assert_inside(texts: &[(String, egui::Rect)], right: f32, width: f32, what: &str) {
        for (text, r) in texts {
            assert!(
                r.right() <= width + 2.0 && r.left() >= -2.0,
                "{what}: {text:?} is cut at {r:?}"
            );
        }
        assert!(right <= width + 2.0, "{what}: drawn up to {right}");
    }

    #[test]
    fn the_form_and_the_login_fit_every_column_in_the_eight_languages() {
        for lang in Lang::ALL {
            for width in [180.0, 220.0, 280.0, 340.0, 420.0, 520.0, 800.0] {
                let mut panel = full_panel();
                let (texts, right) = draw(&mut panel, lang, width);
                let what = format!("{lang:?} {width}");
                assert_inside(&texts, right, width, &what);
                for field in Field::ALL {
                    assert!(has(&texts, tr(lang, field.label())), "{what} {field:?}");
                }
                for key in [
                    ServerText::TokenKept,
                    ServerText::TokenRemove,
                    ServerText::Apply,
                    ServerText::Undo,
                    ServerText::TestConnection,
                    ServerText::LoginUser,
                    ServerText::LoginPassword,
                    ServerText::LoginCancel,
                    ServerText::NotApplied,
                ] {
                    assert!(has(&texts, tr(lang, key)), "{what} {key:?}");
                }
            }
        }
    }

    #[test]
    fn the_sessions_fit_every_column_in_the_eight_languages() {
        let sim = SimServer::new();
        let mut panel = sessions_panel(&sim);
        for lang in Lang::ALL {
            for width in [180.0, 220.0, 280.0, 340.0, 520.0, 900.0] {
                let (texts, right) = draw(&mut panel, lang, width);
                let what = format!("{lang:?} {width}");
                assert_inside(&texts, right, width, &what);
                for key in [
                    ServerText::Logout,
                    ServerText::SessionClose,
                    ServerText::SessionThis,
                    ServerText::SessionCloseOwnWarning,
                    ServerText::SessionConfirm,
                ] {
                    assert!(has(&texts, tr(lang, key)), "{what} {key:?}");
                }
                assert!(has(&texts, "android"), "{what}");
            }
        }
    }

    /// How many rows the words of a text took in a column `width` wide.
    fn rows_of(panel: &mut ServerPanel, lang: Lang, width: f32, wanted: &str) -> usize {
        fn find(shape: &egui::Shape, wanted: &str) -> Option<usize> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == wanted => Some(t.galley.rows.len()),
                egui::Shape::Vec(v) => v.iter().find_map(|s| find(s, wanted)),
                _ => None,
            }
        }
        let ctx = egui::Context::default();
        let column = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 3000.0));
        let mut rows = 0;
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN, 3000.0),
                )),
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                ui.scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                    show(ui, panel, lang);
                });
            });
            rows = out
                .shapes
                .iter()
                .find_map(|c| find(&c.shape, wanted))
                .expect("the text was drawn");
            out.drop_without_applying_deltas();
        }
        rows
    }

    #[test]
    fn the_long_words_of_the_check_take_more_rows_in_a_narrower_column() {
        for lang in Lang::ALL {
            let mut panel = full_panel();
            let label = tr(lang, Field::Mdns.label());
            let wide = rows_of(&mut panel, lang, 800.0, label);
            let narrow = rows_of(&mut panel, lang, 100.0, label);
            assert!(narrow > wide, "{lang:?}: {narrow} {wide}");
        }
    }

    #[test]
    fn a_wide_column_keeps_the_check_in_the_column_of_the_boxes() {
        for lang in Lang::ALL {
            let mut panel = full_panel();
            let of = |texts: &[(String, egui::Rect)], field: Field| {
                let label = tr(lang, field.label());
                texts
                    .iter()
                    .find(|(t, _)| t == label)
                    .map(|(_, r)| *r)
                    .expect("the label")
            };
            let wide = draw(&mut panel, lang, 800.0).0;
            assert!(
                of(&wide, Field::Mdns).left() > of(&wide, Field::Address).right(),
                "{lang:?}"
            );
            // in a narrow column it is a row of its own, from the left edge
            let narrow = draw(&mut panel, lang, 220.0).0;
            assert!(of(&narrow, Field::Mdns).left() < 60.0, "{lang:?}");
        }
    }
}
