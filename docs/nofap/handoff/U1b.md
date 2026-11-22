# Hand-off: U1b Matcher, list reader and heuristics

The Rust half of U1. Where this file and the code differ, the code wins.

## Engineer

U1b is done: `list.rs`, `matcher.rs` and `heuristics.rs` have no stub left, `tests/embedded.rs` runs every row of design 3.6 against the embedded data, and integrator decisions 7, 8 and 12 are applied. The blocklist data was rebuilt once (list version 202611192117).

What exists now:
- `crates/ubiqx-nofap/src/list.rs`: the only `include_bytes!` / `include_str!` of the crate. `HashTable::parse` checks magic, format, reserved flags, length against count, and strictly ascending keys in one walk; `contains` is one SHA-256 and a binary search over the embedded bytes. The line lists (`hosts_tier()`, the three rule files, the two canary files) are split once per process.
- `crates/ubiqx-nofap/src/matcher.rs`: `RuleSet` (name, top-level and second-level forms), `never_block()`, `MatcherData::embedded()`, `Matcher::new / verdict / verdict_for_url / in_hosts_tier`, `normalize_host`, `parents`, `site_key`.
- `crates/ubiqx-nofap/src/heuristics.rs`: `host_rule`, `title_signal`, `path_signal`, `fold_lookalikes`, and the rule tables.
- `crates/ubiqx-nofap/tests/embedded.rs`: 19 tests on the real data. Counts come from `core.meta.json`.
- Data: `never-block.txt`, `mixed.txt`, `never-capture.txt`, `sources.lock.json` edited by hand; `core.bin`, `popular.bin`, `NOTICE.txt`, `core.meta.json` and the review file rebuilt. `hosts-tier.txt` came out byte-identical.

**Numbers (list version 202611192117)**

| Item | Value |
|---|---|
| `core.bin` | 239,521 keys, 1,916,192 bytes (same keys as before; only the version in the header changed) |
| `popular.bin` | 94,091 keys, 752,752 bytes (9 more: names that left `mixed.txt`) |
| `hosts-tier.txt` | 20,000 lines, unchanged |
| Rules | never-block 385, mixed 102, never-capture 272 |
| Canaries | 14 block, 28 allow |
| Tests | 41 unit tests (list 7, matcher 20, heuristics 14), 19 in `tests/embedded.rs` |

**Commands run, all passing on the final state**
- `cargo test -p ubiqx-nofap -- list:: matcher:: heuristics::` (41 passed) and `cargo test -p ubiqx-nofap` (41 + 19)
- `cargo test -p ubiqx-nofap --test embedded` (19 passed, 0.3 s in a debug build)
- `cargo clippy -p ubiqx-nofap --all-targets -- -D warnings`, `cargo fmt -p ubiqx-nofap -- --check`
- `python3 scripts/nofap/build_blocklist.py --offline` (all gates, second run byte-identical; a second build wrote nothing) and `--check` (rebuilt from the cached sources: byte-identical)
- `cargo check -p ubiqx-platform -p ubiqx-engine --all-targets` (the crates that depend on this one still compile)

Not run: `cargo check --workspace`, the frontend, any CI run. The build ran with `--offline` from the download cache; no source was fetched again.

The acceptance line of design section 13, `cargo test -p ubiqx-nofap list:: matcher:: heuristics::`, is not valid cargo syntax (cargo takes one filter before `--`). The working form is `cargo test -p ubiqx-nofap -- list:: matcher:: heuristics::`.

**How the heuristics were tuned (decision 12)**
I measured candidate rules on two sets built from the cached sources: the 307,114 names of the three public lists (do the rules still catch adult sites?) and the 895,033 Chrome UX Report hosts that are on no list and under no rule file (what do the rules block that nobody listed?). The plain substring rules of design 3.5 block a steady trickle of ordinary names there; besides the ones the hand-offs listed I found the Polish "odpornosc" (immunity), Spanish "por nuestra…" and "por nombre", Thai names (Somporn, Siriporn), the Finnish town Pornainen, the tractor maker's `escorts.co.in`, a cooperative at `coomer.com.co`, the webcomic community `kemono.cafe`, and the whole family of "cabin porn / food porn / bike porn" names. The final rules:

