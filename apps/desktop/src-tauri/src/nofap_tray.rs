//! The `nofap` module's items in ubiqX's tray, there only while the module is on: a
//! neutral status line, "Preciso de um respiro" (the page's support tab), a note under the
//! pause and private-mode items that they do not pause the module, and the quit
//! confirmation. A tray menu item has no tooltip, so the note is an item of its own; the
//! shell has no dialog plugin, so the quit confirmation is a submenu in place of "Sair":
//! the true sentence of `quit.confirm_system` / `quit.confirm_none`, "Manter aberto" and
//! "Sair". While the module is off the tray is ubiqX's own, unchanged.

use std::sync::Mutex;

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};
use ubiqx_app::ubiqx_nofap;
use ubiqx_core::UiLanguage;
use ubiqx_nofap::messages::{QuitDialog, TrayLabels};

use crate::AppState;

/// Menu id of the breather item.
pub const BREATHER_ID: &str = "nofap_breather";
/// Menu id of the confirmed quit.
pub const QUIT_CONFIRMED_ID: &str = "nofap_quit";
/// Menu id of "Manter aberto" (closes the menu, nothing else).
pub const KEEP_OPEN_ID: &str = "nofap_keep_open";

/// Where "Preciso de um respiro" takes the main window.
pub const BREATHER_ROUTE: &str = "#/nofap?tab=support";

/// The module's tray items, kept so they can be added, relabelled and taken out.
pub struct NofapTray {
    menu: Menu<Wry>,
    /// ubiqX's own "Sair", replaced by [`quit_menu`](Self::quit_menu) while the module is on.
    quit: MenuItem<Wry>,
    status: MenuItem<Wry>,
    breather: MenuItem<Wry>,
    not_paused: MenuItem<Wry>,
    separator: PredefinedMenuItem<Wry>,
    quit_menu: Submenu<Wry>,
    quit_message: MenuItem<Wry>,
    keep_open: MenuItem<Wry>,
    quit_confirmed: MenuItem<Wry>,
    shown: Mutex<bool>,
}

impl NofapTray {
    /// The items, not yet in `menu`. `quit` is ubiqX's own quit item of that menu.
    pub fn new(app: &AppHandle, menu: Menu<Wry>, quit: MenuItem<Wry>) -> tauri::Result<Self> {
        let item = |id: &str, enabled: bool| MenuItem::with_id(app, id, "", enabled, None::<&str>);
        let quit_message = item("nofap_quit_message", false)?;
        let keep_open = item(KEEP_OPEN_ID, true)?;
        let quit_confirmed = item(QUIT_CONFIRMED_ID, true)?;
        let quit_menu = Submenu::with_items(
            app,
            "",
            true,
            &[
                &quit_message,
                &PredefinedMenuItem::separator(app)?,
                &keep_open,
                &quit_confirmed,
            ],
        )?;
        Ok(Self {
            menu,
            quit,
            status: item("nofap_status", false)?,
            breather: item(BREATHER_ID, true)?,
            not_paused: item("nofap_not_paused", false)?,
            separator: PredefinedMenuItem::separator(app)?,
            quit_menu,
            quit_message,
            keep_open,
            quit_confirmed,
            shown: Mutex::new(false),
        })
    }

    /// The position of the item with `id` in the tray menu.
    fn position(&self, id: &str) -> Option<usize> {
        self.menu
            .items()
            .ok()?
            .iter()
            .position(|item| item.id().as_ref() == id)
    }

    /// Puts the items in (module on, with the quit dialog of the moment) or takes them out.
    fn apply(
        &self,
        on: Option<QuitDialog>,
        lang: UiLanguage,
        quit_label: &str,
    ) -> tauri::Result<()> {
        let labels: TrayLabels = ubiqx_nofap::messages::tray_labels(lang);
        let mut shown = self.shown.lock().unwrap_or_else(|e| e.into_inner());
        match on {
            Some(dialog) => {
                self.status.set_text(&labels.status_on)?;
                self.breather.set_text(&labels.breather)?;
                self.not_paused.set_text(&labels.not_paused)?;
                self.quit_menu.set_text(quit_label)?;
                self.quit_message.set_text(&dialog.message)?;
                self.keep_open.set_text(&dialog.keep_open)?;
                self.quit_confirmed.set_text(&dialog.quit)?;
                if !*shown {
                    // From the bottom up, so each position is still the one it was read as.
                    let quit_at = self.position("quit").unwrap_or(usize::MAX);
                    self.menu.remove(&self.quit)?;
                    self.insert(&self.quit_menu, quit_at)?;
                    let snooze_at = self.position("snooze").unwrap_or(usize::MAX);
                    self.insert(&self.not_paused, snooze_at)?;
                    let after_open = self.position("open").map_or(0, |at| at + 1);
                    self.menu.insert_items(
                        &[&self.separator, &self.status, &self.breather],
                        after_open,
                    )?;
                    *shown = true;
                }
            }
            None if *shown => {
                for item in [
                    &self.separator as &dyn IsMenuItem<Wry>,
                    &self.status,
                    &self.breather,
                    &self.not_paused,
                    &self.quit_menu,
                ] {
                    self.menu.remove(item)?;
                }
                self.menu.append(&self.quit)?;
                *shown = false;
            }
            None => {}
        }
        Ok(())
    }

    fn insert(&self, item: &dyn IsMenuItem<Wry>, at: usize) -> tauri::Result<()> {
        if at == usize::MAX {
            self.menu.append(item)
        } else {
            self.menu.insert(item, at)
        }
    }
}

/// Brings the module's tray items in line with the module: called once the tray exists,
/// after every status event and after a change of language. Blocking (the quit sentence
/// may look at the system once): never on the main thread or under the service's lock.
pub fn sync(app: &AppHandle) {
    let (Some(tray), Some(state)) = (app.try_state::<NofapTray>(), app.try_state::<AppState>())
    else {
        return;
    };
    let nofap = state.app.engine.nofap();
    let lang = nofap.language();
    let dialog = nofap.enabled().then(|| nofap.quit_dialog());
    let quit_label = crate::quit_label(lang);
    if let Err(e) = tray.apply(dialog, lang, quit_label) {
        tracing::warn!(error = %e, "could not update the tray items of the nofap module");
    }
}
