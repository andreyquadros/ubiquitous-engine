//! The AI check's local rules (design section 4): what is a candidate, how often a window
//! is looked at, the caps, and what an answer means. A screenshot is the most sensitive
//! thing this app can create, so a check runs only when every rule here agrees.
//!
//! There is **no idle input anywhere in this module**: watching video produces no input,
//! so idleness must never stop a check.
//!
//! Seeded by the foundation unit: complete types and constants, signatures only.
#![allow(unused_variables, dead_code)]

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::{ExplicitCheck, FrameOpts, VisionPrefs};

use crate::matcher::Verdict;

// ---------------------------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------------------------

/// Longest edge of the image that is sent, and of the capture it is made from.
pub const NOFAP_MAX_EDGE: u32 = 640;
pub const CAPTURE_MAX_EDGE: u32 = 1280;
/// Fraction cropped from the top of a browser window (tab strip, address bar, bookmarks:
/// other tabs' titles). Nothing is cropped from other apps.
pub const BROWSER_CROP_TOP: f32 = 0.12;
/// JPEG quality of the image that is sent.
pub const JPEG_QUALITY: u8 = 60;

/// A frame within this many bits of the last frame answered `none` for the same key, less
/// than [`UNCHANGED_WINDOW_SECS`] ago, is not sent and not counted against the caps.
pub const DHASH_SAME_BITS: u32 = 4;
pub const UNCHANGED_WINDOW_SECS: i64 = 600;

/// Most checks of one key in an hour.
pub const PER_KEY_PER_HOUR: u32 = 6;
/// How long a `suggestive`, `unsure` or refused answer keeps a key at signal cadence.
pub const SIGNAL_MEMORY_SECS: i64 = 1_800;
/// The heightened state after any block or bypass suspicion.
pub const HEIGHTENED_SECS: i64 = 900;
/// Wait before the second frame of a `Confirm`.
pub const CONFIRM_AFTER_SECS: u64 = 4;
/// Wait before the independent re-check "Não era isso" buys on a vision card.
pub const NOT_IT_RECHECK_SECS: i64 = 20;
/// "Não era isso" uses per day before the button becomes "Pedir exceção".
pub const MAX_SNOOZES_PER_DAY: u32 = 3;
/// How long a keyword or TLD snooze lasts.
pub const SNOOZE_SECS: i64 = 86_400;
/// The soft card is shown at most once per key in this long.
pub const ASK_CARD_EVERY_SECS: i64 = 1_800;
/// Consecutive refusals on a key with a local signal that show the soft card.
pub const REFUSALS_FOR_CARD: u32 = 2;

/// A key waits this long after a timeout, a network error, a 5xx or a rate limit.
pub const ERROR_WAIT_SECS: i64 = 300;
/// Consecutive errors that put vision in backoff, and the steps of that backoff.
pub const ERRORS_FOR_DEGRADED: u32 = 3;
pub const DEGRADED_STEPS_SECS: [i64; 3] = [60, 300, 900];
/// Key rejections after which vision is off with `no_key`.
pub const KEY_REJECTIONS_FOR_OFF: u32 = 3;

/// Per-request timeout handed to the client, and the timeout the service wraps the whole
/// call in (the client's retry policy is per client, not per request).
pub const REQUEST_TIMEOUT_SECS: u64 = 10;
pub const CHECK_TIMEOUT_SECS: u64 = 14;

/// How long a hit keeps the tracker redacting that window and vetoing its screenshots.
pub const HIT_MEMORY_SECS: i64 = 600;

/// How often a context is looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cadence {
    /// Seconds in front before the first check.
    pub first_after_secs: i64,
    /// Seconds between the first and the second check (`None`: `every_secs`).
    pub second_after_secs: Option<i64>,
    /// Seconds between later checks.
    pub every_secs: i64,
    /// After this many `none` answers in a row the interval becomes `relaxed_every_secs`.
    pub relax_after_clean: Option<u32>,
    pub relaxed_every_secs: i64,
    /// How long the relaxed interval lasts (`None`: until a signal).
    pub relax_for_secs: Option<i64>,
}

pub const CADENCE_NO_SIGNAL: Cadence = Cadence {
    first_after_secs: 15,
    second_after_secs: Some(90),
    every_secs: 300,
    relax_after_clean: Some(2),
    relaxed_every_secs: 600,
    relax_for_secs: Some(3_600),
};

