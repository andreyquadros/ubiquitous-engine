//! The installer files of the two shells, read from the files themselves: what the Windows
//! uninstall hook does in each way the uninstaller can be started, and the polkit action
//! each `.deb` installs for the executable that elevates itself.
//!
//! Packaging scripts cannot run here (NSIS and polkit live on the target systems), so the
//! hook is run by a small evaluator of the LogicLib subset it is written in. The evaluator
//! refuses any construct it does not know, so a hook that grows past it fails this test
//! instead of passing unread.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Each shell, with the owner its uninstall hook removes.
const SHELLS: [(&str, &str); 2] = [
    ("apps/desktop/src-tauri", "ubiqx"),
    ("apps/nofap/src-tauri", "shield"),
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    let full = repo().join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
}

fn conf(shell: &str) -> Value {
    serde_json::from_str(&read(&format!("{shell}/tauri.conf.json"))).unwrap()
}

/// The executable the shell builds: `mainBinaryName`, else the manifest's `[[bin]]` name,
/// else its package name (how Tauri names the file it installs to `/usr/bin/`).
fn binary_name(shell: &str) -> String {
    if let Some(name) = conf(shell)["mainBinaryName"].as_str() {
        return name.to_owned();
    }
    let manifest = read(&format!("{shell}/Cargo.toml"));
    let section = |header: &str| -> Option<String> {
        let body = manifest.split(header).nth(1)?;
        let body = body.split("\n[").next().unwrap_or(body);
        body.lines()
            .find_map(|line| line.trim().strip_prefix("name"))
            .and_then(|rest| rest.split('"').nth(1))
            .map(str::to_owned)
    };
    section("[[bin]]")
        .or_else(|| section("[package]"))
        .expect("the manifest names its binary")
}

// ---------------------------------------------------------------------------------------
// The Windows uninstall hook
// ---------------------------------------------------------------------------------------

/// How the uninstaller was started, as NSIS sees it inside the uninstaller.
struct Start {
    what: &'static str,
    /// `$UpdateMode`: 1 only when `/UPDATE` is on the command line.
    update_mode: i64,
    /// `$EXEDIR` and `$EXEFILE`: where the running uninstaller is and its file name.
    exe_dir: &'static str,
    exe_file: &'static str,
    /// Whether the app's executable is still in the install folder.
    app_present: bool,
    /// Whether the hook must take the system changes off.
    removes: bool,
}

const INSTDIR: &str = r"C:\Users\Ana\AppData\Local\App";

/// Every way the installed uninstaller gets started. The reinstall page of Tauri's NSIS
/// template (CLI 2.11) runs `"<INSTDIR>\uninstall.exe" [/P] _?=<INSTDIR>`: in place, and
/// with `/UPDATE` only from the updater, which skips that page anyway. A real uninstall
/// starts `uninstall.exe` without `_?=`; NSIS then copies it to a temporary folder and runs
/// the copy, so the hook sees the temporary folder and the copy's name.
const STARTS: [Start; 6] = [
    Start {
        what: "Windows Settings or Control Panel (NSIS runs a temporary copy)",
        update_mode: 0,
        exe_dir: r"C:\Users\Ana\AppData\Local\Temp\~nsuA.tmp",
        exe_file: "Un_A.exe",
        app_present: true,
        removes: true,
    },
    Start {
        what: "a real uninstall whose app executable is already gone",
        update_mode: 0,
        exe_dir: r"C:\Users\Ana\AppData\Local\Temp\~nsuA.tmp",
        exe_file: "Un_A.exe",
        app_present: false,
        removes: false,
    },
    Start {
        what: "the in-app update (/UPDATE)",
        update_mode: 1,
        exe_dir: r"C:\Users\Ana\AppData\Local\Temp\~nsuA.tmp",
        exe_file: "Un_A.exe",
        app_present: true,
        removes: false,
    },
    Start {
        what: "a newer -setup.exe run by hand, default choice (in place, _?=, no /UPDATE)",
        update_mode: 0,
        exe_dir: INSTDIR,
        exe_file: "uninstall.exe",
        app_present: true,
        removes: false,
    },
    Start {
        what: "the same, with the folder spelled in another case",
        update_mode: 0,
        exe_dir: r"c:\users\ana\appdata\local\app",
        exe_file: "uninstall.exe",
        app_present: true,
        removes: false,
    },
    Start {
        what: "the same, passive (/P)",
        update_mode: 0,
        exe_dir: INSTDIR,
        exe_file: "UNINSTALL.EXE",
        app_present: true,
        removes: false,
    },
];

