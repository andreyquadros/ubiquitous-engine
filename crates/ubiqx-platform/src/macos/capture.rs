//! Screenshots through `xcap` (CoreGraphics `CGWindowListCreateImage`).
//!
//! We capture a single window whenever we know its id, so other applications' windows are
//! never part of the image. `ScreenCaptureKit` is the documented upgrade path for macOS 15+.

use ubiqx_core::ports::{CaptureTarget, EncodedImage, ScreenCapturer};
use ubiqx_core::{CoreError, CoreResult};

use crate::image_util::{downscale_and_encode, is_uniform};

fn xcap_err(e: xcap::XCapError) -> CoreError {
    CoreError::Platform(format!("capture: {e}"))
}

fn capture_window(id: u32) -> CoreResult<image::RgbaImage> {
    let window = xcap::Window::all()
        .map_err(xcap_err)?
        .into_iter()
        .find(|w| w.id().ok() == Some(id))
        .ok_or_else(|| CoreError::Platform(format!("window {id} not found")))?;
    window.capture_image().map_err(xcap_err)
}

fn capture_display_at(x: i32, y: i32) -> CoreResult<image::RgbaImage> {
    xcap::Monitor::from_point(x, y)
        .map_err(xcap_err)?
        .capture_image()
        .map_err(xcap_err)
}

fn capture_primary() -> CoreResult<image::RgbaImage> {
    let monitors = xcap::Monitor::all().map_err(xcap_err)?;
    let primary = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())
        .ok_or_else(|| CoreError::Platform("no display".into()))?;
    primary.capture_image().map_err(xcap_err)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CgScreenCapturer;

impl ScreenCapturer for CgScreenCapturer {
    fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage> {
        if !super::permissions::screen_recording_granted() {
            return Err(CoreError::Permission("screen recording".into()));
        }
        let img = match target {
            CaptureTarget::Window(id) => capture_window(id)?,
            CaptureTarget::DisplayAt { x, y } => capture_display_at(x, y)?,
            CaptureTarget::PrimaryDisplay => capture_primary()?,
        };
        if is_uniform(&img) {
            return Err(CoreError::Platform("capture is blank".into()));
        }
        downscale_and_encode(img, max_edge)
    }

    /// Owner names *and* bundle ids of every on-screen normal-layer window, so callers may
    /// match a block list written either way (the default one uses bundle ids).
    fn visible_apps(&self) -> CoreResult<Vec<String>> {
        use objc2_app_kit::NSRunningApplication;
        let mut out: Vec<String> = Vec::new();
        for w in super::frontmost::list_windows()
            .into_iter()
            .filter(|w| w.layer == 0)
        {
            if !w.owner.is_empty() {
                out.push(w.owner.clone());
            }
            if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(w.pid)
            {
                if let Some(bid) = app.bundleIdentifier() {
                    out.push(bid.to_string());
                }
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}
