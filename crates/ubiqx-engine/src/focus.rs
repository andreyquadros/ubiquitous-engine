//! The focus guard: holds blocked apps and sites, runs focus sessions and serves the focus
//! page. [`guard_once`] is the pass the engine runs every couple of seconds; the rest is the
//! API behind `EngineHandle` and the IPC commands.
//!
//! Every decision that needs no I/O lives in `ubiqx_core::focus`; this module only wires it
//! to the platform ports (foreground window, browser URL, enforcer, presenter, notifier) and
//! to the repositories.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Local, Utc};
use ubiqx_core::focus as rules_focus;
use ubiqx_core::normalize::domain_of;
use ubiqx_core::ports::*;
use ubiqx_core::*;

use crate::nudges;
use crate::state::EngineState;

/// How far back [`known_domains`] looks.
const KNOWN_DOMAINS_DAYS: i64 = 30;

/// Application ids (executable stems) the guard never enforces on Windows: the shell that
/// hosts the taskbar and the file manager, Settings, the Task Manager and the lock screen.
pub const EXEMPT_APP_IDS_WINDOWS: &[&str] = &[
    "explorer",
    "systemsettings",
    "taskmgr",
    // The frame that hosts every UWP/Store app: the adapter resolves the hosted app, and when
    // it cannot, closing the host would take all of them down.
    "applicationframehost",
    "lockapp",
    "searchhost",
    "startmenuexperiencehost",
    "shellexperiencehost",
    "logonui",
];

/// Application ids (executable stems) the guard never enforces on Linux: the desktop
/// shells, their file managers and settings.
pub const EXEMPT_APP_IDS_LINUX: &[&str] = &[
    "gnome-shell",
    "plasmashell",
    "nautilus",
    "dolphin",
    "gnome-control-center",
    "systemsettings",
    "systemsettings5",
    "xfce4-panel",
    "thunar",
];

/// Terminal emulators, by app id: like the macOS Terminal they are left alone for the
/// first [`rules_focus::TERMINAL_GRACE_SECS`] of a session (the user may still be closing
/// things down or launching the task's tools).
pub const TERMINAL_APP_IDS: &[&str] = &[
    // Windows
    "windowsterminal",
    "cmd",
    "powershell",
    "pwsh",
    "conhost",
    // Linux
    "gnome-terminal",
    "gnome-terminal-server",
    "konsole",
    "alacritty",
    "kitty",
    "wezterm",
    "wezterm-gui",
    "xterm",
    "tilix",
    "terminator",
    "xfce4-terminal",
    "ptyxis",
];

/// Whether the guard leaves this window alone: the platform-independent rules of
/// `ubiqx_core::focus` (ubiqX, Finder, System Settings, the macOS Terminal grace) plus
/// the shells, settings apps and terminals of Windows and Linux, matched by app id
/// (executable stem) case-insensitively. `session_age_secs` is `None` when no session runs.
pub fn is_exempt_window(app_id: &str, app_name: &str, session_age_secs: Option<i64>) -> bool {
    if rules_focus::is_exempt_app(app_id, app_name, session_age_secs) {
        return true;
    }
    let id = app_id.trim();
    if id.is_empty() {
        return false;
    }
    if EXEMPT_APP_IDS_WINDOWS
        .iter()
        .chain(EXEMPT_APP_IDS_LINUX)
        .any(|x| x.eq_ignore_ascii_case(id))
    {
        return true;
    }
    if TERMINAL_APP_IDS.iter().any(|t| t.eq_ignore_ascii_case(id)) {
        return session_age_secs.is_some_and(|age| age < rules_focus::TERMINAL_GRACE_SECS);
    }
    false
}

/// Id of the sample intervention shown by [`test_intervention`].
pub const TEST_INTERVENTION_ID: &str = "test";

/// In-memory state of the guard, kept on `EngineState`.
#[derive(Debug, Default)]
pub struct FocusRuntime {
    /// The running session, mirrored in the store.
    pub session: Option<FocusSession>,
    /// Last intervention per target key, for the per-key cooldown.
    pub cooldowns: HashMap<String, DateTime<Utc>>,
    /// Position in the message rotation.
    pub message_index: usize,
}

impl FocusRuntime {
    /// Picks up the session the previous run left behind, if any.
    pub fn restore(repo: &dyn FocusRepo) -> Self {
        let session = match repo.active_session() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "could not read the active focus session");
                None
            }
        };
        Self {
            session,
            ..Default::default()
        }
    }

    /// The next intervention message, advancing the rotation.
    fn next_message(&mut self, lang: UiLanguage) -> String {
        let m = rules_focus::intervention_message(self.message_index, lang);
        self.message_index = self.message_index.wrapping_add(1);
        m.to_string()
    }
}

