# Hand-off: U9b CI wiring for the second product, and the documents

Where this file and the code differ, the code wins.

## Engineer

U9b is done: the three app jobs build the standalone app next to ubiqX and upload its
installers under its machine id, `publish-shield` exists behind `NOFAP_PUBLISH` and `main`, the
Windows job lints and tests the standalone shell, the frontend job builds `dist-nofap`, and the
documents describe the module and the second product. `PRIVACY.md` and `REMOVAL.md` were read
again against the landed code, claim by claim; the traces are below.

### What exists

**`.github/workflows/ci.yml`**

| Job | Change |
|---|---|
| `frontend` | `pnpm build:nofap` after `pnpm build` |
| `macos-app` | `ci-app-version.sh --product shield` after ubiqX's; ubiqX's rename step picks `ubiqX.app`, `ubiqX_*.dmg`, `ubiqX.app.tar.gz` by the `productName` of `apps/desktop/src-tauri/tauri.conf.json` (no bare `*.dmg`); then the standalone app is built (`node ../../desktop/node_modules/@tauri-apps/cli/tauri.js build --bundles app,dmg` from `apps/nofap/src-tauri`, ad hoc seal), renamed `ubiqx-shield-macos-<arch>.{dmg,app.zip,app.tar.gz(.sig)}` by its own `productName`, uploaded as `ubiqx-shield-macos-app`. Each rename step lists the `.app`'s `Contents/MacOS` |
| `windows-app` | the second version call; `cargo clippy -p ubiqx-nofap-desktop --all-targets -- -D warnings` and `cargo test -p ubiqx-nofap-desktop`; ubiqX's rename step picks `ubiqX_*pt-BR*.msi` and `ubiqX_*-setup.exe` (the `.nsis.zip` is derived from the `.exe`); the standalone app is built **NSIS only** (`--bundles nsis`, decision 39), renamed `ubiqx-shield-windows-x86_64-setup.exe` / `-setup.nsis.zip(.sig)`, uploaded as `ubiqx-shield-windows-app`. Each rename step prints `!define MAINBINARYNAME` from the NSIS script it just built |
| `linux-app` | the second version call; ubiqX's rename step picks `ubiqX_*.AppImage` and `ubiqX_*.deb`; the standalone app is built (`--bundles appimage,deb`), renamed `ubiqx-shield-linux-x86_64.{AppImage,AppImage.tar.gz(.sig),deb}`, uploaded as `ubiqx-shield-linux-app`. Each rename step prints the `.deb`'s `Package`, `Version` and `/usr/bin` entry |
| `publish-continuous` | unchanged apart from a comment: `pattern: ubiqX-*-app` and `find -name 'ubiqX-*'` are case-sensitive and never take the standalone app's files |
| `publish-shield` (new) | `if: github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.NOFAP_PUBLISH == 'true'`; downloads `ubiqx-shield-*-app`, stamps `--product shield`, publishes `find dist -type f -name 'ubiqx-shield-*' ! -name '*.sig'` with `node scripts/publish-release.mjs --product shield --strict` |
| `env` | a comment: neither `UBIQX_UPDATE_FEED_URL` nor `UBIQX_SHIELD_UPDATE_FEED_URL` is set at workflow or job level, so each shell compiles in the default feed of its own `build.rs` (decision 35); setting one where both shells build would compile a foreign feed into the other app |

ubiqX's artefact names (`ubiqX-<os>-<arch>.<ext>`), its artefact names on Actions
(`ubiqX-<os>-app`), its upload globs and `publish-continuous` are as they were.

**Release scripts**: unchanged. U9a's `--product shield` interface was enough; nothing in
`publish-release.mjs`, `app-version.mjs`, `ci-app-version.sh` or the test needed a change.

**Documents**

