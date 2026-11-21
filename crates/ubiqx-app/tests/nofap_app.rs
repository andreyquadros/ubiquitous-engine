//! The standalone product's composition root (design 8.2, 8.3, 11.2): a scripted session
//! with blocks, an AI check that acts and every command of the page leaves no row in a
//! productivity table; the host commands round-trip; the standalone identity is its own.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use serde_json::{json, Value};
use ubiqx_ai::fake::FakeExplicitDetector;
use ubiqx_app::nofap_app::{
    HostAbout, HostSettings, HostSettingsPatch, NofapApp, NofapAppConfig, NofapHostCommand,
};
use ubiqx_app::AiBackend;
use ubiqx_core::clock::FixedClock;
use ubiqx_core::nofap::{Confidence, ExplicitVerdict, NofapEvent, NofapSink, Overall, TierState};
use ubiqx_core::nofap_ipc::{ConsentArgs, EnableOptions, NofapCommand, NofapView, VisionScope};
use ubiqx_core::ports::{PermissionKind, PermissionState, PermissionStatus};
use ubiqx_core::product::{ProductId, SHIELD, UBIQX};
use ubiqx_core::{AiProvider, UiLanguage, UpdateStatus};
use ubiqx_engine::nofap::{Background, DisplaySession, Tuning};
use ubiqx_platform::mock::{Scenario, ScriptedFrameSource, Step};

const CHROME: &str = "com.google.Chrome";
const VLC: &str = "org.videolan.vlc";

/// The productivity tables the standalone product must never write (design 8.2).
const PRODUCTIVITY_TABLES: [&str; 9] = [
    "blocks",
    "screenshots",
    "interventions",
    "nudges",
    "reports",
    "corrections",
    "rules",
    "focus_sessions",
    "focus_targets",
];

/// The module's own tables.
const NOFAP_TABLES: [&str; 3] = ["nofap_state", "nofap_sites", "nofap_day"];

fn listed() -> &'static str {
    ubiqx_nofap::list::canary_block()[0]
}

/// Records every event, so a test can see the service talk to its shell.
#[derive(Default)]
struct RecordingSink(parking_lot::Mutex<Vec<NofapEvent>>);

impl NofapSink for RecordingSink {
    fn emit(&self, event: NofapEvent) {
        self.0.lock().push(event);
    }
}

struct Session {
    app: NofapApp,
    detector: Arc<FakeExplicitDetector>,
    clock: FixedClock,
    sink: Arc<RecordingSink>,
    dir: tempfile::TempDir,
}

fn session(ai: AiBackend) -> Session {
    let dir = tempfile::tempdir().unwrap();
    let detector = Arc::new(FakeExplicitDetector::new());
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap());
    let sink = Arc::new(RecordingSink::default());
    let config = NofapAppConfig {
        data_dir: Some(dir.path().to_path_buf()),
        scripted_platform: Some(Scenario::new(vec![Step::browser(
            "Google Chrome",
            CHROME,
            "Receitas da semana",
            &format!("https://{}/pagina", listed()),
            1,
        )])),
        ai,
        loops: false,
        detector: Some(detector.clone()),
        frames: Some(Arc::new(ScriptedFrameSource::new())),
        clock: Some(Arc::new(clock.clone())),
        tuning: Some(Tuning {
            confirm_after: Duration::from_millis(5),
            check_timeout: Duration::from_secs(5),
            background: Background::Inline,
            display: DisplaySession::Full,
        }),
        ..NofapAppConfig::default()
    };
    let app = NofapApp::start(config, sink.clone()).unwrap();
    Session {
        app,
        detector,
        clock,
        sink,
        dir,
    }
}

fn options() -> EnableOptions {
    EnableOptions {
        self_use: true,
        hosts: true,
        resolver: None,
        policies: false,
        vision: true,
        vision_consent: Some(ConsentArgs {
            provider: AiProvider::OpenAi,
            managed: false,
        }),
        vision_scope: VisionScope {
            unknown_sites: true,
            media_apps: true,
            messaging_apps: false,
            private_windows: true,
        },
        delay_secs: 600,
        values: "Quero estar presente com quem eu amo.".into(),
        discreet: true,
    }
}

fn count(app: &NofapApp, table: &str) -> i64 {
    app.store
        .db()
        .with(|conn| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(Into::into)
        })
        .unwrap()
}

fn commands_fixture() -> Vec<Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/src/nofap/fixtures/commands.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).expect("commands.json is a JSON array")
}

