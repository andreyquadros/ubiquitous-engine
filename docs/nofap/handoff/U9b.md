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
  with `secure_delete` (decision 20), and the tracker folds the write-ahead log right after a
  scrub (decision 36, `2253638`). What remains is a fold that another reader blocks.
- "Apagar todos os dados não sobrescreve" was false: it wipes with `secure_delete`, folds the
  log before and after `VACUUM`, and deletes the `exports` folder.
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
| Freed bytes are zeroed; the log is folded after a scrub, unless another reader blocks it | `crates/ubiqx-storage/src/db.rs:19-29` (`secure_delete = ON`), `crates/ubiqx-engine/src/tracker.rs:210-218` (`fold_log`), `crates/ubiqx-storage/src/store.rs:36-45` (`fold_log_now`, busy is an error) |
| The hosts block's markers, the canary name, the caps 20,000 / 8,000 | `crates/ubiqx-nofap/src/hosts.rs:44-56` |
| The canary is looked up every 30 minutes | `crates/ubiqx-nofap/src/plan.rs:55` (`CANARY_EVERY_SECS = 1_800`), `crates/ubiqx-engine/src/nofap/system.rs:530` |
| Cloudflare for Families addresses; the probe name `one.one.one.one` | `crates/ubiqx-nofap/src/resolvers.rs:28-39` |
| The previous DNS is noted for display | `crates/ubiqx-core/src/nofap.rs:383` (`previous_dns`) |
| "Restaurar minha rede" is immediate | `crates/ubiqx-engine/src/nofap/system.rs:946` (`restore_network`); commitment 6.3 |
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
| The log's other lines | `crates/ubiqx-engine/src/nofap/service.rs:188`, `367`; `system.rs:385`, `869`, `936`, `956`, `1056`; `mod.rs:338-361`, `498-512`, `745`, `776`; `crates/ubiqx-app/src/nofap_app.rs:680`, `702` (all constant text or an error class) |
| Default limits 20 / 120 / US$ 2 | `crates/ubiqx-core/src/nofap.rs:296-311` |
| The fallback notification: title "UBI", a neutral sentence | `crates/ubiqx-nofap/src/messages.rs:11`, `88-94`; `crates/ubiqx-engine/src/nofap/cards.rs:228-230` |
| The databases and their directories | `crates/ubiqx-core/src/product.rs:91-109`; Tauri's `app_data_dir` in both shells |
| The secrets file fallback | `crates/ubiqx-platform/src/secrets_file.rs:1-19` |
| The shared directory and the lock file, deleted on off and on quit | `crates/ubiqx-nofap/src/primary.rs:25-28`, `crates/ubiqx-app/src/lib.rs:242`, `crates/ubiqx-engine/src/nofap/system.rs:1064-1085`, `service.rs:368`, `878` |
| The export's folder, name and content; ubiqX's export has nothing of the module | `crates/ubiqx-engine/src/nofap/service.rs:842-868`, `crates/ubiqx-engine/src/state.rs:95`, `crates/ubiqx-app/src/nofap_app.rs:419`, `crates/ubiqx-engine/src/handle.rs:341-363` |
| 90 days of counters | `crates/ubiqx-nofap/src/progress.rs:11` |
| "Apagar meu histórico" overwrites and folds the WAL | `crates/ubiqx-storage/src/repos/nofap.rs:74`, `312` |
| "Apagar todos os dados": counters go, choices and lists stay, overwritten, `exports` deleted | `crates/ubiqx-storage/src/repos/maintenance.rs:15-61` (`WIPE`, best-effort folds around `VACUUM`), `crates/ubiqx-engine/src/handle.rs:463-492` |
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
| Data and shared directories | `crates/ubiqx-app/src/lib.rs:242`; Tauri's `app_data_dir` |

### Deviations from the design and the task

- **The standalone app is built with `node ../../desktop/node_modules/@tauri-apps/cli/tauri.js
  build`**, not `pnpm tauri build`: `apps/nofap` has no `package.json`, and the Tauri CLI must run
  with the shell's directory as working directory (hand-off U8). Calling the CLI's JS entry
  through `node` works the same on the three runners (no `.cmd` shim on Windows).
- **The rename steps read `productName` from each `tauri.conf.json`** (`node -p`) instead of
  hard-coding `ubiqX` and the display name, so a renamed display name keeps working and no
  workflow line carries the source word.
- **ubiqX is renamed before the standalone app is built.** Each product's globs name it
  anyway, so the order is a second guard, not the only one.
