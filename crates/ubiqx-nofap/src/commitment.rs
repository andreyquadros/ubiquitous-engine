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
//! Before the module is enabled for the first time everything is `Now`. Cancelling a
//! pending change is always immediate.
//!
//! Seeded by the foundation unit: complete types and constants, signatures only.
#![allow(unused_variables)]

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::{NofapChange, NofapConfig, PendingChange, PendingView};
use ubiqx_core::{CoreResult, Id};

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

/// Decides the effect of `change` on `config`. The `match` inside is exhaustive with no
/// wildcard arm, so a new change kind does not compile until it is classified.
pub fn classify(config: &NofapConfig, change: &NofapChange) -> Effect {
    todo!()
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
pub fn request(
    config: &mut NofapConfig,
    change: NofapChange,
    id: Id,
    now: DateTime<Utc>,
) -> CoreResult<Requested> {
    todo!()
}

/// Applies the part of `change` that lives in the configuration and sanitises it. Returns
/// `false` for a change that only concerns the site lists.
pub fn apply_to_config(config: &mut NofapConfig, change: &NofapChange, now: DateTime<Utc>) -> bool {
    todo!()
}

/// Whether `pending` may be confirmed: `now >= ready_at` **and**
/// `observed_secs >= min(delay, OBSERVED_CAP_SECS)`, where the delay is
/// `ready_at - requested_at`.
pub fn is_ready(pending: &PendingChange, now: DateTime<Utc>) -> bool {
    todo!()
}

/// A request that became ready in this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    pub id: Id,
}

/// One verify pass: adds `min(elapsed_secs, OBSERVE_STEP_MAX_SECS)` to every waiting
/// request, stamps `ready_since` on those that became ready and returns them.
pub fn tick(config: &mut NofapConfig, elapsed_secs: u32, now: DateTime<Utc>) -> Vec<Ready> {
    todo!()
}

/// The user's fresh confirmation of a ready request: removes it from `config.pending` and
/// returns the change for the caller to apply. Fails when the id is unknown or the request
/// is not ready.
pub fn confirm(config: &mut NofapConfig, id: &str, now: DateTime<Utc>) -> CoreResult<NofapChange> {
    todo!()
}

/// "Manter proteção": removes the request. `false` when the id is unknown.
pub fn cancel(config: &mut NofapConfig, id: &str) -> bool {
    todo!()
}

/// Drops ready requests that were not confirmed within [`CONFIRM_WINDOW_SECS`]; returns
/// their ids.
pub fn expire(config: &mut NofapConfig, now: DateTime<Utc>) -> Vec<Id> {
    todo!()
}

/// How a waiting request is shown: its kind and its clock, never its payload.
pub fn pending_view(pending: &PendingChange, now: DateTime<Utc>) -> PendingView {
    todo!()
}
