//! The real ports, against a temporary directory and harmless commands only. Whatever is
//! Unix-only is checked when the port itself reports Unix metadata, so this file needs no
//! conditional compilation.

use std::io::ErrorKind;
use std::path::Path;

use crate::sysblock::sys::{SystemFs, SystemLookup, SystemRunner};
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
}
