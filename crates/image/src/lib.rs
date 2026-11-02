//! Grey levels read from a PGM (P2, P5) or a PNG file, shared by the programs that work from a picture (the laser engraving and the fiducials of the camera). Sizes are limited so that a hostile file cannot ask for gigabytes.

use std::io::Cursor;

pub const MAX_SIDE: usize = 20_000;
pub const MAX_PIXELS: usize = 25_000_000;

/// 0 is black and 255 is white (for the laser, black burns most and white does not burn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gray {
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>,
}

impl Gray {
    pub fn new(w: usize, h: usize, data: Vec<u8>) -> Result<Gray, String> {
        if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
            return Err(format!("{w} x {h} pixels: each side must be from 1 to {MAX_SIDE} and the image at most {MAX_PIXELS} pixels"));
        }
        if data.len() != w * h {
            return Err(format!(
                "{w} x {h} pixels need {} values, not {}",
                w * h,
                data.len()
            ));
        }
        Ok(Gray { w, h, data })
    }

    /// A picture of one grey level (for tests and for the background of a drawing).
    pub fn blank(w: usize, h: usize, value: u8) -> Result<Gray, String> {
        Gray::new(w, h, vec![value; w.saturating_mul(h).min(MAX_PIXELS + 1)])
    }

    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.data[y * self.w + x]
    }

    /// Paints a disc (for tests and for drawing marks on a picture).
    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, value: u8) {
        let (x0, x1) = (
            (cx - r).floor().max(0.0) as usize,
            ((cx + r).ceil().max(0.0) as usize).min(self.w.saturating_sub(1)),
        );
        let (y0, y1) = (
            (cy - r).floor().max(0.0) as usize,
            ((cy + r).ceil().max(0.0) as usize).min(self.h.saturating_sub(1)),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                if ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt() <= r {
                    self.data[y * self.w + x] = value;
                }
            }
        }
    }

    /// The image scaled with a box filter to `w` x `h` pixels (used to make the pixel of the engraving as big as the dot of the laser).
    pub fn resize(&self, w: usize, h: usize) -> Result<Gray, String> {
        let (w, h) = (w.max(1), h.max(1));
        // the size is checked before anything is reserved: a dot far too small asks for terabytes
        if w > MAX_SIDE || h > MAX_SIDE || !matches!(w.checked_mul(h), Some(n) if n <= MAX_PIXELS) {
            return Err(format!("{w} x {h} pixels: each side must be from 1 to {MAX_SIDE} and the image at most {MAX_PIXELS} pixels"));
        }
        let mut out = vec![0u8; w * h];
        for y in 0..h {
            let (y0, y1) = (
                y * self.h / h,
                ((y + 1) * self.h)
                    .div_ceil(h)
                    .min(self.h)
                    .max(y * self.h / h + 1),
            );
            for x in 0..w {
                let (x0, x1) = (
                    x * self.w / w,
                    ((x + 1) * self.w)
                        .div_ceil(w)
                        .min(self.w)
                        .max(x * self.w / w + 1),
                );
                let mut sum = 0u64;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        sum += u64::from(self.at(xx, yy));
                    }
                }
                out[y * w + x] = (sum / ((y1 - y0) * (x1 - x0)) as u64) as u8;
            }
        }
        Gray::new(w, h, out)
    }
}

fn skip_blank_and_comments(bytes: &[u8], i: &mut usize) {
    loop {
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i < bytes.len() && bytes[*i] == b'#' {
            while *i < bytes.len() && bytes[*i] != b'\n' {
                *i += 1;
            }
        } else {
            return;
        }
    }
}

fn number(bytes: &[u8], i: &mut usize) -> Result<usize, String> {
    skip_blank_and_comments(bytes, i);
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    std::str::from_utf8(&bytes[start..*i])
        .ok()
        .filter(|s| !s.is_empty() && s.len() < 10)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "the PGM header has something that is not a number".to_string())
}

