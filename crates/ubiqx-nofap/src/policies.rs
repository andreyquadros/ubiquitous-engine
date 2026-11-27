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

use serde_json::{json, Value};
use ubiqx_core::nofap::{reason, BrowserFamily, Os, PolicyCreated, PolicyObservation, PolicyValue};

/// Name of our own policy file in a Chromium-family `managed` directory on Linux.
pub const OWN_FILE_NAME: &str = "ubiqx-shield.json";

/// The Firefox policy this tier sets, and its two fields. In [`PolicyValue`] keys the pair
/// is flattened with a dot (`DNSOverHTTPS.Enabled`); the back-ends write it as a registry
/// sub-key, a `__` preference name or a JSON object.
pub const FIREFOX_POLICY: &str = "DNSOverHTTPS";
pub const FIREFOX_ENABLED: &str = "DNSOverHTTPS.Enabled";
pub const FIREFOX_LOCKED: &str = "DNSOverHTTPS.Locked";

fn value(key: &str, value: Value) -> PolicyValue {
    PolicyValue {
        key: key.to_owned(),
        value,
    }
}

/// The values this tier wants for `browser` (for Firefox: the flattened pair
/// `DNSOverHTTPS.Enabled`, `DNSOverHTTPS.Locked`).
pub fn desired(browser: BrowserFamily) -> Vec<PolicyValue> {
    let chromium = || {
        vec![
            value("DnsOverHttpsMode", json!("off")),
            value("ForceGoogleSafeSearch", json!(true)),
        ]
    };
    match browser {
        BrowserFamily::Chrome | BrowserFamily::Chromium => chromium(),
        BrowserFamily::Edge => {
            let mut values = chromium();
            values.push(value("ForceBingSafeSearch", json!(2)));
            values
        }
        BrowserFamily::Brave => {
            let mut values = chromium();
            values.push(value("TorDisabled", json!(true)));
            values
        }
        BrowserFamily::Firefox => vec![
            value(FIREFOX_ENABLED, json!(false)),
            value(FIREFOX_LOCKED, json!(true)),
        ],
    }
}

/// Whether the tier writes anything for `browser` on `os` (never for the Chromium family
/// on macOS).
pub fn applies(browser: BrowserFamily, os: Os) -> bool {
    match (browser, os) {
        (BrowserFamily::Firefox, _) => true,
        (
            BrowserFamily::Chrome
            | BrowserFamily::Chromium
            | BrowserFamily::Edge
            | BrowserFamily::Brave,
            os,
        ) => os != Os::Macos,
    }
}

/// The name a preview shows for `browser`.
pub fn display_name(browser: BrowserFamily) -> &'static str {
    match browser {
        BrowserFamily::Chrome => "Chrome",
        BrowserFamily::Chromium => "Chromium",
        BrowserFamily::Edge => "Edge",
        BrowserFamily::Brave => "Brave",
        BrowserFamily::Firefox => "Firefox",
    }
}

/// The content of our policy file for a Chromium-family browser: a JSON object, keys in
/// the order of [`desired`], with a final newline. For Firefox, which has no such file,
/// this is [`firefox_json`], so a caller that mixes the two up still writes a file
/// Firefox understands and no other browser acts on.
pub fn chromium_json(browser: BrowserFamily) -> String {
    if browser == BrowserFamily::Firefox {
        return firefox_json();
    }
    let lines: Vec<String> = desired(browser)
        .iter()
        .map(|entry| format!("  {}: {}", Value::from(entry.key.as_str()), entry.value))
        .collect();
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

/// The content of `policies.json` for Firefox, created only when the file is absent.
pub fn firefox_json() -> String {
    let fields: Vec<String> = desired(BrowserFamily::Firefox)
        .iter()
        .map(|entry| {
            let field = entry.key.rsplit('.').next().unwrap_or(entry.key.as_str());
            format!("      {}: {}", Value::from(field), entry.value)
        })
        .collect();
    format!(
        "{{\n  \"policies\": {{\n    {}: {{\n{}\n    }}\n  }}\n}}\n",
        Value::from(FIREFOX_POLICY),
        fields.join(",\n")
    )
}

/// Whether two policy values mean the same. A registry or a preferences domain has no
/// boolean type, so `true` and `1`, `false` and `0` are equal; numbers compare by value;
/// everything else must be identical.
pub fn same_value(a: &Value, b: &Value) -> bool {
    fn number(value: &Value) -> Option<f64> {
        match value {
            Value::Bool(flag) => Some(f64::from(u8::from(*flag))),
            Value::Number(number) => number.as_f64(),
            _ => None,
        }
    }
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x == y,
        (None, None) => a == b,
        _ => false,
    }
}

