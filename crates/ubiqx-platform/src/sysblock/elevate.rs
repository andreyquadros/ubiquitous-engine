//! The elevation command lines: how this executable is re-run as `--shield-helper <verb>`
//! with administrator rights through each operating system's own prompt, and what the
//! exit of that command means. Pure builders, plus [`RunnerElevator`], which runs the
//! built command through a [`Runner`].
//!
//! | OS | Command | Declined or unavailable |
//! |---|---|---|
//! | macOS | `osascript -e 'do shell script "<exe> --shield-helper …" with administrator privileges'` | error -128 is `Declined` |
//! | Windows | `powershell.exe -NoProfile -NonInteractive -Command "try { $p = Start-Process … -Verb RunAs -Wait -PassThru …; exit $p.ExitCode } catch { exit 1223 }"` | 1223 is `Declined` |
//! | Linux | `pkexec <exe> --shield-helper …`, the plan on standard input (`--plan-b64 -`) | 126 is `Declined`; 127 or no `pkexec` is `no_polkit_agent`; an AppImage is `appimage` without trying |

use std::io;
use std::path::Path;
use std::sync::Arc;

use ubiqx_core::nofap::{reason, Os};

use super::helper::{HELPER_FLAG, OPT_PLAN, PLAN_ON_STDIN};
use super::{ElevatedExit, Elevator, Output, Runner};

/// AppleScript's "user canceled" error number.
pub const OSASCRIPT_CANCELLED: i32 = -128;
/// Windows `ERROR_CANCELLED`: the UAC prompt was declined.
pub const WINDOWS_CANCELLED: i32 = 1223;
/// `pkexec`: the authentication dialog was dismissed.
pub const PKEXEC_DISMISSED: i32 = 126;
/// `pkexec`: not authorised, no agent, or another error of its own.
pub const PKEXEC_FAILED: i32 = 127;

/// `text` as one word of a POSIX shell: single-quoted, with every single quote written as
/// `'\''`. Any character is safe inside, including `$`, backslashes and non-ASCII.
pub fn sh_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// `text` as an AppleScript string literal: double-quoted, with backslashes and double
/// quotes escaped. Everything else, including non-ASCII, stands for itself.
pub fn applescript_quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `text` as a PowerShell single-quoted string. PowerShell ends such a string at an ASCII
/// apostrophe and at the typographic single quotes U+2018 to U+201B alike, so each of
/// them is doubled; nothing else is special inside (`$` and backticks are literal).
pub fn ps_single_quote(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('\'');
    for c in text.chars() {
        quoted.push(c);
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            quoted.push(c);
        }
    }
    quoted.push('\'');
    quoted
}

/// Whether `arg` may be handed to the helper through an elevation command. The helper's
/// own arguments are verbs, option names, ids, tier lists and base64url text, so the
/// alphabet is closed: letters, digits, `-`, `_`, `,` and `=`. PowerShell's
/// `-ArgumentList` joins its items with blanks and no quoting, which is safe for exactly
/// such words.
pub fn is_helper_arg(arg: &str) -> bool {
    !arg.is_empty()
        && arg
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b',' | b'='))
}

/// macOS: `osascript -e 'do shell script "<exe> --shield-helper <args>" with administrator
/// privileges'`. The executable path and every argument go through [`sh_quote`], and the
/// whole shell command through [`applescript_quote`].
pub fn osascript_argv(osascript: &str, exe: &Path, args: &[String]) -> Vec<String> {
    let mut command = sh_quote(&exe.to_string_lossy());
    for arg in std::iter::once(HELPER_FLAG).chain(args.iter().map(String::as_str)) {
        command.push(' ');
        command.push_str(&sh_quote(arg));
    }
    let script = format!(
        "do shell script {} with administrator privileges",
        applescript_quote(&command)
    );
    vec![osascript.to_owned(), "-e".to_owned(), script]
}

/// Windows: PowerShell starts the executable through `Start-Process -Verb RunAs` (the UAC
/// prompt), waits and passes its exit code on; any failure to start, a declined prompt
/// included, exits [`WINDOWS_CANCELLED`]. The path goes through [`ps_single_quote`].
pub fn powershell_argv(powershell: &str, exe: &Path, args: &[String]) -> Vec<String> {
    let list: Vec<String> = std::iter::once(HELPER_FLAG)
        .chain(args.iter().map(String::as_str))
        .map(ps_single_quote)
        .collect();
    let script = format!(
        "try {{ $p = Start-Process -FilePath {} -ArgumentList {} -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode }} catch {{ exit {WINDOWS_CANCELLED} }}",
        ps_single_quote(&exe.to_string_lossy()),
        list.join(","),
    );
    vec![
        powershell.to_owned(),
        "-NoProfile".to_owned(),
        "-NonInteractive".to_owned(),
        "-Command".to_owned(),
        script,
    ]
}

/// Linux: `pkexec <exe> --shield-helper <args>`. No shell is involved.
pub fn pkexec_argv(pkexec: &str, exe: &Path, args: &[String]) -> Vec<String> {
    let mut argv = vec![
        pkexec.to_owned(),
        exe.to_string_lossy().into_owned(),
        HELPER_FLAG.to_owned(),
    ];
    argv.extend(args.iter().cloned());
    argv
}

