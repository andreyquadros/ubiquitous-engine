//! The hosts write: a splice into the file the path resolves to, only when it is root's,
//! with the in-place fallback for a mount point and a read-back that restores.

use std::io::ErrorKind;

use ubiqx_core::nofap::{Os, Tier, TierState};
use ubiqx_core::ProductId;
use ubiqx_nofap::hosts::{self, BlockInspection};
use ubiqx_nofap::plan::{EXIT_APPLIED, EXIT_PARTIAL};

use crate::sysblock::testing::{plan_of, Node, Rig, HOSTS_UNIX, SYSTEM32};

fn hosts_only(owner: ProductId) -> ubiqx_core::nofap::SystemPlan {
    let mut plan = plan_of(owner, &["blocked.example"]);
    plan.resolver = None;
    plan.policies = false;
    plan
}

#[test]
fn a_hosts_file_that_is_a_mount_point_is_written_in_place_with_the_same_bytes() {
    for kind in [ErrorKind::ResourceBusy, ErrorKind::CrossesDevices] {
        for os in Os::ALL {
            let plan = hosts_only(ProductId::Ubiqx);
            let renamed = Rig::new(os);
            renamed.apply(&plan);
            let rig = Rig::new(os);
            rig.fs.fail_rename(&rig.hosts_path(), kind);
            assert_eq!(rig.apply(&plan), EXIT_APPLIED, "{os:?} {kind:?}");
            assert_eq!(rig.hosts_text(), renamed.hosts_text(), "{os:?}");
            assert!(rig
                .fs
                .mutations()
                .contains(&format!("write-in-place {}", rig.hosts_path())));
            assert_eq!(rig.assess(&plan).hosts, TierState::Applied);
            // Removal goes the same way and gives back the original bytes.
            assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
            let fresh = Rig::new(os);
            assert_eq!(rig.hosts_text(), fresh.hosts_text(), "{os:?}");
        }
    }
}

/// What the smoke run on a real Windows machine showed: a hosts file replaced through a
/// rename is a new file, owned by whoever created it, so after the removal the bytes were
/// the user's and the owner was not. On Windows the file is therefore written in place,
/// by apply, by remove and by the restore after a write that did not read back; on Unix
/// the rename stays (the port gives the new file the old owner and mode).
#[test]
fn windows_writes_the_hosts_file_in_place_and_unix_renames_over_it() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let path = rig.hosts_path();
        let (renamed, in_place) = (format!("write {path}"), format!("write-in-place {path}"));
        let hosts_writes = |rig: &Rig| -> Vec<String> {
            let log = rig.fs.mutations();
            log.into_iter()
                .filter(|line| *line == renamed || *line == in_place)
                .collect()
        };
        let expected = if os == Os::Windows {
            in_place.as_str()
        } else {
            renamed.as_str()
        };
        let plan = hosts_only(ProductId::Ubiqx);
        assert_eq!(rig.apply(&plan), EXIT_APPLIED, "{os:?}");
        assert_eq!(hosts_writes(&rig), [expected], "{os:?}");
        rig.fs.clear_log();
        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "{os:?}");
        assert_eq!(hosts_writes(&rig), [expected], "{os:?}");
        assert_eq!(rig.hosts_text(), Rig::new(os).hosts_text(), "{os:?}");

        // A write that does not read back: the bytes are put back the same way.
        rig.fs.clear_log();
        rig.fs.tamper_next_write(&path);
        assert_eq!(rig.apply(&plan), EXIT_PARTIAL, "{os:?}");
        assert_eq!(hosts_writes(&rig), [expected, expected], "{os:?}");
        assert_eq!(rig.hosts_text(), Rig::new(os).hosts_text(), "{os:?}");
    }
}

