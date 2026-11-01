//! Which entries of the context menu of a text box can be used, and which of the keys goes to the text and which to the drawing.
//! Nothing here draws or touches the clipboard: the programs ask their window for that.

/// What the text box under the pointer allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxState {
    /// The box has some text in it.
    pub has_text: bool,
    /// Part of the text is selected.
    pub has_selection: bool,
    /// The box shows text that cannot be changed.
    pub read_only: bool,
    /// The clipboard holds text (true when that cannot be known).
    pub clipboard_has_text: bool,
}

/// The entries of the context menu that are enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuState {
    pub cut: bool,
    pub copy: bool,
    pub paste: bool,
    pub select_all: bool,
    pub undo: bool,
}

/// Cut and Copy need a selection, Cut and Paste and Undo need a box that can be changed, Paste needs something on the clipboard,
/// Select all needs some text.
pub fn menu_state(b: BoxState) -> MenuState {
    MenuState {
        cut: b.has_selection && !b.read_only,
        copy: b.has_selection,
        paste: !b.read_only && b.clipboard_has_text,
        select_all: b.has_text,
        undo: !b.read_only,
    }
}

/// Who gets a clipboard key (Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+A, Delete): the text box when it has the keyboard, the drawing otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOwner {
    Text,
    Drawing,
}

pub fn key_owner(text_wants_keyboard: bool) -> KeyOwner {
    if text_wants_keyboard {
        KeyOwner::Text
    } else {
        KeyOwner::Drawing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(has_text: bool, has_selection: bool, read_only: bool, clip: bool) -> BoxState {
        BoxState {
            has_text,
            has_selection,
            read_only,
            clipboard_has_text: clip,
        }
    }

    #[test]
    fn an_empty_box_only_pastes() {
        let m = menu_state(b(false, false, false, true));
        assert!(!m.cut && !m.copy && !m.select_all);
        assert!(m.paste && m.undo);
    }

    #[test]
    fn a_selection_can_be_cut_and_copied() {
        let m = menu_state(b(true, true, false, true));
        assert!(m.cut && m.copy && m.paste && m.select_all);
    }

    #[test]
    fn a_box_that_cannot_be_changed_only_copies_and_selects() {
        let m = menu_state(b(true, true, true, true));
        assert!(m.copy && m.select_all);
        assert!(!m.cut && !m.paste && !m.undo);
    }

    #[test]
    fn an_empty_clipboard_cannot_be_pasted() {
        assert!(!menu_state(b(true, false, false, false)).paste);
    }

    #[test]
    fn the_text_wins_the_keys_when_it_has_the_keyboard() {
        assert_eq!(key_owner(true), KeyOwner::Text);
        assert_eq!(key_owner(false), KeyOwner::Drawing);
    }
}
