//! The per-OS, per-tier gate (design section 1.8): a system tier is closed on an operating
//! system until that system's checks have passed. A closed tier is shown as `Gated` and the
//! helper refuses that section of a plan.
//!
//! Every entry lands `false`. An entry for `hosts` or `resolver` is flipped in the commit
//! that follows a green run of the CI job `nofap-system-smoke` on that OS (or, for
//! `(Linux, hosts)`, a green run of `scripts/nofap/system-smoke.sh --tiers hosts` in the
//! build sandbox), with the evidence recorded in `docs/nofap/REAL-MACHINE-CHECKLIST.md`.
//! An entry for `policies` is flipped only after the manual browser checks.
//!
//! Seeded by the foundation unit: the table and the types are complete; signatures only.
#![allow(unused_variables)]

use ubiqx_core::nofap::{Os, Tier};

/// Which tier is open on which system.
pub const TIER_GATE: [(Os, Tier, bool); 9] = [
    (Os::Macos, Tier::Hosts, false),
    (Os::Macos, Tier::Resolver, false),
    (Os::Macos, Tier::Policies, false),
    (Os::Windows, Tier::Hosts, false),
    (Os::Windows, Tier::Resolver, false),
    (Os::Windows, Tier::Policies, false),
    (Os::Linux, Tier::Hosts, false),
    (Os::Linux, Tier::Resolver, false),
    (Os::Linux, Tier::Policies, false),
];

/// Environment variable a tester sets to open tiers on their own machine
/// (`hosts,resolver,policies`). The app then passes `--unverified <tiers>` to the helper,
/// because elevation does not carry the environment.
pub const OVERRIDE_ENV: &str = "UBIQX_SHIELD_UNVERIFIED_TIERS";

/// Whether [`TIER_GATE`] opens `tier` on `os`.
pub fn tier_enabled(os: Os, tier: Tier) -> bool {
    todo!()
}

/// Parses the value of [`OVERRIDE_ENV`] or of `--unverified`: tier names separated by
/// commas, blanks ignored, unknown names dropped, no duplicates.
pub fn parse_override(value: Option<&str>) -> Vec<Tier> {
    todo!()
}

/// The gate as one process sees it: the table for its OS plus the tester's override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub os: Os,
    /// Tiers opened by the override; the page labels them "em teste".
    pub unverified: Vec<Tier>,
}

impl Gate {
    pub fn new(os: Os, unverified: Vec<Tier>) -> Self {
        todo!()
    }

    /// Open by the table or by the override.
    pub fn open(&self, tier: Tier) -> bool {
        todo!()
    }

    /// Open only because of the override.
    pub fn is_unverified(&self, tier: Tier) -> bool {
        todo!()
    }

    /// The value for `--unverified` (`hosts,resolver`), or `None` when nothing was
    /// overridden.
    pub fn unverified_arg(&self) -> Option<String> {
        todo!()
    }
}
