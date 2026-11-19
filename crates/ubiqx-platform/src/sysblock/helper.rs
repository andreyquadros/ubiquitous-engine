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
//! | `status` | none | prints `SystemObservation` as JSON |
//! | `elevate-argv --plan-b64 <b64>` | none | prints the elevation command line as a JSON array and runs nothing |
//! | `apply --plan-b64 <b64> [--unverified <tiers>]` | admin | validates, records the owner's wants, applies the union of all owners' wants |
//! | `remove --owner <ubiqx\|shield\|all> --scope <all\|resolver>` | admin | drops that owner's wants, re-applies what other owners still want, removes what nobody wants |
//!
//! Exit codes are `ubiqx_nofap::plan::EXIT_*`.
//!
//! Seeded by the foundation unit: the entry point and the signature of [`run`].
#![allow(unused_variables)]

/// The first argument that turns a start of the app into a helper run. Process arguments
/// are visible to other accounts, so it carries the machine id, not the display name.
pub const HELPER_FLAG: &str = "--shield-helper";

/// Runs the helper and exits the process when the first argument (after the program name)
/// is [`HELPER_FLAG`]; returns at once otherwise.
pub fn main_if_requested(args: impl IntoIterator<Item = String>) {
    let args: Vec<String> = args.into_iter().collect();
    if args.get(1).map(String::as_str) != Some(HELPER_FLAG) {
        return;
    }
    std::process::exit(run(&args[2..]));
}

/// Runs one verb with the real system ports. `args` starts at the verb. Returns the exit
/// code.
pub fn run(args: &[String]) -> i32 {
    todo!()
}
