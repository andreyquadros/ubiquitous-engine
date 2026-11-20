//! Screenshots through X11 `GetImage`: one window when its id is known (its own contents,
//! so other applications' windows only leak in where they overlap it), else the screen.
//! Under Wayland there is no X image to read: the error is a plain platform error, which
//! the engine already tolerates (it keeps tracking without screenshots).
//! [`X11WindowCapturer`] never reads the screen in place of a window: it is what the
//! `nofap` frame source uses, whose picture leaves the computer.

use ubiqx_core::ports::{CaptureTarget, EncodedImage, ScreenCapturer};
use ubiqx_core::{CoreError, CoreResult};

use super::frontmost::app_id_of;
use super::x11::X11;
use crate::image_util::{downscale_and_encode, is_uniform};

fn no_display() -> CoreError {
    CoreError::Platform("capture: no X11 display (Wayland needs XWayland)".into())
}

/// A rectangle as `GetImage` takes it: x, y, width, height.
type Rect = (i16, i16, u16, u16);

/// The part of window `id` that is on the screen: its origin on the screen and the
/// rectangle inside the window.
fn visible_part(x: &X11, id: u32) -> CoreResult<((i16, i16), Rect)> {
    let (wx, wy, w, h) = x
        .window_rect(id)
        .ok_or_else(|| CoreError::Platform(format!("window {id} not found")))?;
    let (sw, sh) = x.screen_size();
    // Clip to the screen: GetImage fails for the off-screen part of a window.
    let left = wx.max(0);
    let top = wy.max(0);
    let right = (wx + w as i32).min(i32::from(sw));
    let bottom = (wy + h as i32).min(i32::from(sh));
    if right <= left || bottom <= top {
        return Err(CoreError::Platform("window is off screen".into()));
    }
    let rect = (
        (left - wx) as i16,
        (top - wy) as i16,
        (right - left) as u16,
        (bottom - top) as u16,
    );
    Ok(((left as i16, top as i16), rect))
}

fn capture_window(x: &X11, id: u32) -> CoreResult<image::RgbaImage> {
    let ((left, top), rect) = visible_part(x, id)?;
    x.image(id, rect)
        .or_else(|| {
            // An unmapped or redirected window: read the screen under it instead.
            x.image(x.root(), (left, top, rect.2, rect.3))
        })
        .ok_or_else(|| CoreError::Platform(format!("window {id} could not be read")))
}

/// Window `id` read from the window itself, or nothing: the screen under it, where
/// another application's window may be, is never read in its place.
fn capture_window_only(x: &X11, id: u32) -> CoreResult<image::RgbaImage> {
    let (_, rect) = visible_part(x, id)?;
    x.image(id, rect)
        .ok_or_else(|| CoreError::Platform(format!("window {id} could not be read")))
}

fn capture_screen(x: &X11) -> CoreResult<image::RgbaImage> {
    let (w, h) = x.screen_size();
    x.image(x.root(), (0, 0, w, h))
        .ok_or_else(|| CoreError::Platform("screen could not be read".into()))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct X11ScreenCapturer;

impl ScreenCapturer for X11ScreenCapturer {
    fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage> {
        let x = X11::connect().ok_or_else(no_display)?;
        let img = match target {
            CaptureTarget::Window(id) => capture_window(&x, id)?,
            // Multi-head layouts are one big X screen: the whole of it is what we have.
            CaptureTarget::DisplayAt { .. } | CaptureTarget::PrimaryDisplay => capture_screen(&x)?,
        };
        if is_uniform(&img) {
            return Err(CoreError::Platform("capture is blank".into()));
        }
        downscale_and_encode(img, max_edge)
    }

    /// App ids and `WM_CLASS` classes of every viewable managed window.
    fn visible_apps(&self) -> CoreResult<Vec<String>> {
        let Some(x) = X11::connect() else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for w in x.client_list() {
            if !x.is_viewable(w) {
                continue;
            }
            let (instance, class) = x.window_class(w).unwrap_or_default();
            let id = app_id_of(x.window_pid(w), &instance, &class);
            if !id.is_empty() {
                out.push(id);
            }
            if !class.trim().is_empty() {
                out.push(class.trim().to_string());
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}

/// Captures one window and nothing else, for the `nofap` frame source
/// ([`crate::frame`]): the picture goes to an AI provider, so it must never show the
/// screen in place of the window. A display target is refused, and a window that cannot
/// be read is an error instead of a crop of the screen under it.
#[derive(Debug, Default, Clone, Copy)]
pub struct X11WindowCapturer;

impl ScreenCapturer for X11WindowCapturer {
    fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage> {
        let CaptureTarget::Window(id) = target else {
            return Err(CoreError::Platform(crate::frame::WINDOW_ONLY.into()));
        };
        let x = X11::connect().ok_or_else(no_display)?;
        let img = capture_window_only(&x, id)?;
        if is_uniform(&img) {
            return Err(CoreError::Platform("capture is blank".into()));
        }
        downscale_and_encode(img, max_edge)
    }

    fn visible_apps(&self) -> CoreResult<Vec<String>> {
        X11ScreenCapturer.visible_apps()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_the_window_only_capturer_cannot_read_is_no_frame() {
        use ubiqx_core::nofap::{FrameOpts, FrameSource};
        // No display, or a display without such a window: nothing is read in its place.
        let source = crate::frame::WindowFrameSource::new(std::sync::Arc::new(X11WindowCapturer));
        let opts = FrameOpts {
            max_edge: 640,
            crop_top_frac: 0.12,
        };
        assert_eq!(source.frame(u32::MAX, opts).unwrap(), None);
    }

    #[test]
    fn the_window_only_capturer_refuses_a_display_before_it_connects() {
        for target in [
            CaptureTarget::PrimaryDisplay,
            CaptureTarget::DisplayAt { x: 10, y: 10 },
        ] {
            let err = X11WindowCapturer.capture(target, 800).unwrap_err();
            assert!(
                matches!(&err, CoreError::Platform(why) if why == crate::frame::WINDOW_ONLY),
                "{err:?}"
            );
        }
        if !super::super::x11::display_available() {
            let err = X11WindowCapturer
                .capture(CaptureTarget::Window(1), 800)
                .unwrap_err();
            assert!(matches!(err, CoreError::Platform(_)));
        }
    }

    #[test]
    fn without_a_display_capture_is_a_platform_error() {
        if !super::super::x11::display_available() {
            let err = X11ScreenCapturer
                .capture(CaptureTarget::PrimaryDisplay, 800)
                .unwrap_err();
            assert!(matches!(err, CoreError::Platform(_)));
            assert!(X11ScreenCapturer.visible_apps().unwrap().is_empty());
        }
    }
}
