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

Files: `/home/user/ubiquitous-engine/crates/ubiqx-nofap/src/list.rs`, `matcher.rs`, `heuristics.rs`; `/home/user/ubiquitous-engine/crates/ubiqx-nofap/tests/embedded.rs`; `/home/user/ubiquitous-engine/crates/ubiqx-nofap/data/`; `/home/user/ubiquitous-engine/scripts/nofap/build_blocklist.py`; `/home/user/ubiquitous-engine/docs/nofap/research/blocklist-top10k-review.txt`.
