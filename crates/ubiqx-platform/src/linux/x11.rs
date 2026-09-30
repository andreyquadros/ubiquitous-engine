//! A thin layer over `x11rb`: one connection per query (cheap on a local socket, and
//! nothing to recover after the server goes away), EWMH properties, keyboard synthesis
//! through XTest and `GetImage` captures. Nothing here panics without a server: every
//! entry point answers `None`/`false` and the missing display is logged once.

use std::sync::Once;

use x11rb::connection::Connection;
use x11rb::properties::WmClass;
use x11rb::protocol::screensaver::ConnectionExt as _;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConnectionExt as _, EventMask, ImageFormat, MapState, Window,
    CLIENT_MESSAGE_EVENT, KEY_PRESS_EVENT, KEY_RELEASE_EVENT,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        _NET_ACTIVE_WINDOW,
        _NET_CLIENT_LIST,
        _NET_WM_NAME,
        _NET_WM_PID,
        UTF8_STRING,
        WM_CHANGE_STATE,
        WM_NAME,
    }
}

/// `IconicState` of `WM_CHANGE_STATE` (ICCCM 4.1.4).
const ICONIC_STATE: u32 = 3;

/// Keysyms used by the enforcer.
pub const KEYSYM_CONTROL_L: u32 = 0xffe3;
pub const KEYSYM_SHIFT_L: u32 = 0xffe1;
pub const KEYSYM_RETURN: u32 = 0xff0d;

/// Whether an X display is configured for this process.
pub fn display_available() -> bool {
    std::env::var_os("DISPLAY").is_some_and(|d| !d.is_empty())
}

fn warn_once(what: &str) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        tracing::warn!(
            "{what}: no X11 display; the foreground window, idle time, screenshots and \
             focus enforcement are unavailable (a Wayland session needs XWayland)"
        );
    });
}

/// An open connection plus the atoms every query needs.
pub struct X11 {
    pub conn: RustConnection,
    pub screen: usize,
    pub atoms: Atoms,
}

