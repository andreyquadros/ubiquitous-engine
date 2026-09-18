//! ubiqX desktop shell: a menubar (tray) application hosting the engine, plus the small
//! always-on-top window UBI uses when the focus guard holds a distraction.

mod commands;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, RunEvent, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_notification::NotificationExt;
use ubiqx_app::{App, AppConfig};
use ubiqx_core::ports::{EventSink, InterventionPresenter, Notifier};
use ubiqx_core::{
    BuildInfo, CoreError, CoreResult, EngineEvent, Intervention, ReleaseInfo, UiLanguage,
};
use ubiqx_engine::PrivateModeDuration;

/// Shared state handed to every command.
pub struct AppState {
    pub app: App,
}

/// Labels of the tray and application menus, per UI language.
struct MenuText {
    open: &'static str,
    pause: &'static str,
    resume: &'static str,
    private_mode: &'static str,
    private_30: &'static str,
    private_60: &'static str,
    private_tomorrow: &'static str,
    private_indefinite: &'static str,
    private_off: &'static str,
    snooze: &'static str,
    /// The update item while no newer build is known.
    check_updates: &'static str,
    /// The update item once a build is available: `(date, sha)` of that build.
    download_update: fn(&str, &str) -> String,
    report_today: &'static str,
    quit: &'static str,
    hide_window: &'static str,
}

impl MenuText {
    fn for_language(lang: UiLanguage) -> Self {
        match lang {
            UiLanguage::PtBr => Self {
                open: "Abrir ubiqX",
                pause: "Pausar rastreamento",
                resume: "Retomar rastreamento",
                private_mode: "Modo privado",
                private_30: "30 minutos",
                private_60: "1 hora",
                private_tomorrow: "Até amanhã",
                private_indefinite: "Até eu desligar",
                private_off: "Desligar modo privado",
                snooze: "Silenciar o UBI por 2 h",
                check_updates: "Verificar atualizações…",
                download_update: |date, sha| format!("Baixar a nova versão ({date} {sha})…"),
                report_today: "Gerar relatórios de hoje",
                quit: "Sair do ubiqX",
                hide_window: "Fechar janela",
            },
            UiLanguage::En => Self {
                open: "Open ubiqX",
                pause: "Pause tracking",
                resume: "Resume tracking",
                private_mode: "Private mode",
                private_30: "30 minutes",
                private_60: "1 hour",
                private_tomorrow: "Until tomorrow",
                private_indefinite: "Until I turn it off",
                private_off: "Turn off private mode",
                snooze: "Mute UBI for 2 h",
                check_updates: "Check for updates…",
                download_update: |date, sha| format!("Download the new version ({date} {sha})…"),
                report_today: "Generate today's reports",
                quit: "Quit ubiqX",
                hide_window: "Close Window",
            },
        }
    }
}

/// The menu items whose labels follow the UI language, kept so a settings change can relabel
/// them in place (the item ids never change, so the event handlers are untouched).
pub struct Menus {
    open: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    resume: MenuItem<tauri::Wry>,
    private_mode: Submenu<tauri::Wry>,
    private_30: MenuItem<tauri::Wry>,
    private_60: MenuItem<tauri::Wry>,
    private_tomorrow: MenuItem<tauri::Wry>,
    private_indefinite: MenuItem<tauri::Wry>,
    private_off: MenuItem<tauri::Wry>,
    snooze: MenuItem<tauri::Wry>,
    update: MenuItem<tauri::Wry>,
    report_today: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
    hide_window: MenuItem<tauri::Wry>,
}

impl Menus {
    /// Rewrites every label in `lang`; the update item names the build in `available`.
    pub fn relabel(&self, lang: UiLanguage, available: Option<&ReleaseInfo>) -> tauri::Result<()> {
        let t = MenuText::for_language(lang);
        self.update.set_text(update_label(&t, lang, available))?;
        self.open.set_text(t.open)?;
        self.pause.set_text(t.pause)?;
        self.resume.set_text(t.resume)?;
        self.private_mode.set_text(t.private_mode)?;
        self.private_30.set_text(t.private_30)?;
        self.private_60.set_text(t.private_60)?;
        self.private_tomorrow.set_text(t.private_tomorrow)?;
        self.private_indefinite.set_text(t.private_indefinite)?;
        self.private_off.set_text(t.private_off)?;
        self.snooze.set_text(t.snooze)?;
        self.report_today.set_text(t.report_today)?;
        self.quit.set_text(t.quit)?;
        self.hide_window.set_text(t.hide_window)?;
        Ok(())
    }
}

