# Code map: engine and storage patterns a new module must follow

Read-only mapping of `crates/ubiqx-engine`, `crates/ubiqx-core`, `crates/ubiqx-storage`,
`crates/ubiqx-platform`, `crates/ubiqx-app`, `crates/ubiqx-cli`, the engine integration tests
and the CI, at commit `410c3d6` (branch `claude/sharp-bardeen-sk27ae`). Every `file:line`
was read in the source; nothing here was compiled or run in this pass.

Sibling maps in this folder go deeper on two neighbours and are not repeated here:
`map-focus-guard.md` (the guard loop, enforcement per OS, the intervention window) and
`map-vision-ai.md` (screenshots, the vision request, providers, budget, privacy controls).

Contents

1. Constraints that shape the design most
2. Layering and crate rules
3. Runtime: start-up, loops, shutdown
4. Shared state and how a settings write is applied
5. Settings: struct, defaults, "validation", storage
6. Storage: connection, migrations, conventions, repository template
7. Ports: how a platform port with three OS adapters and a mock is added
8. How the engine is exposed: `EngineHandle`, read models, IPC, events
9. Error handling conventions
10. Test conventions
11. Lint, format, CI
12. Checklist: files to touch for settings + 2 tables + worker + platform port
13. Surprising, fragile or missing
14. Docs versus code
15. What this means for the standalone product

---

## 1. Constraints that shape the design most

1. **One settings blob, replaced whole, with no friction.** `Settings` is one JSON row
   (`settings.id = 1`); every write replaces the struct (`EngineState::apply_settings`,
   `crates/ubiqx-engine/src/state.rs:120-168`). The UI sends the whole struct
   (`update_settings`, `apps/desktop/src-tauri/src/commands.rs:607`). `#[serde(default)]`
   on the struct means a payload that omits a key silently resets it to the default. Any
   protection switch that lives in `Settings` can be turned off by any settings write.
2. **"Apagar todos os dados" deletes the whole `kv` table** and the tables listed in
   `WIPE` (`crates/ubiqx-storage/src/repos/maintenance.rs:11-21`). Durable module state
   must not live in `kv`, and each new table needs an explicit decision: wiped or kept.
3. **The tracker deliberately does not see private windows.** Incognito titles, private
   mode and `blocked_apps` turn a sample into `"[privado]"` with `url: None` and
   `window_id: None` (`crates/ubiqx-core/src/segmenter.rs:101-150`). Activity blocks
   therefore cannot be the detection source; a module that must look at what is on
   screen needs its own pass over the platform ports, like the focus guard has.
4. **No code in the repository runs with elevated rights.** A search for
   `sudo|pkexec|runas|elevat` over `crates/ubiqx-platform/src` and
   `apps/desktop/src-tauri/src` finds nothing; every adapter acts as the logged-in user.
   System DNS or hosts changes are new territory and need a new, thin, isolated layer.
5. **Every CLI subcommand starts the full engine** (`crates/ubiqx-cli/src/main.rs:221-363`),
   and `ubiqx track` does so on the native platform. Anything a worker does at start-up
   happens there too.
6. **The scripted platform is consumed by reading it.** `ScriptedPlatform::foreground()`
   advances the scenario on every call (`crates/ubiqx-platform/src/mock.rs:221-227`), so a
   second loop that reads the foreground steals samples from the tracker in
   `ubiqx simulate`, in `UBIQX_SCRIPTED` desktop demos and in tests.
7. **Schema version is a hard gate both ways.** `rusqlite_migration` refuses a database
   whose `user_version` is higher than the migrations it knows
   (`MigrationDefinitionError::DatabaseTooFarAhead`). Two products that open the same
   database file must ship the same migration list.
8. **Struct literals without `..Default::default()`** in tests and the composition root
   mean a new field in `LoopConfig`, `PlatformPorts`, `PlatformServices` or `AiPorts`
   breaks compilation in a known list of places (section 12).
9. **AI health ignores ports it does not know.** `key_health` answers `NotConfigured`
   when `deps.ai.remote` and `deps.ai.report_writer` are both `None`
   (`state.rs:256-257`); `remote_allowed()` then refuses. A build that wires only a new
   AI port (the standalone product) is never armed unless this function changes.
10. **CI publishes on every push to any branch.** `ci.yml` triggers on
    `branches: ["**"]` and `publish-continuous` runs `if: github.event_name == 'push'`;
    the header comment says installed apps then offer that build as an update.

---

## 2. Layering and crate rules

| Crate | Role | May depend on |
|---|---|---|
| `ubiqx-core` | Domain types (`model.rs`), ports (`ports.rs`), pure logic (`focus`, `segmenter`, `rules`, `insights`, `scheduler`, `normalize`, `redact`, `lang`, `license`, `update`) | No OS API, no database driver, no HTTP client (`crates/ubiqx-core/src/lib.rs:19`). Dependencies: `async-trait`, `chrono`, `data-encoding`, `ed25519-dalek`, `regex`, `serde`, `serde_json`, `sha2`, `thiserror`, `url`, `uuid` |
| `ubiqx-storage` | SQLite adapter: `Db`, `SqliteStore`, migrations, one module per repo port | `ubiqx-core`, `rusqlite` (bundled), `rusqlite_migration` |
| `ubiqx-platform` | OS adapters behind `cfg(target_os)`, plus `mock`, portable helpers | `ubiqx-core`; per-OS crates in `crates/ubiqx-platform/Cargo.toml` |
| `ubiqx-ai` | HTTP clients and AI port implementations, fakes | `ubiqx-core` |
| `ubiqx-engine` | Application layer: loops, `EngineState`, `EngineHandle`, read models | `ubiqx-core` only; storage, platform and ai are **dev-dependencies** (`crates/ubiqx-engine/Cargo.toml`) |
| `ubiqx-app` | Composition root: `App::start(AppConfig, sink)` | all of the above |
| `ubiqx-cli` | `ubiqx` binary | `ubiqx-app`, `ubiqx-core`, `ubiqx-engine` |
| `apps/desktop/src-tauri` (`ubiqx-desktop`) | Tauri shell: IPC commands, tray, sink, notifier, presenter | `ubiqx-app`, `ubiqx-core`, `ubiqx-engine` |

The engine "never talks to an OS API, a database driver or an HTTP client directly — only
to the traits in `ubiqx_core::ports`" (`crates/ubiqx-engine/src/lib.rs:14-15`). The
established split for a feature is: pure decisions and all user-facing text in
`ubiqx-core/src/<feature>.rs` (constants, PT and EN strings, `validate_*`), wiring in
`ubiqx-engine/src/<feature>.rs`. `crates/ubiqx-engine/src/focus.rs:5-7` states it:
"Every decision that needs no I/O lives in `ubiqx_core::focus`".

Workspace: `Cargo.toml` lists eight members; `default-members` excludes
`apps/desktop/src-tauri` so `cargo test`/`cargo clippy` on Linux need no webkit.
`services/ubi-api` is a separate workspace. Shared dependency versions live in
`[workspace.dependencies]`; a new crate uses `foo.workspace = true`.
`rust-version = "1.85"`, edition 2021, toolchain `stable`.

---

## 3. Runtime: start-up, loops, shutdown

### 3.1 Start-up (`crates/ubiqx-engine/src/engine.rs:63-115`)

`Engine::start(deps)` calls `Engine::start_with(deps, LoopConfig::default())` and must be
called inside a tokio runtime. In order:

1. `std::fs::create_dir_all(&deps.data_dir)` (line 67).
2. `deps.repos.settings.load()?` (69), `restore_open_block` (70, defined at 121).
3. `EngineState::new(deps, settings, open_block)` (71) returns `Arc<EngineState>`.
4. `CancellationToken::new()` (72); `mpsc::channel::<ActivitySample>(256)` (73).
5. Spawns (75-107): the sampler **OS thread** unless `cfg.without_sampler`, then eight
   tokio tasks.
6. `state.refresh_tracker_state()` and one `EngineEvent::AiHealth` (109-112).
7. Returns `(EngineHandle { state, cancel }, tx)`.

