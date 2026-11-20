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

Run 9 is the latest run of each system, on the last commit that changes anything the
workflow watches (`6a80312` is a snapshot commit of the hook; it carries this unit's last
changes to the workflow and to the comment of the budget). Jobs: Ubuntu `136055224331`,
macOS `136055224489`, Windows `136055224452`.

The sandbox run (hosts tier only, the one real-system run allowed here): green on `af28c33`,
`/etc/hosts` byte for byte what it was (same SHA-256 before and after, `cmp` with a saved
copy, owner and mode unchanged), no state directory left. Its output is in section 2 of the
checklist.

## What exists

| File | Content |
|---|---|
| `.github/workflows/nofap-smoke.yml` | the workflow: on push to the branch when `crates/ubiqx-platform/**`, `crates/ubiqx-nofap/**`, `scripts/nofap/**` or the workflow change; its own concurrency group, a run that started always finishes; builds the helper; prints the machine and the helper's `status`; runs the smoke script as administrator with both tiers; on a red run prints what the script saved; compares the hosts file with a copy of its own afterwards; runs the system layer's tests on that system. No secret, nothing published |
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
- **A change under `crates/ubiqx-platform`, `crates/ubiqx-nofap` or `scripts/nofap` starts a
  smoke run** (about 6 minutes on Ubuntu and macOS, 12 to 15 on Windows). The Windows job
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
