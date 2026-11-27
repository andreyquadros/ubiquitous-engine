//! What the focus guard does to a distraction on Linux: `SIGTERM` to the application's
//! processes, `Ctrl+W` / `Ctrl+L about:blank ⏎` to the focused browser through XTest,
//! iconification of the other windows through the window manager, and `sh -c` for the
//! session commands.
//!
//! Everything needs an X11 display (see the module docs of [`super`]); without one every
//! action answers `Ok(false)` except `quit_app` and `run_shortcut`, which do not touch the
//! display.

use std::process::{Command, Stdio};
use std::time::Duration;

use ubiqx_core::ports::Enforcer;
use ubiqx_core::{CoreError, CoreResult};

use super::frontmost::app_id_of;
use super::proc::pids_of_stem;
use super::x11::{KEYSYM_CONTROL_L, KEYSYM_RETURN, X11};
use crate::app_id::is_self_stem;

/// Pause between the shortcut and the check that the browser kept the focus.
const KEY_SETTLE: Duration = Duration::from_millis(150);

/// App id of the active window.
fn active_app_id(x: &X11) -> Option<String> {
    let w = x.active_window()?;
    let (instance, class) = x.window_class(w).unwrap_or_default();
    let id = app_id_of(x.window_pid(w), &instance, &class);
    (!id.is_empty()).then_some(id)
}

fn browser_in_front(x: &X11, browser: &str) -> bool {
    active_app_id(x).is_some_and(|id| id == browser.trim().to_lowercase())
}

/// [`Enforcer`] for Linux.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxEnforcer;

impl Enforcer for LinuxEnforcer {
    fn quit_app(&self, bundle_id: &str) -> CoreResult<bool> {
        let stem = bundle_id.trim().to_lowercase();
        if stem.is_empty() || is_self_stem(&stem) {
            return Ok(false);
        }
        let pids = pids_of_stem(&stem);
        if pids.is_empty() {
            return Ok(false);
        }
        let mut any = false;
        for pid in &pids {
            // SAFETY: `kill` with SIGTERM on a pid we own; a vanished pid answers ESRCH.
            let r = unsafe { libc::kill(*pid as libc::pid_t, libc::SIGTERM) };
            any |= r == 0;
        }
        // Debug and without the app: nofap's "Fechar a janela" quits through here, and the
        // app's log file (info and above) never gets the app.
        tracing::debug!(
            processes = pids.len(),
            signalled = any,
            "focus guard asked an app to quit"
        );
        Ok(any)
    }

    fn close_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool> {
        let Some(x) = X11::connect() else {
            return Ok(false);
        };
        if !browser_in_front(&x, browser_bundle_id) {
            return Ok(false);
        }
        if !x.tap(&[KEYSYM_CONTROL_L], u32::from('w')) {
            return Ok(false);
        }
        std::thread::sleep(KEY_SETTLE);
        // The browser closing its last tab (and itself) also counts as done.
        Ok(browser_in_front(&x, browser_bundle_id) || pids_of_stem(browser_bundle_id).is_empty())
    }

    fn blank_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool> {
        let Some(x) = X11::connect() else {
            return Ok(false);
        };
        if !browser_in_front(&x, browser_bundle_id) {
            return Ok(false);
        }
        let ok = x.tap(&[KEYSYM_CONTROL_L], u32::from('l'))
            && x.type_text("about:blank")
            && x.tap(&[], KEYSYM_RETURN);
        if !ok {
            return Ok(false);
        }
        std::thread::sleep(KEY_SETTLE);
        Ok(browser_in_front(&x, browser_bundle_id))
    }

    fn hide_others(&self, keep_bundle_id: &str) -> CoreResult<bool> {
        let Some(x) = X11::connect() else {
            return Ok(false);
        };
        let keep = keep_bundle_id.trim().to_lowercase();
        let me = std::process::id();
        let mut any = false;
        for w in x.client_list() {
            if !x.is_viewable(w) {
                continue;
            }
            let pid = x.window_pid(w);
            if pid == Some(me) {
                continue;
            }
            let (instance, class) = x.window_class(w).unwrap_or_default();
            let id = app_id_of(pid, &instance, &class);
            if id.is_empty() || id == keep || is_self_stem(&id) {
                continue;
            }
            any |= x.iconify(w);
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
        let child = Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CoreError::Platform(format!("sh -c: {e}")))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_acts_on_itself_or_nothing() {
        assert!(!LinuxEnforcer.quit_app("").unwrap());
        assert!(!LinuxEnforcer.quit_app("ubiqx").unwrap());
        assert!(!LinuxEnforcer
            .quit_app("definitely-not-running-xyz")
            .unwrap());
        assert!(!LinuxEnforcer.run_shortcut("   ").unwrap());
    }

    /// Every event logged while it is the thread's subscriber: its level and its fields.
    struct Capture(std::sync::Arc<parking_lot::Mutex<Vec<(tracing::Level, String)>>>);

    impl tracing::Subscriber for Capture {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            struct Fields(String);
            impl tracing::field::Visit for Fields {
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    self.0.push_str(&format!(" {}={value:?}", field.name()));
                }
            }
            let mut fields = Fields(String::new());
            event.record(&mut fields);
            self.0.lock().push((*event.metadata().level(), fields.0));
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    /// "Fechar a janela" on a nofap card quits the app behind it. The shells keep the info
    /// level in their log file, and the privacy promise is that the file never gets the
    /// app: quitting one leaves no line at info or above, and none that names it.
    #[test]
    fn quitting_an_app_logs_nothing_that_names_it() {
        use std::os::unix::process::CommandExt;
        let Some(sleep) = ["/bin/sleep", "/usr/bin/sleep"]
            .into_iter()
            .find(|path| std::path::Path::new(path).exists())
        else {
            return;
        };
        // A copy under a name nothing else runs, so only this child is signalled. Started
        // as `sleep`, for a binary that serves several tools by the name it is called by.
        let dir = tempfile::tempdir().unwrap();
        let stem = "ubiqxquitprobe";
        let copy = dir.path().join(stem);
        std::fs::copy(sleep, &copy).unwrap();
        let Ok(mut child) = Command::new(&copy).arg0("sleep").arg("30").spawn() else {
            eprintln!("skipped: a copied program cannot run from the temporary directory");
            return;
        };
        let events = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
        let quit = tracing::subscriber::with_default(Capture(events.clone()), || {
            LinuxEnforcer.quit_app(stem)
        });
        let _ = child.kill();
        let _ = child.wait();
        assert!(quit.unwrap(), "the probe was not signalled");
        let events = events.lock();
        for (level, fields) in events.iter() {
            assert!(*level > tracing::Level::INFO, "{level} {fields}");
            assert!(!fields.contains(stem), "{level} {fields}");
        }
    }

    #[test]
    fn runs_a_command_line() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("ran");
        let cmd = format!("touch '{}'", marker.display());
        assert!(LinuxEnforcer.run_shortcut(&cmd).unwrap());
        for _ in 0..50 {
            if marker.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("command did not run");
    }

    #[test]
    fn display_actions_fail_softly_without_a_server() {
        if !super::super::x11::display_available() {
            assert!(!LinuxEnforcer.close_active_tab("firefox").unwrap());
            assert!(!LinuxEnforcer.blank_active_tab("firefox").unwrap());
            assert!(!LinuxEnforcer.hide_others("code").unwrap());
        }
    }
}
