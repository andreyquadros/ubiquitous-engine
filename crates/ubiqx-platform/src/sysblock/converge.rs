//! Brings the machine to what the owners of the state file want together: the one
//! procedure behind `apply` and `remove`.
//!
//! A tier somebody wants is written when the gate is open for it; a tier nobody wants is
//! taken off whatever the gate says, because the way out is never closed. The hosts block
//! is also rewritten when it is already there, so a removal can take one owner's names out
//! of a block the other owner keeps.

use std::collections::BTreeMap;
use std::time::Duration;

use ubiqx_core::nofap::{
    reason, Os, OwnerWants, PolicyCreated, ResolverId, ResolverObservation, SystemState, Tier,
    TierReport, CONFIG_VERSION,
};
use ubiqx_core::ProductId;
use ubiqx_nofap::gate::Gate;
use ubiqx_nofap::plan::{self, EXIT_APPLIED, EXIT_FAILED, EXIT_PARTIAL};
use ubiqx_nofap::resolvers::{self, PROBE_NAME, PROBE_WAIT_SECS};
use ubiqx_nofap::{hosts, list, matcher};

use super::observe::{block_present, observe};
use super::{hostsfile, state, Fs, NameLookup, OsBackend, RunLock, Runner};

/// Everything a helper run works through.
pub struct Ports<'a> {
    pub backend: &'a dyn OsBackend,
    pub runner: &'a dyn Runner,
    pub fs: &'a dyn Fs,
    pub names: &'a dyn NameLookup,
    /// Lets one helper run at a time change the machine.
    pub lock: &'a dyn RunLock,
    /// Waits between two tries of the probe (a real sleep outside tests).
    pub pause: &'a dyn Fn(Duration),
    /// Unix milliseconds, for `applied_at`.
    pub now_ms: i64,
}

fn report(tier: Tier, outcome: Result<(), String>) -> TierReport {
    TierReport {
        tier,
        ok: outcome.is_ok(),
        reason: outcome.err(),
    }
}

/// Whether an owner still asks for anything. One that does not is not an owner: the state
/// file lists only products that want a tier.
fn wants_something(wants: &OwnerWants) -> bool {
    wants.hosts || wants.resolver.is_some() || wants.policies
}

/// Whether DNS already is the resolver `id` everywhere it can be.
fn resolver_in_place(observed: &ResolverObservation, id: ResolverId, os: Os) -> bool {
    if os == Os::Linux {
        return observed.dropin.as_deref() == Some(resolvers::resolved_dropin(id).as_str());
    }
    let mut active = observed.interfaces.iter().filter(|i| i.active).peekable();
    active.peek().is_some() && active.all(|i| resolvers::is_exactly(id, &i.servers))
}

impl Ports<'_> {
    fn probe(&self) -> bool {
        !self.names.lookup(PROBE_NAME).is_empty()
    }

    /// Tries the probe for up to `PROBE_WAIT_SECS` after a DNS change.
    fn probe_recovers(&self) -> bool {
        for attempt in 0..PROBE_WAIT_SECS {
            if self.probe() {
                return true;
            }
            if attempt + 1 < PROBE_WAIT_SECS {
                (self.pause)(Duration::from_secs(1));
            }
        }
        false
    }

    /// Sets the resolver without leaving the machine without names: when the probe
    /// resolved before the change and does not after it, the change is undone in this
    /// same run and the tier is reported `unreachable`.
    ///
    /// `first_time`: no owner had the resolver before this run. A change that then fails
    /// half-way (one network service refuses it, the service does not restart) is undone
    /// too, so a tier reported as failed leaves no DNS setting behind that nobody has
    /// recorded. Where an owner already had it, what is in place stays.
    fn set_resolver(
        &self,
        id: ResolverId,
        observed: &ResolverObservation,
        first_time: bool,
    ) -> Result<(), String> {
        let worked_before = self.probe();
        if let Err(code) = self
            .backend
            .apply_resolver(id, observed, self.runner, self.fs)
        {
            if first_time {
                self.undo_resolver(Some((id, observed)));
            }
            return Err(code);
        }
        if !worked_before || self.probe_recovers() {
            return Ok(());
        }
        self.undo_resolver(None);
        Err(reason::UNREACHABLE.to_owned())
    }

    /// Takes the resolver off again inside the run that set it. `kept`: the resolver and
    /// the look taken before the change; an interface that already had exactly that set
    /// then was not set by this run and keeps it.
    fn undo_resolver(&self, kept: Option<(ResolverId, &ResolverObservation)>) {
        let mut fresh = observe(self.backend, self.runner, self.fs, true);
        if let Some((id, before)) = kept {
            fresh.resolver.interfaces.retain(|interface| {
                !before.interfaces.iter().any(|earlier| {
                    earlier.id == interface.id && resolvers::is_exactly(id, &earlier.servers)
                })
            });
        }
        // Whatever the undo says, the tier is not applied.
        let _ = self.backend.remove_resolver(&fresh, self.runner, self.fs);
    }

    /// Writes the state as it stands; a failure is the helper's `EXIT_FAILED`.
    fn record(&self, state: &SystemState) -> Result<(), i32> {
        state::write(self.backend, self.fs, state).map_err(|_| EXIT_FAILED)
    }
}

