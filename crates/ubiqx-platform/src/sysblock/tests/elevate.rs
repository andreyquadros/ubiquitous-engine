//! The elevation command lines: quoting, exact argv, what an exit means.

use std::io;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use ubiqx_core::nofap::Os;

use crate::sysblock::elevate::{
    applescript_quote, argv, exit_of, is_helper_arg, osascript_argv, pkexec_argv, powershell_argv,
    ps_single_quote, sh_quote, RunnerElevator,
};
use crate::sysblock::{ElevatedExit, Elevator, Output, Runner};

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| (*item).to_owned()).collect()
}

fn exited(status: i32, stderr: &str) -> io::Result<Output> {
    Ok(Output {
        status,
        stdout: String::new(),
        stderr: stderr.to_owned(),
    })
}

fn unavailable(code: &str) -> ElevatedExit {
    ElevatedExit::Unavailable(code.to_owned())
}

#[test]
fn sh_quote_makes_any_text_one_shell_word() {
    assert_eq!(sh_quote("plain"), "'plain'");
    assert_eq!(sh_quote("with space"), "'with space'");
    assert_eq!(sh_quote("it's"), "'it'\\''s'");
    assert_eq!(sh_quote("$HOME `id` \"x\""), "'$HOME `id` \"x\"'");
    assert_eq!(sh_quote("back\\slash"), "'back\\slash'");
    assert_eq!(sh_quote("Aplicações/ação.app"), "'Aplicações/ação.app'");
    assert_eq!(sh_quote(""), "''");
    assert_eq!(sh_quote("'; rm -rf /; '"), "''\\''; rm -rf /; '\\'''");
}

#[test]
fn applescript_quote_escapes_backslashes_and_double_quotes_only() {
    assert_eq!(applescript_quote("plain"), "\"plain\"");
    assert_eq!(applescript_quote("a \"b\" c"), "\"a \\\"b\\\" c\"");
    assert_eq!(applescript_quote("back\\slash"), "\"back\\\\slash\"");
    assert_eq!(applescript_quote("\\\""), "\"\\\\\\\"\"");
    assert_eq!(applescript_quote("it's $x"), "\"it's $x\"");
    assert_eq!(applescript_quote("ação 日本"), "\"ação 日本\"");
}

#[test]
fn ps_single_quote_doubles_every_kind_of_single_quote() {
    assert_eq!(ps_single_quote("plain"), "'plain'");
    assert_eq!(
        ps_single_quote("C:\\Program Files\\App"),
        "'C:\\Program Files\\App'"
    );
    assert_eq!(ps_single_quote("it's"), "'it''s'");
    assert_eq!(ps_single_quote("$env:X `n \"q\""), "'$env:X `n \"q\"'");
    assert_eq!(ps_single_quote("João’s"), "'João’’s'");
    assert_eq!(
        ps_single_quote("\u{2018}a\u{201A}b\u{201B}"),
        "'\u{2018}\u{2018}a\u{201A}\u{201A}b\u{201B}\u{201B}'"
    );
    assert_eq!(ps_single_quote("'; calc; '"), "'''; calc; '''");
}

#[test]
fn the_exact_argv_of_each_elevation() {
    let args = strings(&["apply", "--plan-b64", "eyJ2IjoxfQ", "--unverified", "hosts"]);
    assert_eq!(
        osascript_argv("/usr/bin/osascript", Path::new("/Applications/ubiqX.app/Contents/MacOS/ubiqx"), &args),
        strings(&[
            "/usr/bin/osascript",
            "-e",
            "do shell script \"'/Applications/ubiqX.app/Contents/MacOS/ubiqx' '--shield-helper' 'apply' '--plan-b64' 'eyJ2IjoxfQ' '--unverified' 'hosts'\" with administrator privileges",
        ])
    );
    assert_eq!(
        powershell_argv(
            "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
            Path::new("C:\\Program Files\\ubiqX\\ubiqx.exe"),
            &args
        ),
        strings(&[
            "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "try { $p = Start-Process -FilePath 'C:\\Program Files\\ubiqX\\ubiqx.exe' -ArgumentList '--shield-helper','apply','--plan-b64','eyJ2IjoxfQ','--unverified','hosts' -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode } catch { exit 1223 }",
        ])
    );
    assert_eq!(
        pkexec_argv("/usr/bin/pkexec", Path::new("/usr/bin/ubiqx"), &args),
        strings(&[
            "/usr/bin/pkexec",
            "/usr/bin/ubiqx",
            "--shield-helper",
            "apply",
            "--plan-b64",
            "eyJ2IjoxfQ",
            "--unverified",
            "hosts",
        ])
    );
    // `argv` picks the builder of the system.
    let exe = Path::new("/x");
    assert_eq!(
        argv(Os::Macos, "t", exe, &args),
        osascript_argv("t", exe, &args)
    );
    assert_eq!(
        argv(Os::Windows, "t", exe, &args),
        powershell_argv("t", exe, &args)
    );
    assert_eq!(
        argv(Os::Linux, "t", exe, &args),
        pkexec_argv("t", exe, &args)
    );
}