/// The words of one NSIS line: whitespace-separated, a double-quoted word kept whole
/// without its quotes. A `;` outside quotes starts a comment.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            ';' if !quoted => break,
            c if c.is_whitespace() && !quoted => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    assert!(!quoted, "an unclosed quote in {line:?}");
    if started {
        out.push(word);
    }
    out
}

/// `text` with the variables NSIS would expand for `start`.
fn expand(text: &str, start: &Start) -> String {
    let vars: BTreeMap<&str, String> = [
        ("${MAINBINARYNAME}", "app".to_owned()),
        ("$UpdateMode", start.update_mode.to_string()),
        ("$EXEDIR", start.exe_dir.to_owned()),
        ("$EXEFILE", start.exe_file.to_owned()),
        ("$INSTDIR", INSTDIR.to_owned()),
    ]
    .into_iter()
    .collect();
    let mut out = text.to_owned();
    // The longest names first, so `$INSTDIR` is never read as a prefix of another name.
    let mut names: Vec<&&str> = vars.keys().collect();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    for name in names {
        out = out.replace(*name, &vars[*name]);
    }
    assert!(
        !out.contains('$'),
        "the evaluator does not know a variable of {text:?}"
    );
    out
}

/// One LogicLib condition: `a = n`, `a <> n` (numbers), `a == b`, `a != b` (strings,
/// compared without regard to case, as NSIS does) or `${FileExists} path`.
fn condition(tokens: &[String], start: &Start) -> bool {
    match tokens {
        [test, path] if test == "${FileExists}" => {
            let path = expand(path, start);
            assert_eq!(
                path,
                format!(r"{INSTDIR}\app.exe"),
                "the hook tests a file other than the app's executable"
            );
            start.app_present
        }
        [a, op, b] => {
            let (a, b) = (expand(a, start), expand(b, start));
            let number = |s: &str| -> i64 {
                s.parse()
                    .unwrap_or_else(|_| panic!("{s:?} is not a number"))
            };
            match op.as_str() {
                "=" => number(&a) == number(&b),
                "<>" => number(&a) != number(&b),
                "==" => a.eq_ignore_ascii_case(&b),
                "!=" => !a.eq_ignore_ascii_case(&b),
                other => panic!("the evaluator does not know the operator {other:?}"),
            }
        }
        other => panic!("the evaluator does not know the condition {other:?}"),
    }
}

