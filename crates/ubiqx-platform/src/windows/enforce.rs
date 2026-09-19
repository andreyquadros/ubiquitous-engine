//! What the focus guard does to a distraction on Windows: closes an application's windows
//! (`WM_CLOSE`, then `TerminateProcess` for what survives), closes or blanks the active
//! browser tab with keyboard shortcuts sent through `SendInput`, minimises the other
//! windows, and runs a command line through `cmd /C`.
//!
//! No permission is involved. Keyboard shortcuts only reach the foreground window, so the
//! tab actions check that the browser still owns it before and after.

use std::mem::size_of;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;

use ubiqx_core::ports::Enforcer;
use ubiqx_core::{CoreError, CoreResult};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WAIT_TIMEOUT, WPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_CONTROL, VK_L, VK_RETURN, VK_W,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, PostMessageW, ShowWindow, SW_MINIMIZE, WM_CLOSE,
};

use super::frontmost::{
    is_app_window, top_level_windows, window_identity, window_pid, window_title,
};
use crate::app_id::is_self_stem;

/// How long an application gets to honour `WM_CLOSE` before its process is terminated.
const CLOSE_GRACE: Duration = Duration::from_millis(1500);

/// Pause between the shortcut and the check that the browser kept the foreground.
const KEY_SETTLE: Duration = Duration::from_millis(150);

/// `CREATE_NO_WINDOW`: no console window flashes for `cmd /C`.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Top-level windows (with the pid of the application behind them, the hosted one for
/// UWP frames) owned by processes whose app id is `stem`.
fn windows_of(stem: &str) -> Vec<(HWND, u32)> {
    let stem = stem.trim().to_lowercase();
    let me = std::process::id();
    top_level_windows()
        .into_iter()
        .filter_map(|hwnd| {
            if window_pid(hwnd)? == me {
                return None;
            }
            let (identity, pid) = window_identity(hwnd)?;
            (identity.app_id == stem).then_some((hwnd, pid))
        })
        .collect()
}

/// App id of the application behind the foreground window.
fn foreground_stem() -> Option<String> {
    // SAFETY: no arguments.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    window_identity(hwnd).map(|(i, _)| i.app_id)
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn unicode(unit: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: if up {
                    KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                } else {
                    KEYEVENTF_UNICODE
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Sends the inputs as one batch; true when all of them were queued.
fn send(inputs: &[INPUT]) -> bool {
    if inputs.is_empty() {
        return true;
    }
    // SAFETY: `inputs` is a valid slice of initialised INPUT structs of the stated size.
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    sent as usize == inputs.len()
}

/// `Ctrl+<key>`.
fn ctrl(vk: VIRTUAL_KEY) -> [INPUT; 4] {
    [
        key(VK_CONTROL, false),
        key(vk, false),
        key(vk, true),
        key(VK_CONTROL, true),
    ]
}

/// Types `text` as Unicode key events.
fn typed(text: &str) -> Vec<INPUT> {
    text.encode_utf16()
        .flat_map(|u| [unicode(u, false), unicode(u, true)])
        .collect()
}

/// The browser must own the foreground window for a shortcut to reach it.
fn browser_in_front(browser: &str) -> bool {
    foreground_stem().is_some_and(|s| s == browser.trim().to_lowercase())
}

/// [`Enforcer`] for Windows.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsEnforcer;

impl Enforcer for WindowsEnforcer {
    fn quit_app(&self, bundle_id: &str) -> CoreResult<bool> {
        let stem = bundle_id.trim().to_lowercase();
        if stem.is_empty() || is_self_stem(&stem) {
            return Ok(false);
        }
        let targets = windows_of(&stem);
        if targets.is_empty() {
            return Ok(false);
        }
        for (hwnd, _) in &targets {
            // SAFETY: posting a standard message to a window handle we just enumerated.
            let _ = unsafe { PostMessageW(Some(*hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        std::thread::sleep(CLOSE_GRACE);
        let mut pids: Vec<u32> = targets.iter().map(|(_, pid)| *pid).collect();
        pids.sort_unstable();
        pids.dedup();
        let mut terminated = 0;
        for pid in pids {
            // SAFETY: the handle is closed on every path; a process that already exited
            // fails `OpenProcess` or reports signalled and is left alone.
            unsafe {
                let Ok(handle) = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
                    false,
                    pid,
                ) else {
                    continue;
                };
                if WaitForSingleObject(handle, 0) == WAIT_TIMEOUT
                    && TerminateProcess(handle, 1).is_ok()
                {
                    terminated += 1;
                }
                let _ = CloseHandle(handle);
            }
        }
        tracing::info!(
            app = %stem,
            windows = targets.len(),
            terminated,
            "focus guard closed an app"
        );
        Ok(true)
    }

    fn close_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool> {
        if !browser_in_front(browser_bundle_id) {
            return Ok(false);
        }
        if !send(&ctrl(VK_W)) {
            return Ok(false);
        }
        std::thread::sleep(KEY_SETTLE);
        // The browser closing its last tab (and itself) also counts as done.
        Ok(browser_in_front(browser_bundle_id) || windows_of(browser_bundle_id).is_empty())
    }

    fn blank_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool> {
        if !browser_in_front(browser_bundle_id) {
            return Ok(false);
        }
        let mut inputs: Vec<INPUT> = ctrl(VK_L).to_vec();
        inputs.extend(typed("about:blank"));
        inputs.push(key(VK_RETURN, false));
        inputs.push(key(VK_RETURN, true));
        if !send(&inputs) {
            return Ok(false);
        }
        std::thread::sleep(KEY_SETTLE);
        Ok(browser_in_front(browser_bundle_id))
    }

    fn hide_others(&self, keep_bundle_id: &str) -> CoreResult<bool> {
        let keep = keep_bundle_id.trim().to_lowercase();
        let me = std::process::id();
        let mut any = false;
        for hwnd in top_level_windows() {
            if !is_app_window(hwnd) || window_title(hwnd).is_empty() {
                continue;
            }
            if window_pid(hwnd).is_none_or(|pid| pid == me) {
                continue;
            }
            let Some((identity, _)) = window_identity(hwnd) else {
                continue;
            };
            if identity.app_id == keep || is_self_stem(&identity.app_id) {
                continue;
            }
            // SAFETY: minimising a window we just enumerated.
            let _ = unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
            any = true;
        }
        Ok(any)
    }

    fn run_shortcut(&self, name: &str) -> CoreResult<bool> {
        let command = name.trim();
        if command.is_empty() {
            return Ok(false);
        }
        // Spawned, never awaited here: the session must start right away. The exit status
        // is logged by a watcher thread.
        let child = Command::new("cmd")
            .args(["/C", command])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CoreError::Platform(format!("cmd /C: {e}")))?;
        let command = command.to_string();
        std::thread::Builder::new()
            .name("ubiqx-command".into())
            .spawn(move || match child.wait_with_output() {
                Ok(out) if out.status.success() => {
                    tracing::info!(command = %command, "session command ran");
                }
                Ok(out) => tracing::warn!(
                    command = %command,
                    status = %out.status,
                    stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                    "session command failed"
                ),
                Err(e) => {
                    tracing::warn!(command = %command, error = %e, "session command did not finish")
                }
            })
            .map_err(|e| CoreError::Platform(format!("command watcher: {e}")))?;
        Ok(true)
    }
}
