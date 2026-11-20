//! The real ports, against a temporary directory and harmless commands only. Whatever is
//! Unix-only is checked when the port itself reports Unix metadata, so this file needs no
//! conditional compilation.

use std::io::ErrorKind;
use std::path::Path;

use crate::sysblock::sys::{self, SystemFs, SystemLookup, SystemRunner};
use crate::sysblock::{Fs, NameLookup, Runner};

fn names(dir: &Path) -> Vec<String> {
    SystemFs.list(dir).expect("a listing")
}

#[test]
fn real_fs_writes_through_a_temporary_file_and_leaves_none_behind() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("hosts");
    SystemFs
        .write_atomic(&file, b"first\n", Some(0o644))
        .unwrap();
    assert_eq!(SystemFs.read(&file).unwrap(), b"first\n");
    assert_eq!(names(dir.path()), ["hosts"]);
    let meta = SystemFs.dir_meta(&file).unwrap();
    assert!(meta.exists && !meta.is_dir && !meta.is_link);
    if let Some(mode) = meta.mode {
        assert_eq!(mode, 0o644, "the mode asked for, whatever the umask");
    }
    // Replacing keeps the bytes whole and nothing else in the directory.
    SystemFs
        .write_atomic(&file, b"second, longer\n", None)
        .unwrap();
    assert_eq!(SystemFs.read(&file).unwrap(), b"second, longer\n");
    assert_eq!(names(dir.path()), ["hosts"]);
    assert!(SystemFs.exists(&file));
}

#[test]
fn real_fs_keeps_the_owner_and_mode_of_a_file_it_replaces() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state.json");
    SystemFs.write_atomic(&file, b"{}", Some(0o600)).unwrap();
    let before = SystemFs.dir_meta(&file).unwrap();
    // The mode argument is for a new file only.
    SystemFs
        .write_atomic(&file, b"{\"v\":1}", Some(0o644))
        .unwrap();
    let after = SystemFs.dir_meta(&file).unwrap();
    assert_eq!(after.uid, before.uid);
    assert_eq!(after.mode, before.mode);
    if let Some(mode) = after.mode {
        assert_eq!(mode, 0o600);
    }
    // In place: the same file, new bytes, nothing left of longer old content.
    SystemFs.write_in_place(&file, b"x").unwrap();
    assert_eq!(SystemFs.read(&file).unwrap(), b"x");
    SystemFs.write_in_place(&file, b"longer again\n").unwrap();
    assert_eq!(SystemFs.read(&file).unwrap(), b"longer again\n");
    assert_eq!(SystemFs.dir_meta(&file).unwrap().mode, before.mode);
    let missing = dir.path().join("missing");
    assert_eq!(
        SystemFs.write_in_place(&missing, b"x").unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert!(!SystemFs.exists(&missing), "in place never creates");
}

#[test]
fn real_fs_fails_cleanly_when_the_target_cannot_be_replaced() {
    let dir = tempfile::tempdir().unwrap();
    // A directory where the file belongs: the rename fails and the temporary file goes.
    let occupied = dir.path().join("occupied");
    SystemFs.create_dir(&occupied, Some(0o755)).unwrap();
    assert!(SystemFs.write_atomic(&occupied, b"x", None).is_err());
    assert_eq!(names(dir.path()), ["occupied"]);
    // No directory to write into.
    let nowhere = dir.path().join("no-such-dir").join("file");
    assert!(SystemFs.write_atomic(&nowhere, b"x", None).is_err());
    assert_eq!(names(dir.path()), ["occupied"]);
}

