//! Turns a stream of [`ActivitySample`]s into [`ActivityBlock`]s.
//!
//! The segmenter is a small state machine: it keeps at most one *open* block and decides, for
//! each new sample, whether to extend it, close it, or ignore the sample (idle / blocked app).

use chrono::{DateTime, Duration, Utc};

use crate::lang::UiLanguage;
use crate::model::*;
use crate::normalize::{domain_of, normalize_title};

/// Title given to blocks whose content must not be recorded.
pub const PRIVATE_TITLE: &str = "[privado]";

#[derive(Debug, Clone, PartialEq)]
pub struct SegmenterConfig {
    /// Private mode: every sample is recorded as a redacted "Privado" block.
    pub private_mode: bool,
    pub idle_threshold_secs: u32,
    pub min_block_secs: u32,
    /// If two samples are farther apart than this, the open block is closed (machine slept).
    pub max_gap_secs: u32,
    pub blocked_apps: Vec<String>,
    pub blocked_domains: Vec<String>,
    /// UI language: names the redacted private-mode app ("Privado" / "Private").
    pub language: UiLanguage,
}

impl From<&Settings> for SegmenterConfig {
    fn from(s: &Settings) -> Self {
        Self {
            private_mode: false,
            idle_threshold_secs: s.idle_threshold_secs,
            min_block_secs: s.min_block_secs,
            max_gap_secs: (s.sample_interval_secs * 6).max(30),
            blocked_apps: s.blocked_apps.clone(),
            blocked_domains: s.blocked_domains.clone(),
            language: UiLanguage::from_tag(&s.language),
        }
    }
}

impl Default for SegmenterConfig {
    fn default() -> Self {
        Self::from(&Settings::default())
    }
}

/// What the caller has to persist after feeding a sample.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SegmentOutcome {
    /// A block that was just closed. It is a candidate for classification.
    pub closed: Option<ActivityBlock>,
    /// A block that was just created.
    pub opened: Option<ActivityBlock>,
    /// The open block after this sample (extended or unchanged). `None` when idle.
    pub open: Option<ActivityBlock>,
    /// The sample was dropped (idle, blocked app or no foreground window).
    pub ignored: bool,
}

#[derive(Debug, Default)]
pub struct Segmenter {
    cfg: SegmenterConfig,
    open: Option<OpenBlock>,
}

#[derive(Debug, Clone)]
struct OpenBlock {
    block: ActivityBlock,
    /// Title frequency so the representative title is the most common one, not the first.
    titles: Vec<(String, u32)>,
}

impl Segmenter {
    pub fn new(cfg: SegmenterConfig) -> Self {
        Self { cfg, open: None }
    }

    /// Restores an open block persisted by a previous run so no time is lost on restart.
    pub fn with_open_block(mut self, block: Option<ActivityBlock>) -> Self {
        self.open = block.map(|b| OpenBlock {
            titles: vec![(b.title.clone(), b.sample_count)],
            block: b,
        });
        self
    }

    pub fn config(&self) -> &SegmenterConfig {
        &self.cfg
    }

    pub fn update_config(&mut self, cfg: SegmenterConfig) {
        self.cfg = cfg;
    }

    pub fn open_block(&self) -> Option<&ActivityBlock> {
        self.open.as_ref().map(|o| &o.block)
    }

    fn is_blocked(&self, sample: &ActivitySample) -> bool {
        let app_blocked = self.cfg.blocked_apps.iter().any(|b| {
            b.eq_ignore_ascii_case(&sample.app_id) || b.eq_ignore_ascii_case(&sample.app_name)
        });
        if app_blocked || crate::normalize::is_private_browsing_title(&sample.window_title) {
            return true;
        }
        if let Some(d) = sample.url.as_deref().and_then(domain_of) {
            return self
                .cfg
                .blocked_domains
                .iter()
                .any(|p| crate::normalize::domain_matches(&d, p));
        }
        false
    }

