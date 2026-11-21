//! The standalone product's composition root (design 8.2, 8.3, 11.2): a scripted session
//! with blocks, an AI check that acts and every command of the page leaves no row in a
//! productivity table; the host commands round-trip; the standalone identity is its own.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use ubiqx_ai::fake::FakeExplicitDetector;
use ubiqx_app::nofap_app::{
    HostAbout, HostSettings, HostSettingsPatch, NofapApp, NofapAppConfig, NofapHostCommand,
    GUARD_EVERY,
};
use ubiqx_app::{AiBackend, App, AppConfig};
use ubiqx_core::clock::FixedClock;
use ubiqx_core::nofap::{
    ApplyOutcome, Confidence, ExplicitVerdict, NofapCard, NofapEvent, NofapSink, Overall,
    PlannedChange, TierState,
};
use ubiqx_core::nofap_ipc::{
    ChangeOutcome, ConsentArgs, EnableOptions, ExportResult, NofapCommand, NofapView, SampleFrame,
    SitesView, SupportView, VisionScope,
};
use ubiqx_core::ports::EventSink;
use ubiqx_core::ports::{PermissionKind, PermissionState, PermissionStatus};
use ubiqx_core::product::{ProductId, SHIELD, UBIQX};
use ubiqx_core::EngineEvent;
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

/// Tables the standalone product may write besides the module's own: its settings row,
/// the update checker's state, and the cost ledger of the AI check. Every other table of
/// the schema, including one added later, must stay empty (`categories` keeps the rows its
/// migration seeds).
const STANDALONE_WRITES: [&str; 3] = ["settings", "kv", "ai_usage"];

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

/// Every table of the database and its row count.
fn table_rows(app: &NofapApp) -> BTreeMap<String, i64> {
    let names: Vec<String> = app
        .store
        .db()
        .with(|conn| {
            let mut statement = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            )?;
            let names = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(names)
        })
        .unwrap();
    names
        .into_iter()
        .map(|name| {
            let rows = count(app, &name);
            (name, rows)
        })
        .collect()
}

/// Reads a reply as the type its command documents (design 7.8), or says which it is not.
fn reply_as<T: DeserializeOwned>(cmd: &str, reply: &Value) -> Result<(), String> {
    serde_json::from_value::<T>(reply.clone())
        .map(|_| ())
        .map_err(|error| format!("the reply to {cmd} is not the documented type: {error}"))
}