fn find<'a>(values: &'a [PolicyValue], key: &str) -> Option<&'a Value> {
    values
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| &entry.value)
}

/// Whether the policy location of one browser holds somebody else's choice.
fn conflicting(observation: &PolicyObservation, created: &[PolicyCreated]) -> bool {
    if observation.error.as_deref() == Some(reason::CONFLICT) {
        return true;
    }
    desired(observation.browser).iter().any(|want| {
        let ours = created
            .iter()
            .any(|made| made.browser == observation.browser && made.key == want.key);
        let differs = |values: &[PolicyValue]| {
            find(values, &want.key).is_some_and(|found| !same_value(found, &want.value))
        };
        (!ours && differs(&observation.values)) || differs(&observation.foreign)
    })
}

/// Browsers whose policy location holds a value that differs from ours and that
/// `created` does not list as ours: a managed machine, left alone. A differing value in a
/// policy file that is not ours (`foreign`) is always somebody else's, and so is a
/// location the back-end itself reports as `conflict`. Each browser is named once, in the
/// order of `observed`.
pub fn conflicts(observed: &[PolicyObservation], created: &[PolicyCreated]) -> Vec<BrowserFamily> {
    let mut browsers = Vec::new();
    for observation in observed {
        if conflicting(observation, created) && !browsers.contains(&observation.browser) {
            browsers.push(observation.browser);
        }
    }
    browsers
}

