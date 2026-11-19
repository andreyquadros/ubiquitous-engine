# Map: desktop shell, frontend, CI/release and licensing (for the nofap module)

Read-only code map of `apps/desktop` (Tauri 2 shell + React app), the browser/mock mode, the test
setup, CI/release, and licensing (`ubiqx-core::license`, `ubiqx-engine::license`,
`services/ubi-api`). Written for the engineer who will add the **nofap** module to ubiqX and ship
it a second time as a standalone product. Everything below was read in the code at
`claude/sharp-bardeen-sk27ae` (HEAD `410c3d6`); `file:line` references are to that tree.
Sibling maps: `map-focus-guard.md`, `map-vision-ai.md` (engine side; not repeated here).

Contents

1. The facts that constrain the design
2. Workspace layout as the shell sees it
3. The shell: `lib.rs` (state, plugins, setup, exit)
4. Windows: `main` and `intervention`
5. Tray and menus
6. IPC end to end (commands, errors, events) + recipe
7. Frontend: routing, nav, store, pages (Focus, Settings, Onboarding) + recipe
8. i18n conventions
9. Design tokens and components to reuse
10. Mock / browser mode, tests, screenshots
11. Build, bundle, CI, release
12. Licensing today, and what a second product needs
13. A second Tauri app in the workspace: what it needs, what it costs
14. Surprising, fragile or missing
15. Checklists

---

## 1. The facts that constrain the design

1. **The app runs unprivileged and CI cannot sign for Apple.** NSIS installs per user
   (`tauri.conf.json:70` `"installMode": "currentUser"`), the AppImage runs from anywhere, macOS
   bundles are ad-hoc signed (`tauri.conf.json:50-51` `signingIdentity: null, entitlements: null`;
   `ci.yml:145` `APPLE_SIGNING_IDENTITY: "-"`). There is **no elevation path anywhere in the
   shell** (no helper, no `pkexec`, no UAC manifest, no installer hooks). DNS-level blocking
   (hosts file, resolver settings) needs admin on all three OSes, so the nofap system layer has to
   bring its own elevation, and on macOS it cannot use anything that requires a Developer ID
   (privileged helper via SMAppService, NetworkExtension DNS proxy/content filter).
2. **Only Rust talks to the network.** The CSP is `default-src 'self'; connect-src 'self' ipc:
   http://ipc.localhost blob:` (`tauri.conf.json:27`) and `components/ubi/csp.test.ts` fails the
   build if any directive reaches `http…`. Blocklist downloads and AI calls must live in the
   crates; the webview only invokes commands.
3. **IPC is hand-typed three times**: Rust command (`commands.rs`) → `ipc.ts` wrapper →
   `types.ts` mirror, plus a fourth copy in `mock.ts`. Nothing generates or checks the mirror.
   A missing mock handler throws `mock: comando desconhecido "<cmd>"` (`mock.ts:2157`), which is
   what tests and `pnpm dev` hit.
4. **One settings object, sent whole.** `update_settings(settings: Settings)` takes the full
   struct (`commands.rs:607-636`); `Settings` and `FocusSettings` are `#[serde(default)]`
   (`model.rs:906`, `:872`), so a new `nofap: NofapSettings` sub-struct loads from old rows and
   the frontend patches it with `saveSettings({ nofap: {...} })` exactly like
   `patchFocus` (`pages/Focus.tsx:60-68`).
5. **There is exactly one floating window and it is small**: label `intervention`, 460×188, no
   decorations, always on top, closes itself after 7 s (`lib.rs:246-326`,
   `pages/Intervention.tsx:13`). It is fed by `InterventionPresenter::show(&Intervention)`
   (`ports.rs:134-136`). Counselling copy does not fit that panel or that timeout; a second
   window/route needs a new label, a new capability file and its own presenter path.
6. **Closing never destroys a window, quitting is tray-only.** `on_window_event` turns every
   `CloseRequested` into `hide()` for every window (`lib.rs:628-635`); `ExitRequested` without a
   code is vetoed (`lib.rs:640-647`); the only exit is the tray item `quit` → `engine.shutdown();
   app.exit(0)` (`lib.rs:470-473`).
7. **i18n is flat keys in per-namespace files, PT-BR first, parity enforced by a test**
   (`i18n/parity.test.ts`): same keys, same `{placeholders}`, paired `_one/_other`, no empty
   strings, no ALL-CAPS. A new namespace must be registered in `NAMESPACES`
   (`i18n/index.ts:28-45`). Rust-side copy (tray labels, command messages, intervention lines)
   is a separate `match UiLanguage` table per file.
8. **Every push on every branch publishes the `continuous` release that installed apps
   auto-update to** (`ci.yml:4-5` `branches: ["**"]`, `ci.yml:335` `if: github.event_name ==
   'push'`), and the update check filters neither on branch nor on product
   (`update.rs:304-310`). Pushing the nofap branch ships it to every user. See § 14.1.
9. **App identity is hardcoded in the crates, not injected**: `SELF_BUNDLE_ID = "ai.ubiqx.app"`
   (`ubiqx-core/src/focus.rs:32`), `EXEMPT_APP_NAMES` contains `"ubiqX"` (`focus.rs:45-46`),
   `SELF_STEMS = ["ubiqx","ubiqx-desktop","ubiqx_desktop"]` (`ubiqx-platform/src/app_id.rs:9`),
   keyring service `"ai.ubiqx"` (`macos/keychain.rs:8`, `secrets_file.rs:22`),
   `DEFAULT_FEED_URL` (`ubiqx-core/src/update.rs:22`, duplicated in `src-tauri/build.rs:13`),
   feed product `"ubiqX"` (`ubiqx-engine/src/update.rs:297`). A second app with another bundle
   id is not exempt from its own guard and shares the first app's keyring entries unless these
   become parameters.
10. **Licence claims are a closed, canonical format.** `LicenseClaims` is
    `#[serde(deny_unknown_fields)]` with a hand-written canonical JSON of exactly seven fields
    (`license.rs:110-144`), `Plan` is a two-variant enum, `CLAIMS_VERSION = 1`, and enforcement
    is the compile-time constant `LICENSE_ENFORCEMENT = Soft` (`license.rs:53`). Any new plan or
    claim makes the key read as `invalid` in every build already installed.

---

## 2. Workspace layout as the shell sees it

```
Cargo.toml                         workspace; apps/desktop/src-tauri is a member but NOT a default-member
crates/ubiqx-{core,storage,platform,ai,engine,app,cli}
apps/desktop/
  package.json  pnpm-lock.yaml     standalone pnpm project (no pnpm workspace, no root package.json)
  vite.config.ts tsconfig.json     no path aliases; relative imports only
  index.html                       <title>ubiqX</title>, theme bootstrap script, /ubi/favicon.svg
  public/ubi/{Ubi.glb(4.5 MB),favicon.svg,README.md}
  src/                             React 19 + TS 6 + Tailwind 4 + zustand 5 + react-router 7 (HashRouter)
  src-tauri/
    Cargo.toml                     package `ubiqx-desktop`, lib `ubiqx_desktop_lib` (staticlib,cdylib,rlib)
    build.rs                       stamps UBIQX_BUILD_{EPOCH,NUMBER,SHA,BRANCH} + UBIQX_UPDATE_FEED_URL
    tauri.conf.json                productName ubiqX, identifier ai.ubiqx.app, bundle + updater config
    Info.plist                     LSUIElement, NSAppSleepDisabled, usage strings (merged by Tauri)
    capabilities/{default,intervention}.json
    icons/                         app icons + tray.png / tray@2x.png (generated by scripts/gen-tray-icon.mjs)
    src/{main.rs,lib.rs,commands.rs}
services/ubi-api/                  separate Cargo workspace (licence issuing + managed AI proxy)
scripts/                           build-info.sh, app-version.mjs, ci-*.sh, publish-release.mjs (+ tests)
.github/workflows/ci.yml           rust-linux, frontend, macos-app, windows-app, linux-app, publish-continuous
codemagic.yaml                     optional macOS-only builder publishing into the same release
```

Root `Cargo.toml:15-23` keeps the desktop crate out of `default-members`, and CI excludes it by
name wherever webkit is absent: `cargo clippy --workspace --exclude ubiqx-desktop` (`ci.yml:52`),
`cargo test --workspace --exclude ubiqx-desktop` (`ci.yml:54`, `:214`). A second shell crate must
be excluded the same way in those three places.

Locked versions: `tauri 2.11.5`, `tauri-build 2.6.3`, `tauri-plugin-updater 2.11.0`,
`tauri-plugin-autostart 2.5.1`, `tauri-plugin-notification 2.4.0`, `tauri-plugin-log 2.9.1`,
`@tauri-apps/cli 2.11.4`, `keyring 3.6.3`. Release profile: `codegen-units = 1, lto = true,
opt-level = 3, panic = "abort", strip = true` (`Cargo.toml:72-77`) — every final link is slow.

The desktop crate depends only on `ubiqx-app`, `ubiqx-core`, `ubiqx-engine` (+ chrono, serde,
tokio, tracing, url, log) and six plugins: `opener`, `notification`, `autostart`, `log`,
`process`, `updater` (`src-tauri/Cargo.toml:19-35`). There is **no `tauri-plugin-single-instance`
and no `tauri-plugin-shell`** on the Rust side (the JS package `@tauri-apps/plugin-shell` is in
`package.json` but unused; the same goes for the JS halves of `autostart`, `notification`,
`opener`).

---

## 3. The shell: `apps/desktop/src-tauri/src/lib.rs` (721 lines)

`main.rs` is 6 lines: `ubiqx_desktop_lib::run()`.

### 3.1 State

```rust
pub struct AppState { pub app: App }          // lib.rs:24   (ubiqx_app::App: engine, store, platform, remote, data_dir, scripted, ubi_api_base)
pub struct Menus   { open, pause, resume, private_mode, private_30, private_60, private_tomorrow,
                     private_indefinite, private_off, snooze, update, report_today, quit, hide_window }   // lib.rs:92-107
```

Both are `app.manage(...)`d (`lib.rs:610`, `:621`). Commands receive `State<'_, AppState>` and
call `state.app.engine.clone()` (`commands.rs:78-80`).

### 3.2 Adapters the shell injects into the engine