/// Makes the machine match `owners` and records it. `previous` is the state file as it
/// was (read from a trusted directory); `acting` is the owner of the plan being applied,
/// or `None` for a removal. Returns the helper's exit code.
pub(crate) fn converge(
    ports: &Ports<'_>,
    mut owners: BTreeMap<ProductId, OwnerWants>,
    previous: Option<&SystemState>,
    acting: Option<ProductId>,
    gate: &Gate,
) -> i32 {
    let (backend, runner, fs) = (ports.backend, ports.runner, ports.fs);
    let os = backend.os();
    owners.retain(|_, wants| wants_something(wants));
    let before = previous.cloned().unwrap_or_default();
    let mut state = SystemState {
        v: CONFIG_VERSION,
        owners: owners.clone(),
        report: Vec::new(),
        ..before.clone()
    };
    // The wants are recorded before anything is changed: a helper that cannot write its
    // state (not root, a refused directory) stops here with the machine untouched.
    if !owners.is_empty() {
        if let Err(code) = ports.record(&state) {
            return code;
        }
    }
    let wanted = plan::union(&owners);
    let observed = observe(backend, runner, fs, true);
    let mut reports = Vec::new();
    let mut flush = false;
    // What the acting owner wanted before this run: a tier that fails stays recorded for
    // it only when it was (the earlier apply may still be in place).
    let had = acting.and_then(|owner| before.owners.get(&owner).cloned());
    let had = had.unwrap_or_default();

    // L2: the hosts block.
    let present = block_present(observed.hosts.text.as_deref());
    if wanted.hosts && (gate.open(Tier::Hosts) || present) {
        let names = plan::hosts_names(&owners, list::hosts_tier(), matcher::never_block(), os);
        let written = hostsfile::write(backend, fs, Some(&hosts::render_block(&names)));
        flush |= written == Ok(true);
        match written {
            Ok(_) => state.hosts = Some(hosts::stamp_of(&names)),
            Err(_) if !had.hosts => {
                if let Some(wants) = acting.and_then(|owner| owners.get_mut(&owner)) {
                    wants.hosts = false;
                }
            }
            Err(_) => {}
        }
        reports.push(report(
            Tier::Hosts,
            written.map(|_| ()).map_err(str::to_owned),
        ));
    } else if !wanted.hosts {
        let removed = hostsfile::write(backend, fs, None);
        flush |= removed == Ok(true);
        if removed != Ok(false) || before.hosts.is_some() {
            reports.push(report(
                Tier::Hosts,
                removed.map(|_| ()).map_err(str::to_owned),
            ));
        }
        if removed.is_ok() {
            state.hosts = None;
        }
    }

    // L3: the resolver.
    match wanted.resolver {
        Some(id) if gate.open(Tier::Resolver) => {
            if !resolver_in_place(&observed.resolver, id, os) {
                // DNS that could not be read is not changed blindly.
                let first_time = before.owners.values().all(|wants| wants.resolver.is_none());
                let set = match &observed.resolver.error {
                    Some(code) => Err(code.clone()),
                    None => ports.set_resolver(id, &observed.resolver, first_time),
                };
                flush = true;
                if let Err(code) = &set {
                    // Undone because it broke name resolution: nobody has it any more.
                    let everyone = code == reason::UNREACHABLE;
                    for (owner, wants) in owners.iter_mut() {
                        let failed_first_try = Some(*owner) == acting && had.resolver.is_none();
                        if everyone || failed_first_try {
                            wants.resolver = None;
                        }
                    }
                }
                reports.push(report(Tier::Resolver, set));
            } else {
                reports.push(report(Tier::Resolver, Ok(())));
            }
        }
        Some(_) => {}
        None => {
            let undone = backend.remove_resolver(&observed, runner, fs);
            flush |= undone == Ok(true);
            if undone != Ok(false) {
                reports.push(report(Tier::Resolver, undone.map(|_| ())));
            }
        }
    }

    // L4: browser policies.
    if wanted.policies && gate.open(Tier::Policies) {
        // The values about to be created are recorded before the first one is written: a
        // run that fails or is killed between two of them leaves none `remove` cannot find.
        let mut note = |created: &[PolicyCreated]| {
            state.policies_created = created.to_vec();
            ports.record(&state).map_err(|_| reason::FAILED.to_owned())
        };
        let applied = backend.apply_policies(
            &observed.policies,
            &before.policies_created,
            runner,
            fs,
            &mut note,
        );
        if let Ok(created) = &applied {
            state.policies_created.clone_from(created);
        } else if !had.policies {
            if let Some(wants) = acting.and_then(|owner| owners.get_mut(&owner)) {
                wants.policies = false;
            }
        }
        reports.push(report(Tier::Policies, applied.map(|_| ())));
    } else if !wanted.policies {
        let removed =
            backend.remove_policies(&observed.policies, &before.policies_created, runner, fs);
        if removed != Ok(false) || !before.policies_created.is_empty() {
            reports.push(report(Tier::Policies, removed.clone().map(|_| ())));
        }
        if removed.is_ok() {
            state.policies_created.clear();
        }
    }

    if flush {
        backend.flush_dns(runner);
    }

    let all_ok = reports.iter().all(|report| report.ok);
    owners.retain(|_, wants| wants_something(wants));
    state.owners = owners;
    state.applied_at = ports.now_ms;
    state.report = reports;
    let recorded = if state.owners.is_empty() && all_ok {
        // Nobody wants anything and everything came off: no state is left behind.
        state::clear(backend, fs).map_err(|_| EXIT_FAILED)
    } else {
        ports.record(&state)
    };
    match recorded {
        Err(code) => code,
        Ok(()) if all_ok => EXIT_APPLIED,
        Ok(()) => EXIT_PARTIAL,
    }
}
