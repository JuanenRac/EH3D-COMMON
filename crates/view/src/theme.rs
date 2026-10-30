//! The themes of the modern interface. A theme is data: the colours, how the corners of the cards are cut and whether the accents glow.
//! There are ten made ones (Forge, the default; Neon Arcade; Glass Pro; the classic Windows grey; Midnight; Paper, a light one; Forest;
//! Retro; Ocean; High contrast) and one that follows the colours of Windows (light or dark and the accent colour the user chose there). A user can write their own in a small text file.

/// The colour type of the suite.
pub use eh3d_common::theme::Rgb;

/// How the corners of the cards look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    Square,
    /// A corner cut at 45 degrees.
    Cut,
    Round,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Forge,
    Neon,
    Glass,
    WindowsClassic,
    /// Follows the colours of Windows.
    System,
    Midnight,
    Paper,
    Forest,
    Retro,
    Ocean,
    Contrast,
}

impl ThemeId {
    pub const ALL: [ThemeId; 11] = [
        ThemeId::Forge,
        ThemeId::Neon,
        ThemeId::Glass,
        ThemeId::WindowsClassic,
        ThemeId::System,
        ThemeId::Midnight,
        ThemeId::Paper,
        ThemeId::Forest,
        ThemeId::Retro,
        ThemeId::Ocean,
        ThemeId::Contrast,
    ];

    /// The word of the configuration file (`theme = forge`).
    pub fn key(self) -> &'static str {
        match self {
            ThemeId::Forge => "forge",
            ThemeId::Neon => "neon",
            ThemeId::Glass => "glass",
            ThemeId::WindowsClassic => "windows-classic",
            ThemeId::System => "system",
            ThemeId::Midnight => "midnight",
            ThemeId::Paper => "paper",
            ThemeId::Forest => "forest",
            ThemeId::Retro => "retro",
            ThemeId::Ocean => "ocean",
            ThemeId::Contrast => "contrast",
        }
    }

    pub fn from_key(key: &str) -> Option<ThemeId> {
        let k = key.trim().to_ascii_lowercase();
        ThemeId::ALL.into_iter().find(|t| t.key() == k)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub id: ThemeId,
    pub background: (Rgb, Rgb),
    pub panel: Rgb,
    pub panel_raised: Rgb,
    pub line: Rgb,
    pub text: Rgb,
    pub dim: Rgb,
    pub accent: Rgb,
    pub accent2: Rgb,
    pub ok: Rgb,
    pub warn: Rgb,
    pub danger: Rgb,
    pub table: Rgb,
    pub grid: Rgb,
    pub board: Rgb,
    /// The colours of the drill sizes, from the smallest; with more sizes they start again.
    pub sizes: [Rgb; 4],
    pub corner: Corner,
    pub glow: bool,
}

fn c(hex: &str) -> Rgb {
    Rgb::from_hex(hex).expect("a colour written in this file")
}

impl Theme {
    /// Industrial amber on charcoal: the default.
    pub fn forge() -> Theme {
        Theme {
            id: ThemeId::Forge,
            background: (c("#0f0d0b"), c("#1d1814")),
            panel: c("#171310"),
            panel_raised: c("#221c17"),
            line: c("#ffb000"),
            text: c("#f4ece0"),
            dim: c("#9c8d7a"),
            accent: c("#ffb000"),
            accent2: c("#ff5a1f"),
            ok: c("#9be34a"),
            warn: c("#ffb000"),
            danger: c("#ff5470"),
            table: c("#14110e"),
            grid: c("#2b241d"),
            board: c("#1d2b1f"),
            sizes: [c("#ffb000"), c("#9be34a"), c("#4cc9f0"), c("#ff5a1f")],
            corner: Corner::Cut,
            glow: false,
        }
    }

