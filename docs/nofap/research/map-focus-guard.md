# Code map: the Focus guard (what the `nofap` module will most resemble)

Read-only mapping of the existing focus guard, written for the engineer who will implement an
always-on "category block" (nofap) without re-reading the code. Everything below was read in
the source on branch `claude/sharp-bardeen-sk27ae`; nothing was compiled or run for this report
(items marked *by reading* are conclusions from the code path, not from an executed test).
Paths are relative to the repository root. Line numbers are those of this branch.

Contents

1. The guard in one page
2. Data model (types, tables, settings, events)
3. Detection: the guard pass, the tick, what is known per OS and browser
4. Enforcement per OS ("close the tab / close the app / only warn") and its limits
5. The UBI intervention window: trigger chain, contents, limits
6. Settings storage, cooldowns, caps, friction
7. The nudge channel (UBI's lighter voice)
8. Shell, IPC and front-end plumbing (files a new feature touches)
9. Tests and test doubles
10. Extension points for an always-on category block
11. Surprising, fragile or missing (numbered findings)
12. Docs versus code
13. What this means for the standalone product

---

## 1. The guard in one page

- Pure rules: `crates/ubiqx-core/src/focus.rs` (no I/O). Engine wiring:
  `crates/ubiqx-engine/src/focus.rs`. Ports: `crates/ubiqx-core/src/ports.rs`
  (`ActivitySource`, `BrowserUrlResolver`, `Enforcer`, `InterventionPresenter`, `Notifier`,
  `FocusRepo`). OS adapters: `crates/ubiqx-platform/src/{macos,windows,linux}/enforce.rs`,
  `browser.rs` (macOS AppleScript), `windows/browser.rs` (UI Automation),
  `linux/browser.rs` + `browser_url.rs` (window title).
- A dedicated tokio loop, `focus_loop` (`crates/ubiqx-engine/src/engine.rs:218-234`), calls
  `focus::guard_once(&state)` on the blocking pool every `LoopConfig::focus_every` =
  **2 s** (`engine.rs:48`). The first tick is skipped; missed ticks are skipped
  (`MissedTickBehavior::Skip`, `engine.rs:136-140`); passes never overlap (each is awaited).
- A pass looks only at **the foreground window** (plus, for a supported browser, the URL of
  its **active tab**). If that hits an enabled `FocusTarget` (or, during a focus session, a
  rule mapping to the built-in Distraction category), and the per-key cooldown has elapsed,
  it enforces (quit app, or close tab, falling back to blank tab, falling back to nothing),
  records an `Intervention`, emits events and asks the shell to show UBI's window; if the
  window cannot be shown it sends an OS notification.
- The guard is **independent of the tracker**: `guard_once` never checks
  `tracking_enabled`, `onboarding_done`, the pause flag, private mode or quiet hours. Its
  only switch is `settings.focus.guard_enabled` (`engine/src/focus.rs:173`).
- It is a **speed bump, not a block**: nothing prevents the request, the page is closed after
  it was seen (latency 2 s tick + URL cache), a closed tab can be reopened, a blanked tab has
  the page one "Back" away, and within the cooldown (20 s default) nothing is enforced at
  all. This is the gap DNS-level blocking is meant to close.

---

## 2. Data model

### 2.1 Core types (`crates/ubiqx-core/src/model.rs`)

| Type | Lines | Fields / variants |
|---|---|---|
| `ForegroundWindow` | 20-33 | `app_name`, `app_id` (macOS bundle id; exe stem on Windows/Linux), `window_title` (may be empty), `pid: Option<u32>`, `window_id: Option<u32>`, `bounds: Option<(i32,i32,u32,u32)>` |
| `FocusTargetKind` | 1134-1154 | `App`, `Site`; stored as `"app"` / `"site"` (`as_str`/`parse`) |
| `FocusTarget` | 1160-1169 | `id`, `kind`, `name`, `key` (bundle id / exe stem for apps; lower-case domain for sites, covering subdomains), `enabled`, `created_at`, `last_blocked_at: Option<DateTime<Utc>>`, `blocked_count: u32` |
| `InstalledApp` | 1173-1177 | `name`, `bundle_id`, `path` |
| `InterventionAction` | 1182-1212 | `AppQuit` `"app_quit"`, `TabClosed` `"tab_closed"`, `TabBlanked` `"tab_blanked"`, `Notified` `"notified"` |
| `Intervention` | 1217-1228 | `id`, `at`, `target_id: Option<Id>` (`None` = hit came from a Distraction rule), `kind: FocusTargetKind`, `name`, `key`, `action`, `session_id: Option<Id>`, `message` (what UBI said, already localised) |
| `FocusSession` | 1232-1260 | `id`, `task`, `started_at`, `ends_at`, `ended_at: Option<_>`, `interventions: u32`, `hid_windows`, `ran_shortcut`; helpers `is_active`, `remaining_secs`, `planned_minutes` |
| `FocusStatus` | 1264-1270 | `session`, `remaining_secs`, `targets_enabled`, `interventions_today`, `guard_enabled` |
| `KnownDomain` | 1274-1277 | `domain`, `seconds` |
| `FocusSettings` | 873-903 | see 6.1 |
| `EngineEvent::Intervention { intervention }`, `EngineEvent::FocusSession { session: Option<FocusSession> }` | 1089-1096 | serde `tag = "type"`, snake_case: `"intervention"`, `"focus_session"` |
| `NudgeKind` | 478-525 | `Unproductive, Distracted, BreakSuggested, Praise, Idle, ReportReady, Attention, FocusPrompt` |

There is **no field that says where an intervention came from** other than
`target_id.is_none()` (rule) and `session_id`. A third origin (built-in category list, vision)
is not representable today.

### 2.2 In-memory state

`FocusRuntime` (`crates/ubiqx-engine/src/focus.rs:108-140`), held as
`EngineState.focus: Mutex<FocusRuntime>` (`crates/ubiqx-engine/src/state.rs:42`):

- `session: Option<FocusSession>` (mirrors the store; restored by
  `FocusRuntime::restore(repo)` at start, `state.rs:61`),
- `cooldowns: HashMap<String, DateTime<Utc>>` keyed by the hit's `key`,
- `message_index: usize` (rotation over 5 messages).

Cooldowns and the rotation are **not persisted**: a restart resets both.

Private to the module: `struct Hit { target_id, kind, name, key, app_id }`
(`focus.rs:143-151`), `fn intervene` (287), `fn distraction_hit` (241), `fn attempt` (380),
`FocusRuntime::next_message` (135).

### 2.3 SQLite (`crates/ubiqx-storage/src/migrations.rs`, V2 at 191-232)

```
focus_targets(id PK, kind, name, key, enabled DEFAULT 1, created_at, last_blocked_at,
              blocked_count DEFAULT 0, UNIQUE(kind, key))
interventions(id PK, at, target_id, kind, name, key, action, session_id, message DEFAULT '')
              + idx_interventions_at            -- no FK to targets on purpose
focus_sessions(id PK, task, started_at, ends_at, ended_at, interventions, hid_windows,
               ran_shortcut) + two indexes
```

- Timestamps are unix milliseconds. Schema version is `PRAGMA user_version`; evolve by
  appending `M::up(V3)` to `migrations()` (line 236). The test at 268-274 asserts the version
  is `2` and must be bumped.
- Repository: `impl FocusRepo for SqliteStore` in `crates/ubiqx-storage/src/repos/focus.rs:169`.
  `upsert_target` is `ON CONFLICT(kind, key) DO UPDATE SET enabled = 1` (line 44-49): re-adding
  re-enables and keeps id, name and counters. `list_targets` orders
  `enabled DESC, lower(name), id`.
- Enum columns are decoded strictly with `parse_enum` (`convert.rs:57`) and `query_list`
  collects `Result`s (`store.rs:40-49`): **one row with an unknown `kind` or `action` makes
  the whole `list_interventions` call fail**. New variants are fine going forward, but a
  downgrade to an older build breaks the Focus page and the intervention window.
- "Apagar todos os dados" (`repos/maintenance.rs:11-21`) deletes `interventions` and
  `focus_sessions`; `focus_targets` are kept ("configuration, not activity").
- **No retention for `interventions`**: there is no `delete_before` for them
  (`screenshots::cleanup`, `engine/src/screenshots.rs:141-158`, only purges screenshots). Rows
  accumulate until the user wipes everything.
- `export_json` (`engine/src/handle.rs:331-360`) exports blocks, categories, rules, reports;
  interventions and targets are **not** exported.

### 2.4 `FocusRepo` port (`crates/ubiqx-core/src/ports.rs:284-310`)

`list_targets`, `get_target`, `upsert_target`, `set_target_enabled`, `remove_target`,
`touch_blocked(id, at)`, `insert_intervention`, `get_intervention`, `list_interventions(limit)`,
`count_interventions_since(since)`, `insert_session`, `update_session` (full row,
`execute_expecting_row`: errors with `NotFound` when the row is gone), `active_session`,
`list_sessions(limit)`.

---

## 3. Detection

### 3.1 `guard_once`, step by step (`crates/ubiqx-engine/src/focus.rs:160-237`)

```rust
pub fn guard_once(state: &Arc<EngineState>) -> CoreResult<Option<Intervention>>
```

1. `now`, `settings` snapshot (161-162).
2. **Session timer first** (165-172): a session with `now >= ends_at` is ended
   (`end_session(state, s, now, true)?`). This runs even when the guard is disabled.
3. `if !settings.focus.guard_enabled { return Ok(None) }` (173).
4. `list_targets()` from SQLite **on every pass**, filtered to enabled (176-183).
5. `rules_apply = session.is_some() && settings.focus.block_distraction_in_session` (184).
6. **Early exit** `if targets.is_empty() && !rules_apply { return Ok(None) }` (185-188),
   commented "Nothing to enforce: no need to touch the window server".
7. `platform.activity.foreground()?`; `None` (nothing in front) ends the pass (191-193).
8. Exemptions: `is_exempt_window(&win.app_id, &win.app_name, session_age)` (194-197).
9. URL: only when `platform.urls.supports(&win.app_id)`; `resolve(&win)` errors are logged at
   `debug` and treated as "no URL" (198-208). `domain = url.and_then(domain_of)` (209).
10. Hit: `rules_focus::find_hit(&targets, &win.app_id, domain)`; else, when `rules_apply`,
    `distraction_hit(...)`; else none (211-224).
11. **Per-key cooldown** (226-235): `if now - last < Duration::seconds(intervention_cooldown_secs)`
    then `return Ok(None)`. The cooldown gates **enforcement too**, not only the message.
12. `intervene(state, hit, session, now, settings.ui_language())` (236).

Matching rules (`crates/ubiqx-core/src/focus.rs`):

- `find_hit(targets, app_id, page_domain) -> Option<&FocusTarget>` (176-191): enabled only;
  **apps before sites**; linear scan.
- `app_hits(key, app_id)` (170): trimmed, ASCII case-insensitive equality.
- `domain_hits(key, page_domain)` (165) = `normalize::domain_matches(page_domain, key)`
  (`crates/ubiqx-core/src/normalize.rs:113-120`): lower-case, pattern may start with `*.`,
  match is `d == p || d.ends_with(".{p}")`. `youtube.com` covers `m.youtube.com`, not
  `notyoutube.com`.
- `domain_of(url)` (`normalize.rs:106-110`): `url::Url::parse`, lower-case host, leading
  `www.` stripped. `about:blank` and other host-less URLs give `None`.
- `normalize_key(kind, raw)` (124-148) and `looks_like_domain(text)` (152-161): strip scheme,
  path, query, fragment, credentials, port, trailing dots, leading `www.`; a domain needs a
  dot and only `[A-Za-z0-9.-]`. The UI has its own copies (`looksLikeDomain`,
  `normalizeDomain` in `apps/desktop/src/lib/format.ts`).
- `distraction_hit` (`engine/src/focus.rs:241-284`): builds a synthetic `ActivityBlock` from
  the window and URL, runs `ubiqx_core::rules::best_rule(&rules, &block)`, and hits only when
  `rule.category_id == system_categories::DISTRACTION` (`"sys-distraction"`). The resulting
  `Hit` has `target_id: None`; with a domain it is `kind: Site, name = key = domain`,
  otherwise `kind: App, name = app_name, key = app_id`. **This is the existing template for a
  second, non-list source of hits.**

Exemptions:

- Core (`core/src/focus.rs:32-56`, `is_exempt_app` 196-210): `EXEMPT_APP_IDS` =
  `ai.ubiqx.app`, `com.apple.finder`, `com.apple.systempreferences`,
  `com.apple.SystemPreferences`, `com.apple.loginwindow`; `EXEMPT_APP_NAMES` = `ubiqX`,
  `Finder`, `System Settings`, `System Preferences`, `Ajustes do Sistema`,
  `Preferências do Sistema`, `loginwindow`; `com.apple.Terminal` / name `Terminal` only for
  the first `TERMINAL_GRACE_SECS = 60` of a session.
- Engine (`engine/src/focus.rs:26-102`): `EXEMPT_APP_IDS_WINDOWS` (`explorer`,
  `systemsettings`, `taskmgr`, `applicationframehost`, `lockapp`, `searchhost`,
  `startmenuexperiencehost`, `shellexperiencehost`, `logonui`), `EXEMPT_APP_IDS_LINUX`
  (`gnome-shell`, `plasmashell`, `nautilus`, `dolphin`, `gnome-control-center`,
  `systemsettings`, `systemsettings5`, `xfce4-panel`, `thunar`), `TERMINAL_APP_IDS` (same 60 s
  grace). Both lists are applied on every OS.
- Self-protection in the adapters: `SELF_BUNDLE_ID = "ai.ubiqx.app"`
  (`core/src/focus.rs:32`), `SELF_STEMS = ["ubiqx", "ubiqx-desktop", "ubiqx_desktop"]`
  (`crates/ubiqx-platform/src/app_id.rs:9`), plus the literal name `"ubiqX"` inside
  `hide_others_script` (`macos/enforce.rs:77`).

### 3.2 Which sampler, which tick

There are **two independent observers** of the foreground:

| | Tracker sampler | Focus guard |
|---|---|---|
| Where | `crates/ubiqx-engine/src/sampler.rs` (own OS thread `ubiqx-sampler`) | `focus_loop` + `guard_once` (tokio blocking pool) |
| Period | `settings.sample_interval_secs` (default 5, clamped 1..=60) | `LoopConfig::focus_every` (2 s, not a setting) |
| Runs when | `tracking_enabled && onboarding_done && !paused` (`sampler.rs:29-32`) | always, subject to `focus.guard_enabled` and "something to enforce" |
| URL lookup | only when `(app_id, title)` changed since the last sample (`sampler.rs:68-88`) | every pass (the macOS and Windows resolvers cache 20 s by `(app_id, title)`; Linux parses the title each time) |
| On Automation denial | emits `EngineEvent::PermissionRequired { permission: "automation" }` (`sampler.rs:77-82`) | logs at `debug`, nothing surfaced (`focus.rs:201-204`) |
| Output | `ActivitySample` → segmenter → blocks → screenshots → AI | interventions |

The guard does not consume samples or blocks; it calls `platform.activity.foreground()` and
`platform.urls.resolve()` itself. Both observers share the same resolver object (and its cache).

Privacy behaviour of the tracker that matters for nofap (it is *not* the guard's):
`Segmenter::is_blocked` (`crates/ubiqx-core/src/segmenter.rs:100-115`) redacts a sample when the
app is in `settings.blocked_apps`, when the **title carries a private-browsing marker**
(`normalize.rs:79-91`: `(incognito)`, `(anônima)`, `navegação privada`, `private browsing`,
`inprivate`, `navegação anônima`, ...) or when the domain matches `settings.blocked_domains`.
The redacted sample has title `"[privado]"`, `url: None`, `window_id: None`
(`segmenter.rs:130-153`), lands in category `sys-private`, and
`screenshots::should_capture` refuses that category (`engine/src/screenshots.rs:25-27`).
Consequence: **the existing screenshot/vision pipeline never looks at private-browsing
windows**, and it only captures a block that is at least 20 s old
(`FIRST_CAPTURE_AFTER_SECS`, `screenshots.rs:13`) at most every
`screenshot_interval_secs` (120 s) under `vision_policy`. The guard, by contrast, reads the
URL of private windows like any other.

### 3.3 What is known about the foreground, per OS

| | macOS | Windows | Linux |
|---|---|---|---|
| Adapter | `macos/frontmost.rs` `AppKitActivitySource` | `windows/frontmost.rs` `WindowsActivitySource` | `linux/frontmost.rs` `X11ActivitySource` |
| App identity | bundle id (`com.google.Chrome`) | exe stem, lower-case (`chrome`, `msedge`) (`app_id.rs`) | exe stem from `/proc/<pid>/exe`, else `WM_CLASS` instance |
| App + pid | no permission (`NSWorkspace`) | no permission | X11 only |
| Window title, window id | need **Screen Recording** (`macos/mod.rs:3-13`); empty title otherwise | always | X11 `_NET_WM_NAME` |
| Wayland-native windows | n/a | n/a | **invisible**: `foreground()` returns `None`, the guard does nothing (`linux/mod.rs:10-16`) |

### 3.4 What is known about the page, per OS and browser

`BrowserUrlResolver { fn supports(&self, app_id) -> bool; fn resolve(&self, &ForegroundWindow) -> CoreResult<Option<String>> }`
(`ports.rs:26-29`). Without `supports`, the guard has **no domain** and only app targets can hit.

**macOS: `OsascriptUrlResolver`** (`crates/ubiqx-platform/src/browser.rs`)

- Supported bundle ids (`script_for`, 17-46): Safari `com.apple.Safari`,
  `com.apple.SafariTechnologyPreview`; Chromium family `com.google.Chrome`, `.canary`,
  `.beta`, `org.chromium.Chromium`, `com.brave.Browser`, `.beta`, `com.microsoft.edgemac`,
  `.Beta`, `company.thebrowser.Browser` (Arc), `com.vivaldi.Vivaldi`,
  `com.operasoftware.Opera`, `com.operasoftware.OperaGX`.
- **Firefox and every Gecko fork: unsupported** (no AppleScript tab model); so are Orion,
  DuckDuckGo, Zen, Chrome "app mode" windows (`com.google.Chrome.app.*`), Electron apps and
  in-app webviews.
- Returns the **full URL**, only `http://`, `https://`, `file://` (`parse_url_output`, 63-70).
- Needs the **Automation** grant per browser. First call per browser may block up to
  `FIRST_CONTACT_TIMEOUT = 120 s` (122) while the consent alert is up; afterwards 1.5 s
  (`Default`, 143). A denial (`-1743` / "not authorized") returns `Err(CoreError::Permission)`
  and the browser is left alone for `DENIED_BACKOFF = 30 min` (115), during which `resolve`
  answers `Ok(None)`.
- Cache: `CACHE_TTL = 20 s` per `(app_id, window_title)` (117, 203-208), cleared above 256
  entries.

**Windows: `UiaUrlResolver`** (`crates/ubiqx-platform/src/windows/browser.rs`)

- Supported stems `WINDOWS_BROWSERS = ["chrome","msedge","brave","opera","vivaldi","firefox"]`
  (`browser_url.rs:8`).
- Reads the **text of the address bar**: Chromium = first `Edit` control with a `ValuePattern`
  within `TOOLBAR_HEIGHT_PX = 120` of the window top; Firefox = automation id `urlbar-input`.
  The text goes through `normalize_typed_url` (`browser_url.rs:54-93`): scheme-less
  `host/path` gets `https://` prepended; `chrome://`, `about:`, `edge://` etc. are passed
  through; anything with whitespace or without a dotted host is `None`.
- No permission. Any UIA failure is `Ok(None)`. Cache 20 s per `(app_id lower, title)`.

**Linux: `TitleUrlResolver`** (`crates/ubiqx-platform/src/linux/browser.rs`)

- Supported stems `LINUX_BROWSERS` (`browser_url.rs:12-25`): `chrome`, `google-chrome`,
  `chromium`, `chromium-browser`, `brave`, `brave-browser`, `microsoft-edge`, `msedge`,
  `opera`, `vivaldi`, `firefox`, `firefox-esr`.
- **Title only**: `url_from_title` (98-126) needs the title to end with a known suffix
  (`TITLE_SUFFIXES`, 37-48) **and** to contain a bare-domain word. Pages that do not print
  their domain in the title (most do not) have no URL, so site targets do not fire.

Summary of coverage for "is this page on a blocked domain?":

| Browser | macOS | Windows | Linux (X11) |
|---|---|---|---|
| Safari | yes (Automation) | n/a | n/a |
| Chrome, Edge, Brave, Vivaldi, Opera | yes (Automation) | yes (UIA) | only if the title shows the domain |
| Arc | yes | no | n/a |
| Firefox | **no** | yes (UIA) | only if the title shows the domain |
| Anything else / PWAs / webviews | no | no | no |
| Background tabs, non-foreground windows, other monitors | never | never | never |

---

## 4. Enforcement

### 4.1 The decision (`intervene`, `crates/ubiqx-engine/src/focus.rs:287-377`)

```rust
let action = match hit.kind {
    FocusTargetKind::App  => if attempt("quit app", enforcer.quit_app(&hit.app_id)) { AppQuit } else { Notified },
    FocusTargetKind::Site => if attempt("close tab", enforcer.close_active_tab(&hit.app_id)) { TabClosed }
                             else if attempt("blank tab", enforcer.blank_active_tab(&hit.app_id)) { TabBlanked }
                             else { Notified },
};
```

`attempt` turns `Err` into `false` (logged at `debug`). There is **no user-selectable policy**
("close tab" vs "close app" vs "only warn"): the kind of the hit decides, and "only warn"
(`Notified`) is purely the fallback when the platform refused. Then, in order: next rotated
message; `insert_intervention`; `touch_blocked` (when `target_id` is set); cooldown stamp
(map pruned to entries younger than 1 h once it exceeds 512); session counter
`update_session(&s)?` + `EngineEvent::FocusSession`; `EngineEvent::Intervention`;
`presenter.show(&intervention)`; on `false`/`Err`,
`notifier.notify(&intervention.name, &intervention.message)`.

### 4.2 The port (`crates/ubiqx-core/src/ports.rs:118-130`)

```rust
pub trait Enforcer: Send + Sync {
    fn quit_app(&self, bundle_id: &str) -> CoreResult<bool>;
    fn close_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool>;
    fn blank_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool>;
    fn hide_others(&self, keep_bundle_id: &str) -> CoreResult<bool>;
    fn run_shortcut(&self, name: &str) -> CoreResult<bool>;
}
```

`Ok(true)` = took effect, `Ok(false)` = platform could not, `Err` = unexpected.
Implementations: `MacEnforcer`, `WindowsEnforcer`, `LinuxEnforcer`, `MockEnforcer`
(`crates/ubiqx-platform/src/mock.rs:343-415`, records `EnforcerCall`s; flags `refuse_quit`,
`refuse_close_tab`, `refuse_blank_tab`).

### 4.3 Per OS

| Action | macOS (`macos/enforce.rs`) | Windows (`windows/enforce.rs`) | Linux (`linux/enforce.rs`) |
|---|---|---|---|
| `quit_app` | `NSRunningApplication::terminate()` on every instance of the bundle id (119-135). Graceful; the app may refuse (unsaved changes). No permission. Returns `true` when any `terminate()` returned true, i.e. the request was sent. | `WM_CLOSE` to every top-level window of the stem, **sleep `CLOSE_GRACE` = 1500 ms**, then `TerminateProcess` on every pid still alive (145-189). Returns `Ok(true)` unconditionally once a window was found. Unsaved work is lost. | `SIGTERM` to every process of this user with that exe stem (41-58). No `SIGKILL`. Works without a display. |
| `close_active_tab` | AppleScript `tell application id "<id>" to if (count of windows) > 0 then close current tab|active tab of front window` (47-57), `TAB_TIMEOUT` = 5 s. Only browsers known to `script_for`; **Firefox → `Ok(false)`**. Needs Automation. | Requires the app to own the foreground (`browser_in_front`), sends `Ctrl+W` with `SendInput`, sleeps `KEY_SETTLE` = 150 ms, answers true if the app is still in front or has no windows left (191-201). | Same through XTest: `Ctrl+W`, 150 ms, true if still in front or no processes left (60-73). X11 only. |
| `blank_active_tab` | AppleScript `set URL of current tab|active tab of front window to "about:blank"` (60-70). | `Ctrl+L`, type `about:blank` as Unicode events, `Enter` (203-216). | `Ctrl+L`, `type_text("about:blank")`, `Return` (75-90). |
| `hide_others` | System Events script; its own Automation consent; `HIDE_TIMEOUT` = 60 s | `ShowWindow(SW_MINIMIZE)` on other apps' titled windows | `iconify` other viewable windows |
| `run_shortcut` | `/usr/bin/shortcuts run <name>` (spawned, not awaited) | `cmd /C <command>` with `CREATE_NO_WINDOW` | `sh -c <command>` |
| Permissions | Automation per browser for tab actions | none | none (X11) |

Limits that apply everywhere:

- Only the **active tab of the foreground window** is ever acted on.
- A **closed tab** comes back with "reopen closed tab"; a **blanked tab** keeps its history, so
  Back returns to the page. Neither removes access.
- On Windows and Linux the tab actions are **blind keystrokes** into whatever has the keyboard
  focus. `browser_in_front` only checks that *that app id* is in front, not that it is a
  browser and not that the same tab is still active (so it also "works" for any app where
  `Ctrl+W` closes something, and it can close the wrong tab; see 11.6, 11.7).
- On macOS with **Firefox** there is neither a URL nor a tab action: the only thing the
  current `Enforcer` can do to it is `quit_app` (the whole browser) or nothing.
- `AppQuit`/`TabClosed` mean "the request was accepted", never "verified gone".

---

## 5. The UBI intervention window

### 5.1 Trigger chain

```
focus_loop (2 s) → guard_once → intervene
  → state.deps.sink.emit(EngineEvent::Intervention { .. })      // "engine" event, all windows
  → state.deps.platform.presenter.show(&intervention)           // InterventionPresenter port
      desktop: TauriPresenter::show  (apps/desktop/src-tauri/src/lib.rs:331-343)
        → show_intervention_window(&app, &intervention.id)       (lib.rs:280-326)
        → app.emit("intervention", intervention)                 // payload for the open page
      CLI/tests: LogInterventionPresenter → Ok(false)            (engine/src/deps.rs:11-27)
  → if !shown: notifier.notify(&intervention.name, &intervention.message)
```

`InterventionPresenter` (`ports.rs:134-136`): `fn show(&self, intervention: &Intervention) -> CoreResult<bool>`.
It is injected through `AppConfig.presenter: Option<Arc<dyn InterventionPresenter>>`
(`crates/ubiqx-app/src/lib.rs:87`, default `LogInterventionPresenter` at 346-348) and wired in
the Tauri `setup` (`apps/desktop/src-tauri/src/lib.rs:541-542`, `577`).

### 5.2 The window (`apps/desktop/src-tauri/src/lib.rs:242-326`)

- Label `INTERVENTION_WINDOW = "intervention"`; size `INTERVENTION_SIZE = (460.0, 188.0)`
  logical; `INTERVENTION_TOP_GAP = 24.0`.
- Route `#/intervention?id=<id>` (`intervention_route`, 255-261; the id is filtered to
  `[A-Za-z0-9_-]`).
- Position: horizontally centred, 24 px below the top of **the monitor under the cursor**
  (primary monitor as fallback) (`intervention_position`, 265-276).
- First use builds it: `.title("ubiqX") .decorations(false) .shadow(true) .resizable(false)
  .always_on_top(true) .visible_on_all_workspaces(true) .skip_taskbar(true) .focused(false)
  .visible(false)`, then `show()`. Later uses re-point it with
  `w.eval("window.location.hash = ...")`, reset size/position/always-on-top and `show()`.
- Capability file `apps/desktop/src-tauri/capabilities/intervention.json`: `core:default`,
  `core:window:allow-close`, `allow-hide`, `allow-set-focus`, `allow-start-dragging`.
- Closing: the page calls `getCurrentWindow().close()`; the app-wide
  `on_window_event` turns every `CloseRequested` into `prevent_close()` + `hide()`
  (`lib.rs:628-634`), so the window is kept and re-shown next time.
- macOS runs as `ActivationPolicy::Accessory` (`lib.rs:532`).

### 5.3 The page (`apps/desktop/src/pages/Intervention.tsx`)

- Route registered outside the `Gate`/`AppShell` (`apps/desktop/src/App.tsx:18-19`): no
  onboarding gate, no chrome.
- Data: for `id=test` a local sample; otherwise `ipc.listInterventions(50)` and
  `find(i => i.id === id)`, **falling back to the sample** (YouTube) when not found or on
  error (58-71). It also listens to the Tauri event `intervention` and replaces the data with
  the payload (74-88).
- Shows: `<Ubi variant="auto" size={120} mood="worried" />`; the message as a speech bubble
  (`role="status"`, `data-testid="intervention-message"`); a rose `Badge` with a kind icon
  (`AppWindow`/`Globe`), the target **name** as text and the **key** as `title`; a primary
  button "Ok, foco!" (`focus.intervention.ok`) with `autoFocus`. Whole panel is a
  `data-tauri-drag-region`.
- `INTERVENTION_AUTO_CLOSE_MS = 7000`: closes itself after 7 s (timer restarts when the data
  changes).
- The message text comes from the engine (`Intervention.message`); the copies under
  `focus.message.N` in `apps/desktop/src/i18n/messages/focus.ts` exist for the mock and the
  test button.
- Engine messages: `MESSAGES_PT`/`MESSAGES_EN` (`core/src/focus.rs:59-73`), five
  productivity lines ("Não! Foque na sua produtividade.", ...), rotated by
  `intervention_message(index, lang)`.
- Test button: `test_intervention` → `focus::test_intervention` (`engine/src/focus.rs:624-635`)
  shows `sample_intervention` (id `"test"`), records nothing.

### 5.4 Limits of the window as a counselling surface

Fixed 460×188, one line of text, one button, 7 s, no input, no links, no second action. It
names the target on an always-on-top panel that is visible on every workspace (and on a shared
screen). The fallback OS notification uses **the target name as its title**
(`focus.rs:367-374`).

---

## 6. Settings, cooldowns, caps, friction

### 6.1 `FocusSettings` (`crates/ubiqx-core/src/model.rs:873-903`, `#[serde(default)]`)

| Field | Default | Meaning |
|---|---|---|
| `guard_enabled` | `true` | master switch of the guard loop |
| `block_distraction_in_session` | `true` | during a session also hold anything the rules map to Distraction |
| `hide_others_on_start` | `true` | hide other windows when a session starts |
| `session_minutes` | `45` | default session length |
| `intervention_cooldown_secs` | `20` | seconds between two interventions on the same key |
| `macos_focus_shortcut_on` / `_off` | `None` | Shortcut name (macOS) or command line (Windows/Linux) |

### 6.2 How settings are stored

- `Settings` is one JSON document in `settings(id = 1, json, updated_at)`
  (`crates/ubiqx-storage/src/repos/settings.rs`); missing fields take serde defaults
  (`#[serde(default)]` on `Settings`, `model.rs:906`, and on `FocusSettings`). **Adding
  `Settings.nofap: NofapSettings` with `#[serde(default)]` needs no migration.**
- Writes go through `EngineState::apply_settings` (`engine/src/state.rs:120-168`):
  `sanitize()` (models, language), save, update segmenter config, swap the in-memory copy.
  There is **no validation of `focus` fields** in the engine (a cooldown of 0 is accepted and
  means "every pass").
- The front end does read-modify-write of the **whole** `Settings`:
  `saveSettings(patch)` = `{ ...current.settings, ...patch }` → `update_settings`
  (`apps/desktop/src/lib/store.ts:146-155`; command at
  `apps/desktop/src-tauri/src/commands.rs:607`). Two consequences: a stale copy in another
  window can overwrite a field changed elsewhere, and any state kept in `Settings` can be
  changed by anything that can call `update_settings`.

### 6.3 Constants (all in `crates/ubiqx-core/src/focus.rs` unless noted)

| Constant | Value | Use |
|---|---|---|
| `focus_every` (`engine.rs:48`) | 2 s | guard period |
| `intervention_cooldown_secs` | 20 s | per key; UI clamps 5..=600 (`FocusOptions.tsx:74-78`) |
| `DISABLE_WARNING_WINDOW_MINS` | 15 | three-warning window (engine side unused, see 11.3) |
| `SESSION_MIN_MINUTES` / `SESSION_MAX_MINUTES` | 5 / 240 | `validate_session` |
| `PRAISE_MIN_MINUTES` | 5 | praise only for sessions at least this long |
| `TERMINAL_GRACE_SECS` | 60 | terminals exempt at session start |
| `PROMPT_SWITCH_THRESHOLD` / `PROMPT_WINDOW_MINS` / `PROMPT_COOLDOWN_MINS` | 10 / 15 / 45 | "a lot of windows" prompt |
| `KNOWN_DOMAINS_DAYS` (`engine/src/focus.rs:22`) | 30 | suggestions in the search box |
| URL cache TTL (`browser.rs:117`, `windows/browser.rs:41`) | 20 s | per `(app, title)` |
| `DENIED_BACKOFF` (`browser.rs:115`) | 30 min | macOS Automation denial |
| `INTERVENTION_AUTO_CLOSE_MS` (`Intervention.tsx:13`) | 7000 | panel lifetime |

Caps: interventions have **no daily cap, no quiet hours, no snooze, no "silent apps"**; the
only throttle is the per-key cooldown.

### 6.4 Friction when the user turns protection off

- Disabling or removing a target that was blocked in the last 15 minutes shows three warnings
  in a row (`apps/desktop/src/components/focus/DisableWarnings.tsx`, `RECENT_BLOCK_MS`,
  `TOTAL_STEPS = 3`, copy under `focus.warnings.1..3`); otherwise a single confirmation.
- This is **UI only**. The commands `set_focus_target_enabled`, `remove_focus_target` and
  `update_settings` apply immediately; the `guard_enabled` toggle
  (`FocusOptions.tsx:93-95`) has no confirmation at all; quitting from the tray stops
  everything.

---

## 7. The nudge channel (`crates/ubiqx-engine/src/nudges.rs`)

- `pub fn emit(state, kind: NudgeKind, title, message, os_notification: bool)` (16-40): inserts
  into `nudges`, emits `EngineEvent::Nudge`, optionally an OS notification. **No gating of its
  own**; callers decide. `emit_attention` (42) = `Attention` + notification.
- `run_once` (47, every `nudges_every` = 60 s) is the policy pass and returns early when:
  `!settings.nudges.enabled`; tracker state is not `Running` (53); snoozed; the daily cap
  `settings.nudges.max_per_day` (default **4**) is reached (69-73); the open block's app is in
  `nudges.silent_apps` (80-88). Quiet hours (`QuietHours` default **enabled, 20:00-08:00**,
  `model.rs:771-779`) suppress policy nudges and the focus prompt (144).
- The cap counts every kind except `ReportReady` and `Attention`
  (`crates/ubiqx-storage/src/repos/nudges.rs:37`, `kind NOT IN (?2, ?3)`): a new `NudgeKind`
  would count against the cap, and would also need the `as_str`/`parse` arms
  (`model.rs:498-525`) and the round-trip test list (`model.rs:1353-1360`).
- The focus guard itself uses nudges only for the session-end praise
  (`end_session`, `engine/src/focus.rs:481-496`) and the "a lot of windows" prompt
  (`nudges.rs:142-160`).

---

## 8. Shell, IPC and front-end plumbing

| Layer | File | Focus-related contents |
|---|---|---|
| Engine API | `crates/ubiqx-engine/src/handle.rs:387-445` | `installed_apps`, `known_domains`, `focus_targets`, `add_focus_target(kind, name, key)`, `set_focus_target_enabled`, `remove_focus_target`, `interventions(limit)`, `focus_status`, `start_focus_session(task, minutes)`, `stop_focus_session`, `test_intervention` |
| Ports bundle | `crates/ubiqx-engine/src/deps.rs:29-97` | `PlatformPorts { activity, urls, idle, capturer, permissions, secrets, notifier, update_feed, apps, enforcer, presenter }`, `Repos { .., focus }`, `Repos::from_store` trait bound list |
| Platform bundle | `crates/ubiqx-platform/src/lib.rs:54-121` | `PlatformServices` (same minus `presenter`), `native_in`, `scripted`; per-OS `services()` in `macos/mod.rs:28`, `windows/mod.rs:29`, `linux/mod.rs:38` |
| Composition root | `crates/ubiqx-app/src/lib.rs:319-390` | `App::start(config, sink)`; `AppConfig.presenter`; `APP_ID = "ai.ubiqx.app"` (60); database file `ubiqx.db` |
| Tauri commands | `apps/desktop/src-tauri/src/commands.rs:802-897`, registered at 983-991 | `list_installed_apps`, `list_known_domains`, `list_focus_targets`, `add_focus_target`, `set_focus_target_enabled`, `remove_focus_target`, `list_interventions`, `get_focus_status`, `start_focus_session`, `stop_focus_session`, `test_intervention` (all `blocking(engine(&state), ..)`) |
| Tauri shell | `apps/desktop/src-tauri/src/lib.rs` | `TauriSink` (event `"engine"`), `TauriNotifier`, `TauriPresenter`, `show_intervention_window`, capabilities `default.json` / `intervention.json` |
| IPC client | `apps/desktop/src/lib/ipc.ts:152-170` | typed wrappers; outside Tauri every call is served by `lib/mock.ts` (`commands` map, line ~2100; unknown command throws) |
| Types | `apps/desktop/src/lib/types.ts:241-256, 490-569` | `FocusSettings`, `FocusTarget`, `Intervention`, `InterventionAction` union, `FocusSession`, `FocusStatus`, `EngineEvent` union |
| Store | `apps/desktop/src/lib/store.ts:102-118, 250-264, 338-405` | focus slice; on `intervention` prepends and calls `loadFocus()`; on `focus_session` patches status |
| Pages | `apps/desktop/src/pages/Focus.tsx`, `Intervention.tsx`; routes in `App.tsx`; nav entry in `components/layout/Sidebar.tsx:66` | |
| Components | `apps/desktop/src/components/focus/` | `SessionCard`, `TargetSearch`, `TargetList`, `InterventionsList`, `FocusOptions`, `DisableWarnings`, `DurationChips`, `useCountdown` |
| i18n | `apps/desktop/src/i18n/messages/focus.ts` (pt-BR and en) | `parity.test.ts` fails when a key or placeholder exists in only one locale; `focus.action.<action>` is looked up dynamically in `InterventionsList.tsx:27` |
| CLI | `crates/ubiqx-cli/src/main.rs:133-143` | prints `Intervention` and `FocusSession` events |

Struct-literal sites that break when a field is added to `PlatformPorts`, `LoopConfig` or the
`Repos::from_store` bound: `crates/ubiqx-app/src/lib.rs:360`,
`crates/ubiqx-engine/tests/focus.rs:205, 231, 775`, `tests/pipeline.rs:248, 271`,
`tests/screenshot_read.rs:52`; for `PlatformServices`: the three `services()` functions and
`PlatformServices::scripted`.

---

## 9. Tests and doubles

- `crates/ubiqx-core/src/focus.rs:260-425`: key normalisation, `find_hit` (apps, parent
  domains, disabled targets), exemptions, message rotation, `recently_blocked`, session
  validation, switch counting and prompt rule.
- `crates/ubiqx-engine/src/focus.rs:637-687`: `exemptions_cover_every_desktop`.
- `crates/ubiqx-engine/tests/focus.rs` (951 lines, in-memory SQLite, `FixedClock`, all loops
  parked at 3600 s, `without_sampler: true`, `guard_once` called by hand):
  `blocked_app_is_quit_once_per_cooldown`, `blocked_site_matches_parent_domain_and_falls_back_to_blanking`,
  `adding_an_existing_target_re_enables_it_and_inputs_are_validated`,
  `distraction_rules_only_count_during_a_session`, `exempt_apps_are_never_enforced`,
  `presenter_failure_falls_back_to_a_notification`, `session_start_hides_others_and_runs_the_shortcut`,
  `session_end_stores_the_praise_nudge`, `session_end_praise_follows_the_nudge_toggles`,
  `a_session_left_by_the_previous_run_is_restored`,
  `focus_prompt_fires_at_ten_switches_and_respects_its_cooldown`,
  `known_domains_come_from_the_users_blocks`.
  Doubles defined there: `Foreground` (settable window + URL; `supports` delegates to
  `ubiqx_platform::browser::script_for`, i.e. macOS bundle ids), `RecordingPresenter`,
  `RecordingNotifier`, `CollectSink`, `MemorySecrets`; from the platform crate `MockEnforcer`,
  `MockAppCatalog`, `SyntheticCapturer`, `StaticUpdateFeed`. **This harness is the model for
  nofap engine tests.**
- `crates/ubiqx-storage/src/repos/focus.rs:340+`: upsert/order, interventions newest first and
  count, sessions, wipe keeps targets.
- Platform: `macos/enforce.rs` (script text per family, hide script), `linux/enforce.rs`
  (never acts on itself, `run_shortcut` runs, display actions fail softly), `browser.rs`,
  `browser_url.rs`, `linux/browser.rs`. The Windows enforcer and UIA resolver have **no unit
  tests**.
- Front end: `pages/Intervention.test.tsx`, `pages/Focus.test.tsx`,
  `components/focus/FocusOptions.test.tsx`, `SessionCard.test.tsx`,
  `components/ubi/FocusPrompt.test.tsx`, `lib/mock.test.ts`, `i18n/parity.test.ts`.

---

## 10. Extension points for an always-on category block

Ordered along the path a hit travels.

1. **Switch and early exit** (`engine/src/focus.rs:173-188`). A category block must not hang
   off `focus.guard_enabled`, and must defeat the early exit
   `targets.is_empty() && !rules_apply`. Either extend `guard_once` (compute
   `nofap_active` before line 173 and include it in both conditions) or add a sibling pass
   (`nofap::guard_once`) called from `focus_loop` / its own loop in `engine.rs`
   (`LoopConfig` gets a field; update the literals listed in section 8). A sibling pass keeps
   the session timer and focus semantics untouched but must share the foreground/URL read to
   avoid doubling platform calls.
2. **Hit source** (`focus.rs:211-221`). The slot between `find_hit` and `distraction_hit` is
   where a list lookup goes: `domain` is already computed. Shape of the hit =
   `distraction_hit`'s: `Hit { target_id: None, kind: Site, name, key, app_id }`.
   `Hit`, `intervene`, `attempt` and `next_message` are private to `focus.rs`; a new module
   needs them `pub(crate)` or needs to live in the same file.
3. **Do not store the list as `focus_targets`**: `guard_once` loads all targets from SQLite
   every 2 s and `find_hit` is a linear suffix scan; the Focus page lists every row; wipe
   keeps them. An extensive list belongs in an in-memory set with a label-by-label suffix walk
   (pure, in `ubiqx-core`, unit-testable), reusing `normalize::domain_of` /
   `normalize_key` semantics (lower-case, `www.` stripped, subdomains covered).
4. **Exemptions** (`is_exempt_window`): keep them, and add the new product's own bundle id,
   display name and executable stems to `EXEMPT_APP_IDS`, `EXEMPT_APP_NAMES`,
   `SELF_BUNDLE_ID`, `SELF_STEMS` and the literal in `hide_others_script`.
5. **Cooldown key** (`focus.rs:226-235`, `332-340`): today per target key. For a category the
   natural key is the category (or category + browser), otherwise every new domain is its own
   cooldown and the same domain is free for 20 s.
6. **Enforcement policy** (`intervene`, 294-312): decided by `hit.kind`. A configurable
   policy ("close the tab / close the app / only warn") is a new parameter on the hit or a
   setting read here. New capabilities (for example re-checking the URL before closing,
   closing the window of a non-scriptable browser) are new `Enforcer` methods implemented in
   four places (three OS adapters + `MockEnforcer`).
7. **Message** (`focus.rs:313`): `next_message` rotates productivity copy. A category hit
   needs its own copy source (pure function in core, both languages), selected by hit origin.
8. **Record** (`Intervention`, table `interventions`): to tell origins apart add a column
   (migration V3 `ALTER TABLE interventions ADD COLUMN ...`, `row_to_intervention`,
   `INSERT_INTERVENTION`, TS `Intervention`, mock state) or write to a separate table through
   a new repo port (then also add it to `Repos`, `Repos::from_store`, `MaintenanceRepo` wipe
   list). Decide what is stored: today `name` and `key` (the domain) are persisted forever
   and shown in `InterventionsList`.
9. **Events**: `EngineEvent` (`model.rs:1055-1097`) is the only engine → UI channel; a new
   variant needs the TS union (`types.ts:560-569`) and a `case` in the store's event reducer
   (`store.ts:~200-266`); unknown variants fall to `default: break`.
10. **Presenter** (`InterventionPresenter::show(&Intervention)`): the port takes an
    `Intervention`; the Tauri side keys the window by label `"intervention"`, a fixed size and
    the route `#/intervention?id=`. A counselling panel needs either a second window (new
    label, new capability JSON listing that label, new route outside the `Gate`) or a
    variant of the existing one (size per kind; page branching on the payload).
11. **Notification fallback** (`focus.rs:366-375`): title = target name. Replace with neutral
    text for category hits.
12. **Settings**: add a `#[serde(default)]` struct on `Settings`; mirror in `types.ts`; mock
    defaults. State that should resist a one-click disable (commitment, lock-until) should
    not live in `Settings` (see 6.2); `KvRepo` (`ports.rs:314-317`) or a dedicated table with
    dedicated commands is the alternative. Note `wipe_user_data` clears `kv`.
13. **Tracker-side redaction**: `Segmenter::is_blocked`
    (`crates/ubiqx-core/src/segmenter.rs:100-115`) and `SegmenterConfig::from(&Settings)` are
    where a nofap domain match can be turned into a `[privado]` block so the domain never
    reaches the timeline, the AI classifier, the daily report or the export.
14. **Nudges**: `nudges::emit` is callable from anywhere; a new `NudgeKind` needs the arms
    in `model.rs:498-525`, the test list, the TS union (`types.ts:~123-130`) and a decision
    about the daily cap (`repos/nudges.rs:37`).
15. **Front end**: sidebar item (`Sidebar.tsx:66`), route in `App.tsx`, i18n namespace under
    `apps/desktop/src/i18n/messages/` registered in `i18n/index.ts` (`NAMESPACES`), IPC
    wrappers, mock commands, store slice.
16. **Friction**: `DisableWarnings` is the reusable pattern (and its copy is in the tone the
    brief asks for), but enforce it in the engine if it is meant to hold.

---

## 11. Surprising, fragile or missing

1. **Cooldown suspends enforcement.** Inside `intervention_cooldown_secs` the pass returns
   before `intervene` (`focus.rs:226-235`): a just-closed site can be reopened and used for up
   to 20 s (default), repeatedly.
2. **No retention for `interventions`**, and a hit that cannot be enforced (`Notified`)
   produces a new row, a new window and possibly a new OS notification every cooldown for as
   long as the page stays in front.
3. `ubiqx_core::focus::recently_blocked` and `DISABLE_WARNING_WINDOW_MINS` are used only by
   their own unit test; the 15-minute rule is re-implemented in
   `DisableWarnings.tsx:16-17`. Friction is not enforced server-side.
4. **The guard runs before onboarding consent and regardless of tracking.** Harmless today
   (no targets can exist before the Focus page is reachable), but a built-in list that is
   active by default would enforce from the first launch. It must be opt-in.
5. **The guard never surfaces a missing Automation permission** (`focus.rs:201-204` logs at
   `debug`); only the tracker's sampler emits `PermissionRequired`. With tracking off, a
   denied browser means silent blindness for 30 minutes at a time.
6. **Stale URL + blind close.** The URL comes from a 20 s cache keyed by `(app, title)`; the
   close acts on whatever tab is active when the script/keystroke lands, without re-checking.
   With an empty title (macOS without Screen Recording) every tab of a browser shares one
   cache key, so the guard can be up to ~20 s late or act on the tab the user switched to.
7. **Windows address bar ≠ current page.** `UiaUrlResolver` reads the omnibox *text*. A
   domain-shaped string that is being typed or auto-completed (not yet navigated) is returned
   as `https://<text>` by `normalize_typed_url`, so the guard can close the tab that is
   currently showing something else. `docs/WINDOWS-LINUX.md` says a value being typed "is not
   a URL"; that only holds when the text does not look like a host. (*By reading.*)
8. **Synthetic `Ctrl+W` / `Ctrl+L`** on Windows and Linux combine with whatever modifier keys
   the user is physically holding and go to whatever control has focus; success is inferred
   only from "the same app is still in front".
9. **Windows `quit_app` blocks the guard pass for 1.5 s, then hard-kills** and reports success
   unconditionally. **macOS `quit_app`** reports success when the request was sent, even if
   the app shows a "save changes?" sheet and stays. **Linux** never escalates past `SIGTERM`.
10. **macOS first contact can freeze the guard for up to 120 s**
    (`FIRST_CONTACT_TIMEOUT`), including the session-end timer, since passes are sequential.
11. **Linux coverage is thin**: native Wayland windows are invisible to the guard (Firefox on
    current GNOME, GTK4 apps, Chrome with the Wayland backend), and under X11 a site is only
    recognised when its title prints its domain.
12. **macOS Firefox (and any browser outside `script_for`)**: site targets silently never
    fire; there is not even a `Notified` intervention, because no domain is ever known.
13. **Intervention page falls back to the YouTube sample** when the id is not among the last
    50 interventions or the IPC call fails (`Intervention.tsx:64-67`), so a real hit can show
    sample text.
14. **`delete_all_data` during an active session** removes the `focus_sessions` row but not
    `FocusRuntime.session`. The next `update_session` (`execute_expecting_row`) then returns
    `NotFound`: in `intervene` this happens after enforcement and the insert but before the
    event and the window (`focus.rs:341-344`), and at `ends_at` `end_session` fails before it
    clears the runtime session (`focus.rs:463-470`), so every later pass fails at step 2
    until restart. (*By reading; not executed.*)
15. **Strict enum decoding** (2.3): any new `kind`/`action` value breaks older builds reading
    the same database.
16. **`Settings` is last-writer-wins from the UI** (6.2) and the engine validates none of the
    focus numbers.
17. The fallback OS notification's **title is the target name** (the domain), which for this
    module would put it in the notification centre.
18. `supported_browsers()` (`browser.rs:49-60`) lists 8 ids while `script_for` handles 14;
    harmless, but the two can drift.
19. `EXEMPT_APP_NAMES` matches by display name on every OS: any app whose name is "Finder",
    "Terminal" (during the grace) or "ubiqX" is exempt regardless of its id.
20. The tracker is a second path by which a visited domain is recorded (blocks, timeline, AI
    payload, daily report, export) unless the block is shorter than `min_block_secs` (20 s,
    dropped as insignificant), the window is private browsing, or the domain is in
    `blocked_domains`. Nothing links the guard's hit to that redaction.

---

## 12. Docs versus code

| Doc statement | Code |
|---|---|
| `docs/WINDOWS-LINUX.md` table: macOS closes a blocked app with "`quit` via AppleScript" | `NSRunningApplication::terminate()` (`macos/enforce.rs:119-135`); no AppleScript involved |
| Same doc: on Windows "while you type in the bar the value is not a URL" | True only for text that is not host-shaped (finding 11.7) |
| `docs/ARCHITECTURE.md` | Has no section on the focus guard, the `Enforcer`/`InterventionPresenter` ports, the `focus_loop`, the V2 tables or the intervention window; the loop diagram around line 107 predates them |
| `README.md:83` "Foco: fechar app / aba ... sim / sim / sim" | Accurate only within the browser and display-server coverage of 3.4 (no Firefox on macOS, title-dependent on Linux X11, nothing on native Wayland); the Linux caveat is in `WINDOWS-LINUX.md`, the macOS Firefox one is only in a code comment (`browser.rs:3-4`) |
| `ports.rs:128` doc comment "Runs the macOS Shortcut with this name" | On Windows/Linux the same method runs a command line (`cmd /C`, `sh -c`); the setting names keep the `macos_` prefix |
| `InterventionAction::AppQuit` doc "asked to quit and agreed" | Means "request accepted by the OS call" (finding 11.9) |

---

## 13. What this means for the standalone product

- `ubiqx_app::App::start` + `Engine::start` bring up every loop. With
  `tracking_enabled = false` the sampler thread idles and no blocks, screenshots or AI calls
  are produced, while `focus_loop` keeps running: the guard half already works without the
  productivity half. `Engine::start_with(deps, LoopConfig { without_sampler: true, .. })`
  exists for not even spawning the sampler.
- What stops working without the tracker: Automation-permission prompts to the UI (11.5),
  every policy nudge (`run_once` needs `TrackerState::Running`), `known_domains`.
- Identity is hard-coded in several places a second product must parameterise:
  `ubiqx_app::APP_ID`, `SELF_BUNDLE_ID`, `EXEMPT_APP_IDS/NAMES`, `SELF_STEMS`, the window
  title `"ubiqX"`, `tauri.conf.json` (`productName`, `identifier`), the database file name,
  the autostart argument `--minimized`.
- The only exit path of the desktop app is the tray "Sair" item (`lib.rs:640-647`; closing a
  window and Cmd+Q only hide). The guard, the window and any DNS configuration applied by the
  app share that lifetime unless the blocking state is made to persist outside the process.
