//! The AI check's local rules (design section 4): what is a candidate, how often a window
//! is looked at, the caps, and what an answer means. A screenshot is the most sensitive
//! thing this app can create, so a check runs only when every rule here agrees.
//!
//! There is **no idle input anywhere in this module**: watching video produces no input,
//! so idleness must never stop a check.
//!
//! # How the pieces are called
//!
//! Every guard pass, for the window in front: [`record`] `Front`, then [`decide`]. On
//! `Capture` the caller takes one frame; when [`frame_unchanged`] says it is the picture
//! that was answered `none` a moment ago it records `Unchanged` and sends nothing,
//! otherwise it sends the frame and records `Checked` or `Failed`. Everything is kept in
//! [`SamplerMemory`], in memory only.
//!
//! # Clock changes
//!
//! Every "until" and "since" kept here is read against the length it can have: a time
//! further in the future than its own duration allows (the clock was set back) counts as
//! over, and a look that seems to lie in the future never delays the next one. A clock
//! change can therefore cost a few extra checks, bounded by the caps, but cannot switch
//! the check off for hours.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use ubiqx_core::nofap::{Confidence, ExplicitCheck, ExplicitVerdict, FrameOpts, VisionPrefs};
use ubiqx_core::product::is_own_app;
use ubiqx_core::redact::is_messaging_app;

use crate::matcher::{AllowSource, BlockSource, HostClass, Verdict};

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
    /// Whether the interval doubles with every check of the key in the hour: `every_secs`
    /// after the first, twice that after the second, and so on ([`spread_gap`]).
    pub doubles: bool,
}

pub const CADENCE_NO_SIGNAL: Cadence = Cadence {
    first_after_secs: 15,
    second_after_secs: Some(90),
    every_secs: 300,
    relax_after_clean: Some(2),
    relaxed_every_secs: 600,
    relax_for_secs: Some(3_600),
    doubles: false,
};

/// With a standing signal the six checks a key gets in an hour are spread over it: 5 s in
/// front, then 45 s, 90 s, 3 min, 6 min and 12 min apart (the next would be 24 min later,
/// and the cap of [`PER_KEY_PER_HOUR`] comes first). The gaps start again at 45 s when the
/// key's hour is over.
pub const CADENCE_SIGNAL: Cadence = Cadence {
    first_after_secs: 5,
    second_after_secs: None,
    every_secs: 45,
    relax_after_clean: None,
    relaxed_every_secs: 45,
    relax_for_secs: None,
    doubles: true,
};

pub const CADENCE_MEDIA: Cadence = Cadence {
    first_after_secs: 20,
    second_after_secs: None,
    every_secs: 180,
    relax_after_clean: Some(3),
    relaxed_every_secs: 600,
    relax_for_secs: None,
    doubles: false,
};

