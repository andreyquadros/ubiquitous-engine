//! The system plan: what one product wants applied to the machine, how it travels to the
//! elevated helper, how the helper validates it as root, and how an unprivileged
//! observation of the machine is judged against it.
//!
//! Everything here is pure. The adapter (`ubiqx_platform::sysblock`) runs commands and
//! reads files; it holds no decision.
//!
//! Three functions have a twin that takes its data as arguments ([`build_plan_with`],
//! [`assess_with`], [`preview_with`]); the plain forms hand them the embedded never-block
//! rules and hosts tier. The helper and the app must agree on the names of a block down
//! to the byte, so both get them from one place: [`hosts_names`].

use std::collections::{BTreeMap, HashSet};

use data_encoding::BASE64URL_NOPAD;
use ubiqx_core::nofap::{
    preview as line, reason, BrowserFamily, InterfaceDns, Issue, NofapConfig, NofapSite, Os,
    OwnerWants, PlannedChange, PolicyCreated, PolicyObservation, ResolverId, ResolverObservation,
    SiteList, SiteOrigin, SystemObservation, SystemPlan, SystemState, Tier, TierState,
    CONFIG_VERSION, MAX_PLAN_EXTRA,
};
use ubiqx_core::ProductId;

use crate::gate::Gate;
use crate::hosts::{self, BlockInspection};
use crate::matcher::{self, RuleSet};
use crate::policies;
use crate::resolvers::{self, RESOLVERS, UNREACHABLE_AFTER_FAILS};

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
    match os {
        Os::Windows => ARG_BUDGET_WINDOWS,
        Os::Macos | Os::Linux => ARG_BUDGET_UNIX,
    }
}

/// A plan and what did not fit in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltPlan {
    pub plan: SystemPlan,
    /// User sites that are not in the plan and stay in the in-app guard only: those dropped
    /// to stay inside the argument budget, and those the helper would refuse (a name that
    /// is not a public hostname, or one under the never-block rules). The page says how
    /// many.
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
    build_plan_with(config, sites, owner, os, matcher::never_block())
}

/// Length of the unpadded base64 of `bytes` bytes.
fn base64_len(bytes: usize) -> usize {
    (bytes * 4).div_ceil(3)
}

/// [`build_plan`] with the never-block rules as an argument. A name the helper would
/// refuse is left out here, because one refused name makes the helper refuse the whole
/// plan.
pub fn build_plan_with(
    config: &NofapConfig,
    sites: &[NofapSite],
    owner: ProductId,
    os: Os,
    never_block: &RuleSet,
) -> BuiltPlan {
    let system = &config.system;
    let mut plan = SystemPlan {
        v: CONFIG_VERSION,
        owner,
        hosts: system.hosts,
        extra: Vec::new(),
        resolver: if system.network_restored {
            None
        } else {
            system.resolver
        },
        policies: system.policies,
    };
    let mut extras_left_out = 0u32;
    if system.hosts && system.hosts_user_sites {
        let mut typed: Vec<&NofapSite> = sites
            .iter()
            .filter(|site| site.list == SiteList::Block && site.origin == SiteOrigin::User)
            .collect();
        typed.sort_by_key(|site| std::cmp::Reverse(site.created_at));

        let budget = arg_budget(os);
        // The names are plain ASCII, so each adds its length, two quotes and a comma.
        let mut json_len = serde_json::to_vec(&plan).map_or(usize::MAX, |json| json.len());
        let mut full = false;
        let mut seen: HashSet<String> = HashSet::new();
        for site in typed {
            let name = site
                .host
                .as_deref()
                .and_then(matcher::normalize_host)
                .filter(|name| admissible(name, never_block).is_ok());
            let Some(name) = name else {
                extras_left_out = extras_left_out.saturating_add(1);
                continue;
            };
            if !seen.insert(name.clone()) {
                continue;
            }
            let added = name.len() + 2 + usize::from(!plan.extra.is_empty());
            let fits = !full
                && plan.extra.len() < MAX_PLAN_EXTRA
                && base64_len(json_len.saturating_add(added)) <= budget;
            if fits {
                json_len += added;
                plan.extra.push(name);
            } else {
                // The most recent names that fit are kept: nothing older jumps the queue.
                full = true;
                extras_left_out = extras_left_out.saturating_add(1);
            }
        }
    }
    BuiltPlan {
        plan,
        extras_left_out,
    }
}

/// `plan` without the tiers `gate` keeps closed. The helper refuses a plan that asks for
/// a closed tier, so this is what the app sends.
pub fn without_gated(plan: &SystemPlan, gate: &Gate) -> SystemPlan {
    let hosts = plan.hosts && gate.open(Tier::Hosts);
    SystemPlan {
        v: plan.v,
        owner: plan.owner,
        hosts,
        extra: if hosts {
            plan.extra.clone()
        } else {
            Vec::new()
        },
        resolver: plan.resolver.filter(|_| gate.open(Tier::Resolver)),
        policies: plan.policies && gate.open(Tier::Policies),
    }
}

/// The `--plan-b64` value: base64url without padding of the plan's compact JSON.
pub fn encode_plan(plan: &SystemPlan) -> String {
    // A struct of flags, an enum and strings always serialises.
    let json = serde_json::to_vec(plan).unwrap_or_default();
    BASE64URL_NOPAD.encode(&json)
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
        match self {
            PlanError::Gated { .. } => EXIT_GATED,
            PlanError::TooLong { .. }
            | PlanError::Malformed
            | PlanError::Version { .. }
            | PlanError::TooManyNames { .. }
            | PlanError::InvalidName { .. }
            | PlanError::NeverBlock { .. } => EXIT_INVALID_PLAN,
        }
    }
}

/// A refused name as it may be printed: escaped, so that it cannot move a terminal's
/// cursor or start a new line, and cut short.
fn printable(name: &str) -> String {
    const SHOWN: usize = 80;
    let mut shown: String = name
        .chars()
        .take(SHOWN)
        .flat_map(char::escape_default)
        .collect();
    if name.chars().count() > SHOWN {
        shown.push('…');
    }
    shown
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::TooLong { len } => {
                write!(
                    f,
                    "the plan is {len} characters long (at most {HELPER_ARG_MAX})"
                )
            }
            PlanError::Malformed => f.write_str("the plan cannot be read"),
            PlanError::Version { found } => {
                write!(
                    f,
                    "plan version {found} is not supported (expected {CONFIG_VERSION})"
                )
            }
            PlanError::TooManyNames { count } => {
                write!(
                    f,
                    "the plan carries {count} names (at most {MAX_PLAN_EXTRA})"
                )
            }
            PlanError::InvalidName { name } => {
                write!(
                    f,
                    "\"{}\" is not a hostname that can be blocked",
                    printable(name)
                )
            }
            PlanError::NeverBlock { name } => {
                write!(f, "\"{}\" is on the never-block list", printable(name))
            }
            PlanError::Gated { tier } => {
                write!(f, "the {} tier is closed on this system", tier.as_str())
            }
        }
    }
}

impl std::error::Error for PlanError {}

/// Decodes a `--plan-b64` value. Checks the length and the encoding only; [`validate`]
/// does the rest.
pub fn decode_plan(arg: &str) -> Result<SystemPlan, PlanError> {
    if arg.len() > HELPER_ARG_MAX {
        return Err(PlanError::TooLong { len: arg.len() });
    }
    let json = BASE64URL_NOPAD
        .decode(arg.as_bytes())
        .map_err(|_| PlanError::Malformed)?;
    serde_json::from_slice(&json).map_err(|_| PlanError::Malformed)
}

/// Whether one user-typed name may be written to the hosts file: it passes
/// `hosts::is_valid_hostname` and is not under `never_block`. The helper applies this to
/// the names of a plan and again to the names it reads back from its state file.
pub fn admissible(name: &str, never_block: &RuleSet) -> Result<(), PlanError> {
    if !hosts::is_valid_hostname(name) {
        return Err(PlanError::InvalidName {
            name: name.to_owned(),
        });
    }
    if never_block.covers(name) {
        return Err(PlanError::NeverBlock {
            name: name.to_owned(),
        });
    }
    Ok(())
}

/// Validation as root: version 1; at most `MAX_PLAN_EXTRA` names, each passing
/// `hosts::is_valid_hostname` and not under `never_block`; every requested tier open in
/// `gate`.
pub fn validate(plan: &SystemPlan, never_block: &RuleSet, gate: &Gate) -> Result<(), PlanError> {
    if plan.v != CONFIG_VERSION {
        return Err(PlanError::Version { found: plan.v });
    }
    if plan.extra.len() > MAX_PLAN_EXTRA {
        return Err(PlanError::TooManyNames {
            count: plan.extra.len(),
        });
    }
    for name in &plan.extra {
        admissible(name, never_block)?;
    }
    let requested = [
        (Tier::Hosts, plan.hosts),
        (Tier::Resolver, plan.resolver.is_some()),
        (Tier::Policies, plan.policies),
    ];
    for (tier, wanted) in requested {
        if wanted && !gate.open(tier) {
            return Err(PlanError::Gated { tier });
        }
    }
    Ok(())
}

/// What the plan asks for, as the state file records it.
pub fn wants_of(plan: &SystemPlan) -> OwnerWants {
    OwnerWants {
        hosts: plan.hosts,
        resolver: plan.resolver,
        policies: plan.policies,
        extra: plan.extra.clone(),
    }
}

/// The union of every owner's wants: a tier is applied while anybody wants it; `extra` is
/// every owner's names without duplicates, in owner order. The names of an owner that does
/// not want the hosts tier are not in it. Should two owners name different resolvers, the
/// first owner's is used.
pub fn union(owners: &BTreeMap<ProductId, OwnerWants>) -> OwnerWants {
    let mut all = OwnerWants::default();
    for wants in owners.values() {
        all.hosts |= wants.hosts;
        all.policies |= wants.policies;
        all.resolver = all.resolver.or(wants.resolver);
        if wants.hosts {
            for name in &wants.extra {
                if !all.extra.contains(name) {
                    all.extra.push(name.clone());
                }
            }
        }
    }
    all
}

/// The owners of the state file once `plan` is applied: its owner's wants replaced, the
/// others kept.
pub fn owners_after(
    state: Option<&SystemState>,
    plan: &SystemPlan,
) -> BTreeMap<ProductId, OwnerWants> {
    let mut owners = state.map(|state| state.owners.clone()).unwrap_or_default();
    owners.insert(plan.owner, wants_of(plan));
    owners
}

