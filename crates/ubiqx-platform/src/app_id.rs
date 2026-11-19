//! Application identity on Windows and Linux: the executable's file stem, lower-case and
//! without extension (`chrome`, `msedge`, `firefox`, `code`). macOS keeps its bundle ids.
//!
//! Focus targets, rules and the app catalogue all use this key, so every adapter derives it
//! through [`exe_stem`] and nothing else.

/// Executable stems the two products' shells may run under: ubiqX (Tauri names the binary
/// after the product on Windows and kebab-cases it on Linux; a `cargo run` uses the crate
/// name) and the standalone app, whose binary carries the machine id. The same stems as
/// `ubiqx_core::product::{UBIQX, SHIELD}.exe_stems`, which a test below compares.
pub const SELF_STEMS: &[&str] = &[
    "ubiqx",
    "ubiqx-desktop",
    "ubiqx_desktop",
    "ubiqx-shield",
    "ubiqx_shield",
];

/// The app id of an executable path: the file name without directory and extension,
/// lower-cased. Accepts both separators, so a Windows path parsed on any host gives the
/// same answer. Empty when the path has no file name.
pub fn exe_stem(path: &str) -> String {
    let file = path
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    let stem = match file.rfind('.') {
        // `.hidden` style names keep the dot; `chrome.exe` drops the extension.
        Some(0) | None => file,
        Some(i) => &file[..i],
    };
    stem.to_lowercase()
}

/// Whether an app id belongs to one of the two products (never enforced, never hidden).
pub fn is_self_stem(app_id: &str) -> bool {
    SELF_STEMS.iter().any(|s| s.eq_ignore_ascii_case(app_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_from_both_path_styles() {
        assert_eq!(
            exe_stem(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            "chrome"
        );
        assert_eq!(exe_stem("/usr/lib/firefox/firefox"), "firefox");
        assert_eq!(
            exe_stem("/usr/bin/gnome-terminal-server"),
            "gnome-terminal-server"
        );
        assert_eq!(exe_stem("Code.exe"), "code");
        assert_eq!(exe_stem("/opt/app/bin/app.bin"), "app");
        assert_eq!(exe_stem("/snap/x/.hidden"), ".hidden");
        assert_eq!(exe_stem("  msedge.exe  "), "msedge");
        assert_eq!(exe_stem(""), "");
        assert_eq!(exe_stem("/usr/bin/"), "");
    }

    #[test]
    fn recognises_itself() {
        assert!(is_self_stem("ubiqx"));
        assert!(is_self_stem("UbiqX"));
        assert!(is_self_stem("ubiqx-desktop"));
        assert!(!is_self_stem("chrome"));
    }

    #[test]
    fn both_products_are_exempt() {
        assert!(is_self_stem("ubiqx-shield"));
        assert!(is_self_stem("UBIQX_SHIELD"));
        for product in ubiqx_core::product::ALL {
            for stem in product.exe_stems {
                assert!(SELF_STEMS.contains(stem), "{stem} is missing");
            }
        }
        let known: usize = ubiqx_core::product::ALL
            .iter()
            .map(|product| product.exe_stems.len())
            .sum();
        assert_eq!(SELF_STEMS.len(), known, "a stem no product declares");
    }
}
