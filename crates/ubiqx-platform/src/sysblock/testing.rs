//! Fakes for the tests of the system layer: an in-memory file system, three pretend
//! machines that answer the commands of their back-end and record every call, a scripted
//! name lookup, and an elevator that runs the helper in this process. Nothing here
//! touches the real system.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use ubiqx_core::nofap::{
    NofapConfig, ObserveOpts, Os, ResolverId, SystemObservation, SystemPlan, SystemShield, Tier,
    CONFIG_VERSION,
};
use ubiqx_core::ProductId;
use ubiqx_nofap::gate::Gate;
use ubiqx_nofap::plan::{self, SystemAssessment};
use ubiqx_nofap::state_dir::DirMeta;
use ubiqx_nofap::{list, resolvers};

use super::helper::{self, Ports};
use super::linux::LinuxBackend;
use super::macos::MacosBackend;
use super::windows::{self, WindowsBackend};
use super::{
    macos, parent, ElevatedExit, Elevator, Fs, NameLookup, NativeSystemShield, OsBackend, Output,
    Runner,
};

pub(crate) const SYSTEM32: &str = "C:\\Windows\\System32";
pub(crate) const PROGRAM_FILES: &str = "C:\\Program Files";
pub(crate) const EXE: &str = "/opt/ubiqx-shield/ubiqx-shield";
pub(crate) const CF: ResolverId = ResolverId::CloudflareFamily;
pub(crate) const ALL_TIERS: &str = "hosts,resolver,policies";

// ---------------------------------------------------------------------------------------------
// The file system
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Node {
    File { bytes: Vec<u8>, uid: u32, mode: u32 },
    Dir { uid: u32, mode: u32 },
    Link { target: String },
}

/// An in-memory file system keyed by the path text. Every change is logged, so a test can
/// say "nothing was written".
pub(crate) struct MemFs {
    os: Os,
    nodes: Mutex<BTreeMap<String, Node>>,
    log: Mutex<Vec<String>>,
    rename_fails: Mutex<HashMap<String, ErrorKind>>,
    tamper: Mutex<HashSet<String>>,
    /// The user new files and directories belong to (0: the helper runs as root).
    euid: Mutex<u32>,
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn not_found() -> io::Error {
    io::Error::from(ErrorKind::NotFound)
}

impl MemFs {
    pub(crate) fn new(os: Os) -> Self {
        let fs = Self {
            os,
            nodes: Mutex::new(BTreeMap::new()),
            log: Mutex::new(Vec::new()),
            rename_fails: Mutex::new(HashMap::new()),
            tamper: Mutex::new(HashSet::new()),
            euid: Mutex::new(0),
        };
        if os != Os::Windows {
            fs.nodes.lock().insert(
                "/".into(),
                Node::Dir {
                    uid: 0,
                    mode: 0o755,
                },
            );
        }
        fs
    }

    /// Creates `path` and every directory above it, root-owned, without logging.
    pub(crate) fn dir(&self, path: &str) {
        self.dir_owned(path, 0, 0o755);
    }

    pub(crate) fn dir_owned(&self, path: &str, uid: u32, mode: u32) {
        if let Some(above) = parent(self.os, Path::new(path)) {
            let above = key(&above);
            if above != path && !self.nodes.lock().contains_key(&above) {
                self.dir(&above);
            }
        }
        self.nodes
            .lock()
            .insert(path.into(), Node::Dir { uid, mode });
    }

    pub(crate) fn file(&self, path: &str, bytes: &[u8]) {
        self.file_owned(path, bytes, 0, 0o644);
    }

    pub(crate) fn file_owned(&self, path: &str, bytes: &[u8], uid: u32, mode: u32) {
        if let Some(above) = parent(self.os, Path::new(path)) {
            if !self.nodes.lock().contains_key(&key(&above)) {
                self.dir(&key(&above));
            }
        }
        let bytes = bytes.to_vec();
        self.nodes
            .lock()
            .insert(path.into(), Node::File { bytes, uid, mode });
    }

    pub(crate) fn link(&self, path: &str, target: &str) {
        let target = target.to_owned();
        self.nodes.lock().insert(path.into(), Node::Link { target });
    }

