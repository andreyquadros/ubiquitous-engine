# FU: integrator follow-ups

Status: **in progress**. Section 1 (decision 19) is done and pushed; the other sections are
filled in as each change lands.

## 1. Decision 19: the delay of a new privacy-list entry counts observed running time

### What exists

- `ubiqx_core::nofap::ExclusionSeen` has a third field:

  ```rust
  pub struct ExclusionSeen {
      pub key: String,
      pub first_seen_at: DateTime<Utc>,
      #[serde(default)]
      pub observed_secs: u32,
  }
  ```

  `observed_secs` is the running time, in seconds, the app was observed for since the entry was
  first seen. It grows by at most 150 s per verify pass and stops at 900 (the constants of
  `commitment`: `OBSERVE_STEP_MAX_SECS`, `OBSERVED_CAP_SECS`). Data stored before the field
  existed reads as `0` (test `an_exclusion_stored_before_the_observed_time_existed_still_reads`
  in `ubiqx-core`).
- `ubiqx_nofap::exclusions`:
  - `reconcile(lists, seen, now)` and `effective(lists, seen, enabled_at, delay_secs, now)` keep
    their signatures. `reconcile` gives a new entry `observed_secs: 0` and keeps the value of a
    remembered one.
  - **The rule** (`effective`): an entry is effective when it was first seen at or before
    `enabled_at` (unchanged), or when the clock is `delay_secs` past `first_seen_at` **and**
    `observed_secs >= min(delay_secs, 900)`. It is the rule of `commitment::is_ready`.
  - New: `observe(seen: &mut [ExclusionSeen], elapsed_secs: u32) -> bool`. One verify pass: adds
    `min(elapsed_secs, 150)` to every entry, up to 900. Returns whether anything changed.
  - New: `reconcile_since(lists, seen, enabled_at: Option<DateTime<Utc>>, now)`. `reconcile`
    with one more guard, see "Contract notes".

### Contract notes for U5 (engine service)

- In the verify pass, with the same `elapsed_secs` you give `commitment::tick`:

  ```rust
  let changed = exclusions::observe(&mut config.internal.exclusions_seen, elapsed_secs);
  ```

  Store the configuration when it returns `true`. It returns `false` once every entry is at 900,
  so a standing list costs at most a handful of writes per entry, not one per pass for ever.
  Order: `reconcile` first, then `observe` (a brand-new entry is then credited the pass in which
  it was first seen, at most 150 s; calling them the other way round only delays that credit by
  one pass). Both orders are safe.
- Upgrade: entries stored by an earlier build have `observed_secs == 0`. Those first seen at or
  before `enabled_at` stay effective at once. One that was added later and had already waited its
  delay by the clock waits again until `min(delay, 900)` s of running time were observed. That
  is the stricter side and ends by itself.
- **A clock set back before the enabling instant** (found while writing the tests; it is the
  mirror of the hole decision 19 closes). `effective` treats `first_seen_at <= enabled_at` as
  "was there when the module was enabled". If the clock reads earlier than `enabled_at` when a
  new entry is first reconciled, plain `reconcile` records it with that early time and the entry
  is effective at once, and stays so after the clock is put right. `reconcile_since` closes it:
  while `now < enabled_at` it remembers nothing new (remembered entries stay, removed ones are
  dropped), so the new entry is not effective, and its wait starts at the first pass with a
  clock at or past `enabled_at`. **Please call
  `exclusions::reconcile_since(&lists, &seen, config.enabled_at, now)` in the verify pass**
  instead of `reconcile`. At the enabling instant itself `now == enabled_at`, which is not
  "behind": both functions do the same there. With `enabled_at == None` they are the same
  function.
- A sighting between `enabled_at` and the real time (clock set back by less) is covered by the
  running-time rule: after the clock is put right the clock condition is met at once, the 15
  minutes of observed running time are not.

### Fixtures

`UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden` was run: the three fixture files did
not change. `ExclusionSeen` lives in `Internal`, which is never sent to the page, so no IPC value
and no TypeScript type carries it.