/// The label of the tray's update item: an invitation to check, or the build to download.
fn update_label(t: &MenuText, lang: UiLanguage, available: Option<&ReleaseInfo>) -> String {
    match available {
        Some(r) => (t.download_update)(
            &ubiqx_core::update::format_build_date(r.build.epoch, lang),
            &r.build.sha,
        ),
        None => t.check_updates.to_string(),
    }
}

/// Relabels the tray and application menus after a settings write (called by the
/// `update_settings` command) or once a newer build is known; a no-op before the menus
/// exist.
pub fn relabel_menus(app: &AppHandle, lang: UiLanguage) {
    let Some(menus) = app.try_state::<Menus>() else {
        return;
    };
    let available = app
        .try_state::<AppState>()
        .and_then(|s| s.app.engine.update_status().available);
    if let Err(e) = menus.relabel(lang, available.as_ref()) {
        tracing::warn!(error = %e, "could not relabel the menus");
    }
}

/// Forwards engine events to the webview. A newer build also renames the tray item, so the
/// menubar offers the download without opening the dashboard.
struct TauriSink(AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, event: EngineEvent) {
        if let EngineEvent::UpdateAvailable { .. } = &event {
            if let Some(state) = self.0.try_state::<AppState>() {
                let lang = state.app.engine.settings().ui_language();
                relabel_menus(&self.0, lang);
            }
        }
        if let Err(e) = self.0.emit("engine", &event) {
            tracing::debug!(error = %e, "could not emit engine event");
        }
    }
}

/// Opens the download of a build in the browser. Only web links, for the same reason as
/// `open_external`: the URL came from a JSON feed, never from the user.
pub fn open_download(url: &str) -> CoreResult<()> {
    let parsed = url::Url::parse(url.trim())
        .map_err(|_| CoreError::Invalid(format!("download url: {url}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(CoreError::Invalid(format!(
            "download url: only http(s) links are opened ({url})"
        )));
    }
    tauri_plugin_opener::open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| CoreError::Platform(format!("open download: {e}")))
}

/// The build identity stamped by `build.rs` (`UBIQX_BUILD_*`), a development build when the
/// stamp is empty.
fn build_info() -> BuildInfo {
    BuildInfo::from_stamp(
        env!("CARGO_PKG_VERSION"),
        env!("UBIQX_BUILD_EPOCH"),
        env!("UBIQX_BUILD_NUMBER"),
        env!("UBIQX_BUILD_SHA"),
        env!("UBIQX_BUILD_BRANCH"),
    )
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

// ------------------------------------------------------------------------------------------
// Intervention window
// ------------------------------------------------------------------------------------------

/// Label of the window UBI speaks from when the focus guard holds something.
pub const INTERVENTION_WINDOW: &str = "intervention";

/// Logical size of the intervention panel (matches the page's fixed layout).
const INTERVENTION_SIZE: (f64, f64) = (460.0, 188.0);

/// Gap between the top of the monitor and the panel, in logical pixels.
const INTERVENTION_TOP_GAP: f64 = 24.0;

/// Hash route the page mounts for an intervention id (or `test` for the sample).
fn intervention_route(id: &str) -> String {
    let id: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    format!("#/intervention?id={id}")
}

/// Where the panel goes: centred horizontally, [`INTERVENTION_TOP_GAP`] below the top of the
/// monitor under the cursor (the primary monitor when that cannot be told).
fn intervention_position(app: &AppHandle) -> Option<PhysicalPosition<i32>> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor();
    let width = (INTERVENTION_SIZE.0 * scale).round() as i32;
    let x = monitor.position().x + (monitor.size().width as i32 - width) / 2;
    let y = monitor.position().y + (INTERVENTION_TOP_GAP * scale).round() as i32;
    Some(PhysicalPosition::new(x, y))
}

/// Shows the intervention window for `id`: built on first use, then re-pointed at the new
/// id and re-shown. The page closes itself after a few seconds or on its button.
pub fn show_intervention_window(app: &AppHandle, id: &str) -> CoreResult<()> {
    let route = intervention_route(id);
    let position = intervention_position(app);
    let window = match app.get_webview_window(INTERVENTION_WINDOW) {
        Some(w) => {
            // A hash change re-renders the route without reloading the page.
            w.eval(format!(
                "window.location.hash = {}",
                serde_json::to_string(&route).unwrap_or_else(|_| "\"#/intervention\"".into())
            ))
            .map_err(|e| CoreError::Platform(format!("intervention window: {e}")))?;
            w
        }
        None => {
            let mut builder = WebviewWindowBuilder::new(
                app,
                INTERVENTION_WINDOW,
                WebviewUrl::App(format!("index.html{route}").into()),
            )
            .title("ubiqX")
            .inner_size(INTERVENTION_SIZE.0, INTERVENTION_SIZE.1)
            .decorations(false)
            .shadow(true)
            .resizable(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false);
            if let Some(p) = position {
                builder = builder.position(p.x as f64, p.y as f64);
            }
            builder
                .build()
                .map_err(|e| CoreError::Platform(format!("intervention window: {e}")))?
        }
    };
    let _ = window.set_size(LogicalSize::new(INTERVENTION_SIZE.0, INTERVENTION_SIZE.1));
    if let Some(p) = position {
        let _ = window.set_position(p);
    }
    let _ = window.set_always_on_top(true);
    window
        .show()
        .map_err(|e| CoreError::Platform(format!("intervention window: {e}")))?;
    Ok(())
}

