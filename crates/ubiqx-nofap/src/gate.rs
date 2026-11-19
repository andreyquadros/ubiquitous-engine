//! The per-OS, per-tier gate (design section 1.8): a system tier is closed on an operating
//! system until that system's checks have passed. A closed tier is shown as `Gated` and the
//! helper refuses that section of a plan.
//!
//! Every entry lands `false`. An entry for `hosts` or `resolver` is flipped in the commit
//! that follows a green run of the CI job `nofap-system-smoke` on that OS (or, for
//! `(Linux, hosts)`, a green run of `scripts/nofap/system-smoke.sh --tiers hosts` in the
//! build sandbox), with the evidence recorded in `docs/nofap/REAL-MACHINE-CHECKLIST.md`.
//! An entry for `policies` is flipped only after the manual browser checks.

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

/// Whether [`TIER_GATE`] opens `tier` on `os`. A pair the table does not list is closed.
pub fn tier_enabled(os: Os, tier: Tier) -> bool {
    TIER_GATE
        .iter()
        .any(|(gate_os, gate_tier, open)| *gate_os == os && *gate_tier == tier && *open)
}

/// Parses the value of [`OVERRIDE_ENV`] or of `--unverified`: tier names separated by
/// commas, blanks and letter case ignored, unknown names dropped, no duplicates. The
/// result is in the order of `Tier::ALL`.
pub fn parse_override(value: Option<&str>) -> Vec<Tier> {
    let named: Vec<Tier> = value
        .unwrap_or_default()
        .split(',')
        .filter_map(|name| Tier::parse(name.trim().to_ascii_lowercase().as_str()))
        .collect();
    Tier::ALL
        .into_iter()
        .filter(|tier| named.contains(tier))
        .collect()
}

/// The gate as one process sees it: the table for its OS plus the tester's override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub os: Os,
    /// Tiers opened by the override; the page labels them "em teste".
    pub unverified: Vec<Tier>,
}

impl Gate {
    /// The gate of `os` with the tester's override. Only tiers the table keeps closed are
    /// kept in `unverified` (in the order of `Tier::ALL`, each once): a tier the table
    /// opens is verified, whatever the override says.
    pub fn new(os: Os, unverified: Vec<Tier>) -> Self {
        let unverified = Tier::ALL
            .into_iter()
            .filter(|tier| unverified.contains(tier) && !tier_enabled(os, *tier))
            .collect();
        Self { os, unverified }
    }

    /// Open by the table or by the override.
    pub fn open(&self, tier: Tier) -> bool {
        tier_enabled(self.os, tier) || self.unverified.contains(&tier)
    }

    /// Open only because of the override.
    pub fn is_unverified(&self, tier: Tier) -> bool {
        !tier_enabled(self.os, tier) && self.unverified.contains(&tier)
    }

    /// The value for `--unverified` (`hosts,resolver`), or `None` when nothing was
    /// overridden.
    pub fn unverified_arg(&self) -> Option<String> {
        let names: Vec<&str> = Tier::ALL
            .into_iter()
            .filter(|tier| self.is_unverified(*tier))
            .map(Tier::as_str)
            .collect();
        (!names.is_empty()).then(|| names.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_lists_every_pair_once() {
        for os in Os::ALL {
            for tier in Tier::ALL {
                let rows = TIER_GATE
                    .iter()
                    .filter(|(gate_os, gate_tier, _)| *gate_os == os && *gate_tier == tier)
                    .count();
                assert_eq!(rows, 1, "{os:?} {tier:?}");
            }
        }
        assert_eq!(TIER_GATE.len(), Os::ALL.len() * Tier::ALL.len());
    }

    /// The code lands with every entry closed. An entry is opened in the commit that
    /// follows that system's checks (see the module documentation), and that commit
    /// changes the expectation below for the entry it opens.
    #[test]
    fn every_tier_gate_entry_is_false() {
        for (os, tier, open) in TIER_GATE {
            assert!(!open, "{os:?} {tier:?} is open");
            assert!(!tier_enabled(os, tier));
        }
    }

    #[test]
    fn parse_override_reads_names_and_ignores_the_rest() {
        assert_eq!(parse_override(None), vec![]);
        assert_eq!(parse_override(Some("")), vec![]);
        assert_eq!(parse_override(Some(" , ,")), vec![]);
        assert_eq!(parse_override(Some("hosts")), vec![Tier::Hosts]);
        assert_eq!(
            parse_override(Some("hosts,resolver,policies")),
            vec![Tier::Hosts, Tier::Resolver, Tier::Policies]
        );
        assert_eq!(
            parse_override(Some(" policies , HOSTS,hosts,, vision ,resolver;x")),
            vec![Tier::Hosts, Tier::Policies]
        );
        assert_eq!(parse_override(Some("all")), vec![]);
        assert_eq!(parse_override(Some("hosts resolver")), vec![]);
    }

    #[test]
    fn a_gate_is_the_table_plus_the_override() {
        let closed = Gate::new(Os::Linux, vec![]);
        for tier in Tier::ALL {
            assert!(!closed.open(tier));
            assert!(!closed.is_unverified(tier));
        }
        assert_eq!(closed.unverified_arg(), None);

        let gate = Gate::new(
            Os::Windows,
            vec![Tier::Resolver, Tier::Hosts, Tier::Resolver],
        );
        assert_eq!(gate.unverified, vec![Tier::Hosts, Tier::Resolver]);
        assert!(gate.open(Tier::Hosts) && gate.open(Tier::Resolver));
        assert!(!gate.open(Tier::Policies));
        assert!(gate.is_unverified(Tier::Hosts));
        assert!(!gate.is_unverified(Tier::Policies));
        assert_eq!(gate.unverified_arg().as_deref(), Some("hosts,resolver"));
        // The argument round-trips through the parser the helper uses.
        assert_eq!(
            Gate::new(
                Os::Windows,
                parse_override(gate.unverified_arg().as_deref())
            ),
            gate
        );
    }
}
