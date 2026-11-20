//! The hosts file: the unprivileged look and the privileged write.
//!
//! The write is a splice (`ubiqx_nofap::hosts::splice`), never a rewrite, of the file the
//! hosts path resolves to, and only when that file and its directory belong to root and
//! nobody else can write to the directory. On Unix it goes through a temporary file and a
//! rename (the new file is given the owner and mode of the old one); where the file is a
//! mount point and cannot be renamed over, it is written in place. On Windows it is always
//! written in place: a file renamed over the hosts file is a new file, with the owner of
//! whoever created it instead of the one the hosts file had, and the removal would hand
//! the user back a hosts file that is not the one they had. After every write the file is
//! read back, and on a mismatch the bytes read at the start are put back.

use std::io;
use std::path::{Path, PathBuf};

use ubiqx_core::nofap::{reason, HostsObservation, Os};
use ubiqx_nofap::hosts;
use ubiqx_nofap::state_dir::DirMeta;

use super::{parent, Fs, OsBackend};

/// Permission bits that let somebody other than the owner write.
const WRITABLE_BY_OTHERS: u32 = 0o022;
/// Mode of a hosts file the helper has to create.
const HOSTS_MODE: u32 = 0o644;

fn missing(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound
}

/// Whether `meta` is something root alone controls. Windows reports no owner; there the
/// path is under `<System32>`, which only administrators can write to, and a reparse
/// point is refused instead.
fn root_only(meta: &DirMeta, os: Os, directory: bool) -> bool {
    if !meta.exists || meta.is_link || meta.is_dir != directory {
        return false;
    }
    match os {
        Os::Windows => true,
        Os::Macos | Os::Linux => {
            let closed = meta.mode.is_some_and(|mode| mode & WRITABLE_BY_OTHERS == 0);
            meta.uid == Some(0) && (closed || !directory)
        }
    }
}

/// The file to write: what the hosts path resolves to, when it and its directory are
/// root's. `Ok(None)`: there is no hosts file and its directory is root's, so one may be
/// created. `Err`: the code `write_refused`.
fn target(backend: &dyn OsBackend, fs: &dyn Fs) -> Result<Option<PathBuf>, &'static str> {
    let os = backend.os();
    let refused = reason::WRITE_REFUSED;
    let hosts_path = backend.hosts_path();
    let dir_is_roots = |file: &Path| {
        parent(os, file)
            .and_then(|dir| fs.dir_meta(&dir).ok())
            .is_some_and(|meta| root_only(&meta, os, true))
    };
    let meta = fs.dir_meta(&hosts_path).map_err(|_| refused)?;
    if !meta.exists {
        return dir_is_roots(&hosts_path).then_some(None).ok_or(refused);
    }
    // Windows: the path comes from the system itself and a reparse point is not followed.
    let file = match os {
        Os::Windows => hosts_path,
        Os::Macos | Os::Linux => fs.resolve(&hosts_path).map_err(|_| refused)?,
    };
    let meta = fs.dir_meta(&file).map_err(|_| refused)?;
    if root_only(&meta, os, false) && dir_is_roots(&file) {
        Ok(Some(file))
    } else {
        Err(refused)
    }
}

/// Replaces the bytes of the existing hosts file `file` (see the module documentation
/// for why Windows differs).
fn replace(os: Os, fs: &dyn Fs, file: &Path, bytes: &[u8]) -> io::Result<()> {
    if os == Os::Windows {
        return fs.write_in_place(file, bytes);
    }
    match fs.write_atomic(file, bytes, None) {
        // A mount point cannot be renamed over (containers, some immutable systems).
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ResourceBusy | io::ErrorKind::CrossesDevices
            ) =>
        {
            fs.write_in_place(file, bytes)
        }
        other => other,
    }
}

/// The unprivileged look: the text of the file, and why the tier cannot work here when
/// that can be seen without writing (`unreadable`, `write_refused`).
pub(crate) fn observe(backend: &dyn OsBackend, fs: &dyn Fs) -> HostsObservation {
    let text = match fs.read(&backend.hosts_path()) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        // No hosts file is an empty one: the helper creates it.
        Err(error) if missing(&error) => String::new(),
        Err(_) => {
            return HostsObservation {
                text: None,
                error: Some(reason::UNREADABLE.to_owned()),
            }
        }
    };
    HostsObservation {
        text: Some(text),
        error: target(backend, fs).err().map(str::to_owned),
    }
}

/// Splices `block` into the hosts file (`None` removes it). `Ok(true)` when the file was
/// changed, `Ok(false)` when it already was what it should be. On `Err` (a code of
/// `ubiqx_core::nofap::reason`) the file holds the bytes it held before.
pub(crate) fn write(
    backend: &dyn OsBackend,
    fs: &dyn Fs,
    block: Option<&str>,
) -> Result<bool, &'static str> {
    let hosts_path = backend.hosts_path();
    let original = match fs.read(&hosts_path) {
        Ok(bytes) => bytes,
        Err(error) if missing(&error) => Vec::new(),
        Err(_) => return Err(reason::UNREADABLE),
    };
    let spliced = hosts::splice(&original, block);
    if spliced == original {
        return Ok(false);
    }
    let Some(file) = target(backend, fs)? else {
        // No hosts file yet: create one that holds the block.
        return fs
            .write_atomic(&hosts_path, &spliced, Some(HOSTS_MODE))
            .map(|()| true)
            .map_err(|_| reason::WRITE_REFUSED);
    };
    let os = backend.os();
    if replace(os, fs, &file, &spliced).is_err() {
        return Err(reason::WRITE_REFUSED);
    }
    if fs.read(&file).is_ok_and(|now| now == spliced) {
        return Ok(true);
    }
    // Something rewrote or reverted the file under us (security software): put back what
    // was there and say so.
    if os == Os::Windows || fs.write_atomic(&file, &original, None).is_err() {
        let _ = fs.write_in_place(&file, &original);
    }
    Err(reason::WRITE_REFUSED)
}
