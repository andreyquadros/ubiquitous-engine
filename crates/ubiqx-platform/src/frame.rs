//! The frame the AI check of the `nofap` module looks at: one reduced image of one window,
//! in memory only.
//!
//! [`WindowFrameSource`] is the one portable implementation of
//! `ubiqx_core::nofap::FrameSource`. It calls
//! `ScreenCapturer::capture(CaptureTarget::Window(id), 1280)`, decodes, crops the top
//! fraction asked for (a browser's tab strip and address bar), downscales to the edge asked
//! for, computes a 64-bit difference hash and encodes JPEG. No file, no `screenshots` row,
//! no thumbnail, and a near-uniform capture is "no frame".
//!
//! **A window or nothing.** The frame goes to an AI provider, so it must show the window
//! that was judged a candidate and never whatever else is on the screen. The source asks
//! for a window only, and the capturer it is given must not answer with a display or a
//! region instead. The screenshot capturers of Windows and Linux do exactly that when a
//! window cannot be read (they crop the screen under it, where a messaging or a bank
//! window may be), so `PlatformServices::frames` is built over their window-only twins
//! (`windows::capture::WindowsWindowCapturer`, `linux::capture::X11WindowCapturer`); the
//! macOS capturer has no such fallback. A window that cannot be read is "no frame".
//!
//! Known limit: on an X11 session without a compositing manager, `GetImage` of a window
//! returns the screen's pixels where another window covers it.

use std::sync::Arc;

use image::RgbaImage;
use ubiqx_core::nofap::{Frame, FrameOpts, FrameSource};
use ubiqx_core::ports::{CaptureTarget, ScreenCapturer};
use ubiqx_core::{CoreError, CoreResult};

use crate::image_util::{crop_top, dhash, downscale_and_encode, is_uniform};

/// Longest edge asked of the capturer. The frame is reduced again after the crop, so the
/// crop works on more pixels than the result has.
pub const CAPTURE_MAX_EDGE: u32 = 1280;

/// Why a window-only capturer refuses a display or a region.
pub const WINDOW_ONLY: &str = "only a window is captured";

/// Takes frames through a screen capturer. Outside tests that capturer is a window-only
/// one (see the module documentation): use `PlatformServices::frames`.
pub struct WindowFrameSource {
    capturer: Arc<dyn ScreenCapturer>,
}

impl WindowFrameSource {
    pub fn new(capturer: Arc<dyn ScreenCapturer>) -> Self {
        Self { capturer }
    }
}

/// The frame of one decoded capture: `None` when the picture is near-uniform (a blank or
/// black capture, a window that shows nothing), before or after the crop.
pub fn frame_from(captured: &RgbaImage, opts: FrameOpts) -> CoreResult<Option<Frame>> {
    if captured.width() == 0 || captured.height() == 0 || is_uniform(captured) {
        return Ok(None);
    }
    let cropped = crop_top(captured, opts.crop_top_frac);
    if is_uniform(&cropped) {
        return Ok(None);
    }
    let dhash = dhash(&cropped);
    let image = downscale_and_encode(cropped, opts.max_edge)?;
    Ok(Some(Frame { image, dhash }))
}

