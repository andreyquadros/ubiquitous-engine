//! The wire contract of the `nofap` command, without a shell: every request of
//! `apps/desktop/src/nofap/fixtures/commands.json` (generated from Rust by the golden test
//! of `ubiqx-core`, and type-checked by the frontend) deserialises into a `NofapCommand`,
//! is handled by `ubiqx_engine::nofap::ipc::handle`, and answers with a value that
//! deserialises into the reply type documented on its variant.
//!
//! The fixture's ids are symbolic (`pending-1`, `site-1`, `site-2`, `card-1`). Each such
//! request is replayed twice: as written, which must give the documented answer for an id
//! nobody knows, and with the id of something this test created, which must succeed.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use ubiqx_ai::fake::{FakeExplicitDetector, MemoryUsageRepo};
use ubiqx_core::clock::FixedClock;
use ubiqx_core::nofap::*;
use ubiqx_core::nofap_ipc::{
    ChangeOutcome, ExportResult, NofapCommand, NofapView, SampleFrame, SitesView, SupportView,
};
use ubiqx_core::ports::*;
use ubiqx_core::*;
use ubiqx_engine::nofap::{ipc, Background, DisplaySession, Tuning};
use ubiqx_engine::*;
use ubiqx_platform::mock::{MockEnforcer, MockSystemShield, ScriptedFrameSource};
use ubiqx_storage::{Db, SqliteStore};

const CHROME: &str = "com.google.Chrome";
/// Blocked by a keyword rule, so its card carries the correction button.
const KEYWORD: &str = "videosdeputaria.com.br";

fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap()
}

#[derive(Default)]
struct Foreground(Mutex<Option<(ForegroundWindow, String)>>);

impl ActivitySource for Foreground {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        Ok(self.0.lock().as_ref().map(|(window, _)| window.clone()))
    }
}

impl BrowserUrlResolver for Foreground {
    fn supports(&self, app_id: &str) -> bool {
        app_id == CHROME
    }

    fn resolve(&self, _window: &ForegroundWindow) -> CoreResult<Option<String>> {
        Ok(self.0.lock().as_ref().map(|(_, url)| url.clone()))
    }
}

impl PermissionChecker for Foreground {
    fn status(&self) -> PermissionStatus {
        PermissionStatus {
            screen_recording: PermissionState::Granted,
            automation: PermissionState::Granted,
            accessibility: PermissionState::Granted,
        }
    }

    fn request(&self, _kind: PermissionKind) -> CoreResult<()> {
        Ok(())
    }
}

struct QuietNotifier;

impl Notifier for QuietNotifier {
    fn notify(&self, _title: &str, _body: &str) -> CoreResult<()> {
        Ok(())
    }
}

struct Shown(Mutex<Vec<NofapCard>>);

impl NofapPresenter for Shown {
    fn show(&self, card: &NofapCard) -> CoreResult<bool> {
        self.0.lock().push(card.clone());
        Ok(true)
    }
}

struct Env;

impl NofapEnv for Env {
    fn snapshot(&self) -> NofapEnvSnapshot {
        NofapEnvSnapshot {
            language: UiLanguage::PtBr,
            provider: AiProvider::Anthropic,
            vision_model: "a-vision-model".into(),
            ai: AiReadiness::Ready,
            excluded_apps: Vec::new(),
            excluded_domains: Vec::new(),
        }
    }
}

struct Rig {
    service: Arc<NofapService>,
    foreground: Arc<Foreground>,
    shown: Arc<Shown>,
    clock: FixedClock,
    tmp: tempfile::TempDir,
}

