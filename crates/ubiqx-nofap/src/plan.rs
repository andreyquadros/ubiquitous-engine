//! The system plan: what one product wants applied to the machine, how it travels to the
//! elevated helper, how the helper validates it as root, and how an unprivileged
//! observation of the machine is judged against it.
//!
//! Everything here is pure. The adapter (`ubiqx_platform::sysblock`) runs commands and
//! reads files; it holds no decision.
//!
//! Seeded by the foundation unit: complete types and constants, signatures only.
#![allow(unused_variables)]

use std::collections::BTreeMap;

use ubiqx_core::nofap::{
    Issue, NofapConfig, NofapSite, Os, OwnerWants, PlannedChange, ResolverId, SystemObservation,
    SystemPlan, Tier, TierState,
};
use ubiqx_core::ProductId;

use crate::gate::Gate;
use crate::matcher::RuleSet;

/// Longest `--plan-b64` value the app sends, per system. Windows elevation goes through
/// `ShellExecuteEx`, whose parameter string is reported to fail somewhere above 2,048
/// characters; the budget is raised to what the Windows smoke job measures.
pub const ARG_BUDGET_UNIX: usize = 24_000;
pub const ARG_BUDGET_WINDOWS: usize = 1_800;
/// Longest `--plan-b64` value the helper accepts, on every system.
pub const HELPER_ARG_MAX: usize = 24_000;

/// Exit codes of the helper.
pub const EXIT_APPLIED: i32 = 0;
pub const EXIT_INVALID_PLAN: i32 = 2;
/// A per-tier report is in the state file.
pub const EXIT_PARTIAL: i32 = 3;
pub const EXIT_FAILED: i32 = 4;
pub const EXIT_GATED: i32 = 5;

/// How often the verify pass re-reads the system tiers (unprivileged; it never elevates).
pub const VERIFY_EVERY_SECS: u64 = 120;
/// How often the canary is resolved while the hosts block is applied.
pub const CANARY_EVERY_SECS: i64 = 1_800;
/// Windows: how often the verify pass asks for the full DNS read (`ObserveOpts::full`),
/// and how often while the quick registry read finds nothing usable. The full read is
/// also asked for at apply, when the page opens and on "Verificar agora".
pub const FULL_READ_EVERY_SECS: i64 = 1_800;
pub const FULL_READ_RETRY_SECS: i64 = 600;
/// The drift card is shown once per drift episode and at most once in this long.
pub const DRIFT_CARD_EVERY_SECS: i64 = 21_600;

/// [`ARG_BUDGET_WINDOWS`] or [`ARG_BUDGET_UNIX`].
pub fn arg_budget(os: Os) -> usize {
    todo!()
}

/// A plan and what did not fit in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltPlan {
    pub plan: SystemPlan,
    /// User sites dropped to stay inside the argument budget; they stay in the in-app
    /// guard only, and the page says how many.
    pub extras_left_out: u32,
}

/// The plan `owner` would apply now. `extra` holds the user's blocked hosts (never a
/// learned one) when `system.hosts_user_sites` is on, most recently added first, as many
/// as fit [`arg_budget`]; the resolver is left out while `system.network_restored` is set.
pub fn build_plan(
    config: &NofapConfig,
    sites: &[NofapSite],
    owner: ProductId,
    os: Os,
) -> BuiltPlan {
    todo!()
}

/// The `--plan-b64` value: base64url without padding of the plan's compact JSON.
pub fn encode_plan(plan: &SystemPlan) -> String {
    todo!()
}

/// Why the helper refuses a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// Longer than [`HELPER_ARG_MAX`].
    TooLong {
        len: usize,
    },
    /// Not base64url, not JSON, or a value outside a closed set (owner, resolver).
    Malformed,
    Version {
        found: u8,
    },
    TooManyNames {
        count: usize,
    },
    InvalidName {
        name: String,
    },
    /// A name under the never-block list.
    NeverBlock {
        name: String,
    },
    /// A requested tier is closed for this system.
    Gated {
        tier: Tier,
    },
}

impl PlanError {
    /// [`EXIT_GATED`] for a gated tier, [`EXIT_INVALID_PLAN`] otherwise.
    pub fn exit_code(&self) -> i32 {
        todo!()
    }
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for PlanError {}

/// Decodes a `--plan-b64` value. Checks the length and the encoding only; [`validate`]
/// does the rest.
pub fn decode_plan(arg: &str) -> Result<SystemPlan, PlanError> {
    todo!()
}

/// Validation as root: version 1; at most `MAX_PLAN_EXTRA` names, each passing
/// `hosts::is_valid_hostname` and not under `never_block`; every requested tier open in
/// `gate`.
pub fn validate(plan: &SystemPlan, never_block: &RuleSet, gate: &Gate) -> Result<(), PlanError> {
    todo!()
}

/// What the plan asks for, as the state file records it.
pub fn wants_of(plan: &SystemPlan) -> OwnerWants {
    todo!()
}

/// The union of every owner's wants: a tier is applied while anybody wants it; `extra` is
/// every owner's names without duplicates, in owner order.
pub fn union(owners: &BTreeMap<ProductId, OwnerWants>) -> OwnerWants {
    todo!()
}

/// Everything [`assess`] reads besides the observation.
#[derive(Debug, Clone)]
pub struct AssessInput<'a> {
    pub config: &'a NofapConfig,
    /// What this product would apply now ([`build_plan`]).
    pub plan: &'a SystemPlan,
    pub os: Os,
    pub gate: &'a Gate,
    /// The names of the hosts tier (`list::hosts_tier()` in production).
    pub hosts_tier: &'a [&'a str],
    /// Consecutive verify passes in which the probe name did not resolve.
    pub probe_fail_streak: u32,
}

/// The state of the three tiers and the issues an observation shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemAssessment {
    pub hosts: TierState,
    pub resolver: TierState,
    pub policies: TierState,
    pub issues: Vec<Issue>,
    /// Names the app would add to the hosts block at the next apply.
    pub hosts_waiting: u32,
}

/// Turns an observation into tier states and issues (the tables of design sections 2.2,
/// 2.3 and 2.4). An interface is "new" when `config.internal.previous_dns` does not name
/// it. `observation.canary` of `None` means "not looked up", not "fine": the caller puts
/// the answer of its last lookup there, so `Issue::HostsNotHonoured` does not come and go
/// between two lookups.
pub fn assess(input: &AssessInput<'_>, observation: &SystemObservation) -> SystemAssessment {
    todo!()
}

/// The exact list of changes applying `plan` would make on a machine that looks like
/// `observation`, for the user to read before the OS prompt.
pub fn preview(plan: &SystemPlan, observation: &SystemObservation, os: Os) -> Vec<PlannedChange> {
    todo!()
}

/// Interfaces (by id) the helper may reset to automatic when it undoes the resolver tier:
/// only those whose servers are still exactly the provider's set. An interface the user
/// changed since is left alone.
pub fn reset_targets(observation: &SystemObservation, resolver: ResolverId) -> Vec<String> {
    todo!()
}

/// Whether seeing a host in front means DNS is being bypassed: the host is one of the
/// names of the hosts tier and the hosts block is applied.
pub fn bypass_suspected(in_hosts_tier: bool, hosts: &TierState) -> bool {
    todo!()
}
