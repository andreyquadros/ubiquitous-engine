//! The real ports of the system layer: process spawn, file I/O, directory metadata and
//! `getaddrinfo`. No decision is taken here, and this is the only file of the module that
//! may contain `cfg`: Unix metadata, the Windows no-console flag and the two Windows
//! directory lookups, which are the module's only FFI.

use std::fs;
use std::io::{self, Write};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ubiqx_nofap::state_dir::DirMeta;

use super::{Fs, NameLookup, Output, Runner};

/// Runs system tools as child processes with exactly the environment it is given.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[String], env: &[(String, String)]) -> io::Result<Output> {
        let mut command = Command::new(program);
        command.args(args).env_clear().stdin(Stdio::null());
        command.envs(env.iter().map(|(name, value)| (name, value)));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: no console flashes up behind the app.
            command.creation_flags(0x0800_0000);
        }
        let output = command.output()?;
        Ok(Output {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// The file system of this machine.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemFs;

/// Sets the permission bits of `path` exactly: creation alone is subject to the umask of
/// whoever started the helper, and the state file must stay readable by the app.
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

/// Creates `path` anew (never through an existing file or link) and fills it.
fn create_filled(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Gives `temp` the owner and mode of the file at `path`; the mode `new` when there is
/// no such file yet.
fn keep_owner_and_mode(temp: &Path, path: &Path, new: u32) -> io::Result<()> {
    #[cfg(unix)]
    if let Ok(existing) = fs::metadata(path) {
        use std::os::unix::fs::MetadataExt;
        std::os::unix::fs::chown(temp, Some(existing.uid()), Some(existing.gid()))?;
        return fs::set_permissions(temp, existing.permissions());
    }
    #[cfg(not(unix))]
    let _ = path;
    set_mode(temp, new)
}

impl Fs for SystemFs {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8], mode: Option<u32>) -> io::Result<()> {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".ubiqx-shield.tmp");
        let temp = path.with_file_name(name);
        // A temporary file left by a run that was killed is ours to replace.
        let _ = fs::remove_file(&temp);
        let mode = mode.unwrap_or(0o644);
        let written = create_filled(&temp, bytes, mode)
            .and_then(|()| keep_owner_and_mode(&temp, path, mode))
            .and_then(|()| fs::rename(&temp, path));
        if written.is_err() {
            let _ = fs::remove_file(&temp);
        }
        written
    }

    fn write_in_place(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        // Written over from the start and cut to length afterwards, never emptied first: a
        // write that stops half-way then leaves the old tail behind the new head (for the
        // hosts splice both share everything in front of the block) instead of an empty
        // or cut-off file.
        let mut file = fs::OpenOptions::new().write(true).open(path)?;
        file.write_all(bytes)?;
        file.set_len(bytes.len() as u64)?;
        file.sync_all()
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(path)
    }

    fn create_dir(&self, path: &Path, mode: Option<u32>) -> io::Result<()> {
        fs::create_dir(path)?;
        set_mode(path, mode.unwrap_or(0o755))
    }

    fn dir_meta(&self, path: &Path) -> io::Result<DirMeta> {
        let meta = match fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(DirMeta::default()),
            Err(error) => return Err(error),
        };
        #[cfg(unix)]
        let (is_link, uid, mode) = {
            use std::os::unix::fs::MetadataExt;
            let link = meta.file_type().is_symlink();
            (link, Some(meta.uid()), Some(meta.mode() & 0o7777))
        };
        #[cfg(windows)]
        let (is_link, uid, mode) = {
            use std::os::windows::fs::MetadataExt;
            // FILE_ATTRIBUTE_REPARSE_POINT: a symbolic link, a junction or any other.
            let reparse = meta.file_attributes() & 0x0400 != 0;
            (reparse || meta.file_type().is_symlink(), None, None)
        };
        #[cfg(not(any(unix, windows)))]
        let (is_link, uid, mode) = (meta.file_type().is_symlink(), None, None);
        Ok(DirMeta {
            exists: true,
            is_dir: meta.is_dir(),
            is_link,
            uid,
            mode,
        })
    }

    fn resolve(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(dir)? {
            names.push(entry?.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        Ok(names)
    }

    fn exists(&self, path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok()
    }
}

/// Name resolution through the operating system's own resolver.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemLookup;

impl NameLookup for SystemLookup {
    fn lookup(&self, name: &str) -> Vec<String> {
        // Windows answers an empty name with this machine's own addresses; no name, no answer.
        if name.is_empty() {
            return Vec::new();
        }
        let found = (name, 0).to_socket_addrs();
        let mut addresses: Vec<String> = found
            .map(|found| found.map(|address| address.ip().to_string()).collect())
            .unwrap_or_default();
        addresses.sort();
        addresses.dedup();
        addresses
    }
}

/// `<System32>` and `<ProgramFiles>` as Windows itself reports them
/// (`GetSystemDirectoryW`, `SHGetKnownFolderPath`), never from an environment variable.
#[cfg(windows)]
pub fn windows_dirs() -> Option<(String, String)> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
    use windows::Win32::UI::Shell::{FOLDERID_ProgramFiles, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    let mut buffer = [0u16; 512];
    // SAFETY: the buffer outlives the call, which writes at most its length.
    let length = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;
    if length == 0 || length >= buffer.len() {
        return None;
    }
    let system32 = String::from_utf16(&buffer[..length]).ok()?;
    // SAFETY: the returned string is valid until it is freed below, once.
    let program_files = unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_ProgramFiles, KF_FLAG_DEFAULT, None).ok()?;
        let text = path.to_string();
        CoTaskMemFree(Some(path.as_ptr() as *const core::ffi::c_void));
        text.ok()?
    };
    Some((system32, program_files))
}

/// Makes `SystemRoot` and `windir` of this process the directory Windows itself reports.
///
/// The helper is started through UAC with the variables of the user's session, and
/// Winsock finds its providers through `%SystemRoot%`: in a process started with a false
/// value no name resolves at all (the smoke run on a real Windows machine showed it), so
/// the lookup that guards a DNS change would see "no names before, no names after" and
/// keep a resolver that broke the network. Called before the first lookup. Nothing is
/// read from these variables anywhere in the module. Does nothing on another system.
pub fn pin_system_environment() {
    #[cfg(windows)]
    if let Some((system32, _)) = windows_dirs() {
        if let Some((windows_dir, _)) = system32.trim_end_matches('\\').rsplit_once('\\') {
            std::env::set_var("SystemRoot", windows_dir);
            std::env::set_var("windir", windows_dir);
        }
    }
}

/// There are no Windows directories on another system.
#[cfg(not(windows))]
pub fn windows_dirs() -> Option<(String, String)> {
    None
}
