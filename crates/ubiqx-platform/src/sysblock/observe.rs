//! The unprivileged look at the machine: hosts text, DNS, policy locations and the state
//! file, as raw text and lists. Judging it is `ubiqx_nofap::plan::assess`.

use ubiqx_core::nofap::{reason, SystemObservation, Tier};
use ubiqx_nofap::hosts::{self, BlockInspection};

use super::{hostsfile, state, Fs, OsBackend, Runner};

/// One look. `full` is `ObserveOpts::full`. The canary and the probe are added by the
/// caller when its options ask for them.
pub(crate) fn observe(
    backend: &dyn OsBackend,
    runner: &dyn Runner,
    fs: &dyn Fs,
    full: bool,
) -> SystemObservation {
    let (state, state_error) = state::read(backend, fs);
    let mut hosts = hostsfile::observe(backend, fs);
    // A write the helper was refused is not visible in the file itself: its last report
    // says so, for as long as the block is not there.
    if hosts.error.is_none() && !block_present(hosts.text.as_deref()) {
        hosts.error = state.as_ref().and_then(|state| {
            state
                .report
                .iter()
                .find(|report| report.tier == Tier::Hosts && !report.ok)
                .and_then(|report| report.reason.clone())
                .filter(|code| code == reason::WRITE_REFUSED)
        });
    }
    SystemObservation {
        hosts,
        resolver: backend.observe_resolver(runner, fs, full),
        policies: backend.observe_policies(runner, fs),
        state,
        state_error,
        canary: None,
        probe_ok: None,
    }
}

/// Whether the hosts text holds a block of ours, whole or damaged.
pub(crate) fn block_present(text: Option<&str>) -> bool {
    text.is_some_and(|text| hosts::inspect(text.as_bytes()) != BlockInspection::Absent)
}