/// A PGM file, P2 (text) or P5 (binary), 8 bits (a maximum value up to 255).
pub fn from_pgm(bytes: &[u8]) -> Result<Gray, String> {
    if bytes.len() < 2 || bytes[0] != b'P' || !(bytes[1] == b'2' || bytes[1] == b'5') {
        return Err("not a PGM file (it must begin with P2 or P5)".into());
    }
    let binary = bytes[1] == b'5';
    let mut i = 2;
    let (w, h, max) = (
        number(bytes, &mut i)?,
        number(bytes, &mut i)?,
        number(bytes, &mut i)?,
    );
    if max == 0 || max > 255 {
        return Err(format!(
            "the maximum value {max} is not from 1 to 255 (only 8 bits are read)"
        ));
    }
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
        return Err(format!("{w} x {h} pixels is too large or empty"));
    }
    let scale = |v: usize| (v.min(max) * 255 / max) as u8;
    let mut data = Vec::with_capacity(w * h);
    if binary {
        i += 1; // the single whitespace byte after the maximum value
        let raw = bytes
            .get(i..i + w * h)
            .ok_or("the PGM file is shorter than its header says")?;
        data.extend(raw.iter().map(|&b| scale(usize::from(b))));
    } else {
        for _ in 0..w * h {
            data.push(scale(number(bytes, &mut i)?));
        }
    }
    Gray::new(w, h, data)
}

/// A PNG file in grey, grey with alpha, colour (RGB or RGBA) or with a palette, at 8 or 16 bits; transparent parts count as white.
pub fn from_png(bytes: &[u8]) -> Result<Gray, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("the PNG cannot be read: {e}"))?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
        return Err(format!("{w} x {h} pixels is too large or empty"));
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("the PNG cannot be read: {e}"))?;
    let (channels, has_alpha, colour) = match frame.color_type {
        png::ColorType::Grayscale => (1, false, false),
        png::ColorType::GrayscaleAlpha => (2, true, false),
        png::ColorType::Rgb => (3, false, true),
        png::ColorType::Rgba => (4, true, true),
        png::ColorType::Indexed => return Err("a PNG with a palette was not expanded".into()),
    };
    let mut data = Vec::with_capacity(w * h);
    for px in buf[..frame.buffer_size()].chunks_exact(channels) {
        let luma = if colour {
            (299 * u32::from(px[0]) + 587 * u32::from(px[1]) + 114 * u32::from(px[2])) / 1000
        } else {
            u32::from(px[0])
        };
        let alpha = if has_alpha {
            u32::from(px[channels - 1])
        } else {
            255
        };
        data.push(((luma * alpha + 255 * (255 - alpha)) / 255) as u8);
    }
    Gray::new(w, h, data)
}