pub const CADENCE_SIGNAL: Cadence = Cadence {
    first_after_secs: 5,
    second_after_secs: None,
    every_secs: 45,
    relax_after_clean: None,
    relaxed_every_secs: 45,
    relax_for_secs: None,
};

pub const CADENCE_MEDIA: Cadence = Cadence {
    first_after_secs: 20,
    second_after_secs: None,
    every_secs: 180,
    relax_after_clean: Some(3),
    relaxed_every_secs: 600,
    relax_for_secs: None,
};

pub const CADENCE_MESSAGING: Cadence = Cadence {
    first_after_secs: 30,
    second_after_secs: None,
    every_secs: 300,
    relax_after_clean: None,
    relaxed_every_secs: 300,
    relax_for_secs: None,
};

/// Media players and image viewers, by bundle id (macOS) and executable stem (Windows,
/// Linux), lower-case.
pub const MEDIA_APPS: &[&str] = &[
    "org.videolan.vlc",
    "vlc",
    "io.mpv",
    "mpv",
    "com.colliderli.iina",
    "iina",
    "com.apple.quicktimeplayerx",
    "com.apple.preview",
    "wmplayer",
    "video.ui",
    "microsoft.media.player",
    "microsoft.photos",
    "photos",
    "celluloid",
    "io.github.celluloid_player.celluloid",
    "totem",
    "org.gnome.totem",
    "eog",
    "org.gnome.eog",
];

/// Browsers the URL resolver may not support on a system, by bundle id and executable
/// stem, lower-case. An app the resolver does support is a browser first; this list only
/// keeps the rest from being "any other app", which would escape both the guard and the
/// AI check.
pub const OTHER_BROWSERS: &[&str] = &[
    "org.mozilla.firefox",
    "org.mozilla.firefoxdeveloperedition",
    "org.mozilla.nightly",
    "firefox",
    "firefox-esr",
    "org.torproject.torbrowser",
    "tor browser",
    "torbrowser",
    "start-tor-browser",
    "net.mullvad.mullvadbrowser",
    "mullvad browser",
    "mullvadbrowser",
    "io.gitlab.librewolf-community",
    "librewolf",
    "app.zen-browser.zen",
    "zen",
    "zen-browser",
    "org.mozilla.floorp",
    "floorp",
    "net.waterfox.waterfox",
    "waterfox",
    "com.kagi.kagimacos",
    "orion",
    "com.duckduckgo.macos.browser",
    "duckduckgo",
    "com.operasoftware.opera",
    "com.operasoftware.operagx",
    "opera",
    "opera_gx",
    "ru.yandex.desktop.yandex-browser",
    "yandex",
    "yandex-browser",
    "epiphany",
    "org.gnome.epiphany",
    "falkon",
    "org.kde.falkon",
];

/// Password managers and keychain tools, never captured. Both products' own windows are
/// never captured either (`ubiqx_core::product::is_own_app`).
pub const NEVER_CAPTURE_APPS: &[&str] = &[
    "com.apple.keychainaccess",
    "com.apple.passwords",
    "com.1password.1password",
    "com.agilebits.onepassword7",
    "1password",
    "com.bitwarden.desktop",
    "bitwarden",
    "org.keepassxc.keepassxc",
    "keepassxc",
    "keepass",
    "com.lastpass.lastpass",
    "lastpass",
    "com.dashlane.dashlane",
    "dashlane",
    "me.proton.pass.electron",
    "proton pass",
    "seahorse",
    "org.gnome.seahorse.application",
    "kwalletmanager5",
    "credentialuibroker",
];

// ---------------------------------------------------------------------------------------------
// Candidates
// ---------------------------------------------------------------------------------------------

/// What kind of window is in front, as far as the AI check cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowKind {
    /// A supported browser whose URL was read.
    Browser,
    /// A supported browser that gave no URL, or an app on [`OTHER_BROWSERS`].
    UnreadableBrowser,
    /// An app on [`MEDIA_APPS`].
    Media,
    /// A messaging or mail app (`ubiqx_core::redact::is_messaging_app`).
    Messaging,
    /// An app on [`NEVER_CAPTURE_APPS`], or one of the two products.
    NeverCapture,
    /// Any other app: never a candidate.
    Other,
}

/// Classifies the app in front. `url_supported` is `BrowserUrlResolver::supports`, and
/// `has_url` whether a URL was read for this window.
pub fn window_kind(app_id: &str, app_name: &str, url_supported: bool, has_url: bool) -> WindowKind {
    todo!()
}

