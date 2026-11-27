# Hand-off: U1a Blocklist build pipeline and data

Written from the engineer's and the reviewer's reports. Where this file and the code differ, the code wins.

## Engineer

U1a is complete and pushed: the build script, the embedded data, both notice files and the review list. The script passes every gate, a second run in a fresh interpreter is byte-identical, `--check` passes with and without the download cache, and a rebuild from an emptied cache (real downloads at the pinned commits) gave the same bytes.

**Numbers (list version 202611192000)**

| Item | Result | Bound |
|---|---|---|
| `core.bin` | 239,409 keys, 1,915,296 bytes | 215,000 to 300,000 keys, at most 2.4 MB |
| `popular.bin` | 94,062 keys, 752,520 bytes | at most 1 MB |
| `hosts-tier.txt` | 20,000 lines, 329,571 bytes | at most 20,000 lines |
| Rule files | never-block 381, mixed 94, never-capture 264 rules | |
| Canaries | 14 block, 20 allow | |
| `extra-ptbr.txt` | 27 names, all absent from the public lists | |
| `.br` entries in core | 692 | |
| Review list | 1,094 covered origins in the CrUX top 10,000, 219 of them in the top 1,000 | |

Hosts-tier lines by CrUX rank bucket: 402 (1k), 830 (5k), 796 (10k), 4,109 (50k), 3,120 (100k), 10,743 (500k). The first 9,257 lines are rank 100,000 or better, so a Windows cap of 8,000 cuts inside the 100k bucket.

**Entries per source**

| Source | Unique valid names | Used in core |
|---|---|---|
| StevenBlack porn-only | 76,806 | 64,093 |
| ShadowWhisperer Adult | 213,982 | 213,960 |
| 4skinSkywalker (keyword-filtered) | 70,171 | 8,535 |
| `extra-ptbr.txt` | 27 | 27 |

The union is 257,054 names before suffix de-duplication.

**Sources against the design's expectations**
- All seven URLs answered 200 at the pinned commits, and the SHA-256 values equal the research report's. No file had moved and there was no 404.
- StevenBlack gives 76,806 valid names, not 76,810: three names contain `_` and are dropped as invalid host names. 4skinSkywalker gives 70,171, not 70,174, for the same reason plus one IP literal.
- The 4skinSkywalker filter keeps 8,535 names, where the research measured 10,394 with its wider keyword set. The script uses the block list of design 3.5.
- Core is 239,409 entries against the design's "about 239,449".
- The top-10,000 review list has 1,094 origins, not "about 1,400".
- About 220 of the 1,000 most visited CrUX origins are adult sites, so the top-1,000 gate cannot pass as the design words it (see deviations).

**What the data does with the traps of 3.4 to 3.6**
Every name in the 3.6 false-positive row is looked up by key in the built table and is not covered. `essex.gov.uk`, `sussex.ac.uk`, `planalto.gov.br`, `iowasexoffender.gov`, `xxxlutz.de` and `fapesp.br` are under never-block rules. `reddit.com` is on `mixed.txt`. `PT.PornHub.com.` and `www.xvideos.com:443` are covered after normalisation. `fc2.com` and `blog.fc2.com` are not covered, while `adult.contents.fc2.com` stays listed at its depth.

**Corrections made to the public lists**
- Taken off as general sites named by mistake: `yandex.by`, `kinopoisk.ru`, `hmv.co.jp`.
- Moved to `mixed.txt`: `ekstrabladet.dk`, `fc2.com`, `mangago.me`, `manhwaweb.com`, `suruga-ya.jp`, `1337x.to`, `rutor.info`, `4chan.org`, `boards.4chan.org`, `vk.com`, `dlsite.com`, `dmm.com` and about ten more.
- Removed by never-block rules: `scarleteen.com`, `uos.ac.uk` (a university), `xxx.lanl.gov`, `escortsgroup.com` (a tractor maker) and others.
- Section 3 of the review file lists all 35 removals with the rule and the source.

Two `wip(nofap)` snapshot commits (`cd0be7f`, `fb03e3d`) appeared on the branch while I worked. I did not make them. I inspected `cd0be7f`, which holds a mid-edit state of two of my files; my later commits supersede it. I did not inspect `fb03e3d`.

### Deviations from the design

