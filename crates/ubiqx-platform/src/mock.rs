//! A scripted platform: replays a scenario of foreground windows so the whole engine can run
//! on Linux/CI and in tests without any OS integration.

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use parking_lot::Mutex;
use ubiqx_core::ports::*;
use ubiqx_core::{CoreError, CoreResult, ForegroundWindow, UpdateFeed};

/// One step of a scenario: what is in the foreground for `samples` consecutive samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// `None` = nothing in the foreground (login window / screensaver).
    pub window: Option<ForegroundWindow>,
    pub url: Option<String>,
    pub idle_secs: f64,
    pub samples: u32,
}

impl Step {
    pub fn app(app_name: &str, app_id: &str, title: &str, samples: u32) -> Self {
        Self {
            window: Some(ForegroundWindow {
                app_name: app_name.into(),
                app_id: app_id.into(),
                window_title: title.into(),
                pid: Some(4242),
                window_id: Some(77),
                bounds: Some((0, 0, 1440, 900)),
            }),
            url: None,
            idle_secs: 0.0,
            samples,
        }
    }

    pub fn browser(app_name: &str, app_id: &str, title: &str, url: &str, samples: u32) -> Self {
        let mut s = Self::app(app_name, app_id, title, samples);
        s.url = Some(url.into());
        s
    }

    pub fn idle(secs: f64, samples: u32) -> Self {
        Self {
            window: Some(ForegroundWindow {
                app_name: "Finder".into(),
                app_id: "com.apple.finder".into(),
                window_title: String::new(),
                pid: Some(1),
                window_id: None,
                bounds: None,
            }),
            url: None,
            idle_secs: secs,
            samples,
        }
    }
}

/// A list of steps replayed in order. When the scenario ends it either loops or keeps
/// returning the last step.
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub steps: Vec<Step>,
    pub repeat: bool,
}

impl Scenario {
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            repeat: false,
        }
    }

    pub fn looping(mut self) -> Self {
        self.repeat = true;
        self
    }

    /// A plausible working day for the product's target user, with 5-second samples:
    /// e-mail, SEI (IFRO), coding for the incubator, WhatsApp, a smart-cities document,
    /// a distraction and an idle stretch.
    pub fn demo_day() -> Self {
        Self::demo_day_with(12)
    }

    /// The demo day with a custom number of samples per scenario minute (12 = real time at
    /// 5-second samples; 1 = one sample per "minute", for fast simulations).
    pub fn demo_day_with(samples_per_minute: u32) -> Self {
        let m = |mins: u32| mins * samples_per_minute.max(1);
        Self::new(vec![
            Step::browser(
                "Google Chrome",
                "com.google.Chrome",
                "Caixa de entrada (3) - Gmail",
                "https://mail.google.com/mail/u/0/#inbox",
                m(12),
            ),
            Step::browser(
                "Google Chrome",
                "com.google.Chrome",
                "SEI - Processo 23243.001234/2026-11",
                "https://sei.ifro.edu.br/sei/controlador.php?acao=procedimento_trabalhar&id=1",
                m(35),
            ),
            Step::app(
                "Visual Studio Code",
                "com.microsoft.VSCode",
                "api-incubadora — main.rs",
                m(70),
            ),
            Step::app("WhatsApp", "net.whatsapp.WhatsApp", "WhatsApp", m(9)),
            Step::browser(
                "Google Chrome",
                "com.google.Chrome",
                "Plano de Trabalho Cidades Inteligentes - Google Docs",
                "https://docs.google.com/document/d/abc/edit",
                m(40),
            ),
            Step::idle(600.0, m(15)),
            Step::browser(
                "Google Chrome",
                "com.google.Chrome",
                "Trailer 2026 - YouTube",
                "https://www.youtube.com/watch?v=x",
                m(18),
            ),
            Step::app(
                "Microsoft Teams",
                "com.microsoft.teams2",
                "Reunião semanal - Incubadora | Microsoft Teams",
                m(45),
            ),
            Step::app("Terminal", "com.apple.Terminal", "ubiqx — zsh", m(20)),
        ])
    }
}

/// Replays a [`Scenario`]. Implements the read-only platform ports.
pub struct ScriptedPlatform {
    scenario: Mutex<Scenario>,
    cursor: AtomicUsize,
    /// Sample index inside the current step.
    offset: AtomicUsize,
    permissions: Mutex<PermissionStatus>,
}

impl ScriptedPlatform {
    pub fn new(scenario: Scenario) -> Self {
        Self {
            scenario: Mutex::new(scenario),
            cursor: AtomicUsize::new(0),
            offset: AtomicUsize::new(0),
            permissions: Mutex::new(PermissionStatus {
                screen_recording: PermissionState::Granted,
                automation: PermissionState::Granted,
                accessibility: PermissionState::Granted,
            }),
        }
    }