/// What the hook does for `start`: the arguments of each `ExecShellWait "runas"` of the
/// app's executable. One flat `${If}` / `${ElseIf}` / `${Else}` chain whose conditions
/// may carry `${AndIf}` and `${OrIf}`, read left to right as LogicLib does.
fn run_hook(hook: &str, start: &Start) -> Vec<String> {
    let body = hook
        .split("!macro NSIS_HOOK_PREUNINSTALL")
        .nth(1)
        .and_then(|rest| rest.split("!macroend").next())
        .expect("the hook defines NSIS_HOOK_PREUNINSTALL");
    let mut executed = Vec::new();
    // Whether a branch of the chain was already taken, whether the current one runs, and
    // whether the chain is open.
    let (mut taken, mut running, mut open) = (false, false, false);
    // Whether the current branch's condition is still being read.
    let mut reading = false;
    for line in body.lines() {
        let tokens = words(line);
        let Some(first) = tokens.first() else {
            continue;
        };
        let rest = &tokens[1..];
        match first.as_str() {
            "${If}" => {
                assert!(!open, "the evaluator does not know nested ${{If}}");
                open = true;
                running = condition(rest, start);
                reading = true;
            }
            "${ElseIf}" => {
                assert!(open, "${{ElseIf}} outside ${{If}}");
                taken |= running;
                running = !taken && condition(rest, start);
                reading = true;
            }
            "${AndIf}" | "${OrIf}" => {
                assert!(reading, "{first} after the branch's first statement");
                let value = condition(rest, start);
                if first == "${AndIf}" {
                    running = running && value;
                } else {
                    running = running || value;
                }
                // A branch of the chain already taken stays the only one.
                running = running && !taken;
            }
            "${Else}" => {
                assert!(open, "${{Else}} outside ${{If}}");
                taken |= running;
                running = !taken;
                reading = false;
            }
            "${EndIf}" => {
                assert!(open, "${{EndIf}} outside ${{If}}");
                open = false;
                running = false;
                taken = false;
                reading = false;
            }
            "DetailPrint" => reading = false,
            "ExecShellWait" => {
                reading = false;
                let (verb, file, args) = match rest {
                    [verb, file, args, _show] => (verb, file, args),
                    other => panic!("an ExecShellWait the evaluator does not know: {other:?}"),
                };
                assert_eq!(verb, "runas");
                assert_eq!(expand(file, start), format!(r"{INSTDIR}\app.exe"));
                // Outside any branch a statement always runs.
                if running || !open {
                    executed.push(args.clone());
                }
            }
            other => panic!("the evaluator does not know the statement {other:?}"),
        }
    }
    assert!(!open, "the hook leaves an ${{If}} open");
    executed
}

#[test]
fn the_uninstall_hook_removes_the_system_changes_only_on_a_real_uninstall() {
    for (shell, owner) in SHELLS {
        let path = format!("{shell}/packaging/nsis-hooks.nsh");
        let hook = read(&path);
        let removal = format!("--shield-helper remove --owner {owner} --scope all");
        for start in &STARTS {
            let executed = run_hook(&hook, start);
            let expected = if start.removes {
                vec![removal.clone()]
            } else {
                Vec::new()
            };
            assert_eq!(
                executed,
                expected,
                "{path}, uninstaller started by {}: expected {}",
                start.what,
                if start.removes {
                    "the removal"
                } else {
                    "the system changes kept"
                }
            );
        }
    }
}

#[test]
fn the_hook_evaluator_reads_logiclib_chains_as_nsis_does() {
    let start = &STARTS[0];
    let hook = |body: &str| format!("!macro NSIS_HOOK_PREUNINSTALL\n{body}\n!macroend\n");
    let exec = r#"    ExecShellWait "runas" "$INSTDIR\${MAINBINARYNAME}.exe" "x" SW_HIDE"#;
    // A taken branch keeps the later ones from running, an `${OrIf}` included.
    let chain = format!(
        "  ${{If}} 1 = 1\n    DetailPrint \"a\"\n  ${{ElseIf}} 1 = 1\n  ${{OrIf}} 1 = 1\n{exec}\n  ${{EndIf}}"
    );
    assert!(run_hook(&hook(&chain), start).is_empty());
    let chain = format!("  ${{If}} 1 = 0\n  ${{OrIf}} a == A\n{exec}\n  ${{EndIf}}");
    assert_eq!(run_hook(&hook(&chain), start), vec!["x".to_owned()]);
    let chain = format!("  ${{If}} 1 = 1\n  ${{AndIf}} a != A\n{exec}\n  ${{EndIf}}");
    assert!(run_hook(&hook(&chain), start).is_empty());
    let chain =
        format!("  ${{If}} 1 = 1\n    DetailPrint \"a\"\n  ${{Else}}\n{exec}\n  ${{EndIf}}");
    assert!(run_hook(&hook(&chain), start).is_empty());
    let unknown = std::panic::catch_unwind(|| run_hook(&hook("  Exec \"x\""), start));
    assert!(unknown.is_err(), "an unknown statement must fail the test");
}

