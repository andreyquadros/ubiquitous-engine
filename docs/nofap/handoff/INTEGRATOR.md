# Integrator decisions

Decisions taken while the units land. They override `docs/nofap/DESIGN.md` where they differ.
Each unit reads this file and the other files of this directory before it starts, and writes its
own `docs/nofap/handoff/<unit>.md` when it finishes (deviations, contract notes, open points).

## Standing

1. **Display name.** The owner asked for the name "nofap": `DISPLAY_NAME = "NoFap"`. The machine
   ring stays `ubiqx-shield`, the source ring stays `nofap` (design 0.1).
2. **This sandbox compiles both shells.** webkit2gtk-4.1 and GTK are installed, and the Rust
   targets `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`. `cargo check --workspace
   --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` cover both shells
   here. The CI Linux job still excludes them (it has no webkit).
3. **The code is the contract.** Types are in `crates/ubiqx-core/src/nofap.rs` and `nofap_ipc.rs`,
   signatures in the seeded files. A change to a public item another unit uses goes through the
   integrator: stop and report it.
4. **Snapshot commits.** An automatic hook may commit and push the tracked changes of the working
   tree under a `wip(nofap): snapshot ...` subject while you work. That is expected. Carry on,
   and make your own commits for whatever is still uncommitted.
5. **CI.** `publish-continuous` runs only on `main`. Every push cancels the previous CI run of the
   branch, so push at sound checkpoints.
6. **Line endings.** `.gitattributes` keeps the fixtures, the embedded data files, the notices and
   the blocklist review file LF on every checkout. A new file that a test compares or embeds byte
   for byte needs a line there: report it.

## After wave A (foundation, blocklist data)

