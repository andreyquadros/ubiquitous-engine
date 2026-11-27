//! Whether the helper's state directory can be trusted (design section 2.5).
//!
//! On Unix the directory is created `root`, mode 0755; before use an `lstat` must show
//! uid 0, a directory, not a symlink, not group- or world-writable, otherwise it is removed
//! (never following a link) and recreated empty. On Windows it lives under
//! `<ProgramFiles>`; a reparse point or a non-directory at that path makes the helper touch
//! nothing and exit 4.

use ubiqx_core::nofap::Os;

/// State directory per system. On Windows the path is relative to `<ProgramFiles>`
/// (`SHGetKnownFolderPath`), never to an environment variable.
pub const STATE_DIR_MACOS: &str = "/Library/Application Support/ubiqx-shield";
pub const STATE_DIR_LINUX: &str = "/var/lib/ubiqx-shield";
pub const STATE_DIR_WINDOWS_UNDER_PROGRAM_FILES: &str = "ubiqx-shield";
pub const STATE_FILE: &str = "state.json";
/// Mode of the directory and of the state file on Unix.
pub const STATE_DIR_MODE: u32 = 0o755;
pub const STATE_FILE_MODE: u32 = 0o644;

/// Permission bits that let somebody other than the owner write.
const WRITABLE_BY_OTHERS: u32 = 0o022;

/// What an `lstat` (or the Windows equivalent) says about the state path. Filled by the
/// platform's `Fs::dir_meta`; nothing here follows a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirMeta {
    pub exists: bool,
    pub is_dir: bool,
    /// A symbolic link (Unix) or a reparse point such as a junction (Windows).
    pub is_link: bool,
    /// Owner user id (Unix only).
    pub uid: Option<u32>,
    /// Permission bits (Unix only).
    pub mode: Option<u32>,
}

/// What the helper does with the state path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trust {
    /// Nothing there: create it.
    Absent,
    /// Root-owned, a real directory, not writable by others: use it.
    Trusted,
    /// Unix: remove it without following a link and recreate it empty.
    Reset,
    /// Windows: a reparse point or a non-directory. Touch nothing and exit 4.
    Refuse,
}

/// The decision for `meta` on `os`. On Unix anything that is not provably a root-owned
/// real directory closed to other users is reset; an owner or a mode that could not be
/// read counts as not proven.
pub fn assess(meta: &DirMeta, os: Os) -> Trust {
    if !meta.exists {
        return Trust::Absent;
    }
    match os {
        Os::Windows => {
            if meta.is_link || !meta.is_dir {
                Trust::Refuse
            } else {
                Trust::Trusted
            }
        }
        Os::Macos | Os::Linux => {
            let closed = meta.mode.is_some_and(|mode| mode & WRITABLE_BY_OTHERS == 0);
            if meta.is_dir && !meta.is_link && meta.uid == Some(0) && closed {
                Trust::Trusted
            } else {
                Trust::Reset
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_dir() -> DirMeta {
        DirMeta {
            exists: true,
            is_dir: true,
            is_link: false,
            uid: Some(0),
            mode: Some(0o755),
        }
    }

    #[test]
    fn a_missing_path_is_absent_everywhere() {
        for os in Os::ALL {
            assert_eq!(assess(&DirMeta::default(), os), Trust::Absent);
            // Whatever else the fields say, a path that does not exist is absent.
            let ghost = DirMeta {
                exists: false,
                ..root_dir()
            };
            assert_eq!(assess(&ghost, os), Trust::Absent);
        }
    }

    #[test]
    fn unix_trusts_only_a_root_directory_closed_to_others() {
        for os in [Os::Macos, Os::Linux] {
            assert_eq!(assess(&root_dir(), os), Trust::Trusted);
            for mode in [0o700, 0o750, 0o555, 0o755] {
                let meta = DirMeta {
                    mode: Some(mode),
                    ..root_dir()
                };
                assert_eq!(assess(&meta, os), Trust::Trusted, "{mode:o}");
            }
            let cases = [
                (
                    "a symlink",
                    DirMeta {
                        is_link: true,
                        ..root_dir()
                    },
                ),
                (
                    "a symlink to a file",
                    DirMeta {
                        is_link: true,
                        is_dir: false,
                        ..root_dir()
                    },
                ),
                (
                    "a file",
                    DirMeta {
                        is_dir: false,
                        ..root_dir()
                    },
                ),
                (
                    "owned by a user",
                    DirMeta {
                        uid: Some(1000),
                        ..root_dir()
                    },
                ),
                (
                    "owner unknown",
                    DirMeta {
                        uid: None,
                        ..root_dir()
                    },
                ),
                (
                    "group-writable",
                    DirMeta {
                        mode: Some(0o775),
                        ..root_dir()
                    },
                ),
                (
                    "world-writable",
                    DirMeta {
                        mode: Some(0o757),
                        ..root_dir()
                    },
                ),
                (
                    "sticky world-writable",
                    DirMeta {
                        mode: Some(0o1777),
                        ..root_dir()
                    },
                ),
                (
                    "mode unknown",
                    DirMeta {
                        mode: None,
                        ..root_dir()
                    },
                ),
            ];
            for (what, meta) in cases {
                assert_eq!(assess(&meta, os), Trust::Reset, "{what}");
            }
        }
    }

    #[test]
    fn windows_refuses_a_reparse_point_or_a_file_and_never_resets() {
        let dir = DirMeta {
            exists: true,
            is_dir: true,
            is_link: false,
            uid: None,
            mode: None,
        };
        assert_eq!(assess(&dir, Os::Windows), Trust::Trusted);
        let junction = DirMeta {
            is_link: true,
            ..dir
        };
        assert_eq!(assess(&junction, Os::Windows), Trust::Refuse);
        let file = DirMeta {
            is_dir: false,
            ..dir
        };
        assert_eq!(assess(&file, Os::Windows), Trust::Refuse);
        let link_to_file = DirMeta {
            is_dir: false,
            is_link: true,
            ..dir
        };
        assert_eq!(assess(&link_to_file, Os::Windows), Trust::Refuse);
    }

    #[test]
    fn the_paths_carry_the_machine_id() {
        for path in [
            STATE_DIR_MACOS,
            STATE_DIR_LINUX,
            STATE_DIR_WINDOWS_UNDER_PROGRAM_FILES,
        ] {
            assert!(path.ends_with(ubiqx_core::nofap::SYSTEM_ID), "{path}");
        }
        assert_eq!(STATE_DIR_MODE & WRITABLE_BY_OTHERS, 0);
        assert_eq!(STATE_FILE_MODE & WRITABLE_BY_OTHERS, 0);
    }
}
