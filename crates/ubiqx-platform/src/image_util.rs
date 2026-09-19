//! Image helpers shared by every capturer: downscale to a maximum edge and encode as JPEG.

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

#[cfg(test)]
mod tests {
    use super::*;

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