#[test]
fn a_write_that_does_not_read_back_is_undone_and_reported() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let before = rig.hosts_text();
        rig.fs.tamper_next_write(&rig.hosts_path());
        let mut plan = hosts_only(ProductId::Ubiqx);
        plan.policies = true;
        assert_eq!(rig.apply(&plan), EXIT_PARTIAL, "{os:?}");
        assert_eq!(rig.hosts_text(), before, "the original bytes are back");
        let seen = rig.observe();
        let state = seen.state.clone().expect("the report is in the state file");
        let hosts_report: Vec<_> = state
            .report
            .iter()
            .filter(|r| r.tier == Tier::Hosts)
            .collect();
        assert_eq!(hosts_report.len(), 1);
        assert!(!hosts_report[0].ok);
        assert_eq!(hosts_report[0].reason.as_deref(), Some("write_refused"));
        // The other tier of the plan went through.
        assert!(state
            .report
            .iter()
            .any(|r| r.tier == Tier::Policies && r.ok));
        assert!(
            !state.owners[&ProductId::Ubiqx].hosts,
            "a first try that failed is not recorded"
        );
        assert!(state.owners[&ProductId::Ubiqx].policies);
        // What the app then shows.
        assert_eq!(seen.hosts.error.as_deref(), Some("write_refused"));
        assert_eq!(
            rig.assess(&plan).hosts,
            TierState::Unsupported {
                reason: "write_refused".into()
            }
        );
        // The next try, with nothing interfering, succeeds and clears the report.
        assert_eq!(rig.apply(&plan), EXIT_APPLIED);
        assert_eq!(rig.observe().hosts.error, None);
        assert_eq!(rig.assess(&plan).hosts, TierState::Applied);
    }
}

#[test]
fn a_hosts_path_that_leads_to_a_file_root_does_not_own_is_unsupported() {
    for os in [Os::Linux, Os::Macos] {
        let rig = Rig::new(os);
        rig.fs.dir_owned("/home/ana", 1000, 0o755);
        rig.fs
            .file_owned("/home/ana/hosts", HOSTS_UNIX.as_bytes(), 1000, 0o644);
        rig.fs.link("/etc/hosts", "/home/ana/hosts");
        let plan = hosts_only(ProductId::Ubiqx);
        assert_eq!(rig.observe().hosts.error.as_deref(), Some("write_refused"));
        assert_eq!(
            rig.assess(&plan).hosts,
            TierState::Unsupported {
                reason: "write_refused".into()
            },
            "{os:?}"
        );
        rig.fs.clear_log();
        assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
        let touched: Vec<String> = rig
            .fs
            .mutations()
            .into_iter()
            .filter(|change| !change.contains("ubiqx-shield"))
            .collect();
        assert_eq!(
            touched,
            Vec::<String>::new(),
            "only the state file was written"
        );
        assert_eq!(rig.fs.text("/home/ana/hosts").as_deref(), Some(HOSTS_UNIX));
    }
}

#[test]
fn only_a_file_and_a_directory_that_are_roots_are_written() {
    let plan = hosts_only(ProductId::Ubiqx);
    // The file is root's but its directory is somebody else's, or open to others.
    for (uid, mode) in [(1000, 0o755), (0, 0o777), (0, 0o775)] {
        let rig = Rig::new(Os::Linux);
        rig.fs.dir_owned("/etc", uid, mode);
        assert_eq!(
            rig.observe().hosts.error.as_deref(),
            Some("write_refused"),
            "{uid} {mode:o}"
        );
        assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
        assert_eq!(rig.hosts_text(), HOSTS_UNIX);
    }
    // A link that leads nowhere, and a directory where the file should be.
    let rig = Rig::new(Os::Linux);
    rig.fs.link("/etc/hosts", "/nix/store/gone");
    assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
    // Windows: a reparse point is not followed.
    let rig = Rig::new(Os::Windows);
    let hosts_path = rig.hosts_path();
    rig.fs.file("C:\\Users\\Public\\hosts", b"");
    rig.fs.link(&hosts_path, "C:\\Users\\Public\\hosts");
    assert_eq!(rig.observe().hosts.error.as_deref(), Some("write_refused"));
    assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
    assert_eq!(rig.fs.text("C:\\Users\\Public\\hosts").as_deref(), Some(""));
}

