//! Commitment: which change takes effect now and which waits the delay the user chose,
//! and when a waiting change may be confirmed (design section 6).
//!
//! | `NofapChange` | `Effect::Now` when | `Effect::After(delay)` when |
//! |---|---|---|
//! | `Disable` | never | always |
//! | `AddBlock`, `RemoveAllow` | always | never |
//! | `RemoveBlock`, `AddAllow`, `ForgetLearned` | never | always |
//! | `SetHosts`, `SetPolicies`, `SetVision`, `SetHostsUserSites` | `true` | `false` |
//! | `SetResolver` | `Some` | `None` |
//! | `SetVisionScope` | no flag goes from on to off | any flag goes from on to off |
//! | `SetCaps` | no value is lowered | any value is lowered |
//! | `SetDelay` | raised or equal | lowered |
//! | `SetPrefs`, `SetValues`, `SetPlans` | always | never |
//!
//! While the module is off (before it is enabled for the first time, and after a confirmed
//! `Disable`) everything is `Now`: nothing is protecting, so nothing can be weakened.
//! Cancelling a pending change is always immediate. A change that asks for what is already
//! so (switching off a tier that is off) weakens nothing and is `Now` too.
//!
//! # Clock changes
//!
//! A request is ready when the wall clock has reached `ready_at` **and** the app has been
//! observed running for `min(delay, 15 minutes)` since the request. `ready_at` is fixed at
//! request time, so setting the clock back never shortens a wait, and the observed time
//! grows by at most 150 seconds per verify pass whatever the clock says, so setting it
//! forward still costs up to 15 minutes of real running time. A stored time that lies in
//! the future (the clock was set back after the fact) never counts as passed.

use chrono::{DateTime, Duration, Utc};
use ubiqx_core::nofap::{
    NofapChange, NofapConfig, PendingChange, PendingView, VisionPrefs, DELAY_MAX_SECS, MAX_PENDING,
};
use ubiqx_core::{CoreError, CoreResult, Id};

/// A request is ready only after this much observed running time (or the whole delay,
/// when the delay is shorter).
pub const OBSERVED_CAP_SECS: u32 = 900;
/// Most observed time one verify pass may add.
pub const OBSERVE_STEP_MAX_SECS: u32 = 150;
/// A ready request expires without a confirmation after this long.
pub const CONFIRM_WINDOW_SECS: i64 = 86_400;

/// When a change takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    Now,
    /// After this many seconds (the delay in force), then a fresh confirmation.
    After(u32),
}

/// The caps a `SetCaps` change would leave, clamped like the stored ones.
fn requested_caps(config: &NofapConfig, per_hour: u32, per_day: u32, monthly: f64) -> VisionPrefs {
    let mut vision = VisionPrefs {
        max_per_hour: per_hour,
        max_per_day: per_day,
        monthly_cap_usd: monthly,
        ..config.vision.clone()
    };
    vision.sanitize();
    vision
}

/// Decides the effect of `change` on `config`. The `match` inside is exhaustive with no
/// wildcard arm, so a new change kind does not compile until it is classified.
pub fn classify(config: &NofapConfig, change: &NofapChange) -> Effect {
    if !config.enabled {
        return Effect::Now;
    }
    let system = &config.system;
    let vision = &config.vision;
    let loosens = match change {
        NofapChange::Disable => true,
        NofapChange::AddBlock { .. } | NofapChange::RemoveAllow { .. } => false,
        NofapChange::RemoveBlock { .. }
        | NofapChange::AddAllow { .. }
        | NofapChange::ForgetLearned => true,
        NofapChange::SetHosts { on } => system.hosts && !on,
        NofapChange::SetPolicies { on } => system.policies && !on,
        NofapChange::SetHostsUserSites { on } => system.hosts_user_sites && !on,
        NofapChange::SetVision { on } => vision.enabled && !on,
        NofapChange::SetResolver { resolver } => system.resolver.is_some() && resolver.is_none(),
        NofapChange::SetVisionScope {
            unknown_sites,
            media_apps,
            messaging_apps,
            private_windows,
        } => {
            (vision.unknown_sites && !unknown_sites)
                || (vision.media_apps && !media_apps)
                || (vision.messaging_apps && !messaging_apps)
                || (vision.private_windows && !private_windows)
        }
        NofapChange::SetCaps {
            max_per_hour,
            max_per_day,
            monthly_cap_usd,
        } => {
            let wanted = requested_caps(config, *max_per_hour, *max_per_day, *monthly_cap_usd);
            wanted.max_per_hour < vision.max_per_hour
                || wanted.max_per_day < vision.max_per_day
                || wanted.monthly_cap_usd < vision.monthly_cap_usd
        }
        NofapChange::SetDelay { secs } => (*secs).min(DELAY_MAX_SECS) < config.delay_secs,
        NofapChange::SetPrefs { .. }
        | NofapChange::SetValues { .. }
        | NofapChange::SetPlans { .. } => false,
    };
    if loosens {
        Effect::After(config.delay_secs.min(DELAY_MAX_SECS))
    } else {
        Effect::Now
    }
}

/// What [`request`] did.
#[derive(Debug, Clone, PartialEq)]
pub enum Requested {
    /// The change takes effect now. The caller applies it: [`apply_to_config`] for the
    /// configuration, and the site changes (`AddBlock`, `RemoveAllow`, …) in the repository.
    Apply(NofapChange),
    /// The change was queued in `config.pending`.
    Pending(PendingChange),
}

