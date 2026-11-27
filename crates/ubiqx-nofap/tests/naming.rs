//! The three name rings of the `nofap` module (design section 0.1), kept apart by tests.
//!
//! * **Source**: `nofap`: crates, modules, tables, IPC commands, i18n namespaces, source
//!   directories. Not checked here; it is what the owner asked for.
//! * **Machine**: `ubiqx-shield` (`ubiqx_core::nofap::SYSTEM_ID`): everything written to
//!   the user's machine or published. It can never be renamed without migrating data and
//!   system state, and it is readable by other accounts (process arguments, the hosts
//!   file, the state directory), so it never carries the word.
//! * **Display**: `ubiqx_core::nofap::DISPLAY_NAME`, the single place the user-visible name
//!   is defined. Every sentence takes it through the `{name}` placeholder.
//!
//! The owner decided the display name is "NoFap" (integrator decision 1), so
//! [`ALLOW_NOFAP_IN_DISPLAY`] is `true`: the word may appear where the display name is
//! shown, and only as the display name. Everywhere else in the machine ring these tests
//! still fail on it, in any letter case.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use serde_json::Value;
use ubiqx_core::nofap::{
    BrowserFamily, OwnerWants, ResolverId, SystemPlan, SystemState, DISPLAY_NAME, SYSTEM_ID,
};
use ubiqx_core::product::{ProductId, SHIELD, UBIQX};
use ubiqx_core::UiLanguage;
use ubiqx_nofap::{gate, help, hosts, messages, plan, policies, primary, resolvers, state_dir};

/// Whether the display name may contain the word. Set together with `DISPLAY_NAME`.
const ALLOW_NOFAP_IN_DISPLAY: bool = true;

const WORD: &str = "nofap";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    let full = repo().join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
}

fn has_word(text: &str) -> bool {
    text.to_lowercase().contains(WORD)
}

/// `text` with the display name taken out, when the display name may carry the word.
fn without_display_name(text: &str) -> String {
    if ALLOW_NOFAP_IN_DISPLAY {
        text.replace(DISPLAY_NAME, "")
    } else {
        text.to_owned()
    }
}

/// Fails when a machine-ring value carries the word.
fn assert_machine(what: &str, value: &str) {
    assert!(
        !has_word(value),
        "{what} is in the machine ring and contains \"{WORD}\": {value}"
    );
}

/// Fails when a text shown to the user carries the word other than as the display name.
fn assert_display(what: &str, value: &str) {
    assert!(
        !has_word(&without_display_name(value)),
        "{what} contains \"{WORD}\" other than as the display name: {value}"
    );
}

#[test]
fn the_display_name_follows_the_owners_decision() {
    assert_eq!(
        has_word(DISPLAY_NAME),
        ALLOW_NOFAP_IN_DISPLAY,
        "DISPLAY_NAME ({DISPLAY_NAME}) and ALLOW_NOFAP_IN_DISPLAY disagree: change both together"
    );
    assert!(!DISPLAY_NAME.trim().is_empty());
    assert_eq!(SHIELD.display_name, DISPLAY_NAME);
}

#[test]
fn the_machine_id_is_neutral_and_is_what_the_machine_ring_is_built_from() {
    assert_eq!(SYSTEM_ID, "ubiqx-shield");
    assert_machine("SYSTEM_ID", SYSTEM_ID);
    for (what, value) in [
        ("hosts::BEGIN_PREFIX", hosts::BEGIN_PREFIX),
        ("hosts::END_MARKER", hosts::END_MARKER),
        ("hosts::UNDO_HINT", hosts::UNDO_HINT),
        ("hosts::CANARY_NAME", hosts::CANARY_NAME),
        (
            "resolvers::RESOLVED_DROPIN_PATH",
            resolvers::RESOLVED_DROPIN_PATH,
        ),
        (
            "resolvers::RESOLVED_DROPIN_HINT",
            resolvers::RESOLVED_DROPIN_HINT,
        ),
        ("policies::OWN_FILE_NAME", policies::OWN_FILE_NAME),
        ("state_dir::STATE_DIR_MACOS", state_dir::STATE_DIR_MACOS),
        ("state_dir::STATE_DIR_LINUX", state_dir::STATE_DIR_LINUX),
        (
            "state_dir::STATE_DIR_WINDOWS_UNDER_PROGRAM_FILES",
            state_dir::STATE_DIR_WINDOWS_UNDER_PROGRAM_FILES,
        ),
        ("primary::SHARED_DIR", primary::SHARED_DIR),
    ] {
        assert_machine(what, value);
        assert!(
            value.contains(SYSTEM_ID),
            "{what} does not carry the machine id: {value}"
        );
    }
    for (what, value) in [
        ("state_dir::STATE_FILE", state_dir::STATE_FILE),
        ("primary::LOCK_FILE", primary::LOCK_FILE),
        ("gate::OVERRIDE_ENV", gate::OVERRIDE_ENV),
        ("resolvers::PROBE_NAME", resolvers::PROBE_NAME),
    ] {
        assert_machine(what, value);
    }
    assert!(gate::OVERRIDE_ENV.starts_with("UBIQX_SHIELD_"));
}