| Word | Counts when | Otherwise |
|---|---|---|
| `porn`, `xxx` | the rest of their token reads as site words: nothing or `HEAD_WORDS` before, nothing or `SITE_WORDS` after (`freeporn`, `pornovideos`, `xxx-tube`, `desixxx`); `xxx` must be exactly three | signal |
| long unambiguous words (`hentai`, `bokep`, `putaria`, …, 23 words) | anywhere in a label | |
| `putas` | at the start of a token, or after `de`, `las`, `as` | nothing |
| `nudes` | anywhere, except a token starting `denudes` | nothing |
| `escorts` | anywhere, except a token that is the word alone | signal token |
| `gostosas` | at the start of a label | signal |
| `creampie`, `peladas`, `gozando` | never | signal anywhere |
| 29 brand stems | as written, and as look-alikes (digits read as letters, repeated letters collapsed) for stems made of letters; `xvideos`, `redtube`, `hanime` only at the start of a token; `erome`, `cam4` only as a whole label; `missav` at the end of a token; `fansly` before site words | nothing |

Result on the measured sets: of the unlisted hosts the rules block 1,503 by keyword and 162 by TLD, and mark 934 as a signal. I read the keyword blocks (the names without `porn` or `xxx` one by one, the others grouped by what stands before and after the word) and found no ordinary site; among the 50,000 best ranked hosts no blocked name is a mainstream site. I did not read the 162 TLD blocks. On the public lists' own names the rules block 46,909 and mark 33,017 as a signal. A variant that tested only what follows `porn` and `xxx` blocked 60,282 of them, but also blocked `bikeporn.cz` and would block `cabinporn.com` and any Thai name ending a token. Requiring a known word before as well is the price of decision 12: 13,373 listed names with an unusual compound are a signal for the screen check and not a block.

### Deviations from the design

- **Keyword rules are placement rules, not substrings (decision 12).** See the table above. `creampie`, `peladas` and `gozando` are signal-grade only. Brand stems are 29: `e-hentai` is covered by `hentai`; `kemono` and `coomer` became signal tokens because their only safe form (a whole label) hit two ordinary sites in the measured set.
- **Public constants of `heuristics.rs` changed.** `BLOCK_SUBSTRINGS` and `BRAND_STEMS` (plain word lists) are gone; the rule tables are private. New public lists: `SIGNAL_SUBSTRINGS`, `SITE_WORDS`, `HEAD_WORDS`. `SIGNAL_TOKENS` gained `escorts`, `kemono`, `coomer`. No other crate referenced the removed constants.
- **Look-alike folding.** `fold_lookalikes` is the design's table. Matching also tries the label with repeated letters collapsed and without the `vv→w` step (no stem has a `w`, and `xvv1deos` needs `vv` to stay), and with edge digits read as letters (`0nlyfans`). Stems with a digit (`cam4`, `rule34`) and `xnxx`, `fansly`, `missav`, `erome`, `sxyprn` are matched as written only.
- **A title or a path signals on a block word wherever it stands.** Design 3.6 wants `/r/EarthPorn/` to be a path signal, and "earthporn" would not block as a host. So `title_signal` and `path_signal` fire on any block-grade word, proven place or not ("Pornichet - Wikipedia" is a signal too). Signal tokens (`sexy`, `adult`, …) do not count in titles. `verdict_for_url` reads the query as well as the path, so a search for a block word on a search engine is a signal.
- **The build script's keyword filter is not the run-time rule.** `block_grade` in `build_blocklist.py` stays the plain substring form: it only filters names that an adult-site list already carries. The script's comment said the two sets are kept equal; it now says they are not.
- **Precedence of the rule files.** A top-level or second-level rule of never-block (`gov`, `gov.*`) protects at every depth from the list, learned keys and keywords, and only the user's own entries beat it (the script's contract, rule 6). Mixed and never-capture are checked before the TLD and keyword rules, so a host under them is never keyword-blocked (design 3.3; the order in 3.4 step 4 reads the other way). Between mixed and never-capture the longer rule wins and never-capture wins a tie.
- **A snooze lifts keyword and TLD rules only.** The list, a learned key and a user block still decide first (the seeded doc of `AllowSource::Snooze`; design 3.4 lists a snooze beside the user allow). A snooze without an expiry is ignored.
- **A signal word beats the popular class.** A popular host whose name carries a signal is `Plain { signal: true }`, not `Popular`: the class enum cannot carry both, and design 4.2 looks at popular sites when a local signal fires.
- **Local comes first.** A local name or address is `Unknown(Local)` even when the user typed it into their block list. Shared address space (100.64.0.0/10), the unspecified and the broadcast address count as local too.
- **`verdict_for_url` accepts an address without a scheme** (`example.com/page`, `www.xvideos.com:443/x`), read as `https`, because that is how some address bars show it.
- **`normalize_host` also drops a scheme, path, query and fragment**, converts an internationalised name to punycode and returns IP literals without brackets.
- **`parents` gives nothing for a single label** (the script never tests one).
- **`HashTable::parse` rejects non-zero flags**, as the script's reader does.
- **`Debug` is hand-written** for `HashTable`, `MatcherData` and `Matcher`: counts only. The derived form printed megabytes of keys, the hosts tier, the salt and the user's sites.
- **Data, decision 7:** `bing.com` and `grok.com` are off `never-block.txt`; `grok.com` is also off `never-capture.txt`, or the screen check could never look at it. `x.ai` stays on both and covers `api.x.ai`.
- **Data, decision 8:** `web.telegram.org`, `discord.com`, `discordapp.com` and `web.snapchat.com` are on `never-capture.txt`. `telegram.org`, `discord.com`, `discordapp.com` and `discord.gg` left `mixed.txt`; `t.me`, `telegram.me`, `telegra.ph` and `snapchat.com` stay mixed. `web.snapchat.com` was my addition: it is a one-to-one messaging web app under a mixed platform.
- **The header of `mixed.txt`** said a mixed host is never blocked by the list; it now says a more specific listed subdomain stays blocked.