| Port (ubiqx-core) | Shell type | What it does |
|---|---|---|
| `EventSink` (`ports.rs:329`) | `TauriSink(AppHandle)` `lib.rs:159-173` | `self.0.emit("engine", &event)` to **all** windows; on `UpdateAvailable` also relabels the tray |
| `Notifier` (`ports.rs:104`) | `TauriNotifier` `lib.rs:219-231` | `notification().builder().title().body().show()` |
| `InterventionPresenter` (`ports.rs:134`) | `TauriPresenter` `lib.rs:331-341` | `show_intervention_window(&app, &intervention.id)` then `emit("intervention", intervention)`; returns `Ok(true)` |

`AppConfig` (`ubiqx-app/src/lib.rs:75-101`) is built at `lib.rs:571-582`:

```rust
AppConfig { data_dir: Some(app.path().app_data_dir()?), in_memory_db: false, scripted_platform: scripted,
            ai, notifier: Some(notifier), presenter: Some(presenter), build,
            update_feed_url: Some(env!("UBIQX_UPDATE_FEED_URL").to_string()),
            ubi_api_base: ubi_api_base(),            // option_env!("UBIQX_API_BASE"), lib.rs:193-198
            license_pubkey_hex: None }
```

`App::start(config, sink)` is run with `tauri::async_runtime::block_on` inside `.setup`
(`lib.rs:583`). The data dir comes from the **Tauri identifier** (`ai.ubiqx.app`), the database
is `<data_dir>/ubiqx.db` (`ubiqx-app/src/lib.rs:328`).

Dev switches read at start (`lib.rs:543-561`): `UBIQX_SCRIPTED` (scripted demo platform,
auto-sets `onboarding_done`), `UBIQX_FAKE_AI`, `UBIQX_ALLOW_REAL_AI=1`.

### 3.3 Builder order (`run()`, `lib.rs:503-657`)

1. `--minimized` argv flag (`lib.rs:504`), which autostart passes
   (`tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"]))`,
   `lib.rs:508-511`).
2. Plugins: opener, notification, autostart, log (`LevelFilter::Info`), updater, process.
3. `.on_menu_event` for the app-menu item `hide_window` (`lib.rs:523-529`).
4. `.setup`: macOS `ActivationPolicy::Accessory` (`lib.rs:531-532`); start the engine; sync the
   login item with `settings.launch_at_login` (`lib.rs:598-605`); build app menu + tray in the
   stored UI language; show the main window when `!minimized || !onboarding_done`
   (`lib.rs:623-625`).
5. `.on_window_event`: `CloseRequested` → `api.prevent_close(); window.hide()` for **any**
   window (`lib.rs:628-635`).
6. `.invoke_handler(commands::handler())`.
7. `.run`: `ExitRequested` with `code.is_none()` → `api.prevent_exit()`; with a code →
   `engine.shutdown()`. `RunEvent::Exit` (logout/shutdown) → `engine.shutdown()`
   (`lib.rs:639-656`).

Consequences for nofap: the process is effectively unkillable from the UI except through the
tray; a logout calls `engine.shutdown()` and nothing else — whatever the nofap system layer
installed (hosts entries, resolver settings) stays unless the module itself decides to remove it
on shutdown, and that decision is a product one (a block that disappears when the app quits is
trivially bypassed; one that survives uninstall strands the user). The shell gives no
uninstall hook today.

### 3.4 Build identity

`build.rs` stamps `UBIQX_BUILD_EPOCH/NUMBER/SHA/BRANCH` (env first, git fallback, then
`0/0/dev/""`) and `UBIQX_UPDATE_FEED_URL` (env or the constant at `build.rs:13-14`).
`build_info(version)` (`lib.rs:208-216`) takes the version from
`app.package_info().version`, i.e. from `tauri.conf.json`, which CI rewrites to
`0.1.<commit count>` (`scripts/app-version.mjs`).

---

## 4. Windows

### 4.1 `main`

Declared statically (`tauri.conf.json:13-25`): label `main`, title `ubiqX`, 1280×820, min
980×640, `visible: false`, `titleBarStyle: "Overlay"`, `hiddenTitle: true`. The React shell
leaves a 38 px `data-tauri-drag-region` strip at the top of the rail and of the content column
(`Sidebar.tsx:75`, `AppShell.tsx:73`, `Onboarding.tsx:217,268`).
`show_main_window(&AppHandle)` = `show(); unminimize(); set_focus()` (`lib.rs:233-239`); exposed
as the command `show_window` (`commands.rs:795-799`, no `ipc.ts` wrapper).

Capability `capabilities/default.json`: windows `["main"]`, permissions `core:default`,
`core:window:allow-{show,hide,set-focus,start-dragging,minimize,close}`, `opener:default`,
`notification:default`, `autostart:default`, `log:default`, `updater:default`,
`process:allow-restart`.

### 4.2 `intervention`

Created lazily in Rust, never declared in `tauri.conf.json`:

```rust
pub const INTERVENTION_WINDOW: &str = "intervention";          // lib.rs:246
const INTERVENTION_SIZE: (f64, f64) = (460.0, 188.0);          // lib.rs:249
const INTERVENTION_TOP_GAP: f64 = 24.0;                        // lib.rs:252
fn intervention_route(id: &str) -> String                      // lib.rs:255  "#/intervention?id=<[A-Za-z0-9_-]*>"
fn intervention_position(app) -> Option<PhysicalPosition<i32>> // lib.rs:265  centred, 24 px below the top of the monitor under the cursor
pub fn show_intervention_window(app: &AppHandle, id: &str) -> CoreResult<()>   // lib.rs:280
```

First use: `WebviewWindowBuilder::new(app, "intervention", WebviewUrl::App("index.html#/intervention?id=…"))`
with `.title("ubiqX").inner_size(460,188).decorations(false).shadow(true).resizable(false)
.always_on_top(true).visible_on_all_workspaces(true).skip_taskbar(true).focused(false).visible(false)`
(`lib.rs:294-308`). Later uses: `w.eval("window.location.hash = \"#/intervention?id=…\"")`
(`lib.rs:286-290`), then `set_size`, `set_position`, `set_always_on_top(true)`, `show()`.
The window is **not** transparent (no `.transparent(true)`, no `macOSPrivateApi`), so the
page's 16 px rounded panel sits on the opaque window background.

Capability `capabilities/intervention.json`: windows `["intervention"]`, `core:default`,
`core:window:allow-{close,hide,set-focus,start-dragging}`.

The page (`pages/Intervention.tsx`, route outside `Gate`/`AppShell`, `App.tsx:19`):

- `INTERVENTION_AUTO_CLOSE_MS = 7000` (`:13`); `closeInterventionWindow()` calls
  `getCurrentWindow().close()` (`:16-23`), which the shell turns into `hide()`.
- Data: `id === 'test'` → a local sample (`sample(t)`, `:26-38`: kind `site`, name
  `focus.intervention.sample_name` = "YouTube", message `focus.message.0`). Otherwise
  `ipc.listInterventions(50)` and `find(i => i.id === id)`, **falling back to the same
  sample** when the id is not among the newest 50 or the call fails (`:58-71`).
- Also listens to the Tauri event `intervention` and replaces the data (`:74-88`).
- Layout: `Ubi variant="auto" size={120} mood="worried"`, a `.glass` bubble with
  `data.message`, a rose `Badge` with `data.name` (icon by `data.kind === 'app'`), one primary
  `Button` "Ok, foco!" (`focus.intervention.ok`).
- It loads settings itself (`loadSettings`) so the language follows `Settings.language`.

`Intervention` on the wire (`types.ts:517-528`): `{ id, at, target_id, kind: 'app'|'site',
name, key, action: 'app_quit'|'tab_closed'|'tab_blanked'|'notified', session_id, message }`.
`message` is written by the engine in the UI language; the frontend only translates
`focus.action.<action>` (`InterventionsList.tsx:27`).

What nofap can and cannot reuse here:

- Reusable as is: the presenter port, the "re-point by hash + emit event" mechanism, the
  always-on-top/no-focus-steal window recipe, the position logic.
- Not reusable as is: the size, the 7 s auto-close, the single button, the "YouTube" sample
  fallback, the `kind`/`action` unions (a new action such as "DNS blocked" or "screen flagged"
  needs a new `InterventionAction` variant → Rust enum, `types.ts`, `focus.action.<id>` in both
  locales, `mock.ts`).
- A second window needs: a new label constant, a builder like `show_intervention_window`, a
  route in `App.tsx` outside `Gate`, a `capabilities/<label>.json` (without it the page cannot
  `listen`, close or hide itself; app commands are still callable because `build.rs` calls plain
  `tauri_build::build()` with no `AppManifest`, so every local window may invoke every command),
  and a decision on focus: the current window is built with `.focused(false)` so it never steals
  the keyboard, which is wrong for a dialog the user is meant to read and answer.

---

## 5. Tray and menus (`lib.rs:28-155`, `:343-500`, `:659-721`)

- `MenuText::for_language(UiLanguage) -> MenuText` holds every label for PT-BR and EN
  (`lib.rs:49-88`); `Menus::relabel(lang, available)` rewrites them in place (`lib.rs:111-128`);
  `relabel_menus(&AppHandle, UiLanguage)` is called by the `update_settings` command
  (`commands.rs:625`) and by `TauriSink` on `UpdateAvailable`.
- Tray: `TrayIconBuilder::with_id("main").icon(include_bytes!("../icons/tray@2x.png"))
  .icon_as_template(true).tooltip("ubiqX").menu(&menu).show_menu_on_left_click(true)`
  (`lib.rs:407-414`).
- Item ids and actions (`lib.rs:418-474`): `open` → `show_main_window`; `pause` / `resume` →
  `engine.pause()` / `engine.resume()`; `private_30|private_60|private_tomorrow|
  private_indefinite|private_off` → `engine.set_private_mode(PrivateModeDuration::…)`; `snooze`
  → `engine.snooze_nudges(120)`; `update` → check + `open_download`; `report_today` → generate a
  report per non-system category; `quit` → `engine.shutdown(); app.exit(0)`.
- App menu (macOS only, `lib.rs:613-619`): Tauri's default minus Quit; `hide_window` owns
  `CmdOrCtrl+Q` and hides the dashboard (`HIDE_WINDOW_ID`, `lib.rs:660`). On Windows/Linux the
  menu is built and dropped.

Adding one tray item touches seven places in `lib.rs`: a `MenuText` field, its two language
values, a `Menus` field, a `set_text` line in `relabel`, `MenuItem::with_id` in `build_tray`, the
`Menu::with_items` list, the `on_menu_event` arm, and the `Menus { .. }` literal.