// ---------------------------------------------------------------------------------------
// The polkit action of each .deb
// ---------------------------------------------------------------------------------------

/// The text of every `<tag ...>` element of `xml` whose opening tag contains `attrs`.
fn elements(xml: &str, tag: &str, attrs: &str) -> Vec<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find(&open) {
        let after = &rest[at + open.len()..];
        let end = after.find('>').expect("an opening tag is closed");
        let head = &after[..end];
        let inner = &after[end + 1..];
        let stop = inner.find(&close).expect("an element is closed");
        if (head.is_empty() || head.starts_with(' ')) && head.contains(attrs) {
            out.push(inner[..stop].trim().to_owned());
        }
        rest = &inner[stop..];
    }
    out
}

#[test]
fn each_deb_installs_the_polkit_action_of_the_executable_it_elevates() {
    for (shell, _) in SHELLS {
        let conf = conf(shell);
        let files = conf["bundle"]["linux"]["deb"]["files"]
            .as_object()
            .unwrap_or_else(|| panic!("{shell}: the .deb ships no files (bundle.linux.deb.files)"));
        let actions: Vec<(&String, &Value)> = files
            .iter()
            .filter(|(target, _)| target.starts_with("/usr/share/polkit-1/actions/"))
            .collect();
        assert_eq!(
            actions.len(),
            1,
            "{shell}: the .deb installs one polkit action"
        );
        let (target, source) = actions[0];
        let source = source.as_str().expect("a file's source is a path");
        let name = target.rsplit('/').next().unwrap();
        let identifier = conf["identifier"].as_str().unwrap();
        assert!(
            name.starts_with(identifier) && name.ends_with(".policy"),
            "{shell}: polkit reads only *.policy files, named after the app: {name}"
        );
        assert_eq!(
            Path::new(source).file_name().unwrap().to_string_lossy(),
            name,
            "{shell}: the installed name and the source name differ"
        );
        let policy = read(&format!("{shell}/{source}"));

        let ids: Vec<&str> = policy
            .match_indices("<action id=\"")
            .map(|(at, open)| {
                let rest = &policy[at + open.len()..];
                &rest[..rest.find('"').unwrap()]
            })
            .collect();
        assert_eq!(ids.len(), 1, "{shell}: one action");
        assert!(
            ids[0].starts_with(identifier),
            "{shell}: the action id {} is under the app's identifier",
            ids[0]
        );

        // pkexec picks the action whose exec.path is the program it starts, which is the
        // installed executable (`current_exe`).
        let binary = binary_name(shell);
        assert_eq!(
            elements(
                &policy,
                "annotate",
                "key=\"org.freedesktop.policykit.exec.path\""
            ),
            vec![format!("/usr/bin/{binary}")],
            "{shell}: exec.path is not the executable the .deb installs"
        );

        for when in ["allow_any", "allow_inactive", "allow_active"] {
            assert_eq!(
                elements(&policy, when, ""),
                vec!["auth_admin".to_owned()],
                "{shell}: {when} must ask for an administrator's password every time"
            );
        }
        for tag in ["description", "message"] {
            let all = elements(&policy, tag, "");
            assert_eq!(
                all.len(),
                2,
                "{shell}: a {tag} in English and one in Portuguese"
            );
            assert!(
                all.iter().all(|text| !text.is_empty()),
                "{shell}: an empty {tag}"
            );
            let portuguese = elements(&policy, tag, "xml:lang=\"pt_BR\"");
            assert_eq!(portuguese.len(), 1, "{shell}: a Portuguese {tag}");
        }
        let messages = elements(&policy, "message", "");
        let product = conf["productName"].as_str().unwrap();
        assert!(
            messages.iter().all(|text| text.contains(product)),
            "{shell}: the dialog names the app ({product}): {messages:?}"
        );
    }
}
