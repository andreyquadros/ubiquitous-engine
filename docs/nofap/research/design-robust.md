# Design proposal (enforcement-robustness first): the `shield` module and the standalone product

Status: design only. Nothing in this document was compiled or run. Every per-OS system
behaviour below comes from `research-dns-enforcement.md`, which itself rests on search extracts,
not on real machines; items that must be proven on real hardware are marked **[verify]** and
collected in section 9.4.

Inputs read: `map-focus-guard.md`, `map-vision-ai.md`, `map-engine-storage.md`,
`map-app-shell.md`, `research-dns-enforcement.md`, `research-blocklists.md`,
`research-counselling.md`, and the code where a decision depended on it (`ports.rs`,
`engine.rs`, `deps.rs`, `state.rs`, `client.rs`, `openai.rs`, `segmenter.rs`, workspace
`Cargo.toml`, `tauri.conf.json`, `ci.yml`).

Naming used in this document (section 8 explains why): the code identifier is **`shield`**, the
machine-level owner id is **`ubi-shield`**, the module is called **Escudo** in the ubiqX UI, and
the standalone product has the working name **Leme**. The word "nofap" stays in the
`docs/nofap/` folder and nowhere else.

---

## 0. The design in one page

**Goal of this angle.** In a weak moment the user should find that the easy ways around the
block are closed, that the block came back after a reboot, and that turning it off takes a
deliberate request plus a wait. The user can always get out, through one documented path.

**Seven decisions that define it**

1. **Three protection levels, chosen by the user at opt-in; the recommended one installs a root
   sentinel.** `Guard` (no admin: in-app guard and vision only), `Shield` (one elevation:
   hosts + resolver + browser policies, drift needs a prompt to repair), `Locked` (one
   elevation installs a root-owned helper that the OS scheduler runs every 60 s; it repairs
   drift silently and is the only thing that can remove the protection).
2. **Tightening is instant, loosening waits.** Every change that weakens protection (turn off,
   pause, allow a site, lower the delay, drop a layer, stop the vision check) becomes a pending
   request that matures after the user's delay (default 30 min). In `Locked` the delay is kept
   by the root helper in a root-owned state file, so editing the app's database or settings
   does nothing.
3. **The protective configuration is not in `Settings`.** `Settings` is a whole-struct,
   last-writer-wins blob (map-engine-storage 1.1). Protective state lives in a dedicated table
   and, at levels `Shield`/`Locked`, in the machine state file. `Settings.shield` holds
   cosmetic preferences only.
4. **The in-app guard has no enforcement cooldown and escalates.** Close tab, then blank tab,
   then quit the browser, re-reading the URL without cache before each step. The 20 s window
   of the focus guard (map-focus-guard 11.1) does not exist here.
5. **Lists decide first, vision is for what lists cannot know.** A list or keyword hit never
   sends a screenshot. Vision samples only unknown, mixed or signalled windows, with its own
   budget and its own health, and ignores the tracker's privacy switches (incognito, pause,
   private mode) because each of them is an escape hatch. It honours `local_only`.
6. **What is detected is learned into the system block.** A visited listed domain, a
   block-grade keyword domain and a confirmed vision positive are added to the hosts block, so
   the second attempt fails at DNS level in every browser.
7. **One mechanism, two hosts.** A `ShieldService` built from ports (the `UpdateChecker` shape)
   runs inside ubiqX's engine and inside a small host for Leme. One machine-level state, one
   marker, one helper: two installed products cannot undo each other.

**What it deliberately is not**: covert, unbreakable, a treatment, or a way to watch another
person. An administrator can always remove it by hand; `docs/SHIELD-REMOVAL.md` (section 5.5)
says how. The app never says "unbreakable" (research-counselling 1.5 item 8).

**How it differs from the obvious alternative.** The obvious design is the one the DNS report
recommends for v1: apply hosts + resolver + policies once with an admin prompt, detect drift
from the unprivileged app and ask the user to approve a repair, keep the on/off switch in
settings with UI friction, and close tabs with the focus guard as it is. That design blocks
well on a calm day and fails in the moment it is for: the same person who is tempted is the one
asked to approve the repair, the switch is one settings write away, quitting the app stops the
guard, and a closed tab can be reopened for 20 s at a time. This design moves the repair and
the delay out of the user's session and into a root-owned, scheduler-driven helper, at the
price of a larger and more sensitive system layer (section 11, risk 1).

---

## 1. Enforcement tiers

### 1.1 Layers and levels

| Layer | What it does | Level `Guard` | Level `Shield` | Level `Locked` |
|---|---|---|---|---|
| L0 Guard pass | Foreground window + active-tab URL every 2 s; list/keyword/path match; escalation ladder | yes | yes | yes |
| L1 Hosts block | Marked block in the hosts file: hosts tier, user and learned names, DoH endpoints, web proxies, SafeSearch pins | no | yes | yes |
| L2 Family resolver | System resolvers set to the chosen family filter on every physical interface, IPv4 and IPv6 | no | yes (opt-out) | yes (opt-out) |
| L3 Browser policies | DoH off, SafeSearch forced, Tor window off, for Chrome, Chromium, Edge, Brave, Firefox | no | yes | yes |
| L4 Sentinel | Root helper run by the OS scheduler: verifies and repairs L1-L3, keeps the delay, processes requests | no | no | yes |
| L5 Companion | Keeps the app (and so L0 and L6) running: forced autostart, user-level keep-alive, quit disabled while active | autostart only | autostart only | yes |
| L6 Vision | Screenshot of the front window to the configured AI for unknown/mixed/signalled windows | opt-in | opt-in | opt-in |

`ShieldLevel` is `guard | shield | locked`. Onboarding recommends `locked` and explains the
three in one screen. `Guard` exists for machines where the user has no admin rights.
A level can be raised at any time (instant); lowering it is a loosening (section 5).

Not built (reasons in research-dns-enforcement 5 and 11): loopback DNS proxy (fails closed),
`NEDNSProxyProvider` / content filter (entitlement, Developer ID), `pf` (Apple: not API),
kernel drivers, TLS interception. Port-53 confinement with WFP user-mode filters / nftables is
the named v2 hardening and has a reserved `SysOp` variant, no implementation.

### 1.2 Machine-level layout (shared by ubiqX and Leme)

| | macOS | Windows | Linux |
|---|---|---|---|
| State dir (root-owned, world-readable) | `/Library/Application Support/UbiShield/` | `%ProgramData%\UbiShield\` | `/var/lib/ubi-shield/` |
| Files in it | `state.json`, `journal.jsonl` (max 500 lines), `pack/` (verified list pack), `inbox/` (user-writable drop dir), `result.json` (last elevated run) | same | same |
| Helper binary (root-owned, not user-writable) | `/Library/PrivilegedHelperTools/ai.ubiqx.shield.helper` | `%ProgramFiles%\UbiShield\ubi-shield-helper.exe` | `/usr/local/libexec/ubi-shield/ubi-shield-helper` |
| Sentinel | `/Library/LaunchDaemons/ai.ubiqx.shield.sentinel.plist`: `RunAtLoad`, `StartInterval 60`, `WatchPaths` = hosts file, inbox, `/Library/Preferences/SystemConfiguration/preferences.plist` | Scheduled task `\UbiShield\Sentinel`, principal SYSTEM, triggers: boot, logon, repeat every 1 min, event `Microsoft-Windows-NetworkProfile/Operational` id 10000; created from XML with `schtasks /Create /XML` | `ubi-shield-sentinel.service` (oneshot) + `.timer` (`OnBootSec=15s`, `OnUnitActiveSec=60s`) + `.path` (`PathChanged=` hosts file and inbox) in `/etc/systemd/system/` |
| Sentinel command | `<helper> tick` | `<helper> tick` | `<helper> tick` |
| Elevation for install/apply | `osascript -e 'do shell script "<bundled helper> <verb> --request <file>" with administrator privileges'` | `ShellExecuteExW` verb `runas` on the bundled helper, wait on the handle, read `result.json` | `pkexec <bundled helper> <verb> --request <file>` |
| Hosts marker | `# >>> ubi-shield BEGIN v1 sha256=<hex>` ... `# <<< ubi-shield END` | same, CRLF | same |

Rules for the helper (the only code that runs as root):

- It is a separate small binary, `ubi-shield-helper` (crate `crates/ubiqx-shield-helper`),
  depending only on `crates/ubiqx-shield`. No tokio, no HTTP client, no `ubiqx-core`.
- The sentinel always runs the root-owned copy, never the copy inside the app bundle. `install`
  copies its own executable bytes to the helper path (new file, mode 0755 root, no extended
  attributes) and registers the scheduler entry.
- Closed verb set: `install`, `apply`, `tick`, `status`, `release`, `remove`, `version`. No
  verb takes a command line, a path outside the state dir, or an address that is not an IP
  literal from the provider table or validated custom entry. Hosts content is generated by the
  helper from the verified pack and validated host names; the sink is always `0.0.0.0` / `::`,
  never a caller-chosen address (SafeSearch pins come from constants or the helper's own lookup).
- Unprivileged requests go through `inbox/`: one JSON file per request, at most 4 KB, strict
  schema, accepted only when the file's owner is the `owner` recorded at install (uid on
  Unix, SID on Windows). Inbox verbs: `add_block`, `request` (a loosening, starts the delay),
  `cancel`, `raise_delay`, `net_rescue`, `set_resolver` (family providers only),
  `refresh_pack`, `raise_level`. None of them weakens protection immediately except
  `net_rescue`, which is bounded (section 5.3).
- Every run ends by rewriting `state.json` atomically (temp file, fsync, rename) and appending
  to `journal.jsonl` entries of the form `{at, kind, layer}`; no host names in the journal.

`state.json` (schema `MachineState`, version 1):

```json
{ "v": 1, "owner": "501", "installed_by": "ubiqx", "level": "locked", "active": true,
  "layers": { "hosts": true, "resolver": { "provider": "cloudflare-family" }, "policies": true,
              "safesearch": "on", "youtube": "moderate", "block_doh": true, "block_proxies": true },
  "commitment": { "delay_secs": 1800,
                  "pending": [ { "id": "r1", "kind": "turn_off", "payload": null,
                                 "requested_at": 1795000000, "matures_at": 1795001800, "observed_secs": 240 } ],
                  "rescue": { "uses": [1794990000], "until": null },
                  "pause_until": null },
  "names": { "block": ["example-user-site.com"], "learned": ["x.example"], "allow": [] },
  "backup": { "resolvers": [ { "iface": "Wi-Fi", "was": [] } ], "policies": [ { "id": "chrome", "key": "DnsOverHttpsMode", "was": null } ] },
  "pack": { "version": 202611190000, "sha256": "…" },
  "written": { "hosts_sha256": "…", "resolvers": ["1.1.1.3","1.0.0.3","2606:4700:4700::1113","2606:4700:4700::1003"] },
  "last_tick": 1795000240 }
```

### 1.3 L0: the guard pass

`ShieldService::pass_once()` runs every 2 s (`LoopConfig::shield_every`), independent of
`focus.guard_enabled`, tracking, pause and private mode. It reads the foreground window once
and feeds both the guard decision and the vision scheduler (section 3).

Decision order for a host (function `ubiqx_shield::matcher::decide`, section 2.4): lockdown
window active -> block; user block; pending-or-effective user allow; built-in never-block;
list; adult TLD; block-grade keyword; path rule (mixed platforms); otherwise `Unknown`,
`Mixed` or `Safe`.

Enforcement ladder for a blocked page (state per browser app id, pure function
`ubiqx_core::shield::ladder::next_step`):

