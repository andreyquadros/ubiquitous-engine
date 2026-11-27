//! The standalone product's tray (design 8.4): "Abrir", a neutral status line and
//! "Preciso de um respiro" while the module is on, "Verificar atualizações", and "Sair".
//! No pause, private mode or report items: there is no tracker in this product.
//!
//! The shell has no dialog plugin, so the quit confirmation of design 6.3 is a submenu in
//! place of "Sair" while the module is on: the true sentence (`quit.confirm_system` or
//! `quit.confirm_none`) as a disabled item, then "Manter aberto" and "Sair". With the module
//! off, "Sair" quits at once. The menu is rebuilt whole on every change.

use std::sync::Mutex;

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};
use ubiqx_app::ubiqx_nofap::messages::{self, QuitDialog};

use crate::app::{quit, show_main_route, AppState};

/// Id of the tray icon.
const TRAY_ID: &str = "main";
const OPEN_ID: &str = "open";
const BREATHER_ID: &str = "breather";
const UPDATE_ID: &str = "update";
const QUIT_ID: &str = "quit";
const QUIT_CONFIRMED_ID: &str = "quit_confirmed";
/// "Manter aberto": closes the menu, nothing else.
const KEEP_OPEN_ID: &str = "keep_open";

/// Where "Preciso de um respiro" takes the main window (the page's support tab).
pub const BREATHER_ROUTE: &str = "#/support";
/// Where the update item goes when no newer build is known.
const SETTINGS_ROUTE: &str = "#/settings";

/// One refresh at a time: the module is read and the menu applied under this lock, so a
/// refresh that read "on" just before the module was switched off cannot apply after the
/// one that read "off" (the race found in ubiqX's tray, hand-off U7).
pub struct TrayLock(Mutex<()>);

/// The menu for the module's state: `on` carries the quit sentence of the moment.
fn menu(
    app: &AppHandle,
    labels: &messages::TrayLabels,
    on: Option<&QuitDialog>,
) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, text: &str, enabled: bool| {
        MenuItem::with_id(app, id, text, enabled, None::<&str>)
    };
    let open = item(OPEN_ID, &labels.open, true)?;
    let update = item(UPDATE_ID, &labels.check_updates, true)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator_2 = PredefinedMenuItem::separator(app)?;
    let separator_3 = PredefinedMenuItem::separator(app)?;
    let separator_4 = PredefinedMenuItem::separator(app)?;
    match on {
        Some(dialog) => {
            let status = item("status", &labels.status_on, false)?;
            let breather = item(BREATHER_ID, &labels.breather, true)?;
            let message = item("quit_message", &dialog.message, false)?;
            let keep_open = item(KEEP_OPEN_ID, &dialog.keep_open, true)?;
            let quit = item(QUIT_CONFIRMED_ID, &dialog.quit, true)?;
            let quit_menu = Submenu::with_items(
                app,
                &labels.quit,
                true,
                &[&message, &separator_4, &keep_open, &quit],
            )?;
            let items: [&dyn IsMenuItem<Wry>; 8] = [
                &open,
                &separator,
                &status,
                &breather,
                &separator_2,
                &update,
                &separator_3,
                &quit_menu,
            ];
            Menu::with_items(app, &items)
        }
        None => {
            let quit = item(QUIT_ID, &labels.quit, true)?;
            Menu::with_items(app, &[&open, &separator, &update, &separator_2, &quit])
        }
    }
}

/// Builds the tray icon (with an empty menu until [`sync`] runs).
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    app.manage(TrayLock(Mutex::new(())));
    // Monochrome glyph that macOS renders as a template image (light and dark menu bars).
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip(ubiqx_core::product::SHIELD.display_name)
        .menu(&Menu::new(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_ID => show_main_route(app, None),
            BREATHER_ID => show_main_route(app, Some(BREATHER_ROUTE)),
            UPDATE_ID => check_updates(app),
            QUIT_ID | QUIT_CONFIRMED_ID => quit(app),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// Brings the menu in line with the module and the language: once the tray exists, after
/// every status event and after a settings write. Blocking (the quit sentence may look at
/// the system once): never on the main thread or under the service's lock.
pub fn sync(app: &AppHandle) {
    let (Some(lock), Some(state), Some(tray)) = (
        app.try_state::<TrayLock>(),
        app.try_state::<AppState>(),
        app.tray_by_id(TRAY_ID),
    ) else {
        return;
    };
    let _one_at_a_time = lock.0.lock().unwrap_or_else(|e| e.into_inner());
    let service = &state.app.service;
    let labels = messages::tray_labels(service.language());
    let dialog = service.enabled().then(|| service.quit_dialog());
    match menu(app, &labels, dialog.as_ref()) {
        Ok(menu) => {
            if let Err(e) = tray.set_menu(Some(menu)) {
                tracing::warn!(error = %e, "could not set the tray menu");
            }
        }
        Err(e) => tracing::warn!(error = %e, "could not build the tray menu"),
    }
}

/// [`sync`] on a blocking thread of its own.
pub fn sync_later(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || sync(&app));
}

/// "Verificar atualizações": a known build, or one this check finds, opens its download in
/// the browser; otherwise the settings, whose update section says what the check found.
fn check_updates(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let mut available = state.app.update_status().available;
        if available.is_none() {
            match state.app.updates.check_now().await {
                Ok(status) => available = status.available,
                Err(e) => tracing::warn!(error = %e, "update check failed"),
            }
        }
        // Only web links: the address came from a JSON feed.
        match available.filter(|r| r.download_url.starts_with("https://")) {
            Some(release) => {
                if let Err(e) = tauri_plugin_opener::open_url(&release.download_url, None::<&str>) {
                    tracing::warn!(error = %e, "could not open the download");
                }
            }
            None => show_main_route(&app, Some(SETTINGS_ROUTE)),
        }
    });
}

/// The application menu on macOS: Tauri's default without its Quit item, whose `terminate:`
/// would end the process with no confirmation and without counting the quit. Quitting is the
/// tray's "Sair" (with the confirmation of design 6.3) or the window's own quit button.
#[cfg(target_os = "macos")]
pub fn app_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let name = app.package_info().name.clone();
    let app_menu = Submenu::with_items(
        app,
        name,
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
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
    Menu::with_items(app, &[&app_menu, &edit, &window])
}