impl FrameSource for WindowFrameSource {
    fn frame(&self, window_id: u32, opts: FrameOpts) -> CoreResult<Option<Frame>> {
        let captured = match self
            .capturer
            .capture(CaptureTarget::Window(window_id), CAPTURE_MAX_EDGE)
        {
            Ok(captured) => captured,
            // The window is gone, off screen or blank: nothing usable, not a failure. A
            // missing permission is one, and the caller reports it.
            Err(CoreError::Platform(_) | CoreError::NotFound(_)) => return Ok(None),
            Err(other) => return Err(other),
        };
        let Ok(decoded) = image::load_from_memory(&captured.bytes) else {
            return Ok(None);
        };
        // `captured` and `decoded` are dropped here: the bytes never leave this call other
        // than as the returned frame.
        frame_from(&decoded.to_rgba8(), opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use ubiqx_core::ports::EncodedImage;

    use crate::mock::{DeniedCapturer, SyntheticCapturer};

    const BROWSER: FrameOpts = FrameOpts {
        max_edge: 640,
        crop_top_frac: 0.12,
    };
    const OTHER_APP: FrameOpts = FrameOpts {
        max_edge: 640,
        crop_top_frac: 0.0,
    };

    /// Serves one fixed picture and records what it was asked for.
    struct FixedCapturer {
        picture: RgbaImage,
        asked: Mutex<Vec<(CaptureTarget, u32)>>,
    }

    impl FixedCapturer {
        fn new(picture: RgbaImage) -> Arc<Self> {
            Arc::new(Self {
                picture,
                asked: Mutex::new(Vec::new()),
            })
        }
    }

    impl ScreenCapturer for FixedCapturer {
        fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage> {
            self.asked.lock().push((target, max_edge));
            downscale_and_encode(self.picture.clone(), max_edge)
        }

        fn visible_apps(&self) -> CoreResult<Vec<String>> {
            Ok(Vec::new())
        }
    }

    /// A window whose top 12% is white chrome over a dark page with a bright band.
    fn browser_window() -> RgbaImage {
        RgbaImage::from_fn(1280, 800, |x, y| {
            if y < 96 {
                image::Rgba([255, 255, 255, 255])
            } else if (300..500).contains(&y) {
                image::Rgba([(x % 256) as u8, 200, 90, 255])
            } else {
                image::Rgba([20, 20, 30, 255])
            }
        })
    }

    fn decode(frame: &Frame) -> RgbaImage {
        image::load_from_memory(&frame.image.bytes)
            .unwrap()
            .to_rgba8()
    }

    #[test]
    fn frame_captures_the_window_only_and_reduces_it() {
        let capturer = FixedCapturer::new(browser_window());
        let source = WindowFrameSource::new(capturer.clone());
        let frame = source.frame(41, BROWSER).unwrap().unwrap();
        // One capture, of that window, never a display or a region.
        assert_eq!(
            *capturer.asked.lock(),
            vec![(CaptureTarget::Window(41), CAPTURE_MAX_EDGE)]
        );
        assert_eq!(frame.image.mime, "image/jpeg");
        assert!(frame.image.bytes.starts_with(&[0xFF, 0xD8]));
        assert_eq!(frame.image.width, 640);
        // 800 rows minus the top 12% (96 rows), halved.
        assert_eq!(frame.image.height, 352);
        assert!(frame.image.bytes.len() < 60_000, "a reduced frame is small");
    }

    #[test]
    fn frame_crops_the_browser_chrome_and_nothing_of_other_apps() {
        let source = WindowFrameSource::new(FixedCapturer::new(browser_window()));
        let cropped = decode(&source.frame(1, BROWSER).unwrap().unwrap());
        let top = cropped.get_pixel(320, 2);
        assert!(
            top[0] < 80 && top[1] < 80,
            "the white chrome is gone: {top:?}"
        );
        let whole = source.frame(1, OTHER_APP).unwrap().unwrap();
        assert_eq!((whole.image.width, whole.image.height), (640, 400));
        let top = decode(&whole).get_pixel(320, 2).0;
        assert!(top[0] > 200 && top[1] > 200, "nothing is cropped: {top:?}");
    }

    #[test]
    fn frame_hash_is_stable_under_re_encoding_and_differs_between_pictures() {
        let source = WindowFrameSource::new(FixedCapturer::new(browser_window()));
        let first = source.frame(1, BROWSER).unwrap().unwrap();
        let second = source.frame(1, BROWSER).unwrap().unwrap();
        assert_eq!(first.dhash, second.dhash);
        // The frame itself, decoded and hashed again, is the same picture.
        let again = dhash(&decode(&first));
        assert!(
            (first.dhash ^ again).count_ones() <= 4,
            "{:064b} vs {again:064b}",
            first.dhash
        );
        let other = RgbaImage::from_fn(1280, 800, |x, y| {
            let v = if (x / 160 + y / 100) % 2 == 0 {
                230
            } else {
                15
            };
            image::Rgba([v, v, v, 255])
        });
        let different = WindowFrameSource::new(FixedCapturer::new(other))
            .frame(1, BROWSER)
            .unwrap()
            .unwrap();
        assert!(
            (first.dhash ^ different.dhash).count_ones() >= 12,
            "different pictures, close hashes"
        );
    }

    #[test]
    fn frame_of_a_near_uniform_capture_is_no_frame() {
        let black = RgbaImage::from_pixel(1280, 800, image::Rgba([0, 0, 0, 255]));
        let source = WindowFrameSource::new(FixedCapturer::new(black));
        assert_eq!(source.frame(1, BROWSER).unwrap(), None);
        // Chrome over an empty page: uniform once the chrome is cropped.
        let empty_page = RgbaImage::from_fn(1280, 800, |x, y| {
            if y < 90 {
                image::Rgba([(x % 256) as u8, 40, 200, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        });
        let source = WindowFrameSource::new(FixedCapturer::new(empty_page));
        assert_eq!(source.frame(1, BROWSER).unwrap(), None);
        assert!(source.frame(1, OTHER_APP).unwrap().is_some());
    }

    #[test]
    fn frame_of_a_window_that_is_gone_is_no_frame_and_a_denied_capture_is_an_error() {
        struct Gone;
        impl ScreenCapturer for Gone {
            fn capture(&self, _: CaptureTarget, _: u32) -> CoreResult<EncodedImage> {
                Err(CoreError::Platform("window 9 not found".into()))
            }
            fn visible_apps(&self) -> CoreResult<Vec<String>> {
                Ok(Vec::new())
            }
        }
        assert_eq!(
            WindowFrameSource::new(Arc::new(Gone))
                .frame(9, BROWSER)
                .unwrap(),
            None
        );
        let denied = WindowFrameSource::new(Arc::new(DeniedCapturer)).frame(9, BROWSER);
        assert!(matches!(denied, Err(CoreError::Permission(_))));
        // Bytes that are not an image are nothing usable either.
        struct Garbage;
        impl ScreenCapturer for Garbage {
            fn capture(&self, _: CaptureTarget, _: u32) -> CoreResult<EncodedImage> {
                Ok(EncodedImage {
                    bytes: vec![1, 2, 3],
                    mime: "image/jpeg".into(),
                    width: 1,
                    height: 1,
                })
            }
            fn visible_apps(&self) -> CoreResult<Vec<String>> {
                Ok(Vec::new())
            }
        }
        assert_eq!(
            WindowFrameSource::new(Arc::new(Garbage))
                .frame(9, BROWSER)
                .unwrap(),
            None
        );
    }

    #[test]
    fn the_platform_services_carry_the_frame_source() {
        let (services, _) =
            crate::PlatformServices::scripted(crate::mock::Scenario::new(Vec::new()));
        let frame = services.frames.frame(3, BROWSER).unwrap().unwrap();
        assert_eq!(frame.image.width, 640);
        assert!(frame.image.bytes.starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn frame_works_over_the_synthetic_capturer() {
        let frame = WindowFrameSource::new(Arc::new(SyntheticCapturer))
            .frame(3, BROWSER)
            .unwrap()
            .unwrap();
        assert_eq!(frame.image.width, 640);
    }
}
