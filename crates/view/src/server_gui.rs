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
        ui.horizontal(|ui| {
            ui.strong(tr(lang, ServerText::Sessions));
            if ui.button(tr(lang, ServerText::SessionsRefresh)).clicked() {
                *action = Some(Action::RefreshSessions);
            }
            if ui.button(tr(lang, ServerText::SessionsHide)).clicked() {
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
            egui::Grid::new(ui.id().with("eh3d_server_sessions"))
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
                                ask = Some(row.id.clone());
                            }
                        });
                        ui.end_row();
                    }
                });
        }
        if panel.sessions.is_confirming() {
            ui.colored_label(WARNING, tr(lang, ServerText::SessionCloseOwnWarning));
            ui.horizontal(|ui| {
                if ui.button(tr(lang, ServerText::SessionConfirm)).clicked() {
                    *action = Some(Action::ConfirmCloseSession);
                }
                if ui.button(tr(lang, ServerText::LoginCancel)).clicked() {
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

/// Draws the panel and gives the action the person chose this frame, if any.
pub fn show(ui: &mut egui::Ui, panel: &mut ServerPanel, lang: Lang) -> Option<Action> {
    let mut action = None;
    ui.heading(tr(lang, ServerText::Title));

    // the state of the link and who is logged in
    let state = panel.state();
    ui.horizontal(|ui| {
        link_badge(ui, LinkBadge::new(state), lang);
        if let Some(who) = panel.who() {
            ui.label(format!("{} {}", tr(lang, ServerText::LoggedInAs), who.name));
        }
    });
    ui.add_space(4.0);

    // the boxes
    egui::Grid::new(ui.id().with("eh3d_server_form"))
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr(lang, Field::Address.label()));
            ui.add(egui::TextEdit::singleline(&mut panel.form.address).desired_width(260.0));
            ui.end_row();

            ui.label(tr(lang, Field::Token.label()));
            ui.vertical(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut panel.form.token_input)
                        .password(true)
                        .desired_width(260.0),
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
                    action = Some(Action::RemoveToken);
                }
            });
            ui.end_row();

            ui.label(tr(lang, Field::Machine.label()));
            ui.add(egui::TextEdit::singleline(&mut panel.form.machine).desired_width(260.0));
            ui.end_row();

            ui.label(tr(lang, Field::TimeoutS.label()));
            ui.add(egui::TextEdit::singleline(&mut panel.form.timeout_s).desired_width(60.0));
            ui.end_row();

            ui.label(tr(lang, Field::Retries.label()));
            ui.add(egui::TextEdit::singleline(&mut panel.form.retries).desired_width(60.0));
            ui.end_row();

            ui.label("");
            ui.checkbox(&mut panel.form.mdns, tr(lang, Field::Mdns.label()));
            ui.end_row();
        });

    // what is wrong with the boxes
    for issue in panel.form.issues() {
        ui.colored_label(PROBLEM, issue.text(lang));
    }
    if panel.form.is_dirty() {
        ui.label(tr(lang, ServerText::NotApplied));
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let can_apply = panel.form.is_valid() && panel.form.is_dirty();
        if ui
            .add_enabled(can_apply, egui::Button::new(tr(lang, ServerText::Apply)))
            .clicked()
        {
            action = Some(Action::Apply);
        }
        if ui
            .add_enabled(
                panel.form.is_dirty(),
                egui::Button::new(tr(lang, ServerText::Undo)),
            )
            .clicked()
        {
            action = Some(Action::Undo);
        }
        if ui.button(tr(lang, ServerText::TestConnection)).clicked() {
            action = Some(Action::TestConnection);
        }
        if panel.who().is_some() {
            let (key, next) = if panel.sessions.open {
                (ServerText::SessionsHide, Action::HideSessions)
            } else {
                (ServerText::Sessions, Action::OpenSessions)
            };
            if ui.button(tr(lang, key)).clicked() {
                action = Some(next);
            }
            if ui.button(tr(lang, ServerText::Logout)).clicked() {
                action = Some(Action::Logout);
            }
        } else if ui.button(tr(lang, ServerText::Login)).clicked() {
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
            ui.horizontal(|ui| {
                ui.label(tr(lang, ServerText::LoginUser));
                ui.add(egui::TextEdit::singleline(&mut panel.login.name).desired_width(160.0));
            });
            ui.horizontal(|ui| {
                ui.label(tr(lang, ServerText::LoginPassword));
                ui.add(
                    egui::TextEdit::singleline(&mut panel.login.password)
                        .password(true)
                        .desired_width(160.0),
                );
            });
            if let Some(error) = panel.login.error_text(lang) {
                ui.colored_label(PROBLEM, error);
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        panel.login.can_submit(),
                        egui::Button::new(tr(lang, ServerText::Login)),
                    )
                    .clicked()
                {
                    action = Some(Action::SubmitLogin);
                }
                if ui.button(tr(lang, ServerText::LoginCancel)).clicked() {
                    action = Some(Action::CancelLogin);
                }
            });
        });
    }
    action
}
