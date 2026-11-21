//! The `nofap` module in the ubiqX shell: the one command the page and the card window
//! call, the bridge of the module's events to the webview, and the card window. No
//! decision lives here: the command forwards to `ubiqx_engine::nofap::ipc::handle`, and the
//! engine decides what to show and when.

use serde_json::Value;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, State, WebviewUrl,
    WebviewWindowBuilder,
};
use ubiqx_core::nofap::{CardKind, NofapCard, NofapEvent, NofapPresenter, NofapSink};
use ubiqx_core::nofap_ipc::NofapCommand;
use ubiqx_core::{CoreError, CoreResult};

use crate::commands::IpcError;
use crate::AppState;

/// Label of the card window (`capabilities/shield-card.json` names it).
pub const CARD_WINDOW: &str = "shield-card";

/// Title of the card window: UBI speaks there. Never a site, a page or an app.
const CARD_TITLE: &str = "UBI";

/// Width of the card window; the page asks for the height its content needs.
const CARD_WIDTH: f64 = 420.0;

/// Gap between the top of the monitor and the card, in logical pixels.
const CARD_TOP_GAP: f64 = 24.0;

/// Name of the event every module event is emitted under.
pub const EVENT: &str = "nofap";

/// The module's one command: `{ cmd, args }` in, the reply of that command out. It runs on
/// the blocking pool: it reads the database, and `enable`, `system_apply`,
/// `system_remove`, `restore_network` and some card actions wait for the operating
/// system's password prompt.
#[tauri::command]
pub async fn nofap(state: State<'_, AppState>, command: NofapCommand) -> Result<Value, IpcError> {
    let service = state.app.engine.nofap().clone();
    tauri::async_runtime::spawn_blocking(move || {
        ubiqx_engine::nofap::ipc::handle(&service, command)
    })
    .await
    .map_err(|e| IpcError::from(e.to_string()))?
    .map_err(IpcError::from)
}

/// The height a card opens with, before the page asks for its own (`cardSize` in
/// `src/nofap/labels.ts`): only so the window does not jump when it appears.
fn card_height(card: &NofapCard) -> f64 {
    match card.kind {
        CardKind::Blocked if card.discreet => 132.0,
        CardKind::Blocked | CardKind::BlockedRepeat => 560.0,
        CardKind::Reminder => 160.0,
        CardKind::VisionAsk | CardKind::Drift => 200.0,
        _ => 220.0,
    }
}

/// Hash route of the card window for a card id (only the id's safe characters).
fn card_route(id: &str) -> String {
    let id: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    format!("#/nofap-card?id={id}")
}

/// Centred horizontally, [`CARD_TOP_GAP`] below the top of the monitor under the cursor
/// (the primary monitor when that cannot be told).
fn card_position(app: &AppHandle) -> Option<PhysicalPosition<i32>> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor();
    let width = (CARD_WIDTH * scale).round() as i32;
    let x = monitor.position().x + (monitor.size().width as i32 - width) / 2;
    let y = monitor.position().y + (CARD_TOP_GAP * scale).round() as i32;
    Some(PhysicalPosition::new(x, y))
}

fn platform(e: impl std::fmt::Display) -> CoreError {
    CoreError::Platform(format!("card window: {e}"))
}

/// Shows `card` in the card window: built on first use (always on top, on every
/// workspace, without taking focus), then pointed at the new id and shown again. The page
/// closes the window itself.
pub fn show_card_window(app: &AppHandle, card: &NofapCard) -> CoreResult<()> {
    let route = card_route(&card.id);
    let size = LogicalSize::new(CARD_WIDTH, card_height(card));
    let position = card_position(app);
    let window = match app.get_webview_window(CARD_WINDOW) {
        Some(w) => {
            // A hash change re-renders the route without reloading the page.
            let hash = serde_json::to_string(&route).map_err(platform)?;
            w.eval(format!("window.location.hash = {hash}"))
                .map_err(platform)?;
            w
        }
        None => {
            let mut builder = WebviewWindowBuilder::new(
                app,
                CARD_WINDOW,
                WebviewUrl::App(format!("index.html{route}").into()),
            )
            .title(CARD_TITLE)
            .inner_size(size.width, size.height)
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
            builder.build().map_err(platform)?
        }
    };
    let _ = window.set_size(size);
    if let Some(p) = position {
        let _ = window.set_position(p);
    }
    let _ = window.set_always_on_top(true);
    window.show().map_err(platform)
}

/// [`NofapPresenter`] backed by the card window. Called off the runtime lock, from a
/// blocking thread.
pub struct TauriNofapPresenter(pub AppHandle);

impl NofapPresenter for TauriNofapPresenter {
    fn show(&self, card: &NofapCard) -> CoreResult<bool> {
        show_card_window(&self.0, card)?;
        Ok(true)
    }
}

/// [`NofapSink`]: every event goes to the webview as `nofap`. A status change also
/// refreshes the module's tray items, on a task of its own: the sink must stay cheap and
/// never call back into the service from the thread that emits.
pub struct TauriNofapSink(pub AppHandle);

impl NofapSink for TauriNofapSink {
    fn emit(&self, event: NofapEvent) {
        if matches!(event, NofapEvent::Status { .. }) {
            let app = self.0.clone();
            tauri::async_runtime::spawn(async move {
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    crate::nofap_tray::sync(&app);
                })
                .await;
            });
        }
        if let Err(e) = self.0.emit(EVENT, &event) {
            tracing::debug!(error = %e, "could not emit a nofap event");
        }
    }
}
