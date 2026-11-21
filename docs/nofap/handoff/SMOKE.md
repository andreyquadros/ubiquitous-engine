# Hand-off: SMOKE System smoke job on three systems, and the gate

Where this file and the code differ, the code wins.

**Six of the nine gate entries are open: `hosts` and `resolver` on Linux, macOS and Windows.
A build from this state blocks at DNS level on the three systems. `policies` is closed
everywhere** (it needs the manual browser check, checklist item 9).

Every entry was opened in its own commit after a green run of the real helper as
administrator on that system; the evidence, the output of the sandbox run and the log of
every CI run are in `docs/nofap/REAL-MACHINE-CHECKLIST.md` (sections 1 to 3).

## The gate as it stands

| OS | hosts | resolver | policies |
|---|---|---|---|
| Linux | open (`dae6fc5`) | open (`099b514`) | closed |
| macOS | open (`babb415`) | open (`babb415`) | closed |
| Windows | open (`bb83d78`) | open (`978dafd`) | closed |

`plan::ARG_BUDGET_WINDOWS` went from 1,800 to 24,000 (`38b7d01`), the longest plan the
Windows run sent through the app's own elevation command, and also the longest the helper
accepts.

## The smoke runs

Workflow `.github/workflows/nofap-smoke.yml`, job `nofap-system-smoke` on `ubuntu-latest`,
`macos-latest`, `windows-latest`. Runs are at
`https://github.com/andreyquadros/ubiquitous-engine/actions/runs/<id>`.

| Run | Commit | Ubuntu | macOS | Windows |
|---|---|---|---|---|
| 1, `44656118455` | `af28c33` | green | green | red: the script stopped at an adapter without a DNS configuration |
| 2, `44658216620` | `babb415` | green | green | red, 26 checks: owner of the hosts file changed; names answered with "no data"; registry padding; a false `SystemRoot` |
| 3, `44660200460` | `3518f27` | green | green | red, 12 checks: the "no data" answers, now explained |
| 4, `44662252304` | `3a99960` | green | green | red: a defect of the script (a list of one) |
| 5, `44662940118` | `4a12960` | green | green | red, 8 checks: one adapter with the interface index of another |
| 6, `44663846392` | `06cbc59` | green | green | **green**, 321 checks |
| 7, `44664769823` | `38b7d01` | green | green | **green**, 321 checks (the build with the six entries open and the new budget) |
| 8, `44665208311` | `0d61acf` | green | green | **green**, 321 checks |
| 9, `44665723586` | `6a80312` | green | green | **green**, 321 checks |
| 10, `44668139950` | `162a513` (review) | green | green | **green**, 321 checks |
| 11, `44668395648` | `d55a785` (review) | green | green | **green**, 312 checks (one adapter with a DNS configuration instead of two; three new checks) |

Run 11 is the latest run of each system, on the last commit that changes anything the
workflow watches (`d55a785`, the last commit of the review below). Jobs: Ubuntu
`136064609991`, macOS `136064609868`, Windows `136064610095`. Run 9 (`6a80312`, a snapshot
commit of the hook) was the latest when the engineer finished: Ubuntu `136055224331`,
macOS `136055224489`, Windows `136055224452`.

The sandbox run (hosts tier only, the one real-system run allowed here): green on `af28c33`,
`/etc/hosts` byte for byte what it was (same SHA-256 before and after, `cmp` with a saved
copy, owner and mode unchanged), no state directory left. Its output is in section 2 of the
checklist.

## What exists

