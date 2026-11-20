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

The gate (`ubiqx_nofap::gate::TIER_GATE`, design 1.8) landed with all nine entries closed.
**A build in which every entry is closed blocks nothing at DNS level**: only the in-app guard
and the AI check work. An entry is opened in its own small commit, which touches only
`gate.rs`, its test and this file, after the evidence named here exists. The table below is
the gate as it stands; the list `OPEN` in the tests of `gate.rs` names the same entries.

| OS | Tier | Open? | Opened by (commit subject) | Evidence |
|---|---|---|---|---|
| Linux | hosts | **yes** | "feat(nofap): the gate opens (Linux, hosts)" | section 2 (the sandbox run, green); section 3, run 1, `ubuntu-latest` (green) |
| Linux | resolver | **yes** | "feat(nofap): the gate opens (Linux, resolver)" | section 3, run 1, `ubuntu-latest` (green) |
| Linux | policies | no | | item 9 |
| macOS | hosts | **yes** | "feat(nofap): the gate opens (macOS, hosts) and (macOS, resolver)" | section 3, run 1, `macos-latest` (green) |
| macOS | resolver | **yes** | "feat(nofap): the gate opens (macOS, hosts) and (macOS, resolver)" | section 3, run 1, `macos-latest` (green) |
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

- [x] Run done
  - Date: 2026-11-20, 01:48 UTC
  - Commit: `af28c33` (the helper built from it: `cargo build -p ubiqx-platform --bin
    ubiqx-shield-helper`, debug profile; its SHA-256 is in the last line of the output)
  - Machine: the build sandbox, Linux, root, `/etc/hosts` a regular file (81 bytes, `0:0`,
    mode 644), `hosts: files dns` in `/etc/nsswitch.conf`, no systemd-resolved, no DNS
  - Command: `UBIQX_SHIELD_SMOKE=1 scripts/nofap/system-smoke.sh --tiers hosts`
  - Exit code: 0
  - Checked outside the script: `sha256sum /etc/hosts` before and after the run is
    `d343af6b2bafa7b0d5bce950b58eb3cede0ebbfd7aa6619a0b184b8535abd2b5`, and `cmp` with a copy
    taken before the run reports no difference; owner, group and mode are unchanged;
    `/etc/resolv.conf` has the same SHA-256 as before; `/var/lib/ubiqx-shield` does not
    exist; `find /etc /var/lib -newer <stamp>` lists only `/etc`, `/etc/hosts` and
    `/var/lib` (the hosts file was replaced twice, the state directory created and deleted)
  - What this run cannot show: `example.com` does not resolve in the sandbox, so "another
    name still resolves" was skipped with a note; the Ubuntu run of section 3 covers it
  - Output (all of it):

```text
system-smoke: os=linux tiers=hosts helper=/home/user/ubiquitous-engine/target/debug/ubiqx-shield-helper (flag form)
system-smoke: saved: /etc/hosts (81 bytes), the DNS configuration and the status
system-smoke: apply
system-smoke: checking what was applied
  ok    status: the state file is readable
  ok    state: no tier is reported as failed
  ok    the state directory exists and is a directory, not a link
  ok    the state directory is root's and nobody else can write to it
  ok    the state file is readable by every account
  ok    hosts: every byte of the saved file is still in front, unchanged
  ok    hosts: the block uses the line ending of the file (LF)
  ok    hosts: the block starts right after the saved bytes, on a line of its own
  ok    hosts: the END line closes the file, ended as the saved file was
  note  hosts: the block declares sha256=03d06771ab2c01c4ff19645e8f532e8fc9e7fadfee6e8fa2db044a8be68439b4 names=20000
  ok    hosts: the declared hash is the hash of the lines between the markers
  ok    hosts: the declared count is the number of names (20000)
  ok    hosts: between 2 and 20000 names
  ok    hosts: the canary is the first name
  ok    state: records the hash and the count of the block
  ok    status: the unprivileged read sees the block
  ok    status: no hosts error
  ok    state: the owner 'shield' wants the hosts tier
  ok    the canary resolves to the null address through the operating system
  ok    a name of the list (xhamster.com) resolves to the null address
  note  example.com did not resolve on this machine before the run, so it is not asked for with the tiers applied
system-smoke: remove --owner all --scope all
system-smoke: checking the restore
  ok    hosts: byte for byte the saved file (81 bytes)
  ok    hosts: owner, group and mode are what they were
  ok    the state directory is gone
  ok    status: no state file
  ok    status: the unprivileged read sees no block
  ok    DNS: the configuration is what it was before the run
  ok    the canary no longer resolves
  ok    the listed name (xhamster.com) is no longer pinned to the null address
  note  example.com did not resolve on this machine before the run, so it is not asked for after the removal
system-smoke: GREEN os=linux tiers=hosts commit=af28c33 helper_sha256=f0fbb8205d1429c1788f0d3b14e40e5eb4baf0661d937ec9238adf26a53780dd
```