There is no restart: one engine per process, and the handle is `Clone`.

### 3.2 The loops

`LoopConfig` (`engine.rs:21-37`, defaults 39-54):

| Field | Default | Loop | First pass | Body runs on |
|---|---|---|---|---|
| (settings) `sample_interval_secs` | 5 s, clamped 1..=60 | `sampler::spawn` thread `ubiqx-sampler` (`sampler.rs:14-54`) | immediately | its own OS thread, sleeps in 250 ms steps |
| n/a | n/a | `tracker::run` (`tracker.rs:14-43`) | on first sample | `spawn_blocking(handle_sample)` per sample |
| `classify_every` | 30 s | `classify_loop` (`engine.rs:142`) | **immediately** | async task (repo calls inline, AI awaited) |
| `reports_every` | 30 s | `reports_loop` (158) | after one period (`tick.tick().await` at 163) | async |
| `nudges_every` | 60 s | `nudges_loop` (176) | after one period (178) | `spawn_blocking(nudges::run_once)`, then timed private-mode expiry (187-194) |
| `retention_every` | 3600 s | `retention_loop` (200) | **immediately** | `spawn_blocking(screenshots::cleanup)` |
| `update_initial_delay` / `update_every` | 45 s / 6 h | `update_loop` (264) | after the initial delay | async, `sleep`-based |
| `focus_every` | 2 s | `focus_loop` (218) | after one period (222) | `spawn_blocking(focus::guard_once)` |
| `license_initial_delay` / `license_every` | 5 s / 3600 s | `license_loop` (241) | after the initial delay | async, `sleep`-based |
| `without_sampler` | `false` | skips the sampler thread | | |

Two templates exist. Copy one of them exactly.

Interval template for a blocking pass (`focus_loop`, `engine.rs:218-234`):

```rust
async fn focus_loop(state: Arc<EngineState>, cancel: CancellationToken, every: Duration) {
    let mut tick = interval(every);          // MissedTickBehavior::Skip, engine.rs:136-140
    tick.tick().await;                       // skip the immediate first tick
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {
                let st = state.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || focus::guard_once(&st))
                    .await
                    .unwrap_or_else(|e| Err(CoreError::Other(e.to_string())))
                {
                    tracing::warn!(error = %e, "focus guard pass failed");
                }
            }
        }
    }
}
```

Sleep template with an initial delay (`license_loop`, `engine.rs:241-258`): `let mut delay =
initial_delay; loop { select! { cancelled => break, sleep(delay) => { delay = every; ... } } }`.

Rules the existing loops follow:

- The pass itself is a free function `pub fn <thing>_once(state: &Arc<EngineState>) ->
  CoreResult<...>` in the feature module (`focus::guard_once`, `nudges::run_once`,
  `classify::run_once`, `reports::run_due`). The loop only schedules and logs. Tests call
  the pass directly.
- A failed pass is logged with `tracing::warn!` and the loop continues. No loop ever
  exits on error.
- Platform calls are blocking, so passes that touch the platform go through
  `spawn_blocking`. Repository calls are synchronous too; the classify and report passes
  call them inline on the async task, the others inside `spawn_blocking`.
- A pass that needs both blocking work and an AI call does the blocking part in
  `spawn_blocking` and then awaits the AI (`EngineHandle::advice`,
  `crates/ubiqx-engine/src/handle.rs:269-290` then 321).
- Per-loop cadence is a `LoopConfig` field so tests can stretch it to 3600 s.
- Skipping the first tick is the documented fix for start-up races: see the comment on
  `reports_loop` (`engine.rs:160-162`) and on `focus_loop` (220-221).

### 3.3 Shutdown (`handle.rs:484-500`)

`shutdown()` cancels the token, waits at most `SHUTDOWN_GRACE` (2 s, `handle.rs:34`) on
`tracker_stopped_cv` for the tracker to persist the open block, then closes the block
itself if needed. No other loop is awaited, and an in-flight `spawn_blocking` pass is not
joined. The desktop shell calls it from the tray quit path
(`apps/desktop/src-tauri/src/lib.rs:471`), `RunEvent::ExitRequested` with an exit code
(645) and `RunEvent::Exit` (652). A worker therefore cannot count on a clean-up step at
exit; anything it must leave in a known state has to be idempotent and re-checked at the
next start.

---

## 4. Shared state and how a settings write is applied

`EngineState` (`state.rs:16-43`) is one `Arc` with public fields, each behind its own
`parking_lot` lock: `deps: EngineDeps`, `settings: RwLock<Settings>`,
`segmenter: Mutex<Segmenter>`, `tracker: RwLock<TrackerState>`,
`ai_health: RwLock<AiHealth>`, `license: RwLock<LicenseStatus>`,
`last_capture: Mutex<HashMap<Id, DateTime<Utc>>>`, `paused: RwLock<bool>`,
`permission_warned: Mutex<bool>`, `budget_paused: AtomicBool`,
`tracker_stopped: Mutex<bool>` + `tracker_stopped_cv: Condvar`, `update: UpdateChecker`,
`focus: Mutex<FocusRuntime>`.

Two ways a feature keeps in-memory state, both in use:

- A runtime struct in a mutex on `EngineState`, restored from its repository at
  construction: `FocusRuntime::restore(deps.repos.focus.as_ref())` (`state.rs:61`,
  `crates/ubiqx-engine/src/focus.rs:118-132`). A read failure is logged and treated as
  "nothing there", so the engine still starts.
- A self-contained component that takes only the ports it needs in its constructor and
  never sees `EngineState`: `UpdateChecker::new(current, feed_url, source, kv, settings,
  notifier, sink, clock)` (`crates/ubiqx-engine/src/update.rs:16-86`, built at
  `state.rs:62-71`). Its unit tests build a rig from in-file doubles
  (`update.rs:209` onwards). This shape is the one a second product can reuse without
  the rest of the engine.

Accessors: `state.now()` (the injected clock, `state.rs:90`), `state.settings()` (clone,
94), `state.license_gate()` (106), `state.remote_allowed()` (228-243).

`apply_settings(settings)` (`state.rs:120-168`) is the single write path:

1. `settings.sanitize()` (123).
2. The one cross-field rule that can fail: switching to the managed provider without a
   valid `monthly_managed` license returns `CoreError::LicenseRequired` before anything
   is saved (124-129).
3. `self.deps.repos.settings.save(&settings)?` (130): persisted **before** the in-memory
   copy changes.
4. Side effects computed by comparing old and new: close the open block when tracking
   was turned off or private mode flipped (134-152), recompute AI health when the
   provider changed (155-161), lift a budget pause when the budget changed (164-166).
5. `*self.settings.write() = settings` (157), `refresh_tracker_state()` (158).

This comparison block is the extension point for "a setting changed, the module must
react" (re-apply a system state, start or stop something) and for refusing a transition.

Every caller does read-modify-write on the whole struct: `handle.settings()`, change a
field, `apply_settings`. That includes background code (`nudges_loop` expiring private
mode, `engine.rs:187-194`) and handle methods (`set_private_mode`, `snooze_nudges`,
`handle.rs:99-140`). There is no lock across the read and the write, so two writers can
lose each other's change. A worker should not write `Settings`.

`remote_allowed()` (`state.rs:228-243`) is the consent and health gate for any remote AI
call: `false` when `local_only`, when `!onboarding_done`, when the license blocks AI
(`LICENSE_ENFORCEMENT` is `Soft` today, `crates/ubiqx-core/src/license.rs:53`), or when
health is `NotConfigured`/`Paused`; `Degraded` allows again after `until`. The order used
before a remote call is `budget_allows(state)?` first, then `remote_allowed()`
(`crates/ubiqx-engine/src/classify.rs:370-371`), because the budget check is what lifts a
budget pause.

---

## 5. Settings: struct, defaults, "validation", storage

### 5.1 Shape (`crates/ubiqx-core/src/model.rs`)

- `pub struct Settings` at 905-970, `#[derive(Debug, Clone, PartialEq, Serialize,
  Deserialize)]`, `#[serde(default)]` on the struct (906).