/// The in-memory key of a context: 8 bytes of a hash of the host, or of app id and title
/// for a window without a URL. Kept in memory only and gone at exit.
pub fn sample_key(host: Option<&str>, app_id: &str, title: &str) -> u64 {
    todo!()
}

/// How a frame of this kind of window is taken (the top crop applies to browsers only).
pub fn frame_opts(kind: WindowKind) -> FrameOpts {
    todo!()
}

/// Everything [`decide`] knows about the window in front. Deliberately without any idle
/// time, URL, title or application name.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleCtx {
    pub key: u64,
    pub kind: WindowKind,
    /// The matcher's verdict, for a browser whose URL was read.
    pub verdict: Option<Verdict>,
    /// Whether the OS gave a window id; without one there is no capture.
    pub has_window_id: bool,
    /// A private-browsing window (its title is a signal).
    pub private_window: bool,
    /// A block-grade word in the window title.
    pub title_signal: bool,
    /// A block-grade word in the URL path.
    pub path_signal: bool,
    /// The app or domain is on an effective exclusion of the host's privacy lists.
    pub excluded: bool,
    /// The user's scope switches.
    pub unknown_sites: bool,
    pub media_apps: bool,
    pub messaging_apps: bool,
    pub private_windows: bool,
}

impl SampleCtx {
    /// A context with the scope switches of `prefs` and nothing else set.
    pub fn new(key: u64, kind: WindowKind, prefs: &VisionPrefs) -> Self {
        todo!()
    }
}

/// Why nothing is captured now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Skip {
    /// Not a window the check ever looks at (already decided host, never-capture, local,
    /// popular without a signal, an app outside the lists, a scope switch that is off).
    NotCandidate,
    /// On an effective exclusion.
    Excluded,
    NoWindowId,
    /// Not in front long enough, or the interval since the last check has not passed.
    NotDue,
    /// The key is waiting after an error.
    KeyBackoff,
    /// 6 per key per hour.
    KeyCap,
    HourCap,
    DayCap,
    /// Vision's own health is in backoff.
    Degraded,
}

/// What to do about the window in front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Skip(Skip),
    /// Take a frame and, unless it is unchanged ([`frame_unchanged`]), send it.
    Capture {
        signal: bool,
    },
}

/// The memory of the sampler: per-key history, the hour's and the day's counts, the
/// heightened state and vision's health. In memory only; gone at exit and on "Apagar".
#[derive(Debug, Clone, Default)]
pub struct SamplerMemory {
    keys: HashMap<u64, KeyMemory>,
    front: Option<(u64, DateTime<Utc>)>,
    hour_checks: Vec<DateTime<Utc>>,
    day: Option<(String, u32)>,
    heightened_until: Option<DateTime<Utc>>,
    error_streak: u32,
    degraded_step: usize,
    degraded_until: Option<DateTime<Utc>>,
    key_rejections: u32,
}

#[derive(Debug, Clone, Default)]
struct KeyMemory {
    checks: Vec<DateTime<Utc>>,
    clean_streak: u32,
    relaxed_until: Option<DateTime<Utc>>,
    signal_until: Option<DateTime<Utc>>,
    last_clean: Option<(u64, DateTime<Utc>)>,
    refusal_streak: u32,
    wait_until: Option<DateTime<Utc>>,
    recheck_at: Option<DateTime<Utc>>,
    exception_only_day: Option<String>,
    last_ask_card_at: Option<DateTime<Utc>>,
}

impl SamplerMemory {
    /// Sets the number of checks already made today (from the day counters), so the daily
    /// cap survives a restart. `day` is the local `YYYY-MM-DD`.
    pub fn set_day_count(&mut self, day: &str, checks: u32) {
        todo!()
    }

    /// Whether the heightened state is on at `now`.
    pub fn heightened(&self, now: DateTime<Utc>) -> bool {
        todo!()
    }

    /// Whether `key` has a remembered signal at `now` (a `suggestive`, `unsure` or refused
    /// answer in the last [`SIGNAL_MEMORY_SECS`]).
    pub fn has_signal(&self, key: u64, now: DateTime<Utc>) -> bool {
        todo!()
    }

    /// Until when vision's health is in backoff, if it is at `now`.
    pub fn degraded_until(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        todo!()
    }

    /// The provider rejected the key [`KEY_REJECTIONS_FOR_OFF`] times.
    pub fn key_rejected(&self) -> bool {
        todo!()
    }

    /// Whether the hour's or the day's cap is reached at `now` under `prefs`.
    pub fn cap_reached(&self, prefs: &VisionPrefs, day: &str, now: DateTime<Utc>) -> bool {
        todo!()
    }