    pub(crate) fn text(&self, path: &str) -> Option<String> {
        self.read(Path::new(path))
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    pub(crate) fn node(&self, path: &str) -> Option<Node> {
        self.nodes.lock().get(path).cloned()
    }

    pub(crate) fn snapshot(&self) -> BTreeMap<String, Node> {
        self.nodes.lock().clone()
    }

    /// The files of a snapshot (directories left out: the helper may leave an empty one).
    pub(crate) fn files(&self) -> BTreeMap<String, Node> {
        let mut nodes = self.snapshot();
        nodes.retain(|_, node| !matches!(node, Node::Dir { .. }));
        nodes
    }

    /// Every change made through the `Fs` trait, as `op path`.
    pub(crate) fn mutations(&self) -> Vec<String> {
        self.log.lock().clone()
    }

    pub(crate) fn clear_log(&self) {
        self.log.lock().clear();
    }

    /// Makes the rename of every atomic write to `path` fail with `kind`.
    pub(crate) fn fail_rename(&self, path: &str, kind: ErrorKind) {
        self.rename_fails.lock().insert(path.into(), kind);
    }

    /// Makes the next write to `path` land with other bytes than were written.
    pub(crate) fn tamper_next_write(&self, path: &str) {
        self.tamper.lock().insert(path.into());
    }

    pub(crate) fn set_euid(&self, uid: u32) {
        *self.euid.lock() = uid;
    }

    fn follow(&self, path: &Path) -> io::Result<String> {
        let mut at = key(path);
        for _ in 0..16 {
            match self.nodes.lock().get(&at) {
                Some(Node::Link { target }) => at = target.clone(),
                Some(_) => return Ok(at),
                None => return Err(not_found()),
            }
        }
        Err(io::Error::other("too many links"))
    }

    fn has_dir(&self, path: &Path) -> bool {
        parent(self.os, path)
            .is_some_and(|dir| matches!(self.nodes.lock().get(&key(&dir)), Some(Node::Dir { .. })))
    }

    fn store(&self, at: String, bytes: &[u8], mode: Option<u32>) {
        let mut bytes = bytes.to_vec();
        if self.tamper.lock().remove(&at) {
            bytes.extend_from_slice(b"# rewritten by something else\n");
        }
        let mut nodes = self.nodes.lock();
        let (uid, mode) = match nodes.get(&at) {
            Some(Node::File { uid, mode, .. }) => (*uid, *mode),
            _ => (*self.euid.lock(), mode.unwrap_or(0o644)),
        };
        nodes.insert(at, Node::File { bytes, uid, mode });
    }
}

impl Fs for MemFs {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let at = self.follow(path)?;
        match self.nodes.lock().get(&at) {
            Some(Node::File { bytes, .. }) => Ok(bytes.clone()),
            _ => Err(io::Error::other("not a file")),
        }
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8], mode: Option<u32>) -> io::Result<()> {
        if !self.has_dir(path) {
            return Err(not_found());
        }
        if let Some(kind) = self.rename_fails.lock().get(&key(path)) {
            return Err(io::Error::from(*kind));
        }
        self.log.lock().push(format!("write {}", key(path)));
        self.store(key(path), bytes, mode);
        Ok(())
    }

    fn write_in_place(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let at = self.follow(path)?;
        self.log.lock().push(format!("write-in-place {at}"));
        self.store(at, bytes, None);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut nodes = self.nodes.lock();
        match nodes.get(&key(path)) {
            Some(Node::File { .. } | Node::Link { .. }) => {
                nodes.remove(&key(path));
                self.log.lock().push(format!("remove {}", key(path)));
                Ok(())
            }
            Some(Node::Dir { .. }) => Err(io::Error::other("is a directory")),
            None => Err(not_found()),
        }
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        let at = key(path);
        let mut nodes = self.nodes.lock();
        if !matches!(nodes.get(&at), Some(Node::Dir { .. })) {
            return Err(io::Error::other("not a directory"));
        }
        let inside = format!("{at}{}", if self.os == Os::Windows { '\\' } else { '/' });
        nodes.retain(|name, _| *name != at && !name.starts_with(&inside));
        self.log.lock().push(format!("remove-dir {at}"));
        Ok(())
    }

    fn create_dir(&self, path: &Path, mode: Option<u32>) -> io::Result<()> {
        if !self.has_dir(path) {
            return Err(not_found());
        }
        let mut nodes = self.nodes.lock();
        if nodes.contains_key(&key(path)) {
            return Err(io::Error::from(ErrorKind::AlreadyExists));
        }
        let (uid, mode) = (*self.euid.lock(), mode.unwrap_or(0o755));
        nodes.insert(key(path), Node::Dir { uid, mode });
        self.log.lock().push(format!("mkdir {}", key(path)));
        Ok(())
    }