- **Diagnostics for the real-machine checklist** (item 25): each rename step prints the
  bundle's executable (macOS), the NSIS script's `MAINBINARYNAME` (Windows) and the `.deb`'s
  package name and `/usr/bin` entry (Linux). They only read; they cannot fail a job.
- **`clippy --all-targets` for `ubiqx-nofap-desktop`** on Windows (design 9.2 says without
  `--all-targets`): the shell's tests module (`src/tests.rs`) is linted there too. ubiqX's
  existing Windows clippy step is unchanged.
- **No `cargo test -p ubiqx-nofap-desktop` on Linux**: `rust-linux` has no webkit and the
  `linux-app` job builds in release only. The shell's tests run on macOS (`cargo test
  --workspace`) and Windows.
- **The standalone `tauri.conf.json` still lists `msi` among its targets.** CI passes
  `--bundles nsis` on Windows, so no MSI is built; a local `tauri build` without `--bundles`
  would build one. The file is U8's; see open points.
- **`PRIVACY.md` and `REMOVAL.md` stay PT-BR only**, as U9a left them.

### Contract notes for the units that come after

- **The integrator:** `vars.NOFAP_PUBLISH` does not exist yet; with it unset `publish-shield` is
  skipped on `main` too. Setting it to `true` (Settings → Secrets and variables → Actions →
  Variables) publishes the standalone app on the next push to `main`, under
  `shield-continuous`, with the same signing key as ubiqX. The first publish creates the
  release; `PRODUCTS.shield.latest = false` keeps ubiqX's release "Latest".
- **Whoever changes a `productName`:** nothing in `ci.yml` needs to change; `REMOVAL.md` (part
  2 and 4), `MACOS-TESTING.md` § 7 and `WINDOWS-LINUX.md` § 6 name the install folders after
  it, and the `.deb` package name follows it (Tauri derives `no-fap` from `NoFap`).
- **Never set `UBIQX_UPDATE_FEED_URL` or `UBIQX_SHIELD_UPDATE_FEED_URL` at workflow or job
  level**; set it on the one build step of its product if a fork needs it.
- **A new bundle kind** for either product needs a rename line in its product's step and a
  path in its `upload-artifact`; `publish-release.mjs` infers platform and kind from the
  stable name.

### CI runs

Pushes of the other units of this wave cancelled most runs of this unit's commits part-way
(every push cancels the branch's running CI). What finished, job by job:

| Run (commit) | Job | Outcome and what it showed |
|---|---|---|
| `44966689890` (`d8ca704`, the CI commit) | Windows | clippy `-p ubiqx-nofap-desktop --all-targets` and `cargo test -p ubiqx-nofap-desktop` passed; cancelled in ubiqX's bundle step |
| `44968640263` (`837389c`) | macOS app build | **success**: `ubiqX.app/Contents/MacOS/ubiqx-desktop`, `NoFap.app/Contents/MacOS/ubiqx-shield`; artefact `ubiqX-macos-app` (4 files: `.dmg`, `.app.zip`, `.app.tar.gz`, `.sig`) and `ubiqx-shield-macos-app` (4 files, same kinds) |
| `44970964558` (`1b666de`) | Linux, macOS | **success** both; `ubiqX-linux-app` and `ubiqx-shield-linux-app` (4 files each: `.AppImage`, `.AppImage.tar.gz`, `.sig`, `.deb`); `.deb` packages `ubiq-x` and `no-fap` |
| `44972849562` (`2fd83eb`) | Rust Linux, Frontend (with `pnpm build:nofap`), Linux, macOS | **success**; `usr/bin/ubiqx-desktop` and `usr/bin/ubiqx-shield` in the two `.deb`; Windows: ubiqX's MSI and NSIS built and uploaded under the old names, `MAINBINARYNAME "ubiqx-desktop"`, standalone clippy and tests passed, then cancelled during the standalone NSIS build by a snapshot push |
| **`44975006898` (`da88636`, the commit that added this file)** | all seven | **every job green**: Rust Linux, Frontend, macOS, Windows and Linux app builds success; both publish jobs skipped (feature branch). Six artefacts: `ubiqX-macos-app` (4 files), `ubiqX-windows-app` (4: `.msi`, `-setup.exe`, `-setup.nsis.zip`, `.sig`), `ubiqX-linux-app` (4), `ubiqx-shield-macos-app` (4), `ubiqx-shield-windows-app` (3: `-setup.exe`, `-setup.nsis.zip`, `.sig`; no `.msi`), `ubiqx-shield-linux-app` (4). `MAINBINARYNAME` `ubiqx-desktop` and `ubiqx-shield` |

The updater signing secrets are present: every job above built signed updater artefacts for
both products (`UBIQX_UPDATER_SIGNED=1`). The publish jobs were never reached (`push` on a
feature branch: both are skipped by their conditions). This row was added by a later commit (this
section only); its own run is listed in the final report of this unit.

### Open points

- **ubiqX shows no third-party notices.** It embeds the same lists as the standalone app (MIT
  and CC BY 4.0 inputs), but no screen of ubiqX shows `THIRD-PARTY-NOTICES`, and no installer
  carries the file. `LICENSING.md` § 7 says so. A frontend change (About, or the module's help
  tab), for the integrator.
- **The standalone app's `.deb` package is `no-fap`**, which Tauri derives from the display
  name. `dpkg -l` shows it to every account. It does not contain the source word literally, so
  the naming test passes, but it is the display name in the machine ring; checklist item 15's
  "installer names" row is where a person decides. Tauri 2 has no option for the package name
  short of changing `productName`.
- **The standalone `tauri.conf.json` lists `msi`** among its bundle targets (U8). CI builds
  NSIS only, as decision 39 asks; a local `tauri build` without `--bundles` would still build
  an MSI with no removal hook. Dropping `msi` from that file is a one-word change for its owner.
- **The two `deb-prerm.sh` comments point at "docs/REMOVAL.md of the source tree"**; the file
  is `docs/nofap/REMOVAL.md` (packaging files of U7 and U8).
- **Installed names are read from the bundles, not from installed apps** (checklist item 25):
  the MSI install folder (`%ProgramFiles%\ubiqX\`) and the NSIS one (`%LocalAppData%\<product
  name>\`) are Tauri's defaults, not seen; the autostart entry names are unread.
- **Not run:** any installer, uninstaller or hook on a real machine; any command of
  `REMOVAL.md` (nothing here may touch the system); the publish jobs (they run only on `main`
  with the variable set). `PRIVACY.md`'s log claims are traced to code, not to a log file after
  a day of use (checklist item 19).

### Commands run

| Command | Result |
|---|---|
| `node --test scripts/publish-release.test.mjs` | 51 pass, 0 fail (before and after; the scripts did not change) |
| `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` | parses, seven jobs |
| A scratch simulation of the new rename steps (the `run:` blocks extracted from `ci.yml`, run with bash against fake bundle trees holding both products' files, signed and unsigned) | each product's files renamed, the other's untouched, unsigned updater files removed |
| `cargo fmt`, `cargo clippy`, frontend checks | not run by this unit: it changed no Rust or TypeScript file. CI ran them (table above) |

## Review

An independent review of this unit, done after the engineer's last commit (`ab0c577`).

### What was reproduced

| Check | Result |
|---|---|
| `node --test scripts/publish-release.test.mjs` (before any change) | 51 pass, 0 fail |
| `python3 -c "import yaml; …"` on `ci.yml` | parses; jobs `rust-linux`, `frontend`, `macos-app`, `windows-app`, `linux-app`, `publish-continuous`, `publish-shield` |
| `bash -n scripts/nofap/system-smoke.sh` | passes |
| `apps/desktop/package.json` has `build:nofap` and `dev:nofap` | yes |
| CI run `44977659640` (`ab0c577`), read with `gh api`, job by job and step by step | every step of Rust Linux, Frontend, macOS, Windows and Linux app builds `success`; both publish jobs `skipped` (push to a feature branch). Frontend step 9 is `pnpm build:nofap`. Windows: `cargo clippy -p ubiqx-nofap-desktop --all-targets -- -D warnings` compiled and passed; `cargo test -p ubiqx-nofap-desktop` ran **4 tests, all ok** (`card::tests::the_card_route_keeps_only_the_safe_characters_of_the_id`, `app::tests::the_card_window_may_only_read_the_settings`, `tests::the_two_products_settle_on_one_primary_and_never_both_act`, `tests::an_hour_of_use_records_nothing_of_what_was_on_screen`). Six artefacts: `ubiqX-macos-app`, `ubiqX-windows-app` (4 files: `.msi`, `-setup.exe`, `-setup.nsis.zip`, `.sig`), `ubiqX-linux-app`, `ubiqx-shield-macos-app` (4: `.dmg`, `.app.zip`, `.app.tar.gz`, `.sig`), `ubiqx-shield-windows-app` (3: no `.msi`), `ubiqx-shield-linux-app`. The logs show `MAINBINARYNAME "ubiqx-desktop"` / `"ubiqx-shield"`, `ubiqX.app/Contents/MacOS/ubiqx-desktop`, `NoFap.app/Contents/MacOS/ubiqx-shield`, `.deb` packages `ubiq-x` / `no-fap` with `usr/bin/ubiqx-desktop` / `usr/bin/ubiqx-shield` |
| The NSIS install folder | the NSIS template inside `@tauri-apps/cli` 2.11.4 (the pinned CLI) sets `$INSTDIR` to `$LOCALAPPDATA\${PRODUCTNAME}` for `currentUser`: `REMOVAL.md`'s `%LOCALAPPDATA%\ubiqX\ubiqx-desktop.exe` and `%LOCALAPPDATA%\NoFap\ubiqx-shield.exe` follow from it. The MSI folder could not be read from the CLI |

**The workflow, read for a path off `main` or around the variable.** `publish-shield` runs only
when all three of `github.event_name == 'push'`, `github.ref == 'refs/heads/main'` and
`vars.NOFAP_PUBLISH == 'true'` hold (a `pull_request` run, a feature branch or an unset variable
skip it); there is no `workflow_dispatch` or other trigger. The standalone app's files cannot
reach ubiqX's release by three independent guards: `publish-continuous` downloads only
`ubiqX-*-app` (and even a case-blind matcher would not take `ubiqx-shield-*-app` for it, since the
words after the prefix differ), collects only `find -name 'ubiqX-*'` (case-sensitive on the
Ubuntu runner), and `publish-release.mjs` refuses a file whose name starts with the other
product's prefix, `productName` or `mainBinaryName` before any request (`collectAssets`,
`productOf`). The reverse holds the same way. No feed variable is set at workflow or job level.

**Ten claims of `PRIVACY.md`, traced by the reviewer** (each confirmed in the code): the guard
every 2 s (`nofap_app.rs:48`, `engine.rs:56`); the canary every 30 minutes, only while the
block is in the file (`plan.rs:55`, `system.rs:528-530`); the caps 20,000 / 8,000
(`hosts.rs:44-45`); the export's folder, name `shield-<date>-<time>.json` and content, with no
`internal` and only `learned_count` (`service.rs:844-868`, `nofap_ipc.rs:215-269`); the log
file `ubiqX.log` / `NoFap.log` in Tauri's `app_log_dir` (`tauri-plugin-log` 2.9.1 names the
file after `package_info().name`, the `productName`), at `info` with the HTTP crates at `warn`,
and every module line constant text or an error class (`service.rs:188`, `367`;
`system.rs:385`, `869`, `936`, `956`, `1056`; `mod.rs:338-361`, `498-512`; `vision.rs:372`,
`400`); the databases `ubiqx.db` / `shield.db` under `ai.ubiqx.app` / `ai.ubiqx.shield`
(`product.rs:89-111`, the identifiers in both `tauri.conf.json`); the limits 20 / 120 / US$ 2
(`nofap.rs:53-57`); the notification title "UBI" (`messages.rs:11`); the shared directory
`ubiqx-shield` under the local data directory and `primary.lock` holding product, pid and
heartbeat (`ubiqx-app/src/lib.rs:242`, `primary.rs:25-36`); "Apagar todos os dados" deletes
`ai_usage` and `nofap_day`, keeps `nofap_state` and `nofap_sites`, folds the log around `VACUUM`
without failing on a busy fold, deletes `exports` and clears the module's memory
(`maintenance.rs:15-60`, `handle.rs:463-493`); "Apagar meu histórico" clears values, plans,
counters and snoozes and folds the log (`service.rs:792-811`, `repos/nofap.rs:302-317`); the
secrets file is `0600` on Unix (`secrets_file.rs:93`, `102`); the previous DNS lives in
`internal` and so is not exported (`system.rs:915`).

**Ten commands of `REMOVAL.md`, traced by the reviewer**: the hosts markers the `perl` and
PowerShell expressions match (`hosts.rs:54-56`); the resolved drop-in path
(`resolvers.rs:83`); the five Linux policy paths and Firefox's file (`policy_files.rs:33-65`);
the Firefox JSON shown in part 3.1 is byte for byte `firefox_json()` (`policies.rs:104-117`,
pinned by its test at `policies.rs:287-290`); the state directories (`state_dir.rs:13-15`);
the macOS keys `DNSOverHTTPS__Enabled`, `DNSOverHTTPS__Locked`, `EnterprisePoliciesEnabled`
in `/Library/Preferences/org.mozilla.firefox` (`macos.rs:29-34`); the registry keys and values
(`policy_reg.rs:16-37`, `policies.rs:36-59`); `--shield-helper remove --owner ubiqx|shield|all
--scope all|resolver` (`helper.rs:39-53`, `ProductId::parse`) and the exit codes 0 / 3 / 4
(`plan.rs:45-50`, `converge.rs:281-288`); the keyring services and `secret-tool` /
`cmdkey` attributes (`product.rs:95`, `108`, passed to the platform at `ubiqx-platform/src/lib.rs:102-110`);
the autostart entries `<productName>.plist` / `.desktop` / `Run` value (`tauri-plugin-autostart`
2.5.1 uses `package_info().name`, `auto-launch` 0.5.0 builds the file names); the `.deb` hook
acts only on `remove` and the NSIS hook is skipped in `$UpdateMode` (both `packaging/` files).

### What the review changed

| Commit | Change |
|---|---|
| `d83c7fc` | **A test of the workflow** (`scripts/publish-release.test.mjs`, `describe('ci workflow')`, 11 tests). The engineer's simulation of the rename steps was a scratch file, deleted; nothing guarded the boundaries between the two products. A small reader of the YAML subset `ci.yml` uses pins: the two publish conditions (all three parts for `publish-shield`, nothing negated or or-ed); each publish job's download pattern, `find` filter, `--product` and version call; no feed variable at workflow or job level; each app job builds the standalone app in `apps/nofap/src-tauri` after ubiqX's upload and stamps its version; Windows builds it with `--bundles nsis` only; the Windows clippy and test steps; `pnpm build:nofap`; the two exclusions of `rust-linux`. Then, for each OS, signed and unsigned, the real `run:` blocks of both rename steps are run with bash (Windows with its `-e -o pipefail`) against a bundle tree that holds **both products' outputs at once**, under the names Tauri gives them, and each `upload-artifact` glob (case-blind for Windows, as there) must take exactly that product's stable names, each holding that product's output. Only `ditto` and `dpkg-deb` are stand-ins. Mutations tried on a copy of the workflow, each caught: a bare `*.dmg` in ubiqX's macOS step, a bare `*.deb` in ubiqX's Linux step, `publish-shield` without the variable, `--bundles msi,nsis` for the standalone app, a download pattern `ubiq*-app`, a feed variable in the workflow `env`, a standalone upload glob `nsis/*.sig`. The bash part is skipped on Windows; the file runs in the Ubuntu frontend job |
| `d83c7fc` | `README.md`: the standalone build commands name the bundles of each OS as CI does (without `--bundles` the shell's configuration would build an MSI on Windows, which has no removal hook); `UBIQX_LOG` is the CLI's, the apps log at `info` (the engineer had corrected this in `MACOS-TESTING.md` only) |
| `d83c7fc` | Checklist item 25: what the pinned Tauri CLI says about the NSIS folder |
| this commit | `REMOVAL.md` notes the empty directories a Linux removal leaves (`/etc/systemd/resolved.conf.d`, `/etc/firefox/policies`, `policies/managed`; hand-off U4), with the safe `rmdir`. This hand-off's trace tables had line numbers moved by FU2b's commits (the canary, "Restaurar minha rede", the shared directory, the log lines); they are corrected above |

No bug was found in the workflow itself: the rename globs, upload globs, conditions and
patterns do what the engineer said, and the reviewer's test now proves it on every push.

### Still open, or unverified by the review

- **ubiqX's release now waits on the standalone app's build.** Both products are built in the
  same three jobs, and `publish-continuous` needs all three, so a failing standalone build or
  upload on `main` also stops ubiqX's release. That is what design 8.6 asks ("CI builds the
  standalone app on the three runners in every run"); whoever prefers ubiqX to ship regardless
  would move the standalone steps to their own jobs (they would then rebuild the workspace, about
  ten more minutes per OS). For the integrator.
- `vars.NOFAP_PUBLISH` compares case-blind in GitHub expressions (`TRUE` also opens it); harmless.
- The publish jobs have never run (main only); the reviewer's test covers their inputs, not a
  real publish.
- Everything the engineer listed as open stays open: ubiqX shows no third-party notices; the
  standalone `.deb` is `no-fap`; the standalone `tauri.conf.json` still lists `msi` (U8's file);
  the `deb-prerm.sh` comments name `docs/REMOVAL.md` (U7/U8's files); the MSI folder and the
  autostart entries are unseen on a real machine (item 25); no hook, installer or `REMOVAL.md`
  command has run on a real machine (item 15); the log after a day of use (item 19).
- Not run by the review: `cargo fmt` / `cargo clippy` / the frontend checks (no Rust or
  TypeScript file changed); any installer or system command.
