//! Browser policy values (L4): each browser's own enterprise policy, so that it resolves
//! through the system (where L2 and L3 apply) and forces SafeSearch where a policy exists.
//!
//! | Browser family | Values |
//! |---|---|
//! | Chrome, Chromium | `DnsOverHttpsMode = "off"`, `ForceGoogleSafeSearch = true` |
//! | Edge | the same, plus `ForceBingSafeSearch = 2` |
//! | Brave | the same as Chrome, plus `TorDisabled = true` |
//! | Firefox | `DNSOverHTTPS = { Enabled: false, Locked: true }` |
//!
//! Private browsing is not disabled and YouTube restricted mode is not forced. On macOS
//! nothing is written for the Chromium family (`Issue::PolicyAdvisoryOnly`).
//!
//! Seeded by the foundation unit: signatures only.
#![allow(unused_variables)]

use ubiqx_core::nofap::{BrowserFamily, Os, PolicyCreated, PolicyObservation, PolicyValue};

/// Name of our own policy file in a Chromium-family `managed` directory on Linux.
pub const OWN_FILE_NAME: &str = "ubiqx-shield.json";

/// The values this tier wants for `browser` (for Firefox: the flattened pair
/// `DNSOverHTTPS.Enabled`, `DNSOverHTTPS.Locked`).
pub fn desired(browser: BrowserFamily) -> Vec<PolicyValue> {
    todo!()
}

/// Whether the tier writes anything for `browser` on `os` (never for the Chromium family
/// on macOS).
pub fn applies(browser: BrowserFamily, os: Os) -> bool {
    todo!()
}

/// The content of our policy file for a Chromium-family browser: a JSON object, keys in
/// the order of [`desired`], with a final newline.
pub fn chromium_json(browser: BrowserFamily) -> String {
    todo!()
}

/// The content of `policies.json` for Firefox, created only when the file is absent.
pub fn firefox_json() -> String {
    todo!()
}

/// Browsers whose policy location holds a value that differs from ours and that
/// `created` does not list as ours: a managed machine, left alone.
pub fn conflicts(observed: &[PolicyObservation], created: &[PolicyCreated]) -> Vec<BrowserFamily> {
    todo!()
}

/// Whether `observation` holds exactly the values of [`desired`] for its browser.
pub fn is_applied(observation: &PolicyObservation) -> bool {
    todo!()
}