async fn host(app: &NofapApp, command: Value) -> Value {
    let command: NofapHostCommand = serde_json::from_value(command).expect("a host command");
    app.handle_host(command).await.unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_whole_session_leaves_no_row_in_a_productivity_table() {
    let s = session(AiBackend::Fake);
    let app = &s.app;
    for table in PRODUCTIVITY_TABLES.iter().chain(NOFAP_TABLES.iter()) {
        assert_eq!(count(app, table), 0, "{table} before the session");
    }
    // The AI check needs the onboarding of this product and a provider of its own.
    host(
        app,
        json!({ "cmd": "settings_set", "args": { "onboarding_done": true, "ai_provider": "openai" } }),
    )
    .await;

    let view = app.service.enable(options()).unwrap();
    assert_eq!(view.product, ProductId::Shield);
    assert_eq!(view.display_name, SHIELD.display_name);
    // A scripted platform never gets the machine's system layer.
    assert_eq!(
        view.status.hosts,
        TierState::Unsupported {
            reason: ubiqx_engine::deps::NO_SYSTEM_LAYER.into()
        }
    );

    // A listed site on two passes is closed.
    app.service.guard_pass().unwrap();
    assert!(
        app.service.guard_pass().unwrap().hit,
        "a listed site is closed"
    );

    // A media player with something explicit on screen: one check, and it acts.
    app.scripted
        .as_ref()
        .unwrap()
        .load(Scenario::new(vec![Step::app("VLC", VLC, "ferias.mp4", 1)]));
    app.service.guard_pass().unwrap();
    s.clock.advance(ChronoDuration::seconds(25));
    let job = app.service.guard_pass().unwrap().job.expect("a check");
    s.detector.push(Ok(FakeExplicitDetector::verdict(
        ExplicitVerdict::Explicit,
        Confidence::High,
    )));
    assert!(app.service.vision_check(job).await.unwrap().hit);
    assert_eq!(s.detector.calls(), 1);
    assert!(count(app, "nofap_day") > 0, "the hits were counted");

    // Every request the page and the card window can send.
    for request in commands_fixture() {
        let command: NofapCommand = serde_json::from_value(request.clone())
            .unwrap_or_else(|error| panic!("{request}: {error}"));
        // Some are refused in this state (an unknown id, `reset` while on); what matters
        // here is what they write.
        let _ = app.handle(command);
    }
    let card = s
        .sink
        .0
        .lock()
        .iter()
        .find_map(|event| match event {
            NofapEvent::Card { card } => Some(card.clone()),
            _ => None,
        })
        .expect("the hits produced a card");
    let _ = app.handle(NofapCommand::Card {
        id: card.id.clone(),
    });
    for action in card.actions.clone() {
        let _ = app.handle(NofapCommand::CardAction {
            id: card.id.clone(),
            action,
        });
    }
    app.service.verify_pass().unwrap();

    for table in PRODUCTIVITY_TABLES {
        assert_eq!(count(app, table), 0, "{table} after the session");
    }
    // The module itself did write: its configuration (the fixture's `erase_history`
    // cleared the counters again).
    assert_eq!(count(app, "nofap_state"), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_standalone_identity_is_its_own() {
    let s = session(AiBackend::Fake);
    // Its own database file, next to nothing of ubiqX's.
    assert_eq!(SHIELD.db_file, "shield.db");
    assert!(s.dir.path().join(SHIELD.db_file).is_file());
    assert!(!s.dir.path().join(UBIQX.db_file).exists());
    // Its own keyring service, data directory and feed.
    assert_ne!(SHIELD.keyring_service, UBIQX.keyring_service);
    assert!(ubiqx_app::nofap_app::default_data_dir().ends_with(SHIELD.bundle_id));
    assert_eq!(s.app.update_status().feed_url, SHIELD.feed_url);
    let view: NofapView =
        serde_json::from_value(s.app.handle(NofapCommand::View).unwrap()).unwrap();
    assert_eq!(view.product, ProductId::Shield);
    assert_eq!(view.status.overall, Overall::Off);
    let about: HostAbout =
        serde_json::from_value(host(&s.app, json!({ "cmd": "about" })).await).unwrap();
    assert_eq!(about.display_name, SHIELD.display_name);
    assert_eq!(about.identifier, SHIELD.bundle_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_settings_round_trip_and_keep_the_provider_coherent() {
    let s = session(AiBackend::Fake);
    let app = &s.app;
    let read = |value: Value| -> HostSettings { serde_json::from_value(value).unwrap() };

    let first = read(host(app, json!({ "cmd": "settings_get" })).await);
    assert!(!first.onboarding_done);
    assert_eq!(first.ui_language, UiLanguage::PtBr);

    // Only the fields given change.
    let after = read(
        host(
            app,
            json!({ "cmd": "settings_set", "args": { "ui_language": "en", "check_updates": false } }),
        )
        .await,
    );
    assert_eq!(after.ui_language, UiLanguage::En);
    assert!(!after.check_updates);
    assert_eq!(after.ai_provider, first.ai_provider);
    assert_eq!(
        read(host(app, json!({ "cmd": "settings_get" })).await),
        after
    );
    // The module speaks the product's language at once.
    assert_eq!(app.service.language(), UiLanguage::En);

    // Another vendor brings its own vision model along.
    let xai = read(
        host(
            app,
            json!({ "cmd": "settings_set", "args": { "ai_provider": "xai" } }),
        )
        .await,
    );
    assert_eq!(xai.ai_provider, AiProvider::Xai);
    assert_eq!(
        AiProvider::for_model(&xai.vision_model),
        Some(AiProvider::Xai)
    );
    // A model of another vendor is refused, and so is the managed provider.
    for refused in [
        NofapHostCommand::SettingsSet(HostSettingsPatch {
            vision_model: Some("gpt-4.1-mini".into()),
            ..HostSettingsPatch::default()
        }),
        NofapHostCommand::SettingsSet(HostSettingsPatch {
            ai_provider: Some(AiProvider::Ubi),
            ..HostSettingsPatch::default()
        }),
        NofapHostCommand::ApiKeySet {
            provider: AiProvider::Ubi,
            key: "chave".into(),
        },
        NofapHostCommand::Models {
            provider: AiProvider::Ubi,
        },
    ] {
        let name = refused.name();
        assert!(app.handle_host(refused).await.is_err(), "{name}");
    }
    assert_eq!(read(host(app, json!({ "cmd": "settings_get" })).await), xai);
    let custom = read(
        host(
            app,
            json!({ "cmd": "settings_set", "args": { "vision_model": "grok-2-vision" } }),
        )
        .await,
    );
    assert_eq!(custom.vision_model, "grok-2-vision");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_key_permissions_and_updates_go_through_the_host_command() {
    let s = session(AiBackend::Fake);
    let app = &s.app;
    let secret = AiProvider::OpenAi.secret_key();

    assert_eq!(
        host(
            app,
            json!({ "cmd": "api_key_set", "args": { "provider": "openai", "key": " sk-ana " } }),
        )
        .await,
        Value::Null
    );
    assert_eq!(
        app.platform.secrets.get(secret).unwrap().as_deref(),
        Some("sk-ana")
    );
    assert!(app
        .handle_host(NofapHostCommand::ApiKeySet {
            provider: AiProvider::OpenAi,
            key: "   ".into(),
        })
        .await
        .is_err());
    host(
        app,
        json!({ "cmd": "api_key_clear", "args": { "provider": "openai" } }),
    )
    .await;
    assert_eq!(app.platform.secrets.get(secret).unwrap(), None);

    app.scripted
        .as_ref()
        .unwrap()
        .set_permissions(PermissionStatus {
            screen_recording: PermissionState::Denied,
            automation: PermissionState::Granted,
            accessibility: PermissionState::Granted,
        });
    let permissions: PermissionStatus =
        serde_json::from_value(host(app, json!({ "cmd": "permissions" })).await).unwrap();
    assert_eq!(permissions.screen_recording, PermissionState::Denied);
    let request = NofapHostCommand::PermissionRequest {
        kind: PermissionKind::ScreenRecording,
    };
    assert_eq!(app.handle_host(request).await.unwrap(), Value::Null);

    // The scripted feed has nothing newer: a manual check says so, with this product's feed.
    let status: UpdateStatus =
        serde_json::from_value(host(app, json!({ "cmd": "update_check" })).await).unwrap();
    assert_eq!(status.available, None);
    assert_eq!(status.feed_url, SHIELD.feed_url);
}

#[test]
fn host_commands_have_the_wire_form_of_the_module_command() {
    let commands = [
        json!({ "cmd": "settings_get" }),
        json!({ "cmd": "settings_set", "args": { "check_updates": true } }),
        json!({ "cmd": "api_key_set", "args": { "provider": "anthropic", "key": "k" } }),
        json!({ "cmd": "api_key_clear", "args": { "provider": "xai" } }),
        json!({ "cmd": "models", "args": { "provider": "openai" } }),
        json!({ "cmd": "permissions" }),
        json!({ "cmd": "permission_request", "args": { "kind": "automation" } }),
        json!({ "cmd": "update_check" }),
        json!({ "cmd": "about" }),
    ];
    for value in commands {
        let command: NofapHostCommand =
            serde_json::from_value(value.clone()).unwrap_or_else(|e| panic!("{value}: {e}"));
        assert_eq!(command.name(), value["cmd"].as_str().unwrap());
        assert_eq!(serde_json::to_value(&command).unwrap(), value);
    }
    assert!(serde_json::from_value::<NofapHostCommand>(json!({ "cmd": "run" })).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_loops_start_and_stop_with_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let app = NofapApp::start(
        NofapAppConfig {
            data_dir: Some(PathBuf::from(dir.path())),
            in_memory_db: true,
            scripted_platform: Some(Scenario::new(vec![])),
            ai: AiBackend::None,
            ..NofapAppConfig::default()
        },
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    // In memory: no file is written.
    assert!(!dir.path().join(SHIELD.db_file).exists());
    // Off by default: the loops run and touch nothing.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!app.service.enabled());
    app.shutdown();
    assert_eq!(count(&app, "nofap_state"), 0);
}