### Contract notes for other units

- **U2 (`sampling::decide`) and U5: `Verdict::Allowed(AllowSource::Snooze)` is a candidate at signal cadence.** It is the only `Allowed` the screen check must keep looking at (design 5.4). Treating it like the other `Allowed` values would make "Não era isso" a one-click pass for a day.
- **U2, U5: the local signal of a host is `Unknown(HostClass::Plain { signal: true })`.** `Popular` is returned only for a popular host without a signal word. The path signal is the second value of `verdict_for_url`; the title signal is `heuristics::title_signal(title)`.
- **U2 (learning):** a host returned as `Plain` may be in `popular.bin` when its name carries a signal word. If learning must exclude popular hosts strictly, check `HashTable::parse(list::embedded_popular(), POPULAR_TAG)` yourself.
- **U5:** `Matcher::new` walks the keys once and hashes nothing; rebuild it when `nofap_sites` changes and when a snooze expires (the matcher reads `now` only when built). `verdict` costs one SHA-256 per name of the parent walk (plus one per name when learned sites or snoozes exist) and at most one more for the popular table. Measured: in a debug build, 20 matchers built from the embedded data plus 2,000 verdicts take 0.3 s together; in a release build the keyword rules take about 1.5 microseconds per host (895,033 hosts in 1.4 s).
- **U5:** `verdict` takes a bare host in any form; `verdict_for_url` takes what the URL resolver returns, with or without a scheme. An empty string, `about:blank`, `file:` and browser-internal pages are `(Unknown(Local), false)`.
- **U5:** `in_hosts_tier` matches the host or a parent against the tier with `www.` twins folded; the index is built on its first call.
- **U2, U4 (`plan::validate`, the helper):** `matcher::never_block()` is the refusal list. `covers` is true for a name under a name rule, a top-level rule or a second-level rule. After decision 7 it no longer covers `bing.com` or `grok.com`, so a user may put them in the hosts block.
- **U3:** the matcher never needs the name of a learned site or a snooze, only `host_key`. Rows it ignores: a user row without a host or with an unusable one, `allow / learned`, `block / snooze`, a snooze without `expires_at`.
- **U6:** nothing here reaches the wire. For the strict-mode ticks: a user block on a mixed platform (`tumblr.com`) blocks the platform and every subdomain.
- **U9:** `python3 scripts/nofap/build_blocklist.py --check` passes at this commit without network. `cargo test -p ubiqx-nofap --test embedded` reads `data/core.meta.json` from the source tree at run time.
- **Whoever changes a rule file:** raise `list_version` in `sources.lock.json` and rebuild; `tests/embedded.rs` compares the rule counts with `core.meta.json`, so a rule file edited without a rebuild fails the Rust tests as well as `--check`.

