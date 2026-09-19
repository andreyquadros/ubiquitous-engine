//! Browser URL through UI Automation: the address bar of the foreground browser window.
//!
//! Chromium browsers (Chrome, Edge, Brave, Opera, Vivaldi) expose the omnibox as the first
//! `Edit` control with a `ValuePattern` inside the toolbar area at the top of the window;
//! its `AutomationId` and `Name` vary per locale and version, so the match is structural.
//! Firefox names its field `urlbar-input`. What the user is typing is not a URL yet, so a
//! value that does not parse as one yields `None`; a UI Automation failure of any kind also
//! yields `Ok(None)` (the tracker keeps the window title), never an error.
//!
//! Cost: once a UIA client connects, Chromium exposes the whole page tree, so the search
//! is done with a cache request that brings the bounding rectangle and the value of every
//! candidate back in the same cross-process call (one round trip instead of one per
//! `Edit`), and the `IUIAutomation` object is created once per thread.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use ubiqx_core::ports::BrowserUrlResolver;
use ubiqx_core::{CoreResult, ForegroundWindow};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationCacheRequest, IUIAutomationElement,
    IUIAutomationValuePattern, TreeScope_Descendants, UIA_AutomationIdPropertyId,
    UIA_BoundingRectanglePropertyId, UIA_ControlTypePropertyId, UIA_EditControlTypeId,
    UIA_IsValuePatternAvailablePropertyId, UIA_ValuePatternId, UIA_ValueValuePropertyId,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

use super::frontmost::{hwnd_from_id, window_rect};
use super::util::ensure_com;
use crate::browser_url::{is_firefox, normalize_typed_url, WINDOWS_BROWSERS};

/// The address bar sits within this many pixels of the window's top edge.
pub const TOOLBAR_HEIGHT_PX: i32 = 120;

/// How long a resolved (app, title) pair is reused without re-querying the tree.
const CACHE_TTL: Duration = Duration::from_secs(20);

thread_local! {
    /// One `IUIAutomation` per thread: `CoCreateInstance` of the CUIAutomation class is
    /// not free, and the tracker asks from the same sampling thread every few seconds.
    static AUTOMATION: RefCell<Option<IUIAutomation>> = const { RefCell::new(None) };
}

fn automation() -> Option<IUIAutomation> {
    ensure_com();
    AUTOMATION.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            // SAFETY: COM is initialised on this thread; a failure leaves the slot empty.
            let created: Option<IUIAutomation> =
                unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.ok();
            *slot = created;
        }
        slot.clone()
    })
}

/// A cache request that brings back, with every found element, its bounding rectangle
/// and the current value of its `ValuePattern`.
fn value_cache(automation: &IUIAutomation) -> Option<IUIAutomationCacheRequest> {
    // SAFETY: COM calls on a live interface; every result is checked.
    unsafe {
        let cache = automation.CreateCacheRequest().ok()?;
        cache.AddProperty(UIA_BoundingRectanglePropertyId).ok()?;
        cache.AddPattern(UIA_ValuePatternId).ok()?;
        cache.AddProperty(UIA_ValueValuePropertyId).ok()?;
        Some(cache)
    }
}

/// The value of an element found with [`value_cache`]: cached, else asked live.
fn read_value(element: &IUIAutomationElement) -> Option<String> {
    // SAFETY: COM calls on live interface pointers; failures come back as `Err`.
    unsafe {
        let cached: Option<IUIAutomationValuePattern> =
            element.GetCachedPatternAs(UIA_ValuePatternId).ok();
        if let Some(value) = cached.and_then(|p| p.CachedValue().ok()) {
            return Some(value.to_string());
        }
        let pattern: IUIAutomationValuePattern =
            element.GetCurrentPatternAs(UIA_ValuePatternId).ok()?;
        Some(pattern.CurrentValue().ok()?.to_string())
    }
}

/// The text of the address bar of the browser window `hwnd`, raw (not yet a URL).
pub fn address_bar_text(hwnd: HWND, firefox: bool) -> Option<String> {
    let automation = automation()?;
    let top = window_rect(hwnd).map(|(_, y, _, _)| y).unwrap_or(0);
    // SAFETY: COM calls on interfaces created here; every result is checked.
    unsafe {
        let root = automation.ElementFromHandle(hwnd).ok()?;
        let cache = value_cache(&automation)?;
        if firefox {
            let condition = automation
                .CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from("urlbar-input"))
                .ok()?;
            let element = root
                .FindFirstBuildCache(TreeScope_Descendants, &condition, &cache)
                .ok()?;
            return read_value(&element);
        }
        let is_edit = automation
            .CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_EditControlTypeId.0),
            )
            .ok()?;
        let has_value = automation
            .CreatePropertyCondition(UIA_IsValuePatternAvailablePropertyId, &VARIANT::from(true))
            .ok()?;
        let condition = automation.CreateAndCondition(&is_edit, &has_value).ok()?;
        let found = root
            .FindAllBuildCache(TreeScope_Descendants, &condition, &cache)
            .ok()?;
        let count = found.Length().ok()?;
        for i in 0..count {
            let element = found.GetElement(i).ok()?;
            let Ok(rect) = element
                .CachedBoundingRectangle()
                .or_else(|_| element.CurrentBoundingRectangle())
            else {
                continue;
            };
            if rect.right <= rect.left || rect.bottom <= rect.top {
                continue;
            }
            if rect.top >= top && rect.top - top <= TOOLBAR_HEIGHT_PX {
                return read_value(&element);
            }
        }
        None
    }
}

/// `(app_id, window_title)`.
type CacheKey = (String, String);
/// When the URL was resolved and what it was.
type CacheEntry = (Instant, Option<String>);

/// `BrowserUrlResolver` over UI Automation, cached per (app, title) for [`CACHE_TTL`].
#[derive(Debug, Default)]
pub struct UiaUrlResolver {
    cache: Mutex<HashMap<CacheKey, CacheEntry>>,
}

impl UiaUrlResolver {
    fn hwnd_for(window: &ForegroundWindow) -> HWND {
        match window.window_id {
            Some(id) if id != 0 => hwnd_from_id(id),
            // SAFETY: no arguments.
            _ => unsafe { GetForegroundWindow() },
        }
    }
}

impl BrowserUrlResolver for UiaUrlResolver {
    fn supports(&self, app_id: &str) -> bool {
        let id = app_id.trim().to_lowercase();
        WINDOWS_BROWSERS.contains(&id.as_str())
    }

    fn resolve(&self, window: &ForegroundWindow) -> CoreResult<Option<String>> {
        if !self.supports(&window.app_id) {
            return Ok(None);
        }
        let key = (window.app_id.to_lowercase(), window.window_title.clone());
        if let Some((at, url)) = self.cache.lock().get(&key) {
            if at.elapsed() < CACHE_TTL {
                return Ok(url.clone());
            }
        }
        let hwnd = Self::hwnd_for(window);
        if hwnd.0.is_null() {
            return Ok(None);
        }
        let url = address_bar_text(hwnd, is_firefox(&window.app_id))
            .and_then(|text| normalize_typed_url(&text));
        let mut cache = self.cache.lock();
        if cache.len() > 256 {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), url.clone()));
        Ok(url)
    }
}