#[test]
fn a_file_system_that_refuses_the_write_is_reported_and_nothing_is_lost() {
    for kind in [ErrorKind::ReadOnlyFilesystem, ErrorKind::PermissionDenied] {
        let rig = Rig::new(Os::Linux);
        rig.fs.fail_rename("/etc/hosts", kind);
        assert_eq!(
            rig.apply(&hosts_only(ProductId::Ubiqx)),
            EXIT_PARTIAL,
            "{kind:?}"
        );
        assert_eq!(rig.hosts_text(), HOSTS_UNIX);
        let state = rig.observe().state.unwrap();
        assert_eq!(state.report[0].reason.as_deref(), Some("write_refused"));
    }
}

#[test]
fn a_missing_hosts_file_is_created_with_the_block() {
    let rig = Rig::new(Os::Windows);
    let hosts_path = format!("{SYSTEM32}\\drivers\\etc\\hosts");
    use crate::sysblock::Fs;
    rig.fs
        .remove_file(std::path::Path::new(&hosts_path))
        .unwrap();
    assert_eq!(rig.observe().hosts.text.as_deref(), Some(""));
    let plan = hosts_only(ProductId::Shield);
    assert_eq!(rig.assess(&plan).hosts, TierState::NotApplied);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    let text = rig.hosts_text();
    assert!(text.starts_with(hosts::BEGIN_PREFIX));
    assert!(matches!(
        hosts::inspect(text.as_bytes()),
        BlockInspection::Intact { .. }
    ));
    assert_eq!(
        rig.fs
            .node(&hosts_path)
            .map(|node| matches!(node, Node::File { .. })),
        Some(true)
    );
    assert_eq!(rig.remove("shield", "all"), EXIT_APPLIED);
    assert_eq!(
        rig.hosts_text(),
        "",
        "the block is gone; an empty file stays"
    );
}

#[test]
fn the_rest_of_the_file_keeps_its_bytes_owner_and_mode() {
    let rig = Rig::new(Os::Linux);
    // A byte-order mark, mixed endings, a line without an end, bytes that are not UTF-8.
    let odd: &[u8] =
        b"\xEF\xBB\xBF127.0.0.1 localhost\r\n10.0.0.5\tprinter.lan # caf\xE9\n192.168.0.9 nas";
    rig.fs.file_owned("/etc/hosts", odd, 0, 0o664);
    let plan = hosts_only(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    let Some(Node::File { bytes, uid, mode }) = rig.fs.node("/etc/hosts") else {
        panic!("the hosts file is a file")
    };
    assert!(bytes.starts_with(odd));
    assert_eq!((uid, mode), (0, 0o664), "owner and mode are kept");
    assert_eq!(rig.assess(&plan).hosts, TierState::Applied);
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(
        rig.fs.node("/etc/hosts"),
        Some(Node::File {
            bytes: odd.to_vec(),
            uid: 0,
            mode: 0o664
        })
    );
}

#[test]
fn a_hosts_file_that_cannot_be_read_is_reported_unreadable() {
    let rig = Rig::new(Os::Linux);
    // A directory where the file belongs: reading it fails with something other than
    // "not found".
    rig.fs.dir("/etc/hosts");
    let seen = rig.observe();
    assert_eq!(seen.hosts.text, None);
    assert_eq!(seen.hosts.error.as_deref(), Some("unreadable"));
    let plan = hosts_only(ProductId::Ubiqx);
    assert_eq!(
        rig.assess(&plan).hosts,
        TierState::Unsupported {
            reason: "unreadable".into()
        }
    );
    assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
    assert_eq!(
        rig.observe().state.unwrap().report[0].reason.as_deref(),
        Some("unreadable")
    );
}