### Open points

- **Adult sites the rules now leave to the screen check.** Among the 10,000 best ranked hosts that no public list names, about a dozen are plainly adult sites that are only a signal now, because the word before `porn` is a name (`rex…`, `jos…`, `masa…`, `uz…`) or the host is a bare brand (`kemono.cr`). They belong on a hand-maintained supplement, which today exists only for Brazilian names (`extra-ptbr.txt`). A person should decide whether to add a general supplement; the measurement is easy to repeat from the download cache.
- **Thai names at the end of a token** (`siriporn.com`) and German words in `-sporn` do not block, because an unknown head never confirms `porn`. The cost is that `<unknown word>porn` adult sites are a signal only.
- **Subdomain labels count.** `xxx.fandom.com` (a film wiki) is blocked by the `xxx` label, since a label that is the bare word is the strongest form there is. Shared hosts where anybody names a subdomain are the weak spot of any keyword rule.
- **Gambling mirrors with `xxx` in the name** are blocked when the rest of the token reads as site words, and are a signal otherwise. They are not adult sites; nobody asked for either outcome.
- **`.cam`, `.webcam` and `.tube` are signal TLDs** (design 3.5). In the measured set most `.cam` hosts are film-piracy mirrors, so the signal mostly sends ordinary pages to the screen check.
- **The "learned host leaves no name in a database dump" half of design 3.6** is a storage test (U3). Here the test shows that the row has no name and that a printed matcher holds none.
- **The performance test is a bound, not a proof.** It fails a matcher that hashes names when built; it would not notice one that only copied the keys into a set.
- The open points of U1a stand (the review list has not been read by a person; `extra-ptbr.txt` awaits a human check; the ShadowWhisperer pin can disappear). Its two "design decision needed" points are closed by decisions 7 and 8.
- A snapshot hook committed my first edits of the rule files, `sources.lock.json`, the script and the three Rust files as `127287d`, before the rebuilt outputs existed; `662df58` brought the outputs in line. Between those two commits `--check` would have failed.

## Reviewer

U1b holds up: every acceptance command passes and no input I tried makes it panic, but I found two ways an adult site was read as "not a site", ordinary names that were blocked, and inputs that took seconds to judge. All of that is fixed and tested; the data was rebuilt once (list version 202611192145).