- Top-1,000 gate. The design's wording ("no entry at or above the registrable domain of a CrUX top-1,000 origin") cannot pass: 219 of the 1,000 most visited origins are covered adult sites (pornhub, xvideos and xhamster language subdomains and others). The gate now fails when a listed name covers a top-1,000 origin and is not in `REVIEWED_TOP_1K`, a constant of 166 names in the script. The research had a `reviewed-top.txt` for this; the design dropped it and I may not add files.
- `REVIEWED_TOP_1K` was read by me from the names and from general knowledge; the sandbox cannot open the sites. About ten entries were judged from the name alone: taktube.net, theblowers.com, zebawy.com, topgirl.co, kamareels2.com, sexwabs.com, videoxx.com, darkero.com, av19t.com, indoav.com, luticlip.com.
- `UPSTREAM_MISTAKES` (script constant): yandex.by, kinopoisk.ru and hmv.co.jp are removed from the public lists without going on a rule file. A rule file would also switch the vision check off for them or mark them as mixed.
- The build removes listed names equal to a rule of `mixed.txt` or `never-capture.txt` as well, not only `never-block.txt` as 3.2 says. Section 3.3 says those classes are never blocked.
- `google.com` is deliberately not on never-block. Only Google's sign-in, update, API and Android hosts are, so that Google search stays an ordinary popular site that the vision check can look at when a signal fires.
- `extra-ptbr.txt` has 27 names, not "about 100". Its header says it awaits a human check and marks each name [R] research, [G] a visited, unlisted host with an unambiguous Portuguese adult word, or [K] my own knowledge (only d4swing.com). closefans.com, which the design names, is not in the CrUX top million and is unverified.
- candy.ai, crushon.ai, spicychat.ai and undress.app are already on the public lists. janitorai.com (CrUX top 1,000) went to `mixed.txt` instead of the block supplement, because it is a general chat platform.
- Hosts tier order: lower CrUX rank bucket first; inside a bucket the order is a SHA-256 of the name, not alphabetical. CrUX gives no finer rank, and an alphabetical cut at the cap kept only names starting a to e.
- The hosts tier excludes names under any of the three rule files, not only never-block.
- `popular.bin` holds exact normalised hosts (leading `www.` removed), excludes public suffixes, and is meant for an exact lookup without a parent walk. Otherwise every blog on a hosting platform would inherit the popular class. The design only says "same format".
- Prefix stripping removes `www.`, `m.` and `mobile.` repeatedly, and only while a registrable name remains. The design's "when two labels remain" would cut `mobile.co.uk` down to `co.uk`. core.bin is byte-identical either way on today's data.
- Names with `_` are dropped as invalid (3 distinct names upstream).
- The 4skinSkywalker keyword filter leaves out the brand stems erome, kemono, coomer, hanime and cam4, because they collide with ordinary names (see open points).
- `list_version` comes from `sources.lock.json`, not from the clock, so the build is reproducible. `--pin` sets it; it must be raised by hand when only a hand-maintained file changes.
- Additions the design does not list: `--selftest`, `--pin`, `--offline`, `--ca-bundle`; a tenth pinned file (StevenBlack `readmeData.json`) with a gate that fails when the upstream lists inside porn-only or their licences change; a gate that THIRD-PARTY-NOTICES carries every copyright line of NOTICE.txt; a gate that no listed name sits at or above a NAME rule.
- The review file has two extra sections: listed names kept below a protected name (63), and listed names removed (35).
- `flags` is 0 in both tables and the script's reader rejects any other value. The design leaves the field unspecified.
- THIRD-PARTY-NOTICES and NOTICE.txt say "this application" and carry no product name, since both products ship them.
- Design 3.1 says the lists merged into StevenBlack's file are described by `extensions/porn/*/update.json`. The notice cites `readmeData.json` (key "porn-only") instead, which is the one index of all of them; I read the six update.json files by hand and they agree.

### Contract notes for other units