fn rig() -> Rig {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    let foreground = Arc::new(Foreground::default());
    let shown = Arc::new(Shown(Mutex::new(Vec::new())));
    let clock = FixedClock::new(fixed_now());
    let shield = Arc::new(MockSystemShield::new(Os::Macos));
    shield.set_capabilities(SystemCapabilities {
        os: Some(Os::Macos),
        hosts: TierCapability::Available,
        resolver: TierCapability::Available,
        policies: TierCapability::Available,
        unverified: Tier::ALL.to_vec(),
    });
    let ports = NofapPorts {
        system: shield,
        frames: Arc::new(ScriptedFrameSource::new()),
        detector: Some(Arc::new(FakeExplicitDetector::new())),
        presenter: shown.clone(),
        sink: Arc::new(NullNofapSink),
        repo: store,
        shared_dir: None,
        tuning: Tuning {
            confirm_after: Duration::from_millis(5),
            check_timeout: Duration::from_secs(5),
            background: Background::Inline,
            display: DisplaySession::Full,
        },
    };
    let host = NofapHost {
        env: Arc::new(Env),
        activity: foreground.clone(),
        urls: foreground.clone(),
        enforcer: Arc::new(MockEnforcer::default()),
        permissions: foreground.clone(),
        notifier: Arc::new(QuietNotifier),
        usage: Arc::new(MemoryUsageRepo::new()),
        clock: Arc::new(clock.clone()),
        identity: &ubiqx_core::product::UBIQX,
        export_dir: tmp.path().join("exports"),
    };
    Rig {
        service: NofapService::new(NofapDeps::assemble(ports, host)),
        foreground,
        shown,
        clock,
        tmp,
    }
}

impl Rig {
    fn send(&self, command: &Value) -> CoreResult<Value> {
        let command: NofapCommand = serde_json::from_value(command.clone())
            .unwrap_or_else(|error| panic!("{command} is not a NofapCommand: {error}"));
        ipc::handle(&self.service, command)
    }

    /// Sends a request that must succeed and reads its reply as `T`.
    fn ask<T: DeserializeOwned>(&self, command: &Value) -> T {
        let reply = self
            .send(command)
            .unwrap_or_else(|error| panic!("{command} failed: {error}"));
        serde_json::from_value(reply.clone()).unwrap_or_else(|error| {
            panic!("the reply to {command} is not the documented type: {error}\n{reply}")
        })
    }

    fn view(&self) -> NofapView {
        self.ask(&serde_json::json!({ "cmd": "view" }))
    }

    fn sites(&self) -> SitesView {
        self.ask(&serde_json::json!({ "cmd": "sites" }))
    }

    /// Lets every waiting request become ready: the delay by the clock, and the running
    /// time the verify pass credits.
    fn wait_out_the_delay(&self) {
        self.clock.advance(ChronoDuration::seconds(3_600));
        for _ in 0..6 {
            self.service
                .verify_pass_after(Duration::from_secs(150))
                .unwrap();
        }
    }

    /// A keyword-blocked page held by the guard, and the card it showed.
    fn blocked_card(&self) -> NofapCard {
        let window = ForegroundWindow {
            app_name: "Google Chrome".into(),
            app_id: CHROME.into(),
            window_title: "Receitas da semana".into(),
            pid: Some(1),
            window_id: Some(7),
            bounds: None,
        };
        *self.foreground.0.lock() = Some((window, format!("https://{KEYWORD}/")));
        // Past the ten minutes "sair da tela" keeps the card window busy.
        self.clock.advance(ChronoDuration::seconds(700));
        self.service.guard_pass().unwrap();
        assert!(self.service.guard_pass().unwrap().hit);
        let card = self.shown.0.lock().last().cloned().expect("a card");
        assert_eq!(card.kind, CardKind::Blocked);
        card
    }
}

fn fixture() -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/src/nofap/fixtures/commands.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).expect("commands.json is a JSON array")
}

fn with_id(command: &Value, id: &str) -> Value {
    let mut command = command.clone();
    command["args"]["id"] = Value::String(id.to_owned());
    command
}

fn pending_of(view: &NofapView, kind: ChangeKind) -> String {
    view.config
        .pending
        .iter()
        .find(|waiting| waiting.change.kind() == kind)
        .unwrap_or_else(|| panic!("no waiting {kind:?} request"))
        .id
        .clone()
}