## 2. Decision 18: checks on a standing signal are spread over the hour

### What exists (`crates/ubiqx-nofap/src/sampling.rs`)

- `decide`, `record`, `SampleCtx`, `SampleEvent` and `Decision` are unchanged in shape.
- `Cadence` has one more field, `doubles: bool`; it is `true` for `CADENCE_SIGNAL` only. Nothing
  outside `sampling.rs` builds a `Cadence` (checked with a search of the workspace).
- `pub fn spread_gap(every_secs, checks) -> i64`: the wait after the `checks`-th check of a key
  in its hour: `every_secs` after the first, doubled after each later one, never more than
  3,600 s. With 45 s: 45, 90, 180, 360, 720, 1,440.
- Each key remembers "its hour": the instant of the first frame **sent** for it, and how many
  were sent since. The hour is over 3,600 s after it began (or when it seems to begin in the
  future: the clock was set back); the next frame sent begins a new one, and the wait is 45 s
  again.
- The cap is the old one and is still a sliding hour: `PER_KEY_PER_HOUR` (6) frames of a key in
  any 3,600 s. A new hour of the key resets the gap, never the cap.

### The schedule (tested second by second with the fake clock)

- Standing signal, key in front all the time: frames at **5, 50, 140, 320, 680, 1400** s, then
  nothing until **3605**, and the same six offsets again in every following hour (test
  `the_spread_starts_again_when_the_hour_of_the_key_rolls`, three hours).
- After the sixth check the wait would be 24 minutes (`Skip::NotDue` until 2840); from then on
  the answer is `Skip::KeyCap` until the first check is an hour old (test
  `six_checks_per_key_per_hour`).
- A look that sent nothing (`Unchanged`) and a failed call restart the wait and do not lengthen
  it: only a frame that was sent doubles the gap.

### Deviation from the design, and one consequence to know about

- Design 4.3, row "Signal": "5 s, then every 45 s" becomes "5 s, then 45 s, 90 s, 3 min, 6 min,
  12 min apart" (decision 18).
- **The count is of all frames sent for the key in its hour, whatever the cadence was when they
  were sent.** `record` is not told whether a check was made on a signal (the event carries no
  such flag, and its shape was to stay), so a key that was already checked at the plain cadence
  starts the signal cadence further along: with two earlier checks the first look on a new
  signal comes 90 s after the last look, with three 3 minutes, with four 6 minutes, with five 12
  minutes. Before this change it came 45 s after the last look, and was followed by the rest of
  the six within minutes. Three assertions of the existing test `a_signal_cancels_relaxing` moved
  accordingly (150 → 195, 750 → 885, 150 → 195). In the worst case (five checks in the hour,
  relaxed plain cadence of 600 s) a new signal makes the next look come 2 minutes later than no
  signal would have. If the integrator wants the first look after a new signal to stay at 45 s,
  it needs a `signal: bool` on `SampleEvent::Checked` (an engine-facing change), so that only
  checks made on a signal are counted.

## 3. `plan::reset_targets` leaves a switched-off interface alone

- `reset_targets(observation, resolver)` (same signature) now returns only interfaces that are
  `active` (enabled on macOS, up on Windows) **and** whose servers are exactly the provider's
  set. It used to return every interface with exactly that set, active or not (hand-off U2 says
  so in its contract notes; that sentence is out of date).
- `plan::preview` uses the same function for the line `RESOLVER_RESET`, so no reset is promised
  when the only interface with the provider's servers is switched off.
- Consequence to know: a service we set while it was enabled, which the user then switched off,
  keeps the provider's servers after "remove". It is reset by a later "remove" run while it is
  enabled. The Windows back-end only ever lists adapters that are up, so nothing changes there.
- Tests: `reset_targets_leave_an_interface_the_user_changed` (the switched-off rows are left
  out; a reordered set on an active interface is still ours) and
  `a_switched_off_interface_is_neither_reset_nor_promised_a_reset`. The platform crate's tests
  (169) pass unchanged.
