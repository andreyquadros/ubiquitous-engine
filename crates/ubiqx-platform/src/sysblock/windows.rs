//! The Windows back-end: PowerShell's `DnsClient` and `NetAdapter` modules for DNS per
//! adapter, `reg.exe` for the quick DNS read and for browser policies (`policy_reg`),
//! `ipconfig.exe` for the cache.
//!
//! **No path comes from an environment variable.** An elevated process started through
//! UAC receives the user's variables, and a per-user `windir`, `SystemRoot` or
//! `PSModulePath` is a known way to redirect an elevated program. `<System32>` and
//! `<ProgramFiles>` are handed in by the caller, who asked Windows itself
//! (`sys::windows_dirs`); every program is an absolute path under `<System32>`; the child
//! environment is built here; the fixed scripts name their cmdlets with the module.

use std::path::PathBuf;

use parking_lot::Mutex;
use ubiqx_core::nofap::{
    reason, InterfaceDns, Os, PolicyCreated, PolicyObservation, ResolverId, ResolverObservation,
    SystemObservation,
};
use ubiqx_nofap::resolvers::{self, RESOLVERS};
use ubiqx_nofap::{plan, state_dir};

use super::parse::{self, Adapter};
use super::{policy_reg, Fs, OsBackend, Output, Runner};

/// Lists every adapter that is up with its DNS servers of both address families, as JSON.
pub const FULL_READ_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; $rows = @(NetAdapter\\Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | ForEach-Object { [pscustomobject]@{ index = [int]$_.ifIndex; guid = [string]$_.InterfaceGuid; name = [string]$_.Name; servers = @(DnsClient\\Get-DnsClientServerAddress -InterfaceIndex $_.ifIndex | ForEach-Object { $_.ServerAddresses }) } }); ConvertTo-Json -InputObject $rows -Depth 3 -Compress";
/// Sets the resolver on every adapter that is up (Windows asks all of them in parallel).
/// `{servers}` is replaced with the quoted addresses of the compiled table.
pub const SET_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; NetAdapter\\Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | ForEach-Object { DnsClient\\Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ServerAddresses ({servers}) }";
/// Puts the listed adapters back on automatic DNS. `{indexes}` is replaced with interface
/// index numbers.
pub const RESET_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; foreach ($i in @({indexes})) { DnsClient\\Set-DnsClientServerAddress -InterfaceIndex $i -ResetServerAddresses }";
/// Where static DNS servers live in the registry, per interface GUID.
pub const TCPIP_INTERFACES: &str =
    "HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces";
pub const TCPIP6_INTERFACES: &str =
    "HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip6\\Parameters\\Interfaces";
pub const NAME_SERVER: &str = "NameServer";

pub struct WindowsBackend {
    system32: String,
    program_files: String,
    /// The adapters that were up at the last full read: what the quick read looks at.
    adapters: Mutex<Vec<Adapter>>,
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

impl WindowsBackend {
    /// `system32` and `program_files` are what `GetSystemDirectoryW` and
    /// `SHGetKnownFolderPath(FOLDERID_ProgramFiles)` return.
    pub fn new(system32: &str, program_files: &str) -> Self {
        Self {
            system32: system32.trim_end_matches('\\').to_owned(),
            program_files: program_files.trim_end_matches('\\').to_owned(),
            adapters: Mutex::new(Vec::new()),
        }
    }

    pub fn program_files(&self) -> &str {
        &self.program_files
    }

    pub fn reg(&self) -> String {
        format!("{}\\reg.exe", self.system32)
    }

    pub fn ipconfig(&self) -> String {
        format!("{}\\ipconfig.exe", self.system32)
    }

    pub fn powershell(&self) -> String {
        format!("{}\\WindowsPowerShell\\v1.0\\powershell.exe", self.system32)
    }

    /// The Windows directory: the parent of `<System32>`.
    fn windows_dir(&self) -> &str {
        self.system32
            .rsplit_once('\\')
            .map_or(self.system32.as_str(), |(dir, _)| dir)
    }

    fn run_script(&self, runner: &dyn Runner, script: &str) -> Option<Output> {
        let list = ["-NoProfile", "-NonInteractive", "-Command", script];
        runner
            .run(&self.powershell(), &args(&list), &self.child_env())
            .ok()
            .filter(Output::success)
    }

    fn full_read(&self, runner: &dyn Runner) -> Option<Vec<InterfaceDns>> {
        let output = self.run_script(runner, FULL_READ_SCRIPT)?;
        let adapters = parse::adapters_json(&output.stdout)?;
        let interfaces = adapters
            .iter()
            .map(|adapter| InterfaceDns {
                id: adapter.index.to_string(),
                name: adapter.name.clone(),
                active: true,
                servers: adapter.servers.clone(),
            })
            .collect();
        *self.adapters.lock() = adapters;
        Some(interfaces)
    }

