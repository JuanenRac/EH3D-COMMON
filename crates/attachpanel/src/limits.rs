//! The limits of the panel, taken from the settings file of a program: `attachments.max_mb` and `viewer.max_*_mb`. A program
//! reads its file, keeps one [`PanelLimits`] and gives it to the panel; it has no window, so it is tested without a screen.

use eh3d_project::AttachmentLimits;
use eh3d_viewer::ViewerLimits;

/// The limits of attaching and of showing an attachment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelLimits {
    /// What may be attached or taken out (`attachments.max_mb`).
    pub attachments: AttachmentLimits,
    /// What is shown inside (`viewer.max_attachment_mb`, `viewer.max_pdf_mb`...).
    pub viewer: ViewerLimits,
}

impl PanelLimits {
    /// Whether the key of a settings line belongs to the panel: it starts with `attachments.` or `viewer.`.
    pub fn is_key(key: &str) -> bool {
        let key = key.trim();
        key.starts_with("attachments.") || key.starts_with("viewer.")
    }

    /// Takes one line of a settings file. False for a key that is not of the panel or a value that is not a whole number from
    /// 1 to 4096; the limit then stays as it was.
    pub fn apply(&mut self, key: &str, value: &str) -> bool {
        self.attachments.apply(key, value) || self.viewer.apply(key, value)
    }

    /// The limits that the `key = value` lines of a settings file give; every other line is left alone, and a value that is
    /// not valid keeps the default.
    pub fn from_text(text: &str) -> PanelLimits {
        let mut limits = PanelLimits::default();
        for line in text.trim_start_matches('\u{feff}').lines() {
            if let Some((key, value)) = line.split_once('=') {
                limits.apply(key, value);
            }
        }
        limits
    }
}

/// What a drop of files means for the panel: nothing (the program takes the file), or take it, `Some(true)` when the pointer
/// is known to be over the panel and `Some(false)` when the system did not say where it is, so the person is asked.
///
/// `shown` is whether the panel was drawn on the last frame with a project, `over` is whether the pointer is over it (nothing
/// when the system does not tell while a file is dragged) and `program_opens` whether the program itself opens such a file.
pub fn drop_for_panel(shown: bool, over: Option<bool>, program_opens: bool) -> Option<bool> {
    if !shown {
        return None;
    }
    match over {
        Some(true) => Some(true),
        Some(false) => None,
        None => (!program_opens).then_some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_of_a_settings_file_give_the_limits_and_a_bad_one_keeps_the_default() {
        let text = "\u{feff}attachments.max_mb = 16\nlang = es\nviewer.max_pdf_mb=8\nviewer.max_text_mb = no\n# a note\n";
        let limits = PanelLimits::from_text(text);
        assert_eq!(limits.attachments.max_attachment_bytes, 16 << 20);
        assert_eq!(limits.viewer.max_pdf_bytes, 8 << 20);
        let usual = PanelLimits::default();
        assert_eq!(limits.viewer.max_text_bytes, usual.viewer.max_text_bytes);
        assert_eq!(PanelLimits::from_text("travel = 1\n"), usual);
        assert!(PanelLimits::is_key(" attachments.max_mb "));
        assert!(PanelLimits::is_key("viewer.other"));
        assert!(!PanelLimits::is_key("viewer_open"));
        let mut limits = PanelLimits::default();
        assert!(!limits.apply("attachments.max_mb", "0"));
        assert!(!limits.apply("viewer.other", "4"));
        assert_eq!(limits, usual);
    }

    #[test]
    fn a_drop_goes_to_the_panel_over_it_or_when_nobody_else_wants_the_file() {
        assert_eq!(drop_for_panel(false, Some(true), false), None);
        assert_eq!(drop_for_panel(true, Some(true), true), Some(true));
        assert_eq!(drop_for_panel(true, Some(true), false), Some(true));
        assert_eq!(drop_for_panel(true, Some(false), false), None);
        assert_eq!(drop_for_panel(true, None, false), Some(false));
        assert_eq!(drop_for_panel(true, None, true), None);
    }
}