    /// Replaces the scenario and rewinds.
    pub fn load(&self, scenario: Scenario) {
        *self.scenario.lock() = scenario;
        self.cursor.store(0, Ordering::SeqCst);
        self.offset.store(0, Ordering::SeqCst);
    }

    pub fn set_permissions(&self, status: PermissionStatus) {
        *self.permissions.lock() = status;
    }

    /// Whether every step was consumed (only meaningful for non-looping scenarios).
    pub fn finished(&self) -> bool {
        let s = self.scenario.lock();
        !s.repeat && self.cursor.load(Ordering::SeqCst) >= s.steps.len()
    }

    fn current(&self) -> Option<Step> {
        let s = self.scenario.lock();
        if s.steps.is_empty() {
            return None;
        }
        let idx = self.cursor.load(Ordering::SeqCst);
        if idx >= s.steps.len() {
            return if s.repeat {
                s.steps.first().cloned()
            } else {
                s.steps.last().cloned()
            };
        }
        Some(s.steps[idx].clone())
    }

    /// Advances one sample; called by `foreground()` so each call is one sample.
    fn advance(&self) {
        let s = self.scenario.lock();
        let idx = self.cursor.load(Ordering::SeqCst);
        if idx >= s.steps.len() {
            if s.repeat {
                self.cursor.store(0, Ordering::SeqCst);
                self.offset.store(0, Ordering::SeqCst);
            }
            return;
        }
        let off = self.offset.fetch_add(1, Ordering::SeqCst) + 1;
        if off >= s.steps[idx].samples.max(1) as usize {
            self.offset.store(0, Ordering::SeqCst);
            let next = idx + 1;
            if next >= s.steps.len() && s.repeat {
                self.cursor.store(0, Ordering::SeqCst);
            } else {
                self.cursor.store(next, Ordering::SeqCst);
            }
        }
    }
}

impl ActivitySource for ScriptedPlatform {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        let step = self.current();
        self.advance();
        Ok(step.and_then(|s| s.window))
    }
}

impl BrowserUrlResolver for ScriptedPlatform {
    fn supports(&self, app_id: &str) -> bool {
        crate::browser::script_for(app_id).is_some()
    }

    fn resolve(&self, window: &ForegroundWindow) -> CoreResult<Option<String>> {
        // `foreground()` already advanced the cursor; the URL belongs to the step that was
        // just returned, which is the previous sample.
        let s = self.scenario.lock();
        let idx = self.cursor.load(Ordering::SeqCst);
        let off = self.offset.load(Ordering::SeqCst);
        let step_idx = if off == 0 { idx.saturating_sub(1) } else { idx };
        let step = s.steps.get(step_idx.min(s.steps.len().saturating_sub(1)));
        Ok(step
            .filter(|st| {
                st.window
                    .as_ref()
                    .map(|w| w.app_id == window.app_id)
                    .unwrap_or(false)
            })
            .and_then(|st| st.url.clone()))
    }
}

impl IdleDetector for ScriptedPlatform {
    fn idle_secs(&self) -> CoreResult<Option<f64>> {
        // Same "previous sample" logic as `resolve`.
        let s = self.scenario.lock();
        let idx = self.cursor.load(Ordering::SeqCst);
        let off = self.offset.load(Ordering::SeqCst);
        let step_idx = if off == 0 { idx.saturating_sub(1) } else { idx };
        Ok(s.steps
            .get(step_idx.min(s.steps.len().saturating_sub(1)))
            .map(|st| st.idle_secs))
    }
}

impl PermissionChecker for ScriptedPlatform {
    fn status(&self) -> PermissionStatus {
        self.permissions.lock().clone()
    }

    fn request(&self, _kind: PermissionKind) -> CoreResult<()> {
        Ok(())
    }
}

/// Produces a synthetic (but valid) JPEG so screenshot code paths run without a display.
#[derive(Debug, Default, Clone, Copy)]
pub struct SyntheticCapturer;

impl ScreenCapturer for SyntheticCapturer {
    fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage> {
        let seed = match target {
            CaptureTarget::Window(id) => id,
            CaptureTarget::DisplayAt { x, y } => (x.unsigned_abs()).wrapping_add(y.unsigned_abs()),
            CaptureTarget::PrimaryDisplay => 1,
        };
        let img = image::RgbaImage::from_fn(1440, 900, |x, y| {
            let v = ((x / 40 + y / 40 + seed) % 2) as u8;
            image::Rgba([40 + v * 120, 60 + v * 80, 120 + v * 60, 255])
        });
        crate::image_util::downscale_and_encode(img, max_edge)
    }