#[test]
fn a_hostile_executable_path_stays_one_word_in_every_command() {
    let remove = strings(&["remove", "--owner", "all", "--scope", "all"]);
    // macOS: quoted for the shell, then for AppleScript.
    let exe = Path::new("/Users/ana/My \"Apps\"/it's $HOME\\x/ação.app/Contents/MacOS/app");
    let script = &osascript_argv("/usr/bin/osascript", exe, &remove)[2];
    assert_eq!(
        script,
        "do shell script \"'/Users/ana/My \\\"Apps\\\"/it'\\\\''s $HOME\\\\x/ação.app/Contents/MacOS/app' '--shield-helper' 'remove' '--owner' 'all' '--scope' 'all'\" with administrator privileges"
    );
    // Windows: one single-quoted PowerShell string, apostrophes doubled.
    let exe = Path::new("C:\\Users\\D'Ávila’s PC\\$app\\`x`\\ubiqx.exe");
    let script = &powershell_argv("ps.exe", exe, &remove)[4];
    assert!(script
        .contains("-FilePath 'C:\\Users\\D''Ávila’’s PC\\$app\\`x`\\ubiqx.exe' -ArgumentList"));
    assert_eq!(script.matches("-Verb RunAs").count(), 1);
    // Linux: no shell at all, the path is one argument as it is.
    let exe = Path::new("/home/ana/my apps/it's; rm -rf ~/ubiqx");
    assert_eq!(
        pkexec_argv("/usr/bin/pkexec", exe, &remove)[1],
        "/home/ana/my apps/it's; rm -rf ~/ubiqx"
    );
}

#[test]
fn exit_codes_are_read_without_reading_words() {
    // macOS: AppleScript's error number, whatever the sentence in front says.
    let cancelled = exited(1, "0:120: execution error: User canceled. (-128)\n");
    assert_eq!(exit_of(Os::Macos, &cancelled), ElevatedExit::Declined);
    let cancelled_ptbr = exited(
        1,
        "0:120: erro de execução: Cancelado pelo usuário. (-128)\n",
    );
    assert_eq!(exit_of(Os::Macos, &cancelled_ptbr), ElevatedExit::Declined);
    let partial = exited(
        1,
        "0:98: execution error: The command exited with a non-zero status. (3)\n",
    );
    assert_eq!(exit_of(Os::Macos, &partial), ElevatedExit::Exited(3));
    assert_eq!(exit_of(Os::Macos, &exited(0, "")), ElevatedExit::Exited(0));
    assert_eq!(
        exit_of(Os::Macos, &exited(1, "osascript: no such file")),
        unavailable("failed")
    );
    assert_eq!(
        exit_of(Os::Macos, &exited(1, "error (-1743)")),
        unavailable("failed")
    );
    // Windows: 1223 is the declined prompt, anything else is the helper's own code.
    assert_eq!(
        exit_of(Os::Windows, &exited(1223, "")),
        ElevatedExit::Declined
    );
    assert_eq!(
        exit_of(Os::Windows, &exited(0, "")),
        ElevatedExit::Exited(0)
    );
    assert_eq!(
        exit_of(Os::Windows, &exited(5, "")),
        ElevatedExit::Exited(5)
    );
    // Linux: pkexec's two own codes.
    assert_eq!(exit_of(Os::Linux, &exited(126, "")), ElevatedExit::Declined);
    assert_eq!(
        exit_of(Os::Linux, &exited(127, "")),
        unavailable("no_polkit_agent")
    );
    assert_eq!(exit_of(Os::Linux, &exited(3, "")), ElevatedExit::Exited(3));
    assert_eq!(exit_of(Os::Linux, &exited(0, "")), ElevatedExit::Exited(0));
    // The tool itself is not there.
    let missing = || Err(io::Error::from(io::ErrorKind::NotFound));
    assert_eq!(
        exit_of(Os::Linux, &missing()),
        unavailable("no_polkit_agent")
    );
    assert_eq!(exit_of(Os::Macos, &missing()), unavailable("failed"));
    assert_eq!(exit_of(Os::Windows, &missing()), unavailable("failed"));
}

