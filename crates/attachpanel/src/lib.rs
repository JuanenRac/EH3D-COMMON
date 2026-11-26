//! The panel «Attachments» that every PHOENIX program mounts: the list of the attachments of its `.eh3d` project, attach a file or a
//! link, link to a component, extract and remove with a question first, and the viewer beside the list.
//!
//! * [`model`] holds and does everything, with no window, so it is tested without a screen.
//! * [`gui`] (feature `gui`) draws it with egui, with its own window to choose files and folders.
//! * [`panel_def`] is the entry of the panel for the frame of the program (`eh3d-dock`).
//!
//! In a program (see `docs/ATTACHMENTS_PANEL.md`):
//!
//! ```rust,ignore
//! // among the panels of the frame
//! defs.push(eh3d_attachpanel::panel_def());
//! // kept for as long as the program runs
//! let attachments = AttachPanel::new(Program::new("cnc", env!("CARGO_PKG_VERSION")));
//! // in the body of the panel, on every frame
//! if attachments.show(ui, project_path, lang_code) {
//!     // the project was written: read your own state of it again
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "gui")]
pub mod gui;
pub mod model;
pub mod source;
pub mod texts;

#[cfg(feature = "gui")]
pub use gui::AttachPanel;
pub use model::{Item, LinkForm, Message, Model, Piece, Program, Request};
pub use source::ProjectSource;

use eh3d_dock::core::PanelDef;
use eh3d_dock::texts::Text8;
use texts::Text;

/// The name of the panel (what a settings file stores, so it never changes).
pub const PANEL_ID: &str = "attachments";

/// The title of the panel for the frame, whose order of the languages is English, Spanish, Italian, German, French, Portuguese,
/// Chinese and Japanese.
pub fn panel_title() -> Text8 {
    let [en, es, de, fr, it, ja, zh, pt] = Text::Title.words();
    Text8([en, es, it, de, fr, pt, zh, ja])
}

/// The panel as the frame of a program takes it.
pub fn panel_def() -> PanelDef {
    PanelDef::new(PANEL_ID, panel_title())
        .icon("A")
        .min(260.0, 200.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texts::Lang;

    #[test]
    fn the_panel_has_its_title_in_every_language_of_the_frame() {
        let def = panel_def();
        assert_eq!(def.id, PANEL_ID);
        for lang in Lang::ALL {
            assert_eq!(def.title.get(lang), texts::tr(lang, Text::Title), "{lang:?}");
        }
    }
}
