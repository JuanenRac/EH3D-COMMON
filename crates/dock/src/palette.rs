//! The colours of the PHOENIX family, written once: five tones for each program (fire, emerald, ice blue, amethyst, crimson and gold) and the
//! arithmetic that keeps text legible on them. The principal tone is the one of fills and rings; it is not always legible as text (gold on white,
//! amethyst on a dark window), so text takes [`readable`] and anything written on a fill takes [`ink_on`].

use crate::about::Phoenix;

/// A colour: red, green, blue.
pub type Rgb = [u8; 3];

/// The contrast that text must reach against its background.
pub const TEXT_CONTRAST: f64 = 4.5;

/// The five tones of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tones {
    /// The colour of the program: fills, rings and marks.
    pub main: Rgb,
    /// A step darker than the principal one.
    pub mid: Rgb,
    /// The darkest tone, for grounds and shadows.
    pub deep: Rgb,
    /// A lighter tone, for highlights.
    pub light: Rgb,
    /// The palest tone, for glows.
    pub glow: Rgb,
}

const fn hex(value: u32) -> Rgb {
    let [_, r, g, b] = value.to_be_bytes();
    [r, g, b]
}

const fn tones(main: u32, mid: u32, deep: u32, light: u32, glow: u32) -> Tones {
    Tones {
        main: hex(main),
        mid: hex(mid),
        deep: hex(deep),
        light: hex(light),
        glow: hex(glow),
    }
}

impl Phoenix {
    /// The five tones of the program.
    pub const fn tones(self) -> Tones {
        match self {
            // fire
            Phoenix::Pcb => tones(0xFF5A1F, 0xE2361B, 0xB01010, 0xFFB347, 0xFFD27A),
            // emerald
            Phoenix::Pnp => tones(0x2FFF94, 0x17C47A, 0x0B8C64, 0x8FFFA4, 0xD8FFDE),
            // ice blue
            Phoenix::Cad => tones(0x2F8CFF, 0x1F5FD0, 0x0B2A8C, 0x8FE3FF, 0xD8F6FF),
            // amethyst
            Phoenix::Cnc => tones(0xA52FFF, 0x8A1FD0, 0x6F0B8C, 0xAD8FFF, 0xE2D8FF),
            // crimson
            Phoenix::Laser => tones(0xFF2F43, 0xD01A20, 0x971100, 0xFF8FBC, 0xFFD8E8),
            // gold
            Phoenix::Printer3d => tones(0xFFD53E, 0xE8A317, 0xA86E00, 0xFFDC5E, 0xFFE999),
        }
    }
}

/// The relative luminance of a colour (sRGB), from 0 (black) to 1 (white).
pub fn luminance(colour: Rgb) -> f64 {
    let channel = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = colour;
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

/// The contrast between two colours, from 1 (the same) to 21 (black and white).
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Black or white, whichever reads better when written on a fill of this colour.
pub fn ink_on(fill: Rgb) -> Rgb {
    let (black, white) = ([0, 0, 0], [255, 255, 255]);
    if contrast(black, fill) >= contrast(white, fill) {
        black
    } else {
        white
    }
}

/// The colour to write text with on a background: the colour itself when it reaches [`TEXT_CONTRAST`], otherwise the nearest lightness of the
/// same hue that does (darker on a light background, lighter on a dark one). Every background has such a tone.
pub fn readable(colour: Rgb, background: Rgb) -> Rgb {
    if contrast(colour, background) >= TEXT_CONTRAST {
        return colour;
    }
    let step = if ink_on(background) == [0, 0, 0] {
        -0.01
    } else {
        0.01
    };
    let (hue, mut lightness, saturation) = to_hls(colour);
    loop {
        lightness = (lightness + step).clamp(0.0, 1.0);
        let candidate = from_hls(hue, lightness, saturation);
        if contrast(candidate, background) >= TEXT_CONTRAST || lightness <= 0.0 || lightness >= 1.0
        {
            return candidate;
        }
    }
}

/// What a window does with the colour of its program: only accents (main buttons, selection, the active tab and links). The grounds, the panels
/// and the bars of the window are not touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accents {
    /// Links and any other text in the colour of the program, legible on every ground of the window.
    pub link: Rgb,
    /// The ground of what is selected and of the active tab: the ground of the window with a tint of the program.
    pub selection_fill: Rgb,
    /// Text and outline on [`Accents::selection_fill`].
    pub selection_ink: Rgb,
    /// The fill of a main button.
    pub button_fill: Rgb,
    /// The text of a main button.
    pub button_ink: Rgb,
}

