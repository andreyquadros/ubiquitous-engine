# Design (MVP angle): the pornography shield module and its standalone product

Angle of this proposal: **ship a correct first version first**. The smallest architecture that
fully answers the owner's request on macOS, Windows and Linux with the building blocks already
in the repository, with every decision in pure, unit-testable code and a thin, isolated system
layer. Written on 2026-11-19 from the seven reports in this folder and from the code. Nothing
was compiled or run for this document.

Working name in this document: **shield** (code) / **Escudo do UBI** (UI). Section 8 explains
why the code and the UI do not say "nofap", and section 11 puts the choice to the owner.

Marks: **[U]** = unverified on a real machine or against a primary source; it is a test item
in section 9.4, never a silent assumption.

## 0. The design in one page

| Question | Decision |
|---|---|
| What blocks | Four layers. **L1 guard**: the foreground browser tab's host is matched against an embedded 239k-entry list plus keyword and TLD rules, every 2 s, and the tab is closed. **L2 hosts**: a marked block of about 20,000 popular adult hostnames in the hosts file. **L3 resolver**: system DNS pointed at Cloudflare for Families (opt-in, disclosed). **L4 vision**: the app's existing AI looks at the foreground window when the site or app is unknown. |
| What is not built in v1 | Local DNS proxy, network extension, browser enterprise policies, SafeSearch pinning, downloadable list packs, privileged background helper, accountability partner, hard locks, AI-written counselling. Each has a named place to be added later (section 5.4 and 7.9). |
| Where the logic lives | One new crate, `crates/ubiqx-shield`: pure functions and embedded data, no I/O, no OS calls. Every decision (match, hosts text, drift, sampling, commitment, copy) is tested on Linux. |
| System layer | One port, `SystemShield`, with one adapter whose three OS back-ends are command builders and output parsers compiled and tested on every OS behind a `Runner` trait. The only untestable code is the process spawn and the Windows `ShellExecuteExW` call. |
| Elevation | The app's own executable re-run as `--shield-helper apply` or `--shield-helper remove` through `osascript ... with administrator privileges`, `ShellExecuteExW` `runas`, or `pkexec`. One OS prompt per apply. No daemon. |
| Configuration | Not in `Settings`. One engine-owned row (`shield_state`) changed only through typed requests; requests that weaken protection wait out a user-chosen delay (default 10 minutes). |
| Standalone product | The same binary sources, the same engine, the same frontend project. `LoopConfig.productivity = false` starts no sampler, classifier, reports, nudges, retention or focus loop. A second Tauri config file and a second frontend entry give it its own name, identifier, data directory, window and tray. |
| Licence | No change in v1: blocking is never gated; AI vision needs the user's own key or the existing managed plan. |

How it differs from the obvious alternative is in section 12.

---

## 1. Enforcement layers

Protection is opt-in. Nothing below runs until the user completes the enable wizard
(section 4.1). The four layers are independent: each one has its own state and the product
stays useful when any of them is missing.

### 1.1 L1: domain match in the guard (no privilege)

**What it does.** A pass every 2 s (`LoopConfig.shield_every`) reads the foreground window and,
for a supported browser, the active tab's URL, exactly as `focus::guard_once` does today
(`crates/ubiqx-engine/src/focus.rs:190-209`). The host goes to `Matcher::verdict` (section 2.4).
On `Verdict::Blocked` the pass enforces and counsels.

It is a sibling pass, `shield::guard::pass_once`, in its own loop. It does not touch
`focus::guard_once`, the focus session timer, `focus.guard_enabled` or `focus_targets`.
It returns before any platform call when the shield is off, so `ubiqx simulate` and the
scripted demo are unchanged for users who never enable it.

**Enforcement** (existing `Enforcer` methods only, no new platform code):

| Hit | Action, in order |
|---|---|
| Blocked host in a browser tab | `close_active_tab` → `blank_active_tab` → counselling window only |
| Blocked by vision in a browser tab | same |
| Blocked by vision in any other app | counselling window with a "Fechar {app}" button; `quit_app` runs only when the user presses it |

Three rules fix the weaknesses the focus-guard map found:

1. **No enforcement cooldown.** The existing guard returns before `intervene` during its
   20 s cooldown, so a closed site is usable for 20 s. The shield enforces on every pass in
   which the hit is still in front. Only the *window and the stored event* are rate limited,
   through an episode (below).
2. **Two consecutive passes.** A host is enforced when the same host was the verdict of the
   previous pass too (`CONFIRM_PASSES = 2`, about 2 s of latency). This removes the Windows
   case where host-shaped text still being typed in the address bar is read as the current
   page, and most stale-URL-cache cases.
3. **Episodes.** Hits less than `EPISODE_GAP_SECS = 120` apart are one episode: one stored
   event with an `attempts` counter, the counselling window at the first hit, and once more at
   the third attempt with the "third try" message that offers the urge-surf timer.

**Exemptions.** `focus::is_exempt_window` (already `pub`, `crates/ubiqx-engine/src/focus.rs:83`), plus both products' own bundle
ids, names and executable stems added unconditionally to `EXEMPT_APP_IDS`, `EXEMPT_APP_NAMES`
(`crates/ubiqx-core/src/focus.rs:32-56`), `SELF_STEMS`
(`crates/ubiqx-platform/src/app_id.rs:9`) and the literal in `hide_others_script`.

**Coverage and the user-visible state.** The status read model reports the guard as
`on` or `limited` with a reason, never silently blind:

| Situation | State shown |
|---|---|
| macOS, Automation denied for the browser | `limited: automation_denied` with the button that opens the permission pane. The shield reads `PermissionChecker::status()` itself, because only the tracker's sampler emits `PermissionRequired` today. |
| macOS Firefox (no URL, no tab action) | `limited: browser_without_url`; L2, L3 and L4 carry it |
| Linux X11 (host known only when the title prints it) | `limited: title_only` |
| Linux Wayland (`foreground()` is `None`) | `limited: wayland`; L2 and L3 only |
| Background tabs, other windows | never seen by L1; stated on the page |

**Tracker redaction (ubiqX only).** In `tracker::handle_sample`
(`crates/ubiqx-engine/src/tracker.rs:46`), when the shield is on and the sample's host is
`Verdict::Blocked`, the sample is rewritten the way the segmenter rewrites private browsing
(`window_title = PRIVATE_TITLE`, `url = None`, `window_id = None`) before `segmenter.feed`.
A blocked attempt therefore never reaches the timeline, the classifier, a report, a
screenshot or an export.

### 1.2 L2: hosts file

**What it does.** The elevated helper writes one marked block into the hosts file:

```
# >>> ubiqx-shield v1 BEGIN sha256=<hex of the lines between the markers> names=<n>
0.0.0.0 name1 name2 name3 name4 name5 name6
:: name1 name2 name3 name4 name5 name6
...
# <<< ubiqx-shield END
```

- **Content:** the embedded hosts tier (section 2.2, at most 20,000 hostnames ordered by
  popularity), plus for every user-blocked and vision-learned domain the name itself and
  `www.` + name. Hard cap `HOSTS_MAX_NAMES = 24_000`; the tier is truncated from its tail,
  never the user's entries.
- **Both address families** (`0.0.0.0` and `::`), six names per line. Whether an IPv4-only
  line suppresses the AAAA lookup differs per resolver [U], and Windows reads a limited number
  of names per line [U]; writing both families at six per line is correct under either answer.
  Size is about 0.9 MB, inside the report's proposed Windows ceiling [U, measure].
