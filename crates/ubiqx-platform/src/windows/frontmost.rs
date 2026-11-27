//! Foreground window and process identity through Win32.

use std::ffi::c_void;
use std::mem::size_of;

use ubiqx_core::ports::ActivitySource;
use ubiqx_core::{CoreResult, ForegroundWindow};
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    GWL_EXSTYLE, GW_OWNER, WS_EX_TOOLWINDOW,
};

use super::util::{file_description, from_wide};
use crate::app_id::exe_stem;

/// The `u32` the engine keeps for a window: the HWND value (they fit in 32 bits).
pub fn hwnd_id(hwnd: HWND) -> u32 {
    hwnd.0 as usize as u32
}

pub fn hwnd_from_id(id: u32) -> HWND {
    HWND(id as usize as *mut c_void)
}

/// Process id owning a window.
pub fn window_pid(hwnd: HWND) -> Option<u32> {
    let mut pid = 0u32;
    // SAFETY: `pid` is a valid out-pointer for the call's duration.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

/// Full path of a process's executable (`QueryFullProcessImageNameW`), which needs only
/// `PROCESS_QUERY_LIMITED_INFORMATION` and therefore works for elevated processes too.
pub fn process_image_path(pid: u32) -> Option<String> {
    // SAFETY: the handle is closed before returning; the buffer length is passed along.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 2048];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        result.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// App id and human name of a process: the executable stem and its `FileDescription`
/// (or the stem when the resource is missing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub app_id: String,
    pub app_name: String,
    pub path: Option<String>,
}

pub fn process_identity(pid: u32) -> Option<ProcessIdentity> {
    let path = process_image_path(pid)?;
    let app_id = exe_stem(&path);
    if app_id.is_empty() {
        return None;
    }
    let app_name = file_description(&path).unwrap_or_else(|| app_id.clone());
    Some(ProcessIdentity {
        app_id,
        app_name,
        path: Some(path),
    })
}

/// Executable stem of the frame that hosts UWP/Store apps (Settings, Calculator, Photos,
/// the Store builds of WhatsApp or Spotify…). Their foreground window belongs to this
/// host, and the app itself owns a child window (`Windows.UI.Core.CoreWindow`).
pub const APPLICATION_FRAME_HOST: &str = "applicationframehost";

/// The first child window of `hwnd` owned by another process, if any.
pub fn foreign_child_window(hwnd: HWND, frame_pid: u32) -> Option<HWND> {
    struct Search {
        frame_pid: u32,
        found: Option<HWND>,
    }
    unsafe extern "system" fn visit(child: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the Search pointer passed below, alive for the whole call.
        let search = unsafe { &mut *(lparam.0 as *mut Search) };
        match window_pid(child) {
            Some(pid) if pid != search.frame_pid => {
                search.found = Some(child);
                BOOL(0)
            }
            _ => BOOL(1),
        }
    }
    let mut search = Search {
        frame_pid,
        found: None,
    };
    // SAFETY: the callback only touches the Search it is handed; a FALSE return stops it.
    let _ = unsafe {
        EnumChildWindows(
            Some(hwnd),
            Some(visit),
            LPARAM(&mut search as *mut Search as isize),
        )
    };
    search.found
}

/// The window whose process is the application the user sees: `hwnd` itself, or, when
/// `hwnd` is an ApplicationFrameHost frame, the hosted app's core window and its pid.
/// The frame window stays the one to report (id, title, bounds).
pub fn hosted_app(hwnd: HWND, pid: u32, identity: &ProcessIdentity) -> Option<(HWND, u32)> {
    if identity.app_id != APPLICATION_FRAME_HOST {
        return None;
    }
    let child = foreign_child_window(hwnd, pid)?;
    let child_pid = window_pid(child)?;
    Some((child, child_pid))
}

/// Identity and pid of the application behind a top-level window, seeing through the
/// ApplicationFrameHost frame of UWP apps: the pid is the hosted app's, so that closing or
/// terminating it does not take every other Store app down with the host.
pub fn window_identity(hwnd: HWND) -> Option<(ProcessIdentity, u32)> {
    let pid = window_pid(hwnd)?;
    let identity = process_identity(pid)?;
    if let Some((_, child_pid)) = hosted_app(hwnd, pid, &identity) {
        if let Some(hosted) = process_identity(child_pid) {
            return Some((hosted, child_pid));
        }
    }
    Some((identity, pid))
}

pub fn window_title(hwnd: HWND) -> String {
    // SAFETY: the buffer is sized from the reported length plus the terminator.
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, &mut buf);
        from_wide(&buf[..n.max(0) as usize])
    }
}

/// The window's frame in screen pixels `(x, y, width, height)`: the DWM extended frame
/// (what the user sees, without the invisible resize borders), else `GetWindowRect`.
pub fn window_rect(hwnd: HWND) -> Option<(i32, i32, u32, u32)> {
    let mut rect = RECT::default();
    // SAFETY: `rect` is a valid RECT out-pointer of the stated size.
    let dwm = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut rect as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        )
    };
    if dwm.is_err() {
        // SAFETY: same out-pointer.
        unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    }
    let w = (rect.right - rect.left).max(0) as u32;
    let h = (rect.bottom - rect.top).max(0) as u32;
    Some((rect.left, rect.top, w, h))
}

/// Every top-level window, front to back (`EnumWindows` order).
pub fn top_level_windows() -> Vec<HWND> {
    unsafe extern "system" fn push(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the Vec pointer passed below, alive for the whole call.
        let out = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        out.push(hwnd);
        BOOL(1)
    }
    let mut out: Vec<HWND> = Vec::new();
    // SAFETY: the callback only touches the Vec it is handed.
    let _ = unsafe { EnumWindows(Some(push), LPARAM(&mut out as *mut Vec<HWND> as isize)) };
    out
}

/// A window the user can see as an application window: visible, not minimised, not
/// owned (dialogs, tooltips) and not a tool window.
pub fn is_app_window(hwnd: HWND) -> bool {
    // SAFETY: plain queries on a window handle; an invalid handle answers false/0.
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return false;
        }
        if GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.0.is_null()) {
            return false;
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        ex & WS_EX_TOOLWINDOW.0 == 0
    }
}

fn unknown_identity(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        app_id: format!("pid-{pid}"),
        app_name: "Desconhecido".into(),
        path: None,
    }
}

/// [`ActivitySource`] over `GetForegroundWindow`.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsActivitySource;

impl ActivitySource for WindowsActivitySource {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        // SAFETY: no arguments; a null handle means no foreground window.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return Ok(None);
        }
        let Some(pid) = window_pid(hwnd) else {
            return Ok(None);
        };
        let mut identity = process_identity(pid).unwrap_or_else(|| unknown_identity(pid));
        let mut app_pid = pid;
        let mut title = window_title(hwnd);
        // UWP apps: the frame belongs to ApplicationFrameHost.exe, the app to a child window.
        if let Some((child, child_pid)) = hosted_app(hwnd, pid, &identity) {
            if let Some(hosted) = process_identity(child_pid) {
                identity = hosted;
                app_pid = child_pid;
                if title.is_empty() {
                    title = window_title(child);
                }
            }
        }
        Ok(Some(ForegroundWindow {
            app_name: identity.app_name,
            app_id: identity.app_id,
            window_title: title,
            pid: Some(app_pid),
            window_id: Some(hwnd_id(hwnd)),
            bounds: window_rect(hwnd),
        }))
    }
}
