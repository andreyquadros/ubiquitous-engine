# Design proposal "private": the Guard module and the standalone product

Angle: privacy and safety first. Every byte that is captured, sent or stored is assumed to be
the worst possible byte (an explicit screenshot, a list of sites a person is ashamed of, a
record of lapses). The design meets the owner's request (DNS blocking with an extensive list,
detection with the AI already in the app, counselling, removal of access, a module in ubiqX and
a separate product) and chooses, at every fork, the option that captures less, decides on the
device, keeps nothing a third person could read, and claims only what is true.

Inputs: the four code maps and three research reports in this directory, plus direct reads of
`crates/ubiqx-core/src/ports.rs`, `segmenter.rs`, `crates/ubiqx-platform/src/image_util.rs`,
`linux/capture.rs`, `crates/ubiqx-app/src/lib.rs`, `apps/desktop/src-tauri/tauri.conf.json`,
`apps/desktop/src/App.tsx`, `i18n/index.ts` and the workspace `Cargo.toml`. Nothing was
compiled or run. Statements about macOS and Windows system behaviour come from the research
report, which itself could not open primary sources; section 9.4 lists what has to be checked
on real machines before release.

Vocabulary used below: **Guard** is the module (user-visible name "Guarda do UBI" / "UBI
Guard", internal identifier `shield`); **Leme** is the working name of the standalone product
(section 8). "The request" is the owner's message that asked for a module called nofap.

Contents

0. The twelve decisions, and the alternative each one rejects
1. Enforcement tiers
2. The blocklist
3. Vision detection
4. Counselling
5. Commitment
6. Data model
7. Code architecture
8. Naming and branding
9. Test plan
10. Work breakdown
11. Risks and open questions for the owner

---

## 0. The twelve decisions, and the alternative each one rejects

| # | Decision | Obvious alternative that is rejected, and why |
|---|---|---|
| D1 | **Decide on the device first.** A page is judged by lists, host heuristics and window class before any pixel is read. The AI is asked only when a local rule says "unknown, and worth a look". | Screenshot every N seconds and ask the AI. That sends the user's whole screen life to a provider and stores a rhythm of checks. |
| D2 | **A vision check sends one image and nothing else**: the foreground window only, 640 px longest edge, browser chrome cropped, JPEG, no URL, no title, no app name, no user profile. The answer is one of four words. | Reuse the productivity vision prompt (title, domain, profile, free-text description). A description of an explicit screen is itself sensitive data. |
| D3 | **No image is ever written to disk, and no verdict is stored with a place or a time.** The only record is a counter per local day and hour (`shield_day`). | Reuse `screenshots` and `interventions`. `interventions` keeps the domain for ever and shows it in a list. |
| D4 | **No log of sites, anywhere.** Prompts, notifications, events and tables carry no host and no app name; the `GuardPrompt` type has no field for one. Sites the Guard learns from a vision hit are stored only as salted hashes. | A "blocked attempts" history. It is the artefact a partner, a parent or an employer would look for, and the one a leak would expose. |
| D5 | **Learned and visited sites never go into the hosts file.** The hosts block is the same popularity-ranked list for every user plus names the user typed. | Promote visited listed domains into hosts (blocklist report, section 1). `/etc/hosts` is world-readable: that would publish the user's visits. |
| D6 | **The third-party family resolver is a choice the user makes with the operator's name in front of them**, not a default. Hosts and browser policies need no third party. | Switch DNS to a family filter by default. It sends every DNS query of the machine to one company. |
| D7 | **The Guard has its own consent, its own caps, its own health state and its own budget.** It never changes `AiHealth`, never spends past its own cap, and the productivity settings cannot switch it off by accident (tray pause, private mode, a stale settings write). | Hang it on `Settings` and the shared AI health. One failed check would pause classification; one settings write from a stale window would disable protection. |
| D8 | **A vision verdict is treated as uncertain.** The default reaction hides the window (nothing is destroyed) and asks. A list hit is certain and closes the tab. | Close on any positive. A false positive would close a tab with unsaved work and call the user something they are not. |
| D9 | **Commitment is a delay the user sets while calm, and nothing else.** No partner, no report, no password held by someone else, no uninstall block, no hidden mode. Network repair and full removal are always available. | Accountability partner and tamper alarms. No controlled evidence, real coercion risk, and it is where this category was removed from Google Play. |
| D10 | **Counselling is a reviewed message bank.** The model returns a class, never a sentence shown to the user. | AI-written coaching. Unreviewed text about sex, shame and despair, generated at the worst moment. |
| D11 | **The standalone product is a separate binary that does not link the tracker.** `ubiqx-engine` is not in its dependency tree, and CI checks that. | Ship ubiqX with tracking switched off. "We do not track" would rest on a flag instead of on what the binary contains. |
| D12 | **Nothing user-visible is called "NoFap".** Internal identifier `shield`; a test fails if the string reaches a bundle, a manifest or a system file. | Use the requested name. It is a registered mark of an owner who litigates, and it commits the product to a stance on masturbation it must not take. |

What the request asked for and where it is met: DNS blocking with an extensive list
(sections 1.2 to 1.4 and 2), the embedded AI looking at unknown sites (section 3), advice
(section 4), removal of access (sections 1.1, 3.6), a module in ubiqX and a separate product
(section 7).

---

## 1. Enforcement tiers

Five tiers. Tier 0 and tier 4 need a running app; tiers 1 to 3 are changes to the system that
keep working when the app is closed. Each tier is separately visible in the UI with one of
these states: `off`, `applied`, `partial(detail)`, `changed(detail)`, `unsupported(reason)`.
The page never says "protected" without naming which tiers are on.

Honest ceiling, shown in the consent screen and the Help page: this is friction for the
person who installed it. Anyone with the administrator password can undo every tier; a phone,
a VPN, Tor or another user account is outside it.

### 1.1 Tier 0: the Guard pass (host and title rules on the foreground window)

- **What it does.** Every 2 s it reads the foreground window and, for a supported browser,
  the URL of the active tab (`ActivitySource::foreground`, `BrowserUrlResolver::resolve`).
  The host is classified on the device (section 2.4). A host classed `Adult` is enforced:
  close the active tab, else blank it, else minimise the window, else only tell the user.
  Then one prompt is shown (section 4.1).
- **Differences from the focus guard**, all deliberate:
  1. Enforcement is never suspended by a cooldown. The cooldown (60 s) applies to the prompt
     window only; a blocked host in front is closed again on every pass.
  2. A close needs two consecutive passes that agree on `(app_id, window title, host)`, so a
     stale URL-cache entry cannot close the tab the user moved to.
  3. Nothing is written to `interventions`, and the prompt names no site.
  4. It has its own switch (`ShieldConfig.enabled` plus recorded consent) and ignores
     `focus.guard_enabled`, tracker pause, private mode and quiet hours. It returns before
     `foreground()` when it is off, so scripted demos and `ubiqx simulate` are unchanged.
  5. It reports a missing permission itself (macOS Automation or Screen Recording) as a
     coverage state; it does not rely on the tracker's sampler.
- **Per OS** (existing adapters, existing limits):

  | | macOS | Windows | Linux |
  |---|---|---|---|
  | Host known for | Safari and the Chromium family (Automation grant per browser) | Chrome, Edge, Brave, Opera, Vivaldi, Firefox (UI Automation) | X11 only, and only when the title prints the domain |
  | Not seen | Firefox and forks, PWAs, webviews | other browsers | native Wayland windows (nothing is seen) |
  | Close tab | AppleScript | `Ctrl+W` keystroke | `Ctrl+W` through XTest |
  | Fix shipped with this work | none | `UiaUrlResolver` answers `None` while the address bar has keyboard focus (text being typed is not the page) | none |

- **Privileges.** None beyond what ubiqX already asks for.
- **Verified.** By the next pass: if the same host is still in front, it acts again.
- **Undone.** Turning the Guard off (through the delay, section 5).
- **Failure modes and what the user sees.** Automation denied: "O Guarda não consegue ler o
  endereço no <navegador>" with the button that opens the permission pane. Browser not
  supported: the coverage table on the Guard page says "só DNS e verificação por IA" for it.
  Wayland: "Neste sistema o Guarda só funciona pelas camadas do sistema".
- **New `Enforcer` method**: `minimize_app(&self, app_id: &str) -> CoreResult<bool>`
  (macOS `NSRunningApplication::hide`, Windows `ShowWindow(SW_MINIMIZE)` on the app's
  top-level windows, Linux X11 iconify; `MockEnforcer` records `EnforcerCall::Minimize`).

### 1.2 Tier 1: marked block in the hosts file

- **What it does.** Maps a fixed set of hostnames to `0.0.0.0` and `::` for every program
  that uses the OS resolver, with no process running and no third party.
- **Content.** The hosts tier compiled into the helper (popularity-ranked, built by
  `scripts/shield/build_blocklist.py`, section 2.2) plus the hostnames the user typed into
  "Sites que eu quero bloquear" (apex and `www.` for each). Never learned sites (D5).
  Caps: 12,000 hostnames on Windows, 20,000 on macOS and Linux, nine names per line, one
  `0.0.0.0` line and one `::` line per group. These caps are starting values to be confirmed
  by real-machine test 4 of the DNS report; they are constants in
  `ubiqx_shield_sys::hosts::{CAP_WINDOWS, CAP_UNIX}`.
- **Marker.** `# >>> ubi-guard BEGIN v1 sha256=<hex of the block body>` ...
  `# <<< ubi-guard END`. Apply replaces the block or appends it; remove deletes exactly the
  block. Every other byte, the line-ending convention and the file mode are preserved. A
  canary line inside the block maps `hc-<8 hex of the install id>.test` to `0.0.0.0`.
- **Per OS.** Windows `%SystemRoot%\System32\drivers\etc\hosts`, then `ipconfig /flushdns`;
  macOS `/etc/hosts`, then `dscacheutil -flushcache` and `killall -HUP mDNSResponder`; Linux
  `/etc/hosts`, then `resolvectl flush-caches` when systemd-resolved is active. No
  Microsoft-owned name is ever written (Defender `HostsFileHijack`); the build gate rejects
  them.
- **Privileges.** Administrator, through the helper (section 1.6). One prompt per apply.
- **Verified** without privilege every 60 s: the block is present, its hash matches, and the
  canary name resolves to `0.0.0.0` through the OS resolver. The canary is a reserved `.test`
  name with a per-install label, so when the block is missing the lookup that leaves the
  machine identifies neither the product nor the user.
- **Re-applied** only when the user presses "Restaurar proteção" on the calm drift card
  (section 1.7). Never silently.
- **Undone** by the helper's `remove` verb: the block is deleted, the rest is untouched.
- **Failure modes.** File locked or restored by security software: `partial("o arquivo hosts
  não pôde ser escrito")`. Browser using its own DoH: covered by tier 3 where tier 3 exists,
  otherwise stated as "os navegadores podem contornar esta camada". New pages only: the UI
  says "vale para páginas novas; reinicie o navegador para garantir".

### 1.3 Tier 2: system resolver pointed at a family filter (opt-in, named operator)

- **What it does.** Sets the machine's DNS servers to a public resolver that refuses adult
  domains, which covers the long tail no hosts file can hold.
- **Privacy cost, stated before the choice.** "Todas as consultas de DNS deste computador
  passam a ir para <operador>. O ubiqX não recebe nada." The onboarding step has three radio
  options and none is preselected: Cloudflare for Families (1.1.1.3, 1.0.0.3,
  2606:4700:4700::1113, 2606:4700:4700::1003; marked "recomendado"), CleanBrowsing Family
  (shown only when the compile-time flag `RESOLVER_CLEANBROWSING` is on, see question 2), and
  "Não usar DNS de terceiros". The resolver table is a constant in
  `ubiqx_core::shield::sys::RESOLVERS`; the helper accepts a resolver id, never an address.
- **Per OS.**

  | OS | Apply | Record for undo | Undo | Encryption |
  |---|---|---|---|---|
  | macOS | `networksetup -setdnsservers "<service>" <v4a> <v4b> <v6a> <v6b>` for every enabled service from `-listallnetworkservices` | output of `-getdnsservers` per service (addresses or "none") | restore the recorded value, or `empty` | none in v1 (plain DNS) |
  | Windows | `Set-DnsClientServerAddress -InterfaceIndex <i> -ServerAddresses (...)` on every adapter that is up; state read with `Get-DnsClientServerAddress \| ConvertTo-Json` (never localised text) | previous servers per interface and family, or "DHCP" | `-ResetServerAddresses` or the recorded list | Windows 11: `Add-DnsClientDohServerAddress` with auto-upgrade and UDP fallback allowed |
  | Linux with systemd-resolved | drop-in `/etc/systemd/resolved.conf.d/90-ubi-guard.conf` (`DNS=<ip>#<name> ...`, `Domains=~.`, `DNSOverTLS=opportunistic`), then `systemctl restart systemd-resolved` | nothing (the file is ours) | delete the file, restart the service | DNS over TLS, opportunistic |
  | Linux without systemd-resolved | not offered in v1 | | | `unsupported("este sistema não usa systemd-resolved")` |

  A recorded value is restored only if the current value is still the one the helper wrote;
  otherwise the user's later change is left alone.
- **Privileges.** Administrator, through the helper.
- **Verified** without privilege every 60 s and on wake: `networksetup -getdnsservers`,
  `Get-DnsClientServerAddress`, `resolvectl dns`. No active probe with an adult domain is
  ever made.
- **Failure modes.** A new network service or adapter has DHCP DNS: `changed("uma conexão
  nova está sem o filtro")`. A VPN or another resolver is in effect: `partial("outro DNS está
  em uso; a proteção fica reduzida")`; the Guard does not look for VPN or Tor processes and
  does not fight them. Captive portal or broken name resolution: the **"Restaurar minha
  rede"** action removes only this tier, has no delay (section 5.3) and is on the Guard page,
  in the tray menu and in the Help page.

### 1.4 Tier 3: browser policies (stop the browser resolving around tiers 1 and 2; SafeSearch)

- **What it does.** Sets each installed browser's own enterprise policy so that it uses the
  OS resolver, and forces SafeSearch where a policy exists.
- **Disclosure before applying** (plain words, in the preview list): the browser will show
  "Gerenciado pela sua organização"; browser DNS-over-HTTPS is switched off so that the
  blocks apply, which means the browser's lookups follow the system setting.
- **Values.** Chromium family: `DnsOverHttpsMode = "off"`, `ForceGoogleSafeSearch = true`,
  `ForceYouTubeRestrict = 1` (moderate). Edge also `ForceBingSafeSearch = 2`. Brave also
  `TorDisabled = true`. Firefox: `DNSOverHTTPS = { Enabled: false, Locked: true }`.
  Incognito is not disabled.
- **Per OS.**

  | OS | Where | State |
  |---|---|---|
  | Windows | `HKLM\SOFTWARE\Policies\Google\Chrome`, `...\Microsoft\Edge`, `...\BraveSoftware\Brave`, `...\Chromium`, `...\Mozilla\Firefox\DNSOverHTTPS`; only our named values, previous value of each recorded | `applied`; to be confirmed in `chrome://policy` on an unmanaged Windows 11 (test 3) |
  | Linux | our own file `ubi-guard.json` in `/etc/opt/chrome/policies/managed/`, `/etc/chromium/policies/managed/`, `/etc/chromium-browser/policies/managed/`, `/etc/opt/edge/policies/managed/`, `/etc/brave/policies/managed/` for browsers that are installed; Firefox `/etc/firefox/policies/policies.json` (created, or the one key merged and the previous value recorded) | `applied`; Snap and Flatpak browsers `partial` |
  | macOS | Firefox: `defaults write /Library/Preferences/org.mozilla.firefox` (`EnterprisePoliciesEnabled`, `DNSOverHTTPS__Enabled`, `DNSOverHTTPS__Locked`). Chromium family: **nothing is written in v1** (only "recommended" policies are possible without MDM) | `partial("Chrome, Edge e Brave no macOS ficam sem política; valem as outras camadas")` |

  A machine that already has policy values from a real organisation (values present that the
  state file does not list) is left alone: `unsupported("este computador já é gerenciado")`.
- **SafeSearch by hosts pinning is not done** in v1: a stale pinned address breaks Google or
  YouTube, and the Bing line is a Defender risk. Image search is covered by the policy where
  it exists and otherwise by tier 4 (search result pages are on the "mixed" list).
- **Privileges, verification, undo.** As tier 1; verification reads the registry values or
  files; undo restores recorded values and deletes our files.

### 1.5 Tier 4: vision check

Section 3. No privilege except macOS Screen Recording. State strings: `applied`,
`off`, `unsupported("Wayland")`, `partial("sem permissão de gravação de tela")`,
`partial("sem IA configurada")`, `partial("limite de hoje atingido")`,
`partial("IA indisponível, tentando de novo às HH:MM")`.

### 1.6 The system layer: one helper, fixed verbs, validated plan

- **Crates.** `crates/ubiqx-shield-sys` (library: everything pure, plus the thin OS calls) and
  `crates/ubi-guard-helper` (binary, about 40 lines, calls `ubiqx_shield_sys::helper::run`).
  The helper links neither Tauri nor the capture or AI crates.
- **Verbs.** `status --json` (no privilege), `apply --plan <file> --result <file>`,
  `remove --product <id> --result <file> [--layers hosts,resolver,policies]`,
  `restore-network --result <file>`. Nothing else is accepted; there is no "run this".
- **Plan** (`SystemPlan`, JSON, written by the app into its own data directory, deleted after
  the run): `{ v: 1, product: "ubiqx" | "leme", hosts: { on, extra_names: [..] },
  resolver: null | "cloudflare_family" | "cleanbrowsing_family", policies: { on } }`.
  The helper re-validates everything as root: version, product id from a closed set, resolver
  id from the compiled table, at most 500 extra names, each a syntactically valid hostname of
  at least two labels that is not an IP, not `localhost`, not a public suffix from the small
  embedded deny list and not under a protected suffix (`microsoft.com`, `windows.com`,
  `windowsupdate.com`, `apple.com`, `icloud.com`, `ubuntu.com`, `debian.org`,
  `mozilla.org`, `google.com`, `gstatic.com`, `googleapis.com`, `github.com`,
  `githubusercontent.com`, `cloudflare-dns.com`, `anthropic.com`, `openai.com`, `x.ai`,
  `ubiqx.ai`). The hosts block is generated by the helper from its embedded tier, or from a
  downloaded tier only after the helper itself has verified the signed manifest (section 2.8).
- **Elevation.**

  | OS | How | Prompt |
  |---|---|---|
  | macOS | `osascript -e 'do shell script "<helper> apply --plan <p> --result <r>" with administrator privileges'`; the three paths are produced by the app and rejected unless they match `[A-Za-z0-9 ._/-]+` | system password dialog (names `osascript`) |
  | Windows | `ShellExecuteExW` with verb `runas` on `ubi-guard-helper.exe`, wait on the process handle, read the exit code and the result file in `%ProgramData%\ubi-guard\` | UAC (unknown publisher until the app is signed) |
  | Linux `.deb` | `pkexec /usr/lib/ubi-guard/ubi-guard-helper ...` with `/usr/share/polkit-1/actions/ai.ubiqx.guard.policy` (`auth_admin`, `exec.path` annotation) | polkit dialog with our own sentence |
  | Linux AppImage | **system tiers are not offered** (the helper would be a user-writable file run as root). The Guard page says so and points to the `.deb`. | none |

  Before any prompt the app shows the exact list of changes (`SystemShield::preview`) in plain
  words, and the sentence "O sistema vai pedir sua senha de administrador para aplicar isso".
- **State file** (what was changed, for undo; no user data): macOS
  `/Library/Application Support/ubi-guard/state.json`, Linux `/var/lib/ubi-guard/state.json`,
  Windows `%ProgramData%\ubi-guard\state.json`. It lists the products that asked for each
  tier (`owners: ["ubiqx", "leme"]`). `remove --product leme` deletes a tier only when no other
  owner remains; otherwise the result says "em uso pelo outro app". This is how the module and
  the standalone product share one mechanism without undoing each other.
- **Port** (`ubiqx_core::ports`):

  ```rust
  pub trait SystemShield: Send + Sync {
      fn capabilities(&self) -> SystemCapabilities;                         // per tier: supported or why not
      fn status(&self, want: &SystemWanted) -> CoreResult<SystemStatus>;    // unprivileged, cheap
      fn preview(&self, plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>>;
      fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyReport>;        // blocks on the OS prompt
      fn remove(&self, layers: LayerSet, product: ProductId) -> CoreResult<ApplyReport>;
  }
  ```

  `Err(CoreError::Permission(_))` means the user declined the prompt. `ApplyReport` has one
  entry per tier: `Applied`, `Skipped(reason)` or `Failed(reason)`.
  Implementations: `ubiqx_shield_sys::NativeSystemShield` (wired into `PlatformServices` as
  `system_shield`), `ubiqx_platform::mock::MockSystemShield` (call log, scripted status and
  refusal switches).

### 1.7 Keeping it applied, and what happens when it changes

- `ShieldService::system_pass` runs every 60 s (first run 10 s after start) and calls
  `SystemShield::status`. It never elevates.
- A difference produces one in-app card and at most one OS notification per 24 h whose whole
  text is "O Guarda precisa da sua atenção." The card says what changed in neutral words
  ("A proteção do sistema mudou. Restaurar?") with two buttons: "Restaurar" (preview, then the
  OS prompt) and "Agora não". No counter is incremented, nothing is called tampering, there
  is no penalty.
- A new network service on macOS or adapter on Windows is the same card with the text "Uma
  conexão nova está sem o filtro".

### 1.8 Uninstall and removal

- The system tiers fail safe when the app disappears: the hosts block and a public resolver
  leave the machine working.
- `.deb`: `prerm` runs `ubi-guard-helper remove --product <id>` (Tauri
  `bundle.linux.deb.preRemoveScript`). NSIS: an uninstall hook starts the helper with
  `runas`. macOS and AppImage have no hook.
- A removal path that needs no app: `ubi-guard-helper remove --product <id>` run with
  `sudo` or from an elevated prompt, and `ubiqx guard remove` in the CLI (handled before the
  engine starts). The Help page and `docs/guard/REMOVER.md` give these and the manual steps
  per OS, including the browser policy values.

---

## 2. The blocklist

### 2.1 Tiers and sources

| Tier | Sources | Entries (measured 2026-11-19) | Licence | Where |
|---|---|---|---|---|
| Core | StevenBlack `porn-only`, ShadowWhisperer `Adult`, 4skinSkywalker entries that carry a block-grade keyword, `data/shield/extra-ptbr.txt`, minus `data/shield/allow.txt` | 239,449 | MIT, Unlicense | `crates/ubiqx-core/data/shield/core.bin` (1.92 MB), `include_bytes!` |
| Extended | Core, Bon-Appetit/porn-domains, 4skin entries another list confirms | 523,112 | CC BY-SA 4.0, data file only | signed pack downloaded to `<data_dir>/shield/lists/extended.bin` (4.18 MB) |
| Hosts tier | hostnames of the CrUX top-1M (rank at most 500k) covered by the Core, with apex and `www.` | about 24,500, capped per OS | follows the Core, CrUX CC BY 4.0 | `crates/ubiqx-shield-sys/data/hosts-hot.txt.gz` (about 126 KB), compiled into the helper |
| Popular (not a blocklist) | CrUX hostnames with rank at most 100k that are on no list and not on `mixed.txt` | about 100,000 | CrUX CC BY 4.0 | `crates/ubiqx-core/data/shield/popular.bin` (about 0.8 MB) |

The hosts tier is built from the Core only, so the helper carries no share-alike data.
HaGeZi, oisd and chadmayfield (GPL-3.0) are not shipped. blocklistproject and UT1 are not
used. A "custom list by URL" feature is not in v1.

Hand-maintained inputs in `data/shield/`: `sources.lock.json`, `allow.txt`,
`extra-ptbr.txt`, `mixed.txt` (about 60 hosts: Reddit, X, Tumblr, Imgur, Pinterest, VK,
Telegram web, Discord, Twitch, DeviantArt, Pixiv, search engines), `never-capture.txt`
(section 3.2), `canary-block.txt`, `canary-allow.txt`, `reviewed-top.txt`,
pinned `public_suffix_list.dat` and `crux-top1m.csv.gz` (build-time only).

### 2.2 Build

`scripts/shield/build_blocklist.py` (Python 3 standard library) follows section 12 of the
blocklist report without change: fetch at pinned commits and verify SHA-256; parse; strip
`www.`, `m.`, `mobile.`; drop public suffixes; drop `*.tumblr.com` and `*.blogspot.*`; never
collapse an entry to its registrable domain; apply the allowlist; suffix-deduplicate; emit
the tables, the hosts tier, the Popular table, `NOTICE.txt` and `manifest.json`. Gates a to f
of that section stop the job. The plain-text lists are CI artefacts and are not committed.

### 2.3 Embedded format

Sorted table of 8-byte keys, as recommended by the report, with the product-neutral tag:

- key = first 8 bytes of `SHA-256("ubi-guard-blocklist-v1\0" || normalised_name)`, big-endian;
- file = magic `"UBGBL1\0\0"`, `format: u16`, `flags: u16`, `count: u32`,
  `list_version: u64` (UTC `YYYYMMDDHHMM`), then `count` sorted unique keys;
- lookup = binary search; memory = file size; no new dependency (`sha2` is in the workspace).

The table cannot be enumerated: the app does not put a readable directory of half a million
adult sites on the disk of the person avoiding them. The hosts tier is necessarily plain
text; it is limited to names that are already widely known.

API (`crates/ubiqx-core/src/shield/matcher.rs`):

```rust
pub struct HashTable<'a> { /* validated bytes */ }
impl<'a> HashTable<'a> {
    pub fn parse(bytes: Cow<'a, [u8]>) -> Result<Self, TableError>;   // magic, count, sorted, unique
    pub fn contains(&self, name: &str) -> bool;
    pub fn len(&self) -> usize;
    pub fn version(&self) -> u64;
}
pub fn normalize_host(raw: &str) -> Option<String>;   // lower-case, IDNA, no port, no IP literal, at least two labels
pub fn parents(host: &str) -> impl Iterator<Item = &str>;   // the host, then one label less, down to two labels
```

### 2.4 Matching and the class of a host

`crates/ubiqx-core/src/shield/classify.rs`:

```rust
pub enum HostClass {
    Adult(HitSource),                 // enforce now, no screenshot
    Allowed,                          // user or built-in allow: never enforced, never captured
    NeverCapture,                     // sensitive: never captured, never enforced
    Mixed,                            // reachable; vision-eligible at priority cadence
    Popular,                          // vision only if a local signal fires
    Unknown { signal: Option<Signal> }
}
pub enum HitSource { UserBlock, Learned, List, Keyword, Tld, MixedStrict }
pub enum Signal { WeakKeyword, SignalTld, MixedPath, PrivateWindow, TitleKeyword }
pub fn classify_host(lists: &Lists, prefs: &GuardPrefs, host: &str, now: DateTime<Utc>) -> HostClass;
```

Order of authority, most specific name first (longest match wins, allow beats block at the
same length): user block, user allow (including unexpired "não era isso" entries),
built-in allow, learned, list (Extended when present, else Core), mixed host the user ticked
in strict mode, heuristics, never-capture, mixed, popular, unknown. A user block is never
overridden by a list update. Names on the built-in allowlist, under `gov`, `gov.br`, `jus.br`,
`edu`, `edu.br`, `ac.*` and `mil`, and names in `reviewed-top.txt` are never touched by
heuristics.

### 2.5 Keyword and TLD heuristics (`shield/heuristics.rs`)

Tables taken from section 9 of the blocklist report, as constants with one test per row of
that report's examples.

- **Block-grade** (class `Adult(Keyword)` or `Adult(Tld)`): the block-grade substrings of
  9.1 (`porn`, `xxx`, `hentai`, the explicit act words, `bokep`, `phimsex`, the PT-BR set
  `putaria`, `buceta`, `siririca`, `contoseroticos`, `brasileirinhas`, `safadas`, `novinhas`,
  `gostosas`, `peladas`, `transando`, `gozando`); the brand stems after look-alike folding
  (`0→o`, `1→i`, `3→e`, `4→a`, `5→s`, `7→t`, `vv→w`, edge digits stripped); the TLDs `.xxx`,
  `.porn`, `.sex`, `.sexy`, `.adult`. `milf` and `sexy` are demoted to signals.
- **Signals only** (never block; they raise vision priority): the medium words of 9.2 as
  whole tokens, two weak tokens together, the TLDs `.cam`, `.webcam`, `.tube`, and path rules
  on mixed hosts where the full URL is known (`reddit.com/r/<name with a block-grade word>`,
  `/over18`, `/nsfw` segments, image-search result pages).
- **Never used**: the traps of 9.3 (`sex`, `anal`, `cum`, `ass`, `cock`, `dick`, `tits`,
  `cam`, `gay`, `teen`, `pau`, `rola`, `pica`, `puta`, `bunda`, `pelada` and the rest).
  `gay` is never a signal either: the Guard must not treat an orientation as a risk marker.
- **Window titles** are signals only, never a reason to act: a news article about
  pornography has the word in its title. Tokens are matched after accent folding.

A heuristic hit carries a one-tap "Isso não é conteúdo adulto" in the prompt (section 5.2).

### 2.6 User lists

Table `shield_sites` (section 6.2). "Sites que eu quero bloquear" (typed by the user, stored
in plain text because the user must be able to read and edit their own list, enforced by
tier 0 at once and by tier 1 at the next apply) and "Sites liberados" (allow). Adding a block
is immediate. Removing a block or adding an allow is a loosening change (section 5).

### 2.7 Learned sites

A host becomes "learned" when a vision check said `explicit` and the user confirmed
("Fechar") or two `explicit` verdicts for the same host arrived within 10 minutes without a
"não era isso". Only hosts of class `Unknown` are learned; `Mixed` and `Popular` hosts never
are (it would block Reddit for one post). A learned host is stored as
`host_key = SHA-256(install_salt || 0x00 || host)[..16]` with `host = NULL`. It is enforced
by tier 0 like a list hit. It is not shown by name anywhere, is not exported, and is not
written to the hosts file. The Guard page shows only the count ("3 sites aprendidos") and
"Esquecer sites aprendidos" (a loosening change).

Honest limit, in the privacy text: the salt is stored beside the hashes, so someone with
access to the user's account can test whether one specific site is in the set; they cannot
list the set.

### 2.8 Updates

As section 11 of the blocklist report, with these fixed choices:

- release tag `guard-lists` on this repository; assets `manifest.json`, `manifest.json.sig`
  (Ed25519, new key pair, public key in `ubiqx_core::shield::lists::LISTS_PUBKEY_HEX`),
  `core.bin`, `extended.bin`, `popular.bin`, `hosts-hot.txt.gz`, the plain text of the
  Extended tier, `NOTICE.txt`, `LICENSE-CC-BY-SA-4.0.txt`;
- the client (`ubiqx-ai/src/lists_fetch.rs`, behind the port `ListFetcher`) checks weekly with
  jitter and when the Guard is switched on: signature, `format`, strictly greater
  `list_version`, SHA-256 and length of each file, table structure, canaries, then atomic
  rename. Any failure keeps the previous files; the embedded Core is the floor;
- the request carries no user data. The settings text says: "A atualização baixa um arquivo
  do GitHub. Ele vê que algum computador pediu a lista, não quem você é nem o que você
  acessa." A toggle "Atualizar listas automaticamente" (on by default) controls it;
- a new hosts tier is used at the next apply the user starts for another reason. No weekly
  elevation prompt. The helper verifies the manifest signature and the file hash itself
  before using a downloaded tier;
- CI job `.github/workflows/guard-lists.yml` (weekly and manual) with the secret
  `GUARD_LIST_SIGNING_KEY`. If the repository is private the same files are served by
  `services/ubi-api` at `/v1/guard-lists/<name>` without authentication.

Attribution: `THIRD-PARTY-NOTICES` and the pack's `NOTICE.txt` with the texts of section 13
of the blocklist report (product names replaced), shown in About in both products.

---

## 3. Vision detection

### 3.1 Principle

A screenshot is the most dangerous thing this app can create. It is taken only when all the
local gates of 3.2 agree, it exists only in memory, it is sent only to the provider the user
configured, and the answer is one word. The same port can later be implemented by an
on-device model without any other change (`ExplicitDetector`); v1 uses the AI already in the
app, as requested.

### 3.2 When a check runs (all conditions, in this order)

`ubiqx_core::shield::gate::VisionGate` is a pure state machine driven by the Guard pass; the
clock is injected.

1. The Guard is on, consent version 1 is recorded, and `vision.scope != Off`.
2. The host allows remote AI (`ShieldHost::ai_gate()`): a provider and key are configured;
   in ubiqX `settings.local_only` is false and onboarding is done; for the managed provider
   `vision.allow_managed` is true and the licence is `monthly_managed`.
3. The window has an OS window id. There is never a display or region fallback.
4. The app is eligible: a browser (`BrowserUrlResolver::supports`, plus the known browser
   ids whose URL cannot be read), or a media application from the constant list
   `MEDIA_APP_IDS` (VLC, mpv, IINA, QuickTime Player, Windows Media Player, Movies & TV,
   Photos and image viewers, Celluloid, Totem) when `vision.media_apps` is true (default
   true). Messaging and mail apps (`redact::is_messaging_app`) are eligible only when
   `vision.messaging_apps` is true (default false). Every other app is never captured.
5. Nothing vetoes it: the app is not in the host's never-capture apps (ubiqX:
   `settings.blocked_apps` and `settings.vision_denied_apps`); no such app has a window on
   screen (`ScreenCapturer::visible_apps`); the host is not `NeverCapture` (the built-in
   `never-capture.txt`: banks, government, webmail, document suites, health portals, password
   managers; plus ubiqX `settings.blocked_domains`) and not `Allowed`.
6. The class and scope allow it:

   | `vision.scope` | Checked |
   |---|---|
   | `signals` | `Mixed` hosts; any host or title with a signal (2.5); a private-browsing window on an `Unknown` host when `vision.include_private_windows` is true |
   | `unknown` (the choice marked "recomendado", because it is what the request asked for) | the above, plus every `Unknown` host, browsers whose URL cannot be read, and media apps |
   | `off` | nothing |

   `Popular` hosts are checked only when a signal fires. `Adult` hosts are never captured:
   they are already decided.
7. Dwell and cadence for the context `(app_id, host or hashed title)`, kept in memory only:

   | Context | Checks at foreground dwell of | Then | Stops |
   |---|---|---|---|
   | normal | 6 s, 40 s, 150 s | every 300 s | after three `none` in a row the context is "cleared" for 24 h or until restart; a new signal un-clears it |
   | priority (a signal, a `suggestive` or `unclear` answer, a mixed host) | 6 s, 25 s, 60 s | every 120 s | after five `none` in a row |

   At most five checks per context per hour. There is no idle gate, because watching video
   produces no input; instead a check is skipped when the frame has not changed (step 9).
8. Caps: `vision.max_per_hour` (default 12, range 1 to 40), `vision.max_per_day` (default
   120), `vision.monthly_cap_usd` (default 1.50, range 0.25 to 10.00, computed from
   `ai_usage` rows of kind `shield` in the local calendar month). In ubiqX the general
   `ai_monthly_budget_usd` also applies: if it is spent, no check is made. One check is in
   flight at a time; a pass that finds one in flight does nothing.
9. The frame passes two device-side tests computed by the capturer before encoding
   (`FrameStats`): `media_score >= 0.12` (share of 8×8 cells of a 96×96 downsample that look
   photographic: high local variance and many distinct quantised colours; a window of text,
   code, a spreadsheet or a form scores near zero) and the 64-bit difference hash is more
   than 6 bits away from the last frame of this context that was answered `none`. Skin-tone
   detection is not used: it is unreliable and biased across skin tones.

A near-uniform frame (existing `is_uniform`) is "no frame": no check, no error shown.

Expected volume, to be measured: with these gates a day of ordinary browsing produces tens
of checks, not hundreds. At 640 px on the default Anthropic model one check is about
US$ 0.0006 (estimate from the vision map's table; not measured), so the default monthly cap
is reached only by sustained use of unknown sites.

### 3.3 What is captured and sent

- New port method on `ScreenCapturer`:

  ```rust
  fn capture_frame(&self, window_id: u32, opts: FrameOpts) -> CoreResult<Frame>;
  pub struct FrameOpts { pub max_edge: u32, pub crop_top_frac: f32 }
  pub struct Frame { pub image: EncodedImage, pub stats: FrameStats }
  pub struct FrameStats { pub media_score: f32, pub dhash: u64 }
  ```

  The Guard always passes `max_edge = SHIELD_MAX_EDGE = 640` and `crop_top_frac = 0.12` for
  browsers (removes the tab strip, the address bar and usually the bookmarks bar, which show
  other tabs' titles), `0.0` for other apps. JPEG quality stays 60. At this size most body
  text is not legible; the consent text says "a maior parte do texto fica ilegível", not
  "all".
- The request (`crates/ubiqx-ai/src/explicit.rs`, `build_request(image, model)`): one system
  block, one user message `[Image, Text("Classify this screenshot.")]`, a closed JSON schema,
  `max_tokens = 64`, `usage_kind = AiUsageKind::Shield`, `timeout = 8 s`,
  `disable_thinking = thinking_disabled_for(model)`. It contains no URL, title, app name,
  language, user profile, category catalogue or identifier. A unit test asserts that the only
  text parts are the two constants.
- Model: the host's vision model id (`settings.models.vision`); no new field in `AiModels`.
- The image bytes and the base64 string are dropped when the call returns. Nothing is written
  under the data directory. `client.rs` and `openai.rs` trace logging is changed to print
  `<image: N bytes>` instead of the base64 body, for every caller.
- A button "Ver um exemplo do que é enviado" on the Guard page captures the current
  foreground window with the same options and shows it in the page. It is not sent and not
  stored.

### 3.4 Prompt and response schema

System prompt (constant `EXPLICIT_SYSTEM_PROMPT`, English for every provider):

```
You are a content classifier inside a self-control app that an adult installed on their own
computer to keep away from pornography. You receive one downscaled screenshot of the window
that person is looking at. Decide whether the window is showing pornography.

Answer with JSON only: {"verdict": "explicit" | "suggestive" | "none" | "unclear"}

explicit   - sexual acts, or exposed genitals, anus or female nipples shown in a sexual
             context: photographs, video frames, drawings, animation or generated images.
             A page of thumbnails of such content counts.
suggestive - sexualised but not explicit (erotic poses in lingerie or swimwear, blurred or
             cropped previews), or you suspect pornography but cannot see it clearly.
none       - everything else, including medical or educational anatomy, breastfeeding,
             classical art, sport, beach photographs, news and any page that is mostly text.
unclear    - the image is unreadable or you cannot classify it.

Do not describe the image. Do not transcribe text. Do not identify anyone.
```

Schema: `{"type":"object","properties":{"verdict":{"type":"string","enum":["explicit",
"suggestive","none","unclear"]}},"required":["verdict"],"additionalProperties":false}`.

Port (`ubiqx_core::ports`):

```rust
pub enum ExplicitVerdict { Explicit, Suggestive, None, Unclear }
pub struct ExplicitCheck { pub verdict: ExplicitVerdict, pub refused: bool }
#[async_trait]
pub trait ExplicitDetector: Send + Sync {
    async fn check(&self, image: &EncodedImage, model: &str) -> CoreResult<ExplicitCheck>;
}
```

### 3.5 Provider specifics

- All four providers go through the existing `RoutingLlmClient`; no wire-format change.
- `LlmExplicitDetector::check` maps `Err(CoreError::AiRefused)` to
  `ExplicitCheck { verdict: Unclear, refused: true }` (the clients discard a refused answer
  after billing it). Invalid JSON or an unknown verdict string is `Unclear`.
- Truncation (`StopReason::MaxTokens`): one retry with `max_tokens = 512`, then `Unclear`.
- `openai.rs::reasoning_effort_for` gains the `Shield` arm with the same values as `Vision`.
- The service wraps the call in `tokio::time::timeout(12 s)`: the per-client retry policy
  (four attempts, backoff to 30 s) must not turn a check into minutes.
- Managed provider ("IA do Ubi"): screenshots would pass through the operator's server in
  memory. It is therefore off unless the user ticks `vision.allow_managed` under the sentence
  "As imagens da verificação passam pelo servidor do ubiqX a caminho da Anthropic. O servidor
  não guarda imagens." The proxy gets an explicit `DefaultBodyLimit` of 4 MiB, a test that an
  image block passes through unchanged and a test that no request body reaches the log. The
  request is not tagged as a Guard request: the ledger must not learn which subscribers use
  the Guard.
- Every provider applies its own usage policy and retention to what it receives. The consent
  text says so (section 3.8) and question 3 asks the owner to read each provider's terms for
  this use before launch.

### 3.6 Thresholds and reactions (`shield/policy.rs`, pure)

| Outcome | Reaction |
|---|---|
| `explicit` | A **vision hit**. Default `vision_action = hide_and_ask`: `Enforcer::minimize_app`, then the ask prompt (message `vision.soft`, buttons "Fechar a janela" and "Não era isso"; it stays 60 s and leaves the window minimised if unanswered). "Fechar" closes the tab (browser with a known host) or quits the app (the user asked), learns the host (2.7) and shows the support prompt. With the user-chosen `close_and_tell`, a browser tab is closed at once and the prompt says `vision.closed`; a non-browser app is still only minimised, never quit without a click. |
| `suggestive` | No action, no prompt. The context becomes priority. |
| `none` | Counts towards clearing the context. |
| `unclear` or refused | No action. The context becomes priority. Two in a row on a context that also has a local signal show the ask prompt without minimising anything. |
| timeout, network error, 5xx, rate limit | No action. Vision health becomes `backoff(until)` with steps of 1, 5 and 15 minutes; the Guard page shows it. The shared `AiHealth` is not touched. |
| key rejected (401, 403) three times in a row | Vision health `key_problem`; one in-app note "A verificação por IA parou: a chave foi recusada." |
| cap or budget reached | Vision health `limit_reached` until the next hour, day or month. |

In ubiqX, any hit (list, keyword or vision) also calls `ShieldHost::on_hit`, which makes the
tracker forget the content (section 7.5).

### 3.7 No AI, offline, unsupported

With no provider, with `local_only`, offline, on Wayland, without the macOS Screen Recording
permission, or on the AppImage with no system tiers, the Guard still runs every tier it has
and says which one is missing on the Guard page and during onboarding: "Sem a verificação
por IA, sites desconhecidos só são barrados pelo DNS filtrado e pelas listas." Nothing is
queued for later: a screen that was not checked is never checked afterwards.

### 3.8 What is stored, and what the user is told

Stored per check: `shield_day` counter `vision_check` (and `vision_hit`, `vision_not_it`
when they happen) for the local day and hour, and the normal `ai_usage` row (kind `shield`,
model, tokens, cost, time of the call). No image, no host, no title, no verdict row.

Consent text for this tier (PT-BR master, English twin in the bundle):

> Quando você abre um site que as listas não conhecem, o Guarda tira uma foto reduzida só
> dessa janela e pergunta à IA que você configurou (<provedor>) se há pornografia nela. A
> resposta é uma palavra. A foto não é gravada no seu computador e o ubiqX não a recebe.
> O provedor de IA recebe a imagem e aplica as regras e os prazos de retenção dele.
> Janelas de bancos, e-mail, documentos e os apps que você marcou como privados nunca são
> fotografados. Janelas anônimas são verificadas: [ligado].

---

## 4. Counselling

### 4.1 Surfaces

One new always-on-top window, label `guard`, route `#/guard-prompt` outside the `Gate`,
capability file `capabilities/guard.json`. It is fed in memory: the shell keeps the current
`GuardPrompt` and the page asks for it with `shield_prompt_current`. There is no sample
fallback and nothing is read from a table. The window shows UBI in the calm `idle` mood, not
`worried`.

```rust
pub struct GuardPrompt {
    pub id: Id,
    pub kind: PromptKind,          // Blocked | VisionAsk | VisionClosed | Repeat | Drift | PendingDue | Help
    pub message_key: String,       // key in the reviewed bank
    pub message: String,           // already localised by the service
    pub discreet: bool,
    pub actions: Vec<PromptAction>, // Ok | OpenSupport | CloseWindow | NotIt | Surf | StepAway | TalkToSomeone | Restore
    pub expires_at: DateTime<Utc>,
}
```

The type has no host, app or title field (D4). The fallback OS notification, used when the
window cannot be shown, is always title "UBI" and body "Página bloqueada." (or "O Guarda
precisa da sua atenção." for drift).

| Prompt | Size | Lifetime | Contents |
|---|---|---|---|
| Blocked, discreet (default) | 420×132 | 12 s | "Página bloqueada pelo UBI." and two buttons: "Abrir apoio", "Ok" |
| Support (after "Abrir apoio", or directly when `ui.discreet` is false) | 420×560 | until closed | one message from the bank, the user's values sentence if there is one, the if-then plan that matches the last trigger if there is one, then "Surfar a vontade (3 min)", "Sair da tela por 10 min", "Falar com alguém", "Fechar" |
| Vision ask | 420×200 | 60 s | `vision.soft`, "Fechar a janela", "Não era isso" |
| Repeat (third list hit in 10 minutes, counted in memory) | as Support | until closed | `block.repeat`, with the surf button first |

Timing: enforcement first, then the prompt; at most one prompt per 60 s; while a prompt or a
surf session is open no new prompt replaces it.

### 4.2 Copy principles (binding for every string)

The tone guide of the counselling report, section 6, is adopted whole. The rules that tests
enforce:

1. The user decided; UBI keeps the agreement ("como combinamos", "a lista que você montou").
2. One or two sentences and one action per screen.
3. Tentative about anything the AI saw ("Posso estar enganado").
4. No diagnosis, no neuroscience, no numbers about other people, no promise.
5. No assumption about gender, partner, religion or orientation; gender-neutral Portuguese.
6. "Sou um mascote, não um terapeuta."
7. A path to a human on every Guard screen.
8. A forbidden-word list (`viciado`, `vício`, `dependente`, `recaída`, `fracass`, `limpo`,
   `sujo`, `puro`, `pecado`, `vergonha`, `força de vontade`, `homem de verdade`, `dopamina`,
   `reprogramar`, `reboot`, `detox`, `comprovado`, `nunca mais`, `para sempre`, `nofap`, and
   their English twins) is checked against the `guard` i18n namespace and the Rust bank.

The 40 messages of section 7 of that report are the v1 bank, in
`crates/ubiqx-core/src/shield/copy.rs` (`fn message(key: MessageKey, lang: UiLanguage)`),
mirrored for the mock in `apps/desktop/src/i18n/messages/guard.ts`. The model never writes a
sentence the user sees (D10).

### 4.3 Elements and the finding behind each

| Element | What it is in v1 | Backed by | What it stores |
|---|---|---|---|
| Blocking itself | tiers 0 to 4 | stimulus control (standard CBT relapse-prevention component; no isolated trial for this use) | counters only |
| Urge surf | a 3-minute, six-screen timer (messages `surf.start` to `surf.end`); reachable from every prompt, the tray, the Guard page and the panic sheet; the 0 to 10 rating is shown back in the session and not kept | mindfulness-based relapse prevention and ACT (Crosby and Twohig 2016; Holas et al. 2020; Bowen and Marlatt 2009). The copy says the exercise changes what you do with the urge, not that the urge shrinks or passes in N minutes | `surf_started`, `surf_done` counts |
| Values sentence | one sentence, asked once in onboarding ("Pra que você quer isso?"), shown back in the Support prompt and in the disable flow | ACT | the sentence, in `shield_notes` |
| If-then plans | up to three ("Se eu sentir vontade quando ___, então eu vou ___") | implementation intentions (Gollwitzer and Sheeran 2006; untested for this use) | the plans, in `shield_notes` |
| Check-in | off by default; when on, one in-app question a day at the hour the user chose, four answers, optional trigger chips (fome, raiva, solidão, cansaço, tédio, estresse); never an OS notification | HALT is used as a prompt only; the app never tells the user what their trigger is | mood and trigger counts |
| Lapse flow | started only by the user ("Aconteceu um deslize"). A vision hit is never recorded as a lapse. Screens: `lapse.first`, `lapse.kind`, optional trigger chips, optional plan edit, `lapse.resume` | abstinence violation effect; self-compassion findings (Reid et al. 2014) | `lapse` and trigger counts |
| Progress | "Nos últimos 30 dias, N foram do jeito que você queria" always; the consecutive-day streak is off by default and, when on, is small and never animated | Silverman and Barasch 2023: a broken streak lowers engagement | derived from `shield_day` |
| Panic button | tray item "Preciso de apoio agora", a button on the Guard page and in every prompt; sheet with three choices: surf (3 min), step away (the Guard minimises the front window and shows a 10-minute countdown; nothing is locked), talk to someone (crisis card) | delay and distract; crisis path | `panic` count |
| Professional-help suggestion | card `help.pattern` then the Help page, at most once in 7 days, when in the last 7 days there were 5 lapses, or 10 loosening requests, or 2 "muito difícil" check-ins. Constants `HELP_LAPSES_7D`, `HELP_OVERRIDES_7D`, `HELP_VERY_BAD_7D` in `shield/stats.rs` | ICD-11 6C72 description; Bőthe et al. 2024 treatment gap. These are product heuristics, not clinical cut-offs, and the card says "isso não é um diagnóstico" | `help_shown` count |
| Crisis card | CVV 188, SAMU 192, CAPS, "psicólogo com registro ativo no CRP", findahelpline.com; one step from every Guard screen | counselling report, section 3 | nothing |

Resources live in `crates/ubiqx-core/src/shield/resources.json`. Each entry has
`verified_on` and `verified_by`. A release-mode test fails when an entry lacks them, so a
person has to open each site and dial-check each number before a release; DASA, AISEP and
Sexólicos Anônimos are not in the file. Message `panic.crisis` reads its numbers from the
file.

What the app never claims, in the product, the site or a store listing: the eight items of
section 1.5 of the counselling report (addiction, dopamine reset, sexual-function or hormone
benefit, "superpowers", any success rate, that masturbation is harmful or abstinence the
goal, that it treats or diagnoses, that the block is unbreakable). The user sets the goal;
the app has no masturbation-abstinence mode.

---

## 5. Commitment

### 5.1 The one mechanism: a delay

`ShieldConfig.commit.delay_minutes`, one of 0, 5, 10, 30, 60, 240, 1440; default 10, chosen
in onboarding. Pure state machine in `crates/ubiqx-core/src/shield/commit.rs`:

```rust
pub enum Change { Disable, RemoveSystemLayer(Layer), SetVisionScope(VisionScope),
                  SetVisionAction(VisionAction), RemoveBlockedSite(Id), AddAllowedSite(String),
                  UntickMixedHost(String), ForgetLearned, LowerDelay(u32), /* tightening: */
                  Enable, AddBlockedSite(String), RemoveAllowedSite(Id), RaiseDelay(u32), SetUi(UiPrefs), .. }
pub fn direction(change: &Change, current: &ShieldConfig) -> Direction;        // Tighten | Neutral | Loosen
pub fn request(change: Change, cfg: &ShieldConfig, now: DateTime<Utc>) -> Outcome;   // Applied(cfg) | Pending(PendingChange)
pub fn due(pending: &[PendingChange], now: DateTime<Utc>) -> Vec<PendingChange>;
```

- Tightening and cosmetic changes apply at once.
- A loosening change is stored in `shield_pending` with `effective_at = now + delay` and
  applied by `system_pass` when due. Lowering the delay is itself a loosening change, so it
  takes effect only after the current delay (the Plucky rule).
- Cancelling a pending change is instant and always possible.
- While a change is pending, the Guard page shows a quiet countdown, the values sentence and
  the surf button. There is no second or third warning and no text that argues.
- When `Disable` becomes due, tier 0 and tier 4 stop. Removing the system tiers needs the OS
  prompt, so the page then offers "Remover a proteção do sistema"; until the user does it the
  system tiers stay (fail-safe).
- Each loosening request increments the `override_request` counter (used only for the
  help suggestion).

Because configuration lives in its own table and changes only through
`ShieldService::request_change`, neither `update_settings` nor a stale settings object from
another window can switch the Guard off (D7). "Apagar todos os dados" keeps the Guard's
configuration and lists (section 6.4).

### 5.2 "Não era isso" (correction, not an off switch)

After a vision hit or a heuristic hit, one tap records a 7-day allow for that host
(`shield_sites`, `origin = not_it`, `expires_at`), or for a non-browser app a 2-hour
in-memory exemption. It is immediate, because it corrects an uncertain judgement. It is not
offered for a host on the built-in list. The first three per local day are immediate; from
the fourth the allow goes through the delay.

### 5.3 Exits that are never delayed, by design

| Exit | Why it has no delay |
|---|---|
| "Restaurar minha rede" (removes tier 2 only) | a broken network is a safety problem; the hosts block and policies stay, and the page offers to re-apply |
| `ubi-guard-helper remove` with the administrator password, `ubiqx guard remove`, uninstalling | the machine belongs to the user; hiding the way out would make this a control tool |
| "Apagar meu histórico do Guarda" | erasing one's own data is a right, and it does not weaken any tier |
| Cancelling a pending change | it only tightens |
| Quitting the app from the tray | tiers 1 to 3 remain; nothing is recorded about the time the app was closed |

### 5.4 Deliberately not built, and why

| Not built | Reason |
|---|---|
| Accountability partner, partner alerts, weekly reports to anyone | no controlled evidence; coercion and privacy risk; Google Play's stalkerware policy forbids tracking another adult even with consent; it is the surveillance model the category was removed for |
| Any log or report of sites, screens or verdicts | D3, D4 |
| Password held by another person, lock that cannot be ended, whole-computer lock | lock-out risk and a control relationship |
| Blocking the uninstall, hiding the tray icon, a silent or stealth mode, process self-protection, a watchdog that restarts the app | stalkerware affordances; the app is always visible while the Guard is on |
| Silent re-application of system tiers, a privileged background daemon | needs a signed helper; and a tool that repairs itself without asking is one the user cannot trust to leave |
| Punishment for changing the protection (longer block, shaming text) | conflicts with the no-shame rule |
| Detecting or killing VPN, Tor or proxy processes | false positives on work VPNs; scanning processes is surveillance |
| Local DNS proxy, network extension, `pf` rules, WFP filters | fail closed, or need Apple entitlements (DNS report, section 5) |
| Disabling incognito mode | intrusive, and the Guard already reads private windows |
| Learned sites in the hosts file; SafeSearch pinned in hosts | D5; breakage risk |
| AI-written coaching, chat, community, leaderboards, shared streaks | D10; forum involvement is associated with more distress after a lapse |
| Screening questionnaires | licence and Portuguese validation not checked |
| Cloud sync of Guard data, analytics, telemetry about Guard use | the Quittr leak; there is no server-side Guard data at all |
| Mobile | different enforcement and store rules |
| Installing or configuring the Guard on someone else's device, remote configuration | out of scope by definition |

Protection against misuse on another adult: the Guard produces nothing a second person can
read (no history, no images, no report, no remote view), its tray icon cannot be hidden, a
reminder appears every 30 days ("O Guarda está ligado neste computador. Veja como desligar."
with a link to the Help page), and the removal instructions are in the app and the docs.

---

## 6. Data model

### 6.1 Configuration (not in `Settings`)

`ShieldConfig`, one JSON row in `shield_config`, `#[serde(default)]` on every struct so that
`{}` loads as the defaults (test), defined in `crates/ubiqx-core/src/shield/model.rs`:

```rust
pub struct ShieldConfig {
    pub v: u8,                               // 1
    pub enabled: bool,                       // false
    pub consent: Option<ShieldConsent>,      // { version: 1, at, vision_scope, private_windows, managed }
    pub guard: GuardPrefs,                   // vision_action: HideAndAsk; strict_mixed: Vec<String>; escorts: true; stories: true
    pub vision: VisionPrefs,                 // scope: Off; include_private_windows: true; media_apps: true; messaging_apps: false;
                                             // allow_managed: false; max_per_hour: 12; max_per_day: 120; monthly_cap_usd: 1.5
    pub system: SystemPrefs,                 // hosts: false; resolver: None; policies: false
    pub commit: CommitPrefs,                 // delay_minutes: 10
    pub lists: ListPrefs,                    // auto_update: true
    pub ui: UiPrefs,                         // discreet: true; streak_visible: false; daily_checkin: false; checkin_hour: 21
}
```

`Settings` gets no new field. The TypeScript mirror is `apps/desktop/src/shield/types.ts`.
`ShieldConfig::sanitize()` clamps the numeric ranges; `ShieldService::request_change` is the
only writer.

### 6.2 Tables (migration V3 in `crates/ubiqx-storage/src/migrations.rs`)

```sql
CREATE TABLE shield_config  (id INTEGER PRIMARY KEY CHECK (id = 1), json TEXT NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE shield_meta    (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL, updated_at INTEGER NOT NULL);
   -- install_salt, install_id, lists_version, lists_checked_at, streak_started, help_last_shown, reminder_last_shown
CREATE TABLE shield_pending (id TEXT PRIMARY KEY NOT NULL, change TEXT NOT NULL, requested_at INTEGER NOT NULL, effective_at INTEGER NOT NULL);
CREATE TABLE shield_sites   (id TEXT PRIMARY KEY NOT NULL,
                             list TEXT NOT NULL,          -- block | allow
                             origin TEXT NOT NULL,        -- user | learned | not_it
                             host TEXT,                   -- NULL when origin = learned
                             host_key BLOB NOT NULL,      -- 16 bytes, salted
                             created_at INTEGER NOT NULL, expires_at INTEGER,
                             UNIQUE (list, host_key));
CREATE TABLE shield_day     (day TEXT NOT NULL,           -- local YYYY-MM-DD
                             kind TEXT NOT NULL, hour INTEGER NOT NULL, n INTEGER NOT NULL,
                             PRIMARY KEY (day, kind, hour));
CREATE TABLE shield_notes   (id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL,   -- values | plan
                             a TEXT NOT NULL DEFAULT '', b TEXT NOT NULL DEFAULT '',
                             position INTEGER NOT NULL DEFAULT 0, updated_at INTEGER NOT NULL);
```

`shield_day.kind` values: `block_list`, `block_user`, `block_keyword`, `vision_check`,
`vision_hit`, `vision_not_it`, `surf_started`, `surf_done`, `panic`, `lapse`, `checkin_ok`,
`checkin_meh`, `checkin_hard`, `checkin_very_bad`, `trigger_hungry`, `trigger_angry`,
`trigger_lonely`, `trigger_tired`, `trigger_bored`, `trigger_stressed`, `override_request`,
`network_restore`, `help_shown`. The column is free text read with a lenient parser (unknown
kinds are skipped), so an older build reading a newer database does not fail the query.

There is no row per event, no exact time, no host, no app. The `interventions` table is
never written by the Guard.

The three assertions on schema version 2 (`migrations.rs:273`, `tests/repos.rs:234`, `:242`)
become 3. Both products ship the same migration list.

Port (`ubiqx_core::ports::ShieldRepo`): `load_config`, `save_config`, `meta_get`, `meta_set`,
`list_pending`, `insert_pending`, `delete_pending`, `list_sites`, `upsert_site`,
`delete_site`, `delete_sites_by_origin`, `purge_expired_sites(now)`, `bump_day(day, kind,
hour, by)`, `days_between(from, to) -> Vec<DayCount>`, `delete_days_before(day)`,
`list_notes`, `upsert_note`, `delete_note`, `erase_history()`.

### 6.3 Events

`ubiqx_core::shield::model::ShieldEvent` (serde `tag = "type"`, snake_case):
`status_changed { status }`, `prompt { prompt }`, `prompt_closed { id }`,
`pending_changed`, `system_changed { status }`, `vision_health { health }`,
`lists_updated { version }`. No variant carries a host. In ubiqX they travel as one new
variant `EngineEvent::Shield { event }` (one arm in the CLI's match, one case in the store).
In Leme they are emitted on the channel `guard`.

### 6.4 Retention, erase, export

| Data | Kept for | "Apagar todos os dados" (ubiqX) | "Apagar meu histórico do Guarda" | Export |
|---|---|---|---|---|
| `shield_config`, `shield_pending` | until changed | kept (otherwise the wipe would be an instant off switch) | kept | config without salt |
| `shield_sites` typed by the user | until removed | kept | kept | yes |
| `shield_sites` learned | until "Esquecer sites aprendidos" | kept | kept | count only |
| `shield_sites` `not_it` | 7 days | kept | deleted | no |
| `shield_day` | 90 days, purged daily by `system_pass` | deleted | deleted | yes |
| `shield_notes` | until edited | deleted | deleted | yes |
| `shield_meta` `streak_started`, `help_last_shown` | | deleted | deleted | no |
| `ai_usage` rows of kind `shield` | as today | deleted (already in `WIPE`) | deleted (new `UsageRepo::delete_kind`) | no |
| images | never stored | | | |
| system state file | until the tiers are removed | untouched | untouched | no |

`WIPE` in `repos/maintenance.rs` gains `shield_day` and `shield_notes` (array length 11) and
deletes the two meta keys; the port doc and `delete_all_data` doc are updated.

"Exportar dados do Guarda" is its own button and writes its own file. Guard data is not part
of ubiqX's `export_json`, because that export is the one a person sends to someone else.

At rest the database is not encrypted, as today. The privacy text says so: "Quem tem acesso
à sua conta neste computador pode ver que o Guarda está instalado e ler suas frases e
planos."

---

## 7. Code architecture

### 7.1 Crates

| Crate | New | Role | Depends on |
|---|---|---|---|
| `ubiqx-core` | module `shield/` | types, pure decisions, copy, tables; ports in `ports.rs` | no new dependency |
| `ubiqx-storage` | `repos/shield.rs`, V3 | `impl ShieldRepo for SqliteStore` | |
| `ubiqx-shield-sys` | new library | hosts block, resolver and policy plans, state file, helper logic, elevation launcher, `NativeSystemShield` | `ubiqx-core`, `serde_json`, `sha2`, `ed25519-dalek`, `flate2`; `windows` on Windows |
| `ubi-guard-helper` | new binary | `fn main() { exit(ubiqx_shield_sys::helper::run(args)) }` | `ubiqx-shield-sys` |
| `ubiqx-platform` | `capture_frame`, `minimize_app`, `system_shield` field, mocks | re-exports `NativeSystemShield` | `ubiqx-shield-sys` |
| `ubiqx-ai` | `explicit.rs`, `lists_fetch.rs`, `wiring.rs`, fake | `LlmExplicitDetector`, `HttpListFetcher`, and the client wiring moved out of `ubiqx-app` (`RemoteClients`, `SecretStoreKey`, `StoredProvider`, `StoredLanguage`) so both composition roots use it | |
| `ubiqx-shield` | new library | `ShieldService`: the Guard pass, vision checks, system pass, commitment, read models | `ubiqx-core` only (storage, platform, ai as dev-dependencies) |
| `ubiqx-engine` | hosts the service | two loops, `EngineHost`, tracker hooks | `ubiqx-shield` |
| `ubiqx-app` | wiring | passes ports, exposes `App.shield` | |
| `ubiqx-shield-tauri` | new library | Tauri commands, the `guard` window, `TauriShieldPresenter`, helper path resolution; used by both shells | `tauri`, `ubiqx-shield`, `ubiqx-core` |
| `ubiqx-shield-app` | new library | composition root of the standalone product: `ShieldApp::start(config, sink)` | core, storage, platform, ai, shield. **Not `ubiqx-engine`, not `ubiqx-app`.** |
| `apps/leme/src-tauri` (crate `leme-desktop`) | new shell | tray, main window, updater, autostart, single instance | `ubiqx-shield-app`, `ubiqx-shield-tauri` |

### 7.2 Module layout in core (`crates/ubiqx-core/src/shield/`)

`mod.rs`, `model.rs` (config, prompt, events, status types), `matcher.rs`, `heuristics.rs`,
`classify.rs`, `lists.rs` (`Lists`, manifest verification, canaries), `gate.rs`
(`VisionGate`), `policy.rs` (reaction to a class or a verdict: `fn decide(..) -> Decision`),
`commit.rs`, `copy.rs`, `stats.rs` (progress, help thresholds), `sys.rs` (`SystemPlan`,
`SystemStatus`, `LayerState`, `RESOLVERS`, `ProductId`), `resources.rs` + `resources.json`,
`identity.rs` (`ProductIdentity`).

New ports in `ports.rs`: `ShieldRepo`, `SystemShield`, `ExplicitDetector`, `ShieldPresenter`
(`fn show(&self, prompt: &GuardPrompt) -> CoreResult<bool>`, `fn dismiss(&self, id: &str)`),
`ShieldSink` (`fn emit(&self, event: ShieldEvent)`), `ListFetcher`
(`async fn fetch(&self, name: &str, etag: Option<&str>) -> CoreResult<Fetched>`),
`ShieldHost`; plus `ScreenCapturer::capture_frame`, `Enforcer::minimize_app`,
`AiUsageKind::Shield`, `UsageRepo::totals_for_kind` and `UsageRepo::delete_kind`.

```rust
pub trait ShieldHost: Send + Sync {
    fn language(&self) -> UiLanguage;
    fn ai_gate(&self) -> AiGate;                    // Ready { model, managed } | NoProvider | LocalOnly | NotConsented | BudgetSpent
    fn never_capture_apps(&self) -> Vec<String>;
    fn never_capture_domains(&self) -> Vec<String>;
    fn on_hit(&self, hit: &HitContext);             // in-memory app id, title key and host; never persisted by the service
}
```

### 7.3 The service (`crates/ubiqx-shield/src/`)

Shaped like `UpdateChecker`: it takes ports and a clock, owns its state, and knows nothing
about `EngineState`.

```rust
pub struct ShieldDeps {
    pub activity: Arc<dyn ActivitySource>, pub urls: Arc<dyn BrowserUrlResolver>,
    pub capturer: Arc<dyn ScreenCapturer>, pub enforcer: Arc<dyn Enforcer>,
    pub permissions: Arc<dyn PermissionChecker>, pub system: Arc<dyn SystemShield>,
    pub detector: Option<Arc<dyn ExplicitDetector>>, pub fetcher: Arc<dyn ListFetcher>,
    pub repo: Arc<dyn ShieldRepo>, pub usage: Arc<dyn UsageRepo>,
    pub presenter: Arc<dyn ShieldPresenter>, pub notifier: Arc<dyn Notifier>,
    pub sink: Arc<dyn ShieldSink>, pub host: Arc<dyn ShieldHost>, pub clock: Arc<dyn Clock>,
    pub identity: ProductIdentity, pub lists_dir: PathBuf,
}
impl ShieldService {
    pub fn new(deps: ShieldDeps) -> CoreResult<Arc<Self>>;
    pub fn guard_pass(&self) -> CoreResult<Option<VisionJob>>;       // blocking: foreground, URL, classify, enforce; captures only when the gate asks
    pub async fn vision_check(&self, job: VisionJob) -> CoreResult<()>;   // the only network call; bounded to 12 s
    pub async fn system_pass(&self) -> CoreResult<()>;               // status, due changes, retention, list update, reminder
    pub fn status(&self) -> ShieldStatus;  pub fn config(&self) -> ShieldConfig;
    pub fn request_change(&self, change: Change) -> CoreResult<ChangeOutcome>;
    pub fn cancel_pending(&self, id: &str) -> CoreResult<()>;
    pub fn consent(&self, consent: ShieldConsent) -> CoreResult<()>;
    pub fn system_preview(&self) -> CoreResult<Vec<PlannedChange>>;
    pub fn system_apply(&self) -> CoreResult<ApplyReport>;           // blocking on the OS prompt
    pub fn system_remove(&self) -> CoreResult<ApplyReport>;          // only when disabled or due
    pub fn restore_network(&self) -> CoreResult<ApplyReport>;
    pub fn prompt_action(&self, id: &str, action: PromptAction) -> CoreResult<()>;
    pub fn panic(&self) -> CoreResult<()>;  pub fn surf_event(&self, e: SurfEvent) -> CoreResult<()>;
    pub fn checkin(&self, mood: Mood, triggers: &[Trigger]) -> CoreResult<()>;
    pub fn lapse(&self, triggers: &[Trigger]) -> CoreResult<()>;
    pub fn progress(&self) -> CoreResult<Progress>;  pub fn coverage(&self) -> Coverage;
    pub fn sites(&self) -> CoreResult<Vec<SiteView>>;  pub fn notes(&self) -> CoreResult<Notes>;  pub fn set_notes(&self, n: Notes) -> CoreResult<()>;
    pub fn sample_frame(&self) -> CoreResult<EncodedImage>;          // "ver um exemplo"; never sent
    pub fn erase_history(&self) -> CoreResult<()>;  pub fn export(&self) -> CoreResult<serde_json::Value>;
    pub fn capture_veto(&self, app_id: &str, title: &str, url: Option<&str>) -> bool;   // for the tracker
    pub fn is_adult_host(&self, host: &str) -> bool;                                    // for the segmenter
}
```

Files: `service.rs`, `guard.rs` (the pass), `vision.rs` (job, caps, health), `system.rs`,
`runtime.rs` (in-memory contexts, cooldowns, repeat counter, local site sets), `views.rs`.

`guard_pass` returns before touching the platform when the Guard is off. It never calls a
network function. The frame it captures is handed to `vision_check` inside the `VisionJob`
and dropped there.

### 7.4 Hosting in ubiqX (`ubiqx-engine`)

- `EngineState.shield: Arc<ShieldService>` built in `EngineState::new`; `EngineHandle::shield()`.
- `LoopConfig.shield_guard_every` (2 s) and `shield_system_every` (60 s, first after 10 s);
  `shield_guard_loop` copies `focus_loop` (skip the first tick, `spawn_blocking` for the
  pass) and then awaits `vision_check` when a job came back; `shield_system_loop` copies
  `license_loop`. Tests park both at 3600 s.
- `PlatformPorts` gains `system_shield` and `shield_presenter`; `AiPorts` gains
  `explicit: Option<Arc<dyn ExplicitDetector>>` and `list_fetcher`; `Repos` gains `shield`.
  The literal sites listed in section 12E of the engine map are updated.
- `EngineHost` implements `ShieldHost` from `EngineState`: `ai_gate` uses `local_only`,
  `onboarding_done`, the provider key, the licence for the managed provider and the general
  budget (read-only: it does not call `budget_allows`, which has side effects);
  `never_capture_*` come from `settings.blocked_apps`, `vision_denied_apps` and
  `blocked_domains`.
- `key_health` is unchanged: the Guard does not use `AiHealth`.

### 7.5 What the tracker must forget (ubiqX only)

A Guard hit must not leave traces in the productivity data.

1. `SegmenterConfig` gains `adult: Option<Arc<dyn AdultOracle>>` (trait in core, one method
   `is_adult_host`); `Segmenter::is_blocked` returns true for such a host, so the sample
   becomes `[privado]` like an incognito window: no title, no URL, no screenshot, no AI
   payload, no report line, no export.
2. `EngineHost::on_hit` calls `tracker::scrub_open_block(state)`: the open block gets the
   private title, `url = None`, category `sys-private`; its screenshots are deleted
   (new `ScreenshotRepo::delete_for_block`, files unlinked); the segmenter treats the same
   context key as private for the next 10 minutes.
3. `screenshots::should_capture` asks `shield.capture_veto(..)` and takes no productivity
   screenshot of a window whose host is `Adult` or `Mixed`, or which has a signal, an open
   suspicion or a vision hit in the last 10 minutes.

### 7.6 Hosting in the standalone product (`ubiqx-shield-app`)

`ShieldApp::start(config, sink)` opens the store, builds `PlatformServices`, the AI wiring
from `ubiqx_ai::wiring` and a `ShieldService`, and spawns the same two loops plus the update
check (`UpdateChecker` is constructed directly; it needs only ports). `LemeHost` implements
`ShieldHost` from the `Settings` row (provider, vision model, language, `local_only`) and a
"first-run done" flag in `shield_meta`.

What Leme does **not** run or link: the sampler, the tracker and segmenter, classification,
reports, nudges, the focus guard and focus sessions, screenshots retention, the advisor, the
timeline and every page that shows them. The tables for them exist in its database (one
migration list) and stay empty. Two CI checks hold this: `cargo tree -p leme-desktop -e
normal` must not contain `ubiqx-engine` or `ubiqx-app`, and an integration test runs a
scripted Leme session and asserts zero rows in `blocks`, `screenshots`, `interventions`,
`nudges` and `reports`.

### 7.7 Product identity

`ubiqx_core::shield::identity::ProductIdentity { id: ProductId, bundle_id, display_name,
exe_stems, keyring_service, feed_url, feed_product, user_agent }` with two constants,
`UBIQX` and `LEME`. The hard-coded values become reads of a process-wide
`OnceLock<ProductIdentity>` that defaults to `UBIQX` (existing tests unchanged):
`SELF_BUNDLE_ID` and the exempt lists in `core/focus.rs`, `SELF_STEMS` in
`platform/app_id.rs`, the literal in `macos/enforce.rs::hide_others_script`, the keyring
service in `macos/keychain.rs` and `secrets_file.rs`, the feed URL and product in
`update.rs`, the user agents. The exempt lists always contain both products, so neither acts
on the other. The update check refuses a feed whose `product` differs.

Leme uses its own identifier (`ai.ubiqx.leme`), data directory, database and keyring service:
an API key entered in ubiqX is not visible to Leme and the reverse. Both products share only
the machine-level state file and hosts marker (section 1.6). Both shells add
`tauri-plugin-single-instance`.

### 7.8 IPC (`crates/ubiqx-shield-tauri/src/commands.rs`)

All commands take `tauri::State<Arc<ShieldService>>` and return `Result<T, IpcError>`:
`shield_status`, `shield_config`, `shield_consent`, `shield_request_change`,
`shield_cancel_pending`, `shield_sites`, `shield_system_preview`, `shield_system_apply`,
`shield_system_remove`, `shield_restore_network`, `shield_coverage`, `shield_sample_frame`,
`shield_prompt_current`, `shield_prompt_action`, `shield_panic`, `shield_surf_event`,
`shield_checkin`, `shield_lapse`, `shield_notes`, `shield_set_notes`, `shield_progress`,
`shield_resources`, `shield_erase_history`, `shield_export`. Each shell lists them in its own
`generate_handler!`. `IpcError` moves to this crate and is re-used by the ubiqX shell.

Shell side: `TauriShieldPresenter` (window `guard`, sizes of 4.1, positioned like the
intervention window, `visible_on_all_workspaces`, not focused on show), `TauriShieldSink`
(event `guard`), tray item "Preciso de apoio agora" in both products. In ubiqX the tray's
pause and private-mode items get the tooltip "Não pausa o Guarda".

### 7.9 Frontend

Shared directory `apps/desktop/src/shield/`, used by both flavours:

- `types.ts`, `ipc.ts`, `store.ts` (its own zustand store, fed by the `guard` or `engine`
  event), `mock.ts` (handlers and state, spread into `lib/mock.ts`'s command map);
- pages: `GuardPage.tsx` (status per tier, coverage table, lists, vision, delay, pending
  changes), `GuardSetup.tsx` (the opt-in wizard), `GuardProgress.tsx`, `GuardHelp.tsx`,
  `GuardPrompt.tsx` (the floating window);
- components: `TierCard`, `CoverageTable`, `SystemPreview`, `SiteList`, `VisionExplainer`,
  `DelayPicker`, `PendingBanner`, `SurfTimer`, `PanicSheet`, `LapseFlow`, `CheckinCard`,
  `ValuesField`, `PlanEditor`, `CrisisCard`, `DriftCard`.

The wizard (`GuardSetup`), six steps, nothing preselected where a choice has a privacy cost:
(1) what the Guard is and is not ("sou um mascote, não um terapeuta"; adults, your own
computer; it is friction, not a wall); (2) on-device blocking and the hosts and policy tiers,
with the exact list of system changes; (3) third-party DNS: three radios; (4) vision: three
radios (`off`, `signals`, `unknown`) with the text of 3.8, the private-windows switch and,
for the managed provider, the extra tick; (5) the delay and the values sentence; (6) summary
of what is stored (counts only) and how to turn it off and remove it. Finishing records
`ShieldConsent` and then, if system tiers were chosen, shows the preview and the OS prompt.

ubiqX: nav item "Guarda" (`nav.guard`), route `/guard`, `/guard/progress`, `/guard/help`,
route `/guard-prompt` outside the `Gate`. The main onboarding is not changed; the Guard is
opt-in from its own page. Leme: build flavour `VITE_PRODUCT=leme` selects `main.leme.tsx` and
`AppLeme.tsx` (routes `/welcome`, `/`, `/progress`, `/help`, `/settings`), script
`build:leme` writing to `dist-leme`; `LemeSettings.tsx` re-uses the AI provider and key
form, language, launch at login, updates, licence and data sections.

i18n: one new namespace `apps/desktop/src/i18n/messages/guard.ts` (PT-BR and EN, flat keys)
registered in `NAMESPACES`; brand strings for Leme in `leme.ts`. Rust-side copy in
`shield/copy.rs`. `parity.test.ts` covers both.

### 7.10 Licence and plans

- The Guard's blocking, counselling and commitment are never gated by a licence, in either
  product. A lapsed subscription must never remove a block.
- Vision with the user's own key needs no plan. Vision through the managed provider needs a
  valid `monthly_managed` key, exactly as the managed provider does today, plus the extra
  consent tick.
- v1 adds no `Plan` variant and no claim: Leme accepts the existing `UBIQX-` keys and shows
  the existing licence section. `LICENSE_ENFORCEMENT` stays `Soft`. Selling Leme separately
  is question 6.

---

## 8. Naming and branding

1. Nothing a user, a store, a search engine or the operating system sees contains "NoFap":
   product name, window title, bundle id, binary and helper names, hosts marker, state
   directory, policy file names, release tag, i18n strings, site and docs for users.
   Reasons: NOFAP is a registered US mark (Reg. 4,591,410; 4,724,471; 5,160,831) with 2026
   filings reported in the UK (Class 9) and India; Brazil could not be searched and Brazilian
   law can protect a foreign mark without local registration; the owner litigates. And the
   word commits the product to a masturbation-abstinence stance it must not take.
2. Internal identifier: `shield` (crates `ubiqx-shield*`, module `ubiqx_core::shield`, tables
   `shield_*`, commands `shield_*`). The research directory `docs/nofap/` keeps its name as
   an internal working folder and is not shipped.
3. Module in ubiqX: **"Guarda do UBI" / "UBI Guard"**; nav label "Guarda" / "Guard".
4. Standalone product: **"Leme, by ubiqX"** (working name; "Maré" is the reserve). Subtitle:
   "Bloqueio de pornografia e apoio para mudar o hábito" / "Pornography blocking and support
   for changing the habit". Identifier `ai.ubiqx.leme`, binary `leme`, directory `apps/leme`.
   The name is one constant (`ProductIdentity::LEME`), one `tauri.conf.json` and one i18n
   file, so a rename after trademark clearance is a three-file change.
5. System-level names are product-neutral and shared: `ubi-guard` (hosts marker, state
   directory, helper `ubi-guard-helper`, policy file `ubi-guard.json`, polkit action
   `ai.ubiqx.guard`).
6. Test `no_forbidden_brand` (Rust and Vitest) fails on a case-insensitive match of `nofap`
   or `fapstronaut` in the i18n bundles, both `tauri.conf.json` files, capability files,
   `Info.plist`, every `Cargo.toml` `name` and `description`, `shield/copy.rs`,
   `resources.json`, the constants of `ubiqx-shield-sys` and the release script.
7. None of the names has been cleared. INPI (classes 9, 42, 44, 45) and USPTO searches and a
   domain check come before any public use.

---

## 9. Test plan

Conventions of the repository apply: pure logic in core with in-file unit tests, engine-style
integration tests with in-memory SQLite, `FixedClock` (2026-09-17 15:00 UTC), every loop
parked at 3600 s and passes driven by hand, Vitest against the mock for the frontend.

### 9.1 Unit tests (run on Linux CI)

| Area | File | Cases |
|---|---|---|
| Matcher | `core/src/shield/matcher.rs` | hash vector shared with the Python script; `normalize_host` (IDNA, port, trailing dot, IP literal, single label); parent walk; `parse` rejects bad magic, wrong count, unsorted and duplicate keys; embedded Core passes `canary-block.txt` and `canary-allow.txt` |
| Heuristics | `heuristics.rs` | one case per row of the blocklist report's tables: must hit (`porno365.golf`, `missav123.com`, `xvv1deos.com`, `x.xxx`), must not hit (`essex.gov.uk`, `planalto.gov.br`, `canaltech.com.br`, `motorola.com.br`, `drogariasaopaulo.com.br`, `emilfrey.de`, `peacocktv.com`, `sussexexpress.co.uk`, `lichngaytot.com`, `coroasparavelorio.com.br`); weak words are signals only; `gay` is neither; titles never produce `Adult` |
| Class | `classify.rs` | order of authority; longest match; allow beats block at equal length; expired `not_it` ignored; strict mixed; `NeverCapture` beats signals |
| Vision gate | `gate.rs` | every row of 3.2: scope table, dwell and cadence with the fixed clock, clearing after three `none`, priority after `suggestive`, per-context and global caps, frame tests (`media_score`, hash distance), no window id means no job |
| Reaction | `policy.rs` | every row of 3.6, including "non-browser app is never quit without a click" and "two unclear need a signal" |
| Commitment | `commit.rs` | direction of every `Change`; lowering the delay waits the old delay; cancel; due ordering; `not_it` daily limit |
| Copy | `copy.rs` | every key exists in both languages; forbidden words; no message contains a `{host}`-like placeholder; `panic.crisis` numbers come from `resources.json` |
| Resources | `resources.rs` | schema; release-mode test requires `verified_on` and `verified_by` |
| Stats | `stats.rs` | days-your-way over 30 days; help thresholds; at most one suggestion per 7 days |
| Config | `model.rs` | `{}` deserialises to defaults; sanitise clamps; `GuardPrompt` and `ShieldEvent` serialise without any `host`, `domain`, `url`, `title` or `app` key (asserted on the JSON) |
| Hosts block | `shield-sys/src/hosts.rs` | append, replace, remove; bytes outside the block preserved for LF and CRLF files, with and without a final newline; two address families; caps; canary line; hash in the marker; a foreign block with a similar marker is not touched |
| Plan validation | `shield-sys/src/plan.rs` | unknown verb, version, product, resolver id; too many names; invalid names; protected suffixes; path character whitelist |
| Resolver | `shield-sys/src/resolver/*.rs` | parsing of recorded `networksetup` outputs, PowerShell JSON fixtures, `resolvectl` fixtures; command plans as data for apply and undo; "restore only if still ours" |
| Policies | `shield-sys/src/policies/*.rs` | rendered JSON per browser; Firefox merge and restore; registry plan as data; existing foreign management detected |
| State file | `shield-sys/src/state.rs` | owners set; remove by one owner keeps the tier; idempotent apply and remove; half-removed state |
| Helper | `shield-sys/src/helper.rs` | argument parsing; exit codes; result file; every write goes through an injected `SystemFs` trait backed by a temp directory |
| Frame stats | `platform/src/image_util.rs` | text-like synthetic image scores under 0.12, photo-like noise over it; difference hash stable under re-encoding, different for different images; top crop |
| Detector | `ai/src/explicit.rs` | request shape (image first, the two constant texts only, 640 px image accepted, `max_tokens`, schema, usage kind, timeout); parse of the four verdicts; unknown string and invalid JSON are `Unclear`; `AiRefused` becomes `refused: true`; truncation retried once |
| Wire formats | `ai/tests/*.rs` (wiremock) | Anthropic and OpenAI-compatible bodies for the Guard request; `reasoning_effort` present for `Shield`; trace log contains `<image:` and no base64 |
| List fetch | `ai/src/lists_fetch.rs`, `core/src/shield/lists.rs` | signature, rollback refusal, hash mismatch, canary failure, atomic replace, embedded floor |
| Proxy | `services/ubi-api/tests/proxy.rs` | image block forwarded unchanged; body over the limit is 413; no body in captured logs |

### 9.2 Integration tests with the mock platform and a fake AI

`crates/ubiqx-shield/tests/guard.rs`, harness copied from `ubiqx-engine/tests/focus.rs`:
in-memory `SqliteStore`, settable foreground and URL, `MockEnforcer`, `SyntheticCapturer`
extended with scripted `FrameStats`, `MockSystemShield`, `RecordingShieldPresenter`,
`FakeExplicitDetector` (a queue of verdicts and of errors, with the images it received),
`FixedClock`.

1. Off by default: no platform call, no prompt, no row.
2. Listed host: two passes, then `CloseTab`; refused close falls back to blank, minimise,
   notify; enforced again on the next pass; one prompt per 60 s; `shield_day` has one
   `block_list` count; `interventions` is empty.
3. Stale URL: the host changes between two passes, nothing is closed.
4. Keyword host and adult TLD: enforced; "isso não é conteúdo adulto" creates a 7-day allow.
5. Unknown host, scope `unknown`: checks at 6 s, 40 s and 150 s; three `none` clear the
   context; a title signal un-clears it.
6. Scope `signals`: an unknown host without a signal is never captured; a mixed host is.
7. Never captured: `Popular` without a signal, `NeverCapture`, allowed hosts, an app in
   `never_capture_apps`, a blocked app visible on screen, a window without an id, a
   messaging app with the default settings, an `Adult` host.
8. `explicit` with the default action: minimise, ask prompt; "Fechar" closes and learns the
   host; the database then contains no string equal to that host (byte search of a dump);
   the next visit is a list-style hit with no capture.
9. `explicit` then "Não era isso": window untouched afterwards, 7-day allow, the fourth in a
   day is pending.
10. Refusal, timeout, 401, cap reached, budget spent: no action, the right vision health,
    and (hosted in the engine) `AiHealth` unchanged.
11. Commitment: disable is pending for the delay, the Guard still enforces meanwhile,
    cancel works, when due tier 0 stops and the system tiers are still reported `applied`.
12. System: preview lists changes; a declined prompt is `Permission`; a status difference
    yields one drift card and one notification per 24 h; restore-network removes only tier 2.
13. Erase history deletes `shield_day` and `shield_notes` and keeps configuration and sites.
14. Privacy sweep after a scripted session with twenty checks and three hits: no file under
    the data directory starts with the JPEG magic; no prompt, event or notification captured
    by the recorders contains any host used in the scenario; every `LlmRequest` received by
    the fake has exactly the constant texts.

`crates/ubiqx-engine/tests/shield.rs`: the engine hosts the service; a hit makes the open
block private and deletes its screenshot; the segmenter redacts a listed host; the tracker's
pause and private mode do not stop the Guard; `update_settings` with an old settings object
does not change `ShieldConfig`; "delete all data" keeps configuration and sites.
`crates/ubiqx-storage`: V3 applies on a V2 database; repository round-trips; wipe semantics.
`crates/ubiqx-shield-app/tests/leme.rs`: a scripted session leaves zero rows in `blocks`,
`screenshots`, `interventions`, `nudges`, `reports`.

### 9.3 Frontend tests (Vitest, jsdom, against the mock)

`GuardSetup` (no radio preselected on the DNS and vision steps; consent cannot be recorded
without both choices; the managed tick appears only for the managed provider), `GuardPage`
(each tier state string; AppImage message; pending banner and cancel), `GuardPrompt`
(discreet text only; no element contains a host from the mock state; no sample fallback;
"Não era isso"), `SurfTimer` (six steps, rating not sent to any command), `LapseFlow`,
`PanicSheet` (crisis card one tap away), `DelayPicker`, `SiteList`, `AppLeme` (no route to
timeline, reports, review, categories, insights, focus), `parity.test.ts` with the two new
namespaces, forbidden words, `no_forbidden_brand`, `csp.test.ts` unchanged.

### 9.4 What cannot be tested in this sandbox

Everything below is behind a thin adapter and has to be verified on real machines before the
system tiers are enabled in a release. The ten tests of section 11 of the DNS report are the
checklist; the ones that can change this design are marked.

- The elevation prompts (`osascript`, UAC, `pkexec`) and the helper running as root.
- Hosts caps: Windows DNS Client behaviour at 5k, 12k and 20k names (**changes the caps**);
  whether an IPv4 line also stops the AAAA lookup (**decides whether the `::` lines stay**);
  whether a browser in DoH mode honours hosts.
- `networksetup`, `Set-DnsClientServerAddress` and the resolved drop-in on real networks,
  including a captive portal with the filter set.
- Whether the policy values show as applied on unmanaged Windows 11 Home; Firefox registry
  key path; Snap and Flatpak browsers; the macOS configuration-profile route for Chromium.
- The Windows address-bar focus fix, `minimize_app` on the three systems, the `guard`
  window (always on top, all workspaces, not stealing focus).
- macOS Screen Recording and Automation prompts for the Guard without the tracker.
- Verdict quality, refusal rate, latency and real cost per provider at 640 px; the
  `media_score` threshold on real pages.
- The uninstall hooks of the `.deb` and NSIS packages.

Until that checklist is signed off, the build constant `SYSTEM_TIERS_ENABLED` is false in
release builds for the OS that has not been checked: the Guard page then offers tiers 0 and
4 and says the system tiers are coming.

---

## 10. Work breakdown

Units in the same wave own disjoint files and can run in parallel. A later wave may edit
files an earlier wave created. Every unit ends with `cargo fmt --check`, clippy with
`-D warnings` on the default members and its own tests green; frontend units with
`pnpm typecheck && pnpm test -- --run && pnpm build`.

### Wave 0 (one unit, lands first)

**U0 Contracts and skeletons.**
Files: workspace `Cargo.toml` (members, `flate2`; the two Tauri crates `ubiqx-shield-tauri` and
`apps/leme/src-tauri` stay out of `default-members`, like the desktop shell); `Cargo.toml` and stub `lib.rs` or
`main.rs` of `ubiqx-shield-sys`, `ubi-guard-helper`, `ubiqx-shield`, `ubiqx-shield-tauri`,
`ubiqx-shield-app`; `crates/ubiqx-core/src/ports.rs`, `lib.rs`, `model.rs`
(`EngineEvent::Shield`), `shield/mod.rs`, `shield/model.rs`, `shield/sys.rs`,
`shield/identity.rs` and empty stubs of the other `shield/*.rs`; the identity reads in
`core/src/focus.rs`, `core/src/update.rs`, `platform/src/app_id.rs`,
`platform/src/macos/{enforce,keychain}.rs`, `platform/src/secrets_file.rs`,
`platform/src/update_feed.rs`, `engine/src/update.rs`, the user agent in `ubiqx-ai`; the
CLI match arm; placeholder `core.bin`, `popular.bin` and `hosts-hot.txt.gz` (valid, twenty
names); stub `i18n/messages/guard.ts` and `leme.ts` registered in `i18n/index.ts`.
New port methods on existing traits (`capture_frame`, `minimize_app`, `delete_for_block`,
`totals_for_kind`, `delete_kind`) are added with default bodies so that nothing else breaks.
Acceptance: the workspace builds and every existing test passes unchanged; the types of
sections 1.6, 2.3, 2.4, 3.3, 3.4, 4.1, 6.1, 6.3 and 7.2 exist with the signatures given
here; `{}` loads as `ShieldConfig::default()`; the no-host serialisation test of 9.1 passes;
`ProductIdentity` defaults to ubiqX and the focus tests still pass.

### Wave 1 (seven units in parallel: U1 to U6 and U8)

**U1 Lists and matching.** Files: `core/src/shield/{matcher,heuristics,classify,lists}.rs`,
`crates/ubiqx-core/data/shield/*`, `crates/ubiqx-shield-sys/data/hosts-hot.txt.gz`,
`data/shield/*`, `scripts/shield/*`, `.github/workflows/guard-lists.yml`,
`THIRD-PARTY-NOTICES`. Acceptance: the script is deterministic (gate f) and passes gates a to
e; Core within 10% of 239,449 entries; matcher, heuristics and class tests of 9.1 pass; the
embedded tables pass both canary files; no Microsoft-owned name in the hosts tier.

**U2 Behaviour.** Files: `core/src/shield/{gate,policy,commit,copy,stats,resources}.rs`,
`resources.json`. Acceptance: gate, reaction, commitment, copy, resources and stats tests of
9.1 pass; the 40 messages exist in both languages; the forbidden-word test passes.

**U3 Storage.** Files: `crates/ubiqx-storage/src/migrations.rs`, `repos/shield.rs`,
`repos/mod.rs`, `repos/maintenance.rs`, `repos/usage.rs`, `repos/screenshots.rs`, `lib.rs`,
`store.rs`, `tests/repos.rs`. Acceptance: V3 applies to a V2 database; `ShieldRepo`
round-trips; `bump_day` is an upsert; wipe keeps `shield_config`, `shield_pending`,
`shield_sites` and removes `shield_day`, `shield_notes`; an unknown `shield_day.kind` does
not fail a read.

**U4 System layer.** Files: `crates/ubiqx-shield-sys/**` (except the data file),
`crates/ubi-guard-helper/**`, `packaging/linux/ai.ubiqx.guard.policy`,
`packaging/linux/prerm.sh`, `packaging/windows/guard-hooks.nsh`, `docs/guard/REMOVER.md`.
Acceptance: hosts, plan, resolver, policy, state and helper tests of 9.1 pass on Linux;
`cargo check` passes for the macOS and Windows targets; the helper binary has no dependency
on `tauri`, `xcap`, `reqwest` or `rusqlite` (`cargo tree`); every system call sits behind
`SystemFs` or `CommandRunner`, each under 150 lines per OS.

**U5 Platform.** Files: `crates/ubiqx-platform/**`. Acceptance: `capture_frame` and
`minimize_app` implemented for the three systems and the mocks; frame-stat tests pass;
`MockSystemShield` and the scripted frame capturer exist with call logs; the Windows
resolver returns `None` while the address bar has focus (logic unit-tested through a pure
function over the UIA properties); cross-target `cargo check` passes.

**U6 AI and proxy.** Files: `crates/ubiqx-ai/**`, `services/ubi-api/**`. Acceptance:
detector, wire-format, trace-elision and list-fetch tests of 9.1 pass; `wiring.rs` exposes
the client wiring with no behaviour change (existing router tests pass);
`FakeExplicitDetector` scripts verdicts and errors; proxy tests pass.

**U8 Guard UI.** Files: `apps/desktop/src/shield/**`, `apps/desktop/src/i18n/messages/guard.ts`.
Acceptance: the pages and components of 7.9 render against `shield/mock.ts`; the tests of
9.3 that concern them pass; no string in the namespace names a site; parity holds.

### Wave 2

**U7 Service.** Files: `crates/ubiqx-shield/**`. Depends on U1, U2, U3, U5, U6.
Acceptance: the fourteen scenarios of 9.2 pass; `guard_pass` makes no platform call when the
Guard is off; no function in the crate writes an image or a host string to the repository
(reviewed, and covered by scenario 14).

### Wave 3 (two units in parallel)

**U9 ubiqX engine.** Files: `crates/ubiqx-engine/**`, `crates/ubiqx-app/**`,
`crates/ubiqx-cli/**`, `crates/ubiqx-core/src/segmenter.rs`. Depends on U7.
Acceptance: `tests/shield.rs` of 9.2 passes; existing engine tests pass with the new
`LoopConfig`, `PlatformPorts`, `AiPorts` and `Repos` fields; `ubiqx guard remove` runs
without starting the engine; `ubiqx simulate` output is unchanged with the Guard off.

**U10 Tauri glue.** Files: `crates/ubiqx-shield-tauri/**`. Depends on U7.
Acceptance: the commands of 7.8, `TauriShieldPresenter`, `TauriShieldSink`, the `guard`
window builder and the helper path resolution compile on the three CI runners; the crate
depends on neither `ubiqx-engine` nor `ubiqx-app`.

### Wave 4 (four units in parallel)

**U11 ubiqX shell.** Files: `apps/desktop/src-tauri/**`, `apps/desktop/src/App.tsx`,
`components/layout/Sidebar.tsx`, `lib/mock.ts`, `lib/store.ts`, `lib/engine.ts`,
`i18n/messages/nav.ts`. Depends on U8, U9 and U10. Acceptance: the `guard` window opens from
a mock prompt and shows no site; the commands are registered; the tray has "Preciso de apoio
agora"; single instance is enforced; the desktop crate builds on the three CI runners; the
frontend suite passes.

**U12 Leme.** Files: `crates/ubiqx-shield-app/**`, `apps/leme/**` (crate `leme-desktop`),
`apps/desktop/src/leme/**`, `apps/desktop/src/i18n/messages/leme.ts`,
`apps/desktop/vite.config.ts`, `apps/desktop/package.json`, `apps/desktop/index.leme.html`.
Depends on U7, U8 and U10. Acceptance: `leme.rs` of 9.2 passes;
`cargo tree -p leme-desktop -e normal` contains neither `ubiqx-engine` nor `ubiqx-app`;
`pnpm build:leme` produces `dist-leme` whose bundle has no Timeline, Reports, Review,
Categories, Insights or Focus chunk; the app has its own identifier, data directory, keyring
service and update feed.

**U13 Release and CI.** Files: `.github/workflows/ci.yml`, `codemagic.yaml`,
`scripts/publish-release.mjs` and its test, `scripts/app-version.mjs`,
`scripts/ci-app-version.sh`, `scripts/build-guard-helper.sh`, `site/**`. Acceptance: the
helper is built and placed as `binaries/ubi-guard-helper-<triple>` for both shells before
bundling; Leme artefacts are named `leme-*`, published under the tag `leme-continuous` with
their own `latest.json` and never matched by the ubiqX globs; `publish-continuous` runs only
for `refs/heads/main`; the `.deb` contains the polkit policy, the helper at
`/usr/lib/ubi-guard/` and the `prerm` call; the two `cargo tree` checks and
`no_forbidden_brand` run in CI.

**U14 Documentation.** Files: `docs/ARCHITECTURE.md`, `README.md`, `docs/WINDOWS-LINUX.md`,
`docs/LICENSING.md`, `docs/guard/PRIVACIDADE.md`, `docs/guard/COMO-FUNCIONA.md`,
`docs/guard/TESTES-EM-MAQUINA-REAL.md`. Acceptance: the privacy page states, for each tier,
what is captured, sent and stored, in the words of this design; the limits of section 1 and
the eight no-claims are present; the real-machine checklist is complete; the existing drift
found by the maps in the files this unit touches is corrected.

### Wave 5

**U15 Verification.** Files: `scripts/guard-privacy-check.sh`,
`docs/guard/VERIFICACAO.md`. Runs the whole suite, the privacy sweep, the dependency checks
and a read-through of every user-visible string against 4.2. Acceptance: a written report of
each claim in `PRIVACIDADE.md` with the test that proves it, and a list of what remains
unverified (9.4).

---

## 11. Risks and open questions for the owner

1. **Name.** The request says "nofap". This design uses "Guarda do UBI" for the module,
   "Leme" for the product and `shield` in code, because NOFAP is a registered mark with an
   owner who sues, and Brazil is unknown rather than clear. *Recommended default:* keep the
   names of section 8, and commission INPI and USPTO searches for "Leme" and "Maré" before
   anything is published. If the owner wants `nofap` as the internal identifier, it is a
   mechanical rename; it still must not reach anything a user or the system sees.
2. **Third-party DNS.** The family resolver is the strongest DNS tier and the one with a
   privacy cost. This design makes it a choice with no preselection and ships Cloudflare for
   Families only; CleanBrowsing's free tier steers businesses to a paid plan. *Recommended
   default:* ship as designed, credit Cloudflare as their terms ask, and enable CleanBrowsing
   Family only after written clearance from them.
3. **Where images go.** A check sends a reduced picture of a window to Anthropic, OpenAI or
   xAI under the user's key, or through the ubiqX proxy when the user ticks it. Each provider
   has its own retention and its own policy on sexual content, and may act on illegal
   material it receives. *Recommended default:* own-key providers allowed after the consent
   of 3.8; the managed proxy off unless ticked; before launch, someone reads each provider's
   current usage policy for this use and the consent text is adjusted to what it says.
4. **Default reaction to a vision hit.** This design hides the window and asks, because the
   verdict is uncertain and closing a tab can lose work. The request says "remover o
   acesso", which a list hit does at once and a confirmed vision hit does from then on (the
   site is learned). *Recommended default:* hide and ask, with "fechar sem perguntar" as a
   user setting; scope `unknown` marked as recommended in the wizard.
5. **Shipping.** Today every push to any branch publishes the build that installed apps
   update to, macOS builds are ad-hoc signed (each update resets permissions), and the
   elevation prompts name `osascript` or an unknown publisher. The system tiers are untested
   on real machines. *Recommended default:* restrict `publish-continuous` to `main` before
   this branch is pushed; ship the Guard off by default with `SYSTEM_TIERS_ENABLED` false
   per OS until the checklist of 9.4 is signed off on that OS; budget for an Apple Developer
   ID and a Windows code-signing certificate before the system tiers are promoted.
6. **Licence and price of Leme.** Blocking must never depend on a paid plan. Selling Leme
   separately needs a new plan or a product claim, which older builds read as an invalid key.
   *Recommended default:* v1 with no new plan: the Guard and Leme are free to use, vision
   with the user's own key needs nothing, managed vision needs the existing `monthly_managed`
   key. Decide pricing after the first users; if a Leme plan is added later, add a `Plan`
   variant and gate only managed vision with it.