/// Checks the reply of one request of `commands.json` against the type its command
/// documents.
fn check_reply(request: &Value, reply: &Value) -> Result<(), String> {
    let cmd = request["cmd"].as_str().unwrap_or_default();
    match cmd {
        "view" | "enable" | "consent_vision" | "confirm_pending" | "cancel_pending" => {
            reply_as::<NofapView>(cmd, reply)
        }
        "request" => reply_as::<ChangeOutcome>(cmd, reply),
        "system_preview" => reply_as::<Vec<PlannedChange>>(cmd, reply),
        "system_apply" | "system_remove" | "restore_network" => {
            reply_as::<ApplyOutcome>(cmd, reply)
        }
        "sites" => reply_as::<SitesView>(cmd, reply),
        "support" => reply_as::<SupportView>(cmd, reply),
        "card" => reply_as::<NofapCard>(cmd, reply),
        "sample_frame" => reply_as::<Option<SampleFrame>>(cmd, reply),
        "export" => reply_as::<ExportResult>(cmd, reply),
        "record" | "card_action" | "erase_history" | "reset" | "test_card" => reply
            .is_null()
            .then_some(())
            .ok_or_else(|| format!("the reply to {cmd} is not null: {reply}")),
        other => Err(format!("no documented reply for {other}")),
    }
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
    let seeded_categories = count(app, "categories");
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

    // Every request the page and the card window can send, through the app's own
    // handler: each answer has the type its command documents, and the only refusals are
    // the ones this state calls for.
    let mut refused = Vec::new();
    for request in commands_fixture() {
        let command: NofapCommand = serde_json::from_value(request.clone())
            .unwrap_or_else(|error| panic!("{request}: {error}"));
        match app.handle(command) {
            Ok(reply) => check_reply(&request, &reply).unwrap_or_else(|error| panic!("{error}")),
            Err(error) => refused.push(format!("{}: {error}", request["cmd"])),
        }
    }
    let refused_cmds: Vec<&str> = refused
        .iter()
        .map(|line| line.split(':').next().unwrap().trim_matches('"'))
        .collect();
    assert_eq!(
        refused_cmds,
        [
            // The module was switched on above; later changes go through requests.
            "enable",
            // The fixture consents for "IA do Ubi", which this product does not offer.
            "consent_vision",
            // `site-1` and `site-2` (remove_block, remove_allow) are nobody's ids.
            "request",
            "request",
            // So is `pending-1`.
            "confirm_pending",
            "cancel_pending",
            // Only while the module is off.
            "system_remove",
            "reset",
        ],
        "{refused:#?}"
    );
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
    // Nothing else either: every table outside the module's own and the few the product
    // keeps is empty, and the seeded categories are as the migration left them.
    for (table, rows) in table_rows(app) {
        if table == "categories" {
            assert_eq!(rows, seeded_categories, "categories after the session");
        } else if !NOFAP_TABLES.contains(&table.as_str())
            && !STANDALONE_WRITES.contains(&table.as_str())
        {
            assert_eq!(rows, 0, "{table} after the session");
        }
    }
    let other_usage = app
        .store
        .db()
        .with(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM ai_usage WHERE kind <> 'nofap_vision'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(other_usage, 0, "only the AI check's cost is recorded");
    // The module itself did write: its configuration (the fixture's `erase_history`
    // cleared the counters again).
    assert_eq!(count(app, "nofap_state"), 1);
}

struct NoEvents;

impl EventSink for NoEvents {
    fn emit(&self, _event: EngineEvent) {}
}

/// The zero-rows test above means something only if the standalone database is ubiqX's
/// schema: the same tables at the same version, from the same migration list.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_standalone_database_has_the_schema_of_ubiqx() {
    let s = session(AiBackend::None);
    let ubiqx_dir = tempfile::tempdir().unwrap();
    let ubiqx = App::start(
        AppConfig {
            data_dir: Some(ubiqx_dir.path().to_path_buf()),
            scripted_platform: Some(Scenario::new(vec![])),
            ai: AiBackend::None,
            ..AppConfig::default()
        },
        Arc::new(NoEvents),
    )
    .unwrap();
    let schema = |db: &ubiqx_storage::Db| -> (i64, Vec<String>) {
        db.with(|conn| {
            let version = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            let mut statement = conn.prepare(
                "SELECT type || ' ' || name || ': ' || IFNULL(sql, '') FROM sqlite_master \
                 WHERE name NOT LIKE 'sqlite_%' ORDER BY type, name",
            )?;
            let objects = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok((version, objects))
        })
        .unwrap()
    };
    let standalone = schema(s.app.store.db());
    assert!(s.dir.path().join(SHIELD.db_file).is_file());
    assert!(standalone.0 >= 3, "the module's migration ran");
    assert_eq!(standalone, schema(ubiqx.store.db()));
    for table in PRODUCTIVITY_TABLES.iter().chain(NOFAP_TABLES.iter()) {
        assert!(
            standalone
                .1
                .iter()
                .any(|o| o.starts_with(&format!("table {table}:"))),
            "{table} is in the schema"
        );
    }
    ubiqx.engine.shutdown();
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_managed_provider_cannot_be_consented_to() {
    let s = session(AiBackend::Fake);
    let app = &s.app;
    let managed = ConsentArgs {
        provider: AiProvider::Ubi,
        managed: true,
    };
    let enable = EnableOptions {
        vision_consent: Some(managed),
        ..options()
    };
    assert!(app.handle(NofapCommand::Enable(enable)).is_err());
    assert!(!app.service.enabled(), "a refused set-up changes nothing");

    let view: NofapView =
        serde_json::from_value(app.handle(NofapCommand::Enable(options())).unwrap()).unwrap();
    let consent = view.config.vision.consent.clone();
    assert_eq!(
        consent.as_ref().map(|c| c.provider),
        Some(AiProvider::OpenAi)
    );
    assert!(app.handle(NofapCommand::ConsentVision(managed)).is_err());
    let after: NofapView = serde_json::from_value(app.handle(NofapCommand::View).unwrap()).unwrap();
    assert_eq!(after.config.vision.consent, consent);
    // A vendor of this product is consented to as before.
    let anthropic = ConsentArgs {
        provider: AiProvider::Anthropic,
        managed: false,
    };
    let view: NofapView =
        serde_json::from_value(app.handle(NofapCommand::ConsentVision(anthropic)).unwrap())
            .unwrap();
    assert_eq!(
        view.config.vision.consent.map(|c| c.provider),
        Some(AiProvider::Anthropic)
    );
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
    // The module's clock: each new site below starts an episode of its own, which counts.
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap());
    let app = NofapApp::start(
        NofapAppConfig {
            data_dir: Some(PathBuf::from(dir.path())),
            in_memory_db: true,
            scripted_platform: Some(Scenario::new(vec![])),
            ai: AiBackend::None,
            clock: Some(Arc::new(clock.clone())),
            ..NofapAppConfig::default()
        },
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    // In memory: no file is written.
    assert!(!dir.path().join(SHIELD.db_file).exists());
    let scripted = app.scripted.clone().unwrap();
    // A listed site in front: what the guard counts when its loop runs a pass.
    let open = |host: &str| {
        clock.advance(ChronoDuration::hours(2));
        scripted.load(Scenario::new(vec![Step::browser(
            "Google Chrome",
            CHROME,
            "Receitas da semana",
            &format!("https://{host}/pagina"),
            1,
        )]))
    };
    let counted = || -> i64 {
        app.store
            .db()
            .with(|conn| {
                conn.query_row("SELECT IFNULL(SUM(n), 0) FROM nofap_day", [], |row| {
                    row.get(0)
                })
                .map_err(Into::into)
            })
            .unwrap()
    };
    let wait_for_more_than = |before: i64| {
        let deadline = std::time::Instant::now() + GUARD_EVERY * 5;
        async move {
            while counted() <= before {
                assert!(
                    std::time::Instant::now() < deadline,
                    "no guard pass acted within five intervals"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    };
    let sites = ubiqx_nofap::list::canary_block();

    // Off by default: the loop's passes act on nothing.
    open(sites[0]);
    tokio::time::sleep(GUARD_EVERY + Duration::from_millis(700)).await;
    assert_eq!(counted(), 0, "a module that is off acted");

    // On: the loop's passes close the listed site, and then the next one.
    app.service
        .enable(EnableOptions {
            vision: false,
            vision_consent: None,
            ..options()
        })
        .unwrap();
    wait_for_more_than(0).await;
    let before = counted();
    open(sites[1]);
    wait_for_more_than(before).await;

    // Stopped: the module is still on, and no pass acts on a third listed site.
    app.shutdown();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let stopped_at = counted();
    open(sites[2]);
    tokio::time::sleep(GUARD_EVERY * 2 + Duration::from_millis(500)).await;
    assert!(app.service.enabled());
    assert_eq!(counted(), stopped_at, "a guard pass acted after shutdown");
}
