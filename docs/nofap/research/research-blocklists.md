# Research: public adult-content domain blocklists and how to ship "an extensive list"

Date of all measurements: 2026-11-19, from the Linux sandbox. Everything marked **[M]** was
measured here on the downloaded files (scripts were throwaway, in the session scratchpad).
**[S]** = stated by the source (file header, README, LICENSE). **[U]** = unverified / from
general knowledge, must be checked before it is relied on. Nothing here is legal advice.

Contents

1. Summary and recommendation
2. What could be downloaded from the sandbox
3. The candidate lists, one by one
4. How the lists relate to each other (overlap, provenance)
5. Quality: false positives, stale entries, coverage, Brazilian gap
6. Licences and what they mean for an MIT-licensed commercial app
7. Matching semantics
8. Compact embedded format and measured sizes
9. Keyword and TLD heuristics for unknown domains
10. Categories beyond "porn sites", and what DNS cannot split
11. Update mechanism
12. Build-time script outline
13. Attribution text to ship
14. Open points for the owner

---

## 1. Summary and recommendation

1. **Ship three tiers, not one list.**

   | Tier | Sources | Entries [M] | Size [M] | Licence of the artefact | Where it lives |
   |---|---|---|---|---|---|
   | **Core** | StevenBlack `porn-only` + ShadowWhisperer `Adult` + 4skinSkywalker entries that carry a strong keyword + our own PT-BR supplement, minus our allowlist | **239,449** | 1.92 MB as a hash table; 1.12 MB as gzip text | MIT / public domain (attribution only) | embedded in the binary (`include_bytes!`) |
   | **Extended** (default once downloaded) | Core + Bon-Appetit/porn-domains + 4skin entries corroborated by another list | **523,112** | 4.18 MB hash table; 2.48 MB gzip text | **CC BY-SA 4.0** (data file only) | signed data pack, GitHub release asset, stored in the app data dir |
   | **Hosts tier** | the hostnames from the Chrome UX Report top-1M that the Extended list covers, rank bucket <= 500k, plus apex and `www.` | **~32,000 hostnames** (24.5k if built from Core) | 164 KB gzip; ~0.8 MB once written as `0.0.0.0 name` lines | follows the list it is derived from | written into the marked hosts block by the elevated helper |

   A "maximum" variant (Extended + blocklistproject `porn`, tumblr/blogspot subdomains removed)
   reaches **886,122 entries / 7.09 MB**; it adds little real coverage (section 5) and carries a
   provenance problem (section 4), so it is optional.

2. **Why not one giant list.** The sibling report `research-dns-enforcement.md` caps the hosts
   file at 20-30k names and rules out a local DNS proxy for v1. So the big list cannot be the
   DNS block by itself. It has two other jobs, both local and private: (a) the guard already
   reads the browser URL (`ubiqx_core::normalize::domain_of`), so a hit on the big list is a
   certain answer **without sending a screenshot to any AI provider**; (b) a domain the user
   actually visits that is on the big list but not yet in the hosts block gets promoted into
   the hosts block at the next elevated apply. The hosts tier is ordered by real-world
   popularity, so it blocks what people actually open.

3. **Format:** a sorted table of 8-byte truncated SHA-256 keys (domain-separated), binary
   search, suffix walk over the host's parents. No new dependency (`sha2` and `ed25519-dalek`
   are already workspace dependencies), memory = file size, and the shipped file is **not a
   readable catalogue of porn sites**, which matters for this audience. An `fst` set is the
   alternative if enumeration is wanted (section 8).

4. **Never trust a public list raw.** Measured in the lists we would ship: public suffixes as
   entries (`gob.mx`, `bologna.it`, `tochigi.jp`, `sex.hu`, `blogspot.com`), mainstream sites
   (`deviantart.com`, `vk.com`, `ok.ru`, `tinder.com`, `outbrain.com`, `taboola.com`), a sex
   education site (`scarleteen.com`, in four lists), a cosmetics brand, and in blocklistproject an
   Indian exam portal.
   The build must: drop public-suffix entries, never collapse a subdomain entry to its
   registrable domain, apply our own allowlist, and print every listed name in the CrUX
   top-10k for human review (about 1,400 hostnames today).

5. **Lists miss a lot; heuristics and vision are not optional.** 1,293 hostnames in the CrUX
   top-1M that are on none of the shipped lists match a strong adult keyword, and roughly 90%
   of a 70-name sample were plainly adult (section 9). Brazilian coverage is thin: the Core has
   only 686 `.br` entries. A brand-stem rule (`pornhub`, `xvideos`, `xnxx`, `xhamster`,
   `missav`, ...) and a PT-BR keyword set are block-grade; ambiguous tokens (`sex`, `anal`,
   `cam`, `gay`, `teen`, `pau`, `rola`, `pica`) are signals that only raise the priority of
   the vision check.

6. **Licence obligations:** attribution notices for MIT sources (StevenBlack and its upstream
   lists, 4skinSkywalker), CC BY 4.0 for two tiny upstream lists inside StevenBlack and for
   CrUX, CC BY-SA 4.0 for the Extended data pack (attribution, licence link, statement of
   changes, the pack itself offered under CC BY-SA 4.0, no technical lock on it). **Do not ship
   GPL-3.0 lists (HaGeZi, oisd, chadmayfield)** inside the product; offer a "custom list URL"
   so a user can subscribe on their own machine.

7. **The long tail belongs to the other layers.** Even the 886k-entry union covers only 84.6%
   of HaGeZi's curated list. The family-filter resolver recommended in
   `research-dns-enforcement.md` (Cloudflare for Families `1.1.1.3` / `1.0.0.3`, CleanBrowsing
   Family `185.228.168.168` / `185.228.169.168`), the keyword rules of section 9 and the vision
   check cover what a static list cannot.

---

## 2. What could be downloaded from the sandbox

| Host | Result |
|---|---|
| `raw.githubusercontent.com` | works (all GitHub-hosted lists below) |
| `github.com` (git clone of public repos, partial/blobless) | works; used for last-commit dates and the UT1 mirror file listing |
| `github.com/.../raw/...` (web raw redirect) | 403 |
| `gh api` on third-party repos | refused ("GitHub access to this repository is not enabled for this session") |
| `gitlab.com` (HaGeZi mirror) | works |
| `nsfw.oisd.nl`, `oisd.nl` | blocked (proxy 403 on CONNECT) |
| `dsi.ut-capitole.fr` (UT1 origin) | blocked |
| `sbc.io` (StevenBlack non-GitHub mirror) | blocked |
| `cdn.jsdelivr.net` | blocked |
| `publicsuffix.org` | blocked (the PSL was fetched from `raw.githubusercontent.com/publicsuffix/list`) |
| `tranco-list.eu` | blocked (CrUX top-1M taken from `zakird/crux-top-lists` on GitHub instead) |
| `dns.google`, `cloudflare-dns.com` | blocked, and no system DNS: **no liveness test of domains was possible** |

Consequence for CI: a build job on GitHub Actions has none of these limits, but the build
script should still prefer GitHub raw URLs pinned by commit SHA.

---

## 3. The candidate lists, one by one

Counts are unique valid hostnames after stripping comments and the `0.0.0.0` prefix **[M]**.
"Registrable" = distinct eTLD+1 using the Public Suffix List including the private section.
"Must-block" = how many of a 92-name probe set of well-known adult sites the list covers
(domain or a parent listed); the probe set includes a few unverified Brazilian guesses, so read
it as a relative score only.

### 3.1 StevenBlack/hosts, `porn` extension (alternate `porn-only`)

- URL: `https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn-only/hosts`
- Format: hosts file (`0.0.0.0 name`), exact hostnames, many `www.` duplicates.
- Count: header says 76,811; **76,810** unique, **46,847** registrable, 15,703 `www.`-prefixed.
- Licence: MIT. `license.txt`: "The MIT License (MIT) / Copyright © 2026 Steven Black".
- Cadence: file dated "15 November 2026 20:29:27 (UTC)"; repo last commit 2026-11-15
  (commit `ce25ce24`). Roughly weekly regeneration; upstream sources are marked "occasional".