/// Classifies `change` and, when it has to wait, appends it to `config.pending` with
/// `ready_at = now + delay`. `id` is the id of the pending entry. Fails when
/// `MAX_PENDING` requests already wait.
///
/// Asking again for a change that already waits returns the waiting entry: its clock does
/// not start over and it takes no second place in the queue. With a delay of 0 the entry
/// is ready at once and still needs the confirmation.
pub fn request(
    config: &mut NofapConfig,
    change: NofapChange,
    id: Id,
    now: DateTime<Utc>,
) -> CoreResult<Requested> {
    let delay = match classify(config, &change) {
        Effect::Now => return Ok(Requested::Apply(change)),
        Effect::After(delay) => delay,
    };
    if let Some(waiting) = config.pending.iter().find(|entry| entry.change == change) {
        return Ok(Requested::Pending(waiting.clone()));
    }
    if config.pending.len() >= MAX_PENDING {
        return Err(CoreError::Invalid(format!(
            "{MAX_PENDING} requests are already waiting; cancel one first"
        )));
    }
    let mut pending = PendingChange {
        id,
        change,
        requested_at: now,
        ready_at: now + Duration::seconds(i64::from(delay)),
        observed_secs: 0,
        ready_since: None,
    };
    if is_ready(&pending, now) {
        pending.ready_since = Some(now);
    }
    config.pending.push(pending.clone());
    Ok(Requested::Pending(pending))
}

/// Applies the part of `change` that lives in the configuration and sanitises it. Returns
/// `false` for a change that only concerns the site lists.
///
/// `Disable` switches the module off and drops every waiting request (while the module is
/// off there is nothing left for them to weaken). `SetResolver` also ends the "network
/// restored" state: choosing a resolver again is "Religar".
pub fn apply_to_config(
    config: &mut NofapConfig,
    change: &NofapChange,
    _now: DateTime<Utc>,
) -> bool {
    let before = config.vision.clone();
    let applied = apply_change(config, change);
    if applied {
        carry_tightening(config, &before);
    }
    applied
}

fn apply_change(config: &mut NofapConfig, change: &NofapChange) -> bool {
    match change {
        NofapChange::Disable => {
            config.enabled = false;
            config.pending.clear();
        }
        NofapChange::AddBlock { .. }
        | NofapChange::RemoveBlock { .. }
        | NofapChange::AddAllow { .. }
        | NofapChange::RemoveAllow { .. }
        | NofapChange::ForgetLearned => return false,
        NofapChange::SetHosts { on } => config.system.hosts = *on,
        NofapChange::SetResolver { resolver } => {
            config.system.resolver = *resolver;
            config.system.network_restored = false;
        }
        NofapChange::SetPolicies { on } => config.system.policies = *on,
        NofapChange::SetHostsUserSites { on } => config.system.hosts_user_sites = *on,
        NofapChange::SetVision { on } => config.vision.enabled = *on,
        NofapChange::SetVisionScope {
            unknown_sites,
            media_apps,
            messaging_apps,
            private_windows,
        } => {
            config.vision.unknown_sites = *unknown_sites;
            config.vision.media_apps = *media_apps;
            config.vision.messaging_apps = *messaging_apps;
            config.vision.private_windows = *private_windows;
        }
        NofapChange::SetCaps {
            max_per_hour,
            max_per_day,
            monthly_cap_usd,
        } => {
            config.vision.max_per_hour = *max_per_hour;
            config.vision.max_per_day = *max_per_day;
            config.vision.monthly_cap_usd = *monthly_cap_usd;
        }
        NofapChange::SetDelay { secs } => config.delay_secs = *secs,
        NofapChange::SetPrefs { prefs } => config.ui = prefs.clone(),
        NofapChange::SetValues { text } => config.values = text.clone(),
        NofapChange::SetPlans { plans } => config.plans = plans.clone(),
    }
    config.sanitize();
    true
}

/// A waiting `SetVisionScope` or `SetCaps` carries every value as it was when it was
/// asked for, and confirming it writes all of them. A scope switch turned on, or a cap
/// raised, after it was queued (an immediate change) would then go back to its old
/// value with no wait of its own. So every tightening applied here is carried into the
/// waiting requests of that kind: a switch it turned on is on in them, and a cap it raised
/// is raised in those that did not lower it. What a request asked to loosen, and waited
/// for, it still loosens.
fn carry_tightening(config: &mut NofapConfig, before: &VisionPrefs) {
    let now = config.vision.clone();
    let turned_on = |was: bool, is: bool| !was && is;
    for waiting in &mut config.pending {
        match &mut waiting.change {
            NofapChange::SetVisionScope {
                unknown_sites,
                media_apps,
                messaging_apps,
                private_windows,
            } => {
                for (flag, was, is) in [
                    (unknown_sites, before.unknown_sites, now.unknown_sites),
                    (media_apps, before.media_apps, now.media_apps),
                    (messaging_apps, before.messaging_apps, now.messaging_apps),
                    (private_windows, before.private_windows, now.private_windows),
                ] {
                    if turned_on(was, is) {
                        *flag = true;
                    }
                }
            }
            NofapChange::SetCaps {
                max_per_hour,
                max_per_day,
                monthly_cap_usd,
            } => {
                if now.max_per_hour > before.max_per_hour && *max_per_hour >= before.max_per_hour {
                    *max_per_hour = (*max_per_hour).max(now.max_per_hour);
                }
                if now.max_per_day > before.max_per_day && *max_per_day >= before.max_per_day {
                    *max_per_day = (*max_per_day).max(now.max_per_day);
                }
                if now.monthly_cap_usd > before.monthly_cap_usd
                    && *monthly_cap_usd >= before.monthly_cap_usd
                {
                    *monthly_cap_usd = monthly_cap_usd.max(now.monthly_cap_usd);
                }
            }
            _ => {}
        }
    }
}

/// The delay a request was made under: `ready_at - requested_at`, never negative.
fn delay_of(pending: &PendingChange) -> i64 {
    (pending.ready_at - pending.requested_at)
        .num_seconds()
        .max(0)
}

/// The running time a request must have been observed for.
fn observed_needed(pending: &PendingChange) -> u32 {
    u32::try_from(delay_of(pending).min(i64::from(OBSERVED_CAP_SECS))).unwrap_or(OBSERVED_CAP_SECS)
}

/// Whether `pending` may be confirmed: `now >= ready_at` **and**
/// `observed_secs >= min(delay, OBSERVED_CAP_SECS)`, where the delay is
/// `ready_at - requested_at`.
pub fn is_ready(pending: &PendingChange, now: DateTime<Utc>) -> bool {
    now >= pending.ready_at && pending.observed_secs >= observed_needed(pending)
}