## 3. The CI smoke runs

The workflow `.github/workflows/nofap-smoke.yml` (job `nofap-system-smoke`, one per
operating system) runs on every push to the branch that changes the platform crate, the
`ubiqx-nofap` crate, `scripts/nofap/` or the workflow. It builds the helper binary and runs
the smoke script as administrator, tiers hosts and resolver. It is a workflow of its own,
with its own concurrency group, so it blocks nothing else and is not cancelled with the main
CI; a red job is a red job (there is no `continue-on-error`). After the script, a step of
the job itself compares the hosts file with a copy it took before, whatever the script said.

Neither script had run against a real helper when it was written, and the Windows one had
never been parsed by PowerShell (only by a grammar outside PowerShell). A run that stops
with a PowerShell or Python error naming a line of the script is a defect of the script, not
evidence about the helper: it is logged below as red, fixed, and run again.

**The latest run per system** (run URLs are
`https://github.com/andreyquadros/ubiquitous-engine/actions/runs/<run id>`, a job's log is at
`.../runs/<run id>/job/<job id>`):

| Runner | Tiers | Result | Run, job | Commit | `GREEN` line of the log |
|---|---|---|---|---|---|
| `ubuntu-latest` (Ubuntu 26.04.1, systemd-resolved active, `/etc/resolv.conf` the stub) | hosts, resolver | **green** | run 4: `44662252304`, job `136043022738` | `3a99960` | `system-smoke: GREEN os=linux tiers=hosts,resolver commit=3a99960 helper_sha256=cb0eeafe8872bde8fe821ea9d7d0c507291e8ab190923fc320f03b44b623f1ea` |
| `macos-latest` (macOS 26.7.2, arm64, one network service, "Ethernet") | hosts, resolver | **green** | run 4: `44662252304`, job `136043022962` | `3a99960` | `system-smoke: GREEN os=macos tiers=hosts,resolver commit=3a99960 helper_sha256=9e7a3966fc3ac5302bb62b24eb55d4289f1c07d4c74a4daeff6156380bc91097` |
| `windows-latest` (Windows Server 2025 Datacenter, PowerShell 7.6.9, execution policy `RemoteSigned` for the machine) | hosts, resolver | **red** | run 4: `44662252304`, job `136043022408` | `3a99960` | none: see the log of runs |

### Log of every run

One row per run and system, oldest first. "Changed afterwards" is what the result led to.

| Run | Commit | System | Outcome | What the log shows | Changed afterwards |
|---|---|---|---|---|---|
| 1, `44656118455` | `af28c33` | `ubuntu-latest` | green | Every check `ok`, both tiers. Applied: the block of 20,000 names with the declared hash and count, the canary and `xhamster.com` resolve to the null address, the drop-in holds the four lines, systemd-resolved's global servers are the resolver's four addresses, `1.1.1.3` and `1.0.0.3` answer for `example.com` and refuse `xhamster.com`, the operating system resolves through the family resolver (`nudity.testcategory.com` is refused), `example.com` still resolves. Removed: the hosts file byte for byte the saved one (273 bytes) with its owner and mode, no state directory, DNS as before, the canary no longer resolves. One note: `/etc/systemd/resolved.conf.d` did not exist before and stays, empty. The job's own comparison of the hosts file: identical. | nothing |
| 1, `44656118455` | `af28c33` | `macos-latest` | green | Every check `ok`, both tiers; the same list as on Ubuntu, with "'Ethernet' uses exactly the resolver's four addresses" and "every active interface reads as the resolver's set" for DNS. The hosts file (358 bytes) is byte for byte the saved one afterwards, owner and mode as before, DNS as before. The job's own comparison: identical. | nothing |
| 1, `44656118455` | `af28c33` | `windows-latest` | red | The script stopped in its first step, before anything was changed: `Get-DnsClientServerAddress -InterfaceIndex 9 -AddressFamily IPv4` found no object. The runner has three adapters that are up, and one of them ("Ethernet 4", a Mellanox ConnectX-4 Lx virtual adapter, the accelerated-networking twin of "Ethernet 3") has no DNS client configuration at all. The helper's own scripts ask the same question per adapter with errors made terminating, so its full read would have failed (`unreadable`) and its set script would have stopped at that adapter. A real machine has such adapters too (a network card bound to a Hyper-V switch). The job's own comparison of the hosts file: identical; SDDL of the hosts file before: `O:SYG:SYD:AI(A;ID;FA;;;SY)(A;ID;FA;;;BA)(A;ID;0x1200a9;;;BU)(A;ID;0x1200a9;;;AC)(A;ID;0x1200a9;;;S-1-15-2-2)`. | The helper reads and sets DNS only on adapters that are up **and** have a DNS client configuration (`windows.rs`, with tests); the script does the same and names the adapters it leaves out. |

