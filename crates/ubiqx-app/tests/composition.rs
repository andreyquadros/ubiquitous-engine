//! ubiqX's composition root wires the `nofap` module (design 7.7): the store is its
//! repository, the shell's presenter and sink receive what it shows and says, and the
//! machine's system layer is never wired for a scripted platform or by default.

use std::sync::Arc;

use parking_lot::Mutex;
use ubiqx_app::{AiBackend, App, AppConfig};
use ubiqx_core::nofap::{
    NofapCard, NofapEvent, NofapPresenter, NofapSink, SystemCapabilities, TierState,
};
use ubiqx_core::nofap_ipc::{EnableOptions, NofapCommand, NofapView, VisionScope};
use ubiqx_core::ports::EventSink;
use ubiqx_core::product::ProductId;
use ubiqx_core::{CoreResult, EngineEvent};
use ubiqx_engine::deps::NO_SYSTEM_LAYER;
use ubiqx_platform::mock::Scenario;

struct NoEvents;

impl EventSink for NoEvents {
    fn emit(&self, _event: EngineEvent) {}
}

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<NofapEvent>>,
    shown: Mutex<Vec<NofapCard>>,
}

impl NofapSink for Recorder {
    fn emit(&self, event: NofapEvent) {
        self.events.lock().push(event);
    }
}

impl NofapPresenter for Recorder {
    fn show(&self, card: &NofapCard) -> CoreResult<bool> {
        self.shown.lock().push(card.clone());
        Ok(true)
    }
}

fn start(nofap_system: bool, recorder: Arc<Recorder>, dir: &tempfile::TempDir) -> App {
    App::start(
        AppConfig {
            in_memory_db: true,
            data_dir: Some(dir.path().to_path_buf()),
            scripted_platform: Some(Scenario::new(vec![])),
            ai: AiBackend::None,
            nofap_presenter: Some(recorder.clone()),
            nofap_sink: Some(recorder),
            nofap_system,
            ..AppConfig::default()
        },
        Arc::new(NoEvents),
    )
    .unwrap()
}

fn options() -> EnableOptions {
    EnableOptions {
        self_use: true,
        hosts: true,
        resolver: None,
        policies: false,
        vision: false,
        vision_consent: None,
        vision_scope: VisionScope {
            unknown_sites: true,
            media_apps: true,
            messaging_apps: false,
            private_windows: true,
        },
        delay_secs: 0,
        values: String::new(),
        discreet: true,
    }
}

fn nofap_rows(app: &App) -> i64 {
    app.store
        .db()
        .with(|conn| {
            conn.query_row("SELECT COUNT(*) FROM nofap_state", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(Into::into)
        })
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_module_stores_in_the_database_and_talks_to_the_shell() {
    let recorder = Arc::new(Recorder::default());
    // Asking for the system layer with a scripted platform changes nothing.
    let dir = tempfile::tempdir().unwrap();
    let app = start(true, recorder.clone(), &dir);
    let nofap = app.engine.nofap().clone();
    assert!(!nofap.enabled());
    assert_eq!(nofap_rows(&app), 0);

    let view: NofapView = serde_json::from_value(
        ubiqx_engine::nofap::ipc::handle(&nofap, NofapCommand::Enable(options())).unwrap(),
    )
    .unwrap();
    assert_eq!(view.product, ProductId::Ubiqx);
    assert_eq!(
        view.status.hosts,
        TierState::Unsupported {
            reason: NO_SYSTEM_LAYER.into()
        }
    );
    // The configuration is in the store, not in memory only.
    assert_eq!(nofap_rows(&app), 1);
    assert!(recorder
        .events
        .lock()
        .iter()
        .any(|event| matches!(event, NofapEvent::Status { .. })));

    // The preview card reaches the shell's window and its event channel.
    ubiqx_engine::nofap::ipc::handle(&nofap, NofapCommand::TestCard).unwrap();
    let shown = recorder.shown.lock().clone();
    assert_eq!(shown.len(), 1);
    assert!(recorder
        .events
        .lock()
        .iter()
        .any(|event| matches!(event, NofapEvent::Card { card } if card.id == shown[0].id)));
    app.engine.shutdown();
}

#[test]
fn the_system_layer_is_opt_in() {
    assert!(!AppConfig::default().nofap_system);
    let inert = ubiqx_app::nofap_system_layer(&ubiqx_core::product::UBIQX, false);
    assert_eq!(
        inert.capabilities(),
        SystemCapabilities::none(NO_SYSTEM_LAYER)
    );
}