    pub fn neon() -> Theme {
        Theme {
            id: ThemeId::Neon,
            background: (c("#070b1a"), c("#101a3a")),
            panel: c("#0d1430"),
            panel_raised: c("#121c44"),
            line: c("#27d6ff"),
            text: c("#e8f1ff"),
            dim: c("#7f8fb8"),
            accent: c("#27d6ff"),
            accent2: c("#ff3cac"),
            ok: c("#3dffb0"),
            warn: c("#ffd23c"),
            danger: c("#ff5470"),
            table: c("#0a1230"),
            grid: c("#1a2a5c"),
            board: c("#0f3a2e"),
            sizes: [c("#27d6ff"), c("#3dffb0"), c("#ffd23c"), c("#ff3cac")],
            corner: Corner::Round,
            glow: true,
        }
    }

    pub fn glass() -> Theme {
        Theme {
            id: ThemeId::Glass,
            background: (c("#0e1624"), c("#1b2b45")),
            panel: c("#16233a"),
            panel_raised: c("#1e3050"),
            line: c("#4fd1c5"),
            text: c("#eef4fb"),
            dim: c("#8aa0bd"),
            accent: c("#4fd1c5"),
            accent2: c("#7c8cff"),
            ok: c("#6ee7a8"),
            warn: c("#fbbf24"),
            danger: c("#f87171"),
            table: c("#12203a"),
            grid: c("#22385c"),
            board: c("#1a4a46"),
            sizes: [c("#4fd1c5"), c("#6ee7a8"), c("#fbbf24"), c("#f472b6")],
            corner: Corner::Round,
            glow: false,
        }
    }

    /// The grey of the old Windows: flat, square, navy title.
    pub fn windows_classic() -> Theme {
        Theme {
            id: ThemeId::WindowsClassic,
            background: (c("#c0c0c0"), c("#c0c0c0")),
            panel: c("#d4d0c8"),
            panel_raised: c("#e4e0d8"),
            line: c("#404040"),
            text: c("#000000"),
            dim: c("#404040"),
            accent: c("#000080"),
            accent2: c("#008080"),
            ok: c("#006400"),
            warn: c("#805000"),
            danger: c("#a00000"),
            table: c("#ffffff"),
            grid: c("#c8c8c8"),
            board: c("#7fbf7f"),
            sizes: [c("#000080"), c("#006400"), c("#805000"), c("#a00000")],
            corner: Corner::Square,
            glow: false,
        }
    }

    /// A theme from the colours of Windows: light or dark, and the accent colour the user chose (the standard blue when it is unknown).
    pub fn from_system_values(light: bool, accent: Option<Rgb>) -> Theme {
        let accent = accent.unwrap_or(c("#0078d4"));
        // on a light surface the accent is used as it is; on a dark one it is made lighter so that it can be read
        let readable = if light { accent } else { accent.lighten(0.25) };
        let mut t = if light {
            Theme {
                background: (c("#f3f3f3"), c("#e9e9e9")),
                panel: c("#ffffff"),
                panel_raised: c("#f7f7f7"),
                text: c("#1a1a1a"),
                dim: c("#555555"),
                table: c("#fafafa"),
                grid: c("#d8d8d8"),
                board: c("#bfe3c8"),
                ok: c("#0f7b3f"),
                warn: c("#8a5a00"),
                danger: c("#b3261e"),
                ..Theme::glass()
            }
        } else {
            Theme {
                background: (c("#1c1c1c"), c("#242424")),
                panel: c("#2b2b2b"),
                panel_raised: c("#333333"),
                text: c("#f3f3f3"),
                dim: c("#b0b0b0"),
                table: c("#202020"),
                grid: c("#3a3a3a"),
                board: c("#27493a"),
                ..Theme::glass()
            }
        };
        t.id = ThemeId::System;
        t.line = readable;
        t.accent = readable;
        t.accent2 = readable.lighten(0.35);
        t.sizes = [readable, t.ok, t.warn, t.danger];
        t.corner = Corner::Round;
        t.glow = false;
        t
    }