#[test]
fn real_fs_reports_what_is_at_a_path_without_following_links() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("ubiqx-shield");
    assert_eq!(SystemFs.dir_meta(&state).unwrap(), Default::default());
    assert_eq!(
        SystemFs.read(&state).unwrap_err().kind(),
        ErrorKind::NotFound
    );
    SystemFs.create_dir(&state, Some(0o755)).unwrap();
    let meta = SystemFs.dir_meta(&state).unwrap();
    assert!(meta.exists && meta.is_dir && !meta.is_link);
    if let Some(mode) = meta.mode {
        assert_eq!(mode, 0o755);
    }
    assert_eq!(
        SystemFs.create_dir(&state, None).unwrap_err().kind(),
        ErrorKind::AlreadyExists
    );

    // A link to a directory elsewhere. Windows tells links to files and to directories
    // apart and wants a privilege for either; its reparse points are covered by the
    // tests of the fake file system.
    if std::env::consts::OS == "windows" {
        return;
    }
    let outside = dir.path().join("outside");
    SystemFs.create_dir(&outside, None).unwrap();
    SystemFs
        .write_atomic(&outside.join("keep.txt"), b"kept", None)
        .unwrap();
    let link = state.join("link");
    // The one portable way to make a link; its deprecation is about the name only.
    #[allow(deprecated)]
    std::fs::soft_link(&outside, &link).unwrap();
    let meta = SystemFs.dir_meta(&link).unwrap();
    assert!(meta.exists && meta.is_link && !meta.is_dir);
    assert_eq!(
        SystemFs.resolve(&link).unwrap(),
        std::fs::canonicalize(&outside).unwrap()
    );
    assert_eq!(names(&state), ["link"]);
    // Deleting the tree deletes the link, not what it points at.
    SystemFs.remove_dir_all(&state).unwrap();
    assert!(!SystemFs.exists(&state));
    assert_eq!(SystemFs.read(&outside.join("keep.txt")).unwrap(), b"kept");
    // Unlinking a link is the same promise for a single path.
    #[allow(deprecated)]
    std::fs::soft_link(&outside, &state).unwrap();
    SystemFs.remove_file(&state).unwrap();
    assert_eq!(names(&outside), ["keep.txt"]);
}

#[test]
fn real_runner_gives_the_child_exactly_the_environment_it_is_handed() {
    // `/bin/sh` is the harmless tool every Unix has; a Windows host has no such path.
    if std::env::consts::OS == "windows" {
        return;
    }
    let script =
        "echo \"${SHIELD_TEST:-unset}|${HOME:-unset}|${PATH:-unset}\"; echo oops >&2; exit 3";
    let args = ["-c".to_owned(), script.to_owned()];
    let env = [("SHIELD_TEST".to_owned(), "handed in".to_owned())];
    let output = SystemRunner.run("/bin/sh", &args, &env).unwrap();
    assert_eq!(output.status, 3);
    assert!(!output.success());
    // HOME is set in this process and is not inherited. (A shell supplies a default
    // PATH of its own, so only "not ours" can be said about it.)
    let fields: Vec<&str> = output.stdout.trim_end().split('|').collect();
    assert_eq!(fields[..2], ["handed in", "unset"]);
    assert_ne!(Some(fields[2]), std::env::var("PATH").ok().as_deref());
    assert_eq!(output.stderr, "oops\n");
    let ok = SystemRunner.run("/bin/sh", &["-c".to_owned(), "exit 0".to_owned()], &[]);
    assert!(ok.unwrap().success());
    let missing = SystemRunner.run("/no/such/tool", &[], &[]);
    assert_eq!(missing.unwrap_err().kind(), ErrorKind::NotFound);
}

#[test]
fn real_lookup_answers_addresses_as_text() {
    // An address literal is answered by the resolver library itself: no query leaves.
    assert_eq!(SystemLookup.lookup("192.0.2.7"), ["192.0.2.7"]);
    assert_eq!(SystemLookup.lookup("2001:db8::7"), ["2001:db8::7"]);
    // What a blocked name resolves to, as the canary check reads it.
    assert_eq!(SystemLookup.lookup("0.0.0.0"), ["0.0.0.0"]);
    assert_eq!(SystemLookup.lookup(""), Vec::<String>::new());
    // Asked as a name that is expected to be blocked, an answer is the same answer.
    assert_eq!(SystemLookup.lookup_blocked("0.0.0.0"), ["0.0.0.0"]);
    assert_eq!(SystemLookup.lookup_blocked("192.0.2.7"), ["192.0.2.7"]);
    assert_eq!(SystemLookup.lookup_blocked(""), Vec::<String>::new());
}