- Table format, exactly as seeded in `list.rs`: 24-byte header (`UBXBL1\0\0`, format u16 = 1, flags u16 = 0, count u32, list_version u64), then `count` strictly ascending u64 keys, all big-endian; file length is 24 + 8 * count. Key = first 8 bytes of SHA-256(tag || name); the tags include their trailing NUL. The full contract is in the docstring of `scripts/nofap/build_blocklist.py` and in `core.meta.json` under `format`.
- `core.meta.json.vector` is an array of `{table, tag, name, key}`. `key` is a 16-digit lower-case hex string, and `tag` contains a real NUL written as the JSON escape `\u0000`. Names: example.com, sub.example.co.uk, xn--bcher-kva.example, a.b, for both tags.
- `core.meta.json` also has `core.count`, `popular.count` and `hosts_tier.lines` for the "counts equal core.meta.json" test, the sha256 and byte size of each output, and `inputs` (sha256 of every hand-maintained file).
- Name normalisation for lookups: lower-case ASCII, punycode, no trailing dot, port or user-info, and a leading `www.` removed when two labels remain. Core lookup is the host and then each parent down to two labels. Popular lookup is the exact normalised host with no parent walk; `www.google.com` must be normalised to `google.com` before the lookup.
- Rule files (`never-block.txt`, `mixed.txt`, `never-capture.txt`) have three forms, matching the seeded `RuleSet` doc. `example.com` covers the name and its subdomains. `gov` (one label) covers that TLD. `gov.*` covers any name with the label `gov` immediately before its last label, whatever the last label is. Lines that are empty or start with `#` are ignored; there are no end-of-line comments; no line has leading or trailing spaces.
- A TLD or `x.*` rule decides at every depth; the build removed every listed name under one. For a NAME rule the longest match applies: 63 listed names sit below a NAME rule (for example `adult.contents.fc2.com` under `fc2.com`, `nsfw.reddit.com` under `reddit.com`) and must stay blocked.
- `never-capture.txt` repeats the banks, payment services, public-sector rules and AI providers of `never-block.txt` on purpose, so each file can be read on its own.
- `hosts-tier.txt`: one name per line, LF, no comments or blank lines, 20,000 lines, best first, so the first N lines are the right cut for a smaller cap. No line is under any rule of the three rule files. Every line passes the is_valid_hostname rule of design 2.5.
- `canary-block.txt` has 14 names; the last, `privacy.com.br`, comes from `extra-ptbr.txt`. `canary-allow.txt` has 20 names and includes `fc2.com`, `blogspot.com`, `tumblr.com` and `google.com`.
- `extra-ptbr.txt` is a build input only. `sources.lock.json` has `schema`, `list_version` and `sources[]` with `id, role, repo, commit, path, url, sha256, bytes, licence` (plus `format` and `filter` for the three lists).
- THIRD-PARTY-NOTICES at the repository root is hand-written plain text for the About screens. `--check` fails if it lacks any copyright line of `NOTICE.txt`.
- `.gitignore` now ignores `/scripts/nofap/.cache/`.
- New file `docs/nofap/research/blocklist-top10k-review.txt`, generated by the build. `--check` compares it byte for byte when the sources are cached, so it must not be edited by hand.

### Open points

- A person has not read the review list (`docs/nofap/research/blocklist-top10k-review.txt`, 1,094 origins). I could not judge these top-10,000 names: gate.cc, esa.co.za, fngml.com, fangsung.com, curiowhisper.com, djatoya.com, goldengate.hu, hiden.live, jalf.com, laxd.com, napiszex.com, pxp.pink, qfilm.tv, samusar.com, sawstory.com, zmero.com, bokacoda.com, pvip.se, my-cima.net (possibly a film streaming site). They stay listed because a shipped public list names them.
- `extra-ptbr.txt` awaits a human check, and Brazilian coverage is still thin (692 `.br` names).
- For the heuristics author: `xvv1deos.com` cannot match `xvideos` with the folding table of 3.5. `vv->w` gives `xwideos` and `1->i` gives `xvvideos`; a repeated-letter collapse is needed for that 3.6 row to pass.
- For the heuristics author: several brand stems are unsafe as plain substrings. Among unlisted CrUX hosts, `erome` matches about 25 innocent names (aeromexico.com, jeromes.com, queromeuape.com.br), `hanime` matches evchanime.ru and watchanimeworld.one, and `kemono` matches pokemonoriginheartgold.com. Folding `cam4` gives `cama` and `rule34` gives `ruleea`, so stems with digits must be matched unfolded.
- For the heuristics author: the block substrings `xxx`, `porn` and `putas` have innocent hits (XXXL sizes, `terminalxxxix.com.br`, mixxx.org, xxxx.com.au, Pornic and Pornichet, `uspornybalik.cz`, `atputasvakars.lv`). I put the ones I found on never-block, but that is a patch and new ones will appear. Gambling mirrors with `xxx` in the name (about 15) are left alone.
- `bing.com` and the other Microsoft names are on never-block, as design 2.2 requires. The effect is that Bing image and video search is never looked at by the vision check and cannot be blocked by the user. Splitting the helper's refusal list from the matcher's allowlist would fix it; that is a design decision.
- Google and Bing image search share their host with web search, so they are not on `mixed.txt`. Only `images.google.com`, `images.search.yahoo.com` and `image.baidu.com` are. They rely on the window-title signal.
- `video.fc2.com` and `live.fc2.com` stay blocked, because StevenBlack lists them at that depth, while `fc2.com` is mixed. A person may prefer them mixed.
- Sinfonietta's snuff list comes inside StevenBlack's porn-only file, so a few gore sites (kaotic.com) are blocked too.
- `dmm.co.jp` (the adult storefront) stays listed and `dmm.com` is mixed. The research had put dmm.co.jp under strict mode.
- ShadowWhisperer squashes its history, so the pinned commit `8ed49503` can disappear. The cache is gitignored, so a later full rebuild would then need `--pin`, which changes the data.
- I did not check the CI runs started by my pushes. The `--check` step in the rust-linux job is U9's to add; `--check` needs Python 3.10 or later and no network.
- No Rust test has read these files yet. `tests/embedded.rs` and `list.rs` come from the other half of U1.