| File | Content |
|---|---|
| `.github/workflows/nofap-smoke.yml` | the workflow: on push to the branch when `crates/ubiqx-platform/**`, `crates/ubiqx-nofap/**`, `crates/ubiqx-core/**`, `Cargo.toml`, `Cargo.lock`, `scripts/nofap/**` or the workflow change; its own concurrency group, a run that started always finishes; builds the helper; prints the machine and the helper's `status`; runs the smoke script as administrator with both tiers; on a red run prints what the script saved; compares the hosts file with a copy of its own afterwards; runs the system layer's tests on that system. No secret, nothing published |
| `.github/workflows/ci.yml` | job `rust-linux`: two steps, `python3 scripts/nofap/build_blocklist.py --check` and `cargo test -p ubiqx-nofap --test naming` (both seen green in CI) |
| `scripts/nofap/system-smoke.sh`, `system-smoke.ps1` | changed where the runs showed a defect or a missing check (below) |
| `crates/ubiqx-nofap/src/gate.rs` | the table with six entries open; the tests list the open entries (`OPEN`) and no longer expect a closed table |
| `crates/ubiqx-platform/src/sysblock/**` | five fixes the Windows runs proved necessary, each with a test (below) |
| `docs/nofap/REAL-MACHINE-CHECKLIST.md` | the gate table, the sandbox output, the latest run per system, the log of every run, the measurements, evidence under items 1 to 4, 6, 18, 20 to 22, 26, 27, and three new items (28 to 30) |

## What the real runs found, and what was changed

Linux and macOS were green from the first run: nothing had to change for them.

Windows, in the order the runs showed it:

1. **An adapter that is up without a DNS client configuration** (run 1; the
   accelerated-networking twin of the runner's adapter; on a person's machine a network card
   bound to a virtual switch). The helper asked every adapter that is up, with errors made
   terminating: the DNS read would have failed as a whole (`unreadable`) and the set script
   would have stopped half-way. `windows.rs`: the full read lists the adapters that are up
   and have a DNS client configuration; `SET_SCRIPT` names exactly those by interface index
   (`{indexes}`, like `RESET_SCRIPT`); with none to name, no script is started. Tests:
   `windows_leaves_out_an_adapter_the_dns_client_has_no_configuration_for`,
   `windows_with_no_adapter_to_set_starts_no_set_script`, the script text in
   `windows_runs_exactly_these_commands`.
2. **The hosts file changed owner** (run 2, checklist item 26): `O:BA` and the account's
   group instead of `O:SY G:SY`, kept after the removal. `hostsfile.rs`: on Windows the hosts
   file is written in place (apply, remove, and the restore after a write that did not read
   back); Unix keeps the rename. `sys.rs`: `write_in_place` writes over the file and cuts it
   to length afterwards instead of emptying it first. Test:
   `windows_writes_the_hosts_file_in_place_and_unix_renames_over_it` (two mutations tried,
   both fail it). Seen fixed in runs 3, 5, 6, 7.
3. **A program started with a false `SystemRoot` resolves no name** (run 2, item 21;
   `ping.exe` under the hostile variables). The helper gets the user's variables through
   UAC, and its lookup guards a DNS change. `sys.rs`: `pin_system_environment()` sets
   `SystemRoot` and `windir` of the helper's own process from `GetSystemDirectoryW`;
   `helper::run` calls it first. Test:
   `the_helper_resolves_names_when_started_with_a_false_system_root` (a child process with
   the false variables looks `localhost` up after the pinning; passes on the Windows
   runner, where runs 8 and 9 show that `ping.exe` cannot find `localhost` under those
   variables).
4. **Windows does not hand the null address to a program** (runs 2 and 3). For a name whose
   only answer is the null address, from the hosts file or from the family resolver,
   `getaddrinfo` fails with `WSANO_DATA` (11004) within a millisecond, while the DNS client
   holds the `0.0.0.0` and `::` records; without the block the canary fails with 11001. The
   block works, but `plan::assess` reads an empty canary answer as "the hosts file is not
   honoured": the app would have raised that issue on every Windows machine.
   `mod.rs`: `NameLookup::lookup_blocked` (default: `lookup`); `sys.rs`: on Windows "no data"
   is the null address there, "host not found" stays no answer (`blocked_answer`);
   `shield.rs` asks for the canary that way and for nothing else. Tests:
   `windows_no_data_for_a_blocked_name_reads_as_the_null_address`, the `asked_as_blocked`
   assertions in `names_are_looked_up_only_when_the_caller_asks` and
   `status_prints_the_observation_and_changes_nothing`. Seen on Windows in runs 5 to 7
   (`status: the canary reads as the null address`, and 11001 after each removal).