/// The strongest and the weakest tint of a selection, in hundredths of the colour of the program.
const TINT_STRONGEST: u8 = 40;
const TINT_WEAKEST: u8 = 8;

/// The colour `percent` hundredths of the way from `ground` to `colour`.
fn tint(ground: Rgb, colour: Rgb, percent: u8) -> Rgb {
    let mix = |g: u8, c: u8| {
        let share = u16::from(percent.min(100));
        // both terms are at most 255 * 100, so the sum fits and the result is at most 255
        ((u16::from(g) * (100 - share) + u16::from(c) * share + 50) / 100) as u8
    };
    [
        mix(ground[0], colour[0]),
        mix(ground[1], colour[1]),
        mix(ground[2], colour[2]),
    ]
}

/// The accents of a window: `main` is the principal tone of the program, `ground` the ground of its panels, `others` the other grounds that text
/// is written on (all light or all dark, like `ground`) and `text` the colour of the text of the window. The selection is the strongest tint on
/// which that text still reaches [`TEXT_CONTRAST`].
pub fn accents(main: Rgb, ground: Rgb, others: &[Rgb], text: Rgb) -> Accents {
    let on_grounds = others.iter().fold(readable(main, ground), |found, &other| {
        readable(found, other)
    });
    let selection_fill = (TINT_WEAKEST..=TINT_STRONGEST)
        .rev()
        .step_by(2)
        .map(|percent| tint(ground, main, percent))
        .find(|&fill| contrast(text, fill) >= TEXT_CONTRAST)
        .unwrap_or_else(|| tint(ground, main, TINT_WEAKEST));
    Accents {
        link: on_grounds,
        selection_fill,
        selection_ink: readable(on_grounds, selection_fill),
        button_fill: main,
        button_ink: ink_on(main),
    }
}

/// Hue (in turns), lightness and saturation of a colour.
fn to_hls(colour: Rgb) -> (f64, f64, f64) {
    let [r, g, b] = colour.map(|v| f64::from(v) / 255.0);
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let lightness = (high + low) / 2.0;
    let range = high - low;
    if range <= 0.0 {
        return (0.0, lightness, 0.0);
    }
    let saturation = if lightness <= 0.5 {
        range / (high + low)
    } else {
        range / (2.0 - high - low)
    };
    let sixth = if r >= high {
        (g - b) / range
    } else if g >= high {
        2.0 + (b - r) / range
    } else {
        4.0 + (r - g) / range
    };
    ((sixth / 6.0).rem_euclid(1.0), lightness, saturation)
}

