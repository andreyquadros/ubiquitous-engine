//! The helper of the system layer as a small binary that links no GUI library: the same
//! code path the two apps take when they are started with `--shield-helper`. The smoke
//! job and the manual removal path use it:
//!
//! ```text
//! ubiqx-shield-helper status
//! sudo ubiqx-shield-helper remove --owner all --scope all
//! ```
//!
//! A leading `--shield-helper` is accepted and skipped, so the elevation command that
//! `elevate-argv` prints for this binary starts it the way it starts the apps.

use ubiqx_platform::sysblock::helper;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(helper::run(helper::after_flag(&args)));
}