5. **An adapter that carries the interface index of another** (run 5). Asked by index, the
   DNS client answered for it with the other adapter's servers; it was listed as an adapter
   of its own, and the quick registry read (by GUID) would have shown it as not set: a drift
   that is none. `windows.rs`: the full read keeps the DNS client's answer only when it is
   filed under the adapter's own name (`InterfaceAlias`). Test:
   `windows_leaves_out_an_adapter_that_borrows_the_index_of_another`. **No green run has
   landed on such a runner since** (item 29).

The helper's `status` verb now looks the canary up (through `lookup_blocked`) while the
block is in the hosts file and prints the answer; without the block it looks nothing up.
Both scripts check that member, so a green run shows what the app's own check reads.

The scripts:

- `system-smoke.ps1` had never run. Defects fixed: the DNS snapshot stopped at an adapter
  without a configuration; the registry value was compared with its NUL padding; an `if`
  used as a value turned a list of one into a string. Added: the adapters left out are
  named; the helper's list of adapters is compared with the script's; `reg.exe query … /s
  /v NameServer` is run in the app's environment and parsed; the security descriptor of the
  hosts file is compared with the block in place too and printed when it differs; "no data"
  counts as blocked, and the canary must fail with 11001 after the removal; lookup details
  (per address family, `ping.exe`, `Resolve-DnsName`, the DNS client's cache) are printed
  when a name is not answered as expected; the helper's report is printed after an apply
  that failed; `ping.exe` is asked under the hostile variables. `$ArgumentBudget` follows
  the constant (24,000).
- `system-smoke.sh` ran green as written. Added: the two checks of `status.canary`.

## Deviations from the design and from the task

- **`plan.rs` was touched**, which the unit does not own: (a) three tests assumed a table
  that opens nothing and failed with the first flip (`a_gated_tier_is_refused_…`,
  `hosts_gated_when_the_gate_is_closed`, renamed
  `a_tier_reads_gated_exactly_when_the_gate_keeps_it_closed`,
  `preview_adds_no_line_for_a_tier_that_cannot_work_here`); they now ask the gate which
  tiers it keeps closed and use the policies tier where a closed tier is needed (commit
  `1071f9b`, tests only). (b) `ARG_BUDGET_WINDOWS` and its comment, as the task says, with
  `argument_budget_per_os` and `the_windows_budget_keeps_…` (renamed
  `the_budget_keeps_the_most_recent_names_that_fit`, now with names long enough for the
  budget to bind on every system).
- **Once the table opens `hosts` and `resolver` everywhere, no test can build a gate that
  keeps them closed**: `Gate::open` reads the table. The gated branches of `assess` for those
  two tiers are now exercised only through the policies tier. A gate that carries its table
  as data would restore that; it is a change to a public type, so it was not made here.
- **No `continue-on-error`** (design 9.3): the smoke job is a workflow of its own and blocks
  nothing, so a red job is shown as red.
- **`status` looks a name up** (hand-off U4: "looks no name up"): the canary, only while the
  block is in the file.
- **The flip commits** touch `gate.rs` (table and tests) and the checklist only. The first
  one (`dae6fc5`) also carries the rewrite of the gate's tests; the first Windows one
  (`bb83d78`) also carries the measurements table of the checklist.
- **The sandbox run was done once**, before the script gained the two `status.canary`
  checks. Those are covered by the Ubuntu and macOS runs 4 to 7, not by a second sandbox run.
- `sys.rs` is 278 lines (the design says under 200; 222 before). It still holds no decision
  but one: which Winsock error means "blocked", which is a fact about the system.

## Contract notes for the units that come after

- **Engine (U5)**: nothing to change. `SystemShield::observe` with `canary: true` returns
  `["0.0.0.0"]` on Windows for a honoured block (the null address stands for "no data"); an
  empty list still means "not honoured". The Windows argument budget is 24,000, so
  `extras_left_out` is now rare on Windows.
- **Shells (U7, U8)**: nothing to change. `helper::main_if_requested` sets `SystemRoot` and
  `windir` of the helper process on Windows; the app process is not touched.
- **Whoever opens `policies`**: add the entry to the table and to `OPEN` in the tests of
  `gate.rs`, delete `the_policies_tier_is_closed_on_every_system`, and look at the three
  `plan.rs` tests named above: they use the policies tier as "the tier every table keeps
  closed".
- **Tests of `sysblock`**: `WinAdapter` has a new field `configured`; `FakeLookup` records
  `asked_as_blocked`; the fake PowerShell accepts only the exact three scripts and fails a
  set or reset of an adapter that is not `configured`.
- **A change under `crates/ubiqx-platform`, `crates/ubiqx-nofap`, `crates/ubiqx-core` or
  `scripts/nofap`, or to `Cargo.toml` or `Cargo.lock`, starts a smoke run** (about 6 minutes on Ubuntu and macOS, 12 to 15 on Windows). The Windows job
  is the one to read: its runner differs from run to run (three kinds of third adapter so
  far).

## Open points

- **The consent prompt's path of a long plan is unmeasured.** The runner's shell is already
  elevated and its UAC elevates administrators without asking (`EnableLUA=1
  ConsentPromptBehaviorAdmin=0`), so no prompt was shown. The 2,048-character limit of
  `ShellExecuteEx` did not show up to 24,000; what the prompt adds is unknown. If a machine
  that prompts fails with a long plan, the app says "failed" for a user with hundreds of
  own sites; lowering `ARG_BUDGET_WINDOWS` is the fix (checklist item 18, person).
- **Windows 10 and 11 clients** were not run: the runner is Server 2025. A system that
  hands out `0.0.0.0` reads as blocked too (item 28).
- **The default PowerShell execution policy of Windows Home** (item 6): the runner's is
  `RemoteSigned`. If the modules do not load there, DNS reads `unreadable` and nothing is
  changed.
- **The adapter with a borrowed index** (item 29): fixed, not yet seen green on such a
  machine.
- **An adapter without IPv6** (item 22): not exercised; every runner adapter had it.
- **`REMOVAL.md`** (not this unit's): its Windows DNS command prints an error for an adapter
  the DNS client does not know and goes on (item 30).
- **Linux**: `/etc/systemd/resolved.conf.d` stays behind, empty, when the helper created it
  (the script notes it; U4 listed it).
- **A canary answered "no data" by a resolver** would read as honoured on Windows even if
  the hosts file were not: `.invalid` is reserved and resolvers answer it "no such name",
  so this needs a broken resolver and a hosts file that is ignored at the same time.
- The main CI's macOS and Windows jobs are cancelled by most pushes; the smoke workflow
  runs `cargo test -p ubiqx-platform sysblock` on each system instead, and that is the only
  place I read those tests' results (103 passed on each, run 7).

## Commands run on the final state

In the sandbox, on `6a80312` plus documents (the tree as pushed):

| Command | Result |
|---|---|
| `cargo test -p ubiqx-nofap -p ubiqx-platform` | 301 + 22 + 12 passed (`ubiqx-nofap`), 175 passed and 2 ignored (`ubiqx-platform`; the two ignored tests are the inner halves of two tests that run them in a child process) |
| `cargo clippy -p ubiqx-nofap -p ubiqx-platform --all-targets -- -D warnings` | clean |
| `cargo clippy -p ubiqx-platform --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | clean |
| `cargo clippy -p ubiqx-platform --all-targets --target aarch64-apple-darwin -- -D warnings` | clean |
| `cargo check --workspace --all-targets` | passes |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --exclude ubiqx-desktop --exclude ubiqx-nofap-desktop --all-targets -- -D warnings` | clean |
| `cargo test --workspace --exclude ubiqx-desktop --exclude ubiqx-nofap-desktop` | every test binary passes (no failure) with the six entries open |
| `python3 scripts/nofap/build_blocklist.py --check`, `cargo test -p ubiqx-nofap --test naming` | pass (also in the Linux job of the main CI) |
| `bash -n scripts/nofap/system-smoke.sh` | passes |
| `system-smoke.ps1` through the `tree-sitter-powershell` grammar (scratch virtualenv) | no error node; its real test is the Windows job |
| `UBIQX_SHIELD_SMOKE=1 scripts/nofap/system-smoke.sh --tiers hosts`, once, as root in the sandbox | green; `/etc/hosts` identical afterwards |

How the new tests were checked: the gate's tests against two mutations in a scratch copy
(`Gate::new` keeping a tier the table opens; a table entry opened without the list), the
hosts write test against two (the rename kept on Windows; the restore not in place): each
fails a test. The plan and platform tests were run in a scratch copy with the table closed,
with one entry open and with six open. The Windows-only behaviour (the PowerShell text, the
environment pinning, "no data") cannot be mutated here; the Windows job is what shows it.

**Not run:** the frontend's tests (nothing of it was touched); anything on a Windows or
macOS machine other than the CI runners; the smoke script in the sandbox a second time.

## Review

Independent review of the unit, 2026-11-20. Three commits (`84d82c6`, `162a513`,
`d55a785`) and this section with the checklist entries of runs 10 and 11. **The gate is
unchanged: six entries open, `policies` closed everywhere. No entry had to be closed.**

### What was checked, and what it showed

| Check | Result |
|---|---|
| `cargo test -p ubiqx-nofap -p ubiqx-platform` | before the review: 301 + 22 + 12 and 175 passed, 2 ignored; after it: 176 passed, 2 ignored (`ubiqx-platform`) |
| `cargo clippy -p ubiqx-nofap -p ubiqx-platform --all-targets -- -D warnings`, and `-p ubiqx-platform` for `x86_64-pc-windows-msvc` and `aarch64-apple-darwin` | clean, before and after |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --exclude ubiqx-desktop --exclude ubiqx-nofap-desktop --all-targets -- -D warnings`, `cargo test --workspace --exclude …`, `cargo check --workspace --all-targets` | clean, every test binary passes, passes (with another unit's uncommitted file in the tree) |
| `python3 scripts/nofap/build_blocklist.py --check`, `cargo test -p ubiqx-nofap --test naming`, `bash -n scripts/nofap/system-smoke.sh` | pass; the two offline steps are also green in the job "Rust (portable crates, Linux)" of the main CI on `eb95751` |
| `scripts/nofap/system-smoke.sh --tiers hosts` without `CI=true` or `UBIQX_SHIELD_SMOKE=1`, and with `CI=false` | refuses, exit 2, nothing touched |
| `/etc/hosts` of the sandbox | SHA-256 `d343af6b…abd2b5`, 81 bytes, `0:0`, mode 644, no `ubiqx-shield` line; no `/var/lib/ubiqx-shield`. Its creation time (01:47:53 UTC) is 50 s after the commit the sandbox run names, which fits a file renamed over twice by that run |
| Run 9, the three job logs read in full | each system applied, checked, removed and checked the restore: 42 `ok` lines on Ubuntu, 41 on macOS, 321 on Windows, no `FAIL`; the only skipped steps are the other systems' steps and the one for a red run; the job's own comparison of the hosts file passed; the machine descriptions in the checklist match the logs |
| The workflow | starts on a push to this one branch only (no `pull_request`, no `workflow_dispatch`), `permissions: contents: read`, no secret, no upload, no release step. A pull request from a fork cannot start it |
| The five platform fixes, read for the other two systems | Unix behaviour is unchanged by the hosts change (the rename stays; in place only for a mount point), `lookup_blocked` and the environment pinning do nothing off Windows, the DNS scripts are Windows only. Ubuntu and macOS were green in every run after each fix |

The sandbox run of the smoke script was **not repeated** (it is the one real-system run of
the project); its record was checked against the file's metadata only.

### What was changed

1. **A hosts write in place that stops half-way is undone** (`84d82c6`, `hostsfile.rs`).
   The fix that made Windows write the hosts file in place gave up the atomic rename there,
   and `write` returned at once when the write failed: a write that had started and then
   failed (a full disk, a lock taken by security software) left the start of a block behind
   the user's lines, against the function's own promise that the old bytes are there on an
   error. Unix had the same hole where the file is a mount point. Now the bytes read at the
   start are put back whenever a write did not land and the file no longer holds them.
   `MemFs::stall_next_in_place` makes an in-place write stop half-way; the test
   `an_in_place_write_that_stops_half_way_is_undone_and_reported` fails without the fix on
   every system.
2. **The workflow** (`162a513`): `persist-credentials: false` on the checkout (the job runs
   the repository's scripts as administrator); the path filter also names
   `crates/ubiqx-core/**`, `Cargo.toml` and `Cargo.lock`, which the helper is built from
   too, so the latest green run stays evidence for the code in the branch.
3. **The Windows script checks the exit code of the elevation command** (`d55a785`). The
   runs had only shown exit 0 together with a written block, which a lost exit code would
   also give (hand-off U4 left open whether `exit $p.ExitCode` hands the code back). The
   printed command is now also run with a plan argument the helper refuses before it
   touches anything: run 11 shows the helper's exit code 2 coming back, with the hosts file
   and the state directory untouched.

Runs 10 and 11 are in the checklist (section 3). Run 11's Windows runner was a fourth kind:
two adapters up, one of them an "Azure Network Adapter" without a DNS configuration and
with an index of its own.

### For the units that come after

- **With `hosts` and `resolver` open, a real `NativeSystemShield` applies for real.**
  `NativeSystemShield::native` carries the real elevator, and in a sandbox that runs as
  root `pkexec` needs no prompt: a test that hands such a shield a hosts or resolver plan
  writes `/etc/hosts` and the resolver drop-in of the machine it runs on. Until the flips
  such a plan was refused as gated. Today one test builds a native shield
  (`the_native_shield_of_this_machine_follows_the_gate_table`), and it applies only a plan
  for a tier that is gated. Tests of the shells and of the composition (U7, U8) must go
  through the scripted platform or the fakes, never through `PlatformServices::native_*`
  with a system plan.
- **The smoke workflow runs on this branch only.** After a merge nothing starts it; a
  change to the system layer on another branch needs `branches` changed first.
- A push that touches `ubiqx-core`, `Cargo.toml` or `Cargo.lock` now starts a smoke run too.

### Still open after the review

- Everything in the engineer's list of open points stands. Of those, the ones a user can
  meet: the Windows argument budget and the exit code were measured **without a consent
  prompt** (item 18); no Windows 10 or 11 client has run any of this (item 28); the adapter
  with a borrowed index has still not been seen green (item 29; runs 10 and 11 had none).
- **The Windows full read silences every error of the DNS client per adapter**
  (`-ErrorAction SilentlyContinue` in `FULL_READ_SCRIPT`), not only "no such object". If
  the DNS client failed for every adapter while `Get-NetAdapter` still worked, the read
  would list no adapter, the apply would set nothing and exit 0, and `plan::assess` reads
  "no active interface" as applied. Before the first Windows fix that case read
  `unreadable`. Telling the two errors apart needs a change to a script that only a
  Windows run can test; I left it. The modules not loading at all (item 6) still reads
  `unreadable`: that error stops the script.
- **Branches for a closed `hosts` or `resolver` tier are no longer exercised**, in
  `plan::assess` and in `converge`, because no table keeps them closed. The tests that
  cover them ask the table and now skip that part (for example the loop over
  `resolver_closed` in `cycle.rs`, `a_run_writes_only_the_tiers_its_own_gate_opens`, which
  asserts nothing today). The same branches are unreachable in the product for the same
  reason; they come back, with their tests, if an entry is closed again. A gate that
  carries its table as data is a change to a public type (`mock_nofap.rs` builds the
  struct by its fields), so it is the integrator's.
- The header comments of both smoke scripts still say `--unverified` is passed "because
  the gate is closed until a run like this one is green". The scripts pass it either way
  and the helper ignores it for a tier the table opens, so the runs are unaffected.
- `system-smoke.ps1` has still only been parsed by a grammar outside PowerShell here; its
  test is the Windows job (run 11 for the new lines).
- **Not run by the reviewer:** the frontend's tests (nothing of it was touched); the
  mutation runs the engineer describes (the tree is shared, and a mutation in it could be
  committed by the snapshot hook); the smoke script in the sandbox.

