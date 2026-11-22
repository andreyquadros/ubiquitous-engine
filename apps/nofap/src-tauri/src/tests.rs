//! The standalone product end to end, without its window (a test module of the binary, so
//! it compiles wherever the shell does): `NofapApp` with the scripted
//! platform and a fake AI (design 8.2, 1.6, 11.2). Nothing here touches the machine: the
//! system layer is the inert one, the database and the shared lock live in a temporary
//! directory of the test.
//!
//! - An hour of use writes nothing of a tracker: every table of `shield.db` is listed with
//!   its rows, and no visited address or window title is stored anywhere in the files.
//! - ubiqX and the standalone app on one machine settle on one primary and never both act on
//!   the same page (the primary lock of design 1.6), whichever starts first, in a tie, and
//!   when the primary quits.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use serde_json::{json, Value};
use ubiqx_app::nofap_app::{NofapApp, NofapAppConfig, NofapHostCommand, StandaloneEnv};
use ubiqx_app::ubiqx_ai::fake::FakeExplicitDetector;
use ubiqx_app::ubiqx_engine::nofap::{Background, DisplaySession, Tuning};
use ubiqx_app::ubiqx_engine::{NofapDeps, NofapHost, NofapService};
use ubiqx_app::ubiqx_platform::mock::{MockEnforcer, Scenario, ScriptedFrameSource, Step};
use ubiqx_app::ubiqx_platform::PlatformServices;
use ubiqx_app::ubiqx_storage::{Db, SqliteStore};
use ubiqx_app::{nofap_ports, nofap_system_layer, AiBackend, NofapWiring};
use ubiqx_core::clock::FixedClock;
use ubiqx_core::nofap::{Confidence, ExplicitVerdict, NofapEvent, NofapSink, Overall};
use ubiqx_core::nofap_ipc::{ConsentArgs, EnableOptions, NofapCommand, VisionScope};
use ubiqx_core::product::{ProductIdentity, SHIELD, UBIQX};
use ubiqx_core::{AiProvider, ProductId};

const CHROME: &str = "com.google.Chrome";
const VLC: &str = "org.videolan.vlc";

/// One guard pass every two seconds, as the shell's loop runs it (`GUARD_EVERY`).
const TICK_SECS: i64 = 2;

/// A directory of the test's own under the system's temporary directory, removed at the end.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let dir = std::env::temp_dir().join(format!(
            "ubiqx-shield-test-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct NoEvents;

impl NofapSink for NoEvents {
    fn emit(&self, _event: NofapEvent) {}
}

fn listed() -> &'static str {
    ubiqx_app::ubiqx_nofap::list::canary_block()[0]
}

fn tuning() -> Tuning {
    Tuning {
        confirm_after: Duration::from_millis(5),
        check_timeout: Duration::from_secs(5),
        background: Background::Inline,
        display: DisplaySession::Full,
    }
}