    /// Deep indigo with a violet accent.
    pub fn midnight() -> Theme {
        Theme {
            id: ThemeId::Midnight,
            background: (c("#0b0b1a"), c("#16163a")),
            panel: c("#11112b"),
            panel_raised: c("#1a1a40"),
            line: c("#8b7bff"),
            text: c("#ececff"),
            dim: c("#9a9ac8"),
            accent: c("#8b7bff"),
            accent2: c("#ff7bd0"),
            ok: c("#5be3a0"),
            warn: c("#ffd36b"),
            danger: c("#ff6b8b"),
            table: c("#0c0c26"),
            grid: c("#232356"),
            board: c("#14263a"),
            sizes: [c("#8b7bff"), c("#5be3a0"), c("#ffd36b"), c("#ff7bd0")],
            corner: Corner::Round,
            glow: true,
        }
    }

    /// A light theme: dark ink on warm paper, with a blue accent.
    pub fn paper() -> Theme {
        Theme {
            id: ThemeId::Paper,
            background: (c("#f4f1ea"), c("#e8e3d6")),
            panel: c("#fbf9f4"),
            panel_raised: c("#ffffff"),
            line: c("#2a5db0"),
            text: c("#1d2330"),
            dim: c("#5b6372"),
            accent: c("#2a5db0"),
            accent2: c("#c2410c"),
            ok: c("#15803d"),
            warn: c("#b45309"),
            danger: c("#b91c1c"),
            table: c("#efece3"),
            grid: c("#d6d0c0"),
            board: c("#cfe3d0"),
            sizes: [c("#2a5db0"), c("#15803d"), c("#b45309"), c("#c2410c")],
            corner: Corner::Square,
            glow: false,
        }
    }

    /// Dark green with leaf-green accents.
    pub fn forest() -> Theme {
        Theme {
            id: ThemeId::Forest,
            background: (c("#0a120d"), c("#14241a")),
            panel: c("#101c14"),
            panel_raised: c("#172a1d"),
            line: c("#6bd18a"),
            text: c("#e6f4ea"),
            dim: c("#8fae9b"),
            accent: c("#6bd18a"),
            accent2: c("#e3c14a"),
            ok: c("#9be34a"),
            warn: c("#e3c14a"),
            danger: c("#ff6b6b"),
            table: c("#0c1710"),
            grid: c("#1f3526"),
            board: c("#17331f"),
            sizes: [c("#6bd18a"), c("#e3c14a"), c("#4cc9f0"), c("#ff8a5c")],
            corner: Corner::Cut,
            glow: false,
        }
    }

    /// An amber terminal on black.
    pub fn retro() -> Theme {
        Theme {
            id: ThemeId::Retro,
            background: (c("#000000"), c("#0a0800")),
            panel: c("#0a0800"),
            panel_raised: c("#141000"),
            line: c("#ffb000"),
            text: c("#ffcf5a"),
            dim: c("#b8861f"),
            accent: c("#ffb000"),
            accent2: c("#ff7a00"),
            ok: c("#ffd24a"),
            warn: c("#ffb000"),
            danger: c("#ff4d2e"),
            table: c("#050400"),
            grid: c("#2a1f00"),
            board: c("#1a1400"),
            sizes: [c("#ffb000"), c("#ffd24a"), c("#ff7a00"), c("#ffe9a8")],
            corner: Corner::Square,
            glow: true,
        }
    }

    /// Deep blue-green with a teal accent.
    pub fn ocean() -> Theme {
        Theme {
            id: ThemeId::Ocean,
            background: (c("#06141c"), c("#0c2a38")),
            panel: c("#0b1f2b"),
            panel_raised: c("#123246"),
            line: c("#35c4d8"),
            text: c("#e4f6fb"),
            dim: c("#86abba"),
            accent: c("#35c4d8"),
            accent2: c("#ff9f5a"),
            ok: c("#5be3b0"),
            warn: c("#ffd36b"),
            danger: c("#ff6b7a"),
            table: c("#081a24"),
            grid: c("#17384a"),
            board: c("#0f3a3a"),
            sizes: [c("#35c4d8"), c("#5be3b0"), c("#ffd36b"), c("#ff9f5a")],
            corner: Corner::Round,
            glow: false,
        }
    }