/// The names of the hosts block for `owners`, in the order they are written: empty when
/// nobody wants the tier; otherwise `hosts::names_for` over `tier` and every owner's
/// admissible names. The helper writes exactly these names and the app expects exactly
/// these, so a name that stopped being admissible (the never-block rules grew with an
/// update) drops out on both sides.
pub fn hosts_names(
    owners: &BTreeMap<ProductId, OwnerWants>,
    tier: &[&str],
    never_block: &RuleSet,
    os: Os,
) -> Vec<String> {
    let wanted = union(owners);
    if !wanted.hosts {
        return Vec::new();
    }
    let extras: Vec<String> = wanted
        .extra
        .into_iter()
        .filter(|name| admissible(name, never_block).is_ok())
        .collect();
    hosts::names_for(tier, &extras, os)
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
///
/// A tier the user did not choose is `Off`, unless it is still on the machine: `Applied`
/// when the state file lists it for this product (switched off in the app, not yet removed
/// from the system), `InUseByOther` when it is there for the other product. A tier the
/// user chose is judged in this order: the gate, what cannot work here, then what is
/// found.
///
/// A DNS read that failed (`observation.resolver.error`) is reported as `Unsupported`
/// like any other, so a caller whose quick Windows read found nothing usable keeps the
/// resolver part of its last full read instead of having the failed one assessed.
pub fn assess(input: &AssessInput<'_>, observation: &SystemObservation) -> SystemAssessment {
    assess_with(input, observation, matcher::never_block())
}

/// Who the state file says wants a tier.
#[derive(Debug, Clone, Copy)]
struct Wanted {
    /// The state file could be read.
    known: bool,
    /// It lists the tier for this product.
    mine: bool,
    /// It lists the tier for another product.
    other: bool,
}

impl Wanted {
    fn of(state: Option<&SystemState>, me: ProductId, wants: impl Fn(&OwnerWants) -> bool) -> Self {
        let listed = |own: bool| {
            state.is_some_and(|state| {
                state
                    .owners
                    .iter()
                    .any(|(owner, owned)| (*owner == me) == own && wants(owned))
            })
        };
        Self {
            known: state.is_some(),
            mine: listed(true),
            other: listed(false),
        }
    }

    /// The tier is in place, intact and as wanted, as far as the machine shows. Whose it
    /// is decides how it is called.
    fn in_place(self) -> TierState {
        if self.known && !self.mine {
            if self.other {
                TierState::InUseByOther
            } else {
                TierState::NotApplied
            }
        } else {
            TierState::Applied
        }
    }
}

fn unsupported(code: &str) -> TierState {
    TierState::Unsupported {
        reason: code.to_owned(),
    }
}

/// [`assess`] with the never-block rules as an argument.
pub fn assess_with(
    input: &AssessInput<'_>,
    observation: &SystemObservation,
    never_block: &RuleSet,
) -> SystemAssessment {
    let me = input.plan.owner;
    let state = observation.state.as_ref();
    let mut issues = Vec::new();

    let (hosts, hosts_waiting) = assess_hosts(input, observation, never_block);
    let block_in_place = matches!(hosts, TierState::Applied | TierState::UpdatePending);
    if block_in_place && input.config.system.hosts {
        if let Some(answers) = &observation.canary {
            if !canary_honoured(answers) {
                issues.push(Issue::HostsNotHonoured);
            }
        }
    }

    let (resolver, new_connection) = assess_resolver(input, observation);
    if resolver == TierState::Applied
        && input.config.system.resolver.is_some()
        && input.probe_fail_streak >= UNREACHABLE_AFTER_FAILS
    {
        issues.push(Issue::ResolverUnreachable);
    }
    if new_connection {
        issues.push(Issue::NewConnectionUnfiltered);
    }

    let wanted = Wanted::of(state, me, |wants| wants.policies);
    let created = state.map_or(&[][..], |state| state.policies_created.as_slice());
    let policies = assess_policies(input, &observation.policies, wanted, created, &mut issues);

    SystemAssessment {
        hosts,
        resolver,
        policies,
        issues,
        hosts_waiting,
    }
}

/// Whether the canary resolved to the null address and to nothing else.
fn canary_honoured(answers: &[String]) -> bool {
    !answers.is_empty()
        && answers.iter().all(|answer| {
            answer
                .trim()
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_unspecified())
        })
}

fn assess_hosts(
    input: &AssessInput<'_>,
    observation: &SystemObservation,
    never_block: &RuleSet,
) -> (TierState, u32) {
    let state = observation.state.as_ref();
    let wanted = Wanted::of(state, input.plan.owner, |wants| wants.hosts);
    let text = observation.hosts.text.as_deref();
    let inspection = text.map(|text| hosts::inspect(text.as_bytes()));
    let present = matches!(
        inspection,
        Some(BlockInspection::Intact { .. } | BlockInspection::Changed)
    );

    if !input.config.system.hosts {
        let left = if present && wanted.mine {
            TierState::Applied
        } else if present && wanted.other {
            TierState::InUseByOther
        } else {
            TierState::Off
        };
        return (left, 0);
    }
    if !input.gate.open(Tier::Hosts) {
        return (TierState::Gated, 0);
    }
    if let Some(code) = &observation.hosts.error {
        return (unsupported(code), 0);
    }
    let (Some(text), Some(inspection)) = (text, inspection) else {
        return (unsupported(reason::UNREADABLE), 0);
    };
    match inspection {
        BlockInspection::Absent => {
            let gone = if wanted.mine {
                TierState::Missing
            } else {
                TierState::NotApplied
            };
            (gone, 0)
        }
        // A block that is there for somebody else, or for nobody the state file knows, is
        // not this product's to call changed.
        _ if wanted.known && !wanted.mine => (wanted.in_place(), 0),
        BlockInspection::Changed => (TierState::Changed, 0),
        BlockInspection::Intact { sha256, .. } => {
            let owners = owners_after(state, input.plan);
            let names = hosts_names(&owners, input.hosts_tier, never_block, input.os);
            if hosts::stamp_of(&names).sha256 == sha256 {
                (TierState::Applied, 0)
            } else {
                (TierState::UpdatePending, names_waiting(&names, text))
            }
        }
    }
}

/// How many of `names` the block in `text` does not hold yet. A site and its `www.` twin
/// count as one.
fn names_waiting(names: &[String], text: &str) -> u32 {
    let written = hosts::block_names(text.as_bytes());
    let written: HashSet<&str> = written.iter().map(String::as_str).collect();
    let missing: HashSet<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| !written.contains(name))
        .collect();
    let sites = missing
        .iter()
        .filter(|name| {
            name.strip_prefix("www.")
                .is_none_or(|site| !missing.contains(site))
        })
        .count();
    u32::try_from(sites).unwrap_or(u32::MAX)
}

/// How much of the machine's DNS is the resolver's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InPlace {
    /// Every active interface (or the drop-in, byte for byte).
    All,
    /// Some interfaces, or a drop-in that is not what was written.
    Partly,
    Nothing,
    /// No interface is active: nothing to judge.
    NoInterface,
}

fn active(interfaces: &[InterfaceDns]) -> impl Iterator<Item = &InterfaceDns> {
    interfaces.iter().filter(|interface| interface.active)
}

fn resolver_in_place(observed: &ResolverObservation, id: ResolverId, os: Os) -> InPlace {
    if os == Os::Linux {
        return match &observed.dropin {
            None => InPlace::Nothing,
            Some(text) if *text == resolvers::resolved_dropin(id) => InPlace::All,
            Some(_) => InPlace::Partly,
        };
    }
    let total = active(&observed.interfaces).count();
    let ours = active(&observed.interfaces)
        .filter(|interface| resolvers::is_exactly(id, &interface.servers))
        .count();
    match (total, ours) {
        (0, _) => InPlace::NoInterface,
        (_, 0) => InPlace::Nothing,
        (total, ours) if total == ours => InPlace::All,
        _ => InPlace::Partly,
    }
}

/// Why the resolver tier cannot work on a machine that looks like `observed`, if it
/// cannot.
fn resolver_unsupported(observed: &ResolverObservation, os: Os) -> Option<&str> {
    if let Some(code) = &observed.error {
        return Some(code);
    }
    if os != Os::Linux {
        return None;
    }
    match (observed.resolved_active, observed.resolv_conf_uses_resolved) {
        (Some(true), Some(true)) => None,
        (Some(false), _) | (_, Some(false)) => Some(reason::NO_RESOLVED),
        (None, _) | (_, None) => Some(reason::UNREADABLE),
    }
}

/// The state of the resolver tier, and whether an interface that appeared after the apply
/// is unfiltered.
fn assess_resolver(input: &AssessInput<'_>, observation: &SystemObservation) -> (TierState, bool) {
    let state = observation.state.as_ref();
    let me = input.plan.owner;
    let observed = &observation.resolver;
    let system = &input.config.system;

    let Some(id) = system.resolver else {
        let listed = |own: bool| {
            state.and_then(|state| {
                state
                    .owners
                    .iter()
                    .filter(|(owner, _)| (**owner == me) == own)
                    .find_map(|(_, wants)| wants.resolver)
            })
        };
        let there = |id: ResolverId| {
            matches!(
                resolver_in_place(observed, id, input.os),
                InPlace::All | InPlace::Partly
            )
        };
        let left = if listed(true).is_some_and(there) {
            TierState::Applied
        } else if listed(false).is_some_and(there) {
            TierState::InUseByOther
        } else {
            TierState::Off
        };
        return (left, false);
    };

    let wanted = Wanted::of(state, me, |wants| wants.resolver == Some(id));
    if !input.gate.open(Tier::Resolver) {
        return (TierState::Gated, false);
    }
    if system.network_restored {
        return (TierState::NotApplied, false);
    }
    if let Some(code) = resolver_unsupported(observed, input.os) {
        return (unsupported(code), false);
    }
    match resolver_in_place(observed, id, input.os) {
        InPlace::All => (wanted.in_place(), false),
        InPlace::NoInterface => {
            let judged = if wanted.mine {
                TierState::Applied
            } else {
                TierState::NotApplied
            };
            (judged, false)
        }
        InPlace::Partly | InPlace::Nothing if !wanted.mine => (TierState::NotApplied, false),
        InPlace::Nothing if input.os == Os::Linux => (TierState::Missing, false),
        InPlace::Partly | InPlace::Nothing => {
            let known = &input.config.internal.previous_dns;
            let new_connection = active(&observed.interfaces)
                .filter(|interface| !resolvers::is_exactly(id, &interface.servers))
                .any(|interface| {
                    !known.iter().any(|seen| {
                        seen.interface == interface.id || seen.interface == interface.name
                    })
                });
            (TierState::Changed, new_connection)
        }
    }
}

fn assess_policies(
    input: &AssessInput<'_>,
    observed: &[PolicyObservation],
    wanted: Wanted,
    created: &[PolicyCreated],
    issues: &mut Vec<Issue>,
) -> TierState {
    let os = input.os;
    let relevant: Vec<&PolicyObservation> = observed
        .iter()
        .filter(|browser| browser.present && policies::applies(browser.browser, os))
        .collect();
    let any_applied = relevant.iter().any(|browser| policies::is_applied(browser));

    if !input.config.system.policies {
        return if wanted.mine && (any_applied || !created.is_empty()) {
            TierState::Applied
        } else if wanted.other && any_applied {
            TierState::InUseByOther
        } else {
            TierState::Off
        };
    }
    if !input.gate.open(Tier::Policies) {
        return TierState::Gated;
    }

    let conflicts: Vec<BrowserFamily> = policies::conflicts(observed, created)
        .into_iter()
        .filter(|browser| relevant.iter().any(|seen| seen.browser == *browser))
        .collect();
    if !conflicts.is_empty() {
        issues.push(Issue::PolicyConflict);
    }
    let advisory = observed
        .iter()
        .any(|browser| browser.present && !policies::applies(browser.browser, os));
    if advisory {
        issues.push(Issue::PolicyAdvisoryOnly);
    }

    let manageable: Vec<&PolicyObservation> = relevant
        .iter()
        .copied()
        .filter(|browser| browser.error.is_none() && !conflicts.contains(&browser.browser))
        .collect();
    if manageable.is_empty() {
        let out_of_reach = relevant.iter().find_map(|browser| {
            browser
                .error
                .as_deref()
                .filter(|code| *code != reason::CONFLICT)
        });
        return match out_of_reach {
            Some(code) => unsupported(code),
            None if !conflicts.is_empty() => unsupported(reason::CONFLICT),
            // No browser this tier can do anything for: nothing to apply, nothing missing.
            None if wanted.mine => TierState::Applied,
            None => TierState::NotApplied,
        };
    }

    let applied = |browser: &PolicyObservation| policies::is_applied(browser);
    if manageable.iter().all(|browser| applied(browser)) {
        return wanted.in_place();
    }
    if !wanted.mine {
        return TierState::NotApplied;
    }
    let ours =
        |browser: &PolicyObservation| created.iter().any(|made| made.browser == browser.browser);
    let drifted = manageable
        .iter()
        .any(|browser| ours(browser) && !applied(browser));
    if !drifted {
        // Browsers the helper has not written for yet (installed since the last apply).
        return TierState::UpdatePending;
    }
    let nothing_left = manageable.iter().all(|browser| {
        policies::desired(browser.browser)
            .iter()
            .all(|want| !browser.values.iter().any(|found| found.key == want.key))
    });
    if nothing_left {
        TierState::Missing
    } else {
        TierState::Changed
    }
}

