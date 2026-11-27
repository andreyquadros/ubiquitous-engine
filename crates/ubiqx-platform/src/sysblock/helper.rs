//! The privileged helper: the app's own executable re-run as `--shield-helper <verb>`.
//!
//! [`main_if_requested`] is the first statement of both shells' `main` and the whole body
//! of the small binary `ubiqx-shield-helper`. When the first argument is [`HELPER_FLAG`] it
//! does its work and exits before Tauri, the window server or the database are touched.
//!
//! Verbs (nothing else is accepted; there is no "run this"):
//!
//! | Verb | Privilege | Does |
//! |---|---|---|
//! | `status` | none | prints `SystemObservation` as JSON, with the canary's answer while the block is in the hosts file |
//! | `elevate-argv --plan-b64 <b64>` | none | prints the elevation command line as a JSON array and runs nothing |
//! | `apply --plan-b64 <b64> [--unverified <tiers>]` | admin | validates, records the owner's wants, applies the union of all owners' wants. `--plan-b64 -` reads the plan from standard input (one line), which is how the app hands it over on Linux |
//! | `remove --owner <ubiqx\|shield\|all> --scope <all\|resolver>` | admin | `all`: drops that owner's wants, re-applies what other owners still want, removes what nobody wants; `resolver`: takes the resolver off for every owner (DNS is one setting of the machine) and leaves the rest |
//!
//! Exit codes are `ubiqx_nofap::plan::EXIT_*`. This is the privileged side: it trusts
//! nothing it is given and nothing it reads back. The plan is validated again here, the
//! state file is used only from a directory nobody else can write to, every program is an
//! absolute path, and no value from an argument or a file becomes a path, an address or a
//! command.

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use ubiqx_core::nofap::RemoveScope;
use ubiqx_core::ProductId;
use ubiqx_nofap::gate::{self, Gate};
use ubiqx_nofap::hosts::CANARY_NAME;
use ubiqx_nofap::matcher;
use ubiqx_nofap::plan::{self, EXIT_APPLIED, EXIT_FAILED, EXIT_INVALID_PLAN};

use super::converge::converge;
pub use super::converge::Ports;
use super::{elevate, native_backend, observe, state, sys, RunLockHeld};

/// The first argument that turns a start of the app into a helper run. Process arguments
/// are visible to other accounts, so it carries the machine id, not the display name.
pub const HELPER_FLAG: &str = "--shield-helper";

pub const VERB_STATUS: &str = "status";
pub const VERB_ELEVATE_ARGV: &str = "elevate-argv";
pub const VERB_APPLY: &str = "apply";
pub const VERB_REMOVE: &str = "remove";
pub const OPT_PLAN: &str = "--plan-b64";
/// `--plan-b64 -`: the plan comes on standard input. On Linux the app hands it over that
/// way, because `pkexec` writes every command line it authorises to the system journal
/// and the plan holds the sites the user typed.
pub const PLAN_ON_STDIN: &str = "-";
pub const OPT_UNVERIFIED: &str = "--unverified";
pub const OPT_OWNER: &str = "--owner";
pub const OPT_SCOPE: &str = "--scope";
/// `--owner all`: every owner, for the uninstall and manual removal paths.
pub const OWNER_ALL: &str = "all";
pub const USAGE: &str = "usage: --shield-helper status | elevate-argv --plan-b64 <plan> | \
apply --plan-b64 <plan> [--unverified <tiers>] | remove --owner <ubiqx|shield|all> --scope <all|resolver>";

/// Runs the helper and exits the process when the first argument (after the program name)
/// is [`HELPER_FLAG`]; returns at once otherwise.
pub fn main_if_requested(args: impl IntoIterator<Item = String>) {
    let args: Vec<String> = args.into_iter().collect();
    if args.get(1).map(String::as_str) != Some(HELPER_FLAG) {
        return;
    }
    std::process::exit(run(&args[2..]));
}

/// The arguments of a helper run without the program name and without a leading
/// [`HELPER_FLAG`]. The small helper binary is started both ways: with the verb directly
/// (by hand, by an uninstall hook) and, by the elevation command `elevate-argv` prints,
/// with the flag in front like the apps.
pub fn after_flag(args: &[String]) -> &[String] {
    match args.split_first() {
        Some((first, rest)) if first == HELPER_FLAG => rest,
        _ => args,
    }
}