    /// High contrast: white on black with yellow accents.
    pub fn contrast() -> Theme {
        Theme {
            id: ThemeId::Contrast,
            background: (c("#000000"), c("#000000")),
            panel: c("#000000"),
            panel_raised: c("#1a1a1a"),
            line: c("#ffffff"),
            text: c("#ffffff"),
            dim: c("#c8c8c8"),
            accent: c("#ffee00"),
            accent2: c("#00e5ff"),
            ok: c("#00ff6a"),
            warn: c("#ffee00"),
            danger: c("#ff4040"),
            table: c("#000000"),
            grid: c("#4a4a4a"),
            board: c("#002b00"),
            sizes: [c("#ffee00"), c("#00e5ff"), c("#00ff6a"), c("#ff4040")],
            corner: Corner::Square,
            glow: false,
        }
    }

    /// Whether the theme is a light one (dark ink on a light panel), for the toolkits that have a light and a dark base.
    pub fn is_light(&self) -> bool {
        self.panel.luminance() > 0.5
    }

    /// The theme for the colours Windows is using now. Where there is no Windows, a dark one with the standard accent.
    pub fn system() -> Theme {
        let (light, accent) = read_windows_colours();
        Theme::from_system_values(light, accent)
    }

    pub fn for_id(id: ThemeId) -> Theme {
        match id {
            ThemeId::Forge => Theme::forge(),
            ThemeId::Neon => Theme::neon(),
            ThemeId::Glass => Theme::glass(),
            ThemeId::WindowsClassic => Theme::windows_classic(),
            ThemeId::System => Theme::system(),
            ThemeId::Midnight => Theme::midnight(),
            ThemeId::Paper => Theme::paper(),
            ThemeId::Forest => Theme::forest(),
            ThemeId::Retro => Theme::retro(),
            ThemeId::Ocean => Theme::ocean(),
            ThemeId::Contrast => Theme::contrast(),
        }
    }

    /// A theme of the user: a text of `key = value` lines (`#` starts a comment). `base` names the theme it starts from; the colours
    /// are `#rrggbb`; `corner` is square, cut or round; `glow` is on or off. An unknown key or a wrong value is an error.
    pub fn from_text(text: &str) -> Result<Theme, String> {
        let mut t = Theme::forge();
        // the base first, wherever it is written
        for line in text.lines() {
            if let Some((k, v)) = split(line) {
                if k == "base" {
                    let id = ThemeId::from_key(&v).ok_or(format!("base: unknown theme '{v}'"))?;
                    t = Theme::for_id(id);
                }
            }
        }
        for (n, line) in text.lines().enumerate() {
            let Some((k, v)) = split(line) else { continue };
            let color = |v: &str| {
                Rgb::from_hex(v).ok_or(format!("line {}: '{v}' is not a colour (#rrggbb)", n + 1))
            };
            match k.as_str() {
                "base" => {}
                "background" => t.background = (color(&v)?, t.background.1),
                "background2" => t.background.1 = color(&v)?,
                "panel" => t.panel = color(&v)?,
                "panel_raised" => t.panel_raised = color(&v)?,
                "line" => t.line = color(&v)?,
                "text" => t.text = color(&v)?,
                "dim" => t.dim = color(&v)?,
                "accent" => t.accent = color(&v)?,
                "accent2" => t.accent2 = color(&v)?,
                "ok" => t.ok = color(&v)?,
                "warn" => t.warn = color(&v)?,
                "danger" => t.danger = color(&v)?,
                "table" => t.table = color(&v)?,
                "grid" => t.grid = color(&v)?,
                "board" => t.board = color(&v)?,
                "size1" => t.sizes[0] = color(&v)?,
                "size2" => t.sizes[1] = color(&v)?,
                "size3" => t.sizes[2] = color(&v)?,
                "size4" => t.sizes[3] = color(&v)?,
                "corner" => {
                    t.corner = match v.as_str() {
                        "square" => Corner::Square,
                        "cut" => Corner::Cut,
                        "round" => Corner::Round,
                        _ => {
                            return Err(format!(
                                "line {}: corner must be square, cut or round",
                                n + 1
                            ))
                        }
                    }
                }
                "glow" => {
                    t.glow = match v.as_str() {
                        "on" | "yes" | "true" => true,
                        "off" | "no" | "false" => false,
                        _ => return Err(format!("line {}: glow must be on or off", n + 1)),
                    }
                }
                other => return Err(format!("line {}: unknown setting '{other}'", n + 1)),
            }
        }
        Ok(t)
    }