- Upstream sources of the extension (from `extensions/porn/*/update.json`) **[S]**:

  | Source | URL | Licence | Unique [M] | Last commit [M] | Note |
  |---|---|---|---|---|---|
  | Sinfonietta `pornography-hosts` | `raw.githubusercontent.com/Sinfonietta/hostfiles/master/pornography-hosts` | MIT ("Copyright (c) 2016 Sinfonietta") | 61,154 | 2026-09-08 | the bulk of the extension (88% of its registrable domains) |
  | Sinfonietta `snuff-hosts` | same repo | MIT | not counted | 2026-09-08 | gore, not porn; comes along in `porn-only` |
  | Clefspeare13 `pornhosts` | frozen copy at `StevenBlack/hosts/extensions/porn/clefspeare13/hosts` | MIT **[S, metadata]** | 20,154 | upstream repo no longer exists (404) | the original licence file cannot be re-read |
  | bigdargon hostsVN adult | `raw.githubusercontent.com/bigdargon/hostsVN/master/extensions/adult/hosts-VN` | MIT ("Copyright (c) 2026 BigDargon") | 1,961 | 2026-11-11 | Vietnamese sites only |
  | brijrajparmar27 | `raw.githubusercontent.com/brijrajparmar27/host-sources/master/Porn/hosts` | **CC BY 4.0** per StevenBlack metadata; the repo has no licence file | 231 | 2021-11-28 | abandoned |
  | tiuxo `porn` | `raw.githubusercontent.com/tiuxo/hosts/master/porn` | **CC BY 4.0** | 369 | 2025-11-08 | Japanese sites |

- Quality: the cleanest of the hosts-format lists. Must-block 76/92. Known dual-use entries:
  `4chan.org`, `1337x.to`. 202 `.br` names.
- Redistribution in a commercial MIT app: **yes**, with the MIT notice, plus CC BY 4.0
  attribution for the two small lists (600 hosts in total; they can also simply be subtracted).

### 3.2 ShadowWhisperer/BlockLists, `Lists/Adult`

- URL: `https://raw.githubusercontent.com/ShadowWhisperer/BlockLists/master/Lists/Adult`
- Format: plain domains, one per line; `www.`, `m.`, `mobile.` already removed **[S]**.
- Count: header 213,982; **213,982** unique, 213,689 registrable (almost all eTLD+1).
- Licence: Unlicense. `LICENSE`: "This is free and unencumbered software released into the
  public domain. Anyone is free to copy, modify, publish, use, compile, sell, or distribute
  this software ... for any purpose, commercial or non-commercial".
- Cadence: daily ("Updated: 11/18/2026 17:03"; repo commit 2026-11-18, `8ed49503`). README:
  "Dead/parked domains are periodically removed", "Lists are made from a custom script and
  manual additions", "I will not merge other lists". The repo history is squashed from time to
  time, so pin by content hash, not only by commit.
- Quality: large and fresh, but heavy in machine-generated spam domains (`000354.xyz`, ...).
  Must-block 68/92 (misses `rule34.xxx`, `motherless.com`, `scrolller.com`). Contains public
  suffixes (`bologna.it`, `tochigi.jp`, `sex.hu`, `sex.pl`) and `scarleteen.com`. 579 `.br`.
- Redistribution: **yes**, no conditions.

### 3.3 Bon-Appetit/porn-domains

- URL: resolve through `https://raw.githubusercontent.com/Bon-Appetit/porn-domains/main/meta.json`
  (`blocklist.raw_url`, `allowlist.raw_url`); file names contain hashes and change on purpose
  **[S]**: "Always read `meta.json` to resolve the current filenames instead of hardcoding".
- Format: plain domains, wildcard semantics (domain and subdomains). An **allowlist** file of
  15,109 domains is published alongside.
- Count: `meta.json` says 438,691 lines; **438,691** unique, 434,549 registrable.
- Licence: **CC BY-SA 4.0**. README: "This project is licensed under CC BY-SA 4.0. If you use
  or share this list, please credit it as: `[Bon-Appetit/porn-domains](https://github.com/Bon-Appetit/porn-domains) is licensed under [CC BY-SA 4.0](https://github.com/Bon-Appetit/porn-domains/blob/main/LICENSE).`"
  Also: "Using the list commercially? ... Commercial users are welcome to contribute
  improvements or support the project financially."
- Cadence: daily (`updated: 2026-11-19T08:32:18Z`).
- Quality: has a written inclusion policy (dedicated adult services, explicit user-generated
  content, AI nudification services; excludes general-purpose platforms, sexual-health
  information, non-explicit nudity). Must-block 77/92. Still lists `blogspot.com` whole,
  `gob.mx`, `scarleteen.com`, `proxysite.com`, `rutracker.org`, `faberlic.com`. It is the
  upstream of oisd nsfw (95% of its registrable domains are in oisd). 1,059 `.br`.
- Redistribution: **yes, commercially too**, under share-alike terms (section 6).

### 3.4 blocklistproject/Lists, `porn.txt`

- URL: `https://raw.githubusercontent.com/blocklistproject/Lists/master/porn.txt`
  (domains-only variant: `.../alt-version/porn-nl.txt`).
- Format: hosts. Count: header "Entries: 953,393"; **953,393** unique, 740,223 registrable.
- Licence: `LICENSE` is the Unlicense; README: "This project is licensed under the Unlicense -
  completely free and open source with no restrictions." The file header says "# License: MIT"
  (inconsistent, both permissive).
- Cadence: "Last modified: 2026-07-18"; repo last commit 2026-07-20. **Four months stale** at
  fetch, although the README advertises weekly dead-domain scans.
- Quality: **161,834 `*.tumblr.com` and 70,323 `*.blogspot.*` names** (Tumblr banned adult
  content in December 2018 **[U]**, so that block is almost entirely dead weight).
  Must-block 85/92 (best raw score, because it is the biggest). False positives inherited from
  UT1: `scarleteen.com`, `cosmopolitan.com`, `facens.br`, `apply.tnpscexams.in`, `okcupid.com`,
  `cda.pl`; also `character.ai`.
- **Provenance problem:** 100% of the registrable domains of chadmayfield's 2020 "all" list are
  in this file, and that list describes itself as "a slightly edited list from Université
  Toulouse 1 Capitole"; 52% of this file's registrable domains are in today's UT1 `adult`.
  UT1 is CC BY-SA. A public-domain dedication by a third party does not remove UT1's terms.
- Redistribution: nominally unrestricted; **treat it as CC BY-SA-derived** and keep it out of
  the permissive Core.

### 3.5 4skinSkywalker/Anti-Porn-HOSTS-File

- URL: `https://raw.githubusercontent.com/4skinSkywalker/Anti-Porn-HOSTS-File/master/HOSTS.txt`
- Format: hosts. Count: **70,174** unique, 34,310 registrable, 26,490 `www.`-prefixed.
- Licence: MIT ("Copyright (c) 2022 4skinSkywalker").
- Cadence: last commit 2026-06-02.
- Quality: the noisiest list tested. It contains ad and tracking hosts (`*.2o7.net`,
  `*.openx.net`, `smetrics.att.com`, `tcookie.usatoday.com`), mainstream sites
  (`deviantart.com`, `vk.com`, `ok.ru`, `tinder.com`, `outbrain.com`, `taboola.com`), and public
  suffixes (`gob.mx`, `blogspot.com`). Of its 45,156 `www`-stripped names, 18,396 are in neither
  StevenBlack nor ShadowWhisperer, and 6,024 are in no other list at all; only 1,091 of those
  carry an adult keyword.
- Redistribution: yes (MIT). **Use only entries that carry a strong adult keyword (10,394) or
  that another list corroborates (34,824).**

### 3.6 HaGeZi dns-blocklists, NSFW

- URL: `https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/nsfw-onlydomains.txt`
  (the `domains/nsfw.txt` and `hosts/nsfw.txt` paths return 404; mirror on gitlab.com works).
- Format: wildcard domains. Count: header 85,930; **85,930** unique, 85,637 registrable.
- Licence: **GPL-3.0**. README: "The Lists are published under the GNU General Public License
  v3.0 (GPL-3.0). A copy of the license is included in this repository and has to accompany
  any redistribution. You may redistribute, modify, and adapt the Lists only under the terms of
  that license." It adds that GPL-3.0 "doesn't hand you any rights in the upstream data itself".