/// Whether a ready request has waited too long for its confirmation.
fn is_expired(pending: &PendingChange, now: DateTime<Utc>) -> bool {
    pending
        .ready_since
        .is_some_and(|since| (now - since).num_seconds() >= CONFIRM_WINDOW_SECS)
}

/// A request that became ready in this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    pub id: Id,
}

/// One verify pass: adds `min(elapsed_secs, OBSERVE_STEP_MAX_SECS)` to every waiting
/// request, stamps `ready_since` on those that became ready and returns them.
///
/// `elapsed_secs` is the running time since the previous pass, measured by the caller
/// (ideally on a monotonic clock). A `ready_since` that lies after `now` (the clock was
/// set back) is brought back to `now`, or cleared when the request is no longer ready, so
/// the 24-hour confirmation window cannot be stretched by a clock change.
pub fn tick(config: &mut NofapConfig, elapsed_secs: u32, now: DateTime<Utc>) -> Vec<Ready> {
    let step = elapsed_secs.min(OBSERVE_STEP_MAX_SECS);
    let mut became_ready = Vec::new();
    for pending in &mut config.pending {
        pending.observed_secs = pending.observed_secs.saturating_add(step);
        let ready = is_ready(pending, now);
        match pending.ready_since {
            None if ready => {
                pending.ready_since = Some(now);
                became_ready.push(Ready {
                    id: pending.id.clone(),
                });
            }
            Some(since) if since > now => {
                pending.ready_since = ready.then_some(now);
            }
            _ => {}
        }
    }
    became_ready
}

/// The user's fresh confirmation of a ready request: removes it from `config.pending` and
/// returns the change for the caller to apply. Fails when the id is unknown, the request
/// is not ready, or it was ready for longer than [`CONFIRM_WINDOW_SECS`].
pub fn confirm(config: &mut NofapConfig, id: &str, now: DateTime<Utc>) -> CoreResult<NofapChange> {
    let index = config
        .pending
        .iter()
        .position(|pending| pending.id == id)
        .ok_or_else(|| CoreError::NotFound("no such waiting request".into()))?;
    let pending = &config.pending[index];
    if !is_ready(pending, now) {
        return Err(CoreError::Invalid("the request is still waiting".into()));
    }
    if is_expired(pending, now) {
        return Err(CoreError::Invalid(
            "the request was not confirmed in time; ask again".into(),
        ));
    }
    Ok(config.pending.remove(index).change)
}

/// "Manter proteção": removes the request. `false` when the id is unknown.
pub fn cancel(config: &mut NofapConfig, id: &str) -> bool {
    let before = config.pending.len();
    config.pending.retain(|pending| pending.id != id);
    config.pending.len() != before
}

/// Drops ready requests that were not confirmed within [`CONFIRM_WINDOW_SECS`]; returns
/// their ids.
pub fn expire(config: &mut NofapConfig, now: DateTime<Utc>) -> Vec<Id> {
    let mut expired = Vec::new();
    config.pending.retain(|pending| {
        let gone = is_expired(pending, now);
        if gone {
            expired.push(pending.id.clone());
        }
        !gone
    });
    expired
}