/// A file of either kind, told apart by its first bytes.
pub fn from_bytes(bytes: &[u8]) -> Result<Gray, String> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        from_png(bytes)
    } else {
        from_pgm(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_of(w: u32, h: u32, color: png::ColorType, depth: png::BitDepth, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w, h);
            enc.set_color(color);
            enc.set_depth(depth);
            let mut writer = enc.write_header().unwrap();
            writer.write_image_data(data).unwrap();
        }
        out
    }

    #[test]
    fn pgm_text_binary_and_scaled() {
        let text = from_pgm(
            b"P2
# a comment
3 2
255
0 128 255
255 128 0
",
        )
        .unwrap();
        assert_eq!((text.w, text.h), (3, 2));
        assert_eq!(text.data, vec![0, 128, 255, 255, 128, 0]);
        let mut bin = b"P5
2 2
255
"
        .to_vec();
        bin.extend([10, 20, 30, 40]);
        assert_eq!(from_pgm(&bin).unwrap().data, vec![10, 20, 30, 40]);
        assert_eq!(
            from_pgm(
                b"P2
2 1
15
0 15
"
            )
            .unwrap()
            .data,
            vec![0, 255]
        );
        assert_eq!(from_bytes(&bin).unwrap().at(1, 1), 40);
    }

    #[test]
    fn pgm_refuses_what_is_wrong() {
        for bad in [
            &b""[..],
            b"P6
1 1
255
 ",
            b"P2
0 4
255
",
            b"P2
2 2
256
0 0 0 0",
            b"P2
2 2
0
0 0 0 0",
            b"P2
99999 99999
255
",
            b"P5
4 4
255
 ",
            b"P2
x 2
255
",
            b"P2
2 2
255
0 1 2",
        ] {
            assert!(from_pgm(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
        }
    }

    #[test]
    fn png_grey_colour_alpha_and_depth() {
        let g = from_png(&png_of(
            2,
            2,
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            &[0, 90, 180, 255],
        ))
        .unwrap();
        assert_eq!(g.data, vec![0, 90, 180, 255]);
        let c = from_png(&png_of(
            1,
            2,
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            &[255, 0, 0, 0, 255, 0],
        ))
        .unwrap();
        assert_eq!(c.data, vec![76, 149]);
        // a transparent pixel counts as white
        let a = from_png(&png_of(
            2,
            1,
            png::ColorType::GrayscaleAlpha,
            png::BitDepth::Eight,
            &[0, 255, 0, 0],
        ))
        .unwrap();
        assert_eq!(a.data, vec![0, 255]);
        let sixteen = from_png(&png_of(
            1,
            1,
            png::ColorType::Grayscale,
            png::BitDepth::Sixteen,
            &[0x80, 0x00],
        ))
        .unwrap();
        assert_eq!(sixteen.data, vec![0x80]);
    }

    #[test]
    fn png_refuses_damaged_files() {
        let good = png_of(
            4,
            4,
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            &[7; 16],
        );
        assert!(from_png(&good).is_ok());
        assert!(from_png(&good[..good.len() / 2]).is_err());
        assert!(from_png(&good[..8]).is_err());
        assert!(from_png(b"not a png").is_err());
        assert!(from_bytes(&good[..8]).is_err());
    }

    #[test]
    fn sizes_are_limited() {
        assert!(Gray::new(MAX_PIXELS, 2, vec![]).is_err());
        assert!(Gray::new(MAX_SIDE + 1, 1, vec![]).is_err());
        assert!(Gray::new(2, 2, vec![0; 3]).is_err());
        assert!(Gray::new(0, 2, vec![]).is_err());
        assert!(Gray::blank(MAX_PIXELS, 2, 0).is_err());
        assert!(Gray::blank(usize::MAX, usize::MAX, 0).is_err());
        let g = Gray::blank(3, 2, 9).unwrap();
        assert_eq!((g.w, g.h, g.data), (3, 2, vec![9; 6]));
    }

    #[test]
    fn resize_averages_boxes_and_refuses_huge_sizes() {
        let g = Gray::new(4, 2, vec![0, 0, 255, 255, 0, 0, 255, 255]).unwrap();
        assert_eq!(g.resize(2, 1).unwrap().data, vec![0, 255]);
        assert_eq!(g.resize(4, 2).unwrap(), g);
        let up = Gray::new(2, 1, vec![0, 255]).unwrap().resize(4, 1).unwrap();
        assert_eq!(up.data, vec![0, 0, 255, 255]);
        assert!(g.resize(MAX_PIXELS, 2).is_err());
        assert!(g.resize(usize::MAX, usize::MAX).is_err());
        assert!(g.resize(2_000_000, 1_000_000).is_err());
        // a size of zero counts as one pixel
        assert_eq!(g.resize(0, 0).unwrap().data, vec![127]);
    }

    #[test]
    fn disc_paints_inside_and_clips_at_the_edges() {
        let mut g = Gray::blank(20, 20, 255).unwrap();
        g.disc(10.0, 10.0, 3.0, 0);
        assert_eq!((g.at(10, 10), g.at(12, 10), g.at(14, 10)), (0, 0, 255));
        // a disc half outside the picture, and one wholly outside, do not panic
        g.disc(0.0, 0.0, 4.0, 7);
        assert_eq!(g.at(0, 0), 7);
        g.disc(500.0, 500.0, 3.0, 1);
        g.disc(-50.0, -50.0, 3.0, 1);
        assert_eq!(g.at(19, 19), 255);
    }
}