/// `args` with the plan read from `input` in place of [`PLAN_ON_STDIN`]; `args` as they
/// are when they do not ask for that (nothing is read then). `None` when the input is not
/// one line of at most `HELPER_ARG_MAX` characters (one line break at the end is allowed).
/// What is read is only text here: it is checked like an argument afterwards.
pub fn plan_from_stdin(args: &[String], input: &mut dyn Read) -> Option<Vec<String>> {
    let Some(at) = args
        .windows(2)
        .position(|pair| pair[0] == OPT_PLAN && pair[1] == PLAN_ON_STDIN)
    else {
        return Some(args.to_vec());
    };
    let mut bytes = Vec::new();
    let most = u64::try_from(plan::HELPER_ARG_MAX + 2).unwrap_or(u64::MAX);
    input.take(most).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let line = text.strip_suffix('\n').unwrap_or(&text);
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.is_empty() || line.len() > plan::HELPER_ARG_MAX || line.contains(['\n', '\r']) {
        return None;
    }
    let mut args = args.to_vec();
    args[at + 1] = line.to_owned();
    Some(args)
}

/// Runs one verb with the real system ports. `args` starts at the verb. Returns the exit
/// code.
pub fn run(args: &[String]) -> i32 {
    // Before anything looks a name up: see `sys::pin_system_environment`.
    sys::pin_system_environment();
    let Some(args) = plan_from_stdin(args, &mut std::io::stdin().lock()) else {
        eprintln!("ubiqx-shield: no plan on standard input");
        return EXIT_INVALID_PLAN;
    };
    let args = args.as_slice();
    let (Some(backend), Ok(exe)) = (native_backend(), std::env::current_exe()) else {
        eprintln!("ubiqx-shield: this system has no system layer");
        return EXIT_FAILED;
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        });
    let ports = Ports {
        backend: backend.as_ref(),
        runner: &sys::SystemRunner,
        fs: &sys::SystemFs,
        names: &sys::SystemLookup,
        lock: &sys::SystemRunLock,
        pause: &|time: Duration| std::thread::sleep(time),
        now_ms,
    };
    let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
    run_with(&ports, &exe, args, &mut out, &mut err)
}

/// One parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    Status,
    ElevateArgv {
        plan: String,
        unverified: Option<String>,
    },
    Apply {
        plan: String,
        unverified: Option<String>,
    },
    /// `owner` is `None` for `all`.
    Remove {
        owner: Option<ProductId>,
        scope: RemoveScope,
    },
}

/// The values of the options `allowed` in `args`: each option at most once, each with a
/// value, nothing else on the line.
fn options<'a>(args: &'a [String], allowed: &[&str]) -> Option<Vec<Option<&'a str>>> {
    let mut found = vec![None; allowed.len()];
    let mut rest = args.iter();
    while let Some(name) = rest.next() {
        let slot = allowed.iter().position(|option| option == name)?;
        let value = rest.next()?;
        if found[slot].replace(value.as_str()).is_some() {
            return None;
        }
    }
    Some(found)
}

/// Parses the arguments after [`HELPER_FLAG`]. The set of verbs and options is closed: an
/// unknown word, a repeated option or a missing value is refused.
pub fn parse(args: &[String]) -> Option<Verb> {
    let (verb, rest) = args.split_first()?;
    let owned = |value: Option<&str>| value.map(str::to_owned);
    match verb.as_str() {
        VERB_STATUS if rest.is_empty() => Some(Verb::Status),
        VERB_ELEVATE_ARGV | VERB_APPLY => {
            let found = options(rest, &[OPT_PLAN, OPT_UNVERIFIED])?;
            let (plan, unverified) = (owned(found[0])?, owned(found[1]));
            Some(if verb == VERB_APPLY {
                Verb::Apply { plan, unverified }
            } else {
                Verb::ElevateArgv { plan, unverified }
            })
        }
        VERB_REMOVE => {
            let found = options(rest, &[OPT_OWNER, OPT_SCOPE])?;
            let owner = match found[0]? {
                OWNER_ALL => None,
                id => Some(ProductId::parse(id)?),
            };
            let scope = RemoveScope::parse(found[1]?)?;
            Some(Verb::Remove { owner, scope })
        }
        _ => None,
    }
}

/// The arguments of an `apply` run for an already encoded plan.
pub fn apply_args(plan_b64: &str, unverified: Option<&str>) -> Vec<String> {
    let mut args = vec![
        VERB_APPLY.to_owned(),
        OPT_PLAN.to_owned(),
        plan_b64.to_owned(),
    ];
    if let Some(tiers) = unverified {
        args.extend([OPT_UNVERIFIED.to_owned(), tiers.to_owned()]);
    }
    args
}

/// The arguments of a `remove` run.
pub fn remove_args(owner: ProductId, scope: RemoveScope) -> Vec<String> {
    [
        VERB_REMOVE,
        OPT_OWNER,
        owner.as_str(),
        OPT_SCOPE,
        scope.as_str(),
    ]
    .map(str::to_owned)
    .to_vec()
}

/// How long `apply` and `remove` wait, in seconds, for another helper run to finish. A run
/// takes seconds; the wait covers a slow one (PowerShell, the probe after a DNS change).
pub const LOCK_WAIT_SECS: u64 = 120;