/// Whether `observation` holds exactly the values of [`desired`] for its browser.
pub fn is_applied(observation: &PolicyObservation) -> bool {
    desired(observation.browser).iter().all(|want| {
        find(&observation.values, &want.key).is_some_and(|found| same_value(found, &want.value))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(browser: BrowserFamily, values: Vec<PolicyValue>) -> PolicyObservation {
        PolicyObservation {
            browser,
            present: true,
            values,
            foreign: Vec::new(),
            error: None,
        }
    }

    fn made(browser: BrowserFamily, key: &str) -> PolicyCreated {
        PolicyCreated {
            browser,
            key: key.to_owned(),
        }
    }

    fn pairs(browser: BrowserFamily) -> Vec<(String, Value)> {
        desired(browser)
            .into_iter()
            .map(|entry| (entry.key, entry.value))
            .collect()
    }

    #[test]
    fn desired_values_per_browser() {
        let doh = ("DnsOverHttpsMode".to_owned(), json!("off"));
        let safe = ("ForceGoogleSafeSearch".to_owned(), json!(true));
        assert_eq!(
            pairs(BrowserFamily::Chrome),
            vec![doh.clone(), safe.clone()]
        );
        assert_eq!(
            pairs(BrowserFamily::Chromium),
            vec![doh.clone(), safe.clone()]
        );
        assert_eq!(
            pairs(BrowserFamily::Edge),
            vec![
                doh.clone(),
                safe.clone(),
                ("ForceBingSafeSearch".to_owned(), json!(2))
            ]
        );
        assert_eq!(
            pairs(BrowserFamily::Brave),
            vec![doh, safe, ("TorDisabled".to_owned(), json!(true))]
        );
        assert_eq!(
            pairs(BrowserFamily::Firefox),
            vec![
                ("DNSOverHTTPS.Enabled".to_owned(), json!(false)),
                ("DNSOverHTTPS.Locked".to_owned(), json!(true)),
            ]
        );
    }

    #[test]
    fn nothing_disables_private_browsing_or_forces_youtube() {
        for browser in BrowserFamily::ALL {
            for entry in desired(browser) {
                let key = entry.key.to_ascii_lowercase();
                assert!(
                    !key.contains("incognito") && !key.contains("private"),
                    "{key}"
                );
                assert!(!key.contains("youtube"), "{key}");
            }
        }
    }

    #[test]
    fn the_chromium_family_gets_nothing_on_macos() {
        for browser in BrowserFamily::ALL {
            assert!(applies(browser, Os::Windows));
            assert!(applies(browser, Os::Linux));
            assert_eq!(
                applies(browser, Os::Macos),
                browser == BrowserFamily::Firefox
            );
        }
    }

    #[test]
    fn policy_json_per_browser() {
        assert_eq!(
            chromium_json(BrowserFamily::Chrome),
            "{\n  \"DnsOverHttpsMode\": \"off\",\n  \"ForceGoogleSafeSearch\": true\n}\n"
        );
        assert_eq!(
            chromium_json(BrowserFamily::Chromium),
            chromium_json(BrowserFamily::Chrome)
        );
        assert_eq!(
            chromium_json(BrowserFamily::Edge),
            "{\n  \"DnsOverHttpsMode\": \"off\",\n  \"ForceGoogleSafeSearch\": true,\n  \"ForceBingSafeSearch\": 2\n}\n"
        );
        assert_eq!(
            chromium_json(BrowserFamily::Brave),
            "{\n  \"DnsOverHttpsMode\": \"off\",\n  \"ForceGoogleSafeSearch\": true,\n  \"TorDisabled\": true\n}\n"
        );
        assert_eq!(
            firefox_json(),
            "{\n  \"policies\": {\n    \"DNSOverHTTPS\": {\n      \"Enabled\": false,\n      \"Locked\": true\n    }\n  }\n}\n"
        );
        assert_eq!(chromium_json(BrowserFamily::Firefox), firefox_json());
    }

    #[test]
    fn the_files_parse_back_to_the_desired_values() {
        for browser in [
            BrowserFamily::Chrome,
            BrowserFamily::Chromium,
            BrowserFamily::Edge,
            BrowserFamily::Brave,
        ] {
            let parsed: Value = serde_json::from_str(&chromium_json(browser)).unwrap();
            let object = parsed.as_object().unwrap();
            assert_eq!(object.len(), desired(browser).len());
            for entry in desired(browser) {
                assert_eq!(object.get(&entry.key), Some(&entry.value), "{browser:?}");
            }
        }
        let parsed: Value = serde_json::from_str(&firefox_json()).unwrap();
        assert_eq!(
            parsed,
            json!({ "policies": { "DNSOverHTTPS": { "Enabled": false, "Locked": true } } })
        );
    }

    #[test]
    fn values_from_a_registry_compare_by_meaning() {
        assert!(same_value(&json!(true), &json!(1)));
        assert!(same_value(&json!(0), &json!(false)));
        assert!(same_value(&json!(2), &json!(2.0)));
        assert!(same_value(&json!("off"), &json!("off")));
        assert!(!same_value(&json!(true), &json!(0)));
        assert!(!same_value(&json!(true), &json!(2)));
        assert!(!same_value(&json!("off"), &json!("automatic")));
        assert!(!same_value(&json!("1"), &json!(1)));
        assert!(!same_value(&json!(null), &json!(false)));
        assert!(!same_value(&json!("off"), &json!(["off"])));
    }

    #[test]
    fn is_applied_wants_every_value() {
        for browser in BrowserFamily::ALL {
            assert!(
                is_applied(&observation(browser, desired(browser))),
                "{browser:?}"
            );
            assert!(
                !is_applied(&observation(browser, Vec::new())),
                "{browser:?}"
            );
            let mut one_short = desired(browser);
            one_short.pop();
            assert!(!is_applied(&observation(browser, one_short)), "{browser:?}");
            let mut one_changed = desired(browser);
            one_changed[0].value = json!("something else");
            assert!(
                !is_applied(&observation(browser, one_changed)),
                "{browser:?}"
            );
        }
        // A registry reports booleans as numbers.
        let registry = vec![
            value("DnsOverHttpsMode", json!("off")),
            value("ForceGoogleSafeSearch", json!(1)),
            value("ForceBingSafeSearch", json!(2)),
        ];
        assert!(is_applied(&observation(BrowserFamily::Edge, registry)));
    }

    #[test]
    fn a_value_of_somebody_else_is_a_conflict() {
        let managed = observation(
            BrowserFamily::Chrome,
            vec![value("DnsOverHttpsMode", json!("secure"))],
        );
        assert_eq!(
            conflicts(std::slice::from_ref(&managed), &[]),
            vec![BrowserFamily::Chrome]
        );
        // The same value, listed as created by us and changed since, is drift, not a conflict.
        assert_eq!(
            conflicts(
                std::slice::from_ref(&managed),
                &[made(BrowserFamily::Chrome, "DnsOverHttpsMode")]
            ),
            vec![]
        );
        // What we created for another browser, or another key, proves nothing here.
        assert_eq!(
            conflicts(
                &[managed],
                &[
                    made(BrowserFamily::Edge, "DnsOverHttpsMode"),
                    made(BrowserFamily::Chrome, "ForceGoogleSafeSearch"),
                ]
            ),
            vec![BrowserFamily::Chrome]
        );
    }

    #[test]
    fn equal_values_and_empty_locations_are_no_conflict() {
        // Nothing there, our own values, and somebody else's value that equals ours.
        for browser in BrowserFamily::ALL {
            assert_eq!(conflicts(&[observation(browser, Vec::new())], &[]), vec![]);
            assert_eq!(
                conflicts(&[observation(browser, desired(browser))], &[]),
                vec![]
            );
        }
        // A key this tier does not manage is not looked at.
        let other_key = observation(
            BrowserFamily::Chrome,
            vec![value("HomepageLocation", json!("https://intranet.example"))],
        );
        assert_eq!(conflicts(&[other_key], &[]), vec![]);
    }

    #[test]
    fn a_foreign_policy_file_is_a_conflict_whatever_we_created() {
        let mut linux = observation(BrowserFamily::Brave, desired(BrowserFamily::Brave));
        linux.foreign = vec![value("TorDisabled", json!(false))];
        let created: Vec<PolicyCreated> = desired(BrowserFamily::Brave)
            .iter()
            .map(|entry| made(BrowserFamily::Brave, &entry.key))
            .collect();
        assert_eq!(
            conflicts(std::slice::from_ref(&linux), &created),
            vec![BrowserFamily::Brave]
        );
        // A foreign file that says the same as ours is fine.
        linux.foreign = vec![value("TorDisabled", json!(true))];
        assert_eq!(conflicts(&[linux], &created), vec![]);
    }

    #[test]
    fn conflicts_name_each_browser_once_in_order() {
        let firefox = observation(
            BrowserFamily::Firefox,
            vec![value(FIREFOX_ENABLED, json!(true))],
        );
        let edge = observation(
            BrowserFamily::Edge,
            vec![value("ForceBingSafeSearch", json!(0))],
        );
        let chrome = observation(BrowserFamily::Chrome, desired(BrowserFamily::Chrome));
        let mut reported = observation(BrowserFamily::Brave, Vec::new());
        reported.error = Some(reason::CONFLICT.to_owned());
        let mut flatpak = observation(BrowserFamily::Chromium, Vec::new());
        flatpak.error = Some(reason::FLATPAK.to_owned());
        assert_eq!(
            conflicts(
                &[firefox.clone(), chrome, edge, firefox, reported, flatpak],
                &[]
            ),
            vec![
                BrowserFamily::Firefox,
                BrowserFamily::Edge,
                BrowserFamily::Brave
            ]
        );
    }
}
