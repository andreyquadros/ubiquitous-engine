# Research: blocking domains at the DNS / resolver level from a desktop app

Scope: how the `nofap` module (inside ubiqX and as a standalone product) can block pornography
domains at the resolver level on macOS 13+, Windows 10/11 and Linux (X11 desktops), how
self-control and parental tools do it, what bypasses exist and which of them can be countered
without shipping a signed kernel/system extension.

Framing kept throughout: this is a self-control tool an adult installs on their own computer,
visibly and with consent. Nothing here is designed to hide from the person using the machine.
A local administrator can always undo every mechanism below; the goal is friction and an honest
"protection is on / protection was changed" state, not an unbreakable lock.

## 0. How to read this report

**Method and its limit.** Research was done on 2026-11-19 with web search only. Direct page
fetches were refused by the sandbox network allowlist for every host tried
(`developers.cloudflare.com`, `cleanbrowsing.org`, `adguard-dns.io`, `www.joindns4.eu`,
`www.cisco.com`, `quad9.net`), so **no primary page was opened**; every sourced claim rests on
the extract a search returned for the cited URL. Nothing was executed on macOS or Windows. To
re-verify against primary pages, those hosts must be allowed in the session's network settings.

Each factual claim carries one of two marks:

- **[S]** supported by a source found in this session (URL given next to it or in section 12).
- **[U]** *unverified in this session*: stated from general engineering knowledge or inferred;
  must be tested on a real machine or checked against the primary page before it is relied on.

Where two sources disagree the disagreement is stated, not resolved.

Contents

1. Summary and recommendation
2. (a) Hosts-file blocking
3. (b) Pointing the system resolver at a filtering DNS
4. Public family filters: addresses, behaviour, terms
5. (c) Local DNS proxy / sinkhole versus a network extension
6. (d) Bypasses and counters (browser DoH, policies, VPN/Tor, other browsers, hotspot)
7. (e) SafeSearch enforcement via DNS / hosts / browser policy
8. (f) Privilege elevation, keeping the change applied, undoing it
9. (g) What Tauri 2 offers
10. How existing self-control / parental tools do it
11. Recommendation table and test checklist
12. Sources

---

## 1. Summary and recommendation

1. **No single DNS mechanism is sufficient, and none is tamper-proof against the machine's own
   administrator.** The realistic design is three cheap layers that fail safe, plus the vision-AI
   layer for what DNS cannot see: (i) a marked block in the hosts file, (ii) the system
   resolvers pointed at a public family-filter DNS, (iii) browser enterprise policies that stop
   browsers from resolving around (i) and (ii) and that force SafeSearch.
2. **Hosts file** works on all three systems with one elevation, needs no running process and
   keeps working when the app is closed or uninstalled. Its limits: exact hostnames only (no
   wildcards) [S], cannot return NXDOMAIN or CNAME, and very large files hurt the Windows DNS
   Client service [S]. Use it for a curated list (thousands to low tens of thousands of names),
   not for a million-entry list.
3. **Family-filter resolver** covers the long tail the hosts list misses (millions of domains,
   maintained by someone else) and fails safe: if ubiqX is removed the machine still resolves
   names. It is per network service / per interface, so it needs a re-check loop, and it sends
   all of the user's DNS queries to a third party, which must be disclosed and be the user's
   choice.
4. **Browser DoH is the main silent bypass.** Chrome's default "automatic" mode only upgrades to
   the *same* provider's DoH endpoint [S], so it preserves a family resolver; a user-chosen DoH
   provider, or Firefox with DoH turned on by the user, resolves around the system resolver.
   The counter is the browsers' own enterprise policies (`DnsOverHttpsMode`, Firefox
   `DNSOverHTTPS`), which are files / registry values / plist keys an elevated helper can write
   [S for names and most locations]. On macOS, *mandatory* Chromium policy is only reliable via
   managed preferences (MDM or a user-approved configuration profile) [S / U], which is the
   weakest spot of the whole design.
5. **Do not build a local DNS proxy or a network extension in v1.** A proxy on `127.0.0.1:53`
   fails closed (no internet when the process is not running) and DNSFilter abandoned exactly
   that approach on macOS 11+ as unreliable [S]. `NEDNSProxyProvider` needs a system extension,
   Apple-granted entitlement, Developer ID provisioning profile and user approval, and macOS
   allows only one DNS proxy system-wide [S]. Windows WFP *user-mode* filters need no driver [S]
   and are a credible v2 hardening step (block port 53/853 to any resolver but the chosen one).
6. **Elevation without a signed helper is feasible on all three systems**: `osascript ... with
   administrator privileges` on macOS, `ShellExecuteExW` with the `runas` verb on Windows,
   `pkexec` on Linux [S]. Each shows the OS consent prompt; batch every change into one
   elevated run, make the elevated entry point accept only fixed verbs (apply / remove), and
   make every change reversible from a recorded backup.
7. **What DNS cannot do** (and the vision-AI / focus-guard layer must): mixed-content sites
   (Reddit, X, image search thumbnails), content reached through a VPN, Tor, a remote desktop or
   another device, local files, and any site not yet in a list.

Recommended v1, in order of value per effort: hosts block (all OS) -> resolver switch to a
user-chosen family filter (all OS) -> browser policies (Windows and Linux first, Firefox on
macOS, Chromium-on-macOS via configuration profile after testing) -> status / tamper check loop
with a non-shaming "protection changed" intervention. Section 11 has the full table.

---

## 2. (a) Hosts-file blocking

### 2.1 Paths and privileges

| OS | Path | Who may write | Notes |
|---|---|---|---|
| Windows 10/11 | `C:\Windows\System32\drivers\etc\hosts` | Administrators (elevated) [S] | Editing as a standard user fails silently or errors [S]. Use `%SystemRoot%` rather than a literal `C:` [U]. CRLF line endings, no BOM [U]. |
| macOS 13+ | `/etc/hosts` (a symlink into `/private/etc`) [U] | root | Edited with `sudo` in every guide found [S]. |
| Linux | `/etc/hosts` | root | Consulted by glibc NSS (`files`) and, by default, by systemd-resolved: `ReadEtcHosts=yes` is the compiled-in default [S]. |