    /// Whether "Não era isso" on `key` is, for the rest of local `day`, a delayed
    /// exception request (its independent re-check was positive again).
    pub fn exception_only(&self, key: u64, day: &str) -> bool {
        todo!()
    }

    /// Whether the soft card may be shown for `key` at `now` (at most once per
    /// [`ASK_CARD_EVERY_SECS`]).
    pub fn ask_card_allowed(&self, key: u64, now: DateTime<Utc>) -> bool {
        todo!()
    }

    /// Forgets everything.
    pub fn clear(&mut self) {
        todo!()
    }
}

/// Whether the window described by `ctx` is looked at now (the tables of design sections
/// 4.2 and 4.3). `day` is the local day, for the daily cap.
pub fn decide(ctx: &SampleCtx, memory: &SamplerMemory, day: &str, now: DateTime<Utc>) -> Decision {
    todo!()
}

/// Whether a frame with this difference hash is the same picture as the last one answered
/// `none` for `key` (within [`DHASH_SAME_BITS`] bits, less than [`UNCHANGED_WINDOW_SECS`]
/// ago). Such a frame is not sent and not counted.
pub fn frame_unchanged(memory: &SamplerMemory, key: u64, dhash: u64, now: DateTime<Utc>) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------------------------
// Outcomes
// ---------------------------------------------------------------------------------------------

/// What an answer means (design section 4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// `explicit` + `high`: act.
    Positive,
    /// `explicit` + `medium`: take a second frame [`CONFIRM_AFTER_SECS`] later.
    Confirm,
    /// `explicit` + `low`: no action on the window; the soft card.
    Unsure,
    /// `suggestive`, `unsure`, or a refusal: nothing shown; a signal for 30 minutes.
    Watch,
    /// `none`: counts towards relaxing.
    Clean,
}

/// Which frame of a check an answer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    First,
    /// The second frame after a [`Outcome::Confirm`]: `explicit` at `medium` or `high` is
    /// `Positive`, anything else `Unsure`.
    Second,
}

/// The outcome of one answer. A refusal is never `Positive`.
pub fn outcome(check: &ExplicitCheck, stage: Stage) -> Outcome {
    todo!()
}

/// Why a check produced no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckError {
    /// Timeout, network, 5xx or rate limit.
    Transient,
    /// The provider rejected the key or the account.
    KeyRejected,
}

/// Something the sampler has to remember.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleEvent {
    /// The key is in front in this pass (dwell time starts when the key changes).
    Front,
    /// A frame was sent and answered. `dhash` is remembered for a `Clean` answer only.
    Checked {
        outcome: Outcome,
        refused: bool,
        dhash: u64,
    },
    /// A frame was sent and the call failed.
    Failed(CheckError),
    /// A block or a bypass suspicion: the heightened state starts.
    Heightened,
    /// "Não era isso" on a vision card for this key: one re-check
    /// [`NOT_IT_RECHECK_SECS`] later, whatever the caps say.
    NotIt,
    /// That re-check was positive again: the key's button is "Pedir exceção" for the rest
    /// of the day.
    NotItConfirmed,
    /// The soft card was shown for this key.
    AskCardShown,
}

/// Records `event` for `key` at `now`. `day` is the local day (for the daily count).
pub fn record(
    memory: &mut SamplerMemory,
    key: u64,
    event: SampleEvent,
    day: &str,
    now: DateTime<Utc>,
) {
    todo!()
}

/// Whether the soft card is due after a refusal: [`REFUSALS_FOR_CARD`] refusals in a row
/// on a key that also has a local signal.
pub fn refusal_card_due(memory: &SamplerMemory, key: u64, local_signal: bool) -> bool {
    todo!()
}

/// What "Não era isso" does on a keyword or TLD block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotItEffect {
    /// A 24-hour snooze of the keyword and TLD rules for that host; vision keeps sampling
    /// it at signal cadence.
    Snooze,
    /// A delayed allow request ("Pedir exceção").
    Request,
}

/// The effect of "Não era isso" on a block card: a snooze only for a keyword or TLD block
/// while vision is ready and fewer than [`MAX_SNOOZES_PER_DAY`] were used today; a delayed
/// request for a list hit, a hit on the user's own list, and everything else.
pub fn not_it_on_block(
    source: crate::matcher::BlockSource,
    vision_ready: bool,
    not_it_today: u32,
) -> NotItEffect {
    todo!()
}
