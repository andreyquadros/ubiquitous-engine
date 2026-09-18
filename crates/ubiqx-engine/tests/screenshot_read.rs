//! `screenshot_for_block`: the review screen's read of a block's stored screenshot.

use std::sync::Arc;

use chrono::{Duration, TimeZone, Utc};
use parking_lot::Mutex;
use ubiqx_core::clock::FixedClock;
use ubiqx_core::ports::*;
use ubiqx_core::*;
use ubiqx_engine::screenshots::screenshots_dir;
use ubiqx_engine::*;
use ubiqx_platform::mock::{Scenario, ScriptedPlatform, Step};
use ubiqx_platform::PlatformServices;
use ubiqx_storage::{Db, SqliteStore};

struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: EngineEvent) {}
}

#[derive(Default)]
struct MemorySecrets(Mutex<std::collections::HashMap<String, String>>);

impl SecretStore for MemorySecrets {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        Ok(self.0.lock().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.0.lock().insert(key.into(), value.into());
        Ok(())
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        self.0.lock().remove(key);
        Ok(())
    }
}

struct Fixture {
    state: Arc<EngineState>,
    store: Arc<SqliteStore>,
    _tmp: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    let (platform, _scripted): (PlatformServices, Arc<ScriptedPlatform>) =
        PlatformServices::scripted(Scenario::new(vec![Step::app("A", "a", "t", 1)]));
    let deps = EngineDeps {
        platform: PlatformPorts {
            activity: platform.activity.clone(),
            urls: platform.urls.clone(),
            idle: platform.idle.clone(),
            capturer: platform.capturer.clone(),
            permissions: platform.permissions.clone(),
            secrets: Arc::new(MemorySecrets::default()),
            notifier: platform.notifier.clone(),
        },
        repos: Repos::from_store(store.clone()),
        ai: AiPorts {
            remote: None,
            vision: None,
            report_writer: None,
            advisor: None,
            requires_api_key: false,
        },
        sink: Arc::new(NullSink),
        clock: Arc::new(FixedClock::new(
            Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap(),
        )),
        data_dir: tmp.path().to_path_buf(),
    };
    let state = EngineState::new(deps, Settings::default(), None);
    Fixture {
        state,
        store,
        _tmp: tmp,
    }
}

/// A closed block; `screenshot_id` set when given.
fn block(id: &str, screenshot_id: Option<&str>) -> ActivityBlock {
    let mut b = bare_block(id);
    b.screenshot_id = screenshot_id.map(String::from);
    b
}

fn bare_block(id: &str) -> ActivityBlock {
    let now = Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap();
    let mut b = ActivityBlock::new_manual(
        now - Duration::minutes(30),
        now - Duration::minutes(20),
        "cat-x".into(),
        None,
    );
    b.id = id.into();
    b.is_manual = false;
    b.category_id = None;
    b.app_id = "com.apple.dt.Xcode".into();
    b.app_name = "Xcode".into();
    b
}

/// Inserts the block, then the screenshot row, then links them, in the order the foreign
/// keys allow (the same order the tracker uses).
fn insert_linked(store: &SqliteStore, shot: &Screenshot) {
    let block_id = shot.block_id.clone().unwrap();
    BlockRepo::insert(store, &bare_block(&block_id)).unwrap();
    ScreenshotRepo::insert(store, shot).unwrap();
    BlockRepo::set_screenshot(store, &block_id, &shot.id).unwrap();
}

fn screenshot(id: &str, block_id: &str, path: &std::path::Path) -> Screenshot {
    Screenshot {
        id: id.into(),
        at: Utc.with_ymd_and_hms(2026, 9, 17, 14, 35, 0).unwrap(),
        path: path.to_string_lossy().to_string(),
        width: 640,
        height: 400,
        app_id: "com.apple.dt.Xcode".into(),
        block_id: Some(block_id.into()),
        sent_to_ai: false,
    }
}

/// The smallest valid JPEG stream: SOI, a 1x1 baseline frame and EOI. Only the bytes matter
/// here; the service never decodes the file.
const TINY_JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF,
    0xD9,
];

#[test]
fn block_without_screenshot_yields_none() {
    let f = fixture();
    BlockRepo::insert(f.store.as_ref(), &block("b1", None)).unwrap();
    assert_eq!(screenshot_for_block(&f.state, "b1").unwrap(), None);
}

#[test]
fn unknown_block_is_an_error() {
    let f = fixture();
    assert!(matches!(
        screenshot_for_block(&f.state, "nope"),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn dangling_screenshot_id_yields_none() {
    let f = fixture();
    BlockRepo::insert(f.store.as_ref(), &block("b1", Some("s-missing"))).unwrap();
    assert_eq!(screenshot_for_block(&f.state, "b1").unwrap(), None);
}

#[test]
fn row_without_file_yields_none() {
    let f = fixture();
    let dir = screenshots_dir(&f.state.deps.data_dir).join("2026-09-17");
    std::fs::create_dir_all(&dir).unwrap();
    // Row present, file already purged after classification.
    insert_linked(&f.store, &screenshot("s1", "b1", &dir.join("s1.jpg")));
    assert_eq!(screenshot_for_block(&f.state, "b1").unwrap(), None);
}

#[test]
fn stored_jpeg_comes_back_base64_encoded() {
    let f = fixture();
    let dir = screenshots_dir(&f.state.deps.data_dir).join("2026-09-17");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s1.jpg");
    std::fs::write(&path, TINY_JPEG).unwrap();
    insert_linked(&f.store, &screenshot("s1", "b1", &path));

    let data = screenshot_for_block(&f.state, "b1").unwrap().unwrap();
    assert_eq!(data.mime, "image/jpeg");
    assert_eq!(data.width, Some(640));
    assert_eq!(data.height, Some(400));
    let expected = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode(TINY_JPEG)
    };
    assert_eq!(data.data_base64, expected);
}

#[test]
fn path_outside_the_screenshots_dir_is_never_read() {
    let f = fixture();
    std::fs::create_dir_all(screenshots_dir(&f.state.deps.data_dir)).unwrap();
    // A row whose path escaped the screenshots dir (tampered database) is treated as absent.
    let outside = f.state.deps.data_dir.join("secret.jpg");
    std::fs::write(&outside, TINY_JPEG).unwrap();
    insert_linked(&f.store, &screenshot("s1", "b1", &outside));
    assert_eq!(screenshot_for_block(&f.state, "b1").unwrap(), None);
}