    fn visible_apps(&self) -> CoreResult<Vec<String>> {
        Ok(vec![])
    }
}

/// A capturer that always fails with a permission error (tests for the degraded path).
#[derive(Debug, Default, Clone, Copy)]
pub struct DeniedCapturer;

impl ScreenCapturer for DeniedCapturer {
    fn capture(&self, _target: CaptureTarget, _max_edge: u32) -> CoreResult<EncodedImage> {
        Err(CoreError::Permission("screen recording".into()))
    }

    fn visible_apps(&self) -> CoreResult<Vec<String>> {
        Ok(vec![])
    }
}

/// An update feed served from memory: `Some(feed)` is returned for any URL, `None` fails
/// like an unreachable feed would. Tests and scripted runs use it instead of the network.
#[derive(Debug, Default)]
pub struct StaticUpdateFeed(Mutex<Option<UpdateFeed>>);

impl StaticUpdateFeed {
    pub fn new(feed: Option<UpdateFeed>) -> Self {
        Self(Mutex::new(feed))
    }

    /// Replaces what the next `fetch` returns.
    pub fn set(&self, feed: Option<UpdateFeed>) {
        *self.0.lock() = feed;
    }
}

#[async_trait]
impl UpdateFeedSource for StaticUpdateFeed {
    async fn fetch(&self, url: &str) -> CoreResult<UpdateFeed> {
        self.0
            .lock()
            .clone()
            .ok_or_else(|| CoreError::NotFound(format!("no update feed at {url} (scripted)")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replays_steps_in_order_then_holds_last() {
        let p = ScriptedPlatform::new(Scenario::new(vec![
            Step::app("A", "a", "t1", 2),
            Step::browser("Chrome", "com.google.Chrome", "t2", "https://x.y/z?q=1", 1),
        ]));
        let w = p.foreground().unwrap().unwrap();
        assert_eq!(w.app_id, "a");
        assert_eq!(p.resolve(&w).unwrap(), None);
        assert_eq!(p.foreground().unwrap().unwrap().app_id, "a");
        let w = p.foreground().unwrap().unwrap();
        assert_eq!(w.app_id, "com.google.Chrome");
        assert_eq!(p.resolve(&w).unwrap().as_deref(), Some("https://x.y/z?q=1"));
        assert!(p.finished());
        // After the end the last step is repeated.
        assert_eq!(p.foreground().unwrap().unwrap().app_id, "com.google.Chrome");
    }

    #[test]
    fn looping_scenario_wraps() {
        let p = ScriptedPlatform::new(
            Scenario::new(vec![
                Step::app("A", "a", "t", 1),
                Step::app("B", "b", "t", 1),
            ])
            .looping(),
        );
        let ids: Vec<String> = (0..5)
            .map(|_| p.foreground().unwrap().unwrap().app_id)
            .collect();
        assert_eq!(ids, vec!["a", "b", "a", "b", "a"]);
        assert!(!p.finished());
    }

    #[test]
    fn idle_follows_step() {
        let p = ScriptedPlatform::new(Scenario::new(vec![
            Step::idle(500.0, 1),
            Step::app("A", "a", "t", 1),
        ]));
        p.foreground().unwrap();
        assert_eq!(p.idle_secs().unwrap(), Some(500.0));
        p.foreground().unwrap();
        assert_eq!(p.idle_secs().unwrap(), Some(0.0));
    }

    #[test]
    fn synthetic_capture_is_jpeg() {
        let img = SyntheticCapturer
            .capture(CaptureTarget::Window(3), 640)
            .unwrap();
        assert_eq!(img.mime, "image/jpeg");
        assert_eq!(img.width, 640);
    }

    #[tokio::test]
    async fn static_feed_serves_or_fails() {
        let feed = StaticUpdateFeed::new(None);
        assert!(matches!(
            feed.fetch("https://x/latest.json").await,
            Err(CoreError::NotFound(_))
        ));
        let parsed: UpdateFeed = serde_json::from_str(
            r#"{"version":"0.1.0","build":{"epoch":7},"platforms":{"darwin-aarch64":{"url":"u"}}}"#,
        )
        .unwrap();
        feed.set(Some(parsed.clone()));
        assert_eq!(feed.fetch("any").await.unwrap(), parsed);
    }

    #[test]
    fn demo_day_is_consistent() {
        let d = Scenario::demo_day();
        assert!(d.steps.iter().all(|s| s.samples > 0));
        assert!(d.steps.len() >= 8);
    }
}