    fn dir_meta(&self, path: &Path) -> io::Result<DirMeta> {
        let unix = self.os != Os::Windows;
        Ok(match self.nodes.lock().get(&key(path)) {
            None => DirMeta::default(),
            Some(Node::Link { .. }) => DirMeta {
                exists: true,
                is_dir: false,
                is_link: true,
                uid: unix.then_some(*self.euid.lock()),
                mode: unix.then_some(0o777),
            },
            Some(Node::Dir { uid, mode }) => DirMeta {
                exists: true,
                is_dir: true,
                is_link: false,
                uid: unix.then_some(*uid),
                mode: unix.then_some(*mode),
            },
            Some(Node::File { uid, mode, .. }) => DirMeta {
                exists: true,
                is_dir: false,
                is_link: false,
                uid: unix.then_some(*uid),
                mode: unix.then_some(*mode),
            },
        })
    }

    fn resolve(&self, path: &Path) -> io::Result<PathBuf> {
        self.follow(path).map(PathBuf::from)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<String>> {
        let nodes = self.nodes.lock();
        if !matches!(nodes.get(&key(dir)), Some(Node::Dir { .. })) {
            return Err(not_found());
        }
        Ok(nodes
            .keys()
            .filter(|name| parent(self.os, Path::new(name)).is_some_and(|above| above == dir))
            .filter(|name| **name != key(dir))
            .filter_map(|name| name.rsplit(['/', '\\']).next().map(str::to_owned))
            .collect())
    }

    fn exists(&self, path: &Path) -> bool {
        self.nodes.lock().contains_key(&key(path))
    }
}

// ---------------------------------------------------------------------------------------------
// The pretend machines
// ---------------------------------------------------------------------------------------------

/// One recorded command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl Call {
    /// The command as one line, for assertions on exact command lines.
    pub(crate) fn line(&self) -> String {
        format!("{} {}", self.program, self.args.join(" "))
    }
}

fn ok(stdout: impl Into<String>) -> io::Result<Output> {
    Ok(Output {
        status: 0,
        stdout: stdout.into(),
        stderr: String::new(),
    })
}

fn exit(status: i32, stderr: &str) -> io::Result<Output> {
    Ok(Output {
        status,
        stdout: String::new(),
        stderr: stderr.to_owned(),
    })
}

/// Failures a test plants in a pretend machine, by a piece of the command line
/// ([`Call::line`]).
#[derive(Default)]
pub(crate) struct Faults {
    /// A command whose line contains one of these exits 1 and changes nothing.
    pub failing: Mutex<Vec<String>>,
    /// A command whose line contains this ends the helper there, the way a kill or a power
    /// cut would: the run unwinds and nothing after that command happens.
    pub dying: Mutex<Option<String>>,
}

impl Faults {
    pub(crate) fn fail(&self, piece: &str) {
        self.failing.lock().push(piece.to_owned());
    }

    pub(crate) fn die_at(&self, piece: &str) {
        *self.dying.lock() = Some(piece.to_owned());
    }

    pub(crate) fn clear(&self) {
        self.failing.lock().clear();
        *self.dying.lock() = None;
    }

    /// Whether `call` is one a test made fail. Panics for one it made fatal.
    fn trips(&self, call: &Call) -> bool {
        let line = call.line();
        let dying = self.dying.lock().clone();
        if dying.is_some_and(|piece| line.contains(&piece)) {
            panic!("the helper was killed at: {line}");
        }
        self.failing.lock().iter().any(|piece| line.contains(piece))
    }
}

/// A network service of the pretend Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Service {
    pub name: String,
    pub enabled: bool,
    pub servers: Vec<String>,
}

/// macOS: `networksetup`, `defaults`, the cache tools.
#[derive(Default)]
pub(crate) struct MacMachine {
    pub calls: Mutex<Vec<Call>>,
    pub services: Mutex<Vec<Service>>,
    /// Firefox's policy domain: preference name and `0` or `1`.
    pub firefox: Mutex<BTreeMap<String, String>>,
    /// Programs whose every run fails.
    pub broken: Mutex<HashSet<String>>,
    pub faults: Faults,
}