| File | What changed |
|---|---|
| `README.md` | a section "NoFap: o módulo contra a pornografia, e o app separado" (what it is, the five layers, the gate state, the standalone app in CI and its publishing switch, links); the installer row says the standalone app has no `.msi`; the stack lists `nofap` and the second shell; "Cada push na `main`" (the release is main-only since design 9.1); the feed variable of each product; the development commands exclude both shells and add the blocklist check, the naming test, `dev:nofap`, `build:nofap` and the standalone build |
| `docs/ARCHITECTURE.md` | the tree and the dependency rule with `ubiqx-nofap` and `apps/nofap`; a section 9 (layers, where the logic lives, the two products side by side, the three name rings, CI) |
| `docs/WINDOWS-LINUX.md` | section 6 for a tester of the module: a warning that the system tiers write for real, the standalone installers, a table of what differs on Windows and Linux (guard, elevation, caps, resolver, policies, AppImage), the gate and `UBIQX_SHIELD_UNVERIFIED_TIERS`, the "managed" banner, the uninstallers and the MSI, the Windows release build printing nothing for `--shield-helper` |
| `docs/MACOS-TESTING.md` | section 7, the same for macOS (Automation and Screen Recording, the `osascript` prompt, DNS back to automatic, Firefox only, Private Relay, `status` without a password, what is unverified); the troubleshooting row about `UBIQX_LOG` (the app's level is fixed at `info`; the variable is read by the CLI only) |
| `docs/LICENSING.md` | section 7: blocking never depends on a licence; vision through the managed provider only in ubiqX; the standalone app reads no licence key; the blocklist sources with their licences, what is not shipped and why, the two notice files, and the open point that ubiqX shows no notices |
| `docs/nofap/PRIVACY.md` | corrected against the code (list below) |
| `docs/nofap/REMOVAL.md` | corrected against the code (list below) |
| `docs/nofap/REAL-MACHINE-CHECKLIST.md` | items 15, 23 and 25 and the `secure_delete` row of section 6 follow the code |

### Corrections to `PRIVACY.md`

- The ubiqX tracker section described a scrub of "the open block". The code redacts every
  sample of a blocked page or of a window with a hit in the last ten minutes, scrubs only the
  open block of the window the hit was about (`covers_open_block`), also drops the AI payload,
  and vetoes screenshots of blocked or mixed pages and of windows with a recent signal or hit.
- "The database does not overwrite a changed row" is no longer true: every connection runs
  with `secure_delete` (decision 20). What remains is the copy in `ubiqx.db-wal` until a
  checkpoint (decision 36 is not in the code yet; see open points).
- "Apagar todos os dados não sobrescreve" was false: it wipes with `secure_delete`, then
  `wal_checkpoint(TRUNCATE)` and `VACUUM`, and deletes the `exports` folder.
- The log file: its name, its path on each system, and the lines that can be in it (the `info`
  and `warn` lines of the module, the HTTP libraries held at `warn`).
- The secret store falls back to `secrets.json` in the data directory on Windows and Linux.
- An exception request ("Pedir exceção", or an allow request from the page) stores the host in
  plain text in the configuration until applied, cancelled or lapsed, and it is exported (U5
  review).
- The shared directory holds `primary.lock`, which is deleted when the module is switched off
  or the app quits; the directory stays.
- The export: its folder, its file name, what it carries (`ConfigView` without `internal`,
  the typed sites, the learned count, the counters).
- The managed proxy records the subscriber id (`sub`) with every request; the standalone app
  does not offer the managed provider.
- More counters than the support section listed (`NotIt`, `NetworkRestore`, `Drift`,
  `HelpShown`).
- The guard can also be limited by a denied Automation on macOS.

### Corrections to `REMOVAL.md`

- The uninstaller table: the NSIS and `.deb` hooks exist now in both products' bundles, each
  removing only its own product's wants (`--owner ubiqx` / `--owner shield`); the NSIS hook
  reports nothing after its prompt; dpkg's `prerm` runs as root without a prompt and never fails
  the removal; the in-app update and a package upgrade remove nothing.
- The MSI (ubiqX only, decision 39) removes nothing, and the document gives the command to run
  before uninstalling it.
- The Windows release build prints nothing for `--shield-helper` (`windows_subsystem =
  "windows"` in both `main.rs`): part 2 says the result is only the exit code, and part 5 sends
  Windows users to the compiled helper for `status`.
- The installed executable names follow the CI listings (see "CI run").

### Traces: `PRIVACY.md`

Each claim, and the code that makes it true (paths from the repository root).

| Claim | Code |
|---|---|
| The guard reads the foreground tab every 2 s | `crates/ubiqx-engine/src/engine.rs:56` (`nofap_every`), `crates/ubiqx-app/src/nofap_app.rs:48` (`GUARD_EVERY`) |
| Title only on X11, nothing on Wayland, Firefox on macOS, a denied Automation | `crates/ubiqx-core/src/nofap.rs:642` (`GuardLimit`) |
| ~240 thousand hashed entries, the 20,000 most visited in clear | `crates/ubiqx-nofap/data/core.meta.json` (`core.count` 239,525), `hosts-tier.txt`, `crates/ubiqx-nofap/src/list.rs` |
| What a block stores: a counter per hour and kind (list, own list, keyword) | `crates/ubiqx-core/src/nofap.rs:1517` (`DayKind`); no host in a log line, event or card (U5's `s24`) |
| Card context in memory for up to 10 minutes | `crates/ubiqx-engine/src/nofap/cards.rs:22` (`CARD_TTL_SECS = 600`) |
| An exception request names its host and is exported | `crates/ubiqx-core/src/nofap_ipc.rs:225` (`ConfigView.pending`), `crates/ubiqx-engine/src/nofap/service.rs:842-868` (export) |
| ubiqX: samples of a blocked page or of a window with a recent hit are redacted | `crates/ubiqx-engine/src/tracker.rs:79-86`, `crates/ubiqx-engine/src/nofap/service.rs:888` (`redacts`) |
| ubiqX: the open block of that window is scrubbed, its screenshots deleted | `crates/ubiqx-engine/src/tracker.rs:155-206` (`scrub_open_block`), `service.rs:917` (`covers_open_block`) |
| ubiqX: no screenshot of a blocked or mixed page, or of a window with a recent signal or hit | `crates/ubiqx-engine/src/screenshots.rs:41-47` (`capture_veto`) |
| Freed bytes are zeroed; the WAL keeps them until a checkpoint | `crates/ubiqx-storage/src/db.rs:19-29` (`secure_delete = ON`) |
| The hosts block's markers, the canary name, the caps 20,000 / 8,000 | `crates/ubiqx-nofap/src/hosts.rs:44-56` |
| The canary is looked up every 30 minutes | `crates/ubiqx-nofap/src/plan.rs:55` (`CANARY_EVERY_SECS = 1_800`), `crates/ubiqx-engine/src/nofap/system.rs:495` |
| Cloudflare for Families addresses; the probe name `one.one.one.one` | `crates/ubiqx-nofap/src/resolvers.rs:28-39` |
| The previous DNS is noted for display | `crates/ubiqx-core/src/nofap.rs:383` (`previous_dns`) |
| "Restaurar minha rede" is immediate | `crates/ubiqx-engine/src/nofap/system.rs:921`; commitment 6.3 |
| The policy table; macOS only Firefox | `crates/ubiqx-nofap/src/policies.rs`, `crates/ubiqx-platform/src/sysblock/policy_reg.rs:19-35`, `macos.rs:29-34` |
| The check stops without a key, in local mode, with an outdated consent | `crates/ubiqx-core/src/nofap.rs:706` (`VisionOff`) |
| 640 pixels; the browser's top band is cropped; an unchanged frame is not sent again | `crates/ubiqx-nofap/src/sampling.rs:39-42`, `309-320`, `739` (`frame_unchanged`) |
| What the request says; a closed answer | `crates/ubiqx-ai/src/prompts.rs` (`EXPLICIT_SYSTEM_PROMPT`), `crates/ubiqx-ai/src/explicit.rs` (schema) |
| The managed proxy stores no body; it records `sub`, month, model, tokens, cost, time | `services/ubi-api/src/db.rs:16-25`, `services/ubi-api/src/routes.rs:43`, `482`, `513`; `services/ubi-api/tests/images.rs` |
| The standalone app refuses the managed provider | `crates/ubiqx-app/src/nofap_app.rs` (`NofapApp::handle`, U7 review fix 4) |
| No log line per check; the cost line at the start of the hour | `crates/ubiqx-ai/src/client.rs:349` (`logs_calls`), `357` (`usage_time`) |
| The HTTP libraries at `warn` | `apps/desktop/src-tauri/src/lib.rs:562-573`, `apps/nofap/src-tauri/src/app.rs:188-199` |
| A failed call: a warning with the vendor and the error, the kind not named | `crates/ubiqx-ai/src/retry.rs:133`, `159` (`what` is `"messages"`) |
| The check pauses and comes back: one line each | `crates/ubiqx-engine/src/nofap/vision.rs:372`, `400` |
| The log's other lines | `crates/ubiqx-engine/src/nofap/service.rs:188`, `367`; `system.rs:834`, `901`, `921`, `1021`; `mod.rs:338-361`, `500-510` (all constant text or an error class) |
| Default limits 20 / 120 / US$ 2 | `crates/ubiqx-core/src/nofap.rs:296-311` |
| The fallback notification: title "UBI", a neutral sentence | `crates/ubiqx-nofap/src/messages.rs:11`, `88-94`; `crates/ubiqx-engine/src/nofap/cards.rs:228-230` |
| The databases and their directories | `crates/ubiqx-core/src/product.rs:91-109`; Tauri's `app_data_dir` in both shells |
| The secrets file fallback | `crates/ubiqx-platform/src/secrets_file.rs:1-19` |
| The shared directory and the lock file, deleted on off and on quit | `crates/ubiqx-nofap/src/primary.rs:25-28`, `crates/ubiqx-app/src/lib.rs:182-185`, `crates/ubiqx-engine/src/nofap/system.rs:1064-1085`, `service.rs:368`, `878` |
| The export's folder, name and content; ubiqX's export has nothing of the module | `crates/ubiqx-engine/src/nofap/service.rs:842-868`, `crates/ubiqx-engine/src/state.rs:95`, `crates/ubiqx-app/src/nofap_app.rs:419`, `crates/ubiqx-engine/src/handle.rs:341-363` |
| 90 days of counters | `crates/ubiqx-nofap/src/progress.rs:11` |
| "Apagar meu histórico" overwrites and folds the WAL | `crates/ubiqx-storage/src/repos/nofap.rs:74`, `312` |
| "Apagar todos os dados": counters go, choices and lists stay, overwritten, `exports` deleted | `crates/ubiqx-storage/src/repos/maintenance.rs:15-46`, `crates/ubiqx-engine/src/handle.rs:463-492` |
| "Apagar tudo do NoFap" only while off | `crates/ubiqx-engine/src/nofap/service.rs:820-821` |
| Snoozes last up to 24 hours | `crates/ubiqx-nofap/src/sampling.rs:65` (`SNOOZE_SECS`) |
| The prompts: `osascript`, UAC, `pkexec`; the AppImage cannot | `crates/ubiqx-platform/src/sysblock/elevate.rs:8-10`, `194` |
| Four verbs; absolute tool paths | `crates/ubiqx-platform/src/sysblock/helper.rs:41-50`, `linux.rs:18-22`, `macos.rs:22` |
| Each product reads only its own feed | `crates/ubiqx-core/src/update.rs` (`feed_matches_product`), `UpdateChecker::for_product` |

### Traces: `REMOVAL.md`

| Claim | Code |
|---|---|
| The hosts markers the commands match | `crates/ubiqx-nofap/src/hosts.rs:54-56` |
| The resolved drop-in | `crates/ubiqx-nofap/src/resolvers.rs:83` |
| The policy file name and paths, Firefox's file | `crates/ubiqx-nofap/src/policies.rs:18`, `crates/ubiqx-platform/src/sysblock/policy_files.rs:33-65` |
| The state directories | `crates/ubiqx-nofap/src/state_dir.rs:13-15` |
| The macOS Firefox domain and keys | `crates/ubiqx-platform/src/sysblock/macos.rs:29-34` |
| The registry keys and values | `crates/ubiqx-platform/src/sysblock/policy_reg.rs:19-35` |
| `--owner`, `--scope`, the verbs | `crates/ubiqx-platform/src/sysblock/helper.rs:41-53` |
| Exit codes 0, 2, 3, 4, 5 | `crates/ubiqx-nofap/src/plan.rs:45-50` |
| The helper runs before Tauri, with no window and no database | `apps/desktop/src-tauri/src/main.rs`, `apps/nofap/src-tauri/src/main.rs` (first statement) |
| The Windows release build has no console | the same two files, `windows_subsystem = "windows"` |
| The NSIS hook: `runas`, `--owner <product>`, skipped in an update | `apps/desktop/src-tauri/packaging/nsis-hooks.nsh`, `apps/nofap/src-tauri/packaging/nsis-hooks.nsh` |
| The `.deb` hook: only on `remove`, executable from the package's file list, never fails | `apps/desktop/src-tauri/packaging/deb-prerm.sh`, `apps/nofap/src-tauri/packaging/deb-prerm.sh` |
| Both hooks are in the bundles | `bundle.linux.deb.preRemoveScript`, `bundle.windows.nsis.installerHooks` in both `tauri.conf.json` |
| ubiqX ships an MSI, the standalone app does not | `.github/workflows/ci.yml` (`--bundles msi,nsis` for ubiqX, `--bundles nsis` for the standalone app) |
| Keychain services `ai.ubiqx`, `ai.ubiqx.shield` | `crates/ubiqx-core/src/product.rs:95`, `108`; `crates/ubiqx-platform/src/secrets_file.rs:22` |
| Data and shared directories | `crates/ubiqx-app/src/lib.rs:182-185`; Tauri's `app_data_dir` |