/// Waits for the lock between helper runs. `Err` is the exit code of an `apply` that did
/// not get it. A removal goes on without it once the wait is over, and so does any run on a
/// machine where the lock cannot be taken at all: the way out is never closed, and the
/// lock only guards against two runs overlapping.
fn take_lock(
    ports: &Ports<'_>,
    removal: bool,
    err: &mut dyn Write,
) -> Result<Option<RunLockHeld>, i32> {
    for attempt in 0..LOCK_WAIT_SECS {
        match ports.lock.try_lock(ports.backend) {
            Ok(Some(held)) => return Ok(Some(held)),
            Ok(None) if attempt + 1 < LOCK_WAIT_SECS => (ports.pause)(Duration::from_secs(1)),
            Ok(None) => {}
            Err(_) => return Ok(None),
        }
    }
    if removal {
        let _ = writeln!(
            err,
            "ubiqx-shield: another helper run is still going; removing anyway"
        );
        Ok(None)
    } else {
        let _ = writeln!(err, "ubiqx-shield: another helper run is still going");
        Err(EXIT_FAILED)
    }
}

/// Runs one verb over the given ports; `exe` is the executable an elevation would re-run.
/// What the verb prints goes to `out`, why it refused goes to `err`.
pub fn run_with(
    ports: &Ports<'_>,
    exe: &Path,
    args: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let backend = ports.backend;
    let os = backend.os();
    let Some(verb) = parse(args) else {
        let _ = writeln!(err, "{USAGE}");
        return EXIT_INVALID_PLAN;
    };
    match verb {
        Verb::Status => {
            let mut observed = observe::observe(backend, ports.runner, ports.fs, true);
            // The canary is looked up only while the block is in the file, so the reserved
            // name never leaves a machine without it. It is what the app's own check reads.
            if observe::block_present(observed.hosts.text.as_deref()) {
                observed.canary = Some(ports.names.lookup_blocked(CANARY_NAME));
            }
            match serde_json::to_string_pretty(&observed) {
                Ok(json) => {
                    let _ = writeln!(out, "{json}");
                    EXIT_APPLIED
                }
                Err(_) => EXIT_FAILED,
            }
        }
        Verb::ElevateArgv { plan, unverified } => {
            if let Err(refused) = plan::decode_plan(&plan) {
                let _ = writeln!(err, "ubiqx-shield: {refused}");
                return refused.exit_code();
            }
            // The tier list is written again from what was understood of it.
            let unverified = Gate::new(os, gate::parse_override(unverified.as_deref()));
            let args = apply_args(&plan, unverified.unverified_arg().as_deref());
            let argv = elevate::argv(os, &backend.elevation_tool(), exe, &args);
            let _ = writeln!(out, "{}", serde_json::Value::from(argv));
            EXIT_APPLIED
        }
        Verb::Apply { plan, unverified } => {
            let gate = Gate::new(os, gate::parse_override(unverified.as_deref()));
            let checked = plan::decode_plan(&plan).and_then(|plan| {
                plan::validate(&plan, matcher::never_block(), &gate).map(|()| plan)
            });
            let plan = match checked {
                Ok(plan) => plan,
                Err(refused) => {
                    let _ = writeln!(err, "ubiqx-shield: {refused}");
                    return refused.exit_code();
                }
            };
            let _lock = match take_lock(ports, false, err) {
                Ok(held) => held,
                Err(code) => return code,
            };
            let Ok(previous) = state::open(backend, ports.fs) else {
                let _ = writeln!(err, "ubiqx-shield: the state directory cannot be used");
                return EXIT_FAILED;
            };
            let owners = plan::owners_after(previous.as_ref(), &plan);
            converge(ports, owners, previous.as_ref(), Some(plan.owner), &gate)
        }
        Verb::Remove { owner, scope } => {
            let _lock = match take_lock(ports, true, err) {
                Ok(held) => held,
                Err(code) => return code,
            };
            let Ok(previous) = state::open(backend, ports.fs) else {
                let _ = writeln!(err, "ubiqx-shield: the state directory cannot be used");
                return EXIT_FAILED;
            };
            let mut owners = previous
                .as_ref()
                .map(|state| state.owners.clone())
                .unwrap_or_default();
            let named = |id: &ProductId| owner.is_none_or(|owner| owner == *id);
            match scope {
                RemoveScope::All => owners.retain(|id, _| !named(id)),
                // "Restaurar minha rede": DNS is one setting of the machine, so it goes
                // back to automatic for every owner, whoever asks. Taken off only for the
                // asking owner, it would stay in place for the other product and the run
                // would still report the network as restored. The other product then
                // shows its resolver as not applied, with its own "Religar".
                RemoveScope::Resolver => {
                    for wants in owners.values_mut() {
                        wants.resolver = None;
                    }
                }
            }
            // A removal opens nothing: only the table's gate applies to what stays.
            let gate = Gate::new(os, Vec::new());
            converge(ports, owners, previous.as_ref(), None, &gate)
        }
    }
}
