# Hand-off: U9a Release scripts, smoke scripts and documents (first half of U9)

Where this file and the code differ, the code wins.

## Engineer

U9a is done as far as it can be without a real machine: the release scripts publish two
products and are tested; the two smoke scripts exist and were **not run** (as instructed);
the three documents are written. `.github/workflows/ci.yml`, `README.md` and
`docs/ARCHITECTURE.md` were not touched: they belong to the second half.

**Every gate entry is still closed. A build from this state blocks nothing at DNS level.**

What exists now:

- **Release scripts**
  - `scripts/app-version.mjs`: the table `PRODUCTS` (`ubiqx`, `shield`), `resolveProduct`,
    `--product ubiqx|shield` (default `ubiqx`). `appVersion`, `baseVersion`,
    `tauriConfVersion` and `writeAppVersion` take an optional product.
  - `scripts/publish-release.mjs`: `--product ubiqx|shield` (default `ubiqx`). The standalone
    app is published as `ubiqx-shield-<os>-<arch>.<ext>` under the tag `shield-continuous`,
    with its own `latest.json` (`product: "ubiqx-shield"`) and `updater.json`. New exports:
    `productStems`, `productOf`, `feedBelongsTo`; `inferAsset`, `collectAssets`,
    `productVersion`, `dmgAssetName` and `appZipAssetName` take an optional product.
  - `scripts/ci-app-version.sh`: `--product ubiqx|shield`, passed on to the Node script.
  - `scripts/publish-release.test.mjs`: 48 tests (34 before, all unchanged and passing). The
    14 new ones cover the second product against the mock GitHub, both refusals, the product
    table against `crates/ubiqx-core/src/product.rs`, `update.rs`, `nofap.rs` and both
    `tauri.conf.json`, and the bash wrapper in a scratch checkout.
- **Smoke scripts**
  - `scripts/nofap/system-smoke.sh` (Linux, macOS; bash 3.2 and python3) and
    `scripts/nofap/system-smoke.ps1` (Windows; PowerShell 7).
- **Documents**
  - `docs/nofap/PRIVACY.md`, `docs/nofap/REMOVAL.md` (PT-BR), and
    `docs/nofap/REAL-MACHINE-CHECKLIST.md` (English).

**Commands run on the final state**

| Command | Result |
|---|---|
| `node --test scripts/publish-release.test.mjs` | 48 pass, 0 fail |
| `bash -n scripts/nofap/system-smoke.sh` | passes |
| `bash -n scripts/ci-app-version.sh` | passes |
| `shellcheck` 0.11.0 (installed in a scratch virtualenv, not in the repository) on both shell scripts | clean |

**Checked by other means, and what that means**

- **The new tests can fail.** Fourteen mutations of the three scripts, one at a time (no
  foreign-file guard, no feed-owner guard, no tag guard, another tag, another feed name, the
  display name as prefix, the wrong configuration path, the wide default DMG glob, the bash
  wrapper ignoring the product, and five more): every one failed at least one test. The
  scripts were restored from a copy after each.
- **The checks of the shell smoke script against real output of the landed code.** A
  throwaway crate outside the repository (its own target directory) linked `ubiqx-nofap` and
  `ubiqx-core` and wrote: `plan::encode_plan` for both plans, `hosts::splice` of the real
  20,000-name block into three files (ending with a line break, without one, empty), and
  `SystemObservation` / `SystemState` documents as `status` would print them. The Python
  checker embedded in the script, extracted to a scratch file, was run on those:
  - its plan argument is byte for byte `encode_plan`'s, for both tier sets;
  - all 16 file and state checks pass on the three real files, and the removal checks pass
    on the originals;
  - eleven kinds of damage are each reported (a changed name, a removed line, a changed user
    line in front, text after END, a wrong count, two blocks, CRLF, a state hash that
    differs, a tier reported as failed, one byte more after removal, a state directory left).
  - The lookup checks cannot pass in that setting (the names are not in the real hosts
    file) and failed as expected.
  - This found one wrong expectation of mine, now fixed: a file that did not end with a
    line break still does not after the block is written.