#[test]
fn helper_arguments_have_a_closed_alphabet() {
    for ok in [
        "apply",
        "--plan-b64",
        "eyJ2IjoxfQ-_",
        "hosts,resolver",
        "a=b",
        "--scope",
    ] {
        assert!(is_helper_arg(ok), "{ok}");
    }
    for refused in [
        "",
        "two words",
        "it's",
        "a;b",
        "$x",
        "a\"b",
        "a\\b",
        "ação",
        "a\nb",
        "a|b",
        "a'b",
        "a`b",
    ] {
        assert!(!is_helper_arg(refused), "{refused:?}");
    }
}

/// A program, its arguments and its environment.
type Started = (String, Vec<String>, Vec<(String, String)>);

/// Records what it is asked to run and answers one fixed exit.
struct OneAnswer {
    status: i32,
    calls: Mutex<Vec<Started>>,
}

impl Runner for OneAnswer {
    fn run(&self, program: &str, args: &[String], env: &[(String, String)]) -> io::Result<Output> {
        self.calls
            .lock()
            .push((program.to_owned(), args.to_vec(), env.to_vec()));
        exited(self.status, "")
    }
}

#[test]
fn the_elevator_runs_the_built_command_with_the_cleaned_environment() {
    let runner = Arc::new(OneAnswer {
        status: 126,
        calls: Mutex::new(Vec::new()),
    });
    let env = vec![("PATH".to_owned(), "/usr/bin:/bin".to_owned())];
    let elevator = RunnerElevator::new(
        Os::Linux,
        "/usr/bin/pkexec".into(),
        env.clone(),
        runner.clone(),
        false,
    );
    let args = strings(&["remove", "--owner", "shield", "--scope", "resolver"]);
    assert_eq!(
        elevator.run_elevated(Path::new("/usr/bin/ubiqx-shield"), &args),
        ElevatedExit::Declined
    );
    let calls = runner.calls.lock().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "/usr/bin/pkexec");
    assert_eq!(
        calls[0].1,
        strings(&[
            "/usr/bin/ubiqx-shield",
            "--shield-helper",
            "remove",
            "--owner",
            "shield",
            "--scope",
            "resolver"
        ])
    );
    assert_eq!(calls[0].2, env);
}

#[test]
fn an_appimage_and_an_odd_argument_are_refused_without_running_anything() {
    let runner = Arc::new(OneAnswer {
        status: 0,
        calls: Mutex::new(Vec::new()),
    });
    let appimage = RunnerElevator::new(
        Os::Linux,
        "/usr/bin/pkexec".into(),
        Vec::new(),
        runner.clone(),
        true,
    );
    assert_eq!(
        appimage.run_elevated(
            Path::new("/tmp/.mount_ubiqx/usr/bin/ubiqx"),
            &strings(&["status"])
        ),
        unavailable("appimage")
    );
    let plain = RunnerElevator::new(
        Os::Windows,
        "ps.exe".into(),
        Vec::new(),
        runner.clone(),
        false,
    );
    assert_eq!(
        plain.run_elevated(
            Path::new("C:\\x.exe"),
            &strings(&["apply", "--plan-b64", "a b"])
        ),
        unavailable("failed")
    );
    assert!(runner.calls.lock().is_empty());
    // The same elevator with ordinary arguments does run.
    assert_eq!(
        plain.run_elevated(Path::new("C:\\x.exe"), &strings(&["status"])),
        ElevatedExit::Exited(0)
    );
    assert_eq!(runner.calls.lock().len(), 1);
}