#[test]
fn nothing_written_to_the_machine_carries_the_word() {
    // The hosts block.
    let names: Vec<String> = [hosts::CANARY_NAME, "example.com", "www.example.com"]
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_machine("the hosts block", &hosts::render_block(&names));

    // The resolver drop-in and the policy files.
    for id in ResolverId::ALL {
        assert_machine("the resolved drop-in", &resolvers::resolved_dropin(id));
    }
    for browser in BrowserFamily::ALL {
        assert_machine("a policy file", &policies::chromium_json(browser));
        for value in policies::desired(browser) {
            assert_machine("a policy key", &value.key);
        }
    }
    assert_machine("the Firefox policy file", &policies::firefox_json());

    // The plan (a process argument, visible to other accounts) and the state file.
    let plan = SystemPlan {
        v: 1,
        owner: ProductId::Shield,
        hosts: true,
        extra: vec!["example.com".to_owned()],
        resolver: Some(ResolverId::CloudflareFamily),
        policies: true,
    };
    assert_machine("the plan", &serde_json::to_string(&plan).unwrap());
    let argument = plan::encode_plan(&plan);
    assert_eq!(plan::decode_plan(&argument).unwrap(), plan);
    let owners: BTreeMap<ProductId, OwnerWants> = ProductId::ALL
        .into_iter()
        .map(|owner| (owner, plan::wants_of(&plan)))
        .collect();
    let state = SystemState {
        v: 1,
        owners,
        hosts: Some(hosts::stamp_of(&names)),
        ..SystemState::default()
    };
    assert_machine("the state file", &serde_json::to_string(&state).unwrap());
    for owner in ProductId::ALL {
        assert_machine("an owner id", owner.as_str());
    }

    // The primary lock.
    let lock = primary::PrimaryLock {
        product: ProductId::Shield,
        pid: 1,
        heartbeat_at: Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap(),
    };
    assert_machine("the primary lock", &primary::render_lock(&lock));
}

#[test]
fn both_product_identities_are_neutral_outside_the_display_name() {
    for product in [&UBIQX, &SHIELD] {
        let id = product.id.as_str();
        for (what, value) in [
            ("bundle_id", product.bundle_id),
            ("keyring_service", product.keyring_service),
            ("db_file", product.db_file),
            ("feed_product", product.feed_product),
            ("feed_url", product.feed_url),
        ] {
            assert_machine(&format!("{id}.{what}"), value);
        }
        for stem in product.exe_stems {
            assert_machine(&format!("{id}.exe_stems"), stem);
        }
        assert_display(&format!("{id}.display_name"), product.display_name);
        for name in product.app_names {
            assert_display(&format!("{id}.app_names"), name);
        }
    }
    assert!(SHIELD.bundle_id.ends_with(".shield"));
    assert!(SHIELD.exe_stems.contains(&SYSTEM_ID));
    assert!(SHIELD.feed_url.contains("shield"));
    assert_eq!(SHIELD.feed_product, SYSTEM_ID);
}

/// The values of `const NAME: … str … = "…";` items in Rust source: what a module writes
/// to the machine is named by its string constants.
fn string_constants(source: &str) -> Vec<(String, String)> {
    let mut constants = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("const ") {
        let item = &rest[at + "const ".len()..];
        let end = item.find(';').unwrap_or(item.len());
        let (declaration, after) = item.split_at(end);
        rest = after;
        let Some((signature, value)) = declaration.split_once('=') else {
            continue;
        };
        let Some((name, ty)) = signature.split_once(':') else {
            continue;
        };
        if !ty.contains("str") {
            continue;
        }
        // Every string literal on the right-hand side (a constant may be a list).
        let mut chars = value.chars();
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            let mut literal = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => literal.extend(chars.next()),
                    '"' => break,
                    other => literal.push(other),
                }
            }
            constants.push((name.trim().to_owned(), literal));
        }
    }
    constants
}

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn the_constants_of_the_system_layer_are_neutral() {
    let mut files = Vec::new();
    rust_files(
        &repo().join("crates/ubiqx-platform/src/sysblock"),
        &mut files,
    );
    rust_files(&repo().join("crates/ubiqx-platform/src/bin"), &mut files);
    assert!(
        !files.is_empty(),
        "the system layer has moved: update this test"
    );
    let mut seen = 0;
    let mut helper_flag = None;
    for file in &files {
        let source = std::fs::read_to_string(file).unwrap();
        for (name, value) in string_constants(&source) {
            seen += 1;
            assert_machine(&format!("{} in {}", name, file.display()), &value);
            if name == "HELPER_FLAG" {
                helper_flag = Some(value);
            }
        }
    }
    assert!(seen > 0, "no string constant found in the system layer");
    // Process arguments are visible to other accounts.
    assert_eq!(helper_flag.as_deref(), Some("--shield-helper"));
}