impl Runner for MacMachine {
    fn run(&self, program: &str, args: &[String], env: &[(String, String)]) -> io::Result<Output> {
        let call = Call {
            program: program.to_owned(),
            args: args.to_vec(),
            env: env.to_vec(),
        };
        self.calls.lock().push(call.clone());
        if self.broken.lock().contains(program) || self.faults.trips(&call) {
            return exit(1, "broken");
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        match (program, words.as_slice()) {
            (macos::NETWORKSETUP, ["-listallnetworkservices"]) => {
                let mut text =
                    "An asterisk (*) denotes that a network service is disabled.\n".to_owned();
                for service in self.services.lock().iter() {
                    let mark = if service.enabled { "" } else { "*" };
                    text.push_str(&format!("{mark}{}\n", service.name));
                }
                ok(text)
            }
            (macos::NETWORKSETUP, ["-getdnsservers", name]) => {
                let services = self.services.lock();
                let service = services.iter().find(|service| service.name == *name);
                match service {
                    Some(service) if !service.servers.is_empty() => {
                        ok(format!("{}\n", service.servers.join("\n")))
                    }
                    Some(_) => ok(format!("There aren't any DNS Servers set on {name}.\n")),
                    None => exit(4, "not a recognized network service"),
                }
            }
            (macos::NETWORKSETUP, ["-setdnsservers", name, servers @ ..]) => {
                let mut services = self.services.lock();
                let Some(service) = services.iter_mut().find(|service| service.name == *name)
                else {
                    return exit(4, "not a recognized network service");
                };
                service.servers = if servers == [macos::DNS_EMPTY] {
                    Vec::new()
                } else {
                    servers.iter().map(|server| (*server).to_owned()).collect()
                };
                ok("")
            }
            (macos::DEFAULTS, ["read", macos::FIREFOX_DOMAIN]) => {
                let domain = self.firefox.lock();
                if domain.is_empty() {
                    return exit(
                        1,
                        "Domain /Library/Preferences/org.mozilla.firefox does not exist",
                    );
                }
                let mut text = "{\n".to_owned();
                for (name, value) in domain.iter() {
                    let name = if name.contains("__") {
                        format!("\"{name}\"")
                    } else {
                        name.clone()
                    };
                    text.push_str(&format!("    {name} = {value};\n"));
                }
                text.push_str("}\n");
                ok(text)
            }
            (macos::DEFAULTS, ["write", macos::FIREFOX_DOMAIN, name, "-bool", value]) => {
                let stored = if *value == "TRUE" { "1" } else { "0" };
                self.firefox
                    .lock()
                    .insert((*name).to_owned(), stored.to_owned());
                ok("")
            }
            (macos::DEFAULTS, ["delete", macos::FIREFOX_DOMAIN, name]) => {
                match self.firefox.lock().remove(*name) {
                    Some(_) => ok(""),
                    None => exit(1, "Domain or key not found"),
                }
            }
            (macos::DSCACHEUTIL, ["-flushcache"]) | (macos::KILLALL, ["-HUP", "mDNSResponder"]) => {
                ok("")
            }
            _ => panic!("a command the macOS back-end must never run: {program} {args:?}"),
        }
    }
}

/// A network adapter of the pretend Windows machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WinAdapter {
    pub index: u32,
    pub guid: String,
    pub name: String,
    pub up: bool,
    /// Servers set by hand; empty means automatic.
    pub fixed: Vec<String>,
    /// What the network hands out while nothing is set by hand.
    pub automatic: Vec<String>,
}

/// Windows: PowerShell with the three fixed scripts, `reg.exe`, `ipconfig.exe`.
#[derive(Default)]
pub(crate) struct WinMachine {
    pub calls: Mutex<Vec<Call>>,
    pub adapters: Mutex<Vec<WinAdapter>>,
    /// Registry values: `(key, name)` to `(type, data)`.
    pub registry: Mutex<BTreeMap<(String, String), (String, String)>>,
    pub broken: Mutex<HashSet<String>>,
    pub faults: Faults,
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    let after = text.split_once(open).map_or("", |(_, after)| after);
    after.split_once(close).map_or("", |(inside, _)| inside)
}

impl WinMachine {
    fn powershell(&self, script: &str) -> io::Result<Output> {
        let set_prefix = windows::SET_SCRIPT
            .split("{servers}")
            .next()
            .unwrap_or_default();
        let reset_prefix = windows::RESET_SCRIPT
            .split("{indexes}")
            .next()
            .unwrap_or_default();
        let mut adapters = self.adapters.lock();
        if script == windows::FULL_READ_SCRIPT {
            let rows: Vec<serde_json::Value> = adapters
                .iter()
                .filter(|adapter| adapter.up)
                .map(|adapter| {
                    let servers = if adapter.fixed.is_empty() {
                        &adapter.automatic
                    } else {
                        &adapter.fixed
                    };
                    serde_json::json!({ "index": adapter.index, "guid": adapter.guid,
                                        "name": adapter.name, "servers": servers })
                })
                .collect();
            ok(serde_json::Value::from(rows).to_string())
        } else if script.starts_with(set_prefix) {
            let servers: Vec<String> = between(script, "-ServerAddresses (", ")")
                .split(',')
                .map(|server| server.trim_matches('\'').to_owned())
                .collect();
            for adapter in adapters.iter_mut().filter(|adapter| adapter.up) {
                adapter.fixed.clone_from(&servers);
            }
            ok("")
        } else if script.starts_with(reset_prefix) {
            for index in between(script, "@(", ")").split(',') {
                let index: u32 = index.parse().expect("an interface index");
                let adapter = adapters.iter_mut().find(|adapter| adapter.index == index);
                adapter.expect("a known adapter").fixed.clear();
            }
            ok("")
        } else {
            panic!("a script the Windows back-end must never run: {script}")
        }
    }