impl X11 {
    /// Connects to `$DISPLAY`, or `None` (logged once) when there is no display.
    pub fn connect() -> Option<Self> {
        if !display_available() {
            warn_once("DISPLAY unset");
            return None;
        }
        let (conn, screen) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => {
                warn_once(&format!("connect failed ({e})"));
                return None;
            }
        };
        let atoms = Atoms::new(&conn).ok()?.reply().ok()?;
        Some(Self {
            conn,
            screen,
            atoms,
        })
    }

    pub fn root(&self) -> Window {
        self.conn.setup().roots[self.screen].root
    }

    /// Size of the screen in pixels.
    pub fn screen_size(&self) -> (u16, u16) {
        let s = &self.conn.setup().roots[self.screen];
        (s.width_in_pixels, s.height_in_pixels)
    }

    fn property(&self, window: Window, property: u32, type_: u32, len: u32) -> Option<Vec<u8>> {
        let reply = self
            .conn
            .get_property(false, window, property, type_, 0, len)
            .ok()?
            .reply()
            .ok()?;
        (reply.value_len > 0).then_some(reply.value)
    }

    fn property_u32s(&self, window: Window, property: u32, type_: AtomEnum) -> Vec<u32> {
        let reply = self
            .conn
            .get_property(false, window, property, type_, 0, 4096)
            .ok()
            .and_then(|c| c.reply().ok());
        match reply {
            Some(r) => r.value32().map(|it| it.collect()).unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// `_NET_ACTIVE_WINDOW` of the root, when the window manager sets it.
    pub fn active_window(&self) -> Option<Window> {
        self.property_u32s(self.root(), self.atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW)
            .into_iter()
            .next()
            .filter(|&w| w != 0)
    }

    /// `_NET_CLIENT_LIST`: the windows the manager manages.
    pub fn client_list(&self) -> Vec<Window> {
        self.property_u32s(self.root(), self.atoms._NET_CLIENT_LIST, AtomEnum::WINDOW)
    }

    pub fn window_pid(&self, window: Window) -> Option<u32> {
        self.property_u32s(window, self.atoms._NET_WM_PID, AtomEnum::CARDINAL)
            .into_iter()
            .next()
            .filter(|&p| p != 0)
    }

    /// `(instance, class)` of `WM_CLASS`.
    pub fn window_class(&self, window: Window) -> Option<(String, String)> {
        let class = WmClass::get(&self.conn, window).ok()?.reply().ok()??;
        Some((
            String::from_utf8_lossy(class.instance()).to_string(),
            String::from_utf8_lossy(class.class()).to_string(),
        ))
    }

    /// `_NET_WM_NAME` (UTF-8), else `WM_NAME`.
    pub fn window_name(&self, window: Window) -> String {
        if let Some(v) = self.property(
            window,
            self.atoms._NET_WM_NAME,
            self.atoms.UTF8_STRING,
            1024,
        ) {
            return String::from_utf8_lossy(&v).to_string();
        }
        self.property(window, self.atoms.WM_NAME, AtomEnum::ANY.into(), 1024)
            .map(|v| String::from_utf8_lossy(&v).to_string())
            .unwrap_or_default()
    }

    /// Screen-space frame `(x, y, width, height)`.
    pub fn window_rect(&self, window: Window) -> Option<(i32, i32, u32, u32)> {
        let geo = self.conn.get_geometry(window).ok()?.reply().ok()?;
        let pos = self
            .conn
            .translate_coordinates(window, self.root(), 0, 0)
            .ok()?
            .reply()
            .ok()?;
        Some((
            i32::from(pos.dst_x),
            i32::from(pos.dst_y),
            u32::from(geo.width),
            u32::from(geo.height),
        ))
    }

    pub fn is_viewable(&self, window: Window) -> bool {
        self.conn
            .get_window_attributes(window)
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some_and(|a| a.map_state == MapState::VIEWABLE)
    }

    /// Asks the window manager to iconify `window` (`WM_CHANGE_STATE`).
    pub fn iconify(&self, window: Window) -> bool {
        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_: self.atoms.WM_CHANGE_STATE,
            data: [ICONIC_STATE, 0, 0, 0, 0].into(),
        };
        let sent = self
            .conn
            .send_event(
                false,
                self.root(),
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            )
            .is_ok();
        sent && self.conn.flush().is_ok()
    }

    /// Milliseconds since the last input event (XScreenSaver extension).
    pub fn idle_ms(&self) -> Option<u32> {
        self.conn
            .screensaver_query_info(self.root())
            .ok()?
            .reply()
            .ok()
            .map(|r| r.ms_since_user_input)
    }

    /// The keycode producing `keysym`, and whether Shift is needed for it (column 1 of the
    /// keyboard mapping). `None` when no key produces it in the current layout.
    pub fn keycode_for(&self, keysym: u32) -> Option<(u8, bool)> {
        let setup = self.conn.setup();
        let (min, max) = (setup.min_keycode, setup.max_keycode);
        let count = max.checked_sub(min)? as u16 + 1;
        let map = self
            .conn
            .get_keyboard_mapping(min, count as u8)
            .ok()?
            .reply()
            .ok()?;
        let per = usize::from(map.keysyms_per_keycode).max(1);
        // Prefer an unshifted key, then a shifted one.
        for want_shift in [false, true] {
            for (i, chunk) in map.keysyms.chunks(per).enumerate() {
                let column = usize::from(want_shift);
                if chunk.get(column).copied() == Some(keysym) {
                    return Some((min + i as u8, want_shift));
                }
            }
        }
        None
    }

    /// Presses or releases a keycode through XTest.
    pub fn fake_key(&self, keycode: u8, press: bool) -> bool {
        let kind = if press {
            KEY_PRESS_EVENT
        } else {
            KEY_RELEASE_EVENT
        };
        self.conn
            .xtest_fake_input(kind, keycode, x11rb::CURRENT_TIME, self.root(), 0, 0, 0)
            .is_ok()
    }

    /// Taps a chord: modifiers down, the key, modifiers up. `keys` are keysyms.
    pub fn tap(&self, modifiers: &[u32], keysym: u32) -> bool {
        let Some((code, shift)) = self.keycode_for(keysym) else {
            return false;
        };
        let mut mods: Vec<u8> = modifiers
            .iter()
            .filter_map(|&m| self.keycode_for(m).map(|(c, _)| c))
            .collect();
        if mods.len() != modifiers.len() {
            return false;
        }
        if shift {
            if let Some((s, _)) = self.keycode_for(KEYSYM_SHIFT_L) {
                mods.push(s);
            }
        }
        let mut ok = true;
        for &m in &mods {
            ok &= self.fake_key(m, true);
        }
        ok &= self.fake_key(code, true);
        ok &= self.fake_key(code, false);
        for &m in mods.iter().rev() {
            ok &= self.fake_key(m, false);
        }
        ok && self.conn.flush().is_ok()
    }

    /// Types ASCII text key by key (the layout must produce each character).
    pub fn type_text(&self, text: &str) -> bool {
        text.chars()
            .all(|c| c.is_ascii() && self.tap(&[], u32::from(c)))
    }

    /// `GetImage` of `drawable` at `rect` as RGBA. Depth 24/32 ZPixmap only (what every
    /// modern server offers); the window must be viewable and on screen.
    pub fn image(
        &self,
        drawable: Window,
        (x, y, w, h): (i16, i16, u16, u16),
    ) -> Option<image::RgbaImage> {
        if w == 0 || h == 0 {
            return None;
        }
        let reply = self
            .conn
            .get_image(ImageFormat::Z_PIXMAP, drawable, x, y, w, h, !0)
            .ok()?
            .reply()
            .ok()?;
        zpixmap_to_rgba(&reply.data, reply.depth, u32::from(w), u32::from(h))
    }
}