## Reviewer

U1a holds up: every acceptance command passes and the table matches the raw sources, but the shipped data blocked a number of ordinary sites, some of them in the hosts tier. I fixed those and rebuilt.

**What I verified independently**
- **Pins:** I downloaded all 10 pinned files with curl and compared SHA-256 and size with `sources.lock.json`. All match.
- **Table format:** my own reader, written from design 3.2 only, parses `core.bin` and `popular.bin` (magic, format 1, flags 0, strictly ascending keys, length 24 + 8n).
- **Every key explained:** a second pipeline of mine hashed every name of the three raw lists plus `extra-ptbr.txt`. All 239,409 keys of the engineer's table, and all 239,521 after my fixes, come from a source name. There are no unexplained keys, no entry with a listed ancestor and no public suffix.
- **Canaries and traps:** every canary-block name is covered (also with `www.`, deep subdomains, upper case and a trailing dot). No canary-allow name is. No name of the 3.6 false-positive row is covered. `adult.contents.fc2.com` is listed at its depth, and `fc2.com` and `blog.fc2.com` are not.
- **Samples:** 128 of 139 well-known adult sites are covered. Of the 11 misses, some are names I made up, and most of the rest carry a block-grade keyword or an adult TLD. Of 341 mainstream names (banks, OS update and sign-in hosts, AI providers, CDNs, shared hosts), the only wrong hits were `b-cdn.net` and `grindr.com`.
- **Hosts tier:** 20,000 unique valid lines, none under any of the three rule files, all covered by `core.bin`.
- **Licences:** the four MIT copyright lines equal the pinned licence files. The StevenBlack index names exactly the five upstreams and licences in the notices. ShadowWhisperer's LICENSE at the pinned commit is the Unlicense. This matches design 3.1.
- **Reproducibility:** two offline builds with different hash seeds, a build from an emptied cache with real downloads (in a scratch copy), and `--check` under Python 3.11, 3.12 and 3.13 all give the committed bytes.
- **Gates:** 12 negative tests in a scratch copy of the tree each exit 1 with the right message.

**What was wrong, and is fixed**
- **Ordinary sites on the list.** The engineer read only the top-10,000 list. Between rank 10,000 and 500,000 the public lists name a ticket site (`more.com` and its sign-in host), MercadoLibre Colombia product pages, a supermarket, a university's virtual campus, magazines, a film database, a podcast service, a Tinder policy page and a sperm bank. About 100 hosts-tier lines for such sites would have gone into users' hosts files. `UPSTREAM_MISTAKES` now has 37 names instead of 3.
- **`b-cdn.net` was listed whole.** It is a CDN suffix that is not on the Public Suffix List, so every site served from it counted as listed, and `lgbtqiaplus.b-cdn.net` was in the hosts tier. It is removed; the 153 subdomains the lists name stay listed one by one.
- **Mixed platforms.** `2chan.net`, the joyreactor family and eight torrent indexes went to `mixed.txt`, as `4chan.org` and `1337x.to` already were.
- **`grindr.com`** is off the list. No other mainstream dating app is listed, and the design says an orientation is not a risk marker. This is a judgement call; it is easy to reverse.
- **Rule files.** `never-block.txt` gained `unex.es`, `banco.bradesco`, `bancodobrasil.com.br`, the Snap and Launchpad download hosts and `findahelpline.com` (a help resource of design 5.7). `never-capture.txt` gained the two bank names and `cohere.com`, `groq.com`, `openrouter.ai`.
- **New guard.** `canary-allow.txt` has eight shared delivery and hosting names, so a list that names one of them whole fails the build.
- **Script.** `--check` printed "byte-identical" even when a rebuilt output differed. `--pin` added 1 to a YYYYMMDDHHMM number.