/// Linux: `args` with the plan taken out and [`PLAN_ON_STDIN`] in its place, and the plan,
/// which then goes to the helper on its standard input. `pkexec` writes every command line
/// it authorises to the system journal (`journalctl -t pkexec`), where it stays after the
/// sites are taken off the list and after the app is removed, and other accounts can read
/// a running process's arguments; the plan holds the sites the user typed. `pkexec` hands
/// its standard input to the program it starts. The macOS and Windows prompts hand the
/// helper none, so there the plan stays an argument (`None`).
pub fn plan_to_stdin(os: Os, args: &[String]) -> (Vec<String>, Option<String>) {
    let at = args.windows(2).position(|pair| pair[0] == OPT_PLAN);
    match at {
        Some(at) if os == Os::Linux && args[at + 1] != PLAN_ON_STDIN => {
            let mut moved = args.to_vec();
            let plan = std::mem::replace(&mut moved[at + 1], PLAN_ON_STDIN.to_owned());
            (moved, Some(plan))
        }
        _ => (args.to_vec(), None),
    }
}

/// The elevation command line of `os`: the program first, then its arguments. `tool` is
/// the absolute path of `osascript`, PowerShell or `pkexec`.
pub fn argv(os: Os, tool: &str, exe: &Path, args: &[String]) -> Vec<String> {
    match os {
        Os::Macos => osascript_argv(tool, exe, args),
        Os::Windows => powershell_argv(tool, exe, args),
        Os::Linux => pkexec_argv(tool, exe, args),
    }
}

/// The number AppleScript prints in brackets at the end of an error line
/// (`execution error: User canceled. (-128)`). `do shell script` reports the exit status
/// of a failed command the same way.
fn osascript_error_number(stderr: &str) -> Option<i32> {
    let line = stderr.trim_end();
    let inner = line.strip_suffix(')')?;
    let (_, number) = inner.rsplit_once('(')?;
    number.trim().parse().ok()
}

/// What the end of an elevation command means. Language-independent: only exit codes and
/// AppleScript's error number are read.
pub fn exit_of(os: Os, ran: &io::Result<Output>) -> ElevatedExit {
    let failed = || ElevatedExit::Unavailable(reason::FAILED.to_owned());
    match (os, ran) {
        (Os::Linux, Err(_)) => ElevatedExit::Unavailable(reason::NO_POLKIT_AGENT.to_owned()),
        (Os::Macos | Os::Windows, Err(_)) => failed(),
        (_, Ok(output)) if output.success() => ElevatedExit::Exited(0),
        (Os::Macos, Ok(output)) => match osascript_error_number(&output.stderr) {
            Some(OSASCRIPT_CANCELLED) => ElevatedExit::Declined,
            Some(code) if (1..=255).contains(&code) => ElevatedExit::Exited(code),
            Some(_) | None => failed(),
        },
        (Os::Windows, Ok(output)) if output.status == WINDOWS_CANCELLED => ElevatedExit::Declined,
        (Os::Linux, Ok(output)) if output.status == PKEXEC_DISMISSED => ElevatedExit::Declined,
        (Os::Linux, Ok(output)) if output.status == PKEXEC_FAILED => {
            ElevatedExit::Unavailable(reason::NO_POLKIT_AGENT.to_owned())
        }
        (Os::Windows | Os::Linux, Ok(output)) => ElevatedExit::Exited(output.status),
    }
}

/// The [`Elevator`] of a real machine: builds the command of its OS and runs it through
/// the [`Runner`], with the environment the back-end builds for every child.
pub struct RunnerElevator {
    os: Os,
    tool: String,
    env: Vec<(String, String)>,
    runner: Arc<dyn Runner>,
    appimage: bool,
}

impl RunnerElevator {
    /// `tool` is `OsBackend::elevation_tool`, `env` is `OsBackend::child_env`. `appimage`:
    /// the app runs from an AppImage, whose mount root cannot read, so nothing is tried.
    pub fn new(
        os: Os,
        tool: String,
        env: Vec<(String, String)>,
        runner: Arc<dyn Runner>,
        appimage: bool,
    ) -> Self {
        Self {
            os,
            tool,
            env,
            runner,
            appimage,
        }
    }
}

impl Elevator for RunnerElevator {
    fn run_elevated(&self, exe: &Path, args: &[String]) -> ElevatedExit {
        if self.appimage {
            return ElevatedExit::Unavailable(reason::APPIMAGE.to_owned());
        }
        if !args.iter().all(|arg| is_helper_arg(arg)) {
            return ElevatedExit::Unavailable(reason::FAILED.to_owned());
        }
        let (args, plan) = plan_to_stdin(self.os, args);
        let argv = argv(self.os, &self.tool, exe, &args);
        let ran = match plan {
            Some(plan) => {
                self.runner
                    .run_with_input(&argv[0], &argv[1..], &self.env, plan.as_bytes())
            }
            None => self.runner.run(&argv[0], &argv[1..], &self.env),
        };
        exit_of(self.os, &ran)
    }
}