/// [`InterventionPresenter`] backed by the intervention window. Every window also receives
/// the intervention itself as an `intervention` event, so the page can render it without a
/// round trip.
struct TauriPresenter(AppHandle);

impl InterventionPresenter for TauriPresenter {
    fn show(&self, intervention: &Intervention) -> CoreResult<bool> {
        show_intervention_window(&self.0, &intervention.id)?;
        if let Err(e) = self.0.emit("intervention", intervention) {
            tracing::debug!(error = %e, "could not emit the intervention event");
        }
        Ok(true)
    }
}

/// Builds the tray (labels in `lang`) and returns its relabelable items, without the
/// application-menu item that [`build_app_menu`] adds.
fn build_tray(
    app: &tauri::App,
    lang: UiLanguage,
    hide_window: MenuItem<tauri::Wry>,
) -> tauri::Result<Menus> {
    let t = MenuText::for_language(lang);
    let open = MenuItem::with_id(app, "open", t.open, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", t.pause, true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", t.resume, true, None::<&str>)?;
    let p30 = MenuItem::with_id(app, "private_30", t.private_30, true, None::<&str>)?;
    let p60 = MenuItem::with_id(app, "private_60", t.private_60, true, None::<&str>)?;
    let ptom = MenuItem::with_id(
        app,
        "private_tomorrow",
        t.private_tomorrow,
        true,
        None::<&str>,
    )?;
    let pind = MenuItem::with_id(
        app,
        "private_indefinite",
        t.private_indefinite,
        true,
        None::<&str>,
    )?;
    let poff = MenuItem::with_id(app, "private_off", t.private_off, true, None::<&str>)?;
    let private = Submenu::with_items(
        app,
        t.private_mode,
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
    let snooze = MenuItem::with_id(app, "snooze", t.snooze, true, None::<&str>)?;
    let update = MenuItem::with_id(app, "update", t.check_updates, true, None::<&str>)?;
    let report = MenuItem::with_id(app, "report_today", t.report_today, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", t.quit, true, None::<&str>)?;
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
            &update,
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
                "update" => {
                    // A known build downloads right away; otherwise check now and, when
                    // that finds one, download it, or else show the dashboard (its update
                    // section says when the check ran and what it found).
                    let engine = engine.clone();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let mut available = engine.update_status().available;
                        if available.is_none() {
                            match engine.check_for_updates().await {
                                Ok(status) => available = status.available,
                                Err(e) => tracing::warn!(error = %e, "update check failed"),
                            }
                        }
                        match available {
                            Some(release) => log_err(open_download(&release.download_url)),
                            None => show_main_window(&app),
                        }
                    });
                }
                "report_today" => {
                    let engine = engine.clone();
                    tauri::async_runtime::spawn(async move {
                        let today = ubiqx_engine::today(engine.state());
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
    Ok(Menus {
        open,
        pause,
        resume,
        private_mode: private,
        private_30: p30,
        private_60: p60,
        private_tomorrow: ptom,
        private_indefinite: pind,
        private_off: poff,
        snooze,
        update,
        report_today: report,
        quit,
        hide_window,
    })
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
        .on_menu_event(|app, event| {
            if event.id().as_ref() == HIDE_WINDOW_ID {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
        })
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
            let presenter: Arc<dyn InterventionPresenter> =
                Arc::new(TauriPresenter(handle.clone()));
            let scripted = if std::env::var("UBIQX_SCRIPTED").is_ok() {
                Some(ubiqx_app::ubiqx_platform::mock::Scenario::demo_day().looping())
            } else {
                None
            };
            let ai = if std::env::var("UBIQX_FAKE_AI").is_ok() {
                ubiqx_app::AiBackend::Fake
            } else if scripted.is_some() && std::env::var("UBIQX_ALLOW_REAL_AI").as_deref() != Ok("1")
            {
                // The scripted platform reads the API key from the environment (a store meant
                // for the CLI only): never spend real API calls on the demo scenario unless a
                // developer opts in explicitly.
                tracing::info!(
                    "UBIQX_SCRIPTED: using the fake AI backend (set UBIQX_ALLOW_REAL_AI=1 to use the real vendors)"
                );
                ubiqx_app::AiBackend::Fake
            } else {
                ubiqx_app::AiBackend::Remote
            };
            let build = build_info();
            tracing::info!(
                version = %build.version,
                epoch = build.epoch,
                number = build.number,
                sha = %build.sha,
                branch = %build.branch,
                "ubiqX build"
            );
            let config = AppConfig {
                data_dir: Some(data_dir),
                in_memory_db: false,
                scripted_platform: scripted,
                ai,
                notifier: Some(notifier),
                presenter: Some(presenter),
                build,
                update_feed_url: Some(env!("UBIQX_UPDATE_FEED_URL").to_string()),
            };
            let ubiqx = tauri::async_runtime::block_on(async { App::start(config, sink) })
                .map_err(|e| format!("engine start: {e}"))?;
            if ubiqx.scripted.is_some() {
                let _ = ubiqx_app::demo::seed(ubiqx.engine.state().deps.repos.categories.as_ref());
                // The demo "works by itself": tracking and AI calls are gated on onboarding
                // consent, which the scripted scenario grants up front.
                let mut settings = ubiqx.engine.settings();
                if !settings.onboarding_done {
                    settings.onboarding_done = true;
                    if let Err(e) = ubiqx.engine.update_settings(settings) {
                        tracing::warn!(error = %e, "could not mark the demo onboarding as done");
                    }
                }
            }

            // Keep the login-item setting in sync with the persisted preference.
            let launch = ubiqx.engine.settings().launch_at_login;
            let autostart = app.autolaunch();
            let _ = if launch {
                autostart.enable()
            } else {
                autostart.disable()
            };

            let settings = ubiqx.engine.settings();
            let onboarding_done = settings.onboarding_done;
            let lang = settings.ui_language();
            app.manage(AppState { app: ubiqx });
            // Menus are built once the engine is up, so their labels follow the stored
            // language; `update_settings` relabels them in place afterwards.
            let (app_menu, hide_window) = build_app_menu(&handle, lang)?;
            app.set_menu(app_menu)?;
            let menus = build_tray(app, lang, hide_window)?;
            app.manage(menus);

            if !minimized || !onboarding_done {
                show_main_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Closing the dashboard must not stop tracking: hide instead. The
                // intervention panel is kept too, so the next intervention only re-shows it.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(commands::handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            RunEvent::ExitRequested { api, code, .. } => {
                // Only the tray "Sair" item (which calls app.exit) may end the process.
                if code.is_none() {
                    api.prevent_exit();
                } else if let Some(state) = app.try_state::<AppState>() {
                    state.app.engine.shutdown();
                }
            }
            RunEvent::Exit => {
                // Logout / system shutdown send `terminate:` without an ExitRequested we can
                // veto: at least stop the engine before the process goes away.
                if let Some(state) = app.try_state::<AppState>() {
                    state.app.engine.shutdown();
                }
            }
            _ => {}
        });
}

/// Menu id of the item that owns the Cmd+Q accelerator.
const HIDE_WINDOW_ID: &str = "hide_window";

/// Application menu: Tauri's default minus the predefined Quit item. That item sends
/// `terminate:` straight to NSApplication, which ends the process (and tracking) without any
/// `ExitRequested` we could veto; Cmd+Q instead hides the dashboard, exactly like closing
/// it, and the tray "Sair" item stays the only exit path. Edit and Window are kept so
/// Cmd+C/V/X/A and Cmd+W still work in the webview of an Accessory app (Cmd+W goes through
/// `performClose:` → `CloseRequested`, which is turned into hide above). Returns the menu and
/// the hide item, whose label follows the UI language.
fn build_app_menu(
    app: &AppHandle,
    lang: UiLanguage,
) -> tauri::Result<(Menu<tauri::Wry>, MenuItem<tauri::Wry>)> {
    let name = app.package_info().name.clone();
    let hide_window = MenuItem::with_id(
        app,
        HIDE_WINDOW_ID,
        MenuText::for_language(lang).hide_window,
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    let app_menu = Submenu::with_items(
        app,
        name,
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &hide_window,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let menu = Menu::with_items(app, &[&app_menu, &edit, &window])?;
    Ok((menu, hide_window))
}