- `impl Default for Settings` at 976-1019. Defaults that matter here:
  `tracking_enabled: true`, `sample_interval_secs: 5`, `idle_threshold_secs: 180`,
  `screenshot_interval_secs: 120`, `screenshot_max_edge: 1280`,
  `screenshot_retention_hours: 24`, `vision_policy: VisionPolicy::AllExceptBlocked`,
  `max_vision_per_hour: 20`, `ai_monthly_budget_usd: 5.0`, `local_only: false`,
  `language: "pt-BR"`, `launch_at_login: true`, `onboarding_done: false`,
  `check_updates: true`.
- Feature groups are nested structs with their own `#[serde(default)]` and `Default`:
  `NudgeSettings` (829-868), `FocusSettings` (871-903, field `pub focus: FocusSettings`
  at 969). `FocusSettings` derives `Eq`; `Settings` cannot (it holds `f64`/`f32`).
- A boolean that must default to `true` when missing uses a named function:
  `#[serde(default = "default_true")] pub check_updates: bool` (966-967, function at 972).
  With struct-level `#[serde(default)]` this is redundant but it is the house style.
- `Settings::is_private(now)` (1046), `ui_language()` (1028).

### 5.2 Validation

There is none in the usual sense. `Settings::sanitize()` (1040-1043) does exactly two
things: `reconcile_models()` and `normalize_language()`. It runs on every write
(`state.rs:123`) and on load (`state.rs:54`). Out-of-range numbers are tolerated and
clamped where they are used: `sample_interval_secs.clamp(1, 60)` (`sampler.rs:28`),
`.max(1)` (`tracker.rs:48`), `cooldown_mins.max(10)`
(`crates/ubiqx-engine/src/nudges.rs:108`), `screenshot_retention_hours.max(1)`
(`crates/ubiqx-engine/src/screenshots.rs:142`). Input that arrives through a dedicated
command is validated in core and mapped to `CoreError::Invalid`:
`rules_focus::validate_session(task, minutes).map_err(CoreError::Invalid)?`
(`crates/ubiqx-engine/src/focus.rs:401`; bounds `SESSION_MIN_MINUTES = 5`,
`SESSION_MAX_MINUTES = 240` in `crates/ubiqx-core/src/focus.rs:16-17`).

So: range rules for new settings go into `sanitize()` (clamp silently) or into
`apply_settings` (refuse with an error), and both places are new uses of an existing
hook.

### 5.3 Storage

`SettingsRepo` for `SqliteStore` (`crates/ubiqx-storage/src/repos/settings.rs:16-35`):
`load()` returns `Settings::default()` until the first save, otherwise
`from_json(&raw)`; `save()` upserts row `id = 1` with `json(settings)` and
`ms(Utc::now())`. The table is `settings (id INTEGER PRIMARY KEY CHECK (id = 1), json TEXT
NOT NULL, updated_at INTEGER NOT NULL)` (`migrations.rs:142-146`).

**A new settings field needs no migration** (stated in
`crates/ubiqx-storage/src/lib.rs:18-20`). Two tests enforce the contract and must stay
green:

- `settings_default_serialises_with_missing_fields` (`model.rs:1416-1420`):
  `serde_json::from_str::<Settings>("{}") == Settings::default()`. Every new field must
  deserialise from nothing to its `Default` value.
- `settings_default_then_round_trip_with_serde_defaults`
  (`crates/ubiqx-storage/tests/repos.rs:1389-1438`): round trip and an old JSON
  (`{"language":"en-US"}`) loading with defaults.

The front end mirrors the struct by hand in `apps/desktop/src/lib/types.ts`
(`FocusSettings` at 242, `focus: FocusSettings` at 292); there is no code generation.

### 5.4 Adding a settings group

1. `crates/ubiqx-core/src/model.rs`: `pub struct NofapSettings` with `#[serde(default)]`,
   `impl Default`, a `pub nofap: NofapSettings` field in `Settings` and the same field in
   `impl Default for Settings`.
2. Clamp ranges in `Settings::sanitize()` if any.
3. React to changes in `EngineState::apply_settings`.
4. `apps/desktop/src/lib/types.ts`: the mirror type.
5. Tests: extend the two tests above only if a default is not the type's natural one.

---

## 6. Storage: connection, migrations, conventions, repository template

### 6.1 Connection (`crates/ubiqx-storage/src/db.rs`)

- `Db { conn: Arc<Mutex<Connection>> }` (30-32): **one** connection, all access
  serialised by a `parking_lot::Mutex`.
- `PRAGMAS` (19-22): `journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`,
  `foreign_keys = ON`.
- `Db::open(path)` creates parent directories, applies pragmas, runs migrations (43-49,
  `init` at 56-62). `Db::open_in_memory()` (52) is what tests use.
- `Db::with(f)` (65) for synchronous code; `Db::run(f)` (72) for async code through
  `spawn_blocking`.
- The app opens `<data_dir>/ubiqx.db` (`crates/ubiqx-app/src/lib.rs:328`).

Because there is a single connection, a long statement or a large transaction blocks the
tracker's writes for its whole duration.

### 6.2 Migrations (`crates/ubiqx-storage/src/migrations.rs`)

- One `const Vn: &str = r##"..."##` per version: `V1` at 14 (whole schema plus the four
  system categories), `V2` at 191 (focus guard).
- `pub fn migrations() -> Migrations<'static> { Migrations::new(vec![M::up(V1),
  M::up(V2)]) }` (235-237). Only `M::up`; there are no down migrations.
- The version is SQLite's `user_version`. `apply` (241-252) logs `from`/`to`.
- Rule in the module doc (lines 3-4): "append a new `M::up` ... never edit a migration
  that has shipped."

Adding version 3:

1. Add `const V3: &str = r##" ... "##;` with a doc comment describing it.
2. Append `M::up(V3)` to the vector at 236.
3. Update the three hard-coded version assertions from `2` to `3`:
   `migrations.rs:273`, `crates/ubiqx-storage/tests/repos.rs:234` and `:242`.

### 6.3 Column conventions (`crates/ubiqx-storage/src/lib.rs:13-21`, `convert.rs`)

| Domain type | Column | Helpers (`convert.rs`) |
|---|---|---|
| `DateTime<Utc>` | `INTEGER` unix **milliseconds** | `ms` (13), `opt_ms` (17), `dt` (22), `opt_dt` (27) |
| `NaiveDate` | `TEXT` `YYYY-MM-DD` | `date_str` (32), `date` (36) |
| `NaiveTime` | `TEXT` `HH:MM` | `time_str` (42), `time` (46), `opt_time` (52) |
| enum | `TEXT` through `as_str()`/`parse()` from core | `parse_enum(what, raw, T::parse)` (57), `opt_enum` (65) |
| list or nested struct | `TEXT` JSON | `json` (75), `from_json` (79) |
| `bool` | `INTEGER` 0/1 | rusqlite's own conversion |
| `usize` limit | bound as `i64` | `sql_limit` (84) |
| `COUNT(*)`/`SUM` | `i64` to `u64` | `count(row, idx)` (90) |

