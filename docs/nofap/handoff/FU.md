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