- **The refusals of the shell script**, from a copy cut off before the line "From here on the
  script may leave something behind" (the part above it only parses arguments and refuses):
  no `CI`/`UBIQX_SHIELD_SMOKE`, `CI=false`, a wrong tier, a repeated tier, an unknown option,
  a missing value, a missing helper, a helper that does not answer `status`, and a non-root
  user (`runuser -u nobody`) all exit 2; `CI=true` and `UBIQX_SHIELD_SMOKE=1` with a fake
  helper pass that section, detecting the flag form and the bare form.
- **The hosts one-liner of `REMOVAL.md`** (`perl -0pi …`) on copies of the three real files:
  two come back byte for byte, the file without a final line break comes back with one more
  line break; a block whose END line is missing is left untouched; user lines after the
  block stay. The same regular expression, as PowerShell will apply it, was simulated in
  Python on LF and CRLF copies with the same result.
- **The PowerShell script was read, not parsed**: `pwsh` is not installed here and
  downloading it was not permitted. A rough tokenizer of mine found no unbalanced bracket or
  quote. Nothing else about it is verified.

**Not run:** either smoke script (not even their refusal path, apart from the cut-off copy
above); the helper binary; any command of `REMOVAL.md` on a real system file; anything on
macOS or Windows; any CI run (I did not look at the runs my pushes started).

### Deviations from the design

**Release scripts**

- **The product table lives in `scripts/app-version.mjs`**, and `publish-release.mjs` imports
  it. A third module would have been cleaner, but it is not a file of this unit.
