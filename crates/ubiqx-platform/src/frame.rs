//! The frame the AI check of the `nofap` module looks at: one reduced image of one window,
//! in memory only.
//!
//! [`WindowFrameSource`] is the one portable implementation of
//! `ubiqx_core::nofap::FrameSource`. It calls the existing
//! `ScreenCapturer::capture(CaptureTarget::Window(id), 1280)`, decodes, crops the top
//! fraction asked for (a browser's tab strip and address bar), downscales to the edge asked
//! for, computes a 64-bit difference hash and encodes JPEG. No per-OS code, no file, no
//! `screenshots` row, no thumbnail; there is no display or region fallback, and a
//! near-uniform capture is "no frame".
//!
//! Seeded by the foundation unit: the type and its constructor.
#![allow(unused_variables, dead_code)]

use std::sync::Arc;

use ubiqx_core::nofap::{Frame, FrameOpts, FrameSource};
use ubiqx_core::ports::ScreenCapturer;
use ubiqx_core::CoreResult;

/// Takes frames through the platform's screen capturer.
pub struct WindowFrameSource {
    capturer: Arc<dyn ScreenCapturer>,
}

impl WindowFrameSource {
    pub fn new(capturer: Arc<dyn ScreenCapturer>) -> Self {
        Self { capturer }
    }
}

impl FrameSource for WindowFrameSource {
    fn frame(&self, window_id: u32, opts: FrameOpts) -> CoreResult<Option<Frame>> {
        todo!()
    }
}