    /// The static servers of the adapters seen at the last full read, from the registry.
    /// `None` when there was no such read yet or the registry gave nothing usable.
    fn quick_read(&self, runner: &dyn Runner) -> Option<Vec<InterfaceDns>> {
        let known = self.adapters.lock().clone();
        if known.is_empty() {
            return None;
        }
        let mut found: Vec<(String, Vec<String>)> = Vec::new();
        for key in [TCPIP_INTERFACES, TCPIP6_INTERFACES] {
            let list = ["query", key, "/s", "/v", NAME_SERVER];
            let output = runner
                .run(&self.reg(), &args(&list), &self.child_env())
                .ok()?;
            for value in parse::reg_values(&output.stdout) {
                if value.name.eq_ignore_ascii_case(NAME_SERVER) {
                    let guid = parse::key_leaf(&value.key).to_ascii_uppercase();
                    found.push((guid, parse::name_servers(&value.data)));
                }
            }
        }
        if !known
            .iter()
            .any(|adapter| found.iter().any(|(guid, _)| *guid == adapter.guid))
        {
            return None;
        }
        Some(
            known
                .iter()
                .map(|adapter| InterfaceDns {
                    id: adapter.index.to_string(),
                    name: adapter.name.clone(),
                    active: true,
                    servers: found
                        .iter()
                        .filter(|(guid, _)| *guid == adapter.guid)
                        .flat_map(|(_, servers)| servers.clone())
                        .collect(),
                })
                .collect(),
        )
    }
}

impl OsBackend for WindowsBackend {
    fn os(&self) -> Os {
        Os::Windows
    }

    fn hosts_path(&self) -> PathBuf {
        PathBuf::from(format!("{}\\drivers\\etc\\hosts", self.system32))
    }

    fn state_dir(&self) -> PathBuf {
        PathBuf::from(format!(
            "{}\\{}",
            self.program_files,
            state_dir::STATE_DIR_WINDOWS_UNDER_PROGRAM_FILES
        ))
    }

    fn child_env(&self) -> Vec<(String, String)> {
        let modules = format!("{}\\WindowsPowerShell\\v1.0\\Modules", self.system32);
        vec![
            ("SystemRoot".to_owned(), self.windows_dir().to_owned()),
            ("windir".to_owned(), self.windows_dir().to_owned()),
            ("PATH".to_owned(), self.system32.clone()),
            ("PSModulePath".to_owned(), modules),
        ]
    }

    fn elevation_tool(&self) -> String {
        self.powershell()
    }

    fn observe_resolver(
        &self,
        runner: &dyn Runner,
        _fs: &dyn Fs,
        full: bool,
    ) -> ResolverObservation {
        let read = if full {
            self.full_read(runner)
        } else {
            self.quick_read(runner)
        };
        ResolverObservation {
            error: read.is_none().then(|| reason::UNREADABLE.to_owned()),
            interfaces: read.unwrap_or_default(),
            full,
            ..ResolverObservation::default()
        }
    }

    fn observe_policies(&self, runner: &dyn Runner, fs: &dyn Fs) -> Vec<PolicyObservation> {
        policy_reg::observe(self, runner, fs)
    }

    fn apply_resolver(
        &self,
        id: ResolverId,
        _observed: &ResolverObservation,
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<(), String> {
        // The addresses are constants of the compiled table; the check keeps anything
        // that is not an address out of a script even if the table were wrong.
        let addresses = resolvers::lookup(id).addresses();
        if !addresses
            .iter()
            .all(|a| a.parse::<std::net::IpAddr>().is_ok())
        {
            return Err(reason::FAILED.to_owned());
        }
        let quoted: Vec<String> = addresses.iter().map(|a| format!("'{a}'")).collect();
        let script = SET_SCRIPT.replace("{servers}", &quoted.join(","));
        self.run_script(runner, &script)
            .map(|_| ())
            .ok_or_else(|| reason::FAILED.to_owned())
    }

    fn remove_resolver(
        &self,
        observed: &SystemObservation,
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<bool, String> {
        // Interface indexes, read back as numbers: nothing else can reach the script.
        let mut indexes: Vec<u32> = RESOLVERS
            .iter()
            .flat_map(|row| plan::reset_targets(observed, row.id))
            .filter_map(|id| id.parse().ok())
            .collect();
        indexes.sort_unstable();
        indexes.dedup();
        if indexes.is_empty() {
            return Ok(false);
        }
        let list: Vec<String> = indexes.iter().map(u32::to_string).collect();
        let script = RESET_SCRIPT.replace("{indexes}", &list.join(","));
        self.run_script(runner, &script)
            .map(|_| true)
            .ok_or_else(|| reason::FAILED.to_owned())
    }

    fn apply_policies(
        &self,
        observed: &[PolicyObservation],
        created: &[PolicyCreated],
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<Vec<PolicyCreated>, String> {
        policy_reg::apply(self, runner, observed, created)
    }

    fn remove_policies(
        &self,
        observed: &[PolicyObservation],
        created: &[PolicyCreated],
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<bool, String> {
        policy_reg::remove(self, runner, observed, created)
    }

    fn flush_dns(&self, runner: &dyn Runner) {
        let _ = runner.run(&self.ipconfig(), &args(&["/flushdns"]), &self.child_env());
    }
}
