//! Frontmost application and window.

use std::ffi::c_void;

use objc2_app_kit::NSWorkspace;
use objc2_core_foundation::{CFArray, CFDictionary, CFNumber, CFNumberType, CFString, CGRect};
use objc2_core_graphics::{
    CGRectMakeWithDictionaryRepresentation, CGWindowListCopyWindowInfo, CGWindowListOption,
};
use ubiqx_core::ports::ActivitySource;
use ubiqx_core::{CoreResult, ForegroundWindow};

/// One row of `CGWindowListCopyWindowInfo`.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowInfo {
    pub id: u32,
    pub pid: i32,
    pub owner: String,
    /// `None` when Screen Recording is not granted (or the window has no title).
    pub name: Option<String>,
    pub layer: i32,
    pub bounds: (i32, i32, u32, u32),
}

fn dict_value(dict: &CFDictionary, key: &str) -> *const c_void {
    let key = CFString::from_str(key);
    let key_ptr: *const CFString = &*key;
    // SAFETY: `dict` is a valid CFDictionary from the window server; the key outlives the call.
    unsafe { dict.value(key_ptr.cast()) }
}

fn dict_i32(dict: &CFDictionary, key: &str) -> Option<i32> {
    let v = dict_value(dict, key);
    if v.is_null() {
        return None;
    }
    let mut out: i32 = 0;
    // SAFETY: values under these keys are CFNumbers per the CGWindow documentation.
    let ok = unsafe {
        let num: &CFNumber = &*(v as *const CFNumber);
        num.value(CFNumberType::SInt32Type, (&mut out as *mut i32).cast())
    };
    ok.then_some(out)
}

fn dict_string(dict: &CFDictionary, key: &str) -> Option<String> {
    let v = dict_value(dict, key);
    if v.is_null() {
        return None;
    }
    // SAFETY: values under kCGWindowName / kCGWindowOwnerName are CFStrings.
    let s: &CFString = unsafe { &*(v as *const CFString) };
    Some(s.to_string())
}

fn dict_rect(dict: &CFDictionary, key: &str) -> Option<CGRect> {
    let v = dict_value(dict, key);
    if v.is_null() {
        return None;
    }
    let mut rect = CGRect::default();
    // SAFETY: kCGWindowBounds is a CFDictionary in CGRect dictionary representation.
    let ok = unsafe {
        let d: &CFDictionary = &*(v as *const CFDictionary);
        CGRectMakeWithDictionaryRepresentation(Some(d), &mut rect)
    };
    ok.then_some(rect)
}

/// Lists on-screen windows, front to back, in one pass over the window server data.
pub fn list_windows() -> Vec<WindowInfo> {
    let Some(array) = CGWindowListCopyWindowInfo(
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements,
        0,
    ) else {
        return Vec::new();
    };
    let array: &CFArray = &array;
    let count = array.count();
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        // SAFETY: indices are within bounds; elements are CFDictionaries.
        let dict_ptr = unsafe { array.value_at_index(i) } as *const CFDictionary;
        if dict_ptr.is_null() {
            continue;
        }
        let dict: &CFDictionary = unsafe { &*dict_ptr };
        let (Some(id), Some(pid)) = (
            dict_i32(dict, "kCGWindowNumber"),
            dict_i32(dict, "kCGWindowOwnerPID"),
        ) else {
            continue;
        };
        let bounds = dict_rect(dict, "kCGWindowBounds")
            .map(|r| {
                (
                    r.origin.x as i32,
                    r.origin.y as i32,
                    r.size.width.max(0.0) as u32,
                    r.size.height.max(0.0) as u32,
                )
            })
            .unwrap_or((0, 0, 0, 0));
        out.push(WindowInfo {
            id: id as u32,
            pid,
            owner: dict_string(dict, "kCGWindowOwnerName").unwrap_or_default(),
            name: dict_string(dict, "kCGWindowName"),
            layer: dict_i32(dict, "kCGWindowLayer").unwrap_or(0),
            bounds,
        });
    }
    out
}

/// The frontmost application as AppKit sees it. Needs no permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmostApp {
    pub pid: i32,
    pub bundle_id: Option<String>,
    pub name: Option<String>,
    pub executable: Option<String>,
}

pub fn frontmost_app() -> Option<FrontmostApp> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    Some(FrontmostApp {
        pid: app.processIdentifier(),
        bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
        name: app.localizedName().map(|s| s.to_string()),
        executable: app
            .executableURL()
            .and_then(|u| u.path())
            .map(|p| p.to_string()),
    })
}

/// Picks the window of `pid` that is most likely focused: the frontmost normal-layer window
/// with a reasonable size.
pub fn focused_window_of(windows: &[WindowInfo], pid: i32) -> Option<&WindowInfo> {
    windows
        .iter()
        .find(|w| w.pid == pid && w.layer == 0 && w.bounds.2 >= 50 && w.bounds.3 >= 50)
        .or_else(|| windows.iter().find(|w| w.pid == pid))
}

/// `ActivitySource` combining AppKit (application identity) and CoreGraphics (window title).
#[derive(Debug, Default)]
pub struct AppKitActivitySource;

impl ActivitySource for AppKitActivitySource {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        let Some(app) = frontmost_app() else {
            return Ok(None);
        };
        let windows = list_windows();
        let win = focused_window_of(&windows, app.pid);
        let app_name = app
            .name
            .clone()
            .or_else(|| win.map(|w| w.owner.clone()))
            .unwrap_or_else(|| "Desconhecido".into());
        let app_id = app
            .bundle_id
            .clone()
            .or_else(|| app.executable.clone())
            .unwrap_or_else(|| app_name.clone());
        Ok(Some(ForegroundWindow {
            app_name,
            app_id,
            window_title: win.and_then(|w| w.name.clone()).unwrap_or_default(),
            pid: Some(app.pid as u32),
            window_id: win.map(|w| w.id),
            bounds: win.map(|w| w.bounds),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_window_prefers_normal_layer_and_size() {
        let ws = vec![
            WindowInfo {
                id: 1,
                pid: 9,
                owner: "A".into(),
                name: None,
                layer: 25,
                bounds: (0, 0, 800, 600),
            },
            WindowInfo {
                id: 2,
                pid: 9,
                owner: "A".into(),
                name: Some("tiny".into()),
                layer: 0,
                bounds: (0, 0, 10, 10),
            },
            WindowInfo {
                id: 3,
                pid: 9,
                owner: "A".into(),
                name: Some("main".into()),
                layer: 0,
                bounds: (0, 0, 900, 700),
            },
        ];
        assert_eq!(focused_window_of(&ws, 9).unwrap().id, 3);
        assert!(focused_window_of(&ws, 1).is_none());
    }
}