    fn reg(&self, words: &[&str]) -> io::Result<Output> {
        let mut registry = self.registry.lock();
        match words {
            ["query", key, "/s", "/v", windows::NAME_SERVER] => {
                let v6 = key.contains("Tcpip6");
                let mut text = String::new();
                for adapter in self.adapters.lock().iter() {
                    let servers: Vec<&str> = adapter
                        .fixed
                        .iter()
                        .map(String::as_str)
                        .filter(|server| server.contains(':') == v6)
                        .collect();
                    if v6 && servers.is_empty() {
                        continue;
                    }
                    let full = key.replacen("HKLM", "HKEY_LOCAL_MACHINE", 1);
                    text.push_str(&format!(
                        "\r\n{full}\\{}\r\n    NameServer    REG_SZ    {}\r\n",
                        adapter.guid,
                        servers.join(",")
                    ));
                }
                ok(text)
            }
            ["query", key] => {
                let full = key.replacen("HKLM", "HKEY_LOCAL_MACHINE", 1);
                let mut text = format!("\r\n{full}\r\n");
                let mut any = false;
                for ((at, name), (kind, data)) in registry.iter() {
                    if at == key {
                        any = true;
                        text.push_str(&format!("    {name}    {kind}    {data}\r\n"));
                    }
                }
                if any {
                    ok(text)
                } else {
                    exit(
                        1,
                        "ERROR: The system was unable to find the specified registry key or value.",
                    )
                }
            }
            ["add", key, "/v", name, "/t", kind, "/d", data, "/f"] => {
                let data = if *kind == "REG_DWORD" {
                    format!("0x{:x}", data.parse::<u32>().expect("a number"))
                } else {
                    (*data).to_owned()
                };
                registry.insert(
                    ((*key).to_owned(), (*name).to_owned()),
                    ((*kind).to_owned(), data),
                );
                ok("")
            }
            ["delete", key, "/v", name, "/f"] => {
                match registry.remove(&((*key).to_owned(), (*name).to_owned())) {
                    Some(_) => ok(""),
                    None => exit(
                        1,
                        "ERROR: The system was unable to find the specified registry key or value.",
                    ),
                }
            }
            _ => panic!("a reg.exe command the Windows back-end must never run: {words:?}"),
        }
    }
}

impl Runner for WinMachine {
    fn run(&self, program: &str, args: &[String], env: &[(String, String)]) -> io::Result<Output> {
        let call = Call {
            program: program.to_owned(),
            args: args.to_vec(),
            env: env.to_vec(),
        };
        self.calls.lock().push(call.clone());
        if self.broken.lock().contains(program) || self.faults.trips(&call) {
            return exit(1, "broken");
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        if program == format!("{SYSTEM32}\\WindowsPowerShell\\v1.0\\powershell.exe") {
            match words.as_slice() {
                ["-NoProfile", "-NonInteractive", "-Command", script] => self.powershell(script),
                _ => panic!("PowerShell started another way: {args:?}"),
            }
        } else if program == format!("{SYSTEM32}\\reg.exe") {
            self.reg(&words)
        } else if program == format!("{SYSTEM32}\\ipconfig.exe") && words == ["/flushdns"] {
            ok("")
        } else {
            panic!("a command the Windows back-end must never run: {program} {args:?}")
        }
    }
}

/// Linux: `systemctl` and `resolvectl`.
pub(crate) struct LinuxMachine {
    pub calls: Mutex<Vec<Call>>,
    pub resolved_active: Mutex<bool>,
    pub restarts: Mutex<u32>,
    /// Programs that are not installed.
    pub missing: Mutex<HashSet<String>>,
    pub broken: Mutex<HashSet<String>>,
    pub faults: Faults,
}

impl Default for LinuxMachine {
    fn default() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            resolved_active: Mutex::new(true),
            restarts: Mutex::new(0),
            missing: Mutex::new(HashSet::new()),
            broken: Mutex::new(HashSet::new()),
            faults: Faults::default(),
        }
    }
}