Worth knowing for nofap:

- **Private mode is one click away in the tray** (30 min / 1 h / until tomorrow / indefinitely)
  and `pause` stops tracking. If nofap detection rides on the tracker/sampler, those two items
  are an obvious off switch; whether they also silence nofap is a design decision the shell
  will not make for you.
- On GNOME without the AppIndicator extension there is no tray at all
  (`docs/WINDOWS-LINUX.md` § 1), and since closing only hides, the user then has no way to quit.

---

## 6. IPC end to end

### 6.1 Rust side (`commands.rs`, 993 lines, 56 commands)

Conventions, all visible in the first 130 lines:

```rust
#[derive(Debug, Serialize)] pub struct IpcError { pub code: String, pub message: String }   // :16-20
impl From<CoreError> for IpcError   // :22-43  codes: storage, platform, permission, ai, ai_not_configured,
                                    //         ai_rejected, ai_refused, rate_limited, license_required, invalid, not_found, other
impl From<String> for IpcError      // :45-52  code "other"
type IpcResult<T> = Result<T, IpcError>;                                                    // :54
fn parse_date(&str) -> IpcResult<NaiveDate>      // "%Y-%m-%d"                              // :56
fn parse_ts(&str)   -> IpcResult<DateTime<Utc>>                                             // :61
async fn blocking<T, F>(engine: EngineHandle, f: F) -> IpcResult<T>
    where F: FnOnce(EngineHandle) -> CoreResult<T> + Send + 'static                         // :67-76  spawn_blocking
fn engine(state: &State<'_, AppState>) -> EngineHandle                                      // :78
fn language(e: &EngineHandle) -> UiLanguage                                                 // :83
struct Text { … } impl Text { fn for_language(lang: UiLanguage) -> Self }                   // :88-127  the commands' own PT/EN strings
```

Every command is `#[tauri::command] pub async fn name(state: State<'_, AppState>, …) ->
IpcResult<T>`. Repository work goes through `blocking(engine(&state), move |e| …)`; async
engine calls are awaited directly (`classify_now`, `generate_report`, `get_advice`,
`set_license_key`, `check_for_updates`). Dates and timestamps cross the boundary as strings.
Commands that need the app take `app: AppHandle` as an extra parameter (`get_settings`,
`update_settings`, `restart_app`, `show_window`).

Registration: one list, `tauri::generate_handler![…]` in `pub fn handler()`
(`commands.rs:934-993`). A command missing from that list compiles and fails at runtime.

The focus-guard block is the closest template (`commands.rs:801-897`):

```rust
list_installed_apps(state) -> Vec<InstalledApp>
list_known_domains(state, limit: Option<usize>) -> Vec<KnownDomain>          // default 30
list_focus_targets(state) -> Vec<FocusTarget>
add_focus_target(state, kind: FocusTargetKind, name: String, key: String) -> FocusTarget
set_focus_target_enabled(state, id: String, enabled: bool) -> FocusTarget
remove_focus_target(state, id: String) -> ()
list_interventions(state, limit: Option<usize>) -> Vec<Intervention>         // default 30
get_focus_status(state) -> FocusStatus
start_focus_session(state, task: String, minutes: u32) -> FocusSession
stop_focus_session(state) -> Option<FocusSession>
test_intervention(state) -> ()
```

`SettingsView` (`commands.rs:539-560`) is the one aggregate the UI loads first: `settings`,
`api_key_configured`, `api_key_hint`, `api_keys`, `providers`, `permissions`, `ai_health`,
`tracker_state`, `license`, `ubi_api_base`, `data_dir`, `platform`
(`std::env::consts::OS`: `macos|windows|linux`), `version`. Anything nofap needs on every page
(module on/off, protection state) either goes in here or gets its own status command, the way
focus has `get_focus_status`.

`open_external(url)` accepts only `http(s)` URLs with a host (`commands.rs:782-793`) — the path
for "find professional help" links.

### 6.2 Argument naming

Rust parameters are `snake_case`; Tauri's default converts them to camelCase on the JS side.
`ipc.ts` therefore sends `{ blockId, categoryId, includeArchived, startedAt, endedAt }`.
Struct fields are **not** converted: serde emits `snake_case`, and `types.ts` mirrors that
(`types.ts:1-2`). Enums are `#[serde(rename_all = "snake_case")]` strings; `EngineEvent` is
`#[serde(tag = "type", rename_all = "snake_case")]` (`model.rs:1053-1054`).

### 6.3 TypeScript side

`lib/ipc.ts`:

```ts
export const isTauri = (): boolean => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;   // :39
export function ipcErrorCode(e: unknown): string      // :46  Tauri rejects with {code,message}; the mock throws Error("code: …")
export function ipcErrorMessage(e: unknown): string   // :54
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T>   // :60  invoke() in Tauri, mock.handle() otherwise
export const ipc = { getDashboard, …, testIntervention }                          // :69-171
export async function onEngineEvent(handler: (e: EngineEvent) => void): Promise<() => void>   // :174  listen('engine') or mock.subscribe
```

`lib/types.ts` (569 lines) is the hand-kept mirror; the union `EngineEvent` is at `:556-569`.

### 6.4 Events

Channel `engine`, payload `EngineEvent`. Rust variants (`model.rs:1055-1097`): `BlockOpened`,
`BlockClosed`, `BlocksClassified`, `ReportReady`, `Nudge`, `TrackerState`, `AiHealth`,
`PermissionRequired { permission: String }`, `ScreenshotTaken`, `UpdateAvailable`,
`Intervention`, `FocusSession`.

Frontend path: `useEngineEvents()` (`lib/engine.ts:12-43`) is mounted once in `AppShell`,
debounces the three data events by 400 ms, calls `useAppStore.applyEvent(e)` and raises toasts
for `nudge`, `report_ready`, `permission_required`. `applyEvent` (`store.ts:224-269`) is the
switch that turns events into state; unknown types fall through `default`. A second channel,
`intervention`, carries a bare `Intervention` and is consumed only by the intervention page.

### 6.5 Recipe: add a command `nofap_status`

1. Domain type + engine method in the crates (`ubiqx-core`, `ubiqx-engine::EngineHandle`).
2. `commands.rs`: `#[tauri::command] pub async fn get_nofap_status(state: State<'_, AppState>)
   -> IpcResult<NofapStatus> { blocking(engine(&state), |e| e.nofap_status()).await }` and add
   the name to `generate_handler!`.
3. If it can fail in a new way, extend `CoreError` → the `match` at `commands.rs:24-37` is
   exhaustive and will not compile until the new variant has a code.
4. `types.ts`: `export interface NofapStatus { … }` with the exact serde field names.
5. `ipc.ts`: `getNofapStatus: () => call<NofapStatus>('get_nofap_status')`.
6. `mock.ts`: state in `State`/`freshState()`, a handler in `commands`, a hook on `__mock`.
7. `store.ts`: state + loader + (if events) a `case` in `applyEvent`; add the event to the
   `EngineEvent` union in `types.ts` and to the Rust enum.
8. Any user-facing string the command writes itself: a field on `Text` in both languages.

---

## 7. Frontend

### 7.1 Routing (`App.tsx`, 37 lines)

`HashRouter`. Three layers:

```
/intervention            <InterventionPage/>      no gate, no shell (second window)
<Gate>                                           loads settings; redirects to /onboarding until settings.onboarding_done
  /onboarding            <Onboarding/>            own full-screen layout + its own <Toaster/>
  <AppShell>                                      rail + scroll column + UpdateBanner + LicenseBanner + <Outlet/> + <Toaster/>
    index /timeline /review /reports /categories /insights /settings /focus   and  * → Dashboard
```

`Gate` (`AppShell.tsx:13-44`) shows an engine-error panel when `get_settings` fails
(`nav.engine_error_title` / `nav.engine_error_hint`). `AppShell` (`:47-85`) mounts
`useEngineEvents()`, loads the dashboard for the selected day, categories and update status.

### 7.2 Nav (`components/layout/Sidebar.tsx`)

`items: Item[]` at `:58-67`, `Item = { to, label, Icon: LucideIcon, badge?, dot?, dotLabel?,
dotTestId? }`. Order today: Hoje, Timeline, Revisão, Relatórios, Categorias, Insights,
Configurações, Foco (`Shield` icon, volt dot while a session runs). The rail is 232 px wide, 72
px (icons only) below 1180 px. The brand block is `BrandMark` + the wordmark
`ubiq<span class="text-volt">X</span>` (`:76-81`), repeated in `Onboarding.tsx:218-222`. Footer:
theme toggle + `TrackerPill`.

### 7.3 Recipe: add a page

1. `src/pages/Nofap.tsx` exporting `NofapPage`, root `<div data-testid="page-nofap">`,
   `<PageHeader title={t('nofap.title')} subtitle={t('nofap.subtitle')} />`, then the 12-column
   grid (`col-span-12 min-[1100px]:col-span-8` + aside `col-span-4`) used by `Focus.tsx:143-174`.
2. `App.tsx`: `<Route path="/nofap" element={<NofapPage />} />` inside the `AppShell` route.
3. `Sidebar.tsx`: one entry in `items` with a lucide icon and `t('nav.nofap')`.
4. `i18n/messages/nav.ts`: `nofap` in both locales; `i18n/messages/nofap.ts` + registration.
5. `scripts/screenshots.mjs`: add `['nofap', '/nofap', 'page-nofap']` to `PAGES` if it should be
   in the README captures (Focus is not in that list today).
6. A `pages/Nofap.test.tsx` in the style of `Focus.test.tsx` (see § 10.2).

### 7.4 Store (`lib/store.ts`, zustand, one store `useAppStore`)

Slices: theme; selected `date`; `settingsView` + `loadSettings` / `saveSettings(patch:
Partial<Settings>)` (optimistic merge, then `ipc.updateSettings(next)`) / `applySettingsView` /
`setLanguage`; `license` + `loadLicense` / `setLicenseKey`; categories; `dashboards` cache;
live `trackerState`, `aiHealth`, nudges; `dataVersion` / `reportsVersion` counters; `ubiSpeech`;
updates; in-app install; focus (`focusStatus`, `focusTargets`, `interventions`,
`installedApps`, `knownDomains`, `loadFocusStatus`, `loadFocus`, `loadFocusCatalog`,
`addFocusTarget`, `setFocusTargetEnabled` (optimistic with rollback), `removeFocusTarget`,
`startFocusSession`, `stopFocusSession`).