/// How a waiting request is shown: its kind and its clock, never its payload.
///
/// `ready` is exactly "[`confirm`] would accept it now": a request whose confirmation
/// window has closed is not ready any more, also in the moments before [`expire`] drops
/// it (the app was closed for longer than the window), so the page never offers a
/// confirmation that would then be refused.
pub fn pending_view(pending: &PendingChange, now: DateTime<Utc>) -> PendingView {
    let ready = is_ready(pending, now) && !is_expired(pending, now);
    let by_clock = (pending.ready_at - now).num_seconds().max(0);
    let by_running = i64::from(observed_needed(pending).saturating_sub(pending.observed_secs));
    let remaining_secs = if ready || is_expired(pending, now) {
        0
    } else {
        u32::try_from(by_clock.max(by_running)).unwrap_or(u32::MAX)
    };
    PendingView {
        id: pending.id.clone(),
        kind: pending.change.kind(),
        requested_at: pending.requested_at,
        ready_at: pending.ready_at,
        ready,
        remaining_secs,
        expires_at: pending
            .ready_since
            .map(|since| since + Duration::seconds(CONFIRM_WINDOW_SECS)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use ubiqx_core::nofap::{
        ChangeKind, IfThenPlan, ResolverId, UiPrefs, VISION_MONTHLY_USD_DEFAULT,
    };

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap() + Duration::seconds(secs)
    }

    /// A module that is on with everything chosen, so every loosening has something to
    /// loosen.
    fn enabled() -> NofapConfig {
        let mut config = NofapConfig {
            enabled: true,
            enabled_at: Some(at(-86_400)),
            ..NofapConfig::default()
        };
        config.system.hosts = true;
        config.system.resolver = Some(ResolverId::CloudflareFamily);
        config.system.policies = true;
        config.system.hosts_user_sites = true;
        config.vision.enabled = true;
        config.vision.messaging_apps = true;
        config
    }

    fn scope(unknown: bool, media: bool, messaging: bool, private: bool) -> NofapChange {
        NofapChange::SetVisionScope {
            unknown_sites: unknown,
            media_apps: media,
            messaging_apps: messaging,
            private_windows: private,
        }
    }

    fn caps(hour: u32, day: u32, usd: f64) -> NofapChange {
        NofapChange::SetCaps {
            max_per_hour: hour,
            max_per_day: day,
            monthly_cap_usd: usd,
        }
    }

    const WAIT: Effect = Effect::After(600);

    /// Confirms a waiting request once it is ready and applies it, as the service does.
    fn confirm_and_apply(config: &mut NofapConfig, id: &str, now: DateTime<Utc>) {
        for waiting in &mut config.pending {
            waiting.observed_secs = OBSERVED_CAP_SECS;
        }
        let change = confirm(config, id, now).unwrap();
        assert!(apply_to_config(config, &change, now));
    }

    fn queued(config: &mut NofapConfig, change: NofapChange, id: &str) -> PendingChange {
        match request(config, change, id.into(), at(0)).unwrap() {
            Requested::Pending(waiting) => waiting,
            Requested::Apply(change) => panic!("{change:?} does not wait"),
        }
    }

    fn applied_now(config: &mut NofapConfig, change: NofapChange) {
        match request(config, change, "now".into(), at(10)).unwrap() {
            Requested::Apply(change) => assert!(apply_to_config(config, &change, at(10))),
            Requested::Pending(waiting) => panic!("{:?} waits", waiting.change),
        }
    }

    #[test]
    fn a_switch_turned_on_while_a_scope_request_waits_stays_on_after_its_confirmation() {
        let mut config = enabled();
        config.vision.messaging_apps = false;
        let waiting = queued(&mut config, scope(false, true, false, true), "scope");
        // Turned on at once while the request waits.
        applied_now(&mut config, scope(true, true, true, true));
        assert!(config.vision.messaging_apps);

        confirm_and_apply(&mut config, &waiting.id, at(700));
        assert!(!config.vision.unknown_sites, "what it asked to loosen goes");
        assert!(
            config.vision.messaging_apps,
            "what was turned on meanwhile stays"
        );
        assert!(config.vision.media_apps && config.vision.private_windows);
    }

    #[test]
    fn a_cap_raised_while_a_caps_request_waits_stays_raised_after_its_confirmation() {
        let mut config = enabled();
        let defaults = config.vision.clone();
        let hour = defaults.max_per_hour - 1;
        let waiting = queued(
            &mut config,
            caps(hour, defaults.max_per_day, defaults.monthly_cap_usd),
            "caps",
        );
        // A second request lowers the budget and waits too.
        let budget = queued(
            &mut config,
            caps(defaults.max_per_hour, defaults.max_per_day, 1.0),
            "budget",
        );
        // The budget is raised at once meanwhile.
        applied_now(
            &mut config,
            caps(defaults.max_per_hour, defaults.max_per_day, 5.0),
        );
        assert!((config.vision.monthly_cap_usd - 5.0).abs() < f64::EPSILON);

        confirm_and_apply(&mut config, &waiting.id, at(700));
        assert_eq!(
            config.vision.max_per_hour, hour,
            "the lowered cap is lowered"
        );
        assert!(
            (config.vision.monthly_cap_usd - 5.0).abs() < f64::EPSILON,
            "the raised budget stays raised"
        );
        // A request that asked to lower the budget, and waited, still lowers it.
        confirm_and_apply(&mut config, &budget.id, at(700));
        assert!((config.vision.monthly_cap_usd - 1.0).abs() < f64::EPSILON);
    }

    /// Every change in its tightening and its loosening direction.
    fn directions() -> Vec<(NofapChange, Effect)> {
        vec![
            (NofapChange::Disable, WAIT),
            (
                NofapChange::AddBlock {
                    host: "example.com".into(),
                },
                Effect::Now,
            ),
            (
                NofapChange::RemoveBlock {
                    id: "site-1".into(),
                },
                WAIT,
            ),
            (
                NofapChange::AddAllow {
                    host: "example.com".into(),
                },
                WAIT,
            ),
            (
                NofapChange::RemoveAllow {
                    id: "site-2".into(),
                },
                Effect::Now,
            ),
            (NofapChange::ForgetLearned, WAIT),
            (NofapChange::SetHosts { on: true }, Effect::Now),
            (NofapChange::SetHosts { on: false }, WAIT),
            (
                NofapChange::SetResolver {
                    resolver: Some(ResolverId::CloudflareFamily),
                },
                Effect::Now,
            ),
            (NofapChange::SetResolver { resolver: None }, WAIT),
            (NofapChange::SetPolicies { on: true }, Effect::Now),
            (NofapChange::SetPolicies { on: false }, WAIT),
            (NofapChange::SetHostsUserSites { on: true }, Effect::Now),
            (NofapChange::SetHostsUserSites { on: false }, WAIT),
            (NofapChange::SetVision { on: true }, Effect::Now),
            (NofapChange::SetVision { on: false }, WAIT),
            (scope(true, true, true, true), Effect::Now),
            (scope(false, true, true, true), WAIT),
            (scope(true, false, true, true), WAIT),
            (scope(true, true, false, true), WAIT),
            (scope(true, true, true, false), WAIT),
            (caps(20, 120, 2.0), Effect::Now),
            (caps(60, 1_000, 20.0), Effect::Now),
            (caps(19, 120, 2.0), WAIT),
            (caps(20, 119, 2.0), WAIT),
            (caps(20, 120, 1.99), WAIT),
            (caps(60, 1_000, 0.25), WAIT),
            (NofapChange::SetDelay { secs: 600 }, Effect::Now),
            (NofapChange::SetDelay { secs: 3_600 }, Effect::Now),
            (NofapChange::SetDelay { secs: 599 }, WAIT),
            (NofapChange::SetDelay { secs: 0 }, WAIT),
            (
                NofapChange::SetPrefs {
                    prefs: UiPrefs {
                        discreet: false,
                        show_streak: true,
                        daily_checkin: true,
                    },
                },
                Effect::Now,
            ),
            (
                NofapChange::SetValues {
                    text: "Quero estar presente.".into(),
                },
                Effect::Now,
            ),
            (
                NofapChange::SetValues {
                    text: String::new(),
                },
                Effect::Now,
            ),
            (NofapChange::SetPlans { plans: Vec::new() }, Effect::Now),
        ]
    }

    #[test]
    fn every_change_is_classified_in_both_directions() {
        let config = enabled();
        let table = directions();
        for (change, effect) in &table {
            assert_eq!(classify(&config, change), *effect, "{change:?}");
        }
        // The table covers every kind of change.
        for kind in ChangeKind::ALL {
            assert!(
                table.iter().any(|(change, _)| change.kind() == kind),
                "{kind:?} is missing"
            );
        }
    }

    #[test]
    fn while_the_module_is_off_everything_is_immediate() {
        let never_enabled = NofapConfig::default();
        let mut switched_off = enabled();
        switched_off.enabled = false;
        for (change, _) in directions() {
            assert_eq!(classify(&never_enabled, &change), Effect::Now, "{change:?}");
            assert_eq!(classify(&switched_off, &change), Effect::Now, "{change:?}");
        }
    }

    #[test]
    fn a_change_that_weakens_nothing_does_not_wait() {
        let mut config = enabled();
        config.system.hosts = false;
        config.system.resolver = None;
        config.system.policies = false;
        config.system.hosts_user_sites = false;
        config.vision.enabled = false;
        config.vision.messaging_apps = false;
        for change in [
            NofapChange::SetHosts { on: false },
            NofapChange::SetResolver { resolver: None },
            NofapChange::SetPolicies { on: false },
            NofapChange::SetHostsUserSites { on: false },
            NofapChange::SetVision { on: false },
            // Messaging stays off, the three others stay on.
            scope(true, true, false, true),
        ] {
            assert_eq!(classify(&config, &change), Effect::Now, "{change:?}");
        }
        // The module itself is still on: these always wait.
        assert_eq!(classify(&config, &NofapChange::Disable), WAIT);
        assert_eq!(classify(&config, &NofapChange::ForgetLearned), WAIT);
    }

    #[test]
    fn caps_are_compared_after_clamping() {
        let config = enabled();
        // Above the range is the top of the range, which is not lower than today's value.
        assert_eq!(
            classify(&config, &caps(10_000, 1_000_000, 500.0)),
            Effect::Now
        );
        // Below the range is the bottom of the range.
        assert_eq!(classify(&config, &caps(0, 120, 2.0)), WAIT);
        // Not a number becomes the default, which is today's value here.
        assert_eq!(config.vision.monthly_cap_usd, VISION_MONTHLY_USD_DEFAULT);
        assert_eq!(classify(&config, &caps(20, 120, f64::NAN)), Effect::Now);
        let mut generous = enabled();
        generous.vision.monthly_cap_usd = 10.0;
        assert_eq!(classify(&generous, &caps(20, 120, f64::NAN)), WAIT);
        assert_eq!(classify(&generous, &caps(20, 120, f64::NEG_INFINITY)), WAIT);
    }

    #[test]
    fn the_delay_in_force_is_the_wait() {
        let mut config = enabled();
        config.delay_secs = 1_800;
        assert_eq!(
            classify(&config, &NofapChange::Disable),
            Effect::After(1_800)
        );
        config.delay_secs = 0;
        assert_eq!(classify(&config, &NofapChange::Disable), Effect::After(0));
        // A hand-edited row cannot ask for more than a day.
        config.delay_secs = u32::MAX;
        assert_eq!(
            classify(&config, &NofapChange::Disable),
            Effect::After(DELAY_MAX_SECS)
        );
    }

    #[test]
    fn a_tightening_request_is_handed_back_to_apply() {
        let mut config = enabled();
        let change = NofapChange::AddBlock {
            host: "example.com".into(),
        };
        assert_eq!(
            request(&mut config, change.clone(), "p1".into(), at(0)).unwrap(),
            Requested::Apply(change)
        );
        assert!(config.pending.is_empty());
    }

    #[test]
    fn a_loosening_request_waits_the_delay() {
        let mut config = enabled();
        let requested = request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        let expected = PendingChange {
            id: "p1".into(),
            change: NofapChange::Disable,
            requested_at: at(0),
            ready_at: at(600),
            observed_secs: 0,
            ready_since: None,
        };
        assert_eq!(requested, Requested::Pending(expected.clone()));
        assert_eq!(config.pending, vec![expected]);
        assert!(config.enabled, "nothing is applied yet");
    }

    #[test]
    fn asking_twice_does_not_restart_the_clock_or_fill_the_queue() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        let again = request(&mut config, NofapChange::Disable, "p2".into(), at(300)).unwrap();
        match again {
            Requested::Pending(pending) => {
                assert_eq!(pending.id, "p1");
                assert_eq!(pending.ready_at, at(600));
            }
            other => panic!("expected the waiting entry, got {other:?}"),
        }
        assert_eq!(config.pending.len(), 1);
        // A different change is a different request.
        request(
            &mut config,
            NofapChange::ForgetLearned,
            "p3".into(),
            at(300),
        )
        .unwrap();
        assert_eq!(config.pending.len(), 2);
    }

    #[test]
    fn at_most_twenty_requests_wait() {
        let mut config = enabled();
        for n in 0..MAX_PENDING {
            let change = NofapChange::AddAllow {
                host: format!("site{n}.example"),
            };
            request(&mut config, change, format!("p{n}"), at(0)).unwrap();
        }
        let one_more = NofapChange::AddAllow {
            host: "more.example".into(),
        };
        assert!(matches!(
            request(&mut config, one_more, "p-more".into(), at(0)),
            Err(CoreError::Invalid(_))
        ));
        assert_eq!(config.pending.len(), MAX_PENDING);
        // A tightening change is never refused by a full queue.
        let block = NofapChange::AddBlock {
            host: "example.com".into(),
        };
        assert!(matches!(
            request(&mut config, block, "p-block".into(), at(0)),
            Ok(Requested::Apply(_))
        ));
    }

    #[test]
    fn lowering_the_delay_waits_the_old_delay() {
        let mut config = enabled();
        config.delay_secs = 1_800;
        let requested = request(
            &mut config,
            NofapChange::SetDelay { secs: 0 },
            "p1".into(),
            at(0),
        )
        .unwrap();
        match requested {
            Requested::Pending(pending) => assert_eq!(pending.ready_at, at(1_800)),
            other => panic!("expected a waiting request, got {other:?}"),
        }
        assert_eq!(config.delay_secs, 1_800, "the old delay is still in force");
        // A request made meanwhile waits the old delay too.
        match request(&mut config, NofapChange::Disable, "p2".into(), at(60)).unwrap() {
            Requested::Pending(pending) => assert_eq!(pending.ready_at, at(1_860)),
            other => panic!("expected a waiting request, got {other:?}"),
        }
        // Raising is immediate, and does not move a request that already waits.
        let raise = NofapChange::SetDelay { secs: 7_200 };
        assert_eq!(
            request(&mut config, raise.clone(), "p3".into(), at(90)).unwrap(),
            Requested::Apply(raise.clone())
        );
        assert!(apply_to_config(&mut config, &raise, at(90)));
        assert_eq!(config.delay_secs, 7_200);
        assert_eq!(config.pending[1].ready_at, at(1_860));
    }

    /// Runs verify passes of `step` seconds from `from` to `to`.
    fn run(config: &mut NofapConfig, from: i64, to: i64, step: u32) -> Vec<Ready> {
        let mut ready = Vec::new();
        let mut now = from;
        while now + i64::from(step) <= to {
            now += i64::from(step);
            ready.extend(tick(config, step, at(now)));
        }
        ready
    }

    #[test]
    fn a_request_is_ready_after_the_delay_of_running_time() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        assert!(run(&mut config, 0, 480, 120).is_empty());
        assert!(!is_ready(&config.pending[0], at(480)));
        assert!(matches!(
            confirm(&mut config, "p1", at(480)),
            Err(CoreError::Invalid(_))
        ));
        assert_eq!(
            run(&mut config, 480, 600, 120),
            vec![Ready { id: "p1".into() }]
        );
        assert_eq!(config.pending[0].ready_since, Some(at(600)));
        // Ready, not applied: it is reported once and waits for the confirmation.
        assert!(run(&mut config, 600, 1_200, 120).is_empty());
        assert!(config.enabled);
        assert_eq!(
            confirm(&mut config, "p1", at(1_200)).unwrap(),
            NofapChange::Disable
        );
        assert!(config.pending.is_empty());
    }

    #[test]
    fn a_forward_clock_jump_does_not_make_a_request_ready() {
        let mut config = enabled();
        config.delay_secs = 86_400;
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        // The clock is set two days ahead; one verify pass later nothing is ready, even
        // when the pass claims that two days went by.
        let jumped = 2 * 86_400;
        assert!(tick(&mut config, u32::MAX, at(jumped)).is_empty());
        assert_eq!(config.pending[0].observed_secs, OBSERVE_STEP_MAX_SECS);
        assert!(!is_ready(&config.pending[0], at(jumped)));
        assert!(confirm(&mut config, "p1", at(jumped)).is_err());
        // It still costs 15 minutes of running time: five more passes are not enough …
        for pass in 1..=4 {
            assert!(
                tick(&mut config, 150, at(jumped + pass * 150)).is_empty(),
                "pass {pass}"
            );
        }
        assert_eq!(config.pending[0].observed_secs, 750);
        // … the sixth is.
        assert_eq!(
            tick(&mut config, 150, at(jumped + 750)),
            vec![Ready { id: "p1".into() }]
        );
    }

    #[test]
    fn a_short_delay_needs_its_whole_length_of_running_time() {
        let mut config = enabled();
        config.delay_secs = 300;
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        // The clock jumps past the delay at once.
        assert!(tick(&mut config, 120, at(10_000)).is_empty());
        assert!(tick(&mut config, 120, at(10_120)).is_empty());
        assert_eq!(
            tick(&mut config, 120, at(10_240)),
            vec![Ready { id: "p1".into() }]
        );
    }

    #[test]
    fn a_backward_clock_jump_changes_nothing() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        run(&mut config, 0, 360, 120);
        let before = config.pending[0].clone();
        // The clock is set back a day: the request is the same, and not ready.
        assert!(tick(&mut config, 0, at(-86_400)).is_empty());
        assert_eq!(config.pending[0], before);
        assert_eq!(config.pending[0].ready_at, at(600));
        assert!(!is_ready(&config.pending[0], at(-86_400)));
        // Running on under the wrong clock never gets it ready before `ready_at`.
        assert!(run(&mut config, -86_400, -80_000, 120).is_empty());
        assert!(config.pending[0].observed_secs >= OBSERVED_CAP_SECS);
        assert!(!is_ready(&config.pending[0], at(599)));
        assert!(is_ready(&config.pending[0], at(600)));
    }

    #[test]
    fn a_request_made_under_a_clock_set_back_still_costs_running_time() {
        let mut config = enabled();
        config.delay_secs = 86_400;
        // The clock is set back two days, the request is made, the clock is put right.
        request(
            &mut config,
            NofapChange::Disable,
            "p1".into(),
            at(-2 * 86_400),
        )
        .unwrap();
        assert!(
            !is_ready(&config.pending[0], at(0)),
            "the delay passed on paper only"
        );
        assert!(run(&mut config, 0, 840, 120).is_empty());
        assert_eq!(
            run(&mut config, 840, 960, 120),
            vec![Ready { id: "p1".into() }]
        );
    }

    #[test]
    fn a_ready_request_expires_after_a_day_without_confirmation() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        request(
            &mut config,
            NofapChange::ForgetLearned,
            "p2".into(),
            at(300),
        )
        .unwrap();
        run(&mut config, 0, 600, 120);
        assert_eq!(config.pending[0].ready_since, Some(at(600)));
        assert!(expire(&mut config, at(600 + 86_399)).is_empty());
        // Confirming in the last second still works; one second later it does not.
        let mut copy = config.clone();
        assert!(confirm(&mut copy, "p1", at(600 + 86_399)).is_ok());
        assert!(matches!(
            confirm(&mut config, "p1", at(600 + 86_400)),
            Err(CoreError::Invalid(_))
        ));
        assert_eq!(expire(&mut config, at(600 + 86_400)), vec!["p1".to_owned()]);
        // The other request became ready later and is still there.
        assert_eq!(config.pending.len(), 1);
        assert_eq!(config.pending[0].id, "p2");
        // A request that is still waiting never expires.
        let mut waiting = enabled();
        request(&mut waiting, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        assert!(expire(&mut waiting, at(10 * 86_400)).is_empty());
    }

    #[test]
    fn a_request_whose_window_closed_while_the_app_was_shut_is_not_offered() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        run(&mut config, 0, 600, 120);
        assert!(pending_view(&config.pending[0], at(600)).ready);
        // The app is closed for two days: no verify pass ran, so nothing dropped the
        // request yet. The view and `confirm` must agree that it is over.
        let later = 600 + 2 * 86_400;
        let view = pending_view(&config.pending[0], at(later));
        assert!(!view.ready);
        assert_eq!(view.remaining_secs, 0);
        assert_eq!(view.expires_at, Some(at(600 + 86_400)));
        assert!(confirm(&mut config.clone(), "p1", at(later)).is_err());
        // In the last second of the window both still say yes.
        let last = 600 + 86_399;
        assert!(pending_view(&config.pending[0], at(last)).ready);
        assert!(confirm(&mut config.clone(), "p1", at(last)).is_ok());
        // The first verify pass after the start drops it, and the module is still on.
        assert!(tick(&mut config, 120, at(later)).is_empty());
        assert_eq!(expire(&mut config, at(later)), vec!["p1".to_owned()]);
        assert!(config.enabled && config.pending.is_empty());
    }

    #[test]
    fn a_walk_with_clock_jumps_restarts_and_gaps_never_shortens_the_wait() {
        // A random walk over what can happen to a waiting request: verify passes of any
        // length, the clock set forward or back by minutes to more than a year, the app
        // closed for a while (time passes, no pass runs), a restart (the configuration is
        // what was stored). Whenever `confirm` would accept: the wall clock has reached
        // `ready_at`, and at least `min(delay, 15 minutes)` of running time was credited,
        // at most 150 seconds per pass. Without a clock change the full delay has passed.
        struct Walk(u64);
        impl Walk {
            fn below(&mut self, n: u64) -> u64 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                self.0 % n
            }
            fn pick<T: Copy>(&mut self, items: &[T]) -> T {
                items[usize::try_from(self.below(items.len() as u64)).unwrap()]
            }
        }
        let mut walk = Walk(0x0BAD_C0DE_1234_5678);
        let mut confirmed = 0;
        for case in 0..1_500 {
            let delay = walk.pick(&[0u32, 1, 60, 300, 600, 899, 900, 901, 3_600, 86_400]);
            let mut config = enabled();
            config.delay_secs = delay;
            let (mut wall, mut real, mut credited) = (0i64, 0i64, 0i64);
            let mut honest = true;
            request(&mut config, NofapChange::Disable, "p".into(), at(wall)).unwrap();
            for _ in 0..300 {
                match walk.below(10) {
                    0 => {
                        wall += walk.pick(&[1_000i64, 86_400, 3 * 86_400, 400 * 86_400]);
                        honest = false;
                    }
                    1 => {
                        wall -= walk.pick(&[1_000i64, 86_400, 3 * 86_400, 400 * 86_400]);
                        honest = false;
                    }
                    2 => {
                        let stored = serde_json::to_string(&config).unwrap();
                        config = serde_json::from_str(&stored).unwrap();
                        config.sanitize();
                    }
                    3 => {
                        let gap = walk.pick(&[60i64, 600, 7_200, 90_000]);
                        wall += gap;
                        real += gap;
                    }
                    _ => {
                        let elapsed = walk.pick(&[1u32, 30, 120, 120, 150, 151, 500, 100_000]);
                        wall += i64::from(elapsed);
                        real += i64::from(elapsed);
                        credited += i64::from(elapsed.min(OBSERVE_STEP_MAX_SECS));
                        tick(&mut config, elapsed, at(wall));
                        expire(&mut config, at(wall));
                    }
                }
                let Some(pending) = config.pending.first() else {
                    break;
                };
                let view = pending_view(pending, at(wall));
                let accepted = confirm(&mut config.clone(), "p", at(wall)).is_ok();
                assert_eq!(view.ready, accepted, "case {case}: the view and confirm");
                if accepted {
                    confirmed += 1;
                    assert!(at(wall) >= pending.ready_at, "case {case}");
                    assert!(
                        credited >= i64::from(delay.min(OBSERVED_CAP_SECS)),
                        "case {case}: accepted after {credited} s of running time, delay {delay}"
                    );
                    if honest {
                        assert!(real >= i64::from(delay), "case {case}: {real} s of {delay}");
                    }
                }
            }
        }
        assert!(
            confirmed > 1_000,
            "the walk reaches the ready state: {confirmed}"
        );
    }

    #[test]
    fn the_confirmation_window_cannot_be_stretched_by_setting_the_clock_back() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        run(&mut config, 0, 600, 120);
        // The clock was a year ahead when the request became ready, and is put right.
        config.pending[0].ready_since = Some(at(365 * 86_400));
        assert!(tick(&mut config, 120, at(720)).is_empty());
        assert_eq!(config.pending[0].ready_since, Some(at(720)));
        assert_eq!(expire(&mut config, at(720 + 86_400)), vec!["p1".to_owned()]);
        // Set back to before `ready_at`: not ready any more, so no window is open.
        let mut early = enabled();
        request(&mut early, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        run(&mut early, 0, 600, 120);
        assert!(tick(&mut early, 120, at(100)).is_empty());
        assert_eq!(early.pending[0].ready_since, None);
        assert!(confirm(&mut early, "p1", at(100)).is_err());
    }

    #[test]
    fn with_a_delay_of_zero_the_request_is_ready_and_still_needs_the_confirmation() {
        let mut config = enabled();
        config.delay_secs = 0;
        let requested = request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        match requested {
            Requested::Pending(pending) => {
                assert_eq!(pending.ready_at, at(0));
                assert_eq!(pending.ready_since, Some(at(0)));
                assert!(is_ready(&pending, at(0)));
            }
            other => panic!("expected a waiting request, got {other:?}"),
        }
        assert!(config.enabled, "not applied without the confirmation");
        assert!(pending_view(&config.pending[0], at(0)).ready);
        // It was already ready, so a verify pass does not announce it again.
        assert!(tick(&mut config, 120, at(120)).is_empty());
        assert_eq!(
            confirm(&mut config, "p1", at(121)).unwrap(),
            NofapChange::Disable
        );
    }

    #[test]
    fn cancelling_is_immediate_and_only_removes_that_request() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        request(&mut config, NofapChange::ForgetLearned, "p2".into(), at(0)).unwrap();
        assert!(cancel(&mut config, "p1"));
        assert!(!cancel(&mut config, "p1"));
        assert!(!cancel(&mut config, "unknown"));
        assert_eq!(config.pending.len(), 1);
        assert_eq!(config.pending[0].id, "p2");
        assert!(matches!(
            confirm(&mut config, "p1", at(86_400)),
            Err(CoreError::NotFound(_))
        ));
    }

    #[test]
    fn apply_to_config_changes_the_configuration_part() {
        let mut config = enabled();
        request(&mut config, NofapChange::ForgetLearned, "p1".into(), at(0)).unwrap();
        for site_change in [
            NofapChange::AddBlock {
                host: "example.com".into(),
            },
            NofapChange::RemoveBlock {
                id: "site-1".into(),
            },
            NofapChange::AddAllow {
                host: "example.com".into(),
            },
            NofapChange::RemoveAllow {
                id: "site-2".into(),
            },
            NofapChange::ForgetLearned,
        ] {
            let before = config.clone();
            assert!(
                !apply_to_config(&mut config, &site_change, at(0)),
                "{site_change:?}"
            );
            assert_eq!(config, before);
        }
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetHosts { on: false },
            at(0)
        ));
        assert!(!config.system.hosts);
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetPolicies { on: false },
            at(0)
        ));
        assert!(!config.system.policies);
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetHostsUserSites { on: false },
            at(0)
        ));
        assert!(!config.system.hosts_user_sites);
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetVision { on: false },
            at(0)
        ));
        assert!(!config.vision.enabled);
        assert!(apply_to_config(
            &mut config,
            &scope(false, false, true, false),
            at(0)
        ));
        assert_eq!(
            (
                config.vision.unknown_sites,
                config.vision.media_apps,
                config.vision.messaging_apps,
                config.vision.private_windows
            ),
            (false, false, true, false)
        );
        // Out-of-range values are clamped on the way in.
        assert!(apply_to_config(&mut config, &caps(500, 0, 99.0), at(0)));
        assert_eq!(
            (config.vision.max_per_hour, config.vision.max_per_day),
            (60, 1)
        );
        assert_eq!(config.vision.monthly_cap_usd, 20.0);
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetDelay { secs: u32::MAX },
            at(0)
        ));
        assert_eq!(config.delay_secs, DELAY_MAX_SECS);
        let prefs = UiPrefs {
            discreet: false,
            show_streak: true,
            daily_checkin: true,
        };
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetPrefs {
                prefs: prefs.clone()
            },
            at(0)
        ));
        assert_eq!(config.ui, prefs);
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetValues {
                text: "  Quero estar presente.  ".into()
            },
            at(0)
        ));
        assert_eq!(config.values, "Quero estar presente.");
        let plans = vec![
            IfThenPlan {
                when: "estiver cansado".into(),
                then: "sair para caminhar".into()
            };
            5
        ];
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetPlans { plans },
            at(0)
        ));
        assert_eq!(config.plans.len(), 3);
        assert_eq!(
            config.pending.len(),
            1,
            "other waiting requests are untouched"
        );
    }

    #[test]
    fn restoring_the_network_ends_when_a_resolver_is_chosen_again() {
        let mut config = enabled();
        config.system.network_restored = true;
        let relink = NofapChange::SetResolver {
            resolver: Some(ResolverId::CloudflareFamily),
        };
        assert_eq!(classify(&config, &relink), Effect::Now);
        assert!(apply_to_config(&mut config, &relink, at(0)));
        assert!(!config.system.network_restored);
        assert!(apply_to_config(
            &mut config,
            &NofapChange::SetResolver { resolver: None },
            at(0)
        ));
        assert_eq!(config.system.resolver, None);
    }

    #[test]
    fn disabling_switches_off_and_drops_what_was_waiting() {
        let mut config = enabled();
        request(&mut config, NofapChange::Disable, "p1".into(), at(0)).unwrap();
        request(&mut config, NofapChange::ForgetLearned, "p2".into(), at(0)).unwrap();
        run(&mut config, 0, 600, 120);
        let change = confirm(&mut config, "p1", at(600)).unwrap();
        assert!(apply_to_config(&mut config, &change, at(600)));
        assert!(!config.enabled);
        assert!(config.pending.is_empty());
        // What the user chose stays chosen for the next time the module is switched on.
        assert!(config.system.hosts);
        assert_eq!(config.enabled_at, Some(at(-86_400)));
    }

    #[test]
    fn the_view_shows_the_clock_and_never_the_payload() {
        let mut config = enabled();
        let change = NofapChange::AddAllow {
            host: "example.com".into(),
        };
        request(&mut config, change, "p1".into(), at(0)).unwrap();
        let view = pending_view(&config.pending[0], at(100));
        assert_eq!(
            view,
            PendingView {
                id: "p1".into(),
                kind: ChangeKind::AddAllow,
                requested_at: at(0),
                ready_at: at(600),
                ready: false,
                remaining_secs: 600,
                expires_at: None,
            }
        );
        assert!(!serde_json::to_string(&view).unwrap().contains("example"));
        // The clock has passed but the running time has not: what is left is running time.
        config.pending[0].observed_secs = 200;
        assert_eq!(
            pending_view(&config.pending[0], at(900)).remaining_secs,
            400
        );
        // Both left: the larger one.
        assert_eq!(
            pending_view(&config.pending[0], at(500)).remaining_secs,
            400
        );
        assert_eq!(
            pending_view(&config.pending[0], at(100)).remaining_secs,
            500
        );
        run(&mut config, 900, 1_500, 120);
        let ready = pending_view(&config.pending[0], at(1_500));
        assert!(ready.ready);
        assert_eq!(ready.remaining_secs, 0);
        assert_eq!(
            ready.expires_at,
            config.pending[0]
                .ready_since
                .map(|since| since + Duration::seconds(86_400))
        );
    }
}
