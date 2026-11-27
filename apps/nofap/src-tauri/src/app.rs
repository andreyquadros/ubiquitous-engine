//! The standalone shell: plugins, the main window, the commands (`nofap`, `nofap_host`,
//! `quit_app`), the sink and the notifier; the card window is in `card.rs` and the tray in
//! `tray.rs`. It starts `ubiqx_app::nofap_app::NofapApp`, which never constructs an
//! `Engine`: no sampler, tracker, classifier or report exists in this process. No decision
//! lives here.

use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewWindow, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;
use ubiqx_app::nofap_app::{NofapApp, NofapAppConfig, NofapHostCommand};
use ubiqx_core::nofap::{NofapEvent, NofapSink};
use ubiqx_core::nofap_ipc::NofapCommand;
use ubiqx_core::ports::{EventSink, Notifier, SettingsRepo};
use ubiqx_core::{BuildInfo, CoreError, CoreResult, EngineEvent};

use crate::{card, tray};

/// Shared state handed to every command.
pub struct AppState {
    pub app: NofapApp,
}

/// Name of the event every module event is emitted under (`src/nofap/ipc.ts`).
const EVENT: &str = "nofap";

/// Name of the event a newer build is announced under (the settings page listens).
const UPDATE_EVENT: &str = "update";

/// The error a command rejects with: `{ code, message }`, as in ubiqX's shell.
#[derive(Debug, Serialize)]
pub struct IpcError {
    code: &'static str,
    message: String,
}

impl From<CoreError> for IpcError {
    fn from(e: CoreError) -> Self {
        let code = match &e {
            CoreError::Storage(_) => "storage",
            CoreError::Platform(_) => "platform",
            CoreError::Permission(_) => "permission",
            CoreError::Ai(_) => "ai",
            CoreError::AiNotConfigured => "ai_not_configured",
            CoreError::AiRejected(_) => "ai_rejected",
            CoreError::AiRefused => "ai_refused",
            CoreError::RateLimited { .. } => "rate_limited",
            CoreError::LicenseRequired => "license_required",
            CoreError::Invalid(_) => "invalid",
            CoreError::NotFound(_) => "not_found",
            CoreError::Other(_) => "other",
        };
        Self {
            code,
            message: e.to_string(),
        }
    }
}

fn join_error(e: impl std::fmt::Display) -> IpcError {
    IpcError::from(CoreError::Other(e.to_string()))
}

/// The module's command: `{ cmd, args }` in, the reply of that command out. On the
/// blocking pool: it reads the database, and some commands wait for the system's password
/// prompt.
///
/// The card window may send only what its page sends
/// ([`ubiqx_app::CARD_WINDOW_COMMANDS`]); anything else it sends is refused here, since a
/// capability allows a whole command (integrator decision 34).
#[tauri::command]
async fn nofap(
    app: AppHandle,
    window: WebviewWindow,
    command: NofapCommand,
) -> Result<Value, IpcError> {
    if window.label() == card::CARD_WINDOW && !ubiqx_app::card_window_may_send(&command) {
        return Err(IpcError::from(CoreError::Permission(format!(
            "{} is not answered to the card window",
            command.name()
        ))));
    }
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().app.handle(command))
        .await
        .map_err(join_error)?
        .map_err(IpcError::from)
}

/// The standalone product's own command: its settings, the AI key, permissions, updates,
/// about. On the blocking pool, like the module's command and ubiqX's key and permission
/// commands: the keychain can wait for the user to unlock it and a permission request can
/// raise a system prompt; the vendor calls are awaited there. A settings write relabels the
/// tray (the language may have changed).
///
/// The card window may send this command only to read the settings (its page follows the
/// app's language, read again for every card); everything else it sends is refused here,
/// since a capability allows a whole command (integrator decision 34).
#[tauri::command]
async fn nofap_host(
    app: AppHandle,
    window: WebviewWindow,
    command: NofapHostCommand,
) -> Result<Value, IpcError> {
    if window.label() == card::CARD_WINDOW && !card_may_send(&command) {
        return Err(IpcError::from(CoreError::Permission(format!(
            "{} is not answered to the card window",
            command.name()
        ))));
    }
    let relabel = matches!(command, NofapHostCommand::SettingsSet(_));
    let handle = app.clone();
    let reply = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(handle.state::<AppState>().app.handle_host(command))
    })
    .await
    .map_err(join_error)??;
    if relabel {
        tray::sync_later(&app);
    }
    Ok(reply)
}

/// What the card window's page sends through `nofap_host`: only the settings read.
pub(crate) fn card_may_send(command: &NofapHostCommand) -> bool {
    matches!(command, NofapHostCommand::SettingsGet)
}

/// The window's own way out (design 6.3): the page asks for the confirmation first, with
/// the same sentence as the tray's.
#[tauri::command]
fn quit_app(app: AppHandle) {
    quit(&app);
}

/// Quits for good: the module counts a quit while it is on and gives up the primary lock
/// (a database write, so off the main thread), the loops end, then the process.
pub fn quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(state) = app.try_state::<AppState>() {
            let service = state.app.service.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || service.on_quit()).await;
            state.app.stop_loops().await;
        }
        app.exit(0);
    });
}

/// Shows the main window, at a hash route of the page when one is given.
pub fn show_main_route(app: &AppHandle, route: Option<&str>) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    if let Some(hash) = route.and_then(|r| serde_json::to_string(r).ok()) {
        let _ = w.eval(format!("window.location.hash = {hash}"));
    }
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
}

/// [`NofapSink`]: every event goes to the webview as `nofap`. A status change also
/// refreshes the tray, on a thread of its own: the sink never calls back into the service
/// from the thread that emits.
struct TauriNofapSink(AppHandle);