/// What the smoke run on a real Windows machine showed: `getaddrinfo` never hands out the
/// null address there. A name that resolves to nothing else fails with "valid name, no
/// data" (11004), a name nobody knows with "host not found" (11001).
#[test]
fn windows_no_data_for_a_blocked_name_reads_as_the_null_address() {
    let failed = |code: i32| Err(std::io::Error::from_raw_os_error(code));
    let no_data = sys::WSANO_DATA;
    assert_eq!(no_data, 11004);
    let host_not_found = 11001;
    assert_eq!(sys::blocked_answer(failed(no_data), true), ["0.0.0.0"]);
    assert_eq!(
        sys::blocked_answer(failed(host_not_found), true),
        Vec::<String>::new()
    );
    // Only on Windows: the same number means something else to another system.
    assert_eq!(
        sys::blocked_answer(failed(no_data), false),
        Vec::<String>::new()
    );
    // An answer is the answer, whatever it is: a canary that resolves to a real address
    // is a hosts file that is not honoured.
    let real = vec!["203.0.113.7".to_owned()];
    assert_eq!(sys::blocked_answer(Ok(real.clone()), true), real);
    assert_eq!(sys::blocked_answer(Ok(real.clone()), false), real);
    assert_eq!(
        sys::blocked_answer(Ok(Vec::new()), true),
        Vec::<String>::new()
    );
    let other = std::io::Error::other("no number");
    assert_eq!(sys::blocked_answer(Err(other), true), Vec::<String>::new());
}

const INNER_LOOKUP_TEST: &str =
    "sysblock::tests::sys_ports::a_name_resolves_after_the_environment_is_pinned";
const FALSE_ROOT_MARK: &str = "UBIQX_SHIELD_TEST_FALSE_ROOT";

/// What the smoke run on a real Windows machine showed: a program started with `SystemRoot`
/// and `windir` pointing somewhere else resolves no name (Winsock finds its providers
/// through `%SystemRoot%`), and the helper is started through UAC with the variables of the
/// user's session. The variables are set for a child process only (this test binary,
/// running the one test below), so no other test ever sees them.
#[test]
fn the_helper_resolves_names_when_started_with_a_false_system_root() {
    let elsewhere = tempfile::tempdir().unwrap();
    let this_binary = std::env::current_exe().expect("the test binary");
    let mut child = std::process::Command::new(this_binary);
    child.args(["--exact", INNER_LOOKUP_TEST, "--ignored"]);
    for name in ["SystemRoot", "windir", FALSE_ROOT_MARK] {
        child.env(name, elsewhere.path());
    }
    let output = child.output().expect("the child test run");
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{printed}");
    assert!(
        printed.contains("1 passed"),
        "the inner test did not run: {printed}"
    );
}

#[test]
#[ignore = "run by the_helper_resolves_names_when_started_with_a_false_system_root, in a child process"]
fn a_name_resolves_after_the_environment_is_pinned() {
    let elsewhere = std::env::var(FALSE_ROOT_MARK).expect("started by the outer test");
    assert_eq!(
        std::env::var("SystemRoot").as_deref(),
        Ok(elsewhere.as_str())
    );
    // What `helper::run` does first.
    sys::pin_system_environment();
    match sys::windows_dirs() {
        // Windows: both variables now name the directory above `<System32>`.
        Some((system32, _)) => {
            for name in ["SystemRoot", "windir"] {
                let value = std::env::var(name).expect("set");
                assert_ne!(value, elsewhere, "{name}");
                assert!(
                    system32.to_lowercase().starts_with(&value.to_lowercase()),
                    "{name}={value}, <System32>={system32}"
                );
            }
        }
        // Elsewhere the variables mean nothing and are left alone.
        None => assert_eq!(
            std::env::var("SystemRoot").as_deref(),
            Ok(elsewhere.as_str())
        ),
    }
    // The machine's own name for itself needs no network: every system answers it.
    assert!(
        !SystemLookup.lookup("localhost").is_empty(),
        "no name resolves in this process"
    );
}