**What I verified independently**
- **Acceptance commands**, run by me on the engineer's tree (`40a93d0`): `cargo test -p ubiqx-nofap -- list:: matcher:: heuristics::` (41 passed), `cargo test -p ubiqx-nofap --test embedded` (19 passed), `cargo test -p ubiqx-nofap`, `cargo clippy -p ubiqx-nofap --all-targets -- -D warnings`, `cargo fmt -p ubiqx-nofap -- --check`, `python3 scripts/nofap/build_blocklist.py --check` (rebuilt from the cached sources: byte-identical). The form in design section 13 (`cargo test -p ubiqx-nofap list:: matcher:: heuristics::`) is rejected by cargo, as the engineer says.
- **My own table of 381 names** the engineer's tests do not contain: 154 adult sites in eight languages (35 Brazilian or Portuguese), 44 look-alike spellings and mirrors, 170 ordinary names with risky letters (towns, surnames, Thai given names, companies, roman numerals, sizes, medical and help sites in Portuguese, English and Spanish) and 13 forms of one host (case, port, user-info, trailing dots, full-width letters and dots, percent-encoding). 17 verdicts were wrong; 15 are fixed, 2 are open points.
- **Longest match:** all 66 listed names below a rule-file name (section 2 of the review file) are `Blocked(List)`, also with `www.` and two more labels in front; the 14 names above them and an unlisted sibling under each are `Unknown(Mixed)`.
- **Every Chrome UX Report host** (999,075) through `Matcher::verdict`. I read all 1,512 keyword blocks and all 164 TLD blocks of hosts that are on no list. Ordinary sites among them: `xxx.fandom.com` (a film wiki) and `www.1337xxx.to` (a torrent mirror); two lottery mirrors (`…togel2xxx.org`) and two names of digits before `xxx` that I cannot judge. Everything else is an adult site, an adult shop or an adult app listing.
- **Hostile input:** 138 strings (empty, dots only, 300-letter labels, 2,000 and 50,000 labels, 100,000 letters, control characters, bidi and zero-width characters, emoji, broken punycode, every IP literal form including octal and hex, IPv6 with zones, bad ports, user-info tricks, browser-internal schemes) through `verdict`, `verdict_for_url`, `in_hosts_tier`, `normalize_host`, `host_rule`, `title_signal`, `path_signal` and `fold_lookalikes`. No panic. One pass over the set took 6.6 s before the fixes and takes 0.07 s now.
- **Cost (release build):** `Matcher::new` 0.17 ms (0.2 ms with `MatcherData::embedded()`); `verdict` 3 to 6 microseconds (one million hosts in 3.2 s); `verdict_for_url` 9 microseconds; the first `in_hosts_tier` 0.8 ms, later ones 0.3 microseconds.
- **Rule syntax:** the three forms on the real files, a brand TLD as a name rule, and that no rule line has another shape, are tested by the engineer and I found nothing to add.

**What was wrong, and is fixed** (commits `4381b69`, `568b060`)
- **An address with a second address in it was not a site.** A browser shows an address without its scheme. `pornhub.com/login?next=https://example.org/` contains `://`, so `verdict_for_url` parsed it as it stood, failed and answered `Unknown(Local)`: never blocked, never looked at. `verdict` and `normalize_host` took the host after the `://`, so the same string was judged as `example.org`. A scheme is now read only at the start of an address.
- **Reader and source views were not a site.** `about:reader?url=…` (Firefox), `read://…/?url=…` (Edge), `chrome-distiller://…/?url=…` (Chrome) and `view-source:…` show a page under their own scheme, the first three with its pictures. `verdict_for_url` answered `Unknown(Local)`; it now gives the verdict and the path signal of the page they show.
- **Ordinary names were blocked by a keyword:** `menudesign.com`, `nudesign.com.br` (`nudes`); `chentai.com`, `zhentai.com.cn` (`hentai`); `computaria.com.br`, `disputaria.com` (`putaria`); `seminovinhas.com.br` (`novinhas`); `funeralescorts.com`, `pilotcarescorts.com`, `securityescorts.co.za`, `medicalescorts.org`, `motorcycleescorts.com` (`escorts`). Each word keeps blocking around the exception (`sendmenudes`, `comicshentai`, `telegramputaria`, `londonescorts`); a trade before `escorts` leaves a signal.
- **Time quadratic in the length of the input.** A title, a query or a label that is one long run of letters with a match at every step (`xxxx…`, `pornporn…`) was walked once per match: seconds for 100,000 letters, minutes for a million, on the thread that runs the guard. A verdict also hashed a name once per label with no bound on the labels. The token is now found once, and a name longer than the DNS carries (253 bytes, 63 a label) is `Unknown(Local)`.
- **Random tokens in a URL were a signal.** 0.44% of addresses with a 160-character identifier spelled `xxx` by chance, which sends a search or a video page to the screen check for nothing. An unproven match now counts in free text only in a piece made of letters; the rate is 0.04%.
- **Data:** `xxx.fandom.com` is on `never-block.txt` (a keyword collision, like `xxxlutz.de`); `1337xxx.to` is on `mixed.txt` beside its siblings; four Brazilian sites that no public list names are on `extra-ptbr.txt` (`pornodoido.com.br` and `xvideoporno.com.br`, both among the 50,000 most visited hosts, `fotosporno.blog.br`, `videospornosbr.com.br`), so they are also in the hosts tier.
- **Vocabulary.** Adult sites with Portuguese, Spanish and Russian compound names were only a signal (`pornodoido`, `xvideoporno`, `fotosporno`, `pornoespanol`, `ruporno`, `incestoporno`, `hqporner`). I added about 170 words to `SITE_WORDS` and `HEAD_WORDS` and the endings `er`, `ers`, `uha` after `porn`. Measured as the engineer did: on the 906,811 Chrome UX Report hosts that are on no list the keyword blocks go from 1,512 to 1,567; I read the 55 new ones and all are adult sites. On the names of the public lists the blocks go from 40,426 to 42,394, and the two names that stopped blocking are ordinary (`commercialmedicalescorts.com`, `jp-chentaiji.net`).