/// The exact list of changes applying `plan` would make on a machine that looks like
/// `observation`, for the user to read before the OS prompt. Every line is one of
/// `ubiqx_core::nofap::preview::LINES`, with a `detail` exactly when the line has a
/// placeholder; a tier that is gated or unsupported adds no line.
///
/// The gate is not an argument: hand in the plan as it is sent ([`without_gated`]).
pub fn preview(plan: &SystemPlan, observation: &SystemObservation, os: Os) -> Vec<PlannedChange> {
    preview_with(
        plan,
        observation,
        os,
        crate::list::hosts_tier(),
        matcher::never_block(),
    )
}

fn planned(tier: Tier, text_key: &str, detail: Option<String>) -> PlannedChange {
    PlannedChange {
        tier,
        text_key: text_key.to_owned(),
        detail,
    }
}

/// [`preview`] with the hosts tier and the never-block rules as arguments.
pub fn preview_with(
    plan: &SystemPlan,
    observation: &SystemObservation,
    os: Os,
    hosts_tier: &[&str],
    never_block: &RuleSet,
) -> Vec<PlannedChange> {
    let state = observation.state.as_ref();
    let before = state.map(|state| union(&state.owners)).unwrap_or_default();
    let owners = owners_after(state, plan);
    let after = union(&owners);
    let mut lines = Vec::new();

    // L2: the hosts block.
    if let (None, Some(text)) = (&observation.hosts.error, &observation.hosts.text) {
        let inspection = hosts::inspect(text.as_bytes());
        if after.hosts {
            let names = hosts_names(&owners, hosts_tier, never_block, os);
            // The canary is not a site.
            let sites = names.len().saturating_sub(1).to_string();
            let key = match inspection {
                BlockInspection::Absent => Some(line::HOSTS_ADD),
                BlockInspection::Intact { sha256, .. }
                    if sha256 == hosts::stamp_of(&names).sha256 =>
                {
                    None
                }
                BlockInspection::Intact { .. } | BlockInspection::Changed => {
                    Some(line::HOSTS_UPDATE)
                }
            };
            if let Some(key) = key {
                lines.push(planned(Tier::Hosts, key, Some(sites)));
                if plan.hosts && !plan.extra.is_empty() {
                    lines.push(planned(Tier::Hosts, line::HOSTS_READABLE, None));
                }
            }
        } else if inspection != BlockInspection::Absent {
            lines.push(planned(Tier::Hosts, line::HOSTS_REMOVE, None));
        }
    }

    // L3: the resolver.
    let observed = &observation.resolver;
    if resolver_unsupported(observed, os).is_none() {
        match after.resolver {
            Some(id) => {
                if !matches!(
                    resolver_in_place(observed, id, os),
                    InPlace::All | InPlace::NoInterface
                ) {
                    let operator = resolvers::lookup(id).operator.to_owned();
                    lines.push(planned(Tier::Resolver, line::RESOLVER_SET, Some(operator)));
                    let mut replaced: Vec<&str> = Vec::new();
                    for interface in active(&observed.interfaces) {
                        if resolvers::is_exactly(id, &interface.servers) {
                            continue;
                        }
                        for server in &interface.servers {
                            if !replaced.contains(&server.as_str()) {
                                replaced.push(server);
                            }
                        }
                    }
                    if !replaced.is_empty() {
                        let servers = replaced.join(", ");
                        lines.push(planned(Tier::Resolver, line::DNS_PREVIOUS, Some(servers)));
                    }
                }
            }
            None => {
                let ours_somewhere = RESOLVERS
                    .iter()
                    .any(|row| !reset_targets(observation, row.id).is_empty());
                let to_undo = match os {
                    Os::Linux => observed.dropin.is_some(),
                    Os::Macos | Os::Windows => ours_somewhere,
                };
                if before.resolver.is_some() && to_undo {
                    lines.push(planned(Tier::Resolver, line::RESOLVER_RESET, None));
                }
            }
        }
    }

    // L4: browser policies.
    let created = state.map_or(&[][..], |state| state.policies_created.as_slice());
    if after.policies {
        let relevant = |browser: &&PolicyObservation| {
            browser.present && policies::applies(browser.browser, os)
        };
        let conflicts: Vec<BrowserFamily> = policies::conflicts(&observation.policies, created)
            .into_iter()
            .filter(|browser| {
                observation
                    .policies
                    .iter()
                    .filter(relevant)
                    .any(|seen| seen.browser == *browser)
            })
            .collect();
        let mut to_write: Vec<BrowserFamily> = Vec::new();
        for browser in observation.policies.iter().filter(relevant) {
            let writable = browser.error.is_none() && !conflicts.contains(&browser.browser);
            if writable && !policies::is_applied(browser) && !to_write.contains(&browser.browser) {
                to_write.push(browser.browser);
            }
        }
        let names = |browsers: &[BrowserFamily]| {
            browsers
                .iter()
                .map(|browser| policies::display_name(*browser))
                .collect::<Vec<_>>()
                .join(", ")
        };
        if !to_write.is_empty() {
            lines.push(planned(
                Tier::Policies,
                line::POLICIES_SET,
                Some(names(&to_write)),
            ));
            lines.push(planned(Tier::Policies, line::POLICIES_DISCLOSURE, None));
        }
        if !conflicts.is_empty() {
            lines.push(planned(
                Tier::Policies,
                line::POLICIES_CONFLICT,
                Some(names(&conflicts)),
            ));
        }
    } else if !created.is_empty() {
        lines.push(planned(Tier::Policies, line::POLICIES_REMOVE, None));
    }

    lines
}

/// Interfaces (by id) the helper may reset to automatic when it undoes the resolver tier:
/// only those whose servers are still exactly the provider's set. An interface the user
/// changed since is left alone.
pub fn reset_targets(observation: &SystemObservation, resolver: ResolverId) -> Vec<String> {
    observation
        .resolver
        .interfaces
        .iter()
        .filter(|interface| resolvers::is_exactly(resolver, &interface.servers))
        .map(|interface| interface.id.clone())
        .collect()
}