impl NofapSink for TauriNofapSink {
    fn emit(&self, event: NofapEvent) {
        if matches!(event, NofapEvent::Status { .. }) {
            tray::sync_later(&self.0);
        }
        if let Err(e) = self.0.emit(EVENT, &event) {
            tracing::debug!(error = %e, "could not emit a nofap event");
        }
    }
}

/// The update checker's events: a newer build is announced to the webview.
struct TauriUpdateSink(AppHandle);

impl EventSink for TauriUpdateSink {
    fn emit(&self, event: EngineEvent) {
        if let EngineEvent::UpdateAvailable { .. } = &event {
            if let Err(e) = self.0.emit(UPDATE_EVENT, &event) {
                tracing::debug!(error = %e, "could not emit the update event");
            }
        }
    }
}

/// Desktop notifications through the notification plugin (the card's fallback).
struct TauriNotifier(AppHandle);

impl Notifier for TauriNotifier {
    fn notify(&self, title: &str, body: &str) -> CoreResult<()> {
        self.0
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| CoreError::Platform(format!("notification: {e}")))
    }
}

/// The build identity stamped by `build.rs`; `version` is the one of `tauri.conf.json`,
/// which is what the updater compares (see ubiqX's shell).
fn build_info(version: &str) -> BuildInfo {
    BuildInfo::from_stamp(
        version,
        env!("UBIQX_BUILD_EPOCH"),
        env!("UBIQX_BUILD_NUMBER"),
        env!("UBIQX_BUILD_SHA"),
        env!("UBIQX_BUILD_BRANCH"),
    )
}

fn logger() -> tauri_plugin_log::Builder {
    tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Info)
        // The HTTP stack writes a line per request at debug and trace, naming the provider:
        // here the screen check is the only AI call, so that would be a line per check
        // (hand-off U3, integrator decision 25).
        .level_for("hyper_util", log::LevelFilter::Warn)
        .level_for("hyper", log::LevelFilter::Warn)
        .level_for("reqwest", log::LevelFilter::Warn)
        .level_for("rustls", log::LevelFilter::Warn)
        // The single-instance plugin answers a second launch over D-Bus on Linux.
        .level_for("zbus", log::LevelFilter::Warn)
}

pub fn run() {
    let minimized = std::env::args().any(|a| a == "--minimized");
    tauri::Builder::default()
        // First, so a second launch only brings the running app to the front.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_route(app, None)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(logger().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                app.set_menu(tray::app_menu(app.handle())?)?;
            }
            let handle = app.handle().clone();
            // Its own data directory (`ai.ubiqx.shield`): database, exports and the secrets
            // file Windows and Linux fall back to (integrator decision 25).
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app data dir: {e}"))?;
            let build = build_info(&app.package_info().version.to_string());
            tracing::info!(version = %build.version, number = build.number, sha = %build.sha, "standalone build");
            let config = NofapAppConfig {
                data_dir: Some(data_dir),
                notifier: Some(Arc::new(TauriNotifier(handle.clone()))),
                presenter: Some(Arc::new(card::TauriNofapPresenter(handle.clone()))),
                update_sink: Some(Arc::new(TauriUpdateSink(handle.clone()))),
                // `main` hands `--shield-helper` runs to the helper before Tauri starts,
                // so this executable can be the elevated one.
                system_layer: true,
                build,
                update_feed_url: Some(env!("UBIQX_SHIELD_UPDATE_FEED_URL").to_string()),
                ..NofapAppConfig::default()
            };
            let sink: Arc<dyn NofapSink> = Arc::new(TauriNofapSink(handle.clone()));
            let nofap = tauri::async_runtime::block_on(async { NofapApp::start(config, sink) })
                .map_err(|e| format!("start: {e}"))?;
            let onboarding_done = nofap.store.load().map(|s| s.onboarding_done).unwrap_or(false);
            app.manage(AppState { app: nofap });
            tray::build(&handle)?;
            tray::sync_later(&handle);
            if !minimized || !onboarding_done {
                show_main_route(&handle, None);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Closing the window does not quit (the guard keeps running): it hides.
                // The card window is kept too, so the next card only shows it again.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![nofap, nofap_host, quit_app])
        .build(tauri::generate_context!())
        .expect("error while building the tauri application")
        .run(|app, event| match event {
            // Only `quit` (which calls `exit`) ends the process; the last window hiding
            // does not.
            RunEvent::ExitRequested { api, code, .. } if code.is_none() => api.prevent_exit(),
            RunEvent::Exit => {
                // A logout or a system shutdown can end the process without `quit`: at least
                // stop the loops and give up the primary lock.
                if let Some(state) = app.try_state::<AppState>() {
                    state.app.shutdown();
                    state.app.service.release_primary();
                }
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_card_window_may_only_read_the_settings() {
        let wire = [
            json!({ "cmd": "settings_get" }),
            json!({ "cmd": "settings_set", "args": { "ui_language": "en" } }),
            json!({ "cmd": "api_key_set", "args": { "provider": "openai", "key": "sk-ana" } }),
            json!({ "cmd": "api_key_clear", "args": { "provider": "openai" } }),
            json!({ "cmd": "models", "args": { "provider": "openai" } }),
            json!({ "cmd": "permissions" }),
            json!({ "cmd": "permission_request", "args": { "kind": "screen_recording" } }),
            json!({ "cmd": "update_check" }),
            json!({ "cmd": "about" }),
        ];
        for value in wire {
            let command: NofapHostCommand =
                serde_json::from_value(value.clone()).unwrap_or_else(|e| panic!("{value}: {e}"));
            assert_eq!(
                card_may_send(&command),
                command.name() == "settings_get",
                "{}",
                command.name()
            );
        }
    }
}
