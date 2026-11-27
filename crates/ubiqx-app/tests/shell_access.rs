//! What each window of the two shells may call (integrator decision 34), read from the
//! files themselves: the commands each build script declares to Tauri's access control,
//! the commands each shell registers, the commands each page calls and the capability file
//! of each window. Declaring the commands is what makes Tauri check a window's capability
//! before an app command runs; without it every window could call every command.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

const DESKTOP: &str = "apps/desktop/src-tauri";
const STANDALONE: &str = "apps/nofap/src-tauri";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    let full = repo().join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// The text between `open` and the first `close` after it.
fn between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    let start = text
        .find(open)
        .unwrap_or_else(|| panic!("no {open:?} in the text"))
        + open.len();
    let end = text[start..]
        .find(close)
        .unwrap_or_else(|| panic!("no {close:?} after {open:?}"));
    &text[start..start + end]
}

/// The string literals between the brackets of `const COMMANDS: &[&str] = &[...]`.
fn declared(shell: &str) -> BTreeSet<String> {
    let script = read(&format!("{shell}/build.rs"));
    let list = between(&script, "const COMMANDS: &[&str] = &[", "];");
    let found: BTreeSet<String> = list
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            item.strip_prefix('"')
                .and_then(|item| item.strip_suffix('"'))
                .unwrap_or_else(|| panic!("{shell}/build.rs: {item} is not a string literal"))
                .to_string()
        })
        .collect();
    assert!(!found.is_empty(), "{shell}/build.rs declares no command");
    found
}

/// The commands of `tauri::generate_handler![...]` in `file`, by their last path segment.
fn registered(file: &str) -> BTreeSet<String> {
    let source = read(file);
    between(&source, "generate_handler![", "]")
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.rsplit("::").next().unwrap().to_string())
        .collect()
}

/// The command names `call('…')` or `call<T>('…')` passes in a frontend file.
fn called(file: &str) -> BTreeSet<String> {
    calls_in(&read(file))
}

