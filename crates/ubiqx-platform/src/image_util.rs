//! Image helpers shared by every capturer: downscale to a maximum edge and encode as JPEG;
//! plus the top crop and the difference hash of the `nofap` frame source ([`crate::frame`]).

use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use ubiqx_core::ports::EncodedImage;
use ubiqx_core::{CoreError, CoreResult};

/// JPEG quality used for stored/sent screenshots. 60 keeps text legible at ~100 KB.
pub const JPEG_QUALITY: u8 = 60;

/// Downscales `img` so that its longest edge is at most `max_edge` pixels and encodes it as
/// JPEG. Alpha is dropped.
pub fn downscale_and_encode(img: RgbaImage, max_edge: u32) -> CoreResult<EncodedImage> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err(CoreError::Platform("empty capture".into()));
    }
    let max_edge = max_edge.max(64);
    let scale = (max_edge as f64 / w.max(h) as f64).min(1.0);
    let (tw, th) = (
        ((w as f64 * scale).round() as u32).max(1),
        ((h as f64 * scale).round() as u32).max(1),
    );
    let dynamic = DynamicImage::ImageRgba8(img);
    let resized = if scale < 1.0 {
        dynamic.resize_exact(tw, th, FilterType::Triangle)
    } else {
        dynamic
    };
    let rgb = resized.to_rgb8();
    let mut bytes = Vec::with_capacity((tw * th) as usize / 4);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY);
    encoder
        .encode(rgb.as_raw(), tw, th, image::ExtendedColorType::Rgb8)
        .map_err(|e| CoreError::Platform(format!("jpeg encode: {e}")))?;
    Ok(EncodedImage {
        bytes,
        mime: "image/jpeg".into(),
        width: tw,
        height: th,
    })
}

/// True when the image is (almost) a single colour — typical of a capture made without the
/// Screen Recording permission (wallpaper only) or of a black frame.
pub fn is_uniform(img: &RgbaImage) -> bool {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return true;
    }
    let step = ((w * h) / 4096).max(1) as usize;
    let mut iter = img.pixels().step_by(step);
    let Some(first) = iter.next() else {
        return true;
    };
    let mut differing = 0usize;
    let mut total = 1usize;
    for p in iter {
        total += 1;
        let d = (p[0] as i32 - first[0] as i32).abs()
            + (p[1] as i32 - first[1] as i32).abs()
            + (p[2] as i32 - first[2] as i32).abs();
        if d > 24 {
            differing += 1;
        }
    }
    (differing as f64) / (total as f64) < 0.01
}

/// Largest fraction [`crop_top`] removes: a browser's chrome never covers more than this.
pub const MAX_TOP_CROP: f32 = 0.5;

/// `img` without its top `frac` (a browser's tab strip, address bar and bookmarks, which
/// show other tabs' titles). `frac` is clamped to `0.0..=MAX_TOP_CROP`; a fraction that is
/// not a number crops nothing. At least one row always remains.
pub fn crop_top(img: &RgbaImage, frac: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let frac = if frac.is_finite() {
        frac.clamp(0.0, MAX_TOP_CROP)
    } else {
        0.0
    };
    let cut = ((h as f32 * frac).round() as u32).min(h.saturating_sub(1));
    if cut == 0 {
        return img.clone();
    }
    image::imageops::crop_imm(img, 0, cut, w, h - cut).to_image()
}

/// 64-bit difference hash: the image reduced to 9x8 grey pixels, one bit per pair of
/// horizontal neighbours (set when the left one is brighter). It survives re-encoding and
/// rescaling, and changes when the picture does; two frames are compared by the number of
/// differing bits.
pub fn dhash(img: &RgbaImage) -> u64 {
    const COLS: u32 = 9;
    const ROWS: u32 = 8;
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return 0;
    }
    let small = image::imageops::resize(img, COLS, ROWS, FilterType::Triangle);
    let grey = |x: u32, y: u32| {
        let p = small.get_pixel(x, y);
        299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2])
    };
    let mut hash = 0u64;
    for y in 0..ROWS {
        for x in 0..COLS - 1 {
            hash = (hash << 1) | u64::from(grey(x, y) > grey(x + 1, y));
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_top_removes_the_fraction_and_keeps_the_rest() {
        // Rows are numbered in the red channel.
        let img = RgbaImage::from_fn(40, 100, |_, y| image::Rgba([y as u8, 0, 0, 255]));
        let cropped = crop_top(&img, 0.12);
        assert_eq!(cropped.dimensions(), (40, 88));
        assert_eq!(cropped.get_pixel(0, 0)[0], 12, "the first kept row");
        assert_eq!(cropped.get_pixel(39, 87)[0], 99, "the last row is kept");
        assert_eq!(crop_top(&img, 0.0), img);
        // Clamped and safe on odd input.
        assert_eq!(crop_top(&img, 0.9).height(), 50);
        assert_eq!(crop_top(&img, -1.0), img);
        assert_eq!(crop_top(&img, f32::NAN), img);
        let line = RgbaImage::from_pixel(5, 1, image::Rgba([1, 2, 3, 255]));
        assert_eq!(crop_top(&line, 0.5).dimensions(), (5, 1));
    }

    #[test]
    fn dhash_tells_pictures_apart_and_survives_scaling() {
        let left_bright = RgbaImage::from_fn(320, 200, |x, _| {
            let v = 255 - (x * 255 / 319) as u8;
            image::Rgba([v, v, v, 255])
        });
        let right_bright = RgbaImage::from_fn(320, 200, |x, _| {
            let v = (x * 255 / 319) as u8;
            image::Rgba([v, v, v, 255])
        });
        assert_eq!(dhash(&left_bright), u64::MAX);
        assert_eq!(dhash(&right_bright), 0);
        let smaller = image::imageops::resize(&left_bright, 160, 100, FilterType::Triangle);
        assert_eq!(dhash(&smaller), dhash(&left_bright));
        assert_eq!(dhash(&RgbaImage::new(0, 0)), 0);
    }

    #[test]
    fn downscales_to_max_edge_and_encodes_jpeg() {
        let img = RgbaImage::from_fn(2560, 1600, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
        });
        let out = downscale_and_encode(img, 1280).unwrap();
        assert_eq!(out.width, 1280);
        assert_eq!(out.height, 800);
        assert_eq!(out.mime, "image/jpeg");
        assert!(out.bytes.starts_with(&[0xFF, 0xD8]), "jpeg magic");
        assert!(out.bytes.len() < 400_000);
    }

    #[test]
    fn small_images_are_not_upscaled() {
        let img = RgbaImage::from_pixel(300, 200, image::Rgba([1, 2, 3, 255]));
        let out = downscale_and_encode(img, 1280).unwrap();
        assert_eq!((out.width, out.height), (300, 200));
    }

    #[test]
    fn uniform_detection() {
        assert!(is_uniform(&RgbaImage::from_pixel(
            100,
            100,
            image::Rgba([9, 9, 9, 255])
        )));
        let noisy = RgbaImage::from_fn(100, 100, |x, _| image::Rgba([(x * 2) as u8, 0, 0, 255]));
        assert!(!is_uniform(&noisy));
    }
}
