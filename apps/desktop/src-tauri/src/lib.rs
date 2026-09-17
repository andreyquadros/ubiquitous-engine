//! ubiqX desktop shell: a menubar (tray) application hosting the engine.

mod commands;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;
use ubiqx_app::{App, AppConfig};
use ubiqx_core::ports::{EventSink, Notifier};
use ubiqx_core::{CoreResult, EngineEvent};
use ubiqx_engine::PrivateModeDuration;

/// Shared state handed to every command.
pub struct AppState {
    pub app: App,
}

/// Forwards engine events to the webview.
struct TauriSink(AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, event: EngineEvent) {
        if let Err(e) = self.0.emit("engine", &event) {
            tracing::debug!(error = %e, "could not emit engine event");
        }
    }
}

/// Desktop notifications through the notification plugin.
struct TauriNotifier(AppHandle);

impl Notifier for TauriNotifier {
    fn notify(&self, title: &str, body: &str) -> CoreResult<()> {
        self.0
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| ubiqx_core::CoreError::Platform(format!("notification: {e}")))
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir ubiqX", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pausar rastreamento", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Retomar rastreamento", true, None::<&str>)?;
    let p30 = MenuItem::with_id(app, "private_30", "30 minutos", true, None::<&str>)?;
    let p60 = MenuItem::with_id(app, "private_60", "1 hora", true, None::<&str>)?;
    let ptom = MenuItem::with_id(app, "private_tomorrow", "Até amanhã", true, None::<&str>)?;
    let pind = MenuItem::with_id(
        app,
        "private_indefinite",
        "Até eu desligar",
        true,
        None::<&str>,
    )?;
    let poff = MenuItem::with_id(
        app,
        "private_off",
        "Desligar modo privado",
        true,
        None::<&str>,
    )?;
    let private = Submenu::with_items(
        app,
        "Modo privado",
        true,
        &[
            &p30,
            &p60,
            &ptom,
            &pind,
            &PredefinedMenuItem::separator(app)?,
            &poff,
        ],
    )?;
    let snooze = MenuItem::with_id(app, "snooze", "Silenciar o UBI por 2 h", true, None::<&str>)?;
    let report = MenuItem::with_id(
        app,
        "report_today",
        "Gerar relatórios de hoje",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Sair do ubiqX", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &pause,
            &resume,
            &private,
            &snooze,
            &PredefinedMenuItem::separator(app)?,
            &report,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    // Monochrome (black + alpha) glyph rendered by macOS as a template image, so it
    // follows the menubar appearance (light/dark). 44 px source keeps Retina crisp.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray@2x.png"))
        .expect("embedded tray icon is a valid PNG");
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("ubiqX")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            let engine = state.app.engine.clone();
            match event.id().as_ref() {
                "open" => show_main_window(app),
                "pause" => engine.pause(),
                "resume" => engine.resume(),
                "private_30" => log_err(engine.set_private_mode(PrivateModeDuration::Minutes30)),
                "private_60" => log_err(engine.set_private_mode(PrivateModeDuration::Hour1)),
                "private_tomorrow" => {
                    log_err(engine.set_private_mode(PrivateModeDuration::UntilTomorrow))
                }
                "private_indefinite" => {
                    log_err(engine.set_private_mode(PrivateModeDuration::Indefinite))
                }
                "private_off" => log_err(engine.set_private_mode(PrivateModeDuration::Off)),
                "snooze" => log_err(engine.snooze_nudges(120)),
                "report_today" => {
                    let engine = engine.clone();
                    tauri::async_runtime::spawn(async move {
                        let today = ubiqx_engine::today();
                        let cats = engine
                            .state()
                            .deps
                            .repos
                            .categories
                            .list(false)
                            .unwrap_or_default();
                        for c in cats.into_iter().filter(|c| !c.is_system) {
                            if let Err(e) = engine.generate_report(today, &c.id).await {
                                tracing::warn!(error = %e, "report generation failed");
                            }
                        }
                    });
                }
                "quit" => {
                    engine.shutdown();
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

fn log_err(r: CoreResult<()>) {
    if let Err(e) = r {
        tracing::warn!(error = %e, "tray action failed");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let minimized = std::env::args().any(|a| a == "--minimized");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app data dir: {e}"))?;
            let sink: Arc<dyn EventSink> = Arc::new(TauriSink(handle.clone()));
            let notifier: Arc<dyn Notifier> = Arc::new(TauriNotifier(handle.clone()));
            let ai = if std::env::var("UBIQX_FAKE_AI").is_ok() {
                ubiqx_app::AiBackend::Fake
            } else {
                ubiqx_app::AiBackend::Anthropic
            };
            let scripted = if std::env::var("UBIQX_SCRIPTED").is_ok() {
                Some(ubiqx_app::ubiqx_platform::mock::Scenario::demo_day().looping())
            } else {
                None
            };
            let config = AppConfig {
                data_dir: Some(data_dir),
                in_memory_db: false,
                scripted_platform: scripted,
                ai,
                notifier: Some(notifier),
            };
            let ubiqx = tauri::async_runtime::block_on(async { App::start(config, sink) })
                .map_err(|e| format!("engine start: {e}"))?;
            if ubiqx.scripted.is_some() {
                let _ = ubiqx_app::demo::seed(ubiqx.engine.state().deps.repos.categories.as_ref());
            }

            // Keep the login-item setting in sync with the persisted preference.
            let launch = ubiqx.engine.settings().launch_at_login;
            let autostart = app.autolaunch();
            let _ = if launch {
                autostart.enable()
            } else {
                autostart.disable()
            };

            let onboarding_done = ubiqx.engine.settings().onboarding_done;
            app.manage(AppState { app: ubiqx });
            build_tray(app)?;

            if !minimized || !onboarding_done {
                show_main_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Closing the dashboard must not stop tracking: hide instead.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(commands::handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                // Only the tray "Sair" item (which calls app.exit) may end the process.
                if code.is_none() {
                    api.prevent_exit();
                } else if let Some(state) = app.try_state::<AppState>() {
                    state.app.engine.shutdown();
                }
            }
        });
}
