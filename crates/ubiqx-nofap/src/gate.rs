//! The per-OS, per-tier gate (design section 1.8): a system tier is closed on an operating
//! system until that system's checks have passed. A closed tier is shown as `Gated` and the
//! helper refuses that section of a plan.
//!
//! An entry for `hosts` or `resolver` is opened in its own commit after a green run of the
//! smoke job on that OS (`.github/workflows/nofap-smoke.yml`, which runs
//! `scripts/nofap/system-smoke.sh` or `system-smoke.ps1` with the real helper as
//! administrator); `(Linux, hosts)` also has a green run of the same script in the build
//! sandbox. An entry for `policies` is opened only after the manual browser checks. The
//! evidence of every open entry (the run, its commit, its output) is recorded in section 1
//! of `docs/nofap/REAL-MACHINE-CHECKLIST.md`, and the test `OPEN` list below names the same
//! entries: a flip changes the table, that list and the checklist together.

use ubiqx_core::nofap::{Os, Tier};

/// Which tier is open on which system.
pub const TIER_GATE: [(Os, Tier, bool); 9] = [
    (Os::Macos, Tier::Hosts, true),
    (Os::Macos, Tier::Resolver, true),
    (Os::Macos, Tier::Policies, false),
    (Os::Windows, Tier::Hosts, true),
    (Os::Windows, Tier::Resolver, true),
    (Os::Windows, Tier::Policies, false),
    (Os::Linux, Tier::Hosts, true),
    (Os::Linux, Tier::Resolver, true),
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

    /// The entries that are open, each with its evidence in section 1 of
    /// `docs/nofap/REAL-MACHINE-CHECKLIST.md`. Opening an entry means adding it here in
    /// the commit that changes the table and records the run.
    const OPEN: [(Os, Tier); 6] = [
        (Os::Linux, Tier::Hosts),
        (Os::Linux, Tier::Resolver),
        (Os::Macos, Tier::Hosts),
        (Os::Macos, Tier::Resolver),
        (Os::Windows, Tier::Hosts),
        (Os::Windows, Tier::Resolver),
    ];

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

    #[test]
    fn the_open_entries_are_exactly_the_ones_with_recorded_evidence() {
        for (os, tier, open) in TIER_GATE {
            assert_eq!(open, OPEN.contains(&(os, tier)), "{os:?} {tier:?}");
            assert_eq!(tier_enabled(os, tier), open, "{os:?} {tier:?}");
        }
        for (index, entry) in OPEN.iter().enumerate() {
            assert!(!OPEN[..index].contains(entry), "{entry:?} is listed twice");
        }
    }

    /// The policies tier needs a person in front of each browser (the "managed" banner,
    /// `chrome://policy`, `about:policies`): no smoke run opens it.
    #[test]
    fn the_policies_tier_is_closed_on_every_system() {
        for os in Os::ALL {
            assert!(!tier_enabled(os, Tier::Policies), "{os:?}");
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
    fn a_gate_without_an_override_is_the_table() {
        for os in Os::ALL {
            let gate = Gate::new(os, vec![]);
            assert_eq!(gate.unverified, vec![], "{os:?}");
            assert_eq!(gate.unverified_arg(), None, "{os:?}");
            for tier in Tier::ALL {
                assert_eq!(gate.open(tier), tier_enabled(os, tier), "{os:?} {tier:?}");
                assert!(!gate.is_unverified(tier), "{os:?} {tier:?}");
            }
        }
    }

    #[test]
    fn an_override_opens_what_the_table_keeps_closed_and_labels_only_that() {
        for os in Os::ALL {
            let closed: Vec<Tier> = Tier::ALL
                .into_iter()
                .filter(|tier| !tier_enabled(os, *tier))
                .collect();
            // Out of order and with a repeat: the gate keeps the order of `Tier::ALL`.
            let gate = Gate::new(
                os,
                vec![Tier::Policies, Tier::Resolver, Tier::Hosts, Tier::Resolver],
            );
            assert_eq!(gate.unverified, closed, "{os:?}");
            for tier in Tier::ALL {
                assert!(gate.open(tier), "{os:?} {tier:?}");
                assert_eq!(
                    gate.is_unverified(tier),
                    !tier_enabled(os, tier),
                    "{os:?} {tier:?}"
                );
            }
            let names: Vec<&str> = closed.iter().map(|tier| tier.as_str()).collect();
            assert_eq!(gate.unverified_arg(), Some(names.join(",")), "{os:?}");
            // The argument round-trips through the parser the helper uses.
            assert_eq!(
                Gate::new(os, parse_override(gate.unverified_arg().as_deref())),
                gate
            );
        }
        // One tier named: only that one opens, the others stay as the table has them.
        let gate = Gate::new(Os::Windows, vec![Tier::Policies]);
        assert_eq!(gate.unverified, vec![Tier::Policies]);
        assert!(gate.open(Tier::Policies) && gate.is_unverified(Tier::Policies));
        assert_eq!(
            gate.open(Tier::Hosts),
            tier_enabled(Os::Windows, Tier::Hosts)
        );
        assert!(!gate.is_unverified(Tier::Hosts));
        assert_eq!(gate.unverified_arg().as_deref(), Some("policies"));
    }

    /// A tier the table opens is verified: an override that names it changes nothing, so
    /// the page never labels it "em teste" and the helper is not handed `--unverified`.
    #[test]
    fn an_override_cannot_relabel_a_tier_the_table_opens() {
        assert!(!OPEN.is_empty());
        for (os, tier) in OPEN {
            let gate = Gate::new(os, vec![tier]);
            assert_eq!(gate, Gate::new(os, vec![]), "{os:?} {tier:?}");
            assert!(gate.open(tier), "{os:?} {tier:?}");
            assert!(!gate.is_unverified(tier), "{os:?} {tier:?}");
            assert_eq!(gate.unverified_arg(), None, "{os:?} {tier:?}");
        }
    }
}