**Numbers now (list version 202611192145)**

| Item | Value |
|---|---|
| `core.bin` | 239,525 keys, 1,916,224 bytes |
| `popular.bin` | 94,089 keys, 752,736 bytes |
| `hosts-tier.txt` | 20,000 lines |
| Rules | never-block 386, mixed 103, never-capture 272 |
| Tests | 47 unit tests (list 7, matcher 23, heuristics 17), 22 in `tests/embedded.rs` |

Commands on the final tree, all passing: the five cargo commands above, `python3 scripts/nofap/build_blocklist.py --offline` (all gates, second run byte-identical) and `--check`, `cargo check -p ubiqx-platform -p ubiqx-engine --all-targets`. `cargo clippy --workspace --all-targets -- -D warnings` also passed on the final tree, with the other units' files as they stood at that moment.

### Deviations from the design (the reviewer's)

- The engineer's deviations stand; I checked each against the code and found none hidden.
- `verdict_for_url` unwraps `view-source:` and the three reader views. The design does not mention them.
- `normalize_host` refuses a name over 253 bytes or with a label over 63, so such a name is `Unknown(Local)`.
- Five more block words have exceptions (`nudes`, `hentai`, `putaria`, `novinhas`, `escorts`), written as "not after" and "not before" lists in `heuristics.rs`.
- A weak match (a word in a place that does not prove it) is a title or path signal only in a piece without digits. `/r/EarthPorn/` is a signal; `/r/EarthPorn2/` is not.
- `tests/embedded.rs` reads `docs/nofap/research/blocklist-top10k-review.txt` (section 2) at run time, besides `data/core.meta.json`.

### Contract notes for other units (changes since the engineer's notes)