Schema style in `V1`/`V2`: `id TEXT PRIMARY KEY NOT NULL` (UUID v4 string from
`ubiqx_core::new_id()`, `model.rs:10`), `NOT NULL DEFAULT` on everything optional-looking,
a trailing SQL comment naming enum values (`-- app | site`), `CREATE INDEX idx_<table>_<col>`,
partial indexes for "active" rows (`idx_focus_sessions_active ... WHERE ended_at IS NULL`,
`migrations.rs:231`), `UNIQUE (kind, key)` for natural keys (202). History tables keep no
foreign key to configuration rows so history survives deletion ("Interventions keep no
foreign key to their target", doc at 188-190). Sub-millisecond precision is dropped, so
test fixtures use second-precision instants.

### 6.4 Repository template (`crates/ubiqx-storage/src/repos/focus.rs`)

One file per port, one `impl <Port> for SqliteStore` block (`focus.rs:169`):

- A column list macro, so `INSERT` and `SELECT` cannot drift:
  `macro_rules! target_cols { () => { "id, kind, name, ..." }; }` (18-22).
- Statements as `const` strings built with `concat!` (24-56), named `SELECT_*`,
  `INSERT_*`, `UPSERT_*`, `UPDATE_*`, `DELETE_*`, `COUNT_*`.
- `fn row_to_<type>(row: &Row<'_>) -> StorageResult<T>` reading columns **by name**
  (`row.get("kind")?`) and decoding enums with `parse_enum` (58-70).
- Methods are thin: `self.with(|conn| query_list(conn, SQL, params![..], row_to_x))`.
  Helpers in `crates/ubiqx-storage/src/store.rs`: `SqliteStore::with` (31, maps
  `StorageError` into `CoreError`), `query_list` (40), `query_opt` (52), `query_one` (64),
  `execute` (75), `execute_expecting_row(conn, sql, params, what, id)` (82, turns "0 rows
  changed" into `StorageError::NotFound`). All use `prepare_cached`.
- `named_params!` for inserts and full-row updates, `params!` for the rest.
- Upsert with a natural key: `INSERT ... ON CONFLICT(kind, key) DO UPDATE SET enabled =
  1`, then re-select by key and return the stored row (44-49, 178-208).
- Deletes are idempotent (`remove_target` ignores "0 rows"), updates are not
  (`execute_expecting_row`).
- `DELETE ... RETURNING <cols>` when the caller needs the deleted rows
  (`repos/screenshots.rs:43-46`).
- Multi-statement work uses `conn.transaction()` (`repos/maintenance.rs:32-37`).
- Unit tests sit at the bottom of the repo file (`focus.rs:334-547`) using
  `SqliteStore::new(Db::open_in_memory().expect(..))` and a `&dyn FocusRepo` coercion,
  because several ports share method names (`store.rs:9-14`). Cross-repo tests are in
  `crates/ubiqx-storage/tests/repos.rs`.

Port conventions (`crates/ubiqx-core/src/ports.rs`): every persistence trait is
`pub trait XRepo: Send + Sync` with synchronous methods returning `CoreResult<T>`
("Persistence ports are synchronous on purpose", lines 4-5). Method names are scoped by
the trait, with a noun suffix when a port covers several tables
(`insert_intervention`, `list_sessions`, `ports.rs:284-310`). Doc comments state the
ordering ("Newest first") and the not-found behaviour.

### 6.5 Registering a repository

1. `crates/ubiqx-core/src/ports.rs`: the trait.
2. `crates/ubiqx-storage/src/repos/<name>.rs` plus `mod <name>;` in
   `crates/ubiqx-storage/src/repos/mod.rs` (alphabetical).
3. `crates/ubiqx-engine/src/deps.rs`: a field `pub <name>: Arc<dyn XRepo>` in `Repos`
   (49-62), the bound `+ XRepo` in `from_store` (66-81) and `<name>: store.clone()` in the
   body (82-95; the last field takes `store` by move).
4. `crates/ubiqx-storage/src/lib.rs` doc list (lines 8-10) and `store.rs:9`.

`Repos::from_store` is the only constructor in use, so there is no other literal to fix.

### 6.6 "Delete all data"

`MaintenanceRepo::wipe_user_data` (`repos/maintenance.rs:29-42`): one transaction running
`WIPE` (`screenshots`, `blocks`, `corrections`, `reports`, `nudges`, `ai_usage`, `kv`,
`interventions`, `focus_sessions`) plus learned rules, then
`PRAGMA wal_checkpoint(TRUNCATE); VACUUM;` so deleted text does not linger in free pages.
Kept: categories, user rules, settings, `focus_targets` ("configuration, not activity").
`EngineHandle::delete_all_data` (`handle.rs:452-479`) also removes the screenshot and
export folders and documents what is kept. A new table is either added to `WIPE` (and the
array length in its type, `[&str; 9]`) or documented as kept; the tests
`wipe_user_data_keeps_categories_settings_and_user_rules` (`tests/repos.rs:1515`) and
`wipe_keeps_targets_but_clears_interventions_and_sessions` (`repos/focus.rs:527`) show the
pattern for asserting either choice. The port's doc comment (`ports.rs:320-326`) lists
what is removed and must be updated too.

---

## 7. Ports: how a platform port with three OS adapters and a mock is added

### 7.1 What a platform port looks like

`crates/ubiqx-core/src/ports.rs:16-155`. All platform traits are `Send + Sync` and
synchronous (`ActivitySource`, `BrowserUrlResolver`, `IdleDetector`, `ScreenCapturer`,
`PermissionChecker`, `Notifier`, `AppCatalog`, `Enforcer`, `InterventionPresenter`,
`SecretStore`); only `UpdateFeedSource` is `#[async_trait]`. The closest precedent for an
action port is `Enforcer` (118-130), whose contract is written in its doc comment:

> Every method answers `Ok(true)` when the action took effect and `Ok(false)` when the
> platform could not do it (the engine then falls back to the next action, or to a
> notification); `Err` only for unexpected failures.

The engine wraps each call in `attempt(what, result) -> bool`, which logs an `Err` at
`debug` and treats it as a refusal (`crates/ubiqx-engine/src/focus.rs:380-388`).
Data types that cross the port live in `model.rs` (plain, `Serialize + Deserialize`) or
next to the trait in `ports.rs` (`EncodedImage`, `CaptureTarget`, `PermissionStatus`).

### 7.2 Where the adapters go

Two parallel structs carry the ports, and both must gain the field:

- `ubiqx_platform::PlatformServices` (`crates/ubiqx-platform/src/lib.rs:54-67`), built by
  `macos::services()` (`macos/mod.rs:28-41`), `linux::services(data_dir)`
  (`linux/mod.rs:38-52`), `windows::services(data_dir)` (`windows/mod.rs:29-43`) and
  `PlatformServices::scripted(scenario)` (`lib.rs:106-121`). `native_in` (80-102) picks by
  `cfg(target_os)` and falls back to the scripted mock on any other OS.
- `ubiqx_engine::PlatformPorts` (`crates/ubiqx-engine/src/deps.rs:30-46`), filled field by
  field in `App::start` (`crates/ubiqx-app/src/lib.rs:361-373`). It has one port
  `PlatformServices` lacks: `presenter`, supplied by the shell through
  `AppConfig.presenter` (`lib.rs:87`, default `LogInterventionPresenter`). `notifier` is
  likewise overridden by the shell (`lib.rs:341-343`).

Per-OS files: one module per port under `crates/ubiqx-platform/src/{macos,windows,linux}/`
(for the enforcer: `enforce.rs` in each), declared with `pub mod` in that OS's `mod.rs`,
each exposing a unit struct or a `Default` struct (`MacEnforcer`, `WindowsEnforcer`,
`LinuxEnforcer`). OS modules are compiled only under `#[cfg(target_os = "...")]`
(`lib.rs:43-50`); OS-only crates go under `[target.'cfg(target_os = "...")'.dependencies]`
in `crates/ubiqx-platform/Cargo.toml`.

### 7.3 How adapters are kept testable

The pattern is consistent across the crate: everything that can be decided without the
OS is a pure, public function that is unit-tested on any host, and the system call is a
few lines around it.

- macOS: script text is built by pure functions (`close_tab_script`, `blank_tab_script`,
  `hide_others_script`, `escape`; `macos/enforce.rs:47-90`) with tests at 194-232; only
  `run_osascript` (`crates/ubiqx-platform/src/browser.rs:78`) spawns a process, with a
  timeout and a `(ok, stdout, stderr)` return.
- Portable helpers shared by Windows and Linux live at the crate root so they are tested
  on the Linux CI host: `browser_url.rs`, `app_id.rs`, `catalog.rs`, `secrets_file.rs`.
- Linux adapters answer `Ok(false)` without a display and say so in a test
  (`linux/enforce.rs:182-189`).
- External commands are spawned with absolute paths on macOS (`/usr/bin/osascript`,
  `/usr/bin/shortcuts`, `/usr/bin/plutil`, `/usr/bin/open`), `Stdio::null()` for stdin and
  stdout, stderr piped and logged; long-running ones are waited on by a named watcher
  thread (`macos/enforce.rs:160-191`, `linux/enforce.rs:117-150`). Windows uses
  `CREATE_NO_WINDOW` (`windows/enforce.rs:41`).
- Every `unsafe` block carries a `// SAFETY:` comment (`linux/enforce.rs:52`,
  `windows/enforce.rs:62`).
- Module docs open with what the adapter does and its permission model
  (`macos/mod.rs:1-13`).

### 7.4 The mock

`crates/ubiqx-platform/src/mock.rs`. For an action port the precedent is `MockEnforcer`
(356-413): a `calls: Mutex<Vec<EnforcerCall>>` log, an `enum EnforcerCall` (345-351) with
one variant per method, `AtomicBool` switches that make a method answer `false`
(`refuse_quit`, `refuse_close_tab`, `refuse_blank_tab`), plus `calls()` and `clear()`.
It is wired in `PlatformServices::scripted` (`lib.rs:118`) and unit-tested in the same
file (`mock.rs:515-536`). Engine tests construct it themselves and keep the `Arc` to
inspect calls (`crates/ubiqx-engine/tests/focus.rs:200`, `:216`).

### 7.5 Secrets

`SecretStore` (`ports.rs:145-149`): `get`/`set`/`delete` by string key. Keys in use:
`secret_keys::{ANTHROPIC_API_KEY, OPENAI_API_KEY, XAI_API_KEY}` (151-155) and
`LICENSE_SECRET_KEY = "ubiqx.license"` (`model.rs:572`). Implementations:

- macOS: `KeychainSecretStore`, service `"ai.ubiqx"` (`macos/keychain.rs`).
- Windows and Linux: `FallbackSecretStore::new(Box::new(KeyringStore::default()),
  FileSecretStore::in_dir(&dir))` (`linux/secrets.rs`, `windows/secrets.rs`): the OS
  keyring first, and when it errors a JSON file `<data_dir>/secrets.json`, mode `0600` on
  Unix (`crates/ubiqx-platform/src/secrets_file.rs:19`, 41-107). The module doc calls it
  "a fallback, not a vault".
- Scripted platform: `EnvOrMemorySecretStore` (`secrets.rs:14`), which also reads vendor
  keys from environment variables. Tests replace it with their own `MemorySecrets` so a
  developer's exported keys cannot change outcomes
  (`crates/ubiqx-engine/tests/pipeline.rs:47-66`).

The stored value is readable by any process of the same user in every variant.

### 7.6 AI ports

`AiPorts` (`deps.rs:102-109`): four `Option<Arc<dyn ...>>` (`remote`, `vision`,
`report_writer`, `advisor`) plus `requires_api_key: bool`; `Default` is all `None` with
`requires_api_key: true`. Built in `build_ai` (`crates/ubiqx-app/src/lib.rs:454-510`) for
`AiBackend::{None, Fake, Remote}`. A new AI port is a new `Option` field, a fake in
`crates/ubiqx-ai/src/fake.rs`, an implementation over `Arc<dyn LlmClient>`, and a line in
each `build_ai` arm. See `map-vision-ai.md` for the request side.

---

## 8. How the engine is exposed: `EngineHandle`, read models, IPC, events

### 8.1 `EngineHandle` (`crates/ubiqx-engine/src/handle.rs`)

`#[derive(Clone)] pub struct EngineHandle { pub(crate) state: Arc<EngineState>,
pub(crate) cancel: CancellationToken }` (36-40). Conventions:

- Methods are thin delegations to free functions in the feature module, grouped under a
  banner comment: `// -- Focus guard ---` at 387, e.g.
  `pub fn start_focus_session(&self, task: &str, minutes: u32) -> CoreResult<FocusSession>
  { crate::focus::start_session(&self.state, task, minutes) }` (433-435).
- Synchronous methods for repository and platform work; `async fn` only where an AI or
  network call is awaited (`classify_now` 232, `generate_report` 239, `advice` 250,
  `set_license_key` 74, `check_for_updates` 374).
- Arguments are borrowed (`&str`), results are owned domain types that serialise.
- Doc comments state which `CoreError` a caller can expect.
- User-facing AI entry points call `self.state.license_gate()?` first (233, 244, 252).
- `pub fn state(&self) -> &Arc<EngineState>` (43) exposes everything; the CLI and the
  desktop commands use it to reach repositories directly
  (`app.engine.state().deps.repos.categories`, `crates/ubiqx-cli/src/main.rs:228`, `:339`).
  The handle is a convenience, not a boundary.

### 8.2 Read models (`crates/ubiqx-engine/src/service.rs`)

Free functions `pub fn <name>(state: &EngineState, ...) -> CoreResult<T>` returning
`Serialize + Deserialize` structs defined in the same file (`DashboardData` at 20,
`ScreenshotData` at 73); "Every function here is synchronous (repository calls) and is
meant to be run through `spawn_blocking` by the shell" (lines 1-2). Everything is
re-exported with `pub use service::*;` (`lib.rs:39`). "Today" always comes from the
injected clock: `service::today(state)` (134). Local-day boundaries come from
`ubiqx_core::scheduler::day_range(date)` (`scheduler.rs:72`) and `month_range(now)` (65).

### 8.3 IPC (`apps/desktop/src-tauri/src/commands.rs`)

- Each command is `#[tauri::command] pub async fn name(state: State<'_, AppState>, ...)
  -> IpcResult<T>` and runs its work through
  `blocking(engine(&state), move |e| e.method(..)).await` (helper at 67-76). Example:
  `add_focus_target` at 829.
- Errors cross as `IpcError { code, message }`; `impl From<CoreError> for IpcError`
  (22-43) is an **exhaustive** `match` mapping each variant to a snake_case code.
- Commands are registered by name in `tauri::generate_handler![...]` inside
  `pub fn handler()` (934).
- Command-authored messages are bilingual through a `Text::for_language(lang)` table
  (88-130).
- Shell-side adapters: `TauriSink` (`lib.rs:159`, forwards every event to the webview as
  the `"engine"` event), `TauriNotifier` (219), `TauriPresenter` (331).

### 8.4 Events

`EngineEvent` (`model.rs:1053-1097`): `#[serde(tag = "type", rename_all = "snake_case")]`,
variants carry whole domain objects. Emitted with `state.deps.sink.emit(EngineEvent::X {
.. })` after the row is persisted ("already persisted", doc at 1089). State-change events
are de-duplicated by comparing before emitting (`set_tracker_state`, `state.rs:170-183`;
`set_ai_health`, 204-218). Consumers with an exhaustive `match`:
`PrintSink::emit` in the CLI (`crates/ubiqx-cli/src/main.rs:101-152`, no wildcard arm).
The desktop sink only inspects `UpdateAvailable`. The front end has its own union type in
`types.ts`.

### 8.5 UBI's voice

`nudges::emit(state, kind, title, message, os_notification)`
(`crates/ubiqx-engine/src/nudges.rs:16-40`) stores a `Nudge`, emits `EngineEvent::Nudge`
and optionally calls the notifier; storage or notification failures are logged, never
returned. `emit_attention` (42) is the "user must act" shortcut and bypasses the daily
cap (`NudgeRepo::count_since` excludes `ReportReady` and `Attention`,
`repos/nudges.rs:37`). The policy pass `run_once` (47) only runs while
`TrackerState::Running`, outside snooze, under `max_per_day` (default 4), and silent in
`silent_apps`. A new `NudgeKind` needs `as_str`, `parse` (`model.rs:498-525`), the
`enum_round_trips` test list (1352-1363), the front-end type, and is stored as text (no
migration).

---

## 9. Error handling conventions

- One error type across ports: `CoreError` (`crates/ubiqx-core/src/error.rs:8-52`):
  `Storage(String)`, `Platform(String)`, `Permission(String)`, `Ai(String)`,
  `AiNotConfigured`, `AiRejected(String)`, `AiRefused`, `RateLimited { retry_after_secs }`,
  `LicenseRequired`, `Invalid(String)`, `NotFound(String)`, `Other(String)`.
  `CoreResult<T>` (54). `is_transient()` is true for `RateLimited` and `Ai` (58-60).
- Adapters keep their own error type and convert at the boundary: `StorageError`
  (`crates/ubiqx-storage/src/error.rs:8-33`) with `From<StorageError> for CoreError`
  (38-47) preserving `Invalid` and `NotFound`, everything else becoming `Storage`.
- Platform adapters build `CoreError::Platform(format!("<what>: {e}"))` by hand, lower
  case, no trailing period.
- Join errors from `spawn_blocking` become `CoreError::Other(e.to_string())`.
- "Could not do it" is `Ok(false)` or `Ok(None)`, not an error (`Enforcer`,
  `BrowserUrlResolver::resolve`, `ActivitySource::foreground`).
- Background code logs and continues: `warn!` for a failed pass or a failed write that
  loses data, `debug!` for expected refusals (enforcement, notification, unreadable URL),
  `info!` for state changes and actions taken. Structured fields, `error = %e`.
- Best-effort side effects are written `let _ = ...;` or `if let Err(e) = ... { warn }`.
- `anyhow` appears only in the CLI binary.
- No `.unwrap()` outside test modules in the core, storage, platform, engine and app
  crates (counted with a script); `expect("valid")` is used for compile-time constants
  (`NaiveTime::from_hms_opt(18, 0, 0).expect("valid")`) and `expect("spawn sampler thread")`.
- Adding a `CoreError` variant forces an edit of the exhaustive match at
  `apps/desktop/src-tauri/src/commands.rs:24-37`, which the Linux CI job does not compile
  (the desktop crate is excluded there).

---

## 10. Test conventions

### 10.1 Layers

| Layer | Where | Style |
|---|---|---|
| Pure logic | `#[cfg(test)] mod tests` at the bottom of each core module | plain `#[test]`, table-driven loops, both languages asserted literally |
| Repositories | bottom of the repo file, and `crates/ubiqx-storage/tests/repos.rs` | `Db::open_in_memory()`, fixed instants, `&dyn Trait` coercion |
| Platform helpers | bottom of each helper/adapter file | pure functions only; display-dependent checks guarded |
| Engine components | in-file rig with in-file doubles (`crates/ubiqx-engine/src/update.rs:209`) | no engine start |
| Engine end to end | `crates/ubiqx-engine/tests/{pipeline,focus,screenshot_read}.rs` | `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` |

### 10.2 The harness shape (`crates/ubiqx-engine/tests/focus.rs:147-259`)

- `SqliteStore::new(Db::open_in_memory().unwrap())` in an `Arc`, `tempfile::tempdir()` for
  `data_dir`, kept alive in the harness (`_tmp`).
- `FixedClock::new(fixed_now())` with
  `fixed_now() = Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0)`: "Mid-afternoon UTC, outside
  the default quiet hours in any plausible test timezone" (187-190). The clock is cloned
  into the deps and kept in the harness; tests move time with
  `h.clock.advance(ChronoDuration::seconds(..))` (`crates/ubiqx-core/src/clock.rs:26-47`).
- Hand-written doubles in the test file: `Foreground` (28-81, a settable window and URL
  implementing `ActivitySource`, `BrowserUrlResolver`, `IdleDetector`,
  `PermissionChecker`), `MemorySecrets` (84), `RecordingNotifier` (104),
  `RecordingPresenter` (114), `CollectSink` (135). From the platform crate: `MockEnforcer`,
  `MockAppCatalog`, `StaticUpdateFeed`, `SyntheticCapturer`.
- `LoopConfig` with **every** period at 3600 s and `without_sampler: true` (231-242), so
  no background pass runs; the test calls `ubiqx_engine::focus::guard_once(state)` itself
  (164-166) and asserts on the return value, the repository, the mock's call log, the
  sink's events and the notifier.
- After start: `onboarding_done = true`, `quiet_hours.enabled = false` through
  `handle.update_settings` (244-247).
- AI: `AiPorts { requires_api_key: false, ..AiPorts::default() }` for "no AI" (220-223);
  `ubiqx_ai::fake::{FakeClassifier, FakeVisionClassifier, FakeReportWriter, FakeAdvisor}`
  for deterministic AI (`pipeline.rs:96-110`); a local `FailingClassifier(fn() ->
  CoreError)` to inject errors (79-94).
- A fixture without any loop: `EngineState::new(deps, Settings::default(), None)` and
  calls on the free functions (`screenshot_read.rs:47-88`, `focus.rs:775-800`).

### 10.3 The start-up tick race

`pipeline.rs` runs the classify loop for real (`classify_every: 200 ms`, 271-282) and
waits with `settle()` = `sleep(800 ms)` (355-357). `classify_loop` fires its first tick
immediately, and three tests carry comments about working around that:

- `pipeline.rs:884-888`: "the classify worker's first tick fires at start-up, and on a
  slow Windows runner it landed between the two inserts".
- `pipeline.rs:1056-1059`: a block left pending "would race this test ... which turned
  the assertion below into a flaky 0 on slower machines".
- `pipeline.rs:1272-1273`: a block "Parked past its retry window so the classify worker,
  whose first tick fires at start-up, cannot answer it underneath the assertions".

The fix applied to `reports_loop` and `focus_loop` was to skip the first tick. For a new
worker: skip the first tick (or use an initial delay), add its period to `LoopConfig`,
set it to 3600 s in both harnesses and drive the pass by hand. Do not add a test that
depends on a real timer.

### 10.4 Other habits

- Assertions compare whole values (`assert_eq!(h.handle.interventions(10).unwrap(),
  vec![i.clone()])`), which is why domain types derive `PartialEq`.
- User-facing strings are asserted literally in both languages
  (`classify.rs:566-584`).
- Tests that depend on the local day derive it from the fixed clock with
  `.with_timezone(&Local).date_naive()`.
- `expect("...")` messages in storage tests, `unwrap()` in engine tests.

---

## 11. Lint, format, CI

- `rustfmt.toml`: `edition = "2021"`, `max_width = 100`. CI: `cargo fmt --all -- --check`.
- No `clippy.toml`, no `[lints]` table, no crate-level `#![deny]`. CI runs default clippy
  with warnings as errors:
  `cargo clippy --workspace --exclude ubiqx-desktop --all-targets -- -D warnings`
  (`.github/workflows/ci.yml`, job `rust-linux`). `--all-targets` includes tests.
- Existing `#[allow]`s are few and local: `clippy::too_many_arguments` on
  `UpdateChecker::new`/`with_target` (`update.rs:35`, `:60`) and `unused_variables` on
  `PlatformServices::native_in` (`crates/ubiqx-platform/src/lib.rs:79`).
- Tests on Linux: `cargo test --workspace --exclude ubiqx-desktop`.
- macOS and Windows adapter code is only **type-checked** from Linux:
  `cargo check -p ubiqx-platform --target aarch64-apple-darwin` and
  `--target x86_64-pc-windows-msvc`. Clippy never runs on those two OS modules. Their
  unit tests run on the `macos-app` job (`cargo test --workspace`) and the `windows-app`
  job (`cargo test --workspace --exclude ubiqx-desktop`); the Windows job also runs
  `cargo clippy -p ubiqx-desktop -- -D warnings`.
- A new OS-only dependency must build for those targets with `cargo check` from Linux,
  which rules out crates that compile C for the target (the CI comment gives `ring` and
  `libsqlite3-sys` as examples for the desktop crate).
- Front end: `pnpm typecheck`, `pnpm test -- --run`, `pnpm build`.
- `services/ubi-api` is linted and tested by manifest path in the same job.
- The Linux job installs `libdbus-1-dev pkg-config` for the keyring backend.
- `README.md:158` documents `cargo clippy --workspace --all-targets -- -D warnings`,
  which includes the desktop crate and so needs webkit/gtk on Linux; CI excludes it.

---

## 12. Checklist: files to touch for settings + 2 tables + worker + platform port

Names below (`nofap`, `NofapRepo`, `DnsBlocker`) are placeholders for the design stage.

### A. Core (`crates/ubiqx-core`)

1. `src/model.rs`
   - `NofapSettings` (`#[serde(default)]`, `Default`) and `pub nofap: NofapSettings` in
     `Settings` plus `impl Default for Settings` (976-1019).
   - Domain types for the two tables and any status read model, with
     `Debug, Clone, PartialEq, Serialize, Deserialize`; enums with
     `#[serde(rename_all = "snake_case")]`, `as_str()` and `parse()`.
   - `EngineEvent` variants (1053-1097) if the UI must hear about it.
   - `NudgeKind` variants if UBI's nudge channel is used (plus `as_str`/`parse` and the
     `enum_round_trips` list at 1352-1363).
   - Range clamps in `Settings::sanitize()` (1040) if needed.
2. `src/ports.rs`
   - `pub trait NofapRepo: Send + Sync` in the persistence section.
   - `pub trait DnsBlocker: Send + Sync` (or the chosen name) in the platform section,
     with the `Ok(true)`/`Ok(false)`/`Err` contract spelled out like `Enforcer`'s.
   - An AI port if detection gets its own (`#[async_trait]`).
   - `AiUsageKind` variant (349-375, with `as_str`/`parse`) if calls are costed
     separately; the column is text, no migration.
   - Update the `MaintenanceRepo::wipe_user_data` doc (320-326).
3. `src/nofap.rs` (new) and `pub mod nofap;` in `src/lib.rs` (21-36, alphabetical), plus
   the module list in the crate doc (5-17): constants, domain matching, list parsing,
   decision functions, PT and EN copy, `validate_*`. Unit tests in the file.
4. `src/error.rs` only if a new variant is unavoidable (then also `commands.rs:24-37`).

### B. Storage (`crates/ubiqx-storage`)

5. `src/migrations.rs`: `const V3`, append `M::up(V3)` at 236, fix the assertion at 273.
6. `src/repos/nofap.rs` (new) following `repos/focus.rs`; `mod nofap;` in
   `src/repos/mod.rs`.
7. `src/repos/maintenance.rs`: add the tables to `WIPE` (and bump `[&str; 9]`) or leave
   them out on purpose; update the comment at 8-10.
8. `src/lib.rs` (doc list 8-10), `src/store.rs:9` (doc).
9. `tests/repos.rs`: version assertions at 234 and 242; a wipe assertion.

### C. Platform (`crates/ubiqx-platform`)

10. `src/<portable>.rs` (new, crate root) for everything testable on Linux: file content
    generation and parsing, command lines, state detection. `pub mod` in `src/lib.rs`
    (32-41) and a line in the crate doc.
11. `src/macos/<name>.rs`, `src/windows/<name>.rs`, `src/linux/<name>.rs` (new), each
    declared in its `mod.rs` and constructed in its `services()`
    (`macos/mod.rs:29`, `windows/mod.rs:31`, `linux/mod.rs:40`).
12. `src/lib.rs`: field in `PlatformServices` (54-67) and in `scripted()` (108-119).
13. `src/mock.rs`: a recording mock (`calls`, `enum ...Call`, failure switches) with a
    unit test.
14. `Cargo.toml`: OS-only dependencies under the right `cfg`, and for Windows any extra
    `windows` crate feature in the list at the `[target.'cfg(target_os = "windows")']`
    table.

### D. Engine (`crates/ubiqx-engine`)

15. `src/deps.rs`: field in `PlatformPorts` (30-46); field, bound and assignment in
    `Repos`/`from_store` (49-97); field in `AiPorts` (102-121) if there is an AI port.
16. `src/nofap.rs` (new) and `pub mod nofap;` in `src/lib.rs` (17-31): the runtime struct,
    `pub fn <pass>_once(state: &Arc<EngineState>) -> CoreResult<..>`, the functions behind
    the handle, read models.
17. `src/state.rs`: runtime field on `EngineState` (16-43), restore in `new` (46-88),
    reaction to settings changes in `apply_settings` (120-168); `key_health` (251-272) if
    a new AI port must arm health on its own.
18. `src/engine.rs`: `LoopConfig` field(s) and default (21-54), `tokio::spawn(nofap_loop(..))`
    in `start_with` (after 107), the loop function copied from `focus_loop` (218-234) or
    `license_loop` (241-258).
19. `src/handle.rs`: a `// -- Nofap ---` section of thin methods; decide what
    `delete_all_data` (452-479) and `shutdown` (484-500) do for the module, and update the
    `delete_all_data` doc.
20. `src/lib.rs`: crate doc (5-12) and re-exports.
21. `tests/nofap.rs` (new) modelled on `tests/focus.rs`.

### E. Literals that stop compiling when a field is added

| Struct | Literal sites |
|---|---|
| `LoopConfig` | `crates/ubiqx-engine/tests/focus.rs:231`, `crates/ubiqx-engine/tests/pipeline.rs:271` |
| `PlatformPorts` | `crates/ubiqx-app/src/lib.rs:361`, `tests/focus.rs:206` and `:776`, `tests/pipeline.rs:249`, `tests/screenshot_read.rs:53` |
| `PlatformServices` | `macos/mod.rs:29`, `windows/mod.rs:31`, `linux/mod.rs:40`, `crates/ubiqx-platform/src/lib.rs:108` |
| `AiPorts` (full literals) | `crates/ubiqx-app/src/lib.rs:463` and `:493`, `tests/pipeline.rs:97`, `tests/screenshot_read.rs:67` (`tests/focus.rs:220` and `pipeline.rs:113` use struct update syntax) |
| `EngineDeps` | `crates/ubiqx-app/src/lib.rs:360`, `tests/focus.rs:205` and `:775`, `tests/pipeline.rs:248`, `tests/screenshot_read.rs:52` |
| `AppConfig` | `apps/desktop/src-tauri/src/lib.rs` near 571 (full literal); the CLI uses `..AppConfig::default()` |
| `EngineEvent` (new variant) | `crates/ubiqx-cli/src/main.rs:101-152` |
| `CoreError` (new variant) | `apps/desktop/src-tauri/src/commands.rs:24-37` |

The macOS and Windows `services()` literals are not compiled by `cargo test` on Linux;
only the two cross-target `cargo check` steps catch a missed field there.

### F. Composition root and shells

22. `crates/ubiqx-app/src/lib.rs`: pass the new platform port into `PlatformPorts`
    (361-373); wire any AI port in the three `build_ai` arms (454-510); add an
    `AppConfig` field (75-117) if the shell must inject something.
23. `crates/ubiqx-cli/src/main.rs`: `PrintSink` arms for new events; subcommands if wanted.
24. `apps/desktop/src-tauri/src/commands.rs`: commands and the `generate_handler!` list
    (934); `apps/desktop/src-tauri/src/lib.rs` for any shell-side adapter.
25. `apps/desktop/src/lib/types.ts` and the IPC wrapper for the mirrored types.
26. Docs: `docs/ARCHITECTURE.md`, `README.md`, `docs/WINDOWS-LINUX.md`.

---

## 13. Surprising, fragile or missing

1. **Settings writes are unguarded and lossy.** See 1.1 and section 4. A stale or partial
   `Settings` object from the webview resets omitted keys to defaults, and concurrent
   read-modify-write cycles can drop a change. Nothing distinguishes a protected field
   from any other.
2. **`kv` is wiped with user data**, including `update.notified_epoch` and
   `update.dismissed_epoch` (`update.rs:5-7`), so an already-announced update is
   announced again after "delete all data". Evidence that `kv` is treated as a cache.
3. **The scripted platform's cursor is shared.** The focus guard avoids touching it by
   returning before `foreground()` when there is nothing to enforce
   (`crates/ubiqx-engine/src/focus.rs:185-188`, "no need to touch the window server"). A
   new always-on pass has no such early return and will shift `ubiqx simulate` results
   and the `UBIQX_SCRIPTED` desktop demo (`apps/desktop/src-tauri/src/lib.rs` near 543).
4. **The CLI starts everything.** `status`, `classify`, `report`, `monthly`, `categories`
   and `export` start the engine on an empty scripted scenario and call `pause()`;
   `track` starts it on the native platform (`main.rs:249-264`). `pause()` stops the
   tracker only; the focus loop keeps running.
5. **The focus guard ignores consent and pause state.** `guard_once` checks only
   `settings.focus.guard_enabled` (`focus.rs:173`); it does not look at `paused`,
   `tracking_enabled`, `onboarding_done` or private mode. The tracker checks all of them
   (`tracker.rs:53`). A new module must decide its own gates explicitly; the only
   existing remote-AI consent gate is `remote_allowed()`.
6. **Two loops fire at start-up** (`classify_loop`, `retention_loop`); the others wait.
   The pattern is inconsistent on purpose after two race fixes.
7. **Only the tracker is awaited at shutdown** (section 3.3).
8. **Strict enum decoding.** `parse_enum` fails the whole query on an unknown tag
   (`convert.rs:57-63`). A row written by a newer build with a new `NudgeKind` or
   `InterventionAction` makes `list_recent`/`list_interventions` fail in an older build
   (the CLI installed next to a newer app, or the other product).
9. **Schema downgrade is fatal** (1.7). `StorageError::Migration` documents "the database
   was written by a newer build" (`crates/ubiqx-storage/src/error.rs:12-14`), and
   `App::start` propagates it, so the app does not start.
10. **A single SQLite connection** (6.1). Bulk-loading a large domain list into a table
    would hold the mutex and delay the tracker; `wipe_user_data` runs `VACUUM` under the
    same lock.
11. **`PlatformServices` and `PlatformPorts` duplicate each other** and are copied field
    by field; forgetting one side compiles on Linux if the forgotten side is an OS module.
12. **`AiPorts` health coupling** (1.9), and the `max_vision_per_hour` cap counts rows in
    `screenshots` with `sent_to_ai = 1` (`classify.rs:464-465`), so an image sent without
    a screenshot row is invisible to it. `budget_allows` has side effects: it sets AI
    health and emits the 80% attention nudge (`classify.rs:105-147`).
13. **`CoreError::AiRefused` exists because models refuse some content**
    (`error.rs:29-32`); the classifier sends such blocks straight to review and never
    retries (`classify.rs:275-285`). A detector whose input is explicit imagery has to
    treat a refusal as a possible outcome rather than a failure.
14. **Clock leaks.** `SettingsRepo::save` and `KvRepo::set` stamp `Utc::now()`
    (`repos/settings.rs:31`, `repos/kv.rs:23`); `set_private_mode` uses
    `chrono::Local::now()` for "tomorrow" (`handle.rs:116-119`). Day boundaries use the
    process time zone through `chrono::Local`, which tests cannot inject; hence the
    mid-afternoon fixed instant.
15. **`TrackerState::Blocked` is never produced.** The only reference outside the enum is
    the arm that preserves it (`state.rs:196`).
16. **The secrets fallback is a plaintext file** on Windows and Linux (7.5).
17. **Linux sees a page's domain only when the window title shows it**
    (`crates/ubiqx-platform/src/linux/browser.rs:1-8`), Firefox on macOS exposes no URL
    (`browser.rs:3-4`), and Wayland-native windows are not seen at all
    (`linux/mod.rs:10-16`). Details in `map-focus-guard.md`.
18. **macOS updates cost the permissions.** The CI comment on the macOS build says the
    ad-hoc signature "is also why an auto-update costs the macOS permissions"
    (`ci.yml`, `APPLE_SIGNING_IDENTITY: "-"`). Anything that depends on a per-binary
    grant inherits that.
19. **Continuous release on every push** (1.10). `publish-release.mjs` records the branch
    but nothing read here filters on it, and `ubiqx_core::update` compares build epochs,
    not branches.
20. **`EngineHandle::state()` is public** and used by the shells to bypass the handle
    (8.1), so "only through the handle" is not an invariant to rely on.

---

## 14. Docs versus code

| Claim | Where | What the code does |
|---|---|---|
| "spawns the sampler thread and the tokio tasks (tracker, classifier, report scheduler, nudges, retention, update checks)" | `crates/ubiqx-engine/src/lib.rs:5-7` | Also spawns `focus_loop` and `license_loop` (`engine.rs:101-107`) |
| `run_due`: "Called every 30 s and at start-up" | `crates/ubiqx-engine/src/reports.rs:195` | Not at start-up since the first tick is skipped (`engine.rs:160-163`) |
| `default_data_dir()`: "`~/Library/Application Support/ai.ubiqx.app` on macOS" | `crates/ubiqx-app/src/lib.rs:141` | It calls `ProjectDirs::from("ai", "ubiqx", "ubiqX")` (143), which by the `directories` crate's scheme is `ai.ubiqx.ubiqX` on macOS. The desktop shell passes Tauri's `app_data_dir()` (identifier `ai.ubiqx.app`, `tauri.conf.json:5`). The CLI default and the desktop app therefore appear to use different directories; not run here |
| Storage example path `ubiqx.sqlite` | `crates/ubiqx-storage/src/lib.rs:29` | The app opens `ubiqx.db` (`crates/ubiqx-app/src/lib.rs:328`) |
| Platform crate description: "macOS foreground window ... plus scripted mocks for other OSes" | `crates/ubiqx-platform/Cargo.toml` | Windows and Linux adapters exist |
| `cargo clippy --workspace --all-targets -- -D warnings` | `README.md:158` | CI excludes `ubiqx-desktop` |
| `SecretStore`: "Secret storage (macOS Keychain, etc.). Used for API keys." | `ports.rs:144` | Also holds the license key, and falls back to a file |
| `MaintenanceRepo` doc: "the key/value cache" | `ports.rs:321-323` | Accurate, and it means everything in `kv` |

---

## 15. What this means for the standalone product

- The engine is one unit: `Engine::start_with` always spawns the tracker, classifier,
  reports, nudges, retention, update, focus and license loops. There is no feature flag
  or partial start. A product without productivity tracking needs either a second, small
  start function over the same crates or a `LoopConfig`-style switch per loop;
  `without_sampler` (`engine.rs:35-36`, 75-77) is the only existing precedent for
  switching one off.
- The reusable shape already in the tree is the `UpdateChecker` one: a component that
  takes `Arc<dyn Port>`s and a `Clock`, owns its state, and is driven by a loop that lives
  elsewhere. A module built that way can be hosted by `EngineState` in ubiqX and by a
  minimal host in the other product.
- `Repos::from_store` requires one object implementing **every** repository trait
  (`deps.rs:66-81`). A host that needs only some repositories builds its own smaller
  struct; `SqliteStore` still implements them all.
- Identity is hard-coded in several places: `APP_ID = "ai.ubiqx.app"`
  (`crates/ubiqx-app/src/lib.rs:60`), `ProjectDirs::from("ai", "ubiqx", "ubiqX")` (143),
  keyring service `"ai.ubiqx"` (`secrets_file.rs:22`, `macos/keychain.rs:8`),
  `secrets_file::default_dir()` (26-37), `SELF_BUNDLE_ID` (`crates/ubiqx-core/src/focus.rs:32`),
  database file name `ubiqx.db`, log variable `UBIQX_LOG`. A second product must choose
  whether it shares these (same database, same schema version, same keys, same
  system-level state) or has its own; sharing the database ties both products to one
  migration list (13.9), and sharing nothing means two installs could both try to manage
  the same system setting.
- The licence types know two plans, `AnnualOwnKey` and `MonthlyManaged`
  (`crates/ubiqx-core/src/license.rs:76-84`); enforcement is `Soft`. A separately sold
  product has no plan to map to yet.
- `Settings` carries every productivity field. A separate product either reuses it
  (simple, but its UI must send the whole struct back unchanged) or has its own settings
  row; `SettingsRepo` is typed to `Settings` only.