pub const CADENCE_MESSAGING: Cadence = Cadence {
    first_after_secs: 30,
    second_after_secs: None,
    every_secs: 300,
    relax_after_clean: None,
    relaxed_every_secs: 300,
    relax_for_secs: None,
    doubles: false,
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

fn listed(list: &[&str], app_id: &str, app_name: &str) -> bool {
    list.iter()
        .any(|entry| entry.eq_ignore_ascii_case(app_id) || entry.eq_ignore_ascii_case(app_name))
}

/// Classifies the app in front. `url_supported` is `BrowserUrlResolver::supports`, and
/// `has_url` whether a URL was read for this window. The never-capture list and the two
/// products come first, whatever else an app may be.
pub fn window_kind(app_id: &str, app_name: &str, url_supported: bool, has_url: bool) -> WindowKind {
    if is_own_app(app_id, app_name) || listed(NEVER_CAPTURE_APPS, app_id, app_name) {
        WindowKind::NeverCapture
    } else if url_supported && has_url {
        WindowKind::Browser
    } else if url_supported || listed(OTHER_BROWSERS, app_id, app_name) {
        WindowKind::UnreadableBrowser
    } else if listed(MEDIA_APPS, app_id, app_name) {
        WindowKind::Media
    } else if is_messaging_app(app_id, app_name) {
        WindowKind::Messaging
    } else {
        WindowKind::Other
    }
}

/// The in-memory key of a context: 8 bytes of a hash of the host, or of app id and title
/// for a window without a URL. Kept in memory only and gone at exit.
pub fn sample_key(host: Option<&str>, app_id: &str, title: &str) -> u64 {
    let mut hasher = Sha256::new();
    match host {
        Some(host) => {
            hasher.update(b"host\0");
            hasher.update(host.to_ascii_lowercase().as_bytes());
        }
        None => {
            hasher.update(b"window\0");
            hasher.update(app_id.to_lowercase().as_bytes());
            hasher.update([0u8]);
            hasher.update(title.as_bytes());
        }
    }
    let digest = hasher.finalize();
    let mut key = [0u8; 8];
    key.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(key)
}

/// How a frame of this kind of window is taken (the top crop applies to browsers only).
pub fn frame_opts(kind: WindowKind) -> FrameOpts {
    let crop_top_frac = match kind {
        WindowKind::Browser | WindowKind::UnreadableBrowser => BROWSER_CROP_TOP,
        WindowKind::Media
        | WindowKind::Messaging
        | WindowKind::NeverCapture
        | WindowKind::Other => 0.0,
    };
    FrameOpts {
        max_edge: NOFAP_MAX_EDGE,
        crop_top_frac,
    }
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
    /// The user's caps on checks per hour and per day.
    pub max_per_hour: u32,
    pub max_per_day: u32,
}

impl SampleCtx {
    /// A context with the scope switches and the caps of `prefs` and nothing else set.
    pub fn new(key: u64, kind: WindowKind, prefs: &VisionPrefs) -> Self {
        Self {
            key,
            kind,
            verdict: None,
            has_window_id: false,
            private_window: false,
            title_signal: false,
            path_signal: false,
            excluded: false,
            unknown_sites: prefs.unknown_sites,
            media_apps: prefs.media_apps,
            messaging_apps: prefs.messaging_apps,
            private_windows: prefs.private_windows,
            max_per_hour: prefs.max_per_hour,
            max_per_day: prefs.max_per_day,
        }
    }

    /// Whether the window itself carries a signal: a signal-grade word in the host or the
    /// path, a block-grade word in the title, a private-browsing title, a mixed host, or a
    /// host whose keyword block the user snoozed. (The heightened state and a remembered
    /// answer are signals too; those live in [`SamplerMemory`].)
    pub fn local_signal(&self) -> bool {
        let host_signal = match self.verdict {
            Some(Verdict::Unknown(HostClass::Mixed))
            | Some(Verdict::Unknown(HostClass::Plain { signal: true }))
            | Some(Verdict::Allowed(AllowSource::Snooze)) => true,
            Some(Verdict::Unknown(
                HostClass::Plain { signal: false }
                | HostClass::Popular
                | HostClass::Local
                | HostClass::NeverCapture,
            ))
            | Some(Verdict::Allowed(AllowSource::User | AllowSource::BuiltIn))
            | Some(Verdict::Blocked(_))
            | None => false,
        };
        host_signal || self.title_signal || self.path_signal || self.private_window
    }

    /// Whether the check ever looks at this window (the table of design section 4.2).
    /// `signal` is any signal, local or remembered.
    fn is_candidate(&self, signal: bool) -> bool {
        let private_ok = !self.private_window || self.private_windows;
        match self.kind {
            WindowKind::NeverCapture | WindowKind::Other => false,
            WindowKind::Browser => {
                private_ok
                    && match self.verdict {
                        // Already decided.
                        Some(Verdict::Blocked(_)) => false,
                        Some(Verdict::Allowed(AllowSource::User | AllowSource::BuiltIn)) => false,
                        // "Não era isso" lifted a keyword block: the check keeps looking.
                        Some(Verdict::Allowed(AllowSource::Snooze)) => true,
                        Some(Verdict::Unknown(HostClass::Local | HostClass::NeverCapture)) => false,
                        Some(Verdict::Unknown(HostClass::Popular)) => signal,
                        Some(Verdict::Unknown(HostClass::Mixed)) => true,
                        Some(Verdict::Unknown(HostClass::Plain { .. })) => self.unknown_sites,
                        // A URL was read but could not be judged: as an unknown site.
                        None => self.unknown_sites,
                    }
            }
            WindowKind::UnreadableBrowser => private_ok && self.unknown_sites,
            WindowKind::Media => self.media_apps,
            WindowKind::Messaging => self.messaging_apps,
        }
    }

    /// The cadence of this window. Messaging apps keep their own slow cadence whatever
    /// the signals say: those are other people's messages.
    fn cadence(&self, signal: bool) -> &'static Cadence {
        match self.kind {
            WindowKind::Messaging => &CADENCE_MESSAGING,
            _ if signal => &CADENCE_SIGNAL,
            WindowKind::Media => &CADENCE_MEDIA,
            WindowKind::Browser
            | WindowKind::UnreadableBrowser
            | WindowKind::NeverCapture
            | WindowKind::Other => &CADENCE_NO_SIGNAL,
        }
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

/// Seconds in an hour: the window of the hourly caps.
const HOUR_SECS: i64 = 3_600;
/// Keys kept before the ones not seen for a day are dropped.
const MAX_KEYS: usize = 2_048;
const KEY_IDLE_SECS: i64 = 86_400;
/// `none` answers in a row whose times are kept (the longest streak a cadence asks for).
const CLEAN_TIMES_KEPT: usize = 3;

/// Whether a time kept as "until" still lies ahead at `now`. One further ahead than
/// `max_secs` cannot have been set by this module under the present clock (the clock was
/// set back), and counts as over.
fn still(until: Option<DateTime<Utc>>, now: DateTime<Utc>, max_secs: i64) -> bool {
    until.is_some_and(|until| until > now && (until - now).num_seconds() <= max_secs)
}

/// The wait after the `checks`-th check of a key in its hour, at a cadence that doubles:
/// `every_secs` after the first, twice the previous wait after each later one, and never
/// more than the hour itself. Before the first check of the hour it is `every_secs`.
pub fn spread_gap(every_secs: i64, checks: u32) -> i64 {
    let doublings = checks.saturating_sub(1);
    // A shift into or past the sign bit is "more than any hour".
    let factor = 1_i64
        .checked_shl(doublings)
        .filter(|factor| *factor > 0)
        .unwrap_or(i64::MAX);
    every_secs.saturating_mul(factor).clamp(0, HOUR_SECS)
}

/// How many of `times` lie in the hour that ends at `now`.
fn in_last_hour(times: &[DateTime<Utc>], now: DateTime<Utc>) -> u32 {
    let count = times
        .iter()
        .filter(|time| (0..HOUR_SECS).contains(&(now - **time).num_seconds()))
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The memory of the sampler: per-key history, the hour's and the day's counts, the
/// heightened state and vision's health. In memory only; gone at exit and on "Apagar".
#[derive(Debug, Clone, Default)]
pub struct SamplerMemory {
    keys: HashMap<u64, KeyMemory>,
    /// The key in front and since when.
    front: Option<(u64, DateTime<Utc>)>,
    /// Frames sent in the last hour, for any key.
    hour_checks: Vec<DateTime<Utc>>,
    /// The local day and the frames sent on it.
    day: Option<(String, u32)>,
    heightened_until: Option<DateTime<Utc>>,
    /// Failed calls in a row (timeouts, network, 5xx, rate limits).
    error_streak: u32,
    degraded_until: Option<DateTime<Utc>>,
    key_rejections: u32,
}

#[derive(Debug, Clone, Default)]
struct KeyMemory {
    /// Frames sent for this key in the last hour.
    checks: Vec<DateTime<Utc>>,
    /// The hour this key's checks are spread over: when the first frame of it was sent,
    /// and how many were sent in it so far. It is over an hour after it began, and the
    /// next frame sent begins a new one.
    spread: Option<(DateTime<Utc>, u32)>,
    /// Looks at this key in all, sent or found unchanged (the second look has its own
    /// interval).
    looks: u32,
    /// The last look: a frame sent, or one found unchanged.
    last_look_at: Option<DateTime<Utc>>,
    /// `none` answers in a row, and when the first [`CLEAN_TIMES_KEPT`] of them came.
    clean_streak: u32,
    clean_at: Vec<DateTime<Utc>>,
    signal_until: Option<DateTime<Utc>>,
    /// Difference hash and time of the last frame answered `none`.
    last_clean: Option<(u64, DateTime<Utc>)>,
    refusal_streak: u32,
    wait_until: Option<DateTime<Utc>>,
    recheck_at: Option<DateTime<Utc>>,
    exception_only_day: Option<String>,
    last_ask_card_at: Option<DateTime<Utc>>,
    /// When the key was last in front.
    seen_at: Option<DateTime<Utc>>,
}

impl KeyMemory {
    fn end_clean_streak(&mut self) {
        self.clean_streak = 0;
        self.clean_at.clear();
        self.last_clean = None;
    }

    /// Whether the relaxed interval of `cadence` applies at `now`: enough `none` answers
    /// in a row, and, where the cadence limits it, not longer ago than it allows.
    fn relaxed(&self, cadence: &Cadence, now: DateTime<Utc>) -> bool {
        let Some(needed) = cadence.relax_after_clean else {
            return false;
        };
        if self.clean_streak < needed {
            return false;
        }
        let since = usize::try_from(needed)
            .ok()
            .and_then(|needed| self.clean_at.get(needed.checked_sub(1)?));
        match (cadence.relax_for_secs, since) {
            (None, _) => true,
            (Some(limit), Some(since)) => (0..limit).contains(&(now - *since).num_seconds()),
            (Some(_), None) => false,
        }
    }

    /// Frames sent for this key in its present hour ([`KeyMemory::spread`]) at `now`; 0
    /// when that hour is over or seems to begin in the future (the clock was set back).
    fn spread_checks(&self, now: DateTime<Utc>) -> u32 {
        match self.spread {
            Some((since, checks)) if (0..HOUR_SECS).contains(&(now - since).num_seconds()) => {
                checks
            }
            _ => 0,
        }
    }

    /// Whether the one re-check "Não era isso" bought is waiting, and whether it is due.
    fn recheck(&self, now: DateTime<Utc>) -> Option<bool> {
        self.recheck_at
            .map(|at| now >= at || (at - now).num_seconds() > NOT_IT_RECHECK_SECS)
    }
}

impl SamplerMemory {
    /// Sets the number of checks already made today (from the day counters), so the daily
    /// cap survives a restart. `day` is the local `YYYY-MM-DD`.
    pub fn set_day_count(&mut self, day: &str, checks: u32) {
        self.day = Some((day.to_owned(), checks));
    }

    /// Whether the heightened state is on at `now`.
    pub fn heightened(&self, now: DateTime<Utc>) -> bool {
        still(self.heightened_until, now, HEIGHTENED_SECS)
    }

    /// Whether `key` has a remembered signal at `now` (a `suggestive`, `unsure` or refused
    /// answer in the last [`SIGNAL_MEMORY_SECS`]).
    pub fn has_signal(&self, key: u64, now: DateTime<Utc>) -> bool {
        self.keys
            .get(&key)
            .is_some_and(|memory| still(memory.signal_until, now, SIGNAL_MEMORY_SECS))
    }

    /// Until when vision's health is in backoff, if it is at `now`.
    pub fn degraded_until(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let longest = DEGRADED_STEPS_SECS[DEGRADED_STEPS_SECS.len() - 1];
        self.degraded_until
            .filter(|_| still(self.degraded_until, now, longest))
    }

    /// The provider rejected the key [`KEY_REJECTIONS_FOR_OFF`] times.
    pub fn key_rejected(&self) -> bool {
        self.key_rejections >= KEY_REJECTIONS_FOR_OFF
    }

    fn checks_today(&self, day: &str) -> u32 {
        match &self.day {
            Some((counted, checks)) if counted == day => *checks,
            _ => 0,
        }
    }

    /// Whether the hour's or the day's cap is reached at `now` under `prefs`.
    pub fn cap_reached(&self, prefs: &VisionPrefs, day: &str, now: DateTime<Utc>) -> bool {
        in_last_hour(&self.hour_checks, now) >= prefs.max_per_hour
            || self.checks_today(day) >= prefs.max_per_day
    }

    /// Whether "Não era isso" on `key` is, for the rest of local `day`, a delayed
    /// exception request (its independent re-check was positive again).
    pub fn exception_only(&self, key: u64, day: &str) -> bool {
        self.keys
            .get(&key)
            .is_some_and(|memory| memory.exception_only_day.as_deref() == Some(day))
    }

    /// Whether the soft card may be shown for `key` at `now` (at most once per
    /// [`ASK_CARD_EVERY_SECS`]).
    pub fn ask_card_allowed(&self, key: u64, now: DateTime<Utc>) -> bool {
        let shown = self
            .keys
            .get(&key)
            .and_then(|memory| memory.last_ask_card_at);
        // A card that seems to have been shown in the future does not hold the next one.
        !shown.is_some_and(|shown| (0..ASK_CARD_EVERY_SECS).contains(&(now - shown).num_seconds()))
    }

    /// Forgets everything.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Seconds `key` has been in front at `now`; 0 when another key is in front.
    fn dwell_secs(&self, key: u64, now: DateTime<Utc>) -> i64 {
        match self.front {
            Some((front, since)) if front == key => (now - since).num_seconds().max(0),
            _ => 0,
        }
    }
}

/// Whether the window described by `ctx` is looked at now (the tables of design sections
/// 4.2 and 4.3). `day` is the local day, for the daily cap.
pub fn decide(ctx: &SampleCtx, memory: &SamplerMemory, day: &str, now: DateTime<Utc>) -> Decision {
    let signal = ctx.local_signal() || memory.heightened(now) || memory.has_signal(ctx.key, now);
    if !ctx.is_candidate(signal) {
        return Decision::Skip(Skip::NotCandidate);
    }
    if ctx.excluded {
        return Decision::Skip(Skip::Excluded);
    }
    if !ctx.has_window_id {
        return Decision::Skip(Skip::NoWindowId);
    }
    if memory.degraded_until(now).is_some() || memory.key_rejected() {
        return Decision::Skip(Skip::Degraded);
    }
    let key = memory.keys.get(&ctx.key);

    // The re-check "Não era isso" bought comes next for this key, whatever the caps say.
    match key.and_then(|key| key.recheck(now)) {
        Some(true) => return Decision::Capture { signal: true },
        Some(false) => return Decision::Skip(Skip::NotDue),
        None => {}
    }
    if key.is_some_and(|key| still(key.wait_until, now, ERROR_WAIT_SECS)) {
        return Decision::Skip(Skip::KeyBackoff);
    }

    let cadence = ctx.cadence(signal);
    if memory.dwell_secs(ctx.key, now) < cadence.first_after_secs {
        return Decision::Skip(Skip::NotDue);
    }
    if let Some(key) = key {
        let interval = if key.relaxed(cadence, now) {
            cadence.relaxed_every_secs
        } else if cadence.doubles {
            spread_gap(cadence.every_secs, key.spread_checks(now))
        } else if key.looks == 1 {
            cadence.second_after_secs.unwrap_or(cadence.every_secs)
        } else {
            cadence.every_secs
        };
        // A look that seems to lie in the future (the clock was set back) does not delay.
        let waited = key
            .last_look_at
            .is_none_or(|last| !(0..interval).contains(&(now - last).num_seconds()));
        if !waited {
            return Decision::Skip(Skip::NotDue);
        }
        if in_last_hour(&key.checks, now) >= PER_KEY_PER_HOUR {
            return Decision::Skip(Skip::KeyCap);
        }
    }
    if in_last_hour(&memory.hour_checks, now) >= ctx.max_per_hour {
        return Decision::Skip(Skip::HourCap);
    }
    if memory.checks_today(day) >= ctx.max_per_day {
        return Decision::Skip(Skip::DayCap);
    }
    Decision::Capture { signal }
}

/// Whether a frame with this difference hash is the same picture as the last one answered
/// `none` for `key` (within [`DHASH_SAME_BITS`] bits, less than [`UNCHANGED_WINDOW_SECS`]
/// ago). Such a frame is not sent and not counted.
pub fn frame_unchanged(memory: &SamplerMemory, key: u64, dhash: u64, now: DateTime<Utc>) -> bool {
    memory
        .keys
        .get(&key)
        .and_then(|memory| memory.last_clean)
        .is_some_and(|(clean, at)| {
            (clean ^ dhash).count_ones() <= DHASH_SAME_BITS
                && (0..UNCHANGED_WINDOW_SECS).contains(&(now - at).num_seconds())
        })
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
    match stage {
        Stage::First => {
            if check.refused {
                return Outcome::Watch;
            }
            match (check.verdict, check.confidence) {
                (ExplicitVerdict::Explicit, Confidence::High) => Outcome::Positive,
                (ExplicitVerdict::Explicit, Confidence::Medium) => Outcome::Confirm,
                (ExplicitVerdict::Explicit, Confidence::Low) => Outcome::Unsure,
                (ExplicitVerdict::Suggestive | ExplicitVerdict::Unsure, _) => Outcome::Watch,
                (ExplicitVerdict::None, _) => Outcome::Clean,
            }
        }
        Stage::Second => match (check.refused, check.verdict, check.confidence) {
            (false, ExplicitVerdict::Explicit, Confidence::High | Confidence::Medium) => {
                Outcome::Positive
            }
            (
                _,
                ExplicitVerdict::Explicit
                | ExplicitVerdict::Suggestive
                | ExplicitVerdict::None
                | ExplicitVerdict::Unsure,
                _,
            ) => Outcome::Unsure,
        },
    }
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
    /// A frame was taken and [`frame_unchanged`] said it is the picture answered `none`
    /// before: nothing was sent and nothing is counted, and the interval starts again.
    Unchanged,
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
///
/// A `Positive` answer starts the heightened state by itself. The heightened state, any
/// answer other than `none` and any failure end the key's run of `none` answers, and with
/// it the relaxed interval.
pub fn record(
    memory: &mut SamplerMemory,
    key: u64,
    event: SampleEvent,
    day: &str,
    now: DateTime<Utc>,
) {
    match event {
        SampleEvent::Front => {
            let same = memory
                .front
                .is_some_and(|(front, since)| front == key && since <= now);
            if !same {
                memory.front = Some((key, now));
                if memory.keys.len() > MAX_KEYS {
                    memory.keys.retain(|_, entry| {
                        entry
                            .seen_at
                            .is_some_and(|seen| (now - seen).num_seconds().abs() < KEY_IDLE_SECS)
                    });
                }
            }
            memory.keys.entry(key).or_default().seen_at = Some(now);
        }
        SampleEvent::Checked {
            outcome,
            refused,
            dhash,
        } => {
            memory
                .hour_checks
                .retain(|time| (0..HOUR_SECS).contains(&(now - *time).num_seconds()));
            memory.hour_checks.push(now);
            let today = memory.checks_today(day);
            memory.day = Some((day.to_owned(), today.saturating_add(1)));
            memory.error_streak = 0;
            memory.degraded_until = None;
            memory.key_rejections = 0;

            let entry = memory.keys.entry(key).or_default();
            entry
                .checks
                .retain(|time| (0..HOUR_SECS).contains(&(now - *time).num_seconds()));
            entry.checks.push(now);
            entry.spread = Some(match (entry.spread, entry.spread_checks(now)) {
                (Some((since, _)), checks @ 1..) => (since, checks.saturating_add(1)),
                (_, _) => (now, 1),
            });
            entry.looks = entry.looks.saturating_add(1);
            entry.last_look_at = Some(now);
            let was_recheck = entry.recheck_at.take().is_some();
            entry.wait_until = None;
            entry.refusal_streak = if refused {
                entry.refusal_streak.saturating_add(1)
            } else {
                0
            };
            match outcome {
                Outcome::Clean => {
                    entry.clean_streak = entry.clean_streak.saturating_add(1);
                    if entry.clean_at.len() < CLEAN_TIMES_KEPT {
                        entry.clean_at.push(now);
                    }
                    entry.last_clean = Some((dhash, now));
                }
                Outcome::Positive | Outcome::Confirm | Outcome::Unsure | Outcome::Watch => {
                    entry.end_clean_streak();
                    entry.signal_until = Some(now + Duration::seconds(SIGNAL_MEMORY_SECS));
                }
            }
            if was_recheck {
                // Whatever the re-check answered, the key stays at signal cadence.
                entry.signal_until = Some(now + Duration::seconds(SIGNAL_MEMORY_SECS));
            }
            if outcome == Outcome::Positive {
                start_heightened(memory, now);
            }
        }
        SampleEvent::Unchanged => {
            let entry = memory.keys.entry(key).or_default();
            entry.looks = entry.looks.saturating_add(1);
            entry.last_look_at = Some(now);
        }
        SampleEvent::Failed(error) => {
            let entry = memory.keys.entry(key).or_default();
            entry.last_look_at = Some(now);
            entry.wait_until = Some(now + Duration::seconds(ERROR_WAIT_SECS));
            if entry.recheck_at.take().is_some() {
                // The re-check could not be made: the key stays at signal cadence.
                entry.signal_until = Some(now + Duration::seconds(SIGNAL_MEMORY_SECS));
            }
            entry.end_clean_streak();
            match error {
                CheckError::Transient => {
                    memory.error_streak = memory.error_streak.saturating_add(1);
                    if memory.error_streak >= ERRORS_FOR_DEGRADED {
                        let over = memory.error_streak - ERRORS_FOR_DEGRADED;
                        let step = usize::try_from(over)
                            .unwrap_or(usize::MAX)
                            .min(DEGRADED_STEPS_SECS.len() - 1);
                        memory.degraded_until =
                            Some(now + Duration::seconds(DEGRADED_STEPS_SECS[step]));
                    }
                }
                CheckError::KeyRejected => {
                    memory.key_rejections = memory.key_rejections.saturating_add(1);
                }
            }
        }
        SampleEvent::Heightened => start_heightened(memory, now),
        SampleEvent::NotIt => {
            memory.keys.entry(key).or_default().recheck_at =
                Some(now + Duration::seconds(NOT_IT_RECHECK_SECS));
        }
        SampleEvent::NotItConfirmed => {
            memory.keys.entry(key).or_default().exception_only_day = Some(day.to_owned());
        }
        SampleEvent::AskCardShown => {
            memory.keys.entry(key).or_default().last_ask_card_at = Some(now);
        }
    }
}

/// Starts the heightened state. It is a signal for every key, and a signal ends relaxing.
fn start_heightened(memory: &mut SamplerMemory, now: DateTime<Utc>) {
    memory.heightened_until = Some(now + Duration::seconds(HEIGHTENED_SECS));
    for entry in memory.keys.values_mut() {
        entry.end_clean_streak();
    }
}

/// Whether the soft card is due after a refusal: [`REFUSALS_FOR_CARD`] refusals in a row
/// on a key that also has a local signal.
pub fn refusal_card_due(memory: &SamplerMemory, key: u64, local_signal: bool) -> bool {
    local_signal
        && memory
            .keys
            .get(&key)
            .is_some_and(|entry| entry.refusal_streak >= REFUSALS_FOR_CARD)
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
pub fn not_it_on_block(source: BlockSource, vision_ready: bool, not_it_today: u32) -> NotItEffect {
    let by_rule = match source {
        BlockSource::Keyword | BlockSource::Tld => true,
        BlockSource::User | BlockSource::Learned | BlockSource::List => false,
    };
    if by_rule && vision_ready && not_it_today < MAX_SNOOZES_PER_DAY {
        NotItEffect::Snooze
    } else {
        NotItEffect::Request
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const DAY: &str = "2026-09-17";
    const KEY: u64 = 0xA11CE;

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap() + Duration::seconds(secs)
    }

    fn prefs() -> VisionPrefs {
        VisionPrefs::default()
    }

    /// A browser window with an id, on `verdict`.
    fn browser(verdict: Verdict) -> SampleCtx {
        SampleCtx {
            verdict: Some(verdict),
            has_window_id: true,
            ..SampleCtx::new(KEY, WindowKind::Browser, &prefs())
        }
    }

    fn plain() -> SampleCtx {
        browser(Verdict::Unknown(HostClass::Plain { signal: false }))
    }

    fn app(kind: WindowKind) -> SampleCtx {
        SampleCtx {
            has_window_id: true,
            ..SampleCtx::new(KEY, kind, &prefs())
        }
    }

    fn clean(dhash: u64) -> SampleEvent {
        SampleEvent::Checked {
            outcome: Outcome::Clean,
            refused: false,
            dhash,
        }
    }

    fn answered(outcome: Outcome) -> SampleEvent {
        SampleEvent::Checked {
            outcome,
            refused: false,
            dhash: 0,
        }
    }

    /// A memory in which `ctx`'s key has been in front long enough for any cadence.
    fn in_front(ctx: &SampleCtx) -> SamplerMemory {
        let mut memory = SamplerMemory::default();
        record(&mut memory, ctx.key, SampleEvent::Front, DAY, at(-3_600));
        memory
    }

    /// Whether `ctx` is a candidate: looked at once it has been in front for a while.
    fn candidate(ctx: &SampleCtx) -> bool {
        match decide(ctx, &in_front(ctx), DAY, at(0)) {
            Decision::Capture { .. } => true,
            Decision::Skip(Skip::NotCandidate) => false,
            other => panic!("unexpected decision {other:?}"),
        }
    }

    /// Runs one guard pass per second from `from` to `to` on `ctx`; every frame the check
    /// asks for is answered with `answer`. Returns the seconds at which frames were sent.
    fn simulate(
        memory: &mut SamplerMemory,
        ctx: &SampleCtx,
        from: i64,
        to: i64,
        answer: Outcome,
    ) -> Vec<i64> {
        let mut sent = Vec::new();
        for second in from..=to {
            record(memory, ctx.key, SampleEvent::Front, DAY, at(second));
            if let Decision::Capture { .. } = decide(ctx, memory, DAY, at(second)) {
                let event = SampleEvent::Checked {
                    outcome: answer,
                    refused: false,
                    // A different picture every time.
                    dhash: (second as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
                };
                record(memory, ctx.key, event, DAY, at(second));
                sent.push(second);
            }
        }
        sent
    }

    // -- 4.2: candidates -------------------------------------------------------------------

    #[test]
    fn window_kinds() {
        // A supported browser, with and without a URL.
        assert_eq!(
            window_kind("com.google.Chrome", "Google Chrome", true, true),
            WindowKind::Browser
        );
        assert_eq!(
            window_kind("com.google.Chrome", "Google Chrome", true, false),
            WindowKind::UnreadableBrowser
        );
        // A browser the resolver does not support here.
        assert_eq!(
            window_kind("org.mozilla.firefox", "Firefox", false, false),
            WindowKind::UnreadableBrowser
        );
        assert_eq!(
            window_kind("tor browser", "Tor Browser", false, false),
            WindowKind::UnreadableBrowser
        );
        assert_eq!(
            window_kind("x", "LibreWolf", false, false),
            WindowKind::UnreadableBrowser
        );
        // The same app where the resolver does support it is a browser first.
        assert_eq!(
            window_kind("firefox", "Firefox", true, true),
            WindowKind::Browser
        );
        assert_eq!(
            window_kind("org.videolan.vlc", "VLC", false, false),
            WindowKind::Media
        );
        assert_eq!(window_kind("MPV", "mpv", false, false), WindowKind::Media);
        assert_eq!(
            window_kind("com.apple.Preview", "Preview", false, false),
            WindowKind::Media
        );
        assert_eq!(
            window_kind("com.tinyspeck.slackmacgap", "Slack", false, false),
            WindowKind::Messaging
        );
        assert_eq!(
            window_kind("com.1password.1password", "1Password", false, false),
            WindowKind::NeverCapture
        );
        assert_eq!(
            window_kind("keepassxc", "KeePassXC", false, false),
            WindowKind::NeverCapture
        );
        assert_eq!(
            window_kind("com.microsoft.VSCode", "Code", false, false),
            WindowKind::Other
        );
        assert_eq!(window_kind("", "", false, false), WindowKind::Other);
    }

    #[test]
    fn both_products_are_never_captured_whatever_else_they_look_like() {
        for product in ubiqx_core::product::ALL {
            assert_eq!(
                window_kind(product.bundle_id, product.display_name, true, true),
                WindowKind::NeverCapture
            );
            for stem in product.exe_stems {
                assert_eq!(
                    window_kind(stem, "", false, false),
                    WindowKind::NeverCapture
                );
            }
        }
    }

    #[test]
    fn the_app_lists_are_lower_case_and_do_not_overlap() {
        for list in [MEDIA_APPS, OTHER_BROWSERS, NEVER_CAPTURE_APPS] {
            for entry in list {
                assert_eq!(*entry, entry.to_lowercase(), "{entry}");
            }
        }
        for entry in NEVER_CAPTURE_APPS {
            assert!(
                !MEDIA_APPS.contains(entry) && !OTHER_BROWSERS.contains(entry),
                "{entry}"
            );
        }
        for entry in MEDIA_APPS {
            assert!(!OTHER_BROWSERS.contains(entry), "{entry}");
        }
    }

    #[test]
    fn decided_hosts_are_never_candidates() {
        for source in [
            BlockSource::User,
            BlockSource::Learned,
            BlockSource::List,
            BlockSource::Tld,
            BlockSource::Keyword,
        ] {
            assert!(!candidate(&browser(Verdict::Blocked(source))), "{source:?}");
        }
        assert!(!candidate(&browser(Verdict::Allowed(AllowSource::User))));
        assert!(!candidate(&browser(Verdict::Allowed(AllowSource::BuiltIn))));
        // Not even with every signal there is.
        let mut loud = browser(Verdict::Allowed(AllowSource::User));
        loud.title_signal = true;
        loud.path_signal = true;
        loud.private_window = true;
        assert!(!candidate(&loud));
    }

    #[test]
    fn never_capture_and_local_hosts_are_never_candidates() {
        for class in [HostClass::NeverCapture, HostClass::Local] {
            let mut ctx = browser(Verdict::Unknown(class));
            assert!(!candidate(&ctx), "{class:?}");
            ctx.title_signal = true;
            ctx.path_signal = true;
            assert!(!candidate(&ctx), "{class:?} with signals");
            let mut memory = in_front(&ctx);
            record(&mut memory, 1, SampleEvent::Heightened, DAY, at(-10));
            assert_eq!(
                decide(&ctx, &memory, DAY, at(0)),
                Decision::Skip(Skip::NotCandidate)
            );
        }
    }

    #[test]
    fn a_popular_host_is_a_candidate_only_with_a_signal() {
        let popular = browser(Verdict::Unknown(HostClass::Popular));
        assert!(!candidate(&popular));
        let mut with_title = popular.clone();
        with_title.title_signal = true;
        assert_eq!(
            decide(&with_title, &in_front(&with_title), DAY, at(0)),
            Decision::Capture { signal: true }
        );
        let mut with_path = popular.clone();
        with_path.path_signal = true;
        assert!(candidate(&with_path));
        let mut private = popular.clone();
        private.private_window = true;
        assert!(candidate(&private));
        // The heightened state is a signal for every key …
        let mut memory = in_front(&popular);
        record(&mut memory, 99, SampleEvent::Heightened, DAY, at(-60));
        assert_eq!(
            decide(&popular, &memory, DAY, at(0)),
            Decision::Capture { signal: true }
        );
        assert_eq!(
            decide(&popular, &memory, DAY, at(841)),
            Decision::Skip(Skip::NotCandidate)
        );
        // … and so is a remembered answer for this key.
        let mut memory = in_front(&popular);
        record(&mut memory, KEY, answered(Outcome::Watch), DAY, at(-600));
        assert_eq!(
            decide(&popular, &memory, DAY, at(0)),
            Decision::Capture { signal: true }
        );
        assert_eq!(
            decide(&popular, &memory, DAY, at(1_201)),
            Decision::Skip(Skip::NotCandidate)
        );
    }

    #[test]
    fn a_mixed_host_is_a_candidate_at_signal_cadence() {
        let mut mixed = browser(Verdict::Unknown(HostClass::Mixed));
        assert_eq!(
            decide(&mixed, &in_front(&mixed), DAY, at(0)),
            Decision::Capture { signal: true }
        );
        // Not governed by the "unknown sites" switch.
        mixed.unknown_sites = false;
        assert!(candidate(&mixed));
    }

    #[test]
    fn a_plain_host_is_a_candidate_when_unknown_sites_are_on() {
        assert_eq!(
            decide(&plain(), &in_front(&plain()), DAY, at(0)),
            Decision::Capture { signal: false }
        );
        let with_word = browser(Verdict::Unknown(HostClass::Plain { signal: true }));
        assert_eq!(
            decide(&with_word, &in_front(&with_word), DAY, at(0)),
            Decision::Capture { signal: true }
        );
        let mut off = plain();
        off.unknown_sites = false;
        assert!(!candidate(&off));
        let mut off_with_word = with_word;
        off_with_word.unknown_sites = false;
        assert!(!candidate(&off_with_word));
    }

    #[test]
    fn a_snoozed_keyword_block_is_still_looked_at_at_signal_cadence() {
        let snoozed = browser(Verdict::Allowed(AllowSource::Snooze));
        assert_eq!(
            decide(&snoozed, &in_front(&snoozed), DAY, at(0)),
            Decision::Capture { signal: true }
        );
    }

    #[test]
    fn a_browser_whose_url_cannot_be_read_is_a_candidate_when_unknown_sites_are_on() {
        let mut unreadable = app(WindowKind::UnreadableBrowser);
        assert_eq!(
            decide(&unreadable, &in_front(&unreadable), DAY, at(0)),
            Decision::Capture { signal: false }
        );
        unreadable.unknown_sites = false;
        assert!(!candidate(&unreadable));
        // A URL that was read but not judged is treated the same way.
        let mut unjudged = app(WindowKind::Browser);
        assert!(candidate(&unjudged));
        unjudged.unknown_sites = false;
        assert!(!candidate(&unjudged));
    }

    #[test]
    fn a_private_window_follows_its_host_and_its_switch() {
        let mut private = plain();
        private.private_window = true;
        // The private title is a signal.
        assert_eq!(
            decide(&private, &in_front(&private), DAY, at(0)),
            Decision::Capture { signal: true }
        );
        private.private_windows = false;
        assert!(!candidate(&private));
        let mut unreadable = app(WindowKind::UnreadableBrowser);
        unreadable.private_window = true;
        assert!(candidate(&unreadable));
        unreadable.private_windows = false;
        assert!(!candidate(&unreadable));
        // "As its host says": a private window on a listed or allowed host is not looked at.
        let mut allowed = browser(Verdict::Allowed(AllowSource::User));
        allowed.private_window = true;
        assert!(!candidate(&allowed));
    }

    #[test]
    fn media_and_messaging_apps_follow_their_switches() {
        let mut media = app(WindowKind::Media);
        assert!(candidate(&media));
        media.media_apps = false;
        assert!(!candidate(&media));
        // Messaging is off by default.
        let mut messaging = app(WindowKind::Messaging);
        assert!(!prefs().messaging_apps);
        assert!(!candidate(&messaging));
        messaging.messaging_apps = true;
        assert!(candidate(&messaging));
    }

    #[test]
    fn other_apps_and_never_capture_apps_are_never_candidates() {
        for kind in [WindowKind::Other, WindowKind::NeverCapture] {
            let mut ctx = app(kind);
            ctx.title_signal = true;
            ctx.messaging_apps = true;
            assert!(!candidate(&ctx), "{kind:?}");
        }
    }

    #[test]
    fn a_window_without_an_id_is_never_captured() {
        let mut ctx = plain();
        ctx.has_window_id = false;
        assert_eq!(
            decide(&ctx, &in_front(&ctx), DAY, at(0)),
            Decision::Skip(Skip::NoWindowId)
        );
    }

    #[test]
    fn an_effective_exclusion_is_never_captured() {
        let mut ctx = browser(Verdict::Unknown(HostClass::Mixed));
        ctx.excluded = true;
        ctx.title_signal = true;
        assert_eq!(
            decide(&ctx, &in_front(&ctx), DAY, at(0)),
            Decision::Skip(Skip::Excluded)
        );
    }

    #[test]
    fn the_context_has_no_idle_input_and_names_nothing() {
        // Every field, by name: adding one does not compile until it is listed here, and
        // whoever lists it reads the two rules below.
        let SampleCtx {
            key: _,
            kind: _,
            verdict: _,
            has_window_id: _,
            private_window: _,
            title_signal: _,
            path_signal: _,
            excluded: _,
            unknown_sites: _,
            media_apps: _,
            messaging_apps: _,
            private_windows: _,
            max_per_hour: _,
            max_per_day: _,
        } = plain();
        // 1. No idle time: watching video produces no input, so idleness must not stop a
        //    check. 2. No URL, title or application name: the context carries verdicts.
        let printed = format!("{:?}", plain()).to_lowercase();
        for word in [
            "idle", "input", "url", "title:", "app_name", "app_id", "host:",
        ] {
            assert!(
                !printed.contains(word),
                "SampleCtx has a field about `{word}`: {printed}"
            );
        }
        // And a window that got no input for hours is looked at like any other.
        let ctx = app(WindowKind::Media);
        let mut memory = SamplerMemory::default();
        let sent = simulate(&mut memory, &ctx, 0, 7_200, Outcome::Clean);
        assert!(sent.len() >= 10, "{sent:?}");
        assert!(*sent.last().unwrap() > 6_500, "{sent:?}");
    }

    // -- 4.3: cadence ----------------------------------------------------------------------

    #[test]
    fn cadence_without_a_signal() {
        // 15 s in front, 90 s later, then every 300 s. The first frame is sent; the later
        // ones show the same picture, so nothing here relaxes and nothing is capped.
        let ctx = plain();
        let mut memory = SamplerMemory::default();
        let mut looks = Vec::new();
        for second in 0..=1_400 {
            record(&mut memory, KEY, SampleEvent::Front, DAY, at(second));
            if let Decision::Capture { signal } = decide(&ctx, &memory, DAY, at(second)) {
                assert!(!signal);
                let event = if looks.is_empty() {
                    clean(7)
                } else {
                    SampleEvent::Unchanged
                };
                record(&mut memory, KEY, event, DAY, at(second));
                looks.push(second);
            }
        }
        assert_eq!(looks, vec![15, 105, 405, 705, 1_005, 1_305]);
    }

    #[test]
    fn dwell_time_starts_again_when_the_window_changes() {
        let ctx = plain();
        let mut memory = SamplerMemory::default();
        assert_eq!(
            decide(&ctx, &memory, DAY, at(0)),
            Decision::Skip(Skip::NotDue),
            "never in front"
        );
        record(&mut memory, KEY, SampleEvent::Front, DAY, at(0));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(14)),
            Decision::Skip(Skip::NotDue)
        );
        // Another window comes to the front and goes.
        record(&mut memory, 2, SampleEvent::Front, DAY, at(14));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(15)),
            Decision::Skip(Skip::NotDue)
        );
        record(&mut memory, KEY, SampleEvent::Front, DAY, at(16));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(30)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(31)),
            Decision::Capture { signal: false }
        );
    }

    #[test]
    fn two_clean_answers_relax_the_cadence() {
        let ctx = plain();
        let mut memory = SamplerMemory::default();
        // 15 s, 90 s later, and from the second `none` on every 600 s. The seventh look
        // would be due at 3105, but six an hour is the most one key gets.
        let sent = simulate(&mut memory, &ctx, 0, 3_700, Outcome::Clean);
        assert_eq!(sent, vec![15, 105, 705, 1_305, 1_905, 2_505, 3_615]);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_700)),
            Decision::Skip(Skip::NotDue)
        );
    }

    #[test]
    fn relaxing_lasts_sixty_minutes() {
        let ctx = plain();
        let mut memory = in_front(&ctx);
        record(&mut memory, KEY, clean(1), DAY, at(15));
        record(&mut memory, KEY, clean(2), DAY, at(105));
        // Relaxed from the second answer on: 300 s is not enough, 600 s is.
        assert_eq!(
            decide(&ctx, &memory, DAY, at(405)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(704)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(705)),
            Decision::Capture { signal: false }
        );
        // A look late in the hour: still relaxed just before the hour is over …
        record(&mut memory, KEY, clean(3), DAY, at(3_400));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_704)),
            Decision::Skip(Skip::NotDue)
        );
        // … and back to every 300 s once it is (105 + 3600).
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_705)),
            Decision::Capture { signal: false }
        );
    }

    #[test]
    fn a_signal_cancels_relaxing() {
        let relaxed = |memory: &mut SamplerMemory| {
            record(memory, KEY, clean(1), DAY, at(15));
            record(memory, KEY, clean(2), DAY, at(105));
            assert_eq!(
                decide(&plain(), memory, DAY, at(500)),
                Decision::Skip(Skip::NotDue)
            );
        };
        // A local signal: the signal cadence applies at once, with the wait two checks in
        // the key's hour give (90 s after the last look, not 600).
        let mut memory = in_front(&plain());
        relaxed(&mut memory);
        let mut titled = plain();
        titled.title_signal = true;
        assert_eq!(
            decide(&titled, &memory, DAY, at(194)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&titled, &memory, DAY, at(195)),
            Decision::Capture { signal: true }
        );

        // An answer that is not `none` ends the run; when its 30 minutes are over the key
        // is back at the plain cadence, not the relaxed one.
        let mut memory = in_front(&plain());
        relaxed(&mut memory);
        record(&mut memory, KEY, answered(Outcome::Watch), DAY, at(705));
        // It was the key's third check of the hour: the next look is three minutes later.
        assert_eq!(
            decide(&plain(), &memory, DAY, at(884)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&plain(), &memory, DAY, at(885)),
            Decision::Capture { signal: true }
        );
        assert_eq!(
            decide(&plain(), &memory, DAY, at(705 + 1_800 + 1)),
            Decision::Capture { signal: false }
        );
        record(&mut memory, KEY, clean(3), DAY, at(2_506));
        assert_eq!(
            decide(&plain(), &memory, DAY, at(2_805)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&plain(), &memory, DAY, at(2_806)),
            Decision::Capture { signal: false }
        );

        // The heightened state ends every key's run.
        let mut memory = in_front(&plain());
        relaxed(&mut memory);
        record(&mut memory, 42, SampleEvent::Heightened, DAY, at(110));
        assert_eq!(
            decide(&plain(), &memory, DAY, at(194)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&plain(), &memory, DAY, at(195)),
            Decision::Capture { signal: true }
        );
        // 15 minutes later it is over, and the key is not relaxed any more.
        assert_eq!(
            decide(&plain(), &memory, DAY, at(1_011)),
            Decision::Capture { signal: false }
        );
        record(&mut memory, KEY, clean(3), DAY, at(1_011));
        assert_eq!(
            decide(&plain(), &memory, DAY, at(1_311)),
            Decision::Capture { signal: false }
        );
    }

    #[test]
    fn spread_gap_doubles_from_the_second_check_and_stays_within_the_hour() {
        let gaps: Vec<i64> = (0..=9).map(|checks| spread_gap(45, checks)).collect();
        // 45 s, 90 s, 3 min, 6 min, 12 min, 24 min, 48 min, then the hour.
        assert_eq!(
            gaps,
            vec![45, 45, 90, 180, 360, 720, 1_440, 2_880, 3_600, 3_600]
        );
        assert_eq!(spread_gap(45, 64), 3_600);
        assert_eq!(spread_gap(45, 65), 3_600);
        assert_eq!(spread_gap(45, u32::MAX), 3_600);
        assert_eq!(spread_gap(i64::MAX, 3), 3_600);
        assert_eq!(spread_gap(300, 2), 600);
        assert_eq!(spread_gap(0, 5), 0);
    }

    #[test]
    fn cadence_with_a_signal_spreads_six_checks_over_the_hour_and_does_not_relax() {
        // 5 s in front, then 45 s, 90 s, 3 min, 6 min and 12 min apart, whatever the
        // answers are: `none` every time relaxes nothing.
        for answer in [Outcome::Clean, Outcome::Watch] {
            let ctx = browser(Verdict::Unknown(HostClass::Mixed));
            let mut memory = SamplerMemory::default();
            let sent = simulate(&mut memory, &ctx, 0, 3_600, answer);
            assert_eq!(sent, vec![5, 50, 140, 320, 680, 1_400], "{answer:?}");
        }
        // Only the signal cadence doubles.
        let doubling: Vec<bool> = [
            CADENCE_NO_SIGNAL,
            CADENCE_SIGNAL,
            CADENCE_MEDIA,
            CADENCE_MESSAGING,
        ]
        .iter()
        .map(|cadence| cadence.doubles)
        .collect();
        assert_eq!(doubling, vec![false, true, false, false]);
    }

    #[test]
    fn the_spread_starts_again_when_the_hour_of_the_key_rolls() {
        // A standing signal for three hours: the same six checks in each, an hour apart,
        // counted from the first check of the hour.
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = SamplerMemory::default();
        let sent = simulate(&mut memory, &ctx, 0, 10_800, Outcome::Clean);
        let hour = [5, 50, 140, 320, 680, 1_400];
        let expected: Vec<i64> = (0..3)
            .flat_map(|n| hour.iter().map(move |second| second + n * 3_600))
            .collect();
        assert_eq!(sent, expected);
        // The cost is the cap's: never more than six of them in any hour.
        for (n, first) in sent.iter().enumerate() {
            let within = sent[n..]
                .iter()
                .filter(|second| *second - first < 3_600)
                .count();
            assert!(within <= 6, "{within} checks in the hour from {first}");
        }
    }

    #[test]
    fn a_new_hour_of_the_key_resets_the_gap_but_not_the_cap() {
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = in_front(&ctx);
        let mut picture = 0u64;
        let mut check = |memory: &mut SamplerMemory, second: i64| {
            picture += 0x0101_0101_0101_0101;
            record(memory, KEY, clean(picture), DAY, at(second));
        };
        // The key's hour begins with its first check, at 5.
        for second in [5, 50, 140] {
            check(&mut memory, second);
        }
        assert_eq!(
            decide(&ctx, &memory, DAY, at(319)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(320)),
            Decision::Capture { signal: true }
        );
        // The window was elsewhere for most of the hour. Its fourth and fifth checks come
        // late, and the wait after the fifth is 12 minutes …
        check(&mut memory, 3_000);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_359)),
            Decision::Skip(Skip::NotDue)
        );
        check(&mut memory, 3_590);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_604)),
            Decision::Skip(Skip::NotDue)
        );
        // … until the hour that began at 5 is over: the wait is 45 s again, counted from
        // the last look.
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_634)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_635)),
            Decision::Capture { signal: true }
        );
        // The check at 3635 begins the next hour of the key: 45 s, then 90 s.
        check(&mut memory, 3_635);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_679)),
            Decision::Skip(Skip::NotDue)
        );
        check(&mut memory, 3_680);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_769)),
            Decision::Skip(Skip::NotDue)
        );
        check(&mut memory, 3_770);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_949)),
            Decision::Skip(Skip::NotDue)
        );
        // The check at 3950 is the sixth since 3000 (3000, 3590, 3635, 3680, 3770 and this
        // one). When its 6 minutes are over, the cap of six in any hour holds, new hour of
        // the key or not, until the check at 3000 is an hour old.
        check(&mut memory, 3_950);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(4_309)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(4_310)),
            Decision::Skip(Skip::KeyCap)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(6_599)),
            Decision::Skip(Skip::KeyCap)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(6_600)),
            Decision::Capture { signal: true }
        );
    }

    #[test]
    fn a_frame_that_was_not_sent_does_not_lengthen_the_gap() {
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = in_front(&ctx);
        record(&mut memory, KEY, clean(1), DAY, at(5));
        record(&mut memory, KEY, clean(2), DAY, at(50));
        // Two checks: the wait is 90 s. Looks that found the same picture, and a call
        // that failed, start that wait again and leave it at 90 s.
        record(&mut memory, KEY, SampleEvent::Unchanged, DAY, at(140));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(229)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(230)),
            Decision::Capture { signal: true }
        );
        record(&mut memory, KEY, SampleEvent::Unchanged, DAY, at(230));
        record(
            &mut memory,
            KEY,
            SampleEvent::Failed(CheckError::Transient),
            DAY,
            at(320),
        );
        // The failed call waits its own 300 s, which is longer than the 90.
        assert_eq!(
            decide(&ctx, &memory, DAY, at(619)),
            Decision::Skip(Skip::KeyBackoff)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(620)),
            Decision::Capture { signal: true }
        );
        // The third check is what makes it 3 minutes.
        record(&mut memory, KEY, clean(3), DAY, at(620));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(799)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(800)),
            Decision::Capture { signal: true }
        );
    }

    #[test]
    fn a_clock_set_back_starts_the_spread_again() {
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = in_front(&ctx);
        for (n, second) in [10_000, 10_045, 10_135, 10_315].into_iter().enumerate() {
            record(&mut memory, KEY, clean(n as u64 + 1), DAY, at(second));
        }
        assert_eq!(
            decide(&ctx, &memory, DAY, at(10_674)),
            Decision::Skip(Skip::NotDue),
            "four checks: 6 minutes"
        );
        // The clock goes back by hours. The looks seem to lie in the future and do not
        // delay, and the hour they were counted in is not the present one.
        assert_eq!(
            decide(&ctx, &memory, DAY, at(100)),
            Decision::Capture { signal: true }
        );
        record(&mut memory, KEY, clean(9), DAY, at(100));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(144)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(145)),
            Decision::Capture { signal: true }
        );
        // Set forward by a day: the hour of the key is long over.
        record(&mut memory, KEY, clean(10), DAY, at(145));
        record(&mut memory, KEY, SampleEvent::Front, DAY, at(90_000));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(90_000)),
            Decision::Capture { signal: true }
        );
        record(&mut memory, KEY, clean(11), DAY, at(90_000));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(90_044)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(90_045)),
            Decision::Capture { signal: true }
        );
    }

    #[test]
    fn cadence_of_a_media_app() {
        let ctx = app(WindowKind::Media);
        let mut memory = SamplerMemory::default();
        // 20 s, every 180 s, and every 600 s after three `none` in a row, with no end
        // (the gap after 2180 is the cap of six an hour per key).
        let sent = simulate(&mut memory, &ctx, 0, 6_000, Outcome::Clean);
        assert_eq!(
            sent,
            vec![20, 200, 380, 980, 1_580, 2_180, 3_620, 4_220, 4_820, 5_420]
        );
        // Relaxed without an end: 599 s after a look, hours later, it is still not due.
        let mut memory = in_front(&ctx);
        for (n, second) in [20, 200, 380, 20_000].into_iter().enumerate() {
            record(&mut memory, KEY, clean(n as u64), DAY, at(second));
        }
        assert_eq!(
            decide(&ctx, &memory, DAY, at(20_599)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(20_600)),
            Decision::Capture { signal: false }
        );
        // With a signal (a block-grade word in the title) it is looked at like one.
        let mut titled = app(WindowKind::Media);
        titled.title_signal = true;
        let mut memory = SamplerMemory::default();
        assert_eq!(
            simulate(&mut memory, &titled, 0, 150, Outcome::Clean),
            vec![5, 50, 140]
        );
    }

    #[test]
    fn cadence_of_a_messaging_app_never_relaxes_or_speeds_up() {
        let mut ctx = app(WindowKind::Messaging);
        ctx.messaging_apps = true;
        let mut memory = SamplerMemory::default();
        let sent = simulate(&mut memory, &ctx, 0, 1_600, Outcome::Clean);
        assert_eq!(sent, vec![30, 330, 630, 930, 1_230, 1_530]);
        // Not even with a signal: those are other people's messages.
        ctx.title_signal = true;
        let mut memory = SamplerMemory::default();
        record(&mut memory, 5, SampleEvent::Heightened, DAY, at(0));
        assert_eq!(
            simulate(&mut memory, &ctx, 0, 400, Outcome::Clean),
            vec![30, 330]
        );
    }

    // -- 4.3: caps -------------------------------------------------------------------------

    #[test]
    fn six_checks_per_key_per_hour() {
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = SamplerMemory::default();
        let sent = simulate(&mut memory, &ctx, 0, 3_700, Outcome::Clean);
        // Six, further and further apart, then nothing until the first one is an hour old.
        assert_eq!(sent, vec![5, 50, 140, 320, 680, 1_400, 3_605, 3_650]);
        let mut capped = SamplerMemory::default();
        simulate(&mut capped, &ctx, 0, 1_500, Outcome::Clean);
        // A seventh would wait 24 minutes after the sixth; when that is over it is the
        // cap that holds it.
        assert_eq!(
            decide(&ctx, &capped, DAY, at(2_839)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &capped, DAY, at(2_840)),
            Decision::Skip(Skip::KeyCap)
        );
        assert_eq!(
            decide(&ctx, &capped, DAY, at(3_604)),
            Decision::Skip(Skip::KeyCap)
        );
        assert_eq!(
            decide(&ctx, &capped, DAY, at(3_605)),
            Decision::Capture { signal: true }
        );
        // Another key is not held back by this one.
        let other = SampleCtx {
            key: 77,
            ..ctx.clone()
        };
        record(&mut capped, 77, SampleEvent::Front, DAY, at(300));
        assert_eq!(
            decide(&other, &capped, DAY, at(305)),
            Decision::Capture { signal: true }
        );
    }

    #[test]
    fn the_hourly_cap_counts_every_key_and_slides_with_the_hour() {
        let mut limited = prefs();
        limited.max_per_hour = 3;
        let mut memory = SamplerMemory::default();
        for (key, second) in [(1u64, 0), (2, 600), (3, 1_200)] {
            record(&mut memory, key, clean(key), DAY, at(second));
        }
        let ctx = SampleCtx {
            has_window_id: true,
            verdict: Some(Verdict::Unknown(HostClass::Plain { signal: false })),
            ..SampleCtx::new(4, WindowKind::Browser, &limited)
        };
        record(&mut memory, 4, SampleEvent::Front, DAY, at(1_200));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(1_300)),
            Decision::Skip(Skip::HourCap)
        );
        assert!(memory.cap_reached(&limited, DAY, at(1_300)));
        assert!(
            !memory.cap_reached(&prefs(), DAY, at(1_300)),
            "20 an hour is not reached"
        );
        // Across the hour boundary: the first check is an hour old at 3600.
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_599)),
            Decision::Skip(Skip::HourCap)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(3_600)),
            Decision::Capture { signal: false }
        );
        assert!(!memory.cap_reached(&limited, DAY, at(3_600)));
        // With the user's cap raised the same memory allows it at once.
        let roomy = SampleCtx {
            max_per_hour: 4,
            ..ctx.clone()
        };
        assert_eq!(
            decide(&roomy, &memory, DAY, at(1_300)),
            Decision::Capture { signal: false }
        );
    }

    #[test]
    fn the_daily_cap_survives_a_restart_and_ends_with_the_day() {
        let mut limited = prefs();
        limited.max_per_day = 5;
        let ctx = SampleCtx {
            has_window_id: true,
            verdict: Some(Verdict::Unknown(HostClass::Plain { signal: false })),
            ..SampleCtx::new(KEY, WindowKind::Browser, &limited)
        };
        // Four checks were counted before the app restarted.
        let mut memory = in_front(&ctx);
        memory.set_day_count(DAY, 4);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(0)),
            Decision::Capture { signal: false }
        );
        assert!(!memory.cap_reached(&limited, DAY, at(0)));
        record(&mut memory, 9, clean(9), DAY, at(0));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(1)),
            Decision::Skip(Skip::DayCap)
        );
        assert!(memory.cap_reached(&limited, DAY, at(1)));
        // Midnight: the next local day starts at zero.
        let next_day = "2026-09-18";
        assert_eq!(
            decide(&ctx, &memory, next_day, at(32_400)),
            Decision::Capture { signal: false }
        );
        assert!(!memory.cap_reached(&limited, next_day, at(32_400)));
        record(&mut memory, 9, clean(9), next_day, at(32_400));
        assert!(!memory.cap_reached(&limited, next_day, at(32_401)));
        assert!(
            !memory.cap_reached(&limited, DAY, at(32_401)),
            "yesterday's count is gone"
        );
    }

    #[test]
    fn a_failed_call_and_an_unchanged_frame_are_not_counted_against_the_caps() {
        let mut limited = prefs();
        limited.max_per_hour = 1;
        limited.max_per_day = 1;
        let mut memory = SamplerMemory::default();
        record(
            &mut memory,
            1,
            SampleEvent::Failed(CheckError::Transient),
            DAY,
            at(0),
        );
        record(&mut memory, 2, SampleEvent::Unchanged, DAY, at(0));
        assert!(!memory.cap_reached(&limited, DAY, at(1)));
        record(&mut memory, 3, clean(3), DAY, at(1));
        assert!(memory.cap_reached(&limited, DAY, at(2)));
    }

    // -- 4.3: unchanged frame --------------------------------------------------------------

    #[test]
    fn a_frame_like_the_last_clean_one_is_not_sent() {
        let mut memory = SamplerMemory::default();
        let picture = 0xF0F0_F0F0_1234_5678u64;
        assert!(
            !frame_unchanged(&memory, KEY, picture, at(0)),
            "nothing was answered yet"
        );
        record(&mut memory, KEY, clean(picture), DAY, at(0));
        assert!(frame_unchanged(&memory, KEY, picture, at(1)));
        assert!(
            frame_unchanged(&memory, KEY, picture ^ 0b1111, at(599)),
            "4 bits apart"
        );
        assert!(
            !frame_unchanged(&memory, KEY, picture ^ 0b1_1111, at(1)),
            "5 bits apart"
        );
        assert!(
            !frame_unchanged(&memory, KEY, picture, at(600)),
            "10 minutes later"
        );
        assert!(
            !frame_unchanged(&memory, KEY + 1, picture, at(1)),
            "another key"
        );
        assert!(
            !frame_unchanged(&memory, KEY, picture, at(-5)),
            "a clock set back"
        );
    }

    #[test]
    fn only_a_clean_answer_is_remembered_as_a_picture() {
        for outcome in [
            Outcome::Watch,
            Outcome::Unsure,
            Outcome::Confirm,
            Outcome::Positive,
        ] {
            let mut memory = SamplerMemory::default();
            record(&mut memory, KEY, clean(42), DAY, at(0));
            record(
                &mut memory,
                KEY,
                SampleEvent::Checked {
                    outcome,
                    refused: false,
                    dhash: 42,
                },
                DAY,
                at(10),
            );
            assert!(!frame_unchanged(&memory, KEY, 42, at(11)), "{outcome:?}");
        }
        let mut memory = SamplerMemory::default();
        record(&mut memory, KEY, clean(42), DAY, at(0));
        record(
            &mut memory,
            KEY,
            SampleEvent::Failed(CheckError::Transient),
            DAY,
            at(10),
        );
        assert!(!frame_unchanged(&memory, KEY, 42, at(11)));
    }

    #[test]
    fn an_unchanged_frame_restarts_the_interval_without_a_check() {
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = in_front(&ctx);
        record(&mut memory, KEY, clean(42), DAY, at(0));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(45)),
            Decision::Capture { signal: true }
        );
        assert!(frame_unchanged(&memory, KEY, 42, at(45)));
        record(&mut memory, KEY, SampleEvent::Unchanged, DAY, at(45));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(89)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(90)),
            Decision::Capture { signal: true }
        );
        // Ten skipped frames later the key has still used one of its six checks.
        for n in 2..12 {
            record(&mut memory, KEY, SampleEvent::Unchanged, DAY, at(45 * n));
        }
        assert_eq!(
            decide(&ctx, &memory, DAY, at(45 * 12)),
            Decision::Capture { signal: true }
        );
    }

    // -- 4.3: errors -----------------------------------------------------------------------

    #[test]
    fn a_failed_call_makes_the_key_wait() {
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = in_front(&ctx);
        record(
            &mut memory,
            KEY,
            SampleEvent::Failed(CheckError::Transient),
            DAY,
            at(0),
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(1)),
            Decision::Skip(Skip::KeyBackoff)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(299)),
            Decision::Skip(Skip::KeyBackoff)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(300)),
            Decision::Capture { signal: true }
        );
        // Another key is not held back, and one failure does not degrade vision.
        let other = SampleCtx {
            key: 2,
            ..ctx.clone()
        };
        record(&mut memory, 2, SampleEvent::Front, DAY, at(1));
        assert_eq!(
            decide(&other, &memory, DAY, at(10)),
            Decision::Capture { signal: true }
        );
        assert_eq!(memory.degraded_until(at(10)), None);
    }

    #[test]
    fn three_failures_in_a_row_degrade_vision_in_steps() {
        let ctx = plain();
        let mut memory = in_front(&ctx);
        let fail = |memory: &mut SamplerMemory, key: u64, second: i64| {
            record(
                memory,
                key,
                SampleEvent::Failed(CheckError::Transient),
                DAY,
                at(second),
            );
        };
        fail(&mut memory, 1, 0);
        fail(&mut memory, 2, 10);
        assert_eq!(memory.degraded_until(at(10)), None);
        fail(&mut memory, 3, 20);
        assert_eq!(memory.degraded_until(at(21)), Some(at(80)), "1 minute");
        assert_eq!(
            decide(&ctx, &memory, DAY, at(21)),
            Decision::Skip(Skip::Degraded)
        );
        assert_eq!(memory.degraded_until(at(80)), None);
        assert_eq!(
            decide(&ctx, &memory, DAY, at(80)),
            Decision::Capture { signal: false }
        );
        fail(&mut memory, 4, 80);
        assert_eq!(memory.degraded_until(at(81)), Some(at(380)), "5 minutes");
        fail(&mut memory, 5, 380);
        assert_eq!(
            memory.degraded_until(at(381)),
            Some(at(1_280)),
            "15 minutes"
        );
        fail(&mut memory, 6, 1_280);
        assert_eq!(
            memory.degraded_until(at(1_281)),
            Some(at(2_180)),
            "and it stays at 15"
        );
        // One answer and vision is healthy again; the count starts over.
        record(&mut memory, 7, clean(7), DAY, at(2_180));
        assert_eq!(memory.degraded_until(at(2_181)), None);
        fail(&mut memory, 8, 2_200);
        fail(&mut memory, 9, 2_210);
        assert_eq!(memory.degraded_until(at(2_211)), None);
    }

    #[test]
    fn a_key_rejected_three_times_switches_the_check_off() {
        let ctx = plain();
        let mut memory = in_front(&ctx);
        for n in 0..3 {
            assert!(!memory.key_rejected());
            record(
                &mut memory,
                n,
                SampleEvent::Failed(CheckError::KeyRejected),
                DAY,
                at(i64::try_from(n).unwrap()),
            );
        }
        assert!(memory.key_rejected());
        assert_eq!(
            memory.degraded_until(at(5)),
            None,
            "a rejected key is not an outage"
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(86_400)),
            Decision::Skip(Skip::Degraded)
        );
        // The user changes the key: the caller forgets everything.
        memory.clear();
        assert!(!memory.key_rejected());
        record(&mut memory, KEY, SampleEvent::Front, DAY, at(86_400));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(86_415)),
            Decision::Capture { signal: false }
        );
    }

    // -- 4.6: outcomes ---------------------------------------------------------------------

    fn check(verdict: ExplicitVerdict, confidence: Confidence, refused: bool) -> ExplicitCheck {
        ExplicitCheck {
            verdict,
            confidence,
            refused,
        }
    }

    const VERDICTS: [ExplicitVerdict; 4] = [
        ExplicitVerdict::Explicit,
        ExplicitVerdict::Suggestive,
        ExplicitVerdict::None,
        ExplicitVerdict::Unsure,
    ];
    const CONFIDENCES: [Confidence; 3] = [Confidence::High, Confidence::Medium, Confidence::Low];

    #[test]
    fn outcomes_of_a_first_frame() {
        use Confidence::{High, Low, Medium};
        use ExplicitVerdict::{Explicit, Suggestive, Unsure};
        let first = |verdict, confidence| outcome(&check(verdict, confidence, false), Stage::First);
        assert_eq!(first(Explicit, High), Outcome::Positive);
        assert_eq!(first(Explicit, Medium), Outcome::Confirm);
        assert_eq!(first(Explicit, Low), Outcome::Unsure);
        for confidence in CONFIDENCES {
            assert_eq!(first(Suggestive, confidence), Outcome::Watch);
            assert_eq!(first(Unsure, confidence), Outcome::Watch);
            assert_eq!(first(ExplicitVerdict::None, confidence), Outcome::Clean);
        }
    }

    #[test]
    fn outcomes_of_the_second_frame_of_a_confirm() {
        use Confidence::{High, Low, Medium};
        use ExplicitVerdict::Explicit;
        let second =
            |verdict, confidence| outcome(&check(verdict, confidence, false), Stage::Second);
        assert_eq!(second(Explicit, High), Outcome::Positive);
        assert_eq!(second(Explicit, Medium), Outcome::Positive);
        assert_eq!(second(Explicit, Low), Outcome::Unsure);
        for verdict in [
            ExplicitVerdict::Suggestive,
            ExplicitVerdict::None,
            ExplicitVerdict::Unsure,
        ] {
            for confidence in CONFIDENCES {
                assert_eq!(
                    second(verdict, confidence),
                    Outcome::Unsure,
                    "{verdict:?} {confidence:?}"
                );
            }
        }
    }

    #[test]
    fn a_refusal_is_never_positive() {
        for verdict in VERDICTS {
            for confidence in CONFIDENCES {
                let refused = check(verdict, confidence, true);
                assert_eq!(
                    outcome(&refused, Stage::First),
                    Outcome::Watch,
                    "{verdict:?} {confidence:?}"
                );
                assert_eq!(
                    outcome(&refused, Stage::Second),
                    Outcome::Unsure,
                    "{verdict:?} {confidence:?}"
                );
            }
        }
    }

    #[test]
    fn answers_that_are_not_none_leave_a_signal_for_thirty_minutes() {
        for outcome in [
            Outcome::Watch,
            Outcome::Unsure,
            Outcome::Confirm,
            Outcome::Positive,
        ] {
            let mut memory = SamplerMemory::default();
            assert!(!memory.has_signal(KEY, at(0)));
            record(&mut memory, KEY, answered(outcome), DAY, at(0));
            assert!(memory.has_signal(KEY, at(1)), "{outcome:?}");
            assert!(memory.has_signal(KEY, at(1_799)), "{outcome:?}");
            assert!(!memory.has_signal(KEY, at(1_800)), "{outcome:?}");
            assert!(
                !memory.has_signal(KEY + 1, at(1)),
                "{outcome:?}: another key"
            );
        }
        let mut memory = SamplerMemory::default();
        record(&mut memory, KEY, clean(1), DAY, at(0));
        assert!(!memory.has_signal(KEY, at(1)));
    }

    #[test]
    fn the_heightened_state_lasts_fifteen_minutes() {
        let mut memory = SamplerMemory::default();
        assert!(!memory.heightened(at(0)));
        record(&mut memory, KEY, SampleEvent::Heightened, DAY, at(0));
        assert!(memory.heightened(at(0)) && memory.heightened(at(899)));
        assert!(!memory.heightened(at(900)));
        // A positive starts it too; nothing else does.
        let mut memory = SamplerMemory::default();
        record(&mut memory, KEY, answered(Outcome::Positive), DAY, at(0));
        assert!(memory.heightened(at(1)));
        for outcome in [
            Outcome::Watch,
            Outcome::Unsure,
            Outcome::Confirm,
            Outcome::Clean,
        ] {
            let mut memory = SamplerMemory::default();
            record(&mut memory, KEY, answered(outcome), DAY, at(0));
            assert!(!memory.heightened(at(1)), "{outcome:?}");
        }
    }

    #[test]
    fn two_refusals_in_a_row_on_a_key_with_a_local_signal_show_the_soft_card() {
        let refusal = SampleEvent::Checked {
            outcome: Outcome::Watch,
            refused: true,
            dhash: 0,
        };
        let mut memory = SamplerMemory::default();
        assert!(!refusal_card_due(&memory, KEY, true));
        record(&mut memory, KEY, refusal, DAY, at(0));
        assert!(!refusal_card_due(&memory, KEY, true), "one refusal alone");
        record(&mut memory, KEY, refusal, DAY, at(45));
        assert!(refusal_card_due(&memory, KEY, true));
        assert!(!refusal_card_due(&memory, KEY, false), "no local signal");
        assert!(!refusal_card_due(&memory, KEY + 1, true), "another key");
        // An answer in between breaks the row.
        record(&mut memory, KEY, answered(Outcome::Watch), DAY, at(90));
        assert!(!refusal_card_due(&memory, KEY, true));
        record(&mut memory, KEY, refusal, DAY, at(135));
        assert!(!refusal_card_due(&memory, KEY, true));
    }

    #[test]
    fn the_local_signal_of_a_context() {
        assert!(!plain().local_signal());
        assert!(!browser(Verdict::Unknown(HostClass::Popular)).local_signal());
        assert!(browser(Verdict::Unknown(HostClass::Mixed)).local_signal());
        assert!(browser(Verdict::Unknown(HostClass::Plain { signal: true })).local_signal());
        assert!(browser(Verdict::Allowed(AllowSource::Snooze)).local_signal());
        for flag in 0..3 {
            let mut ctx = plain();
            match flag {
                0 => ctx.title_signal = true,
                1 => ctx.path_signal = true,
                _ => ctx.private_window = true,
            }
            assert!(ctx.local_signal(), "flag {flag}");
        }
    }

    #[test]
    fn the_soft_card_is_shown_at_most_once_in_thirty_minutes_per_key() {
        let mut memory = SamplerMemory::default();
        assert!(memory.ask_card_allowed(KEY, at(0)));
        record(&mut memory, KEY, SampleEvent::AskCardShown, DAY, at(0));
        assert!(!memory.ask_card_allowed(KEY, at(1)));
        assert!(!memory.ask_card_allowed(KEY, at(1_799)));
        assert!(memory.ask_card_allowed(KEY, at(1_800)));
        assert!(memory.ask_card_allowed(KEY + 1, at(1)));
    }

    // -- 5.4: "Não era isso" ---------------------------------------------------------------

    #[test]
    fn not_it_buys_one_recheck_twenty_seconds_later_whatever_the_caps_say() {
        let mut limited = prefs();
        limited.max_per_hour = 1;
        limited.max_per_day = 1;
        let ctx = SampleCtx {
            has_window_id: true,
            verdict: Some(Verdict::Unknown(HostClass::Plain { signal: false })),
            ..SampleCtx::new(KEY, WindowKind::Browser, &limited)
        };
        let mut memory = in_front(&ctx);
        // Six checks on this key and both global caps used up.
        for second in 0..6 {
            record(&mut memory, KEY, answered(Outcome::Watch), DAY, at(second));
        }
        record(&mut memory, KEY, answered(Outcome::Positive), DAY, at(10));
        assert!(memory.cap_reached(&limited, DAY, at(11)));
        record(&mut memory, KEY, SampleEvent::NotIt, DAY, at(12));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(13)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(31)),
            Decision::Skip(Skip::NotDue)
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(32)),
            Decision::Capture { signal: true }
        );
        // It is one re-check: once it is answered the caps hold again.
        record(&mut memory, KEY, clean(5), DAY, at(32));
        assert!(!matches!(
            decide(&ctx, &memory, DAY, at(90)),
            Decision::Capture { .. }
        ));
        // Whatever it answered, the key stays at signal cadence for 30 minutes.
        assert!(memory.has_signal(KEY, at(33)));
        assert!(!memory.has_signal(KEY, at(32 + 1_800)));
    }

    #[test]
    fn a_recheck_that_is_positive_again_makes_the_button_an_exception_request_for_the_day() {
        let mut memory = SamplerMemory::default();
        assert!(!memory.exception_only(KEY, DAY));
        record(&mut memory, KEY, SampleEvent::NotIt, DAY, at(0));
        record(&mut memory, KEY, answered(Outcome::Positive), DAY, at(20));
        record(&mut memory, KEY, SampleEvent::NotItConfirmed, DAY, at(20));
        assert!(memory.exception_only(KEY, DAY));
        assert!(!memory.exception_only(KEY + 1, DAY));
        assert!(
            !memory.exception_only(KEY, "2026-09-18"),
            "only for the rest of the day"
        );
    }

    #[test]
    fn a_recheck_that_could_not_be_made_is_not_repeated() {
        let ctx = plain();
        let mut memory = in_front(&ctx);
        record(&mut memory, KEY, SampleEvent::NotIt, DAY, at(0));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(20)),
            Decision::Capture { signal: true }
        );
        record(
            &mut memory,
            KEY,
            SampleEvent::Failed(CheckError::Transient),
            DAY,
            at(20),
        );
        assert_eq!(
            decide(&ctx, &memory, DAY, at(21)),
            Decision::Skip(Skip::KeyBackoff)
        );
        assert!(memory.has_signal(KEY, at(21)));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(320)),
            Decision::Capture { signal: true }
        );
    }

    #[test]
    fn not_it_on_a_block_card() {
        use NotItEffect::{Request, Snooze};
        // A keyword or TLD block while vision is ready: a snooze.
        assert_eq!(not_it_on_block(BlockSource::Keyword, true, 0), Snooze);
        assert_eq!(not_it_on_block(BlockSource::Tld, true, 2), Snooze);
        // Vision not ready: with nothing left to look at the page, a delayed request.
        assert_eq!(not_it_on_block(BlockSource::Keyword, false, 0), Request);
        assert_eq!(not_it_on_block(BlockSource::Tld, false, 0), Request);
        // A list hit, a learned site or the user's own list: always a delayed request.
        for source in [BlockSource::List, BlockSource::Learned, BlockSource::User] {
            assert_eq!(not_it_on_block(source, true, 0), Request, "{source:?}");
        }
    }

    #[test]
    fn at_most_three_snoozes_a_day() {
        assert_eq!(MAX_SNOOZES_PER_DAY, 3);
        for used in 0..3 {
            assert_eq!(
                not_it_on_block(BlockSource::Keyword, true, used),
                NotItEffect::Snooze
            );
        }
        for used in [3, 4, 100, u32::MAX] {
            assert_eq!(
                not_it_on_block(BlockSource::Keyword, true, used),
                NotItEffect::Request
            );
        }
    }

    // -- keys, frames, memory --------------------------------------------------------------

    #[test]
    fn sample_keys() {
        let host = sample_key(Some("example.com"), "chrome", "Example Domain");
        assert_eq!(
            host,
            sample_key(Some("EXAMPLE.com"), "firefox", "another title")
        );
        assert_ne!(
            host,
            sample_key(Some("example.org"), "chrome", "Example Domain")
        );
        let window = sample_key(None, "vlc", "holiday.mp4");
        assert_eq!(window, sample_key(None, "VLC", "holiday.mp4"));
        assert_ne!(window, sample_key(None, "vlc", "other.mp4"));
        assert_ne!(window, sample_key(None, "mpv", "holiday.mp4"));
        // A window key never equals the key of a host with the same text.
        assert_ne!(sample_key(Some("vlc"), "", ""), sample_key(None, "vlc", ""));
        assert_ne!(sample_key(None, "ab", "c"), sample_key(None, "a", "bc"));
    }

    #[test]
    fn only_browser_frames_are_cropped() {
        for kind in [WindowKind::Browser, WindowKind::UnreadableBrowser] {
            assert_eq!(
                frame_opts(kind),
                FrameOpts {
                    max_edge: 640,
                    crop_top_frac: 0.12
                }
            );
        }
        for kind in [
            WindowKind::Media,
            WindowKind::Messaging,
            WindowKind::Other,
            WindowKind::NeverCapture,
        ] {
            assert_eq!(
                frame_opts(kind),
                FrameOpts {
                    max_edge: 640,
                    crop_top_frac: 0.0
                }
            );
        }
    }

    #[test]
    fn a_context_takes_its_switches_and_caps_from_the_preferences() {
        let mut chosen = prefs();
        chosen.unknown_sites = false;
        chosen.media_apps = false;
        chosen.messaging_apps = true;
        chosen.private_windows = false;
        chosen.max_per_hour = 7;
        chosen.max_per_day = 70;
        let ctx = SampleCtx::new(3, WindowKind::Media, &chosen);
        assert_eq!(
            ctx,
            SampleCtx {
                key: 3,
                kind: WindowKind::Media,
                verdict: None,
                has_window_id: false,
                private_window: false,
                title_signal: false,
                path_signal: false,
                excluded: false,
                unknown_sites: false,
                media_apps: false,
                messaging_apps: true,
                private_windows: false,
                max_per_hour: 7,
                max_per_day: 70,
            }
        );
    }

    #[test]
    fn clear_forgets_everything() {
        let ctx = browser(Verdict::Unknown(HostClass::Mixed));
        let mut memory = in_front(&ctx);
        simulate(&mut memory, &ctx, 0, 300, Outcome::Watch);
        record(&mut memory, KEY, SampleEvent::Heightened, DAY, at(300));
        record(&mut memory, KEY, SampleEvent::NotItConfirmed, DAY, at(300));
        memory.set_day_count(DAY, 500);
        memory.clear();
        assert!(!memory.heightened(at(301)));
        assert!(!memory.has_signal(KEY, at(301)));
        assert!(!memory.exception_only(KEY, DAY));
        assert!(!memory.cap_reached(&prefs(), DAY, at(301)));
        assert_eq!(
            decide(&ctx, &memory, DAY, at(301)),
            Decision::Skip(Skip::NotDue)
        );
    }

    #[test]
    fn a_clock_set_back_does_not_switch_the_check_off() {
        let ctx = plain();
        let mut memory = in_front(&ctx);
        // Everything that holds a check back was set "now" …
        for key in 1..=3 {
            record(
                &mut memory,
                key,
                SampleEvent::Failed(CheckError::Transient),
                DAY,
                at(0),
            );
        }
        record(
            &mut memory,
            KEY,
            SampleEvent::Failed(CheckError::Transient),
            DAY,
            at(0),
        );
        record(&mut memory, KEY, clean(1), DAY, at(0));
        record(
            &mut memory,
            KEY,
            SampleEvent::Failed(CheckError::Transient),
            DAY,
            at(0),
        );
        record(&mut memory, KEY, SampleEvent::Heightened, DAY, at(0));
        record(&mut memory, KEY, answered(Outcome::Watch), DAY, at(0));
        record(&mut memory, KEY, SampleEvent::AskCardShown, DAY, at(0));
        record(&mut memory, KEY, SampleEvent::NotIt, DAY, at(0));
        // … and then the clock goes back a day.
        let yesterday = -86_400;
        assert_eq!(memory.degraded_until(at(yesterday)), None);
        assert!(!memory.heightened(at(yesterday)));
        assert!(!memory.has_signal(KEY, at(yesterday)));
        assert!(memory.ask_card_allowed(KEY, at(yesterday)));
        record(&mut memory, KEY, SampleEvent::Front, DAY, at(yesterday));
        // The re-check that was bought is made, not postponed by a day.
        assert_eq!(
            decide(&ctx, &memory, DAY, at(yesterday + 15)),
            Decision::Capture { signal: true }
        );
        record(&mut memory, KEY, clean(2), DAY, at(yesterday + 15));
        // And the looks that seem to lie in the future do not delay the next one.
        let other = SampleCtx {
            key: 50,
            ..ctx.clone()
        };
        record(&mut memory, 50, clean(3), DAY, at(0));
        record(&mut memory, 50, SampleEvent::Front, DAY, at(yesterday + 20));
        assert_eq!(
            decide(&other, &memory, DAY, at(yesterday + 40)),
            Decision::Capture { signal: false }
        );
    }

    #[test]
    fn the_memory_does_not_grow_without_bound() {
        let mut memory = SamplerMemory::default();
        for key in 0..5_000u64 {
            record(
                &mut memory,
                key,
                SampleEvent::Front,
                DAY,
                at(i64::try_from(key).unwrap() * 60),
            );
        }
        // 5000 windows over three and a half days: only the last day's are kept.
        assert!(memory.keys.len() <= MAX_KEYS + 1, "{}", memory.keys.len());
        assert!(memory.keys.contains_key(&4_999));
        assert!(!memory.keys.contains_key(&0));
    }
}