Pattern for a status the rail depends on: a cheap loader called once on mount in `Sidebar`
(`loadFocusStatus`, `Sidebar.tsx:54-56`) and kept fresh by events. Loaders that may hit an
engine without the command swallow the error and return `null` (`store.ts:343-352`).

`lib/useAsync.ts` is the small fetch hook for page-local data (`useAsync(fn, deps)` →
`{ data, loading, error, reload, setData }`). `lib/toast.tsx`: `useToast().success|error|info(
title, message?)`, at most 4 toasts, 3.5 s (6 s for errors).

### 7.5 `pages/Focus.tsx` (180 lines) — the closest existing page

Composition: `SessionCard`, a `Card` with `CardHeader` + `TargetSearch` + guard-off notice +
`TargetList` or `EmptyState`, a `Card` with `InterventionsList`, and `FocusOptions` in the
aside; `DisableWarnings` at the end. Settings are patched with
`saveSettings({ focus: { ...focus, ...patch } })`.

`components/focus/DisableWarnings.tsx` is directly relevant to "make it hard to switch off":
`RECENT_BLOCK_MS = 15 * 60_000`; a target blocked in the last 15 minutes gets three steps
(`focus.warnings.1..3`) in a small `Dialog` with a flat worried UBI, where the **primary** button
is always "Manter o bloqueio" and continuing is a `ghost` button; otherwise one confirmation.
The copy of those three warnings is motivational ("Não desista dos seus sonhos") — the tone the
nofap brief asks to keep supportive and non-shaming; write new copy rather than reuse the keys.

`PRAISE_MIN_MINUTES = 5` is duplicated here from the engine (`Focus.tsx:22`), an example of the
"constants mirrored by hand" habit.

### 7.6 `pages/Settings.tsx` (1377 lines)

- `SECTIONS: { id, key, Icon }[]` (`:61-72`): `geral`, `licenca` (`LICENSE_SECTION_ID`), `ia`,
  `rastreamento`, `privacidade`, `relatorios`, `ubi`, `atualizacoes` (`UPDATES_SECTION_ID`),
  `permissoes` (macOS only), `sobre`. Ids are stable Portuguese anchors (`sec-<id>`); labels are
  `t('settings.section.<key>')`.
- `useSettingsDraft()` (`:77-114`): local draft + `patch(p)` debounced 500 ms into
  `saveSettings`, with a `saving/saved/error` indicator in the header.
- `Section({ id, title, description?, action?, children })` (`:233-246`) is the card wrapper;
  helpers `Divider`, `Emphasis` (renders `<b>…</b>` inside one translated string),
  `NumberField`, `RadioRow`.
- Deep link: `navigate('/settings', { state: { section: '<id>' } })` scrolls to that section
  (`:155-160`).
- `PrivacySection` (`:760-867`) already owns everything adjacent to nofap: the "what leaves
  this computer" panel with `AiSentDialog` (the audit list of `ai_payload` per block, `:882-912`),
  private mode, `vision_policy` (`never | only_apps | all_except_blocked`), `vision_denied_apps`,
  `blocked_apps`, `blocked_domains` (`TagInput`s), screenshot interval / retention / max edge and
  `keep_screenshots_for_review`. **`blocked_apps` / `blocked_domains` here mean "private, never
  recorded"**, the opposite of nofap's "blocked, never reachable"; the copy must keep the two
  apart.
- Exported and reused by onboarding: `LicenseKeyForm`, `ApiKeyForm`, `ProviderPicker`,
  `PermissionRows`, `providerSwitchPatch`, `planLabel`, `fmtLicenseDate`, `licenseChip`,
  `PLATFORM_NAMES`, `INSTALL_NOTES`, `REPO_URL`.
- OS-specific copy convention: `settings.privacy.description` vs `…description.other`
  (`:791`), `keyStoreHintKey(platform)` (`:35`).

Adding a section: an entry in `SECTIONS`, a component `<Section id="…">`, its line in the
render list (`:215-224`), `section.<key>` and the section's keys in `settings.ts` (both locales).

### 7.7 `pages/Onboarding.tsx` (734 lines)

`ALL_STEPS = [intro, ai, perms, cats, times, vision, finish]` (`:26-34`); `stepsFor(platform)`
drops `perms` off macOS (`:39`); `platformKey(key, platform)` appends `.other` off macOS (`:42`);
`?step=N` opens a step (`:45-52`). Each step is a function component; titles and UBI's rail
line come from `onboarding.step.<id>` and `onboarding.rail.<id>`. `next()` saves categories on
`cats` and settings on `times`/`vision`; `finish()` saves `{ ...draft, onboarding_done: true }`
and navigates to `/`.

Everything before `onboarding_done` is consent-gated: docs/ARCHITECTURE.md § 7 states that
tracking, screenshots and every remote call require it, and the shell shows the window on launch
until it is set (`lib.rs:623`). The intro step already lists "what is logged / screenshots / what
goes to the AI / where it is stored" (`:329-355`) and the `vision` step is the explicit choice
about sending screenshots (`:638-676`). A nofap consent step slots in as one more entry of
`ALL_STEPS` plus `onboarding.step.<id>` / `onboarding.rail.<id>` keys; the existing test asserts
"Passo 1 de 7" (`Onboarding.test.tsx`), so the count changes there.

A standalone product needs a different step list altogether (no categories, no report time),
which argues for making the step list data rather than a module constant.

---

## 8. i18n conventions

Files: `src/i18n/{index.ts,locale.ts,types.ts}` and one file per namespace in
`src/i18n/messages/`: `common, nav, ui, charts, ubi, icons, dashboard, timeline, review, reports,
categories, insights, settings, onboarding, updates, focus`.

Shape of a namespace file:

```ts
import type { NamespaceMessages } from '../types';
// what it covers. t('focus.<key>').
const messages = {
  'pt-BR': { title: 'Foco', 'session.title': 'Sessão de foco', 'targets.blocked_count_one': '{count} bloqueio', 'targets.blocked_count_other': '{count} bloqueios', … },
  en:      { title: 'Focus', … },
} satisfies NamespaceMessages;
export default messages;
```

Rules (from `index.ts`, `locale.ts`, `parity.test.ts`):

- Lookup key = `<namespace>.<key>`; keys inside a file are flat strings, dot-separated by
  convention (`'session.start_failed'`), never nested objects.
- Placeholders are `{name}`; `t(key, { count })` picks `key_one` when `count === 1`, else
  `key_other`, when those exist (`index.ts:72-88`).
- `Locale = 'pt-BR' | 'en'`. `normaliseLocale` maps anything starting with `en` to `en`,
  everything else to `pt-BR` (`locale.ts:15-19`). Initial locale: `?lang=` > localStorage
  `ubiqx.locale` > `navigator.language` > `pt-BR`; `Settings.language` then wins once settings
  load, unless `?lang=` is present (`store.ts:156-160`).
- `useT()` in components (re-renders on language change); `t()` in plain modules; `hasKey()`
  for optional keys.
- A missing key renders the key itself and warns once in dev.
- Parity test, per namespace: both locales present; identical key sets; identical placeholder
  sets; every `_one` has an `_other`; no empty strings; **no ALL-CAPS strings** (more than three
  letters, all upper case).
- Variants by OS: suffix `.other` (Windows + Linux together) or `.<platform>`.
- Enum labels by template: `t(`focus.action.${i.action}`)`, `t(`common.tracker.${state}`)`,
  `t(`settings.license.plan.${plan}`)`. Adding an enum value without its key shows the raw key.
- Tests pin `pt-BR` (`src/test/setup.ts`) and assert Portuguese copy literally.

Rust-side strings that are **not** in these files: tray/app menu (`lib.rs` `MenuText`), command
messages (`commands.rs` `Text`), `Info.plist` usage strings (Portuguese only), engine-written
text (intervention lines `MESSAGES_PT`/EN in `ubiqx-core/src/focus.rs`, nudges, AI-health
reasons, `managed_budget_reached_text` in `ubiqx-engine/src/license.rs:148`). `focus.message.0..4`
in `focus.ts` is a hand-kept copy of the engine's list for the mock. Counselling copy written by
the engine will follow the same split: source of truth in Rust, a mirror for the mock.

Design-contract copy rules (`DESIGN.md` § 2, § 6): pt-BR, sentence case, verbs first, plain
words, no "A · B" middle-dot strings, no "WORD — fragment" labels, no arrows in button text;
errors say what to do next; empty states invite the next action.

---

## 9. Design tokens and components to reuse

Source of truth: `apps/desktop/DESIGN.md` ("instrument panel at night") and `src/index.css`.
Never raw hex in components (category colours excepted).

Tokens (`index.css:12-73`, exposed as Tailwind colours at `:110-144`):

| Token | Role | Dark | Light |
|---|---|---|---|
| `canvas` / `panel` / `panel-2` / `panel-3` | background / card / nested, input / track | `#060a14` `#0c1220` `#121a2b` `#18213a` | `#f3f5fa` `#fff` `#f6f8fc` `#eceff6` |
| `line` / `line-2` | hairline / stronger hairline | 14 % / 28 % of `rgb(126 158 214)` | 8 % / 16 % of slate |
| `ink` / `ink-2` / `ink-3` / `ink-4` | text primary / secondary / tertiary / non-text marks | `#eaf0fa` `#9daec7` `#7487a6` `#5f6f8c` | `#0f172a` `#475569` `#5b6b85` `#8b98b0` |
| `volt` (+ `volt-soft`, `on-volt`) | primary, focus, links | `#4d8dff` | `#2a62d6` |
| `ember` (+ `ember-soft`, `on-ember`) | attention, rationed | `#ff7a1f` | `#c2410c` |
| `signal` / `rose` / `violet` / `amber` | productive / distraction, danger / pauses, private / caution | `#2ee6a6` `#ff5c7a` `#9b8cff` `#ffc24d` | `#0e9f6e` `#e11d48` `#6d5bd0` `#a3660f` |

Radii `rounded-shell` 20, `rounded-card` 14, `rounded-control` 10, `rounded-pill`. Fonts: Sora
(`.display`) + Inter, self-hosted. Classes: `.panel`, `.panel-raised`, `.glass` (floating only),
`.display`, `.num` (tabular figures on every number), `.eyebrow`, `.glow-volt` (dial, active nav,
primary buttons only), `.glow-ember`, `.scroll-thin`, `.md`. No shadows on cards; `--hero-glow`
only behind the Hoje hero, the onboarding mascot and the intervention panel. Icons: lucide,
`strokeWidth={1.75}`, 18 px in nav/buttons, 14 px in chips, decorative ones `aria-hidden`.
Motion: `useReducedMotion()` guards every animation; dialog 180 ms in.