| 2, `44658216620` | `babb415` | `ubuntu-latest` | green | As run 1, with the gate entries of Linux and macOS open in the build. | nothing |
| 2, `44658216620` | `babb415` | `macos-latest` | green | As run 1. | nothing |
| 2, `44658216620` | `babb415` | `windows-latest` | red, 26 checks | The script ran to its end for the first time: three cycles (this shell's environment, the elevation command, the hostile environment), each applied and removed by the helper, exit 0 every time. **What held:** the helper's `status` verb works in the four-variable environment (Windows PowerShell started, the `DnsClient` and `NetAdapter` modules loaded under the runner's execution policy) and lists exactly the two adapters with a DNS configuration; the block of 8,000 names with its hash and count behind the saved bytes; the state directory under Program Files readable and not writable by a standard user; both adapters carry the resolver's two IPv4 and two IPv6 addresses (cmdlets and the helper's own read); both resolvers answer and refuse a listed name; after every removal the hosts file is byte for byte the saved one (919 bytes), DNS and the registry are what they were, no state directory; the elevation command ran in the four-variable environment and its helper exited 0 with the block written; nothing was created in the scratch directory of the hostile environment. **What failed:** (a) the hosts file had another owner and group after the first write and kept them after the removal: `O:BAG:S-1-5-21-…-513` instead of `O:SYG:SY`, the access list unchanged (item 26); (b) with the block in place the canary, `xhamster.com` and, with the resolver set, `nudity.testcategory.com` gave **no address at all** through `GetHostAddresses` in 30 s of asking, where the script expects the null address; `example.com` resolved throughout, and after the removal `xhamster.com` resolved again: the names are blocked, but not in the shape the script and the app's canary check expect; (c) the script's reading of the registry value failed on padding: Windows writes `NameServer` as `1.1.1.3,1.0.0.3` followed by NUL characters; (d) the measurement of the argument budget stopped at its first, shortest plan because of (a) and (b), although the command itself worked; (e) `ping.exe`, started with the hostile variables, could not resolve `example.com`: a program started with a false `SystemRoot` resolves no name, and the helper is such a program when the user's session has that variable. | (a) The helper writes the Windows hosts file in place (`hostsfile.rs`, with tests), and `write_in_place` no longer empties the file first. (c) The script reads the value up to the first NUL and also reads it the way the helper does, through `reg.exe`. (e) The helper sets `SystemRoot` and `windir` of its own process from what Windows reports before it looks a name up (`sys.rs`, with a test that runs it under a false `SystemRoot`). (b) The script prints, when such a lookup fails, what each way of asking says (`getaddrinfo` per address family with its error code and time, `ping.exe`, `Resolve-DnsName`, the DNS client's cache), to decide what the app's canary check has to read on Windows. The job also runs the system layer's own tests on each system, after the smoke run. |

| 3, `44660200460` | `3518f27` | `ubuntu-latest` | green | As run 1. The system layer's own tests on the runner: 101 passed. | nothing |
| 3, `44660200460` | `3518f27` | `macos-latest` | green | As run 1. The system layer's own tests on the runner: 101 passed. | nothing |
| 3, `44660200460` | `3518f27` | `windows-latest` | red, 12 checks | **Fixed and seen:** with the in-place write the hosts file keeps its owner and access list, with the block in place and after the removal, in all three cycles (item 26); `reg.exe query … /s /v NameServer`, run in the app's environment, prints the pairs for both adapters (item 6); the system layer's own tests pass on the runner, 101, among them the one that starts the lookup under a false `SystemRoot` (item 21). **Still failing, and now explained:** with the block in place `getaddrinfo` fails for the canary and for `xhamster.com` with Winsock error 11004, `WSANO_DATA` ("the requested name is valid, but no data of the requested type was found"), within a millisecond and for every address family; `ping.exe` says it cannot find the host; `Resolve-DnsName` prints `A 0.0.0.0` and `AAAA ::` for both, and the DNS client's cache lists `xhamster.com` as an alias of the canary (the first name of its hosts line). The same for `nudity.testcategory.com` with the resolver set. **Windows does not hand the null address to a program: a name whose only answer is the null address has "no data".** The block works (the names do not resolve; they do again after the removal), but the script waited for the null address, and the app's canary check, which reads an empty answer as "the hosts file is not honoured", would have raised that alarm on every Windows machine. The budget measurement again stopped at the shortest plan for that reason. | The lookup of a name that is expected to be blocked (`NameLookup::lookup_blocked`, used for the canary only) reads Winsock's "no data" as the null address on Windows; "host not found" stays "no answer" (`sys.rs`, with tests). The helper's `status` verb looks the canary up that way while the block is in the file and prints the answer, so both scripts now check what the app's own check reads. The Windows script accepts "no data" as blocked and checks that the canary fails with 11001 once the block is gone. |

| 4, `44662252304` | `3a99960` | `ubuntu-latest` | green | As run 1, plus the two new checks: with the block in place `status` reads the canary as the null address, and without it the canary is not asked for. The system layer's own tests: 102 passed. | nothing |
| 4, `44662252304` | `3a99960` | `macos-latest` | green | As on Ubuntu. 102 tests passed. | nothing |
| 4, `44662252304` | `3a99960` | `windows-latest` | red, a defect of the script | The script stopped in the first cycle at the new check of the canary in `status`: `The property 'Count' cannot be found on this object` (an `if` used as a value hands a list of one back as that one item). Every check before it was `ok`, the owner and access list of the hosts file among them. The script then put the machine back itself: "the saved bytes are back", "every adapter has the servers it had before the run", the state directory removed; the job's own comparison of the hosts file: identical. The system layer's own tests: 102 passed. | The list is collected with `@(...)` around the `if`. |

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
| `the elevation command works at all with the shortest plan` (Windows; the command runs in the app's four-variable environment) | 20, 27 |
| `hosts: owner and access list are what they were` (Windows) | 26 |

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
small binary `ubiqx-shield-helper`, as first written, took the verb directly and refused
that form. It accepts a leading `--shield-helper` since commit `412e146` (checked in the
sandbox with `status` and `elevate-argv`, which change nothing). What is left is the proof on
Windows. **CI:** the line `the elevation command works at all with the shortest plan` of the
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

### [ ] 25. Installed paths and package names of both products (review of U9a)

`REMOVAL.md` part 2 names the installed executable of each product and part 4 names the
autostart entries. None of these names was read from an installed build: ubiqX's follow
`docs/WINDOWS-LINUX.md` and `docs/MACOS-TESTING.md`, the standalone app's follow its
`tauri.conf.json`. ubiqX's shell has no `mainBinaryName`, and its Cargo package is
`ubiqx-desktop`, so the executable may carry that name instead. **Person, each OS and each
installer format**, with both products installed:

| What | Expected by `REMOVAL.md` | Found |
|---|---|---|
| macOS executable | `/Applications/ubiqX.app/Contents/MacOS/ubiqX`, `/Applications/NoFap.app/Contents/MacOS/ubiqx-shield` | |
| Linux `.deb` executable and package name (`dpkg -S`) | `/usr/bin/ubiqx`, `/usr/bin/ubiqx-shield` | |
| Windows `-setup.exe` and `.msi` executable | `%LocalAppData%\ubiqX\ubiqX.exe`, `%ProgramFiles%\ubiqX\ubiqX.exe`, `%LocalAppData%\NoFap\ubiqx-shield.exe` | |
| Autostart entry | `ubiqX.plist` / `NoFap.plist`, `ubiqX.desktop` / `NoFap.desktop`, the `Run` values `ubiqX` / `NoFap` | |
| The command of part 2 run as written | exits 0 and removes the tiers | |

Change `REMOVAL.md` to what is found.

- Evidence (date, OS versions, installer file names, the listings):

### [ ] 26. Windows: owner and access list of the hosts file after a write (review of U9a)

The helper writes the hosts file through a temporary file and a rename. On Windows the new
file takes the access list its directory hands down and the account that created it as
owner, which need not be what the file had. The Windows script compares the security
descriptor before the run and after the removal. **CI:** the line `hosts: owner and access
list are what they were`. If it fails, the helper has to write in place on Windows or copy
the descriptor; the hosts tier of Windows stays closed until then.

- Evidence: the Windows run of section 3.

### [ ] 27. The elevation command in the environment the app gives it (hand-off U4)

The app starts `powershell.exe`, `osascript` and `pkexec` with a cleared environment (on
Windows only `SystemRoot`, `windir`, `PATH` and `PSModulePath`). Whether Windows PowerShell
starts and `Start-Process -Verb RunAs` works with only those four is not known. Step 5 of
the Windows script runs the command in exactly that environment. **CI** for Windows;
**person** for macOS and Linux (item 5 covers the prompts themselves).

- Evidence: the Windows run of section 3; item 5 for the other two.

## 6. Still open, and who decides

Carried from the design's "Still open" list, for whoever reads this file first.

| Point | Decided by | Decision |
|---|---|---|
| Should quitting the app wait the delay while the module is on? (question 6) | owner | |
| The name: neither "NoFap" nor "Escudo" is cleared (item 17) | owner | |
| The elevated executable is the app itself; bundling the small helper and signing the builds is deferred (design 12) | owner | |
| The per-key cap front-loads the AI check on a host with a standing signal (hand-off U2) | integrator | |
| `PRAGMA secure_delete` for every table of both products (hand-off U3, item 23) | integrator | |