- **No Microsoft-owned name ever** (Defender's `HostsFileHijack`), enforced by the same
  `PROTECTED_SUFFIXES` check that validates user entries.
- **Splice, never rewrite.** `hosts::splice(existing, Some(block))` replaces the block if the
  markers are present, else appends; everything outside the markers is preserved byte for
  byte, including the file's own line-ending convention. `splice(existing, None)` removes
  exactly the block. Write is temp file + rename in the same directory, mode and owner kept.
- Paths: `%SystemRoot%\System32\drivers\etc\hosts`, `/etc/hosts`.
- Cache flush after writing: `ipconfig /flushdns`; `dscacheutil -flushcache` and
  `killall -HUP mDNSResponder`; `resolvectl flush-caches` (ignored when absent).

**Verify** (unprivileged, every 120 s and on demand): read the file, `hosts::inspect`:

| Observation | Hosts state | What the user sees |
|---|---|---|
| Block present, marker hash = hash of its lines = hash of what the app would write now | `applied` | "Lista no sistema: ativa" |
| Block present and intact, but the app would now write something else (new learned domain, new app version) | `update_pending` | Quiet line "atualização pendente"; applied with the next elevated run. Never a prompt on its own. |
| Markers present, hash of the lines differs from the marker | `changed` | Drift (below) |
| No block although the state file says applied | `missing` | Drift |
| Never applied, or the user declined the OS prompt | `not_applied` | "Lista no sistema: não aplicada" + "Aplicar" |
| File unreadable, or packaging cannot elevate (AppImage) | `unavailable(reason)` | Reason and the manual steps |

**Limits, stated in the UI:** exact names only, so the long tail belongs to L1 and L3; pages
already open are unaffected ("vale para páginas novas; reinicie o navegador para garantir");
whether a browser with its own DoH still honours the hosts file is unverified [U].

### 1.3 L3: system resolver to a family filter

**What it does.** Points the system's DNS servers at a filtering public resolver so the
millions of names no list of ours holds are refused by someone who maintains that list.

v1 ships exactly one provider row and the choice "none":

| `ResolverId` | IPv4 | IPv6 | Notes |
|---|---|---|---|
| `cloudflare_family` | `1.1.1.3`, `1.0.0.3` | `2606:4700:4700::1113`, `2606:4700:4700::1003` | Blocks malware and adult content. Attribution to Cloudflare in About and next to the toggle (their terms contemplate integrators) [U, re-read the terms]. |
| `none` | | | "Só a lista neste computador": L1, L2 and L4 only |

The table is data (`ubiqx_shield::resolvers::RESOLVERS`); a second provider is one row. The
helper accepts a `ResolverId`, never an address, so nothing a user-level process writes can
point the machine's DNS at an arbitrary server.

**Disclosure.** The wizard says, before the user confirms: "Com esta opção, todos os nomes de
sites que este computador consultar passam pela Cloudflare (1.1.1.3), no lugar do seu
provedor de internet. Você pode escolher 'só a lista neste computador'."

**Apply, per OS** (elevated; previous values recorded in the state file first):

| OS | Read | Set | Restore |
|---|---|---|---|
| macOS | `networksetup -listallnetworkservices` (skip the header line and names starting with `*`), `networksetup -getdnsservers <svc>` | `networksetup -setdnsservers <svc> <v4a> <v4b> <v6a> <v6b>` for every enabled service | recorded list, or `empty` when it was DHCP |
| Windows | PowerShell `Get-DnsClientServerAddress` joined with `Get-NetAdapter` (status `Up`), output through `ConvertTo-Json`; the static list from `HKLM\SYSTEM\CurrentControlSet\Services\Tcpip{,6}\Parameters\Interfaces\{GUID}\NameServer` [U] | `Set-DnsClientServerAddress -InterfaceIndex <n> -ServerAddresses (...)` on every adapter that is up (Windows asks all interfaces in parallel) | recorded static list, or `-ResetServerAddresses` when there was none |
| Linux | `systemctl is-active systemd-resolved`; the drop-in file | write `/etc/systemd/resolved.conf.d/90-ubiqx-shield.conf` (`[Resolve]`, `DNS=<v4a> <v4b> <v6a> <v6b>`, `Domains=~.`), `systemctl restart systemd-resolved` | delete the file, restart the service |

Plain DNS on all three in v1 (no DoT/DoH), so behaviour is the same everywhere and a network
that blocks port 853 cannot leave the machine without names. PowerShell output is JSON and
never localised text; `netsh` is not used. Linux without systemd-resolved reports
`unsupported` in v1 and shows the two manual commands; L1, L2 and L4 still run.

**Restore only what is still ours.** `plan::restore_actions(state, observed)` restores an
interface only when its current servers equal the set the helper wrote. A value the user
changed in the meantime is left alone. `plan::merge_previous` never records our own filter
addresses as "previous" when apply runs twice.

**Verify** (unprivileged, same 120 s loop; the reads above need no privilege; on Windows the
read is one PowerShell start every two minutes, a few hundred milliseconds of CPU):

| Observation | Resolver state | What the user sees |
|---|---|---|
| Every active interface (v4 and v6) uses the provider's set; on Linux the drop-in is present and equal | `filtered(provider)` | "DNS com filtro: ativo (Cloudflare for Families)" |
| At least one active interface uses something else | `drifted([names])` | Drift (below). New dongle, new VPN adapter and a manual reset all land here; the copy does not guess which. |
| User chose `none` | `off` | "DNS com filtro: desligado por você" |
| User pressed "Restaurar minha rede" | `network_restored` | Same line, with "Religar" |
| No systemd-resolved; AppImage | `unsupported(reason)` | Reason and manual steps |

**Known gaps, stated on the page:** a VPN brings its own resolver; a browser where the user
picked a specific DoH provider resolves around it (Chrome's default automatic mode keeps the
system's provider); captive portals may need "Restaurar minha rede" [U].

### 1.4 Drift: what happens when L2 or L3 stops matching

There is no silent self-repair, because re-applying needs the OS prompt and there is no
privileged daemon in v1.

1. The verify pass computes the new `ShieldStatus`; when it changes the engine emits
   `EngineEvent::Shield { status }` and the page and tray show "Proteção reduzida".
2. Once per drift episode (and at most once per 6 h) the counselling window shows a calm
   card: "A proteção do sistema mudou às {time}. Quer restaurar?" with "Restaurar" (runs
   `apply`, OS prompt) and "Agora não". It accuses nobody and starts no timer.
3. An event `drift` is stored with the layer name only.

### 1.5 The elevated helper

**Entry point.** `ubiqx_platform::sysblock::helper::main_if_requested(std::env::args())` is
the first statement of the shell's `run()`. When the first argument is `--shield-helper` it
does its work and exits before Tauri, AppKit or the database are touched; otherwise it
returns and the app starts normally. No second binary, no sidecar, no bundling change.

**Verbs and input.** `apply --plan <path>` and `remove --scope all|resolver`. The plan is a
small JSON file the app writes in its data directory:

```json
{ "v": 1, "owner": "ai.ubiqx.app", "hosts": true, "resolver": "cloudflare_family",
  "extra": ["example-added-by-user.com", "www.example-added-by-user.com"] }
```

The helper trusts none of it: `plan::validate` rejects an unknown version, an unknown
resolver id, more than 4,000 `extra` names, any name that fails `hosts::is_valid_hostname`
(ASCII letters, digits, hyphen, dots; 2 to 253 bytes; at least two labels) and any name under
`PROTECTED_SUFFIXES` (`localhost`, `local`, the OS vendors' update and sign-in domains, the
resolver's own names). The hosts tier comes from the helper's own embedded data. The worst a
hostile same-user process can obtain, and only after the user approves the OS prompt, is
"some extra hostnames resolve to 0.0.0.0".

**Output.** The exit code (0 applied, 2 invalid plan, 3 partial, 4 failed) and the state file,
written by the helper at a fixed root-owned, world-readable path. The helper never writes
into a user-controlled directory, so there is no symlink target for a root write.

| OS | State file |
|---|---|
| macOS | `/Library/Application Support/ubiqx-shield/state.json` |
| Linux | `/var/lib/ubiqx-shield/state.json` |
| Windows | `%ProgramData%\ubiqx-shield\state.json` |

```json
{ "v": 1, "owner": "ai.ubiqx.app", "applied_at": 1795100000000,
  "hosts": { "sha256": "...", "names": 20044 },
  "resolver": { "id": "cloudflare_family",
                "previous": [ { "iface": "Wi-Fi", "servers": null },
                              { "iface": "Ethernet", "servers": ["192.168.0.1"] } ] } }
```

**Elevation, per OS:**

| OS | How | Declined or unavailable |
|---|---|---|
| macOS | `/usr/bin/osascript -e 'do shell script "<exe> --shield-helper apply --plan <path>" with administrator privileges'`. Both paths are app-controlled and pass through `elevate::sh_quote` then `elevate::applescript_quote` (pure, tested). | User cancel is osascript error -128 → `ApplyOutcome::Declined` |
| Windows | `ShellExecuteExW`, verb `runas`, `SEE_MASK_NOCLOSEPROCESS`, wait, `GetExitCodeProcess`. `Win32_UI_Shell` and `Win32_System_Threading` are already enabled in `crates/ubiqx-platform/Cargo.toml`. | `ERROR_CANCELLED` (1223) → `Declined` |
| Linux | `pkexec <exe> --shield-helper ...` | exit 126 → `Declined`; 127 or no `pkexec` → `Unavailable(no_polkit_agent)`; running from an AppImage (`APPIMAGE` set) → `Unavailable(appimage)` without trying, because root cannot read the user's FUSE mount and the binary cannot load its libraries outside it |

Before the OS prompt the app shows one sentence saying what will change ("vou escrever uma
lista no arquivo hosts e trocar o DNS para 1.1.1.3"), because the macOS prompt names
`osascript` and the Windows prompt says "unknown publisher" for an unsigned build.

**Two products on one machine** share the marker and the state file. Apply is idempotent, the
last apply sets `owner`, either product's `remove` removes. Neither fights the other.

**Undo.** `remove --scope all`: delete the hosts block, restore resolvers per
`restore_actions`, delete the Linux drop-in, delete the state file, flush caches. It is safe
when half of it is already gone. Because nothing in v1 fails closed, a deleted app leaves a
working machine: names resolve, listed sites stay blocked until the block is removed.
`docs/SHIELD.md` documents the removal for every packaging format: the button, the command
(`sudo <exe> --shield-helper remove --scope all`, or an elevated prompt on Windows) and the
manual steps (delete the lines between the markers; set DNS back to automatic; delete the
drop-in).

### 1.6 L4: vision

Specified in section 3. It is the layer for what DNS cannot see: unknown sites, mixed
platforms, local files in a media player, content reached through a VPN.

### 1.7 Failure modes at a glance

| Failure | Effect | State shown |
|---|---|---|
| User declines the OS prompt | L1 and L4 run; L2 and L3 `not_applied` | "Proteção parcial" + "Aplicar" |
| App quit from the tray | L2 and L3 stay; L1 and L4 stop. The quit confirmation says so. Enabling the shield turns on launch-at-login (existing autostart plugin), stated in the wizard. | n/a |
| App deleted without disabling | L2 and L3 stay and the machine works; removal is documented | n/a |
| No AI key, offline, budget spent, `local_only` | L4 off; L1, L2, L3 carry the product | "Visão por IA: desligada" + reason |
| Wayland | L1 and L4 off | "Neste sistema: só a lista e o DNS" |
| Hosts write refused (security software) | helper exit 3; resolver may still be applied | `hosts: unavailable(write_refused)` |
| Network breaks after the resolver change | One undelayed button restores DNS (section 5.3) | `resolver: network_restored` |

---

## 2. The blocklist

### 2.1 Sources and licences

v1 ships the **Core tier only**, embedded, and nothing downloadable.

| Input | Entries (measured 2026-11-19) | Licence | Use |
|---|---|---|---|
| StevenBlack/hosts `porn-only` | 76,810 | MIT; two small upstream inputs are CC BY 4.0 | Core |
| ShadowWhisperer/BlockLists `Adult` | 213,982 | Unlicense | Core |
| 4skinSkywalker/Anti-Porn-HOSTS-File | 70,174 | MIT | only entries that carry a block-grade keyword |
| `extra-ptbr.txt` (ours, hand-curated) | starts at about 100 | ours | Core; the Brazilian gap (`privacy.com.br`, `closefans.com` are on no public list) |
| `allow.txt` (ours) | starts at about 300 | ours | removed from every tier; also the run-time built-in allowlist |
| Chrome UX Report top-1M | | CC BY 4.0 | build-time ordering of the hosts tier |
| Public Suffix List | | MPL-2.0 | build-time filter only, not shipped |

Result: about 239,449 entries. **Not shipped:** Bon-Appetit (CC BY-SA 4.0), blocklistproject
(UT1-derived), HaGeZi and oisd (GPL-3.0). The share-alike Extended pack roughly doubles list
coverage but needs a separately licensed data file, a download path, a signing key and a
weekly CI job. L3 covers that long tail at the DNS level in v1 with none of that machinery.

Licence handling: `THIRD-PARTY-NOTICES` at the repository root and in the About screen of
both products, with the text of section 13 of `research-blocklists.md`; the MIT copyright
lines are copied by the build script from the pinned commits, not typed.

### 2.2 Build and embedded format

`scripts/shield/build_blocklist.py` (Python 3 standard library) is run by a maintainer, not by
`cargo build` and not by CI. Its outputs are committed, so builds need no network and are
reproducible:

| File under `crates/ubiqx-shield/data/` | Content |
|---|---|
| `sources.lock.json` | URL, commit SHA and SHA-256 of every upstream file, the PSL and the CrUX snapshot |
| `core.bin` (about 1.92 MB) | header `"UBXBL1\0\0"`, `format u16`, `flags u16`, `count u32`, `list_version u64`, then `count` sorted unique big-endian `u64` keys. Key = first 8 bytes of SHA-256 over the bytes `"ubiqx-shield-blocklist-v1\0"` followed by the host |
| `hosts-tier.txt` (about 20,000 lines) | plain hostnames for L2: CrUX origins with rank bucket ≤ 500k covered by Core, plus the apex and `www.` of each covered registrable domain, most popular first |
| `allow.txt`, `mixed.txt`, `sensitive.txt`, `extra-ptbr.txt` | hand-maintained inputs, also embedded with `include_str!` |
| `canary-block.txt`, `canary-allow.txt` | a dozen names each; a unit test runs them against the embedded table |
| `core.meta.json`, `NOTICE.txt` | counts, hashes, generated attribution |

Cleaning rules (all from `research-blocklists.md` section 12): lower-case; strip `www.`,
`m.`, `mobile.`; drop IP literals and public-suffix entries; drop `*.tumblr.com` and
`*.blogspot.*`; never collapse a subdomain entry to its registrable domain; remove allowlisted
names; suffix-deduplicate. Script gates: canaries, no public suffix, no single label, entry
count within −10% / +25% of the previous `core.meta.json`, byte-identical output on a second
run, and a printed review list of every listed name inside the CrUX top-10k.

Why a hash table: no new dependency (`sha2` is in the workspace), memory equals file size,
lookup is a binary search, and the shipped file is not a readable directory of porn sites on
the disk of someone trying to stay away from them. False match probability for a benign
name is about 1.3e-14.

The data lives in `ubiqx-shield`, not in `ubiqx-core`, because `services/ubi-api` depends on
`ubiqx-core` and must not embed it.

### 2.3 Keyword and TLD rules (`ubiqx_shield::heuristics`)

Applied only to hosts no list or user entry decides, never to names under the built-in
allowlist or under `gov`, `gov.*`, `edu`, `edu.*`, `ac.*`, `mil`, `jus.br`.

| Grade | Rule | Effect |
|---|---|---|
| Block | TLD in `xxx`, `porn`, `sex`, `sexy`, `adult` | `Blocked(Tld)` |
| Block | substring, after look-alike folding (`0→o 1→i 3→e 4→a 5→s 7→t vv→w`, edge digits stripped) for brand stems: `porn`, `xxx`, `hentai`, the brand stems of the report's section 9.1, the explicit act words, `bokep`, `phimsex`, and PT-BR `putaria`, `buceta`, `siririca`, `contoseroticos`, `brasileirinhas`, `safadas`, `novinhas`, `gostosas`, `peladas`, `transando`, `gozando` | `Blocked(Keyword)` |
| Weak | whole-token match of `milf`, `sexy`, `adult`, `escort`, `naked`, `nude`, `pussy`, `sexo`, `bdsm`, `webcam`, `cams`, `teen`, `amador`; TLD `cam`, `webcam`, `tube` | `Unknown { signal: Weak }`: raises vision priority, never blocks |
| Never | `sex`, `anal`, `cum`, `ass`, `cock`, `dick`, `tits`, `cam`, `gay`, `jav`, `pau`, `rola`, `pica`, `puta`, `bunda`, `pelada` as substrings | a unit test asserts `essex.gov.uk`, `canaltech.com.br`, `planalto.gov.br`, `motorola.com.br`, `drogariasaopaulo.com.br`, `peacocktv.com` stay unknown |

`heuristics::title_signal(title)` applies the block-grade substrings to a window title. A
title never blocks; it makes the window a high-priority vision candidate (section 3.1) and,
without AI, produces the soft "quer que eu feche?" card.

### 2.4 Matching semantics (`ubiqx_shield::matcher`)

```rust
pub enum Verdict { Blocked(BlockSource), Allowed(AllowSource), Unknown { signal: Signal } }
pub enum BlockSource { User, Learned, List, Tld, Keyword }
pub enum AllowSource { User, BuiltIn }
pub enum Signal { None, Weak, Mixed }

impl Matcher {
    pub fn new(core: &'static [u8], domains: &[ShieldDomain]) -> CoreResult<Self>;
    pub fn verdict(&self, host: &str) -> Verdict;
    pub fn verdict_for_url(&self, url: &str) -> Verdict;   // uses normalize::domain_of semantics
}
```

1. Normalise: lower-case, strip trailing dot, port and `www.`; `url::Url` already yields the
   punycode host. An IP literal is `Unknown`.
2. Walk the host and its parents down to two labels. Each entry covers itself and its
   subdomains.
3. The most specific match wins; at equal specificity the order is user block, user allow,
   built-in allow, learned, list.
4. No match: heuristics (2.3); then `mixed.txt` membership gives `Unknown { signal: Mixed }`.

The matcher is built once and rebuilt only when `shield_domains` changes. The list is never
loaded into SQLite and never becomes `focus_targets`.

### 2.5 User additions, allowlist and learned domains

One table, `shield_domains (domain, kind, origin, created_at)`:

| `kind` / `origin` | Created by | Effect |
|---|---|---|
| `block` / `user` | "Adicionar site" on the page; the "strict" ticks on mixed platforms (Reddit, X, Tumblr and the others in `mixed.txt`) | L1 at once; L2 at the next elevated apply |
| `block` / `vision` | a confirmed vision positive on a host that is not on `mixed.txt`, `allow.txt` or `sensitive.txt` (`learn_domains`, default on, stated in the wizard) | same. Shown under "Sites que o UBI adicionou", each with "Remover" |
| `allow` / `user` | "Nunca bloquear" on the page | beats list and heuristics for that name |

Adding a block is immediate. Removing a block and adding an allow weaken protection and go
through the delay of section 5.

**Deliberately not stored:** hosts that were blocked by the list or by a keyword. A hit is
counted, never named. There is no "promotion" of visited listed hosts into the hosts block in
v1, because that table would be a browsing history of attempts.

### 2.6 Update path

v1: the list is refreshed with the app. Both products already auto-update; a maintainer runs
the script before a release and commits the outputs. The seam for v1.1 is in place:
`Matcher::new` takes the table as a byte slice, so a verified pack read from the data
directory replaces the embedded one without touching the matcher. The signed-manifest design
is in `research-blocklists.md` section 11.

---

## 3. Vision detection

### 3.1 When a check runs

The 2 s guard pass decides; the check itself runs outside the pass, one at a time
(`ShieldRuntime.vision_in_flight`). The decision is a pure function:

```rust
// ubiqx_shield::sampling
pub fn decide(ctx: &SampleCtx, mem: &SamplerMemory, now: DateTime<Utc>) -> SampleDecision;
pub enum SampleDecision { Skip(SkipReason), Check { risk: Risk } }
```

**Gates (any one skips, with the reason shown as the vision state):**

| Gate | Source |
|---|---|
| Shield off, or `vision_enabled` false | `ShieldConfig` |
| Consent missing or given for another provider (`consented_provider != settings.ai_provider`) | `ShieldConfig` |
| `!state.remote_allowed()` (covers `local_only`, onboarding, licence, AI health) | `state.rs:228` |
| `!classify::budget_allows(state)?` | `classify.rs:105` |
| No detector wired (`ShieldPorts.detector` is `None`) | deps |
| Screen Recording not granted (macOS); no window id; Wayland | `PermissionChecker`, `ForegroundWindow` |
| User idle ≥ 60 s | `IdleDetector` |
| Exempt window; app in `NEVER_CAPTURE_APPS` (password managers, keychain tools) or in the user's `vision_excluded_apps` | constants, `ShieldConfig` |
| Host verdict `Allowed`, or host in `sensitive.txt` (banks, payment, government, health portals, password vaults) or under `gov*`, `jus.br`, `mil` | matcher, data |
| Hourly or daily cap reached (`max_checks_per_hour` default 20, `max_checks_per_day` default 80) | `SamplerMemory` |

The shield does **not** honour the productivity tracker's switches (pause, private mode,
quiet hours, `vision_policy`, `vision_denied_apps`, `blocked_apps`, the incognito rule). Those
are one click away in the tray and would each be an off switch. The shield has its own
consent and its own exclusions. Private-browsing windows are checked; the wizard says so.

**Candidates and cadence** (constants in `sampling.rs`; the key is an 8-byte hash, kept in
memory only and gone at exit):

| Foreground | Key | First check after | Then | Stops |
|---|---|---|---|---|
| Browser, host `Unknown { None }` | host | 20 s in front | every 300 s | after 2 clean verdicts the host is trusted for this run |
| Browser, host `Unknown` with signal `Weak` or `Mixed`, or a block-grade word in the title, or a private-browsing title, or a `suggestive`/unsure verdict in the last 30 min | host | 5 s | every 45 s | never trusted |
| Browser without a URL (Firefox on macOS, Linux title without a host) | app id + title hash | 20 s (5 s with a title signal) | every 300 s | 2 clean for that title |
| Media and chat apps (`MEDIA_APPS`: VLC, mpv, IINA, QuickTime, Photos/Preview/image viewers, Telegram, Discord, WhatsApp) | app id | 30 s | every 300 s; every 900 s after 3 clean in a row | never trusted |
| Any other app | | never, unless its title carries a block-grade word (then as the high-risk row) | | |

An identical frame is not sent twice: when the SHA-256 of the captured JPEG equals the last
one sent for that key, the check is skipped and counted as clean.

**Cost** (estimates from `map-vision-ai.md` section 9, not measured): at 768 px on
`claude-haiku-4-5` about US$ 0.0009 per check. The daily cap of 80 bounds it at about
US$ 0.07 a day and US$ 2.20 a month; expected use is 20 to 40 checks a day. Calls are
recorded in the shared `ai_usage` ledger under a new `AiUsageKind::ShieldVision`
(`"shield_vision"`), so Settings shows what the shield spent, and they count against the same
monthly budget as classification. The wizard shows the estimate and the two caps.

### 3.2 What is captured and sent

- `ScreenCapturer::capture(CaptureTarget::Window(id), 768)`: the foreground window only,
  never a display. No window id means no check. JPEG quality 60 (existing constant).
- The bytes stay in memory: no file, no `screenshots` row, no thumbnail. They are dropped when
  the call returns. `screenshots::capture_for_block` is not used.
- The request carries the image and a fixed instruction. **No URL, no title, no app name, no
  user text.**
- Destination: the provider in `settings.ai_provider`, with `settings.models.vision`
  (`claude-haiku-4-5`, `gpt-5-mini`, `grok-4-1-fast-non-reasoning`, `ubi-fast`). `AiModels`
  gets no new field (it has no serde default and a new field would break stored settings).
- "IA do Ubi": the image transits the operator's proxy on its way to Anthropic. It is allowed
  only after a consent line that says exactly that. Changing provider later pauses vision
  until the user confirms the new one (`consented_provider`).
- With `UBIQX_LOG=trace` the clients log the whole request body, base64 image included
  (`trace!(vendor, body = %body, "request body")` at `crates/ubiqx-ai/src/client.rs:745` and
  `crates/ubiqx-ai/src/openai.rs:689`). The unit that owns `ubiqx-ai` replaces image data in
  those two trace lines with `<image: N bytes>` for every request kind, through one small
  pure function with a unit test. A shield screenshot must never reach a log file.

### 3.3 Prompt and response

System prompt (`ubiqx_ai::prompts::explicit_system_prompt()`, English, constant):

> You are a content-safety classifier inside a self-control app. The user installed it on
> their own computer and asked it to check their own screen. Look at the screenshot and
> classify what is visible. Do not describe the image. Do not identify people.
> `explicit`: pornography or explicit sexual content: visible genitals, sexual acts, or
> nudity presented for sexual arousal, photographic, drawn or animated, including as
> thumbnails.
> `suggestive`: sexualised but not explicit (lingerie, swimwear poses, partial nudity), or
> too small or blurred to judge.
> `none`: everything else, including medical or educational material, classical art,
> breastfeeding, sports, and text about sexuality without explicit images.
> Answer with the JSON object only.

User content: `LlmContent::Image { media_type, base64 }` then
`LlmContent::Text("Classify this screenshot.")`.

Schema (closed, all required, as both vendors' strict modes need):

```json
{ "type": "object",
  "properties": { "verdict":    { "type": "string", "enum": ["explicit", "suggestive", "none"] },
                  "confidence": { "type": "string", "enum": ["high", "medium", "low"] } },
  "required": ["verdict", "confidence"], "additionalProperties": false }
```

Request: `max_tokens = 200`, `json_schema = Some(schema)`,
`disable_thinking = thinking_disabled_for(&model)`, `usage_kind = AiUsageKind::ShieldVision`,
`timeout = Some(15 s)`. The engine wraps the call in `tokio::time::timeout(20 s)` because
retries are per client (4 attempts, 2 s to 30 s backoff), not per request.

Provider specifics: none in `explicit.rs`. `RoutingLlmClient` already produces the Anthropic
`/v1/messages` image block and the OpenAI/xAI `image_url` data URL. One edit elsewhere:
`reasoning_effort_for` in `crates/ubiqx-ai/src/openai.rs:176` gets the `ShieldVision` arm with
the same effort as `Vision`.

**Refusal is a verdict.** `Err(CoreError::AiRefused)` from `client.complete` is caught in
`LlmExplicitDetector::check` and returned as `ExplicitCheck { refused: true, .. }`. An
unknown verdict string or invalid JSON is `CoreError::Ai`. A truncated answer is retried once
with doubled `max_tokens`, as `vision.rs:105-112` does.

### 3.4 Thresholds and outcomes (`ubiqx_shield::sampling::outcome`)

| Model answer | Outcome | What happens |
|---|---|---|
| `explicit` + `high` | **Positive** | enforce (1.1), learn the host (2.5), store one event (`kind = block`, `source = vision`), show the window with "Fechei a janela, como você pediu. Nenhuma imagem foi guardada." and the buttons "Ok", "Não era isso", "Respirar 3 min" |
| `explicit` + `medium` | **Confirm** | a second capture 3 s later; `explicit` at `medium` or `high` again → Positive, otherwise Unsure |
| `explicit` + `low`; refused | **Unsure** | no enforcement; the key becomes high-risk for 30 min; soft card "Posso estar enganado, mas o que está na tela parece fugir do seu combinado. Quer que eu feche?" with "Fechar" and "Não era isso". At most one soft card per key per 30 min. |
| `suggestive` (any) | **Watch** | nothing shown; high-risk cadence for 30 min |
| `none` | **Clean** | clean counter for the key |

Before acting on Positive the engine re-reads the foreground: it acts only when the window id
equals the captured one and, where a host was known, the host is unchanged. Otherwise the
verdict is dropped. This is the guard against closing a tab the user has since switched to.

"Não era isso": the key is snoozed for 30 min and an event `false_positive` is stored. If the
host had just been learned, the learned row is removed at once (undoing the app's own guess
is not a weakening). At most `MAX_SNOOZES_PER_DAY = 3`; after that the button explains that
a permanent exception is added on the page, under the delay.

| Condition | Behaviour | Vision state shown |
|---|---|---|
| Timeout, network error, rate limit | no verdict; that key waits 300 s; 3 consecutive errors → no checks for 15 min. The shield keeps its own in-memory health and never writes the shared `AiHealth`, so a vision failure cannot pause classification. | `degraded` |
| No key, key rejected, `local_only`, budget spent | no checks | `off` with reason `no_ai`, `local_only` or `budget` |
| Offline | as timeout | `degraded` |
| Capture error "capture is blank" | skip, counts as nothing | unchanged |

### 3.5 What is stored

Per check: nothing but two in-memory counters. Per Positive, soft card or "não era isso": one
`shield_events` row with kind, source, action and time. Never the image, a hash of it, the
title or the URL. The only host ever written is a learned one, in `shield_domains`, because
it has to be blocked afterwards; the user sees it and can remove it.

The page shows: "Hoje: {n} verificações. Cada uma envia uma imagem da janela ativa (até
768 px) para {provider}. Nada é guardado."

---

## 4. Counselling

All copy comes from the reviewed 40-message bank in `research-counselling.md` section 7,
shipped as the i18n namespace `shield` (PT-BR and EN) with the keys of that table. No text
shown to the user is written by a model. The tone rules of that report's section 6 are
acceptance criteria for the copy review, including the "never say" table.

### 4.1 Enable wizard (consent)

Four steps on the Escudo page (ubiqX) or as the onboarding (standalone). Nothing is applied
before the last button.

| Step | Content | Backed by |
|---|---|---|
| 1 "O que é" | A barrier and a calm coach, for an adult on their own computer; not a treatment; what it cannot do (another device, a determined admin) | framing; competitor lesson on surveillance tools |
| 2 "Bloqueio" | Hosts list on/off; DNS filter choice with the disclosure of 1.3; "o sistema vai pedir sua senha de administrador"; launch at login | stimulus control |
| 3 "Olhar da IA" | On/off; provider by name; exactly what is sent and what is kept; cost estimate and caps; private windows are included; apps never checked; learned sites | privacy framing |
| 4 "Seu combinado" | One values sentence (optional), the disable delay (default 10 min), discreet messages on by default | ACT values; commitment devices |

### 4.2 The counselling window

A second floating window, label `shield`, route `#/shield-panel?id=<intervention id>` outside
`Gate`, capability file `capabilities/shield.json`. 480×340, always on top, on all
workspaces, shown without taking focus. It does not reuse the `intervention` window (460×188,
closes after 7 s, falls back to a YouTube sample).

| Card | Trigger | Text (keys) | Buttons |
|---|---|---|---|
| Intercept | list, keyword, user or learned hit | discreet (default): `block.discreet`, "Página bloqueada pelo UBI."; detailed: rotation of `block.default`, `block.breathe`, `block.choice`, `block.next`, plus the user's own values sentence | "Ok", "Respirar 3 min", "Ajuda" |
| Intercept, third attempt in an episode | 1.1 rule 3 | `block.repeat` | "Fazer a onda de 3 min", "Ok" |
| Vision positive | 3.4 | `vision.closed`, `vision.added` when learned | "Ok", "Não era isso", "Respirar 3 min" |
| Vision soft / app | 3.4 Unsure; positive in a non-browser app | `vision.soft` | "Fechar", "Não era isso" |
| Drift | 1.4 | calm restore line | "Restaurar", "Agora não" |
| Pending disable ready | 5.2 | "O tempo de espera terminou. Ainda quer desligar?" | "Manter proteção", "Desligar" |

The discreet intercept closes itself after 12 s when untouched; every other card stays until
answered. The OS-notification fallback (no shell window) uses the neutral title "UBI" and
`block.discreet`; a blocked host never appears in the notification centre.

### 4.3 Flows

| Flow | Entry points | Steps | Stored | Backed by |
|---|---|---|---|---|
| **Urge surf** (3 min) | any card; the page; the tray; the panic flow | six screens of 30 s: `surf.start`, `surf.body`, `surf.breath`, `surf.wave`, `surf.thought`, then `surf.rate` (0 to 10, optional) and `surf.end` | `urge_surf` with the rating | mindfulness-based relapse prevention, ACT; framed as practice, with no claim that the urge shrinks |
| **Panic button** | tray item "Preciso de um respiro"; button on the page | `panic.open` → `panic.options`: surf / step away 10 min (`panic.away`, a countdown, no lock) / talk to someone (crisis card) | `panic` | delay and distract; crisis path |
| **Lapse** | "Aconteceu" on the page, one tap | `lapse.first` → optional trigger chips (cansaço, tédio, estresse, solidão, outro: `lapse.learn`) → optional if-then plan (`lapse.plan`) → `lapse.resume` | `lapse` with the optional trigger tag | abstinence-violation effect, self-compassion; implementation intentions |
| **Check-in** | a card on the page, never a notification | `checkin.daily`: four moods, one tap; on "difícil" the HALT chips (`checkin.halt`) | `checkin` with the mood | HALT as a prompt, not as a fact |
| **Values and plans** | the page | one sentence (`checkin.values`), up to three "se… então…" plans (`checkin.ifthen`); shown back on detailed intercepts and in the panic flow (`panic.reason`) | in `ShieldConfig` | ACT; Gollwitzer and Sheeran 2006 |
| **Progress** | the page | "Nos últimos 30 dias, {days} foram do jeito que você queria" (days with the shield on and no lapse the user reported). The streak counter exists, is off by default, and after a lapse shows `streak.after_reset`. Milestones `streak.day1/week/month` appear on the page only. | derived from events | Silverman and Barasch 2023: broken streaks lower engagement |
| **Professional help** | card on the page and in the window | shown when, in the last 7 days: ≥ 5 lapses, or ≥ 10 override attempts (snoozes, disable requests, removals requested), or ≥ 2 "muito difícil" check-ins. At most once every 7 days. `help.pattern`, `help.how`, `help.limits`. The thresholds are constants in `help.rs` and are product heuristics, not clinical cut-offs. | `help_shown` | ICD-11 description; only 4 to 10% of people at risk ever seek treatment |
| **Help page / crisis card** | one step from every shield screen | resources from `crates/ubiqx-shield/data/resources.json`: CVV 188 and `cvv.org.br/chat`, SAMU 192, CAPS, "procure um psicólogo com registro ativo no CRP", `findahelpline.com`. `panic.crisis` is the only message that carries a phone number and it reads it from this file. | | referral research |

Every number and URL in `resources.json` carries `"verified": false` until a person opens the
live page; the release checklist blocks on it. DASA, SAA Brazil and AISEP are not listed.

Not in v1: the evening prompt (`checkin.evening`, needs a pattern detector), structured
programmes, questionnaires, any AI-written reply.

---

## 5. Commitment: turning protection off

### 5.1 Model

`ShieldConfig` is not part of `Settings`. The UI's whole-object, last-writer-wins settings
save therefore cannot flip it, and neither can "restore defaults". It changes only through

```rust
EngineHandle::shield_request(change: ShieldChange) -> CoreResult<ShieldView>
```

and `ubiqx_shield::commitment::classify(&config, &change) -> Effect` decides:

| `Effect::Now` (strengthening or neutral) | `Effect::After(delay)` (weakening) |
|---|---|
| `Enable`, `AddBlock`, `RemoveAllow`, `VisionOn`, `IncludeApp`, `RaiseDelay`, `SetResolver(Some)`, `SetCaps`, `SetPrefs`, `SetValues`, `SetPlans`, removal of a learned host through "não era isso" | `Disable`, `RemoveBlock`, `AddAllow`, `RemoveLearned`, `VisionOff`, `ExcludeApp`, `LowerDelay`, `SetResolver(None)`, `HostsOff` |

Before the shield is enabled for the first time everything is `Now`: the wizard has no delay.

### 5.2 The delay

- `disable_delay_secs`: default 600, range 0 to 86,400. Raising is immediate; lowering takes
  effect after the delay in force (the Plucky model).
- A weakening request becomes a `PendingChange { id, change, requested_at, ready_at }` in
  `ShieldConfig.pending` (at most 20). The page shows a countdown, "Manter proteção"
  (cancels, one tap, always available) and the urge-surf button.
- At `ready_at` the change is **ready**, not applied. The user confirms it within 24 h
  (card or page), then it is applied; unconfirmed, it expires. `Disable` then asks for the
  OS prompt to remove L2 and L3 (`remove --scope all`); declining the prompt leaves the
  system layers in place and the page says so.
- With a delay of 0 the request is applied after one confirmation dialog built on the existing
  `DisableWarnings` component ("keep the protection" is the primary button).
- Each request is stored as `disable_requested`; cancelling as `disable_cancelled`. Requests
  count toward the professional-help prompt.

Backed by: soft commitment devices (Bryan, Karlan and Nelson 2010); the Plucky and Freedom
friction models; delay-and-distract.

### 5.3 "Restaurar minha rede"

One undelayed action for breakage (captive portal, corporate VPN, a provider outage): it runs
`remove --scope resolver`, which restores DNS and flushes caches and touches nothing else.
L1, L2 and L4 stay on. It is stored as `network_restore`, shown on the page as
`resolver: network_restored` with "Religar", and does not count as an override. The hosts
block cannot break connectivity to names it does not list, so it needs no such exit.

### 5.4 What is deliberately not built, and why

| Not built | Why |
|---|---|
| Accountability partner, reports to another person | No controlled evidence; coercion and privacy risk; it is the mechanism that got comparable apps removed from Google Play. Never a way to watch another adult. |
| Password held by someone else, whole-computer lock, "can't quit" | Lockout and support risk; invites a control relationship; an admin can undo it anyway. |
| Punishment for tampering (extending the block) | Conflicts with the non-shaming rule. Drift gets a calm restore card. |
| Privileged background helper that re-applies silently | Needs signing (macOS `SMAppService`), a service on Windows, a systemd unit; a security surface. v2, opt-in. |
| Defence against editing the SQLite file or killing the process | The user is the machine's admin. The product is friction and honesty about its state, not a lock. |
| Hiding the app, its process or its hosts block | The tool is visible by design. |
| Browser policies (DoH off, SafeSearch), SafeSearch pinning in hosts | Many per-browser locations, the "Managed by your organization" banner, mandatory policy on macOS unproven, a stale SafeSearch address breaks Google. In v1 the hole they close is covered by L1 (the guard reads the URL whatever resolver the browser uses) and L4. First item of v1.1, as an extra section of the same plan and helper. |

---

## 6. Data model

### 6.1 Configuration (`ubiqx_core::shield::ShieldConfig`)

One JSON document in the single row of `shield_state`. `#[serde(default)]` on the struct, so
new fields need no migration and `{}` deserialises to the default (unit test).

| Field | Type | Default | Changed by |
|---|---|---|---|
| `enabled`, `enabled_at` | `bool`, `Option<DateTime<Utc>>` | `false`, `None` | `Enable` / `Disable` |
| `hosts_enabled` | `bool` | `true` | wizard; `HostsOff` (delayed) |
| `resolver` | `Option<ResolverId>` | `None` until the wizard | `SetResolver` |
| `network_restored` | `bool` | `false` | "Restaurar minha rede" / "Religar" |
| `vision_enabled` | `bool` | `false` | `VisionOn` / `VisionOff` |
| `consented_provider` | `Option<AiProvider>` | `None` | consent step |
| `learn_domains` | `bool` | `true` | `SetPrefs` |
| `vision_excluded_apps` | `Vec<String>` | empty | `ExcludeApp` (delayed) / `IncludeApp` |
| `max_checks_per_hour`, `max_checks_per_day` | `u32` | 20, 80 (clamped 1..=120, 1..=1000) | `SetCaps` (immediate: cost is the user's call) |
| `disable_delay_secs` | `u32` | 600 (0..=86,400) | `RaiseDelay` / `LowerDelay` |
| `pending` | `Vec<PendingChange>` | empty | engine only |
| `discreet`, `show_streak` | `bool` | `true`, `false` | `SetPrefs` |
| `values_sentence`, `plans` | `String` (≤ 280 chars), `Vec<IfThenPlan>` (≤ 3) | empty | `SetValues`, `SetPlans` |
| `last_help_shown_at` | `Option<DateTime<Utc>>` | `None` | engine only |

`Settings` gets **no** shield field. The shield reads `ui_language`, `ai_provider`,
`models.vision`, `local_only`, `onboarding_done` and the monthly budget from it and writes
nothing to it.

### 6.2 Tables (migration V3, `crates/ubiqx-storage/src/migrations.rs`)

```sql
CREATE TABLE shield_state (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    config     TEXT NOT NULL,                    -- ShieldConfig as JSON
    updated_at INTEGER NOT NULL
);
CREATE TABLE shield_domains (
    domain     TEXT PRIMARY KEY NOT NULL,        -- normalised host
    kind       TEXT NOT NULL,                    -- block | allow
    origin     TEXT NOT NULL,                    -- user | vision
    created_at INTEGER NOT NULL
);
CREATE TABLE shield_events (
    id     TEXT PRIMARY KEY NOT NULL,
    at     INTEGER NOT NULL,
    kind   TEXT NOT NULL,
    source TEXT,                                 -- list | keyword | tld | user | learned | vision
    action TEXT,                                 -- tab_closed | tab_blanked | asked | app_quit | notified
    detail TEXT                                  -- small tag or number, never a host, title or URL
);
CREATE INDEX idx_shield_events_at ON shield_events(at);
```

`shield_events.kind`: `block`, `vision_ask`, `false_positive`, `urge_surf` (detail = rating),
`panic`, `lapse` (detail = trigger tag), `checkin` (detail = mood), `enabled`, `disabled`,
`disable_requested`, `disable_cancelled`, `network_restore`, `drift` (detail = layer),
`help_shown`. `block` rows carry `detail = attempts`.

The existing `interventions` table is not written by the shield and is not altered, so no
existing enum gains a variant and no older reader breaks. `kv` is not used (it is wiped as a
cache). Mechanical changes: `M::up(V3)` appended; the three hard-coded `2`s become `3`
(`migrations.rs:273`, `tests/repos.rs:234`, `:242`). Both products use the same migration
list in separate database files, so neither can leave the other with an unknown schema.

### 6.3 Events to the UI

Two new `EngineEvent` variants (`crates/ubiqx-core/src/model.rs:1055`):

```rust
Shield { status: ShieldStatus },                         // any layer or pending change moved
ShieldIntervention { intervention: ShieldIntervention }, // a card to show
```

```rust
pub struct ShieldStatus {
    pub overall: ShieldOverall,          // off | protected | reduced | pending_disable
    pub guard: GuardState,               // off | on | limited { reason }
    pub hosts: HostsState,               // 1.2
    pub resolver: ResolverState,         // 1.3
    pub vision: VisionState,             // off { reason } | on | degraded
    pub pending: Vec<PendingChange>,
}
pub struct ShieldIntervention {
    pub id: Id, pub at: DateTime<Utc>,
    pub card: ShieldCard,                // intercept | intercept_repeat | vision_closed | vision_ask | drift | pending_ready
    pub message_key: String,             // i18n key in the `shield` namespace
    pub app_name: Option<String>,        // only for vision_ask in a non-browser app; never stored
    pub learned: bool,
}
```

The last 20 interventions live in memory (`ShieldRuntime`) for the window's
`shield_intervention(id)` call. An unknown id renders the neutral discreet card, not a sample.
The CLI's exhaustive match (`crates/ubiqx-cli/src/main.rs:101-152`) gets two arms.

### 6.4 Retention, export, delete

| Data | Kept | Removed by |
|---|---|---|
| Screenshots taken for vision | never written | |
| Per-check memory (keys, clean counts, caps) | process lifetime | exit |
| `shield_events` | 400 days (pruned daily by the verify loop) | "Apagar histórico do Escudo" (one confirmation, immediate, followed by the same checkpoint and `VACUUM` as the existing wipe); ubiqX's "Apagar todos os dados" (`DELETE FROM shield_events` joins `WIPE`, `[&str; 9]` becomes 10) |
| `shield_state`, `shield_domains` | until removed | kept by "Apagar todos os dados", like `focus_targets`: they are configuration and a commitment, and wiping them would be an undelayed off switch. "Apagar tudo do Escudo" exists only while the shield is off. |
| System state file, hosts block, DNS | until `remove` | disable flow; documented manual removal |

Export: `EngineHandle::shield_export()` writes `<data_dir>/exports/shield-<date>.json`
(config, domains, events) on request. ubiqX's existing `export_data` does **not** include
shield data, so a productivity export shared with someone else never carries it.

---

## 7. Code architecture

### 7.1 Layering

```
ubiqx-core      shield.rs    types + ports (no logic)          product.rs  Product, ProductIdentity
ubiqx-shield    NEW crate    pure logic + embedded data (depends on ubiqx-core, sha2, serde, chrono, url)
ubiqx-storage   repos/shield.rs + V3
ubiqx-ai        explicit.rs  LlmExplicitDetector, FakeExplicitDetector
ubiqx-platform  sysblock/    SystemShield adapter, helper, elevation, three OS back-ends, mock
ubiqx-engine    shield/      runtime, guard pass, vision job, system orchestration, handle methods
ubiqx-app       product-aware composition
apps/desktop    src-tauri: shield.rs (commands, presenter, window), tray per product
                src: pages, components, i18n namespace, standalone entry
```

### 7.2 `ubiqx-core`

`crates/ubiqx-core/src/shield.rs` (new): `ShieldConfig`, `ShieldChange`, `PendingChange`,
`IfThenPlan`, `ResolverId`, `ShieldDomain`, `DomainKind`, `DomainOrigin`, `ShieldEvent`,
`ShieldEventKind`, `ShieldStatus` and its parts, `ShieldIntervention`, `ShieldCard`,
`ExplicitCheck`, `ExplicitVerdict`, `Confidence`, `SystemPlan`, `SystemObservation`,
`SystemStateFile`, `ApplyOutcome`, `RemoveScope`, `SystemCapabilities`, and four ports:

```rust
/// Machine-level blocking. `observe` is unprivileged and cheap; `apply` and `remove` raise
/// the OS consent prompt and block until the helper exits.
pub trait SystemShield: Send + Sync {
    fn capabilities(&self) -> SystemCapabilities;          // hosts / resolver available here, and why not
    fn observe(&self) -> CoreResult<SystemObservation>;    // hosts text, per-interface DNS, drop-in, state file
    fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyOutcome>;   // Applied | Partial | Declined | Unavailable(reason)
    fn remove(&self, scope: RemoveScope) -> CoreResult<ApplyOutcome>;
}

#[async_trait]
pub trait ExplicitDetector: Send + Sync {
    async fn check(&self, image: &EncodedImage, model: &str) -> CoreResult<ExplicitCheck>;
}

/// Shows the counselling window; `Ok(false)` falls back to a neutral OS notification.
pub trait ShieldPresenter: Send + Sync {
    fn show(&self, intervention: &ShieldIntervention) -> CoreResult<bool>;
}

pub trait ShieldRepo: Send + Sync {
    fn load_config(&self) -> CoreResult<ShieldConfig>;
    fn save_config(&self, config: &ShieldConfig) -> CoreResult<()>;
    fn list_domains(&self) -> CoreResult<Vec<ShieldDomain>>;
    fn upsert_domain(&self, domain: &ShieldDomain) -> CoreResult<()>;
    fn remove_domain(&self, domain: &str) -> CoreResult<()>;
    fn insert_event(&self, event: &ShieldEvent) -> CoreResult<()>;
    fn bump_attempts(&self, event_id: &str, attempts: u32) -> CoreResult<()>;
    fn list_events(&self, since: DateTime<Utc>) -> CoreResult<Vec<ShieldEvent>>;
    fn delete_events(&self) -> CoreResult<()>;
    fn prune_events(&self, before: DateTime<Utc>) -> CoreResult<u64>;
}
```

`SystemObservation` is raw (text and lists). Turning it into `HostsState` / `ResolverState`
is `ubiqx_shield::plan::assess`, so the adapter has no decisions in it.

Other core edits: `EngineEvent` variants; `AiUsageKind::ShieldVision` with `as_str`/`parse`;
`focus.rs` exemption constants; `product.rs`:

```rust
pub enum Product { Ubiqx, Shield }
pub struct ProductIdentity {
    pub product: Product,
    pub bundle_id: &'static str,       // ai.ubiqx.app            | ai.ubiqx.leme
    pub display_name: &'static str,    // ubiqX                   | Leme
    pub keyring_service: &'static str, // ai.ubiqx                | ai.ubiqx.leme
    pub db_file: &'static str,         // ubiqx.db                | leme.db
    pub feed_product: &'static str,    // ubiqX                   | Leme
    pub productivity: bool,            // true                    | false
}
impl Product { pub fn identity(self) -> &'static ProductIdentity; pub fn from_bundle_id(id: &str) -> Product; }
```

### 7.3 `crates/ubiqx-shield` (new, pure)

| Module | Public surface | Tested by |
|---|---|---|
| `list.rs` | `HashTable::parse(&[u8])`, `contains(&str)`, `key_of(&str) -> u64`, `EMBEDDED_CORE` | hashing vector, bad magic, unsorted, wrong count, canaries |
| `matcher.rs` | 2.4 | precedence table, suffix walk, IDN, IP literal |
| `heuristics.rs` | `host_signal`, `title_signal`, `fold_lookalikes` | block, weak and trap tables of 2.3 |
| `hosts.rs` | `render_block(names) -> String`, `splice(existing, Option<&str>) -> String`, `inspect(text) -> BlockInspection`, `is_valid_hostname`, `names_for(tier, domains) -> Vec<String>` | round trips with CRLF and LF files, foreign content preserved byte for byte, cap, protected suffixes, idempotence |
| `resolvers.rs` | `RESOLVERS`, `ResolverSet { v4, v6 }`, `lookup(ResolverId)` | table sanity (parse as IP literals) |
| `plan.rs` | `build_plan(&ShieldConfig, &[ShieldDomain], owner) -> SystemPlan`, `validate(&SystemPlan)`, `assess(&ShieldConfig, &SystemPlan, &SystemObservation) -> (HostsState, ResolverState)`, `merge_previous`, `restore_actions` | every row of the state tables in 1.2 and 1.3; double apply; "user changed it meanwhile" |
| `commitment.rs` | `classify`, `request`, `tick(&mut ShieldConfig, now) -> Vec<Ready>`, `confirm`, `cancel` | each change kind; raise/lower delay; expiry after 24 h; delay 0 |
| `sampling.rs` | `decide`, `record`, `outcome`, constants | each gate, each cadence row, caps across hour and day boundaries, snooze limit, trust after two clean |
| `episode.rs` | `EpisodeTracker::observe(hit, now) -> EpisodeStep` (`Enforce`, `EnforceAndShow(card)`, `Wait`) | confirm passes, gap, third attempt |
| `messages.rs` | `intercept_key(discreet, rotation)`, `notification_text(lang)`; the Rust-side copies of the few lines the engine itself needs (notification fallback, tray) in PT-BR and EN | no host in any output; parity of the two languages |
| `help.rs` | `help_due(&[ShieldEvent], last_shown, now) -> bool`, `RESOURCES` (parsed `resources.json`) | thresholds, once per 7 days |
| `progress.rs` | `days_your_way(&[ShieldEvent], now, 30)`, `streak` | lapses, days off, local-day boundaries with an injected offset |

### 7.4 `ubiqx-storage`

`repos/shield.rs` on the `repos/focus.rs` template (column macro, `concat!` SQL constants,
`row_to_*` by name, in-file tests); `impl ShieldRepo for SqliteStore`; V3; `WIPE`.

### 7.5 `ubiqx-ai`

`explicit.rs`: `LlmExplicitDetector::new(llm)`, `build_request(image, model)`,
`explicit_schema()`, `parse_explicit(text)`, refusal mapping (3.3).
`prompts.rs`: `explicit_system_prompt()`. `fake.rs`: `FakeExplicitDetector` (scripted queue
of `CoreResult<ExplicitCheck>`, call counter) and `ScriptedLlmClient::push_error(CoreError)`,
which the fake cannot do today. `openai.rs:176`: the `ShieldVision` arm. `client.rs:745`,
`openai.rs:689`: image bodies out of trace lines.

### 7.6 `ubiqx-platform`: `src/sysblock/`

| File | Content | Compiled on | Tested on Linux |
|---|---|---|---|
| `mod.rs` | `NativeSystemShield { os: Box<dyn OsBackend>, runner: Arc<dyn Runner>, fs: Arc<dyn Fs>, elevator: Arc<dyn Elevator>, exe, plan_dir }` implementing `SystemShield`; traits `Runner` (`run(program, args) -> Output`), `Fs` (read, atomic write, remove), `Elevator` (`run_elevated(exe, args) -> ElevatedExit`) | all | yes, with fakes |
| `macos.rs` | `MacBackend`: argv builders for `networksetup`, parsers for its two outputs, flush commands, paths | **all** (no `cfg`) | yes: recorded sample outputs as fixtures |
| `windows.rs` | `WindowsBackend`: the fixed PowerShell scripts as constants, the JSON parser for their output, argv builders, paths | **all** | yes |
| `linux.rs` | `LinuxBackend`: drop-in text, `systemctl` argv, detection | **all** | yes |
| `helper.rs` | `main_if_requested(args)`, verb parsing, `run_apply`, `run_remove` over `OsBackend` + `Runner` + `Fs`; state-file read/write | all | yes: whole apply → observe → remove cycles against a temp directory and a fake runner, for each of the three back-ends |
| `elevate.rs` | `sh_quote`, `applescript_quote`, `osascript_argv`, `pkexec_argv`, exit-code mapping (pure); `SystemElevator` whose Windows arm is the only `cfg(windows)` FFI (`ShellExecuteExW`) | pure parts all | pure parts yes |
| `../mock.rs` | `MockSystemShield` (call log, scripted observation and outcomes), in `PlatformServices::scripted()` | all | yes |

`PlatformServices` gets one field, `system_shield: Arc<dyn SystemShield>`, set in the three
`services()` functions and in `scripted()`. `PlatformServices::native_for(identity, data_dir)`
passes the keyring service name; `native_in` keeps today's behaviour. `SELF_STEMS` gains the
standalone's stems.

Because the three back-ends are plain data-in, data-out modules, a Linux `cargo test` executes
the macOS and Windows command construction and parsing. What it cannot execute is listed in
9.4.

### 7.7 `ubiqx-engine`

`deps.rs`: two new fields on `EngineDeps` (`shield: ShieldPorts`,
`identity: &'static ProductIdentity`), so only the five `EngineDeps` literals change:

```rust
pub struct ShieldPorts {
    pub system: Arc<dyn SystemShield>,
    pub presenter: Arc<dyn ShieldPresenter>,
    pub detector: Option<Arc<dyn ExplicitDetector>>,
    pub repo: Arc<dyn ShieldRepo>,
}
impl ShieldPorts { pub fn inert() -> Self }   // NoSystemShield, LogShieldPresenter, no detector, MemoryShieldRepo
```

`NoSystemShield`, `LogShieldPresenter` and `MemoryShieldRepo` live in `deps.rs` next to
`LogInterventionPresenter` and `NoLicenseServer`. `PlatformPorts`, `AiPorts` and `Repos` are
untouched (they are full literals in many tests). `identity` gives the plan its `owner` and
gives `UpdateChecker` the product name it must find in the feed
(`crates/ubiqx-engine/src/update.rs`, `crates/ubiqx-core/src/update.rs`: a feed whose
`product` differs is ignored; today nothing checks it).

`src/shield/`:

| File | Content |
|---|---|
| `mod.rs` | `ShieldRuntime { config, matcher, episodes, sampler, vision_in_flight, vision_errors, interventions, status, last_drift_notice }` behind `EngineState.shield: Mutex<ShieldRuntime>`; `status(state)`; every function behind the handle; `record_user_event` |
| `guard.rs` | `pass_once(state) -> CoreResult<GuardOutcome>`: gate, foreground, URL, `Matcher::verdict`, `EpisodeTracker`, enforcement, event, presenter, and `GuardOutcome.vision_job` from `sampling::decide` |
| `vision.rs` | `async run(state, job)`: capture on the blocking pool, `detector.check` under `timeout(20 s)`, `sampling::outcome`, re-check of the foreground, enforce, learn, present |
| `system.rs` | `apply(state)`, `remove(state, scope)`, `verify_once(state)` (observe → `plan::assess` → status → drift card → `commitment::tick` → daily prune) |

`engine.rs`: `LoopConfig` gains `shield_every` (2 s), `shield_verify_every` (120 s) and
`productivity` (`true`). Two loops, both skipping their first tick like `focus_loop`:

```rust
async fn shield_loop(state, cancel, every)         // pass_once on the blocking pool; spawns vision::run when a job is returned and none is in flight
async fn shield_verify_loop(state, cancel, every)  // verify_once on the blocking pool
```

With `productivity == false`, `start_with` does not start the sampler and does not spawn
`classify_loop`, `reports_loop`, `nudges_loop`, `retention_loop` or `focus_loop`. It still
spawns the tracker task (idle, so `EngineHandle::shutdown` keeps its wait), `update_loop`,
`license_loop` and the two shield loops.

`tracker.rs`: the redaction of 1.1. `focus.rs`: the standalone's ids join
`EXEMPT_APP_IDS_WINDOWS` and `EXEMPT_APP_IDS_LINUX`.
`state.rs`: the runtime field, restored in `new` from `ShieldRepo`.

`handle.rs`, section `// -- Shield --`:

| Method | IPC command | Purpose |
|---|---|---|
| `shield_view()` | `shield_view` | status, config, counts for today, capabilities, vision cost line |
| `shield_enable(EnableOptions)` | `shield_enable` | wizard's last button: save config, apply L2/L3 (OS prompt) |
| `shield_request(ShieldChange)` | `shield_request` | 5.1 |
| `shield_confirm_pending(id)` / `shield_cancel_pending(id)` | same names | 5.2 |
| `shield_apply_system()` | `shield_apply_system` | "Aplicar", "Restaurar", "Religar" |
| `shield_restore_network()` | `shield_restore_network` | 5.3 |
| `shield_domains()` | `shield_domains` | the three lists of 2.5 |
| `shield_support()` | `shield_support` | values, plans, progress, check-in state, `help_due`, resources |
| `shield_record(UserEvent)` | `shield_record` | lapse, check-in, surf, panic, false positive, card answered |
| `shield_intervention(id)` | `shield_intervention` | payload for the floating window |
| `shield_close_app(intervention_id)` | `shield_close_app` | the "Fechar {app}" button |
| `shield_erase_history()`, `shield_reset()`, `shield_export()` | same names | 6.4 |
| `shield_test_intervention()` | `shield_test_intervention` | preview card, records nothing |

Calls that elevate (`shield_enable`, `shield_apply_system`, `shield_restore_network`, a
confirmed `Disable`) block until the OS prompt is answered; the Tauri commands run them
through the shell's existing `blocking` helper.

`delete_all_data` clears the in-memory episode and sampler state and leaves the config.
`shutdown` needs nothing from the shield: nothing it applied depends on the process.

### 7.8 `ubiqx-app` and the shell

`crates/ubiqx-app/src/lib.rs`: `AppConfig.product: Product` (default `Ubiqx`) and
`AppConfig.shield_presenter`; database file and default data directory from the identity;
`PlatformServices::native_for`; `ShieldPorts` built from `platform.system_shield`, the store
and, in `build_ai`'s `Remote` arm, `LlmExplicitDetector::new(llm.clone())` (`Fake` arm:
`FakeExplicitDetector`); `LoopConfig { productivity: identity.productivity, .. }`; for
`Product::Shield`, `tracking_enabled` is forced to `false` at start. `build_ai` still wires
the classifier and report writer in the standalone build, unused, because `key_health`
(`state.rs:256`) reports `NotConfigured` without them; this avoids touching the AI-health code.

`apps/desktop/src-tauri/src/`:

- `lib.rs`: `sysblock::helper::main_if_requested` first; `Product::from_bundle_id(&context.config().identifier)`;
  product passed to `AppConfig`; tray built per product; the shield window; quit confirmation
  when the shield is on.
- `shield.rs` (new): the 16 commands, `TauriShieldPresenter` (window label `shield`, built
  like `intervention` at `lib.rs:246-326` with the size and behaviour of 4.2).
- `commands.rs`: the names added to `generate_handler!`.
- `capabilities/shield.json`: `core:default`, window close/hide/set-focus/start-dragging for
  the label `shield`.
- Tray, ubiqX: one new item "Preciso de um respiro" (visible while the shield is on) and a
  status line. Tray, standalone: "Abrir", status line, "Preciso de um respiro", "Verificar
  atualizações", "Sair". No pause, private mode, snooze or report items.

Frontend (`apps/desktop/src/`):

| File | Content |
|---|---|
| `lib/shield.ts` | TypeScript mirrors of the shield types and the `shieldIpc` wrappers |
| `lib/shieldMock.ts` | browser/test implementation of every shield command, with the commitment timer and scripted statuses; `lib/mock.ts` delegates `shield_*` to it |
| `lib/store.ts` | `shield` slice (`view`, `loadShield`), `applyEvent` cases for `shield` and `shield_intervention` |
| `i18n/messages/shield.ts` | namespace `shield`, PT-BR and EN, registered in `NAMESPACES`; covered by `parity.test.ts` |
| `pages/Shield.tsx` | status header (four layer rows with the states of section 1), pending-change banner, wizard entry, tabs "Proteção", "Apoio", "Listas", "Ajuda" |
| `pages/ShieldPanel.tsx` | the floating window: the cards of 4.2, urge surf, panic |
| `components/shield/` | `EnableWizard`, `LayerRow`, `PendingBanner`, `DomainLists`, `UrgeSurf`, `PanicFlow`, `LapseFlow`, `CheckIn`, `ValuesAndPlans`, `Progress`, `HelpCard`, `ResourceList`, `VisionConsent` |
| `App.tsx`, `components/layout/Sidebar.tsx` | route `/shield`, route `/shield-panel` outside `Gate`, nav item "Escudo" |

Wording rule for the copy: Settings › Privacidade already uses "bloqueados" for "never
recorded". The shield says "sites barrados pelo Escudo" and never reuses that label.

### 7.9 The standalone product

Same crates, same shell crate, same Vite project; a second configuration.

| Piece | How |
|---|---|
| Rust | no `cfg`, no Cargo feature. The product is read at start from the bundle identifier compiled into the Tauri context. One clippy and one test run cover both. |
| Tauri config | `apps/desktop/src-tauri/tauri.shield.conf.json`, merged with `tauri build --config`: `productName`, `identifier` (`ai.ubiqx.leme`), `mainBinaryName`, window title, `build.beforeBuildCommand` (`pnpm build:shield`), `build.frontendDist` (`../dist-shield`), `bundle.icon`, updater endpoint |
| Frontend | `VITE_PRODUCT=shield`: `main.tsx` renders `ShieldApp` in place of `App`. `ShieldApp.tsx` has its own route table and `components/shield/ShieldShell.tsx` (nav: Proteção, Apoio, Listas, Ajuda, Ajustes). `pages/ShieldOnboarding.tsx` = language, the wizard of 4.1, and the AI key step built from the already exported `ProviderPicker`, `ApiKeyForm` and `LicenseKeyForm` (`pages/Settings.tsx:306,455,540`). `pages/ShieldSettings.tsx` = AI provider and key, vision model, monthly budget, language, updates, licence, data, about. |
| Data | own data directory (from the identifier), own `leme.db`, own keyring service. Installing both products shares nothing except the system marker and state file (1.5). |
| Update feed | own tag (`continuous-leme`) and feed URL stamped at build through the existing `UBIQX_UPDATE_FEED_URL`; the product check of 7.7 keeps either app from ever installing the other's build |
| Release | `scripts/publish-release.mjs`, `scripts/app-version.mjs` and the `ci.yml` globs take a product parameter; artifact names start with the product name so the ubiqX publish step never picks them up |
| `Info.plist` | shared file. Its two usage strings are reworded so they are true for both products ("…para saber qual site está aberto", "…para ler a janela ativa"), which avoids depending on a per-config plist override [U] |

**What the standalone app does not run:** the sampler thread, activity blocks, the
classifier, daily and monthly reports, productivity nudges, screenshot retention, the focus
guard and focus sessions, and the pages Hoje, Linha do tempo, Revisão, Relatórios,
Categorias, Insights and Foco. The tables exist, empty. It does run the shield loops, the
update check and the licence check.

**Licence and plans.** No change to `license.rs`, the proxy or the claims in v1:

- Blocking (L1, L2, L3), counselling and commitment are never gated. A lapsed subscription
  must never remove a block.
- AI vision needs what AI needs today: the user's own Anthropic, OpenAI or xAI key, or a
  valid `monthly_managed` licence for "IA do Ubi". `LICENSE_ENFORCEMENT` stays `Soft`.
- An existing ubiqX key is accepted by the standalone app as it is. Selling the standalone
  product at its own price needs a `Plan` variant or a product claim; that decision is the
  owner's (section 11) and does not block v1.

---

## 8. Naming and branding

Recommendation, given `research-counselling.md` section 5:

| Where | Name | Reason |
|---|---|---|
| Code: crate, module, settings, tables, IPC, system marker, process and bundle ids | `shield` (`ubiqx-shield`, `shield_*`, `ai.ubiqx.leme`) | NOFAP is a registered US mark (Classes 41, 42, 25, 35) of an owner that litigates, with 2026 filings in the UK and India; identifiers leak into logs, paths, package metadata and a public repository. "Fap" also commits the product to a masturbation-abstinence stance it must not take. |
| Module inside ubiqX | **Escudo do UBI** / **UBI Shield** | "Guarda"/"Guard" would collide with the existing focus guard in code and docs |
| Standalone product | **Leme, by ubiqX** (subtitle "Bloqueio de pornografia e apoio para mudar o hábito") | the report's preferred candidate; **not cleared**: needs an INPI and USPTO search and a domain check before any public use |
| Working title | `nofap` stays only as the name of this research folder and of the task | it is what the owner called the request |

If the owner decides to keep "nofap" as the module's name, the change is a mechanical rename
of the crate, the module and the IPC prefix; nothing in the architecture depends on the
word. The recommendation against showing it to users stands either way.

Guard rail: a test (`crates/ubiqx-shield/tests/naming.rs` plus a Vitest case) fails when the
string "nofap" appears, in any letter case, in an i18n bundle, a Tauri config, a
`Cargo.toml` package name or description, or `resources.json`. The user-visible product name
appears in exactly two places (`ProductIdentity.display_name`, `tauri.shield.conf.json`) and
the i18n key `shield.product_name`, so renaming is a three-line change.

Copy also never says "vício", "viciado", "recaída", "limpo", "dopamina", "reboot" or a
success rate; the Vitest parity suite gets a forbidden-words list for the `shield` namespace.

---

## 9. Test plan

### 9.1 Unit tests (pure, run by `cargo test` on Linux)

`ubiqx-shield` is the bulk of the logic and has no I/O, so its tests are the specification.
The per-module list is the right-hand column of the table in 7.3. Cross-cutting cases:

- **Embedded data:** `core.bin` parses, is sorted and unique, has the count in
  `core.meta.json`; every name in `canary-block.txt` is `Blocked(List)`; every name in
  `canary-allow.txt` and in the trap list of 2.3 is not blocked; no line of `hosts-tier.txt`
  fails `is_valid_hostname` or falls under `PROTECTED_SUFFIXES`; the tier has at most 20,000
  lines.
- **Hosts splice property test** (hand-rolled generator, no new dependency): for random
  surrounding content and line endings, `splice(splice(x, Some(b)), None) == x`, and
  `splice` applied twice equals `splice` applied once.
- **Plan and restore:** apply twice keeps the first `previous`; restore leaves an interface
  the user changed; `assess` yields each state of the tables in 1.2 and 1.3.
- **Commitment:** table-driven over every `ShieldChange`.
- **Sampling:** each gate, each cadence row, caps, snooze limit, the confirm step.
- **Naming and copy:** no "nofap", no forbidden word, no host in any engine-side text.

Other crates: `ShieldConfig` deserialises from `{}`; enum round trips; `repos/shield.rs`
in-file tests; V3 version assertions; the wipe test asserts `shield_events` is empty and
`shield_state` and `shield_domains` survive; `explicit.rs` tests for request shape (image
then text, closed schema, `usage_kind`, no title or URL anywhere in the serialised request),
parsing, refusal mapped to a verdict, truncation retry; trace-line redaction.

### 9.2 Integration tests (mock platform + fake AI)

`crates/ubiqx-platform/src/sysblock` (Linux, fakes for `Runner`, `Fs`, `Elevator`):

- For each of `MacBackend`, `WindowsBackend`, `LinuxBackend`: `apply` → `observe` →
  `assess` is `applied`/`filtered`; `remove` returns the temp "system" to its exact initial
  bytes; `remove` after a partial apply; `remove --scope resolver` leaves the hosts block.
- The exact argv of every command, and the PowerShell script constants, are asserted, so a
  change to a privileged command line is a visible diff.
- Parsers run against fixture outputs, including a PT-BR localised `networksetup` "there
  aren't any DNS servers" message [U: fixture to be recorded on a real Mac] and adapters with
  no IPv6.
- Helper argument parsing: unknown verb, missing plan, oversized plan, a plan with an IP as
  resolver, a plan with `localhost` in `extra` → exit 2 and nothing written.
- Elevation mapping: -128, 1223, 126, 127, `APPIMAGE` set.

`crates/ubiqx-engine/tests/shield.rs` (copy of the `tests/focus.rs` harness: in-memory SQLite,
`FixedClock`, every loop at 3600 s, passes driven by hand, `ScriptedPlatform`,
`MockEnforcer`, `FakeExplicitDetector`, a local recording `SystemShield`):

1. Shield off: `pass_once` makes no platform call (the scripted cursor does not advance).
2. Listed host on two passes → `close_active_tab`, one event without a host, one card;
   one pass only → nothing.
3. Close refused → blank; both refused → card only.
4. Five hits in one episode → five enforcements, one event with `attempts = 5`, cards at
   the first and third.
5. Keyword and TLD hosts block; trap hosts do not.
6. User allow beats the list; user block beats user allow.
7. ubiqX mode: a blocked sample reaches the segmenter as `[privado]` with no URL.
8. Vision: unknown host, 20 s dwell → one detector call with a 768-px window capture;
   `explicit/high` → tab closed, host learned, event `source = vision`, and the next pass
   blocks it as `Learned` without a detector call.
9. Vision positive but the foreground changed during the call → nothing closed.
10. `explicit/medium` then `none` → soft card, no enforcement; refusal → soft card.
11. Gates: `local_only`, no consent, consent for another provider, budget spent, cap reached,
    idle, sensitive host, excluded app, no window id → zero detector calls and the right
    `VisionState`.
12. Three detector errors → `degraded`; `state.ai_health()` unchanged.
13. No detector call ever carries the title or URL (the fake records its inputs).
14. Commitment: `Disable` with a 600 s delay stays on; clock +600 s → ready card; confirm →
    `system.remove(All)` called once; cancel → still on; unconfirmed for 24 h → expired.
15. Drift: scripted observation without the block → status `reduced`, one card, none on the
    next verify.
16. `productivity = false`: after start, no block is ever opened and no classifier call is
    made while the shield pass runs; `shutdown` returns.
17. "Apagar todos os dados" keeps the shield on.

### 9.3 Frontend tests (Vitest + jsdom against the mock, PT-BR copy asserted)

- Wizard: nothing is called before the last step; the DNS disclosure and the provider name
  are on screen; declining the simulated OS prompt shows "Proteção parcial".
- Layer rows render every state of section 1 from mocked statuses.
- Disable: countdown, "Manter proteção" cancels, confirm only after the delay.
- "Restaurar minha rede" has no delay.
- Panel: discreet card shows only "Página bloqueada pelo UBI."; "Não era isso" calls
  `shield_record`; urge surf advances six steps with fake timers; crisis card shows 188
  from the resources payload.
- Lapse, check-in, values and plans, progress with and without the streak.
- Standalone flavour: `ShieldApp` has no route to Timeline, Review, Reports, Categories,
  Insights or Focus; onboarding finishes with `onboarding_done`.
- `parity.test.ts` covers the `shield` namespace; `csp.test.ts` unchanged (no network from the
  webview).

### 9.4 What cannot be tested in this sandbox

Everything below is behind `Runner`, `Fs` or `Elevator`, is thin, and needs a real machine
before release. It is the release checklist, in priority order:

1. macOS 13 to 15: the `osascript` prompt with an ad-hoc signed app, its wording, cancel
   code, and that the helper runs as root from inside the `.app`.
2. macOS: `networksetup` outputs in PT-BR and EN; `empty` restores DHCP DNS for v4 and v6.
3. Windows 10 and 11 Home: UAC for an unsigned per-user install; the PowerShell scripts
   under the default execution policy (`-NoProfile -NonInteractive -Command`); the registry
   path for the static DNS list; `-ResetServerAddresses`.
4. Windows: a 20,000-name, two-family, six-per-line block: DNS Client CPU, time to first
   resolution, Defender's reaction; the names-per-line limit.
5. All: does an IPv4-only hosts line suppress AAAA; does the hosts block win when the
   browser uses DoH (Chrome secure mode, Firefox TRR).
6. Linux: the resolved drop-in on Ubuntu 22.04/24.04 and Fedora; `pkexec` from the `.deb`
   install; the AppImage message.
7. Captive portal with the filter set, per OS; "Restaurar minha rede" fixes it.
8. Cloudflare for Families: the answer for a blocked name, current addresses, terms.
9. Vision on real providers: refusal rate on explicit screenshots per vendor, false-positive
   rate on a set of ordinary and borderline screens, measured cost per check at 768 px.
10. Tauri: the `shield` window on all workspaces without taking focus; `--config` merge with
    `mainBinaryName`; two products installed side by side.
11. Every phone number and URL in `resources.json`.

No cargo build was started for this design; the plan above is by reading.

---

## 10. Work breakdown

File ownership is disjoint. A unit edits only its own files; anything it needs from another
unit is fixed here as a signature (sections 6 and 7).

### Wave 0 (one agent, first): contracts

**U0 Contracts.** Owns: `Cargo.toml` (workspace member and dependency),
`crates/ubiqx-core/src/shield.rs`, `product.rs`, `lib.rs`, the `EngineEvent` variants in
`model.rs`, `AiUsageKind::ShieldVision` in `ports.rs`, the exemption constants in
`crates/ubiqx-core/src/focus.rs`; `crates/ubiqx-shield/Cargo.toml` and `src/lib.rs` declaring every module of 7.3
with an empty file each; the two new arms in `crates/ubiqx-cli/src/main.rs`; the one-line
`reasoning_effort_for` arm in `crates/ubiqx-ai/src/openai.rs`.
*Accept:* the workspace compiles and existing tests pass; `ShieldConfig` from `{}` equals
the default; enum round-trip tests; no behaviour change.

### Wave 1 (parallel, each depends only on U0)

| Unit | Owns | Accept |
|---|---|---|
| **U1 List** | `scripts/shield/build_blocklist.py`; `crates/ubiqx-shield/data/*` except `resources.json`; `src/list.rs`, `matcher.rs`, `heuristics.rs`; `THIRD-PARTY-NOTICES` | script gates pass and a second run is byte-identical; canary and trap tests pass; `core.bin` ≤ 2.2 MB; `hosts-tier.txt` ≤ 20,000 valid names |
| **U2 System plan** | `crates/ubiqx-shield/src/hosts.rs`, `resolvers.rs`, `plan.rs` | splice property test; every state row of 1.2 and 1.3; double apply; restore rules |
| **U3 Behaviour** | `crates/ubiqx-shield/src/commitment.rs`, `sampling.rs`, `episode.rs`, `messages.rs`, `help.rs`, `progress.rs`; `data/resources.json`; `tests/naming.rs` | table tests of 9.1; naming test |
| **U4 Storage** | `crates/ubiqx-storage/src/migrations.rs`, `repos/shield.rs`, `repos/mod.rs`, `repos/maintenance.rs`, `lib.rs`, `store.rs`, `tests/repos.rs` | V3 applies on a V2 database; repo tests; wipe test |
| **U5 AI** | `crates/ubiqx-ai/src/explicit.rs`, `prompts.rs`, `fake.rs`, `lib.rs`, the two trace lines in `client.rs` and `openai.rs`, `tests/explicit.rs` | request-shape, parse, refusal, truncation and redaction tests; wiremock test for one Anthropic and one OpenAI-compatible round trip with an image |
| **U6 System adapter** | `crates/ubiqx-platform/src/sysblock/*`, `lib.rs`, `mock.rs`, `app_id.rs`, the secrets modules (keyring service as a parameter), the three `*/mod.rs`, `macos/enforce.rs` (the literal), `Cargo.toml` | 9.2 platform tests; both cross-target `cargo check` steps pass. Starts with the back-ends and helper against U2's signatures; integrates when U2 lands. |
| **U7 Frontend module** | `apps/desktop/src/lib/shield.ts`, `shieldMock.ts`, `i18n/messages/shield.ts`, `pages/Shield.tsx`, `pages/ShieldPanel.tsx`, `components/shield/*` except `ShieldShell.tsx`; small edits in `App.tsx`, `Sidebar.tsx`, `i18n/index.ts`, `i18n/messages/nav.ts`, `lib/store.ts`, `lib/mock.ts`, `lib/types.ts` | 9.3 except the standalone cases; `pnpm typecheck`; parity and forbidden-word tests. Built against the mock, so it needs no backend. |

### Wave 2

**U8 Engine** (after U1 to U5). Owns `crates/ubiqx-engine/src/shield/*`, `engine.rs`,
`state.rs`, `handle.rs`, `deps.rs`, `lib.rs`, `tracker.rs`, `update.rs`, the standalone's
ids in the engine's own exemption lists in `crates/ubiqx-engine/src/focus.rs`, `tests/shield.rs`, the `EngineDeps` and `LoopConfig`
literals in `tests/focus.rs`, `tests/pipeline.rs`, `tests/screenshot_read.rs`, and the feed
product check in `crates/ubiqx-core/src/update.rs`.
*Accept:* the 17 cases of 9.2; a feed with another product is ignored (unit test); existing
engine tests unchanged in behaviour; clippy clean.

### Wave 3 (parallel, after U6, U7, U8)

| Unit | Owns | Accept |
|---|---|---|
| **U9 Composition and shell** | `crates/ubiqx-app/src/lib.rs`; `apps/desktop/src-tauri/src/lib.rs`, `shield.rs`, `commands.rs`, `main.rs`; `capabilities/shield.json`; `Info.plist` | ubiqX builds and starts with the shield off and unchanged behaviour; `UBIQX_SCRIPTED` demo unchanged; `--shield-helper` with a bad verb exits 2 before any window; with `AiBackend::Fake` the full flow works against the scripted platform |
| **U10 Standalone product** | `apps/desktop/src-tauri/tauri.shield.conf.json`, `icons-shield/`; `apps/desktop/src/main.tsx`, `ShieldApp.tsx`, `product.ts`, `pages/ShieldOnboarding.tsx`, `pages/ShieldSettings.tsx`, `components/shield/ShieldShell.tsx`; `vite.config.ts`, `package.json` scripts; `scripts/publish-release.mjs`, `scripts/app-version.mjs`, `.github/workflows/ci.yml` | `pnpm build:shield` output has no productivity route; standalone Vitest cases; the release script's own tests pass for both products; ubiqX artifacts and feed names unchanged |
| **U11 Docs** | `docs/SHIELD.md` (what it does, what it sends, manual removal per OS and packaging), `docs/ARCHITECTURE.md`, `README.md`, `docs/WINDOWS-LINUX.md`, `docs/LICENSING.md` | every state and limit of section 1 documented; removal steps for all packaging formats; the doc/code drifts the maps found in the touched sections corrected |

Final gate: `cargo fmt --check`, clippy `-D warnings`, `cargo test`, the two cross-target
checks, `pnpm test`, and the section 9.4 checklist filed as release blockers.

---

## 11. Risks and open questions for the owner

| # | Question | Recommended default |
|---|---|---|
| 1 | **Name.** The request says "nofap", a registered and litigated mark. | Code `shield`; UI "Escudo do UBI"; standalone "Leme, by ubiqX" pending an INPI/USPTO search. "nofap" appears nowhere a user, a store or a package registry can see. |
| 2 | **Third-party DNS.** L3 sends every DNS query of the machine to Cloudflare. | Preselected in the wizard with the plain disclosure, with "só a lista neste computador" one click away. One provider at launch; CleanBrowsing Family (stricter, also blocks Reddit and VPN sites) only after written clearance of its free-tier terms. |
| 3 | **Vision defaults.** Private windows are checked; a high-confidence positive closes the tab without asking; learned sites are stored; "IA do Ubi" routes screenshots through the operator's server. | On only after the consent step; caps 20/hour and 80/day; auto-close tabs, ask for apps; learning on; managed proxy allowed with its own consent line. |
| 4 | **How hard is "off"?** | 10-minute delay, confirmation after the wait, no partner, no lock, no punishment. An admin can always undo it and the product says so. |
| 5 | **Selling and shipping the standalone.** No product-specific plan exists; CI publishes a continuous release on every push to every branch and installed apps take it. | v1: existing keys work in both apps and blocking is never gated. Before the first push of this work, restrict `publish-continuous` to `main` (or add the product and branch check to the update filter), and publish the standalone under its own tag. |
| 6 | **Scope cut and real-machine gaps.** v1 has no browser policies (DoH, SafeSearch), no SafeSearch pinning, no downloadable list, no silent re-apply, no system layer for AppImage; macOS and Windows system calls are verified by reading only. | Accept for v1; treat section 9.4 as release blockers; v1.1 = browser policies as an extra plan section, then the signed Extended list pack. |

Residual risks the owner should know even with those defaults: the hosts block may be
ignored by a browser using DoH [U]; the guard is blind on Wayland and for Firefox on macOS;
vision models may refuse or misjudge, so false positives will happen and the copy is written
for that; an ad-hoc signed macOS app loses its permissions at each update, which now also
costs the vision layer its Screen Recording grant.

---

## 12. How this differs from the obvious alternative

The obvious build is "real DNS blocking": a local resolver or network extension with the full
list, browser policies and SafeSearch on every browser, a downloadable 500k-entry list, the
list stored in the existing focus tables, the existing vision pipeline reused, the switch in
`Settings`, and a second app as a second codebase. This design chooses otherwise:

| Obvious | Here | Why |
|---|---|---|
| Local DNS proxy / `NEDNSProxyProvider` | Hosts block + public family resolver | A proxy fails closed (no internet when the app is gone) and the extension needs a Developer ID the project does not have. Both chosen mechanisms fail safe and need one OS prompt. |
| The big list is the DNS block | The big list (239k, hashed) serves the in-app guard; the hosts file gets the 20k most visited names; the resolver covers the tail | Hosts files have no wildcards and Windows degrades with large files. |
| Browser policies and SafeSearch in v1 | Deferred; the guard reads the URL whatever resolver the browser uses, and vision covers image search | Many locations, a "Managed by your organization" surprise, unproven on macOS. |
| List in SQLite / `focus_targets` | Embedded sorted hash table, in-memory matcher | One mutex-guarded connection; the guard reloads targets every 2 s; a readable porn directory on disk is the wrong artefact. |
| Signed list updates and the share-alike pack | Embedded Core, refreshed with app updates | Removes a key, a CI job, a download path and a licence obligation from v1; the seam is one function argument. |
| Reuse `classify::run_once` for vision | A separate real-time check with its own gates, caps, health and no stored image | The existing path runs after a block closes, skips private windows, stores screenshots and shares AI health. |
| A `Settings.nofap` struct | Engine-owned `ShieldConfig` changed through typed, delay-aware requests | The UI saves `Settings` whole, last writer wins; a protection switch there is a one-write bypass. |
| New `Enforcer` methods | None | Four implementations per method; the existing three actions suffice with "ask before quitting an app". |
| Second app = second shell crate or fork | One shell crate, one engine with `productivity = false`, one Vite project, a second config file | No duplicated shell, no drift, one lint and test run for both. |
| OS code tested only on its OS | Back-ends as argv builders and parsers compiled everywhere behind `Runner`/`Fs`/`Elevator` | The macOS and Windows logic runs in the Linux test suite; only the spawn is untested. |