/// Whether seeing a host in front means DNS is being bypassed: the host is one of the
/// names of the hosts tier and the hosts block is applied.
pub fn bypass_suspected(in_hosts_tier: bool, hosts: &TierState) -> bool {
    in_hosts_tier && matches!(hosts, TierState::Applied | TierState::UpdatePending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, TimeZone, Utc};
    use serde_json::json;
    use ubiqx_core::nofap::{HostsObservation, HostsStamp, PolicyValue, PreviousDns, TierReport};

    const TIER: &[&str] = &["tier-one.example", "tier-two.example", "tier-three.example"];
    const CF: ResolverId = ResolverId::CloudflareFamily;
    const CF_SERVERS: [&str; 4] = [
        "1.1.1.3",
        "1.0.0.3",
        "2606:4700:4700::1113",
        "2606:4700:4700::1003",
    ];

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap() + Duration::seconds(secs)
    }

    fn rules() -> RuleSet {
        RuleSet::parse(["gov", "gov.*", "bank.example", "accounts.vendor.example"])
    }

    /// Every tier open (through the tester's override: the table opens none).
    fn open(os: Os) -> Gate {
        Gate::new(os, Tier::ALL.to_vec())
    }

    fn closed(os: Os) -> Gate {
        Gate::new(os, Vec::new())
    }

    fn site(host: &str, list: SiteList, origin: SiteOrigin, created: i64) -> NofapSite {
        NofapSite {
            id: format!("site-{host}-{created}"),
            list,
            origin,
            host: (origin == SiteOrigin::User).then(|| host.to_owned()),
            host_key: [0; 16],
            created_at: at(created),
            expires_at: None,
        }
    }

    fn blocked(host: &str, created: i64) -> NofapSite {
        site(host, SiteList::Block, SiteOrigin::User, created)
    }

    /// A configuration with all three tiers chosen.
    fn chosen() -> NofapConfig {
        let mut config = NofapConfig {
            enabled: true,
            ..NofapConfig::default()
        };
        config.system.hosts = true;
        config.system.resolver = Some(CF);
        config.system.policies = true;
        config
    }

    fn plan(extra: &[&str]) -> SystemPlan {
        SystemPlan {
            v: 1,
            owner: ProductId::Ubiqx,
            hosts: true,
            extra: extra.iter().map(|name| (*name).to_owned()).collect(),
            resolver: Some(CF),
            policies: true,
        }
    }

    fn wants(
        hosts: bool,
        resolver: Option<ResolverId>,
        policies: bool,
        extra: &[&str],
    ) -> OwnerWants {
        OwnerWants {
            hosts,
            resolver,
            policies,
            extra: extra.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    fn state(owners: &[(ProductId, OwnerWants)]) -> SystemState {
        SystemState {
            v: 1,
            applied_at: 1_795_100_000_000,
            owners: owners.iter().cloned().collect(),
            hosts: None,
            policies_created: Vec::new(),
            report: Vec::new(),
        }
    }

    /// The block the helper writes for `owners` on `os`.
    fn block_for(owners: &[(ProductId, OwnerWants)], os: Os) -> String {
        let owners: BTreeMap<ProductId, OwnerWants> = owners.iter().cloned().collect();
        hosts::render_block(&hosts_names(&owners, TIER, &rules(), os))
    }

    fn hosts_file(block: &str) -> HostsObservation {
        HostsObservation {
            text: Some(format!("127.0.0.1 localhost\n{block}")),
            error: None,
        }
    }

    fn interface(id: &str, active: bool, servers: &[&str]) -> InterfaceDns {
        InterfaceDns {
            id: id.to_owned(),
            name: format!("Interface {id}"),
            active,
            servers: servers.iter().map(|server| (*server).to_owned()).collect(),
        }
    }

    fn applied_policy(browser: BrowserFamily) -> PolicyObservation {
        PolicyObservation {
            browser,
            present: true,
            values: policies::desired(browser),
            foreign: Vec::new(),
            error: None,
        }
    }

    fn empty_policy(browser: BrowserFamily) -> PolicyObservation {
        PolicyObservation {
            values: Vec::new(),
            ..applied_policy(browser)
        }
    }

    fn created_for(browsers: &[BrowserFamily]) -> Vec<PolicyCreated> {
        browsers
            .iter()
            .flat_map(|browser| {
                policies::desired(*browser)
                    .into_iter()
                    .map(|value| PolicyCreated {
                        browser: *browser,
                        key: value.key,
                    })
            })
            .collect()
    }

    /// Runs `assess_with` for ubiqX on `os` with every tier open.
    fn assessed(config: &NofapConfig, observation: &SystemObservation, os: Os) -> SystemAssessment {
        assessed_with(config, observation, os, &open(os), 0)
    }

    fn assessed_with(
        config: &NofapConfig,
        observation: &SystemObservation,
        os: Os,
        gate: &Gate,
        probe_fail_streak: u32,
    ) -> SystemAssessment {
        let built = build_plan_with(
            config,
            &[blocked("mine.example", 0)],
            ProductId::Ubiqx,
            os,
            &rules(),
        );
        let input = AssessInput {
            config,
            plan: &built.plan,
            os,
            gate,
            hosts_tier: TIER,
            probe_fail_streak,
        };
        assess_with(&input, observation, &rules())
    }

    /// What the helper records for ubiqX after applying the plan of [`assessed`].
    fn my_wants() -> OwnerWants {
        wants(true, Some(CF), true, &["mine.example"])
    }

    // -- the plan --------------------------------------------------------------------------

    #[test]
    fn argument_budget_per_os() {
        assert_eq!(arg_budget(Os::Windows), 1_800);
        assert_eq!(arg_budget(Os::Macos), 24_000);
        assert_eq!(arg_budget(Os::Linux), 24_000);
        assert!(arg_budget(Os::Linux) <= HELPER_ARG_MAX);
    }

    #[test]
    fn a_plan_carries_what_the_user_chose() {
        let config = chosen();
        let built = build_plan_with(&config, &[], ProductId::Shield, Os::Linux, &rules());
        assert_eq!(
            built.plan,
            SystemPlan {
                v: 1,
                owner: ProductId::Shield,
                hosts: true,
                extra: Vec::new(),
                resolver: Some(CF),
                policies: true,
            }
        );
        assert_eq!(built.extras_left_out, 0);
        let nothing = build_plan_with(
            &NofapConfig::default(),
            &[],
            ProductId::Ubiqx,
            Os::Linux,
            &rules(),
        );
        assert_eq!(wants_of(&nothing.plan), OwnerWants::default());
    }

    #[test]
    fn the_resolver_is_left_out_while_the_network_is_restored() {
        let mut config = chosen();
        config.system.network_restored = true;
        let built = build_plan_with(&config, &[], ProductId::Ubiqx, Os::Macos, &rules());
        assert_eq!(built.plan.resolver, None);
        assert!(built.plan.hosts && built.plan.policies);
    }

    #[test]
    fn extra_holds_the_users_blocked_sites_most_recent_first() {
        let sites = vec![
            blocked("old.example", 10),
            blocked("WWW.Newest.Example", 30),
            blocked("middle.example", 20),
            // Never in a plan: learned sites, snoozes and the allow list.
            site("learned.example", SiteList::Block, SiteOrigin::Learned, 40),
            site("snoozed.example", SiteList::Allow, SiteOrigin::Snooze, 41),
            site("allowed.example", SiteList::Allow, SiteOrigin::User, 42),
            // The same site twice is one name.
            blocked("https://old.example/page", 5),
        ];
        let built = build_plan_with(&chosen(), &sites, ProductId::Ubiqx, Os::Linux, &rules());
        assert_eq!(
            built.plan.extra,
            vec!["newest.example", "middle.example", "old.example"]
        );
        assert_eq!(built.extras_left_out, 0);
    }

    #[test]
    fn names_the_helper_would_refuse_stay_out_of_the_plan_and_are_counted() {
        let sites = vec![
            blocked("fine.example", 1),
            blocked("planalto.gov.br", 2),
            blocked("login.bank.example", 3),
            blocked("printer.local", 4),
            blocked("192.168.0.1", 5),
            blocked("not a host", 6),
            blocked("localhost", 7),
        ];
        let built = build_plan_with(&chosen(), &sites, ProductId::Ubiqx, Os::Linux, &rules());
        assert_eq!(built.plan.extra, vec!["fine.example"]);
        assert_eq!(built.extras_left_out, 6);
        // What is built always passes the helper's validation.
        assert_eq!(validate(&built.plan, &rules(), &open(Os::Linux)), Ok(()));
    }

    #[test]
    fn user_sites_stay_in_the_app_when_the_user_says_so_or_the_tier_is_off() {
        let sites = vec![blocked("one.example", 1), blocked("two.example", 2)];
        let mut app_only = chosen();
        app_only.system.hosts_user_sites = false;
        let built = build_plan_with(&app_only, &sites, ProductId::Ubiqx, Os::Linux, &rules());
        assert!(built.plan.hosts);
        assert!(built.plan.extra.is_empty());
        assert_eq!(built.extras_left_out, 0);

        let mut no_hosts = chosen();
        no_hosts.system.hosts = false;
        let built = build_plan_with(&no_hosts, &sites, ProductId::Ubiqx, Os::Linux, &rules());
        assert!(!built.plan.hosts && built.plan.extra.is_empty());
        assert_eq!(built.extras_left_out, 0);
    }

    #[test]
    fn the_windows_budget_keeps_the_most_recent_names_that_fit() {
        let sites: Vec<NofapSite> = (0..300)
            .map(|n| blocked(&format!("user-site-{n:03}.example"), n))
            .collect();
        let built = build_plan_with(&chosen(), &sites, ProductId::Ubiqx, Os::Windows, &rules());
        let kept = built.plan.extra.len();
        assert!((40..120).contains(&kept), "{kept} names fit");
        assert_eq!(built.extras_left_out as usize, 300 - kept);
        // The most recent ones, in order.
        assert_eq!(built.plan.extra[0], "user-site-299.example");
        assert_eq!(
            built.plan.extra[kept - 1],
            format!("user-site-{:03}.example", 300 - kept)
        );
        // Inside the budget, and one more name would not be.
        assert!(encode_plan(&built.plan).len() <= ARG_BUDGET_WINDOWS);
        let mut one_more = built.plan.clone();
        one_more
            .extra
            .push(format!("user-site-{:03}.example", 299 - kept));
        assert!(encode_plan(&one_more).len() > ARG_BUDGET_WINDOWS);
        // The same sites all fit on the other systems.
        let unix = build_plan_with(&chosen(), &sites, ProductId::Ubiqx, Os::Linux, &rules());
        assert_eq!(unix.plan.extra.len(), 300);
        assert_eq!(unix.extras_left_out, 0);
    }

    #[test]
    fn at_most_four_hundred_names_and_never_past_the_unix_budget() {
        let many: Vec<NofapSite> = (0..450)
            .map(|n| blocked(&format!("s{n}.example"), n))
            .collect();
        let built = build_plan_with(&chosen(), &many, ProductId::Ubiqx, Os::Macos, &rules());
        assert_eq!(built.plan.extra.len(), MAX_PLAN_EXTRA);
        assert_eq!(built.extras_left_out, 50);
        assert_eq!(validate(&built.plan, &rules(), &open(Os::Macos)), Ok(()));

        // Very long names run into the budget before the count.
        let label = "a".repeat(60);
        let long: Vec<NofapSite> = (0..400)
            .map(|n| blocked(&format!("{label}.{label}.{label}.n{n}.example"), n))
            .collect();
        let built = build_plan_with(&chosen(), &long, ProductId::Ubiqx, Os::Linux, &rules());
        assert!(built.plan.extra.len() < 400);
        assert_eq!(built.plan.extra.len() + built.extras_left_out as usize, 400);
        let encoded = encode_plan(&built.plan);
        assert!(encoded.len() <= ARG_BUDGET_UNIX);
        assert!(decode_plan(&encoded).is_ok());
    }

    #[test]
    fn the_estimate_of_the_argument_length_is_exact() {
        for count in [0usize, 1, 2, 7, 50] {
            let sites: Vec<NofapSite> = (0..count)
                .map(|n| blocked(&format!("site-number-{n}.example"), n as i64))
                .collect();
            let built = build_plan_with(&chosen(), &sites, ProductId::Shield, Os::Linux, &rules());
            let json = serde_json::to_vec(&built.plan).unwrap();
            assert_eq!(
                encode_plan(&built.plan).len(),
                base64_len(json.len()),
                "{count} names"
            );
        }
    }

    #[test]
    fn a_plan_round_trips_through_the_argument() {
        let original = plan(&["example.com", "other.example"]);
        let argument = encode_plan(&original);
        assert!(argument
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        assert_eq!(decode_plan(&argument), Ok(original));
    }

    #[test]
    fn an_argument_that_is_not_a_plan_is_refused() {
        let encode = |json: serde_json::Value| BASE64URL_NOPAD.encode(json.to_string().as_bytes());
        let good = json!({ "v": 1, "owner": "ubiqx", "hosts": true, "extra": [], "resolver": null, "policies": false });
        assert!(decode_plan(&encode(good.clone())).is_ok());
        let with = |key: &str, value: serde_json::Value| {
            let mut changed = good.clone();
            changed[key] = value;
            encode(changed)
        };
        for argument in [
            String::new(),
            "not base64!".to_owned(),
            "--plan".to_owned(),
            "../../etc/passwd".to_owned(),
            BASE64URL_NOPAD.encode(b"not json"),
            BASE64URL_NOPAD.encode(b"[]"),
            BASE64URL_NOPAD.encode(b"{}"),
            // Standard base64 with padding is not the encoding.
            data_encoding::BASE64.encode(good.to_string().as_bytes()),
            with("owner", json!("somebody")),
            with("owner", json!("all")),
            with("resolver", json!("1.2.3.4")),
            with("resolver", json!("quad9")),
            with("extra", json!("example.com")),
            with("extra", json!([1, 2])),
            with("hosts", json!("yes")),
            with("v", json!(-1)),
            with("v", json!(300)),
        ] {
            assert_eq!(
                decode_plan(&argument),
                Err(PlanError::Malformed),
                "{argument}"
            );
        }
        let long = "A".repeat(HELPER_ARG_MAX + 1);
        assert_eq!(
            decode_plan(&long),
            Err(PlanError::TooLong {
                len: HELPER_ARG_MAX + 1
            })
        );
    }

    // -- validation as root ----------------------------------------------------------------

    #[test]
    fn a_good_plan_is_valid() {
        assert_eq!(
            validate(
                &plan(&["example.com", "xn--bcher-kva.example"]),
                &rules(),
                &open(Os::Linux)
            ),
            Ok(())
        );
        let nothing = SystemPlan {
            hosts: false,
            resolver: None,
            policies: false,
            ..plan(&[])
        };
        assert_eq!(validate(&nothing, &rules(), &closed(Os::Linux)), Ok(()));
    }

    #[test]
    fn an_unknown_version_is_refused() {
        for found in [0u8, 2, 255] {
            let wrong = SystemPlan {
                v: found,
                ..plan(&[])
            };
            let error = validate(&wrong, &rules(), &open(Os::Linux)).unwrap_err();
            assert_eq!(error, PlanError::Version { found });
            assert_eq!(error.exit_code(), EXIT_INVALID_PLAN);
        }
    }

    #[test]
    fn too_many_names_are_refused() {
        let names: Vec<String> = (0..=MAX_PLAN_EXTRA)
            .map(|n| format!("s{n}.example"))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(
            validate(&plan(&refs[..MAX_PLAN_EXTRA]), &rules(), &open(Os::Linux)),
            Ok(())
        );
        assert_eq!(
            validate(&plan(&refs), &rules(), &open(Os::Linux)),
            Err(PlanError::TooManyNames {
                count: MAX_PLAN_EXTRA + 1
            })
        );
    }

    #[test]
    fn hostile_names_are_refused() {
        for name in [
            "",
            "localhost",
            "mypc.local",
            "127.0.0.1",
            "0x7f.1",
            "::1",
            "example.com\n127.0.0.1 accounts.vendor.example",
            "example.com 0.0.0.0",
            "example.com\t#",
            "# <<< ubiqx-shield END",
            "../../etc/shadow",
            "/etc/hosts",
            "C:\\Windows\\System32\\drivers\\etc\\hosts",
            "--owner=all",
            "-rf.example.com",
            "$(id).example.com",
            "a;reboot.example.com",
            "EXAMPLE.COM",
            "Planalto.Gov.Br",
            "exa_mple.com",
            "éxample.com",
            "example.com.",
            "example.com\0",
        ] {
            let error =
                validate(&plan(&["fine.example", name]), &rules(), &open(Os::Linux)).unwrap_err();
            assert_eq!(
                error,
                PlanError::InvalidName {
                    name: name.to_owned()
                },
                "{name:?}"
            );
            assert_eq!(error.exit_code(), EXIT_INVALID_PLAN);
        }
    }

    #[test]
    fn names_under_the_never_block_rules_are_refused() {
        for name in [
            "bank.example",
            "login.bank.example",
            "deep.er.login.bank.example",
            "www.bank.example",
            "accounts.vendor.example",
            "whitehouse.gov",
            "planalto.gov.br",
            "www.receita.fazenda.gov.br",
        ] {
            let error = validate(&plan(&[name]), &rules(), &open(Os::Linux)).unwrap_err();
            assert_eq!(
                error,
                PlanError::NeverBlock {
                    name: name.to_owned()
                },
                "{name}"
            );
            assert_eq!(error.exit_code(), EXIT_INVALID_PLAN);
        }
        // A sibling of a protected name is not protected.
        assert_eq!(
            validate(
                &plan(&["vendor.example", "notbank.example"]),
                &rules(),
                &open(Os::Linux)
            ),
            Ok(())
        );
    }

    #[test]
    fn a_gated_tier_is_refused_with_its_own_exit_code() {
        let all = plan(&[]);
        for os in Os::ALL {
            let error = validate(&all, &rules(), &closed(os)).unwrap_err();
            assert_eq!(error, PlanError::Gated { tier: Tier::Hosts });
            assert_eq!(error.exit_code(), EXIT_GATED);
        }
        let hosts_only = Gate::new(Os::Linux, vec![Tier::Hosts]);
        assert_eq!(
            validate(&all, &rules(), &hosts_only),
            Err(PlanError::Gated {
                tier: Tier::Resolver
            })
        );
        let no_policies = Gate::new(Os::Linux, vec![Tier::Hosts, Tier::Resolver]);
        assert_eq!(
            validate(&all, &rules(), &no_policies),
            Err(PlanError::Gated {
                tier: Tier::Policies
            })
        );
        // What the app sends has the closed tiers taken out.
        let sent = without_gated(&plan(&["example.com"]), &hosts_only);
        assert_eq!(
            sent,
            SystemPlan {
                resolver: None,
                policies: false,
                ..plan(&["example.com"])
            }
        );
        assert_eq!(validate(&sent, &rules(), &hosts_only), Ok(()));
        let nothing = without_gated(&plan(&["example.com"]), &closed(Os::Linux));
        assert_eq!(wants_of(&nothing), OwnerWants::default());
        assert_eq!(validate(&nothing, &rules(), &closed(Os::Linux)), Ok(()));
    }

    #[test]
    fn an_error_message_cannot_carry_control_characters() {
        let hostile = "evil\u{1b}[2J\nname".to_owned() + &"x".repeat(500);
        let message = PlanError::InvalidName { name: hostile }.to_string();
        assert!(!message.contains('\u{1b}') && !message.contains('\n'));
        assert!(message.chars().count() < 200, "{message}");
        assert!(PlanError::Gated {
            tier: Tier::Resolver
        }
        .to_string()
        .contains("resolver"));
        for error in [
            PlanError::TooLong { len: 30_000 },
            PlanError::Malformed,
            PlanError::Version { found: 9 },
            PlanError::TooManyNames { count: 401 },
            PlanError::NeverBlock {
                name: "bank.example".into(),
            },
        ] {
            assert!(!error.to_string().is_empty());
        }
    }

    // -- owners ----------------------------------------------------------------------------

    #[test]
    fn union_of_two_owners() {
        let owners: BTreeMap<ProductId, OwnerWants> = [
            (
                ProductId::Shield,
                wants(true, None, false, &["shared.example", "shield.example"]),
            ),
            (
                ProductId::Ubiqx,
                wants(true, Some(CF), true, &["ubiqx.example", "shared.example"]),
            ),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            union(&owners),
            wants(
                true,
                Some(CF),
                true,
                &["ubiqx.example", "shared.example", "shield.example"]
            )
        );
        // An owner that does not want the hosts tier contributes no names.
        let owners: BTreeMap<ProductId, OwnerWants> = [
            (
                ProductId::Ubiqx,
                wants(false, None, true, &["ubiqx.example"]),
            ),
            (
                ProductId::Shield,
                wants(true, None, false, &["shield.example"]),
            ),
        ]
        .into_iter()
        .collect();
        assert_eq!(union(&owners), wants(true, None, true, &["shield.example"]));
        assert_eq!(union(&BTreeMap::new()), OwnerWants::default());
    }

    #[test]
    fn applying_a_plan_replaces_its_owners_wants_only() {
        let before = state(&[
            (
                ProductId::Ubiqx,
                wants(true, Some(CF), true, &["old.example"]),
            ),
            (
                ProductId::Shield,
                wants(true, None, false, &["shield.example"]),
            ),
        ]);
        let mine = SystemPlan {
            resolver: None,
            policies: false,
            ..plan(&["new.example"])
        };
        let after = owners_after(Some(&before), &mine);
        assert_eq!(
            after[&ProductId::Ubiqx],
            wants(true, None, false, &["new.example"])
        );
        assert_eq!(after[&ProductId::Shield], before.owners[&ProductId::Shield]);
        assert_eq!(owners_after(None, &mine).len(), 1);
    }

    #[test]
    fn the_names_of_a_block_drop_what_stopped_being_admissible() {
        let owners: BTreeMap<ProductId, OwnerWants> = [
            (
                ProductId::Ubiqx,
                wants(true, None, false, &["mine.example", "login.bank.example"]),
            ),
            (
                ProductId::Shield,
                wants(
                    true,
                    None,
                    false,
                    &["theirs.example", "not valid", "printer.lan"],
                ),
            ),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            hosts_names(&owners, TIER, &rules(), Os::Linux),
            vec![
                hosts::CANARY_NAME,
                "mine.example",
                "www.mine.example",
                "theirs.example",
                "www.theirs.example",
                "tier-one.example",
                "tier-two.example",
                "tier-three.example",
            ]
        );
        let nobody: BTreeMap<ProductId, OwnerWants> = [(
            ProductId::Ubiqx,
            wants(false, Some(CF), true, &["mine.example"]),
        )]
        .into_iter()
        .collect();
        assert!(hosts_names(&nobody, TIER, &rules(), Os::Linux).is_empty());
    }

    // -- assess: hosts (table of 2.2) ------------------------------------------------------

    fn hosts_observation(
        block: Option<&str>,
        owners: &[(ProductId, OwnerWants)],
    ) -> SystemObservation {
        SystemObservation {
            hosts: match block {
                Some(block) => hosts_file(block),
                None => HostsObservation {
                    text: Some("127.0.0.1 localhost\n".to_owned()),
                    error: None,
                },
            },
            state: Some(state(owners)),
            ..SystemObservation::default()
        }
    }

    fn hosts_only() -> NofapConfig {
        let mut config = chosen();
        config.system.resolver = None;
        config.system.policies = false;
        config
    }

    #[test]
    fn hosts_applied_when_the_block_is_what_would_be_written_now() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        let observation = hosts_observation(Some(&block_for(&owners, Os::Linux)), &owners);
        let assessment = assessed(&hosts_only(), &observation, Os::Linux);
        assert_eq!(assessment.hosts, TierState::Applied);
        assert_eq!(assessment.hosts_waiting, 0);
        assert!(assessment.issues.is_empty());
        // The block of another system has another cap and may differ; here it does not.
        assert_eq!(
            assessed(&hosts_only(), &observation, Os::Windows).hosts,
            TierState::Applied
        );
        // The other product's names are part of what would be written.
        let both = [
            (ProductId::Ubiqx, my_wants()),
            (
                ProductId::Shield,
                wants(true, None, false, &["theirs.example"]),
            ),
        ];
        let observation = hosts_observation(Some(&block_for(&both, Os::Linux)), &both);
        assert_eq!(
            assessed(&hosts_only(), &observation, Os::Linux).hosts,
            TierState::Applied
        );
    }

    #[test]
    fn hosts_update_pending_when_the_app_would_now_write_something_else() {
        // The block was written before the user added "mine.example".
        let written = [(ProductId::Ubiqx, wants(true, None, false, &[]))];
        let observation = hosts_observation(Some(&block_for(&written, Os::Linux)), &written);
        let assessment = assessed(&hosts_only(), &observation, Os::Linux);
        assert_eq!(assessment.hosts, TierState::UpdatePending);
        assert_eq!(
            assessment.hosts_waiting, 1,
            "the site and its www twin are one"
        );
        // A new app version brought a longer tier: two more names wait.
        let old_tier: BTreeMap<ProductId, OwnerWants> =
            [(ProductId::Ubiqx, my_wants())].into_iter().collect();
        let old_block =
            hosts::render_block(&hosts_names(&old_tier, &TIER[..1], &rules(), Os::Linux));
        let observation = hosts_observation(Some(&old_block), &[(ProductId::Ubiqx, my_wants())]);
        let assessment = assessed(&hosts_only(), &observation, Os::Linux);
        assert_eq!(
            (assessment.hosts, assessment.hosts_waiting),
            (TierState::UpdatePending, 2)
        );
        // A site the user took off the list: something else would be written, nothing waits.
        let more = [(
            ProductId::Ubiqx,
            wants(true, None, false, &["mine.example", "gone.example"]),
        )];
        let observation = hosts_observation(Some(&block_for(&more, Os::Linux)), &more);
        let assessment = assessed(&hosts_only(), &observation, Os::Linux);
        assert_eq!(
            (assessment.hosts, assessment.hosts_waiting),
            (TierState::UpdatePending, 0)
        );
    }

    #[test]
    fn hosts_changed_when_the_lines_are_not_what_the_marker_declares() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        let edited = block_for(&owners, Os::Linux).replace("tier-two.example", "tier-2.example");
        let observation = hosts_observation(Some(&edited), &owners);
        assert_eq!(
            assessed(&hosts_only(), &observation, Os::Linux).hosts,
            TierState::Changed
        );
        // The END marker deleted.
        let cut = block_for(&owners, Os::Linux).replace("# <<< ubiqx-shield END\n", "");
        let observation = hosts_observation(Some(&cut), &owners);
        assert_eq!(
            assessed(&hosts_only(), &observation, Os::Linux).hosts,
            TierState::Changed
        );
    }

    #[test]
    fn hosts_missing_when_the_state_lists_it_and_the_block_is_gone() {
        let observation = hosts_observation(None, &[(ProductId::Ubiqx, my_wants())]);
        assert_eq!(
            assessed(&hosts_only(), &observation, Os::Linux).hosts,
            TierState::Missing
        );
    }

    #[test]
    fn hosts_not_applied_when_never_applied_or_declined() {
        // No state file at all.
        let never = SystemObservation {
            hosts: HostsObservation {
                text: Some("127.0.0.1 localhost\n".into()),
                error: None,
            },
            ..SystemObservation::default()
        };
        assert_eq!(
            assessed(&hosts_only(), &never, Os::Linux).hosts,
            TierState::NotApplied
        );
        // A state file that does not list this product for the tier.
        let other_tiers = hosts_observation(
            None,
            &[(ProductId::Ubiqx, wants(false, Some(CF), false, &[]))],
        );
        assert_eq!(
            assessed(&hosts_only(), &other_tiers, Os::Linux).hosts,
            TierState::NotApplied
        );
    }

    #[test]
    fn hosts_gated_when_the_gate_is_closed() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        let observation = hosts_observation(Some(&block_for(&owners, Os::Linux)), &owners);
        for os in Os::ALL {
            let assessment = assessed_with(&chosen(), &observation, os, &closed(os), 0);
            assert_eq!(assessment.hosts, TierState::Gated);
            assert_eq!(assessment.resolver, TierState::Gated);
            assert_eq!(assessment.policies, TierState::Gated);
            assert!(assessment.issues.is_empty());
        }
    }

    #[test]
    fn hosts_unsupported_when_the_file_cannot_be_read_or_the_app_is_an_appimage() {
        for code in [reason::UNREADABLE, reason::APPIMAGE, reason::WRITE_REFUSED] {
            let observation = SystemObservation {
                hosts: HostsObservation {
                    text: Some("127.0.0.1 localhost\n".into()),
                    error: Some(code.to_owned()),
                },
                ..SystemObservation::default()
            };
            assert_eq!(
                assessed(&hosts_only(), &observation, Os::Linux).hosts,
                unsupported(code)
            );
        }
        let no_text = SystemObservation::default();
        assert_eq!(
            assessed(&hosts_only(), &no_text, Os::Linux).hosts,
            unsupported(reason::UNREADABLE)
        );
    }

    #[test]
    fn hosts_off_when_not_chosen_and_in_use_by_other_when_it_is_the_other_products() {
        let mut config = hosts_only();
        config.system.hosts = false;
        let nothing = hosts_observation(None, &[]);
        assert_eq!(assessed(&config, &nothing, Os::Linux).hosts, TierState::Off);
        let theirs = [(
            ProductId::Shield,
            wants(true, None, false, &["theirs.example"]),
        )];
        let observation = hosts_observation(Some(&block_for(&theirs, Os::Linux)), &theirs);
        assert_eq!(
            assessed(&config, &observation, Os::Linux).hosts,
            TierState::InUseByOther
        );
        // Chosen here too, but only the other product has applied it so far.
        assert_eq!(
            assessed(&hosts_only(), &observation, Os::Linux).hosts,
            TierState::InUseByOther
        );
        // Switched off in the app and not yet removed from the system.
        let mine = [(ProductId::Ubiqx, my_wants())];
        let left = hosts_observation(Some(&block_for(&mine, Os::Linux)), &mine);
        assert_eq!(
            assessed(&config, &left, Os::Linux).hosts,
            TierState::Applied
        );
        // Off and closed is still off: the gate matters for what is chosen.
        assert_eq!(
            assessed_with(&config, &nothing, Os::Linux, &closed(Os::Linux), 0).hosts,
            TierState::Off
        );
    }

    #[test]
    fn the_canary_must_resolve_to_the_null_address() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        let with_canary = |answers: Option<&[&str]>| {
            let mut observation = hosts_observation(Some(&block_for(&owners, Os::Linux)), &owners);
            observation.canary = answers.map(|list| list.iter().map(|a| (*a).to_owned()).collect());
            assessed(&hosts_only(), &observation, Os::Linux).issues
        };
        assert!(
            with_canary(None).is_empty(),
            "not looked up is not a finding"
        );
        assert!(with_canary(Some(&["0.0.0.0"])).is_empty());
        assert!(with_canary(Some(&["::"])).is_empty());
        assert!(with_canary(Some(&["0.0.0.0", "::", "0:0:0:0:0:0:0:0"])).is_empty());
        assert_eq!(
            with_canary(Some(&[])),
            vec![Issue::HostsNotHonoured],
            "it did not resolve"
        );
        assert_eq!(
            with_canary(Some(&["93.184.216.34"])),
            vec![Issue::HostsNotHonoured]
        );
        assert_eq!(
            with_canary(Some(&["0.0.0.0", "10.0.0.1"])),
            vec![Issue::HostsNotHonoured]
        );
        assert_eq!(
            with_canary(Some(&["garbage"])),
            vec![Issue::HostsNotHonoured]
        );
        // Without the block in place the canary says nothing.
        let mut missing = hosts_observation(None, &owners);
        missing.canary = Some(Vec::new());
        assert!(assessed(&hosts_only(), &missing, Os::Linux)
            .issues
            .is_empty());
    }

    // -- assess: resolver (table of 2.3) ---------------------------------------------------

    fn resolver_only() -> NofapConfig {
        let mut config = chosen();
        config.system.hosts = false;
        config.system.policies = false;
        config.internal.previous_dns = vec![
            PreviousDns {
                interface: "Wi-Fi".into(),
                servers: vec!["192.168.0.2".into()],
            },
            PreviousDns {
                interface: "Ethernet".into(),
                servers: Vec::new(),
            },
        ];
        config
    }

    fn dns_observation(
        interfaces: Vec<InterfaceDns>,
        owners: &[(ProductId, OwnerWants)],
    ) -> SystemObservation {
        SystemObservation {
            resolver: ResolverObservation {
                interfaces,
                full: true,
                ..ResolverObservation::default()
            },
            state: Some(state(owners)),
            ..SystemObservation::default()
        }
    }

    #[test]
    fn resolver_applied_when_every_active_interface_uses_the_set() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        for os in [Os::Macos, Os::Windows] {
            let observation = dns_observation(
                vec![
                    interface("Wi-Fi", true, &CF_SERVERS),
                    interface(
                        "Ethernet",
                        true,
                        &[
                            "2606:4700:4700::1003",
                            "1.0.0.3",
                            "1.1.1.3",
                            "2606:4700:4700::1113",
                        ],
                    ),
                    // A disabled service does not count.
                    interface("Thunderbolt Bridge", false, &[]),
                ],
                &owners,
            );
            let assessment = assessed(&resolver_only(), &observation, os);
            assert_eq!(assessment.resolver, TierState::Applied, "{os:?}");
            assert!(assessment.issues.is_empty());
        }
    }

    #[test]
    fn resolver_changed_when_an_interface_uses_something_else() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        // An interface that was there at the apply and was changed since: no guess about why.
        let known = dns_observation(
            vec![
                interface("Wi-Fi", true, &CF_SERVERS),
                interface("Ethernet", true, &["192.168.0.2"]),
            ],
            &owners,
        );
        let assessment = assessed(&resolver_only(), &known, Os::Macos);
        assert_eq!(assessment.resolver, TierState::Changed);
        assert!(assessment.issues.is_empty());
        // An interface that appeared after the apply.
        let new = dns_observation(
            vec![
                interface("Wi-Fi", true, &CF_SERVERS),
                interface("USB 10/100 LAN", true, &[]),
            ],
            &owners,
        );
        let assessment = assessed(&resolver_only(), &new, Os::Macos);
        assert_eq!(assessment.resolver, TierState::Changed);
        assert_eq!(assessment.issues, vec![Issue::NewConnectionUnfiltered]);
        // Only half of the set (the IPv6 servers were removed) is something else too.
        let half = dns_observation(vec![interface("Wi-Fi", true, &CF_SERVERS[..2])], &owners);
        assert_eq!(
            assessed(&resolver_only(), &half, Os::Windows).resolver,
            TierState::Changed
        );
        // Every interface back to automatic.
        let none = dns_observation(vec![interface("Wi-Fi", true, &[])], &owners);
        assert_eq!(
            assessed(&resolver_only(), &none, Os::Windows).resolver,
            TierState::Changed
        );
    }

    #[test]
    fn resolver_off_when_the_user_chose_none() {
        let mut config = resolver_only();
        config.system.resolver = None;
        let observation = dns_observation(vec![interface("Wi-Fi", true, &[])], &[]);
        assert_eq!(
            assessed(&config, &observation, Os::Macos).resolver,
            TierState::Off
        );
        // In place for the other product only.
        let theirs = [(ProductId::Shield, wants(false, Some(CF), false, &[]))];
        let observation = dns_observation(vec![interface("Wi-Fi", true, &CF_SERVERS)], &theirs);
        assert_eq!(
            assessed(&config, &observation, Os::Macos).resolver,
            TierState::InUseByOther
        );
        assert_eq!(
            assessed(&resolver_only(), &observation, Os::Macos).resolver,
            TierState::InUseByOther
        );
        // Somebody who set the same servers by hand is not "the other product".
        let by_hand = dns_observation(vec![interface("Wi-Fi", true, &CF_SERVERS)], &[]);
        assert_eq!(
            assessed(&config, &by_hand, Os::Macos).resolver,
            TierState::Off
        );
    }

    #[test]
    fn resolver_not_applied_after_restoring_the_network_or_before_any_apply() {
        let owners = [(ProductId::Ubiqx, wants(true, None, true, &[]))];
        let mut restored = resolver_only();
        restored.system.network_restored = true;
        let observation = dns_observation(vec![interface("Wi-Fi", true, &[])], &owners);
        let assessment = assessed(&restored, &observation, Os::Macos);
        assert_eq!(assessment.resolver, TierState::NotApplied);
        assert!(assessment.issues.is_empty(), "restoring is not drift");
        // Chosen and never applied (the prompt was declined).
        assert_eq!(
            assessed(&resolver_only(), &observation, Os::Macos).resolver,
            TierState::NotApplied
        );
        let no_state = SystemObservation {
            resolver: ResolverObservation {
                interfaces: vec![interface("Wi-Fi", true, &["192.168.0.2"])],
                ..Default::default()
            },
            ..SystemObservation::default()
        };
        assert_eq!(
            assessed(&resolver_only(), &no_state, Os::Windows).resolver,
            TierState::NotApplied
        );
    }

    #[test]
    fn resolver_without_an_active_interface_follows_the_state_file() {
        let offline = vec![interface("Wi-Fi", false, &CF_SERVERS)];
        let applied = dns_observation(offline.clone(), &[(ProductId::Ubiqx, my_wants())]);
        assert_eq!(
            assessed(&resolver_only(), &applied, Os::Macos).resolver,
            TierState::Applied
        );
        let never = dns_observation(offline, &[]);
        assert_eq!(
            assessed(&resolver_only(), &never, Os::Macos).resolver,
            TierState::NotApplied
        );
    }

    fn linux_dns(
        dropin: Option<String>,
        active: Option<bool>,
        uses: Option<bool>,
        owners: &[(ProductId, OwnerWants)],
    ) -> SystemObservation {
        SystemObservation {
            resolver: ResolverObservation {
                dropin,
                resolved_active: active,
                resolv_conf_uses_resolved: uses,
                ..ResolverObservation::default()
            },
            state: Some(state(owners)),
            ..SystemObservation::default()
        }
    }

    #[test]
    fn resolver_on_linux_is_the_drop_in() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        let ours = resolvers::resolved_dropin(CF);
        let applied = linux_dns(Some(ours.clone()), Some(true), Some(true), &owners);
        assert_eq!(
            assessed(&resolver_only(), &applied, Os::Linux).resolver,
            TierState::Applied
        );
        let edited = linux_dns(
            Some(ours.replace("1.1.1.3", "9.9.9.9")),
            Some(true),
            Some(true),
            &owners,
        );
        assert_eq!(
            assessed(&resolver_only(), &edited, Os::Linux).resolver,
            TierState::Changed
        );
        let deleted = linux_dns(None, Some(true), Some(true), &owners);
        assert_eq!(
            assessed(&resolver_only(), &deleted, Os::Linux).resolver,
            TierState::Missing
        );
        let never = linux_dns(None, Some(true), Some(true), &[]);
        assert_eq!(
            assessed(&resolver_only(), &never, Os::Linux).resolver,
            TierState::NotApplied
        );
    }

    #[test]
    fn resolver_unsupported_on_linux_without_systemd_resolved_in_use() {
        let ours = Some(resolvers::resolved_dropin(CF));
        let owners = [(ProductId::Ubiqx, my_wants())];
        for (active, uses, code) in [
            (Some(false), Some(true), reason::NO_RESOLVED),
            // Running, but /etc/resolv.conf does not point at it: the drop-in changes nothing.
            (Some(true), Some(false), reason::NO_RESOLVED),
            (Some(false), None, reason::NO_RESOLVED),
            (None, Some(true), reason::UNREADABLE),
            (Some(true), None, reason::UNREADABLE),
        ] {
            let observation = linux_dns(ours.clone(), active, uses, &owners);
            assert_eq!(
                assessed(&resolver_only(), &observation, Os::Linux).resolver,
                unsupported(code),
                "{active:?} {uses:?}"
            );
        }
        // A read that failed, on any system.
        let mut failed = dns_observation(Vec::new(), &owners);
        failed.resolver.error = Some(reason::UNREADABLE.to_owned());
        assert_eq!(
            assessed(&resolver_only(), &failed, Os::Windows).resolver,
            unsupported(reason::UNREADABLE)
        );
    }

    #[test]
    fn resolver_unreachable_after_three_failed_probes_while_applied() {
        let owners = [(ProductId::Ubiqx, my_wants())];
        let applied = dns_observation(vec![interface("Wi-Fi", true, &CF_SERVERS)], &owners);
        let issues = |streak: u32, observation: &SystemObservation| {
            assessed_with(
                &resolver_only(),
                observation,
                Os::Macos,
                &open(Os::Macos),
                streak,
            )
            .issues
        };
        assert!(issues(0, &applied).is_empty());
        assert!(issues(2, &applied).is_empty());
        assert_eq!(issues(3, &applied), vec![Issue::ResolverUnreachable]);
        assert_eq!(issues(40, &applied), vec![Issue::ResolverUnreachable]);
        // Not while the tier is not applied: then the resolver is not the reason.
        let not_applied = dns_observation(vec![interface("Wi-Fi", true, &[])], &[]);
        assert!(issues(40, &not_applied).is_empty());
    }

    // -- assess: policies (rules of 2.4) ---------------------------------------------------

    fn policies_only() -> NofapConfig {
        let mut config = chosen();
        config.system.hosts = false;
        config.system.resolver = None;
        config
    }

    fn policy_observation(
        policies: Vec<PolicyObservation>,
        owners: &[(ProductId, OwnerWants)],
        created: &[BrowserFamily],
    ) -> SystemObservation {
        let mut recorded = state(owners);
        recorded.policies_created = created_for(created);
        SystemObservation {
            policies,
            state: Some(recorded),
            ..SystemObservation::default()
        }
    }

    #[test]
    fn policies_applied_when_every_browser_holds_our_values() {
        use BrowserFamily::{Chrome, Edge, Firefox};
        let owners = [(ProductId::Ubiqx, my_wants())];
        let mut absent = empty_policy(BrowserFamily::Brave);
        absent.present = false;
        let observation = policy_observation(
            vec![
                applied_policy(Chrome),
                applied_policy(Edge),
                applied_policy(Firefox),
                absent,
            ],
            &owners,
            &[Chrome, Edge, Firefox],
        );
        for os in [Os::Windows, Os::Linux] {
            let assessment = assessed(&policies_only(), &observation, os);
            assert_eq!(assessment.policies, TierState::Applied, "{os:?}");
            assert!(assessment.issues.is_empty());
        }
    }

    #[test]
    fn policies_not_applied_before_any_apply() {
        use BrowserFamily::{Chrome, Firefox};
        let observation =
            policy_observation(vec![empty_policy(Chrome), empty_policy(Firefox)], &[], &[]);
        assert_eq!(
            assessed(&policies_only(), &observation, Os::Windows).policies,
            TierState::NotApplied
        );
        let no_state = SystemObservation {
            policies: vec![empty_policy(Chrome)],
            ..Default::default()
        };
        assert_eq!(
            assessed(&policies_only(), &no_state, Os::Linux).policies,
            TierState::NotApplied
        );
    }

    #[test]
    fn policies_changed_missing_and_update_pending() {
        use BrowserFamily::{Brave, Chrome, Firefox};
        let owners = [(ProductId::Ubiqx, my_wants())];
        // A value we created was changed.
        let mut tampered = applied_policy(Chrome);
        tampered.values[0].value = json!("automatic");
        let observation = policy_observation(
            vec![tampered, applied_policy(Firefox)],
            &owners,
            &[Chrome, Firefox],
        );
        assert_eq!(
            assessed(&policies_only(), &observation, Os::Windows).policies,
            TierState::Changed
        );
        // One of our values was deleted, the others are still there.
        let mut partial = applied_policy(Chrome);
        partial.values.pop();
        let observation = policy_observation(vec![partial], &owners, &[Chrome]);
        assert_eq!(
            assessed(&policies_only(), &observation, Os::Linux).policies,
            TierState::Changed
        );
        // Everything we created is gone.
        let observation = policy_observation(
            vec![empty_policy(Chrome), empty_policy(Firefox)],
            &owners,
            &[Chrome, Firefox],
        );
        assert_eq!(
            assessed(&policies_only(), &observation, Os::Windows).policies,
            TierState::Missing
        );
        // A browser installed since the last apply: nothing drifted, something waits.
        let observation = policy_observation(
            vec![applied_policy(Chrome), empty_policy(Brave)],
            &owners,
            &[Chrome],
        );
        assert_eq!(
            assessed(&policies_only(), &observation, Os::Windows).policies,
            TierState::UpdatePending
        );
    }

    #[test]
    fn a_managed_machine_is_a_conflict_and_is_left_alone() {
        use BrowserFamily::{Chrome, Firefox};
        let owners = [(ProductId::Ubiqx, my_wants())];
        let mut managed = empty_policy(Chrome);
        managed.values = vec![PolicyValue {
            key: "DnsOverHttpsMode".into(),
            value: json!("secure"),
        }];
        // The other browser is ours and applied: the tier is applied, with the issue.
        let observation = policy_observation(
            vec![managed.clone(), applied_policy(Firefox)],
            &owners,
            &[Firefox],
        );
        let assessment = assessed(&policies_only(), &observation, Os::Windows);
        assert_eq!(assessment.policies, TierState::Applied);
        assert_eq!(assessment.issues, vec![Issue::PolicyConflict]);
        // Nothing but the managed browser: the tier cannot do anything here.
        let observation = policy_observation(vec![managed], &owners, &[]);
        let assessment = assessed(&policies_only(), &observation, Os::Windows);
        assert_eq!(assessment.policies, unsupported(reason::CONFLICT));
        assert_eq!(assessment.issues, vec![Issue::PolicyConflict]);
    }

    #[test]
    fn chromium_browsers_on_macos_are_advisory_only() {
        use BrowserFamily::{Chrome, Firefox};
        let owners = [(ProductId::Ubiqx, my_wants())];
        let observation = policy_observation(
            vec![empty_policy(Chrome), applied_policy(Firefox)],
            &owners,
            &[Firefox],
        );
        let assessment = assessed(&policies_only(), &observation, Os::Macos);
        assert_eq!(assessment.policies, TierState::Applied);
        assert_eq!(assessment.issues, vec![Issue::PolicyAdvisoryOnly]);
        // Only Chrome installed: applying wrote nothing, and nothing is missing.
        let observation = policy_observation(vec![empty_policy(Chrome)], &owners, &[]);
        let assessment = assessed(&policies_only(), &observation, Os::Macos);
        assert_eq!(assessment.policies, TierState::Applied);
        assert_eq!(assessment.issues, vec![Issue::PolicyAdvisoryOnly]);
        let never = policy_observation(vec![empty_policy(Chrome)], &[], &[]);
        assert_eq!(
            assessed(&policies_only(), &never, Os::Macos).policies,
            TierState::NotApplied
        );
        // The same observation on Windows is simply not applied yet, without the issue.
        let assessment = assessed(&policies_only(), &never, Os::Windows);
        assert_eq!(
            (assessment.policies, assessment.issues),
            (TierState::NotApplied, vec![])
        );
    }

    #[test]
    fn a_flatpak_browser_is_unsupported_for_that_browser_only() {
        use BrowserFamily::{Chromium, Firefox};
        let owners = [(ProductId::Ubiqx, my_wants())];
        let mut flatpak = empty_policy(Chromium);
        flatpak.error = Some(reason::FLATPAK.to_owned());
        let alone = policy_observation(vec![flatpak.clone()], &owners, &[]);
        assert_eq!(
            assessed(&policies_only(), &alone, Os::Linux).policies,
            unsupported(reason::FLATPAK)
        );
        let with_firefox =
            policy_observation(vec![flatpak, applied_policy(Firefox)], &owners, &[Firefox]);
        assert_eq!(
            assessed(&policies_only(), &with_firefox, Os::Linux).policies,
            TierState::Applied
        );
    }

    #[test]
    fn policies_off_and_in_use_by_other() {
        use BrowserFamily::Chrome;
        let mut config = policies_only();
        config.system.policies = false;
        let nothing = policy_observation(vec![empty_policy(Chrome)], &[], &[]);
        assert_eq!(
            assessed(&config, &nothing, Os::Windows).policies,
            TierState::Off
        );
        let theirs = [(ProductId::Shield, wants(false, None, true, &[]))];
        let observation = policy_observation(vec![applied_policy(Chrome)], &theirs, &[Chrome]);
        assert_eq!(
            assessed(&config, &observation, Os::Windows).policies,
            TierState::InUseByOther
        );
        assert_eq!(
            assessed(&policies_only(), &observation, Os::Windows).policies,
            TierState::InUseByOther
        );
        // Switched off in the app, still on the machine for this product.
        let mine = [(ProductId::Ubiqx, my_wants())];
        let left = policy_observation(vec![applied_policy(Chrome)], &mine, &[Chrome]);
        assert_eq!(
            assessed(&config, &left, Os::Windows).policies,
            TierState::Applied
        );
    }

    #[test]
    fn issues_come_in_a_fixed_order() {
        use BrowserFamily::{Chrome, Firefox};
        let owners = [(ProductId::Ubiqx, my_wants())];
        let mut managed = empty_policy(Firefox);
        managed.values = vec![PolicyValue {
            key: policies::FIREFOX_ENABLED.into(),
            value: json!(true),
        }];
        let mut observation = hosts_observation(Some(&block_for(&owners, Os::Macos)), &owners);
        observation.canary = Some(vec!["203.0.113.7".to_owned()]);
        observation.resolver.interfaces = vec![interface("Wi-Fi", true, &CF_SERVERS)];
        observation.policies = vec![empty_policy(Chrome), managed];
        let assessment = assessed_with(&chosen(), &observation, Os::Macos, &open(Os::Macos), 3);
        assert_eq!(
            assessment.issues,
            vec![
                Issue::HostsNotHonoured,
                Issue::ResolverUnreachable,
                Issue::PolicyConflict,
                Issue::PolicyAdvisoryOnly,
            ]
        );
        assert_eq!(assessment.hosts, TierState::Applied);
        assert_eq!(assessment.resolver, TierState::Applied);
        // A state file that could not be read is no state file: the tiers are judged by
        // what is on the machine.
        observation.state = None;
        observation.state_error = Some(reason::UNREADABLE.to_owned());
        let assessment = assessed_with(&chosen(), &observation, Os::Macos, &open(Os::Macos), 0);
        assert_eq!(assessment.hosts, TierState::Applied);
        assert_eq!(assessment.resolver, TierState::Applied);
        assert_eq!(assessment.policies, unsupported(reason::CONFLICT));
    }

    // -- preview ---------------------------------------------------------------------------

    fn previewed(
        plan: &SystemPlan,
        observation: &SystemObservation,
        os: Os,
    ) -> Vec<(String, Option<String>)> {
        let lines = preview_with(plan, observation, os, TIER, &rules());
        for planned in &lines {
            let (_, tier, placeholder) = line::LINES
                .iter()
                .find(|(key, _, _)| *key == planned.text_key)
                .unwrap_or_else(|| panic!("{} is not a preview line", planned.text_key));
            assert_eq!(planned.tier, *tier, "{}", planned.text_key);
            assert_eq!(
                planned.detail.is_some(),
                placeholder.is_some(),
                "{}",
                planned.text_key
            );
        }
        lines
            .into_iter()
            .map(|planned| (planned.text_key, planned.detail))
            .collect()
    }

    fn keys(lines: &[(String, Option<String>)]) -> Vec<&str> {
        lines.iter().map(|(key, _)| key.as_str()).collect()
    }

    /// A machine nothing was applied to: a hosts file, one interface with a manual DNS,
    /// Chrome and Firefox installed.
    fn untouched(os: Os) -> SystemObservation {
        SystemObservation {
            hosts: HostsObservation {
                text: Some("127.0.0.1 localhost\n".into()),
                error: None,
            },
            resolver: if os == Os::Linux {
                ResolverObservation {
                    resolved_active: Some(true),
                    resolv_conf_uses_resolved: Some(true),
                    ..Default::default()
                }
            } else {
                ResolverObservation {
                    interfaces: vec![
                        interface("Wi-Fi", true, &["192.168.0.2", "192.168.0.3"]),
                        interface("Ethernet", true, &[]),
                        interface("Old VPN", false, &["10.9.9.9"]),
                    ],
                    full: true,
                    ..Default::default()
                }
            },
            policies: vec![
                empty_policy(BrowserFamily::Chrome),
                empty_policy(BrowserFamily::Firefox),
            ],
            ..SystemObservation::default()
        }
    }

    #[test]
    fn preview_of_a_first_apply() {
        let lines = previewed(
            &plan(&["mine.example"]),
            &untouched(Os::Windows),
            Os::Windows,
        );
        assert_eq!(
            lines,
            vec![
                // Three tier names, the user's site and its www twin; the canary is no site.
                (line::HOSTS_ADD.to_owned(), Some("5".to_owned())),
                (line::HOSTS_READABLE.to_owned(), None),
                (
                    line::RESOLVER_SET.to_owned(),
                    Some("Cloudflare for Families".to_owned())
                ),
                (
                    line::DNS_PREVIOUS.to_owned(),
                    Some("192.168.0.2, 192.168.0.3".to_owned())
                ),
                (
                    line::POLICIES_SET.to_owned(),
                    Some("Chrome, Firefox".to_owned())
                ),
                (line::POLICIES_DISCLOSURE.to_owned(), None),
            ]
        );
        // Without sites of the user nothing readable is added; on macOS Chrome gets nothing.
        let lines = previewed(&plan(&[]), &untouched(Os::Macos), Os::Macos);
        assert_eq!(
            lines,
            vec![
                (line::HOSTS_ADD.to_owned(), Some("3".to_owned())),
                (
                    line::RESOLVER_SET.to_owned(),
                    Some("Cloudflare for Families".to_owned())
                ),
                (
                    line::DNS_PREVIOUS.to_owned(),
                    Some("192.168.0.2, 192.168.0.3".to_owned())
                ),
                (line::POLICIES_SET.to_owned(), Some("Firefox".to_owned())),
                (line::POLICIES_DISCLOSURE.to_owned(), None),
            ]
        );
        // Linux has no interface list: no "previous DNS" line.
        let lines = previewed(&plan(&[]), &untouched(Os::Linux), Os::Linux);
        assert_eq!(
            keys(&lines),
            vec![
                line::HOSTS_ADD,
                line::RESOLVER_SET,
                line::POLICIES_SET,
                line::POLICIES_DISCLOSURE
            ]
        );
    }

    #[test]
    fn preview_is_empty_when_everything_is_in_place() {
        let owners = [(
            ProductId::Ubiqx,
            wants(true, Some(CF), true, &["mine.example"]),
        )];
        let mut observation = hosts_observation(Some(&block_for(&owners, Os::Windows)), &owners);
        observation.resolver.interfaces = vec![interface("Wi-Fi", true, &CF_SERVERS)];
        observation.policies = vec![applied_policy(BrowserFamily::Chrome)];
        assert!(previewed(&plan(&["mine.example"]), &observation, Os::Windows).is_empty());
        // One more site: only the hosts block changes.
        let lines = previewed(
            &plan(&["mine.example", "second.example"]),
            &observation,
            Os::Windows,
        );
        assert_eq!(
            lines,
            vec![
                (line::HOSTS_UPDATE.to_owned(), Some("7".to_owned())),
                (line::HOSTS_READABLE.to_owned(), None),
            ]
        );
    }

    #[test]
    fn preview_of_switching_everything_off() {
        let owners = [(
            ProductId::Ubiqx,
            wants(true, Some(CF), true, &["mine.example"]),
        )];
        let mut observation = hosts_observation(Some(&block_for(&owners, Os::Macos)), &owners);
        observation.resolver.interfaces = vec![interface("Wi-Fi", true, &CF_SERVERS)];
        observation.policies = vec![applied_policy(BrowserFamily::Firefox)];
        if let Some(state) = &mut observation.state {
            state.policies_created = created_for(&[BrowserFamily::Firefox]);
        }
        let off = SystemPlan {
            hosts: false,
            resolver: None,
            policies: false,
            ..plan(&[])
        };
        let lines = previewed(&off, &observation, Os::Macos);
        assert_eq!(
            keys(&lines),
            vec![
                line::HOSTS_REMOVE,
                line::RESOLVER_RESET,
                line::POLICIES_REMOVE
            ]
        );
        // While the other product still wants the tiers, nothing is removed: the block is
        // rewritten without this product's sites.
        let both = [
            (
                ProductId::Ubiqx,
                wants(true, Some(CF), true, &["mine.example"]),
            ),
            (ProductId::Shield, wants(true, Some(CF), true, &[])),
        ];
        let mut shared = hosts_observation(Some(&block_for(&both, Os::Macos)), &both);
        shared.resolver.interfaces = vec![interface("Wi-Fi", true, &CF_SERVERS)];
        shared.policies = vec![applied_policy(BrowserFamily::Firefox)];
        let lines = previewed(&off, &shared, Os::Macos);
        assert_eq!(
            lines,
            vec![(line::HOSTS_UPDATE.to_owned(), Some("3".to_owned()))]
        );
    }

    #[test]
    fn preview_never_promises_to_reset_what_is_not_ours() {
        // The user set the same servers by hand and never used the tier.
        let mut observation = untouched(Os::Macos);
        observation.resolver.interfaces = vec![interface("Wi-Fi", true, &CF_SERVERS)];
        let off = SystemPlan {
            hosts: false,
            resolver: None,
            policies: false,
            ..plan(&[])
        };
        assert!(previewed(&off, &observation, Os::Macos).is_empty());
        // The tier was ours, and every interface was changed since: nothing to reset.
        let owners = [(ProductId::Ubiqx, wants(false, Some(CF), false, &[]))];
        let changed = dns_observation(vec![interface("Wi-Fi", true, &["192.168.0.2"])], &owners);
        assert!(previewed(&off, &changed, Os::Macos).is_empty());
    }

    #[test]
    fn preview_adds_no_line_for_a_tier_that_cannot_work_here() {
        let mut observation = untouched(Os::Linux);
        observation.hosts.error = Some(reason::APPIMAGE.to_owned());
        observation.resolver.resolved_active = Some(false);
        let mut flatpak = empty_policy(BrowserFamily::Chromium);
        flatpak.error = Some(reason::FLATPAK.to_owned());
        observation.policies = vec![flatpak];
        assert!(previewed(&plan(&["mine.example"]), &observation, Os::Linux).is_empty());
        // A gated tier is not in the plan that is sent, so it adds no line either.
        let sent = without_gated(
            &plan(&["mine.example"]),
            &Gate::new(Os::Linux, vec![Tier::Policies]),
        );
        let lines = previewed(&sent, &untouched(Os::Linux), Os::Linux);
        assert_eq!(
            keys(&lines),
            vec![line::POLICIES_SET, line::POLICIES_DISCLOSURE]
        );
    }

    #[test]
    fn preview_names_the_browsers_that_are_left_alone() {
        let mut managed = empty_policy(BrowserFamily::Edge);
        managed.values = vec![PolicyValue {
            key: "ForceBingSafeSearch".into(),
            value: json!(0),
        }];
        let mut observation = untouched(Os::Windows);
        observation.policies = vec![managed, empty_policy(BrowserFamily::Brave)];
        let only_policies = SystemPlan {
            hosts: false,
            resolver: None,
            ..plan(&[])
        };
        assert_eq!(
            previewed(&only_policies, &observation, Os::Windows),
            vec![
                (line::POLICIES_SET.to_owned(), Some("Brave".to_owned())),
                (line::POLICIES_DISCLOSURE.to_owned(), None),
                (line::POLICIES_CONFLICT.to_owned(), Some("Edge".to_owned())),
            ]
        );
    }

    // -- undo ------------------------------------------------------------------------------

    #[test]
    fn reset_targets_leave_an_interface_the_user_changed() {
        let observation = dns_observation(
            vec![
                interface("5", true, &CF_SERVERS),
                interface("7", true, &["192.168.0.2"]),
                interface("9", true, &[]),
                // Still ours although it is down: it is reset too.
                interface(
                    "11",
                    false,
                    &[
                        "1.0.0.3",
                        "1.1.1.3",
                        "2606:4700:4700::1003",
                        "2606:4700:4700::1113",
                    ],
                ),
                // Ours plus one more server: the user changed it.
                interface(
                    "13",
                    true,
                    &[
                        "1.1.1.3",
                        "1.0.0.3",
                        "2606:4700:4700::1113",
                        "2606:4700:4700::1003",
                        "8.8.8.8",
                    ],
                ),
            ],
            &[],
        );
        assert_eq!(reset_targets(&observation, CF), vec!["5", "11"]);
        assert!(reset_targets(&SystemObservation::default(), CF).is_empty());
    }

    #[test]
    fn a_listed_host_in_front_means_bypass_only_while_the_block_is_in_place() {
        for (state, in_place) in [
            (TierState::Applied, true),
            (TierState::UpdatePending, true),
            (TierState::Off, false),
            (TierState::Changed, false),
            (TierState::Missing, false),
            (TierState::NotApplied, false),
            (TierState::Gated, false),
            (unsupported(reason::UNREADABLE), false),
            (TierState::InUseByOther, false),
        ] {
            assert_eq!(bypass_suspected(true, &state), in_place, "{state:?}");
            assert!(!bypass_suspected(false, &state), "{state:?}");
        }
    }

    #[test]
    fn the_stamp_in_the_state_file_is_the_stamp_of_the_block() {
        let owners: BTreeMap<ProductId, OwnerWants> =
            [(ProductId::Ubiqx, my_wants())].into_iter().collect();
        let names = hosts_names(&owners, TIER, &rules(), Os::Linux);
        let stamp: HostsStamp = hosts::stamp_of(&names);
        assert_eq!(
            hosts::inspect(hosts::render_block(&names).as_bytes()),
            BlockInspection::Intact {
                sha256: stamp.sha256,
                names: stamp.names
            }
        );
        // A report in the state file does not change any judgement.
        let mut recorded = state(&[(ProductId::Ubiqx, my_wants())]);
        recorded.report = vec![TierReport {
            tier: Tier::Resolver,
            ok: false,
            reason: Some(reason::UNREACHABLE.into()),
        }];
        let observation = SystemObservation {
            hosts: hosts_file(&hosts::render_block(&names)),
            state: Some(recorded),
            ..SystemObservation::default()
        };
        assert_eq!(
            assessed(&hosts_only(), &observation, Os::Linux).hosts,
            TierState::Applied
        );
    }
}
