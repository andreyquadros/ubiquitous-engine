//! The two products built from this workspace: ubiqX itself and the standalone app of the
//! `nofap` module. Everything that differs between them at run time (bundle identifier,
//! data file, keyring service, update feed) is one [`ProductIdentity`], so the shared code
//! never branches on a name.
//!
//! Both identities are in every exempt list of every guard (`focus::EXEMPT_APP_IDS`,
//! `focus::EXEMPT_APP_NAMES`, the platform's self stems), so neither product ever acts on
//! the other's windows.

use serde::{Deserialize, Serialize};

use crate::nofap::DISPLAY_NAME;
use crate::update::DEFAULT_FEED_URL;

/// Which product a process, a system plan or a state-file entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductId {
    /// ubiqX, hosting the module.
    Ubiqx,
    /// The standalone app (machine id `ubiqx-shield`).
    Shield,
}

impl ProductId {
    pub const ALL: [ProductId; 2] = [ProductId::Ubiqx, ProductId::Shield];

    /// Stable id used in the system state file, the helper's `--owner` argument and the
    /// primary lock.
    pub fn as_str(self) -> &'static str {
        match self {
            ProductId::Ubiqx => "ubiqx",
            ProductId::Shield => "shield",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ubiqx" => Some(ProductId::Ubiqx),
            "shield" => Some(ProductId::Shield),
            _ => None,
        }
    }

    /// The other product (the one this process may share a machine with).
    pub fn other(self) -> Self {
        match self {
            ProductId::Ubiqx => ProductId::Shield,
            ProductId::Shield => ProductId::Ubiqx,
        }
    }

    pub fn identity(self) -> &'static ProductIdentity {
        match self {
            ProductId::Ubiqx => &UBIQX,
            ProductId::Shield => &SHIELD,
        }
    }
}

/// Everything that names a product on the user's machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductIdentity {
    pub id: ProductId,
    /// Bundle identifier (macOS) and Tauri identifier.
    pub bundle_id: &'static str,
    /// Name shown to the user.
    pub display_name: &'static str,
    /// Window and application names the guards treat as "this product" when the id is
    /// missing.
    pub app_names: &'static [&'static str],
    /// Executable stems the shell may run under on Windows and Linux.
    pub exe_stems: &'static [&'static str],
    /// Service name of the OS keyring entries.
    pub keyring_service: &'static str,
    /// File name of the SQLite database inside the data directory.
    pub db_file: &'static str,
    /// The `product` field of this product's update feed; a feed naming another product is
    /// ignored (see [`crate::update::feed_matches_product`]).
    pub feed_product: &'static str,
    /// Default URL of this product's update feed (`latest.json`).
    pub feed_url: &'static str,
}

/// Update feed of the standalone app: its own rolling release, named after the machine id.
pub const SHIELD_FEED_URL: &str =
    "https://github.com/andreyquadros/ubiquitous-engine/releases/download/shield-continuous/latest.json";

pub static UBIQX: ProductIdentity = ProductIdentity {
    id: ProductId::Ubiqx,
    bundle_id: "ai.ubiqx.app",
    display_name: "ubiqX",
    app_names: &["ubiqX"],
    exe_stems: &["ubiqx", "ubiqx-desktop", "ubiqx_desktop"],
    keyring_service: "ai.ubiqx",
    db_file: "ubiqx.db",
    feed_product: "ubiqX",
    feed_url: DEFAULT_FEED_URL,
};

pub static SHIELD: ProductIdentity = ProductIdentity {
    id: ProductId::Shield,
    bundle_id: "ai.ubiqx.shield",
    display_name: DISPLAY_NAME,
    // The window title is the display name; a development build runs under the machine id.
    app_names: &[DISPLAY_NAME, "ubiqx-shield"],
    exe_stems: &["ubiqx-shield", "ubiqx_shield"],
    keyring_service: "ai.ubiqx.shield",
    db_file: "shield.db",
    feed_product: "ubiqx-shield",
    feed_url: SHIELD_FEED_URL,
};

/// Both identities, ubiqX first.
pub static ALL: [&ProductIdentity; 2] = [&UBIQX, &SHIELD];

