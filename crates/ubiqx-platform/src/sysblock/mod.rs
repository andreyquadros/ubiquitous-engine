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
//! Seeded by the foundation unit: the traits and the constructor signatures. The
//! back-ends (`macos.rs`, `windows.rs`, `linux.rs`), `elevate.rs`, `sys.rs` and the body of
//! `helper.rs` belong to the owner of this module.
#![allow(unused_variables, dead_code)]

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ubiqx_core::nofap::{
    ApplyOutcome, ObserveOpts, Os, PlannedChange, RemoveScope, SystemCapabilities,
    SystemObservation, SystemPlan, SystemShield,
};
use ubiqx_core::{CoreResult, ProductId};
use ubiqx_nofap::gate::Gate;
use ubiqx_nofap::state_dir::DirMeta;

pub mod helper;

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
}

/// One operating system's way of doing each tier: paths, argv builders, the fixed scripts
/// and the parsers of what the tools print. Implementations hold no `cfg`: all three
/// compile on every OS.
pub trait OsBackend: Send + Sync {
    fn os(&self) -> Os;
    /// The hosts file.
    fn hosts_path(&self) -> PathBuf;
    /// The helper's state directory.
    fn state_dir(&self) -> PathBuf;
    /// The environment every child process gets (nothing inherited).
    fn child_env(&self) -> Vec<(String, String)>;
    /// Unprivileged look at the machine: hosts text, DNS per interface, the drop-in,
    /// policy values, the state file. `full` is `ObserveOpts::full` (Windows: the
    /// PowerShell read instead of the quick registry read). The canary and the probe are
    /// added by the caller when its `ObserveOpts` asks for them.
    fn observe(&self, runner: &dyn Runner, fs: &dyn Fs, full: bool) -> SystemObservation;
    /// Points system DNS at the resolver of `plan` (privileged). Returns the commands'
    /// failure, if any, as a short reason code.
    fn apply_resolver(
        &self,
        plan: &SystemPlan,
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<(), String>;
    /// Resets to automatic the interfaces whose servers are still exactly ours, and removes
    /// the drop-in (privileged).
    fn remove_resolver(
        &self,
        observation: &SystemObservation,
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<(), String>;
    /// Writes the browser policy values (privileged); returns what it created.
    fn apply_policies(
        &self,
        observation: &SystemObservation,
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<Vec<ubiqx_core::nofap::PolicyCreated>, String>;
    /// Deletes the policy values and files listed in `created` that still equal ours
    /// (privileged).
    fn remove_policies(
        &self,
        created: &[ubiqx_core::nofap::PolicyCreated],
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<(), String>;
    /// Flushes the system's name caches after the hosts file changed (failures ignored).
    fn flush_dns(&self, runner: &dyn Runner);
}

/// The system layer of this machine: implements [`SystemShield`] over an [`OsBackend`] and
/// the four ports. `apply` and `remove` re-run `exe` elevated; everything else is
/// unprivileged.
pub struct NativeSystemShield {
    os: Box<dyn OsBackend>,
    runner: Arc<dyn Runner>,
    fs: Arc<dyn Fs>,
    elevator: Arc<dyn Elevator>,
    names: Arc<dyn NameLookup>,
    exe: PathBuf,
    owner: ProductId,
    gate: Gate,
}

impl NativeSystemShield {
    /// Builds the shield from explicit parts (tests use fakes for every port). `exe` is the
    /// executable that is re-run elevated; `owner` is the product whose wants `apply`
    /// records; `gate` is the per-OS gate with the tester's override.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        os: Box<dyn OsBackend>,
        runner: Arc<dyn Runner>,
        fs: Arc<dyn Fs>,
        elevator: Arc<dyn Elevator>,
        names: Arc<dyn NameLookup>,
        exe: PathBuf,
        owner: ProductId,
        gate: Gate,
    ) -> Self {
        Self {
            os,
            runner,
            fs,
            elevator,
            names,
            exe,
            owner,
            gate,
        }
    }

    /// The shield of the operating system this binary runs on, with the real ports, the
    /// current executable and the gate read from the environment. `None` on a system
    /// without a back-end.
    pub fn native(owner: ProductId) -> Option<Self> {
        todo!()
    }
}

impl SystemShield for NativeSystemShield {
    fn capabilities(&self) -> SystemCapabilities {
        todo!()
    }

    fn observe(&self, opts: ObserveOpts) -> CoreResult<SystemObservation> {
        todo!()
    }

    fn preview(&self, plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>> {
        todo!()
    }

    fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyOutcome> {
        todo!()
    }

    fn remove(&self, owner: ProductId, scope: RemoveScope) -> CoreResult<ApplyOutcome> {
        todo!()
    }
}