| Step | Condition | Action |
|---|---|---|
| 1 | first pass that sees the hit; URL re-read with `BrowserUrlResolver::resolve_fresh` (new default method, bypasses the 20 s cache) still matches | `Enforcer::close_active_tab` |
| 2 | a hit on the same app within 10 s of step 1, or step 1 returned `Ok(false)` | `Enforcer::blank_active_tab` |
| 3 | a hit on the same app within 10 s of step 2, or step 2 returned `Ok(false)` | `Enforcer::quit_app`, when `ShieldConfig.may_quit_browser` (default true) |
| reset | 60 s without a hit on that app | back to step 1 |

- There is no enforcement cooldown. The counselling window is shown at most once per 45 s per
  app; enforcement between two windows is silent. One `block` event is recorded per 60 s.
- Windows only: the address bar text can be something still being typed
  (map-focus-guard 11.7). A hit is acted on when two consecutive passes see the same host and
  the same window title. macOS and Linux act on the first pass.
- Non-browser apps (vision positive in a media player, a chat app): the only lever is
  `quit_app`; it is used after a positive, never on a soft result.
- A blocked host seen in the foreground while L1/L2 are applied means DNS was bypassed (browser
  DoH, VPN, cached page). The pass records issue `doh_bypass_suspected`, sends `add_block` for
  that exact host, and puts vision in the heightened state for 15 min.
- **Lockdown** (user-initiated tightening, from the panic flow): `lockdown_until = now + 10
  min`; every supported browser in the foreground is treated as a hit at step 3.
- The pass emits `EngineEvent::PermissionRequired { permission: "automation" }` itself (once
  per browser per 30 min), because in Leme no sampler exists to do it (map-focus-guard 11.5).
- Self-exemption uses `ProductIdentity` (section 7.5), not constants.

Coverage holes that remain and how the status reports them (`ShieldIssue::GuardBlind`):

| Hole | Reported as | What still protects |
|---|---|---|
| macOS Firefox and other non-scriptable browsers (no URL) | `guard_blind: unsupported_browser` while such a browser is in front | L1-L3 (Firefox policy is reliable on macOS), L6 with `quit_app` |
| macOS Automation denied | `guard_blind: automation_denied` + the permission event | L1-L3, L6 |
| Linux X11 (domain only when the title prints it) | `guard_blind: title_only` | L1-L3, L6 |
| Linux native Wayland (no foreground, no capture) | `guard_blind: wayland`, `vision_off: wayland` | L1-L3 only; the UI says so and suggests the strict resolver |
| Background tabs, second monitor, other user accounts | documented, not reported | L1-L3 |

### 1.4 L1: hosts block

Content, in this order, capped at 30,000 names (Windows 20,000) **[verify the cap]**:

1. Probe canary: `probe.ubi-shield.invalid` (lets any process verify that the hosts file is
   honoured by resolving it and expecting `0.0.0.0`).
2. User block names and learned names: the exact host, its registrable-looking parent as
   entered, and `www.` + that (up to 2,000 names, newest first).
3. DoH endpoints not belonging to a family filter (`data/doh-endpoints.txt`: `dns.google`,
   `cloudflare-dns.com`, `chrome.cloudflare-dns.com`, `mozilla.cloudflare-dns.com`,
   `one.one.one.one`, `dns.quad9.net`, `doh.opendns.com`, `dns.nextdns.io`,
   `dns.adguard-dns.com`, `unfiltered.adguard-dns.com`, `doh.dns.sb`, `dns.mullvad.net`, ...)
   when `block_doh` (default on). Effect: a browser set by hand to another DoH provider cannot
   bootstrap it; in Chromium's automatic mode and Firefox TRR mode 2 it falls back to the
   system resolver. This is the counter for Chromium on macOS, where policies are only
   "recommended". **[verify]**
4. Web proxies and unblockers (`data/bypass.txt`, hand-curated, about 150 names) when
   `block_proxies` (default on).
5. SafeSearch pins when `safesearch = on`: `www.google.com`, `www.google.com.br` and the Google
   country domains in `data/google-domains.txt` -> the address the helper resolves for
   `forcesafesearch.google.com` (fallback `216.239.38.120`); the five YouTube names ->
   `restrictmoderate.youtube.com` (fallback `216.239.38.119`) or `restrict.youtube.com`
   (`216.239.38.120`) per `youtube = moderate | strict | off`, default `moderate`. Both A and
   AAAA answers are written. No Bing line (Defender `HostsFileHijack` risk); Bing is forced by
   policy in Edge and, in other browsers, only by a resolver that enforces SafeSearch.
6. The hosts tier from the verified pack (most popular listed host names first) until the cap.

Format: `0.0.0.0 name` and `:: name`; one name per line on macOS and Linux, nine names per line
on Windows; the file's existing line-ending convention is kept; everything outside the marker
is preserved byte for byte.

| | macOS | Windows | Linux |
|---|---|---|---|
| Write | temp sibling + fsync + rename, root:wheel 0644 | temp + `MoveFileExW` replace | temp + rename, root:root 0644 |
| Flush | `dscacheutil -flushcache`; `killall -HUP mDNSResponder` | `ipconfig /flushdns` | `resolvectl flush-caches` when resolved is active, else `nscd -i hosts` when present |
| Verify (unprivileged) | read file, recompute block hash, compare with the hash in the marker and in `state.json`; resolve the canary | same | same |
| Re-apply | sentinel tick (`Locked`); prompt (`Shield`) | same | same |
| Undo | delete exactly the marked block | same | same |

Failure modes: file locked or restored by security software -> helper reports
`hosts: write_failed`, status `needs_attention: hosts_missing`; block present but hash differs
-> `hosts_stale` (repaired at the next tick); canary does not resolve to `0.0.0.0` while the
block is intact -> `hosts_not_honoured` (shown as "reduced").

### 1.5 L2: family resolver

Provider table `crates/ubiqx-shield/data/resolvers.json` (data, updatable with the pack):
`cloudflare-family` (default; attribution "DNS by Cloudflare" in the UI), `cleanbrowsing-family`
(labelled "strict"; shipped disabled behind `requires_clearance: true` until the owner has
written clearance, risk 3), `adguard-family`, `opendns-familyshield`, `dns4eu-child`, and
`none` (hosts-only mode). Custom addresses are accepted only at enable time (tightening) and
only as IP literals.

| | macOS | Windows | Linux |
|---|---|---|---|
| Apply | for every enabled service from `networksetup -listallnetworkservices`: `networksetup -setdnsservers "<svc>" <v4...> <v6...>` | for every adapter from `GetAdaptersAddresses` with `OperStatus` up and `IfType` in {6, 71, 243, 244}: `Set-DnsClientServerAddress -InterfaceIndex N -ServerAddresses (...)`; on Windows 11 also `Add-DnsClientDohServerAddress ... -AllowFallbackToUdp $True -AutoUpgrade $True` per address | systemd-resolved active: write `/etc/systemd/resolved.conf.d/90-ubi-shield.conf` (`DNS=<addr>#<dot-host> ...`, `Domains=~.`, `DNSOverTLS=opportunistic`), `systemctl restart systemd-resolved`; else NetworkManager: `nmcli connection modify` on every active connection (`ipv4.dns`, `ipv4.ignore-auto-dns yes`, IPv6 twins) + reactivation; else `unsupported` |
| Tunnel / VPN interfaces | untouched | untouched (other `IfType`) | untouched (links with their own `~.` route win) |
| Backup | previous servers per service ("empty" = DHCP) in `state.json.backup` | previous per-adapter static servers or "dhcp" | drop-in: nothing to back up; nmcli: previous values per connection uuid |
| Verify (unprivileged) | `networksetup -getdnsservers` per service | `GetAdaptersAddresses` DNS list per adapter | drop-in content hash + `systemctl is-active systemd-resolved`; nmcli values |
| Undo | `-setdnsservers "<svc>" empty` or the backed-up list, only where the current value still equals what was written | `-ResetServerAddresses` or the backed-up list, same rule; `Remove-DnsClientDohServerAddress` | delete the drop-in and restart resolved; restore nmcli values |

`DNSOverTLS=opportunistic` and UDP fallback are chosen so that a network that blocks port 853
or DoH degrades to plain filtered DNS instead of to no DNS.

Failure handling in the tick:

- New interface or service with other resolvers -> set it (benign drift, journal `resolver_set`).
- **Resolver unreachable**: the helper sends one UDP query for `example.com` to each provider
  address (`probe.rs`, 2 s timeout). Five consecutive ticks with a link up and no answer from
  any address -> automatic rescue: L2 reverted for 15 min, journal `resolver_unreachable`,
  status `rescue`. It retries every 15 min and does not count against the user's rescue quota.
  This covers provider outages and captive portals the user did not handle by hand.
- VPN tunnel up -> `reduced: vpn_active`; vision goes to the heightened state. The tool does
  not fight VPNs (research-dns-enforcement 6.4).
- iCloud Private Relay detected (macOS, `networksetup`/`scutil` proxy facts **[verify]**) ->
  `reduced: private_relay` with the System Settings path.

Disclosure: enabling L2 names the operator and says "todas as consultas de DNS deste
computador passam a ir para <operador>". `none` is one tap away on the same screen.

### 1.6 L3: browser policies

Values: Chromium family `DnsOverHttpsMode = "off"`, `ForceGoogleSafeSearch = true`,
`ForceYouTubeRestrict = 1` (2 when `youtube = strict`); Edge also `ForceBingSafeSearch = 2`;
Chrome has no Bing policy, so Bing in Chrome relies on L2; Brave
also `TorDisabled = true`; Firefox `DNSOverHTTPS = { Enabled: false, Locked: true }`.
Incognito is not disabled: L0 reads private windows and L6 captures them.

| Browser | macOS | Windows (HKLM, named values only) | Linux (our own file `ubi-shield.json`) |
|---|---|---|---|
| Chrome | `/Library/Preferences/com.google.Chrome.plist` (recommended level only; status `policy_advisory_only`) | `SOFTWARE\Policies\Google\Chrome` | `/etc/opt/chrome/policies/managed/` |
| Chromium | `org.chromium.Chromium` plist, same caveat | `SOFTWARE\Policies\Chromium` | `/etc/chromium/policies/managed/` and `/etc/chromium-browser/policies/managed/` |
| Edge | `com.microsoft.Edge` plist, same caveat | `SOFTWARE\Policies\Microsoft\Edge` | `/etc/opt/edge/policies/managed/` |
| Brave | `com.brave.Browser` plist, same caveat | `SOFTWARE\Policies\BraveSoftware\Brave` | `/etc/brave/policies/managed/` |
| Firefox | `defaults write /Library/Preferences/org.mozilla.firefox EnterprisePoliciesEnabled -bool TRUE`, `DNSOverHTTPS__Enabled -bool FALSE`, `DNSOverHTTPS__Locked -bool TRUE` | `distribution\policies.json` next to `firefox.exe` when no such file exists; otherwise `SOFTWARE\Policies\Mozilla\Firefox` values **[verify key names]** | `/etc/firefox/policies/policies.json` when absent; when present and not ours: `policy_conflict` |

Rules: only browsers that are installed get a policy; a value that already exists with a
different content and was not written by us is left alone and reported as `policy_conflict`
(a real organisation manages this machine); prior existence of each value is recorded in
`backup`; undo removes only values that still equal what we wrote. Before the first apply the
UI shows, in plain words, that browsers will display "Gerenciado pela sua organização" and
why. Snap/Flatpak browsers on Linux are reported as `policy_missing` **[verify paths]**.

On macOS, Chromium policies are advisory. The honest state is `reduced:
policy_advisory_only`; the effective counters there are item 3 of the hosts block, same-provider
DoH auto-upgrade (which keeps a family resolver), and L6. A user-approved configuration profile
is the v1.1 candidate once test 1 of section 9.4 passes.

### 1.7 L4: the sentinel tick