/// The colour of a hue (in turns), a lightness and a saturation.
fn from_hls(hue: f64, lightness: f64, saturation: f64) -> Rgb {
    let top = if lightness <= 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let bottom = 2.0 * lightness - top;
    let channel = |turn: f64| {
        let turn = turn.rem_euclid(1.0);
        let value = if turn < 1.0 / 6.0 {
            bottom + (top - bottom) * turn * 6.0
        } else if turn < 0.5 {
            top
        } else if turn < 2.0 / 3.0 {
            bottom + (top - bottom) * (2.0 / 3.0 - turn) * 6.0
        } else {
            bottom
        };
        // the value is held between 0 and 255, so the conversion loses nothing but the fraction
        (value * 255.0).round().clamp(0.0, 255.0) as u8
    };
    [
        channel(hue + 1.0 / 3.0),
        channel(hue),
        channel(hue - 1.0 / 3.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The grounds of the windows: white and the dark of the brand, and those of the light and dark looks of egui.
    const GROUNDS: [Rgb; 4] = [hex(0xFFFFFF), hex(0x161B22), hex(0xF8F8F8), hex(0x1B1B1B)];

    #[test]
    fn the_five_tones_of_each_program_go_from_the_principal_one_to_a_darker_and_a_paler_one() {
        let mains = [
            0xFF5A1F,
            0x2FFF94,
            0x2F8CFF,
            0xA52FFF,
            0xFF2F43,
            0xFFD53E_u32,
        ];
        for (program, main) in Phoenix::ALL.into_iter().zip(mains) {
            let t = program.tones();
            assert_eq!(t.main, hex(main), "{program:?}");
            assert_eq!(program.accent(), t.main, "{program:?}");
            assert!(luminance(t.deep) < luminance(t.main), "{program:?}");
            assert!(luminance(t.glow) > luminance(t.main), "{program:?}");
            assert!(luminance(t.mid) < luminance(t.main), "{program:?}");
        }
    }

    #[test]
    fn the_text_tone_of_each_program_reads_on_light_and_on_dark_windows_and_keeps_its_hue() {
        for program in Phoenix::ALL {
            let main = program.tones().main;
            for ground in GROUNDS {
                let text = readable(main, ground);
                assert!(
                    contrast(text, ground) >= TEXT_CONTRAST,
                    "{program:?} {ground:?} {text:?}"
                );
                let turn = (to_hls(text).0 - to_hls(main).0).abs();
                assert!(
                    turn.min(1.0 - turn) < 0.02,
                    "{program:?} {ground:?} {text:?}"
                );
                if contrast(main, ground) >= TEXT_CONTRAST {
                    assert_eq!(text, main);
                }
            }
        }
    }

    #[test]
    fn the_text_tones_are_those_of_the_colour_study_of_the_family() {
        let (white, brand_dark) = (hex(0xFFFFFF), hex(0x161B22));
        let expected = [
            (Phoenix::Pcb, 0xDC3A00, 0xFF5A1F),
            (Phoenix::Pnp, 0x008641, 0x2FFF94),
            (Phoenix::Cad, 0x006EF6, 0x2F8CFF),
            (Phoenix::Cnc, 0xA52FFF, 0xB453FF),
            (Phoenix::Laser, 0xEC0017, 0xFF2F43),
            (Phoenix::Printer3d, 0x907000, 0xFFD53E),
        ];
        for (program, on_white, on_dark) in expected {
            for (ground, value) in [(white, on_white), (brand_dark, on_dark)] {
                let found = readable(program.tones().main, ground);
                let near = found
                    .iter()
                    .zip(hex(value))
                    .all(|(a, b)| a.abs_diff(b) <= 2);
                assert!(near, "{program:?} {ground:?} {found:?}");
            }
        }
    }

    #[test]
    fn every_ground_has_a_legible_tone_and_every_fill_a_legible_ink() {
        for level in 0..=255_u8 {
            let grey = [level, level, level];
            for program in Phoenix::ALL {
                let text = readable(program.tones().main, grey);
                assert!(contrast(text, grey) >= TEXT_CONTRAST, "{program:?} {level}");
            }
            assert!(contrast(ink_on(grey), grey) >= TEXT_CONTRAST, "{level}");
        }
        for program in Phoenix::ALL {
            let t = program.tones();
            for fill in [t.main, t.mid, t.deep, t.light, t.glow] {
                assert!(
                    contrast(ink_on(fill), fill) >= TEXT_CONTRAST,
                    "{program:?} {fill:?}"
                );
            }
        }
        assert!((contrast([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.001);
        assert_eq!(from_hls(0.0, 0.5, 1.0), [255, 0, 0]);
    }

    #[test]
    fn the_accents_of_each_program_read_on_light_and_on_dark_windows() {
        // ground of the panels, the other grounds and the text: the dark and light looks of egui and of the brand
        let looks: [(u32, [u32; 2], u32); 4] = [
            (0x1B1B1B, [0x1B1B1B, 0x0A0A0A], 0xB4B4B4),
            (0xF8F8F8, [0xF8F8F8, 0xFFFFFF], 0x3C3C3C),
            (0x161B22, [0x21262D, 0x0D1117], 0xE6EDF3),
            (0xFFFFFF, [0xF4F0EA, 0xE9E3DA], 0x221E1A),
        ];
        for program in Phoenix::ALL {
            let main = program.tones().main;
            for (ground, others, text) in looks {
                let (ground, others, text) = (hex(ground), others.map(hex), hex(text));
                let a = accents(main, ground, &others, text);
                let what = format!("{program:?} {ground:?} {a:?}");
                for g in others.into_iter().chain([ground]) {
                    assert!(contrast(a.link, g) >= TEXT_CONTRAST, "link {what}");
                }
                assert!(contrast(text, a.selection_fill) >= TEXT_CONTRAST, "{what}");
                assert!(
                    contrast(a.selection_ink, a.selection_fill) >= TEXT_CONTRAST,
                    "{what}"
                );
                assert!(
                    contrast(a.button_ink, a.button_fill) >= TEXT_CONTRAST,
                    "{what}"
                );
                assert_eq!(a.button_fill, main, "{what}");
                // what is selected can be told from the ground it lies on
                assert!(contrast(a.selection_fill, ground) > 1.05, "{what}");
            }
        }
        assert_eq!(tint([0, 0, 0], [255, 255, 255], 0), [0, 0, 0]);
        assert_eq!(tint([0, 0, 0], [255, 255, 255], 100), [255, 255, 255]);
        assert_eq!(tint([10, 20, 30], [110, 120, 130], 50), [60, 70, 80]);
    }
}
