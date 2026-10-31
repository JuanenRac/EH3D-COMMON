//! The picture as grey pixels, and the threshold that tells the dark from the light.

/// A grey picture: `w` x `h` pixels, row by row, 0 black and 255 white.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gray {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Gray {
    pub fn new(w: usize, h: usize, fill: u8) -> Gray {
        Gray {
            w,
            h,
            px: vec![fill; w * h],
        }
    }

    /// The grey of `w` x `h` RGB pixels (3 bytes each), with the weights of the eye.
    pub fn from_rgb(w: usize, h: usize, rgb: &[u8]) -> Result<Gray, String> {
        if w == 0 || h == 0 || rgb.len() != w * h * 3 {
            return Err("the picture has the wrong number of pixels".to_string());
        }
        let px = rgb
            .chunks_exact(3)
            .map(|p| ((p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000) as u8)
            .collect();
        Ok(Gray { w, h, px })
    }

    /// The grey of a JPEG (what the camera of the head sends).
    pub fn from_jpeg(bytes: &[u8]) -> Result<Gray, String> {
        let mut d = jpeg_decoder::Decoder::new(bytes);
        let data = d.decode().map_err(|e| format!("not a JPEG: {e}"))?;
        let info = d.info().ok_or("the JPEG has no header")?;
        let (w, h) = (info.width as usize, info.height as usize);
        match info.pixel_format {
            jpeg_decoder::PixelFormat::L8 => {
                if data.len() != w * h {
                    return Err("the JPEG is cut short".to_string());
                }
                Ok(Gray { w, h, px: data })
            }
            jpeg_decoder::PixelFormat::RGB24 => Gray::from_rgb(w, h, &data),
            _ => Err("the JPEG is not grey or RGB".to_string()),
        }
    }

    #[inline]
    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.px[y * self.w + x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, v: u8) {
        self.px[y * self.w + x] = v;
    }

    /// The mean grey of the whole picture.
    pub fn mean(&self) -> f64 {
        if self.px.is_empty() {
            return 0.0;
        }
        self.px.iter().map(|&v| v as f64).sum::<f64>() / self.px.len() as f64
    }

    /// The pixels of a window `[x0, x1) x [y0, y1)` cut to the picture.
    pub fn window(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<u8> {
        let (x1, y1) = (x1.min(self.w), y1.min(self.h));
        let mut out = Vec::new();
        for y in y0..y1 {
            out.extend_from_slice(&self.px[y * self.w + x0.min(x1)..y * self.w + x1]);
        }
        out
    }
}

/// The threshold of Otsu: the grey that splits the pixels into two groups with the least spread inside each. When many greys split them equally well (a picture of two flat
/// greys), the middle of those is taken. A picture of one flat grey gives 128.
pub fn otsu(px: &[u8]) -> u8 {
    let mut hist = [0u64; 256];
    for &v in px {
        hist[v as usize] += 1;
    }
    let total: u64 = hist.iter().sum();
    if total == 0 {
        return 128;
    }
    let sum_all: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| i as f64 * c as f64)
        .sum();
    let (mut w0, mut sum0) = (0f64, 0f64);
    let mut var = [0f64; 256];
    for (t, &c) in hist.iter().enumerate() {
        w0 += c as f64;
        sum0 += t as f64 * c as f64;
        let w1 = total as f64 - w0;
        if w0 == 0.0 || w1 == 0.0 {
            continue;
        }
        let (m0, m1) = (sum0 / w0, (sum_all - sum0) / w1);
        var[t] = w0 * w1 * (m0 - m1) * (m0 - m1);
    }
    let best = var.iter().cloned().fold(0f64, f64::max);
    if best <= 0.0 {
        return 128;
    }
    // the greys at or below `t` are one group: the split at t is the same for every t between two populated greys
    let first = var
        .iter()
        .position(|&v| v >= best * (1.0 - 1e-9))
        .unwrap_or(128);
    let last = var
        .iter()
        .rposition(|&v| v >= best * (1.0 - 1e-9))
        .unwrap_or(128);
    // the threshold is the first grey of the lighter group, so the split `v < level` / `v >= level` falls in the middle of the plateau
    (((first + last) / 2) + 1).min(255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_becomes_grey_with_the_weights_of_the_eye() {
        let g = Gray::from_rgb(3, 1, &[255, 255, 255, 255, 0, 0, 0, 255, 0]).unwrap();
        assert_eq!(g.px, vec![255, 76, 149]);
        assert!(Gray::from_rgb(2, 2, &[0; 11]).is_err());
        assert!(Gray::from_rgb(0, 2, &[]).is_err());
    }

    #[test]
    fn otsu_splits_two_groups() {
        let mut px = vec![30u8; 500];
        px.extend(vec![200u8; 500]);
        let t = otsu(&px);
        assert!((30..200).contains(&t), "threshold {t}");
        assert_eq!(otsu(&[77u8; 100]), 128);
        assert_eq!(otsu(&[]), 128);
    }

    #[test]
    fn a_window_is_cut_to_the_picture() {
        let mut g = Gray::new(4, 3, 0);
        g.set(1, 1, 9);
        assert_eq!(g.window(1, 1, 3, 2), vec![9, 0]);
        assert_eq!(g.window(2, 2, 99, 99).len(), 2);
        assert!(g.window(9, 9, 12, 12).is_empty());
        assert!((g.mean() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn a_jpeg_that_is_not_one_is_refused() {
        assert!(Gray::from_jpeg(&[1, 2, 3]).is_err());
    }
}