Semantic colour rule that matters for a sensitive module: `rose` is "distraction, danger"; the
existing intervention chip is a rose `Badge`. For non-shaming UI prefer `volt`/`signal`/neutral
and keep `rose` for destructive actions.

Components (`src/components/ui`, signatures as exported):

- `Card({ padded = true, tone = 'panel' | 'raised' | 'plain', …div })`,
  `CardHeader({ title, subtitle?, action?, className? })`,
  `StatTile({ label, value, hint?, accent?, icon?, className? })`
- `Button({ variant = 'secondary' | 'primary' | 'ghost' | 'danger' | 'accent', size = 'md' | 'sm' | 'lg', loading?, icon? })`,
  `IconButton({ label, size?, active? })`
- `Badge({ tone = 'neutral' | 'volt' | 'ember' | 'signal' | 'rose' | 'violet' | 'amber' …, title? })`,
  `StatusPill({ color, pulse? })`
- `Field({ label, hint?, inline?, children: (id) => node })`, `Input`, `Select`, `Textarea`
- `Toggle({ checked, onChange, id?, disabled?, label?, size? })` (role `switch`)
- `Dialog({ open, onClose, title, description?, footer?, width = 'md' | 'sm' | 'lg' })`
- `Popover`, `TagInput({ value, onChange, placeholder?, id?, disabled? })`, `DayNav`, `PageHeader({ title, subtitle?, actions? })`
- `misc`: `Progress`, `Spinner`, `EmptyState({ icon?, title, description?, action? })`, `Tabs`, `Kbd`, `Skeleton`, `SectionTitle`
- `UsageBar({ label, value, spent, budget, hint? })`

Focus components that generalise: `focus/TargetSearch` (combobox over installed apps + known
domains + "block the site <domain>"), `focus/TargetList`, `focus/InterventionsList({
interventions, limit = 12 })`, `focus/DisableWarnings`, `focus/DurationChips`,
`focus/useCountdown`.

UBI: `Ubi({ mood: 'sleeping'|'calm'|'focused'|'excited'|'worried', size?, speaking?, variant?:
'auto'|'flat'|'svg', crop?: 'full'|'head' })`. `speaking` renders the glass speech bubble.
`auto` tries the 3D model (lazy chunk with three.js) → PNG → SVG; `flat` skips WebGL (use it in
dialogs and small windows). `UbiCard({ data, nudge })` is the assistant strip on Hoje;
`NUDGE_ICON: Record<NudgeKind, LucideIcon>` (`UbiCard.tsx:9-18`) is exhaustive, so a new
`NudgeKind` does not compile until it has an icon. `ubi/FocusPrompt` shows how a nudge turns the
bubble into a small form. The mascot has no neutral "supportive" mood: the five moods map to
glow colours volt/signal/ember/rose/grey (`ubi/moods.ts:6-12`), and `worried` (rose) is what the
existing interventions use.

Where UBI's speech comes from on Hoje (`Dashboard.tsx:178`): `ubiSpeech ?? nudge?.message ??
t('ubi.tip.<mood>')`; `ubiSpeech` is set by the `nudge` event (`store.ts:231-233`). A nofap
nudge emitted as `EngineEvent::Nudge` therefore appears in the bubble, in a toast
(`engine.ts:28`) and in `UbiCard` with no frontend work beyond the new kind — which also means
it appears on a screen someone else may be looking at. Decide what the nudge title/message may
contain.

---

## 10. Mock / browser mode, tests, screenshots

### 10.1 `lib/mock.ts` (2266 lines)

Used whenever `isTauri()` is false: `pnpm dev` in a browser, Vitest, Playwright captures.
`handle<T>(cmd, args)` (`:2155-2162`) looks the command up in `const commands: Record<string,
Cmd>` (`:1727-2153`), adds 90 ms + jitter latency outside tests (`LATENCY_MS`, `:64`) and
returns a `structuredClone`. Errors are `throw new Error('<code>: message')` so
`ipcErrorCode` can read the code.

State: `interface State` (`:999-1018`) built by `freshState()` (`:1028-1051`);
`defaultSettings()` (`:525-577`) is a full `Settings` literal that must gain every new field;
`subscribe(handler)` (`:1672`) feeds fake `block_opened` / `nudge` events every 20 s outside
tests; `emit(e)` (`:1627`).

URL switches read at module load: `?theme=`, `?lang=`, `?onboarding=1`, `?step=N`,
`?platform=windows|linux|macos`, `?update=available`, `?session=active`,
`?nudge=focus_prompt`, `?license=annual|managed|expired`.

Test hooks: `export const __mock = { reset, setOnboardingDone, setPlatform, setUpdate,
sampleRelease, setUpdateInstall, setFocus, intervene, sampleSession, setLicense,
sampleLicenseKeys, state }` (`:2168-2248`), a subset mirrored on `window.__ubiqxMock`
(`:2264-2266`). `openInterventionPreview(id)` opens `#/intervention?id=…` as a 460×188 popup in
the browser (`:1612-1616`).

The mock re-implements behaviour, not just data (licence parsing with base32, model
reconciliation, focus sessions, interventions). A nofap page needs the same treatment: seeded
state, handlers, `__mock.setNofap(...)`, and ideally a `?nofap=` switch for captures.

### 10.2 Tests (Vitest 5 + Testing Library + jsdom)

Config in `vite.config.ts:50-55`: `environment: 'jsdom'`, `setupFiles: ['./src/test/setup.ts']`,
`include: ['src/**/*.test.tsx', 'src/**/*.test.ts']`, `css: false`. `setup.ts` pins the locale
to `pt-BR` before and after each test and stubs `ResizeObserver`, `matchMedia`,
`HTMLCanvasElement.getContext` (returns null, so UBI falls back to SVG) and `scrollIntoView`.

Page-test pattern (`Focus.test.tsx:1-27`):

```ts
vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
const resetStore = async () => {
  __mock.reset();
  useToastStore.setState({ toasts: [] });
  useAppStore.setState({ settingsView: null, focusStatus: null, focusTargets: [], interventions: [], … });
  await useAppStore.getState().loadSettings();
};
render(<MemoryRouter><FocusPage /></MemoryRouter>);
expect(await screen.findByRole('heading', { name: 'Foco' })).toBeInTheDocument();
```

The store is a module singleton, so each test resets the slices it touches by hand. Tests assert
Portuguese strings and `data-testid`s; switch to English with `setLocale('en')` plus
`__mock.state().settings.language = 'en'`.

Existing suites worth copying from: `Focus.test.tsx`, `Intervention.test.tsx`,
`components/focus/*.test.tsx`, `LicenseBanner.test.tsx`, `Settings.test.tsx`,
`Onboarding.test.tsx`, `lib/mock.test.ts`, `i18n/parity.test.ts`, `components/ubi/csp.test.ts`
(reads `src-tauri/tauri.conf.json?raw`).

There are **no Rust tests in `apps/desktop/src-tauri`** and no end-to-end test of the Tauri
shell: `lib.rs` and `commands.rs` are only compiled (macOS job runs `cargo test --workspace`,
Windows runs `cargo clippy -p ubiqx-desktop -- -D warnings`, Linux builds the bundle). Keep
shell code thin; put logic where `cargo test --workspace --exclude ubiqx-desktop` reaches it.

Commands: `pnpm typecheck` (`tsc --noEmit`, `strict`, `noUnusedLocals`, `noUnusedParameters`),
`pnpm test -- --run`, `pnpm build` (typecheck + vite build).

### 10.3 Screenshots

`scripts/screenshots.mjs` (Playwright Chromium, needs `pnpm dev` on :1420) captures `PAGES`
(dashboard, timeline, review, reports, categories, insights, settings, onboarding) at 1440×900
in light and dark into `docs/screenshots/<name>[-dark].png`; flags `--url`, `--out`, `--lang
en|pt-BR`, `--themes`. It waits for `[data-testid="<page-…>"]`. `scripts/hero.mjs` renders the
mascot for the README; `scripts/gen-tray-icon.mjs` regenerates `tray.png`, `tray@2x.png`,
`app-icon.png` from signed-distance shapes (no dependencies) — `pnpm tauri icon app-icon.png`
then produces the rest of `icons/`.

---

## 11. Build, bundle, CI, release

### 11.1 `tauri.conf.json`

`productName: "ubiqX"`, `version: "0.1.0"` (rewritten by CI), `identifier: "ai.ubiqx.app"`;
`build`: `beforeDevCommand: "pnpm dev"`, `devUrl: http://localhost:1420`, `beforeBuildCommand:
"pnpm build"`, `frontendDist: "../dist"`; `bundle.targets: [app, dmg, msi, nsis, appimage,
deb]`, `createUpdaterArtifacts: "v1Compatible"`, icons `32x32.png, 128x128.png,
128x128@2x.png, icon.icns, icon.ico`; macOS `minimumSystemVersion: "13.0"`; Windows
`webviewInstallMode: downloadBootstrapper (silent)`, WiX languages `pt-BR, en-US`, NSIS
languages `PortugueseBR, English`, `displayLanguageSelector: true`, `installMode:
"currentUser"`; Linux deb depends `libwebkit2gtk-4.1-0, libgtk-3-0,
libayatana-appindicator3-1 | libappindicator3-1, librsvg2-2`, AppImage `bundleMediaFramework:
false`; `plugins.updater`: `pubkey` (minisign, base64), `endpoints:
["https://github.com/andreyquadros/ubiquitous-engine/releases/download/continuous/updater.json"]`,
`windows.installMode: "passive"`.

### 11.2 Two feeds

- **`latest.json`** (announcement; schema 1, `product`, `version`, `build{epoch,number,sha,
  branch}`, `published_at`, `notes`, `release_url`, `platforms{"<os>-<arch>": {url, kind, size,
  app_zip_url, alternates?}}`): read by the Rust checker (`ubiqx-core::update`,
  `ubiqx-engine::update`) from `UBIQX_UPDATE_FEED_URL`; drives the banner, the tray item and
  Settings › Atualizações. Checked 45 s after start and every 6 h (`types.ts:289`).
