//! Whether the helper's state directory can be trusted (design section 2.5).
//!
//! On Unix the directory is created `root`, mode 0755; before use an `lstat` must show
//! uid 0, a directory, not a symlink, not group- or world-writable, otherwise it is removed
//! (never following a link) and recreated empty. On Windows it lives under
//! `<ProgramFiles>`; a reparse point or a non-directory at that path makes the helper touch
//! nothing and exit 4.
//!
//! Seeded by the foundation unit: complete types, signatures only.
#![allow(unused_variables)]

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

/// The decision for `meta` on `os`.
pub fn assess(meta: &DirMeta, os: Os) -> Trust {
    todo!()
}