    /// The colour of a drill size by its place in the legend.
    pub fn size_color(&self, index: usize) -> Rgb {
        self.sizes[index % self.sizes.len()]
    }
}

fn split(line: &str) -> Option<(String, String)> {
    // a colour starts with '#', so the comment is a '#' that begins the line or follows a space after the value
    let raw = line.trim();
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    let (k, v) = raw.split_once('=')?;
    let v = v.trim();
    let v = match v.find(" #") {
        Some(i) => &v[..i],
        None => v,
    };
    Some((k.trim().to_ascii_lowercase(), v.trim().to_ascii_lowercase()))
}

/// The value of a `REG_DWORD` in the output of `reg query` (`    AppsUseLightTheme    REG_DWORD    0x1`).
pub fn parse_reg_dword(output: &str, name: &str) -> Option<u32> {
    for line in output.lines() {
        let mut it = line.split_whitespace();
        if it.next() == Some(name) && it.next() == Some("REG_DWORD") {
            let v = it.next()?;
            return u32::from_str_radix(v.trim_start_matches("0x"), 16).ok();
        }
    }
    None
}

/// The accent colour of Windows from the `AccentColor` value of DWM, which is stored as 0xAABBGGRR.
pub fn accent_from_dword(v: u32) -> Rgb {
    Rgb(
        (v & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        ((v >> 16) & 0xff) as u8,
    )
}

#[cfg(windows)]
fn read_windows_colours() -> (bool, Option<Rgb>) {
    use std::process::Command;
    let query = |key: &str, name: &str| -> Option<u32> {
        let out = Command::new("reg")
            .args(["query", key, "/v", name])
            .output()
            .ok()?;
        parse_reg_dword(&String::from_utf8_lossy(&out.stdout), name)
    };
    let light = query(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        "AppsUseLightTheme",
    )
    .map(|v| v != 0)
    .unwrap_or(false);
    let accent =
        query(r"HKCU\Software\Microsoft\Windows\DWM", "AccentColor").map(accent_from_dword);
    (light, accent)
}

#[cfg(not(windows))]
fn read_windows_colours() -> (bool, Option<Rgb>) {
    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Theme> {
        vec![
            Theme::forge(),
            Theme::neon(),
            Theme::glass(),
            Theme::windows_classic(),
            Theme::from_system_values(false, Some(Rgb(0, 120, 212))),
            Theme::from_system_values(true, Some(Rgb(0, 120, 212))),
            Theme::from_system_values(false, None),
        ]
    }

    #[test]
    fn colours_are_written_and_read_back() {
        assert_eq!(Rgb::from_hex("#ffb000"), Some(Rgb(255, 176, 0)));
        assert_eq!(Rgb::from_hex("FFB000"), Some(Rgb(255, 176, 0)));
        assert_eq!(Rgb::from_hex("#ffb00"), None);
        assert_eq!(Rgb::from_hex("#gggggg"), None);
        assert_eq!(Rgb(255, 176, 0).to_hex(), "#ffb000");
        assert!((Rgb(0, 0, 0).contrast(Rgb(255, 255, 255)) - 21.0).abs() < 1e-9);
    }

    #[test]
    fn every_theme_can_be_read() {
        for t in all() {
            // the text on the cards and the dim text are readable, and so are the accents against the background
            assert!(t.text.contrast(t.panel) >= 7.0, "{:?} text", t.id);
            assert!(t.dim.contrast(t.panel) >= 4.5, "{:?} dim", t.id);
            assert!(t.accent.contrast(t.panel) >= 3.0, "{:?} accent", t.id);
            for (i, s) in t.sizes.iter().enumerate() {
                assert!(
                    s.contrast(t.board) >= 1.5 || s.contrast(t.table) >= 3.0,
                    "{:?} size {i}",
                    t.id
                );
            }
        }
    }

    #[test]
    fn every_theme_has_a_name_in_the_file_and_come_back() {
        for id in ThemeId::ALL {
            assert_eq!(ThemeId::from_key(id.key()), Some(id));
            assert_eq!(Theme::for_id(id).id, id);
        }
        assert_eq!(
            ThemeId::from_key(" Windows-Classic "),
            Some(ThemeId::WindowsClassic)
        );
        assert_eq!(ThemeId::from_key("none"), None);
        assert_eq!(Theme::forge().corner, Corner::Cut);
        assert_eq!(Theme::windows_classic().corner, Corner::Square);
        assert!(Theme::neon().glow && !Theme::forge().glow);
    }

    #[test]
    fn the_theme_follows_the_windows_colours() {
        let dark = Theme::from_system_values(false, Some(Rgb(0, 120, 212)));
        let light = Theme::from_system_values(true, Some(Rgb(0, 120, 212)));
        assert!(dark.panel.luminance() < light.panel.luminance());
        // the accent of the user is the accent of the interface (lighter on dark, so that it can be read)
        assert_eq!(light.accent, Rgb(0, 120, 212));
        assert!(dark.accent.luminance() > Rgb(0, 120, 212).luminance());
        assert_eq!(dark.id, ThemeId::System);
        // without an accent, the standard blue
        assert_eq!(
            Theme::from_system_values(true, None).accent,
            Rgb(0, 120, 212)
        );
    }

    #[test]
    fn the_output_of_reg_query_is_read() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\r\n    AppsUseLightTheme    REG_DWORD    0x0\r\n    SystemUsesLightTheme    REG_DWORD    0x1\r\n";
        assert_eq!(parse_reg_dword(out, "AppsUseLightTheme"), Some(0));
        assert_eq!(parse_reg_dword(out, "SystemUsesLightTheme"), Some(1));
        assert_eq!(parse_reg_dword(out, "Other"), None);
        // the accent is stored as AABBGGRR: 0xffd47800 is the blue 0078d4
        assert_eq!(accent_from_dword(0xffd4_7800), Rgb(0x00, 0x78, 0xd4));
    }

    #[test]
    fn a_theme_of_the_user_starts_from_another_and_changes_what_it_says() {
        let t = Theme::from_text("# my theme\nbase = windows-classic\naccent = #ff0000\ncorner = cut\nglow = on  # shiny\n").unwrap();
        assert_eq!(t.accent, Rgb(255, 0, 0));
        assert_eq!(t.corner, Corner::Cut);
        assert!(t.glow);
        // what it does not say stays as in the base
        assert_eq!(t.panel, Theme::windows_classic().panel);
        assert!(Theme::from_text("accent = red")
            .unwrap_err()
            .contains("not a colour"));
        assert!(Theme::from_text("color = #ffffff")
            .unwrap_err()
            .contains("unknown setting"));
        assert!(Theme::from_text("corner = pointy").is_err());
        assert!(Theme::from_text("base = mine").is_err());
    }

    #[test]
    fn the_sizes_start_again_after_the_fourth() {
        let t = Theme::forge();
        assert_eq!(t.size_color(0), t.size_color(4));
        assert_ne!(t.size_color(0), t.size_color(1));
    }
}