- Cadence: several times a day ("Expires: 8 hours", version `2026.1119.1552.04`).
- Quality: the best curated list here; the only debatable whole-domain entries found were
  `4chan.org`, `okcupid.com` and `character.ai`. It is the only curated list that covers all
  four AI-companion and "undress" sites probed (`janitorai.com`, `candy.ai`, `crushon.ai`,
  `undress.app`); `janitorai.com` is otherwise only in blocklistproject. Must-block 77/92.
- Useful companions in the same repo (same licence): "safesearch not supported" (202 search
  engines without SafeSearch) and "DoH/VPN/TOR/Proxy bypass" (16,615 domains; DoH-only 3,570).
- Redistribution inside the product: **not recommended** (section 6). Used here only as an
  independent yardstick for recall.

### 3.7 oisd nsfw

- URL: `https://nsfw.oisd.nl/domainswild2` (blocked here); GitHub copy
  `https://raw.githubusercontent.com/sjhgvr/oisd/main/domainswild2_nsfw.txt`.
- Format: wildcard domains. Count: header 496,504; **496,500** unique, 487,293 registrable.
- Licence: the file header points to `github.com/sjhgvr/oisd/blob/main/LICENSE`, which is the
  **GPL-3.0** text. The project site could not be read from here to check for extra terms **[U]**.
- Cadence: hourly ("Expires: 1 hours", version `202611191733`).
- Quality: an aggregate; 85% of its registrable domains are in Bon-Appetit and 95% of
  Bon-Appetit is in it. 59,737 blogspot names. Must-block 79/92.
- Redistribution: as for HaGeZi, **no** for the shipped product.

### 3.8 UT1 (Université Toulouse Capitole) blacklists, `adult`

- URL: `https://dsi.ut-capitole.fr/blacklists/download/adult.tar.gz` (blocked here); GitHub
  mirror `olbat/ut1-blacklists`, file split as `blacklists/adult/domains.0`, `.1`, `.2`
  (52 MB + 52 MB + 20 MB).
- Format: plain domains, plus a `urls` file (path-level entries, 61 KB) and two regular
  expressions for URL filtering (`very_restrictive_expression`).
- Count: mirror README says 4,600,371; **4,591,363** unique valid (7,227 IP addresses and 691
  malformed lines dropped), but only **769,126** registrable: 3,626,143 names are blogspot
  subdomains and 168,227 tumblr subdomains.
- Licence: **CC BY-SA 4.0**. Mirror README: "the lists are available to download and use under
  the condition of the Creative Commons BY-SA license" (link to by-sa/4.0). The origin page
  could not be read **[U]**.
