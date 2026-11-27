//! The Linux back-end: a systemd-resolved drop-in for DNS and policy files for the
//! browsers (`policy_files`). The resolver tier needs systemd-resolved running and
//! `/etc/resolv.conf` pointing at it; without both the drop-in would change nothing, and
//! the observation says so. NetworkManager is not touched.

use std::io;
use std::path::{Path, PathBuf};

use ubiqx_core::nofap::{
    reason, Os, PolicyCreated, PolicyObservation, ResolverId, ResolverObservation,
    SystemObservation,
};
use ubiqx_nofap::resolvers::{self, RESOLVED_DROPIN_PATH};
use ubiqx_nofap::state_dir;

use super::{parent, parse, policy_files, Fs, NoteCreated, OsBackend, Runner};

pub const SYSTEMCTL: &str = "/usr/bin/systemctl";
pub const RESOLVECTL: &str = "/usr/bin/resolvectl";
pub const PKEXEC: &str = "/usr/bin/pkexec";
pub const HOSTS_PATH: &str = "/etc/hosts";
pub const RESOLV_CONF: &str = "/etc/resolv.conf";
pub const RESOLVED_UNIT: &str = "systemd-resolved";
/// What `systemctl is-active` prints for a running unit.
pub const ACTIVE: &str = "active";
/// Mode of the directories and files the helper creates under `/etc`.
pub const DIR_MODE: u32 = 0o755;
pub const FILE_MODE: u32 = 0o644;

#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxBackend;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

/// Creates `dir` and every missing directory above it, root-owned and closed to others.
pub(crate) fn create_dirs(fs: &dyn Fs, dir: &Path) -> io::Result<()> {
    if fs.dir_meta(dir)?.exists {
        return Ok(());
    }
    if let Some(above) = parent(Os::Linux, dir).filter(|above| above != dir) {
        create_dirs(fs, &above)?;
    }
    fs.create_dir(dir, Some(DIR_MODE))
}

impl LinuxBackend {
    fn restart_resolved(&self, runner: &dyn Runner) -> Result<(), String> {
        let ran = runner.run(
            SYSTEMCTL,
            &args(&["restart", RESOLVED_UNIT]),
            &self.child_env(),
        );
        match ran {
            Ok(output) if output.success() => Ok(()),
            _ => Err(reason::FAILED.to_owned()),
        }
    }
}

/// Whether the drop-in could take effect on a machine that looks like `observed`.
fn resolved_in_use(observed: &ResolverObservation) -> bool {
    observed.resolved_active == Some(true) && observed.resolv_conf_uses_resolved == Some(true)
}

impl OsBackend for LinuxBackend {
    fn os(&self) -> Os {
        Os::Linux
    }

    fn hosts_path(&self) -> PathBuf {
        PathBuf::from(HOSTS_PATH)
    }

    fn state_dir(&self) -> PathBuf {
        PathBuf::from(state_dir::STATE_DIR_LINUX)
    }

    fn child_env(&self) -> Vec<(String, String)> {
        vec![
            (
                "PATH".to_owned(),
                "/usr/bin:/bin:/usr/sbin:/sbin".to_owned(),
            ),
            ("LANG".to_owned(), "C".to_owned()),
        ]
    }

    fn elevation_tool(&self) -> String {
        PKEXEC.to_owned()
    }

    fn observe_resolver(
        &self,
        runner: &dyn Runner,
        fs: &dyn Fs,
        _full: bool,
    ) -> ResolverObservation {
        // A machine without `systemctl` has no systemd-resolved to be active.
        let active = runner
            .run(
                SYSTEMCTL,
                &args(&["is-active", RESOLVED_UNIT]),
                &self.child_env(),
            )
            .is_ok_and(|output| output.success() && output.stdout.trim() == ACTIVE);
        let uses_resolved = match fs.read(Path::new(RESOLV_CONF)) {
            Ok(bytes) => Some(parse::resolv_conf_uses_stub(&String::from_utf8_lossy(
                &bytes,
            ))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Some(false),
            Err(_) => None,
        };
        let dropin = fs
            .read(Path::new(RESOLVED_DROPIN_PATH))
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
        ResolverObservation {
            interfaces: Vec::new(),
            dropin,
            resolved_active: Some(active),
            resolv_conf_uses_resolved: uses_resolved,
            full: true,
            error: None,
        }
    }

    fn observe_policies(&self, _runner: &dyn Runner, fs: &dyn Fs) -> Vec<PolicyObservation> {
        policy_files::observe(fs)
    }

    fn apply_resolver(
        &self,
        id: ResolverId,
        observed: &ResolverObservation,
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<(), String> {
        if !resolved_in_use(observed) {
            return Err(reason::NO_RESOLVED.to_owned());
        }
        let dropin = Path::new(RESOLVED_DROPIN_PATH);
        let failed = |_| reason::FAILED.to_owned();
        if let Some(dir) = parent(Os::Linux, dropin) {
            create_dirs(fs, &dir).map_err(failed)?;
        }
        fs.write_atomic(
            dropin,
            resolvers::resolved_dropin(id).as_bytes(),
            Some(FILE_MODE),
        )
        .map_err(failed)?;
        self.restart_resolved(runner)
    }

    fn remove_resolver(
        &self,
        observed: &SystemObservation,
        runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<bool, String> {
        // The file name is ours, whatever the file holds by now.
        match fs.remove_file(Path::new(RESOLVED_DROPIN_PATH)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(_) => return Err(reason::FAILED.to_owned()),
        }
        // Only a running service has the file loaded. `systemctl restart` starts a unit
        // that is stopped, and a resolver the user switched off is not ours to switch on.
        if observed.resolver.resolved_active != Some(true) {
            return Ok(true);
        }
        self.restart_resolved(runner).map(|()| true)
    }

    fn apply_policies(
        &self,
        observed: &[PolicyObservation],
        created: &[PolicyCreated],
        _runner: &dyn Runner,
        fs: &dyn Fs,
        note: &mut NoteCreated<'_>,
    ) -> Result<Vec<PolicyCreated>, String> {
        policy_files::apply(fs, observed, created, note)
    }

    fn remove_policies(
        &self,
        _observed: &[PolicyObservation],
        _created: &[PolicyCreated],
        _runner: &dyn Runner,
        fs: &dyn Fs,
    ) -> Result<bool, String> {
        policy_files::remove(fs)
    }

    fn flush_dns(&self, runner: &dyn Runner) {
        // Ignored when `resolvectl` is absent or fails.
        let _ = runner.run(RESOLVECTL, &args(&["flush-caches"]), &self.child_env());
    }
}