/// Converts a 32-bits-per-pixel ZPixmap (BGRX in server byte order, little-endian on every
/// platform we ship on) to RGBA.
pub fn zpixmap_to_rgba(data: &[u8], depth: u8, w: u32, h: u32) -> Option<image::RgbaImage> {
    if !(depth == 24 || depth == 32) {
        return None;
    }
    let needed = (w as usize) * (h as usize) * 4;
    if data.len() < needed {
        return None;
    }
    let mut out = Vec::with_capacity(needed);
    for px in data[..needed].chunks_exact(4) {
        out.extend_from_slice(&[px[2], px[1], px[0], 255]);
    }
    image::RgbaImage::from_raw(w, h, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bgrx_to_rgba() {
        let data = [1u8, 2, 3, 0, 4, 5, 6, 0];
        let img = zpixmap_to_rgba(&data, 24, 2, 1).unwrap();
        assert_eq!(img.get_pixel(0, 0).0, [3, 2, 1, 255]);
        assert_eq!(img.get_pixel(1, 0).0, [6, 5, 4, 255]);
        assert!(zpixmap_to_rgba(&data, 16, 2, 1).is_none());
        assert!(zpixmap_to_rgba(&data, 24, 3, 1).is_none(), "short buffer");
    }

    #[test]
    fn with_a_display_every_query_answers() {
        // Under a real server (Xvfb in CI or a developer's session) the queries must run
        // without panicking; what they answer depends on the window manager.
        let Some(x) = X11::connect() else {
            return;
        };
        let (w, h) = x.screen_size();
        assert!(w > 0 && h > 0);
        let _ = x.active_window();
        let _ = x.client_list();
        let _ = x.idle_ms();
        let _ = x.keycode_for(u32::from('w'));
        let _ = x.keycode_for(KEYSYM_CONTROL_L);
        let img = x.image(x.root(), (0, 0, w.min(64), h.min(64)));
        assert!(img.is_some_and(|i| i.width() == u32::from(w.min(64))));
        assert!(!x.is_viewable(0));
    }

    #[test]
    fn no_display_means_no_connection() {
        // A CI runner or a build box has no DISPLAY: nothing here may panic.
        if !display_available() {
            assert!(X11::connect().is_none());
        }
    }
}
