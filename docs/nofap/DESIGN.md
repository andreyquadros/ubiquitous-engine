# nofap: the design to build

Module `nofap` inside ubiqX, and a standalone product built from the same code.
Written 2026-11-19 from the three candidate designs, the two judge reports, the seven
research reports in `docs/nofap/research/` and direct reads of the code. Nothing was compiled
for this document. An adversarial review on the same day changed it in place; the last
section, "Review changes", lists what changed and why, and what is still open.

Marks used below:

- **[U]** unverified on a real machine or against a primary source. Every [U] is behind a
  gate (section 1.8) or is a named item of the checklist in section 11.4. None is assumed.
- **[V]** verified in this sandbox today (a download, a file read, a command).

## 0. Decisions in one page

**Base.** Both judges chose the "mvp" design (38 and 37 points). This document starts from
it, grafts the listed ideas from "private" and "robust", and resolves every must-fix. Where
the judges disagreed, the choice and the reason are in section 0.2.

| Question | Decision |
|---|---|
| What blocks | Five layers, each with its own state. **L1 guard**: every 2 s the foreground browser tab's host is matched on the device against a 239k-entry embedded list, keyword and TLD rules, and the user's list; a hit closes the tab. **L2 hosts**: a marked block of the most visited adult hostnames in the hosts file. **L3 resolver**: system DNS pointed at Cloudflare for Families (the user's choice, operator named). **L4 browser policies**: browser DNS-over-HTTPS off and SafeSearch on, so L2 and L3 hold inside the browser. **L5 vision**: the AI provider the app already uses looks at one reduced image of the foreground window when the site or app is unknown. |
| Where the logic lives | One new pure crate, `crates/ubiqx-nofap` (decisions and embedded data, no I/O). The runtime is `ubiqx_engine::nofap::NofapService`, built from ports only (the `UpdateChecker` shape), so ubiqX's engine and the standalone app host the same service. |
| System layer | One port, `SystemShield`. One adapter whose three OS back-ends are command builders and output parsers compiled on every OS behind `Runner`, `Fs` and `Elevator` traits. Its only FFI is two Windows directory lookups in `sys.rs` (2.5). |
| Elevation | The app's own executable re-run as `--shield-helper <verb>` through `osascript ... with administrator privileges`, PowerShell `Start-Process -Verb RunAs`, or `pkexec`. One OS prompt per change. No daemon, no scheduled task. |
| Configuration | Not in `Settings`. One row (`nofap_state`) changed only through typed requests; requests that weaken protection wait out a delay the user chose (default 10 minutes) and then need a fresh confirmation. |
| Counselling | A reviewed message bank (section 5.6), never text written by a model. |
| Standalone product | A second Tauri shell in `apps/nofap/` that starts `NofapApp` (in `ubiqx-app`), which never constructs an `Engine`: no sampler, tracker, classifier or reports exist in that process. |
| What ships switched on | Nothing. The module is off until the user finishes its wizard. L2, L3 and L4 are additionally closed per operating system until that system's checks pass (section 1.8). |

### 0.1 The name (decision the owner must confirm: question 1)

The request says the module is called **nofap**. This design keeps that name where the owner
works and changes it where the public or the operating system would see it:

| Ring | Name | Where |
|---|---|---|
| Source | `nofap` | crate `ubiqx-nofap`, modules `ubiqx_core::nofap` and `ubiqx_engine::nofap`, tables `nofap_*`, IPC commands `nofap` and `nofap_host`, frontend `src/nofap/`, i18n namespaces `nofap` and `nofap_app`, app directory `apps/nofap/`, docs `docs/nofap/` |
| Machine | `ubiqx-shield` (constant `ubiqx_core::nofap::SYSTEM_ID`) | everything written to the user's machine or published: hosts-file marker, system state directory, policy and packaging file names, the helper flag `--shield-helper` (process arguments are visible to other accounts), the card window's label, export file names, the standalone app's bundle identifier `ai.ubiqx.shield`, binary name, data directory, keyring service, release tag and artefact names |
| Display | `Escudo` (constant `ubiqx_core::nofap::DISPLAY_NAME`, the single place) | sidebar label in ubiqX, the standalone product's name, window titles, tray, every sentence that names the feature (through the `{name}` placeholder) |

Reasons for a display name other than "NoFap":

1. **Trademark.** NOFAP is a registered US mark of NoFap LLC (Reg. 4,591,410 class 41;
   4,724,471 class 42, which covers online services like this one; 5,160,831 classes 25 and
   35), with 2026 filings reported in the UK (class 9, software) and India. The owner
   litigates, including a trademark-dilution count. INPI could not be searched, so Brazil is
   unknown, not clear, and Brazilian law can protect a foreign mark without local
   registration (LPI art. 124 XXIII, art. 126). Source: `research-counselling.md` section 5,
   which rests on search extracts, not on the registers themselves [U].
2. **Discretion.** The product name is on the Dock, the taskbar, the tray tooltip and the
   process list, where another person can read it. "Escudo" says nothing about the subject.
3. **Stance.** "Fap" commits the product to a position on masturbation. The counselling
   research says it must take none; the user sets the goal.

The machine ring is neutral for a different reason: a bundle identifier or a hosts marker
cannot be renamed later without migrating data and system state. `ubiqx-shield` works with
either display name, so the owner's decision never needs a migration.

If the owner wants the product shown as "NoFap": change `DISPLAY_NAME`, the `productName` and
window `title` in `apps/nofap/src-tauri/tauri.conf.json` (a test names both when they
differ), and set `ALLOW_NOFAP_IN_DISPLAY = true` in `crates/ubiqx-nofap/tests/naming.rs`.
Nothing else changes. "Escudo" itself has not been cleared either (INPI classes 9, 42, 44, 45
and USPTO); the standalone product is not published until it is (section 8.6).

Guard rail: `crates/ubiqx-nofap/tests/naming.rs` and a Vitest twin fail when "nofap" (any
letter case) appears in an i18n value, a `tauri.conf.json`, a capability file, an
`Info.plist`, a packaging file, `resources.json`, the constants of
`ubiqx_platform::sysblock` (marker, paths, helper flag), or the standalone artefact names,
unless `DISPLAY_NAME` itself contains it.

### 0.2 Where the judges disagreed, and what was chosen

| Point | Judge 1 | Judge 2 | Chosen | Why |
|---|---|---|---|---|
| Browser policies (DoH off, SafeSearch) | must be in v1, behind the per-OS gate | named v1.1 | **In this build, behind the gate** | `research-dns-enforcement.md` 2.5 says not to rely on the hosts file surviving browser DoH and puts policies in its recommended v1. The work is pure text and argv rendering, testable here. |
| `minimize_app` Enforcer method | add if trivial on three systems | not listed | **Not added** | Three untestable OS implementations. The existing `Enforcer::hide_others` is the reversible reaction (section 4.6). |
| Windows DNS verification | `GetAdaptersAddresses` FFI | not listed | **`reg.exe query` parser, PowerShell only on demand** | No FFI that cannot be run here; no PowerShell start every two minutes (section 2.3). |
| Signed Extended list pack | v1.1 | v1.1 | **Deferred** (section 12) | Agreed. `Matcher::new` takes the table as bytes, which is the seam. |
| "Popular" table | unmeasured (flaw of private) but never-capture list wanted | add both | **Both added**; popular hosts are captured only on a local signal | Without it almost every site is photographed. The table is data built by the same script. |
| System tiers in the first build | per-OS gate | per OS and per tier, and do not ship with no DNS blocking | **Per OS and per tier; opened by a CI smoke run on that OS** (section 1.8) | The request's headline is DNS blocking. GitHub's three runners can run the helper as administrator for real, which turns most [U] items into machine checks. |

### 0.3 What the request asked for, and where it is

| Request | Section |
|---|---|
| "travar via DNS os sites pornôs" | 2 (hosts block, family resolver, browser policies). Each tier is closed by the gate of 1.8 until the gate-opening step of section 13 has evidence for that operating system; `(Linux, hosts)` can be opened from a run in this sandbox |
| "uma extensa lista de sites" | 3 (239,449 entries embedded; about 20,000 hostnames at DNS level; the resolver for the rest) |
| "use a IA já embarcada para visualizar o que o usuário estiver vendo" | 4 |
| "aconselhá-lo" | 5 |
| "remover o acesso" | 1 (tab closed on every pass, no cooldown), 2, 4.6 |
| "módulo … incorporado no ubiqX" | 7 |
| "módulo separado … +1 produto" | 8 |

---

## 1. The guard (L1) and the rules common to every layer

### 1.1 The pass

`NofapService::guard_pass()` runs every 2 s on the blocking pool
(`LoopConfig.nofap_every`, loop copied from `focus_loop`,
`crates/ubiqx-engine/src/engine.rs:218-234`, first tick skipped). It is independent of
`focus::guard_once`, `focus.guard_enabled`, tracking, pause, private mode and quiet hours.

1. Off, or this process is not the primary (1.6): return before any platform call. ubiqX's
   scripted demo and `ubiqx simulate` are therefore unchanged for users who never enable it.
2. `ActivitySource::peek_foreground()` (new default method that calls `foreground()`;
   `ScriptedPlatform` overrides it to return the step `foreground()` last served, so the pass
   never consumes tracker steps, `crates/ubiqx-platform/src/mock.rs:221-227`).
3. Exempt windows are skipped: `ubiqx_engine::focus::is_exempt_window` plus both products'
   identities (section 7.2).
4. For a supported browser, `BrowserUrlResolver::resolve`; host through
   `ubiqx_nofap::matcher::Matcher::verdict` (section 3.4).
5. `Verdict::Blocked` goes to enforcement (1.2). Anything else goes to the vision scheduler
   (section 4), which may return a job. The job runs on its own task, one in flight at a
   time (7.7), so a check that takes 14 s never delays the next guard pass.

### 1.2 Enforcement

| Hit | Action, in order, with existing `Enforcer` methods only |
|---|---|
| Blocked host in a browser tab (list, user, learned, keyword, TLD) | `close_active_tab` → `blank_active_tab` → card only |
| Vision positive in a browser tab (4.6) | same |
| Vision positive in another app | `hide_others(<own bundle id>)` (reversible), then a card with "Fechar a janela"; `quit_app` runs only on that click |

Four rules, each fixing a fault found in the code or by the judges:

1. **No enforcement cooldown.** The focus guard returns before acting during its 20 s
   cooldown (`crates/ubiqx-engine/src/focus.rs:223-232`), so a closed site is usable for
   20 s. Here a hit in front is acted on in every pass. Only the card and the counter are
   rate limited (rule 4).
2. **Two consecutive passes.** A host is acted on when the previous pass saw the same host in
   the same app (`CONFIRM_PASSES = 2`). This is the guard against host-shaped text being
   typed in the Windows address bar. It does **not** protect against the URL cache, because
   both passes read the same 20 s entry (`crates/ubiqx-platform/src/browser.rs:117,203-207`);
   rule 3 does.
3. **Fresh re-read immediately before acting.** `BrowserUrlResolver::resolve_fresh` (new
   default method that calls `resolve`; `OsascriptUrlResolver` and `UiaUrlResolver` override
   it to skip their cache) and `peek_foreground()` are read again. The action runs only when
   the app id, the window id (when known) and the host are unchanged. On Windows the action
   is a keystroke into whatever is active
   (`crates/ubiqx-platform/src/windows/enforce.rs:191-216`), so this check is what keeps it
   from landing on another tab. Also shipped: while the address bar has keyboard focus its
   text may be something the user is still typing, so `UiaUrlResolver` does not read it. It
   answers the URL it last read for that window while the bar was not focused, as long as
   the window title has not changed since, and `None` otherwise (pure decision
   `committed_url(has_keyboard_focus, bar_text, last_committed, title)`, one extra cached
   UIA property; type-checked for the Windows target, [U] at run time). Answering `None`
   whenever the bar is focused would let one click on the address bar keep a listed page
   open.
4. **Episodes.** Hits less than `EPISODE_GAP_SECS = 120` apart are one episode: the card at
   the first hit and once more at the third, one counter increment per episode.

A vision result or a model refusal never quits an app or a browser by itself.

### 1.3 Coverage, reported, never silent

`GuardState` is `on`, `off` or `limited { reason }`:

| Situation | Reason shown | What still protects |
|---|---|---|
| macOS, Automation denied for the browser in front | `automation_denied`, with the button that opens the permission pane | L2, L3, L4 (Firefox), L5 |
| macOS Firefox and forks, and on every system a browser the URL resolver does not support (`OTHER_BROWSERS`, 4.2: Tor Browser, Opera on macOS, Orion, Zen, LibreWolf and others) (no URL, no tab action) | `browser_without_url` | L2, L3, L4 where a policy exists, L5 |
| Linux X11 (host known only when the title prints it) | `title_only` | L2, L3, L4, L5 |
| Linux Wayland (`foreground()` is `None`, no capture) | `wayland` | L2, L3, L4 only |
| Background tabs, other windows, other accounts | stated on the page, not a state | L2, L3, L4 |

The last column assumes the system tiers are applied. While a tier is gated (1.8), declined
or not chosen, it is left out of what the page lists. When nothing is left (Wayland with no
system tier; a browser without URL with vision off and no system tier) the status is
`overall: reduced` with the line "Neste momento nada está barrando sites nesta janela", never
`protected`.

The service reads `PermissionChecker::status()` itself (today only the tracker's sampler
raises `PermissionRequired`, and the standalone app has no sampler). macOS builds are ad-hoc
signed, so each update resets Screen Recording and Automation: after an update the page shows
`limited` or `vision: off { screen_recording }` with the button, never a silent gap.

### 1.4 What ubiqX's tracker must forget (ubiqX only)

A hit must leave nothing in the productivity data. Three hooks, all in `ubiqx-engine`:

1. `tracker::handle_sample` (`crates/ubiqx-engine/src/tracker.rs:46`): before
   `segmenter.feed`, a sample for which `NofapService::redacts(&sample)` is true is rewritten
   as the segmenter rewrites private browsing (`window_title = PRIVATE_TITLE`, `url = None`,
   `window_id = None`). True when the host is `Blocked`, or the window's context key had a
   hit in the last 10 minutes.
2. On any hit, `tracker::scrub_open_block(state)`: closes the open block through the
   segmenter, sets title to `PRIVATE_TITLE`, `url`, `domain`, `description`, `ai_payload` to
   `None`, category `sys-private`, persists it, and deletes its screenshots
   (`ScreenshotRepo::delete_for_block`, new; files unlinked).
3. `screenshots::should_capture` asks `NofapService::capture_veto(app_id, domain, title)`:
   no productivity screenshot of a window whose host is `Blocked` or mixed, or which has a
   signal or a hit in the last 10 minutes.

### 1.5 Status model

```rust
pub struct NofapStatus {
    pub overall: Overall,                 // off | protected | reduced | pending_off | secondary
    pub guard: GuardState,                // 1.3
    pub hosts: TierState,                 // 2.2
    pub resolver: TierState,              // 2.3
    pub policies: TierState,              // 2.4
    pub vision: VisionState,              // off { reason } | on | degraded { until }
    pub issues: Vec<Issue>,               // each has copy and at most one action
    pub pending: Vec<PendingView>,
}
pub enum TierState { Off, Applied, UpdatePending, Changed, Missing, NotApplied,
                     Gated, Unsupported { reason: String }, InUseByOther }
pub enum Issue { HostsNotHonoured, DnsBypassSuspected, ResolverUnreachable, NewConnectionUnfiltered,
                 PolicyConflict, PolicyAdvisoryOnly, AutomationDenied, ScreenRecordingDenied,
                 VisionBudgetSpent, VisionCapReached, VisionNoProvider, ConsentOutdated,
                 OtherProductPrimary }
```

`protected` is shown only with the list of layers that are on. No domain, and no word about
the subject, appears in the tray, a notification or a window title.

### 1.6 Two products on one machine

- **System layers** are shared through an `owners` set in the system state file (2.5).
  Removing for one product keeps a layer the other still wants.
- **Passes.** Only one process runs the guard and vision. `ubiqx_nofap::primary::decide(lock,
  me, now) -> Role` reads `<data_local_dir>/ubiqx-shield/primary.lock`
  (`{ product, pid, heartbeat_at }`, rewritten every 30 s by the primary). A product writes
  the lock only while its own module is enabled and its guard loop is running, and deletes
  it on disable and on quit, so "the other app" is never one whose module is off. A
  heartbeat from the other product that is at most 90 s old makes this process `Secondary`:
  it runs no pass, shows `overall: secondary` ("Proteção ativa pelo outro app") and takes
  over when the heartbeat goes stale. A heartbeat more than 90 s in the future (a clock
  change, a file edited by hand) is stale too, so a bad timestamp cannot switch both
  products off. The standalone app wins a tie.
- Both products exempt each other in every guard (section 7.2).
- Both shells add `tauri-plugin-single-instance`, which also ends the two-engines-on-one-
  database fault found by the shell map.

### 1.7 Failure modes at a glance

| Failure | Effect | Shown as |
|---|---|---|
| OS prompt declined | L1 and L5 run; chosen system tiers stay `not_applied` | "Proteção parcial" + "Aplicar" |
| App quit from the tray | L1 and L5 stop. L2 to L4 stay only where they are applied; where none is, everything stops. The quit dialog says which of the two is true (`quit.confirm_system` or `quit.confirm_none`, 5.6) and the quit is counted (6.3). Nothing prevents quitting. | n/a |
| App deleted without disabling | L2 to L4 stay and the machine works; removal is documented (2.7) | n/a |
| No AI key, offline, `local_only`, vision budget spent | L5 off; the other layers carry the product | "Verificação por IA: desligada" + reason |
| Hosts write refused by security software | helper exit 3; other tiers may be applied | `hosts: unsupported { write_refused }` |
| Names stop resolving after the resolver change | one undelayed button restores DNS (6.3) | `Issue::ResolverUnreachable` |

### 1.8 The per-OS, per-tier gate

`ubiqx_nofap::gate::tier_enabled(os: Os, tier: Tier) -> bool` reads the constant table
`TIER_GATE: [(Os, Tier, bool); 9]`. A closed entry shows the tier as `Gated` ("Em verificação
neste sistema") and the helper refuses that section of a plan.

- **The code lands with all nine entries `false`.** L1, L5 and all counselling work
  everywhere. Opening entries is a named step of the final gate (section 13), not something
  left for a later release: a build in which every entry is still `false` blocks nothing at
  DNS level, which is the first thing the request asks for, and has to be reported to the
  owner in those words.
- An entry for `hosts` or `resolver` is flipped to `true` in the commit that follows a green
  run of the CI job `nofap-system-smoke` on that OS (section 9.3), with the run URL recorded
  in `docs/nofap/REAL-MACHINE-CHECKLIST.md`. The job runs the real helper as administrator on
  GitHub's Windows, macOS and Ubuntu runners: apply, observe, resolve the canary, remove,
  compare the hosts file byte for byte with the original, read DNS back.
- `(Linux, hosts)` does not have to wait for CI. This sandbox runs as root with a regular
  `/etc/hosts` and `hosts: files dns` [V], so the same script runs here:
  `UBIQX_SHIELD_SMOKE=1 scripts/nofap/system-smoke.sh --tiers hosts`. A green run, with its
  output pasted into the checklist, opens that entry before any push. `(Linux, resolver)`
  needs systemd-resolved, which the sandbox does not run, so it waits for the CI job.
- An entry for `policies` is flipped only after the manual browser checks of 11.4 (the banner,
  `chrome://policy`, `about:policies`).
- Testers set `UBIQX_SHIELD_UNVERIFIED_TIERS=hosts,resolver,policies` to open tiers on their
  own machine. The app then passes `--unverified <tiers>` to the helper (elevation does not
  carry the environment), and the page labels those tiers "em teste".

What the smoke run cannot show stays on the manual checklist and is disclosed in the UI:
the elevation prompts themselves (a failed prompt is `Declined` or `Unavailable`, which is
safe), localised command output (the parsers do not depend on it, 2.3), and browser DoH
behaviour.

---
## 2. DNS-level blocking (L2, L3, L4) and the system layer

All three tiers are changes to the system that keep working when the app is closed. All
three fail safe: with the app deleted the machine still resolves names. All three need
administrator rights once, through the helper (2.5). None is applied before the user has
seen the exact list of changes (`SystemShield::preview`) and the sentence "O sistema vai
pedir sua senha de administrador".

Honest ceiling, shown in the wizard and on the help page: this is friction for the person
who installed it. Anyone with the administrator password can undo every tier; a phone, a VPN,
Tor or another network is outside it.

### 2.1 The plan

```rust
// ubiqx_core::nofap
pub struct SystemPlan {
    pub v: u8,                         // 1
    pub owner: ProductId,              // Ubiqx | Shield  ("ubiqx" | "shield")
    pub hosts: bool,
    pub extra: Vec<String>,            // hostnames the user typed, lower-case, without `www.`
                                       // (the helper writes the `www.` twin itself); at most 400
    pub resolver: Option<ResolverId>,  // None | CloudflareFamily
    pub policies: bool,
}
```

Built by `ubiqx_nofap::plan::build_plan(&NofapConfig, &[NofapSite], owner, os) -> BuiltPlan
{ plan, extras_left_out: u32 }`. Learned sites are never in `extra` (section 3.7). The plan
travels to the helper as one argument, `--plan-b64 <base64url(JSON)>`, so the helper reads no
file from a user-writable place.

**Argument budget** (`plan::ARG_BUDGET[os]`, the longest `--plan-b64` value the app will
send): 24,000 characters on macOS and Linux, and **1,800 on Windows** [U]. Elevation on
Windows goes through `ShellExecuteEx`, whose parameter string has no documented limit and
has been reported to fail somewhere above 2,048 characters; the 32,767 of `CreateProcess`
does not apply to it. `build_plan` keeps the most recently added names that fit (about 80
on Windows) and counts the rest in `extras_left_out`. The page then says "{n} sites ficam só
na guarda do app", which is true because L1 blocks them. The Windows smoke job sends a
24,000-character plan through the real elevation command (9.3), and the budget is raised to
what that run shows in the commit that opens the Windows gate. The helper itself accepts up
to 24,000 characters on every system.

### 2.2 L2: the hosts block

**Content**, in this order, generated by the helper from its own embedded data:

1. One canary line: `0.0.0.0 canary.ubiqx-shield.invalid` (a fixed reserved name, the same on
   every machine, so it identifies nobody).
2. The validated `extra` names of every owner in the state file.
3. The hosts tier (section 3.2), most visited first, until the cap.

```
# >>> ubiqx-shield v1 BEGIN sha256=<hex of the lines between the markers> names=<n>
# Added by the ubiqx-shield app on this computer. To undo, delete every line from BEGIN to END.
0.0.0.0 name1 name2 name3 name4 name5 name6
:: name1 name2 name3 name4 name5 name6
...
# <<< ubiqx-shield END
```

| Constant (`ubiqx_nofap::hosts`) | Value | Status |
|---|---|---|
| `CAP_WINDOWS` | 8,000 names | [U] conservative start; the Windows DNS Client re-parses the file and degrades with large ones (`research-dns-enforcement.md` 2.3). Raised only with a measurement from the smoke job. |
| `CAP_UNIX` | 20,000 names | [U] |
| `NAMES_PER_LINE` | 6 | [U] inside the per-line limit reported for Windows |
| Address families | one `0.0.0.0` line and one `::` line per group | correct whether or not an IPv4 line suppresses the AAAA lookup [U] |
| Undo hint | the comment line after BEGIN (inside the hash), and a first line `# ubiqx-shield: delete this file and restart systemd-resolved to undo` in the Linux drop-in | a person who deleted the app, or never saw `REMOVAL.md`, finds the way out in the file itself. The text names only the machine id. |

- **Splice, never rewrite.** `hosts::splice(existing, Some(block))` replaces the block when
  the markers are present, else appends. Every byte outside the markers, the file's
  line-ending convention and a missing final newline are preserved. `splice(existing, None)`
  removes exactly the block. Write is temp file + rename in the same directory, owner and
  mode kept. The helper works on the file the hosts path resolves to, and only when that
  file and its directory belong to root (Administrators or SYSTEM on Windows). A hosts file
  that is a link into a read-only store (NixOS) or sits on a read-only file system is
  `Unsupported { write_refused }`. When the rename fails because the file is a mount point
  (`EBUSY` or `EXDEV`, as in containers), the helper writes the spliced bytes in place and
  syncs. After every write it reads the file back and compares; on a mismatch it puts back
  the bytes it read at the start and exits 3.
- **Never-block list.** `ubiqx_nofap::matcher::NEVER_BLOCK` (operating-system vendors, update
  and sign-in hosts, payment and bank suffixes, `gov*`, `edu*`, `mil`, `jus.br`, AI
  providers, the resolver's own names, sexual-health and education sites) is both the
  matcher's built-in allowlist and the helper's refusal list for `extra`. No Microsoft-owned
  name is ever written (Defender `HostsFileHijack`).
- **Not written in v1:** SafeSearch pins (a stale address breaks Google; a Bing line is a
  Defender risk) and DoH-endpoint names (section 12).
- Paths: `<System32>\drivers\etc\hosts`, where `<System32>` is what `GetSystemDirectoryW`
  returns and never an environment variable (2.5); `/etc/hosts`. Flush after writing:
  `ipconfig /flushdns`; `dscacheutil -flushcache` and `killall -HUP mDNSResponder`;
  `resolvectl flush-caches` (ignored when absent).

**Verify** (unprivileged, every 120 s and on demand): read the file, `hosts::inspect`.

| Observation | `TierState` | What the user sees |
|---|---|---|
| Block present, marker hash = hash of its lines = hash of what would be written now | `Applied` | "Lista no sistema: ativa" |
| Block intact, but the app would now write something else (a site the user added, a new app version) | `UpdatePending` | quiet line "1 site aguardando entrar na lista do sistema" + "Aplicar". Never a prompt by itself. The guard already blocks that site. |
| Markers present, hash of the lines differs | `Changed` | drift card (2.6) |
| No block although the state file lists hosts for this product | `Missing` | drift card |
| Never applied, or the prompt was declined | `NotApplied` | "Lista no sistema: não aplicada" + "Aplicar" |
| Gate closed (1.8) | `Gated` | "Em verificação neste sistema" |
| File unreadable; AppImage | `Unsupported { reason }` | the reason and the manual steps |

The canary is resolved through the OS (`getaddrinfo`) at apply and every 30 minutes, only
while the block is `Applied`. An answer other than `0.0.0.0` or `::` is
`Issue::HostsNotHonoured` (shown as "reduced").

Stated in the UI: exact names only, so the long tail belongs to L1 and L3; "vale para páginas
novas; reinicie o navegador para garantir"; names in this file can be read by other accounts
on this computer, which is why only widely known names and the ones the user typed go in.

### 2.3 L3: the family resolver

One provider row and the choice "none" (`ubiqx_nofap::resolvers::RESOLVERS`; a second
provider is one row):

| `ResolverId` | IPv4 | IPv6 |
|---|---|---|
| `cloudflare_family` | `1.1.1.3`, `1.0.0.3` | `2606:4700:4700::1113`, `2606:4700:4700::1003` |

Addresses from `research-dns-enforcement.md` section 4 [U against Cloudflare's own page,
which the sandbox could not open; the smoke job asserts they answer]. Cloudflare is credited
next to the toggle and in About ("DNS filtrado por Cloudflare for Families"); their terms
must be re-read before release (11.4).

**Disclosure before the choice** (the option is preselected as "recomendado"; the other
option is one click away; the user confirms the step):

> Com esta opção, todos os nomes de sites que este computador consultar passam pela
> Cloudflare (1.1.1.3), no lugar do seu provedor de internet. É a Cloudflare que decide o
> que barrar: sites adultos e sites que ela considera perigosos. O ubiqX não recebe nada.
> Se preferir, escolha "só a lista neste computador".

The helper accepts a `ResolverId`, never an address. There are no custom addresses.

| OS | Read (unprivileged) | Set (helper) | Undo (helper) |
|---|---|---|---|
| macOS | `/usr/sbin/networksetup -listallnetworkservices` (first line is a header; names starting with `*` are disabled), `-getdnsservers <svc>` | `-setdnsservers <svc> <v4a> <v4b> <v6a> <v6b>` for every enabled service | `-setdnsservers <svc> empty` |
| Windows | quick: `reg.exe query` of `HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces` and `Tcpip6\...` for the value `NameServer` [U]; full: PowerShell `Get-DnsClientServerAddress` joined with `Get-NetAdapter` (status `Up`) through `ConvertTo-Json` | `Set-DnsClientServerAddress -InterfaceIndex <n> -ServerAddresses (...)` on every adapter that is up (Windows asks all interfaces in parallel) | `Set-DnsClientServerAddress -InterfaceIndex <n> -ResetServerAddresses` |
| Linux | `/usr/bin/systemctl is-active systemd-resolved`; the drop-in file | write `/etc/systemd/resolved.conf.d/90-ubiqx-shield.conf` (`[Resolve]`, `DNS=<v4a> <v4b> <v6a> <v6b>`, `Domains=~.`), `systemctl restart systemd-resolved` | delete the file, restart the service |

Rules:

- **Parsers do not depend on the system language.** A line of `-getdnsservers` output that
  parses as an IP address is a server; no such line means "automatic". PowerShell output is
  JSON. `netsh` and `ipconfig /all` are never parsed.
- **Undo never writes a recorded value.** The helper re-reads every interface and resets to
  automatic only those whose servers are still exactly the provider's set. An interface the
  user changed since is left alone. A manual DNS that was in place before is therefore not
  put back by the helper: the app records it (unprivileged, in its own database) before
  applying, shows it in the preview ("Seu DNS atual, 192.168.0.2, será substituído. Ao
  desligar, o DNS volta ao automático.") and again when the tier is removed. This removes the
  path by which a value from a file could become a DNS server.
- Plain DNS on all three systems (no DoT or DoH): the same behaviour everywhere, and a
  network that blocks port 853 cannot leave the machine without names.
- Linux without systemd-resolved, or with it running while `/etc/resolv.conf` does not point
  at it (not a link to a `systemd/resolve` stub file and no `nameserver 127.0.0.53` line, so
  the drop-in would change nothing): `Unsupported`, with the manual steps. NetworkManager
  support is deferred.
- **The helper does not leave the machine without names.** Before changing DNS it resolves
  the probe name (`one.one.one.one`) through the OS. After the change it tries again for up
  to 8 s. If the probe worked before and fails after, the helper undoes the resolver tier in
  the same elevated run, leaves the other tiers applied and reports
  `Partial([resolver: unreachable])`. The page shows `resolver: not_applied` with "Esta rede
  não alcança o DNS filtrado" (some networks block outside DNS). The user gets the network
  back without a second prompt.
- Windows quick read runs every 120 s for the interface GUIDs seen at the last full read;
  the full read runs at apply, when the page opens, on "Verificar agora" and every 30
  minutes. If the registry read returns nothing usable, the full read runs every 10 minutes.

| Observation | `TierState` / issue |
|---|---|
| Every active interface, IPv4 and IPv6, uses the provider's set; on Linux the drop-in is present and equal | `Applied` |
| At least one active interface uses something else | `Changed` + `Issue::NewConnectionUnfiltered` when the interface is new. The copy does not guess whether it was a dongle, a VPN or a manual change. |
| The user chose "none" | `Off` |
| The user pressed "Restaurar minha rede" (6.3) | `NotApplied`, with "Religar" |
| `getaddrinfo("one.one.one.one")` fails on three consecutive checks while this tier is applied | `Issue::ResolverUnreachable`: a card offers "Restaurar minha rede". Nothing changes by itself. |

Known gaps, stated on the page: a VPN brings its own resolver; a browser where the user
picked a specific DoH provider resolves around it (L4 closes that where L4 exists); Safari
with iCloud Private Relay resolves through Apple's relay and may ignore both this resolver
and the hosts block [U, checklist item 8], which leaves L1 for Safari; captive portals may
need "Restaurar minha rede". The app does not look for VPN or Tor processes.

### 2.4 L4: browser policies

**What it does.** Sets each browser's own enterprise policy so that it resolves through the
system (where L2 and L3 apply) and forces SafeSearch where a policy exists.

| Browser family | Values (`ubiqx_nofap::policies::desired`) |
|---|---|
| Chrome, Chromium | `DnsOverHttpsMode = "off"`, `ForceGoogleSafeSearch = true` |
| Edge | the same, plus `ForceBingSafeSearch = 2` |
| Brave | the same as Chrome, plus `TorDisabled = true` |
| Firefox | `DNSOverHTTPS = { Enabled: false, Locked: true }` |

Private browsing is not disabled (the guard and vision read private windows). YouTube
restricted mode is not forced.

| OS | Where | Notes |
|---|---|---|
| Windows | named values under `HKLM\SOFTWARE\Policies\Google\Chrome`, `...\Chromium`, `...\Microsoft\Edge`, `...\BraveSoftware\Brave`, and `...\Mozilla\Firefox\DNSOverHTTPS` (`Enabled`, `Locked`) through `reg.exe add` | [U] that an unmanaged Windows 11 Home honours them; [U] Firefox value names |
| Linux | our own file `ubiqx-shield.json` in `/etc/opt/chrome/policies/managed/`, `/etc/chromium/policies/managed/`, `/etc/chromium-browser/policies/managed/`, `/etc/opt/edge/policies/managed/`, `/etc/brave/policies/managed/`, only where the parent directory exists; Firefox `/etc/firefox/policies/policies.json`, created only when absent | The Firefox snap, Ubuntu's default, reads `/etc/firefox/policies/policies.json` like the `.deb` (Ubuntu's own guide) and the Chromium snap reads `/etc/chromium-browser/policies/managed/` [U], so both are covered by the paths on the left. Flatpak browsers: `Unsupported` for that browser [U] |
| macOS | Firefox: `/usr/bin/defaults write /Library/Preferences/org.mozilla.firefox` with `EnterprisePoliciesEnabled`, `DNSOverHTTPS__Enabled`, `DNSOverHTTPS__Locked` [U]. Chromium family: **nothing is written**. | Without MDM those browsers accept only "recommended" policies, which the user can override, so writing them would buy a "managed" banner for no effect. Shown as `Issue::PolicyAdvisoryOnly`: "No macOS, Chrome, Edge e Brave ficam sem esta camada". |

Rules:

- **A managed machine is left alone.** If a browser's policy location already holds a value
  that differs from ours and that the state file does not list as created by us, nothing is
  written for that browser: `Issue::PolicyConflict`.
- **Undo** deletes only values that still equal our constants and that the state file lists
  as created by this helper, and deletes our own files. No value read from a file is ever
  written back.
- **Disclosure before applying**, in the preview: "Os navegadores vão mostrar 'Gerenciado
  pela sua organização'. O DNS próprio do navegador é desligado para que os bloqueios
  valham."

Verification reads the registry values (`reg.exe query`) or the files, unprivileged.

### 2.5 The helper

**Entry point.** `ubiqx_platform::sysblock::helper::main_if_requested(std::env::args())` is
the first statement of both shells' `main`. When the first argument is `--shield-helper` it
does its work and exits before Tauri, the window server or the database are touched. The same
function is the whole body of a small binary target,
`crates/ubiqx-platform/src/bin/ubiqx-shield-helper.rs`, which links no GUI library. The smoke
job and the manual removal path use that binary. Bundling it as the elevation target, so that
the GUI executable is never run as root, is deferred (section 12); the code path is the same.

**Verbs.** Nothing else is accepted; there is no "run this".

| Verb | Privilege | Does |
|---|---|---|
| `status` | none | prints `SystemObservation` as JSON |
| `elevate-argv --plan-b64 <b64>` | none | prints, as a JSON array, the elevation command line the app would run for that plan on this OS, and runs nothing. It exists so the smoke job exercises the real builder (9.3 step 5). |
| `apply --plan-b64 <b64> [--unverified <tiers>]` | admin | validates, records the owner's wants, applies the union of all owners' wants |
| `remove --owner <ubiqx\|shield\|all> --scope <all\|resolver>` | admin | drops that owner's wants (or only its resolver), re-applies what other owners still want, removes what nobody wants |

**Validation as root** (`ubiqx_nofap::plan::validate`): version 1; owner from the closed set;
resolver id from the compiled table; at most 400 `extra` names, each passing
`hosts::is_valid_hostname` (ASCII letters, digits, hyphen, dots; 4 to 253 bytes; at least two
labels; not an IP literal) and not under `NEVER_BLOCK`; every requested tier open in the gate
for this OS. The worst a hostile same-user process can obtain, and only after the user
approves the OS prompt, is "some extra hostnames resolve to 0.0.0.0".

**Rules for the privileged code:**

- System tools are called by absolute path with an environment the helper builds itself and
  inherits nothing from: `/usr/sbin/networksetup`, `/usr/bin/dscacheutil`,
  `/usr/bin/killall`, `/usr/bin/defaults`, `/usr/bin/systemctl`, `/usr/bin/resolvectl`
  (`PATH=/usr/bin:/bin:/usr/sbin:/sbin`, `LANG=C`); `<System32>\reg.exe`, `ipconfig.exe`,
  `<System32>\WindowsPowerShell\v1.0\powershell.exe`.
- **On Windows no path comes from an environment variable.** An elevated process started
  through UAC receives the user's variables, and a per-user `windir` or `SystemRoot` value
  under `HKCU\Environment` is a known way to redirect an elevated program. `<System32>` is
  the result of `GetSystemDirectoryW` and `<ProgramFiles>` of
  `SHGetKnownFolderPath(FOLDERID_ProgramFiles)`: two calls in `sys.rs` under `cfg(windows)`,
  through the `windows` crate features the platform crate already enables
  (`Win32_System_SystemInformation`, `Win32_UI_Shell`), type-checked for the Windows target
  and exercised by the smoke job. The child environment is exactly `SystemRoot` and `windir`
  (the parent of `<System32>`), `PATH=<System32>`, and
  `PSModulePath=<System32>\WindowsPowerShell\v1.0\Modules`, so PowerShell cannot load a
  module from a user-writable place; the fixed scripts name their cmdlets with the module
  (`DnsClient\Set-DnsClientServerAddress`, `NetAdapter\Get-NetAdapter`).
- The helper writes only to the hosts file, the resolver drop-in, the policy locations and
  its state directory. It never writes to a path it was given.
- **State file**, world-readable, written atomically:

  | OS | Path |
  |---|---|
  | macOS | `/Library/Application Support/ubiqx-shield/state.json` |
  | Linux | `/var/lib/ubiqx-shield/state.json` |
  | Windows | `<ProgramFiles>\ubiqx-shield\state.json` (not `%ProgramData%`, see below) |

  ```json
  { "v": 1, "applied_at": 1795100000000,
    "owners": { "ubiqx":  { "hosts": true, "resolver": "cloudflare_family", "policies": true,
                            "extra": ["example.com"] },
                "shield": { "hosts": true, "resolver": null, "policies": false, "extra": [] } },
    "hosts": { "sha256": "…", "names": 8003 },
    "policies_created": [ { "browser": "chrome", "key": "DnsOverHttpsMode" } ] }
  ```

  **Nothing read back from this file becomes an address, a path or a command.** The only
  values that reach a system file again are the other owner's `extra` hostnames, re-validated
  like a plan. That is what makes a tampered or pre-created state file harmless.
- State directory: on Unix created `root`, mode 0755; before use `lstat` must show uid 0,
  a directory, not a symlink, not group- or world-writable (`state_dir::assess`), otherwise
  it is removed and recreated empty. "Removed" means: a link or a file is unlinked and never
  followed; a directory is deleted with `std::fs::remove_dir_all`, which does not follow
  links inside it. On Windows the directory lives under `<ProgramFiles>`, where a standard
  user cannot create anything and a folder created by the elevated helper inherits "Users:
  read" with no further step. `%ProgramData%` is not used: a standard user can pre-create a
  folder there, and the earlier plan of repairing its ACL with `icacls ... /T` would have
  followed a junction planted at that path and rewritten the ACLs of whatever it pointed
  to. If the path exists and is a reparse point or is not a directory, the helper touches
  nothing and exits 4.
- Exit codes: 0 applied, 2 invalid plan, 3 partial (a per-tier report is in the state file),
  4 failed, 5 tier gated.

**Elevation** (`ubiqx_platform::sysblock::elevate`, all pure argv builders):

| OS | Command | Declined or unavailable |
|---|---|---|
| macOS | `/usr/bin/osascript -e 'do shell script "<exe> --shield-helper apply --plan-b64 <b64>" with administrator privileges'`; the executable path goes through `sh_quote` then `applescript_quote` (any character, including non-ASCII, is handled by quoting, not by a whitelist) | error -128 → `Declined` |
| Windows | `powershell.exe -NoProfile -NonInteractive -Command "try { $p = Start-Process -FilePath '<exe>' -ArgumentList '--shield-helper','apply','--plan-b64','<b64>' -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode } catch { exit 1223 }"`; the path goes through `ps_single_quote`. Chosen over `ShellExecuteExW` because it needs no FFI that this sandbox cannot run; it is the same Windows mechanism underneath [U]. | 1223 → `Declined` |
| Linux | `/usr/bin/pkexec <exe> --shield-helper ...`; the `.deb` installs a polkit action with `org.freedesktop.policykit.exec.path` so the dialog shows our sentence [U] | 126 → `Declined`; 127 or no `pkexec` → `Unavailable { no_polkit_agent }`; `APPIMAGE` set → `Unavailable { appimage }` without trying, because root cannot read the user's FUSE mount |

Stated plainly in `docs/nofap/PRIVACY.md` and the checklist: the macOS prompt names
`osascript`; the Windows prompt says "unknown publisher" for an unsigned build; the
executable that gets elevated is the app itself, and wherever the user's account can write
to it (a per-user Windows install, an app in `~/Applications` or dragged into
`/Applications` by an administrator account, a development build) it is only as trustworthy
as that account, the same as any per-user installer. The helper adds no way to run anything
else: its verbs are closed and it executes only the system tools listed above.

Open [U] on the Windows path, on the checklist: whether the default `Restricted` execution
policy of Windows Home lets `powershell.exe -Command` load the `DnsClient` and `NetAdapter`
modules. If it does not, the resolver tier stays gated on Windows until the back-end sets
DNS with `netsh.exe interface ipv4|ipv6 set dnsservers` and reads adapters from the
registry; `-ExecutionPolicy Bypass` is not used, because security software reads it as an
attack pattern.

**Port** (`ubiqx_core::nofap`):

```rust
pub trait SystemShield: Send + Sync {
    fn capabilities(&self) -> SystemCapabilities;                     // per tier: Available | Gated | Unsupported(reason)
    fn observe(&self) -> CoreResult<SystemObservation>;               // unprivileged: hosts text, DNS per interface,
                                                                      // drop-in, policy values, state file, canary, probe
    fn preview(&self, plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>>;
    fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyOutcome>;   // blocks on the OS prompt
    fn remove(&self, owner: ProductId, scope: RemoveScope) -> CoreResult<ApplyOutcome>;
}
pub enum ApplyOutcome { Applied, Partial(Vec<TierReport>), Declined, Unavailable(String) }
```

`SystemObservation` is raw text and lists. Turning it into `TierState`s is
`ubiqx_nofap::plan::assess`, so the adapter holds no decision.

### 2.6 Drift

There is no silent repair: re-applying needs the OS prompt, and there is no privileged
background process.

1. `NofapService::verify_pass()` (every 120 s, never elevates) computes the new status. A
   change emits `NofapEvent::Status`.
2. Once per drift episode and at most once every 6 hours, a calm card: "A proteção do sistema
   mudou. Quer restaurar?" with "Restaurar" (preview, then the OS prompt) and "Agora não".
   Nothing is called tampering; nothing is counted against the user; no timer starts.
3. The counter `drift` is incremented for the day.

### 2.7 Undo, uninstall, removal without the app

- `remove --owner <me> --scope all`: delete this owner from the state; if nobody else wants a
  tier, delete the hosts block, reset DNS where it is still ours, delete the drop-in, delete
  policy values and files that are ours; flush caches; delete the state file when no owner
  remains. Safe when half of it is already gone.
- `.deb`: `prerm` runs the installed binary with `--shield-helper remove --owner <id> --scope
  all` [U Tauri `bundle.linux.deb.preRemoveScript`], **only when its first argument is
  `remove`**. dpkg also runs `prerm` with `upgrade` on every package update, and an update
  must not take the protection off. NSIS: an uninstall hook does the same
  [U `bundle.windows.nsis.installerHooks`], skipped when the uninstaller runs as part of an
  update (`$UpdateMode`, [U]). The ubiqX installer is per-user and not elevated, so the hook
  starts the helper through the same `Start-Process -Verb RunAs` command as the app, which
  shows one UAC prompt; if the user declines, the uninstall finishes and the last page
  points to `REMOVAL.md`. macOS drag-to-trash and AppImage have no hook.
- `docs/nofap/REMOVAL.md` lists, per OS and packaging format: the button, the command
  (`sudo <app executable> --shield-helper remove --owner all --scope all`, or the helper
  binary, or an elevated prompt on Windows), and the manual steps with copy-paste commands
  (delete the lines between the markers; DNS back to automatic; delete the drop-in; delete
  each policy value and file). The help page links to it and says an administrator can
  always remove the protection.

---
## 3. The blocklist

### 3.1 Sources and licences

This build embeds the **Core tier only**. Every URL below answered HTTP 200 from this
sandbox on 2026-11-19 [V]; entry counts are the blocklist report's measurements of that day.

| Input | URL (pinned by commit and SHA-256 in `sources.lock.json`) | Entries | Licence | Use |
|---|---|---|---|---|
| StevenBlack `porn-only` | `raw.githubusercontent.com/StevenBlack/hosts/<sha>/alternates/porn-only/hosts` | 76,810 | MIT; two small inputs are CC BY 4.0 | Core |
| ShadowWhisperer `Adult` | `raw.githubusercontent.com/ShadowWhisperer/BlockLists/<sha>/Lists/Adult` | 213,982 | Unlicense | Core |
| 4skinSkywalker | `raw.githubusercontent.com/4skinSkywalker/Anti-Porn-HOSTS-File/<sha>/HOSTS.txt` | 70,174 | MIT | only entries carrying a block-grade keyword (the list also names ad trackers, `deviantart.com`, `vk.com`) |
| `extra-ptbr.txt` | ours, hand-checked | about 100 at first | ours | Core; the Brazilian gap (`privacy.com.br`, `closefans.com` are on no public list) |
| `never-block.txt` | ours | about 400 rules | ours | removed from every output; run-time built-in allowlist; helper refusal list |
| Public Suffix List | `raw.githubusercontent.com/publicsuffix/list/<sha>/public_suffix_list.dat` | | MPL-2.0 | build-time filter only, not shipped |
| Chrome UX Report top-1M | `raw.githubusercontent.com/zakird/crux-top-lists/<sha>/data/global/current.csv.gz` | | CC BY 4.0 | build-time: ordering of the hosts tier, the Popular table |

Result: about **239,449** entries. Not shipped: Bon-Appetit and UT1 (CC BY-SA 4.0, section
12), HaGeZi and oisd (GPL-3.0), blocklistproject (largely UT1-derived), mhhakim (no licence).
ShadowWhisperer squashes its history, so a pinned commit can disappear: the script then
fails and a maintainer re-pins.

Licence files: `THIRD-PARTY-NOTICES` at the repository root (text of
`research-blocklists.md` section 13, product name replaced), shown in About in both
products; `crates/ubiqx-nofap/data/NOTICE.txt` generated with the MIT copyright lines copied
by the script from the pinned commits (`license.txt`, `LICENSE` of StevenBlack, Sinfonietta,
bigdargon, 4skinSkywalker), the CC BY 4.0 credit for brijrajparmar27, tiuxo and CrUX, and
the statement that the lists were merged, filtered and hashed.

### 3.2 Build pipeline

`scripts/nofap/build_blocklist.py` (Python 3.11 standard library only: `urllib`, `hashlib`,
`gzip`, `json`, `struct`) is run by a maintainer before a release, never by `cargo build` and
never by CI. Its outputs are committed, so builds need no network and are reproducible.

Steps: fetch at the pinned commits and verify SHA-256 → parse (drop comments and the
`0.0.0.0` prefix; lower-case; strip trailing dots; drop IP literals and invalid names) →
strip `www.`, `m.`, `mobile.` when two labels remain → drop public-suffix entries (ICANN and
private sections) → drop `*.tumblr.com` and `*.blogspot.*` → 4skinSkywalker: keep only
block-grade-keyword entries → drop names equal to a `never-block.txt` rule → **never collapse
an entry to its registrable domain** (doing so "blocked" `apple.com` and `uol.com.br` in the
report's test) → add `extra-ptbr.txt` → suffix-deduplicate → write outputs.

| Output in `crates/ubiqx-nofap/data/` | Content | Size |
|---|---|---|
| `core.bin` | header `"UBXBL1\0\0"` (8), `format: u16 = 1`, `flags: u16`, `count: u32`, `list_version: u64` (UTC `YYYYMMDDHHMM`), then `count` sorted unique `u64` keys; all integers big-endian. Key = first 8 bytes of SHA-256 over `"ubiqx-nofap-blocklist-v1\0"` followed by the name | about 1.92 MB |
| `popular.bin` | same format, tag `"ubiqx-nofap-popular-v1\0"`: CrUX hostnames with rank ≤ 100,000 that are on no list and not on `mixed.txt` | about 0.8 MB |
| `hosts-tier.txt` | plain hostnames for L2, most visited first: CrUX origins with rank ≤ 500,000 covered by Core, plus the apex and `www.` of each covered registrable domain; at most 20,000 lines | about 0.4 MB |
| `never-block.txt`, `mixed.txt`, `never-capture.txt` | hand-maintained, embedded with `include_str!` | small |
| `canary-block.txt`, `canary-allow.txt` | a dozen names each | small |
| `sources.lock.json`, `core.meta.json`, `NOTICE.txt` | pins, counts and hashes, attribution | small |

`extra-ptbr.txt` is an input only; it is not embedded as text. Total added to each binary:
about 3.2 MB.

Script gates (any failure writes nothing): every canary-block name is covered and no
canary-allow name is; no public suffix, single label or IP; entry count within −10% / +25%
of the previous `core.meta.json`; no entry at or above the registrable domain of a CrUX
top-1,000 origin; byte-identical output on a second run; and a printed review list of every
listed name inside the CrUX top-10,000 (about 1,400 names at the first run) for a person to
read. `python3 scripts/nofap/build_blocklist.py --check` re-verifies the committed outputs
against `sources.lock.json` without network when the downloads are cached.

Why a hash table: no new dependency (`sha2` is in the workspace), memory equals file size,
lookup is a binary search, and the shipped file is not a readable directory of adult sites
on the disk of a person avoiding them. False match probability for a benign name is about
1.3e-14. The data lives in `ubiqx-nofap`, not `ubiqx-core`, because `services/ubi-api`
depends on `ubiqx-core`.

### 3.3 Host classes (hand-maintained data)

| File | What | Effect |
|---|---|---|
| `never-block.txt` | OS vendors, update and sign-in hosts, payment and banks, AI providers, `gov`, `gov.*`, `edu`, `edu.*`, `ac.*`, `mil`, `jus.br`, sexual-health and education sites (`scarleteen.com`, `plannedparenthood.org`), known keyword collisions (`xxxlutz.de`) | never blocked by list or heuristics; never accepted by the helper |
| `mixed.txt` | about 60 platforms that cannot be split by domain: Reddit, X, Tumblr, Imgur, Pinterest, VK, Telegram web, Discord, Twitch, DeviantArt, Pixiv, image-search pages | not blocked; vision at signal cadence; blockable one by one in "modo estrito" (a user block) |
| `never-capture.txt` | banks and payment, webmail, document and storage suites, health portals, government, password managers, the AI providers' own sites | never captured by vision, never blocked |
| `popular.bin` | 3.2 | captured by vision only when a local signal fires |

### 3.4 Matching semantics (`ubiqx_nofap::matcher`)

```rust
pub enum Verdict { Blocked(BlockSource), Allowed(AllowSource), Unknown(HostClass) }
pub enum BlockSource { User, Learned, List, Tld, Keyword }
pub enum AllowSource { User, Snooze, BuiltIn }
pub enum HostClass { Local, NeverCapture, Mixed, Popular, Plain { signal: bool } }

impl Matcher {
    pub fn new(core: &'static [u8], popular: &'static [u8], sites: &[NofapSite], salt: &[u8; 16],
               now: DateTime<Utc>) -> CoreResult<Self>;
    pub fn verdict(&self, host: &str) -> Verdict;
    pub fn verdict_for_url(&self, url: &str) -> (Verdict, bool /* path signal */);
    pub fn in_hosts_tier(&self, host: &str) -> bool;      // for the bypass check
}
```

1. **Normalise:** lower-case; strip a trailing dot, port, user-info and `www.`; `url::Url`
   already yields the punycode host. A single label, `localhost`, a loopback, private,
   link-local or unique-local IP literal, and a name under `.local`, `.localhost`,
   `.internal`, `.lan`, `.home.arpa`, `.test` or `.corp` is `Unknown(Local)`: a router page,
   a development server or an intranet is not an unknown website, so it is never blocked by
   a rule and never captured (4.2). A public IP literal is `Unknown(Plain { signal: false })`.
2. **Suffix walk:** test the host, then drop the left-most label, down to two labels. Every
   entry covers itself and its subdomains.
3. **Most specific match wins.** At equal specificity: user block, user allow (including an
   unexpired snooze), built-in never-block, learned, list.
4. **No match:** adult TLD, then keyword rules (3.5), then the class from `never-capture`,
   `mixed`, `popular`.
5. Learned sites are matched by `host_key = SHA-256(salt ‖ 0x00 ‖ name)[..16]` for the host
   and each parent.

The matcher is rebuilt only when `nofap_sites` changes. The list never enters SQLite and
never becomes `focus_targets`.

### 3.5 Keyword and TLD rules (`ubiqx_nofap::heuristics`)

Applied only to hosts that no list, user entry or never-block rule decides.

| Grade | Rule | Effect |
|---|---|---|
| Block | TLD `xxx`, `porn`, `sex`, `sexy`, `adult` | `Blocked(Tld)` |
| Block | substring: `porn`, `xxx`, `hentai`, `blowjob`, `gangbang`, `cumshot`, `handjob`, `bukkake`, `deepthroat`, `creampie`, `camgirl`, `shemale`, `upskirt`, `masturbat`, `bokep`, `phimsex`, `nudes`, `putas`, `escorts`; PT-BR `putaria`, `buceta`, `bucetinha`, `siririca`, `contoseroticos`, `brasileirinhas`, `pornozao`, `safadas`, `novinhas`, `gostosas`, `peladas`, `transando`, `gozando` | `Blocked(Keyword)` |
| Block | brand stem after look-alike folding (`0→o 1→i 3→e 4→a 5→s 7→t vv→w`, edge digits stripped): the 31 stems of `research-blocklists.md` 9.1 (`pornhub`, `xvideos`, `xnxx`, `xhamster`, `redtube`, `youporn`, `spankbang`, `chaturbate`, `stripchat`, `onlyfans`, `missav`, `rule34`, …) | `Blocked(Keyword)` |
| Signal | whole token: `milf`, `sexy`, `adult`, `escort`, `naked`, `nude`, `pussy` (not followed by digits), `sexo`, `bdsm`, `webcam`, `cams`, `erotic`, `erotica`, `fetish`, `voyeur`, `fuck`, `amadoras`, `acompanhantes`; TLD `cam`, `webcam`, `tube`; a block-grade word in the **path** of a mixed host; a block-grade word in the **window title** | `signal: true`: raises vision priority, never blocks |
| Never | `sex`, `anal`, `cum`, `ass`, `cock`, `dick`, `tits`, `cam`, `jav`, `gay`, `teen`, `incest`, `pau`, `rola`, `pica`, `puta`, `bunda`, `pelada`, `foda` | not used at all. `gay` is never a signal: an orientation is not a risk marker. |

A keyword or TLD block carries "Não era isso" on its card (5.4).

### 3.6 Test cases the matcher must pass

| Input | Expected | What it guards |
|---|---|---|
| each name in `canary-block.txt`; `PT.PornHub.com.`; `www.xvideos.com:443` | `Blocked(List)` | normalisation and the parent walk on the embedded table |
| each name in `canary-allow.txt` (`wikipedia.org`, `reddit.com`, `youtube.com`, `apple.com`, `uol.com.br`, `globo.com`, `scarleteen.com`, …) | not `Blocked` | upstream mistakes, the collapse trap |
| synthetic table with `adult.contents.fc2.com` listed and `fc2.com` on never-block | subdomain `Blocked(List)`; `fc2.com` and `blog.fc2.com` not blocked | entry kept at its listed depth; longest match |
| `porno365.golf`, `missav123.com`, `xvv1deos.com`, `anything.xxx`, `videosdeputaria.com.br` | `Blocked(Keyword)` or `Blocked(Tld)` | block-grade rules, look-alike folding |
| `essex.gov.uk`, `sussex.ac.uk`, `sussexexpress.co.uk`, `planalto.gov.br`, `canaltech.com.br`, `motorola.com.br`, `drogariasaopaulo.com.br`, `peacocktv.com`, `emilfrey.de`, `milfordasset.com`, `lichngaytot.com`, `coroasparavelorio.com.br`, `nakedwines.com`, `skylinewebcams.com`, `iowasexoffender.gov`, `pussy888.com`, `cambridge.org`, `xxxlutz.de`, `scunthorpe.co.uk`, `penistone.co.uk`, `ninjavan.co`, `fapesp.br` | not `Blocked` | the false-positive traps of the report |
| `https://www.reddit.com/r/EarthPorn/` | `Unknown(Mixed)` with the path signal, not blocked | a keyword in a path is never a block |
| `192.168.0.1`, `[::1]`, `localhost`, `nas.local`, `intranet`, empty | `Unknown(Local)` | no match on literals; local hosts are never captured |
| `93.184.216.34` | `Unknown(Plain)` | a public address is still an unknown site |
| user block `example.com` + user allow `example.com` | `Blocked(User)` | precedence |
| user allow `sub.listed.example` under a listed parent | `Allowed(User)` | longest match |
| expired snooze | ignored | time handling |
| learned host | `Blocked(Learned)`, and a dump of the database holds no byte string equal to the host | hashing |
| malformed table: bad magic, unsorted, duplicate keys, wrong count | `Matcher::new` fails | table validation |
| hashing vector shared with the Python script (`core.meta.json.vector`) | equal | the two implementations agree |

### 3.7 User list, learned sites

Table `nofap_sites` (7.1).

| Entry | Created by | Effect | Stored as |
|---|---|---|---|
| block / user | "Adicionar site"; the ticks of "modo estrito" on mixed platforms; "Adicionar à minha lista" on a vision card | L1 at once; L2 at the next apply the user starts | plain text (the user reads and edits their own list), and in the world-readable hosts block. The add dialog says so and offers "guardar só neste app" (`hosts_user_sites = false`). |
| block / learned | a vision Positive on a host of class `Plain` (never mixed, popular, never-capture or never-block) | L1 on every later visit, no capture | **salted hash only**, `host = NULL`. Never in the hosts file, never shown by name, never exported. The page shows the count and "Esquecer sites aprendidos". |
| allow / user | "Nunca bloquear" on the page | beats list and heuristics for that name | plain text |
| allow / snooze | "Não era isso" (5.4) | short, expiring | salted hash, `expires_at` |

Honest limit, in the privacy text: the salt is stored beside the hashes, so someone with
access to the user's account can test whether one specific site is in the set; they cannot
list the set.

Visited sites that were blocked by the list or by a keyword are counted, never named and
never stored. There is no promotion of visited sites into the hosts block.

### 3.8 Updates

The list is refreshed with the app: a maintainer runs the script before a release and commits
the outputs. The signed weekly pack with the share-alike Extended tier is deferred (section
12); `Matcher::new` taking the tables as byte slices is the seam.

---

## 4. Vision (L5)

The layer for what DNS and lists cannot see: unknown sites, mixed platforms, local files in a
media player, content reached through a VPN. A screenshot is the most sensitive thing this
app can create, so a check runs only when every local rule agrees, sends one reduced image
and nothing else, and keeps nothing.

### 4.1 Readiness (read-only; independent of the productivity AI in both directions)

`NofapService::vision_ready() -> Result<(), VisionOff>` never calls
`classify::budget_allows` (it sets `AiHealth::Paused` and emits a nudge,
`crates/ubiqx-engine/src/classify.rs:105-145`) and never reads `remote_allowed()` or
`AiHealth` (`state.rs:228-243`). It checks, in order:

| Check | Source | `VisionOff` reason |
|---|---|---|
| `vision.enabled` | `NofapConfig` | `disabled` |
| consent recorded for the provider now selected (and, for "IA do Ubi", the managed tick) | `NofapConfig.vision.consent`, `NofapEnv` | `consent_outdated` |
| a detector is wired | `NofapDeps.detector` | `no_provider` |
| `NofapEnv::snapshot().ai == Ready`: a key for the selected provider (or a valid `monthly_managed` licence), `local_only` false, ubiqX onboarding done | host | `no_key`, `local_only`, `licence` |
| Screen Recording granted (macOS); not Wayland | `PermissionChecker` | `screen_recording`, `wayland` |
| vision's own month total under `vision.monthly_cap_usd` | `UsageRepo::totals_by_kind` (new), kind `nofap_vision` only | `budget_spent` |
| hourly and daily caps | in-memory counters + `nofap_day` | `cap_reached` |
| vision's own health not in backoff | `NofapRuntime` | `degraded { until }` |

The other direction: `classify::budget_allows` sums every kind except `nofap_vision`, so a
month of checks cannot pause classification. `local_only`, removing the API key and changing
provider stop vision at once and cannot be delayed: they are privacy rights. The page shows
the reason, and each one counts as an override for the help heuristic (5.5).

`key_health` is not touched and the standalone app wires no classifier.

### 4.2 What is a candidate

Decided by the pure function `ubiqx_nofap::sampling::decide(&SampleCtx, &SamplerMemory, now)`.

| Foreground | Candidate? |
|---|---|
| Browser, host `Blocked` or `Allowed` | never (already decided) |
| Browser, host `NeverCapture` | never |
| Browser, host `Popular` | only with a signal |
| Browser, host `Mixed` | yes, signal cadence |
| Browser, host `Plain` | yes when `vision.unknown_sites` (default on) |
| Browser, host `Local` | never |
| Browser whose URL cannot be read: a supported browser that gave no URL, or an app on `OTHER_BROWSERS` (data in `sampling.rs`: bundle ids and executable stems of browsers the URL resolver does not support on that system, such as Firefox forks, Tor Browser, Mullvad Browser, LibreWolf, Zen, Floorp, Waterfox, Orion, DuckDuckGo, Opera and Opera GX, Yandex, Epiphany, Falkon) | yes when `vision.unknown_sites`. Without this list an unsupported browser would be "any other app" and escape both the guard and vision. |
| Private-browsing window | as its host says, when `vision.private_windows` (default on; the consent says so); the private title is a signal |
| Media players and image viewers (`MEDIA_APPS`: VLC, mpv, IINA, QuickTime Player, Windows Media Player, Movies & TV, Photos, Preview, Celluloid, Totem, eog) | when `vision.media_apps` (default on) |
| Messaging and mail apps (`redact::is_messaging_app`) | only when `vision.messaging_apps` (**default off**: other people's messages) |
| Any other app | never |
| A window without an OS window id | never; there is no display or region fallback |
| Password managers, keychain tools, both products' own windows (`NEVER_CAPTURE_APPS`) | never |
| An app or domain on ubiqX's privacy lists (`blocked_apps`, `vision_denied_apps`, `blocked_domains`) | never, once the exclusion is effective (below) |

**Exclusions are honoured, but are not an instant off switch.**
`ubiqx_nofap::exclusions::effective(lists, seen, delay, now)`: an entry present when the
module was enabled is effective at once; an entry added to those Settings lists later is
effective for vision after the commitment delay (it is effective for the tracker at once, as
today). `reconcile` records first-seen times in `NofapConfig.internal.exclusions_seen`. The
Settings hint says: "O {name} passa a respeitar esta exclusão em {delay}."

A **signal** is any of: a signal-grade word in the host or path (3.5), a block-grade word in
the window title, a private-browsing title, a mixed host, the heightened state, or a
`suggestive`, `unsure` or refused answer for this key in the last 30 minutes.
The **heightened state** lasts 15 minutes after any block or bypass suspicion
(`Issue::DnsBypassSuspected`: a host that is in the applied hosts block was seen in front).

### 4.3 Cadence and caps

Keys are 8-byte hashes of the host, or of app id + title for windows without a URL, kept in
memory only and gone at exit.

| Context | First check after | Then | Relaxes |
|---|---|---|---|
| No signal | 15 s in front | 90 s later, then every 300 s | after 2 `none` in a row: every 600 s, for 60 minutes; any signal ends it |
| Signal | 5 s | every 45 s | not while the signal holds |
| Media app | 20 s | every 180 s | every 600 s after 3 `none` in a row |
| Messaging app (opt-in) | 30 s | every 300 s | no |

- **No idle gate.** Watching video produces no input, so idleness must not stop checks.
- **Unchanged frame.** A frame whose 64-bit difference hash is within 4 bits of the last
  frame answered `none` for the same key, less than 10 minutes ago, is not sent and not
  counted against the caps.
- Caps: 6 per key per hour; `vision.max_per_hour` 20 (1 to 60); `vision.max_per_day` 120
  (1 to 1,000); `vision.monthly_cap_usd` 2.00 (0.25 to 20). One check in flight at a time.
- Errors: timeout, network, 5xx, rate limit → that key waits 300 s; three in a row → vision
  health `degraded` with steps of 1, 5, 15 minutes. Key rejected three times → `no_key` and
  one calm in-app note. Nothing is queued: a screen that was not checked is never checked
  later.

### 4.4 What is captured and sent

- New port `FrameSource` (`ubiqx_core::nofap`), one portable implementation
  `ubiqx_platform::frame::WindowFrameSource { capturer }`:

  ```rust
  pub trait FrameSource: Send + Sync {
      fn frame(&self, window_id: u32, opts: FrameOpts) -> CoreResult<Frame>;
  }
  pub struct FrameOpts { pub max_edge: u32, pub crop_top_frac: f32 }
  pub struct Frame { pub image: EncodedImage, pub dhash: u64 }
  ```

  It calls the existing `ScreenCapturer::capture(CaptureTarget::Window(id), 1280)`, decodes,
  crops the top `0.12` of a browser window (tab strip, address bar, bookmarks: other tabs'
  titles) and nothing of other apps, downscales to `NOFAP_MAX_EDGE = 640`, computes the
  hash, and encodes JPEG quality 60. No per-OS code is added. A near-uniform capture is
  "no frame".
- The bytes stay in memory: no file, no `screenshots` row, no thumbnail. They are dropped
  when the call returns.
- The request carries the image and two constant strings. **No URL, title, app name,
  language, user profile or identifier.** A unit test asserts the only text parts are the
  constants.
- Destination: the provider in settings, with `settings.models.vision`. `AiModels` gets no
  new field (it has no serde default; a new field would break stored settings).
- **"IA do Ubi"** (ubiqX only): the image passes through the operator's proxy on its way to
  Anthropic. It is used only after its own consent tick under the sentence "As imagens da
  verificação passam pelo servidor do ubiqX a caminho da Anthropic. O servidor não guarda
  imagens. Ele registra que uma verificação foi feita (hora e custo), não o que havia na
  tela." The request is not tagged by feature, but the ledger records time, model and token
  counts for every request, and a run of small image requests is recognisable, so the
  operator can tell that a subscriber uses the check. The sentence above says so; the
  earlier claim that the ledger "does not learn" it was wrong. `services/ubi-api` gets an explicit `DefaultBodyLimit` of
  4 MiB on `/v1/messages`, a test that an image block passes through unchanged, and a test
  that no request body reaches the log.
- **Trace logging.** `trace!(vendor, body = %body, "request body")` at
  `crates/ubiqx-ai/src/client.rs:745` and `crates/ubiqx-ai/src/openai.rs:689` prints the
  base64 image. Both lines log through `redact_images(&body)`, which replaces image data with
  `<image: N bytes>`, for every request kind, with a unit test.
- **No log line per check.** Today every completed call writes an `info!` line with the
  kind, the token counts and the stop reason (`client.rs:797-810`, the twin in `openai.rs`),
  a `debug!` line before sending, and a `trace!` line with the response text
  (`client.rs:811`, `openai.rs:757`). The desktop shell writes `info` and above to a log
  file (`tauri_plugin_log`, `apps/desktop/src-tauri/src/lib.rs:513`). Left alone, that file
  would hold one timestamped line per screen check, with `stop_reason = refusal` marking the
  ones a model declined, and a trace log would hold the verdicts. For
  `AiUsageKind::NofapVision` the client therefore logs nothing on the success path at any
  level (the three lines are skipped), and failure lines carry no kind. `ubiqx_engine::nofap`
  logs state changes only (enabled, tier applied, vision degraded) and never a hit, a
  verdict, a host, a title or an app. A test installs a capturing subscriber at `trace`,
  runs the scripted session of 11.2 (24) and asserts that no captured line contains
  `nofap_vision`, a verdict word or a host of the scenario, and that the number of lines
  does not grow with the number of checks.
- **What the provider does with the image is outside the app.** Providers run their own
  safety systems on what they receive and keep it for the period their terms state. An
  image a provider judges illegal can lead it to suspend the key or to report it, as its
  terms and the law where it operates require. The consent names the provider and says it
  applies its own rules and retention; `PRIVACY.md` says this in plain words. The app adds
  no reporting of its own and keeps nothing.
- **"Ver um exemplo do que é enviado"**: when the user presses the button the window in
  front is the app's own, which is never captured. The service therefore remembers, in
  memory only, the window id of the last window that was a vision candidate, and captures
  that window with the same options if it still exists; otherwise the page says "Abra um
  site qualquer e volte aqui". The image is shown in the page, is not sent and is not
  stored.

### 4.5 Prompt and response

System prompt (`ubiqx_ai::prompts::EXPLICIT_SYSTEM_PROMPT`, English for every provider):

```
You are a content classifier inside a self-control app that an adult installed on their own
computer to stay away from pornography. You receive one downscaled screenshot of the window
that person is looking at. Classify what is visible.

explicit   - pornography: sexual acts, exposed genitals or anus, or nudity presented for
             sexual arousal. Photographs, video frames, drawings, animation or generated
             images all count. A page of thumbnails of such content counts.
suggestive - sexualised but not explicit: erotic posing in lingerie or swimwear, partial
             nudity, blurred or cropped previews, an adult site's landing or age-check page.
none       - everything else, including medical or educational anatomy, breastfeeding,
             classical art, sport, beach and family photographs, news, and pages that are
             mostly text.
unsure     - the image is too small, dark or ambiguous to tell.

Give your confidence in the verdict as high, medium or low.
Do not describe the image. Do not transcribe text. Do not identify anyone.
Answer with the JSON object only.
```

User content: `LlmContent::Image { .. }` then `LlmContent::Text("Classify this screenshot.")`.

Schema (closed, all required; no free-text field, so the model cannot return a description):

```json
{ "type": "object",
  "properties": {
    "verdict":    { "type": "string", "enum": ["explicit", "suggestive", "none", "unsure"] },
    "confidence": { "type": "string", "enum": ["high", "medium", "low"] } },
  "required": ["verdict", "confidence"], "additionalProperties": false }
```

Request: `max_tokens = 64` (one retry at 256 on `StopReason::MaxTokens`),
`json_schema = Some(schema)`, `disable_thinking = thinking_disabled_for(&model)`,
`usage_kind = AiUsageKind::NofapVision` (`"nofap_vision"`, added to the `matches!` in
`openai.rs::reasoning_effort_for` so `gpt-5*` runs at minimal effort),
`timeout = Some(10 s)`. The service wraps the call in `tokio::time::timeout(14 s)`, because
the retry policy is per client (4 attempts, backoff to 30 s), not per request.

```rust
// ubiqx_core::nofap
pub enum ExplicitVerdict { Explicit, Suggestive, None, Unsure }
pub enum Confidence { High, Medium, Low }
pub struct ExplicitCheck { pub verdict: ExplicitVerdict, pub confidence: Confidence, pub refused: bool }
#[async_trait]
pub trait ExplicitDetector: Send + Sync {
    async fn check(&self, image: &EncodedImage, model: &str) -> CoreResult<ExplicitCheck>;
}
```

`LlmExplicitDetector::check` maps `Err(CoreError::AiRefused)` to
`ExplicitCheck { verdict: Unsure, confidence: Low, refused: true }`. Invalid JSON or an
unknown verdict string is `Unsure`. No provider-specific code: `RoutingLlmClient` already
produces the Anthropic image block and the OpenAI/xAI `image_url` data URL.

### 4.6 Thresholds and reactions (`ubiqx_nofap::sampling::outcome`)

| Answer | Outcome | What happens |
|---|---|---|
| `explicit` + `high` | **Positive** | Re-read the foreground and, in a browser, `resolve_fresh`: act only when the window id and the host are those that were captured, otherwise drop the verdict. Browser: close the tab (1.2); learn the host if its class is `Plain`; card `vision.closed` with "Ok", "Não era isso", "Respirar 3 min", "Adicionar à minha lista". Other app: `hide_others`, card `vision.soft` with "Fechar a janela" and "Não era isso". Heightened state starts. |
| `explicit` + `medium` | **Confirm** | a second frame 4 s later; `explicit` at `medium` or `high` again → Positive, otherwise Unsure |
| `explicit` + `low` | **Unsure** | no action on the window; the key gets a signal for 30 minutes; card `vision.soft` ("Posso estar enganado…") with "Fechar" and "Não era isso", at most once per key per 30 minutes |
| `suggestive`, `unsure` | **Watch** | nothing shown; signal for 30 minutes |
| refused | **Watch** | a refusal alone is never a Positive. Two refusals in a row on a key that also has a local signal show the `vision.soft` card. |
| `none` | **Clean** | counts towards relaxing |

A vision hit is never recorded as a lapse.

### 4.7 Cost

Estimates, not measurements (`map-vision-ai.md` section 9; the price table is
`crates/ubiqx-ai/src/pricing.rs`, `claude-haiku-4-5` at US$ 1 / 5 per million tokens in /
out): a 640×400 image is about 340 image tokens, the prompt about 260, the answer about 15,
so roughly **US$ 0.0007 per check** on `claude-haiku-4-5` or `ubi-fast`; OpenAI and xAI
default models are cheaper per token but their image tokenisation is not in the code. The
daily cap of 120 bounds a day at about US$ 0.08 and the monthly cap of US$ 2.00 buys about
2,900 checks. Expected use is 20 to 50 checks a day. Checks are recorded in `ai_usage` under
their own kind, and the module's page shows its own month total (`cost_line` in `NofapView`,
from `totals_by_kind`); ubiqX's Settings total includes it, as it sums every kind. The wizard shows the estimate and
the caps. Real cost per provider is a checklist item (11.4).

### 4.8 What is stored

Per check: the day and hour counter `vision_check` and one `ai_usage` row (kind, model,
tokens, cost) whose `at` is truncated to the start of the hour for this kind
(`AiUsage::at` is set by the client; for `NofapVision` it is `now - now % 3600 s`). The
ledger has millisecond times for every other kind, and an exact time per screen check would
be the per-event trail that 7.3 says does not exist. Monthly totals, which is all the budget
needs, are unchanged. Per Positive: counter `vision_hit`, and the learned host as a salted
hash. Never the image, a hash of it, the title, the URL or a per-verdict row, and no log
line (4.4).

The page shows: "Hoje: {n} verificações. Cada uma envia uma imagem reduzida da janela ativa
para {provider}. Nada é guardado neste computador."

---
## 5. Counselling

### 5.1 Rules that tests enforce

The module is "a barrier plus a calm coach", not a treatment. Every string shown to the user
comes from the reviewed bank in 5.6. The model returns a class, never a sentence.

1. The user decided; UBI keeps the agreement ("como combinamos").
2. One or two sentences and one action per screen.
3. Tentative about anything vision saw ("Posso estar enganado").
4. No diagnosis, no neuroscience, no numbers about other people, no promise, no success
   rate. Not claimed anywhere (product, site, store text): addiction, dopamine reset or
   "reboot", sexual-function or hormone benefits, "superpowers", that masturbation is harmful
   or that abstinence is the goal, that the block cannot be undone.
5. The user sets the goal. There is no masturbation-abstinence mode.
6. No assumption about gender, partner, religion or orientation; gender-neutral Portuguese.
7. A path to a human one step from every screen of the module.
8. Discreet by default: the first thing on screen after a block is "Página bloqueada pelo
   UBI." and nothing about why.

`crates/ubiqx-nofap/data/forbidden-words.json` (read by a Rust test over
`ubiqx_nofap::messages` and `resources.json`, and by a Vitest test over the namespaces
`nofap` and `nofap_app`), matched as whole words, case- and accent-insensitively (an entry
ending in `*` is a stem): `viciado`, `vício`,
`dependente`, `dependência`, `recaída`, `fracass*`, `falhou`, `limpo`, `sujo`, `puro`,
`pecado`, `vergonha`, `força de vontade`, `homem de verdade`, `seja forte`, `dopamina`,
`reprogramar`, `reboot`, `detox`, `comprovado`, `nunca mais`, `para sempre`, `sua esposa`,
`sua parceira`, `fapstronaut`, and the English twins (`addict*`, `relapse*`,
`clean`, `dirty`, `pure`, `sin`, `shame`, `willpower`, `real man`, `dopamine`, `rewire`,
`reboot`, `detox`, `proven`, `never again`, `forever`).

### 5.2 The wizard (consent)

On the module's page in ubiqX; as the onboarding of the standalone app. Nothing is applied
before the last button, and nothing in it is delayed (the delay starts once the module is on).

| Step | Content | Recorded |
|---|---|---|
| 1 "O que é" | `wizard.what`, `wizard.limits`, `wizard.adult` (5.6). The user ticks "É para mim, no meu computador". | `self_use_at` |
| 2 "Bloqueio no sistema" | the three tiers with their states (a gated tier is shown as such); the DNS disclosure (2.3); the policy disclosure (2.4); "Abrir junto com o computador" (on by default, a visible switch, never forced); the exact list of changes from `preview` | `hosts`, `resolver`, `policies` |
| 3 "Verificação por IA" | on by default when a provider is configured, and it needs the unticked box "Entendi o que é enviado" under `vision.consent` (5.6) naming the provider; the cost line and caps; switches for unknown sites, media apps, messaging apps (off), private windows; for "IA do Ubi" the second tick; "Ver um exemplo do que é enviado" | `vision.*`, `vision.consent { provider, managed, at }` |
| 4 "Seu combinado" | one optional values sentence (`checkin.values`); the delay (0, 5, 10, 30, 60 minutes, 4 h, 24 h; default 10 minutes); detailed messages (off) | `delay_secs`, `values`, `discreet` |
| 5 "Resumo" | what is stored (counts per day and hour, the sites the user typed, no visited site, no image), how to turn it off, how to remove it without the app | `enabled_at` |

Changing the AI provider later pauses vision until the user confirms the new one
(`Issue::ConsentOutdated`).

### 5.3 The card window

A second floating window in each shell: label `shield-card`, route `#/nofap-card?id=<id>` outside
`Gate`, capability file `capabilities/shield-card.json` (`core:default` plus close, hide,
set-focus, start-dragging for that label), title "UBI", always on top, on all workspaces,
shown without taking focus. It does not reuse the `intervention` window (which closes after
7 s and falls back to a YouTube sample). UBI is in the calm `idle` mood, not `worried`.

```rust
pub struct NofapCard {
    pub id: Id,
    pub kind: CardKind,            // blocked | blocked_repeat | vision_closed | vision_ask | drift
                                   // | pending_ready | resolver_unreachable | reminder | help
    pub message_key: String,       // key in the `nofap` namespace
    pub discreet: bool,
    pub actions: Vec<CardAction>,  // ok | support | surf | close_window | not_it | add_to_list
                                   // | restore | keep_protection | turn_off | restore_network | help
    pub expires_at: DateTime<Utc>,
}
```

The type has **no host, title, URL or app field**, asserted on the serialised JSON. The
engine keeps what a button needs (app id, host, window id) in memory, keyed by card id, for
10 minutes. The last 20 cards live in memory for `NofapCommand::Card { id }`; an unknown id
renders the neutral discreet card, never a sample.

| Card | Trigger | Size, lifetime | Message | Buttons |
|---|---|---|---|---|
| Blocked, discreet (default) | list, user, learned, keyword or TLD hit | 420×132, 12 s | `block.discreet` | "Abrir apoio", "Ok" |
| Support (after "Abrir apoio", or directly when detailed messages are on) | | 420×560, until closed | rotation of `block.breathe`, `block.choice`, `block.next` (and `block.default` for a hit on the user's own list); the values sentence; the if-then plan | "Respirar 3 min", "Sair da tela por 10 min", "Falar com alguém", "Fechar" |
| Blocked, repeat | third hit of an episode | as Support | `block.repeat` | "Fazer a onda de 3 min", "Ok" |
| Vision closed | 4.6 Positive in a browser | 420×220, until answered | `vision.closed`, then `vision.added` when learned | "Ok", "Não era isso", "Respirar 3 min", "Adicionar à minha lista" |
| Vision ask | 4.6 Unsure; Positive in another app | 420×200, 60 s | `vision.soft` | "Fechar a janela", "Não era isso" |
| Drift | 2.6 | 420×200 | `drift.card` | "Restaurar", "Agora não" |
| Pending ready | 6.2 | 420×220 | `pending.ready` | "Manter proteção" (primary), "Desligar" |
| Resolver unreachable | 2.3 | 420×220 | `resolver.unreachable` | "Restaurar minha rede", "Agora não" |
| Reminder | every 30 days while on | 420×160 | `reminder.on` | "Ok", "Como desligar" |

Order: enforcement first, then the card. At most one card per 60 s; an open card or surf
session is not replaced. When the window cannot be shown, the fallback OS notification is
title "UBI", body `block.discreet` (or `drift.notice`), from `ubiqx_nofap::messages`. The
operating system keeps its own notification history, with times, which the app cannot
erase; the fallback is therefore sent at most once per episode, and `PRIVACY.md` says where
that history is cleared on each system.

### 5.4 "Não era isso" (a correction, not an off switch)

| On | Effect |
|---|---|
| a vision card | The host learned by that verdict is unlearned and nothing more is done to the window for now. One fresh frame is taken 20 s later (`NOT_IT_RECHECK_SECS`), whatever the caps say. If that second, independent check is again `explicit` + `high`, the tab is closed, the host is learned again, and for the rest of the day the button on that key reads "Pedir exceção" (a delayed allow request). Any other answer leaves the key at signal cadence. A press costs a wrong verdict 20 s and buys a right one 20 s. The earlier rule (no action on that key for 10 minutes) was a one-click pass, three times a day. |
| a keyword or TLD block, while vision is ready | The host is exempt from the keyword and TLD rules for 24 hours (`allow / snooze`). Vision still samples it at signal cadence. |
| a keyword or TLD block, while vision is not ready (off, no key, cap or budget reached, Wayland) | Creates an allow request, which waits the delay, as for a list hit. With nothing left to look at the page, a snooze would be a one-click pass for a day. |
| a list hit or a hit on the user's own list | Creates an allow request, which waits the delay (section 6). The card says so. |

At most `MAX_SNOOZES_PER_DAY = 3`; after that the button reads "Pedir exceção" and creates a
delayed allow request. Each use increments the counter `not_it`.

### 5.5 Flows

| Flow | Entry | Steps | Counter | Backed by |
|---|---|---|---|---|
| **Urge surf** (3 minutes) | every card, the page, the tray, the panic sheet | `surf.start`, `surf.body`, `surf.breath`, `surf.wave`, `surf.thought` (30 s each), `surf.rate` (0 to 10, shown back in the session, not stored), `surf.end` | `surf_started`, `surf_done` | mindfulness-based relapse prevention, ACT. The copy says the exercise changes what you do with the urge, not that the urge shrinks. |
| **Panic button** | tray item "Preciso de um respiro"; the page; every card | `panic.open` → `panic.options`: surf / step away (`panic.away`: `Enforcer::hide_others`, a 10-minute countdown, nothing locked) / talk to someone (crisis card) | `panic` | delay and distract |
| **Lapse** | "Aconteceu um deslize" on the page, one tap; only the user starts it | `lapse.first`, `lapse.kind` → optional trigger chips (`lapse.learn`) → optional plan (`lapse.plan`) → `lapse.resume` | `lapse`, `trigger_*` | abstinence-violation effect, self-compassion |
| **Check-in** | a card on the page, never a notification; off by default | `checkin.daily`: four moods; on "difícil" the chips of `checkin.halt` | `checkin_ok`, `_meh`, `_hard`, `_very_bad`, `trigger_*` | HALT as a prompt, not as a fact |
| **Values and plans** | the page | one sentence (`checkin.values`, at most 280 characters), up to three "se… então…" plans (`checkin.ifthen`); shown back on Support cards, in the panic sheet (`panic.reason`) and while a change is pending | in `NofapConfig` | ACT; implementation intentions |
| **Progress** | the page | `progress.days`. The streak exists, is off by default, is small and never animated; after a lapse it shows `streak.after_reset`. Milestones on the page only. | derived from `nofap_day` | broken streaks lower engagement |
| **Professional help** | card on the page and in the window, at most once in 7 days | when, in the last 7 days: `HELP_LAPSES_7D = 5` lapses, or `HELP_OVERRIDES_7D = 10` override requests, or `HELP_VERY_BAD_7D = 2` "muito difícil" check-ins. `help.pattern`, `help.how`, `help.limits`. Constants in `ubiqx_nofap::help`; product heuristics, not clinical cut-offs, and the card says "isso não é um diagnóstico". | `help_shown` | ICD-11 description; the treatment gap |
| **Crisis card** | one step from every screen of the module; first in the panic sheet's "falar com alguém" | `panic.crisis` and the resource list | none | referral research |

Not built: the evening prompt (`checkin.evening` needs a pattern detector), programmes,
questionnaires, any reply written by a model, accountability partners.

### 5.6 Copy bank (final strings; namespace `nofap`, PT-BR and EN)

`{name}` is `DISPLAY_NAME`. `{provider}`, `{delay}`, `{days}`, `{n}`, `{reason}` are computed
locally. Phone numbers and addresses come from `resources.json` (5.7).

**Blocks and vision**

| Key | PT-BR | English |
|---|---|---|
| `block.discreet` | Página bloqueada pelo UBI. | Page blocked by UBI. |
| `block.default` | Esse endereço está na lista que você montou. Segurei a porta pra você. | That address is on the list you set up. I held the door for you. |
| `block.breathe` | Bloqueado, como combinamos. Respira comigo por um minuto? | Blocked, as we agreed. Breathe with me for a minute? |
| `block.choice` | Você decidiu isso num momento de calma. Vale confiar naquela decisão agora. | You decided this in a calm moment. It's worth trusting that decision now. |
| `block.next` | A vontade veio. Tudo bem, ela passa. O que você ia fazer antes disso? | The urge showed up. That's okay, it passes. What were you about to do before this? |
| `block.repeat` | Terceira tentativa em pouco tempo. Parece um momento difícil. Quer fazer a onda de 3 minutos comigo? | Third try in a short while. Looks like a tough moment. Want to ride the 3-minute wave with me? |
| `vision.soft` | Posso estar enganado, mas o que está na tela parece fugir do seu combinado. Quer que eu feche? | I might be wrong, but what's on screen looks outside your plan. Want me to close it? |
| `vision.closed` | Fechei a aba, como você pediu que eu fizesse. Nenhuma imagem ficou guardada neste computador. | I closed the tab, as you asked me to. No image was kept on this computer. |
| `vision.added` | Guardei esse site pra ele não te pegar de surpresa de novo. | I'll remember that site so it won't catch you off guard again. |
| `vision.wrong` | Errei? Me avisa aqui e eu ajusto. Sem drama. | Did I get it wrong? Tell me here and I'll adjust. No fuss. |
| `vision.after` | Passou. Sem sermão. Quer dois minutos pra respirar ou prefere seguir o dia? | It's over. No lecture. Want two minutes to breathe, or would you rather get on with your day? |
| `notit.snoozed` | Anotado. Vou deixar passar por enquanto. | Noted. I'll let it through for now. |
| `notit.request` | Esse site está na lista. Pedi a exceção; ela vale daqui a {delay}. | That site is on the list. I've requested the exception; it applies in {delay}. |

**Urge surf**

| Key | PT-BR | English |
|---|---|---|
| `surf.start` | Vamos surfar essa vontade juntos. São 3 minutos. Você não precisa lutar contra ela, só observar. | Let's surf this urge together. Three minutes. You don't have to fight it, only watch it. |
| `surf.body` | Onde você sente a vontade no corpo? Peito, barriga, mãos? Só repare. | Where do you feel the urge in your body? Chest, stomach, hands? Just notice. |
| `surf.breath` | Inspira contando até 4… solta contando até 6. Deixa o ar fazer o trabalho. | Breathe in for 4… out for 6. Let the breath do the work. |
| `surf.wave` | Vontade é onda: sobe, chega no pico e desce. Você está em cima da prancha. | An urge is a wave: it rises, peaks and falls. You're on the board. |
| `surf.thought` | Se vier um pensamento tipo "só dessa vez", repare nele e deixe passar. Pensamento não é ordem. | If a thought like "just this once" shows up, notice it and let it go by. A thought is not an order. |
| `surf.rate` | De 0 a 10, como está a vontade agora? | From 0 to 10, how strong is the urge now? |
| `surf.end` | Pronto. Você ficou com a vontade sem obedecer. Isso é treino, e conta. | Done. You stayed with the urge without obeying it. That is practice, and it counts. |

**Lapse, progress, check-in**

| Key | PT-BR | English |
|---|---|---|
| `lapse.first` | Aconteceu. Um deslize não apaga o que você já construiu. | It happened. One slip doesn't erase what you've built. |
| `lapse.kind` | Fale com você como falaria com um amigo na mesma situação. | Talk to yourself the way you'd talk to a friend in the same spot. |
| `lapse.learn` | Sem culpa, só curiosidade: o que estava acontecendo antes? Cansaço, tédio, estresse, solidão? | No blame, only curiosity: what was going on beforehand? Tiredness, boredom, stress, loneliness? |
| `lapse.plan` | Quer ajustar um plano "se… então…" pra próxima vez que esse gatilho aparecer? | Want to adjust an "if… then…" plan for the next time that trigger shows up? |
| `lapse.resume` | O próximo passo é pequeno: fechar a aba, beber água, voltar pro seu dia. Eu sigo aqui. | The next step is small: close the tab, drink some water, get back to your day. I'm still here. |
| `progress.days` | Nos últimos 30 dias, {days} foram do jeito que você queria. | In the last 30 days, {days} went the way you wanted. |
| `streak.day1` | Um dia do jeito que você escolheu. Bom começo. | One day the way you chose. Good start. |
| `streak.week` | Uma semana. Sem fogos, só um aceno: você está praticando. | One week. No fireworks, just a nod: you're practising. |
| `streak.month` | Trinta dias. O que mudou na sua rotina que vale manter? | Thirty days. What changed in your routine that's worth keeping? |
| `streak.after_reset` | A contagem recomeça, a prática não. Nos últimos 30 dias, {days} foram do jeito que você queria. | The count restarts; the practice doesn't. In the last 30 days, {days} went the way you wanted. |
| `checkin.daily` | Como você está hoje? Um toque basta. | How are you today? One tap is enough. |
| `checkin.halt` | Checagem rápida: fome, raiva, solidão ou cansaço? Qualquer um deles deixa tudo mais difícil. | Quick check: hungry, angry, lonely or tired? Any of them makes everything harder. |
| `checkin.ifthen` | Complete: "Se eu sentir vontade quando ____, então eu vou ____." | Fill in: "If I feel the urge when ____, then I will ____." |
| `checkin.values` | Pra que você quer isso? Escreva uma frase. Eu te lembro dela quando apertar. | What do you want this for? Write one sentence. I'll remind you of it when things get tough. |

**Panic and help**

| Key | PT-BR | English |
|---|---|---|
| `panic.open` | Estou aqui. Você fez bem em apertar. | I'm here. You did well to press this. |
| `panic.options` | O que ajuda agora: surfar a vontade (3 min), sair da tela (10 min) ou falar com alguém? | What helps right now: surf the urge (3 min), step away from the screen (10 min), or talk to someone? |
| `panic.away` | Levanta, bebe água, vai até a janela. Eu seguro tudo por aqui por 10 minutos. | Get up, drink some water, go to the window. I'll hold everything here for 10 minutes. |
| `panic.reason` | Você escreveu: "{reason}". Continua valendo? | You wrote: "{reason}". Still true? |
| `panic.crisis` | Se o que você sente é maior do que vontade (desespero, ou pensamentos de se machucar), ligue {crisis_phone}. É o {crisis_name}: grátis, 24 horas. Também há chat em {crisis_chat}. Em emergência, ligue {emergency_phone}. | If what you feel is bigger than an urge (despair, or thoughts of hurting yourself), please reach a crisis line now. In Brazil, call {crisis_phone} ({crisis_name}, free, 24 hours). Elsewhere, see {intl_directory}. In an emergency, call your local emergency number. |
| `help.pattern` | Tem sido difícil várias vezes nesta semana. Isso não é falta de caráter, e você não precisa resolver sozinho. Um psicólogo pode ajudar de verdade. Isso não é um diagnóstico. | It's been hard several times this week. That's not a character flaw, and you don't have to sort it out alone. A psychologist can really help. This is not a diagnosis. |
| `help.how` | Quer ver como encontrar atendimento? Há psicólogos com registro ativo no CRP, o CAPS pelo SUS e grupos de apoio mútuo. | Want to see how to find support? There are licensed therapists, public mental-health services and mutual-help groups. |
| `help.limits` | Eu sou um mascote, não um terapeuta. Posso bloquear, lembrar e acompanhar. Cuidar da causa é trabalho pra fazer com um profissional. | I'm a mascot, not a therapist. I can block, remind and keep you company. Working on the cause is something to do with a professional. |

**Consent, disclosure and state**

| Key | PT-BR | English |
|---|---|---|
| `wizard.what` | O {name} bloqueia sites de pornografia neste computador e te acompanha quando a vontade aperta. É uma barreira e um apoio, não um tratamento. | {name} blocks pornography sites on this computer and keeps you company when the urge gets strong. It is a barrier and a support, not a treatment. |
| `wizard.limits` | Ele dificulta, não tranca. Quem tem a senha de administrador consegue desfazer tudo, e ele não alcança o celular nem outra rede. | It adds friction; it does not lock. Anyone with the administrator password can undo all of it, and it does not reach a phone or another network. |
| `wizard.adult` | É para uma pessoa adulta usar no próprio computador. Não serve para vigiar outra pessoa, e não manda relatório nem aviso para ninguém. | It is for an adult to use on their own computer. It is not for watching someone else, and it sends no report or alert to anyone. |
| `dns.disclosure` | Com esta opção, todos os nomes de sites que este computador consultar passam pela Cloudflare (1.1.1.3), no lugar do seu provedor de internet. É a Cloudflare que decide o que barrar: sites adultos e sites que ela considera perigosos. O ubiqX não recebe nada. Se preferir, escolha "só a lista neste computador". | With this option, every site name this computer looks up goes through Cloudflare (1.1.1.3) instead of your internet provider. Cloudflare decides what to stop: adult sites and sites it considers dangerous. ubiqX receives nothing. If you prefer, choose "only the list on this computer". |
| `dns.previous` | Seu DNS atual ({servers}) será substituído. Ao desligar, o DNS volta ao automático. | Your current DNS ({servers}) will be replaced. When you turn this off, DNS goes back to automatic. |
| `hosts.readable` | Os sites que você adiciona entram na lista do sistema, que outras contas deste computador conseguem ler. | Sites you add go into the system list, which other accounts on this computer can read. |
| `policies.disclosure` | Os navegadores vão mostrar "Gerenciado pela sua organização". O DNS próprio do navegador é desligado para que os bloqueios valham. | Browsers will show "Managed by your organization". The browser's own DNS is switched off so that the blocks apply. |
| `system.prompt_next` | O sistema vai pedir sua senha de administrador para aplicar isso. | The system will ask for your administrator password to apply this. |
| `system.gated` | Em verificação neste sistema. Por enquanto valem o bloqueio no app e a verificação por IA. | Being verified on this system. For now the in-app block and the AI check apply. |
| `vision.consent` | Quando você abre um site ou vídeo que as listas não conhecem, o {name} tira uma foto reduzida só dessa janela e pergunta a {provider} se há pornografia nela. A resposta é uma palavra. A foto não é gravada neste computador. {provider} recebe a imagem, aplica as regras dele ao que recebe e guarda pelo prazo que os termos dele definem. Sites conhecidos de bancos, e-mail e documentos, endereços da sua rede local e os sites da sua lista "nunca bloquear" não são fotografados. Janelas anônimas são verificadas. Pausar o ubiqX ou usar o modo privado não pausa esta verificação. | When you open a site or video the lists do not know, {name} takes a reduced picture of that window only and asks {provider} whether it shows pornography. The answer is one word. The picture is not saved on this computer. {provider} receives the image, applies its own rules to what it receives and keeps it for the period its terms state. Well-known bank, e-mail and document sites, addresses on your local network and the sites on your "never block" list are not pictured. Private-browsing windows are checked. Pausing ubiqX or using private mode does not pause this check. |
| `vision.managed` | As imagens da verificação passam pelo servidor do ubiqX a caminho da Anthropic. O servidor não guarda imagens. Ele registra que uma verificação foi feita (hora e custo), não o que havia na tela. | Check images pass through the ubiqX server on their way to Anthropic. The server keeps no images. It records that a check was made (time and cost), not what was on screen. |
| `vision.cost` | Cada verificação custa cerca de US$ 0,001. Limite: {per_day} por dia e US$ {cap} por mês. | Each check costs about US$ 0.001. Limit: {per_day} a day and US$ {cap} a month. |
| `vision.today` | Hoje: {n} verificações. Cada uma envia uma imagem reduzida da janela ativa para {provider}. Nada é guardado neste computador. | Today: {n} checks. Each sends a reduced image of the active window to {provider}. Nothing is kept on this computer. |
| `vision.off_note` | Sem a verificação por IA, sites desconhecidos só são barrados pelo DNS filtrado e pelas listas. | Without the AI check, unknown sites are stopped only by filtered DNS and the lists. |
| `exclusion.delay` | O {name} passa a respeitar esta exclusão em {delay}. | {name} will honour this exclusion in {delay}. |
| `drift.card` | A proteção do sistema mudou. Quer restaurar? | The system protection changed. Restore it? |
| `drift.notice` | O {name} precisa da sua atenção. | {name} needs your attention. |
| `resolver.unreachable` | Os nomes de sites não estão resolvendo. Se você está sem internet, ignore. Se a internet funciona em outros aparelhos, restaure a rede. | Site names are not resolving. If you are offline, ignore this. If the internet works on other devices, restore the network. |
| `pending.wait` | Pedido anotado. Vale daqui a {delay}. Você pode cancelar quando quiser. | Request noted. It applies in {delay}. You can cancel whenever you like. |
| `pending.ready` | O tempo de espera terminou. Ainda quer desligar? | The waiting time is over. Do you still want to turn it off? |
| `off.system_left` | A proteção no app foi desligada. A lista do sistema e o DNS continuam aplicados até você remover. | In-app protection is off. The system list and DNS stay applied until you remove them. |
| `quit.confirm_system` (at least one system tier is `Applied`) | Com o app fechado, o que está aplicado no sistema continua valendo; o bloqueio no app e a verificação por IA param. | With the app closed, what is applied to the system still holds; the in-app block and the AI check stop. |
| `quit.confirm_none` (no system tier is `Applied`) | Com o app fechado, nada fica barrando sites neste computador até você abrir o app de novo. | With the app closed, nothing stops sites on this computer until you open the app again. |
| `reminder.on` | O {name} está ligado neste computador. Veja como desligar. | {name} is on on this computer. See how to turn it off. |
| `secondary.note` | A proteção está ativa pelo outro app. | Protection is running in the other app. |
| `privacy.at_rest` | Quem tem acesso à sua conta neste computador pode ver que o {name} está instalado e ler suas frases e planos. | Anyone with access to your account on this computer can see that {name} is installed and read your sentences and plans. |

Buttons, headings and state labels (about 70 short keys: `btn.*`, `tier.*`, `issue.*`,
`tab.*`) are written by the frontend unit under the rules of 5.1 and reviewed with the
forbidden-word test; they carry no claim.

### 5.7 Help resources

`crates/ubiqx-nofap/data/resources.json`, one entry per resource: `id`, `country`,
`name`, `phone`, `url`, `note_pt`, `note_en`, `verified_on`, `verified_by`.

| Entry | Content to verify |
|---|---|
| `cvv` | CVV, phone 188, chat `cvv.org.br/chat`, free, 24 hours |
| `samu` | SAMU, phone 192 |
| `caps` | CAPS (SUS), walk-in; `gov.br/saude/pt-br/caps` |
| `crp` | "Procure um psicólogo com registro ativo no CRP" (text only) |
| `findahelpline` | `findahelpline.com`, other countries |

Every entry ships with `verified_on` and `verified_by` empty. A release-mode test
(`cargo test -p ubiqx-nofap --release resources_verified`) fails while any is empty, so a
person has to open each site and check each number before a release. Not listed: DASA, SAA
Brazil, Sexólicos Anônimos, AISEP (none confirmed by the research).

---

## 6. Commitment: turning protection off

### 6.1 Model

`NofapConfig` is not part of `Settings`, so the UI's whole-object, last-writer-wins settings
save cannot flip it, and neither can "restaurar padrões" or "Apagar todos os dados". It
changes only through `NofapService::request(change)`, and
`ubiqx_nofap::commitment::classify(&NofapConfig, &NofapChange) -> Effect` decides. The
`match` is exhaustive with no wildcard arm, so a new change kind does not compile until it is
classified.

| `NofapChange` | `Effect::Now` when | `Effect::After(delay)` when |
|---|---|---|
| `Disable` | never | always |
| `AddBlock { host }`, `RemoveAllow { id }` | always | never |
| `RemoveBlock { id }`, `AddAllow { host }`, `ForgetLearned` | never | always |
| `SetHosts(bool)`, `SetPolicies(bool)`, `SetVision(bool)`, `SetHostsUserSites(bool)` | `true` | `false` |
| `SetResolver(Option<ResolverId>)` | `Some` | `None` |
| `SetVisionScope { unknown_sites, media_apps, messaging_apps, private_windows }` | no flag goes from on to off | any flag goes from on to off |
| `SetCaps { max_per_hour, max_per_day, monthly_cap_usd }` | no value is lowered | any value is lowered |
| `SetDelay { secs }` | raised or equal | lowered |
| `SetPrefs(UiPrefs)`, `SetValues(String)`, `SetPlans(Vec<IfThenPlan>)` | always | never |

"Modo estrito" on a mixed platform is `AddBlock` / `RemoveBlock` of that host. Cancelling a
pending change is its own call and is always immediate.

Before the module is enabled for the first time everything is `Now`.

### 6.2 The delay

- `delay_secs`: default 600, range 0 to 86,400. Raising is immediate; lowering takes effect
  after the delay in force.
- A loosening becomes `PendingChange { id, change, requested_at, ready_at, observed_secs }`
  (at most 20). The page shows a quiet countdown, the values sentence, "Manter proteção"
  (cancels, one tap, always available) and the surf button. No text argues.
- **Ready, not applied.** A request is ready when `now >= ready_at` **and**
  `observed_secs >= min(delay_secs, 900)`. `observed_secs` grows by at most 150 s per verify
  pass, and only while the app runs. Moving the system clock forward therefore still costs
  up to 15 minutes of real running time; moving it back changes nothing, because `ready_at`
  was fixed at request time.
- A ready request needs a **fresh confirmation** within 24 hours (card or page), otherwise
  it expires. With a delay of 0 the confirmation is the existing `DisableWarnings` dialog
  with "Manter proteção" as the primary button.
- A confirmed `Disable` stops L1 and L5, then asks for the OS prompt to remove L2 to L4. If
  the prompt is declined the page says `off.system_left` and offers "Remover do sistema".
- Each loosening request increments `override_request`; a cancellation, `override_cancel`.

Honest limit, on the help page: the delay is kept in the app's own database. Someone who
deletes the database or edits it ends the delay. The product is friction and a true account
of its own state, not a lock.

### 6.3 Exits that are never delayed

| Exit | Why |
|---|---|
| "Restaurar minha rede": `remove --scope resolver` for this owner; L1, L2, L4, L5 stay; shown with "Religar"; counter `network_restore`, not an override | a broken network is a safety problem |
| Removal with the administrator password; uninstalling | the machine belongs to the user |
| "Apagar meu histórico do {name}" | erasing one's own data is a right and weakens no tier |
| Cancelling a pending change | it only tightens |
| Quitting the app (after `quit.confirm_system` or `quit.confirm_none`, whichever is true) | hiding the way out would make this a control tool. It is also the cheapest way around L1 and L5, and around everything while no system tier is applied, so it is not free of consequence: a quit while the module is on increments `quit_on`, which counts towards `HELP_OVERRIDES_7D`; the dialog's primary button is "Manter aberto"; with "Abrir junto com o computador" on, the guard is back at the next login. Whether quitting should also wait the delay is question 6. |
| `local_only`, removing the API key | privacy rights (4.1) |

### 6.4 Deliberately not built

| Not built | Why |
|---|---|
| Accountability partner, reports or alerts to anyone | no controlled evidence; coercion and privacy risk; the mechanism that got comparable apps removed from Google Play. Nothing the module stores can be read by a second person. |
| Password held by someone else, whole-computer lock, disabled quit, forced autostart, a watchdog that restarts the app, a hidden mode | stalkerware affordances; lock-out risk. The tray icon is always visible while the module is on. |
| Privileged background process that re-applies silently (root daemon, SYSTEM scheduled task) | needs code signing and real-machine proof; explicit opt-in, later (section 12) |
| Punishment for changing the protection | conflicts with the no-shame rule |
| Detecting or stopping VPN, Tor or proxy processes; disabling private browsing | false positives on work VPNs; scanning processes is surveillance |
| Local DNS proxy, network extension, `pf`, WFP | fail closed, or need Apple entitlements and a Developer ID |
| Any log of sites, screens or verdicts; cloud sync; telemetry about use of the module | the Quittr leak; there is no server-side data about the module at all |

---
## 7. Data model and code architecture

### 7.1 Configuration (`ubiqx_core::nofap::NofapConfig`)

One JSON document in the single row of `nofap_state`. `#[serde(default)]` on every struct, so
`{}` deserialises to the default (unit test) and a new field needs no migration.
`NofapConfig::sanitize()` clamps the ranges. `Settings` gets **no** new field.

```rust
pub struct NofapConfig {
    pub v: u8,                               // 1
    pub enabled: bool,                       // false
    pub enabled_at: Option<DateTime<Utc>>,   // None
    pub self_use_at: Option<DateTime<Utc>>,  // None; the wizard's "é para mim" tick
    pub system: SystemPrefs,
    pub vision: VisionPrefs,
    pub delay_secs: u32,                     // 600   (0..=86_400)
    pub pending: Vec<PendingChange>,         // empty (at most 20)
    pub ui: UiPrefs,
    pub values: String,                      // ""    (at most 280 characters)
    pub plans: Vec<IfThenPlan>,              // empty (at most 3; { when, then })
    pub internal: Internal,                  // never exported
}
pub struct SystemPrefs {
    pub hosts: bool,                         // false until the wizard
    pub resolver: Option<ResolverId>,        // None
    pub policies: bool,                      // false
    pub hosts_user_sites: bool,              // true
    pub network_restored: bool,              // false
}
pub struct VisionPrefs {
    pub enabled: bool,                       // false
    pub consent: Option<VisionConsent>,      // { provider: AiProvider, managed: bool, at }
    pub unknown_sites: bool,                 // true
    pub media_apps: bool,                    // true
    pub messaging_apps: bool,                // false
    pub private_windows: bool,               // true
    pub max_per_hour: u32,                   // 20    (1..=60)
    pub max_per_day: u32,                    // 120   (1..=1_000)
    pub monthly_cap_usd: f64,                // 2.0   (0.25..=20.0)
}
pub struct UiPrefs { pub discreet: bool /* true */, pub show_streak: bool /* false */,
                     pub daily_checkin: bool /* false */ }
pub struct Internal {
    pub salt_hex: String,                    // 16 random bytes, created at first enable
    pub exclusions_seen: Vec<ExclusionSeen>, // { key, first_seen_at }
    pub last_help_at: Option<DateTime<Utc>>,
    pub last_reminder_at: Option<DateTime<Utc>>,
    pub last_drift_card_at: Option<DateTime<Utc>>,
    pub previous_dns: Vec<PreviousDns>,      // { interface, servers }, for display only
}
pub struct PendingChange { pub id: Id, pub change: NofapChange, pub requested_at: DateTime<Utc>,
                           pub ready_at: DateTime<Utc>, pub observed_secs: u32 }
pub enum NofapChange {                       // serde tag = "kind", snake_case
    Disable,
    AddBlock { host: String }, RemoveBlock { id: Id }, AddAllow { host: String },
    RemoveAllow { id: Id }, ForgetLearned,
    SetHosts(bool), SetResolver(Option<ResolverId>), SetPolicies(bool), SetHostsUserSites(bool),
    SetVision(bool),
    SetVisionScope { unknown_sites: bool, media_apps: bool, messaging_apps: bool, private_windows: bool },
    SetCaps { max_per_hour: u32, max_per_day: u32, monthly_cap_usd: f64 },
    SetDelay { secs: u32 },
    SetPrefs(UiPrefs), SetValues(String), SetPlans(Vec<IfThenPlan>),
}
pub const DISPLAY_NAME: &str = "Escudo";
pub const SYSTEM_ID: &str = "ubiqx-shield";
```

### 7.2 Product identity (`ubiqx_core::product`)

```rust
pub enum ProductId { Ubiqx, Shield }          // serialised "ubiqx" | "shield"
pub struct ProductIdentity {
    pub id: ProductId,
    pub bundle_id: &'static str,        // "ai.ubiqx.app"         | "ai.ubiqx.shield"
    pub display_name: &'static str,     // "ubiqX"                | nofap::DISPLAY_NAME
    pub app_names: &'static [&'static str],   // window/app names for the exempt lists
    pub exe_stems: &'static [&'static str],   // ["ubiqx", "ubiqx-desktop", "ubiqx_desktop"] | ["ubiqx-shield"]
    pub keyring_service: &'static str,  // "ai.ubiqx"             | "ai.ubiqx.shield"
    pub db_file: &'static str,          // "ubiqx.db"             | "shield.db"
    pub feed_product: &'static str,     // "ubiqX"                | "ubiqx-shield"
    pub feed_url: &'static str,         // DEFAULT_FEED_URL       | .../releases/download/shield-continuous/latest.json
}
pub static UBIQX: ProductIdentity;  pub static SHIELD: ProductIdentity;
```

Both identities are added unconditionally to `EXEMPT_APP_IDS` and `EXEMPT_APP_NAMES`
(`crates/ubiqx-core/src/focus.rs:32-56`), to `SELF_STEMS`
(`crates/ubiqx-platform/src/app_id.rs:9`), to the engine's Windows and Linux exempt lists and
to the literal in `macos/enforce.rs::hide_others_script`, so neither product ever acts on the
other. `PlatformServices::native_for(identity, data_dir)` passes the keyring service;
`native_in` keeps today's behaviour. The update check ignores a feed whose `product` differs
from `identity.feed_product` (`ubiqx_core::update::feed_matches_product`; today nothing
checks it).

### 7.3 Tables (migration V3, `crates/ubiqx-storage/src/migrations.rs`)

```sql
CREATE TABLE nofap_state (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    config     TEXT NOT NULL,                 -- NofapConfig as JSON
    updated_at INTEGER NOT NULL               -- unix milliseconds
);
CREATE TABLE nofap_sites (
    id         TEXT PRIMARY KEY NOT NULL,
    list       TEXT NOT NULL,                 -- block | allow
    origin     TEXT NOT NULL,                 -- user | learned | snooze
    host       TEXT,                          -- NULL unless origin = user
    host_key   BLOB NOT NULL,                 -- 16 bytes, SHA-256(salt || 0x00 || host)[..16]
    created_at INTEGER NOT NULL,
    expires_at INTEGER,                       -- snooze only
    UNIQUE (list, host_key)
);
CREATE TABLE nofap_day (
    day  TEXT NOT NULL,                       -- local YYYY-MM-DD
    hour INTEGER NOT NULL,                    -- local hour 0..23
    kind TEXT NOT NULL,
    n    INTEGER NOT NULL,
    PRIMARY KEY (day, hour, kind)
);
```

`nofap_day.kind`: `block_list`, `block_user`, `block_keyword`, `vision_check`, `vision_hit`,
`not_it`, `surf_started`, `surf_done`, `panic`, `lapse`, `checkin_ok`, `checkin_meh`,
`checkin_hard`, `checkin_very_bad`, `trigger_tired`, `trigger_bored`, `trigger_stressed`,
`trigger_lonely`, `trigger_hungry`, `trigger_angry`, `trigger_other`, `override_request`,
`override_cancel`, `quit_on`, `network_restore`, `drift`, `help_shown`. The column is read with a
lenient parser (unknown kinds are skipped), so an older build reading a newer database does
not fail the query.

There is no row per event, no exact time, no host, no app. `interventions` is never written
by the module and no existing enum column gains a variant, except `ai_usage.kind`
(`nofap_vision`), whose rows of that kind carry the hour, not the time (4.8). Mechanical changes: `const V3`, `M::up(V3)` appended; the three hard-coded
`2`s become `3` (`migrations.rs:273`, `tests/repos.rs:234`, `:242`). Both products use the
same migration list in separate database files.

### 7.4 Retention, export, delete

| Data | Kept | "Apagar todos os dados" (ubiqX) | "Apagar meu histórico do {name}" | Export |
|---|---|---|---|---|
| Images taken for vision | never written | | | |
| Per-key memory, caps, cards | process lifetime | cleared | cleared | no |
| `nofap_day` | 90 days, pruned daily by the verify pass | deleted (`DELETE FROM nofap_day` joins `WIPE`; `[&str; 9]` becomes 10) | deleted | yes |
| `nofap_state` (config, values, plans, pending) | until changed | **kept**: wiping it would be an undelayed off switch | values and plans cleared; the rest kept | yes, without `internal` |
| `nofap_sites`, user entries | until removed | kept | kept | yes |
| `nofap_sites`, learned | until "Esquecer sites aprendidos" (delayed) | kept | kept | count only |
| `nofap_sites`, snooze | until expiry | kept | deleted | no |
| `ai_usage` rows of kind `nofap_vision` | as today | deleted (already in `WIPE`) | kept (the cost ledger is not history of use by site) | no |
| System state file, hosts block, DNS, policies | until `remove` | untouched | untouched | no |

"Exportar dados do {name}" is its own button and writes
`<data_dir>/exports/shield-<date>.json`. ubiqX's existing `export_data` does not include the
module's data, because that export is the one a person sends to someone else. "Apagar tudo do
{name}" (config, sites, history) exists only while the module is off. The database is not
encrypted at rest, as today; the privacy text says so (`privacy.at_rest`).

### 7.5 Ports and events (`crates/ubiqx-core/src/nofap.rs`: types and traits, no logic)

```rust
pub trait SystemShield      // 2.5
pub trait ExplicitDetector  // 4.5
pub trait FrameSource       // 4.4
pub trait NofapPresenter: Send + Sync { fn show(&self, card: &NofapCard) -> CoreResult<bool>;
                                        fn dismiss(&self, id: &str) {} }
pub trait NofapSink: Send + Sync { fn emit(&self, event: NofapEvent); }
pub trait NofapEnv: Send + Sync { fn snapshot(&self) -> NofapEnvSnapshot; }
pub struct NofapEnvSnapshot {
    pub language: UiLanguage, pub provider: AiProvider, pub vision_model: String,
    pub ai: AiReadiness,                  // Ready | NoKey | LocalOnly | NotOnboarded | LicenceRequired
    pub excluded_apps: Vec<String>,       // ubiqX: blocked_apps + vision_denied_apps; standalone: empty
    pub excluded_domains: Vec<String>,    // ubiqX: blocked_domains
}
pub trait NofapRepo: Send + Sync {
    fn load_config(&self) -> CoreResult<NofapConfig>;
    fn save_config(&self, config: &NofapConfig) -> CoreResult<()>;
    fn list_sites(&self) -> CoreResult<Vec<NofapSite>>;
    fn upsert_site(&self, site: &NofapSite) -> CoreResult<()>;
    fn delete_site(&self, id: &str) -> CoreResult<()>;
    fn delete_sites_by_origin(&self, origin: SiteOrigin) -> CoreResult<u64>;
    fn purge_expired_sites(&self, now: DateTime<Utc>) -> CoreResult<u64>;
    fn bump_day(&self, day: &str, hour: u8, kind: DayKind, by: u32) -> CoreResult<()>;
    fn days_between(&self, from: &str, to: &str) -> CoreResult<Vec<DayCount>>;
    fn delete_days_before(&self, day: &str) -> CoreResult<u64>;
    fn erase_history(&self) -> CoreResult<()>;
}
pub enum NofapEvent {                     // serde tag = "type", snake_case; no variant has a host
    Status { status: NofapStatus }, Card { card: NofapCard }, CardClosed { id: Id },
    PendingChanged, SitesChanged,
}
```

Additions to existing traits, all with default bodies so nothing else breaks:
`ActivitySource::peek_foreground`, `BrowserUrlResolver::resolve_fresh`,
`UsageRepo::totals_by_kind(range) -> CoreResult<Vec<(AiUsageKind, AiUsageTotals)>>`,
`ScreenshotRepo::delete_for_block(block_id) -> CoreResult<Vec<Screenshot>>`; and
`AiUsageKind::NofapVision` with `as_str` / `parse`. `EngineEvent` gets no new variant (the
module has its own sink), so the CLI's exhaustive match is untouched.

### 7.6 `crates/ubiqx-nofap` (new; pure; depends on `ubiqx-core`, `sha2`, `serde`, `serde_json`, `chrono`, `url`)

| Module | Public surface | Section |
|---|---|---|
| `list.rs` | `HashTable::parse(&[u8])`, `contains(&str)`, `key_of(tag, &str) -> u64`, `EMBEDDED_CORE`, `EMBEDDED_POPULAR`, `HOSTS_TIER` | 3.2 |
| `matcher.rs` | `Matcher`, `Verdict`, `normalize_host`, `parents`, `NEVER_BLOCK`, `site_key(salt, host)` | 3.4 |
| `heuristics.rs` | `host_rule(&str) -> HostRule`, `title_signal(&str) -> bool`, `path_signal(&str) -> bool`, `fold_lookalikes` | 3.5 |
| `hosts.rs` | `render_block(names, os) -> String`, `splice`, `inspect`, `is_valid_hostname`, `names_for(tier, extras, os) -> Vec<String>`, caps | 2.2 |
| `resolvers.rs` | `RESOLVERS`, `lookup(ResolverId) -> &ResolverSet` | 2.3 |
| `policies.rs` | `desired(browser) -> Vec<PolicyValue>`, `chromium_json()`, `firefox_json()`, `conflicts(observed, created)` | 2.4 |
| `plan.rs` | `build_plan`, `validate`, `union(owners)`, `assess(&NofapConfig, &SystemObservation, os) -> SystemAssessment`, `preview(plan, observation) -> Vec<PlannedChange>`, `reset_targets(observation, resolver)`, `bypass_suspected` | 2 |
| `gate.rs` | `TIER_GATE`, `tier_enabled(os, tier)`, `parse_override(env)` | 1.8 |
| `state_dir.rs` | `assess(DirMeta) -> Trust` | 2.5 |
| `commitment.rs` | `classify`, `request`, `tick(&mut NofapConfig, elapsed, now) -> Vec<Ready>`, `confirm`, `cancel`, `expire` | 6 |
| `sampling.rs` | `decide`, `record`, `outcome`, `SamplerMemory`, constants | 4 |
| `episode.rs` | `EpisodeTracker::observe(hit, now) -> EpisodeStep` | 1.2 |
| `exclusions.rs` | `effective`, `reconcile` | 4.2 |
| `primary.rs` | `decide(lock, me, now) -> Role` | 1.6 |
| `messages.rs` | `notification_text(kind, lang)`, `tray_labels(lang)`, `rotation_key(discreet, user_list, n)`; the few lines the engine and shells need in Rust, PT-BR and EN | 5.3 |
| `help.rs` | `help_due(&[DayCount], last_shown, today) -> bool`, `RESOURCES` | 5.5, 5.7 |
| `progress.rs` | `days_your_way(&[DayCount], today, 30)`, `streak` | 5.5 |

### 7.7 Other crates

| Crate | Change |
|---|---|
| `ubiqx-storage` | `repos/nofap.rs` on the `repos/focus.rs` template (`impl NofapRepo for SqliteStore`); V3; `WIPE`; `UsageRepo::totals_by_kind`; `ScreenshotRepo::delete_for_block` |
| `ubiqx-ai` | `explicit.rs` (`LlmExplicitDetector`, `build_request`, `explicit_schema`, `parse_explicit`); `prompts.rs` (`EXPLICIT_SYSTEM_PROMPT`); `fake.rs` (`FakeExplicitDetector`: a queue of `CoreResult<ExplicitCheck>` and the images it received; `ScriptedLlmClient::push_error`; `MemoryUsageRepo::totals_by_kind`); `openai.rs` (the `NofapVision` arm); `redact_images` at the two trace lines; in `client.rs` and `openai.rs`, for `AiUsageKind::NofapVision`: no log line on the success path and `AiUsage::at` truncated to the hour (4.4, 4.8) |
| `services/ubi-api` | explicit body limit; image passthrough test; no-body-in-log test |
| `ubiqx-platform` | `sysblock/` (below); `frame.rs` (`WindowFrameSource`, difference hash, crop, in `image_util`); `browser.rs` and `windows/browser.rs` (`resolve_fresh`, address-bar focus); `mock.rs` (`peek_foreground`, `MockSystemShield` with a call log and scripted observation, `ScriptedFrameSource`); `app_id.rs`; `macos/enforce.rs` (the literal); the secrets modules (keyring service as a parameter); `src/bin/ubiqx-shield-helper.rs` |
| `ubiqx-engine` | `nofap/` (below); `deps.rs` (`EngineDeps.nofap: NofapPorts`, `EngineDeps.identity`, `NofapPorts::inert()` with `NoSystemShield`, `NoFrameSource`, `LogNofapPresenter`, `NullNofapSink`, `MemoryNofapRepo`); `engine.rs` (`LoopConfig.nofap_every` 2 s, `nofap_verify_every` 120 s, two loops); `state.rs` (`EngineState.nofap: Arc<NofapService>`, built with `Arc::new_cyclic` so `EngineEnv` holds a `Weak<EngineState>`); `handle.rs` (`EngineHandle::nofap()`; `delete_all_data` clears the service's memory); `tracker.rs`, `screenshots.rs` (1.4); `classify.rs` (budget sum without `nofap_vision`); `update.rs` (feed product); `focus.rs` (exempt lists) |
| `ubiqx-app` | `lib.rs` (`AppConfig.nofap_presenter`, `nofap_sink`; `NofapPorts` from the platform, store and detector; identity); `nofap_app.rs` (section 8) |

`crates/ubiqx-platform/src/sysblock/`:

| File | Content | Tested on Linux |
|---|---|---|
| `mod.rs` | `NativeSystemShield { os: Box<dyn OsBackend>, runner, fs, elevator, exe, owner }` implementing `SystemShield`; traits `Runner` (`run(program, args, env) -> Output`), `Fs` (read, atomic write, remove, `dir_meta`, list), `Elevator` (`run_elevated(exe, args) -> ElevatedExit`), `NameLookup` (canary and probe) | yes, with fakes |
| `macos.rs`, `windows.rs`, `linux.rs` | one `OsBackend` each: argv builders, the fixed PowerShell scripts as constants, parsers, paths. **No `cfg`**: all three compile on every OS. | yes: fixtures of recorded outputs |
| `helper.rs` | `main_if_requested(args)`, `run(args) -> i32`, verb parsing, `apply`, `remove`, `status` over `OsBackend` + `Runner` + `Fs`; state file | yes: whole apply → observe → remove cycles in a temp tree with a fake runner, for each back-end |
| `elevate.rs` | `sh_quote`, `applescript_quote`, `ps_single_quote`, `osascript_argv`, `powershell_argv`, `pkexec_argv`, exit-code mapping | yes |
| `sys.rs` | the real `Runner`, `Fs`, `Elevator` and `NameLookup`: process spawn, file I/O, directory metadata, `getaddrinfo`. Under 200 lines, no decision. The only file of the module that may contain `cfg` (Unix metadata, the Windows no-console flag, and the two Windows directory lookups `GetSystemDirectoryW` and `SHGetKnownFolderPath`, which are the module's only FFI). | compiles; exercised by the smoke job |

`crates/ubiqx-engine/src/nofap/`:

| File | Content |
|---|---|
| `mod.rs` | `NofapDeps` (the ports of 7.5 plus `activity`, `urls`, `enforcer`, `permissions`, `notifier`, `usage`, `clock`, `identity`, `shared_dir`, `export_dir`), `NofapService::new(deps) -> CoreResult<Arc<Self>>`, `NofapRuntime` (config, matcher, episodes, sampler memory, cards, card actions, vision health, status, primary role) behind one mutex |
| `guard.rs` | `guard_pass(&self) -> CoreResult<GuardOutcome { hit: bool, job: Option<VisionJob> }>` |
| `vision.rs` | `async vision_check(&self, job) -> CoreResult<VisionOutcome { hit: bool }>`; readiness; health |
| `system.rs` | `verify_pass`, `system_preview`, `system_apply`, `system_remove`, `restore_network` |
| `service.rs` | `view`, `enable`, `request`, `confirm_pending`, `cancel_pending`, `consent_vision`, `sites`, `support`, `record`, `card`, `card_action`, `sample_frame`, `erase_history`, `export`, `test_card`; `redacts`, `capture_veto` |
| `ipc.rs` | `handle(&NofapService, NofapCommand) -> CoreResult<serde_json::Value>` (7.8). The command, argument and view types are not here: they are `ubiqx_core::nofap_ipc`, written by U0. |
| `env.rs` | `EngineEnv` implementing `NofapEnv` from `EngineState` (settings, licence, key presence), read-only |

The engine's `nofap_loop` calls `guard_pass` on the blocking pool; on `hit` it calls
`tracker::scrub_open_block`. A job is **spawned**, not awaited: `vision_check` runs on its
own task while the loop keeps ticking, an `AtomicBool` keeps one check in flight, and its
completion scrubs again on a hit. Awaiting it in the loop would stop the guard for up to
32 s (14 s, the 4 s wait of Confirm, 14 s) every time a check ran.
`NofapService` never references `EngineState`, which is what lets the standalone app host it.

Locking rule for the service: the `NofapRuntime` mutex is never held across a platform call
or an await (`resolve`, `resolve_fresh`, an `Enforcer` method, `frame`, `check`, `observe`,
`apply`, `remove`, `show`). `system_apply` blocks on the OS password prompt for as long as
the user takes; holding the mutex there would stop every guard pass while the prompt is
open. Each method copies what it needs, releases the lock, calls out, and locks again to
record the result. A test holds `MockSystemShield::apply` on a channel and asserts that
`guard_pass` still closes a listed tab meanwhile.

### 7.8 IPC

One Tauri command per shell, `nofap(command: NofapCommand) -> Result<serde_json::Value,
IpcError>`, whose body is `ubiqx_engine::nofap::ipc::handle` on the blocking pool. The
contract is therefore unit-tested in `ubiqx-engine` on Linux, which matters because the shell
crates cannot be compiled in this sandbox (no webkit). `NofapCommand` is
`#[serde(tag = "cmd", content = "args", rename_all = "snake_case")]`.

**The wire contract is fixed by U0, in Rust, before anyone else starts.**
`crates/ubiqx-core/src/nofap_ipc.rs` defines `NofapCommand` and every type that crosses the
boundary (`EnableOptions`, `ChangeOutcome`, `NofapView`, `PendingView`, `PlannedChange`,
`SitesView`, `SupportView`, `UserEvent`, `CardAction`, `SampleFrame`), with one constructor
of a representative value for each. The test `cargo test -p ubiqx-core nofap_ipc_golden`
serialises those values and compares them with two committed files,
`apps/desktop/src/nofap/fixtures/commands.json` (one request per `cmd`) and
`fixtures/views.json` (one value per return type, per `NofapEvent` variant, and one
`NofapStatus` per `TierState` and `Issue`); with `UBIQX_BLESS=1` it writes them. The files
belong to U0. The frontend imports them with `satisfies` against `types.ts`, so
`pnpm typecheck` fails when the TypeScript mirror and the Rust serialisation differ, and
the engine's `nofap_ipc` test feeds every request of `commands.json` to `handle`. Nobody
writes a fixture by hand from the table below.

| `cmd` | `args` | Returns | Notes |
|---|---|---|---|
| `view` | | `NofapView { status, config (without `internal`), capabilities, today: { blocks, checks }, display_name, provider, cost_line }` | |
| `enable` | `EnableOptions { hosts, resolver, policies, vision, vision_scope, delay_secs, values, discreet }` | `NofapView` | the wizard's last button; may raise the OS prompt |
| `consent_vision` | `{ provider, managed }` | `NofapView` | |
| `request` | `NofapChange` | `ChangeOutcome` = `{ applied: NofapView }` or `{ pending: PendingView }` | 6.1 |
| `confirm_pending`, `cancel_pending` | `{ id }` | `NofapView` | |
| `system_preview` | | `Vec<PlannedChange { tier, text_key, detail }>` | |
| `system_apply` | | `ApplyOutcome` | "Aplicar", "Restaurar", "Religar"; OS prompt |
| `system_remove` | | `ApplyOutcome` | only while disabled; OS prompt |
| `restore_network` | | `ApplyOutcome` | undelayed; OS prompt |
| `sites` | | `SitesView { blocked: [{ id, host, created_at }], allowed: [...], learned_count }` | no learned name |
| `support` | | `SupportView { values, plans, progress { days, streak }, checkin_today, help_due, resources }` | |
| `record` | `UserEvent` = `lapse { triggers }`, `checkin { mood, triggers }`, `surf_started`, `surf_done`, `panic`, `step_away` | `()` | counters only |
| `card` | `{ id }` | `NofapCard` | the floating window |
| `card_action` | `{ id, action: CardAction }` | `()` | `close_window`, `not_it`, `add_to_list`, `restore`, … |
| `sample_frame` | | `SampleFrame { mime, base64, width, height }`, or `null` when no candidate window is remembered | "Ver um exemplo" (4.4): the last candidate window, never the app's own; never sent, never stored |
| `erase_history`, `reset` | | `()` | `reset` only while disabled |
| `export` | | `{ path }` | |
| `test_card` | | `()` | preview card; counts nothing |

Events: both shells emit the Tauri event `nofap` with a `NofapEvent` payload.

Frontend mirror: `apps/desktop/src/nofap/ipc.ts` (`nofapIpc.view()` …), `types.ts`,
`mock.ts` (an in-memory implementation of every `cmd`, including the commitment timer and
scripted statuses; `lib/mock.ts` delegates the command `nofap` to it).

### 7.9 Shells and frontend in ubiqX

`apps/desktop/src-tauri/src/`: `main.rs` (helper entry first); `nofap.rs` (the command,
`TauriNofapPresenter`, `TauriNofapSink`, the window builder); `lib.rs` (wiring into
`AppConfig`, tray item "Preciso de um respiro" and a neutral status line while the module is
on, the quit dialog with `quit.confirm_system` or `quit.confirm_none` and the `quit_on`
counter, `tauri-plugin-single-instance`); `commands.rs` (`nofap` added to
`generate_handler!`); `capabilities/shield-card.json`. The tray's pause and private-mode items get
the tooltip "Não pausa o {name}".

`apps/desktop/src/nofap/` (shared by both products): `types.ts`, `ipc.ts`, `events.ts`,
`store.ts` (its own zustand store), `mock.ts`; pages `NofapPage.tsx` (status header with one
row per layer, pending banner, tabs "Proteção", "Apoio", "Listas", "Ajuda"),
`NofapWizard.tsx`, `NofapCard.tsx` (the floating window, urge surf, panic sheet);
components `TierRow`, `IssueList`, `SystemPreview`, `PendingBanner`, `SiteLists`,
`VisionConsent`, `VisionScope`, `DelayPicker`, `SurfTimer`, `PanicSheet`, `LapseFlow`,
`CheckinCard`, `ValuesAndPlans`, `ProgressCard`, `HelpCard`, `ResourceList`.

ubiqX edits: `App.tsx` (routes `/nofap`, and `/nofap-card` outside `Gate`),
`components/layout/Sidebar.tsx` (item labelled with `display_name`), `lib/mock.ts` (one
delegation line), `i18n/index.ts` (namespaces `nofap`, `nofap_app`), `pages/Settings.tsx`
(the `exclusion.delay` hint under the privacy lists while the module is on). Wording rule:
Settings › Privacidade already uses "bloqueados" for "never recorded", so the module says
"sites barrados" and never reuses that label.

---

## 8. The standalone product

### 8.1 Shape

| Piece | Decision |
|---|---|
| Directory | `apps/nofap/` |
| Shell | `apps/nofap/src-tauri/`: package `ubiqx-nofap-desktop` (workspace member, not a default member), binary `ubiqx-shield`. Files: `Cargo.toml`, `build.rs`, `tauri.conf.json`, `Info.plist`, `capabilities/{default,shield-card}.json`, `icons/`, `src/main.rs` (helper entry, then `app::run()`), `src/app.rs` (plugins, tray, main window, the two commands, presenter, sink). About 300 lines, no logic. |
| Composition root | `ubiqx_app::nofap_app::NofapApp::start(config, sink) -> CoreResult<NofapApp>` in the existing `ubiqx-app` crate |
| Frontend | the same Vite project. `VITE_PRODUCT=nofap` builds `index.nofap.html` → `src/nofap-app/main.tsx` into `dist-nofap` (`pnpm build:nofap`); `tauri.conf.json` points `frontendDist` at `../../desktop/dist-nofap` and `beforeBuildCommand` at `pnpm --dir ../../desktop build:nofap` |
| Identity | `ProductIdentity::SHIELD` (7.2): own identifier, data directory, `shield.db`, keyring service, update feed and updater endpoint. Installing both products shares nothing except the system marker, the system state file and the primary lock. |

### 8.2 What it runs, and what cannot run in it

`NofapApp::start` opens the store (same migration list), builds
`PlatformServices::native_for(&SHIELD, ..)`, `RemoteClients` and the `RoutingLlmClient`
(reusing `ubiqx-app`'s existing wiring), `LlmExplicitDetector`, `NativeSystemShield`,
`WindowFrameSource` and one `NofapService`; then spawns three loops: guard (2 s), verify
(120 s) and the update check (`UpdateChecker` constructed directly, as its constructor takes
only ports and a clock).

It **never calls `Engine::start`**. No `Engine`, `EngineState`, sampler, tracker, segmenter,
classifier, report writer, nudge loop, focus guard or retention loop exists in the process,
and `NofapApp` holds no `EngineHandle`. The productivity tables exist in `shield.db` (one
migration list) and stay empty. Proof, in `crates/ubiqx-app/tests/nofap_app.rs`: a scripted
session with blocks, a vision hit and every flow leaves zero rows in `blocks`,
`screenshots`, `interventions`, `nudges`, `reports`, `corrections`, `rules` (non-system),
`focus_sessions`. The binary still links `ubiqx-engine` (the service lives there); a
composition crate that makes `cargo tree` prove the absence is deferred (section 12).

`StandaloneEnv` implements `NofapEnv` from the settings row and the secret store: language,
provider, vision model, key presence. `local_only` and the privacy lists do not exist there.

### 8.3 Host commands (standalone only)

A second command, `nofap_host(command: NofapHostCommand)`, handled by
`NofapApp::handle_host` (unit-tested in `ubiqx-app`):

| `cmd` | `args` | Purpose |
|---|---|---|
| `settings_get` | | `{ ui_language, ai_provider, vision_model, check_updates, onboarding_done }` from the `Settings` row |
| `settings_set` | the same fields, all optional | read-modify-write of those fields only |
| `api_key_set` | `{ provider, key }` | validates with the vendor (`validate_api_key` logic), then stores in the keyring |
| `api_key_clear` | `{ provider }` | |
| `models` | `{ provider }` | model ids for the picker |
| `permissions`, `permission_request` | `{ kind }` | Screen Recording, Automation |
| `update_check` | | manual check |
| `about` | | version, build, notices |

### 8.4 Frontend flavour

`apps/desktop/src/nofap-app/`: `main.tsx`, `AppNofap.tsx` (routes `/welcome`, `/`,
`/support`, `/lists`, `/help`, `/settings`, `/nofap-card`), `NofapShell.tsx` (navigation),
`Welcome.tsx` (language, the wizard of 5.2, the AI key step), `NofapSettings.tsx` (provider
and key, vision model, language, launch at login, updates, data, about and notices),
`hostIpc.ts`, `hostMock.ts`. It imports the shared `src/nofap/` and the UI kit; it imports no
page of ubiqX. Namespace `nofap_app` holds its chrome. A Vitest case asserts that the module
graph of `nofap-app/main.tsx` contains no file under `pages/` of ubiqX.

Tray: "Abrir", a neutral status line, "Preciso de um respiro", "Verificar atualizações",
"Sair" (with the quit dialog of 6.3). No pause, private mode or report items.

### 8.5 Licence and plans

- Blocking (L1 to L4), counselling and commitment are never gated, in either product. A
  lapsed subscription must never remove a block.
- Vision with the user's own Anthropic, OpenAI or xAI key needs no plan.
- **ubiqX:** vision through "IA do Ubi" needs a valid `monthly_managed` licence, as the
  managed provider does today, plus its own consent tick. No change to `license.rs`, the
  claims or the proxy's plan logic. `LICENSE_ENFORCEMENT` stays `Soft`.
- **Standalone:** the provider picker offers Anthropic, OpenAI and xAI only. "IA do Ubi" and
  licence keys are not in the standalone app in this build, because selling it separately
  needs a `Plan` variant or a product claim, which older builds read as an invalid key
  (question 5).

### 8.6 Release

- Artefacts are named after the machine id: `ubiqx-shield-<os>-<arch>.<ext>`, tag
  `shield-continuous`, own `latest.json` and `updater.json`. The ubiqX globs (`ubiqX-*`)
  never match them.
- CI builds the standalone app on the three runners in every run, which is the only place
  its shell is compiled. **Publishing is off** until the owner has cleared the display name:
  the job `publish-shield` runs only when the repository variable `NOFAP_PUBLISH` is `true`.
- `scripts/publish-release.mjs` and `scripts/app-version.mjs` take `--product ubiqx|shield`
  (default `ubiqx`, so today's calls are unchanged).

---

## 9. CI and the release channel

### 9.1 The release channel (before the first push of this work)

Today `.github/workflows/ci.yml` runs on every push to every branch (`branches: ["**"]`) and
`publish-continuous` runs on any push (`if: github.event_name == 'push'`), and installed apps
take that build. Code that can raise an administrator prompt must not reach installed apps
from a feature branch. Changes, all in the same commit series as the module:

1. `publish-continuous`: `if: github.event_name == 'push' && github.ref == 'refs/heads/main'`.
2. The update check ignores a feed whose `product` is not the running product (7.2).
3. The module ships off, with no automatic prompt, and with every system tier gated (1.8).

### 9.2 Changes to existing jobs

- `rust-linux`: clippy and tests exclude both shells (`--exclude ubiqx-desktop --exclude
  ubiqx-nofap-desktop`); two new steps, `python3 scripts/nofap/build_blocklist.py --check`
  (offline consistency of the committed data) and `cargo test -p ubiqx-nofap --test naming`.
- `frontend`: adds `pnpm build:nofap`.
- `macos-app`, `windows-app`, `linux-app`: each also runs `pnpm tauri build` in
  `apps/nofap/src-tauri` and uploads `ubiqx-shield-*` artefacts; `windows-app` also runs
  `cargo clippy -p ubiqx-nofap-desktop -- -D warnings`.

### 9.3 New job: `nofap-system-smoke` (matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`)

Runs the real helper binary with administrator rights on a real machine, passing
`--unverified hosts,resolver`:

1. `cargo build -p ubiqx-platform --bin ubiqx-shield-helper`; save a copy of the hosts file
   and the output of `status`.
2. `apply` (hosts + `cloudflare_family`) through `sudo` or the elevated runner shell.
3. Assert: the block is present with the expected hash; `status` reports `Applied` for both;
   the canary resolves to `0.0.0.0`; one name from the hosts tier resolves to `0.0.0.0`;
   `example.com` still resolves; on Windows, the time from writing the file to the first
   successful lookup is under 10 s (the measurement that justifies `CAP_WINDOWS`).
4. `remove --owner all --scope all`. Assert: the hosts file is byte-identical to the saved
   copy; DNS is automatic again; the state directory is gone.
5. Windows only: run the app's real elevation command (`elevate::powershell_argv`, printed
   by `ubiqx-shield-helper elevate-argv`, a verb that only prints) with plans of 1,800,
   4,000, 12,000 and 24,000 characters and record the longest that reaches the helper
   intact. The runner is an administrator with UAC off, so `Start-Process -Verb RunAs`
   starts without a prompt but still goes through `ShellExecuteEx`. This is the measurement
   behind `ARG_BUDGET` (2.1).
6. Windows only: repeat steps 2 to 4 with `windir`, `SystemRoot`, `ProgramData` and
   `PSModulePath` pointing at a scratch directory in the caller's environment, and assert
   the same result and that nothing was created in that directory (2.5).

Scripts: `scripts/nofap/system-smoke.sh` and `system-smoke.ps1`, with `--tiers
hosts[,resolver]` (default both). A green run on an OS is the evidence that opens `hosts`
and `resolver` for that OS (1.8). The shell script also runs in this sandbox for
`--tiers hosts` (1.8). `continue-on-error: true` until the first green run, so it cannot
block unrelated work; the gate-opening step of section 13 reads the result.

---
## 10. File tree (new and changed)

```
Cargo.toml, Cargo.lock                                  members, default-members, workspace dep
THIRD-PARTY-NOTICES                                     new
scripts/nofap/build_blocklist.py                        new
scripts/nofap/system-smoke.sh, system-smoke.ps1         new
scripts/publish-release.mjs (+ .test.mjs), app-version.mjs, ci-app-version.sh   --product
.github/workflows/ci.yml                                9
docs/nofap/DESIGN.md                                    this file
docs/nofap/PRIVACY.md, REMOVAL.md, REAL-MACHINE-CHECKLIST.md   new
docs/ARCHITECTURE.md, README.md, docs/WINDOWS-LINUX.md, docs/LICENSING.md   updated

crates/ubiqx-core/src/
  nofap.rs            new: types, ports, DISPLAY_NAME, SYSTEM_ID
  nofap_ipc.rs        new: NofapCommand and every wire type; the golden test (7.8)
  product.rs          new: ProductId, ProductIdentity, UBIQX, SHIELD
  lib.rs, ports.rs, focus.rs, update.rs                 7.2, 7.5

crates/ubiqx-nofap/                                     new crate
  Cargo.toml
  src/lib.rs, list.rs, matcher.rs, heuristics.rs, hosts.rs, resolvers.rs, policies.rs,
      plan.rs, gate.rs, state_dir.rs, commitment.rs, sampling.rs, episode.rs, exclusions.rs,
      primary.rs, messages.rs, help.rs, progress.rs
  data/core.bin, popular.bin, hosts-tier.txt, never-block.txt, mixed.txt, never-capture.txt,
       extra-ptbr.txt, canary-block.txt, canary-allow.txt, sources.lock.json, core.meta.json,
       NOTICE.txt, resources.json, forbidden-words.json
  tests/naming.rs, tests/embedded.rs

crates/ubiqx-storage/src/
  migrations.rs, repos/nofap.rs (new), repos/mod.rs, repos/maintenance.rs, repos/usage.rs,
  repos/screenshots.rs, lib.rs, store.rs;  tests/repos.rs

crates/ubiqx-ai/src/
  explicit.rs (new), prompts.rs, fake.rs, lib.rs, client.rs, openai.rs;  tests/explicit.rs (new)
services/ubi-api/src/…, tests/proxy.rs                  body limit, passthrough, no body in logs

crates/ubiqx-platform/
  Cargo.toml (owned by the foundation unit)
  src/sysblock/mod.rs, macos.rs, windows.rs, linux.rs, helper.rs, elevate.rs, sys.rs   new
  src/sysblock/fixtures/*                                                      new
  src/bin/ubiqx-shield-helper.rs                                               new
  src/frame.rs (new), image_util.rs, lib.rs, mock.rs, browser.rs, app_id.rs,
  src/secrets.rs, secrets_file.rs, macos/{mod,enforce,keychain}.rs,
  src/windows/{mod,browser,secrets}.rs, linux/{mod,secrets}.rs

crates/ubiqx-engine/src/
  nofap/mod.rs, guard.rs, vision.rs, system.rs, service.rs, ipc.rs, env.rs     new
  deps.rs, engine.rs, state.rs, handle.rs, lib.rs, tracker.rs, screenshots.rs,
  classify.rs, update.rs, focus.rs
  tests/nofap.rs (new), tests/nofap_ipc.rs (new), tests/focus.rs, pipeline.rs, screenshot_read.rs

crates/ubiqx-app/src/lib.rs, nofap_app.rs (new);  tests/nofap_app.rs (new)

apps/desktop/src-tauri/
  src/main.rs, lib.rs, commands.rs, nofap.rs (new)
  capabilities/shield-card.json (new), tauri.conf.json, Info.plist
  packaging/ai.ubiqx.app.shield.policy, prerm.sh, shield-hooks.nsh               new

apps/desktop/
  index.nofap.html (new), vite.config.ts, package.json
  src/nofap/**                                           new (7.9)
  src/nofap/fixtures/commands.json, views.json           new, generated by ubiqx-core's golden test
  src/nofap-app/**                                       new (8.4)
  src/i18n/index.ts, messages/nofap.ts (new), messages/nofap_app.ts (new)
  src/App.tsx, components/layout/Sidebar.tsx, lib/mock.ts, pages/Settings.tsx

apps/nofap/
  README.md
  src-tauri/Cargo.toml, build.rs, tauri.conf.json, Info.plist
  src-tauri/src/main.rs, app.rs
  src-tauri/capabilities/default.json, shield-card.json
  src-tauri/icons/*
  src-tauri/packaging/ai.ubiqx.shield.policy, prerm.sh, shield-hooks.nsh
```

---

## 11. Test plan

Conventions of the repository apply: pure logic with in-file unit tests; engine-style
integration tests with in-memory SQLite, `FixedClock` (2026-09-17 15:00 UTC), every loop
parked at 3600 s and passes driven by hand; Vitest with jsdom against the mock, asserting
PT-BR copy. macOS and Windows targets are type-checked from Linux after
`rustup target add aarch64-apple-darwin x86_64-pc-windows-msvc` (the distribution host
answered from this sandbox [V]).

### 11.1 Unit tests (pure; `cargo test -p <crate>` on Linux)

| Crate | Cases |
|---|---|
| `ubiqx-core` | `NofapConfig` from `{}` equals the default; `sanitize` clamps; enum round trips; `NofapCard`, `NofapEvent` and `NofapStatus` serialise without any key named `host`, `domain`, `url`, `title`, `app`, `app_name` or `app_id` (asserted on the JSON, recursively); `feed_matches_product`; both identities are in the exempt lists |
| `ubiqx-nofap` | every row of 3.6; embedded data (`core.bin` and `popular.bin` parse, sorted, unique, counts equal `core.meta.json`; no line of `hosts-tier.txt` fails `is_valid_hostname` or falls under `NEVER_BLOCK`; at most 20,000 lines); **hosts splice property test** (hand-rolled generator: for random surrounding content, LF and CRLF, with and without a final newline, `splice(splice(x, Some(b)), None) == x` and splice is idempotent; a foreign block with a similar marker is untouched); caps per OS; `plan::validate` (unknown version, owner, resolver, too many names, invalid names, never-block names, gated tier); `plan::assess` for every row of the tables in 2.2, 2.3 and 2.4; `union` of two owners; `reset_targets` leaves an interface the user changed; policy JSON per browser and conflict detection; `gate` table and override parsing; `state_dir::assess`; `commitment` (direction of every `NofapChange` in both directions; lowering the delay waits the old delay; `observed_secs` with a forward clock jump: not ready; backward jump: unchanged; expiry after 24 h; delay 0; cancel); `sampling` (every row of 4.2, 4.3, 4.6: candidates, cadence with the fixed clock, relax and its 60-minute expiry, signal cancels relax, per-key and global caps across hour and day boundaries, unchanged-frame skip, **no idle input anywhere in `SampleCtx`**, refusal is never Positive, snooze limit); `episode`; `exclusions` (present at enable: immediate; added later: after the delay; removed: dropped); `primary`; `help` thresholds and once per 7 days; `progress` with an injected UTC offset; `messages` parity of the two languages and no host placeholder; `tests/naming.rs`; forbidden words over `messages.rs` and `resources.json`; `resources_verified` (release mode only) |
| `ubiqx-storage` | V3 applies on a V2 database; the three version assertions; `NofapRepo` round trips; `bump_day` is an upsert; an unknown `nofap_day.kind` does not fail a read; wipe deletes `nofap_day` and keeps `nofap_state` and `nofap_sites`; `totals_by_kind`; `delete_for_block` |
| `ubiqx-ai` | request shape (image then text; the only text parts are the two constants; closed schema; `usage_kind`; `max_tokens`; timeout); parse of the four verdicts and three confidences; unknown string and invalid JSON are `Unsure`; `AiRefused` becomes `refused: true`; truncation retried once; `reasoning_effort_for` with `NofapVision`; `redact_images` (the trace output contains `<image:` and no base64); a `NofapVision` call through each client leaves no line in a capturing subscriber at `trace` and records a usage row whose `at` is on the hour, while a `Classify` call logs and records as before; wiremock round trips for one Anthropic and one OpenAI-compatible body with an image |
| `services/ubi-api` | image block forwarded unchanged; a body over the limit is 413; no request body in captured logs |
| `ubiqx-platform` | `frame`: top crop, 640 px edge, difference hash stable under re-encoding and different for different images, near-uniform capture is "no frame"; `elevate`: the three quoting functions with spaces, quotes, `$`, backslashes and non-ASCII, exact argv of each elevation, exit-code mapping (-128, 1223, 126, 127, `APPIMAGE`); parsers against fixtures (`networksetup` with and without servers, disabled services, a PT-BR header line; PowerShell JSON with one and several adapters and no IPv6; `reg.exe` output; `systemctl`); `committed_url` (bar not focused: the bar text; focused with the same title: the last committed URL; focused after a title change: `None`); `peek_foreground` does not advance the scenario; `resolve_fresh` bypasses the cache |

### 11.2 Integration tests (mock platform, fake AI)

`crates/ubiqx-platform` (`cargo test -p ubiqx-platform sysblock`), fakes for `Runner`, `Fs`,
`Elevator`, `NameLookup`, for each of the three back-ends:

1. `apply` → `observe` → `assess` is `Applied` for the requested tiers; `remove` returns the
   temp "system" to its exact initial bytes.
2. Two owners: `remove --owner ubiqx` keeps the tiers `shield` still wants; removing the
   second owner removes everything and the state file.
3. `remove` after a partial apply; `remove --scope resolver` leaves the hosts block.
4. The exact argv of every privileged command and the PowerShell script constants are
   asserted, so a change to a privileged command line is a visible diff. Every program is an
   absolute path and the environment is the cleaned one.
5. Helper argument parsing: unknown verb, missing plan, oversized plan, an IP as resolver, a
   never-block name in `extra`, a gated tier → the documented exit code and nothing written.
6. A pre-existing state directory that is a symlink, or group-writable, is reset without
   following the link; a state file with an address-shaped value in any field changes no
   DNS command. Windows back-end: a reparse point at the state path → exit 4, nothing
   written; no command line contains `icacls`; with `SystemRoot`, `windir`, `ProgramData`
   and `PSModulePath` set to a hostile directory in the helper's own environment, every
   program path and the whole child environment are unchanged.
7. Policies: a foreign value is a conflict and is left alone; undo removes only our values.
8. Resolver roll-back: the probe resolves before the change and not after → the resolver
   commands are undone in the same run, the hosts block stays, the outcome is
   `Partial([resolver: unreachable])`. The probe failing before the change (offline) →
   the resolver is applied and no roll-back happens.
9. Hosts write: rename fails with `EBUSY` → written in place, same bytes; the read-back
   differs → the original bytes are restored and the exit code is 3; the hosts path is a
   link to a file not owned by root → `Unsupported`, nothing written.
10. A plan longer than `ARG_BUDGET[os]` is never sent: `build_plan` drops the oldest extras,
    reports the count, and the helper derives the `www.` twin of each name.

`crates/ubiqx-engine/tests/nofap.rs` (harness copied from `tests/focus.rs`: settable
foreground and URL, `MockEnforcer`, `ScriptedFrameSource`, `FakeExplicitDetector`,
`MockSystemShield`, recording presenter and sink):

1. Off by default: `guard_pass` makes no platform call; no card; no row.
2. Listed host on two passes → `CloseTab`, one counter, one discreet card, no row in
   `interventions`; one pass only → nothing.
3. Close refused → blank; both refused → card only.
4. Five hits in one episode → five enforcements, one counter increment, cards at the first
   and third.
5. **Stale cache:** `resolve` returns the listed host on two passes but `resolve_fresh`
   returns another host → nothing is closed.
6. Foreground app changes between detection and action → nothing is closed.
7. Keyword and TLD hosts block; every trap host does not.
8. User allow beats the list; user block beats user allow; a snooze expires.
9. Vision, unknown host, 15 s dwell → one detector call with a 640 px frame; `explicit/high`
   → tab closed, host learned, and the next pass blocks it as `Learned` with no detector
   call; a byte search of a database dump finds no string equal to that host.
10. `explicit/high` but the window id or the fresh host changed during the call → nothing
    closed, nothing learned.
11. `explicit/medium` then `none` → ask card, nothing closed; a refusal → nothing shown; two
    refusals on a key with a signal → ask card; never an enforcement.
12. Non-browser app, `explicit/high` → `HideOthers`, ask card; `QuitApp` only after
    `card_action(close_window)`.
13. Never captured: `Popular` without a signal, `NeverCapture`, allowed and blocked hosts, a
    messaging app with default settings, a window without an id, an excluded app.
14. **No idle gate:** the idle detector reports 600 s idle and checks still run; an unchanged
    frame is skipped and not counted.
15. Readiness: `local_only`, no key, no consent, consent for another provider, vision budget
    spent, cap reached, no screen-recording permission → zero detector calls and the right
    `VisionOff`; `state.ai_health()` is unchanged in every case, and after three detector
    errors.
16. **Budgets are separate:** vision usage over its cap does not change `budget_allows`; a
    spent productivity budget does not stop vision.
17. "Não era isso" on a vision card: unlearned; a second frame 20 s later; `explicit/high`
    again → closed, learned, and the button for that key is now a pending allow; `none` →
    nothing closed, signal cadence. On a keyword block with vision not ready → a pending
    allow, no snooze. The fourth report in a day is a pending allow.
18. An app added to `blocked_apps` after enable is still a candidate until the delay passes,
    then never.
19. Commitment: `Disable` with a 600 s delay keeps enforcing; clock +600 s with 60 s of
    observed time → not ready; with 600 s observed → ready card; confirm →
    `system.remove(Shield-or-Ubiqx, All)` once; cancel → still on; unconfirmed for 24 h →
    expired.
20. **Settings cannot flip it:** `update_settings` with an old `Settings` object, and
    "Apagar todos os dados", leave `NofapConfig` and `nofap_sites` unchanged and the guard
    still enforcing.
21. Drift: an observation without the block → `reduced`, one card, none on the next pass;
    `restore_network` removes only the resolver.
22. Tracker (ubiqX): a blocked sample reaches the segmenter as `[privado]` with no URL; after
    a vision hit the open block is private with no URL and its screenshot rows and files are
    gone; `should_capture` is false for that window for 10 minutes.
23. Secondary: a fresh heartbeat from the other product → no pass, `overall: secondary`.
24. **Privacy sweep** after a scripted session with twenty checks and three hits: no file
    under the data directory starts with the JPEG magic; no card, event or notification
    captured by the recorders contains any host of the scenario; every `LlmRequest` the fake
    received has exactly the constant texts.

`crates/ubiqx-engine/tests/nofap_ipc.rs`: every request in
`apps/desktop/src/nofap/fixtures/commands.json` deserialises into a `NofapCommand`, is
handled, and returns a value that deserialises into the documented return type. The
fixtures are generated by U0's golden test in `ubiqx-core` (7.8) and are also read by
`apps/desktop/src/nofap/ipc.test.ts`.

`crates/ubiqx-app/tests/nofap_app.rs`: the zero-rows test of 8.2; `handle_host` round trips;
the standalone identity uses `shield.db` and its own keyring service.

### 11.3 Frontend tests (`pnpm test -- --run`)

- Wizard: nothing is called before the last step; the DNS and policy disclosures and the
  provider name are on screen; vision cannot be enabled without the tick; the managed tick
  appears only for "IA do Ubi"; a declined OS prompt shows "Proteção parcial"; a gated tier
  shows `system.gated`.
- `TierRow` and `IssueList` render every `TierState` and `Issue` from mocked statuses.
- Disable: countdown, "Manter proteção" cancels, confirm only when ready;
  "Restaurar minha rede" has no delay.
- Card window: the discreet card shows only "Página bloqueada pelo UBI."; no element
  contains a host from the mock state; an unknown id renders the neutral card; "Não era
  isso" calls `card_action`; urge surf advances with fake timers and sends no rating; the
  crisis card shows the phone number from the resources payload.
- Lapse, check-in, values and plans, progress with and without the streak.
- Standalone flavour: no ubiqX page in the module graph; welcome finishes with
  `onboarding_done`; the provider picker has no "IA do Ubi".
- `parity.test.ts` covers `nofap` and `nofap_app`; the forbidden-word test; the naming test;
  `csp.test.ts` unchanged (no network from the webview).

### 11.4 What cannot be tested in this sandbox (the real-machine checklist)

Kept in `docs/nofap/REAL-MACHINE-CHECKLIST.md`. Items marked **CI** are covered by the smoke
job of 9.3; the others need a person and block the claim they concern.

| # | Item | How |
|---|---|---|
| 1 | Hosts block applies, is honoured, and is removed byte-exactly on Windows, macOS, Ubuntu | **CI** |
| 2 | Windows DNS Client with 8,000 names at six per line: time to first lookup, CPU | **CI** (time); person (CPU, Defender) |
| 3 | `networksetup`, `Set-DnsClientServerAddress` / `-ResetServerAddresses`, resolved drop-in set and reset | **CI** |
| 4 | Cloudflare for Families addresses answer and refuse a listed name; terms and attribution re-read | **CI** (answer); person (terms) |
| 5 | Elevation prompts: `osascript` with an ad-hoc signed app (wording, cancel code -128, helper runs as root from inside the `.app`); UAC for an unsigned per-user install through `Start-Process -Verb RunAs` (exit code, 1223 on cancel); `pkexec` from the `.deb` with the policy file | person, each OS |
| 6 | Windows registry path of static DNS (`NameServer`) for the quick read; the state directory under `<ProgramFiles>` is readable by a standard user and not writable; `powershell.exe -Command` loads `DnsClient` and `NetAdapter` under the default `Restricted` execution policy of Windows Home | **CI** (first two); person (execution policy: the runner's policy is not the default) |
| 7 | PT-BR macOS and Windows: parsers unaffected | person |
| 8 | Does an IPv4 hosts line suppress AAAA; does the block hold when Chrome or Firefox use DoH; does Safari with iCloud Private Relay honour the hosts block and the resolver | person |
| 9 | Policies: `chrome://policy`, `edge://policy`, `about:policies` on unmanaged Windows 11 Home and Ubuntu; Firefox on macOS; the "managed" banner wording; Snap and Flatpak | person |
| 10 | Captive portal with the filter set; "Restaurar minha rede" fixes it | person |
| 11 | Vision on real providers: refusal rate on explicit screenshots, false positives on ordinary and borderline screens, latency, real cost per check at 640 px | person, with consented test images |
| 12 | Each AI provider's current usage and retention policy for sending possibly explicit screenshots; the consent text adjusted to what they say | person (owner) |
| 13 | The card window on all workspaces without taking focus; `hide_others` on the three systems; the Windows address-bar focus fix | person |
| 14 | macOS: Screen Recording and Automation prompts raised by the module without the tracker; state after an update | person |
| 15 | `.deb` `prerm` and NSIS uninstall hooks run the removal on uninstall **and do not run it on an update** (dpkg `upgrade`, the in-app updater on Windows); the NSIS hook's UAC prompt; two products installed side by side | person |
| 16 | Every phone number and URL in `resources.json` | person; the release-mode test enforces it |
| 17 | INPI and USPTO searches for the display name | owner |
| 18 | Longest `--plan-b64` argument that survives `Start-Process -Verb RunAs` | **CI** (9.3 step 5) |
| 19 | The log file of each shell after a day of use holds no line per check and no host (4.4) | person; the capture test covers the Rust side |

---

## 12. Deferred, each with its seam

| Item | When | Seam already in this build |
|---|---|---|
| Signed list pack with the Extended tier (CC BY-SA 4.0, 523k entries), weekly CI job, Ed25519 key | v1.1 | `Matcher::new` takes tables as byte slices; `ed25519-dalek` is in the workspace |
| Small helper binary as the elevation target (the GUI executable is never run as root) | v1.1 | `src/bin/ubiqx-shield-helper.rs` exists and is what CI runs; only bundling (`externalBin`) and the `exe` passed to `Elevator` change |
| DoH-endpoint and web-proxy names, and SafeSearch pins, in the hosts block | after checklist item 8 | `hosts::names_for` takes ordered groups |
| Learned sites in the hosts block (opt-in, with the "other accounts can read it" sentence) | v1.1 | would need the plain name; today only the hash is kept |
| NetworkManager and plain `resolv.conf` on Linux; DoT/DoH resolver modes; a second resolver (CleanBrowsing Family needs written clearance) | v1.1 | `OsBackend`, `RESOLVERS` |
| macOS configuration profile for Chromium policies and encrypted DNS | v1.1 | `policies::desired` |
| `minimize_app`; closing only the offending window of a non-browser app | later | `CardAction::CloseWindow` |
| "IA do Ubi" and a plan for the standalone product | owner decision (question 5) | `NofapEnv::AiReadiness::LicenceRequired` |
| Standalone composition crate that does not link `ubiqx-engine` | v1.1 | `NofapService` already depends on ports only |
| Privileged background helper that re-applies silently | v2, explicit opt-in, after code signing | `plan::assess` is the pure decision it would reuse |
| Evening prompt, programmes, questionnaires, AI-written replies, accountability partner, mobile, cloud sync | not planned for v1.x | |
| On-device image model in place of the remote provider | research | `ExplicitDetector` |

---
## 13. Work breakdown

Ten units. **File ownership is disjoint:** after U0 lands, a file is edited only by the unit
that lists it. U0 owns every shared file (workspace and crate manifests, `ports.rs`,
`lib.rs` of core and of the new crate, the i18n index) and **seeds** the files marked
"seeded by U0" with the public signatures of section 7 and `todo!()` bodies, or with valid
placeholder data, so that every other unit compiles against the contract from the first
minute. Seeded stubs carry `#[allow(unused_variables)]` so that clippy stays clean; the
owning unit removes it. A seeded file then belongs to the unit named for it; U0 never
touches it again.
No unit other than U0 adds a dependency or edits a `Cargo.toml` or `Cargo.lock`.

Every Rust unit ends with `cargo fmt --all -- --check` and
`cargo clippy --workspace --exclude ubiqx-desktop --exclude ubiqx-nofap-desktop --all-targets -- -D warnings`.
The two shell crates cannot be compiled in this sandbox (no webkit); their code is kept thin,
is checked by `cargo fmt`, and is compiled by the three CI app jobs.

### Order

```
U0 ──┬─ U1 ─┐
     ├─ U2 ─┼─ U4 ─┐
     ├─ U3 ─┴──────┼─ U5 ── U7 ──┐
     ├─ U6 ────────┴─────────────┼─ U8
     └─ U9 (starts after U0; its final check runs after U8)
```

Wave 1 in parallel: U1, U2, U3, U6 (and U9's documents). U4 may start in wave 1 against the
seeded stubs and finishes when U1 and U2 have landed. Then U5, then U7, then U8.

### U0 Foundation and contracts (first, alone)

Owns: `Cargo.toml` (members `crates/ubiqx-nofap` and `apps/nofap/src-tauri`; the former also
a default member; workspace dependency `ubiqx-nofap`), `Cargo.lock`;
`crates/ubiqx-nofap/Cargo.toml`;
`crates/ubiqx-platform/Cargo.toml`, `crates/ubiqx-engine/Cargo.toml`,
`crates/ubiqx-app/Cargo.toml`, `apps/desktop/src-tauri/Cargo.toml`
(`tauri-plugin-single-instance`), `apps/nofap/src-tauri/Cargo.toml`,
`apps/nofap/src-tauri/build.rs`, `apps/nofap/src-tauri/src/main.rs`;
`crates/ubiqx-core/src/nofap.rs`, `nofap_ipc.rs`, `product.rs`, `lib.rs`, `ports.rs`,
`focus.rs`, `update.rs`; `crates/ubiqx-nofap/src/lib.rs`;
`apps/desktop/src/i18n/index.ts`; `apps/desktop/src/nofap/fixtures/commands.json` and
`views.json` (generated by the golden test of 7.8; U0 keeps them for the whole build).

If the crate registry cannot be reached from the sandbox, `tauri-plugin-single-instance` is
left out, U0 says so in its hand-off, and the two-engines fault stays as it is today;
nothing else in this design depends on the plugin.

Seeds (then owned by the unit in brackets). A seeded Rust file holds its **complete type
definitions** (every enum, struct, trait and constant of sections 1 to 7 that lives in that
file, with serde attributes) and every public function signature with a `todo!()` body.
Only bodies are left to the owner.

- Every module file of `crates/ubiqx-nofap/src/` [U1, U2].
- Placeholder data [U1, U2]: valid `core.bin` and `popular.bin` with a dozen keys,
  `hosts-tier.txt` with twenty names, `never-block.txt`, `mixed.txt` and `never-capture.txt`
  with five real entries each, the other data files empty.
- `crates/ubiqx-engine/src/deps.rs` and `engine.rs` [U5]: `NofapPorts`,
  `NofapPorts::inert()` with its inert implementations, the fields `EngineDeps.nofap` and
  `EngineDeps.identity`, `LoopConfig.nofap_every` and `nofap_verify_every`. The five places
  that build those structs by literal get the new fields at the same time:
  `crates/ubiqx-engine/tests/focus.rs` (twice), `tests/pipeline.rs`,
  `tests/screenshot_read.rs` [U5] and `crates/ubiqx-app/src/lib.rs:360` [U7]. These are the
  only breaking changes to existing structs. U0 makes them once, because otherwise U5 would
  leave `ubiqx-app`, which it does not own, uncompilable until U7.
- `crates/ubiqx-platform/src/sysblock/mod.rs` [U4] with the traits `OsBackend`, `Runner`,
  `Fs`, `Elevator`, `NameLookup` and the constructor signature of `NativeSystemShield`;
  `crates/ubiqx-platform/src/frame.rs` [U4] with `WindowFrameSource`.
- `apps/nofap/src-tauri/src/app.rs` [U8] containing `pub fn run() {}`. `main.rs` declares
  `mod app;`, and `cargo fmt --all` fails on a module whose file does not exist.
- `apps/desktop/src/i18n/messages/nofap.ts` [U6] and `nofap_app.ts` [U8] as empty
  namespaces.

Contract rules for every later unit:

1. The public items U0 seeded are the contract. Where this document and the seeded code
   differ, the code wins, and U0 lists each difference in its hand-off.
2. No unit changes the signature or the serialised form of a public item that another unit
   uses. A unit that needs such a change stops and reports it; the integrator makes it in
   one commit across the files it touches. A new item that nobody else uses is free.
3. Pure functions receive their data as arguments (`validate(plan, never_block, gate)`,
   `names_for(tier, extras, os)`, `Matcher::new(core, popular, …)`); only `list.rs` reads the
   embedded files. The tests of U2 and U4 build their own small tables, so they pass
   whatever U1 has landed. Assertions about the real embedded data live only in U1's
   `tests/embedded.rs`.

Accept:
- the workspace compiles and every existing test passes unchanged:
  `cargo test --workspace --exclude ubiqx-desktop --exclude ubiqx-nofap-desktop`
- `cargo test -p ubiqx-core nofap` (config from `{}`, sanitise, round trips, the no-host
  serialisation test, identities in the exempt lists, `feed_matches_product`, and
  `nofap_ipc_golden` equal to the two committed fixture files)
- `cargo fmt --all -- --check` passes with the new workspace member
- `cd apps/desktop && pnpm typecheck && pnpm test -- --run` (parity holds with the two empty
  namespaces)
- `cargo metadata --format-version 1 >/dev/null` succeeds with the new member

### U1 Blocklist pipeline and matcher (after U0)

Owns: `scripts/nofap/build_blocklist.py`; `THIRD-PARTY-NOTICES`;
`crates/ubiqx-nofap/src/list.rs`, `matcher.rs`, `heuristics.rs`;
`crates/ubiqx-nofap/data/core.bin`, `popular.bin`, `hosts-tier.txt`, `never-block.txt`,
`mixed.txt`, `never-capture.txt`, `extra-ptbr.txt`, `canary-block.txt`, `canary-allow.txt`,
`sources.lock.json`, `core.meta.json`, `NOTICE.txt`; `crates/ubiqx-nofap/tests/embedded.rs`.

Accept:
- `python3 scripts/nofap/build_blocklist.py` passes its gates and a second run is
  byte-identical; `python3 scripts/nofap/build_blocklist.py --check` passes
- `core.bin` holds 215,000 to 300,000 keys and is at most 2.4 MB; `hosts-tier.txt` has at
  most 20,000 valid names, none under `never-block.txt`; `popular.bin` at most 1 MB
- `cargo test -p ubiqx-nofap list:: matcher:: heuristics::` and
  `cargo test -p ubiqx-nofap --test embedded`: every row of 3.6
- the review list of listed names inside the CrUX top-10,000 is attached to the hand-off

### U2 Decision logic and copy data (after U0)

Owns: `crates/ubiqx-nofap/src/hosts.rs`, `resolvers.rs`, `policies.rs`, `plan.rs`,
`gate.rs`, `state_dir.rs`, `commitment.rs`, `sampling.rs`, `episode.rs`, `exclusions.rs`,
`primary.rs`, `messages.rs`, `help.rs`, `progress.rs`;
`crates/ubiqx-nofap/data/resources.json`, `forbidden-words.json`;
`crates/ubiqx-nofap/tests/naming.rs`.

Accept:
- `cargo test -p ubiqx-nofap hosts:: plan:: policies:: gate:: state_dir:: commitment:: sampling:: episode:: exclusions:: primary:: messages:: help:: progress::`:
  the cases of 11.1 for this crate, including the splice property test, the exhaustive
  `classify`, the clock-jump cases and the absence of any idle input in `SampleCtx`
- `cargo test -p ubiqx-nofap --test naming`
- every `TIER_GATE` entry is `false`

### U3 Storage and AI adapters (after U0)

Owns: `crates/ubiqx-storage/src/**`, `crates/ubiqx-storage/tests/**`;
`crates/ubiqx-ai/src/**`, `crates/ubiqx-ai/tests/**`; `services/ubi-api/**`.

Accept:
- `cargo test -p ubiqx-storage`: V3 on a V2 database, the repository, the wipe, lenient
  kinds, `totals_by_kind`, `delete_for_block`
- `cargo test -p ubiqx-ai`: `explicit` request shape, parsing, refusal, truncation, trace
  redaction, no log line and an hour-truncated usage row for `NofapVision`, the two wiremock
  round trips; the existing suites unchanged
- `cargo test --manifest-path services/ubi-api/Cargo.toml` and its clippy and fmt

### U4 Platform system layer (after U0; finishes after U1 and U2)

Owns: `crates/ubiqx-platform/src/**` (including `sysblock/`, `frame.rs`,
`bin/ubiqx-shield-helper.rs`, fixtures).

Accept:
- `cargo test -p ubiqx-platform`: the unit cases of 11.1 and the seven integration cases of
  11.2 for each of the three back-ends
- `cargo check -p ubiqx-platform --target aarch64-apple-darwin` and
  `--target x86_64-pc-windows-msvc`
- `cargo run -p ubiqx-platform --bin ubiqx-shield-helper -- status` prints valid JSON on
  Linux; `-- bogus` exits 2; `-- apply --plan-b64 <valid>` exits 5 (gated) and writes nothing
- `grep -rnE "cfg\((target_os|windows|unix)" crates/ubiqx-platform/src/sysblock` matches only
  `sys.rs`
- no file under `sysblock/` is longer than 400 lines apart from fixtures and tests

### U5 Engine service (after U1 to U4)

Owns: `crates/ubiqx-engine/src/**`, `crates/ubiqx-engine/tests/**`.

Accept:
- `cargo test -p ubiqx-engine --test nofap`: the 24 scenarios of 11.2
- `cargo test -p ubiqx-engine --test nofap_ipc`
- `cargo test -p ubiqx-engine`: the existing suites pass with the new `EngineDeps` and
  `LoopConfig` fields and unchanged behaviour
- `grep -rn "EngineState" crates/ubiqx-engine/src/nofap` matches only `env.rs`

### U6 Frontend module (after U0; needs no backend)

Owns: `apps/desktop/src/nofap/**` except `fixtures/` (U0's, read-only here);
`apps/desktop/src/i18n/messages/nofap.ts`;
`apps/desktop/src/App.tsx`, `components/layout/Sidebar.tsx`, `lib/mock.ts`,
`pages/Settings.tsx`, `pages/Settings.test.tsx`.

Accept, in `apps/desktop`:
- `pnpm typecheck`
- `pnpm test -- --run`: the cases of 11.3 except the standalone ones; parity; forbidden
  words; naming; the IPC fixture test
- `pnpm build`
- every string of 5.6 is present with the exact PT-BR and EN text

### U7 Composition and ubiqX shell (after U5)

Owns: `crates/ubiqx-app/src/**`, `crates/ubiqx-app/tests/**`;
`apps/desktop/src-tauri/src/**`, `apps/desktop/src-tauri/capabilities/shield-card.json`,
`apps/desktop/src-tauri/tauri.conf.json`, `apps/desktop/src-tauri/Info.plist`,
`apps/desktop/src-tauri/packaging/**`.

Accept:
- `cargo test -p ubiqx-app`: the zero-rows test and `handle_host` of 11.2; the existing test
- `cargo test -p ubiqx-cli` passes and `ubiqx simulate` gives the same output as before
  (the module is off, so its pass touches no platform port)
- `cargo fmt --all -- --check` covers the shell files; the shell compiles in the three CI
  app jobs
- the shell adds no logic: `apps/desktop/src-tauri/src/nofap.rs` is under 200 lines

### U8 Standalone app (after U6 and U7)

Owns: `apps/nofap/**` except the three files of U0; `apps/desktop/src/nofap-app/**`;
`apps/desktop/src/i18n/messages/nofap_app.ts`; `apps/desktop/index.nofap.html`,
`apps/desktop/vite.config.ts`, `apps/desktop/package.json`.

Accept, in `apps/desktop`:
- `pnpm typecheck`; `pnpm test -- --run src/nofap-app`: the standalone cases of 11.3
- `pnpm build` and `pnpm build:nofap`; `dist-nofap` contains no chunk of Timeline, Review,
  Reports, Categories, Insights or Focus
- `apps/nofap/src-tauri/tauri.conf.json`: identifier `ai.ubiqx.shield`, `productName` equal
  to `DISPLAY_NAME` (`cargo test -p ubiqx-nofap --test naming` reads it), its own updater
  endpoint
- `apps/nofap/src-tauri/src/app.rs` is under 350 lines; it compiles in the three CI app jobs

### U9 CI, release and documents (starts after U0; final check after U8)

Owns: `.github/workflows/ci.yml`; `scripts/publish-release.mjs`,
`scripts/publish-release.test.mjs`, `scripts/app-version.mjs`, `scripts/ci-app-version.sh`,
`scripts/nofap/system-smoke.sh`, `scripts/nofap/system-smoke.ps1`;
`docs/nofap/PRIVACY.md`, `REMOVAL.md`, `REAL-MACHINE-CHECKLIST.md`; `docs/ARCHITECTURE.md`,
`README.md`, `docs/WINDOWS-LINUX.md`, `docs/LICENSING.md`.

Accept:
- `node --test scripts/publish-release.test.mjs` passes for both products; the ubiqX
  artefact and feed names are unchanged
- `publish-continuous` has the `refs/heads/main` condition; `publish-shield` is conditional
  on `vars.NOFAP_PUBLISH`; both shells are excluded from the Linux clippy and test steps
- `bash -n scripts/nofap/system-smoke.sh` passes; the script refuses to run unless it is
  root and either `CI=true` or `UBIQX_SHIELD_SMOKE=1` is set, so it can never be started by
  accident on a developer's machine; its real runs are the job of 9.3 and, for
  `--tiers hosts` on Linux, the gate-opening step below
- `PRIVACY.md` states, for each layer, what is captured, sent and stored, in the words of
  this design; `REMOVAL.md` has copy-paste commands for every packaging format; the
  checklist of 11.4 is complete; the guard and the module are documented in
  `ARCHITECTURE.md`

### Final gate (whoever integrates)

```
cargo fmt --all -- --check
cargo clippy --workspace --exclude ubiqx-desktop --exclude ubiqx-nofap-desktop --all-targets -- -D warnings
cargo test --workspace --exclude ubiqx-desktop --exclude ubiqx-nofap-desktop
cargo check -p ubiqx-platform --target aarch64-apple-darwin
cargo check -p ubiqx-platform --target x86_64-pc-windows-msvc
cargo test --manifest-path services/ubi-api/Cargo.toml
python3 scripts/nofap/build_blocklist.py --check
(cd apps/desktop && pnpm typecheck && pnpm test -- --run && pnpm build && pnpm build:nofap)
node --test scripts/publish-release.test.mjs
```

**Gate-opening step** (the integrator, after the commands above; each flip is its own small
commit touching only `gate.rs`, its test and the checklist):

1. In this sandbox:
   `cargo build -p ubiqx-platform --bin ubiqx-shield-helper && UBIQX_SHIELD_SMOKE=1 scripts/nofap/system-smoke.sh --tiers hosts`.
   Green → `(Linux, hosts) = true`, output pasted into `REAL-MACHINE-CHECKLIST.md`.
2. After the branch is pushed and `nofap-system-smoke` has run: for each OS whose run is
   green, `hosts` and `resolver` for that OS become `true`, with the run URL in the
   checklist. On Windows the same commit sets `ARG_BUDGET` from step 5 of the job.
3. `policies` stays `false` everywhere until a person has done checklist item 9.
4. The hand-off to the owner lists, per OS, which tiers are open. If none is, it says in its
   first line that this build does not block at DNS level yet.

### If time runs out: what to cut, in this order

Each cut leaves the contracts of U0 intact and a truthful status in the UI.

1. L4 back-ends (the tier reports `Unsupported { reason: "not_built" }`; `policies.rs`
   stays). It is the last tier to open in any case.
2. The streak, the daily check-in, the export, "Ver um exemplo do que é enviado".
3. Vision in media and messaging apps (browsers stay).
4. The Windows quick registry read (the full read every 10 minutes replaces it).
5. The standalone updater feed and `publish-shield` (the app is still built in CI).

Not to be cut: L1; the hosts tier with gate evidence on at least one OS; vision in browsers
with its consent, caps, and the logging and usage rules of 4.4 and 4.8; the copy bank and
the forbidden-word test; the commitment delay; `REMOVAL.md` and the undelayed exits; the
standalone shell with the zero-rows test; the release-channel change of 9.1.

---

## 14. Questions for the owner (each with the default that is built in)

| # | Question | Default in this build |
|---|---|---|
| 1 | **The name.** You asked for a module called "nofap". NOFAP is a registered and litigated US mark, Brazil is unknown, and the name on the Dock tells anyone nearby what the app is for. Do you want users to see "NoFap"? | Code, tables, IPC and directories are `nofap`. Users see **"Escudo"** (one constant). Showing "NoFap" is a three-line change (0.1). "Escudo" is not cleared either: the standalone app is built in CI but not published until an INPI and USPTO search is done. |
| 2 | **Third-party DNS.** The family resolver sends every DNS query of the machine to Cloudflare. | Preselected as "recomendado" with the plain disclosure; "só a lista neste computador" is one click away. One provider. |
| 3 | **Vision.** One reduced image of the window goes to Anthropic, OpenAI or xAI under the user's key (or through the ubiqX proxy after a second tick); private windows are checked; an `explicit` + `high` answer closes the tab without asking. Someone must read each provider's current policy on receiving such images before launch. | On in the wizard when a provider is configured, behind an unticked consent box; messaging apps off; caps 20 an hour, 120 a day, US$ 2 a month; close on high confidence, ask otherwise; learned sites kept as hashes only. No log line per check and no exact time in the usage ledger. The consent says the provider applies its own rules to what it receives; that can include suspending a key or reporting material it judges illegal, which the app neither does nor prevents. |
| 4 | **Release.** Today every push to any branch is offered to installed apps, and the system tiers (hosts, DNS, policies) cannot be tested from this sandbox. | `publish-continuous` restricted to `main`; the module ships off; each system tier opens per OS when the CI smoke run on that OS is green (`policies` only after the manual browser check). Until then the module blocks in-app and by vision, and says so. `(Linux, hosts)` can be opened from a run in the build sandbox; the other entries need the pushed branch's CI run. A build with no entry open does not block at DNS level, and the hand-off must say so in its first line. |
| 5 | **Price of the standalone product.** No plan exists for it, and a new plan makes older builds read the key as invalid. | Blocking and counselling are free in both products and never licence-gated. The standalone app takes the user's own AI key; "IA do Ubi" is ubiqX-only for now. |
| 6 | **How hard is "off"?** | A 10-minute delay chosen by the user, then a confirmation; "Restaurar minha rede" is never delayed; no partner, no lock, no forced autostart, no silent re-apply; an administrator can always remove it and the app says so. Sites the user adds go into the hosts file (readable by other accounts) unless they choose "guardar só neste app". **Quitting the app is immediate** (with a truthful dialog, a counter and "Manter aberto" as the primary button); while no system tier is applied, quitting switches everything off. The alternative, if you want it stricter, is that "Sair" waits the same delay while the module is on, with the operating system's force-quit as the way out. Likewise `local_only`, removing the AI key and changing the vision model stop the AI check at once, because they are privacy controls. |

---

## Appendix A. Where each must-fix of the judges is resolved

| Must-fix | Section |
|---|---|
| Name put to the owner, mechanical rename, mapping stated | 0.1, 14 (1) |
| Release channel restricted before elevating code lands; module off by default | 9.1, 1.8 |
| Unverified OS behaviour behind a per-OS, per-tier gate; conservative Windows cap | 1.8, 2.2, 9.3, 11.4 |
| No root daemon, inbox, keep-alive or disabled quit | 2.5, 6.4 |
| Windows state directory ACL; nothing read back is trusted | 2.5 (state under `<ProgramFiles>`, no ACL repair; "nothing read back … becomes an address, a path or a command") |
| Root never writes to or follows a caller-chosen path | 2.1 (plan as an argument), 2.5 |
| No network setting from unprivileged or network input | 2.3 (resolver by id; undo is "automatic"), 2.2 (no pins) |
| Vision idle gate removed; unchanged-frame skip | 4.3 |
| Vision independent of `budget_allows`, `remote_allowed` and `AiHealth`, both ways | 4.1, 11.2 (15, 16) |
| Stale URL cache; fresh re-read before close | 1.2 (rules 2 and 3), 11.2 (5, 6) |
| No destructive action on an uncertain signal; refusal never a positive | 1.2, 4.6 |
| Tracker forgets a hit | 1.4, 11.2 (22) |
| Messaging apps off by default; never-capture list; popular table; no display fallback | 4.2, 3.3 |
| No visited or learned host in a world-readable file | 3.7, 2.2 |
| Capture exclusions honoured, but effective after the delay | 4.2, 11.2 (18) |
| "Não era isso" is short, limited, and does not stop sampling | 5.4 |
| Two products: owners set and a primary lock | 1.6, 2.5 |
| Browser DoH: policy tier; nothing written for Chromium on macOS | 2.4 |
| Loosening needs a fresh confirmation and expires; declined prompt is stated | 6.2 |
| Clock, Settings write and "Apagar todos os dados" cannot defeat the delay | 6.1, 6.2, 11.2 (19, 20) |
| Image logging redacted | 4.4 |
| Provider and proxy terms; consent names the provider; no per-feature tag | 4.4, 5.2, 14 (3), 11.4 (12) |
| macOS permissions checked by the module itself | 1.3 |
| Hosts canary is one fixed name | 2.2 |
| Elevation quoting by tested functions; AppImage refused | 2.5 |
| Standalone provably does not track; own identity; refuses the other feed | 8.2, 7.2, 11.2 |
| Counselling release gates | 5.1, 5.7, 11.4 (16) |
| Blocklist data outside `ubiqx-core`; 90-day retention; separate export | 3.2, 7.4 |
| Licences: permissive sources only embedded; no GPL; blocking never gated | 3.1, 8.5 |
| Never a trap: undelayed restore, removal without the app, correction path | 6.3, 2.7, 5.4 |

---

## Review changes

Adversarial review of 2026-11-19, before any implementation. The design was read in full
against the request, the research reports, the code and, for three operating-system facts
(the `ShellExecuteEx` parameter limit, per-user environment variables in an elevated
process, policies for the Firefox snap), the web. Everything below was changed in place in the sections named.

### What was changed, and why

**The request**

| # | Change | Why | Where |
|---|---|---|---|
| R1 | Opening the DNS gate is now a named step of the final gate, and `(Linux, hosts)` can be opened from a run in the build sandbox. A build with no entry open must be reported as not blocking at DNS level. | Every gate entry shipped `false` and no unit owned flipping one, so the build as designed delivered no DNS blocking, which is the first thing the request asks for. The sandbox is root with a regular `/etc/hosts` and `hosts: files dns` (checked). | 0.3, 1.8, 9.3, 13, 14 (4) |

**Operating-system mechanisms**

| # | Change | Why | Where |
|---|---|---|---|
| R2 | Plan argument budget: 1,800 characters on Windows until measured; `build_plan` drops what does not fit and says how many; the helper writes the `www.` twin itself; the smoke job measures the real limit. | Windows elevation goes through `ShellExecuteEx`. Its parameter string has no documented limit and is reported to fail above about 2,048 characters. The design sent up to 24,000. | 2.1, 9.3 (5), 11.2 (10), 11.4 (18) |
| R3 | The Firefox snap and the Chromium snap are covered by the `/etc` policy paths; only Flatpak is unsupported. | Ubuntu's own guide says the Firefox snap reads `/etc/firefox/policies/policies.json`. The design had marked the default Ubuntu browser as unsupported. | 2.4 |
| R4 | Linux resolver tier is `Unsupported` when `/etc/resolv.conf` does not point at systemd-resolved. | The service can be active and unused; the drop-in would then change nothing while the status said `Applied`. | 2.3 |
| R5 | The helper probes name resolution before and after the DNS change and undoes the resolver tier itself when the change broke it. | On a network that blocks outside DNS the user was left without names for about six minutes and then needed a second administrator prompt. | 2.3, 11.2 (8) |
| R6 | Hosts write: owner kept, link targets checked, in-place fallback for a mount point, read-back with restore. | `rename` over `/etc/hosts` fails in containers and on immutable systems; nothing verified the result. | 2.2, 11.2 (9) |
| R7 | `prerm` acts only on `remove`; the NSIS hook is skipped on update and raises its own UAC prompt. | dpkg runs `prerm upgrade` on every update, which would have removed the protection each time. The Windows installer is per-user and cannot change system files unelevated. | 2.7, 11.4 (15) |
| R8 | Safari with iCloud Private Relay added to the known gaps and the checklist. | It resolves outside the system resolver and may ignore the hosts file (unverified). | 2.3, 11.4 (8) |
| R9 | Open point recorded: PowerShell's default `Restricted` execution policy and the `DnsClient` module, with the `netsh` fallback named. | Not verifiable here; the CI runner's policy is not the default. | 2.5, 11.4 (6) |

**Bypass in a weak moment, and the way out**

| # | Change | Why | Where |
|---|---|---|---|
| R10 | "Não era isso" on a vision card buys one independent re-check 20 s later, not ten minutes without action. | It was a one-click pass on a screen judged explicit, three times a day. | 5.4, 11.2 (17) |
| R11 | "Não era isso" on a keyword block is a delayed request when vision is not ready. | With vision off, the 24-hour snooze was a one-click pass with nothing left watching. | 5.4 |
| R12 | Browsers the URL resolver does not support are vision candidates (`OTHER_BROWSERS`). | Tor Browser, Opera on macOS, Zen and others were "any other app": no guard and no vision. | 1.3, 4.2 |
| R13 | Windows: a focused address bar no longer hides the page's URL; the last committed URL is used while the title is unchanged. | Clicking the address bar made the resolver answer `None`, which kept a listed page open. | 1.2, 11.1 |
| R14 | Primary lock: written only while that product's module is on; a heartbeat in the future is stale. | A product with the module off, or a bad timestamp, could make the other one stand down and show "Proteção ativa pelo outro app". | 1.6 |
| R15 | Quitting: the dialog says what is actually left, the quit is counted towards the help heuristic, and the status never reads `protected` when nothing protects. | `quit.confirm` promised that the system list and DNS stay; in a build with the gate closed that is false, and quitting ends everything. Whether quitting should wait the delay is left to the owner. | 1.3, 1.7, 5.6, 6.3, 7.3, 14 (6) |
| R16 | The hosts block and the Linux drop-in carry a one-line undo hint. | After the app is deleted the help page is gone; the way out should be in the file. | 2.2 |

**Privacy**

| # | Change | Why | Where |
|---|---|---|---|
| R17 | No log line per vision check; a capture test enforces it. | The AI client writes an `info` line per call (kind, tokens, stop reason) and the shell logs `info` to a file: one timestamped line per screen check, with refusals marked, plus the verdict at `trace`. | 4.4, 7.7, 11.1, 11.4 (19) |
| R18 | `ai_usage.at` is truncated to the hour for this kind. | The ledger stores milliseconds per call, which contradicted "no row per event, no exact time". | 4.8, 7.3 |
| R19 | Local hosts (`localhost`, private addresses, `.local`, single labels) are never captured. | They were `Plain`, so a router page, a development server or an intranet was sent to the AI provider. | 3.4, 3.6, 4.2 |
| R20 | The claim that the proxy's ledger cannot tell who uses the check was removed; the managed-route consent now says what the server records. | The ledger has time and token counts per request; the pattern is recognisable. | 4.4, 5.6 |
| R21 | Copy corrected: `wizard.adult`, `vision.closed`, `vision.today`, `vision.consent`, `dns.disclosure`, the wizard summary. | They said "keeps nothing another person could read" (the hosts file and the database say otherwise), "nothing is kept" (the provider keeps what its terms say), "bank windows are never pictured" (only listed ones), and did not say Cloudflare also blocks what it calls dangerous. | 2.3, 5.2, 5.6 |
| R22 | "Ver um exemplo" captures the last candidate window. | As written it captured the foreground window, which at that moment is the app itself and is never captured. | 4.4, 7.8 |
| R23 | Stated that the provider applies its own safety rules, and that the OS keeps a notification history. | Both are outside the app and were not disclosed. | 4.4, 5.3, 14 (3) |

**Privilege and injection**

| # | Change | Why | Where |
|---|---|---|---|
| R24 | On Windows no path comes from an environment variable; the child environment, including `PSModulePath`, is built by the helper; cmdlets are module-qualified. | An elevated process started through UAC receives the user's variables. A per-user `windir` or `SystemRoot` is a documented way to redirect an elevated program, and a per-user `PSModulePath` would have let PowerShell load a module from a user folder. | 2.2, 2.5, 9.3 (6), 11.2 (6) |
| R25 | Windows state directory moved under `<ProgramFiles>`; the `icacls ... /T` repair is gone. | A standard user can pre-create a folder, or a junction, under `%ProgramData%`. The recursive ACL repair would have followed the junction and rewritten the ACLs of its target. | 2.5, 11.4 (6), Appendix A |
| R26 | "Removed and recreated" for the Unix state directory is defined as never following a link. | Unspecified before. | 2.5 |
| R27 | The disclosure about a user-writable executable covers macOS and development builds too, and says what the helper cannot be made to do. | It mentioned only per-user Windows installs. | 2.5 |

**Work breakdown and contracts**

| # | Change | Why | Where |
|---|---|---|---|
| R28 | The wire types move to `ubiqx_core::nofap_ipc` and U0 generates the fixtures from Rust; the frontend checks them with `satisfies`. | U6 was to write the fixtures by hand from a table while U5 implemented the handler: two units inventing the same shapes in parallel. | 7.7, 7.8, 10, 11.2, 13 |
| R29 | U0 makes the breaking struct changes (`EngineDeps`, `LoopConfig`) and patches the five literal sites. | `crates/ubiqx-app/src/lib.rs:360` builds `EngineDeps` by literal. U5 adding fields would have left a crate it does not own uncompilable, and its own acceptance (workspace clippy) unreachable. | 13 (U0) |
| R30 | U0 seeds complete type definitions, `sysblock/mod.rs` traits, and `apps/nofap/src-tauri/src/app.rs`. Three contract rules added. | "Signatures of section 7" left about twenty types undefined; `cargo fmt --all` fails on `mod app;` without the file; U2's tests depended on U1's data. | 13 |
| R31 | The vision job is spawned, not awaited; the runtime mutex is never held across a platform call. | Awaiting the check stopped the guard for up to 32 s per check; holding the lock during the password prompt stopped it for as long as the prompt was open. | 1.1, 7.7 |
| R32 | A cut order. | The scope is large for one build; cuts should not be improvised or break a contract. | 13 |
| R33 | Small corrections: the learned-sites cross-reference (3.7, not 3.6); the state-file example; where the module's spend is shown; an `elevate-argv` verb that only prints, for the smoke job. | Accuracy. | 2.1, 2.5, 4.7 |

### Checked and left as it is

- Commands, paths and values: `networksetup -setdnsservers … empty`,
  `Set-DnsClientServerAddress … -ResetServerAddresses`, the registry value `NameServer`, the
  resolved drop-in with `Domains=~.`, the Cloudflare for Families addresses, the Chromium,
  Edge, Brave and Firefox policy names and locations, `pkexec` exit codes 126 and 127,
  AppleScript error -128, and nothing written for Chromium on macOS. These were compared
  with the research reports and with what the reviewer knows of each tool, not run. Whether
  Chrome on an unmanaged Windows machine honours the policies stays [U] (checklist item 9):
  a search found no confirmation either way.
- Licences: the embedded sources are MIT, Unlicense and CC BY 4.0 with a notice file; no
  share-alike or GPL list is shipped; CrUX is attribution-only.
- The code references the design relies on: the `Enforcer` methods, the 20 s URL cache, the
  focus guard's cooldown, the two trace lines, the `WIPE` list, the migration version.
- The counselling copy: no medical or pseudo-scientific claim, no shaming word, help
  resources gated on a person verifying them.

### Still open (needs a decision or a real machine)

1. **Should quitting wait the delay?** Today "Sair" is immediate. While no system tier is
   applied it switches everything off in two clicks, so the 10-minute delay protects only
   against the in-app switch. The stricter option is in question 6. The same applies to
   `local_only`, removing the AI key and changing the vision model, which stop the AI check
   at once because they are privacy controls.
2. **A second window.** The guard and vision look only at the window in front. A listed page
   loaded and left in a window that is not in front is not closed. Only the DNS tiers cover
   it, which is one more reason the gate has to be opened in this build.
3. **Wayland.** With no system tier open, nothing protects a Wayland session. The status
   says so; opening `(Linux, hosts)` from the sandbox is what changes it.
4. **The Windows numbers.** `ARG_BUDGET` (1,800), `CAP_WINDOWS` (8,000) and the PowerShell
   execution-policy question are settled only by the Windows smoke run and one check on a
   Windows Home machine.
5. **Providers' terms.** Nobody has read what Anthropic, OpenAI and xAI say today about
   receiving screenshots that may be sexually explicit, their retention, and what their
   safety systems do with material they judge illegal. This blocks switching vision on for
   users, not building it (checklist item 12).
6. **The name.** Neither "NoFap" nor "Escudo" is cleared (question 1). Publishing the
   standalone product stays off until one is.
7. **The elevated executable is the app itself.** Where the user's account can write to it,
   a program running as that user can replace it and wait for the next administrator
   prompt. Bundling the small helper and signing the builds is deferred (section 12); the
   owner should know this ships as is.
8. **Scope.** Ten units, three operating-system back-ends that cannot run here, and two
   shells that cannot be compiled here. The cut order in section 13 is the fallback.