Sources: Windows path/privilege and flush
(https://www.freecodecamp.org/news/how-to-find-and-edit-a-windows-hosts-file/,
https://helpdeskgeek.com/hosts-file-not-working-windows-11/); `ReadEtcHosts`
(https://manpages.debian.org/resolved.conf(5),
https://wiki.archlinux.org/title/Systemd-resolved).

### 2.2 Format limits

- **No wildcards.** Each hostname, including each subdomain, needs its own line; `*.example.com`
  does not work [S] (https://usekudu.com/guides/privacy/windows-use-hosts-file-to-block-tracking-domains,
  https://www.timeatlas.com/hosts-file). Practical consequence: for each blocked registrable
  domain write at least the apex and `www.`; a list that only has `example.com` leaves
  `m.example.com`, `cdn.example.com` reachable.
- **Cannot express NXDOMAIN or CNAME.** A hosts line maps a name to an address. That matters for
  canary domains (section 6.2, 6.6) and SafeSearch (section 7).
- **Sink address.** `0.0.0.0` is the convention of the large public lists; sources claim it fails
  faster than `127.0.0.1` because nothing is connected to, but those claims are informal and the
  difference is probably negligible [S, weak]
  (https://bitbucket.org/ancile_development/ancile/issues/8/127001-instead-of-0000). Whether an
  IPv4-only line also suppresses the AAAA lookup for that name differs per resolver [U]; test it,
  and if it does not, add a `::` line per name (doubling the block).

### 2.3 Size and performance

- **Windows.** The DNS Client service (`Dnscache`) parses the hosts file at service start and
  again whenever it notices the file changed; with a "large" file it can pin a core at 100%
  until it finishes [S] (Wikipedia "Microsoft DNS", https://en.wikipedia.com/wiki/Microsoft_DNS;
  https://www.smallvoid.com/article/winnt-services-dnscache.html). User reports: a hosts file
  taking more than five minutes to process with no connectivity meanwhile
  (https://learn.microsoft.com/en-us/answers/questions/2811975/windows-10-dnsclient-bug); 8%
  idle CPU with a 445k file; trouble above ~5 MB (https://www.ghacks.net/?p=25618). **No source
  gives an authoritative threshold**, and some users report large files working fine. One
  mitigation reported to help is fewer, longer lines (several hostnames per line) [S, forum].
  Design consequence: cap the Windows block conservatively (suggested starting point: at most
  20-30k hostnames, about 1 MB, several names per line) and measure on a real machine [U for the
  numbers; they are a proposal, not a finding].
- **Linux.** The classic resolver does a linear search through the text file on every lookup
  [S] (https://kiwix.lema.org/serverfault.com_en_all_2023-11/questions/759562/size-limit-of-etc-hosts-linux;
  the same observation is why Android added a sorted hosts cache). One anecdote: 1.7 million
  records added about half a second per lookup. With systemd-resolved in front, the file is read
  by the daemon instead [S for `ReadEtcHosts`; caching behaviour U]. Tens of thousands of entries
  are in routine use via public lists [U].
- **macOS.** Nothing found that measures current macOS. Old Apple Community threads report high
  CPU from DirectoryServices with a 16.5k-line file (2012-era systems) and slow page loads with
  an ~90k-line file [S, old and anecdotal] (https://discussions.apple.com/thread/2209460).
  Treat as **unverified**; measure with the real list on macOS 13-15.
- **List size for reference.** Third-party mirrors of StevenBlack/hosts show the porn-only
  alternate at 76,747 and 83,971 entries and "unified + porn" at 154,651 (mirror snapshots,
  dates unknown); the project is MIT-licensed [S, via mirrors]
  (https://github.com/StevenBlack/hosts, https://awesome.ecosyste.ms/projects/github.com%2Fstevenblack%2Fhosts).
  The list research is a separate report; the point here is that a full public porn list is
  already larger than what is comfortable on Windows.

### 2.4 Flushing caches

| OS | Command | Mark |
|---|---|---|
| Windows | `ipconfig /flushdns` (elevated prompt in the guides; Windows caches positive and negative answers) | [S] https://helpdeskgeek.com/hosts-file-not-working-windows-11/ |
| macOS | `dscacheutil -flushcache; killall -HUP mDNSResponder` (both as root) | [S] https://appleinsider.com/inside/macos/tips/how-to-clear-your-dns-cache-on-a-mac |
| Linux (systemd-resolved) | `resolvectl flush-caches` | [U] not returned by the searches; standard command, confirm in `resolvectl(1)` |
| Linux (nscd / dnsmasq) | `nscd -i hosts`; restart dnsmasq | [U] |
| Browsers | Chrome keeps its own host cache (`chrome://net-internals/#dns`); already-open connections and tabs are unaffected | [U] |

A hosts or resolver change does not close pages that are already open, and browsers keep their
own DNS cache for a short time. The UI should say "takes effect for new pages; restart the
browser to be sure".

### 2.5 Interactions that matter

- **Windows Defender.** Since August 2020 Defender flags hosts files that redirect *specific
  Microsoft domains* as `SettingsModifier:Win32/HostsFileHijack` (the reports are about
  telemetry hosts) [S] (https://support.microsoft.com/en-us/servicing/os/windows/2017/01/hosts-file-is-detected-as-malware-in-windows-defender,
  https://winaero.com/microsoft-now-flags-hosts-files-that-block-windows-10-telemetry). The exact
  matching rule is not public. Consequences: never put Microsoft-owned names in the block list,
  and **pinning `www.bing.com` to the `strict.bing.com` address in hosts (section 7) is a
  candidate trigger** [U]; do Bing SafeSearch through the resolver or the Edge/Chrome policy
  instead, or test that exact line against Defender first.
- **Other security software** may lock or restore the hosts file [U]. The helper must report
  "could not write" distinctly from "wrote".
- **Browsers with DoH.** Chrome's resolver code includes reading the HOSTS file [S]
  (https://chromium.googlesource.com/chromium/src/+/main/net/dns/README.md), but whether HOSTS is
  consulted before a DoH query in "secure" mode was not confirmed [U]. Firefox lists "domains
  listed in /etc/hosts" among the names excluded from DoH [S]
  (https://firefox-source-docs.mozilla.org/networking/dns/dns-over-https-trr.html) under the pref
  `network.trr.exclude-etc-hosts`, yet a Firefox 145 support thread reports the hosts file being
  ignored with that pref true [S, user report] (https://support.mozilla.org/en-US/questions/1548686).
  Treat "hosts survives browser DoH" as **unverified** and disable or pin browser DoH by policy
  (section 6) rather than depend on it.

---

## 3. (b) Pointing the system resolver at a filtering DNS

General properties: the setting is **per network service (macOS) / per interface (Windows) /
per link or per connection profile (Linux)**; it must be set for IPv4 **and** IPv6, otherwise an
IPv6 resolver handed out by the router keeps answering; a VPN normally brings its own resolver
that takes precedence while connected; the previous values must be recorded to restore them.

### 3.1 macOS

```sh
networksetup -listallnetworkservices                 # names; a leading * marks a disabled service
networksetup -getdnsservers "Wi-Fi"                  # current manual servers, or a "there aren't any" message
sudo networksetup -setdnsservers "Wi-Fi" 1.1.1.3 1.0.0.3 2606:4700:4700::1113 2606:4700:4700::1003
sudo networksetup -setdnsservers "Wi-Fi" empty       # restore: back to DHCP-provided DNS
scutil --dns                                         # verify what the resolver really uses
```

- `empty` (sources also write `Empty`) removes the manual entries and reverts to DHCP [S]
  (https://maketecheasier.com/change-dns-servers-terminal-mac/,
  https://sthq.pages.dev/guides/commands/networksetup-macos). IPv6 servers go in the same list
  [S]; no source explicitly confirmed that `empty` also clears them (it replaces the whole list,
  so it should) [U].
- It is **per service**: apply to every enabled service (Wi-Fi, Ethernet, USB tethering, ...)
  and re-check, because a new service (a new dongle, iPhone USB) starts with DHCP DNS [U].
  It is *not* per Wi-Fi network: the manual DNS stays when the Mac joins another SSID or a phone
  hotspot [U], which is what we want.
- Resolution order across services follows the service order; `scutil --dns` shows the resolver
  list and scoped (per-interface, per-domain) resolvers [S]
  (https://www.scoutdns.com/library/why-dns-behavior-differs-by-operating-system). mDNSResponder
  "does not fully respect interface priorities or manually set DNS servers in all cases" [S,
  forum] (https://discuss.zerotier.com/t/how-can-we-assign-dns-servers-for-all-domains/27117).
- **Encrypted alternative without an entitlement: a configuration profile.** A `.mobileconfig`
  with payload type `com.apple.dnsSettings.managed` and a `DNSSettings` dictionary
  (`DNSProtocol` = `HTTPS` or `TLS`, `ServerURL` / `ServerName`, `ServerAddresses`) makes the
  whole system use that DoH/DoT resolver; without `SupplementalMatchDomains` it applies to all
  queries [S] (https://help.swif.ai/en/articles/8714684-creating-a-custom-macos-profile-policy,
  https://docs.sophos.com/central/Customer/help/en-us/ManageYourProducts/DNSProtection/NetworkSetup/ConfigureMacDevicesToUseSecureDNS/).
  The user opens the file, then approves it in System Settings > Profiles, with an "unsigned"
  warning for a do-it-yourself profile [S] (Infoblox guide, https://infoblox-docs.atlassian.net/wiki/x/sxN7Mg).
  `ProhibitDisablement` (cannot be turned off) is honoured on supervised devices only [S], so on
  a personal Mac the user can remove the profile in two clicks. An app cannot install a profile
  silently on current macOS [U]; this is a guided, visible step, which fits the product.
- `NEDNSSettingsManager` (an app publishing DoH/DoT settings the user enables in System
  Settings) exists but needs a Network Extension entitlement [U]; not researched further.

### 3.2 Windows 10/11

PowerShell (elevated):

```powershell
Get-DnsClientServerAddress                                   # per interface, both families
Set-DnsClientServerAddress -InterfaceIndex 12 -ServerAddresses ("1.1.1.3","1.0.0.3","2606:4700:4700::1113","2606:4700:4700::1003")
Set-DnsClientServerAddress -InterfaceIndex 12 -ResetServerAddresses   # restore DHCP / default
Clear-DnsClientCache
```

- `-ResetServerAddresses` resets the interface to its default DNS servers [S]
  (https://learn.microsoft.com/de-de/powershell/module/dnsclient/Set-DnsClientServerAddress).
  IPv6 addresses are passed the same way [S]
  (https://oneuptime.com/blog/post/2026-03-20-configure-ipv6-dns-servers-windows/view).
- `netsh` equivalents: `netsh interface ipv6 set dnsservers "Ethernet" source=dhcp` returns to
  automatic [S]; the IPv4 form `netsh interface ipv4 set dnsservers name="Wi-Fi" static 1.1.1.3
  primary` followed by `netsh interface ipv4 add dnsservers name="Wi-Fi" 1.0.0.3 index=2` is the
  commonly used syntax but was **not returned by the searches** [U]. Prefer the Win32 API or
  PowerShell; `netsh` output is localised (PT-BR Windows) and must never be parsed [U].
- The same values live in the registry under
  `HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\{GUID}\NameServer` (and
  `Tcpip6`) [U].
- **All interfaces.** By default Windows sends a name query out of every connected interface
  ("smart multi-homed name resolution") and uses the first answer [S]
  (https://www.ghacks.net/?p=136550, SANS paper https://www.sans.org/reading-room/whitepapers/dns/preventing-windows-10-smhnr-dns-leakage-40165).
  So the filter must be set on *every* adapter that is up, or the unfiltered one may win. The
  policy value `HKLM\Software\Policies\Microsoft\Windows NT\DNSClient\DisableSmartNameResolution`
  turns the parallel behaviour off [S]; sources disagree on whether queries then still go to all
  adapters sequentially.
- **Windows 11 native DoH** (keeps the filter and encrypts):
  `Add-DnsClientDohServerAddress -ServerAddress '1.1.1.3' -DohTemplate 'https://family.cloudflare-dns.com/dns-query' -AllowFallbackToUdp $False -AutoUpgrade $True`,
  or `netsh dns add encryption server=1.1.1.3 dohtemplate=https://family.cloudflare-dns.com/dns-query autoupgrade=yes udpfallback=no`;
  `netsh dns show encryption` lists the table [S]
  (https://cleanbrowsing.org/help/docs/configure-encrypted-dns-on-windows-11-with-powershell-doh/,
  https://www.elevenforum.com/t/how-to-set-dns-over-https-via-command-prompt.1232/). Per-interface
  activation is the `DohFlags` QWORD under
  `HKLM\System\CurrentControlSet\Services\Dnscache\InterfaceSpecificParameters\{GUID}\DohInterfaceSettings\Doh\<ip>`
  [S, community]. The client DoH feature is supported from Windows 11 / Server 2022 [S]; on
  Windows 10 use plain addresses.

### 3.3 Linux

Three situations; detect which one applies before touching anything.

1. **systemd-resolved + NetworkManager** (Ubuntu, Fedora, most current desktops):
   - Runtime, per link, lost when the link reconfigures:
     `resolvectl dns wlp2s0 1.1.1.3 1.0.0.3`, undo with `resolvectl revert wlp2s0`
     ("all per-interface DNS settings are reset to their defaults") [S]
     (https://man7.org/linux/man-pages/man1/resolvectl.1.html). Fails on links managed by
     systemd-networkd [S].
   - Persistent, per connection profile:
     `nmcli connection modify "<name>" ipv4.dns "1.1.1.3 1.0.0.3" ipv4.ignore-auto-dns yes ipv6.dns "2606:4700:4700::1113 2606:4700:4700::1003" ipv6.ignore-auto-dns yes`
     then re-activate the connection. Without `ignore-auto-dns` the manual servers are *added to*
     the DHCP ones rather than replacing them [S]
     (https://lists.gnome.org/archives/networkmanager-list/2021-December/msg00002.html). Restore
     with `ipv4.dns "" ipv4.ignore-auto-dns no` (and IPv6) [U, inferred from nm-settings]. A Red
     Hat bug reports a DHCPv6 DNS server from the cached lease surviving
     `ipv6.ignore-auto-dns` after `nmcli device reapply`; do a full down/up [S]
     (https://bugzilla.redhat.com/show_bug.cgi?id=1487084). Per profile means every new Wi-Fi
     network gets DHCP DNS again: the re-check loop has to notice.
   - Persistent and global, the cleanest for our purpose: a drop-in
     `/etc/systemd/resolved.conf.d/90-ubiqx-nofap.conf`

     ```ini
     [Resolve]
     DNS=1.1.1.3#family.cloudflare-dns.com 1.0.0.3#family.cloudflare-dns.com
     Domains=~.
     DNSOverTLS=yes
     ```

     then `systemctl restart systemd-resolved`. Drop-ins in `resolved.conf.d` override the main
     file; `Domains=~.` makes the global servers the route for every name, without it
     "systemd-resolved might use the per-link DNS servers" [S]
     (https://wiki.archlinux.org/title/Systemd-resolved,
     https://manpages.debian.org/testing/systemd-resolved/resolved.conf.d.5). Undo = delete the
     one file and restart the service, which is the easiest clean uninstall of the three OSes.
     If a link itself sets `~.` (some VPNs), it competes [U].
2. **NetworkManager without systemd-resolved**: the `nmcli` commands above; NetworkManager
   rewrites `/etc/resolv.conf`. A `[global-dns-domain-*]` section in
   `/etc/NetworkManager/conf.d/*.conf` is the global equivalent [S, truncated example;
   check `NetworkManager.conf(5)`].
3. **Static `/etc/resolv.conf`** (no manager): edit the `nameserver` lines, keeping a backup. If
   the file is a symlink into `/run`, something manages it and a direct edit will be overwritten
   [S for the resolved symlink case, `resolvectl(1)`].

---

## 4. Public family filters: addresses, behaviour, terms

Addresses below were consistent across the sources found unless noted. They are Anycast service
addresses that have been stable for years, but the app should ship them as data (updatable) and
verify them once against the vendor pages before release.

| Service | IPv4 | IPv6 | DoH | DoT hostname | Blocks |
|---|---|---|---|---|---|
| Cloudflare 1.1.1.1 for Families, "malware + adult" | `1.1.1.3`, `1.0.0.3` | `2606:4700:4700::1113`, `2606:4700:4700::1003` | `https://family.cloudflare-dns.com/dns-query` | `family.cloudflare-dns.com` | malware + adult content [S] |
| Cloudflare for Families, "malware only" (not enough for us) | `1.1.1.2`, `1.0.0.2` | `2606:4700:4700::1112`, `::1002` [U] | `https://security.cloudflare-dns.com/dns-query` | `security.cloudflare-dns.com` | malware [S] |
| CleanBrowsing Family Filter | `185.228.168.168`, `185.228.169.168` | `2a0d:2a00:1::`, `2a0d:2a00:2::` | `https://doh.cleanbrowsing.org/doh/family-filter/` | `family-filter-dns.cleanbrowsing.org` | adult, **proxies and VPNs, mixed-content sites such as Reddit**; enforces SafeSearch on Google, Bing, YouTube [S] |
| CleanBrowsing Adult Filter | `185.228.168.10`, `185.228.169.11` (**one source says `.169.10`**) | `2a0d:2a00:1::1`, `2a0d:2a00:2::1` | `https://doh.cleanbrowsing.org/doh/adult-filter/` | `adult-filter-dns.cleanbrowsing.org` | adult only; **allows** proxies/VPNs and mixed-content sites [S]; SafeSearch behaviour [U] |
| OpenDNS FamilyShield (Cisco) | `208.67.222.123`, `208.67.220.123` | `2620:119:35::123`, `2620:119:53::123` | `https://doh.familyshield.opendns.com/dns-query` | `familyshield.opendns.com` | adult content, preconfigured, not customisable [S]; SafeSearch [U] |
| AdGuard DNS "Family protection" | `94.140.14.15`, `94.140.15.16` | `2a10:50c0::bad1:ff`, `2a10:50c0::bad2:ff` | `https://family.adguard-dns.com/dns-query` | `family.adguard-dns.com` (also DoQ) | ads, trackers, adult content; "enable Safe Search and Safe Mode, where possible" [S] |
| DNS4EU "Protective + child protection" | `86.54.11.12`, `86.54.11.212` | `2a13:1001::86:54:11:12`, `2a13:1001::86:54:11:212` | `https://child.joindns4.eu/dns-query` | `child.joindns4.eu` [U for DoT] | malicious sites + sexual, violent and drug content [S] |
| DNS4EU child protection + ad blocking | `86.54.11.11` (second address [U]) | [U] | [U] | [U] | as above + ads [S] |
| Quad9 `9.9.9.9` | | | | | **malware only, no adult filtering** [U here: the vendor page could not be opened; consistent with the task brief] |

Sources: Cloudflare (https://blog.cloudflare.com/ja-jp/introducing-1-1-1-1-for-families/,
https://dnsrobot.net/blog/cloudflare-dns-servers); CleanBrowsing
(https://cleanbrowsing.org/learn/configure-free-filtering, https://cleanbrowsing.org/ip-address,
https://cleanbrowsing.org/support/troubleshooting/chrome-secure-dns,
https://dnsrobot.net/blog/cleanbrowsing-dns-servers); OpenDNS
(https://www.cisco.com/c/en/us/support/docs/security/umbrella/225803-understand-opendns-familyshield.pdf,
https://adguard-dns.io/kb/general/dns-providers/); AdGuard (https://adguard-dns.io/public-dns.html);
DNS4EU (https://www.joindns4.eu/for-public, https://joindns4.eu/learn/dns4eu-public-service-launched,
https://www.captaindns.com/en/blog/dns-dns4eu).

Behaviour notes a designer needs:

- **Coverage differs a lot.** CleanBrowsing *Family* is the only one found that also blocks
  proxy/VPN domains and mixed-content platforms and forces SafeSearch; that is the strongest
  setting for this product and also the one with the most collateral blocking (Reddit goes dark).
  CleanBrowsing *Adult* and Cloudflare Families are narrower. Offer "standard" and "strict".
- **Cloudflare and SafeSearch.** Cloudflare says its adult category "aimed to mirror the Google
  SafeSearch criteria", which is about categorisation; whether `1.1.1.3` forces SafeSearch on
  search engines was **not confirmed** (one low-reliability source says it does not). Test:
  `dig www.google.com @1.1.1.3` and look for a `forcesafesearch.google.com` CNAME.
- **Over-blocking happens.** At launch Cloudflare's `1.1.1.3` blocked LGBTQIA+ sites by mistake
  and published a post-mortem [S]
  (https://blog.cloudflare.com/the-mistake-that-caused-1-1-1-3-to-block-lgbtqia-sites-today).
  The product needs a visible "this site is wrongly blocked" path that does not require turning
  everything off.
- **Privacy.** Switching the resolver hands every DNS query of the machine to that operator.
  For this product that is the most sensitive data flow after the screenshots; it must be an
  explicit, named choice with a hosts-only mode available.

Terms for a commercial app that *configures* the user's machine to use these resolvers (not
legal advice; the full terms pages could not be opened):

| Service | What was found | Reading |
|---|---|---|
| Cloudflare 1.1.1.1 / Families | Use is under the Cloudflare Website and Online Services Terms of Use; an ISP or network-equipment provider that integrates 1.1.1.1 must give attribution per the trademark guidelines; no SLA, limited support [S] (https://developers.cloudflare.com/1.1.1.1/terms-of-use/, https://developers.cloudflare.com/1.1.1.1/infrastructure/sla-and-support/) | Integration by third parties is contemplated; show attribution ("DNS by Cloudflare"), make no availability promise. Lowest-risk default. |
| CleanBrowsing free filters | Free service "can be leveraged by any individual and small government and non-profit organization at no cost"; businesses are steered to paid plans from $59.99/year; terms dated 2019-09-15 cover free, trial and paid use; a third-party review says the free tier is throttled [S] (https://cleanbrowsing.org/terms, https://cleanbrowsing.org/learn/free-vs-paid, https://www.techlockdown.com/blog/cleanbrowsing-setup-and-review) | The end user is an individual, but a commercial product steering its user base there is a grey area. Ask CleanBrowsing in writing or buy a plan before making it a default. |
| OpenDNS FamilyShield | "You may use the Cloud Service solely for your personal use" [S] (https://www.opendns.com/consumer-terms/) | Fine for the individual; not something to build a business default on without asking Cisco. |
| AdGuard DNS public | "You can use public AdGuard DNS servers for free"; the EULA governs; the server software is dual-licensed AGPLv3 / commercial (that concerns the software, not querying the public service) [S] (https://adguard-dns.io/eula.html) | Usable as a user-selectable option; read the EULA for bulk/commercial clauses. |
| DNS4EU | EU co-funded public resolver "for end users and businesses"; no terms text found [S/U] | Good EU option; terms unverified. |

Proposal: default to Cloudflare for Families with attribution; offer CleanBrowsing Family as the
"strict" option after written clearance; list AdGuard Family, OpenDNS FamilyShield and DNS4EU as
alternatives; let the user type custom addresses; never route users' DNS through our own
servers.

---

## 5. (c) Local DNS proxy / sinkhole versus a network extension

### 5.1 Local proxy on 127.0.0.1:53

Idea: run our own resolver (Rust: `hickory-dns`, formerly trust-dns, dual MIT / Apache-2.0 [S],
https://github.com/hickory-dns/hickory-dns), answer blocked names with NXDOMAIN, forward the rest
over DoH to an upstream, and point the system at `127.0.0.1`.

Advantages: real wildcard/suffix matching (`*.example.com`), any list size, proper NXDOMAIN (so
canary domains and Private Relay can be signalled), CNAME rewriting (real SafeSearch), our own
block statistics without a third party seeing them.

Costs and risks:

- **Fails closed.** If the process is not running (crash, update, uninstall, app quit before
  login completes) the machine has no DNS and "the internet is broken". For a consumer product
  without a privileged, always-on, auto-restarting daemon this is the dominant risk.
- **Port 53.** Linux needs root or `CAP_NET_BIND_SERVICE`, and systemd-resolved already listens
  on `127.0.0.53:53` [U]. Windows can have port 53 taken by Internet Connection Sharing /
  Hyper-V / WSL networking [U]. macOS rules for binding low ports were not confirmed [U].
- **macOS `/etc/resolver/<domain>`** files give a per-domain resolver and even a non-standard
  port (`nameserver 127.0.0.1` + `port 5300`) [S] (`resolver(5)`,
  https://keith.github.io/xcode-man-pages/resolver.5.html). They are per-domain, so they suit
  split DNS for a TLD, not a catch-all default route [U]; a report says the mechanism broke on
  macOS 26 for TLDs outside the IANA root [S, single bug report]
  (https://github.com/amphp/dns/issues/69 was returned for a related issue; treat as a lead).
- **Windows NRPT** (`Add-DnsClientNrptRule -Namespace "." -NameServers 127.0.0.1`) can route
  namespaces to a chosen server [S for the cmdlet,
  https://learn.microsoft.com/powershell/module/dnsclient/add-dnsclientnrptrule]; that `"."`
  works as a catch-all was shown only in a community gist [U].
- **Precedent against it on macOS.** DNSFilter's roaming client 1.x redirected DNS to
  `127.0.0.1`; the vendor states the method "became unreliable as macOS 11 and later introduced
  new network security restrictions", declared 1.x end of life and moved to
  `NEDNSProxyProvider` [S]
  (https://help.dnsfilter.com/hc/en-us/articles/43021602300563-How-DNSFilter-adapted-to-macOS-Network-Security-Loopback-Method-vs-System-Extension).
  A related article describes machines losing connectivity because a stuck loopback DNS entry
  had to be removed by hand [S] (https://help.dnsfilter.com/hc/en-us/articles/41871975449363).

Verdict: not for v1. If wildcard matching becomes necessary, do it inside a proper extension
(5.2), not with a loopback resolver.

### 5.2 Network extension / kernel-adjacent filtering

| Mechanism | What it needs | Notes |
|---|---|---|
| macOS `NEDNSProxyProvider` (system extension) | Apple Developer ID; the `com.apple.developer.networking.networkextension` entitlement with the `dns-proxy-systemextension` value for Developer ID builds; a provisioning profile; notarisation; the user approves the system extension and the network configuration (MDM can pre-approve) [S] | Developers report signing friction (profile mismatch, `OSSystemExtensionErrorDomain` 9). "macOS allows only one DNS proxy to intercept DNS traffic system-wide", so it conflicts with corporate agents; VPN conflicts reported [S]. Sources: https://developer.apple.com/forums/thread/815340, https://developer.apple.com/forums/thread/812857, DNSFilter article above. |
| macOS content filter (`NEFilterDataProvider`) | Same family: `content-filter-provider-systemextension`, user approval [S] | Sees flows, not only DNS; heavier still. |
| macOS `pf` rules (`pfctl`) | root | Apple TN3165: Packet Filter "is not considered API... do not use Packet Filter in a software product that you distribute to a wide audience" [S] (https://developer.apple.com/tutorials/data/documentation/technotes/tn3165-packet-filter-is-not-api.md). SelfControl's firewall half is this kind of mechanism. Do not ship. |
| Windows Filtering Platform, **user-mode filters** | Administrator; **no driver**: `FwpmEngineOpen0` / `FwpmFilterAdd0` with conditions on remote port 53 | OpenVPN's `block-dns` does exactly this (block port-53 traffic except on the tunnel) [S] (https://build.openvpn.net/doxygen/wfp__block_8c_source.html, https://learn.microsoft.com/en-us/windows/win32/api/Fwpmu/nf-fwpmu-fwpmfilteradd0). Plain filters do not reference a callout; only filters that need to inspect or modify packets need a callout driver [S]. Persistent filters and their flags have pitfalls [S]. Credible v2: "allow UDP/TCP 53 and TCP 853 only to the chosen resolver". Does not stop DoH on 443. |
| Windows WFP **callout driver** | Kernel driver, EV code-signing certificate and Microsoft attestation signing [U] | Out of scope. |
| Linux nftables/iptables rule (drop port 53 except chosen resolver) | root | Simple technically; distro firewalls (ufw, firewalld) own the ruleset; leave for later [U]. |

---

## 6. (d) Bypasses and counters

### 6.1 Chromium browsers (Chrome, Edge, Brave, others): Secure DNS

- **Default "automatic" mode is same-provider only.** Chrome maps the system's classic resolver
  to that provider's DoH endpoint using the table in `DohProviderEntry::GetList()`; if the
  system resolver is not in the table it keeps using system DNS. The stated purpose: "malware
  protection or parental control features offered by the DNS provider will continue to work"
  [S] (https://www.chromium.org/developers/dns-over-https,
  https://blog.chromium.org/2019/09/experimenting-with-same-provider-dns.html,
  https://chromium.googlesource.com/chromium/src/+/main/net/docs/adding_doh_providers.md). A
  `CleanBrowsingFamily` entry with `family-filter-dns.cleanbrowsing.org` exists in Chromium's
  tests [S]; presence of the Cloudflare-family and FamilyShield entries was **not confirmed**
  [U]. So with defaults, switching the system resolver to a family filter is respected.
- **The bypass** is the user choosing another provider under Settings > Privacy and security >
  Use secure DNS > "With: <provider>", which ignores the system resolver.
- **Secure DNS turns itself off when Chrome sees enterprise policies**: "Secure DNS will also be
  automatically disabled ... if Chrome detects one or more enterprise policies" [S] (ghacks on
  Chrome 83, https://www.ghacks.net/?p=161743). Setting *any* policy therefore already removes
  automatic DoH; set the mode explicitly anyway.
- **Policy `DnsOverHttpsMode`** (string): `off` | `automatic` (DoH with insecure fallback) |
  `secure` (DoH only). With `secure`, `DnsOverHttpsTemplates` must be set and non-empty [S]
  (CIS/Tenable, https://tenable.com/audits/items/CIS_Google_Chrome_L2_v3.0.0.audit:8742caa40e1afc6f78621ddec1bc8b6b;
  Chromium `secure_dns_policy_handler`). Two valid configurations for us:
  - `DnsOverHttpsMode = "off"`: the browser uses the OS resolver, so hosts and the system
    family resolver both apply. Simplest and the one whose behaviour is certain.
  - `DnsOverHttpsMode = "secure"` + `DnsOverHttpsTemplates = "https://family.cloudflare-dns.com/dns-query"`:
    encrypted, filtered, independent of the system resolver and of the network; whether the
    hosts block is still honoured is unverified (2.5). Test before preferring it.
- **Related policies worth setting together**: `ForceGoogleSafeSearch` (boolean; forces
  SafeSearch and locks the setting) [S]; `ForceYouTubeRestrict` (integer; sources agree on
  0 = off, 1 = moderate, 2 = strict, the official page was not opened) [S, secondary];
  `IncognitoModeAvailability` (0 = available, 1 = disabled; 2 = forced, from memory [U]) -
  optional and intrusive, so off by default; `URLBlocklist` exists but "only accepts 1000 URLs"
  [S] (https://chromium.googlesource.com/chromium/src.git/+/HEAD/docs/enterprise/policy_design.md),
  so it cannot carry the list. Edge adds `ForceBingSafeSearch` (0 none, 1 moderate, 2 strict)
  and `InPrivateModeAvailability` [S]
  (https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/forcebingsafesearch).
  Brave adds `TorDisabled` (1 disables Tor windows) [S]
  (https://support.brave.app/hc/en-us/articles/360039248271).
- **Visible side effect**: once any policy is set the browser shows "Managed by your
  organization" in its menu and `chrome://policy` lists the values [U for the banner; S for
  `chrome://policy`]. The app must announce this before applying, in plain words, or the user
  will think the computer was compromised.

Where the policies live:

| Browser | Windows (registry, HKLM, elevated) | macOS | Linux (root-owned JSON) |
|---|---|---|---|
| Chrome | `HKLM\SOFTWARE\Policies\Google\Chrome` - `DnsOverHttpsMode` REG_SZ, `ForceGoogleSafeSearch` REG_DWORD, `ForceYouTubeRestrict` REG_DWORD [S] | bundle id `com.google.Chrome`; **mandatory** = `/Library/Managed Preferences/[<user>/]com.google.Chrome.plist` (managed preferences); **recommended** = `/Library/Preferences/com.google.Chrome.plist` or `defaults write` [S] | `/etc/opt/chrome/policies/managed/*.json` (create the directory) [S] |
| Chromium | `HKLM\SOFTWARE\Policies\Chromium` [U] | `org.chromium.Chromium` [U] | `/etc/chromium/policies/managed/`; Ubuntu packaging uses `/etc/chromium-browser/policies/managed/` [S]; snap/flatpak builds [U] |
| Edge | `HKLM\SOFTWARE\Policies\Microsoft\Edge` - `DnsOverHttpsMode` REG_SZ, `ForceGoogleSafeSearch` REG_DWORD, `ForceBingSafeSearch` REG_DWORD [S] | `com.microsoft.Edge.plist`, same managed/recommended split [S] | `/etc/opt/edge/policies/managed/*.json` [S] |
| Brave | `HKLM\SOFTWARE\Policies\BraveSoftware\Brave` (one user needed the `WOW6432Node` path) [S] | `com.brave.Browser` (`.beta`, `.nightly` variants) [S] | `/etc/brave/policies/managed/*.json`; a community thread reports a file there not being applied on Linux Mint [S] |

Sources: Chrome registry and Linux path (https://support.google.com/chrome/a/answer/9131254,
https://support.google.com/chrome/a/answer/9027408,
https://chromium.googlesource.com/chromium/src/+/HEAD/docs/enterprise/policies.md); Edge
(https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/dnsoverhttpsmode,
https://learn.microsoft.com/deployedge/configure-microsoft-edge-on-mac,
https://fleetdm.com/guides/block-edge-extensions); Brave (support article above,
https://community.brave.com/t/group-policies-dont-apply-on-linux-mint/574077).

Two platform caveats:

- **Windows without a domain.** Google documents registry policies for PCs that are not
  domain-joined, but states that some policies only work on machines joined to Active Directory
  (or enrolled in management); Edge marks many the same way [S]
  (https://support.google.com/chrome/a/answer/9131254, Chromium bug 259236). No list was found
  saying which side `DnsOverHttpsMode` and the SafeSearch policies fall on [U]. They are widely
  used on home PCs by DNS-filter vendors (CleanBrowsing publishes `.reg` files with exactly
  these values [S], https://cleanbrowsing.org/support/troubleshooting/harden-edge), which
  suggests they apply; verify in `chrome://policy` / `edge://policy` on an unmanaged Windows 11.
- **macOS without MDM.** `defaults write com.google.Chrome ...` and `/Library/Preferences`
  produce *recommended* policies ("the policy will be set at the recommended level, not
  mandatory"; Chromium calls the method test-only) [S]
  (https://www.chromium.org/administrators/mac-quick-start/). Recommended means the user can
  change the setting, and not every policy accepts the recommended level (whether
  `DnsOverHttpsMode` does: [U]). A plist written by hand into `/Library/Managed Preferences` is
  wiped at boot when no MDM is present [S, community repo]
  (https://github.com/TommyTran732/Microsoft-Edge-Policies). The remaining route is a
  **user-approved configuration profile** carrying the browser's preference domain as a managed
  payload, which should land as forced preferences [U - not confirmed by any source found;
  highest-priority item to test on a real Mac]. Until that is proven, on macOS rely on
  same-provider auto-upgrade plus the hosts block, and say so in the UI.

### 6.2 Firefox (TRR)

- **Default rollout is regional**: DoH is on by default in the United States (2019-2020),
  Canada (2021), Russia and Ukraine (from March 2022) [S]
  (https://support.mozilla.org/en-US/kb/firefox-dns-over-https). Elsewhere (including Brazil,
  not in that list) it is off unless the user turned it on.
- **Heuristics**: before enabling default DoH Firefox checks for parental controls and managed
  DNS; and "if enterprise policies are active, DoH is disabled unless it is explicitly enabled
  by the DNSOverHTTPS policy" [S] (https://wiki.mozilla.org/Security/DNS_Over_HTTPS/Heuristics).
- **Canary domain** `use-application-dns.net`: Firefox resolves it through the OS resolver; any
  response other than NOERROR (NXDOMAIN, SERVFAIL), or NOERROR without A/AAAA records, disables
  application DNS. It "only applies to users who have DoH enabled as the default option", not to
  users who turned DoH on themselves [S]
  (https://support.mozilla.org/en-US/kb/canary-domain-use-application-dnsnet). A hosts line
  cannot produce NXDOMAIN, so the canary can only be signalled by the resolver; whether each
  family filter does so was not checked [U].
- **Policy** (`policies.json`), the reliable counter:

  ```json
  { "policies": { "DNSOverHTTPS": { "Enabled": false, "Locked": true } } }
  ```

  Keys: `Enabled`, `ProviderURL`, `Locked`, `ExcludedDomains`, `Fallback` (Fallback since 124)
  [S] (https://firefox-admin-docs.mozilla.org/reference/policies/dnsoverhttps/). The filtered,
  encrypted variant is `"Enabled": true, "ProviderURL": "https://family.cloudflare-dns.com/dns-query", "Locked": true`.
- **Locations** [S] (https://firefox-admin-docs.mozilla.org/guides/policies-configuration/,
  https://support.mozilla.org/en-US/kb/managing-policies-macos-desktops):
  - Windows: `distribution\policies.json` next to `firefox.exe`; Group Policy / registry
    (`HKLM\SOFTWARE\Policies\Mozilla\Firefox`, [U] for the exact key).
  - macOS: `Firefox.app/Contents/Resources/distribution/policies.json` (inside the bundle: it is
    replaced by updates and changing a signed bundle is undesirable [U]); or, **without MDM**,
    `sudo defaults write /Library/Preferences/org.mozilla.firefox EnterprisePoliciesEnabled -bool TRUE`
    then keys flattened with `__`, e.g. `DNSOverHTTPS__Enabled -bool FALSE`,
    `DNSOverHTTPS__Locked -bool TRUE` [S]. Firefox is therefore *easier* than Chrome on macOS.
  - Linux: `/etc/firefox/policies/policies.json` system-wide, or `distribution/` in the install
    directory; Flatpak uses
    `~/.local/share/flatpak/extension/org.mozilla.firefox.systemconfig/<arch>/stable/policies`
    [S]; Snap path not found [U].
  - `about:policies` shows what is active [S].

### 6.3 Safari and iCloud Private Relay (macOS)

With Private Relay on, Safari's DNS and traffic go through Apple's relays and ignore the local
resolver. The documented network-side signal is to make `mask.icloud.com` and
`mask-h2.icloud.com` fail with NXDOMAIN or "NOERROR, no answer"; Apple asks not to time out or
drop silently [S, via vendor and forum citations of Apple's "Prepare your network" page]
(https://docs.controld.com/docs/icloud-private-relay,
https://community.fortinet.com/fortigate-3/technical-tip-how-to-block-icloud-private-relay-from-bypassing-the-security-inspection-117445).
Some filters also list `mask-api.icloud.com` and `mask.apple-dns.net` [S]. Only a resolver can
return NXDOMAIN; whether each public family filter does this is unverified [U], and a hosts
entry (`0.0.0.0 mask.icloud.com`) is not the documented signal and may cause delays [U]. For
v1: detect that Private Relay is on (it is a paid iCloud+ feature, not universal) and tell the
user it weakens the protection, with the System Settings path to turn it off.

### 6.4 VPNs, Tor, proxies

- A VPN client normally installs its own resolver that takes precedence while the tunnel is up,
  and tunnelled traffic never touches the local resolver choice [S]
  (https://cleanbrowsing.org/learn/what-is-a-vpn, Cisco's FamilyShield guide lists "VPN DNS or
  browser DNS-over-HTTPS" as the reasons filtering fails). The hosts file still applies to
  programs that use the OS resolver while on a VPN [U].
- Tor Browser resolves names inside the Tor network and ignores both hosts and the system
  resolver [U]. Brave's "private window with Tor" does the same and can be disabled with
  `TorDisabled` [S].
- Web proxies and "unblocker" sites are ordinary websites; CleanBrowsing Family blocks the
  category [S]; a hosts list can include the best-known ones.
- Counter that fits a self-control tool: do not try to defeat a VPN. Detect "a tunnel interface
  is up" or "a Tor / known VPN process is running" as a *state* (the platform layer already
  enumerates windows and apps), show it as "protection reduced", and let the vision-AI layer and
  the counselling flow carry the load. Blocking a work VPN would be a product-killing false
  positive.

### 6.5 Other browsers and apps

The system resolver and hosts cover every program that uses OS name resolution: all browsers
without their own DoH, native apps, Electron apps. Each additional Chromium fork (Opera,
Vivaldi, Arc, ...) has its own policy key and its own secure-DNS toggle [U]; covering the big
four (Chrome, Edge, Brave, Firefox) plus Safari guidance is the reasonable v1 scope, with the
vision layer as the catch-all.

### 6.6 Another network, another device, and plain undoing

- **Phone hotspot / other Wi-Fi**: hosts is network-independent. Manual DNS persists per service
  (macOS) and per interface (Windows) across networks [U]; on Linux with per-connection
  `nmcli` settings a new network reverts to DHCP DNS, which is why the global resolved drop-in
  is preferable there.
- **Another device** (the phone itself) is out of reach of a desktop app. Say so honestly;
  suggest the same family DNS on the phone or the router as a manual step.
- **Undoing it**: anyone with the admin password can delete the hosts block, reset DNS or
  remove a policy file. Section 8.3 covers detection and the response.
- **What DNS never sees**: mixed-content platforms, search-engine thumbnails (hence SafeSearch),
  IP-literal URLs, cached or downloaded material, streaming inside allowed apps.

---

## 7. (e) SafeSearch enforcement

| Engine | Hostnames to redirect | Target (CNAME) | Address for hosts files | Mark |
|---|---|---|---|---|
| Google Search | `www.google.com` and every country domain in use (`www.google.com.br`, ...) | `forcesafesearch.google.com` | `216.239.38.120` | [S] Google's own help gives the hosts example `216.239.38.120 www.google.com` (https://support.google.com/websearch/answer/186669) |
| YouTube, strict | `www.youtube.com`, `m.youtube.com`, `youtubei.googleapis.com`, `youtube.googleapis.com`, `www.youtube-nocookie.com` | `restrict.youtube.com` | `216.239.38.120` | [S] (https://support.opendns.com/hc/en-us/articles/227986807, https://cleanbrowsing.org/support/troubleshooting/verify-safe-search) |
| YouTube, moderate | same five | `restrictmoderate.youtube.com` | `216.239.38.119` | [S] |
| Bing | `www.bing.com` | `strict.bing.com` | `204.79.197.220` (the alias can vary by region) | [S] Microsoft: use a CNAME where possible; "the hosts file needs the actual IP for strict.bing.com rather than a CNAME" (https://support.microsoft.com/en-us/bing/blocking-explicit-content-with-safesearch, Cisco Umbrella docs) |
| DuckDuckGo | `duckduckgo.com` | `safe.duckduckgo.com` | not stable | [S, community only; one user reports it unreliable] (https://raw.githubusercontent.com/rfmarves/ForceSafeSearch/master/hosts, https://help.nextdns.io/t/q6hwda6/enforce-strict-safesearch) |

Limits of doing this in a hosts file:

1. **No CNAME**: the address is pinned. Google's and Microsoft's VIPs have been stable for
   years but are not guaranteed; Microsoft explicitly recommends the CNAME for that reason [S].
   Mitigation: the app resolves `forcesafesearch.google.com`, `restrict.youtube.com`,
   `strict.bing.com` itself at apply time and on each re-check, writes what it got, and falls
   back to the constants above; a changed answer triggers a rewrite (which needs elevation, see
   8.3).
2. **IPv6**: a line that pins only the A record may leave the AAAA lookup going to DNS and
   returning the unrestricted address [U]. Add the AAAA of the same targets as second lines
   (resolve them at runtime; no IPv6 literal was verified in this session).
3. **Country domains**: every Google ccTLD the user might use has to be listed (Google publishes
   the list of supported domains [U]); `google.com.br` is the one that matters first for a
   PT-BR product.
4. **Bing line and Defender**: see 2.5. Prefer the policy `ForceBingSafeSearch = 2` in Edge and
   a resolver that enforces SafeSearch.
5. **Per-account settings are not affected**: this forces the network-level mode only.

Better layers for the same goal, in order of robustness: (1) a resolver that enforces SafeSearch
for every program (CleanBrowsing Family; AdGuard Family "where possible") [S]; (2) browser
policies `ForceGoogleSafeSearch`, `ForceYouTubeRestrict`, `ForceBingSafeSearch` for the browsers
we manage [S]; (3) hosts pinning as the fallback for everything else.

---

## 8. (f) Privilege elevation, persistence, undo

### 8.1 Elevation per OS without a signed privileged helper

**macOS**

- `osascript -e 'do shell script "<command>" with administrator privileges'` raises the system
  authentication dialog and runs the command as root. It needs no entitlement and no helper
  [S, described in MITRE ATT&CK T1548.004 "Elevated Execution with Prompt" and in Apple forum
  threads]. Apple DTS says it does not consider the AppleScript approach "a valid alternative"
  to the deprecated API [S] (https://developer.apple.com/forums/thread/708765) - it works, it is
  simply not the blessed long-term design. The prompt names `osascript`, not the app [U]; give
  the user a sentence beforehand explaining exactly what will be changed. Touch ID in that
  dialog: not confirmed [U].
- `AuthorizationExecuteWithPrivileges`: deprecated since macOS 10.7, still functional in
  current releases according to MITRE; reports of breakage with recent SDKs [S]
  (https://developer.apple.com/documentation/security/authorizationexecutewithprivileges). Do
  not use.
- `SMJobBless` is the old privileged-helper API; `SMAppService` (macOS 13+) replaces it: the
  daemon's launchd plist is bundled in `Contents/Library/LaunchDaemons`, `register()` makes it
  launchable "subject to user approval" in System Settings > Login Items, with an admin
  authentication [S] (https://developer.apple.com/documentation/servicemanagement/smappservice,
  https://theevilbit.github.io/posts/smappservice). Requires a properly signed app; developers
  report approval prompts that never appear or cannot be re-triggered [S]
  (https://developer.apple.com/forums/thread/721919). This is the right v2 for unattended
  re-application; it is the "signed privileged helper" the brief says not to depend on now.
- Escaping: build the root command as a single script file written to a private temp directory
  with fixed content, never by interpolating user-controlled strings into the AppleScript.

**Windows**

- `ShellExecuteExW` with verb `runas` triggers UAC; pass `SEE_MASK_NOCLOSEPROCESS` to get the
  process handle, `WaitForSingleObject`, then `GetExitCodeProcess` [S]
  (https://learn.microsoft.com/windows/desktop/api/Shellapi/ns-shellapi-shellexecuteinfoa; Rust
  examples in the `runas`, `privesc` and `elevator` crates,
  https://docs.rs/crate/runas/latest/source/src/impl_windows.rs). Stdout/stderr of the elevated
  child cannot be captured [S] (https://docs.rs/crate/privesc/1.2.0): report through the exit
  code and a result file.
- No separate helper binary is required: relaunch the app's own executable (or the existing
  `ubiqx-cli`) with a dedicated argument. The app is currently unsigned (see
  `docs/WINDOWS-LINUX.md`), so the UAC dialog will say the publisher is unknown [U]; code
  signing improves that prompt more than any design choice here.
- A scheduled task created once with `/RL HIGHEST` can later be started without a prompt
  (`schtasks /run`); creating it requires elevation [S]
  (https://learn.microsoft.com/windows/win32/taskschd/schtasks). The same pattern is catalogued
  as a UAC-bypass technique and has detection rules [S] (ManageEngine, Cymulate). If used at
  all, it must be an explicit opt-in ("keep protection applied without asking me again"), with
  the task's action restricted to the fixed apply verb.

**Linux**

- `pkexec <absolute path> <args>` asks polkit to run a program as root with the desktop's
  authentication dialog. A policy file in `/usr/share/polkit-1/actions/<namespace>.policy`,
  with the annotation `org.freedesktop.policykit.exec.path` pointing at the binary, gives a
  proper message and lets us choose `auth_admin` or `auth_admin_keep` (authorisation kept for a
  short period, about five minutes) [S]
  (https://polkit.pages.freedesktop.org/polkit/polkit.8.html,
  https://documentation.suse.com/sles/html/SLES-all/cha-security-polkit.html).
- The `.deb` can install that policy file and a helper at a fixed path (`/usr/bin/ubiqx` is
  already fixed). The **AppImage** runs from a mount point that changes per launch, so
  `exec.path` cannot match it [U]; there, fall back to plain `pkexec` (generic "run a program
  as root" prompt) on a helper script copied to a stable location, or state that persistent
  protection needs the `.deb`.
- `pkexec` needs a polkit authentication agent in the session; GNOME, KDE, Cinnamon, XFCE ship
  one, bare window managers may not [U]. Detect failure and show the manual commands.

**Security rule for all three**: the elevated entry point is an attack surface. It must accept
only a closed set of verbs (`apply`, `remove`, `status`) and validated parameters (resolver
addresses checked against the known-provider table or parsed as IP literals; hosts content
generated by the helper itself from a list file whose path is fixed and whose content is
validated as hostnames). Never "run this command line as root".

### 8.2 Making the change durable and reversible

- **Marker block** in hosts, e.g. `# >>> ubiqx-nofap BEGIN v1 sha256=<hash of block>` ...
  `# <<< ubiqx-nofap END`. Apply = replace the block if present, else append; remove = delete
  exactly the block. Everything else in the file is preserved byte for byte, including the
  file's existing line-ending convention. This is pure string logic and fully unit-testable.
- **Backup before change**: previous resolver list per service/interface (including "was
  DHCP"), previous existence/content of each policy file or registry value, kept in a state
  file. Restore from the state file, not from guesses. Handle "the user changed it in the
  meantime" by restoring only values that still equal what we wrote.
- **Atomic write**: write a sibling temp file, fsync, rename; keep permissions and ownership
  (root:wheel 0644 on macOS, root:root 0644 on Linux) [U].
- **Policy files we own**: on Linux write our own file name (`ubiqx-nofap.json`) in the
  `managed/` directory - Chromium merges all JSON files there [S for Edge reading every file];
  never edit someone else's file. On Windows, set only our named values and record whether each
  existed before. Do not touch machines that already have other policies from a real
  organisation (detect a non-empty policy set and ask).

### 8.3 Keeping it applied: re-check loop and tamper response

Reading the state needs no privilege on any of the three systems (hosts is world-readable,
`networksetup -getdnsservers`, `Get-DnsClientServerAddress`, `resolvectl status`, policy files
and `HKLM` reads) [U for completeness, standard]. So the unprivileged app can check cheaply:

- every 60 s or so, and on network-change / wake events: hosts block present and hash matches;
  each active interface's resolvers equal the chosen set (v4 and v6); policy files / values
  present; optionally an **active probe** - resolve a known test domain through the OS resolver
  and expect the sink address or the provider's block answer. The probe catches VPNs, DoH and
  third-party tools without knowing how they did it. (Pick a harmless canary name; do not make
  the app itself request real adult domains. Whether each provider offers a test hostname: [U].)
- Re-applying needs elevation, and without a privileged daemon that means a prompt. That is a
  feature in this product: the response to drift is a calm intervention ("the protection was
  changed at 22:14 - restore it?") with one button that triggers the OS prompt, and an event in
  the user's own log. No silent self-repair, no punishment timers by default.
- Distinguish causes in copy and logic: a new network interface (benign, auto-offer), a VPN
  connecting (benign, "reduced protection"), block deleted or resolver reset (ask, don't
  accuse).
- Cold Turkey's own guide tells users to make the hosts file read-only to make undoing harder,
  and an older review describes it detecting hosts tampering and extending the block by a week
  [S] (https://getcoldturkey.com/support/user-guide,
  https://www.pcworld.com/article/460421/swear_off_time_wasting_websites_with_cold_turkey_free_edition.html).
  The punitive variant conflicts with the non-shaming requirement; an optional, user-chosen
  "cooling-off delay before protection can be turned off" is the acceptable form of friction.

### 8.4 Clean undo on disable and uninstall

- **Disable in the app**: one elevated `remove` that deletes the hosts block, restores resolvers
  from the state file, removes our policy file / values, flushes caches. It must be idempotent
  and safe to run when half of it is already gone.
- **Uninstall**: the Windows NSIS installer is per-user and unelevated (`docs/WINDOWS-LINUX.md`),
  so the uninstaller cannot revert system changes without its own UAC prompt [U]; the `.deb`
  can ship a `prerm` that runs `remove` as root [U]; a macOS app dragged to the Trash and an
  AppImage that is simply deleted run nothing at all. Therefore: (1) the design must **fail
  safe when the app disappears** - hosts entries and a public resolver keep the machine working,
  which is the decisive argument against the loopback proxy; (2) ship and document a standalone
  removal path (a button, a CLI verb `ubiqx nofap remove`, and copy-pasteable manual steps per
  OS); (3) the "Managed by your organization" label disappears only when the policy values are
  removed, so the removal instructions must cover them.
- **Updates**: on Linux/macOS a browser update can replace `distribution/policies.json` inside
  the app bundle [U]; prefer the system-wide locations.

---

## 9. (g) What Tauri 2 offers

State of the repo (`apps/desktop/src-tauri`): Tauri 2 with `tauri-plugin-opener`,
`-notification`, `-autostart`, `-log`, `-process`, `-updater`. No shell plugin and no
single-instance plugin are present.

- **Sidecar / external binaries.** `bundle.externalBin` in `tauri.conf.json` bundles extra
  executables; each file must be named `<name>-$TARGET_TRIPLE`; from Rust it is
  `app.shell().sidecar("name")` (trait `tauri_plugin_shell::ShellExt`), from JS
  `Command.sidecar(...)`, and the capability needs a `shell:allow-execute` entry naming the
  sidecar [S] (https://v2.tauri.app/develop/sidecar). For this feature the shell plugin is
  **not needed**: the elevation launcher is Rust backend code and can use `std::process::Command`
  (`osascript`, `pkexec`) and the Win32 API directly, which keeps process spawning out of the
  webview's reach. A sidecar is only useful as the *thing being elevated*; reusing the main
  binary or `ubiqx-cli` with a dedicated argument avoids a new artefact to sign and bundle.
- **Single instance.** `tauri_plugin_single_instance::init(|app, args, cwd| ...)`; on Linux it
  registers a D-Bus service [S] (https://v2.tauri.app/plugin/single-instance/). Relevant because
  two instances (ubiqX and the standalone product, or two copies) must not fight over the hosts
  block and resolver state. The elevated helper invocation must be a mode that exits before any
  Tauri/GUI initialisation and is not swallowed by the single-instance handoff.
- **Autostart.** Already in use: `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, args)`
  with `enable()`, `disable()`, `is_enabled()` and the `autostart:allow-*` permissions [S]
  (https://v2.tauri.app/plugin/autostart). It starts the app at login in the user's session,
  unprivileged - enough for the re-check loop, not a privileged daemon.
- **Not offered by Tauri**: privilege elevation, a privileged service, DNS configuration,
  network-change events. Those belong in `crates/ubiqx-platform` behind a port, with the pure
  logic (hosts block rendering/merging, the per-OS command plan as data, the state/backup
  model, drift detection) in a crate that unit-tests on Linux CI.
- **Two products, one mechanism.** If both ubiqX and the standalone app are installed, they
  must share the marker, the state-file location and the helper verb set, or one will undo the
  other. Decide a single owner identity (e.g. marker `ubiqx-nofap`, one state file under a
  shared machine-level path) at design time.

---

## 10. How existing tools do it

| Tool | Mechanism (as far as sources show) | Lesson |
|---|---|---|
| SelfControl (macOS, GPL-3.0, open source) | System-level block that survives app quit and restart until a timer ends; early versions used the OS firewall (`ipfw`); later releases added IPv6 blocking and better handling of Google/YouTube. Hosts-file use and the `pf` + privileged-helper details are widely described but were **not confirmed from the source in this session** [S for the first part, U for the second] (https://github.com/SelfControlApp/selfcontrol, https://tidbits.com/2009/04/06/selfcontrol-blocks-internet-time-bandits/) | A root helper that outlives the app is what makes it "unbreakable"; domain blocks have collateral (blocking `google.com` breaks many services, per its FAQ). GPL: read for ideas, do not copy code into ubiqX. |
| Cold Turkey Blocker (Windows/macOS) | App-level blocking plus browser extensions; hosts file involved (its guide suggests making it read-only); historic tamper detection [S] | Browser extensions give URL-level precision DNS cannot; extensions are a separate distribution problem. |
| Freedom (desktop) | "a combination of a local proxy, and optional browser plugins"; the proxy listens on port 7769 and uses compiled regular expressions; on iOS a local VPN; another active VPN can break it [S] (https://support.freedom.to/en/articles/1347493-freedom-privacy-questions-and-answers) | A local proxy covers all browsers but conflicts with VPNs and dev environments (Freedom has support articles about exactly that). |
| DNSFilter roaming client (business) | 1.x: redirect DNS to `127.0.0.1`; 2.x: `NEDNSProxyProvider` system extension, MDM for silent install [S] | The loopback approach is being abandoned on macOS; the proper one needs Apple's entitlement. |
| CleanBrowsing, OpenDNS FamilyShield, Cloudflare Families | Pure resolver-side filtering configured on the device or router [S] | Zero software, but trivially bypassed without the browser-policy layer; the vendors themselves publish the hardening `.reg` / policy snippets. |
| OpenVPN `block-dns` (Windows) | WFP user-mode filters on port 53 [S] | Proof that DNS can be confined without a driver. |
| Covenant Eyes, Canopy | Screen capture with accountability reports; on-page image analysis; network filtering on some platforms [S, mostly marketing and affiliate pages] | The market already pairs DNS filtering with screen analysis; ubiqX's differentiator is that it is local-first and for the user alone, with no reports to a third person. |

---

## 11. Recommendation table and test checklist

Robustness = how hard it is to get around *by accident or impulse* (not by a determined admin).

| Mechanism | macOS 13+ | Windows 10/11 | Linux (X11) | Privilege | Robustness | Effort | Main risk | Verdict |
|---|---|---|---|---|---|---|---|---|
| Hosts block (marked, curated list) | `/etc/hosts` | `%SystemRoot%\System32\drivers\etc\hosts` | `/etc/hosts` | one elevation per change | Medium: all apps using OS DNS; no wildcards; bypassed by browser DoH (unverified), VPN, Tor | Low | Windows DNS Client slowdown with big files; Defender if Microsoft names are present; AV locking the file | **v1, layer 1** |
| System resolver -> family filter (plain DNS) | `networksetup -setdnsservers` per service | `Set-DnsClientServerAddress` per interface (all adapters) | resolved drop-in (`DNS=` + `Domains=~.`), else `nmcli` | one elevation; re-elevate on drift | Medium-high for coverage (millions of domains), low against VPN / user-chosen DoH | Low-medium (three back-ends + restore) | Sends all DNS to a third party; captive portals; corporate split DNS breaks; per-interface drift | **v1, layer 2, opt-in with named provider** |
| Same, encrypted | DNS settings configuration profile (user approves) | Windows 11 `Add-DnsClientDohServerAddress` | `DNSOverTLS=yes` in the drop-in | elevation (Win/Linux); user approval (macOS) | As above + cannot be rewritten by the network | Medium | Profile UX on macOS; Windows 10 lacks it | v1.1 |
| Browser policy: DoH off or pinned to family DoH; SafeSearch forced | Firefox via `/Library/Preferences/org.mozilla.firefox`; Chromium only "recommended" unless a managed profile is proven | HKLM policy keys for Chrome, Edge, Brave; Firefox `policies.json`/registry | JSON in `/etc/opt/chrome`, `/etc/chromium`, `/etc/opt/edge`, `/etc/brave`, `/etc/firefox/policies` | one elevation | High for the managed browsers | Medium (many locations, each testable as pure file rendering) | "Managed by your organization" surprise; domain-joined-only policies on Windows (unverified which); Snap/Flatpak paths | **v1, layer 3** (Windows + Linux + Firefox/macOS first) |
| SafeSearch via hosts pinning | yes | yes (avoid the Bing line until tested with Defender) | yes | with the hosts change | Medium; address can change; IPv6 gap | Low | Stale IP breaks Google/YouTube until refreshed | v1 as fallback; prefer resolver + policy |
| Local DNS proxy on 127.0.0.1 | unreliable since macOS 11 (DNSFilter) | NRPT or adapter DNS -> 127.0.0.1 | conflicts with resolved | root/daemon, always running | High coverage (wildcards, NXDOMAIN, CNAME) | High | Fails closed: no internet if the process dies or the app is removed | **No** |
| macOS `NEDNSProxyProvider` / content filter | system extension | n/a | n/a | Apple entitlement, Developer ID, notarisation, user approval | High | Very high | One DNS proxy system-wide; signing friction; review by Apple | Later, if the product earns it |
| Windows WFP user-mode filters (confine port 53/853 to the chosen resolver) | n/a | `FwpmFilterAdd0`, no driver | (nftables analogue) | admin | Medium-high vs. alternative plain resolvers; nothing vs. DoH on 443 | Medium-high | Breaking VPN/corporate DNS; persistent-filter pitfalls | v2 hardening |
| `pf` rules on macOS | Apple: "not API", do not ship | n/a | n/a | root | n/a | n/a | Breaks across OS updates | **No** |
| Privileged background helper (SMAppService daemon / Windows service or highest-privilege task / systemd unit) for silent re-apply | yes, signed app + user approval | yes | yes (`.deb` only) | install-time elevation | Raises every layer to "survives impulse" | High | Security surface; signing; uninstall hygiene | v2, explicit opt-in |
| Vision AI + focus guard (existing) | yes | yes | yes | none (screen-recording permission on macOS) | Catches what DNS cannot (mixed content, VPN, unknown sites) | separate report | Privacy, cost, false positives | **v1, layer 4** |

### What must be tested on real machines before the design is frozen

1. macOS: does a user-approved configuration profile with a `com.google.Chrome` (and Edge,
   Brave) managed-preferences payload yield *mandatory* policies in `chrome://policy`? Can it be
   combined with the DNS settings payload in one profile?
2. macOS: `osascript ... with administrator privileges` prompt text, Touch ID availability, and
   behaviour when the app is not yet notarised.
3. Windows 11 (unmanaged, Home edition): do `DnsOverHttpsMode`, `ForceGoogleSafeSearch`,
   `ForceYouTubeRestrict`, `ForceBingSafeSearch` show as applied (not "ignored") in
   `chrome://policy`, `edge://policy`, `brave://policy`?
4. Windows: hosts block of 5k / 20k / 80k names - DNS Client CPU and time to first resolution
   after writing; several names per line versus one; Defender reaction to a `www.bing.com` line.
5. All: does a hosts entry still win when the browser uses DoH (Chrome "secure" mode, Firefox
   TRR mode 2 and 3)?
6. All: does an IPv4-only hosts line suppress the AAAA answer? (Decides whether `::` lines are
   needed.)
7. Per provider: answer returned for a blocked name (0.0.0.0, NXDOMAIN, block-page IP), answer
   for `use-application-dns.net` and `mask.icloud.com`, and whether `www.google.com` comes back
   as `forcesafesearch.google.com`.
8. Linux: resolved drop-in on Ubuntu 22.04/24.04 and Fedora with NetworkManager; `pkexec` from
   the AppImage; Snap Chromium / Snap Firefox policy paths.
9. Captive portals (hotel/airport Wi-Fi) with a manual resolver set, per OS: does the login
   page still appear?
10. Uninstall paths: what is left behind in each packaging format, and does the machine keep
    working?

---

## 12. Sources

All URLs below were returned by searches on 2026-11-19; none could be opened directly from the
sandbox (see section 0).

Resolvers and terms
- Cloudflare: https://blog.cloudflare.com/ja-jp/introducing-1-1-1-1-for-families/ ;
  https://developers.cloudflare.com/1.1.1.1/terms-of-use/ ;
  https://developers.cloudflare.com/1.1.1.1/infrastructure/sla-and-support/ ;
  https://blog.cloudflare.com/the-mistake-that-caused-1-1-1-3-to-block-lgbtqia-sites-today ;
  https://dnsrobot.net/blog/cloudflare-dns-servers
- CleanBrowsing: https://cleanbrowsing.org/learn/configure-free-filtering ;
  https://cleanbrowsing.org/ip-address ; https://cleanbrowsing.org/terms ;
  https://cleanbrowsing.org/learn/free-vs-paid ;
  https://cleanbrowsing.org/support/troubleshooting/chrome-secure-dns ;
  https://cleanbrowsing.org/learn/how-to-prevent-filter-bypass ;
  https://dnsrobot.net/blog/cleanbrowsing-dns-servers ;
  https://www.techlockdown.com/blog/cleanbrowsing-setup-and-review
- OpenDNS: https://www.cisco.com/c/en/us/support/docs/security/umbrella/225803-understand-opendns-familyshield.pdf ;
  https://www.opendns.com/consumer-terms/ ; https://support.opendns.com/hc/en-us/articles/227986807
- AdGuard: https://adguard-dns.io/public-dns.html ; https://adguard-dns.io/kb/general/dns-providers/ ;
  https://adguard-dns.io/eula.html
- DNS4EU: https://www.joindns4.eu/for-public ; https://joindns4.eu/learn/dns4eu-public-service-launched ;
  https://www.captaindns.com/en/blog/dns-dns4eu ; https://en.wikipedia.org/wiki/DNS4EU

Hosts file
- https://en.wikipedia.com/wiki/Microsoft_DNS ; https://www.smallvoid.com/article/winnt-services-dnscache.html ;
  https://learn.microsoft.com/en-us/answers/questions/2811975/windows-10-dnsclient-bug ;
  https://www.ghacks.net/?p=25618 ;
  https://support.microsoft.com/en-us/servicing/os/windows/2017/01/hosts-file-is-detected-as-malware-in-windows-defender ;
  https://winaero.com/microsoft-now-flags-hosts-files-that-block-windows-10-telemetry ;
  https://kiwix.lema.org/serverfault.com_en_all_2023-11/questions/759562/size-limit-of-etc-hosts-linux ;
  https://discussions.apple.com/thread/2209460 ; https://github.com/StevenBlack/hosts ;
  https://helpdeskgeek.com/hosts-file-not-working-windows-11/ ;
  https://appleinsider.com/inside/macos/tips/how-to-clear-your-dns-cache-on-a-mac

System resolver configuration
- macOS: https://maketecheasier.com/change-dns-servers-terminal-mac/ ;
  https://sthq.pages.dev/guides/commands/networksetup-macos ;
  https://help.swif.ai/en/articles/8714684-creating-a-custom-macos-profile-policy ;
  https://docs.sophos.com/central/Customer/help/en-us/ManageYourProducts/DNSProtection/NetworkSetup/ConfigureMacDevicesToUseSecureDNS/ ;
  https://keith.github.io/xcode-man-pages/resolver.5.html
- Windows: https://learn.microsoft.com/de-de/powershell/module/dnsclient/Set-DnsClientServerAddress ;
  https://cleanbrowsing.org/help/docs/configure-encrypted-dns-on-windows-11-with-powershell-doh/ ;
  https://www.elevenforum.com/t/how-to-set-dns-over-https-via-command-prompt.1232/ ;
  https://www.ghacks.net/?p=136550 ;
  https://learn.microsoft.com/powershell/module/dnsclient/add-dnsclientnrptrule
- Linux: https://man7.org/linux/man-pages/man1/resolvectl.1.html ;
  https://wiki.archlinux.org/title/Systemd-resolved ; https://manpages.debian.org/resolved.conf(5) ;
  https://lists.gnome.org/archives/networkmanager-list/2021-December/msg00002.html ;
  https://bugzilla.redhat.com/show_bug.cgi?id=1487084

Browser DoH and policies
- https://www.chromium.org/developers/dns-over-https ;
  https://blog.chromium.org/2019/09/experimenting-with-same-provider-dns.html ;
  https://chromium.googlesource.com/chromium/src/+/main/net/docs/adding_doh_providers.md ;
  https://chromium.googlesource.com/chromium/src/+/main/net/dns/README.md ;
  https://chromium.googlesource.com/chromium/src/+/HEAD/docs/enterprise/policies.md ;
  https://chromium.googlesource.com/chromium/src.git/+/HEAD/docs/enterprise/policy_design.md ;
  https://www.chromium.org/administrators/mac-quick-start/ ;
  https://support.google.com/chrome/a/answer/9131254 ; https://support.google.com/chrome/a/answer/9027408 ;
  https://support.google.com/chrome/a/answer/9037717 ; https://www.ghacks.net/?p=161743
- https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/dnsoverhttpsmode ;
  https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/forcegooglesafesearch ;
  https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/forcebingsafesearch ;
  https://learn.microsoft.com/deployedge/configure-microsoft-edge-on-mac ;
  https://github.com/TommyTran732/Microsoft-Edge-Policies ; https://fleetdm.com/guides/block-edge-extensions ;
  https://cleanbrowsing.org/support/troubleshooting/harden-edge
- https://support.brave.app/hc/en-us/articles/360039248271 ;
  https://community.brave.com/t/group-policies-dont-apply-on-linux-mint/574077
- https://firefox-admin-docs.mozilla.org/reference/policies/dnsoverhttps/ ;
  https://firefox-admin-docs.mozilla.org/guides/policies-configuration/ ;
  https://support.mozilla.org/en-US/kb/managing-policies-macos-desktops ;
  https://support.mozilla.org/en-US/kb/canary-domain-use-application-dnsnet ;
  https://support.mozilla.org/en-US/kb/firefox-dns-over-https ;
  https://wiki.mozilla.org/Security/DNS_Over_HTTPS/Heuristics ;
  https://firefox-source-docs.mozilla.org/networking/dns/dns-over-https-trr.html
- Private Relay: https://docs.controld.com/docs/icloud-private-relay ;
  https://community.fortinet.com/fortigate-3/technical-tip-how-to-block-icloud-private-relay-from-bypassing-the-security-inspection-117445

SafeSearch
- https://support.google.com/websearch/answer/186669 ;
  https://support.microsoft.com/en-us/bing/blocking-explicit-content-with-safesearch ;
  https://cleanbrowsing.org/learn/how-to-enforce-safe-search ;
  https://cleanbrowsing.org/support/troubleshooting/verify-safe-search ;
  https://docs.umbrella.com/deployment-umbrella/v1.0.6/docs/enforce-safesearch/ ;
  https://raw.githubusercontent.com/rfmarves/ForceSafeSearch/master/hosts ;
  https://help.nextdns.io/t/q6hwda6/enforce-strict-safesearch

Extensions, proxies, firewalls
- https://help.dnsfilter.com/hc/en-us/articles/43021602300563-How-DNSFilter-adapted-to-macOS-Network-Security-Loopback-Method-vs-System-Extension ;
  https://help.dnsfilter.com/hc/en-us/articles/41871975449363 ;
  https://developer.apple.com/forums/thread/815340 ; https://developer.apple.com/forums/thread/812857 ;
  https://developer.apple.com/forums/thread/675304 ;
  https://developer.apple.com/tutorials/data/documentation/technotes/tn3165-packet-filter-is-not-api.md ;
  https://build.openvpn.net/doxygen/wfp__block_8c_source.html ;
  https://learn.microsoft.com/en-us/windows/win32/api/Fwpmu/nf-fwpmu-fwpmfilteradd0 ;
  https://github.com/hickory-dns/hickory-dns

Elevation
- https://developer.apple.com/forums/thread/708765 ;
  https://developer.apple.com/documentation/security/authorizationexecutewithprivileges ;
  https://developer.apple.com/documentation/servicemanagement/smappservice ;
  https://developer.apple.com/forums/thread/721919 ; https://theevilbit.github.io/posts/smappservice ;
  https://learn.microsoft.com/windows/desktop/api/Shellapi/ns-shellapi-shellexecuteinfoa ;
  https://docs.rs/crate/runas/latest/source/src/impl_windows.rs ; https://docs.rs/crate/privesc/1.2.0 ;
  https://learn.microsoft.com/windows/win32/taskschd/schtasks ;
  https://polkit.pages.freedesktop.org/polkit/polkit.8.html ;
  https://documentation.suse.com/sles/html/SLES-all/cha-security-polkit.html

Tauri
- https://v2.tauri.app/develop/sidecar ; https://v2.tauri.app/plugin/single-instance/ ;
  https://v2.tauri.app/plugin/autostart

Existing tools
- https://github.com/SelfControlApp/selfcontrol ;
  https://tidbits.com/2009/04/06/selfcontrol-blocks-internet-time-bandits/ ;
  https://getcoldturkey.com/support/user-guide ;
  https://www.pcworld.com/article/460421/swear_off_time_wasting_websites_with_cold_turkey_free_edition.html ;
  https://support.freedom.to/en/articles/1347493-freedom-privacy-questions-and-answers ;
  https://canopy.us/canopy-vs-covenant-eyes/