    /// Feeds one sample. `sample_interval` is used to extend the block end to cover the
    /// interval the sample represents.
    pub fn feed(&mut self, sample: &ActivitySample, sample_interval: Duration) -> SegmentOutcome {
        let mut out = SegmentOutcome::default();

        // 1. Idle → close whatever is open, ignore the sample.
        if sample.is_idle(self.cfg.idle_threshold_secs) {
            out.ignored = true;
            out.closed = self.close(sample.at);
            return out;
        }

        // 2. Blocked app / private browsing / private mode → keep the time, drop the content.
        let redacted;
        let sample = if self.is_blocked(sample) || self.cfg.private_mode {
            redacted = ActivitySample {
                at: sample.at,
                app_name: if self.cfg.private_mode {
                    self.cfg.language.pick("Privado", "Private").into()
                } else {
                    sample.app_name.clone()
                },
                app_id: if self.cfg.private_mode {
                    "privado".into()
                } else {
                    sample.app_id.clone()
                },
                window_title: PRIVATE_TITLE.into(),
                url: None,
                idle_secs: sample.idle_secs,
                window_id: None,
            };
            &redacted
        } else {
            sample
        };
        let is_private = sample.window_title == PRIVATE_TITLE;

        let title_key = normalize_title(&sample.window_title, &sample.app_name);
        let domain = sample.url.as_deref().and_then(domain_of);
        let key = context_key(&sample.app_id, &title_key, domain.as_deref());

        // 3. Same context and no big gap → extend.
        if let Some(open) = self.open.as_mut() {
            let gap = sample.at - open.block.ended_at;
            let same = context_key(
                &open.block.app_id,
                &open.block.title_key,
                open.block.domain.as_deref(),
            ) == key;
            if same && gap <= Duration::seconds(self.cfg.max_gap_secs as i64) {
                open.block.ended_at = sample.at + sample_interval;
                open.block.sample_count += 1;
                if open.block.url.is_none() {
                    open.block.url = sample.url.clone();
                }
                bump_title(&mut open.titles, &sample.window_title);
                open.block.title = representative_title(&open.titles);
                out.open = Some(open.block.clone());
                return out;
            }
        }

        // 4. Different context → close the old block and open a new one.
        out.closed = self.close(sample.at);
        let block = ActivityBlock {
            id: new_id(),
            started_at: sample.at,
            ended_at: sample.at + sample_interval,
            app_name: sample.app_name.clone(),
            app_id: sample.app_id.clone(),
            title: sample.window_title.clone(),
            title_key,
            url: sample.url.clone(),
            domain,
            // Private blocks are classified on the spot and never reach a remote model.
            category_id: is_private.then(|| system_categories::PRIVATE.to_string()),
            confidence: if is_private { 1.0 } else { 0.0 },
            source: is_private.then_some(ClassificationSource::Rule),
            description: None,
            screenshot_id: None,
            sample_count: 1,
            is_open: true,
            classify_attempts: 0,
            next_attempt_at: None,
            needs_review: false,
            ai_payload: None,
            ai_sent_at: None,
            is_manual: false,
            note: None,
        };
        self.open = Some(OpenBlock {
            titles: vec![(sample.window_title.clone(), 1)],
            block: block.clone(),
        });
        out.opened = Some(block.clone());
        out.open = Some(block);
        out
    }

    /// Closes the open block (e.g. when tracking is paused). The block end is clamped to `at`
    /// so a block never extends past the moment it was closed.
    pub fn close(&mut self, at: DateTime<Utc>) -> Option<ActivityBlock> {
        let open = self.open.take()?;
        let mut block = open.block;
        if block.ended_at > at && at > block.started_at {
            block.ended_at = at;
        }
        block.is_open = false;
        Some(block)
    }

    /// Whether a closed block is long enough to be kept on its own.
    pub fn is_significant(&self, block: &ActivityBlock) -> bool {
        block.duration_secs() >= self.cfg.min_block_secs as i64
    }
}

fn context_key(app_id: &str, title_key: &str, domain: Option<&str>) -> String {
    match domain {
        Some(d) => format!("{}\u{1}{}", app_id.to_lowercase(), d),
        None => format!("{}\u{1}{}", app_id.to_lowercase(), title_key),
    }
}