- **`updater.json`** (install; the `tauri-plugin-updater` static format with minisign
  signatures): read by the plugin from `plugins.updater.endpoints`; drives "Atualizar agora"
  (`lib/updater.ts`, `store.installUpdate`).

### 11.3 CI (`.github/workflows/ci.yml`)

Triggers: every push to any branch and every PR, except changes under `video/**`; concurrency
group `ci-${{ github.ref }}` with cancel-in-progress. Env: `TAURI_SIGNING_PRIVATE_KEY`,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (repository secrets).

| Job | Runner | What |
|---|---|---|
| `rust-linux` | ubuntu-latest | `cargo fmt --all -- --check`; `cargo clippy --workspace --exclude ubiqx-desktop --all-targets -- -D warnings`; `cargo test --workspace --exclude ubiqx-desktop`; `cargo check -p ubiqx-platform --target aarch64-apple-darwin` and `--target x86_64-pc-windows-msvc`; fmt/clippy/test of `services/ubi-api` |
| `frontend` | ubuntu-latest | pnpm 10 / Node 22; `pnpm install --frozen-lockfile`, `pnpm typecheck`, `pnpm test -- --run`, `pnpm build`; `node --test scripts/publish-release.test.mjs`; `node --test scripts/ubi-model.test.mjs` |
| `macos-app` | macos-latest (arm64) | build identity, `ci-app-version.sh`, `ci-updater-signing.sh`, `cargo test --workspace`, `pnpm tauri build --bundles app,dmg $UBIQX_TAURI_SIGN_FLAG` with `APPLE_SIGNING_IDENTITY: "-"`, ditto-zip, rename to stable names, upload artifact `ubiqX-macos-app` |
| `windows-app` | windows-latest (bash) | same preamble, `cargo test --workspace --exclude ubiqx-desktop`, `cargo clippy -p ubiqx-desktop -- -D warnings`, `pnpm tauri build --bundles msi,nsis`, rename (pt-BR MSI is the published one), artifact `ubiqX-windows-app` |
| `linux-app` | ubuntu-22.04 | apt webkit2gtk-4.1/appindicator/rsvg/patchelf/xdo/gtk3/dbus/ssl/fuse2, `pnpm tauri build --bundles appimage,deb`, rename, artifact `ubiqX-linux-app` |
| `publish-continuous` | ubuntu-latest | `if: github.event_name == 'push'`; downloads `pattern: ubiqX-*-app`; `node scripts/publish-release.mjs --strict --version "$UBIQX_VERSION" --asset …` for every `find dist -type f -name 'ubiqX-*' ! -name '*.sig'` |

Stable asset names (the only names the feeds, the site and the docs know):
`ubiqX-macos-<arch>.dmg`, `.app.zip`, `.app.tar.gz(+.sig)`; `ubiqX-windows-<arch>-setup.exe`,
`.msi`, `-setup.nsis.zip(+.sig)`; `ubiqX-linux-<arch>.AppImage`, `.deb`,
`.AppImage.tar.gz(+.sig)`.

The app jobs locate bundle outputs with globs over the shared workspace target dir, e.g.
`ls target/release/bundle/dmg/*.dmg | head -n 1` (`ci.yml:151`),
`target/release/bundle/macos/ubiqX.app` (`:150`), `…/msi/*pt-BR*.msi`, `…/nsis/*-setup.exe`,
`…/appimage/*.AppImage`, `…/deb/*.deb`. Two apps bundled in one job would both land in those
directories and make the globs ambiguous.

Scripts:

- `scripts/build-info.sh` prints `export UBIQX_BUILD_{EPOCH,NUMBER,SHA,BRANCH}` (unshallows
  first).
- `scripts/app-version.mjs` computes `<major>.<minor>.<UBIQX_BUILD_NUMBER>` from
  `[workspace.package] version` and, with `--write`, rewrites the `"version"` of the one file
  named by `TAURI_CONF = 'apps/desktop/src-tauri/tauri.conf.json'` (`:35`).
  `scripts/ci-app-version.sh` wraps it and exports `UBIQX_VERSION`.
- `scripts/ci-updater-signing.sh` exports `UBIQX_TAURI_SIGN_FLAG` (`--no-sign` without the two
  secrets) and `UBIQX_UPDATER_SIGNED`; unsigned updater artifacts are deleted before upload.
- `scripts/publish-release.mjs` (711 lines, tested by `publish-release.test.mjs` against a mock
  GitHub API): moves the tag to the built commit, rewrites the release, replaces assets by
  name, uploads `latest.json` and (when updater artifacts exist) `updater.json`. Hardcoded
  product facts: `DEFAULTS = { tag: 'continuous', releaseName: 'ubiqX (build contínuo)',
  product: 'ubiqX', … }` (`:49-62`), `ASSET_KINDS[*].stableName = ubiqX-<os>-<arch>…`
  (`:257-270`), `STABLE_NAME = /^ubiqX-(macos|windows|linux)-…/` (`:272`),
  `FEED_ASSET_NAME = 'latest.json'` (`:361`), `UPDATER_ASSET_NAME = 'updater.json'` (`:375`),
  and `productVersion()` reading `apps/desktop/src-tauri/tauri.conf.json` (`:201`). `--tag` is
  the only product-ish option. Feeds of the same build epoch are merged per platform key; an
  older build never overwrites a newer feed. Assets it does not upload are left in the release.
- `scripts/codesign-dev.sh` signs a local macOS bundle with a self-signed "ubiqX Dev" identity
  so TCC grants survive rebuilds.

`codemagic.yaml`: one macOS workflow (`mac_mini_m2`) on `main` and `claude/*`, same steps,
optional groups `ubiqx_apple`, `ubiqx_github`, `ubiqx_updater`; publishes with
`--dmg … --app-zip …` into the same release when `GITHUB_TOKEN` is set.

Other consumers of the stable names: `site/src/config.ts` (`DOWNLOAD_URLS`,
`ALT_DOWNLOAD_URLS`, `MACOS_FIX_COMMANDS`, `LINUX_INSTALL_COMMANDS`),
`pages/Settings.tsx:1007-1018` (`INSTALL_NOTES`), `mock.ts:736-737`, `docs/WINDOWS-LINUX.md`,
`docs/MACOS-TESTING.md`.

### 11.4 What macOS ad-hoc signing costs

`docs/MACOS-TESTING.md` § 3.2 and `lib/updater.ts:96-101` (`losesMacPermissions`): every
auto-update replaces the bundle with a differently-signed one, so macOS asks for Screen
Recording and Automation again. The same holds for any Keychain ACL and for anything else the
OS ties to code identity. A nofap layer that depends on a granted permission or an
"always allow" Keychain answer loses it on each update until there is a Developer ID.

---

## 12. Licensing

### 12.1 What exists

`crates/ubiqx-core/src/license.rs` (pure, shared by app and proxy):

```rust
pub const UBIQX_LICENSE_PUBKEY_HEX: &str = "ba31f58756b2ea408d3ea7fa8116ca3a802aff3986919492f482ea7bf73e2282";  // dev pair
pub const KEY_PREFIX: &str = "UBIQX-";
pub const CLAIMS_VERSION: u8 = 1;
pub const KEY_HINT_LEN: usize = 4;
pub const LICENSE_ENFORCEMENT: LicenseEnforcement = LicenseEnforcement::Soft;
pub const UBIQX_API_BASE_DEFAULT: &str = "https://api.ubiqx.ai";
pub const UBI_MODEL_FAST: &str = "ubi-fast";   pub const UBI_MODEL_SMART: &str = "ubi-smart";
pub const HEADER_UBIQX_PLAN: &str = "x-ubiqx-plan";   pub const HEADER_UBIQX_COST_USD: &str = "x-ubiqx-cost-usd";

pub enum Plan { AnnualOwnKey, MonthlyManaged }      // ids "annual_own_key" | "monthly_managed"; Plan::ALL, id(), parse(), is_managed()
#[serde(deny_unknown_fields)]
pub struct LicenseClaims { v: u8, plan: Plan, sub: String, email_hash: String, issued_at: i64, expires_at: i64, seats: u32 }
impl LicenseClaims { fn canonical_json(&self) -> Vec<u8> }   // hand-formatted: email_hash, expires_at, issued_at, plan, seats, sub, v
pub fn parse_key(raw) -> Result<ParsedKey, LicenseError>      // rejects v != 1 (UnsupportedVersion) and non-canonical payloads
pub fn verify_key_with(raw, pubkey_hex) -> Result<LicenseClaims, LicenseError>     // signature only; expiry is the caller's
pub enum LicenseState { Unlicensed, Valid, Expired, Invalid }
pub struct LicenseStatus { state, plan: Option<Plan>, expires_at: Option<String>, days_left: Option<i64>,
                           key_hint: Option<String>, enforcement: LicenseEnforcement, managed_usage: Option<ManagedUsage> }
impl LicenseStatus { evaluate(key, now), evaluate_with(key, pubkey_hex, now), is_valid(), allows_managed_ai(), blocks_ai() }
```

Key format `UBIQX-<base32(canonical claims)>-<base32(Ed25519 signature)>`; verification is
offline. The key is stored in the OS secret store under `LICENSE_SECRET_KEY = "ubiqx.license"`
(`model.rs:572`), which is also the credential of the managed provider
(`AiProvider::Ubi.secret_key()`, `model.rs:659`).

`crates/ubiqx-engine/src/license.rs`: `stored_key`, `evaluate`, `reevaluate`, `current`
(re-verifies when the cached expiry has passed), `refresh_managed_usage` (asks the proxy),
`set_key` (stores any non-blank key, even an invalid one), `managed_budget_exhausted`.

What the licence gates today:

- Selecting the managed provider requires a valid `monthly_managed` key
  (`CoreError::LicenseRequired` → IPC code `license_required`), in every enforcement mode.
- Under `Hard` (not the current build): `EngineState::license_gate()` (`state.rs:106`) guards
  the AI entry points (`handle.rs:233,244,252`) and `remote_allowed()` (`state.rs:228`) stops
  background AI calls (`classify.rs:371`, `reports.rs:141`, `handle.rs:265`).
- Under `Soft` (current): nothing else. `LicenseBanner` reminds every 7 days
  (`LICENSE_NAG_KEY = 'ubiqx.license_nag'`, `LICENSE_NAG_DAYS = 7`).
- Never gated: tracking, timeline, manual categorisation — **and the focus guard**: no focus
  command or guard path consults the licence.

