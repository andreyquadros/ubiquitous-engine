//! Screenshots through `xcap` (GDI / Desktop Duplication), cropped to the foreground window.
//!
//! A window capture by id is tried first; when xcap cannot see the window (a different
//! desktop, a UWP host) the display under the window is captured and cropped to its DWM
//! extended frame, so other applications' windows only leak in where they overlap it.

use ubiqx_core::ports::{CaptureTarget, EncodedImage, ScreenCapturer};
use ubiqx_core::{CoreError, CoreResult};

use super::frontmost::{
    hwnd_from_id, is_app_window, top_level_windows, window_identity, window_rect, window_title,
};
use crate::image_util::{downscale_and_encode, is_uniform};

fn xcap_err(e: xcap::XCapError) -> CoreError {
    CoreError::Platform(format!("capture: {e}"))
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

/// Crops `img` (a display capture whose origin is `(ox, oy)` in screen pixels) to `rect`.
pub fn crop_to(
    img: &image::RgbaImage,
    (ox, oy): (i32, i32),
    (x, y, w, h): (i32, i32, u32, u32),
) -> Option<image::RgbaImage> {
    let left = (x - ox).max(0) as u32;
    let top = (y - oy).max(0) as u32;
    if left >= img.width() || top >= img.height() {
        return None;
    }
    let right = ((x - ox) + w as i32).clamp(0, img.width() as i32) as u32;
    let bottom = ((y - oy) + h as i32).clamp(0, img.height() as i32) as u32;
    if right <= left || bottom <= top {
        return None;
    }
    Some(image::imageops::crop_imm(img, left, top, right - left, bottom - top).to_image())
}

fn capture_window(id: u32) -> CoreResult<image::RgbaImage> {
    if let Ok(windows) = xcap::Window::all() {
        if let Some(w) = windows.into_iter().find(|w| w.id().ok() == Some(id)) {
            match w.capture_image() {
                Ok(img) if img.width() > 0 && img.height() > 0 => return Ok(img),
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!(window = id, error = %e, "window capture failed; cropping the display")
                }
            }
        }
    }
    let rect = window_rect(hwnd_from_id(id))
        .ok_or_else(|| CoreError::Platform(format!("window {id} not found")))?;
    let (x, y, w, h) = rect;
    if w == 0 || h == 0 {
        return Err(CoreError::Platform("window has no size".into()));
    }
    let monitor =
        xcap::Monitor::from_point(x + (w / 2) as i32, y + (h / 2) as i32).map_err(xcap_err)?;
    let origin = (
        monitor.x().map_err(xcap_err)?,
        monitor.y().map_err(xcap_err)?,
    );
    let img = monitor.capture_image().map_err(xcap_err)?;
    crop_to(&img, origin, rect).ok_or_else(|| CoreError::Platform("window is off screen".into()))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsScreenCapturer;

impl ScreenCapturer for WindowsScreenCapturer {
    fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage> {
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

    /// App ids *and* names of every visible application window, so a block list written
    /// either way matches.
    fn visible_apps(&self) -> CoreResult<Vec<String>> {
        let mut out: Vec<String> = Vec::new();
        for hwnd in top_level_windows() {
            if !is_app_window(hwnd) || window_title(hwnd).is_empty() {
                continue;
            }
            let Some((identity, _)) = window_identity(hwnd) else {
                continue;
            };
            out.push(identity.app_id);
            out.push(identity.app_name);
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}
