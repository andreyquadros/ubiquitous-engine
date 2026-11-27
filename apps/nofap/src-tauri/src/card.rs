//! The card window of the standalone product: UBI's floating card (design 5.3). The same
//! window as ubiqX's (`apps/desktop/src-tauri/src/nofap.rs`): label `shield-card`, no
//! decorations, always on top, on every workspace, no taskbar entry, shown without focus.
//! The engine decides what to show and when; this only places the window.

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};
use ubiqx_core::nofap::{CardKind, NofapCard, NofapPresenter};
use ubiqx_core::{CoreError, CoreResult};

/// Label of the card window (`capabilities/shield-card.json` names it).
pub const CARD_WINDOW: &str = "shield-card";

/// Title of the card window: UBI speaks there. Never a site, a page or an app.
const CARD_TITLE: &str = "UBI";

/// Width of the card window; the page asks for the height its content needs.
const CARD_WIDTH: f64 = 420.0;

/// Gap between the top of the monitor and the card, in logical pixels.
const CARD_TOP_GAP: f64 = 24.0;

/// The height a card opens with, before the page asks for its own (`cardSize` in
/// `src/nofap/labels.ts`): only so the window does not jump when it appears.
fn card_height(card: &NofapCard) -> f64 {
    match card.kind {
        CardKind::Blocked if card.discreet => 132.0,
        CardKind::Blocked | CardKind::BlockedRepeat => 560.0,
        CardKind::Reminder => 160.0,
        CardKind::VisionAsk | CardKind::Drift => 200.0,
        CardKind::VisionClosed
        | CardKind::PendingReady
        | CardKind::ResolverUnreachable
        | CardKind::Help => 220.0,
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

/// Shows `card` in the card window: built on first use, then pointed at the new id and
/// shown again. The page closes the window itself. Not built with `resizable(false)`: on
/// Linux (GTK) such a window keeps its content's requested size and ignores `set_size`
/// (hand-off U7, review).
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
    // The card never takes the keyboard from the app the person is typing in (integrator
    // decision 38): `show` makes the window key on macOS, and activates it on Windows from
    // the second card on, unless it cannot take focus at that moment. It is focusable again
    // once shown, so its buttons answer a click as before.
    let _ = window.set_focusable(false);
    let shown = window.show().map_err(platform);
    let _ = window.set_focusable(true);
    shown
}

/// [`NofapPresenter`] backed by the card window. Called from a blocking thread.
pub struct TauriNofapPresenter(pub AppHandle);

impl NofapPresenter for TauriNofapPresenter {
    fn show(&self, card: &NofapCard) -> CoreResult<bool> {
        show_card_window(&self.0, card)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_route_keeps_only_the_safe_characters_of_the_id() {
        assert_eq!(card_route("card-1_a"), "#/nofap-card?id=card-1_a");
        assert_eq!(
            card_route("x\"; alert(1); //&lang=en"),
            "#/nofap-card?id=xalert1langen"
        );
    }
}