fn bump_title(titles: &mut Vec<(String, u32)>, title: &str) {
    if let Some(t) = titles.iter_mut().find(|(t, _)| t == title) {
        t.1 += 1;
    } else if titles.len() < 32 {
        titles.push((title.to_string(), 1));
    }
}

fn representative_title(titles: &[(String, u32)]) -> String {
    titles
        .iter()
        .max_by_key(|(_, n)| *n)
        .map(|(t, _)| t.clone())
        .unwrap_or_default()
}

/// Merges short blocks into the previous block when they share the same app. Called by the
/// engine on a day's worth of closed blocks before reporting; keeps the timeline readable.
pub fn merge_short_blocks(blocks: Vec<ActivityBlock>, min_secs: i64) -> Vec<ActivityBlock> {
    let mut out: Vec<ActivityBlock> = Vec::with_capacity(blocks.len());
    for b in blocks {
        if b.duration_secs() < min_secs {
            if let Some(prev) = out.last_mut() {
                if prev.app_id == b.app_id && prev.category_id == b.category_id {
                    prev.ended_at = prev.ended_at.max(b.ended_at);
                    prev.sample_count += b.sample_count;
                    continue;
                }
            }
        }
        out.push(b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 9, 0, 0).unwrap() + Duration::seconds(secs)
    }

    fn sample(secs: i64, app: &str, title: &str, url: Option<&str>) -> ActivitySample {
        ActivitySample {
            at: t(secs),
            app_name: app.into(),
            app_id: format!("com.test.{}", app.to_lowercase()),
            window_title: title.into(),
            url: url.map(|u| u.to_string()),
            idle_secs: Some(0.0),
            window_id: None,
        }
    }

    #[test]
    fn extends_same_context_and_closes_on_switch() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let iv = Duration::seconds(5);
        let o = seg.feed(&sample(0, "Xcode", "main.swift", None), iv);
        assert!(o.opened.is_some());
        let o = seg.feed(&sample(5, "Xcode", "main.swift", None), iv);
        assert!(o.opened.is_none());
        assert_eq!(o.open.unwrap().sample_count, 2);
        let o = seg.feed(&sample(10, "Slack", "general", None), iv);
        let closed = o.closed.expect("switch closes block");
        assert_eq!(closed.app_name, "Xcode");
        assert_eq!(closed.duration_secs(), 10);
        assert!(!closed.is_open);
        assert_eq!(o.opened.unwrap().app_name, "Slack");
    }

    #[test]
    fn browser_groups_by_domain_not_title() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let iv = Duration::seconds(5);
        seg.feed(
            &sample(
                0,
                "Google Chrome",
                "SEI - Proc 1",
                Some("https://sei.ifro.edu.br/a"),
            ),
            iv,
        );
        let o = seg.feed(
            &sample(
                5,
                "Google Chrome",
                "SEI - Proc 2",
                Some("https://sei.ifro.edu.br/b"),
            ),
            iv,
        );
        assert!(o.closed.is_none(), "same domain should extend");
        let o = seg.feed(
            &sample(10, "Google Chrome", "YouTube", Some("https://youtube.com/")),
            iv,
        );
        assert!(o.closed.is_some(), "different domain should close");
    }

    #[test]
    fn idle_closes_block_and_is_ignored() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let iv = Duration::seconds(5);
        seg.feed(&sample(0, "Xcode", "main.swift", None), iv);
        let mut idle = sample(300, "Xcode", "main.swift", None);
        idle.idle_secs = Some(400.0);
        let o = seg.feed(&idle, iv);
        assert!(o.ignored);
        let closed = o.closed.unwrap();
        // The block was last extended at 0+5s; idle at 300s must not stretch it.
        assert_eq!(closed.duration_secs(), 5);
        assert!(seg.open_block().is_none());
    }

    #[test]
    fn blocked_apps_become_private_blocks() {
        let cfg = SegmenterConfig {
            blocked_apps: vec!["1Password".into()],
            ..Default::default()
        };
        let mut seg = Segmenter::new(cfg);
        let o = seg.feed(&sample(0, "1Password", "Vault", None), Duration::seconds(5));
        assert!(!o.ignored);
        let b = o.opened.unwrap();
        assert_eq!(b.title, PRIVATE_TITLE);
        assert_eq!(b.app_name, "1Password");
        assert_eq!(b.category_id.as_deref(), Some(system_categories::PRIVATE));
        assert!(b.url.is_none());
        // Private browsing windows are treated the same way.
        let o = seg.feed(
            &sample(
                5,
                "Google Chrome",
                "Google - Navegação anônima",
                Some("https://x.com/secret"),
            ),
            Duration::seconds(5),
        );
        let b = o.opened.unwrap();
        assert_eq!(b.title, PRIVATE_TITLE);
        assert!(b.url.is_none());
    }

    #[test]
    fn private_mode_app_name_follows_the_language() {
        let cfg = SegmenterConfig {
            private_mode: true,
            language: UiLanguage::En,
            ..Default::default()
        };
        let mut seg = Segmenter::new(cfg);
        let o = seg.feed(
            &sample(0, "Xcode", "main.swift", None),
            Duration::seconds(5),
        );
        let b = o.opened.unwrap();
        assert_eq!(b.app_id, "privado");
        assert_eq!(b.app_name, "Private");
        assert_eq!(b.title, PRIVATE_TITLE);
    }

    #[test]
    fn private_mode_records_time_only() {
        let cfg = SegmenterConfig {
            private_mode: true,
            ..Default::default()
        };
        let mut seg = Segmenter::new(cfg);
        let o = seg.feed(
            &sample(0, "Xcode", "main.swift", None),
            Duration::seconds(5),
        );
        let b = o.opened.unwrap();
        assert_eq!(b.app_id, "privado");
        assert_eq!(b.app_name, "Privado");
        assert_eq!(b.category_id.as_deref(), Some(system_categories::PRIVATE));
        let o = seg.feed(&sample(5, "Slack", "general", None), Duration::seconds(5));
        assert!(o.closed.is_none(), "all private samples share one block");
    }

    #[test]
    fn large_gap_starts_new_block() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let iv = Duration::seconds(5);
        seg.feed(&sample(0, "Xcode", "main.swift", None), iv);
        let o = seg.feed(&sample(600, "Xcode", "main.swift", None), iv);
        assert!(o.closed.is_some());
        assert!(o.opened.is_some());
    }

    #[test]
    fn representative_title_is_most_frequent() {
        let mut seg = Segmenter::new(SegmenterConfig::default());
        let iv = Duration::seconds(5);
        seg.feed(&sample(0, "Xcode", "a.swift", None), iv);
        // Same title key? no — different titles are different contexts for non-browsers.
        // Use a browser so grouping is by domain and titles vary.
        let mut seg2 = Segmenter::new(SegmenterConfig::default());
        seg2.feed(
            &sample(0, "Safari", "Doc A", Some("https://docs.google.com/1")),
            iv,
        );
        seg2.feed(
            &sample(5, "Safari", "Doc B", Some("https://docs.google.com/2")),
            iv,
        );
        let o = seg2.feed(
            &sample(10, "Safari", "Doc B", Some("https://docs.google.com/2")),
            iv,
        );
        assert_eq!(o.open.unwrap().title, "Doc B");
        let _ = seg;
    }

    #[test]
    fn merge_short() {
        let mk = |s: i64, e: i64, app: &str| ActivityBlock {
            id: new_id(),
            started_at: t(s),
            ended_at: t(e),
            app_name: app.into(),
            app_id: app.into(),
            title: String::new(),
            title_key: String::new(),
            url: None,
            domain: None,
            category_id: None,
            confidence: 0.0,
            source: None,
            description: None,
            screenshot_id: None,
            sample_count: 1,
            is_open: false,
            classify_attempts: 0,
            next_attempt_at: None,
            needs_review: false,
            ai_payload: None,
            ai_sent_at: None,
            is_manual: false,
            note: None,
        };
        let merged = merge_short_blocks(
            vec![mk(0, 100, "A"), mk(100, 105, "A"), mk(105, 110, "B")],
            20,
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].duration_secs(), 105);
    }
}