UI: `LicenseBanner` + `LicenseRequiredDialog` + `useLicenseGate()` (`components/layout/
LicenseBanner.tsx`), `LicenseSection` and `LicenseKeyForm` (`Settings.tsx:306-453`), the two
plan cards in onboarding (`Onboarding.tsx:383-535`), helpers in `lib/providers.ts`
(`PLAN_IDS`, `licenseAllowsManaged`, `licenseBlocksAi`, `SITE_URL =
'https://andreyquadros.github.io/ubiquitous-engine/#planos'`, `LICENSE_KEY_PREFIX = 'UBIQX-'`).
Prices live only in copy: `common.license.banner.*`, `settings.license.price.<plan>`,
`settings.license.plans_line`, onboarding `ai.*.price` (R$ 197/year or 10× R$ 25; R$ 49/month).

`services/ubi-api` (own Cargo workspace, depends on `ubiqx-core` with feature `sign`):

- `issue.rs`: `keygen`, `derive_sub(external_id, email_hash)`, `claims_for(plan, sub,
  email_hash, months, now)` (always `seats: 1`), `issue(claims, signing_key)`.
- `routes.rs`: `verdict()` (`:189-204`) → `Expired | Revoked | NotManaged | Valid`;
  `authorize_managed()` (`:207-247`) accepts only `Plan::MonthlyManaged` (`403
  plan_not_managed` otherwise); `GET /v1/license/status` (`:269-321`); `POST /v1/messages`
  (`:324…`) also rejects any `x-ubiqx-plan` header other than `monthly_managed` (`:331-336`) and
  any model other than the two aliases; admin issue (`:573-640`) and the generic payment
  webhook (`:901…`) default `months` by `match plan { AnnualOwnKey => 12, MonthlyManaged => 1 }`
  (`:621-622`, `:969-970`).
- Plan ids as bare strings: `db.rs:401` (`expiring_window_days`), `db.rs:415-417` (stats),
  `panel.html:193-194, 243, 247, 397`, `bin/ubi-license.rs:81`.
- No request/response body logging in `routes.rs` (no `tracing::` calls there); the usage
  ledger stores `sub, month, model, tokens, cost`.

`docs/LICENSING.md` matches the code. Checkout is not live: `site/src/config.ts:5-6` has empty
`CHECKOUT_ANNUAL_URL` / `CHECKOUT_MONTHLY_URL`.

### 12.2 What a second product changes

The question has three independent parts: what a key says, what each app accepts, and what the
proxy serves.

**(a) What a key says.** Options, cheapest first:

1. *Nothing changes.* The two existing plans unlock both apps. No code in `license.rs`, no
   proxy change, existing keys keep working everywhere. Cannot sell nofap on its own or price
   it differently.
2. *New `Plan` variants* (for example a nofap-only own-key plan and/or a nofap managed plan).
   Old builds fail to deserialise the unknown variant → `LicenseError::Malformed` →
   `LicenseState::Invalid` — acceptable for a nofap-only key pasted into an old ubiqX. New
   builds need a notion of "valid for this product", because today `is_valid()` is
   product-agnostic. Exhaustive matches that will stop compiling (good): `Plan::id`,
   `Plan::is_managed`, `routes.rs:621`, `:969`. Places that will silently mis-handle a new id
   (bad): `Plan::ALL` and `Plan::parse` aliases, `db.rs:401`, `db.rs:415-417`, `panel.html`,
   `ubi-license.rs` help text, frontend `Plan` union (`types.ts:166`), `PLAN_IDS`
   (`providers.ts:28`), `mock.ts:929` `isPlan`, i18n `settings.license.plan.<id>` /
   `license.price.<id>` (a missing key renders the key).
3. *A product claim* (`product` or `products`) with `CLAIMS_VERSION = 2`. Old builds reject v2
   keys outright (`UnsupportedVersion` → `invalid`), so v1 must stay accepted and mean "ubiqX".
   `deny_unknown_fields`, the fixed `canonical_json()` and the check `claims.canonical_json()
   != payload` (`license.rs:270`) all have to become version-aware; the frontend mock's
   `parseLicenseKey` (`mock.ts:932`) mirrors the claims too.

Whichever is chosen, `seats` is always 1 and nothing counts devices, so "one licence, both
apps on one machine" is already how the system behaves.

**(b) What each app accepts.** There is no product identity in the engine today
(`LicenseDeps { pubkey_hex, api_base, server }`). A standalone app needs one of: a
`product` in `LicenseDeps`/`AppConfig` consulted by `LicenseStatus` (or by a new
`allows(product)` helper), or a compile-time constant like `LICENSE_ENFORCEMENT`. Also decide
what the licence gates in nofap at all: with `Soft` enforcement nothing is gated, and gating a
self-control tool's blocking behind payment has its own product risk (a lapsed subscription
removing the block). The existing rule "tracking and manual work are never gated; only AI is"
maps naturally to "DNS blocking is never gated; AI vision may be".

Shared secret store: both apps would read `ai.ubiqx` / `ubiqx.license` (and the vendor API
keys). That is convenient (enter the key once) but couples them (removing the key in one app
removes it for the other; on macOS each differently-signed app triggers a Keychain prompt). The
file fallback (`<data_dir>/secrets.json`) is per app. Make the service name a parameter and
choose deliberately.

**(c) What the proxy serves.** If nofap's vision checks may run through "IA do Ubi", the proxy
must accept the plan (`authorize_managed`, the `x-ubiqx-plan` check, `HEADER_UBIQX_PLAN` sent by
the client) and the budget must fit a vision-heavy workload (`UBI_MONTHLY_BUDGET…` per
subscriber; the engine pauses AI when `managed_usage` says the month is spent,
`license.rs:163-167`). It also means the most sensitive screenshots the app can take would
transit the operator's server; the passthrough does not log bodies, but that is a statement
about today's code, not a guarantee, and belongs in the consent copy for that plan.

UI work for any option beyond 1: plan labels and prices in `settings.ts` and `common.ts`,
`LicenseSection`, the onboarding plan cards, `LicenseBanner` text (names ubiqX and both prices),
`SITE_URL`, the site's Pricing section, `docs/LICENSING.md`, the admin panel.

---

## 13. A second Tauri app in the workspace

### 13.1 What it needs, whatever the layout

Identity

- `productName`, `identifier` (drives the data dir, the webview profile, the autostart entry,
  the Windows uninstall key, the deb package name), window `title`, `mainBinaryName` or a
  distinct Cargo package name (today the binary is `ubiqx-desktop`; `SELF_STEMS` lists it).
- Icons: the full `icons/` set (`pnpm tauri icon <1024 png>`) plus `tray.png` / `tray@2x.png`,
  which `lib.rs:407` embeds with `include_bytes!`.
- Strings in the shell: tray tooltip `"ubiqX"` (`lib.rs:412`), intervention window title
  (`:299`), `MenuText` ("Abrir ubiqX", "Sair do ubiqX", "Pausar rastreamento", "Modo privado",
  "Gerar relatórios de hoje" — half of the tray makes no sense without tracking), `Info.plist`
  usage descriptions (name ubiqX and describe activity classification).
- Strings in the frontend: `index.html` title and favicon, the wordmark in `Sidebar.tsx:78-80`
  and `Onboarding.tsx:219-222`, 46 message lines containing "ubiqX" (settings 22, onboarding 10,
  common 4, updates 4, nav 2, timeline 2, ui 2), `Settings.tsx:1095,1323` ("ubiqX {version}"),
  `REPO_URL`, `INSTALL_NOTES`, `SITE_URL`.

Engine parameters that are constants today (§ 1 item 9): self bundle id / exempt names / self
stems (otherwise the app can act on its own windows and records itself), keyring service,
default feed URL, feed product, HTTP user agents (`"ubiqx/<ver>"` in `ubiqx-ai`,
`"ubiqX/<ver>"` in `update_feed.rs:44`).

Update channel

- Own `latest.json` URL: already overridable at build time through `UBIQX_UPDATE_FEED_URL`
  (`build.rs:30`).
- Own `plugins.updater.endpoints` (and optionally its own minisign key pair; sharing the
  existing pair and secrets works).
- `publish-release.mjs` must stop assuming one product: prefix in `ASSET_KINDS`/`STABLE_NAME`,
  `releaseName`, `product`, the `tauri.conf.json` path in `productVersion`. Publishing under a
  second tag (`--tag <other>`) keeps `latest.json` / `updater.json` names and the existing
  tests intact; publishing into the same release needs different feed file names.
- `app-version.mjs` writes one `TAURI_CONF`; it needs a second path or a parameter.
- The engine does not check `feed.product`, so pointing the second app at the first app's feed
  would offer ubiqX installers as its update. Keep the feeds separate and add the check.

CI

- Exclude the new crate in `ci.yml:52`, `:54`, `:214`; add it to the Windows clippy step.
- Artifact names must not match `ubiqX-*-app` / `ubiqX-*` (`ci.yml:348`, `:372`), or the
  publish job will feed them to the ubiqX release, where `inferAsset` would map e.g. a second
  `.dmg` onto the stable name `ubiqX-macos-<arch>.dmg`.
- Bundle outputs share `target/release/bundle/*`; product-specific globs are required if both
  apps build in one job.
- Codemagic has the same single-product assumptions.

Frontend

- A product-specific route table, nav, onboarding step list and Settings section list (the
  standalone app has no Timeline, Review, Reports, Categories, Insights).
- The mock, the tests and the i18n namespaces come along for free if the code is shared.

Docs and site: download names, install commands, the pricing section.

### 13.2 Three ways to lay it out

**A. Second shell crate + shared frontend built twice** (`apps/nofap/src-tauri` beside the
existing one; one Vite project with a build-time flavour such as `VITE_PRODUCT=nofap` choosing
the entry/route table/brand; the second `tauri.conf.json` points `frontendDist` and
`beforeBuildCommand` at that build).

- New: a crate with its own `Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/`,
  `icons/`, `Info.plist`, `lib.rs`, `commands.rs`. To avoid forking 1 700 lines of shell,
  the common parts (IpcError, `blocking`, the settings/licence/update/focus commands, window
  helpers, sink/notifier/presenter) should first move into a shared `rlib` crate; Tauri commands
  can live in a library, but each app still writes its own `generate_handler!` list.
- Keeps a single `node_modules` and lockfile; `pnpm tauri build` for the second app needs the
  Tauri CLI to find its `src-tauri` (run with the CLI from `apps/desktop` and a working
  directory/`--config` arrangement, or give `apps/nofap` a minimal `package.json`).