impl Runner for LinuxMachine {
    fn run(&self, program: &str, args: &[String], env: &[(String, String)]) -> io::Result<Output> {
        let call = Call {
            program: program.to_owned(),
            args: args.to_vec(),
            env: env.to_vec(),
        };
        self.calls.lock().push(call.clone());
        if self.missing.lock().contains(program) {
            return Err(not_found());
        }
        if self.broken.lock().contains(program) || self.faults.trips(&call) {
            return exit(1, "broken");
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        match (program, words.as_slice()) {
            ("/usr/bin/systemctl", ["is-active", "systemd-resolved"]) => {
                if *self.resolved_active.lock() {
                    ok("active\n")
                } else {
                    Ok(Output {
                        status: 3,
                        stdout: "inactive\n".into(),
                        stderr: String::new(),
                    })
                }
            }
            ("/usr/bin/systemctl", ["restart", "systemd-resolved"]) => {
                *self.restarts.lock() += 1;
                ok("")
            }
            ("/usr/bin/resolvectl", ["flush-caches"]) => ok(""),
            _ => panic!("a command the Linux back-end must never run: {program} {args:?}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Names, elevation, the rig
// ---------------------------------------------------------------------------------------------

/// What a name resolves to.
type Answer = dyn Fn(&str) -> Vec<String> + Send + Sync;

/// A name lookup that answers from a closure and records what was asked.
pub(crate) struct FakeLookup {
    pub asked: Mutex<Vec<String>>,
    answer: Box<Answer>,
}

impl FakeLookup {
    pub(crate) fn new(answer: impl Fn(&str) -> Vec<String> + Send + Sync + 'static) -> Self {
        Self {
            asked: Mutex::new(Vec::new()),
            answer: Box::new(answer),
        }
    }
}

impl NameLookup for FakeLookup {
    fn lookup(&self, name: &str) -> Vec<String> {
        self.asked.lock().push(name.to_owned());
        (self.answer)(name)
    }
}

pub(crate) fn backend_for(os: Os) -> Box<dyn OsBackend> {
    match os {
        Os::Macos => Box::new(MacosBackend),
        Os::Linux => Box::new(LinuxBackend),
        Os::Windows => Box::new(WindowsBackend::new(SYSTEM32, PROGRAM_FILES)),
    }
}

/// One pretend machine of `os` with its file system, its tools and its name lookup.
#[derive(Clone)]
pub(crate) struct Rig {
    pub os: Os,
    pub fs: Arc<MemFs>,
    pub mac: Arc<MacMachine>,
    pub win: Arc<WinMachine>,
    pub linux: Arc<LinuxMachine>,
    pub names: Arc<FakeLookup>,
    pub pauses: Arc<Mutex<Vec<Duration>>>,
    /// What the next elevations answer instead of running the helper.
    pub elevations: Arc<Mutex<VecDeque<ElevatedExit>>>,
    pub elevated: Arc<Mutex<Vec<Vec<String>>>>,
}

pub(crate) const HOSTS_UNIX: &str = "127.0.0.1\tlocalhost\n::1\tlocalhost\n";
pub(crate) const HOSTS_WINDOWS: &str = "# Copyright (c) 1993-2009 Microsoft Corp.\r\n#\r\n";

impl Rig {
    /// A clean machine: a hosts file without a block, automatic DNS, Chrome and Firefox
    /// installed without policies, no state directory, names resolving.
    pub(crate) fn new(os: Os) -> Self {
        Self::with_names(os, |_| vec!["203.0.113.10".to_owned()])
    }

    pub(crate) fn with_names(
        os: Os,
        answer: impl Fn(&str) -> Vec<String> + Send + Sync + 'static,
    ) -> Self {
        let fs = MemFs::new(os);
        let (mac, win, linux) = (
            MacMachine::default(),
            WinMachine::default(),
            LinuxMachine::default(),
        );
        match os {
            Os::Linux => {
                fs.file("/etc/hosts", HOSTS_UNIX.as_bytes());
                fs.file(
                    "/etc/resolv.conf",
                    b"nameserver 127.0.0.53\noptions edns0 trust-ad\n",
                );
                fs.dir("/var/lib");
                fs.dir("/opt/google/chrome");
                fs.file("/usr/bin/firefox", b"");
            }
            Os::Macos => {
                fs.file("/private/etc/hosts", HOSTS_UNIX.as_bytes());
                fs.dir("/etc");
                fs.link("/etc/hosts", "/private/etc/hosts");
                fs.dir("/Library/Application Support");
                fs.dir("/Applications/Google Chrome.app");
                fs.dir("/Applications/Firefox.app");
                *mac.services.lock() = vec![
                    Service {
                        name: "USB 10/100/1000 LAN".into(),
                        enabled: true,
                        servers: Vec::new(),
                    },
                    Service {
                        name: "Wi-Fi".into(),
                        enabled: true,
                        servers: Vec::new(),
                    },
                    Service {
                        name: "Thunderbolt Bridge".into(),
                        enabled: false,
                        servers: Vec::new(),
                    },
                ];
            }
            Os::Windows => {
                fs.file(
                    &format!("{SYSTEM32}\\drivers\\etc\\hosts"),
                    HOSTS_WINDOWS.as_bytes(),
                );
                fs.dir(PROGRAM_FILES);
                fs.file(
                    &format!("{PROGRAM_FILES}\\Google\\Chrome\\Application\\chrome.exe"),
                    b"",
                );
                fs.file(
                    &format!("{PROGRAM_FILES}\\Mozilla Firefox\\firefox.exe"),
                    b"",
                );
                *win.adapters.lock() = vec![
                    WinAdapter {
                        index: 12,
                        guid: "{11111111-2222-3333-4444-555555555555}".into(),
                        name: "Wi-Fi".into(),
                        up: true,
                        fixed: Vec::new(),
                        automatic: vec!["192.168.0.1".into()],
                    },
                    WinAdapter {
                        index: 7,
                        guid: "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}".into(),
                        name: "Ethernet".into(),
                        up: false,
                        fixed: Vec::new(),
                        automatic: Vec::new(),
                    },
                ];
            }
        }
        Self {
            os,
            fs: Arc::new(fs),
            mac: Arc::new(mac),
            win: Arc::new(win),
            linux: Arc::new(linux),
            names: Arc::new(FakeLookup::new(answer)),
            pauses: Arc::new(Mutex::new(Vec::new())),
            elevations: Arc::new(Mutex::new(VecDeque::new())),
            elevated: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn runner(&self) -> Arc<dyn Runner> {
        match self.os {
            Os::Macos => self.mac.clone(),
            Os::Windows => self.win.clone(),
            Os::Linux => self.linux.clone(),
        }
    }

    pub(crate) fn calls(&self) -> Vec<Call> {
        match self.os {
            Os::Macos => self.mac.calls.lock().clone(),
            Os::Windows => self.win.calls.lock().clone(),
            Os::Linux => self.linux.calls.lock().clone(),
        }
    }

    pub(crate) fn clear_calls(&self) {
        self.mac.calls.lock().clear();
        self.win.calls.lock().clear();
        self.linux.calls.lock().clear();
    }

    /// The failures planted in this machine's tools.
    pub(crate) fn faults(&self) -> &Faults {
        match self.os {
            Os::Macos => &self.mac.faults,
            Os::Windows => &self.win.faults,
            Os::Linux => &self.linux.faults,
        }
    }

    pub(crate) fn hosts_path(&self) -> String {
        match self.os {
            Os::Linux => "/etc/hosts".into(),
            Os::Macos => "/private/etc/hosts".into(),
            Os::Windows => format!("{SYSTEM32}\\drivers\\etc\\hosts"),
        }
    }

    pub(crate) fn hosts_text(&self) -> String {
        self.fs.text(&self.hosts_path()).unwrap_or_default()
    }

    pub(crate) fn state_file(&self) -> String {
        key(&super::state::file(backend_for(self.os).as_ref()))
    }

    /// Runs the helper in this process, as root, and returns its exit code and what it
    /// printed to stdout and stderr.
    pub(crate) fn helper(&self, args: &[&str]) -> (i32, String, String) {
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
        self.helper_owned(&args)
    }

    pub(crate) fn helper_owned(&self, args: &[String]) -> (i32, String, String) {
        let backend = backend_for(self.os);
        let runner = self.runner();
        let pauses = self.pauses.clone();
        let pause = move |time: Duration| pauses.lock().push(time);
        let ports = Ports {
            backend: backend.as_ref(),
            runner: runner.as_ref(),
            fs: self.fs.as_ref(),
            names: self.names.as_ref(),
            pause: &pause,
            now_ms: 1_795_100_000_000,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = helper::run_with(&ports, Path::new(EXE), args, &mut out, &mut err);
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).expect("UTF-8 output");
        (code, text(out), text(err))
    }

    /// `apply` with every tier opened the way a tester opens them.
    pub(crate) fn apply(&self, plan: &SystemPlan) -> i32 {
        let encoded = plan::encode_plan(plan);
        self.helper(&["apply", "--plan-b64", &encoded, "--unverified", ALL_TIERS])
            .0
    }

    pub(crate) fn remove(&self, owner: &str, scope: &str) -> i32 {
        self.helper(&["remove", "--owner", owner, "--scope", scope])
            .0
    }

    /// The full unprivileged look, canary and probe included.
    pub(crate) fn observe(&self) -> SystemObservation {
        self.shield(ProductId::Ubiqx, all_open(self.os))
            .observe(ObserveOpts {
                full: true,
                canary: true,
                probe: true,
            })
            .expect("an observation")
    }

    /// A shield over this machine whose elevations run the helper in this process.
    pub(crate) fn shield(&self, owner: ProductId, gate: Gate) -> NativeSystemShield {
        NativeSystemShield::new(
            backend_for(self.os),
            self.runner(),
            self.fs.clone(),
            Arc::new(self.clone()),
            self.names.clone(),
            PathBuf::from(EXE),
            owner,
            gate,
        )
    }

    /// How `plan`'s owner would judge the machine as it is now.
    pub(crate) fn assess(&self, plan: &SystemPlan) -> SystemAssessment {
        let mut config = NofapConfig::default();
        config.system.hosts = plan.hosts;
        config.system.resolver = plan.resolver;
        config.system.policies = plan.policies;
        plan::assess(
            &plan::AssessInput {
                config: &config,
                plan,
                os: self.os,
                gate: &all_open(self.os),
                hosts_tier: list::hosts_tier(),
                probe_fail_streak: 0,
            },
            &self.observe(),
        )
    }
}

impl Elevator for Rig {
    fn run_elevated(&self, exe: &Path, args: &[String]) -> ElevatedExit {
        assert_eq!(exe, Path::new(EXE));
        self.elevated.lock().push(args.to_vec());
        if let Some(scripted) = self.elevations.lock().pop_front() {
            return scripted;
        }
        ElevatedExit::Exited(self.helper_owned(args).0)
    }
}

/// The gate of `os` with every tier opened by the tester's override.
pub(crate) fn all_open(os: Os) -> Gate {
    Gate::new(os, Tier::ALL.to_vec())
}

pub(crate) fn plan_of(owner: ProductId, extra: &[&str]) -> SystemPlan {
    SystemPlan {
        v: CONFIG_VERSION,
        owner,
        hosts: true,
        extra: extra.iter().map(|name| (*name).to_owned()).collect(),
        resolver: Some(CF),
        policies: true,
    }
}

/// The four addresses of the family resolver.
pub(crate) fn cf_servers() -> Vec<String> {
    resolvers::lookup(CF)
        .addresses()
        .iter()
        .map(|address| (*address).to_owned())
        .collect()
}

/// Base64url without padding of `text`: what `--plan-b64` carries, for plans a test
/// writes by hand.
pub(crate) fn b64(text: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::new();
    for chunk in text.as_bytes().chunks(3) {
        let bits = chunk
            .iter()
            .fold(0u32, |bits, byte| (bits << 8) | u32::from(*byte))
            << (8 * (3 - chunk.len()));
        for position in 0..=chunk.len() {
            let index = (bits >> (18 - 6 * position)) & 0x3F;
            encoded.push(char::from(ALPHABET[index as usize]));
        }
    }
    encoded
}

/// A tier the gate table keeps closed on `os`, if one is left. Tests about closed tiers
/// ask here, so they keep their meaning when an entry of the table is opened.
pub(crate) fn closed_tier(os: Os) -> Option<Tier> {
    Tier::ALL
        .into_iter()
        .find(|tier| !ubiqx_nofap::gate::tier_enabled(os, *tier))
}

/// The tiers of `named` that only a tester's override opens on `os`, as `--unverified`
/// carries them; `None` when the table opens them all.
pub(crate) fn unverified_arg(os: Os, named: &[Tier]) -> Option<String> {
    let names: Vec<&str> = Tier::ALL
        .into_iter()
        .filter(|tier| named.contains(tier) && !ubiqx_nofap::gate::tier_enabled(os, *tier))
        .map(Tier::as_str)
        .collect();
    (!names.is_empty()).then(|| names.join(","))
}

/// A plan that asks for `tier` and nothing else.
pub(crate) fn plan_for(owner: ProductId, tier: Tier) -> SystemPlan {
    SystemPlan {
        v: CONFIG_VERSION,
        owner,
        hosts: tier == Tier::Hosts,
        extra: Vec::new(),
        resolver: (tier == Tier::Resolver).then_some(CF),
        policies: tier == Tier::Policies,
    }
}
