//! The colours every EH3D program shares, so the suite looks like one product.
//!
//! The values are the dark industrial theme of the suite: calm surfaces, one cyan accent and
//! colours for states that stay distinguishable in a dim workshop.

use serde::{Deserialize, Serialize};

/// An opaque colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// `#rrggbb`.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    /// Reads `#rrggbb` (or `rrggbb`).
    pub fn from_hex(text: &str) -> Option<Self> {
        let digits = text.strip_prefix('#').unwrap_or(text);
        if digits.len() != 6 || !digits.is_ascii() {
            return None;
        }
        let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&digits[range], 16).ok();
        Some(Self(byte(0..2)?, byte(2..4)?, byte(4..6)?))
    }

    /// The relative luminance (WCAG), from 0.0 (black) to 1.0 (white).
    pub fn luminance(self) -> f64 {
        let channel = |value: u8| {
            let c = f64::from(value) / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(self.0) + 0.7152 * channel(self.1) + 0.0722 * channel(self.2)
    }

    /// The WCAG contrast ratio against `other`, from 1.0 to 21.0.
    pub fn contrast(self, other: Self) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        let (light, dark) = if a > b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }
}

/// The background of windows.
pub const BACKGROUND: Rgb = Rgb(0x14, 0x17, 0x1c);
/// The background of panels over the window.
pub const SURFACE: Rgb = Rgb(0x1e, 0x23, 0x2b);
/// Text.
pub const TEXT: Rgb = Rgb(0xe6, 0xea, 0xef);
/// Less important text.
pub const TEXT_MUTED: Rgb = Rgb(0x9a, 0xa5, 0xb1);
/// The accent colour of the suite.
pub const ACCENT: Rgb = Rgb(0x00, 0xe5, 0xff);
/// Something is fine.
pub const OK: Rgb = Rgb(0x3d, 0xd6, 0x8c);
/// Something needs attention.
pub const WARNING: Rgb = Rgb(0xff, 0xb0, 0x20);
/// Something is wrong or dangerous.
pub const DANGER: Rgb = Rgb(0xff, 0x4d, 0x5e);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_and_bad_text_is_refused() {
        assert_eq!(ACCENT.to_hex(), "#00e5ff");
        assert_eq!(Rgb::from_hex("#00E5FF"), Some(ACCENT));
        assert_eq!(Rgb::from_hex("00e5ff"), Some(ACCENT));
        for bad in ["", "#12345", "#1234567", "#gg0000", "éééééé"] {
            assert_eq!(Rgb::from_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn contrast_follows_wcag() {
        let black = Rgb(0, 0, 0);
        let white = Rgb(255, 255, 255);
        assert!((black.contrast(white) - 21.0).abs() < 1e-6);
        assert!((white.contrast(white) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn text_and_states_are_readable_on_the_dark_surfaces() {
        for surface in [BACKGROUND, SURFACE] {
            for foreground in [TEXT, TEXT_MUTED, ACCENT, OK, WARNING, DANGER] {
                assert!(
                    foreground.contrast(surface) >= 4.5,
                    "{} on {}",
                    foreground.to_hex(),
                    surface.to_hex()
                );
            }
        }
    }
}