**Numbers now (list version 202611192045)**

| Item | Value |
|---|---|
| `core.bin` | 239,521 keys, 1,916,192 bytes |
| `popular.bin` | 94,082 keys, 752,680 bytes |
| `hosts-tier.txt` | 20,000 lines, 329,636 bytes |
| Rules | never-block 387, mixed 106, never-capture 269 |
| Canaries | 14 block, 28 allow |
| Review file | 1,093 top-10k origins, 66 kept below a protected name, 84 removed |

An automatic snapshot committed and pushed most of my data changes before I could: they are in the two `wip(nofap)` commits `cc0c064` and `d6e2835`, mixed with another unit's files. HEAD passes `--check`.

Files: `/home/user/ubiquitous-engine/scripts/nofap/build_blocklist.py`, `/home/user/ubiquitous-engine/crates/ubiqx-nofap/data/`, `/home/user/ubiquitous-engine/docs/nofap/research/blocklist-top10k-review.txt`, `/home/user/ubiquitous-engine/THIRD-PARTY-NOTICES`.

### Deviations from the design

- All deviations in the engineer's report stand; I checked them against the code and found none hidden. The ones below are mine or changed by me.
- `UPSTREAM_MISTAKES` grew from 3 to 37 names. They were judged from the name and general knowledge; nobody opened the sites. The least certain are `my-cima.net` (taken as a film streaming site; four mirror hosts removed with it), `acidcow.com` and `crazydaysandnights.net`.
- `grindr.com` is taken off the list and treated as an ordinary site, although two public lists name it. Reason: the lists name no other mainstream dating app, and design 3.5 says an orientation is not a risk marker. Explicit hookup sites (adam4adam, manhunt, sniffies, recon) stay listed.
- `b-cdn.net` is handled through `UPSTREAM_MISTAKES`, not through a general rule for shared hosts. The design's "drop public-suffix entries" does not catch it because it is not on the Public Suffix List.
- `mixed.txt` now has 106 rules (design: "about 60"). Added: `2chan.net`, `reactor.cc`, `joyreactor.cc`, `joyreactor.com`, and the torrent indexes `1337xx.to`, `rutracker.net`, `rargb.to`, `thepiratebay0.org`, `kickasstorrents.to`, `katcr.to`, `torrentdownloads.pro`, `dmhy.org`.
- `canary-allow.txt` has 28 names (design: "a dozen"). The eight new ones are shared delivery and hosting names: `b-cdn.net`, `cloudfront.net`, `amazonaws.com`, `akamaized.net`, `googleusercontent.com`, `github.io`, `pages.dev`, `wordpress.com`.
- `never-block.txt` and `never-capture.txt` include `banco.bradesco`, a name under a brand TLD. It is a NAME rule of two labels, valid in the existing syntax.
- `list_version` was raised by hand twice (202611192039, then 202611192045), following the script's own rule for changes to hand-maintained files.

### Contract notes for other units

- Counts changed: `core.bin` 239,521 keys (1,916,192 bytes), `popular.bin` 94,082 keys (752,680 bytes), `hosts-tier.txt` 20,000 lines (329,636 bytes), `list_version` 202611192045. A Rust test must read counts from `core.meta.json`, not hard-code them.
- `canary-allow.txt` now has 28 names. The eight new ones are shared hosting names; five of them (`cloudfront.net`, `amazonaws.com`, `github.io`, `pages.dev`, and `googleusercontent.com` in part) are public suffixes or near it. `tests/embedded.rs` must only assert "not Blocked" for them, not a particular class; `wordpress.com` is in `popular.bin`, the others are not.
- `mixed.txt` has 106 rules, `never-block.txt` 387, `never-capture.txt` 269. `core.meta.json.rules` carries the counts.
- 66 listed names now sit below a NAME rule of a rule file (was 63); new are `img.2chan.net`, `img10.joyreactor.cc`, `img10.reactor.cc`. The matcher's longest match must keep them `Blocked(List)` while the parent is `Unknown(Mixed)`.
- `never-block.txt` and `never-capture.txt` contain `banco.bradesco`: a two-label NAME rule whose last label is a brand TLD. `RuleSet::parse` must not assume a known TLD.
- `UPSTREAM_MISTAKES` in `scripts/nofap/build_blocklist.py` removes exact names only. Listed subdomains of a removed name stay listed (153 under `b-cdn.net`, `xxx.startpagina.nl`).
- Table format, key derivation, rule syntax, `core.meta.json` layout and the hosts-tier format are unchanged from the engineer's report.
- `--check` now prints "rebuilt from the cached sources: byte-identical" only when no rebuilt output differs. Exit codes are unchanged.

