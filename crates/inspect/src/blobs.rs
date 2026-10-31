//! The patches of a black-and-white picture: the groups of pixels that touch, with their size, their centre and how round they are.

/// A black-and-white picture: `true` is the thing looked for (a hole, copper).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    pub w: usize,
    pub h: usize,
    pub on: Vec<bool>,
}

impl Mask {
    pub fn new(w: usize, h: usize) -> Mask {
        Mask {
            w,
            h,
            on: vec![false; w * h],
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.on[y * self.w + x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, v: bool) {
        self.on[y * self.w + x] = v;
    }

    pub fn count(&self) -> usize {
        self.on.iter().filter(|&&v| v).count()
    }
}

/// One patch.
#[derive(Debug, Clone, PartialEq)]
pub struct Blob {
    pub area: usize,
    /// The centre of the pixels (a pixel centre is at its integer coordinates).
    pub cx: f64,
    pub cy: f64,
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
    /// How round: the ratio of the short to the long axis of the ellipse with the same moments (1 is a circle, near 0 a line).
    pub roundness: f64,
    /// The patch touches the edge of the picture, so it may go on outside.
    pub touches_edge: bool,
    /// How much of its box the patch fills: a disc fills about 0.7 to 0.79, a ring or a line much less.
    pub fill: f64,
}

/// The patches of the mask, found with 8-neighbour connection, those of at least `min_area` pixels.
pub fn blobs(mask: &Mask, min_area: usize) -> Vec<Blob> {
    let (w, h) = (mask.w, mask.h);
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for sy in 0..h {
        for sx in 0..w {
            if !mask.get(sx, sy) || seen[sy * w + sx] {
                continue;
            }
            seen[sy * w + sx] = true;
            stack.push((sx, sy));
            let (mut n, mut sumx, mut sumy) = (0usize, 0f64, 0f64);
            let (mut sxx, mut syy, mut sxy) = (0f64, 0f64, 0f64);
            let (mut x0, mut y0, mut x1, mut y1) = (sx, sy, sx, sy);
            while let Some((x, y)) = stack.pop() {
                n += 1;
                let (fx, fy) = (x as f64, y as f64);
                sumx += fx;
                sumy += fy;
                sxx += fx * fx;
                syy += fy * fy;
                sxy += fx * fy;
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let i = ny as usize * w + nx as usize;
                        if mask.on[i] && !seen[i] {
                            seen[i] = true;
                            stack.push((nx as usize, ny as usize));
                        }
                    }
                }
            }
            if n < min_area {
                continue;
            }
            let nf = n as f64;
            let (cx, cy) = (sumx / nf, sumy / nf);
            // the covariance, with the spread of a pixel itself (1/12) so a single pixel is round
            let vxx = sxx / nf - cx * cx + 1.0 / 12.0;
            let vyy = syy / nf - cy * cy + 1.0 / 12.0;
            let vxy = sxy / nf - cx * cy;
            let tr = vxx + vyy;
            let det = vxx * vyy - vxy * vxy;
            let disc = (tr * tr / 4.0 - det).max(0.0).sqrt();
            let (l1, l2) = (tr / 2.0 + disc, (tr / 2.0 - disc).max(0.0));
            out.push(Blob {
                area: n,
                cx,
                cy,
                x0,
                y0,
                x1,
                y1,
                roundness: if l1 > 0.0 { (l2 / l1).sqrt() } else { 1.0 },
                touches_edge: x0 == 0 || y0 == 0 || x1 + 1 == w || y1 + 1 == h,
                fill: n as f64 / ((x1 - x0 + 1) * (y1 - y0 + 1)) as f64,
            });
        }
    }
    out
}

/// The mask of the pixels of a window of `pixels` (row by row, `w` x `h`) that are darker than `threshold` (`dark`) or at least as light (not `dark`).
pub fn threshold(pixels: &[u8], w: usize, h: usize, level: u8, dark: bool) -> Mask {
    let mut m = Mask::new(w, h);
    for (i, &v) in pixels.iter().enumerate().take(w * h) {
        m.on[i] = if dark { v < level } else { v >= level };
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn disc(w: usize, h: usize, cx: f64, cy: f64, r: f64) -> Mask {
        let mut m = Mask::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                m.set(x, y, dx * dx + dy * dy <= r * r);
            }
        }
        m
    }

    #[test]
    fn a_disc_is_one_round_patch_with_its_centre_and_area() {
        let b = blobs(&disc(60, 50, 30.0, 20.0, 10.0), 1);
        assert_eq!(b.len(), 1);
        let b = &b[0];
        assert!((b.cx - 30.0).abs() < 0.05 && (b.cy - 20.0).abs() < 0.05);
        assert!((b.area as f64 - 314.0).abs() < 25.0, "area {}", b.area);
        assert!(b.roundness > 0.97 && !b.touches_edge);
        assert!(b.fill > 0.65 && b.fill < 0.8, "fill {}", b.fill);
    }

    #[test]
    fn a_ring_does_not_fill_its_box() {
        let mut ring = disc(60, 60, 30.0, 30.0, 20.0);
        let inner = disc(60, 60, 30.0, 30.0, 15.0);
        for i in 0..ring.on.len() {
            ring.on[i] &= !inner.on[i];
        }
        let b = blobs(&ring, 1);
        assert_eq!(b.len(), 1);
        assert!(
            b[0].fill < 0.55 && b[0].roundness > 0.95,
            "fill {}",
            b[0].fill
        );
    }

    #[test]
    fn a_bar_is_not_round_and_touching_the_edge_is_noticed() {
        let mut m = Mask::new(40, 20);
        for x in 0..40 {
            for y in 8..11 {
                m.set(x, y, true);
            }
        }
        let b = blobs(&m, 1);
        assert_eq!(b.len(), 1);
        assert!(b[0].roundness < 0.2 && b[0].touches_edge);
    }

    #[test]
    fn small_specks_are_dropped_and_separate_patches_stay_separate() {
        let mut m = disc(60, 30, 15.0, 15.0, 6.0);
        let d2 = disc(60, 30, 45.0, 15.0, 6.0);
        for i in 0..m.on.len() {
            m.on[i] |= d2.on[i];
        }
        m.set(30, 2, true);
        assert_eq!(blobs(&m, 1).len(), 3);
        assert_eq!(blobs(&m, 5).len(), 2);
        // touching diagonally joins
        let mut d = Mask::new(4, 4);
        d.set(1, 1, true);
        d.set(2, 2, true);
        assert_eq!(blobs(&d, 1).len(), 1);
    }

    #[test]
    fn the_threshold_picks_the_dark_or_the_light() {
        let px = [10u8, 200, 90, 120];
        let dark = threshold(&px, 2, 2, 100, true);
        assert_eq!(dark.on, vec![true, false, true, false]);
        let light = threshold(&px, 2, 2, 100, false);
        assert_eq!(light.on, vec![false, true, false, true]);
        assert_eq!(dark.count(), 2);
    }
}