#[test]
fn nofap_ipc_every_fixture_request_is_handled_and_answers_its_documented_type() {
    let commands = fixture();
    let rig = rig();
    // Two sites of the user's own, added while the module is off (nothing waits then), so
    // that the fixture's "remove" requests have something real to name.
    for (kind, host) in [
        ("add_block", "exemplo-barrado.example"),
        ("add_allow", "exemplo-permitido.example"),
    ] {
        let added: ChangeOutcome = rig.ask(&serde_json::json!({
            "cmd": "request",
            "args": { "kind": kind, "host": host }
        }));
        assert!(matches!(added, ChangeOutcome::Applied(_)));
    }
    let sites = rig.sites();
    let (block_id, allow_id) = (sites.blocked[0].id.clone(), sites.allowed[0].id.clone());

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for command in &commands {
        let cmd = command["cmd"]
            .as_str()
            .expect("every request has a cmd")
            .to_owned();
        let kind = command["args"]["kind"].as_str().unwrap_or_default();
        seen.insert(cmd.clone());
        match cmd.as_str() {
            "view" | "enable" | "consent_vision" => {
                let view: NofapView = rig.ask(command);
                assert_eq!(view.display_name, DISPLAY_NAME);
                if cmd == "enable" {
                    assert!(view.config.enabled);
                    assert_eq!(view.status.hosts, TierState::Applied);
                    assert_eq!(view.status.resolver, TierState::Applied);
                }
                if cmd == "consent_vision" {
                    let consent = view.config.vision.consent.expect("recorded");
                    assert_eq!((consent.provider, consent.managed), (AiProvider::Ubi, true));
                }
            }
            "request" if kind == "remove_block" || kind == "remove_allow" => {
                // An id nobody knows is "not found"; a real one is handled.
                assert!(
                    matches!(rig.send(command), Err(CoreError::NotFound(_))),
                    "{command}"
                );
                let id = if kind == "remove_block" {
                    &block_id
                } else {
                    &allow_id
                };
                let outcome: ChangeOutcome = rig.ask(&with_id(command, id));
                // Taking a block away loosens and waits; taking an allow away tightens.
                assert_eq!(
                    matches!(outcome, ChangeOutcome::Pending(_)),
                    kind == "remove_block",
                    "{command}"
                );
            }
            "request" => {
                let outcome: ChangeOutcome = rig.ask(command);
                let waits = matches!(
                    kind,
                    "disable" | "add_allow" | "forget_learned" | "set_hosts_user_sites"
                );
                assert_eq!(
                    matches!(outcome, ChangeOutcome::Pending(_)),
                    waits,
                    "{command}"
                );
            }
            "confirm_pending" => {
                assert!(matches!(rig.send(command), Err(CoreError::NotFound(_))));
                let id = pending_of(&rig.view(), ChangeKind::AddAllow);
                // Still waiting: refused. Once the delay and the running time are over:
                // confirmed, and the site is on the allow list.
                assert!(matches!(
                    rig.send(&with_id(command, &id)),
                    Err(CoreError::Invalid(_))
                ));
                rig.wait_out_the_delay();
                let view: NofapView = rig.ask(&with_id(command, &id));
                assert!(!view.config.pending.iter().any(|waiting| waiting.id == id));
                assert!(rig
                    .sites()
                    .allowed
                    .iter()
                    .any(|site| site.host == "example.org"));
            }
            "cancel_pending" => {
                assert!(matches!(rig.send(command), Err(CoreError::NotFound(_))));
                let id = pending_of(&rig.view(), ChangeKind::Disable);
                let view: NofapView = rig.ask(&with_id(command, &id));
                assert!(view.config.enabled);
                assert!(!view.config.pending.iter().any(|waiting| waiting.id == id));
            }
            "system_preview" => {
                let lines: Vec<PlannedChange> = rig.ask(command);
                for line in &lines {
                    assert!(
                        preview::LINES.iter().any(|(key, tier, placeholder)| {
                            *key == line.text_key
                                && *tier == line.tier
                                && placeholder.is_some() == line.detail.is_some()
                        }),
                        "{line:?} is not a line the frontend knows"
                    );
                }
            }
            "system_apply" | "restore_network" => {
                let outcome: ApplyOutcome = rig.ask(command);
                assert_eq!(outcome, ApplyOutcome::Applied);
            }
            "system_remove" => {
                // Only while the module is off.
                assert!(matches!(rig.send(command), Err(CoreError::Invalid(_))));
            }
            "sites" => {
                let sites: SitesView = rig.ask(command);
                assert!(sites.blocked.iter().any(|site| site.host == "example.com"));
            }
            "support" => {
                let support: SupportView = rig.ask(command);
                assert!(!support.resources.is_empty());
            }
            "record" | "erase_history" | "test_card" => {
                assert_eq!(rig.send(command).unwrap(), Value::Null, "{command}");
            }
            "card" => {
                // An id nobody knows: the neutral discreet card, never an error.
                let neutral: NofapCard = rig.ask(command);
                assert_eq!((neutral.kind, neutral.discreet), (CardKind::Blocked, true));
                assert_eq!(neutral.id, command["args"]["id"].as_str().unwrap());
                let real = rig.blocked_card();
                let card: NofapCard = rig.ask(&with_id(command, &real.id));
                assert_eq!(card, real);
            }
            "card_action" => {
                assert_eq!(
                    rig.send(command).unwrap(),
                    Value::Null,
                    "an unknown card does nothing"
                );
                let before = rig.view().config.pending.len();
                let real = rig
                    .shown
                    .0
                    .lock()
                    .last()
                    .cloned()
                    .expect("the card of `card`");
                assert_eq!(rig.send(&with_id(command, &real.id)).unwrap(), Value::Null);
                // The consent now names another provider than the one selected, so the AI
                // check is not there to keep looking: the correction is a delayed request.
                let after = rig.view().config.pending;
                assert_eq!(after.len(), before + 1);
                assert!(after.iter().any(|waiting| {
                    waiting.change
                        == NofapChange::AddAllow {
                            host: KEYWORD.into(),
                        }
                }));
            }
            "sample_frame" => {
                let frame: Option<SampleFrame> = rig.ask(command);
                assert_eq!(frame, None, "no candidate window was in front");
            }
            "reset" => {
                // Only while the module is off.
                assert!(matches!(rig.send(command), Err(CoreError::Invalid(_))));
            }
            "export" => {
                let export: ExportResult = rig.ask(command);
                assert!(PathBuf::from(&export.path).starts_with(rig.tmp.path()));
                assert!(std::fs::metadata(&export.path).is_ok());
            }
            other => panic!("commands.json has a request this test does not know: {other}"),
        }
    }
    let expected: BTreeSet<String> = NofapCommand::NAMES
        .iter()
        .map(|name| name.to_string())
        .collect();
    assert_eq!(seen, expected, "the fixture must hold every cmd");

    // The two commands that need the module off, once it is: turn it off (a new request,
    // its delay, its confirmation), then remove from the system and wipe.
    let off: ChangeOutcome =
        rig.ask(&serde_json::json!({ "cmd": "request", "args": { "kind": "disable" } }));
    let ChangeOutcome::Pending(waiting) = off else {
        panic!("turning it off waits");
    };
    rig.wait_out_the_delay();
    let view: NofapView =
        rig.ask(&serde_json::json!({ "cmd": "confirm_pending", "args": { "id": waiting.id } }));
    assert!(!view.config.enabled);
    let removed: ApplyOutcome = rig.ask(&serde_json::json!({ "cmd": "system_remove" }));
    assert_eq!(removed, ApplyOutcome::Applied);
    assert_eq!(
        rig.send(&serde_json::json!({ "cmd": "reset" })).unwrap(),
        Value::Null
    );
    let view = rig.view();
    assert_eq!(view.status, NofapStatus::off());
    assert!(view.config.pending.is_empty() && view.config.values.is_empty());
    assert!(rig.sites().blocked.is_empty());
}

#[test]
fn nofap_ipc_a_request_that_is_not_a_command_does_not_deserialise() {
    for bad in [
        serde_json::json!({ "cmd": "run_this" }),
        serde_json::json!({ "cmd": "request", "args": { "kind": "set_hosts" } }),
        serde_json::json!({ "cmd": "card_action", "args": { "id": "x", "action": "quit_everything" } }),
        serde_json::json!({ "args": {} }),
    ] {
        assert!(
            serde_json::from_value::<NofapCommand>(bad.clone()).is_err(),
            "{bad}"
        );
    }
}