fn calls_in(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut offset = 0;
    while let Some(at) = source[offset..].find("call") {
        let before = source[..offset + at].chars().next_back();
        offset += at + "call".len();
        // `recall(` or `do_call(` is another function.
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        let rest = &source[offset..];
        let mut chars = rest.char_indices().peekable();
        // Skips a type argument, `<Array<Block> | null>` included.
        if let Some((_, '<')) = chars.peek() {
            let mut depth = 0;
            for (_, c) in chars.by_ref() {
                match c {
                    '<' => depth += 1,
                    '>' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
            }
        }
        if !matches!(chars.next(), Some((_, '('))) {
            continue;
        }
        let Some((start, _)) = chars.find(|(_, c)| !c.is_whitespace()) else {
            break;
        };
        let Some(literal) = rest[start..].strip_prefix('\'') else {
            continue;
        };
        let end = literal.find('\'').expect("an unterminated string");
        found.insert(literal[..end].to_string());
    }
    found
}

/// The value of `export const <name> = '…';` in a frontend file.
fn constant(file: &str, name: &str) -> String {
    let source = read(file);
    between(&source, &format!("export const {name} = '"), "'").to_string()
}

/// The permission a capability names to allow `command` (the rule of both build scripts).
fn permission(command: &str) -> String {
    format!(
        "allow-{}",
        command.replace("nofap", "shield").replace('_', "-")
    )
}

fn permissions(commands: &BTreeSet<String>) -> BTreeSet<String> {
    commands.iter().map(|c| permission(c)).collect()
}

fn capability(shell: &str, name: &str) -> Value {
    let path = format!("{shell}/capabilities/{name}");
    serde_json::from_str(&read(&path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|v| v.as_str().expect("a string").to_string())
        .collect()
}

/// The capability's windows, its app-command permissions and its window and plugin ones
/// (those carry a `plugin:` prefix).
fn granted(shell: &str, name: &str) -> (Vec<String>, BTreeSet<String>, Vec<String>) {
    let capability = capability(shell, name);
    let (plugin, app): (Vec<String>, Vec<String>) = strings(&capability["permissions"])
        .into_iter()
        .partition(|p| p.contains(':'));
    let app_set: BTreeSet<String> = app.iter().cloned().collect();
    assert_eq!(
        app.len(),
        app_set.len(),
        "{shell}/{name} repeats a permission"
    );
    (strings(&capability["windows"]), app_set, plugin)
}

#[test]
fn each_shell_declares_every_command_it_registers() {
    assert_eq!(
        declared(DESKTOP),
        registered(&format!("{DESKTOP}/src/commands.rs"))
    );
    assert_eq!(
        declared(STANDALONE),
        registered(&format!("{STANDALONE}/src/app.rs"))
    );
    // The parse found the handlers, not an empty list on both sides.
    assert!(declared(DESKTOP).contains("delete_all_data"));
    assert!(declared(DESKTOP).contains("nofap"));
    assert_eq!(
        declared(STANDALONE),
        set(&["nofap", "nofap_host", "quit_app"])
    );
}

#[test]
fn ubiqx_main_window_may_call_what_its_pages_call_and_nothing_else() {
    let mut pages = called("apps/desktop/src/lib/ipc.ts");
    assert!(pages.len() > 50, "only {} commands were read", pages.len());
    assert!(pages.contains("get_dashboard") && pages.contains("delete_all_data"));
    pages.insert(constant("apps/desktop/src/nofap/ipc.ts", "NOFAP_COMMAND"));
    // Every command a page calls is registered (a typo in a page fails here, not in the app).
    let declared = declared(DESKTOP);
    assert!(
        pages.is_subset(&declared),
        "called but not registered: {:?}",
        pages.difference(&declared).collect::<Vec<_>>()
    );

    let (windows, app, plugin) = granted(DESKTOP, "default.json");
    assert_eq!(windows, ["main"]);
    assert_eq!(app, permissions(&pages));
    assert_eq!(
        plugin,
        [
            "core:default",
            "core:window:allow-show",
            "core:window:allow-hide",
            "core:window:allow-set-focus",
            "core:window:allow-start-dragging",
            "core:window:allow-minimize",
            "core:window:allow-close",
            "opener:default",
            "notification:default",
            "autostart:default",
            "log:default",
            "updater:default",
            "process:allow-restart",
        ]
    );
}

/// ubiqX's card page calls the module's command and reads the settings for its language
/// (`UbiqxNofapCard`, `loadSettings`); the intervention page reads the settings and the
/// interventions. Nothing else, and never a command that deletes or changes anything.
#[test]
fn ubiqx_floating_windows_may_call_only_what_their_pages_call() {
    let ipc = read("apps/desktop/src/lib/ipc.ts");
    assert!(ipc.contains("getSettings: () => call<SettingsView>('get_settings')"));
    assert!(ipc.contains("call<Intervention[]>('list_interventions'"));
    let card_host = read("apps/desktop/src/nofap/ubiqx.tsx");
    assert!(card_host.contains("export function UbiqxNofapCard()"));
    assert!(card_host.contains("loadSettings()"));
    let intervention = read("apps/desktop/src/pages/Intervention.tsx");
    assert!(intervention.contains("loadSettings()"));
    assert!(intervention.contains(".listInterventions("));

    let (windows, app, plugin) = granted(DESKTOP, "shield-card.json");
    assert_eq!(windows, ["shield-card"]);
    assert_eq!(app, set(&["allow-shield", "allow-get-settings"]));
    assert_eq!(
        plugin,
        [
            "core:default",
            "core:window:allow-close",
            "core:window:allow-set-size",
            "core:window:allow-start-dragging",
        ]
    );

    let (windows, app, plugin) = granted(DESKTOP, "intervention.json");
    assert_eq!(windows, ["intervention"]);
    assert_eq!(
        app,
        set(&["allow-get-settings", "allow-list-interventions"])
    );
    assert_eq!(
        plugin,
        [
            "core:default",
            "core:window:allow-close",
            "core:window:allow-hide",
            "core:window:allow-set-focus",
            "core:window:allow-start-dragging",
        ]
    );
}

#[test]
fn the_standalone_windows_may_call_only_what_their_pages_call() {
    let module = constant("apps/desktop/src/nofap/ipc.ts", "NOFAP_COMMAND");
    let host = constant("apps/desktop/src/nofap-app/hostIpc.ts", "HOST_COMMAND");
    let quit = constant("apps/desktop/src/nofap-app/hostIpc.ts", "QUIT_COMMAND");
    let pages: BTreeSet<String> = [module.clone(), host.clone(), quit].into();
    assert_eq!(pages, declared(STANDALONE));

    let (windows, app, plugin) = granted(STANDALONE, "default.json");
    assert_eq!(windows, ["main"]);
    assert_eq!(app, permissions(&pages));
    assert_eq!(
        plugin,
        strings(&capability(DESKTOP, "default.json")["permissions"])
            .into_iter()
            .filter(|p| p.contains(':'))
            .collect::<Vec<_>>()
    );

    // The card reads the settings through the host command for its language; the shell
    // answers nothing else of that command to this window (`card_may_send` in `app.rs`).
    let card_page = read("apps/desktop/src/nofap-app/AppNofap.tsx");
    assert!(card_page.contains("function StandaloneCard()"));
    let shell = read(&format!("{STANDALONE}/src/app.rs"));
    assert!(shell.contains("window.label() == card::CARD_WINDOW && !card_may_send(&command)"));
    let (windows, app, plugin) = granted(STANDALONE, "shield-card.json");
    assert_eq!(windows, ["shield-card"]);
    assert_eq!(app, permissions(&[module, host].into()));
    assert_eq!(
        plugin,
        [
            "core:default",
            "core:window:allow-close",
            "core:window:allow-set-size",
            "core:window:allow-start-dragging",
        ]
    );
}

/// The `cmd`s the module's card page sends through `nofapIpc`: each `nofapIpc.<method>` in
/// `NofapCard.tsx`, and in the module's components it imports, mapped to its `cmd` through
/// `nofap/ipc.ts` (`method: (…) => send<…>({ cmd: '…'`).
fn card_page_sends() -> BTreeSet<String> {
    let ipc = read("apps/desktop/src/nofap/ipc.ts");
    let mut sources = vec![read("apps/desktop/src/nofap/NofapCard.tsx")];
    // The components the page imports, and the ones they import in turn.
    let mut seen = BTreeSet::new();
    let mut next = 0;
    while next < sources.len() {
        let imported: Vec<String> = sources[next]
            .lines()
            .filter_map(|line| line.trim().strip_prefix("import "))
            .filter_map(|rest| {
                let from = rest.split(" from '").nth(1)?.split('\'').next()?;
                let name = from
                    .strip_prefix("./components/")
                    .or_else(|| (next > 0).then(|| from.strip_prefix("./")).flatten())?;
                Some(name.to_string())
            })
            .collect();
        for component in imported {
            if seen.insert(component.clone()) {
                sources.push(read(&format!(
                    "apps/desktop/src/nofap/components/{component}.tsx"
                )));
            }
        }
        next += 1;
    }
    assert!(
        seen.contains("PanicSheet") && seen.contains("SurfTimer"),
        "the card page's components were not followed: {seen:?}"
    );
    let mut sent = BTreeSet::new();
    for source in &sources {
        let mut rest = source.as_str();
        while let Some(at) = rest.find("nofapIpc") {
            rest = &rest[at + "nofapIpc".len()..];
            let Some(after_dot) = rest.trim_start().strip_prefix('.') else {
                continue;
            };
            let method: String = after_dot
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let definition = ipc
                .lines()
                .find(|line| line.trim_start().starts_with(&format!("{method}: (")))
                .unwrap_or_else(|| panic!("nofapIpc.{method} is not in nofap/ipc.ts"));
            sent.insert(between(definition, "cmd: '", "'").to_string());
        }
    }
    sent
}

/// The card window of both shells is answered only what its page sends of the module's
/// command (`ubiqx_app::CARD_WINDOW_COMMANDS`); each shell's handler asks before it runs
/// anything.
#[test]
fn the_card_window_is_answered_only_what_its_page_sends() {
    let sent = card_page_sends();
    assert!(sent.contains("card") && sent.contains("card_action"));
    assert_eq!(sent, set(&ubiqx_app::CARD_WINDOW_COMMANDS));
    for (file, label) in [
        (format!("{DESKTOP}/src/nofap.rs"), "CARD_WINDOW"),
        (format!("{STANDALONE}/src/app.rs"), "card::CARD_WINDOW"),
    ] {
        let shell = read(&file);
        assert!(
            shell.contains(&format!(
                "window.label() == {label} && !ubiqx_app::card_window_may_send(&command)"
            )),
            "{file} does not filter what the card window sends"
        );
    }
}

/// Every permission a capability names is one the build script generates: Tauri refuses
/// an unknown one when it builds, and this says which file and which name.
#[test]
fn every_permission_a_capability_names_exists() {
    for (shell, files) in [
        (
            DESKTOP,
            &["default.json", "intervention.json", "shield-card.json"][..],
        ),
        (STANDALONE, &["default.json", "shield-card.json"][..]),
    ] {
        let generated = permissions(&declared(shell));
        for file in files {
            let (_, app, _) = granted(shell, file);
            let unknown: Vec<_> = app.difference(&generated).collect();
            assert!(unknown.is_empty(), "{shell}/{file}: {unknown:?}");
        }
    }
}

#[test]
fn the_readers_read() {
    let source = "async function call<T>(cmd: string) {}\n\
                  const a = () => call<Array<Block> | null>('get_one', { x });\n\
                  const b = () => call(\n  'get_two');\n\
                  const c = () => recall('not_this');\n\
                  const d = () => api.call('nor_this');\n";
    assert_eq!(calls_in(source), set(&["get_one", "get_two"]));
    assert_eq!(between("a [x, y] b];", "[", "]"), "x, y");
    assert_eq!(permission("nofap_host"), "allow-shield-host");
    assert_eq!(permission("get_dashboard"), "allow-get-dashboard");
}