### Open points

- DESIGN DECISION NEEDED: `bing.com` is on `never-block.txt`, as design 2.2 requires for Microsoft names. Under design 4.2 an `Allowed` host is never a vision candidate, so Bing image and video search is never looked at and cannot be blocked by the user. The fix is to separate the helper's refusal list from the matcher's allowlist, in `matcher.rs` / `hosts.rs` (not my files). Taking `bing.com` off the data file alone would let a user-typed Bing entry reach the hosts file, which the design forbids.
- Same class, smaller: `x.ai` and `grok.com`, `linkedin.com`, `github.com`, `msn.com` are on never-block, so vision never looks at them.
- The long tail is not vetted. I read the covered names down to rank bucket 500,000 only where the name carries no adult-looking word (about 3,000 names), by name and general knowledge. ShadowWhisperer's list, which supplies 189,000 names found on no other list, visibly contains domains that look ordinary (`minnesotaopera.org`, `homeofficeseattle.com`, `getnamibiavisa.com`); most are probably expired domains now serving adult spam, and they are outside the hosts tier.
- Left listed, borderline: sex-toy and lingerie shops (`lovehoney.*`, `lelo.com`, `agentprovocateur.com`, `fredericks.com`, `e-kondomy.cz`), `maxim.com`, `modelmayhem.com`, `omegle.com`, `chatroulette.com`, gore sites, general dating sites (`pinalove.com`, `simpatie.ro`, `superencontre.com`), image and file hosts (`imagebam.com`, `streamtape.com`, `k2s.cc`, `bunkr.*`), anime boards (`donmai.us`, `yande.re`, `konachan.com`, `inkbunny.net`, `booru.org`).
- Torrent indexes of rank bucket 500,000 are still listed (`nyaa.net`, `tokyotosho.info`, `torrentfunk.com`), while the eight of bucket 100,000 or better are now mixed. The line is arbitrary.
- Subdomains kept blocked under a now-mixed parent: `img.2chan.net` (the main Futaba image board), `img10.joyreactor.cc`, `img10.reactor.cc`, besides the engineer's `video.fc2.com`, `live.fc2.com`, `summary.fc2.com`. That is the design's longest-match rule; a person may prefer them off.
- `vidiox-web.b-cdn.net` (rank bucket 50,000) was covered only through `b-cdn.net`. No list names it, so it is now an ordinary popular site. I do not know what it is.
- `gov.*`, `edu.*`, `ac.*` decide at every depth, so hacked school and government subdomains that serve adult spam are never blocked and never looked at (`demo.corjesu-malang.sch.id` is listed; `jutiporn.ac.th` was removed by the rule). This is the design's rule.
- Design 3.3 puts Telegram web and Discord on `mixed.txt`. That means private conversations on `web.telegram.org` and `discord.com` are pictured at signal cadence, while `web.whatsapp.com` and `messenger.com` are on never-capture. Someone should confirm this is intended.
- `extra-ptbr.txt` is unchanged at 27 names and still awaits a human check. Brazilian coverage is thin (692 `.br` names). Among my test names, `testosteronahot.com.br` is a likely real site that is neither listed nor caught by a keyword.
- `REVIEWED_TOP_1K` (166 names) was not re-read name by name. I read the top-10,000 list once and found nothing mainstream except `my-cima.net`. The names the engineer flagged as unknown (`gate.cc`, `qfilm.tv`, `curiowhisper.com`, `jalf.com`, `laxd.com` and others) are still unknown to me and stay listed.
- `docs/nofap/research/blocklist-top10k-review.txt` and `THIRD-PARTY-NOTICES` are not under `eol=lf` in `.gitattributes` (another unit's file). It only matters if `--check` is ever run with cached sources on a CRLF checkout.
- CI does not run `--check` yet (U9). The engineer's other open points (the ShadowWhisperer pin can disappear; `xvv1deos.com` cannot match with the 3.5 folding table; ambiguous brand stems) stand unchanged.
- A process outside my control commits and pushes `wip(nofap)` snapshots of the working tree. Two of them carry my data changes together with another unit's files, so those changes have no commit message of their own.