#[test]
fn the_constant_reader_reads_constants() {
    let source = r#"
        /// const in a comment = "ignored because it has no type";
        pub const HELPER_FLAG: &str = "--shield-helper";
        const LIMIT: usize = 3;
        pub(crate) const PATHS: &[&str] = &["/etc/one", "/etc/\"two\""];
        static OTHER: &str = "not a constant";
        const LATER: &'static str =
            "after a line break";
    "#;
    assert_eq!(
        string_constants(source),
        vec![
            ("HELPER_FLAG".to_owned(), "--shield-helper".to_owned()),
            ("PATHS".to_owned(), "/etc/one".to_owned()),
            ("PATHS".to_owned(), "/etc/\"two\"".to_owned()),
            ("LATER".to_owned(), "after a line break".to_owned()),
        ]
    );
}

/// Keys of `tauri.conf.json` whose value is text shown to the user.
const DISPLAY_KEYS: &[&str] = &[
    "productName",
    "title",
    "shortDescription",
    "longDescription",
];

/// Walks a `tauri.conf.json`: a display key may carry the display name, everything else
/// is the machine ring. `build` holds source paths and commands of the build machine and
/// `$schema` an address; neither reaches the user's machine.
fn check_tauri_conf(what: &str, key: &str, value: &Value, top_level: bool) {
    match value {
        Value::String(text) => {
            let label = format!("{what}: {key}");
            if DISPLAY_KEYS.contains(&key) {
                assert_display(&label, text);
            } else {
                assert_machine(&label, text);
            }
        }
        Value::Array(items) => {
            for item in items {
                check_tauri_conf(what, key, item, false);
            }
        }
        Value::Object(fields) => {
            for (field, inner) in fields {
                if top_level && (field == "build" || field == "$schema") {
                    continue;
                }
                assert_machine(&format!("{what}: a key under {key}"), field);
                check_tauri_conf(what, field, inner, false);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

const SHELLS: [&str; 2] = ["apps/desktop/src-tauri", "apps/nofap/src-tauri"];

#[test]
fn the_shell_configurations_keep_the_rings_apart() {
    for shell in SHELLS {
        let path = format!("{shell}/tauri.conf.json");
        let conf: Value = serde_json::from_str(&read(&path)).unwrap();
        check_tauri_conf(&path, "", &conf, true);
    }

    // The standalone app: the name the user sees is the display name, the names the
    // machine sees are the machine id. (ubiqX's own name is not this module's.)
    let conf: Value = serde_json::from_str(&read("apps/nofap/src-tauri/tauri.conf.json")).unwrap();
    assert_eq!(
        conf["productName"], DISPLAY_NAME,
        "productName and DISPLAY_NAME differ"
    );
    assert_eq!(conf["mainBinaryName"], SYSTEM_ID);
    assert_eq!(conf["identifier"], SHIELD.bundle_id);
    let windows = conf["app"]["windows"].as_array().unwrap();
    assert!(!windows.is_empty());
    for window in windows {
        if window["label"] == "main" {
            assert_eq!(
                window["title"], DISPLAY_NAME,
                "the window title and DISPLAY_NAME differ"
            );
        }
    }
    let ubiqx: Value =
        serde_json::from_str(&read("apps/desktop/src-tauri/tauri.conf.json")).unwrap();
    assert_eq!(ubiqx["identifier"], UBIQX.bundle_id);
    assert!(
        !has_word(&ubiqx.to_string()),
        "ubiqX's configuration names the module"
    );
}

/// Every file of a shell that is neither Rust source, the manifest, the build script,
/// generated output nor an icon: capability files, `Info.plist`, packaging scripts,
/// installer hooks, polkit actions.
fn packaging_files(dir: &Path, top: bool, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            let skipped = top && ["src", "gen", "icons", "target"].contains(&name.as_str());
            if !skipped {
                packaging_files(&path, false, found);
            }
        } else if !(top
            && ["Cargo.toml", "build.rs", "tauri.conf.json", ".gitignore"].contains(&name.as_str()))
        {
            found.push(path);
        }
    }
}

#[test]
fn capability_plist_and_packaging_files_are_neutral() {
    let mut checked = 0;
    for shell in SHELLS {
        let mut files = Vec::new();
        packaging_files(&repo().join(shell), true, &mut files);
        for file in files {
            assert_machine("a file name", &file.file_name().unwrap().to_string_lossy());
            // Binary files (images) are not text and carry no name.
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            checked += 1;
            assert_display(&file.display().to_string(), &text);
        }
    }
    // ubiqX's capability files and Info.plist exist today; the standalone app's come
    // with its shell and are covered by the same walk.
    assert!(checked >= 3, "only {checked} packaging files were found");
    for path in [
        "apps/desktop/src-tauri/capabilities/default.json",
        "apps/desktop/src-tauri/Info.plist",
    ] {
        assert_machine(path, &read(path));
    }
}

#[test]
fn the_standalone_binary_is_named_after_the_machine_id() {
    let manifest = read("apps/nofap/src-tauri/Cargo.toml");
    let bin = manifest
        .split("[[bin]]")
        .nth(1)
        .expect("the standalone shell declares its binary");
    let name = bin
        .lines()
        .find_map(|line| line.trim().strip_prefix("name"))
        .and_then(|rest| rest.split('"').nth(1))
        .expect("the binary has a name");
    assert_eq!(name, SYSTEM_ID);
    let description = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("description"))
        .expect("the manifest has a description");
    assert_machine(
        "the manifest's description (installers fall back to it)",
        description,
    );
}

#[test]
fn sentences_take_the_name_from_the_placeholder_only() {
    for (key, pt, en) in messages::all() {
        for text in [pt, en] {
            assert!(
                !has_word(text),
                "messages `{key}` spells the name out instead of using {{name}}: {text}"
            );
        }
    }
    // Rendered, the word appears only as the display name.
    for lang in UiLanguage::ALL {
        let tray = messages::tray_labels(lang);
        for label in [
            &tray.open,
            &tray.status_on,
            &tray.breather,
            &tray.check_updates,
            &tray.quit,
            &tray.not_paused,
        ] {
            assert_display("a tray label", label);
        }
        for kind in [messages::Notice::Blocked, messages::Notice::Drift] {
            let (title, body) = messages::notification_text(kind, lang);
            assert_machine("the notification title", &title);
            assert_display("a notification", &body);
        }
        for system_applied in [true, false] {
            let dialog = messages::quit_dialog(system_applied, lang);
            assert_display("the quit dialog", &dialog.message);
            assert_machine("a quit dialog button", &dialog.keep_open);
            assert_machine("a quit dialog button", &dialog.quit);
        }
    }
    // What is on the OS's notification list and in a window title never names the module.
    assert_machine("NOTIFICATION_TITLE", messages::NOTIFICATION_TITLE);
}

#[test]
fn the_help_resources_do_not_name_the_module() {
    let embedded = help::resources();
    assert!(!embedded.is_empty());
    assert_machine(
        "resources.json",
        &read("crates/ubiqx-nofap/data/resources.json"),
    );
    for entry in embedded {
        for text in [&entry.id, &entry.name, &entry.note_pt, &entry.note_en] {
            assert_machine("a help resource", text);
        }
    }
}

#[test]
fn the_checks_themselves_can_fail() {
    assert!(has_word("ai.ubiqx.NoFap"));
    assert!(has_word("/var/lib/ubiqx-nofap/state.json"));
    assert!(!has_word("/var/lib/ubiqx-shield/state.json"));
    // The display name is taken out exactly as it is written, and only then.
    assert!(!has_word(&without_display_name(DISPLAY_NAME)));
    assert!(has_word(&without_display_name("ai.ubiqx.nofap")));
    assert!(has_word(&without_display_name(&format!(
        "{DISPLAY_NAME} (nofap-helper)"
    ))));
    let hostile = std::panic::catch_unwind(|| assert_machine("a marker", "# >>> nofap v1 BEGIN"));
    assert!(hostile.is_err());
    let conf: Value = serde_json::json!({
        "productName": DISPLAY_NAME,
        "build": { "frontendDist": "../dist-nofap" },
        "bundle": { "linux": { "deb": { "files": { "/usr/share/nofap/hook": "hook" } } } }
    });
    let walked = std::panic::catch_unwind(|| check_tauri_conf("a test", "", &conf, true));
    assert!(
        walked.is_err(),
        "a machine key with the word must fail the walk"
    );
}
