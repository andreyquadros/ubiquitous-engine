//! The helper's state directory and state file.
//!
//! The file is written by the helper only, in a directory nobody else can write to, and
//! is world-readable so the app can judge what it sees. **Nothing read back from it
//! becomes an address, a path or a command**: the helper uses the owners' flags, a
//! resolver id from the compiled table, hostnames it validates again, and the list of
//! policy values it created, which it only ever compares with its own constants.

use std::io;
use std::path::PathBuf;

use ubiqx_core::nofap::{reason, SystemState, CONFIG_VERSION};
use ubiqx_nofap::state_dir::{self, Trust, STATE_DIR_MODE, STATE_FILE, STATE_FILE_MODE};

use super::{join, Fs, OsBackend};

/// A state file larger than this is not one the helper wrote, and is not parsed.
const MAX_STATE_BYTES: usize = 1 << 20;

/// The state file of `backend`.
pub(crate) fn file(backend: &dyn OsBackend) -> PathBuf {
    join(backend.os(), &backend.state_dir(), STATE_FILE)
}

fn parse(bytes: &[u8]) -> Option<SystemState> {
    if bytes.len() > MAX_STATE_BYTES {
        return None;
    }
    let state: SystemState = serde_json::from_slice(bytes).ok()?;
    (state.v == CONFIG_VERSION).then_some(state)
}

/// The unprivileged read: the state, or the code `unreadable` when a file is there and
/// cannot be read as one.
pub(crate) fn read(backend: &dyn OsBackend, fs: &dyn Fs) -> (Option<SystemState>, Option<String>) {
    match fs.read(&file(backend)) {
        Ok(bytes) => match parse(&bytes) {
            Some(state) => (Some(state), None),
            None => (None, Some(reason::UNREADABLE.to_owned())),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => (None, None),
        Err(_) => (None, Some(reason::UNREADABLE.to_owned())),
    }
}

/// Removes whatever is at the state path without following a link: a link or a file is
/// unlinked, a directory is deleted with its content.
fn remove_untrusted(fs: &dyn Fs, dir: &std::path::Path, is_real_dir: bool) -> io::Result<()> {
    if is_real_dir {
        fs.remove_dir_all(dir)
    } else {
        fs.remove_file(dir)
    }
}

/// The privileged read. The state is used only from a directory that can be trusted
/// (`ubiqx_nofap::state_dir::assess`): on Unix anything else is removed, never following
/// a link, and counts as "no state"; on Windows a reparse point or a non-directory makes
/// the helper stop (`Err`). Nothing is created here.
pub(crate) fn open(backend: &dyn OsBackend, fs: &dyn Fs) -> Result<Option<SystemState>, ()> {
    let dir = backend.state_dir();
    let meta = fs.dir_meta(&dir).map_err(|_| ())?;
    match state_dir::assess(&meta, backend.os()) {
        Trust::Absent => Ok(None),
        Trust::Trusted => Ok(fs.read(&file(backend)).ok().and_then(|bytes| parse(&bytes))),
        Trust::Reset => {
            remove_untrusted(fs, &dir, meta.is_dir && !meta.is_link).map_err(|_| ())?;
            Ok(None)
        }
        Trust::Refuse => Err(()),
    }
}

/// Writes the state file, creating the directory when it is not there. The directory must
/// be trusted afterwards: a helper that does not run as root creates one that is not, and
/// stops here before it changed anything.
pub(crate) fn write(backend: &dyn OsBackend, fs: &dyn Fs, state: &SystemState) -> io::Result<()> {
    let dir = backend.state_dir();
    let refused = || io::Error::from(io::ErrorKind::PermissionDenied);
    if !fs.dir_meta(&dir)?.exists {
        fs.create_dir(&dir, Some(STATE_DIR_MODE))?;
    }
    if state_dir::assess(&fs.dir_meta(&dir)?, backend.os()) != Trust::Trusted {
        return Err(refused());
    }
    let json = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
    fs.write_atomic(&file(backend), &json, Some(STATE_FILE_MODE))
}

/// Deletes the state file and its directory (when no owner remains). A directory that
/// holds something else is left with it.
pub(crate) fn clear(backend: &dyn OsBackend, fs: &dyn Fs) -> io::Result<()> {
    let dir = backend.state_dir();
    if !fs.dir_meta(&dir)?.exists {
        return Ok(());
    }
    match fs.remove_file(&file(backend)) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    if fs.list(&dir)?.is_empty() {
        fs.remove_dir_all(&dir)?;
    }
    Ok(())
}