- Cadence: the mirror says it syncs daily; its last commit was 2026-10-12 (38 days old).
- Quality: broad ("from erotic to hard pornography"), old, noisy. It keeps separate categories
  that are useful as design input: `mixed_adult` (156 sites with adult sections, e.g.
  `4chan.org`, `archiveofourown.org`, `deviantart.com`), `sexual_education` (15 sites "which
  can be misdetected as porn"), `lingerie` (200), `dating` (10,564). Must-block 75/92.
- Redistribution: allowed with share-alike; **not worth its size** (section 5).

### 3.9 chadmayfield/my-pihole-blocklists

- URLs: `.../lists/pi_blocklist_porn_top1m.list` (11,868 unique, 5,916 registrable) and
  `.../lists/pi_blocklist_porn_all.list` (1,905,909 unique, 296,181 registrable).
- Licence: GPL-3.0 (LICENSE file), although the heavy list is UT1 data (CC BY-SA).
- Cadence: last commit **2020-09-10**. Abandoned.
- Quality: the "top1m" list contains `urbandictionary.com`, `okcupid.com`, `dmm.com`, `cda.pl`.
- Verdict: **do not use.**

### 3.10 mhhakim/pihole-blocklist, `porn.txt`

- 240,824 unique names, no licence file, last update 2023-03-23.
- The README says the file includes StevenBlack's unified lists with fakenews, gambling and
  social. Measured: it contains `analytics.google.com`, `discovery.hb.omtrdc.net`, phishing
  domains. **It is not a porn list. Do not use.**

### 3.11 Summary table

| List | Unique | Registrable | Licence | Last update | Must-block /92 | Verdict |
|---|---|---|---|---|---|---|
| StevenBlack porn-only | 76,810 | 46,847 | MIT (+2 small CC BY 4.0 inputs) | 2026-11-15 | 76 | **Core** |
| ShadowWhisperer Adult | 213,982 | 213,689 | Unlicense | 2026-11-18 | 68 | **Core** |
| 4skinSkywalker | 70,174 | 34,310 | MIT | 2026-06-02 | 72 | Core, filtered |
| Bon-Appetit | 438,691 | 434,549 | CC BY-SA 4.0 | 2026-11-19 | 77 | **Extended** |
| blocklistproject porn | 953,393 | 740,223 | Unlicense (UT1-derived) | 2026-07-18 | 85 | optional "max" |
| HaGeZi NSFW | 85,930 | 85,637 | GPL-3.0 | 2026-11-19 | 77 | not shipped |
| oisd nsfw | 496,500 | 487,293 | GPL-3.0 | 2026-11-19 | 79 | not shipped |
| UT1 adult | 4,591,363 | 769,126 | CC BY-SA 4.0 | mirror 2026-10-12 | 75 | not used |
| chadmayfield top1m / all | 11,868 / 1,905,909 | 5,916 / 296,181 | GPL-3.0 | 2020-09-10 | 55 / 59 | rejected |
| mhhakim porn | 240,824 | 106,343 | none | 2023-03-23 | 67 | rejected |
| Sinfonietta | 61,154 | 41,310 | MIT | 2026-09-08 | 74 | via StevenBlack |
| Clefspeare13 (frozen) | 20,154 | 7,410 | MIT | repo gone | 56 | via StevenBlack |
| bigdargon VN | 1,961 | 1,866 | MIT | 2026-11-11 | 0 | via StevenBlack |
| brijrajparmar27 | 231 | 201 | CC BY 4.0 | 2021-11-28 | 0 | via StevenBlack |
| tiuxo porn | 369 | 350 | CC BY 4.0 | 2025-11-08 | 0 | via StevenBlack |

---

## 4. How the lists relate to each other

Share of the row's registrable domains that are present in the column **[M]**:

| row \ column | SB | Shadow | BLP | 4skin | HaGeZi | oisd | Bon-App. | UT1 |
|---|---|---|---|---|---|---|---|---|
| StevenBlack | 100 | 48 | 70 | 40 | 34 | 50 | 74 | 55 |
| ShadowWhisperer | 11 | 100 | 94 | 7 | 20 | 63 | 65 | 40 |
| blocklistproject | 4 | 27 | 100 | 3 | 8 | 45 | 37 | 52 |
| 4skinSkywalker | 55 | 41 | 70 | 100 | 31 | 50 | 79 | 56 |
| HaGeZi | 18 | 50 | 71 | 12 | 100 | 78 | 78 | 37 |
| oisd | 5 | 28 | 68 | 4 | 14 | 100 | 85 | 28 |
| Bon-Appetit | 8 | 32 | 62 | 6 | 15 | 95 | 100 | 24 |
| UT1 | 3 | 11 | 50 | 2 | 4 | 18 | 13 | 100 |
| chadmayfield all | 4 | 13 | 100 | 4 | 5 | 20 | 18 | 97 |

Reading it:

- blocklistproject has absorbed ShadowWhisperer (94%) and the old UT1 snapshot (chadmayfield
  100%); oisd has absorbed Bon-Appetit (95%). These are aggregators, not independent sources.
- The eight groups together name 1,313,189 registrable domains; only 6,035 are in all eight,
  93,386 in five or more, 367,891 in three or more (within the largest shippable union).
  Agreement between lists is a usable confidence signal.
- Independent, curated, permissive: StevenBlack and ShadowWhisperer. Independent, curated,
  share-alike: Bon-Appetit. Independent, curated, GPL: HaGeZi.

---

## 5. Quality

### 5.1 False positives found in the lists we would ship

| Kind | Examples [M] | In which list |
|---|---|---|
| Public suffix as an entry (blocks a whole country sector or hosting platform) | `gob.mx`, `blogspot.com` | 4skin, Bon-Appetit |
| | `bologna.it`, `tochigi.jp`, `sex.hu`, `sex.pl` | ShadowWhisperer (and BLP, UT1) |
| | `co.cz`, `in.net`, `idv.tw`, `eu.cc`, `sa.com` | BLP, Bon-Appetit, oisd |
| Mainstream platforms | `deviantart.com`, `vk.com`, `ok.ru`, `tinder.com` | 4skin |
| Ad networks | `outbrain.com`, `taboola.com`, `*.2o7.net`, `*.openx.net` | 4skin |
| Sexual education | `scarleteen.com` | ShadowWhisperer, BLP, Bon-Appetit, UT1 |
| Unrelated | `apply.tnpscexams.in` (exam portal), `facens.br` (university), `cosmopolitan.com` | BLP (from UT1) |
| | `faberlic.com` (cosmetics) | 4skin, Bon-Appetit |
| Arguable | `character.ai` (BLP, HaGeZi), `1337x.to` (SB, 4skin, Bon-Appetit), `rutracker.org`, `proxysite.com` (Bon-Appetit), `4chan.org` (SB, 4skin, HaGeZi, Bon-Appetit) | |

Checked and **not** listed anywhere as a whole domain: `reddit.com`, `x.com`, `twitter.com`,
`tumblr.com`, `imgur.com`, `discord.com`, `t.me`, `telegram.org`, `pinterest.com`,
`wikipedia.org`, `archive.org`, `youtube.com`, `instagram.com`, `plannedparenthood.org`, and the
classic substring traps `essex.gov.uk`, `sussex.ac.uk`, `scunthorpe.co.uk`, `penistone.co.uk`.

A trap in our own tooling: when subdomain entries were collapsed to their registrable domain,
the union suddenly "blocked" `apple.com`, `reddit.com`, `uol.com.br`, `globo.com`,
`mercadolivre.com.br`, `microsoft.com`. Lists contain names like `adult.contents.fc2.com` or a
single user page on a big portal; **an entry must be kept at the depth it was listed.**

### 5.2 Stale entries

- No DNS from the sandbox, so no liveness measurement. Proxies:
  - 234,248 tumblr/blogspot subdomain entries in the union that includes blocklistproject; 6,955
    in the Core sources.
  - Only 19,528 hostnames of the CrUX top-1M are covered by the Core (26,212 by Extended), so
    more than 90% of the entries are long tail, parked, or dead. A dead entry costs 8 bytes in the
    hash table and nothing at lookup time, so pruning is a size optimisation, not a correctness
    need. It matters only for the hosts tier, which is built from live popularity data anyway.

### 5.3 Coverage against an independent yardstick

HaGeZi NSFW (not shipped) used as the reference set of 85,930 curated domains **[M]**:

| Candidate union (cleaned) | Entries | HaGeZi names covered | CrUX top-1M hostnames covered |
|---|---|---|---|
| Core (SB + Shadow + filtered 4skin) | 239,449 | 52.7% | 19,528 |
| Extended (Core + Bon-Appetit) | 523,112 | 80.5% | 26,212 |
| Max (Extended + blocklistproject) | 886,122 | 84.6% | 28,167 |

So doubling the Core with Bon-Appetit is worth it (+28 points, +6.7k live hostnames); adding
blocklistproject's 363k entries buys +4 points and +2k hostnames.

### 5.4 Brazilian and Portuguese-language gap

`.br` entries per list **[M]**: StevenBlack 202, ShadowWhisperer 579, 4skin 224, HaGeZi 249,
Bon-Appetit 1,059, oisd 1,050, blocklistproject 2,676, UT1 59,209 (almost all
`*.blogspot.com.br`). Names containing `buceta` (same order, without UT1): 12 / 18 / 14 / 18 / 44 / 61 / 110. Names
containing `acompanhante`: 19 / 7 / 23 / 7 / 61 / 69 / 96.

Probe results for sites a Brazilian user would know: `xvideos.com`, `redtube.com.br`,
`cameraprive.com`, `pornocarioca.com`, `fatalmodel.com`, `photoacompanhantes.com` are widely
covered; `brasileirinhas.com.br`, `casadoscontos.com.br`, `contoseroticos.com.br`,
`tufos.com.br`, `skokka.com` only in some lists; `privacy.com.br` and `closefans.com` (creator
subscription platforms **[U]**) in none.

Conclusion: keep a small curated file in this repo (`data/nofap/extra-ptbr.txt`, a few hundred
names, each one checked by a person) and treat the PT-BR keyword set of section 9 as
first-class. If the owner has a Google Cloud project, the CrUX country table for `br` gives a
Brazilian popularity ranking for the hosts tier.

---

## 6. Licences and what they mean for an MIT-licensed commercial app

| Licence | Lists | Commercial redistribution | What we must do |
|---|---|---|---|
| MIT | StevenBlack, Sinfonietta, bigdargon, Clefspeare13 (per metadata), 4skinSkywalker | yes | keep the copyright and permission notice with every copy of the data: a `THIRD-PARTY-NOTICES` file shipped with the app and shown in About |
| Unlicense | ShadowWhisperer, blocklistproject | yes | nothing required; credit anyway |
| CC BY 4.0 | brijrajparmar27 (231), tiuxo (369), CrUX popularity data (per Google's documentation) | yes | credit, link to the licence, say that the data was modified |
| CC BY-SA 4.0 | Bon-Appetit, UT1 (and, in substance, UT1-derived parts of blocklistproject) | yes | credit + licence link + statement of changes; **the adapted list must itself be offered under CC BY-SA 4.0**; no extra restrictions and no technical measure that stops a recipient from using it under that licence |
| GPL-3.0 | HaGeZi, oisd, chadmayfield | permitted by the GPL, with the GPL's conditions | the derived list must be GPL-3.0, the licence text must accompany it, and merging it into the app's own data blurs "mere aggregation". Avoid. |
| none | mhhakim | no | do not use |

Design consequences:

1. **Core = permissive only**, so it can be compiled into the MIT binary with nothing more than
   notices. Subtract the two CC BY 4.0 inputs (600 hosts) if the owner wants a Core with no
   Creative Commons attribution at all; the cost is negligible.
2. **Extended = a separate data file under CC BY-SA 4.0.** Do not `include_bytes!` it into the
   executable. Ship it as its own file (release asset, or a resource next to the binary) with
   its own `LICENSE` and `NOTICE`. The application code stays MIT; the file is data the
   application reads. The build script and the list of upstream URLs are public in this repo,
   and the plain-text form of the pack is published next to the binary form, so nobody is
   locked out of the adapted material. A signature that only proves origin is not a
   restriction on use; still, document the format.
3. **GPL lists stay outside the product.** Offer "add a list by URL" (hosts or domains format):
   the user's own machine downloads from the upstream, we redistribute nothing.
4. CC BY-SA 4.0 is one-way compatible with GPL-3.0 **[U]**, so an all-inclusive pack would
   have to be GPL-3.0. Not worth it.
5. Whether a list of domain names is protected at all varies by country (EU database right,
   for example). Following the stated licences is the safe course regardless.
6. Bon-Appetit explicitly asks commercial users to contribute or donate. Reporting the false
   positives found here (section 5.1) upstream is a cheap way to do that.

---

## 7. Matching semantics

Input is a host name taken from a URL the guard already has, or from the elevated helper's
input when building the hosts block.

1. **Normalise.** Lower-case; strip a trailing dot, a port, user-info, brackets. Convert to
   ASCII with IDNA (`url::Url::parse` already returns the punycode host; the `idna` crate is in
   `Cargo.lock` through `url`). An IP literal never matches the list; return "unknown".
2. **Suffix walk.** Test the host, then drop the left-most label and test again, down to two
   labels. Never test a single label. `a.b.example.co.uk` gives `a.b.example.co.uk`,
   `b.example.co.uk`, `example.co.uk`, `co.uk`: at most labels-1 lookups, usually one to three.
   Measured depth of entries in the "max" union: 90% two labels, 9.5% three, 0.6% four or more.
3. **Entry semantics.** Every entry blocks itself and all its subdomains. `www.`, `m.` and
   `mobile.` prefixes are stripped at build time, so `www.x` matches through its parent.
4. **Longest match wins, allow beats block at equal length.** Walk from most specific to least
   specific and stop at the first name found in either set. This lets the allowlist hold
   `fc2.com` while the blocklist holds `adult.contents.fc2.com`.
5. **Order of authority:** user block > user allow > built-in allow > list block > heuristics.
   A user block must never be overridden by a list update; a built-in allow protects against
   upstream mistakes.
6. **Public-suffix awareness happens at build time, not at run time.** The build drops any
   entry that is a public suffix (ICANN or private section of the PSL): 20 such entries were
   found, including `gob.mx` and `blogspot.com`. Subdomains of private suffixes
   (`x.blogspot.com`, `x.pages.dev`, `x.github.io`) are ordinary entries and stay. Because the
   shipped list is guaranteed free of public suffixes, the run-time walk needs no PSL, which
   avoids embedding the PSL (MPL-2.0 data, 343 KB raw / 92 KB gzip, 10,467 rules) and keeping
   it fresh. The PSL is a build-time input only.
7. **IDN.** 970 entries (0.1%) of the max union are punycode (`xn--...`); they match like any
   other name once step 1 has run. For the keyword heuristics, decode `xn--` labels to Unicode
   and fold accents (`pornô` -> `porno`, `tesão` -> `tesao`) before matching, and add a handful
   of non-Latin keywords (`порно`, `секс`, `色情`, `エロ`, `سكس`) **[U, not measured]**.
8. **Look-alikes.** `xvv1deos.com` is in the CrUX top-10k and on no list. For brand-stem
   matching only, fold `0->o`, `1->i`, `3->e`, `4->a`, `5->s`, `7->t`, `vv->w`, and strip
   digits at the edges (`missav123`, `xhamster46`, `faphouse2`, `okxxx1`).
9. **Hosts tier.** Hosts files match exact names only. The tier therefore stores real
   hostnames seen in popularity data (`pt.pornhub.com`, `rt.pornhub.com`, `amp.xvideos.com`,
   `br.stripchat.com`), plus the apex and `www.` of each registrable domain.

The existing `ubiqx_core::normalize::domain_matches(page_domain, key)` and `domain_of(url)`
already implement steps 1 to 3 for user-entered targets; the list matcher should reuse them and
add the hashed set lookup.

---

## 8. Compact embedded format and measured sizes

All sizes measured on the cleaned unions, except the FST byte figures, which are estimates from
a measured automaton (no Rust build was allowed in this run). The three columns are the
cleaned unions of StevenBlack + 4skin + ShadowWhisperer (247,912 entries), the same plus
Bon-Appetit (524,982) and the same plus blocklistproject (887,964); the recommended tiers of
section 1 are within 4% of the first two.

| Representation | 248k entries | 525k entries | 888k entries | Notes |
|---|---|---|---|---|
| Sorted plain text | 4.01 MB | 8.41 MB | 14.66 MB | |
| gzip -9 of sorted text | 1.18 MB | 2.50 MB | 4.22 MB | `flate2` is in `Cargo.lock` |
| xz -6 of sorted text | 0.99 MB | 2.07 MB | 3.48 MB | zstd not available here; expect between gzip and xz |
| Reversed-label sort + gzip | 1.22 MB | 2.50 MB | not comparable | measured before cleaning (255k / 532k entries); no better than plain sorting |
| Front-coded + gzip | 1.00 MB | 2.02 MB | not comparable | measured before cleaning; 15-20% smaller than plain gzip, not worth a custom codec |
| **Sorted 64-bit hash table** | **1.98 MB** | **4.20 MB** | **7.10 MB** | 8 bytes per entry; incompressible (gzip saves 4%) |
| Sorted 48-bit hash table | 1.49 MB | 3.15 MB | 5.33 MB | |
| Binary fuse filter, 16-bit | ~0.56 MB | ~1.18 MB | ~2.0 MB | false-positive rate 1/65,536 per lookup; needs the `xorf` crate |
| Binary fuse filter, 8-bit | ~0.28 MB | ~0.59 MB | ~1.0 MB | false-positive rate 1/256: unusable alone |
| Minimal automaton (FST) | 352k states, 579k transitions; est. 1.4-2.0 MB | 710k states, 1.18M transitions; est. 2.9-4.1 MB | not built | `fst` crate; enumerable, mmap-able |
| `HashSet<String>` in memory | ~15-20 MB | ~35-45 MB | ~60-70 MB | estimate; avoid |

Entry length: median 15 bytes, 99th percentile 29-36, maximum 86.

**Recommended: sorted 64-bit hash table.**

- Key: first 8 bytes of `SHA-256("ubiqx-nofap-blocklist-v1\0" || normalised_domain)`, compared
  as big-endian unsigned integers. `sha2` is already a workspace dependency, the function is
  identical in Python (`hashlib`) for the build script, and it is stable across Rust versions
  (unlike `std::hash::DefaultHasher`).
- File layout: `magic "UBXBL1\0\0"` (8) | `format u16` | `flags u16` | `count u32` |
  `list_version u64` (UTC `YYYYMMDDHHMM`) | `count * u64` big-endian sorted, unique.
  The file's SHA-256 lives in the signed manifest, not in the file.
- Lookup: binary search, about 20 probes for a million keys; at most a handful of lookups per
  host. The table can be a `&'static [u8]` from `include_bytes!` or a `Vec<u8>` read from the
  data dir; no parsing step, no allocation per entry.
- False match probability for a benign name: entries / 2^64, about 5e-14 at 888k entries.
- It cannot be enumerated. That is a feature here: the app does not put a browsable directory
  of half a million porn sites on the disk of a person who is trying to stay away from them.
  The price is debuggability, so CI keeps the plain-text list as a build artefact, and the
  "why was this blocked" screen shows the matched suffix (known from the query) and the tier.
- The **hosts tier** is necessarily plain text; it is small and limited to sites that are
  already widely known.
- Pure-logic unit tests: hashing vector, suffix walk, longest-match and allow precedence,
  rejection of malformed files (bad magic, unsorted, wrong count), must-block and
  must-not-block canaries against the embedded Core.

If the design panel prefers an enumerable structure, use `fst::Set` (MIT/Unlicense, no
dependencies): similar size, exact, supports "list everything under this suffix". It needs a
Rust build step to produce the file, where the hash table needs only Python.

---

## 9. Keyword and TLD heuristics for unknown domains

Method **[M]**: "positive" corpus = 535,674 registrable domains from the curated lists;
"negative" corpus = 724,494 registrable domains of the CrUX top-1M that are on none of the 15
lists. The negative corpus still contains unlisted porn, so a hit there is not automatically a
false positive; examples were read by hand. `substr` = keyword anywhere in the name left of the
public suffix; `token` = keyword as a whole token after splitting on non-letters.

### 9.1 Block-grade substrings (safe anywhere in the name)

| Keyword | Listed names containing it | Unlisted CrUX names containing it | What the unlisted ones are |
|---|---|---|---|
| `porn` | 60,704 (11.3%) | 212 | adult mirrors (`porno365.golf`, `josporn.red`, ...) |
| `xxx` | 21,480 | 130 | mostly adult; a few gambling (`...togel8xxx.org`) and `xxxlutz.de` (furniture) |
| `hentai` | 2,696 | 77 | adult |
| `xvideos` / `xnxx` / `xhamster` / `pornhub` / `brazzers` | 3,072 / 2,617 / 468 / 1,180 / 306 | 22 / 23 / 5 / 0 / 1 | mirrors |
| `blowjob`, `gangbang`, `cumshot`, `handjob`, `bukkake`, `deepthroat`, `threesome`, `hotwife`, `voyeur` | 60-636 each | 0 | |
| `creampie`, `cuckold`, `femdom`, `fetish`, `shemale`, `camgirl`, `upskirt`, `masturbat`, `incest` | 118-2,606 each | 1-7 each | adult |
| `bokep` (Indonesian), `phimsex` (Vietnamese) | 1,401 / 1,607 | 222 / 29 | adult mirrors |
| `fuck` | 4,693 | 31 | mostly adult; `ismypubfucked.com` is not |
| `erotic`, `erotik`, `erotica` | 3,145 / 1,283 / 1,030 | 7 / 10 / 0 | adult stories and shops |
| PT-BR: `putaria`, `buceta`, `bucetinha`, `siririca`, `contoseroticos`, `brasileirinhas`, `pornozao`, `safadas`, `novinhas`, `gostosas`, `peladas`, `transando`, `gozando` | 1-79 each | 0-2 each | adult |
| ES/FR/TR: `desnudas`, `sexe` (FR), `travestis` | 37 / 1,300 / 90 | 0 / 6 / 0 | `sussexexpress.co.uk` matches `sexe`: require a token or pair it |

Brand stems (block-grade as substrings after look-alike folding): `pornhub`, `xvideos`, `xnxx`,
`xhamster`, `redtube`, `youporn`, `spankbang`, `eporner`, `chaturbate`, `stripchat`,
`bongacams`, `livejasmin`, `cam4`, `myfreecams`, `onlyfans`, `fansly`, `brazzers`, `missav`,
`nhentai`, `hanime`, `rule34`, `e-hentai`, `fapello`, `erome`, `coomer`, `kemono`, `simpcity`,
`noodlemagazine`, `sxyprn`, `faphouse`, `javhd`, `supjav`. Evidence for the need: the CrUX
top-5k alone contains `missav.ai`, `missav.live`, `missav.ws`, `missav123.com`, dozens of
`*.xhamster*.desi` mirrors, `faphouse2.com`, `okxxx1.com`.

### 9.2 Medium: good as substrings but with known brand collisions

| Keyword | Collisions seen [M] | Rule |
|---|---|---|
| `sexy` | Asian casino brands `sexygame...`, `sexy365bet`, `ufasexygame` | block-grade only together with a second signal |
| `adult` | `literacyandnumeracyforadults.com`, Spanish/Portuguese `adultos`, `adultas`; in Brazil "educação de jovens e adultos" | token `adult` + second signal |
| `escort` | car model; `escorts` is cleaner (4,163 listed, 10 unlisted, all adult) | `escorts`, `escort` + second signal |
| `naked` | `nakedwines.com`, `naked-science.ru`, `nakedcapitalism.com`, `nakednutrition.com` | token + second signal |
| `nude` | `nude-project.com` (clothing), German `nudeln` | `nudes` block-grade; `nude` as token |
| `milf` | `emilfrey.de`, `milfordasset.com`, `tamilfonts.net`, `newmilfordps.org` | **token only** |
| `bdsm` | `sbdsmartorder.com.br`, `bdsmktg.com` | token only |
| `sexo` | `iowasexoffender.gov`, `sexoffender.go.kr` | token, or substring unless followed by `ffend`/`log` |
| `pussy` | `pussy888.*`, `pussy777.*` (casino brand) | token + not followed by digits |
| `webcam`, `cams` | `skylinewebcams.com`, weather and traffic cameras | only with a second signal |
| `teen` | `fourteenfish.com`, `dteenergy.com`, Finnish `...tieteen...` | token + second signal |
| `acompanhantes` | escort listings (the category itself); `acompanhantes.com` etc. are real listings | treat as the escort category, user-selectable |
| `amador` / `amadoras` | `amadori.it`, `cm-amadora.pt`, `programador...`, `reclamador.es` | `amadoras` ok; `amador` token + second signal |
| `casadas`, `coroas` | `casadascapas.store`, `casadasaliancas.com.br`; `coroasparavelorio.com.br` (funeral wreaths) | never alone |

### 9.3 Traps: never as substrings, weak even as tokens

| Keyword | Listed | Unlisted CrUX | Examples of innocent matches |
|---|---|---|---|
| `sex` (substr) | 58,825 | 564 | `aiimsexams.ac.in`, `correosexpress.com`, `lotusexchange2.co`, `essex`, `sussex`, `middlesex`, `unisex`, `sextant` |
| `anal` | 2,524 | 544 | `canal26.com`, `planalto.gov.br`, `canaltech.com.br`, `artificialanalysis.ai`, `analytics` |
| `cum` | 1,458 | 244 | `documento...`, `tucuman`, `vademecum.es`, `cucumber` |
| `ass` | 4,301 | 4,583 | `classlink.com`, `atlassian.com`, `glassdoor.com`, `sassa.gov.za` |
| `cock` | 1,075 | 77 | `peacocktv.com`, `johnhancock.com`, `babcock.edu.ng`, `cocktail...` |
| `dick` | 657 | 43 | `dickies.com`, `dickinson.edu`, `dickeys.com` |
| `tits` | 1,036 | 28 | French `petits...` (`lespetitsculottes.com`) |
| `cam` | 18,850 | 2,541 | `cambridge.org`, `camara.leg.br`, `campus...` |
| `jav` | 4,011 | 313 | `prajavani.net`, `lojavirtualtim.com.br`, `ninjavan.co` |
| `gay` | 13,091 | 184 | Vietnamese `ngay` ("day"): `lichngaytot.com`; also community and health sites. Never block on this token alone. |
| `lust`, `kink`, `fap`, `tube`, `hot`, `av`, `18`, `69`, `x` | | 44-41,596 | `illust`, `cluster`, `kinkos`, `fapesp.br`, `rutube.ru`, `hotnews.ro`, `avito.ru`, `news18.com` |
| PT-BR `pau` | 127 | 441 | `drogariasaopaulo.com.br`, `paulo`, `pausa` |
| PT-BR `rola` | 60 | 181 | `motorola.com.br`, `iberdrola.es`, `controla`, `rolamentos` |
| PT-BR `pica` | 64 | 156 | `tropical...`, `olimpica.com`, `picard.fr`, `picanha` |
| PT-BR `puta` | 241 | 77 | `computa...`, `diputados.gob.mx`, `reputação`, `disputa` (but `putas` is clean: 92 / 2) |
| PT-BR `bunda` | 14 | 25 | `abundance...`, Indonesian `bunda` ("mother"), `bundaberg` |
| PT-BR `foda`, `foder`, `corno`, `vadia`, `pelada`, `contos`, `culos` | | 1-53 | `infodata`, Danish `foder`, `capricorn`, `nova diagnostico`, `papelada`, `descontos`, `veiculos`, `oculos` |
| IT `sesso` | 92 | 23 | `assessoria...`, `assessor` |

`pelada` deserves a note for Brazil: besides "naked" it means a pick-up football game.

### 9.4 Composite rule, measured

Rule: adult TLD, or any block-grade substring, or two weak tokens. On the "max" union it
explains 29.8% of the listed entries; **67.5% of listed adult domains carry no keyword at
all**, so heuristics complement lists and cannot replace them.

On the 971,813 CrUX hostnames not covered by the max union:

- adult-TLD rule: 17 hits; two are casino names on `.adult`, the rest look adult;
- strong-substring rule: **1,293 hits**; in a random sample of 70, about 62 were plainly adult
  and 8 were not (`portal.milfordasset.com`, `milford-place.click`, `www.emilfrey.de`,
  `member.ufasexygame19.com`, `nadimtogel8xxx.org`, `www.ismypubfucked.com`, two unclear).
  With `milf` and `sexy` demoted as in 9.2 the sample precision is about 95%;
- two-weak-tokens rule: 2 hits;
- single weak token: 592 hits, mostly innocent (`www.repository.cam.ac.uk`, `av.gov.mk`,
  `hot-thai-kitchen.com`, `girls-und-panzer.jp`, `www.pica.gov.jm`).

Use in the product:

- **Block-grade hit** on an unknown domain: act as if listed (intervention, add to the user's
  learned blocks), with a one-click "this is not adult content" that puts the name on the local
  allowlist. No screenshot is needed, which is also the most private outcome.
- **Weak hit**: do not block. Raise the priority of the next vision check for that window.
- Never apply keywords to names on the built-in allowlist, to `gov`, `edu`, `ac.*`, `mil`
  suffixes, or to the top few thousand CrUX origins that were reviewed.

### 9.5 Adult TLDs

| TLD | Listed registrable | Unlisted CrUX | Verdict |
|---|---|---|---|
| `.xxx` | 1,939 | 6 (`rule34vault.xxx`, `ind.xxx`, ...) | block the whole TLD |
| `.porn` | 571 | 3 | block |
| `.sex` | 530 | 3 | block |
| `.sexy` | 204 | 0 | block |
| `.adult` | 176 | 2 (casino names) | block |
| `.cam` | 1,584 | 152 (`futbol-libre.cam`, `streaming-community.cam`, `poly.cam`, betting) | **signal only**; the TLD is generic |
| `.webcam` | 170 | 5 | signal only |
| `.tube` | 289 | 21 (`onepiece.tube`, `happymod.tube`) | signal only |
| `.dating`, `.singles`, `.fans` | 71 / 39 / 214 | 13 / 2 / 66 | no |

Blocking five TLDs wholesale is a rule, not list entries; it covers 5,127 entries of the max
union (0.6%) and every future registration. In a hosts file a TLD cannot be expressed; it is a
guard-level rule.

---

## 10. Categories beyond "porn sites", and what DNS cannot split

| Category | Examples | In the lists? | Proposed handling |
|---|---|---|---|
| Tube sites, studios, pay sites | `pornhub.com`, `brazzers.com` | all lists | block |
| Live cams | `chaturbate.com`, `stripchat.com`, `cameraprive.com` | all lists | block |
| Creator subscription platforms | `onlyfans.com`, `fansly.com`; BR: `privacy.com.br`, `closefans.com` | first two widely; BR ones in none | block; add BR ones to the supplement after checking |
| Hentai, doujin, boorus | `nhentai.net`, `e-hentai.org`, `rule34.xxx`, `gelbooru.com`, `danbooru.donmai.us`, `e621.net` | widely (ShadowWhisperer misses `rule34.xxx`) | block |
| Erotic stories | `literotica.com`, `casadoscontos.com.br`, `contoseroticos.com.br` | first widely, BR ones partly | block; this is text, the vision layer will not catch it reliably |
| Escort listings | `skokka.com`, `fatalmodel.com`, `photoacompanhantes.com`, `listcrawler.eu` | mostly | block by default, own category so the user can see why |
| Leak forums and aggregators | `simpcity.*`, `coomer.*`, `kemono.*`, `fapello.com`, `erome.com` | partly; they change TLD often | block + brand-stem rule |
| AI companions and generators, "undress" tools | `candy.ai`, `crushon.ai`, `janitorai.com`, `spicychat.ai`, `undress.app` | HaGeZi best; others partial | block the explicit ones via the supplement; `character.ai` stays allowed |
| Adult games and shops | `f95zone.to`, `dlsite.com`, `dmm.co.jp` adult sections | partly | strict mode |
| Image boards | `4chan.org` (listed whole in four lists), `boards.4chan.org` | yes | strict mode only |
| Dating and hook-up | `adultfriendfinder.com`, `ashleymadison.com` vs `tinder.com`, `badoo.com`, `okcupid.com` | both kinds appear | block the explicit ones; allowlist mainstream dating |
| Sex shops, lingerie | UT1 keeps `lingerie` separate | scattered | not blocked by default |
| Sexual education and health | `scarleteen.com`, `plannedparenthood.org`, UT1 `sexual_education` | `scarleteen.com` wrongly listed in four lists | **hard allowlist** |
| Torrent and piracy indexes | `1337x.to`, `rutracker.org` | some lists | not this module's job; allowlist or leave to a separate category |
| Web proxies, VPN, DoH | `proxysite.com`, `croxyproxy.com` | a few in porn lists | a separate "bypass" category, see the DNS report |
| Mixed platforms | `reddit.com`, `x.com`, `tumblr.com`, `imgur.com`, `pinterest.com`, `vk.com`, `telegram` web, `discord`, `twitch.tv`, `deviantart.com`, `pixiv.net`, `archiveofourown.org`, search engines' image tabs | not listed as whole domains (correctly) | see below |

**Mixed platforms cannot be split by domain.** `reddit.com/r/<nsfw>` and `reddit.com/r/brasil`
are the same DNS name; so are a sensitive post on `x.com` and a news post, a Telegram channel
and a family chat. What existing tools do:

- CleanBrowsing's Family filter blocks "mixed content" sites such as Reddit outright; its Adult
  filter leaves them open (vendor documentation, via search).
- Bon-Appetit's policy refuses to list general-purpose platforms and lists a dedicated adult
  subdomain when one exists. UT1 keeps a separate `mixed_adult` category.
- Covenant Eyes relies on screen monitoring and reports rather than blocking; Canopy advertises
  real-time image filtering on any site (vendor claims, not independently tested).
- Search engines can be forced into SafeSearch by DNS: `forcesafesearch.google.com`
  (`216.239.38.120`), `restrict.youtube.com` (`216.239.38.120`) or
  `restrictmoderate.youtube.com` (`216.239.38.119`), `strict.bing.com` (`204.79.197.220`),
  `safe.duckduckgo.com` **[U for the last]**; details and caveats are in
  `research-dns-enforcement.md` section 7. Reddit, X, Telegram and Discord offer no such switch.

Proposal for ubiqX, in three levels the user chooses:

1. **Standard:** mixed platforms stay reachable. They go on a short built-in "mixed" list
   (about 60 names) that does one thing: while one of them is in the foreground, the vision
   check runs at its highest allowed cadence. Where the guard has the full URL (macOS and
   Windows browsers today), add path rules for the obvious cases: `reddit.com/r/<name>` when
   the name carries a block-grade keyword, `reddit.com/over18`, `x.com/search?...` with a
   block-grade query word, `/nsfw` path segments.
2. **Strict:** mixed platforms the user ticks are blocked whole, the way CleanBrowsing Family
   does it. The user sees the list and picks; nothing is hidden.
3. **Apps:** Telegram and Discord desktop are apps, not sites. The existing Focus guard already
   supports app targets; the vision layer covers what is on screen.

---

## 11. Update mechanism

- **Base in the binary:** the Core table (1.92 MB) and the hosts tier derived from it
  (126 KB gzip), refreshed at each app release. The app is useful offline and on first run.
- **Refresh from a GitHub release of this repo.** One fixed tag, e.g. `nofap-lists`, whose
  assets are replaced by CI (`gh release upload --clobber`):

  | Asset | Content | Size |
  |---|---|---|
  | `manifest.json` | `format`, `list_version` (monotonic `YYYYMMDDHHMM`), `min_app_version`, per-file `name`, `sha256`, `bytes`, `entries`, `licence`; upstream sources with commit SHA and content hash | < 4 KB |
  | `manifest.json.sig` | Ed25519 signature over the exact manifest bytes | 64 bytes (hex or base64) |
  | `blocklist-core.bin` | permissive tier | 1.9 MB |
  | `blocklist-extended.bin` | CC BY-SA tier | 4.2 MB |
  | `hosts-hot.txt.gz` | hosts tier | ~165 KB |
  | `blocklist-extended.txt.gz`, `LICENSE-CC-BY-SA-4.0.txt`, `NOTICE.txt` | plain form and notices for the share-alike pack | 2.5 MB |

  URL shape: `https://github.com/andreyquadros/ubiquitous-engine/releases/download/nofap-lists/manifest.json`.
  If the repository is private, release assets are not publicly downloadable; serve the same
  files through `services/ubi-api` instead.
- **Verification, in this order:** Ed25519 signature of the manifest against a public key
  compiled into the app (a new key pair, separate from the licence key; reuse
  `ubiqx_core::license::verifying_key_from_hex` and `ed25519-dalek`); `format` supported;
  `list_version` strictly greater than the installed one (no rollback); download; SHA-256 and
  byte length of each file equal to the manifest; structural check of the table (magic, count,
  sorted); canary check (a dozen must-block names hit, a dozen must-not-block names miss);
  write to a temporary file and rename. On any failure keep the previous list; the embedded
  Core is the floor that can never be removed.
- **Cadence:** check weekly with jitter, and on "enable protection"; conditional request
  (`If-None-Match`) so an unchanged manifest costs one small response. Weekly download
  volume for the Extended tier: about 4.4 MB. Delta updates are not worth the complexity.
- **Privacy of the update:** the request carries nothing about the user or their browsing; it
  shows GitHub only that some installation checked for a list. Say so in the settings screen.
- **Hosts tier refresh** needs elevation to rewrite the hosts block. Do not prompt weekly:
  apply the new tier the next time an elevated run happens for another reason, or when the
  user opens the protection screen and confirms.
- **CI job** (`.github/workflows/nofap-lists.yml`, weekly schedule + manual): run the build
  script, run the gates of section 12, sign with a repository secret
  (`NOFAP_LIST_SIGNING_KEY`), upload. A failed gate publishes nothing.
- **Custom lists:** "add a list by URL" downloads straight from the upstream to the user's
  machine, parses hosts or domain format with the same cleaning rules, and stores a local
  table. This is how a user can opt into HaGeZi or oisd without us redistributing GPL data.

---

## 12. Build-time script outline

`scripts/nofap/build_blocklist.py` (Python 3 standard library only: `urllib`, `hashlib`,
`gzip`, `json`, `struct`), run in CI and reproducible locally. Signing is a separate small
step so the script never sees the private key.

```
inputs (data/nofap/):
  sources.lock.json      url, commit SHA, sha256, licence, tier for each upstream file
  allow.txt              our allowlist (hand-maintained)
  extra-ptbr.txt         our curated additions (hand-maintained)
  mixed.txt              mixed platforms (not blocked; used by the guard)
  canary-block.txt       names that must be blocked
  canary-allow.txt       names that must never be blocked
  public_suffix_list.dat pinned copy, build-time only (MPL-2.0)
  crux-top1m.csv.gz      pinned copy, build-time only (CC BY 4.0)

steps:
  1  fetch each source at its pinned commit; verify sha256
     (Bon-Appetit: read meta.json first, then the hashed file name)
  2  parse: drop comments; drop the "0.0.0.0 " / "127.0.0.1 " prefix; lower-case; strip
     trailing dots; drop IP literals, localhost names, anything that is not a valid host name
  3  strip "www.", "m.", "mobile." when at least two labels remain
  4  drop every entry that is a public suffix (ICANN and private sections)
  5  drop "*.tumblr.com" and "*.blogspot.*" entries
  6  4skinSkywalker only: keep an entry when it has a block-grade keyword (core tier) or when
     another source also lists it (extended tier)
  7  drop entries equal to an allowlist name (an allowlisted parent does not remove a listed
     subdomain; an allowlisted name removes the same listed name)
  8  never rewrite an entry to its registrable domain
  9  tiers: core = permissive sources + extra-ptbr; extended = core + share-alike sources
  10 suffix-deduplicate inside each tier (drop a name whose listed parent already covers it)
  11 hosts tier: for each CrUX origin with rank bucket <= 500k whose host is covered by the
     tier, emit that host, its registrable domain and "www." + registrable domain (the last two
     only when the registrable domain itself is covered); cap at 30,000 names, lowest bucket first
  12 write blocklist-<tier>.bin (sorted unique truncated SHA-256 keys), the plain text of the
     share-alike tier, hosts-hot.txt.gz, NOTICE.txt, manifest.json

gates (any failure stops the job and publishes nothing):
  a  every name in canary-block.txt is covered; no name in canary-allow.txt is covered
  b  entry count of each tier within -10% / +25% of the previous published version
  c  no public suffix, no single-label entry, no IP
  d  no entry at or above the registrable domain of a CrUX top-1k origin unless that name is in
     data/nofap/reviewed-top.txt
  e  review report: listed names inside the CrUX top-10k that are new since the last run are
     printed for a human to read (about 1,400 hostnames at the first run)
  f  output is byte-identical when the script is run twice on the same inputs
```

A Rust unit test in the crate that owns the matcher loads the embedded Core and runs the two
canary files, so a bad embedded table fails `cargo test` on Linux.

Expected output at the first run (from today's data): Core 239,449 entries, Extended 523,112,
hosts tier about 32,000 names from Extended (24,541 from Core).

---

## 13. Attribution text to ship

`THIRD-PARTY-NOTICES` (shown in About, shipped with both products):

```
Adult-content blocklist (core tier) is compiled from:

- StevenBlack/hosts, "porn" extension - Copyright (c) Steven Black - MIT License
  https://github.com/StevenBlack/hosts
  which unifies: Sinfonietta/hostfiles (MIT, Copyright (c) 2016 Sinfonietta);
  Clefspeare13/pornhosts (MIT); bigdargon/hostsVN (MIT, Copyright (c) BigDargon);
  brijrajparmar27/host-sources (CC BY 4.0); tiuxo/hosts (CC BY 4.0)
- ShadowWhisperer/BlockLists, "Adult" - released into the public domain (Unlicense)
  https://github.com/ShadowWhisperer/BlockLists
- 4skinSkywalker/Anti-Porn-HOSTS-File - Copyright (c) 2022 4skinSkywalker - MIT License
  https://github.com/4skinSkywalker/Anti-Porn-HOSTS-File

The lists were merged, filtered against an allowlist and the Public Suffix List, and converted
to a hashed lookup table. The full text of the MIT License accompanies this notice.
Creative Commons Attribution 4.0: https://creativecommons.org/licenses/by/4.0/

Site popularity ordering uses the Chrome UX Report by Google, licensed under CC BY 4.0
(https://developer.chrome.com/docs/crux), via https://github.com/zakird/crux-top-lists.
```

`NOTICE.txt` inside the Extended pack:

```
This data file ("ubiqX adult-content blocklist, extended tier") is adapted from
Bon-Appetit/porn-domains (https://github.com/Bon-Appetit/porn-domains), which is licensed
under CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/), together with the
sources of the core tier listed above.

Changes: merged with other lists, filtered against an allowlist and the Public Suffix List,
de-duplicated, and converted to a hashed lookup table. A plain-text copy is published next to
this file.

This data file is licensed under CC BY-SA 4.0. The ubiqX application is licensed separately
(MIT).
```

The MIT notices need the upstream copyright lines exactly as in their licence files; copy them
at build time from the pinned commits rather than typing them.

---

## 14. Open points for the owner

1. Is the repository public? The release-asset update path depends on it.
2. Core only, or Core + Extended by default? Extended doubles real coverage and adds a
   share-alike data file with its own notice.
3. Should escort listings and erotic-story sites be blocked by default (proposed: yes), and
   image boards, adult game shops and mainstream dating left to strict mode (proposed: yes)?
4. Who maintains `allow.txt` and `extra-ptbr.txt`? Both need a person; the Brazilian
   supplement is the weakest part of every public list.
5. Not measured here and worth doing in CI, where the network is open: resolve the hosts tier
   and drop names that no longer resolve; fetch UT1 from its origin; read oisd's own terms.
6. The numbers in this report describe the lists on 2026-11-19. ShadowWhisperer, Bon-Appetit,
   HaGeZi and oisd change daily.

---

## Files

- This report: `docs/nofap/research/research-blocklists.md`
- Samples (first 50 lines each, with source and licence header):
  `docs/nofap/research/samples/stevenblack-porn-only.sample.txt`,
  `docs/nofap/research/samples/shadowwhisperer-adult.sample.txt`,
  `docs/nofap/research/samples/bonappetit-porn-domains.sample.txt`
- Sibling reports referred to: `docs/nofap/research/research-dns-enforcement.md`,
  `docs/nofap/research/map-focus-guard.md`

Content hashes of the files measured (SHA-256): StevenBlack `a75e8aec01c5...`, 4skinSkywalker
`8fe7424c7ab7...`, ShadowWhisperer `7315f4331640...`, blocklistproject `425590c37e3e...`,
Bon-Appetit `7ae9760f1e2d...`. Repository commits: StevenBlack `ce25ce24`, 4skinSkywalker
`e9cec3fb`, ShadowWhisperer `8ed49503`, blocklistproject `ee3bdbad`, Bon-Appetit `86c86b96`,
HaGeZi `9c00bfd1`, oisd `f8c19aea`.
