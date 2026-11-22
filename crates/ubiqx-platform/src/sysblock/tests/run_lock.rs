//! The lock between helper runs: two runs that overlap (the two products, an uninstall
//! hook during an apply) must not lose what the other one recorded.

use std::time::Duration;

use ubiqx_core::nofap::Os;
use ubiqx_core::ProductId;
use ubiqx_nofap::plan::{EXIT_APPLIED, EXIT_FAILED};

use crate::sysblock::helper::LOCK_WAIT_SECS;
use crate::sysblock::sys;
use crate::sysblock::testing::{plan_of, Rig};

#[test]
fn a_run_that_starts_meanwhile_does_not_lose_what_the_other_one_records() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let nested = std::sync::Arc::new(parking_lot::Mutex::new(None));
        // While NoFap's apply writes the hosts file, ubiqX's apply starts.
        let (other, seen) = (rig.clone(), nested.clone());
        rig.fs.meanwhile("hosts", move || {
            let code = other.apply(&plan_of(ProductId::Ubiqx, &["second.example"]));
            *seen.lock() = Some(code);
        });
        assert_eq!(rig.apply(&plan_of(ProductId::Shield, &[])), EXIT_APPLIED);
        let nested = nested.lock().expect("the second run started");
        let state = rig.observe().state.expect("a state file");
        // Whatever a run reports as applied is still recorded once both have ended.
        if nested == EXIT_APPLIED {
            assert!(
                state.owners.contains_key(&ProductId::Ubiqx),
                "{os:?}: the second run's owner was lost"
            );
        }
        // It waited for the first one and gave up, leaving everything to it.
        assert_eq!(nested, EXIT_FAILED, "{os:?}");
        assert_eq!(
            state.owners.keys().copied().collect::<Vec<_>>(),
            [ProductId::Shield],
            "{os:?}"
        );
        assert!(!rig.lock.is_held(), "{os:?}: released after the run");
        // Asked again once the first run is over, it goes through.
        assert_eq!(
            rig.apply(&plan_of(ProductId::Ubiqx, &["second.example"])),
            EXIT_APPLIED
        );
        let state = rig.observe().state.unwrap();
        assert_eq!(state.owners.len(), 2, "{os:?}");
    }
}

#[test]
fn an_apply_waits_then_gives_up_and_a_removal_goes_on() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        assert_eq!(rig.apply(&plan_of(ProductId::Shield, &[])), EXIT_APPLIED);
        let held = rig.lock.hold();
        rig.fs.clear_log();
        rig.clear_calls();
        rig.pauses.lock().clear();
        assert_eq!(
            rig.apply(&plan_of(ProductId::Ubiqx, &[])),
            EXIT_FAILED,
            "{os:?}"
        );
        assert_eq!(rig.fs.mutations(), Vec::<String>::new(), "{os:?}");
        assert!(rig.calls().is_empty(), "{os:?}: nothing was run");
        let waited = usize::try_from(LOCK_WAIT_SECS - 1).unwrap();
        assert_eq!(
            *rig.pauses.lock(),
            vec![Duration::from_secs(1); waited],
            "{os:?}"
        );
        // The way out is never closed: a removal goes on once the wait is over.
        let (code, _, err) = rig.helper(&["remove", "--owner", "shield", "--scope", "all"]);
        assert_eq!(code, EXIT_APPLIED, "{os:?}");
        assert!(err.contains("removing anyway"), "{os:?}: {err}");
        assert!(rig.observe().state.is_none(), "{os:?}");
        drop(held);
    }
}

#[test]
fn a_machine_where_the_lock_cannot_be_taken_still_runs() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        *rig.lock.broken.lock() = true;
        assert_eq!(rig.apply(&plan_of(ProductId::Ubiqx, &[])), EXIT_APPLIED);
        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "{os:?}");
        assert!(rig.pauses.lock().is_empty(), "{os:?}: no wait");
    }
}

#[test]
fn only_apply_and_remove_take_the_lock() {
    let rig = Rig::new(Os::Linux);
    let _held = rig.lock.hold();
    let (code, _, _) = rig.helper(&["status"]);
    assert_eq!(code, EXIT_APPLIED);
    assert_eq!(*rig.lock.tries.lock(), 0);
}

#[test]
fn the_real_lock_lets_one_holder_in_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("ubiqx-shield");
    let first = sys::lock_beside(&state_dir).unwrap();
    assert!(first.is_some());
    // From another thread, as another process would see it (a Windows mutex is
    // re-entrant for the thread that owns it).
    let other = |state_dir: std::path::PathBuf| {
        std::thread::spawn(move || sys::lock_beside(&state_dir).unwrap().is_some())
            .join()
            .unwrap()
    };
    assert!(!other(state_dir.clone()), "a second holder got in");
    drop(first);
    assert!(other(state_dir.clone()), "the lock was not released");
    // Nothing was left on disk.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