/// Whether an application id or name belongs to either product (case-insensitive): a
/// bundle id, an executable stem or an application name.
pub fn is_own_app(app_id: &str, app_name: &str) -> bool {
    ALL.iter().any(|p| {
        p.bundle_id.eq_ignore_ascii_case(app_id)
            || p.exe_stems.iter().any(|s| s.eq_ignore_ascii_case(app_id))
            || p.app_names.iter().any(|n| n.eq_ignore_ascii_case(app_name))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::focus::{is_exempt_app, EXEMPT_APP_IDS, EXEMPT_APP_NAMES, SELF_BUNDLE_ID};

    #[test]
    fn nofap_product_ids_round_trip() {
        for id in ProductId::ALL {
            assert_eq!(ProductId::parse(id.as_str()), Some(id));
            assert_eq!(
                serde_json::to_string(&id).unwrap(),
                format!("\"{}\"", id.as_str())
            );
            assert_eq!(id.identity().id, id);
            assert_eq!(id.other().other(), id);
            assert_ne!(id.other(), id);
        }
        assert_eq!(ProductId::parse("all"), None);
        assert_eq!(ProductId::parse("Ubiqx"), None);
    }

    #[test]
    fn nofap_identities_share_nothing_on_disk() {
        assert_eq!(UBIQX.bundle_id, SELF_BUNDLE_ID);
        assert_ne!(UBIQX.bundle_id, SHIELD.bundle_id);
        assert_ne!(UBIQX.keyring_service, SHIELD.keyring_service);
        assert_ne!(UBIQX.db_file, SHIELD.db_file);
        assert_ne!(UBIQX.feed_product, SHIELD.feed_product);
        assert_ne!(UBIQX.feed_url, SHIELD.feed_url);
        assert!(SHIELD.feed_url.contains("/shield-continuous/"));
        for stem in SHIELD.exe_stems {
            assert!(!UBIQX.exe_stems.contains(stem));
        }
    }

    #[test]
    fn nofap_shell_build_scripts_compile_in_their_own_feed() {
        // A build script cannot depend on this crate, so each shell repeats its default
        // feed URL. A shell stamped with the other product's feed would never be offered an
        // update (the checker ignores a feed that names another product).
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (script, own, other) in [
            ("apps/desktop/src-tauri/build.rs", &UBIQX, &SHIELD),
            ("apps/nofap/src-tauri/build.rs", &SHIELD, &UBIQX),
        ] {
            let text = std::fs::read_to_string(root.join(script))
                .unwrap_or_else(|e| panic!("{script}: {e}"));
            assert!(
                text.contains(&format!("\"{}\"", own.feed_url)),
                "{script} does not default to {}",
                own.feed_url
            );
            assert!(
                !text.contains(other.feed_url),
                "{script} names the other product's feed"
            );
        }
    }

    #[test]
    fn nofap_identities_are_in_the_exempt_lists() {
        for p in ALL {
            assert!(
                EXEMPT_APP_IDS.contains(&p.bundle_id),
                "{} is not an exempt id",
                p.bundle_id
            );
            for stem in p.exe_stems {
                assert!(EXEMPT_APP_IDS.contains(stem), "{stem} is not an exempt id");
            }
            for name in p.app_names {
                assert!(
                    EXEMPT_APP_NAMES.contains(name),
                    "{name} is not an exempt name"
                );
            }
            assert!(is_exempt_app(p.bundle_id, "", None));
            assert!(is_exempt_app("", p.display_name, None));
            assert!(is_own_app(p.bundle_id, ""));
            assert!(is_own_app("", p.display_name));
        }
        assert!(is_own_app("UBIQX-SHIELD", ""));
        assert!(!is_own_app("com.google.Chrome", "Google Chrome"));
        assert!(!is_own_app("", ""));
    }

    #[test]
    fn nofap_machine_ring_never_carries_the_source_name() {
        // Section 0.1: what the operating system sees is neutral, whatever the display name.
        for p in ALL {
            for machine in [
                p.bundle_id,
                p.keyring_service,
                p.db_file,
                p.feed_product,
                p.feed_url,
            ]
            .into_iter()
            .chain(p.exe_stems.iter().copied())
            {
                assert!(
                    !machine.to_ascii_lowercase().contains("nofap"),
                    "{machine} names the module"
                );
            }
        }
    }
}