`ubiqx_shield::tick::decide(state, facts, inbox, now) -> TickOutcome { state, ops, journal }`
is a pure function; the helper's `tick` verb gathers `Facts` (hosts file bytes, resolver values
per interface, policy values, link state, probe result), calls it, executes `ops`, writes
state and journal. Order inside `decide`:

1. Advance clocks: `observed_secs += min(now - last_tick, 120)` for every pending request
   (only positive deltas).
2. Process inbox requests (validated; invalid files are deleted and journalled as `inbox_rejected`).
3. Mature pending requests (section 5.2) and apply their effect to `state`.
4. If `active` and not paused: compare `facts` with the desired state; emit `SysOp`s to repair
   L1, L2 (unless a rescue window is open), L3; journal `repaired:<layer>` when the cause was
   not benign.
5. If a `turn_off` matured: emit the full removal plan, then the sentinel removes its own
   scheduler entry and helper binary, leaving `state.json` with `active: false` for 30 days.
6. Re-arm after `pause_until` / rescue `until`.

The app reads `state.json` and `journal.jsonl` every 60 s (`ShieldService::maintain_once`) and
on window focus. A `repaired:*` entry becomes a calm notice ("A proteção foi alterada às 22:14
e eu recoloquei. Tudo certo?") and counts as an override attempt for the help heuristic
(section 4.8). It never extends a delay or adds a penalty.

At level `Shield` there is no sentinel: `maintain_once` computes the same `decide` with
unprivileged facts; a non-empty repair plan becomes status `needs_attention` and a counselling
session of kind `restore` with one button that runs `apply` through the OS prompt. Inbox
requests (`add_block`, `refresh_pack`) wait for that next elevated run; the status screen shows
"N alterações aguardando aplicar" with the same button.

Survival: reboot (scheduler trigger at boot); app quit, crash or deletion (L1-L4 do not depend
on the app); app update (helper version is checked by `maintain_once`; a newer bundled helper
is installed with one prompt, an older state schema is migrated by the helper); OS update that
resets the hosts file or DNS (next tick). macOS: the user can switch the daemon off under
Login Items -> `needs_attention: sentinel_disabled`, effective level shown as `Shield`.

### 1.8 L5: the companion

While protection is active: autostart is forced on (the settings toggle is disabled and says
why); the tray "Sair" item hides the window and shows "O Escudo continua ativo. Para
encerrar, use Pausar ou Desligar (leva {delay})"; a user-level keep-alive restarts the app if
it is killed: macOS `~/Library/LaunchAgents/ai.ubiqx.shield.companion.plist` (`KeepAlive`),
Linux `~/.config/systemd/user/ubi-shield-companion.service` (`Restart=always`), Windows a
per-user scheduled task every 5 min running the app with `--ensure-running` **[verify]**.
`tauri-plugin-single-instance` is added to both shells so a second launch only focuses the
first (it also fixes the two-engines-on-one-database bug in map-app-shell). The keep-alive is
installed only at level `Locked`, is listed on the status screen, and is removed on release.
It is user-level and removable by the user; its absence is reported, not fought.

### 1.9 User-visible state

`ShieldStatus { level, effective_level, health, issues: Vec<ShieldIssue>, layers:
{guard, hosts, resolver, policies, sentinel, companion, vision}: LayerState, pending:
Vec<PendingView>, rescue_until, pause_until, lockdown_until, managed_by: ProductId }`.

`ShieldHealth`: `off`, `protected`, `reduced` (works, with a named gap), `needs_attention`
(a layer the user chose is not applied), `releasing` (a `turn_off`/`pause` is counting down),
`paused`, `rescue`. `LayerState`: `off | ok | reduced | broken | unsupported`.

`ShieldIssue` (each has PT-BR/EN copy and at most one action button): `hosts_missing`,
`hosts_stale`, `hosts_not_honoured`, `resolver_drift`, `resolver_unsupported`,
`resolver_unreachable`, `policy_missing{browser}`, `policy_advisory_only{browser}`,
`policy_conflict{browser}`, `sentinel_missing`, `sentinel_disabled`, `helper_outdated`,
`companion_missing`, `vpn_active`, `private_relay`, `doh_bypass_suspected`,
`guard_blind{reason}`, `vision_off{reason}`, `managed_by_other_product`.

The tray icon carries a small state dot (ok / attention); no domain, no word about the
subject, appears in the tray, in notifications or in window titles.

---

## 2. The blocklist

### 2.1 Sources and tiers (from `research-blocklists.md`, adopted as measured)

| Tier | Sources | Entries | Size | Licence | Where |
|---|---|---|---|---|---|
| Core | StevenBlack `porn-only` + ShadowWhisperer `Adult` + 4skinSkywalker entries with a block-grade keyword + `extra-ptbr.txt`, minus `allow.txt` | 239,449 | 1.92 MB hash table | MIT / Unlicense (two 300-host inputs CC BY 4.0) | `include_bytes!` in `crates/ubiqx-shield/data/blocklist-core.bin` |
| Extended | Core + Bon-Appetit/porn-domains + corroborated 4skin entries | 523,112 | 4.18 MB | CC BY-SA 4.0, data file only | signed pack downloaded to the user-level shared dir; never compiled in |
| Hosts tier | CrUX top-1M host names (rank <= 500k) covered by the tier, plus apex and `www.` | 24,541 (Core) / about 32,000 (Extended) | 164 KB gzip | follows its source | Core-derived copy embedded (`hosts-hot.txt.gz`); Extended-derived copy in the pack |
| Safe set | CrUX top origins reviewed by a person (target 3,000 registrable domains) | 3,000 | 24 KB hash table | CC BY 4.0 (CrUX) | embedded `safe.bin`; only purpose: skip vision |
| Never-block | OS vendors, update and identity hosts, payment and bank suffixes, `gov`, `edu`, `mil`, `ac.*`, sexual-health and education sites (`scarleteen.com`, `plannedparenthood.org`) | about 400 rules | text | ours | embedded `never-block.txt`; also the helper's refusal list for `add_block` |
| Mixed | Platforms that cannot be split by domain (`reddit.com`, `x.com`, `tumblr.com`, `imgur.com`, `pinterest.com`, `vk.com`, `web.telegram.org`, `discord.com`, `twitch.tv`, `deviantart.com`, `pixiv.net`, search engines without enforceable SafeSearch, ...) | about 60 | text | ours | embedded `mixed.txt`, grouped (`social`, `image`, `search`, `chat`) |

GPL-3.0 lists (HaGeZi, oisd) are not shipped. "Add a list by URL" is deferred to v1.1; the
`ShieldRepo` list model already has a `custom` source so it needs no migration.

Build: `scripts/shield/build_blocklist.py` (standard library only) with inputs in
`data/shield/` (`sources.lock.json`, `allow.txt`, `extra-ptbr.txt`, `mixed.txt`,
`never-block.txt`, `safe-reviewed.txt`, `doh-endpoints.txt`, `bypass.txt`,
`google-domains.txt`, `canary-block.txt`, `canary-allow.txt`, pinned PSL and CrUX copies).
Steps and gates are exactly sections 12 of the blocklist report (drop public suffixes, never
collapse to the registrable domain, drop `*.tumblr.com` / `*.blogspot.*`, allowlist, canaries,
size delta gate, top-10k review print, byte-identical rerun). The script also emits
`THIRD-PARTY-NOTICES-shield.txt` and the pack's `NOTICE.txt` from the pinned upstream licence
files (texts in section 13 of that report).

### 2.2 Embedded format

`UBXBL1` table as specified in research-blocklists section 8: header (`magic "UBXBL1\0\0"`,
`format u16`, `flags u16`, `count u32`, `list_version u64`) then `count` sorted unique
big-endian `u64` keys; key = first 8 bytes of `SHA-256("ubi-shield-blocklist-v1\0" ||
host)`. `Blocklist::from_static(&'static [u8])` and `Blocklist::from_vec(Vec<u8>)` validate
magic, count and ordering once; `contains(&self, host: &str) -> bool` is a binary search. The
shipped table cannot be enumerated; CI keeps the plain text as a build artefact.

### 2.3 Normalisation

`ubiqx_shield::host::normalize(raw) -> Option<Host>`: lower-case, strip trailing dot, port,
user-info and brackets; reject IP literals, single labels and anything outside
`[a-z0-9.-]` after IDNA-to-ASCII (`idna` crate, already in `Cargo.lock`). `Host::walk()`
yields the host and each parent down to two labels. Callers pass the host of the URL they
already have (`normalize::domain_of` in core keeps stripping `www.`).

### 2.4 Matching semantics

`matcher::decide(&Lists, &Host, path: Option<&str>, now) -> Verdict`, first match wins while
walking from the most specific name to the least specific:

| Order | Source | Verdict |
|---|---|---|
| 1 | lockdown window (handled by the caller) | `Block(Lockdown)` |
| 2 | user block list (exact or parent) | `Block(User)` |
| 3 | user allow list, **effective entries only** (matured, section 5) | `Allow(User)` |
| 4 | "not it" exemptions (24 h, heuristic and learned sources only, section 4.4) | skips 7-9 |
| 5 | never-block | `Allow(BuiltIn)` |
| 6 | Extended table if a verified pack is loaded, else Core | `Block(List)` |
| 7 | learned names | `Block(Learned)` |
| 8 | adult TLD: `.xxx`, `.porn`, `.sex`, `.sexy`, `.adult` | `Block(Tld)` |
| 9 | block-grade keyword or brand stem in the name left of the last two labels' suffix | `Block(Keyword)` |
| 10 | path rule on a mixed host (`reddit.com/r/<name>` with a block-grade keyword, `/over18`, a `/nsfw` segment, a search query containing a block-grade word; learned path prefixes) | `Block(Path)` |
| 11 | strict-mode mixed hosts the user ticked | `Block(Strict)` |
| 12 | mixed list | `Mixed` |
| 13 | safe set | `Safe` |
| 14 | weak token (`sexy`, `adult`, `escort`, `nude`, `naked`, `milf`, `bdsm`, `webcam`, `.cam`, `.webcam`, `.tube`, PT-BR `acompanhantes`, `amador`) | `Unknown { weak: true }` |
| 15 | anything else | `Unknown { weak: false }` |

Longest match wins between block and allow lists; at equal length a user block beats a user
allow. A list update can never remove a user block.

### 2.5 Keyword and TLD heuristics

`heuristics.rs`, tables taken from research-blocklists 9.1-9.5:

- Block-grade substrings: `porn`, `xxx`, `hentai`, `fuck`, `blowjob`, `gangbang`, `cumshot`,
  `handjob`, `bukkake`, `deepthroat`, `threesome`, `creampie`, `cuckold`, `femdom`, `shemale`,
  `camgirl`, `upskirt`, `masturbat`, `incest`, `bokep`, `phimsex`, `erotic`, `erotik`,
  `escorts`, `nudes`, `putas`; PT-BR `putaria`, `buceta`, `bucetinha`, `siririca`,
  `contoseroticos`, `brasileirinhas`, `pornozao`, `safadas`, `novinhas`, `gostosas`,
  `peladas`, `transando`, `gozando`.
- Brand stems after look-alike folding (`0->o`, `1->i`, `3->e`, `4->a`, `5->s`, `7->t`,
  `vv->w`, edge digits stripped): the 31 stems of section 9.1.
- Never used as substrings: `sex`, `anal`, `cum`, `ass`, `cock`, `dick`, `tits`, `cam`,
  `jav`, `gay`, `teen`, `pau`, `rola`, `pica`, `puta`, `bunda`, `pelada`, `foda`.
- `xn--` labels are decoded and accent-folded before matching.
- Keywords are never applied to never-block names or to the safe set.
- The same block-grade table is applied to **window titles** as a vision trigger only
  (section 3.2), never as a block.

Unit tests carry the trap corpus from the report (`essex.gov.uk`, `planalto.gov.br`,
`motorola.com.br`, `drogariasaopaulo.com.br`, `emilfrey.de`, `canaltech.com.br`,
`sussexexpress.co.uk`, ...) as must-not-block cases.

### 2.6 User additions, allow-list, learned names

Table `shield_domains` (section 6.2) is the app's copy; at levels `Shield`/`Locked` the
machine state's `names` is authoritative and the table is reconciled from it every
`maintain_once` (union for block and learned, machine state wins for allow).

- **User block**: instant. Stored, sent as `add_block`, lands in the hosts block at the next
  tick (`Locked`) or the next elevated apply (`Shield`).
- **User allow**: a loosening. Created as pending; effective after the delay. Never-block and
  list entries can be allowed this way; a user block must be removed first (also delayed).
- **Learned**, three sources, all automatic, all visible on the list screen with their source:
  1. `list_hit`: a host the guard blocked from the big table that is not in the hosts block.
  2. `keyword`: a host blocked by TLD or keyword.
  3. `vision`: the host of a confirmed positive (one `explicit/high`, or two positives within
     10 min). Never learned: mixed hosts, safe-set hosts, never-block names, hosts with two
     labels that are public-suffix-like (`*.blogspot.com` style parents are kept at full depth).
     For a mixed host with a known URL, the first two path segments are learned as a path
     rule in `shield_paths` (guard-only).
  Cap 2,000 learned names, least recently hit evicted. `ShieldConfig.learn_to_hosts`
  (default on) controls promotion to the hosts block; turning it off is a loosening.

Privacy note shown at opt-in: names in the hosts block are readable by other accounts on
this computer.

### 2.7 Update path

Fixed release tag `shield-lists` with `manifest.json`, `manifest.json.sig` (Ed25519, new key
pair, public key compiled into `ubiqx-shield`; private key in the repository secret
`SHIELD_LIST_SIGNING_KEY`), `blocklist-core.bin`, `blocklist-extended.bin`,
`hosts-hot.txt.gz`, `lists.json` (resolvers, DoH endpoints, bypass, mixed, safe), the plain
text of the Extended tier, `LICENSE-CC-BY-SA-4.0.txt`, `NOTICE.txt`. Workflow
`.github/workflows/shield-lists.yml` (weekly + manual) runs the script, the gates and the
signing step, and uploads with `--clobber`.

Client (`ShieldService::update_lists`, weekly with jitter and on enable): fetch through the
existing `UpdateFeedSource`-style port `ListSource` (HTTP in production); verify signature,
`format`, strictly greater `list_version`, per-file SHA-256 and length, table structure, the
embedded canaries; write to `<shared user dir>/packs/<version>/` and rename. On any failure
the previous pack stays; the embedded Core is the floor. Then `refresh_pack { path }` goes to
the inbox; the helper verifies the same signature itself before copying the pack into the
state dir and rewriting the hosts block. If the repository is private the files are served by
`services/ubi-api` at `/v1/shield/lists/*` (static passthrough, no auth, no logging).

---

## 3. Vision detection

### 3.1 Preconditions (all must hold; otherwise layer state `vision: off{reason}`)

- `ShieldConfig.vision_enabled` and `shield_profile.vision_consent_at` set. The consent screen
  names the provider and says: one reduced image of the front window, sent when the site or
  app is unknown to the lists, including private-browsing windows; nothing is stored.
- A detector is wired and the selected provider has a key (or a valid managed licence for
  "IA do Ubi", where the consent text adds that images pass through the operator's proxy).
- `!settings.local_only`. This switch is honoured; it is the user's promise that nothing
  leaves the machine. Tracking pause, private mode, incognito redaction, `vision_policy`,
  `vision_denied_apps` and `blocked_apps` are **not** honoured: they belong to the tracker
  and each would be an off switch.
- Screen capture is possible (macOS Screen Recording granted; not Wayland) and the window has
  a window id. A missing window id means no capture; the full-display fallback of the tracker
  (map-vision-ai 15.1) is never used.
- The app in front is not in `never_capture` (built in: password managers, the product's own
  windows, system settings; user additions are loosenings).
- Shield vision budget not spent, hourly and daily caps not reached, `ShieldAiHealth` not in
  backoff.

### 3.2 When a check runs

`ubiqx_core::shield::schedule::next_check(&WatchState, &Observation, now) -> Option<Check>`
(pure). A window is a **candidate** when any of these holds:

| Trigger | Dwell before first check | Repeat while it stays in front and verdicts are `none` |
|---|---|---|
| Browser, verdict `Unknown { weak: false }` | 8 s | 90 s, 180 s, then every 300 s |
| Browser, `Unknown { weak: true }`, `Mixed`, or private-browsing title marker | 5 s | 45 s, 90 s, then every 180 s |
| Any app whose window title contains a block-grade keyword | 3 s | 45 s, 90 s, then every 180 s |
| Media and chat apps (`media_apps` table: VLC, mpv, IINA, QuickTime, Windows Media Player, Telegram, Discord, ...) | 20 s | every 300 s |
| Unsupported browser (no URL available) | 8 s | 60 s, 120 s, then every 300 s |
| **Heightened state** (15 min after any block, bypass suspicion or VPN detection; and during the user's `risky_hours`) applies to every browser window that is not `Safe` | 5 s | 30 s, 60 s, then every 120 s |

`Safe` hosts, blocked hosts and every other app are never captured. A change of host or of
app resets the candidate. A check that would exceed a cap is skipped and counted.

Local prefilter: `ImageDigest::luma_signature(&EncodedImage) -> u64` (64-bit difference hash,
implemented once in `ubiqx-platform::image_util`). If the signature is within Hamming
distance 4 of the last image sent for the same window and that verdict was `none` less than
10 min ago, the image is dropped unsent.

Caps and cost: `max_checks_per_hour` 30 (range 10-60), 200 per day, monthly budget
`vision_budget_usd` 3.00, accounted on usage rows of kind `shield_vision` only. The
productivity budget stops counting that kind (`UsageRepo::totals_by_kind`, new method), so a
shield month cannot pause classification and a classification month cannot switch the shield's
eyes off. Estimate (map-vision-ai section 9, not measured): about US$ 0.0009 per check at
768 px on `claude-haiku-4-5`, so the budget buys about 3,300 checks. Measure before fixing the
defaults.

### 3.3 What is captured and sent

`ScreenCapturer::capture(CaptureTarget::Window(id), 768)`: JPEG quality 60, in memory. The
bytes go into one `LlmRequest` and are dropped when the call returns. No file, no
`screenshots` row, no base64 in logs: the `trace!` lines at `client.rs:745` and
`openai.rs:689` are changed to log a copy of the body with image data replaced by
`<image N bytes>` (for every request, which also fixes map-vision-ai 15.11).
No window title, URL, app name or user profile is sent: the request is the image and a fixed
sentence.

### 3.4 Prompt and schema

`crates/ubiqx-ai/src/explicit.rs`, `prompts::explicit_system_prompt()`:

```
You are a content-safety classifier inside a self-control app that an adult installed on
their own computer to stay away from pornography. You receive one screenshot of the window
that is in front. Decide whether it shows pornography.

explicit   - real, drawn, animated or generated depiction of sexual acts, exposed genitals,
             or nudity presented for sexual arousal; a listing page of an adult site full of
             such thumbnails counts.
suggestive - sexualised content without the above: lingerie or swimwear posing, erotic
             stories as text, an adult site's landing or age-gate page, blurred thumbnails.
none       - everything else, including medical or educational material, classical art,
             breastfeeding, sport, news and ordinary photos of people.
unsure     - the image is too small, dark or ambiguous to tell.

Do not describe the image. Do not transcribe text. Do not identify anyone.
Answer with the JSON object only.
```

User content: `[Image, Text("Classify this screenshot.")]`. Schema (closed, all required):
`{ "verdict": "explicit"|"suggestive"|"none"|"unsure", "confidence": "high"|"medium"|"low",
"kind": "photo_video"|"drawing"|"text"|"none" }`. No free-text field exists, so the model
cannot return a description of what the user was looking at.

Request: `model = settings.models.vision` (no new `AiModels` field), `max_tokens = 60`
(one retry at 120 on truncation), `disable_thinking = thinking_disabled_for(&model)`,
`usage_kind = AiUsageKind::ShieldVision` (new variant, `as_str = "shield_vision"`; added to
the `matches!` in `openai.rs::reasoning_effort_for` so `gpt-5*` runs at minimal effort),
`timeout = Some(10 s)`, wrapped in `tokio::time::timeout(12 s)` because retries are
per client (map-vision-ai 1.8).

Provider specifics: none in the wire format (the `RoutingLlmClient` covers Anthropic, OpenAI,
xAI and the Ubi proxy). `Err(CoreError::AiRefused)` is caught in `LlmExplicitDetector::check`
and returned as `ExplicitCheck { verdict: Unsure, confidence: Low, refused: true }`. Proxy:
`services/ubi-api` gets an explicit `DefaultBodyLimit` of 4 MiB on `/v1/messages`, an image
passthrough test, and a separate per-subscriber ledger for requests carrying
`x-ubiqx-kind: shield_vision` (cap `UBI_SHIELD_BUDGET_USD`, default 3).

Port (`ubiqx-core/src/ports.rs`):

```rust
pub enum ExplicitVerdict { Explicit, Suggestive, None, Unsure }
pub enum ExplicitConfidence { High, Medium, Low }
pub struct ExplicitCheck { pub verdict: ExplicitVerdict, pub confidence: ExplicitConfidence, pub refused: bool }
#[async_trait]
pub trait ExplicitDetector: Send + Sync {
    async fn check(&self, image: &EncodedImage, model: &str) -> CoreResult<ExplicitCheck>;
}
```

### 3.5 Thresholds and outcomes

`ubiqx_core::shield::schedule::judge(&WatchState, &ExplicitCheck, ctx) -> Outcome`:

| Result | Outcome |
|---|---|
| `explicit` + `high` or `medium` | **Positive**: enforce (ladder for browsers, `quit_app` otherwise), counselling session `vision_positive`, learn (section 2.6), heightened state |
| `explicit` + `low`, or `suggestive` + `high` | **Suspect**: no action; re-check after 15 s. Two suspects within 60 s on the same host/app -> counselling session `vision_soft` ("Posso estar enganado... Quer que eu feche?") with "Fechar" and "Não era isso". In strict mode `suggestive` + `high` is a Positive |
| Refusal | **Suspect**. Becomes Positive when a second refusal arrives within 60 s, or when the window is `weak`, `Mixed`, private-browsing or in the heightened state |
| `suggestive` + `medium`/`low`, `unsure` | one re-check after 30 s, then normal backoff |
| `none` | normal backoff |
| Timeout, network error, 5xx | `ShieldAiHealth` backoff 1, 2, 4, 8, 15 min; no change to the shared `AiHealth`; nothing shown unless it lasts 30 min (then `vision_off: offline`) |
| 401/403, `AiRejected` | `vision_off: no_key` / `rejected` until settings change; one calm notice |
| `RateLimited` | backoff `max(retry_after, 60 s)` |
| Budget or cap reached | `vision_off: budget_spent` / `rate_capped`, shown on the status screen |

Without vision (no AI, offline, `local_only`, Wayland, no permission, budget spent) the product
still runs L0-L5. Two fallbacks replace it: a block-grade keyword in a window title opens the
`vision_soft` session directly (no enforcement), and the heightened state offers the urge-surf
session after the third block in 15 min. The status screen states plainly "Verificação por IA:
desligada (<motivo>)".

### 3.6 What is stored

Per check: nothing except counters in `shield_daily` (`vision_checks`, `vision_positives`,
`vision_cost_usd`) and the ordinary `ai_usage` row (tokens, cost, kind). Per positive or soft
session: one `shield_events` row without host. The learned host goes to `shield_domains`
because it has to be written to the hosts block; the user sees it there and can ask to
remove it. No image, no description, no title, no URL is persisted or logged. The "Dados
enviados à IA" dialog gains one line per day: "N verificações de tela do Escudo, imagens não
guardadas".

Tracker side: `SegmenterConfig` gains `sensitive: Option<Arc<dyn HostPredicate>>`; when the
shield is on, a sample whose host is `Block(*)` is redacted to `[privado]` in
`Segmenter::is_blocked`, so a blocked attempt never reaches the timeline, the classifier, the
reports or the export (map-focus-guard 10.13, 11.20).

---

## 4. Counselling

Principles (research-counselling section 6): the user decided, UBI keeps the agreement; no
shame and no inflated praise; tentative about what vision saw; one or two sentences and one
action; no neuroscience, percentages or diagnoses; discreet by default; always one step from a
human. All copy comes from the reviewed 40-message bank (section 7 of that report), stored
with stable keys in `ubiqx_core::shield::copy` (PT-BR and EN). No model-generated text is
shown in v1. A test fails when any string contains a word from the "never say" list
(`viciado`, `recaída`, `limpo`, `dopamina`, `reboot`, `força de vontade`, ...).

### 4.1 The counselling window

New window, label `counsel`, route `#/counsel?session=<id>` outside the `Gate`, capability
file `capabilities/counsel.json`, 480x300 for a single message and 560x520 inside a flow,
always on top, on all workspaces, takes focus (it replaces what was closed), not in the
taskbar. Title "UBI". The discreet first screen closes itself after 20 s without input; a
flow never closes by itself. Fallback when the window cannot be shown: OS notification with
title "UBI" and body `block.discreet` (never a domain).

Port: `InterventionPresenter` gains `fn show_counsel(&self, s: &CounselSession) ->
CoreResult<bool> { Ok(false) }` (default method, so `LogInterventionPresenter` and the mock
compile unchanged). `CounselSession { id, kind, discreet, message_key, values_sentence:
Option<String>, plan: Option<IfThenPlan>, can_report_wrong: bool, created_at }` carries no
host. "Ver por quê" inside the window asks the engine for `reason` (`sua lista`, `lista`,
`palavra no endereço`, `IA`) and, only then, the matched name.

| Session kind | Trigger | First screen (discreet on / off) | Actions |
|---|---|---|---|
| `block` | L0 block | `block.discreet` / rotating `block.default`, `block.breathe`, `block.choice`, `block.next` | "Ok", "Preciso de um minuto" (surf), help link |
| `block_repeat` | third block in 15 min | `block.repeat` | surf (primary), "Ok", help |
| `vision_positive` | section 3.5 Positive | `block.discreet` / `vision.closed` then `vision.added` | "Ok", "Não era isso", surf, help |
| `vision_soft` | section 3.5 Suspect x2, or title keyword without AI | `vision.soft` | "Fechar", "Não era isso" |
| `restore` | drift at level `Shield`, or a repaired tamper at `Locked` | calm notice | "Recolocar" (prompt) / "Ok", "Preciso de um minuto" |
| `release_wait` | a loosening was requested | countdown + `panic.reason` with the values sentence | "Cancelar pedido" (primary), surf, "Continuar esperando" |
| `panic` | tray item, button on the page | `panic.open`, `panic.options` | surf, "Sair da tela por 10 min" (lockdown), "Falar com alguém" (crisis card) |
| `lapse` | "Aconteceu" button | `lapse.first`, `lapse.kind` | optional trigger chips (`lapse.learn`), optional plan (`lapse.plan`), `lapse.resume` |
| `help` | thresholds of 4.8, or any help link | `help.pattern`, `help.how`, `help.limits` | resource list |

### 4.2 Urge surf

Three minutes, one step per screen, keys `surf.start` -> `surf.body` -> `surf.breath` (4 in,
6 out, animated by UBI) -> `surf.wave` -> `surf.thought` -> `surf.rate` (0-10) -> `surf.end`.
Reachable from every session, the tray and the page. Stored: one `surf_done` event with the
rating. Backing: mindfulness-based relapse prevention and ACT (Crosby and Twohig 2016; Holas
2020), with the honest framing that it changes what the person does with the urge, not its
size.

### 4.3 Check-ins and plans

- Daily one-tap mood (five levels) on the page, optional evening reminder through the nudge
  channel (at most one a day, text `checkin.daily`, which names nothing). Optional HALT chips
  (`checkin.halt`), framed as a prompt, never as an explanation.
- One values sentence (`checkin.values`) and up to three if-then plans (`checkin.ifthen`),
  shown back in `release_wait`, `panic` and (when discreet is off) `block`. Backing: ACT
  values work; implementation intentions (Gollwitzer and Sheeran 2006, untested for this use).
- `checkin.evening` appears only when the user's own `shield_daily` rows show most blocks in
  the same three-hour band on at least 5 of the last 14 days.

### 4.4 "Não era isso"

One tap, no argument. For heuristic and vision sources (keyword, TLD, learned-by-vision,
soft sessions) the host gets a 24 h exemption from rules 7-9 of section 2.4, at most three
times per 24 h; further reports that day become ordinary allow requests (delayed). The
exemption never stops vision from sampling that host; two more `explicit/high` results
re-learn it and later reports for that host are delayed. For list and user-list hits the
button creates an allow request (delayed) and says so. Every report is a `wrong_report`
event; the weekly count is shown to the user as a quality signal.

### 4.5 Lapse handling

A lapse is only what the user reports with "Aconteceu" (page, tray, end of a
`vision_positive` session). Vision positives are never labelled lapses. The flow is
`lapse.first` -> `lapse.kind` -> optional `lapse.learn` chips (cansaço, tédio, estresse,
solidão, outro) -> optional `lapse.plan` -> `lapse.resume`. Nothing is reset on screen.
Backing: abstinence violation effect (Marlatt and Gordon), self-compassion findings (Reid
2014).

### 4.6 Progress and streak

Default view: "dias do seu jeito nos últimos 30" (days without a reported lapse). The current
streak is an opt-in preference (`Settings.shield.show_streak`, default false), with quiet
milestones `streak.day1`, `streak.week`, `streak.month` and `streak.after_reset` after a
lapse. No goal about masturbation exists anywhere; the user's goal text is their own. Backing:
Silverman and Barasch 2023 (broken streaks lower engagement).

### 4.7 Panic button

Tray item "Preciso de um minuto" (ubiqX and Leme) and a button on the page. Opens `panic`.
"Sair da tela por 10 min" sets the lockdown window (section 1.3) and shows `panic.away` with
a countdown. "Falar com alguém" opens the crisis card.

### 4.8 Professional help

`ubiqx_core::shield::help::should_suggest(&Counts7d, last_shown) -> bool` with constants
`LAPSES_7D = 5`, `OVERRIDES_7D = 10` (release requests, allow requests beyond the free
"not it" reports, user-triggered rescues, repaired tampers), `VERY_BAD_CHECKINS_7D = 2`,
at most once per 7 days; plus the onboarding answers (lasting six months or more, several
attempts to cut down, harm to work, sleep, money or a relationship) which show the card once.
These are product heuristics, stated as such in the code comment and never shown as numbers.
The crisis card (`panic.crisis`) is reachable in one step from every counselling screen.
Resources come from `crates/ubiqx-core/data/shield/help-resources.json` (CVV 188 and chat,
SAMU 192, CAPS, "psicólogo com registro ativo no CRP"; `findahelpline.com` for other
countries), each with a `verified_on` date that is empty today: the release checklist blocks
on a person opening each one. DASA, AISEP and Sexaholics Anonymous are not listed
(research-counselling 3.2).

---

## 5. Commitment: how protection is turned off

### 5.1 Tightening and loosening

| Instant (tightening) | Delayed (loosening) |
|---|---|
| enable, raise level, add a block, add a strict-mode platform, turn vision on, raise the delay, raise caps, start a lockdown, switch between family resolvers | turn off, pause (15 min, 1 h, until tomorrow 06:00), lower level, allow a site, remove a block or learned name, untick a strict platform, resolver `none`, drop the policy layer, `learn_to_hosts` off, `may_quit_browser` off, vision off, add an app to `never_capture`, lower the delay, lower caps |

`ShieldService::request(Loosening) -> PendingView` creates a pending request and opens the
`release_wait` session; `cancel(id)` is instant. A matured request is applied without another
confirmation and recorded as an event. A pause re-arms by itself.

### 5.2 The delay

`delay_secs`: default 1,800 (30 min); presets 10 min, 30 min, 2 h, 24 h; range 0-86,400. With
0 the product behaves like an ordinary blocker and says so. Raising is instant; lowering is
itself a pending request that matures after the current delay (the Plucky rule).

Maturity (`ubiqx_shield::state::matured(&Pending, delay_secs, now) -> bool`, one function for
both places that keep a delay): `now >= matures_at` **and** `observed_secs >= min(delay_secs,
900)`, where `observed_secs` grows by at most 120 s per tick. Moving the system clock forward
therefore still costs 15 minutes of real, powered-on time (the whole delay when it is shorter).
A clock moved backwards never shortens anything, because `matures_at` is fixed at request time.

Where the delay is kept:

- `Locked`: in the machine state, by the root helper. The app database, `Settings` and the
  process being killed have no effect on it.
- `Shield`: requests that change the system layers (turn off, pause, lower level, drop a
  layer, resolver `none`) are registered by an elevated run, which stamps `requested_at` in the
  machine state, and completed by a second elevated run, which the helper refuses before
  `matures_at` (`release` exits with "matures at ..."). There are no ticks at this level, so
  the `observed_secs` rule does not apply and the delay is wall-clock only. List-level
  loosenings (allow a site, remove a learned name) are kept in the app's `shield_pending` and
  reach the hosts block at the next elevated apply. The level's description says it is weaker
  than `Locked` in exactly these two ways.
- `Guard`: in the app's table `shield_pending`; `maintain_once` plays the role of the tick.
  This is weaker (deleting the database or killing the app ends it) and the level's
  description says so.

### 5.3 Network rescue (the one undelayed exit)

"Consertar minha rede": reverts L2 only, for 15 minutes, then the sentinel re-applies it
(`Locked`) or the app asks to (`Shield`). L0, L1, L3 and L6 stay. Three uses per rolling
24 h; a fourth is a delayed request. It exists because a filtering resolver can break hotel
portals and corporate DNS, and a blocker that breaks the network gets removed entirely.
Each use is an event and counts toward the help heuristic.

### 5.4 Confirmation and partner

Requesting a loosening takes one screen with "Manter a proteção" as the primary button
(the existing `DisableWarnings` pattern), then the wait. There is no typed phrase, no
password, no penalty and no extension for tampering.

No accountability partner in v1: no controlled evidence, a real coercion and privacy risk,
and store-policy exposure (research-counselling 2 and 4). Nothing in the data model sends
anything to another person.

### 5.5 Deliberately not built

| Not built | Why |
|---|---|
| Password or key held by someone else; partner reports; uninstall alerts | The harm in this category is the surveillance model; this is a tool a person points at themselves |
| Hidden processes, obscured file names, resisting manual removal by an administrator | Visible and consented is a hard constraint. `docs/SHIELD-REMOVAL.md` lists every file, scheduler entry, policy value and resolver setting, with copy-paste commands per OS; `ubi-shield-helper release` works without the app |
| Punishment timers, delay extension on tamper, streak resets as a consequence | Non-shaming; abstinence violation effect |
| Blocking VPNs, Tor, other user accounts; disabling private browsing | Product-killing false positives; reported as "reduced" instead |
| Local DNS proxy, network extension, `pf`, kernel driver, TLS interception | Fail-closed or unavailable without Developer ID / EV signing |
| Images, titles or URLs stored; AI-written counselling; cloud sync; community | Privacy and safety (Quittr lesson) |
| "Add a list by URL", macOS configuration profiles, WFP/nftables port-53 confinement | v1.1 / v2 |

---

## 6. Data model

### 6.1 Settings (cosmetic only)

`Settings.shield: ShieldPrefs` with `#[serde(default)]` (`crates/ubiqx-core/src/model.rs`):
`discreet: bool = true`, `show_streak: bool = false`, `checkin_reminder: bool = false`,
`risky_hours: Vec<HourRange> = []` (adding is a tightening; it only raises vision cadence).
A lost or reset `Settings` therefore cannot weaken protection. Test: `{}` deserialises to the
defaults.

### 6.2 Tables

Migration V3 in `crates/ubiqx-storage/src/migrations.rs`; the three hard-coded `2`s
(`migrations.rs:273`, `tests/repos.rs:234`, `:242`) become `3`.

| Table | Columns | Wiped by "Apagar todos os dados" | Notes |
|---|---|---|---|
| `shield_config` | `id = 1`, `json`, `updated_at` | no | `ShieldConfig`: `enabled`, `level`, `resolver_provider`, `layers`, `safesearch`, `youtube`, `block_doh`, `block_proxies`, `strict_mixed: Vec<String>`, `vision_enabled`, `learn_to_hosts`, `may_quit_browser`, `never_capture: Vec<String>`, `delay_secs`, `max_checks_per_hour`, `vision_budget_usd`. Written only by `ShieldService` through the tighten/loosen rule |
| `shield_pending` | `id`, `kind`, `payload`, `requested_at`, `matures_at`, `observed_secs`, `status` | no | authoritative at level `Guard`, a mirror otherwise |
| `shield_domains` | `host` PK, `list` (`block`/`allow`/`learned`), `source` (`user`/`list_hit`/`keyword`/`vision`/`custom`), `created_at`, `hits`, `last_hit_at`, `effective_at` | no | the only place a host name is stored |
| `shield_paths` | `id`, `host`, `prefix`, `source`, `created_at` | no | mixed-platform path rules |
| `shield_profile` | `id = 1`, `consent_at`, `vision_consent_at`, `values_sentence`, `onboarding_answers` | text fields yes, consent timestamps no | |
| `shield_plans` | `id`, `when_text`, `then_text`, `position` | yes | at most three |
| `shield_events` | `id`, `at`, `kind`, `source`, `value` | yes | no host, no title, no URL. Kinds: `block`, `vision_positive`, `vision_soft`, `wrong_report`, `lapse`, `surf_done`, `panic`, `lockdown`, `checkin`, `requested`, `cancelled`, `matured`, `rescue`, `repaired`, `help_shown` |
| `shield_daily` | `date` PK, `blocks`, `vision_checks`, `vision_positives`, `vision_cost_usd`, `lapses`, `surfs`, `mood` | yes | feeds the 30-day view |

Enum columns go through `as_str` / `parse`; `shield_events.kind` and `.source` parse unknown
tags to `Other` instead of failing the query, so an older build that shares the database
(impossible across the V3 gate anyway) or a newer event kind cannot break a read. The existing
`interventions` table is not touched and receives no shield rows.

Protective tables survive "Apagar todos os dados" on purpose (the wipe would otherwise be a
one-click off switch); the dialog says "O Escudo continua ativo; para desligá-lo use a tela do
Escudo". "Apagar dados do Escudo" on the shield page erases `shield_events`, `shield_daily`,
`shield_plans` and the profile texts immediately.

Retention: `shield_events` 90 days (trimmed by `maintain_once`), `shield_daily` until erased,
`journal.jsonl` 500 lines. Export: counts, check-ins, plans and the values sentence; host names
only when the user ticks "incluir minha lista".

### 6.3 Machine state

Section 1.2. The shared user-level directory (`ProjectDirs::from("ai", "ubiqx",
"UbiShield")`) holds downloaded packs and `primary.lock` (product id, pid, heartbeat) used at
level `Guard` to decide which installed product runs the passes.

### 6.4 Events and usage

`EngineEvent::Shield { status: ShieldStatus }` on every status change, and
`EngineEvent::ShieldCounsel { session: CounselSession }` when a session opens. Both need the
TypeScript twins and the CLI's `PrintSink` arms. `AiUsageKind::ShieldVision`. No new
`CoreError` variant: system-layer failures map to `CoreError::Platform`, elevation refusals to
`CoreError::Permission`, and loosening attempts that must wait return `Ok(PendingView)`.

---

## 7. Code architecture

### 7.1 Crates

| Crate | New / changed | Content |
|---|---|---|
| `crates/ubiqx-shield` | **new** (no dependency on `ubiqx-core`, no tokio) | Pure: `host.rs`, `blocklist.rs`, `heuristics.rs`, `matcher.rs`, `lists.rs`, `pack.rs` (manifest + Ed25519), `hosts_file.rs` (marker render/merge/remove), `resolver.rs` (provider table, per-OS plans), `policies.rs` (per-browser, per-OS renderings), `sentinel.rs` (launchd plist, systemd units, task XML), `state.rs` (`MachineState`, `Pending`, `matured`), `inbox.rs` (verbs, validation), `plan.rs` (`SysOp`, `plan_apply`, `plan_remove`), `drift.rs` (`Facts -> LayerState`), `probe.rs` (DNS query encode/decode), `tick.rs` (`decide`), `elevate.rs` (launcher command rendering). Thin: `sys/mod.rs` (`trait SysExec { fn facts(&self) -> Facts; fn run(&self, op: &SysOp) -> Result<(), SysError>; }`), `sys/linux.rs`, `sys/macos.rs`, `sys/windows.rs`, `sys/fake.rs` (records ops, serves scripted facts). Data in `data/` |
| `crates/ubiqx-shield-helper` | **new** bin `ubi-shield-helper` | Argument parsing for the seven verbs, `main` = gather facts, call `tick::decide` or `plan_*`, execute, write state and `result.json` |
| `crates/ubiqx-core` | changed | `model.rs`: `ShieldPrefs`, `ShieldConfig`, `ShieldLevel`, `ShieldStatus`, `ShieldIssue`, `LayerState`, `PendingView`, `Loosening`, `CounselSession`, `CounselKind`, `ShieldEvent*`, `EngineEvent::{Shield, ShieldCounsel}`, `ProductIdentity`. `ports.rs`: `ShieldSystem`, `ShieldRepo`, `ExplicitDetector`, `ImageDigest`, `ListSource`, `Companion`, `InterventionPresenter::show_counsel`, `BrowserUrlResolver::resolve_fresh`, `UsageRepo::totals_by_kind`, `AiUsageKind::ShieldVision`. New module `shield/`: `copy.rs`, `counsel.rs` (session selection, rotation), `ladder.rs`, `schedule.rs` (`next_check`, `judge`), `commitment.rs` (tighten/loosen classification), `help.rs`, `progress.rs` (30-day view, streak). `segmenter.rs`: `sensitive` predicate. Depends on `ubiqx-shield` for `Verdict`, `MachineState`, `InboxVerb` |
| `crates/ubiqx-storage` | changed | `migrations.rs` V3, `repos/shield.rs` (`impl ShieldRepo`), `repos/usage.rs` (`totals_by_kind`), `repos/maintenance.rs` (wipe list) |
| `crates/ubiqx-platform` | changed | `shield_system.rs` (`NativeShieldSystem`: unprivileged `status()` through `ubiqx_shield::sys`, elevated `install/apply/release` through `elevate`, inbox writes), `companion.rs`, `image_util.rs` (`LumaDigest`), `list_source.rs` (HTTP), `mock.rs` (`MockShieldSystem`, `MockCompanion`), resolvers' `resolve_fresh`, self-exemption from `ProductIdentity`, fields in `PlatformServices` |
| `crates/ubiqx-ai` | changed | `explicit.rs` (`LlmExplicitDetector`), `prompts.rs` (`explicit_system_prompt`), `fake.rs` (`FakeExplicitDetector`; `ScriptedLlmClient::push_error`), `openai.rs` (usage-kind arm), `client.rs` + `openai.rs` (image-free trace) |
| `crates/ubiqx-engine` | changed | `shield/mod.rs` (`ShieldService`, `ShieldPorts`, `ShieldEnv`), `shield/guard.rs`, `shield/watch.rs`, `shield/maintain.rs`, `shield/commit.rs`, `shield/lists.rs`, `shield/read.rs`; `deps.rs`, `state.rs`, `engine.rs` (`shield_loop`, `shield_maintain_loop`, `LoopConfig::{shield_every, shield_maintain_every}`), `handle.rs` (`fn shield(&self) -> &Arc<ShieldService>`), `classify.rs` (budget excludes `shield_vision`) |
| `crates/ubiqx-app` | changed | `ProductIdentity` in `AppConfig`; `ShieldApp::start(AppConfig) -> ShieldAppHandle` (the standalone host); wiring of the detector in `build_ai` |
| `crates/ubiqx-cli` | changed | `PrintSink` arms; subcommand `shield` with `status`, `enable`, `request-off`, `cancel` and `remove-info`, which talks to `ShieldSystem` without starting the engine |
| `apps/shell-shield` | **new** lib `ubiqx-shell-shield` (Tauri-dependent, not in `default-members`) | The shield `#[tauri::command]`s over `Arc<ShieldService>`, the `counsel` window builder, the presenter adapter's `show_counsel`, tray helpers |
| `apps/desktop/src-tauri` | changed | registers the shared commands, tray item, `capabilities/counsel.json`, single-instance plugin, `externalBin` for the helper |
| `apps/leme/src-tauri` | **new** | Leme shell: own `tauri.conf.json`, icons, `Info.plist`, capabilities, small `lib.rs` |
| `services/ubi-api` | changed | body limit, image passthrough test, shield ledger, `/v1/shield/lists/*` (only if the repo is private) |

### 7.2 `ShieldService`

```rust
pub struct ShieldPorts {
    pub activity: Arc<dyn ActivitySource>, pub urls: Arc<dyn BrowserUrlResolver>,
    pub capturer: Arc<dyn ScreenCapturer>, pub digest: Arc<dyn ImageDigest>,
    pub enforcer: Arc<dyn Enforcer>, pub presenter: Arc<dyn InterventionPresenter>,
    pub notifier: Arc<dyn Notifier>, pub system: Arc<dyn ShieldSystem>,
    pub companion: Arc<dyn Companion>, pub lists: Arc<dyn ListSource>,
    pub detector: Option<Arc<dyn ExplicitDetector>>,
    pub repo: Arc<dyn ShieldRepo>, pub usage: Arc<dyn UsageRepo>,
    pub clock: Arc<dyn Clock>, pub sink: Arc<dyn EventSink>,
}
pub trait ShieldEnv: Send + Sync {            // what the host knows
    fn language(&self) -> UiLanguage; fn prefs(&self) -> ShieldPrefs;
    fn vision_model(&self) -> String; fn remote_ready(&self) -> VisionReadiness;
    fn identity(&self) -> &ProductIdentity;
}
impl ShieldService {
    pub fn new(ports: ShieldPorts, env: Arc<dyn ShieldEnv>) -> CoreResult<Arc<Self>>;
    pub fn pass_once(&self) -> CoreResult<PassReport>;               // 2 s, blocking pool
    pub async fn watch_once(&self) -> CoreResult<Option<Outcome>>;   // vision call decided by pass_once
    pub fn maintain_once(&self) -> CoreResult<()>;                   // 60 s
    pub fn status(&self) -> ShieldStatus;
    pub fn enable(&self, req: EnableRequest) -> CoreResult<ShieldStatus>;      // elevation when level > guard
    pub fn tighten(&self, change: Tightening) -> CoreResult<ShieldStatus>;
    pub fn request(&self, change: Loosening) -> CoreResult<PendingView>;
    pub fn cancel(&self, id: &str) -> CoreResult<()>;
    pub fn net_rescue(&self) -> CoreResult<ShieldStatus>;
    pub fn restore(&self) -> CoreResult<ShieldStatus>;               // level shield: elevated re-apply
    pub fn domains(&self) -> CoreResult<Vec<ShieldDomain>>;
    pub fn add_block(&self, raw: &str) -> CoreResult<ShieldDomain>;
    pub fn report_wrong(&self, session_id: &str) -> CoreResult<WrongOutcome>;
    pub fn counsel(&self, id: &str) -> CoreResult<CounselView>;      // window payload, reason on demand
    pub fn record(&self, action: CounselAction) -> CoreResult<()>;   // surf done, lapse, check-in, panic, lockdown
    pub fn progress(&self) -> CoreResult<ShieldProgress>;
    pub fn erase_history(&self) -> CoreResult<()>;
}
```

`ShieldSystem` (core port, `Ok(true)` done / `Ok(false)` could not / `Err` unexpected, like
`Enforcer`): `status() -> CoreResult<MachineView>`, `install(&InstallRequest)`,
`apply(&ApplyRequest)`, `submit(&InboxVerb)`, `release_now()` (only when nothing is pending or
the delay is 0), `capabilities() -> SystemCaps` (which levels and layers this OS/session
supports).

ubiqX: `EngineState` owns `Arc<ShieldService>` and implements `ShieldEnv`.
`remote_ready()` does not call `EngineState::remote_allowed()`: it checks `local_only`,
`onboarding_done`, `license.blocks_ai()` and the presence of the selected provider's key, and
ignores the shared `AiHealth`, so a paused classifier does not blind the shield and a failing
shield call never pauses classification (map-vision-ai 1.7).
`Engine::start_with` spawns `shield_loop` and `shield_maintain_loop` (both skip their first
tick, like `focus_loop`). With `ScriptedPlatform` the shield reads the foreground through a new
non-consuming `peek_foreground()`, because a second consumer of `foreground()` would steal
samples from the tracker (map-engine-storage 1.6). Disabled is the default: with `shield_config.enabled = false` the pass returns before
touching the window server, so nothing changes for existing users until they opt in.

### 7.3 IPC (thin, in `apps/shell-shield/src/commands.rs`)

`shield_status`, `shield_capabilities`, `shield_enable`, `shield_tighten`, `shield_request`,
`shield_cancel`, `shield_net_rescue`, `shield_restore`, `shield_domains`, `shield_add_block`,
`shield_report_wrong`, `shield_counsel`, `shield_record`, `shield_progress`, `shield_plans_get`,
`shield_plans_set`, `shield_profile_get`, `shield_profile_set`, `shield_help_resources`,
`shield_erase_history`, `shield_removal_guide`. Each is mirrored in `ipc.ts`, `types.ts` and
`mock.ts`. Network access stays in Rust (CSP).

### 7.4 Frontend (`apps/desktop/src`)

- `shield/pages/Shield.tsx` (status, layers with issue rows, level, delay, pending requests
  with countdown, panic button, 30-day view), `shield/pages/ShieldLists.tsx` (block, learned
  with source, allow with "effective at"), `shield/pages/ShieldSupport.tsx` (values, plans,
  check-in, help resources), `shield/pages/Counsel.tsx` (the window; routes by session kind),
  `shield/pages/ShieldOnboarding.tsx` (five steps: what it is and is not; level with what
  each installs; resolver disclosure; vision consent; delay and "how to leave").
- Components: `LayerRow`, `IssueBanner`, `DelayPicker`, `PendingCard`, `LoosenDialog`
  (primary button keeps protection), `UrgeSurf`, `CrisisCard`, `WhyBlocked`, `ProgressGrid`.
- Store: slice `shield` in `useAppStore`, `applyEvent` cases for `shield` and `shield_counsel`.
- Routes: `/shield`, `/shield/lists`, `/shield/support` inside `AppShell`; `#/counsel` outside
  the `Gate`. Sidebar entry "Escudo".
- i18n: namespaces `shield.ts` and `counsel.ts`, PT-BR and EN, registered in `NAMESPACES`;
  Rust-side tray and notification strings in a `match UiLanguage` table in `shell-shield`.
  Settings › Privacidade keeps "apps/domínios bloqueados" for "never recorded"; the shield
  copy says "sites que o Escudo barra" to avoid the collision.

### 7.5 One code base, two products

| | ubiqX | Leme |
|---|---|---|
| Host | `Engine` + `EngineState` | `ubiqx_app::ShieldApp`: `SqliteStore`, platform services, `RoutingLlmClient` with only the detector, `ShieldService`, `UpdateChecker`, two loops |
| Runs | everything + shield loops | shield pass, maintain, update check, licence re-evaluation. **No** sampler, tracker, segmenter, classifier, screenshots table, reports, nudges, insights, focus sessions |
| Frontend | full app + `/shield/*` | same Vite project built with `VITE_PRODUCT=leme`: entry `src/leme/main.tsx`, routes = Shield pages + a reduced Settings (language, AI provider and key, licence, updates, about), onboarding = `ShieldOnboarding` |
| Shell | `apps/desktop/src-tauri` | `apps/leme/src-tauri`: tray (open, "Preciso de um minuto", "Aconteceu", pause/turn off through the delay), `main` and `counsel` windows |
| Identity | `ai.ubiqx.app`, binary `ubiqx-desktop` | `ai.ubiqx.leme`, binary `leme-desktop`, own data dir and database (same migration list), own update feed (`UBIQX_UPDATE_FEED_URL`) and release tag |
| Settings | `Settings` (whole struct) | the same `Settings` row; the reduced UI reads, patches and writes the whole object |
| Machine state | shared: one `state.json`, one marker, one helper. `installed_by` names the managing product; the other shows "O Escudo deste computador é gerenciado pelo <produto>" and runs no passes | same |
| Secrets | keyring service `ai.ubiqx` | `ProductIdentity.keyring_service = "ai.ubiqx.leme"` (separate; no shared Keychain prompts) |

`ProductIdentity { id, display_name, bundle_id, exe_stems, keyring_service, user_agent,
feed_product }` replaces the constants `SELF_BUNDLE_ID`, `EXEMPT_APP_IDS/NAMES` (self part),
`SELF_STEMS`, the literal in `hide_others_script`, the keyring service and the HTTP user
agents; `UpdateChecker` additionally refuses a feed whose `product` differs from
`feed_product` (map-app-shell 13.1).

Licence: v1 keeps the two existing plans and `Soft` enforcement, and both unlock both apps
(option 1 of map-app-shell 12.2: no change to claims, old keys keep working). L0-L5 are never
gated: a lapsed payment must not remove a block. Vision with the user's own key is not gated.
Vision through "IA do Ubi" needs a valid `monthly_managed` key, as every managed call does.
Selling Leme at its own price needs new `Plan` variants and is deferred until checkout exists
(`CHECKOUT_*_URL` are empty today).

Release hygiene: `publish-release.mjs`, `app-version.mjs` and the CI globs take a product
parameter; Leme artefacts are named `Leme-*` and published under tag `leme-continuous`.

---

## 8. Naming and branding

- **Do not ship the word "nofap"** in any product name, window title, bundle id, process or
  service name, file path on the user's machine, DNS marker, i18n string, package metadata or
  site copy. NOFAP is a registered US mark (classes 41, 42, 25, 35) with reported 2026 filings
  in the UK and India and an owner that litigates; Brazil is unknown, not clear
  (research-counselling 5). The term also commits the product to a masturbation-abstinence
  stance it must not take.
- **Code identifier: `shield`** (crate `ubiqx-shield`, settings key `shield`, tables
  `shield_*`). "guard" is already taken by the focus guard.
- **Machine-level owner id: `ubi-shield`** (marker, directories, scheduler entries, helper
  name). It is independent of the product names on purpose: files on users' machines cannot be
  renamed later.
- **Module in ubiqX: "Escudo" (PT-BR) / "Shield" (EN)**, shown as "Escudo do UBI".
- **Standalone product: working name "Leme"** ("você no leme"), subtitle "Bloqueio de
  pornografia e apoio para mudar o hábito". Not cleared: INPI and USPTO searches (classes 9,
  42, 44) and a domain check come before any public build. The name is one field of
  `ProductIdentity` plus the Leme i18n bundle and icons.
- Test `no_reserved_terms`: fails if "nofap" (any case) appears in `apps/*/src/i18n`,
  any `tauri.conf.json`, `Info.plist`, `Cargo.toml` package metadata or
  `crates/ubiqx-shield/src/sentinel.rs` renderings.

---

## 9. Test plan

### 9.1 Unit (pure logic, Linux CI)

| Area | Cases |
|---|---|
| `host`, `blocklist` | hash vector shared with the Python script; malformed tables rejected (magic, count, order); suffix walk depth; IDN |
| `matcher`, `heuristics` | authority order of section 2.4; longest match; trap corpus must not block; brand look-alikes; embedded Core passes `canary-block.txt` / `canary-allow.txt`; path rules |
| `hosts_file` | append, replace, remove; bytes outside the marker identical; CRLF kept; cap and nine-per-line on Windows; A + AAAA lines; hash in marker; damaged or duplicated markers |
| `resolver`, `policies`, `sentinel` | golden renderings per OS (command lines, drop-in, plist, registry ops, task XML, systemd units); undo restores only values still equal to what was written; conflict detection |
| `tick::decide` | repair per layer; benign vs non-benign drift; inbox validation (owner, size, schema, never-block refusal, cap); pending maturity including clock jumps forward and backward, `observed_secs` rule; rescue quota and auto-rescue after five failed probes; pause re-arm; matured `turn_off` emits full removal and self-uninstall; idempotence (second run with repaired facts emits no ops) |
| `pack` | signature, rollback refusal, hash and length mismatch, canary failure keeps the old pack |
| core `ladder`, `schedule`, `commitment`, `help`, `progress`, `copy` | escalation timing; candidate table of 3.2; `judge` table of 3.5 including refusals; every `ShieldConfig` change classified as tightening or loosening (exhaustive match, so a new field cannot default to "instant"); thresholds; key parity PT-BR/EN; "never say" list; `no_reserved_terms` |
| storage | V3 migration from a V2 database; repo round trips; wipe keeps protective tables; unknown event kind parses to `Other`; `totals_by_kind` |
| ai `explicit` | request shape (image first, no metadata, schema closed), parse, unknown verdict -> `Unsure`, `AiRefused` -> `refused: true`, truncation retry; wiremock for Anthropic and OpenAI shapes; trace output contains no base64 |

### 9.2 Integration (`crates/ubiqx-engine/tests/shield.rs`, mock platform + fakes)

Harness copied from `tests/focus.rs`: in-memory SQLite, `FixedClock`, loops at 3600 s, passes
driven by hand, `MockEnforcer`, `MockShieldSystem` (scripted `MachineView`, records verbs and
elevated calls, refusal switches), `FakeExplicitDetector` (scripted verdict queue, call
counter), `SyntheticCapturer` / `DeniedCapturer`, `MemoryUsageRepo`.

1. Disabled: no platform call at all.
2. Listed host: close -> blank -> quit across three passes; no cooldown; one counselling
   session per 45 s; no screenshot taken.
3. Windows-style confirmation: one pass does not act, the second does.
4. Unknown host: no capture before the dwell; one capture after; `none` backs off; caps and
   budget stop calls; unchanged frame is not sent.
5. `explicit/high`: enforcement, `add_block` verb submitted, `shield_domains` row with source
   `vision`, event without host, no file written anywhere under the data dir.
6. Refusal twice -> positive; refusal once on a safe context -> re-check only.
7. Vision preconditions: `local_only`, no consent, no detector, denied capture, missing
   window id (asserts `PrimaryDisplay` is never requested).
8. Incognito title and tracker pause do not stop the shield pass or the capture.
9. Loosening: `request(TurnOff)` leaves status active; advancing the clock without ticks does
   not mature at level `Guard`; ticks do; `cancel` works; raising the delay is instant.
10. A whole-struct `update_settings` with default `Settings` leaves the shield enabled.
11. `delete_all_data` leaves the shield enabled and erases events.
12. Drift at level `Shield` opens a `restore` session; at `Locked` a journal `repaired` entry
    becomes one notice and one override count.
13. Bypass suspicion: listed host in front while L1 reports ok -> issue + heightened state.
14. Segmenter redaction: a blocked host produces a `[privado]` block.
15. Second product: `managed_by_other_product` and no passes.
16. Productivity budget unaffected by `shield_vision` rows and vice versa.
17. `ShieldApp::start` (standalone host): no `blocks`, `screenshots` or `reports` rows after
    a scripted hour.

Helper-level integration on Linux CI: `ubi-shield-helper` built with `sys::fake` behind a
test feature runs `install` -> `tick` -> inbox request -> ticks to maturity -> removal against
a temp directory tree standing in for `/etc`, asserting the final tree equals the initial one.

### 9.3 Frontend (Vitest + jsdom against `mock.ts`, asserting PT-BR copy)

Shield page states (off, protected, reduced with each issue, needs attention, releasing with
countdown, rescue); onboarding consent cannot be skipped and vision stays off without its own
consent; `LoosenDialog` primary button keeps protection; lists page shows source and
"effective at"; Counsel window per session kind, discreet text by default, no host rendered
before "Ver por quê"; urge-surf steps; crisis card one step from each screen; i18n parity and
`csp.test.ts` unchanged; Leme flavour renders no Timeline/Reports/Categories routes.

### 9.4 Cannot be tested here (real machines, before the design is frozen)

1. macOS: a root-owned ad-hoc-signed helper run by a LaunchDaemon installed through
   `osascript` (launch, Background Items notification, behaviour after the user disables it,
   behaviour after an app update); `WatchPaths` on the hosts file.
2. Windows: SYSTEM scheduled task from an unsigned binary; Defender / SmartScreen reaction to
   it and to the hosts write; one-minute repetition; the per-user companion task.
3. Linux: `pkexec` from an AppImage; resolved drop-in on Ubuntu 22.04/24.04 and Fedora;
   systemd `.path` unit; non-systemd fallback.
4. Hosts: size cap and nine-names-per-line on Windows (5k / 20k / 30k names: CPU and time to
   first resolution); whether an A-only line suppresses AAAA; whether hosts wins over browser
   DoH; effect of the DoH-endpoint entries on Chrome "secure" mode and Firefox TRR.
5. Policies: applied (not ignored) on unmanaged Windows 11 Home; Firefox on macOS through
   `defaults`; Chromium on macOS (advisory level; configuration-profile route); Snap/Flatpak
   paths.
6. Resolver: per-provider answers for blocked names, `use-application-dns.net`,
   `mask.icloud.com`, SafeSearch CNAMEs; captive portals with the auto-rescue; IPv6-only and
   dual-stack networks; VPN interplay.
7. Guard: `resolve_fresh` timing on macOS (AppleScript) and Windows (UIA); the ladder against
   real browsers; Windows typed-text false hits.
8. Vision: real cost per check per provider, refusal rates on explicit images, false positives
   on swimwear/medical/art pages, latency against the 12 s bound.
9. Uninstall paths per packaging format with protection on (what remains, that the machine
   keeps working, that `ubi-shield-helper release` works without the app).

---

## 10. Work breakdown

File ownership is disjoint between units that run at the same time. U0 lands first and is the
only unit that touches the shared contract files; after it, a unit edits only the files it
owns.

| Unit | Depends on | Owns | Acceptance |
|---|---|---|---|
| **U0 Contracts and scaffolding** | none | `crates/ubiqx-core/src/{model.rs,ports.rs,lib.rs,segmenter.rs}` (types, port traits with default methods, `ProductIdentity`, events), empty `crates/ubiqx-core/src/shield/mod.rs`; `crates/ubiqx-shield/{Cargo.toml,src/lib.rs}` with the public types (`Verdict`, `MachineState`, `InboxVerb`, `SysOp`, `Facts`) and `todo`-free stub modules; `crates/ubiqx-storage/src/migrations.rs` (V3) + version assertions; `crates/ubiqx-engine/src/deps.rs`, `LoopConfig` fields and every struct-literal site listed in map-engine-storage 12E; `crates/ubiqx-platform/src/{lib.rs,mock.rs}` (`MockShieldSystem`, `MockCompanion`, service fields with no-op natives); `crates/ubiqx-cli/src/main.rs` event arms; root `Cargo.toml` members | Workspace builds and all existing tests pass on Linux; cross-target `cargo check` passes; `Settings` from `{}` unchanged; shield disabled by default and inert |
| **U1 Lists and matcher** | U0 | `crates/ubiqx-shield/src/{host,blocklist,heuristics,matcher,lists,pack}.rs`, `crates/ubiqx-shield/data/*`, `data/shield/*`, `scripts/shield/*`, `.github/workflows/shield-lists.yml` | Section 9.1 rows 1-2 and `pack`; script reproduces the measured counts within the gate; embedded Core passes canaries; notices generated |
| **U2 System logic (pure)** | U0 | `crates/ubiqx-shield/src/{hosts_file,resolver,policies,sentinel,state,inbox,plan,drift,probe,tick,elevate}.rs`, `sys/{mod,fake}.rs` | Section 9.1 rows 3-5; `decide` idempotence test; golden files for all three OSes |
| **U3 System executors and helper** | U2 | `crates/ubiqx-shield/src/sys/{linux,macos,windows}.rs`, `crates/ubiqx-shield-helper/**` | Helper temp-tree integration test passes on Linux; macOS and Windows targets `cargo check`; each executor function is under 40 lines and has no branching beyond error mapping; verbs reject anything outside the closed set (tests) |
| **U4 Storage** | U0 | `crates/ubiqx-storage/src/repos/{shield,usage,maintenance}.rs`, `crates/ubiqx-storage/tests/repos.rs` | Repo round trips; wipe semantics of 6.2; `totals_by_kind` |
| **U5 AI detector and proxy** | U0 | `crates/ubiqx-ai/src/{explicit,prompts,fake,openai,client}.rs` (+ tests), `services/ubi-api/**` | Section 9.1 last row; proxy image passthrough test; body limit; shield ledger |
| **U6 Counselling logic and copy** | U0 | `crates/ubiqx-core/src/shield/{copy,counsel,ladder,schedule,commitment,help,progress}.rs`, `crates/ubiqx-core/data/shield/help-resources.json` | Section 9.1 core row; 40 messages in both languages; "never say" and `no_reserved_terms` tests |
| **U7 Platform adapters** | U2, U3 | `crates/ubiqx-platform/src/{shield_system,companion,list_source,image_util}.rs`, the three OS resolvers' `resolve_fresh`, `app_id.rs` and `macos/enforce.rs` identity parameter, the three `services()` | Mock-free pure parts tested (companion renderings, digest stability and distance on synthetic images); cross-target `cargo check`; `status()` works unprivileged against a temp tree on Linux |
| **U8 Engine service** | U1, U4, U5, U6 (uses U0 mocks) | `crates/ubiqx-engine/src/shield/**`, `engine.rs`, `state.rs`, `handle.rs`, `classify.rs` budget line, `crates/ubiqx-engine/tests/shield.rs` | All 17 integration cases of 9.2 except the helper one; existing engine tests unchanged and green |
| **U9 Shells (ubiqX)** | U8 | `apps/shell-shield/**`, `apps/desktop/src-tauri/**` | Commands registered; `counsel` window opens from a mock session; tray item; single instance; helper bundled as `externalBin`; shell crates `cargo check` on three targets |
| **U10 Frontend** | U0 types (can start with the mock), finishes after U9 | `apps/desktop/src/shield/**`, `src/i18n/messages/{shield,counsel}.ts`, the shield parts of `lib/{ipc,types,mock,store}.ts`, `App.tsx`, `Sidebar.tsx` | Section 9.3; parity and CSP tests green |
| **U11 Standalone product** | U8, U9, U10 | `crates/ubiqx-app/src/**`, `crates/ubiqx-cli/src/main.rs` (shield subcommands), `apps/leme/**`, `apps/desktop/src/leme/**`, `scripts/{publish-release.mjs,app-version.mjs,ci-app-version.sh}` + tests, `.github/workflows/ci.yml`, `codemagic.yaml` | Case 17 of 9.2; Leme builds on three OSes with its own identifier, feed and tag; ubiqX artefact names unchanged; update check refuses the other product's feed |
| **U12 Docs and release gates** | U3, U8 | `docs/SHIELD.md`, `docs/SHIELD-REMOVAL.md`, `docs/ARCHITECTURE.md`, `README.md`, `docs/WINDOWS-LINUX.md`, `docs/LICENSING.md`, `THIRD-PARTY-NOTICES` | Removal guide lists every artefact of section 1.2 per OS; the nine real-machine checks of 9.4 exist as a checklist with pass/fail fields; resource verification checklist |

Parallel groups after U0: {U1, U2, U4, U5, U6} -> {U3, U8 (once U1/U4/U5/U6 land)} -> {U7,
U9, U10} -> {U11, U12}.

---

## 11. Risks and open questions for the owner

1. **A root sentinel installed by an app that is unsigned on Windows and ad-hoc signed on
   macOS.** It is what makes the block hold, and it is also the part most likely to trip
   Defender, SmartScreen or macOS background-item controls, and the largest security surface
   in the product. *Recommended default:* build all three levels; ship `Locked` as the
   recommended level on Linux and Windows after checks 2-3 of 9.4, and on macOS only after
   check 1 passes (until then macOS recommends `Shield`). Buy code signing (Developer ID,
   Windows certificate) before any public release of Leme.
2. **Default delay and automatic learning.** *Recommended default:* 30 minutes; learned names
   promoted to the hosts block automatically, with the opt-in screen stating that the hosts
   file is readable by other accounts on the computer. The counselling report's suggestion is
   10 minutes; choose it if first-run friction matters more than holding power.
3. **Resolver default and terms.** *Recommended default:* Cloudflare for Families with
   attribution, opt-out to hosts-only on the same screen; CleanBrowsing Family stays hidden
   until written clearance or a paid plan.
4. **Vision looks at private-browsing windows and ignores the tracker's pause and private
   mode.** It is the only way those do not become off switches; it is also the most sensitive
   capture the product makes. *Recommended default:* yes, behind a separate, explicit consent
   that says exactly that, with `local_only` honoured and nothing stored. "IA do Ubi" is
   allowed for vision with the extra sentence that images pass through the operator's proxy.
5. **Names.** *Recommended default:* `shield` in code, "Escudo" in ubiqX, "Leme" as the
   working name of the product, trademark searches before a public build; "nofap" nowhere a
   user or a store can see.
6. **Release channel.** CI publishes every push on every branch to every installed app
   (map-app-shell 11.3 and 14). Code that installs a root helper must not reach users that way.
   *Recommended default:* before U3 or U9 is pushed, restrict `publish-continuous` to `main`
   and keep `shield_config.enabled = false` with no automatic prompt, so an update alone
   changes nothing on a user's machine. Licence: existing plans unlock both apps, blocking is
   never gated.
