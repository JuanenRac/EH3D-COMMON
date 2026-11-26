//! Pictures shown inside the program: PNG, JPEG, GIF (its first image), BMP and WebP. The format is
//! read from the bytes, the size is checked before any pixel is decoded, and a picture larger than
//! the window can hold as a texture is reduced.

use std::io::Cursor;

use image::{ImageReader, Limits};

use crate::error::ViewerError;
use crate::limits::ViewerLimits;

/// The longest side of what is handed to the window.
pub const MAX_SIDE: u32 = 4096;

/// A decoded picture, four bytes per pixel (red, green, blue, alpha).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

fn damaged(e: impl std::fmt::Display) -> ViewerError {
    ViewerError::Damaged(format!("picture: {e}"))
}

fn reader(data: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, ViewerError> {
    ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(damaged)
}

/// The same picture with no side longer than `max_side`, sampled without smoothing; for a window
/// that cannot show a larger one.
pub fn fit(raster: &Raster, max_side: u32) -> Raster {
    let longest = raster.width.max(raster.height);
    let max_side = max_side.max(1);
    if longest <= max_side {
        return raster.clone();
    }
    let step = |side: u32| {
        let reduced = u64::from(side) * u64::from(max_side) / u64::from(longest);
        reduced.max(1) as u32
    };
    let (width, height) = (step(raster.width), step(raster.height));
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        let from_y = (u64::from(y) * u64::from(raster.height) / u64::from(height)) as usize;
        for x in 0..width {
            let from_x = (u64::from(x) * u64::from(raster.width) / u64::from(width)) as usize;
            let at = (from_y * raster.width as usize + from_x) * 4;
            rgba.extend_from_slice(&raster.rgba[at..at + 4]);
        }
    }
    Raster {
        width,
        height,
        rgba,
    }
}

/// Decodes a picture within the limits.
pub fn decode(data: &[u8], limits: &ViewerLimits) -> Result<Raster, ViewerError> {
    let (width, height) = reader(data)?.into_dimensions().map_err(damaged)?;
    if width == 0 || height == 0 {
        return Err(damaged("no size"));
    }
    if u64::from(width) * u64::from(height) > limits.max_image_pixels {
        return Err(ViewerError::TooLarge {
            limit: limits.max_image_pixels,
        });
    }
    let mut full = reader(data)?;
    let mut bounds = Limits::default();
    bounds.max_alloc = Some(limits.max_image_pixels.saturating_mul(8));
    full.limits(bounds);
    let mut picture = full.decode().map_err(damaged)?;
    if width > MAX_SIDE || height > MAX_SIDE {
        picture = picture.thumbnail(MAX_SIDE, MAX_SIDE);
    }
    let rgba = picture.into_rgba8();
    Ok(Raster {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]))
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn a_picture_is_decoded_and_a_wide_one_is_reduced() {
        let small = decode(&png(3, 2), &ViewerLimits::default()).unwrap();
        assert_eq!((small.width, small.height, small.rgba.len()), (3, 2, 24));
        assert_eq!(&small.rgba[..4], &[10, 20, 30, 255]);
        let wide = decode(&png(MAX_SIDE + 1000, 10), &ViewerLimits::default()).unwrap();
        assert_eq!(wide.width, MAX_SIDE);
    }

    #[test]
    fn a_picture_is_fitted_to_the_side_of_the_window() {
        let raster = decode(&png(40, 10), &ViewerLimits::default()).unwrap();
        let small = fit(&raster, 8);
        assert_eq!((small.width, small.height, small.rgba.len()), (8, 2, 64));
        assert_eq!(fit(&raster, 40), raster);
    }

    #[test]
    fn too_many_pixels_and_broken_files_are_refused() {
        let limits = ViewerLimits {
            max_image_pixels: 5,
            ..ViewerLimits::default()
        };
        assert_eq!(
            decode(&png(3, 2), &limits),
            Err(ViewerError::TooLarge { limit: 5 })
        );
        let whole = png(8, 8);
        assert!(decode(&whole[..40], &ViewerLimits::default()).is_err());
        assert!(decode(b"not a picture", &ViewerLimits::default()).is_err());
        assert!(decode(b"", &ViewerLimits::default()).is_err());
    }
}
