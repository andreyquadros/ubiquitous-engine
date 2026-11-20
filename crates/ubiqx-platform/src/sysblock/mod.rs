//! The system layer of the `nofap` module (L2 to L4): the hosts block, the family resolver
//! and the browser policies, behind the one port `ubiqx_core::nofap::SystemShield`.
//!
//! One adapter, [`NativeSystemShield`], whose three OS back-ends ([`OsBackend`]) are command
//! builders and output parsers compiled on every OS. Everything that touches the machine
//! goes through four small traits, so whole apply → observe → remove cycles run in tests
//! against an in-memory file system and a recorded command runner:
//!
//! * [`Runner`]: runs one system tool, by absolute path, with an environment the caller
//!   built itself;
//! * [`Fs`]: reads, atomic writes, removals, directory metadata;
//! * [`Elevator`]: re-runs this executable as `--shield-helper <verb>` through the
//!   operating system's own prompt;
//! * [`NameLookup`]: resolves the canary and the probe name through the OS.
//!
//! The adapter holds no decision: validation, the hosts splice, the policy values, the
//! gate and the judgement of an observation are `ubiqx_nofap`.
//!
//! | File | Content |
//! |---|---|
//! | `shield.rs` | [`NativeSystemShield`]: the unprivileged side |
//! | `helper.rs` | the privileged side: verbs, argument parsing, exit codes |
//! | `converge.rs` | brings the machine to what the owners in the state file want |
//! | `hostsfile.rs`, `state.rs`, `observe.rs` | the hosts file, the state directory, the look at the machine |
//! | `macos.rs`, `windows.rs`, `linux.rs`, `policy_reg.rs`, `policy_files.rs` | the back-ends |
//! | `parse.rs` | parsers of what the system tools print |
//! | `elevate.rs` | the elevation command lines and their exit codes |
//! | `sys.rs` | the real ports; the only file with `cfg` |

use std::io;
use std::path::{Path, PathBuf};

use ubiqx_core::nofap::{
    Os, PolicyCreated, PolicyObservation, ResolverId, ResolverObservation, SystemObservation,
};
use ubiqx_nofap::state_dir::DirMeta;

pub mod elevate;
pub mod helper;
pub mod linux;
pub mod macos;
pub mod parse;
pub mod sys;
pub mod windows;

mod converge;
mod hostsfile;
mod observe;
mod policy_files;
mod policy_reg;
mod shield;
mod state;

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

pub use shield::NativeSystemShield;

/// What a system tool printed and how it exited.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Output {
    /// Exit code; `-1` when the process was killed by a signal.
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

/// Runs one system tool. `program` is an absolute path and `env` is the whole environment
/// of the child: nothing is inherited.
pub trait Runner: Send + Sync {
    fn run(&self, program: &str, args: &[String], env: &[(String, String)]) -> io::Result<Output>;
}

/// The file operations of the system layer. Nothing here follows a link unless it says so.
pub trait Fs: Send + Sync {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    /// Writes through a temporary file in the same directory and a rename, keeping the
    /// owner and mode of an existing file (`mode` is used for a new one, on Unix).
    fn write_atomic(&self, path: &Path, bytes: &[u8], mode: Option<u32>) -> io::Result<()>;
    /// Overwrites the file in place and syncs: the fallback when the rename fails because
    /// the file is a mount point.
    fn write_in_place(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    /// Deletes a directory tree without following links inside it.
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
    /// Creates one directory (`mode` on Unix).
    fn create_dir(&self, path: &Path, mode: Option<u32>) -> io::Result<()>;
    /// `lstat` of `path`: never follows a link; a missing path is `exists: false`.
    fn dir_meta(&self, path: &Path) -> io::Result<DirMeta>;
    /// The file a path resolves to (links followed), for the hosts file.
    fn resolve(&self, path: &Path) -> io::Result<PathBuf>;
    /// File names inside a directory, sorted.
    fn list(&self, dir: &Path) -> io::Result<Vec<String>>;
    fn exists(&self, path: &Path) -> bool;
}

/// How an elevated run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElevatedExit {
    /// The helper ran and exited with this code (`ubiqx_nofap::plan::EXIT_*`).
    Exited(i32),
    /// The user declined the OS prompt.
    Declined,
    /// Elevation cannot work here; a short code (`no_polkit_agent`, `appimage`, …).
    Unavailable(String),
}

/// Runs `exe --shield-helper <args>` with administrator rights through the operating
/// system's own prompt and waits for it.
pub trait Elevator: Send + Sync {
    fn run_elevated(&self, exe: &Path, args: &[String]) -> ElevatedExit;
}

/// Resolves a name through the operating system (`getaddrinfo`).
pub trait NameLookup: Send + Sync {
    /// The addresses `name` resolves to; empty when it does not resolve.
    fn lookup(&self, name: &str) -> Vec<String>;