fn options(vision: bool) -> EnableOptions {
    EnableOptions {
        self_use: true,
        hosts: true,
        resolver: None,
        policies: false,
        vision,
        vision_consent: vision.then_some(ConsentArgs {
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

/// Every table of the database with every row, each row as the text of its columns.
fn dump(app: &NofapApp) -> BTreeMap<String, Vec<String>> {
    app.store
        .db()
        .with(|conn| {
            let names: Vec<String> = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type = 'table' \
                     AND name NOT LIKE 'sqlite_%' ORDER BY name",
                )?
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<_, _>>()?;
            let mut tables = BTreeMap::new();
            for name in names {
                let columns: Vec<String> = conn
                    .prepare(&format!("SELECT name FROM pragma_table_info('{name}')"))?
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<_, _>>()?;
                // Every column as text (a blob as its bytes), joined into one line per row.
                let line = columns
                    .iter()
                    .map(|c| format!("IFNULL(CAST(\"{c}\" AS TEXT), 'NULL')"))
                    .collect::<Vec<_>>()
                    .join(" || ' | ' || ");
                let rows: Vec<String> = conn
                    .prepare(&format!("SELECT CAST(({line}) AS BLOB) FROM \"{name}\""))?
                    .query_map([], |row| row.get::<_, Vec<u8>>(0))?
                    .map(|bytes| bytes.map(|b| String::from_utf8_lossy(&b).into_owned()))
                    .collect::<Result<_, _>>()?;
                tables.insert(name, rows);
            }
            Ok(tables)
        })
        .unwrap()
}

/// The bytes of every file under `dir`, with their paths.
fn files_under(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut queue = vec![dir.to_path_buf()];
    while let Some(next) = queue.pop() {
        for entry in std::fs::read_dir(&next).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                queue.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                out.push((path, bytes));
            }
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// The productivity tables of ubiqX's schema (design 8.2).
const TRACKER_TABLES: [&str; 9] = [
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

/// What the standalone process may write: the module's tables, its settings row, the update
/// checker's state and the cost of the AI checks. `categories` keeps what its migration seeds.
const MAY_WRITE: [&str; 6] = [
    "nofap_state",
    "nofap_sites",
    "nofap_day",
    "settings",
    "kv",
    "ai_usage",
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_hour_of_use_records_nothing_of_what_was_on_screen() {
    let dir = TempDir::new("hour");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap());
    // The AI check finds something once, then nothing.
    let detector = Arc::new(FakeExplicitDetector::always(FakeExplicitDetector::verdict(
        ExplicitVerdict::None,
        Confidence::High,
    )));
    detector.push(Ok(FakeExplicitDetector::verdict(
        ExplicitVerdict::Explicit,
        Confidence::High,
    )));
    let app = NofapApp::start(
        NofapAppConfig {
            data_dir: Some(dir.path().to_path_buf()),
            scripted_platform: Some(Scenario::new(vec![])),
            ai: AiBackend::Fake,
            loops: false,
            detector: Some(detector.clone()),
            frames: Some(Arc::new(ScriptedFrameSource::new())),
            clock: Some(Arc::new(clock.clone())),
            tuning: Some(tuning()),
            ..NofapAppConfig::default()
        },
        Arc::new(NoEvents),
    )
    .unwrap();

    // A fresh start: the schema of ubiqX, empty but for the seeded categories.
    let start = dump(&app);
    for table in TRACKER_TABLES.iter().chain(MAY_WRITE.iter()) {
        assert!(start.contains_key(*table), "{table} is in the schema");
    }
    for (table, rows) in &start {
        if table != "categories" {
            assert!(rows.is_empty(), "{table} at start: {rows:?}");
        }
    }

    let host = |command: Value| {
        let command: NofapHostCommand = serde_json::from_value(command).unwrap();
        app.handle_host(command)
    };
    host(json!({ "cmd": "settings_set", "args": { "onboarding_done": true, "ai_provider": "openai", "ui_language": "en" } }))
        .await
        .unwrap();
    app.service.enable(options(true)).unwrap();

    // What was on the screen during the hour: none of it may be stored.
    let ordinary = "https://docs.example.org/guia";
    let unknown = "https://galeria-fotos.example.net/album-ana";
    let listed_url = format!("https://{}/pagina", listed());
    let steps = [
        Step::browser(
            "Google Chrome",
            CHROME,
            "Planilha de custos da Ana",
            ordinary,
            1,
        ),
        Step::browser(
            "Google Chrome",
            CHROME,
            "Receitas da semana",
            &listed_url,
            1,
        ),
        Step::app("VLC", VLC, "ferias-na-praia.mp4", 1),
        Step::browser("Google Chrome", CHROME, "Album da Ana", unknown, 1),
    ];
    let private = [
        "docs.example.org",
        "galeria-fotos.example.net",
        listed(),
        "Planilha de custos",
        "Receitas da semana",
        "ferias-na-praia",
        "Album da Ana",
    ];

    let (mut hits, mut checks) = (0, 0);
    let ticks = 60 * 60 / TICK_SECS;
    for tick in 0..ticks {
        // A new window in front every five minutes.
        if tick % (5 * 60 / TICK_SECS) == 0 {
            let step = steps[(tick / (5 * 60 / TICK_SECS)) as usize % steps.len()].clone();
            app.scripted
                .as_ref()
                .unwrap()
                .load(Scenario::new(vec![step]));
        }
        clock.advance(ChronoDuration::seconds(TICK_SECS));
        let outcome = app.service.guard_pass().unwrap();
        hits += usize::from(outcome.hit);
        if let Some(job) = outcome.job {
            checks += 1;
            let _ = app.service.vision_check(job).await.unwrap();
        }
        // The page reads its view now and then; the verify pass runs every two minutes.
        if tick % 30 == 0 {
            app.handle(NofapCommand::View).unwrap();
        }
        if tick % 60 == 0 {
            app.service.verify_pass().unwrap();
        }
    }
    assert!(hits > 0, "the listed site was closed");
    // A job can end without a call (the caps of design 4.3); every call came from a job.
    assert!(detector.calls() > 0, "the AI check looked at something");
    assert!(detector.calls() <= checks);

    let after = dump(&app);
    // The record of this run (`cargo test -- --nocapture` prints it).
    for (table, rows) in &after {
        println!("{table}: {} rows", rows.len());
        for row in rows {
            println!("  {row}");
        }
    }
    for table in TRACKER_TABLES {
        assert!(after[table].is_empty(), "{table}: {:?}", after[table]);
    }
    for (table, rows) in &after {
        if table == "categories" {
            assert_eq!(rows, &start["categories"], "categories changed");
        } else if !MAY_WRITE.contains(&table.as_str()) {
            assert!(rows.is_empty(), "{table}: {rows:?}");
        }
    }
    assert_eq!(after["settings"].len(), 1, "one settings row");
    for row in &after["ai_usage"] {
        assert!(
            row.contains("nofap_vision"),
            "only the AI check costs: {row}"
        );
    }
    assert!(!after["nofap_day"].is_empty(), "the module counted its day");

    // No address and no title in any row, nor anywhere in the files of the data directory
    // (the database, its journal, anything else).
    for (table, rows) in &after {
        for row in rows {
            for secret in private {
                assert!(!row.contains(secret), "{table} stores {secret:?}: {row}");
            }
        }
    }
    let files = files_under(dir.path());
    assert!(
        files.iter().any(|(path, _)| path.ends_with(SHIELD.db_file)),
        "the database is in the data directory"
    );
    for (path, bytes) in &files {
        for secret in private {
            assert!(
                !contains(bytes, secret),
                "{} holds {secret:?}",
                path.display()
            );
        }
    }
}

/// One product's module on a machine both share: its own database and scripted platform,
/// the lock directory of the test, the inert system layer.
struct Product {
    service: Arc<NofapService>,
    enforcer: Arc<MockEnforcer>,
}

fn product(identity: &'static ProductIdentity, shared: &Path, clock: &FixedClock) -> Product {
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    let page = format!("https://{}/pagina", listed());
    let (mut platform, _scripted) = PlatformServices::scripted(Scenario::new(vec![Step::browser(
        "Google Chrome",
        CHROME,
        "Receitas",
        &page,
        1,
    )]));
    let enforcer = Arc::new(MockEnforcer::default());
    platform.enforcer = enforcer.clone();
    let mut ports = nofap_ports(
        &platform,
        store.clone(),
        NofapWiring {
            system: nofap_system_layer(identity, false),
            detector: None,
            presenter: None,
            sink: None,
            shared_dir: Some(shared.to_path_buf()),
        },
    );
    ports.tuning = tuning();
    let env = Arc::new(StandaloneEnv::new(
        store.clone(),
        platform.secrets.clone(),
        false,
    ));
    let service = NofapService::new(NofapDeps::assemble(
        ports,
        NofapHost {
            env,
            activity: platform.activity.clone(),
            urls: platform.urls.clone(),
            enforcer: platform.enforcer.clone(),
            permissions: platform.permissions.clone(),
            notifier: platform.notifier.clone(),
            usage: store.clone(),
            clock: Arc::new(clock.clone()),
            identity,
            export_dir: shared.join("exports"),
        },
    ));
    service.enable(options(false)).unwrap();
    Product { service, enforcer }
}

impl Product {
    /// One guard pass; whether this product acted on the page.
    fn pass(&self) -> bool {
        let before = self.enforcer.calls().len();
        self.service.guard_pass().unwrap();
        self.enforcer.calls().len() > before
    }

    fn overall(&self) -> Overall {
        self.service.view().unwrap().status.overall
    }
}

/// Both on the same blocked page for `ticks` passes; how many times each acted.
fn run(clock: &FixedClock, ubiqx: &Product, shield: &Product, ticks: i64) -> (usize, usize) {
    let (mut a, mut b) = (0, 0);
    for _ in 0..ticks {
        clock.advance(ChronoDuration::seconds(TICK_SECS));
        let acted = (ubiqx.pass(), shield.pass());
        assert!(
            !(acted.0 && acted.1),
            "both products acted on the same pass"
        );
        a += usize::from(acted.0);
        b += usize::from(acted.1);
    }
    (a, b)
}

fn lock_owner(shared: &Path) -> Option<ProductId> {
    let text =
        std::fs::read_to_string(shared.join(ubiqx_app::ubiqx_nofap::primary::LOCK_FILE)).ok()?;
    ubiqx_app::ubiqx_nofap::primary::parse_lock(&text).map(|lock| lock.product)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_two_products_settle_on_one_primary_and_never_both_act() {
    // Distinct identities first: nothing but the lock directory is shared.
    assert_ne!(SHIELD.bundle_id, UBIQX.bundle_id);
    assert_ne!(SHIELD.db_file, UBIQX.db_file);
    assert_ne!(SHIELD.keyring_service, UBIQX.keyring_service);
    assert_ne!(SHIELD.feed_url, UBIQX.feed_url);

    // ubiqX was on first: it stays the primary and the standalone app stands back.
    let shared = TempDir::new("lock-first");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap());
    let ubiqx = product(&UBIQX, shared.path(), &clock);
    clock.advance(ChronoDuration::seconds(TICK_SECS));
    ubiqx.pass();
    let shield = product(&SHIELD, shared.path(), &clock);
    let (a, b) = run(&clock, &ubiqx, &shield, 120);
    assert!(a > 0, "the primary acts");
    assert_eq!(b, 0, "the secondary never acts");
    assert_eq!(shield.overall(), Overall::Secondary);
    assert_ne!(ubiqx.overall(), Overall::Secondary);
    assert_eq!(lock_owner(shared.path()), Some(ProductId::Ubiqx));

    // The primary quits: its lock goes at once, and the other takes over at its next
    // heartbeat (thirty seconds at most).
    ubiqx.service.on_quit();
    assert_eq!(lock_owner(shared.path()), None);
    let mut took_over = None;
    for tick in 0..(30 / TICK_SECS + 1) {
        clock.advance(ChronoDuration::seconds(TICK_SECS));
        if shield.pass() {
            took_over = Some(tick);
            break;
        }
    }
    assert!(took_over.is_some(), "the standalone app took over");
    assert_ne!(shield.overall(), Overall::Secondary);
    assert_eq!(lock_owner(shared.path()), Some(ProductId::Shield));

    // A tie: both took the role at once (each read the lock before the other wrote). At
    // the next heartbeat the standalone app keeps it and ubiqX stands back, for good.
    let shared = TempDir::new("lock-tie");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap());
    let ubiqx = product(&UBIQX, shared.path(), &clock);
    let shield = product(&SHIELD, shared.path(), &clock);
    clock.advance(ChronoDuration::seconds(TICK_SECS));
    ubiqx.pass();
    std::fs::remove_file(
        shared
            .path()
            .join(ubiqx_app::ubiqx_nofap::primary::LOCK_FILE),
    )
    .unwrap();
    shield.pass();
    // Settled within one heartbeat; from then on only the standalone app acts.
    let mut settle = 0;
    while ubiqx.overall() != Overall::Secondary {
        clock.advance(ChronoDuration::seconds(TICK_SECS));
        ubiqx.pass();
        shield.pass();
        settle += 1;
        assert!(settle * TICK_SECS <= 32, "no settlement after a heartbeat");
    }
    let (a, b) = run(&clock, &ubiqx, &shield, 120);
    assert_eq!(a, 0, "ubiqX stands back after the tie");
    assert!(b > 0, "the standalone app acts");
    assert_eq!(lock_owner(shared.path()), Some(ProductId::Shield));

    // The standalone app quits: ubiqX takes the role back.
    shield.service.on_quit();
    let mut took_back = false;
    for _ in 0..(30 / TICK_SECS + 1) {
        clock.advance(ChronoDuration::seconds(TICK_SECS));
        if ubiqx.pass() {
            took_back = true;
            break;
        }
    }
    assert!(took_back, "ubiqX took the role back");
}