/// Something in the foreground the guard decided to hold.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hit {
    target_id: Option<Id>,
    kind: FocusTargetKind,
    name: String,
    key: String,
    /// The bundle id to act on: the app itself, or the browser showing the site.
    app_id: String,
}

// ---------------------------------------------------------------------------------------------
// Guard pass
// ---------------------------------------------------------------------------------------------

/// One pass of the guard: ends a session that reached its end, then looks at the foreground
/// window and holds it when it hits a blocked target (or, during a session, a distraction
/// rule). Returns the intervention it made, if any.
pub fn guard_once(state: &Arc<EngineState>) -> CoreResult<Option<Intervention>> {
    let now = state.now();
    let settings = state.settings();

    // The session timer.
    let session = state.focus.lock().session.clone();
    let session = match session {
        Some(s) if now >= s.ends_at => {
            end_session(state, s, now, true)?;
            None
        }
        other => other,
    };
    if !settings.focus.guard_enabled {
        return Ok(None);
    }
    let targets: Vec<FocusTarget> = state
        .deps
        .repos
        .focus
        .list_targets()?
        .into_iter()
        .filter(|t| t.enabled)
        .collect();
    let rules_apply = session.is_some() && settings.focus.block_distraction_in_session;
    if targets.is_empty() && !rules_apply {
        // Nothing to enforce: no need to touch the window server.
        return Ok(None);
    }

    let platform = &state.deps.platform;
    let Some(win) = platform.activity.foreground()? else {
        return Ok(None);
    };
    let session_age = session.as_ref().map(|s| (now - s.started_at).num_seconds());
    if is_exempt_window(&win.app_id, &win.app_name, session_age) {
        return Ok(None);
    }
    let url = if platform.urls.supports(&win.app_id) {
        match platform.urls.resolve(&win) {
            Ok(u) => u,
            Err(e) => {
                tracing::debug!(error = %e, "focus guard could not read the browser url");
                None
            }
        }
    } else {
        None
    };
    let domain = url.as_deref().and_then(domain_of);

    let hit = match rules_focus::find_hit(&targets, &win.app_id, domain.as_deref()) {
        Some(t) => Some(Hit {
            target_id: Some(t.id.clone()),
            kind: t.kind,
            name: t.name.clone(),
            key: t.key.clone(),
            app_id: win.app_id.clone(),
        }),
        None if rules_apply => distraction_hit(state, &win, url.as_deref(), domain.as_deref())?,
        None => None,
    };
    let Some(hit) = hit else {
        return Ok(None);
    };

    // Per-key cooldown.
    let cooldown = Duration::seconds(i64::from(settings.focus.intervention_cooldown_secs));
    {
        let runtime = state.focus.lock();
        if let Some(last) = runtime.cooldowns.get(&hit.key) {
            if now - *last < cooldown {
                return Ok(None);
            }
        }
    }
    intervene(state, hit, session, now, settings.ui_language()).map(Some)
}

/// During a session: whether the user's own rules put the foreground in the built-in
/// distraction category.
fn distraction_hit(
    state: &EngineState,
    win: &ForegroundWindow,
    url: Option<&str>,
    domain: Option<&str>,
) -> CoreResult<Option<Hit>> {
    let rules = state.deps.repos.rules.list()?;
    if rules.is_empty() {
        return Ok(None);
    }
    let now = state.now();
    let mut block = ActivityBlock::new_manual(now, now, String::new(), None);
    block.is_manual = false;
    block.category_id = None;
    block.source = None;
    block.app_id = win.app_id.clone();
    block.app_name = win.app_name.clone();
    block.title = win.window_title.clone();
    block.title_key = ubiqx_core::normalize::normalize_title(&win.window_title, &win.app_name);
    block.url = url.map(String::from);
    block.domain = domain.map(String::from);
    let Some(rule) = ubiqx_core::rules::best_rule(&rules, &block) else {
        return Ok(None);
    };
    if rule.category_id != system_categories::DISTRACTION {
        return Ok(None);
    }
    Ok(Some(match domain {
        Some(d) => Hit {
            target_id: None,
            kind: FocusTargetKind::Site,
            name: d.to_string(),
            key: d.to_string(),
            app_id: win.app_id.clone(),
        },
        None => Hit {
            target_id: None,
            kind: FocusTargetKind::App,
            name: win.app_name.clone(),
            key: win.app_id.clone(),
            app_id: win.app_id.clone(),
        },
    }))
}