    /// [`lookup`](Self::lookup) for a name that is expected to be blocked (the canary of
    /// the hosts block). A system whose resolver library never hands out the null address
    /// says "blocked" another way, and that answer is given here as the null address, so
    /// the judgement reads one shape on every system. Windows is such a system: for a
    /// name whose only answer is the null address, from the hosts file or from DNS,
    /// `getaddrinfo` fails with "valid name, no data", while a name nobody knows fails
    /// with "host not found".
    fn lookup_blocked(&self, name: &str) -> Vec<String> {
        self.lookup(name)
    }
}

/// Records the policy values a back-end is about to create (see
/// [`OsBackend::apply_policies`]). An `Err` is a code of `ubiqx_core::nofap::reason`.
pub type NoteCreated<'a> = dyn FnMut(&[PolicyCreated]) -> Result<(), String> + 'a;

/// One operating system's way of doing each tier: paths, argv builders, the fixed scripts
/// and the parsers of what the tools print. Implementations hold no `cfg`: all three
/// compile on every OS. The hosts file and the state file are handled by shared code
/// (`hostsfile`, `state`) from the paths given here.
pub trait OsBackend: Send + Sync {
    fn os(&self) -> Os;
    /// The hosts file.
    fn hosts_path(&self) -> PathBuf;
    /// The helper's state directory.
    fn state_dir(&self) -> PathBuf;
    /// The environment every child process gets (nothing inherited).
    fn child_env(&self) -> Vec<(String, String)>;
    /// Absolute path of the tool the elevation command starts (`osascript`, PowerShell,
    /// `pkexec`).
    fn elevation_tool(&self) -> String;
    /// Unprivileged look at system DNS. `full` is `ObserveOpts::full` (Windows: the
    /// PowerShell read instead of the quick registry read; ignored elsewhere).
    fn observe_resolver(&self, runner: &dyn Runner, fs: &dyn Fs, full: bool)
        -> ResolverObservation;
    /// Unprivileged look at the browsers' policy locations.
    fn observe_policies(&self, runner: &dyn Runner, fs: &dyn Fs) -> Vec<PolicyObservation>;
    /// Points system DNS at the resolver `id` (privileged). `observed` is a fresh look.
    /// A failure is one of the codes of `ubiqx_core::nofap::reason`.
    fn apply_resolver(
        &self,
        id: ResolverId,
        observed: &ResolverObservation,
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<(), String>;
    /// Resets to automatic the interfaces whose servers are still exactly a provider's
    /// set, and removes the drop-in (privileged). No recorded value is ever written back.
    /// `Ok(true)` when something was undone. A service that is not running is never
    /// started for it.
    fn remove_resolver(
        &self,
        observed: &SystemObservation,
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<bool, String>;
    /// Writes the policy values of the browsers that are present, can be managed and hold
    /// nobody else's choice (privileged): a value that is missing, and a value of ours
    /// that somebody changed. `created` is what the state file lists as ours; the answer
    /// is the list after this run. Before the first write the back-end hands that list to
    /// `note`, which records it: a run that fails or dies half-way then leaves no value
    /// the state file does not know about. When `note` fails nothing is written.
    fn apply_policies(
        &self,
        observed: &[PolicyObservation],
        created: &[PolicyCreated],
        runner: &dyn Runner,
        fs: &dyn Fs,
        note: &mut NoteCreated<'_>,
    ) -> Result<Vec<PolicyCreated>, String>;
    /// Deletes our own policy files, and the values `created` lists that still equal ours
    /// (privileged). `Ok(true)` when something was deleted.
    fn remove_policies(
        &self,
        observed: &[PolicyObservation],
        created: &[PolicyCreated],
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<bool, String>;
    /// Flushes the system's name caches after a change (failures ignored).
    fn flush_dns(&self, runner: &dyn Runner);
}

/// The back-end of the operating system this binary runs on; `None` where there is none,
/// or where Windows does not say where its system directories are.
pub fn native_backend() -> Option<Box<dyn OsBackend>> {
    Some(match Os::current()? {
        Os::Macos => Box::new(macos::MacosBackend),
        Os::Linux => Box::new(linux::LinuxBackend),
        Os::Windows => {
            let (system32, program_files) = sys::windows_dirs()?;
            Box::new(windows::WindowsBackend::new(&system32, &program_files))
        }
    })
}

/// The path separator of `os`. Paths of the system layer are built and taken apart as
/// text with it, never with the host's own rules, so the Windows back-end gives the same
/// paths when its tests run on Linux.
fn separator(os: Os) -> char {
    match os {
        Os::Windows => '\\',
        Os::Macos | Os::Linux => '/',
    }
}

/// `base` + separator + `name`.
pub(crate) fn join(os: Os, base: &Path, name: &str) -> PathBuf {
    let base = base.to_string_lossy();
    let base = base.trim_end_matches(separator(os));
    PathBuf::from(format!("{base}{}{name}", separator(os)))
}

/// The directory that holds `path`; `None` for a path without a separator.
pub(crate) fn parent(os: Os, path: &Path) -> Option<PathBuf> {
    let text = path.to_string_lossy();
    let (dir, _) = text.rsplit_once(separator(os))?;
    Some(PathBuf::from(if dir.is_empty() {
        separator(os).to_string()
    } else {
        dir.to_owned()
    }))
}
