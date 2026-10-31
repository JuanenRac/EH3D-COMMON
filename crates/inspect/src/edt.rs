//! The distance of every pixel to the nearest pixel of something else: how far from the copper is each pixel of a channel. The channel is widest where that distance is
//! greatest, so the ridge of the distances is the middle of the channel and twice the distance there is its width.

use crate::blobs::Mask;

const INF: f64 = 1e20;

/// One row or column of the exact transform (Felzenszwalb and Huttenlocher): the lower envelope of the parabolas of the squared distances.
fn pass(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = -INF;
    z[1] = INF;
    for q in 1..n {
        let mut s;
        loop {
            let p = v[k];
            s = ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64))
                / (2.0 * q as f64 - 2.0 * p as f64);
            // the parabola of the last one is covered by the new one from before where it began: drop it
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                break;
            }
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = INF;
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dq = q as f64 - v[k] as f64;
        *out = dq * dq + f[v[k]];
    }
}

/// The distance, in pixels, from every pixel to the nearest pixel that is **off** in the mask (zero for the pixels that are off). A mask with nothing off gives a very large
/// distance everywhere.
pub fn distance_to_off(mask: &Mask) -> Vec<f64> {
    let (w, h) = (mask.w, mask.h);
    let mut g: Vec<f64> = mask
        .on
        .iter()
        .map(|&on| if on { INF } else { 0.0 })
        .collect();
    let n = w.max(h);
    let (mut f, mut d) = (vec![0f64; n], vec![0f64; n]);
    let (mut v, mut z) = (vec![0usize; n], vec![0f64; n + 1]);
    for x in 0..w {
        for y in 0..h {
            f[y] = g[y * w + x];
        }
        pass(&f[..h], &mut d[..h], &mut v[..h], &mut z[..h + 1]);
        for y in 0..h {
            g[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        f[..w].copy_from_slice(&g[y * w..y * w + w]);
        pass(&f[..w], &mut d[..w], &mut v[..w], &mut z[..w + 1]);
        g[y * w..y * w + w].copy_from_slice(&d[..w]);
    }
    g.iter().map(|&s| s.sqrt()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_distance_to_a_wall_is_exact() {
        // a strip of 11 pixels between two walls: the middle is 6 pixels from either
        let mut m = Mask::new(13, 5);
        for y in 0..5 {
            for x in 1..12 {
                m.set(x, y, true);
            }
        }
        let d = distance_to_off(&m);
        assert_eq!(d[2 * 13], 0.0);
        assert_eq!(d[2 * 13 + 1], 1.0);
        assert_eq!(d[2 * 13 + 6], 6.0);
        assert_eq!(d[2 * 13 + 11], 1.0);
    }

    #[test]
    fn the_distance_is_euclidean_not_a_chessboard() {
        // one off pixel in a corner: the pixel 3 right and 4 down is 5 away
        let mut m = Mask::new(8, 8);
        m.on.iter_mut().for_each(|v| *v = true);
        m.set(0, 0, false);
        let d = distance_to_off(&m);
        assert!((d[4 * 8 + 3] - 5.0).abs() < 1e-9);
        assert!((d[7 * 8 + 7] - (98f64).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn nothing_off_is_far_everywhere() {
        let mut m = Mask::new(4, 4);
        m.on.iter_mut().for_each(|v| *v = true);
        assert!(distance_to_off(&m).iter().all(|&d| d > 1e6));
        let none = Mask::new(4, 4);
        assert!(distance_to_off(&none).iter().all(|&d| d == 0.0));
    }

    #[test]
    fn narrow_pictures_work() {
        let mut m = Mask::new(1, 7);
        m.on.iter_mut().for_each(|v| *v = true);
        m.set(0, 0, false);
        let d = distance_to_off(&m);
        assert_eq!(d[3], 3.0);
        let mut row = Mask::new(6, 1);
        row.on.iter_mut().for_each(|v| *v = true);
        row.set(5, 0, false);
        assert_eq!(distance_to_off(&row)[0], 5.0);
    }
}