/// Enforces a hit, records it, tells the shells and shows UBI.
fn intervene(
    state: &Arc<EngineState>,
    hit: Hit,
    session: Option<FocusSession>,
    now: DateTime<Utc>,
    lang: UiLanguage,
) -> CoreResult<Intervention> {
    let enforcer = &state.deps.platform.enforcer;
    let action = match hit.kind {
        FocusTargetKind::App => {
            if attempt("quit app", enforcer.quit_app(&hit.app_id)) {
                InterventionAction::AppQuit
            } else {
                InterventionAction::Notified
            }
        }
        FocusTargetKind::Site => {
            if attempt("close tab", enforcer.close_active_tab(&hit.app_id)) {
                InterventionAction::TabClosed
            } else if attempt("blank tab", enforcer.blank_active_tab(&hit.app_id)) {
                InterventionAction::TabBlanked
            } else {
                InterventionAction::Notified
            }
        }
    };
    let message = state.focus.lock().next_message(lang);
    let intervention = Intervention {
        id: new_id(),
        at: now,
        target_id: hit.target_id.clone(),
        kind: hit.kind,
        name: hit.name.clone(),
        key: hit.key.clone(),
        action,
        session_id: session.as_ref().map(|s| s.id.clone()),
        message,
    };
    let repos = &state.deps.repos;
    repos.focus.insert_intervention(&intervention)?;
    if let Some(id) = &hit.target_id {
        if let Err(e) = repos.focus.touch_blocked(id, now) {
            tracing::warn!(error = %e, "could not update the blocked target");
        }
    }
    {
        let mut runtime = state.focus.lock();
        runtime.cooldowns.insert(hit.key.clone(), now);
        if runtime.cooldowns.len() > 512 {
            runtime
                .cooldowns
                .retain(|_, at| now - *at < Duration::hours(1));
        }
    }
    if let Some(mut s) = session {
        s.interventions = s.interventions.saturating_add(1);
        repos.focus.update_session(&s)?;
        state.focus.lock().session = Some(s.clone());
        state
            .deps
            .sink
            .emit(EngineEvent::FocusSession { session: Some(s) });
    }
    tracing::info!(
        name = %intervention.name,
        key = %intervention.key,
        action = intervention.action.as_str(),
        "focus guard intervened"
    );
    state.deps.sink.emit(EngineEvent::Intervention {
        intervention: intervention.clone(),
    });
    let shown = match state.deps.platform.presenter.show(&intervention) {
        Ok(shown) => shown,
        Err(e) => {
            tracing::debug!(error = %e, "intervention window failed");
            false
        }
    };
    if !shown {
        if let Err(e) = state
            .deps
            .platform
            .notifier
            .notify(&intervention.name, &intervention.message)
        {
            tracing::debug!(error = %e, "intervention notification failed");
        }
    }
    Ok(intervention)
}