7. **`bing.com` is treated like `google.com`.** It comes off `never-block.txt`; Microsoft's
   sign-in, update, store and API hosts stay on it. Bing is then an ordinary popular site: the
   vision check can look at it when a local signal fires, and a user may block it. The same for
   `grok.com`; `api.x.ai` and `x.ai` stay protected (the app's own AI provider).
8. **Private messaging on the web is never captured by default.** `web.telegram.org` and
   `discord.com` (and any other one-to-one messaging web app on `mixed.txt`) move from `mixed.txt`
   to `never-capture.txt`, next to `web.whatsapp.com` and `messenger.com`. The design keeps
   messaging apps off unless the user opts in; the web versions follow the stricter side. Public
   platforms (Reddit, X, Tumblr, Imgur and the like) stay mixed. A per-host opt-in is deferred.
9. **`grindr.com` stays off the list** (the reviewer's call: an orientation is not a risk marker).
10. **An unreadable stored configuration is an error, never "default".** The default is "module
    off", so a parse failure must not silently switch protection off: `NofapRepo::load_config`
    returns an error for a row that does not parse (U3), and the service surfaces it as a degraded
    state and keeps the last good configuration in memory when it has one (U5).
11. **The canary answer is carried.** `plan::assess` is pure per observation; the verify pass
    keeps its last canary and probe answers and puts them into observations that did not look
    (U2 documents, U5 does it).
12. **Heuristics must not block ordinary names.** The word lists of design 3.5 have known innocent
    hits (hand-off U0 and U1a list them: `putas` in "disputas", `porn` in "Pornichet", `xxx` in
    roman numerals and sizes, `erome` in "aeromexico", `cam4` folding to "cama", and more). U1
    chooses matching rules that pass both the block rows and these innocent rows (label or token
    boundaries, digit stems matched unfolded, a repeated-letter collapse for look-alikes), and
    tests every name those hand-offs mention. When a rule cannot tell, it does not block: the
    vision check is the layer for doubt.
13. **The frontend type-checks the fixtures through `fixtures/golden.ts`** (`satisfies` on a JSON
    import cannot work; hand-off U0).
14. **"Verificar agora" needs no command.** The service does the full read when `view` is called.

## During wave B

15. **A frontend test fails in CI and under load** (U6 and its reviewer). On commit `40a93d0` the
    job "Frontend (Vite + TypeScript)" failed in `src/nofap/NofapCard.test.tsx`, "the card window
    closes by itself only while untouched > not once the user opened support": the card was closed
    after the 60 s advance. It passes when the file runs alone and fails when the whole suite runs
    (`pnpm test -- --run`, reproduced here twice). A trace showed `close` called from a timer
    (`callTimer` of the fake clock), so a close timer is still armed, or armed again, after support
    opens. Find the cause in the component or in the test's use of `shouldAdvanceTime`, fix it,
    and prove it with the whole suite run several times, not with the single file. The integrator
    added a guard so that a late answer to the first `card` request cannot reset an open flow
    (`shown` in `NofapCard.tsx`); it is a real fix but not the cause of this failure.

    **Cause found and fixed by the integrator** (`NofapCard.tsx`): React runs the cleanup of the
    auto-close effect in a later task than the render that shows the opened flow, and that task is a
    real `setImmediate` the fake clock does not control. Under load the test advanced 60 s before
    the cleanup had run, so the old timer fired. The same gap exists in the product for a click that
    lands just before the timer is due. The timer now asks when it fires whether the card is still
    untouched (`touched`, `modeNow`). The whole suite passed three times in a row here afterwards.
    U6's reviewer: keep this behaviour, and check the other timers of the module for the same gap.

16. **A container restart interrupted two reviews** (U4 and U9a) on 2026-11-20. The reviewers
    start again from the beginning, but their earlier work is in the repository: for U4 the
    commits `a127af1`, `412e146`, `088e403` and the review notes in `U4.md` if any; for U9a the
    snapshot commits `18512ac` and `865faaf` (release scripts) and the snapshot of
    `scripts/nofap/system-smoke.sh` committed with this note. Read those diffs first, keep what
    is right, finish what was left half-way, and do not repeat checks whose result a commit
    message already records unless you doubt it. Two CI failures found meanwhile were fixed by
    the integrator: the native-shield test expected Linux on every runner (`08a39ac`), and the
    name lookup answered the machine's own addresses for an empty name on Windows (`ce86d96`).
    Tests of the platform crate run on the macOS and Windows runners too: nothing in them may
    assume Linux (paths, `/bin/sh`, the current OS) without a `cfg` or a check of `Os::current()`.

## After wave B (matcher, decision logic, platform, frontend, storage and AI, scripts)

17. **"Apagar meu histórico" also clears what the person wrote** (design 7.4 wins over the port's
    comment): the counters, the snoozes, and the values sentence and the plans in the
    configuration. Protection settings stay. U5 does the clearing in the service; the dialog text
    `data.erase_body` says so in both languages (follow-up unit FU).
18. **Checks on a standing signal are spread over the hour.** With a 45 s cadence and six checks
    per key per hour, a host with a standing signal was checked six times in four minutes and then
    not for 56. Same cap, spaced: after a check on a key, the next one on that key waits twice as
    long as the previous gap (45 s, 90 s, 3 min, 6 min, 12 min, 24 min), and the gap resets when
    the hour's window rolls. Cost per hour is unchanged (FU, in `sampling.rs`; the signature of
    `decide` stays).
19. **The delay of a new privacy-list entry counts observed running time**, like the commitment
    delay, so a clock set forward does not make it effective at once. FU adds the field to
    `ExclusionSeen` in `ubiqx-core` (serde default, fixtures blessed) and the rule to
    `exclusions.rs`; U5 feeds it from the same tick that feeds the commitment delay.
20. **Deleted rows are overwritten in the database file**: `PRAGMA secure_delete = ON` for every
    connection (FU, `crates/ubiqx-storage/src/db.rs`), so a scrubbed block, an erased history and
    "Apagar todos os dados" leave no readable text in freed pages. It applies to both products.
21. **`view` is cheap.** The frontend calls it often. The service answers from memory; the full
    system read behind decision 14 runs at most once every 60 s, and never blocks the answer
    (the view carries the last observation and a fresh one arrives as an event).
22. **An identical pending request is not created twice.** The service returns the existing
    pending change when the same change is requested again (U5); the page disables a switch whose
    change is waiting (FU).
23. **A snoozed host is sampled at signal cadence** (`Allowed(AllowSource::Snooze)`; hand-off U1b),
    by U5.
24. **Copy with the managed provider.** The consent sentence must read correctly when the provider
    is "IA do Ubi" (no "as regras dele" pointing at nothing): FU adjusts the sentence or adds the
    variant, in both languages, and keeps the forbidden-word and parity tests green.
25. **The standalone app passes its own data directory to `PlatformServices::native_for`** (U8), or
    its secrets file would fall into ubiqX's directory. Both shells hold `hyper_util` and `reqwest`
    at `warn` in the logger (U7, U8).
26. **Shell scripts stay LF** (`.gitattributes`, done by the integrator).
27. **Not decided here, for the owner:** the help resources are unverified (the release-mode test
    fails until a person fills `verified_on` and `verified_by`); nobody has read the top-10k review
    list or `extra-ptbr.txt`; providers' and Cloudflare's terms are unread.

## During wave C

28. **A real native shield now applies for real.** With `hosts` and `resolver` open on the three
    systems, `NativeSystemShield::native` (and `PlatformServices::native_*`) carries the real
    elevator, and this sandbox runs as root, so `pkexec` needs no prompt: a test or a manual run
    that hands such a shield a hosts or resolver plan writes `/etc/hosts` and the resolver
    drop-in of the machine it runs on (hand-off SMOKE, review). Tests of the composition and of
    the shells (U7, U8) go through the scripted platform or the fakes. A manual start of an app
    in this sandbox must not enable the module or apply a tier; if it ever happens, compare
    `/etc/hosts` with a copy taken before and restore it.
29. **The status race of the engine** (`bc5e225`, integrator): a pass that read "enabled" just
    before the module was switched off could store "protected" over the "off". One status at a
    time is now computed, stored and announced (`status_gate`), and the second lock re-checks the
    switch. The reviewer of U5 checks this fix like the rest of the unit.
30. **The workflow agents' commits carry the attribution of the model now serving**:
    `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and the same `Claude-Session` line.