- **U5:** hand `verdict_for_url` whatever the address bar holds. It now handles an address without a scheme that carries another address in its query, `view-source:` and the reader views of Firefox, Edge and Chrome. Any other string without a usable host is still `(Unknown(Local), false)`.
- **U5 (learning):** hash the name `normalize_host` returns. It returns `None` for a name the DNS cannot carry.
- **U2, U5:** nothing changed in `Verdict`, `HostClass`, `HostRule` or any signature. `Allowed(Snooze)` must still be sampled at signal cadence (the engineer's first note); the integrator has not yet written that into `INTEGRATOR.md`.
- **U2, U4:** `never_block()` now also covers `xxx.fandom.com`.
- **U9:** `cargo test -p ubiqx-nofap --test embedded` needs `docs/nofap/research/blocklist-top10k-review.txt` in the checkout. It reads the file by lines, so line endings do not matter to it.
- Counts changed again; tests read them from `core.meta.json`.

### Open points (the reviewer's)

- **`shentai`** blocks. Shen Tai is a Chinese company name (`shentai.cn` in my table), but four names on the public lists are adult sites called so, and the rule cannot tell them apart.
- **`skokka.com.br`** and other adult sites without a telling word in their name (`videosdesexo.blog`, `amadorasbrasil.com`, `camerahot.com.br`, `24video.net`, `mmsbee.com` in my table) are plain unknown sites when no list names them. `br.skokka.com` is covered. Only the screen check catches the others.
- **A token that is `xxx` alone blocks**, also after a hyphen or digits: `superbowl-xxx.com` and `vol-xxx.example.org` would block, and the lottery mirrors and `0145xxx.com` do. I found no such ordinary site among the visited hosts, so I left the rule.
- **Words I only suspect:** `bukkake` (a noodle dish), `shemale` inside a Persian surname, `camgirl` in "camgirlfriend", `handjob` in a German compound, `deepthroat` (Watergate), `bokep` in "bokephoto", `transando`, `safadas`, `gostosas` at the start of a label. All are invented names; none is among the 906,811 visited hosts.
- **Web proxies** are out of reach of a name rule. Google Translate's proxy shows `beeg-com.translate.goog`; only a name with a brand or a keyword in it is caught there.
- **The vocabulary is a long tail.** After my additions 27,944 names of the public lists are still only a signal. Each new word buys a few dozen names. The engineer's open point stands: a hand-kept general supplement, read by a person, would do more than more words.
- **Not checked by me:** any CI run; the frontend.
- The engineer's open points stand, except "subdomain labels count" for `xxx.fandom.com`, which is now on `never-block.txt`.

Files: `/home/user/ubiquitous-engine/crates/ubiqx-nofap/src/list.rs`, `matcher.rs`, `heuristics.rs`; `/home/user/ubiquitous-engine/crates/ubiqx-nofap/tests/embedded.rs`; `/home/user/ubiquitous-engine/crates/ubiqx-nofap/data/`; `/home/user/ubiquitous-engine/scripts/nofap/build_blocklist.py`; `/home/user/ubiquitous-engine/docs/nofap/research/blocklist-top10k-review.txt`.

## Final review

Six findings of the adversarial review, each reproduced first by a test that failed. The data
was rebuilt once (list version 202611221200). `core.bin` and `hosts-tier.txt` keep the same
names; `popular.bin` lost one name (94,088 keys). Rules: never-block 399, mixed 105,
never-capture 277.

- **A second-level rule reaches only under a country code** (`RuleSet::suffix_covers`,
  `OPEN_COUNTRY_CODES`). `gov.*`, `edu.*`, `ac.*` and the rest covered any `label.tld`, so
  `gov.com`, `mil.win`, `ac.cab` and `edu.eu` were protected private names, and `gov.porn`
  or `videos.edu.xxx` beat the adult TLDs: `Allowed(BuiltIn)`, never looked at, never
  learned. Now the last label must be two letters and not `eu`. The Public Suffix List was
  not used: it leaves out `gov.uz`, `gov.hu`, `gov.kr`, `gov.np` and about 1,200 more public
  hosts of the Chrome UX Report. The public sectors under other domains that the report
  shows are name rules on both never-block and never-capture: `gov.wales`, `gov.scot`,
  `gov.taipei`, `gov.krd`, `edu.krd`. The build script has the same test
  (`is_public_sector_country_code`), and so does the helper's refusal list, which is the
  same `RuleSet`. Tests: `a_second_level_rule_reaches_only_under_a_country_code`,
  `second_level_rules_reach_only_under_a_country_code` (embedded), `rule_sets_read_the_three_forms`.
- **Public file hosts are mixed, not built-in.** `googleapis.com` and
  `githubusercontent.com` left never-block for mixed.txt, so `storage.googleapis.com`,
  `firebasestorage.googleapis.com` and `raw.githubusercontent.com` are looked at at signal
  cadence. These hosts stay on never-block: the sign-in, update and API hosts (`update`,
  `oauth2`, `openidconnect`, `securetoken`, `identitytoolkit`, `android`, `play`,
  `safebrowsing` and `generativelanguage.googleapis.com`) and the release downloads
  (`objects` and `release-assets.githubusercontent.com`; both products update from GitHub
  releases). OneDrive, SharePoint and iCloud were left alone: they are on never-capture on
  purpose (PRIVACY.md). No public list names a host under the two domains, so the list is
  unchanged. Test: `public_file_hosts_are_mixed_and_sign_in_and_update_hosts_are_allowed`.
- **`normalize_host` drops a leading `*.` or `.`** and refuses a label with anything but
  letters, digits, `-` and `_`. `*.adultsite-ana.example` was stored as typed and matched
  nothing. `typed_host` in the engine and `Matcher::new` both go through `normalize_host`, so
  the entry is stored, and an old row is read, as the bare name. Tests:
  `a_wildcard_typed_by_the_user_means_the_name`, `hosts_are_normalised`,
  `a_wildcard_in_the_user_list_blocks_the_name` (embedded).
- **`blob:`, `filesystem:` and local media.** `shown_page` unwraps `blob:` and `filesystem:`
  as it unwraps `view-source:`. A `file:` address whose last segment ends with one of
  `MEDIA_EXTENSIONS` is now `Unknown(Plain { signal: false })`, with the path signal, so a
  picture or video opened in a browser is sampled when "unknown sites" is on. Any other
  `file:` address (a PDF, a page, a folder) stays `Unknown(Local)`. The URL resolvers of
  macOS and Windows drop `blob:` and `filesystem:` today, so that half is defence in depth.
  For a `file:` page `normalize_host` gives no host, so nothing is learned and no card
  names a host. Test: `wrapped_addresses_and_local_media_are_judged`,
  `local_media_in_a_browser_is_an_unknown_site` (embedded).
- **Free text is read up to `MAX_TEXT` (8 KB)** (`readable_start`). When the cut falls inside
  a word of at most `MAX_WORD` bytes, it moves back to before that word ("pornichet" never
  reads as "porn"). A 2 MB path took 0.4 s in release and 2.6 s in debug; it is now under a
  millisecond. Test: `only_the_start_of_a_very_long_text_is_read`.
- **Block words that are also ordinary words.** `nudes` blocks when its tail is empty or
  reads as site words (linking `do`, `da`, `dos`, `das`, `de`, and `ex` / `exy` for "nude sex
  …"). `novinhas` blocks at the start of its token, after site words, or after `das`, `dos`,
  `de`, `so` or `tem`. `bukkake` does not block beside `udon` or `soba`. A lone `xxx` beside
  an edition word (`EDITION_WORDS`: congress, jornada, festival, superbowl, vol, …) does not
  block. All four are a signal where they do not block. The old exceptions (`menudesign`,
  `nudesign`, `nudestix`, `seminovinhas`) still carry nothing. New `Rule` fields
  `innocent_tails` and `ordinary_beside`, and a new `Before::HeadWordsOrAfter`;
  `After::NotBefore` had no user left and is gone. Measured with `host_rule` over the
  998,744 Chrome UX Report hosts: 3 hosts stop being blocked and become a signal
  (`nudesboys.com`, `nudesdosfamososnus.com`, `sendnudeselfie.com`). Over the 307,121 names
  of the three cached public lists: 89 names stop being keyword blocks, all of them
  "nude s…" names (`nudestars`, `nudescene`, `nudeselfie`). The list still blocks every one
  of them. Nothing changed for `novinhas`, `bukkake` or `xxx` on either set. Test:
  `block_words_that_are_also_ordinary_words_do_not_block`. `nudesexpics.example` in
  `the_exceptions_for_ordinary_words_are_narrow` still blocks.

Outside the files this unit owns: `docs/nofap/research/blocklist-top10k-review.txt` was
rewritten by the build (only its version line changed).

Commands on the final tree, all passing: `cargo test -p ubiqx-nofap` (309 unit, 26 embedded,
12 naming), `cargo test -p ubiqx-engine -p ubiqx-platform`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`,
`python3 scripts/nofap/build_blocklist.py --offline` (all gates, second run byte-identical) and
`--check`.

Open points:
- `OPEN_COUNTRY_CODES` holds only `eu`. Other country codes sold as generic names (`.tv`,
  `.cc`, `.nu`, the old free ones) may have a private `gov.` or `edu.` name. The Chrome UX
  Report shows none, and some of them have a real public sector under the label (`gov.tv`).
- PRIVACY.md lists "um player de vídeo ou visualizador de imagens" among what is
  photographed. A local picture or video opened in a browser now counts as that. The text is
  still true, but it names apps only, and the integrator may want to say so.
- `fonts.googleapis.com` and `ajax.googleapis.com` are now under mixed. They are never in
  front, and a user who blocks `googleapis.com` whole in "modo estrito" breaks web fonts on
  purpose.