- Clean separation of identity, capabilities and plist. Highest up-front cost.

**B. One shell crate, two configurations** (`tauri build --config
src-tauri/tauri.nofap.conf.json --features nofap`): the override file changes `productName`,
`identifier`, `mainBinaryName`, window title, icons, updater endpoints and the frontend build
command; a Cargo feature (or a build-time env stamped like `UBIQX_UPDATE_FEED_URL`) selects the
tray, the command list and the product identity.

- Smallest diff and no duplicated shell; both flavours are compiled from the same sources, so
  they cannot drift.
- Costs: `cfg(feature)` branches in `lib.rs`; the single `Info.plist` and `capabilities/`
  directory are shared, so product-specific usage strings need a config override or a second
  plist selected at build time (check what the locked `tauri-build 2.6.3` schema allows before
  relying on it); clippy/test must run for both feature sets.

**C. Separate frontend package importing from `../desktop/src`.** Two `node_modules` trees
resolve React twice unless deduped, Tailwind 4 must be told to scan the other tree, `tsc`'s
`include` and Vitest's `include` need widening. More moving parts than A for no extra
isolation; listed for completeness.

### 13.3 Cost

Engineering (one-off):

| Item | A | B |
|---|---|---|
| Make app identity a parameter in core/platform/engine/app (self id, exempt names, stems, keyring service, feed URL/product, user agent) | needed | needed |
| Shell: tray/menus/commands per product | new crate + extraction of a shared shell lib | feature-gated branches |
| Frontend flavour: entry, routes, nav, onboarding steps, settings sections, brand strings | needed | needed |
| Icons, tray glyph, plist strings, capability files | new set | new set + build-time selection |
| `publish-release.mjs` + tests, `app-version.mjs`, `ci-app-version.sh` parameterised | needed | needed |
| `ci.yml` (three app jobs + publish) and `codemagic.yaml` | needed | needed |
| Site: second download block, pricing; docs | needed | needed |
| Licence: per § 12.2 | independent of layout | independent of layout |

Recurring:

- CI time. The crates compile once per job (shared target dir), but each app has its own final
  link with `lto = true, codegen-units = 1` and its own bundling/notarisation-free packaging;
  expect each of the three app jobs to grow by one LTO link + one bundle + one `vite build`
  if both apps build in the same job, or to double (plus cache size) if they get separate jobs.
  macOS minutes are the expensive ones.
- Release surface doubles: 9 more assets (plus 3 signatures) and 2 more feed files per push, under a second tag or
  with prefixed names.
- Every ubiqX regression risk from shared code now applies to two auto-updating products,
  with the branch-publishing behaviour of § 14.1 multiplying it.
- Bundle size: the 4.5 MB `Ubi.glb` and the three.js chunk ship with any flavour that shows the
  3D mascot.

Side-by-side installation: different identifiers give separate data dirs, databases, settings
and onboarding; shared keyring entries unless the service name is split; two samplers and (if
both enable it) two enforcers of the same system-wide DNS state, which the nofap layer must
tolerate (idempotent apply, ownership marker).

---

## 14. Surprising, fragile or missing

1. **Feature branches ship to users.** `on.push.branches: ["**"]` + `publish-continuous` on any
   push + Codemagic on `claude/*` + no branch/product filter in `is_newer`
   (`ubiqx-core/src/update.rs:304-310`) + version = commit count: a push of this branch becomes
   the `continuous` release and every installed app is offered it (and installs it with one
   click). `docs/WINDOWS-LINUX.md` § 1 says "cada push na `main`"; the workflow says otherwise.
   For a module that changes system DNS this needs a decision before the first push (gate
   publishing on `refs/heads/main`, or ship the module disabled by default).
2. **No single-instance guard.** Nothing prevents a second process (no
   `tauri-plugin-single-instance`). Two engines would share one SQLite file and, with nofap,
   race on the same hosts/DNS state.
3. **The login item is registered before consent.** `Settings::default().launch_at_login` is
   `true` (`model.rs:1013`) and the shell calls `autostart.enable()` on the first launch, before
   onboarding (`lib.rs:598-605`). A standalone blocker probably wants this, but it should be a
   stated choice.
4. **Intervention page falls back to a fake "YouTube" sample** when the id is unknown or not in
   the newest 50 (`Intervention.tsx:66-67`). A nofap event that is deliberately *not* persisted
   (for privacy) would render as "YouTube / Não! Foque na sua produtividade."
5. **`emit("engine", …)` and `emit("intervention", …)` go to every window**, and every local
   window may invoke every command (no `AppManifest` in `build.rs`). Fine for one trusted
   bundle; relevant if a nofap window ever shows remote content (it must not).
6. **`Info.plist` uses `NSSystemAdministrationUsageDescription`** for the Screen Recording
   explanation (`:14-15`). That key is the System Administration (admin Apple events) usage
   string, not a screen-capture one; it is harmless today and misleading the day the app really
   asks for administrator rights.
7. **Private mode and pause are tray-level off switches** (§ 5), and Settings › Privacidade
   uses "blocked" for "never recorded". Both collide with nofap semantics.
8. **`docs/ARCHITECTURE.md` is behind the code**: its `EngineEvent` list stops at
   `ScreenshotTaken`, its `Nudge` kinds omit `focus_prompt`, § 6 lists no Foco page or
   intervention window, § 2 says logs go to `~/Library/Logs/ubiqX` (the log plugin is
   initialised with defaults, so the directory follows the identifier `ai.ubiqx.app`), and its
   header still presents a macOS product. README and `WINDOWS-LINUX.md` are current.
9. **Two different default data dirs.** The shell uses Tauri's `app_data_dir()`
   (identifier-based); `ubiqx_app::default_data_dir()` uses `ProjectDirs::from("ai", "ubiqx",
   "ubiqX")` (`ubiqx-app/src/lib.rs:142-146`) for the CLI, and `secrets_file::default_dir()` has
   a third set of fallbacks (`secrets_file.rs:26-37`). The CLI and the desktop app do not see
   the same database by default.
10. **Mirrors kept by hand**: `types.ts` ↔ Rust models; `mock.ts` ↔ engine behaviour;
    `focus.message.N` ↔ `MESSAGES_PT/EN`; `PRAISE_MIN_MINUTES` (`Focus.tsx:22`);
    `UBI_MODELS` (`providers.ts:19`); `DEFAULT_FEED_URL` in `build.rs` and `update.rs`;
    comments in `ipc.ts`/`types.ts` still say "this Mac" / "DMG". No test compares the mirrors.
11. **The shell has no tests** (§ 10.2); `commands.rs` is checked only by compilation, and only
    on macOS (tests), Windows (clippy) and Linux (bundle build) runners.
12. **Unused JS dependencies**: `@tauri-apps/plugin-shell` (no Rust counterpart),
    `plugin-autostart`, `plugin-notification`, `plugin-opener`. `Focus.test.tsx` and
    `Intervention.test.tsx` mock `@react-three/drei`, which is not a dependency.
13. **`scripts/screenshots.mjs` does not capture the Foco page**, although the desktop README
    says it captures "as oito páginas"; Foco is the ninth route.
14. **Linux tray dependency**: without AppIndicator support there is no tray, hence no quit,
    no pause, no reopen-after-close other than relaunching (which, per item 2, starts a second
    instance).
15. **Licence docs promise nothing about the focus guard**, and the code gates nothing there;
    with `Soft` enforcement the whole app is free. Any nofap pricing rests on changing that or
    on gating only AI.
16. **macOS updates reset permissions** (ad-hoc signing, § 11.4) — a recurring support cost
    that grows with every permission the nofap layer adds.

---

## 15. Checklists

### 15.1 nofap inside ubiqX (shell + frontend only)

- [ ] `Settings.nofap: NofapSettings` with `#[serde(default)]`; mirror in `types.ts`
      (`Settings`), `mock.ts` `defaultSettings()`.
- [ ] Commands in `commands.rs` + `generate_handler!`; wrappers in `ipc.ts`; types in
      `types.ts`; handlers, state and `__mock` hook in `mock.ts`.
- [ ] New `EngineEvent` variants → Rust enum, `types.ts` union, `store.applyEvent`, toasts in
      `engine.ts` if wanted.
- [ ] New `InterventionAction` / `NudgeKind` values → `types.ts`, `focus.action.<id>`,
      `NUDGE_ICON`, `settings.ubi.kind.<kind>.*` if user-switchable.
- [ ] Page `pages/Nofap.tsx` + route + nav item + `nav.nofap` + `i18n/messages/nofap.ts`
      registered in `NAMESPACES`; parity test green.
- [ ] Settings: decide between a section in `SECTIONS` and options on the page (Focus keeps
      its options on the page).
- [ ] Onboarding: consent step in `ALL_STEPS` + `onboarding.step.<id>` / `onboarding.rail.<id>`;
      update the "Passo 1 de 7" expectation.
- [ ] Counselling window: label, builder, route outside `Gate`, capability file, presenter
      path; no sample fallback; decide focus and auto-close.
- [ ] Tray: decide whether pause/private mode affect nofap; add items if any (seven edits).
- [ ] Rust-side copy in both languages (`MenuText`, `Text`, engine messages) + mock mirror.
- [ ] Tests: page test, component tests, `mock.test.ts` cases; `pnpm typecheck && pnpm test --
      --run && pnpm build`.
- [ ] Elevation path for the DNS layer on each OS, isolated behind a port in `ubiqx-platform`,
      with the shell only relaying a "needs administrator approval" state to the UI.
- [ ] Before pushing: § 14.1.

### 15.2 Standalone product

- [ ] Identity parameters out of the crates (self id, exempt names, stems, keyring service,
      feed URL + product check, user agent).
- [ ] Shell flavour (layout A or B), icons, tray glyph, plist strings, capabilities.
- [ ] Frontend flavour: routes, nav, onboarding steps, settings sections, brand strings,
      `index.html`.
- [ ] Release tooling: `publish-release.mjs` (+ tests), `app-version.mjs`,
      `ci-app-version.sh`, `ci.yml`, `codemagic.yaml`; separate tag or prefixed feeds;
      artifact names that do not match `ubiqX-*`.
- [ ] Licence decision per § 12.2 and the matching changes in core, proxy, panel, UI, site,
      docs.
- [ ] Site and docs: downloads, install notes, pricing.