/// `true` when an enforcement action reported success; errors are logged as a refusal.
fn attempt(what: &str, result: CoreResult<bool>) -> bool {
    match result {
        Ok(done) => done,
        Err(e) => {
            tracing::debug!(what, error = %e, "enforcement failed");
            false
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------------------------

/// Starts a focus session on `task` for `minutes` (a running one is ended first): hides the
/// other windows and runs the "on" Shortcut when configured, persists and announces it.
pub fn start_session(
    state: &Arc<EngineState>,
    task: &str,
    minutes: u32,
) -> CoreResult<FocusSession> {
    rules_focus::validate_session(task, minutes).map_err(CoreError::Invalid)?;
    let now = state.now();
    let settings = state.settings();
    // The guard is released before `end_session` takes it again.
    let current = state.focus.lock().session.clone();
    if let Some(current) = current {
        end_session(state, current, now, false)?;
    }
    let platform = &state.deps.platform;
    let mut session = FocusSession {
        id: new_id(),
        task: task.trim().to_string(),
        started_at: now,
        ends_at: now + Duration::minutes(i64::from(minutes)),
        ended_at: None,
        interventions: 0,
        hid_windows: false,
        ran_shortcut: false,
    };
    if settings.focus.hide_others_on_start {
        let keep = platform
            .activity
            .foreground()
            .ok()
            .flatten()
            .map(|w| w.app_id)
            .unwrap_or_default();
        session.hid_windows = attempt("hide others", platform.enforcer.hide_others(&keep));
    }
    if let Some(name) = shortcut_name(settings.focus.macos_focus_shortcut_on.as_deref()) {
        session.ran_shortcut = attempt("shortcut on", platform.enforcer.run_shortcut(name));
    }
    state.deps.repos.focus.insert_session(&session)?;
    state.focus.lock().session = Some(session.clone());
    tracing::info!(task = %session.task, minutes, "focus session started");
    state.deps.sink.emit(EngineEvent::FocusSession {
        session: Some(session.clone()),
    });
    Ok(session)
}

/// Ends the running session now, if any.
pub fn stop_session(state: &Arc<EngineState>) -> CoreResult<Option<FocusSession>> {
    let current = state.focus.lock().session.clone();
    match current {
        Some(s) => end_session(state, s, state.now(), false).map(Some),
        None => Ok(None),
    }
}

/// Ends `session` at `now`: persists `ended_at`, runs the "off" Shortcut, announces the end
/// and stores the praise nudge (skipped for a session stopped by hand within the first
/// minutes, and whenever nudges or the praise kind are switched off in the settings, like the
/// policy nudges). `timed_out` marks the planned end, which also notifies the OS since the
/// user is likely elsewhere.
fn end_session(
    state: &Arc<EngineState>,
    mut session: FocusSession,
    now: DateTime<Utc>,
    timed_out: bool,
) -> CoreResult<FocusSession> {
    let settings = state.settings();
    session.ended_at = Some(now.max(session.started_at));
    state.deps.repos.focus.update_session(&session)?;
    {
        let mut runtime = state.focus.lock();
        if runtime.session.as_ref().map(|s| s.id.as_str()) == Some(session.id.as_str()) {
            runtime.session = None;
        }
    }
    if let Some(name) = shortcut_name(settings.focus.macos_focus_shortcut_off.as_deref()) {
        attempt(
            "shortcut off",
            state.deps.platform.enforcer.run_shortcut(name),
        );
    }
    tracing::info!(task = %session.task, timed_out, interventions = session.interventions, "focus session ended");
    state.deps.sink.emit(EngineEvent::FocusSession {
        session: Some(session.clone()),
    });
    let lasted = (session.ended_at.unwrap_or(now) - session.started_at).num_minutes();
    let praise_on = settings.nudges.enabled && settings.nudges.praise;
    if praise_on && (timed_out || lasted >= rules_focus::PRAISE_MIN_MINUTES) {
        let minutes = if timed_out {
            session.planned_minutes()
        } else {
            lasted
        };
        let (title, message) = rules_focus::praise_text(
            settings.ui_language(),
            minutes,
            &session.task,
            session.interventions,
        );
        nudges::emit(state, NudgeKind::Praise, &title, &message, timed_out);
    }
    Ok(session)
}

fn shortcut_name(configured: Option<&str>) -> Option<&str> {
    configured.map(str::trim).filter(|n| !n.is_empty())
}

// ---------------------------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------------------------

/// Adds a target (or re-enables the one already there for the same kind and key), with the
/// key normalised as [`ubiqx_core::focus::normalize_key`] does.
pub fn add_target(
    state: &EngineState,
    kind: FocusTargetKind,
    name: &str,
    key: &str,
) -> CoreResult<FocusTarget> {
    let key = rules_focus::normalize_key(kind, key);
    if key.is_empty() {
        return Err(CoreError::Invalid("key must not be empty".into()));
    }
    let name = name.trim();
    let name = if name.is_empty() { key.as_str() } else { name };
    let target = FocusTarget {
        id: new_id(),
        kind,
        name: name.to_string(),
        key,
        enabled: true,
        created_at: state.now(),
        last_blocked_at: None,
        blocked_count: 0,
    };
    state.deps.repos.focus.upsert_target(&target)
}

pub fn set_target_enabled(state: &EngineState, id: &str, enabled: bool) -> CoreResult<FocusTarget> {
    state.deps.repos.focus.set_target_enabled(id, enabled)
}

pub fn remove_target(state: &EngineState, id: &str) -> CoreResult<()> {
    state.deps.repos.focus.remove_target(id)
}

pub fn list_targets(state: &EngineState) -> CoreResult<Vec<FocusTarget>> {
    state.deps.repos.focus.list_targets()
}

pub fn list_interventions(state: &EngineState, limit: usize) -> CoreResult<Vec<Intervention>> {
    state.deps.repos.focus.list_interventions(limit)
}

pub fn installed_apps(state: &EngineState) -> CoreResult<Vec<InstalledApp>> {
    state.deps.platform.apps.installed_apps()
}

// ---------------------------------------------------------------------------------------------
// Read models
// ---------------------------------------------------------------------------------------------

/// The focus page's header: the running session, enabled targets, today's interventions.
pub fn status(state: &EngineState) -> CoreResult<FocusStatus> {
    let now = state.now();
    let session = state.focus.lock().session.clone();
    let remaining_secs = session.as_ref().map(|s| s.remaining_secs(now));
    let repos = &state.deps.repos;
    let targets_enabled = repos
        .focus
        .list_targets()?
        .iter()
        .filter(|t| t.enabled)
        .count() as u32;
    let day_start = ubiqx_core::scheduler::day_range(now.with_timezone(&Local).date_naive()).from;
    let interventions_today = repos.focus.count_interventions_since(day_start)? as u32;
    Ok(FocusStatus {
        session,
        remaining_secs,
        targets_enabled,
        interventions_today,
        guard_enabled: state.settings.read().focus.guard_enabled,
    })
}

/// Domains seen in the user's own blocks over the last month, most time first.
pub fn known_domains(state: &EngineState, limit: usize) -> CoreResult<Vec<KnownDomain>> {
    let now = state.now();
    let range = TimeRange::new(now - Duration::days(KNOWN_DOMAINS_DAYS), now);
    let mut totals: HashMap<String, i64> = HashMap::new();
    for b in state.deps.repos.blocks.list_in_range(range)? {
        let Some(d) = b.domain.as_deref().map(str::trim).filter(|d| !d.is_empty()) else {
            continue;
        };
        *totals.entry(d.to_lowercase()).or_default() += b.duration_secs();
    }
    let mut out: Vec<KnownDomain> = totals
        .into_iter()
        .map(|(domain, seconds)| KnownDomain { domain, seconds })
        .collect();
    out.sort_by(|a, b| {
        b.seconds
            .cmp(&a.seconds)
            .then_with(|| a.domain.cmp(&b.domain))
    });
    out.truncate(limit);
    Ok(out)
}

/// The sample intervention the "test" button shows (id [`TEST_INTERVENTION_ID`]).
pub fn sample_intervention(state: &EngineState) -> Intervention {
    let lang = state.settings.read().ui_language();
    Intervention {
        id: TEST_INTERVENTION_ID.into(),
        at: state.now(),
        target_id: None,
        kind: FocusTargetKind::Site,
        name: "YouTube".into(),
        key: "youtube.com".into(),
        action: InterventionAction::TabClosed,
        session_id: None,
        message: rules_focus::intervention_message(0, lang).to_string(),
    }
}

/// Shows the intervention window with the sample; nothing is recorded. Falls back to an OS
/// notification like a real intervention would.
pub fn test_intervention(state: &EngineState) -> CoreResult<()> {
    let sample = sample_intervention(state);
    let shown = state.deps.platform.presenter.show(&sample)?;
    if !shown {
        state
            .deps
            .platform
            .notifier
            .notify(&sample.name, &sample.message)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exemptions_cover_every_desktop() {
        // The core rules still apply.
        assert!(is_exempt_window("ai.ubiqx.app", "ubiqX", None));
        assert!(is_exempt_window("com.apple.finder", "Finder", Some(1000)));
        assert!(is_exempt_window("com.apple.Terminal", "Terminal", Some(30)));
        assert!(!is_exempt_window(
            "com.apple.Terminal",
            "Terminal",
            Some(61)
        ));
        // Windows and Linux shells, by executable stem, whatever the case.
        assert!(is_exempt_window("explorer", "Windows Explorer", None));
        assert!(is_exempt_window("Explorer", "", Some(500)));
        assert!(is_exempt_window("systemsettings", "Settings", None));
        assert!(is_exempt_window("taskmgr", "Task Manager", None));
        assert!(is_exempt_window(
            "applicationframehost",
            "Application Frame Host",
            None
        ));
        assert!(is_exempt_window("gnome-shell", "GNOME Shell", None));
        assert!(is_exempt_window("plasmashell", "Plasma", None));
        assert!(is_exempt_window("nautilus", "Files", None));
        assert!(is_exempt_window("dolphin", "Dolphin", None));
        assert!(is_exempt_window("gnome-control-center", "Settings", None));
        // Terminals get the grace period only.
        for t in [
            "windowsterminal",
            "cmd",
            "powershell",
            "gnome-terminal-server",
            "konsole",
            "alacritty",
            "kitty",
            "wezterm-gui",
        ] {
            assert!(is_exempt_window(t, "", Some(10)), "{t} in grace");
            assert!(!is_exempt_window(t, "", Some(60)), "{t} after grace");
            assert!(!is_exempt_window(t, "", None), "{t} without a session");
        }
        // Ordinary apps are never exempt.
        assert!(!is_exempt_window("chrome", "Google Chrome", Some(10)));
        assert!(!is_exempt_window("firefox", "Firefox", None));
        assert!(!is_exempt_window("", "", None));
    }
}
