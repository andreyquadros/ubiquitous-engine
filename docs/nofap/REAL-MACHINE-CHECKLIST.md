# Real-machine checklist

What could not be tested in the build sandbox, item by item, with a place for the evidence.
It is the list of `docs/nofap/DESIGN.md` 11.4, complete, plus the points the hand-offs of the
units added (`docs/nofap/handoff/`). An item marked **CI** is covered by the job
`nofap-system-smoke` (design 9.3; scripts `scripts/nofap/system-smoke.sh` and
`system-smoke.ps1`). An item marked **person** needs somebody at a real machine, and blocks
the claim it concerns until it is filled in.

Rules for filling it in:

- Evidence is a run URL, pasted command output, or the name of a screenshot kept next to
  this file. "Looked fine" is not evidence.
- Write the date, the operating system with its version and language, the kind of account
  (administrator or standard), and the commit of the build that was tested.
- No personal data: no real name, e-mail, host name, serial number or home directory in what
  is pasted. Replace them (`ana`, `ana@example.com`).
- A failed check is recorded as failed, with its output. It is not deleted.
- When an item is done, change `[ ]` to `[x]` in its heading line and fill in its block.

## 1. Which system tiers are open

The gate (`ubiqx_nofap::gate::TIER_GATE`, design 1.8) lands with all nine entries closed.
**A build in which every entry is closed blocks nothing at DNS level**: only the in-app guard
and the AI check work. An entry is opened in its own small commit, which touches only
`gate.rs`, its test and this table, after the evidence named here exists.

| OS | Tier | Open? | Opened by (commit) | Evidence |
|---|---|---|---|---|
| Linux | hosts | no | | section 2, or the Ubuntu run of section 3 |
| Linux | resolver | no | | Ubuntu run of section 3 |
| Linux | policies | no | | item 9 |
| macOS | hosts | no | | macOS run of section 3 |
| macOS | resolver | no | | macOS run of section 3 |
| macOS | policies | no | | item 9 |
| Windows | hosts | no | | Windows run of section 3 |
| Windows | resolver | no | | Windows run of section 3; item 6 (execution policy) |
| Windows | policies | no | | item 9 |

Testers can open tiers on their own machine without a new build:
`UBIQX_SHIELD_UNVERIFIED_TIERS=hosts,resolver,policies` in the environment of the app. The
page then labels those tiers "em teste".

## 2. The sandbox run that opens `(Linux, hosts)`

