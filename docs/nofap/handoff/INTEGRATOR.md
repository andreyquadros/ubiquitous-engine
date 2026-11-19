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