- **Three refusals the design does not ask for**, each because the alternative is one
  product's installer reaching the other product's installed apps:
  - a file whose name starts with the other product's published prefix, `productName` or
    `mainBinaryName` (read from that product's `tauri.conf.json`) is refused before any
    request, in both directions;
  - a release whose `latest.json` names the other product is left alone (exit 1, no write).
    The rule is `feed_matches_product`'s: the exact name, and a feed without the field is
    ubiqX's;
  - `--tag` may not be the other product's tag (exit 2).
- **ubiqX's bare call looks for `target/release/bundle/dmg/ubiqX*.dmg`**, not `*.dmg`: both
  shells bundle into the same directories, so "the newest DMG" could be the other product's.
  Every call made today passes `--asset` or `--dmg` and is unchanged. The standalone app has
  no such default: `--product shield` without `--asset` is an error.
- **The release title of the standalone app is `ubiqx-shield (build contínuo)`**: the machine
  id, since a release title is published and the display name is not cleared.

**Smoke scripts**

- **They refuse a machine that already carries the product** (a marked block, a state
  directory, a resolver drop-in), besides the two conditions of the acceptance. The removal
  step would take a real user's protection off.
- **`status` prints the raw observation, not tier states.** Design 9.3 step 3 says "`status`
  reports `Applied` for both". The scripts check what `Applied` means instead: the block
  with its hash and count, the state file's stamp and owners, no failed tier in the report,
  the DNS configuration, and the unprivileged read of all of it.
- **The scripts do not build the helper** (step 1 of 9.3 is the job's). They take
  `--helper` / `-Helper`, default `<target>/debug/ubiqx-shield-helper`.
- **The shell script needs python3.** The checks are one Python program written into the
  work directory by the script (the hash, the JSON, lookups through `getaddrinfo`, plain DNS
  questions over UDP). It keeps the logic in one place that could be tested here.
- **Checks beyond 9.3:** every saved byte still in front of the block; the state directory's
  owner and mode (Windows: its access list, checklist item 6); owner, group and mode of the
  hosts file after removal (Windows: its SDDL); both IPv4 resolvers asked directly for
  `example.com` and for a listed name; and, when the resolver refuses its operator's test
  name `nudity.testcategory.com` directly, the same name through the operating system, which
  shows that the system really asks that resolver. If the resolver does not refuse the test
  name, that check is skipped with a note rather than failed. **That the test name exists
  and behaves so is from memory of Cloudflare's documentation, not verified.**
- **`example.com` is asked for only if it resolved before the run.** The sandbox has no DNS,
  so the hosts run there skips it with a note.
- **`remove` runs whatever `apply` did**, so the helper's own way back is exercised also
  after a failed apply. Only if the restore is then not verified does the script put the
  saved bytes, DNS and state directory back itself. The exit code is 1 either way.
- **`--tiers resolver` alone is accepted** (the design writes `hosts[,resolver]`).
- **PowerShell:** the option is `-Tiers hosts,resolver` (PowerShell's form, not `--tiers`);
  it needs PowerShell 7 (`ProcessStartInfo.ArgumentList`, so that an argument reaches the
  program exactly as the app's `Command` would pass it); `-SkipElevation` and
  `-SkipHostileEnvironment` leave out steps 5 and 6.
- **Step 3's "under 10 s"** is measured from the helper returning to the first successful
  lookup of the canary, which is the first thing done after `apply`. The time since the
  file's last write and the duration of the lookup call are printed next to it.
- **Step 5** first runs the elevation command with the shortest plan. If that already fails,
  the measurement stops with a named failure instead of reporting "0 characters". A plan
  that the helper decodes and then stops at the gate (exit 5) counts as arrived. The run
  fails if a plan of the current budget (1,800) does not arrive.
- **Padding names of step 5** are `pNNN-xxxxxxxxxxxxx.smoke.ubiqx-shield.invalid`: valid for
  `is_valid_hostname`, under a reserved top-level domain, on no never-block rule (checked
  against `never-block.txt`).

**Documents**

- **`PRIVACY.md` and `REMOVAL.md` are in PT-BR only**, like the user documents of `docs/`;
  the checklist is in English, like the design and the hand-offs it is filled in from. An
  English version of the two user documents does not exist.
- **The checklist has more than the 19 items of 11.4**: the gate table, a section for the
  sandbox run, a table for the three CI runs and the Windows measurements, a map from the
  scripts' check lines to the items, and five items the hand-offs raised (20 to 24).
- **`PRIVACY.md` says "cerca de 240 mil"** for the embedded list: `core.meta.json` counts
  239,525 entries, the design says 239,449.

### Contract notes for other units

**U9b (CI, the second half)**

- Version: after `bash scripts/ci-app-version.sh`, call
  `bash scripts/ci-app-version.sh --product shield` in every job that builds or publishes the
  standalone app. Both write the same number and export the same `UBIQX_VERSION`.
- **Both shells bundle into the same `target/release/bundle/<kind>/` directories.** The
  existing rename steps use `ls target/release/bundle/dmg/*.dmg | head -n 1` and the like;
  once the second app is built in the same job those globs match two files. Rename one
  product's files before building the other, or use globs that name the product.
- Publishing: artefact files `ubiqx-shield-<os>-<arch>.<ext>`, uploaded under artefact names
  `ubiqx-shield-<os>-app`. `publish-continuous` collects with `pattern: ubiqX-*-app` and
  `find … -name 'ubiqX-*'`, which are case-sensitive and do not match them; if a glob were
  ever wrong, the script refuses the other product's file. `publish-shield`:
  `node scripts/publish-release.mjs --product shield --strict --version "$UBIQX_VERSION" "${args[@]}"`
  with `find dist -type f -name 'ubiqx-shield-*' ! -name '*.sig'`.
- Smoke job. Linux and macOS:
  `cargo build -p ubiqx-platform --bin ubiqx-shield-helper` then
  `sudo env "PATH=$PATH" CI=true scripts/nofap/system-smoke.sh` (`sudo` drops the
  environment, and root needs `python3` on its `PATH`). Windows, in the runner's elevated
  shell: `pwsh -File scripts/nofap/system-smoke.ps1`. Exit 0 green, 1 red, 2 refused. The
  line to copy into the checklist starts with `system-smoke: GREEN`. The Ubuntu job needs
  the same `libdbus-1-dev pkg-config` as `rust-linux` to build the platform crate.
- `node --test scripts/publish-release.test.mjs` now starts `bash` once (skipped on
  Windows) and reads `crates/ubiqx-core/src/{product,update,nofap}.rs` and both shells'
  `tauri.conf.json`.

**U4 (platform system layer)**

- **The small binary and the elevation command disagree.** `elevate::argv` always re-runs
  `<exe> --shield-helper <verb> …`. `src/bin/ubiqx-shield-helper.rs` (read again at
  `a127af1`) passes `args().skip(1)` to `run`, so it takes the verb directly and answers
  `--shield-helper apply …` with exit 2. The elevation command printed
  by `ubiqx-shield-helper elevate-argv` therefore cannot work for that same binary, and step
  5 of the Windows smoke run fails at its first, shortest plan. The binary should accept and
  skip a leading `--shield-helper`. The smoke scripts accept both forms for their own calls
  (they try `--shield-helper status` first, and the bare form only for a file named
  `ubiqx-shield-helper`).
- What the scripts read from `status` (exit 0, JSON): `hosts.text`, `hosts.error`,
  `resolver.dropin`, `resolver.resolved_active`, `resolver.resolv_conf_uses_resolved`,
  `resolver.interfaces[].{name,active,servers}`, `state.owners.shield.{hosts,resolver}`,
  `state.hosts.{sha256,names}`, `state.report[].{tier,ok,reason}`, `state_error`.
- What they assume of the helper: `apply` exits 0 only when every tier is in place; the
  block is appended after the last byte of a file that had none; `remove --owner all --scope
  all` exits 0 also when nothing is applied and leaves no state directory; the state
  directory is root's and not writable by anyone else, the state file readable by all; on
  macOS every enabled network service gets the four addresses, on Windows every adapter
  that is up, both address families, with the registry value `NameServer` holding them.
- `REMOVAL.md` part 3 was written from the constants of your files (policy paths and value
  names, `defaults` keys, the registry keys), compared again with the committed files at
  `a127af1`. A later change to one of them needs the same change there.

**U7 and U8 (shells)**

- `REMOVAL.md` part 2 relies on both shells calling `helper::main_if_requested` first, so
  that `<app executable> --shield-helper remove --owner all --scope all` works with no window
  and no database. On Windows it starts the executable through `Start-Process -Wait`, because
  a GUI-subsystem program does not hold the console.
- Paths the documents assume for the standalone app, to check once its bundle is active:
  `/Applications/NoFap.app/Contents/MacOS/ubiqx-shield`, `/usr/bin/ubiqx-shield`,
  `%LocalAppData%\NoFap\ubiqx-shield.exe`; data directory `ai.ubiqx.shield`; autostart
  entries named after the product name (`NoFap.plist`, `NoFap.desktop`, the `Run` value
  `NoFap`); logs in `~/Library/Logs/<identifier>` and `<local data>/<identifier>/logs`.
- Tauri names the `.deb` package and the NSIS folder after `productName`. Check that nothing
  installed carries the source name unless it is the display name (checklist item 15).
- The standalone `tauri.conf.json`, once it has `plugins.updater.endpoints`, must point at
  `https://github.com/andreyquadros/ubiquitous-engine/releases/download/shield-continuous/updater.json`:
  the release test compares it.
- Sentences of `PRIVACY.md` that describe code of yours and of U5 that had not landed: the
  log file holds state changes only and the HTTP library's targets are held at `warn`; the
  fallback notification has the title "UBI" and a neutral sentence; "Apagar tudo do NoFap"
  exists only while the module is off; the export goes to the app's `exports/` folder
  without `internal`; the shared lock lives in `<local data>/ubiqx-shield/`; the tracker's
  open block is scrubbed and its screenshots deleted on a hit. If the code differs, the
  document has to change with it.

**The integrator**

- The gate-opening run is unchanged from the design:
  `cargo build -p ubiqx-platform --bin ubiqx-shield-helper && UBIQX_SHIELD_SMOKE=1 scripts/nofap/system-smoke.sh --tiers hosts`.
  Paste the whole output into section 2 of the checklist. In this sandbox `example.com` does
  not resolve, so that check is skipped with a note, and a lookup of a name that is not in
  the hosts file waits for the DNS timeout: the checks after the removal are slow here.
- **A line for `.gitattributes`:** `scripts/**/*.sh text eol=lf`. `system-smoke.sh` holds a
  Python program in a here-document; a CRLF checkout would break both.
- The second half still owes, from section 13: `ci.yml`, `README.md`,
  `docs/ARCHITECTURE.md`, `docs/WINDOWS-LINUX.md`, `docs/LICENSING.md`.

### Open points

- **Neither smoke script has run against the real helper**, and the PowerShell one has never
  been parsed. The first CI run is their first run. Expect a syntax or cmdlet error in the
  PowerShell script before a meaningful result; what it does to the machine before the first
  change is only reading.
- **The macOS path of the shell script is untested**: bash 3.2 compatibility and the
  `networksetup` parsing are by reading only.
- **The helper landed while I wrote the scripts** (`97dbf67` to `a127af1`). I read its verbs,
  options, exit codes and the binary again at `a127af1`; I did not build or run it, and its
  hand-off did not exist yet.
- **`Resolve-DnsName … -DnsOnly -NoHostsFile`, `Get-NetAdapter` and
  `Get-DnsClientServerAddress` under PowerShell 7** on the runner: from memory, not run.
- **Whether Winsock works in a helper started with `SystemRoot` pointing at an empty
  directory** (step 6): unknown. If the helper's lookups fail there, the second cycle of the
  Windows run differs from the first (checklist item 21).
- **An adapter without IPv6** on the Windows runner would fail the script's IPv6 checks and,
  by `is_exactly`, the app's own reading (hand-off U2; checklist item 22).
- **`REMOVAL.md`:** only the hosts one-liner was exercised, on copies. The DNS, policy and
  per-user commands were written from the code and from what the tools document.
- **The uninstall hooks** (`.deb` `prerm`, NSIS) are described in `REMOVAL.md` as the design
  specifies them; they do not exist yet.
- **Providers' terms and Cloudflare's terms are unread** (checklist items 12 and 4).
  `PRIVACY.md` says so in its last section.
- CI: I did not look at any run for my pushes.

## Reviewer

Independent review of U9a, finished after a container restart (integrator decision 16). The
earlier part of the review is in the snapshot commits `865faaf` (release scripts) and
`68aeb66` (shell smoke script); I read both diffs, kept them, and went on from there. My
commits: `04ea4c3`, `55e2e8a` (and the snapshot `8f3d13b`, which carries the first half of
my changes to `system-smoke.ps1`), plus the commit of this section.

**Still true:** every gate entry is closed; neither smoke script has run on a real machine;
`system-smoke.ps1` has never been run or parsed by PowerShell.

### What I ran

| Command | Result |
|---|---|
| `node --test scripts/publish-release.test.mjs` | 51 pass, 0 fail (50 before my test; the engineer's 48 plus 2 from the earlier review) |
| `bash -n scripts/nofap/system-smoke.sh`, `bash -n scripts/ci-app-version.sh` | pass |
| `shellcheck` 0.11.0 (scratch virtualenv) on both shell scripts | clean |
| `cargo build -p ubiqx-platform --bin ubiqx-shield-helper` | built |
| `ubiqx-shield-helper status`, `--shield-helper status`, `elevate-argv` (verbs that only read or print) | exit 0; the JSON has every member the scripts read; a wrong verb exits 2 |
| The Python checker of `system-smoke.sh`, extracted, on files written by the landed `hosts::splice` (six inputs: LF, CRLF, each with and without a final line break, empty, ending in a lone CR) | plan argument identical to `encode_plan` for the three tier sets; 17 file and state checks pass per input; the two lookup checks fail, as they must outside a real hosts file |
| A copy of `system-smoke.sh` with every system path pointed into a scratch directory and every system tool pointed nowhere, against a fake helper | green run exits 0; a helper that fails `apply`, half-writes, fails `remove`, eats a byte, replaces the file with another owner and mode, or leaves the state directory: exit 1 each time, the scratch hosts file byte for byte the saved one, owner and mode back, state directory gone |
| The same copy, interrupted during `apply` by TERM, HUP, and INT (to the shell alone, to its process group, and repeated) | exit 1, saved state back every time |
| The refusals of that copy: no variable, `CI=false`, `CI=1`, a wrong tier, a missing helper, an unknown option, a block, a state directory or a drop-in already present | exit 2 each, nothing written |
| `system-smoke.ps1` through the `tree-sitter-powershell` grammar (scratch virtualenv) | no error node; four seeded mistakes (a brace, a quote, a parenthesis, an `@(`) are each reported. **This is a grammar outside PowerShell, not a PowerShell parse, and says nothing about behaviour** |
| The hosts commands of `REMOVAL.md`: `perl -0pi` on three real spliced files; the Windows regular expression simulated in Python on six, plus a file with a byte-order mark and a byte that is not UTF-8 | see "Fixed" |

**Not run:** `scripts/nofap/system-smoke.sh` and `system-smoke.ps1` themselves (not even
`--help`); the helper's `apply` and `remove`; any `REMOVAL.md` command on a system file;
anything on macOS or Windows; PowerShell. I did not read any CI run.

### Fixed

**Release scripts** (`04ea4c3`)

- **The standalone release would have taken the "Latest" mark of the repository.** GitHub
  serves one release at `/releases/latest`; both rolling releases were created as ordinary
  releases, so the newer one would have answered there. `PRODUCTS.shield.latest` is `false`
  and its create and update requests carry `make_latest: "false"`. ubiqX's requests are
  byte for byte what they were (the test checks that no such member is sent). Nothing in
  this repository reads `/releases/latest`; a person following that address does.
- From the earlier review (`865faaf`): a release that holds the other product's files is
  left alone also when it has no `latest.json` or one that cannot be read
  (`foreignAssets`). Without it a publish to a tag that is nobody's default could have
  replaced `updater.json`, which has no product field.
- I found no other path where one product deletes or replaces the other's asset or feed:
  files are refused by name before any request, the tag of the other product is refused,
  a feed of the other product and files of the other product stop the run before the first
  write, and the only asset names both products upload are the two feeds.

**`system-smoke.ps1`** (`8f3d13b`, `55e2e8a`)

- **It could not have been green.** `Wait-Lookup` returned `, $found` and every caller
  wraps the call in `@(...)`, which gives an array inside an array. Two addresses then read
  as one string that is never the null address (the canary check fails), and no address
  reads as one empty answer (`the canary no longer resolves` fails, the removal is "not
  verified", the fallback restore runs, the run is red). It returns the addresses plainly
  now.
- **The fallback restore did not restore DNS.** The same wrapping in `Restore-Saved` made
  `-ServerAddresses` receive two nested lists, also for an adapter that had no static
  server, so the call failed and the adapter kept the resolver. And where an adapter had
  static IPv4 servers only, setting them left the resolver's IPv6 pair in place. The
  restore now resets each adapter first (both families), then sets the saved static
  servers, reads the configuration back and says whether it is what it was.
- The fallback restore reports an owner or access list of the hosts file that differs from
  the saved one (it does not try to set it).
- **Step 5 runs the elevation command in the app's environment.** The app starts
  `powershell.exe` with four variables and nothing else; the script started it with the
  environment of PowerShell 7, whose `PSModulePath` names modules Windows PowerShell cannot
  load. `Start-Tool` has `-OnlyEnvironment`, and step 5 passes `SystemRoot`, `windir`,
  `PATH` and `PSModulePath` as the back-end builds them. This is also the first evidence for
  U4's open point about the cleaned environment (checklist item 27).
- The helper's output is decoded as UTF-8; no adapter up is an empty list, not a list of one
  null; the Program Files lookup is written without the nested type name.

**`system-smoke.sh`** (earlier review, `68aeb66`; I confirmed each by the simulation above)

- The fallback restore also puts back the owner and mode of the hosts file; HUP is trapped;
  an interrupt during the restore is ignored; the block must use the line ending of the
  file; the DNS comparison waits for systemd-resolved to settle and does not count the
  drop-in's directory, which the helper leaves behind.

**`REMOVAL.md`**

- **The Windows hosts command changed the user's file outside the block**: read and
  written with the default encoding, it dropped a byte-order mark and turned every byte
  that is not UTF-8 into U+FFFD. It reads and writes Latin-1 now, which keeps every byte
  (simulated: byte for byte on the six files and on a file with a mark and such a byte; a
  file without a final line break keeps one extra line break, as the document says).
- The uninstall table said what the `-setup.exe` and `.deb` uninstallers do. They do not
  exist yet. The document now says the table is the design and that the safe order is
  "remove, then uninstall, then check".
- "o pacote do ubiqX se chama `ubiqx`" and "os do ubiqX são os que os instaladores
  publicados usam hoje" were statements nobody checked (ubiqX's shell has no
  `mainBinaryName` and its Cargo package is `ubiqx-desktop`). Each block of part 2 now says
  how to find the executable when the name differs, and the note says the names are
  unconfirmed (checklist item 25).
- The macOS DNS loop gives `sudo` no standard input, so it cannot eat the list the loop
  reads.
- Compared with the code and found right: marker lines, the drop-in path, the five
  `ubiqx-shield.json` paths, Firefox's file and its exact content, the three state
  directories, the `defaults` domain and keys, the five registry keys and six value names
  with their values, exit codes 0, 3 and 4, the keyring service names, the bundle ids.

**`PRIVACY.md`**

- It said the request carries "duas frases fixas" and nothing about the app. The system
  prompt is a paragraph that tells the provider the image comes from "a self-control app
  that an adult installed on their own computer to stay away from pornography". The
  document now says so, in the layer and in "Quem consegue ver o quê".
- It did not mention the app's own lookup of `one.one.one.one` while the resolver tier is
  on (design 2.3). It does now.
- Checked against the code and found right: the caps of the block, the canary and its
  30 minutes, the resolver's addresses, the policy table, 640 pixels, the three default
  limits, the four verdict words, the hour-truncated cost line, what a failed call logs,
  what "Apagar meu histórico" overwrites, the database file names, the shared directory.

**`REAL-MACHINE-CHECKLIST.md`**

- Item 20 follows the fixed helper binary (`412e146`). New items 25 (installed names),
  26 (the hosts file's security descriptor on Windows) and 27 (the elevation environment).
  A note in section 3 that a first run stopping inside a script is a script defect.

### Tests added

- `publish-release.test.mjs`: "the standalone app's release never becomes the repository's
  'Latest' one; ubiqX's requests carry no such member". It fails when the member is left out
  and when it is sent for ubiqX (both tried).
- **None for the PowerShell defects.** There is no PowerShell here and no test in the
  repository runs that script. The CI run is the test.

### Still wrong or unverified

- **`system-smoke.ps1` is unproven.** The array defects above were found by reading, with
  PowerShell's rules from memory (`@(f)` around a function that returns `, $x` nests). I am
  confident of the rule, not of having found every such mistake. Expect the first Windows
  run to need another round.
- **Windows: the hosts file after a write.** `write_atomic` replaces the file through a
  temporary file and a rename and keeps owner and mode only on Unix. On Windows the new file
  takes the directory's inherited access list and the creating account as owner. The script
  compares the security descriptor after the removal, so the first Windows run may be red on
  `hosts: owner and access list are what they were`. That would be a finding about
  `sysblock/sys.rs` (U4), not about the script (checklist item 26).
- **macOS: the helper puts DNS back to automatic, not to what it was.** A network service
  that had static servers before the run fails `DNS: the configuration is what it was`, and
  the fallback restore then sets them again. Runners have automatic DNS; a person's machine
  may not. This is the design (9.3 step 4: "DNS is automatic again").
- **The elevation command in the four-variable environment** may not start at all on
  Windows. If the shortest plan fails in step 5, that is the answer to U4's open point, and
  `ARG_BUDGET_WINDOWS` stays unmeasured.
- `nudity.testcategory.com` is still from memory; the check is skipped with a note if the
  resolver does not refuse it.
- `REMOVAL.md`: the installed names (item 25), every command but the two hosts commands, and
  the uninstall hooks, which do not exist.
- `PRIVACY.md` still describes U5, U7 and U8 from the design (log levels in the shells, the
  fallback notification, "Apagar tudo do NoFap", the export folder, the shared lock, the
  tracker scrub). It has to be read again against that code.
- The smoke scripts refuse a machine with a block, a state directory or a drop-in. They do
  not look for the product's browser policy files; `remove --owner all` deletes those too.
  A machine that has them has a state directory, so I left it.

### For the units that follow

- **U9b:** nothing in the interface of the scripts changed. The Windows job needs
  PowerShell 7 (`pwsh -File scripts/nofap/system-smoke.ps1`) in the runner's elevated shell,
  with `CI=true` (set by GitHub). `.gitattributes` now keeps `*.sh` and `*.ps1` LF
  (integrator, `f284172` and `8f3d13b`).
- **U4:** checklist items 26 and 27 are about your code; the Windows run will say.
- **U7, U8:** if the uninstall hooks are cut, `REMOVAL.md` part 1 needs only its table
  changed; the paragraph under it already tells the user not to rely on them. If the
  installed executable of ubiqX is not `ubiqX` / `ubiqx` (item 25), change part 2.
- **The integrator:** the gate-opening command is unchanged. The simulation above is the
  nearest thing to a rehearsal: the checker passes on real blocks, and the restore held
  under every failure I could stage.