The build sandbox runs as root with a regular `/etc/hosts` and `hosts: files dns`, so the
hosts tier can be exercised there before any push (design 1.8, section 13 "Gate-opening
step"). It is the single real-system run of the project and belongs to the integrator.

```sh
cargo build -p ubiqx-platform --bin ubiqx-shield-helper
UBIQX_SHIELD_SMOKE=1 scripts/nofap/system-smoke.sh --tiers hosts
```

The run is green when the last line starts with `system-smoke: GREEN os=linux tiers=hosts`
and the exit code is 0. The script saves the hosts file first and puts it back itself if
anything goes wrong; it refuses to start on a machine that already carries a block, a state
directory or a resolver drop-in of the product.

- [ ] Run done
  - Date:
  - Commit:
  - Exit code:
  - Output (paste all of it, from `system-smoke: os=linux` to the `GREEN` or `RED` line):

```text
(paste here)
```

## 3. The CI smoke runs

One run per operating system of the job `nofap-system-smoke`, on a commit of this branch.
The job is `continue-on-error` until its first green run, so a red run does not fail the
workflow: read the job's own result, not the workflow's.

| Runner | Tiers | Result | Run URL | Commit | `GREEN` line of the log |
|---|---|---|---|---|---|
| `ubuntu-latest` | hosts, resolver | | | | |
| `macos-latest` | hosts, resolver | | | | |
| `windows-latest` | hosts, resolver | | | | |

Numbers the Windows run reports, and the constants they decide (design 2.1, 2.2):

| Measurement | Value from the run | Constant | Value in the code | Changed to |
|---|---|---|---|---|
| Names in the block | | `hosts::CAP_WINDOWS` | 8,000 | |
| Seconds from the helper returning to the first successful lookup of the canary | | (limit: 10 s) | | |
| Longest `--plan-b64` value that reached the helper through the elevation command | | `plan::ARG_BUDGET_WINDOWS` | 1,800 | |

What the scripts check, so that a green run can be read against the items below:

| Check in the log | Items |
|---|---|
| `hosts: every byte of the saved file is still in front`, `the declared hash is the hash of the lines between the markers`, `the declared count is the number of names`, `state: records the hash and the count` | 1 |
| `the canary resolves to the null address through the operating system`, `a name of the list (…) resolves to the null address`, `example.com still resolves` | 1 |
| `hosts: byte for byte the saved file`, `owner, group and mode are what they were` (Windows: `owner and access list`), `the state directory is gone`, `the canary no longer resolves` | 1 |
| `the first lookup after the write comes within 10 seconds` (Windows) | 2 |
| `DNS: … uses exactly the resolver's four addresses` (macOS), `DNS: adapter … uses the resolver's two IPv4 / IPv6 addresses` (Windows), `DNS: the drop-in holds exactly the expected four lines` and `systemd-resolved's global servers are the resolver's four addresses` (Linux), `DNS: the configuration is what it was before the run` | 3 |
| `the resolver 1.1.1.3 answers for example.com`, `the resolver 1.1.1.3 refuses a listed name`, the same for `1.0.0.3`, `the operating system resolves through the family resolver` | 4 |
| `registry: Tcpip NameServer of … holds the IPv4 pair`, `registry: Tcpip6 NameServer of … holds the IPv6 pair`, `… cannot be written by a standard user`, `… can be read by a standard user` (Windows) | 6 |
| `ARG_BUDGET evidence: the longest --plan-b64 value that reached the helper intact has … characters` (Windows) | 18 |
| `nothing was created in the scratch directory the variables pointed at` (Windows, hostile environment) | design 2.5, 9.3 step 6 |

## 4. The items of design 11.4

### [ ] 1. Hosts block applies, is honoured, and is removed byte-exactly on Windows, macOS, Ubuntu

**CI.** Sections 2 and 3.

- Evidence: the three run URLs of section 3 (and the sandbox output of section 2 for Linux).

### [ ] 2. Windows DNS Client with 8,000 names at six per line: time to first lookup, CPU

**CI** for the time; **person** for CPU and Defender.

Person, on Windows 11 with Defender on and the hosts tier applied:

1. Open Task Manager, Details, and find the `svchost.exe` that hosts the service `Dnscache`
   (`tasklist /svc /fi "services eq Dnscache"` gives its PID).
2. Browse ordinary sites for five minutes. Note the CPU of that process at rest and while
   pages load.
3. Open Windows Security, Protection history: no `HostsFileHijack` or similar entry may
   appear for the hosts file.

- Evidence (time): the Windows row of the measurements table in section 3.
- Evidence (CPU, Defender):
  - Date, Windows version and language:
  - CPU at rest / while browsing:
  - Protection history (paste or screenshot name):

### [ ] 3. `networksetup`, `Set-DnsClientServerAddress` / `-ResetServerAddresses`, resolved drop-in set and reset

**CI.** Section 3.

- Evidence: the three run URLs of section 3.

### [ ] 4. Cloudflare for Families addresses answer and refuse a listed name; terms and attribution re-read

**CI** for the answers; **person** for the terms.

Person: read Cloudflare's current terms and privacy statement for 1.1.1.1 for Families, and
check three things against the app: that a consumer app may point its users at the service,
what attribution it asks for (the app credits "DNS filtrado por Cloudflare for Families"
next to the toggle and in About), and what the service says it logs (PRIVACY.md says "ela
trata esses dados pelos termos dela").

- Evidence (answers): section 3.
- Evidence (terms):
  - Date read, and the documents read (title and date of each):
  - Anything the app or PRIVACY.md has to change:

### [ ] 5. Elevation prompts

**Person, each OS.** With a build installed the way a user installs it, switch a system tier
on from the page, and once more decline the prompt.

| OS | What to check | Result |
|---|---|---|
| macOS, ad-hoc signed app in `/Applications` | the prompt appears; its wording (it names `osascript`); Cancel gives "not applied" on the page (AppleScript error -128); with the password, the helper runs as root from inside the `.app` and the tier reads "ativa" | |
| Windows, unsigned per-user install (`-setup.exe`) | the UAC prompt appears through `Start-Process -Verb RunAs`; it says "unknown publisher"; "No" gives "not applied" (exit code 1223); "Yes" applies | |
| Linux, `.deb` on Ubuntu with a desktop session | `pkexec` shows the product's sentence (the polkit action file is installed); Cancel gives "not applied" (exit 126); with the password the tier applies | |
| Linux, AppImage | no prompt is tried; the page says the system tiers are unavailable in this format (`appimage`) | |
| Linux, no polkit agent (a bare window manager, or over SSH) | the page says so (`no_polkit_agent`), nothing hangs | |

- Evidence, per row (date, OS version and language, account type, screenshot name):

What the prompts say, stated here as in PRIVACY.md (design 2.5): the macOS prompt names
`osascript`; the Windows prompt says "unknown publisher" for an unsigned build; the
executable that gets elevated is the app itself, and wherever the user's account can write
to it (a per-user Windows install, an app in `~/Applications` or dragged into `/Applications`
by an administrator account, a development build) it is only as trustworthy as that account.
The helper adds no way to run anything else: its verbs are closed and it executes only the
system tools it names by absolute path.

### [ ] 6. Windows: registry path of static DNS, the state directory, PowerShell execution policy

**CI** for the first two; **person** for the execution policy (the runner's policy is not
the default of Windows Home).

Person, on Windows 11 Home with nothing changed (`Get-ExecutionPolicy -List` shows
`Undefined` everywhere, which means `Restricted`):

1. Apply the resolver tier from the app.
2. The tier must read "ativa", and `Get-DnsClientServerAddress` must show the four addresses
   on every adapter that is up.
3. If the tier fails instead, record the helper's exit code and the per-tier report in
   `C:\Program Files\ubiqx-shield\state.json`. The resolver tier then stays closed on Windows
   until the back-end sets DNS with `netsh.exe` (design 2.5); `-ExecutionPolicy Bypass` is not
   an option.

- Evidence (registry, state directory): section 3, Windows.
- Evidence (execution policy):
  - Date, Windows edition and version:
  - Output of `Get-ExecutionPolicy -List`:
  - Result of step 2 or 3:

### [ ] 7. PT-BR macOS and Windows: parsers unaffected

**Person.** On a macOS and a Windows whose display language is Portuguese (Brazil):

1. Run the helper's `status` verb before and after applying the hosts and resolver tiers.
2. The page must show the same states as on an English system: "ativa" after applying,
   "não aplicada" after removing, the DNS servers listed in the preview.
3. On macOS, also with a network service whose name has an accent or a space.

- Evidence (date, OS version, language, the two `status` outputs with names replaced):

### [ ] 8. IPv4 line and AAAA; DoH in Chrome and Firefox; Safari with iCloud Private Relay

**Person.** With the hosts tier applied and the browser restarted:

| Question | How | Result |
|---|---|---|
| Does a `0.0.0.0` line alone suppress the AAAA lookup? | informative only: the block always writes both lines | |
| Chrome with "Use secure DNS" on its default (automatic) | open a listed site: blocked or not? | |
| Chrome with a named secure-DNS provider chosen | the same | |
| Firefox with DNS over HTTPS at its default | the same | |
| Firefox with DNS over HTTPS "Max protection" | the same | |
| Safari with iCloud Private Relay on | a listed site with only the hosts tier; then with only the resolver tier | |

What the answers decide: whether the page has to say that the system list does not hold in a
browser until the policies tier is on, and whether DoH-endpoint names go into the block
(design 12).

- Evidence (date, OS and browser versions, one line per row):

### [ ] 9. Browser policies

**Person.** This item opens the `policies` entries of section 1; nothing else does.

| Where | What to check | Result |
|---|---|---|
| Windows 11 Home, not joined to anything | after applying: `chrome://policy`, `edge://policy`, `brave://policy` list the values with status OK; `about:policies` in Firefox lists `DNSOverHTTPS`; a DoH lookup is not used (the secure-DNS switch is greyed out) | |
| Ubuntu, Chrome `.deb`, Chromium snap, Firefox snap, Firefox `.deb` | the same pages; the Firefox snap reads `/etc/firefox/policies/policies.json`; the Chromium snap reads `/etc/chromium-browser/policies/managed/` | |
| Ubuntu, a Flatpak browser | the page says this browser is not covered (`flatpak`) | |
| macOS, Firefox | `about:policies` lists `DNSOverHTTPS` after `defaults write`; Chrome, Edge and Brave are untouched and the page says "No macOS, Chrome, Edge e Brave ficam sem esta camada" | |
| every system | the wording of the "managed" banner, to compare with the disclosure in the preview | |
| every system | a machine that already has a different value for one of the keys: nothing is written for that browser and the page shows the conflict | |
| every system | after removal: the values and files are gone, the banner is gone after a restart of the browser | |

- Evidence (date, OS and browser versions, screenshot names):

### [ ] 10. Captive portal with the filter set; "Restaurar minha rede" fixes it

**Person.** With the resolver tier applied, join a network with a captive portal (a hotel or
café network, or a phone hotspot with a portal).

1. Does the portal page open? If not, does the card "Esta rede não alcança o DNS filtrado"
   or the issue on the page appear within about six minutes?
2. "Restaurar minha rede" must bring the portal back at once, with one administrator prompt
   and no waiting time.
3. "Religar" afterwards sets the resolver again.

- Evidence (date, OS, kind of network, what happened at each step):

### [ ] 11. Vision on real providers

**Person, with consented test images.** A fixed set of images the tester has the right to
use, opened full-window in a browser: explicit, suggestive, and ordinary or borderline ones
(medical anatomy, classical art, beach and sport photographs, a text page, a dark frame).
Per provider and vision model:

| Provider, model | Explicit: caught / total | Refusals | False positives on ordinary / borderline | Median latency | Cost per check at 640 px |
|---|---|---|---|---|---|
| Anthropic, | | | | | |
| OpenAI, | | | | | |
| xAI, | | | | | |
| IA do Ubi (ubiqX only) | | | | | |

Also record (hand-off U3):

- how each provider declines an image it will not look at (a refusal stop reason, an HTTP
  400 with which error code, something else), since the three codes the client maps to a
  quiet refusal were not observed against a live API;
- whether a reasoning model (an o-series model, `grok-4`) answers within the 256 output
  tokens of the retry, or the check ends as an error and vision goes into backoff;
- whether the managed route accepts the request as the client sends it for `ubi-fast`.

- Evidence (date, the models tried, the table, the log lines of any failed check):

### [ ] 12. Each AI provider's policy on possibly explicit screenshots

**Person (owner).** This blocks switching vision on for users, not building it.

For Anthropic, OpenAI and xAI, read the current usage policy, the API data-retention terms
and what they say their safety systems do with material they judge illegal. Then adjust the
consent text (`vision.consent`, `vision.managed`) and PRIVACY.md to what they say.

| Provider | Documents read (title, date) | May an app send such screenshots for classification? | Retention | What happens to material judged illegal | Change needed in the consent |
|---|---|---|---|---|---|
| Anthropic | | | | | |
| OpenAI | | | | | |
| xAI | | | | | |

- Evidence (date read, who decided):

### [ ] 13. The card window, `hide_others`, the Windows address-bar fix

**Person, each OS.**

| What | How | Result |
|---|---|---|
| The card shows on every workspace or virtual desktop, on top, without taking keyboard focus | type in another app while a block happens; the typing must go on in that app | |
| The card closes by itself after its time, and not once the user opened support | | |
| `hide_others` after a vision hit in a media player | the player is hidden and comes back from the Dock or taskbar; nothing is quit | |
| Windows: a listed page is open, click the address bar, wait | the tab is still closed at the next pass (the last committed URL is used) | |
| Windows: type the name of a listed site in the address bar of an ordinary page without pressing Enter | nothing is closed | |

- Evidence (date, OS version, one line per row):

### [ ] 14. macOS: Screen Recording and Automation prompts raised by the module without the tracker

**Person.** In the standalone app (which has no tracker) and in ubiqX with tracking paused:

1. Enabling the guard with a supported browser in front raises the Automation prompt for
   that browser; declining shows `limited` with the button that opens the permission pane.
2. Enabling the AI check raises the Screen Recording prompt; declining shows
   "Verificação por IA: desligada" with the reason and the button.
3. After an update of an ad-hoc signed build both permissions are gone: the page must show
   the limited states with their buttons, never a silent gap.

- Evidence (date, macOS version, build before and after the update, screenshot names):

### [ ] 15. Uninstall hooks, and two products side by side

**Person.**

| What | How | Result |
|---|---|---|
| `.deb` removal | apply the hosts tier, `sudo apt remove <package>`: the block, the state directory and the drop-in are gone | |
| `.deb` upgrade | apply, install a newer `.deb` over it: **nothing is removed** (`prerm` acts only on `remove`) | |
| NSIS uninstall | apply, uninstall from Settings: one UAC prompt, then the block and the state directory are gone | |
| NSIS uninstall, prompt declined | the uninstall finishes and its last page points to `REMOVAL.md` | |
| In-app update on Windows | apply, let the app update itself: **nothing is removed** (`$UpdateMode`) | |
| MSI, macOS `.app`, AppImage | no hook: the commands of `REMOVAL.md` part 2 work with the installed executable, and part 3 works after the app is gone | |
| Both products installed | apply the hosts tier in both; remove in one: the block stays, the state file lists the other owner; remove in the other: everything is gone | |
| Both products running | only one runs the guard; the other shows "Proteção ativa pelo outro app" and takes over within about 90 seconds of the first one quitting | |
| Installer names | no installed file, package name, menu entry description or uninstall entry carries the source name of the module unless it is the display name itself (design 0.1) | |

- Evidence (date, OS versions, package versions, the outputs of the helper's `status` verb):

### [ ] 16. Every phone number and URL in `resources.json`

**Person.** The release-mode test `resources_verified` fails until each entry of
`crates/ubiqx-nofap/data/resources.json` is marked verified, so a release build cannot ship
unverified help resources.

| Resource id | Checked against (the organisation's own page, date) | Number or URL correct? | Hours and notes correct? |
|---|---|---|---|
| | | | |

- Evidence (date, who called or opened each one):

### [ ] 17. INPI and USPTO searches for the display name

**Owner.** The display name is `NoFap` (integrator decision 1). NOFAP is reported as a
registered US mark; Brazil is unknown. The standalone product is built in CI and **not
published** (`NOFAP_PUBLISH` is not `true`) until this is done.

| Register | Classes searched | Date | Result |
|---|---|---|---|
| INPI | 9, 42, 44, 45 | | |
| USPTO | 9, 42, 44, 45 | | |

- Decision (keep the name, or change `DISPLAY_NAME` and the two lines of the standalone
  shell's `tauri.conf.json`), and who took it:

### [ ] 18. Longest `--plan-b64` argument that survives `Start-Process -Verb RunAs`

**CI.** Step 5 of the Windows script: the app's real elevation command, printed by the
helper's `elevate-argv` verb, is run with plans of about 1,800, 4,000, 12,000 and 24,000
characters.

- Evidence: the `ARG_BUDGET evidence` line of the Windows run, copied into the measurements
  table of section 3.

### [ ] 19. The log file of each shell after a day of use

**Person.** The capture test covers the Rust side; this is the real file.

1. Use each app for a day with the AI check on, including a few blocks and a few checks.
2. Open the log file (`~/Library/Logs/<identifier>/` on macOS,
   `%LocalAppData%\<identifier>\logs\` on Windows, `~/.local/share/<identifier>/logs/` on
   Linux; the identifiers are `ai.ubiqx.app` and `ai.ubiqx.shield`).
3. It must hold no line per check, no host name of a site that was visited or blocked, no
   window title and no verdict word. Count the lines and compare with the number of checks
   the page shows for the day: the first must not grow with the second.
4. Raise the log level to `debug` and repeat for an hour: the HTTP library's connection
   lines (`hyper_util`, `reqwest`) must not appear either (hand-off U3: the shells have to
   hold those targets at `warn`).

- Evidence (date, product, number of checks, number of log lines, `grep` commands used and
  their output):

## 5. Points the hand-offs added

Not in design 11.4. Each names the hand-off it comes from.

### [ ] 20. The helper binary and the elevation command (this unit, U9a)

The elevation command always re-runs the executable as `<exe> --shield-helper <verb> …`. The
small binary `ubiqx-shield-helper`, as first written, takes the verb directly and refuses
that form, so step 5 of the Windows smoke run cannot work with it until it accepts the flag
in front. **CI:** the line `the elevation command works at all with the shortest plan` of the
Windows run.

- Evidence: the Windows run of section 3.

### [ ] 21. A hostile environment and name lookups inside the helper (this unit, U9a)

Step 6 of the Windows script starts the helper with `SystemRoot` and `windir` pointing at an
empty directory. Whether Winsock initialises in that process, and so whether the helper's
own probe lookups work there, is not known. **CI:** the second cycle of the Windows run; if
its checks differ from the first cycle, record which.

- Evidence: the Windows run of section 3.

### [ ] 22. An adapter without IPv6 (hand-off U2)

`resolvers::is_exactly` wants the IPv6 pair too. If Windows reports an adapter whose IPv6 is
unbound with only the two IPv4 servers, the resolver tier reads "changed" there. **CI** on
the runner's adapters; **person** on a machine with IPv6 switched off on the active adapter.

- Evidence:

### [ ] 23. The scrubbed block in ubiqX's database file (hand-off U3)

After a hit, the open activity block is rewritten as private with a plain update, so the old
title and address can stay in freed bytes of the database file. **Person:** after a block in
ubiqX, close the app and search the database file and its write-ahead log for the host name
(`strings ubiqx.db | grep <host>`). Whatever the result, PRIVACY.md must match it.

- Evidence (date, result of the search):

### [ ] 24. "Apagar meu histórico" while the CLI has the database open (hand-off U3)

The erase overwrites what it frees and folds the write-ahead log back, except while another
process holds the database. **Person:** run the erase with `ubiqx` (the CLI) open, close
both, and search the files for a snooze or a counter that was erased.

- Evidence (date, result of the search):

## 6. Still open, and who decides

Carried from the design's "Still open" list, for whoever reads this file first.

| Point | Decided by | Decision |
|---|---|---|
| Should quitting the app wait the delay while the module is on? (question 6) | owner | |
| The name: neither "NoFap" nor "Escudo" is cleared (item 17) | owner | |
| The elevated executable is the app itself; bundling the small helper and signing the builds is deferred (design 12) | owner | |
| The per-key cap front-loads the AI check on a host with a standing signal (hand-off U2) | integrator | |
| `PRAGMA secure_delete` for every table of both products (hand-off U3, item 23) | integrator | |
