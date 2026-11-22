# Hand-off: FU2a Copy for two products, and the unreadable-configuration issue

Written by the engineer. Where this file and the code differ, the code wins.

Status: **done**. Integrator decisions 32 (section 1) and 33 (section 2). Commands and their
results are in section 3, open points in section 4.

## 1. Decision 32: copy that is true in both products

### How the strings were found

Every string of the namespaces `nofap` and `nofap_app` (both languages, and `PANIC_CRISIS`) and
every string of `ubiqx_nofap::messages` was searched for ubiqX by name, its tracker, its pause,
its private mode and its local mode. `ubiqx-shield BEGIN … END` in `helptab.remove.body` is the
marker written into the hosts file by both products, so it stays. "Janelas anônimas" /
"Private-browsing windows" are the browser's private windows, not ubiqX's private mode. "Em
pausa", "fica em pausa" and "as pausas de regra" are the AI check's own pause and the snoozes,
true in both products.

### What changed

| Key | Old | New | Kind |
|---|---|---|---|
| `dns.disclosure` (5.6) | … sites que ela considera perigosos. **O ubiqX não recebe nada.** Se preferir … / … sites it considers dangerous. **ubiqX receives nothing.** If you prefer … | … **Este app não recebe nada.** … / … **This app receives nothing.** … | neutral, both products |
| `vision.consent` (5.6) | … Janelas anônimas são verificadas. **Pausar o ubiqX ou usar o modo privado não pausa esta verificação.** / … Private-browsing windows are checked. **Pausing ubiqX or using private mode does not pause this check.** | ends at "Janelas anônimas são verificadas." / "Private-browsing windows are checked." | neutral, both products |
| `vision.consent_ubi` (interface variant of FU) | the same last sentence | the same cut | neutral (shown with the managed provider only) |
| `vision.pause_note` (new, 5.6, "ubiqX only") | | Pausar o ubiqX ou usar o modo privado não pausa esta verificação. / Pausing ubiqX or using private mode does not pause this check. | added after the consent sentence when `view.product` is `ubiqx` |
| `helptab.ai.body` (interface) | O modo só local e a remoção da chave de IA param a verificação na hora, sem espera. Ficam nas configurações do app. / Local-only mode and removing the AI key stop the check at once, with no wait. They are in the app's settings. | A verificação para na hora, sem espera, quando você remove a chave de IA nas configurações do app. / The check stops at once, with no wait, when you remove the AI key in the app's settings. | neutral, both products |
| `helptab.ai.local_only` (new, interface) | | O modo só local, também nas configurações, faz o mesmo. / Local-only mode, also in the settings, does the same. | added after `helptab.ai.body` (Help tab and the "Parar a verificação por IA" exit) when `view.product` is `ubiqx` |
| `nofap_app.settings.ai_hint` (standalone only) | Remover a chave para a verificação na hora, sem tempo de espera. | A verificação para na hora, sem tempo de espera, quando você remove a chave. | Portuguese only: "para" read as "for" as well as "stops" (the English was clear and is unchanged) |

In ubiqX the consent sentence reads exactly as before the split (the copy test checks it).

### ubiqX-only strings, and what keeps each out of the standalone app

`UBIQX_ONLY` in `i18n/messages/nofap.ts` lists them; `copy.test.ts` fails for any other string of
either namespace, in either language, that names ubiqX, its pause, its private mode or its local
mode, and for a listed key that no longer does.

| Key | Kept out by |
|---|---|
| `vision.pause_note`, `helptab.ai.local_only` | `product`: `labels.ts::withUbiqxLine` adds them only when `view.product` is `ubiqx` (`consentText`, `aiExitText`) |
| `vision.managed`, `vision.managed_tick` | `managed`: shown only with the managed provider "IA do Ubi", which the standalone app refuses (backend and `hostMock`) |
| `vision.off.local_only` | `local_only`: `VisionOff::LocalOnly` comes only from ubiqX's settings (`StandaloneEnv` has no local-only mode) |
| `exclusion.delay` (does not name ubiqX) | shown only by ubiqX's Settings page (the tracker's privacy lists) |
| Rust `messages::TrayLabels::not_paused` ("Não pausa o {name}") | only ubiqX's tray (`nofap_tray.rs`) uses it; a test in `messages.rs` fails if the standalone shell's sources name it |

### The mock and the two products

`nofap/mock.ts` has `__nofapMock.setProduct(product)`; `view.product` was always `ubiqx`. The
standalone's `hostMock.ts` sets `shield` whenever it syncs, so the standalone preview, its tests
and the screenshots show the standalone's copy. `testing.tsx::resetNofap(scenario, product =
'ubiqx')` sets it for the module's tests.

### Tests that pin it

- `copy.test.ts`: the copy bank has 69 rows (was 68); "copy for two products (integrator
  decision 32)": no shared string names ubiqX; the `UBIQX_ONLY` list is exact and each entry does
  name ubiqX; the old sentences are gone in both languages; `consentText` and `aiExitText` add the
  line in ubiqX and nothing in the standalone, for every provider, in both languages.
- `NofapWizard.test.tsx`, "the wizard in the two products": the DNS disclosure in both products;
  the pause line in ubiqX only; the whole standalone wizard in English names nothing of ubiqX.
- `NofapPage.test.tsx`, "the two products": the Help tab and the AI exit dialog in both products;
  the consent dialog of the page in both products and both languages. The existing test of the
  AI exit has the new text.
- `messages.rs`: `the_label_about_ubiqx_pause_is_shown_by_ubiqx_alone`.
- Mutation: with `withUbiqxLine` always adding the line, six of these tests fail.

## 2. Decision 33: an unreadable stored configuration

### Core

`ubiqx_core::nofap::Issue::ConfigUnreadable`, serde name `config_unreadable`, last in
`Issue::ALL` (now 14) and in the exhaustive `assert_all_listed!`; a test of its wire name. The
fixtures were regenerated with `UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden` (the
enum list and a fourteenth status of `statuses()`, which carries the new issue); `types.ts` has
the member, and `contract.ts` checks it as it checks every other.

### Engine (`crates/ubiqx-engine/src/nofap/system.rs`)

- `compute_status` while the configuration cannot be read: `overall: reduced` as before, now with
  `issues: [config_unreadable]` (tiers, guard and vision as in `NofapStatus::off()`).
- **A read that works ends it.** `read_config_again` runs at the start of every verify pass while
  the error stands: it reads the row again (only `load_config`; nothing else of the repository is
  asked), and a read that works takes the stored configuration (sanitized), clears the flag,
  reloads the sites and the matcher (the learned sites are salted with the configuration),
  forgets the per-window memory and refreshes the status (an event without the issue). The pass
  then goes on as for any module, so a stored "on" starts enforcing at once. A read that fails
  changes nothing and writes nothing.
- A reset (`replace_config`) and the wizard (`enable`) already cleared the flag; the issue goes
  with it.
- Tests (`crates/ubiqx-engine/tests/nofap.rs`):
  - `d33_an_unreadable_configuration_is_its_own_issue_until_a_reset_replaces_it`: status and view
    say `reduced` with the issue only; guard and verify passes keep it; after `reset` the status
    is `off`, the last status event is `off`, the stored row is the default.
  - `d33_a_read_that_works_later_replaces_the_defaults_and_ends_the_issue`: a module stored on,
    then unreadable: one `load_config` per verify pass, nothing saved; once the row reads, the
    next verify pass takes it, the issue goes (status and event), the guard blocks a listed site,
    later passes do not read it again, and writes are accepted.
  - `d33_the_wizard_also_ends_the_issue`.
  - Mutation: without the call to `read_config_again` the second test fails.
- **One existing assertion changed**: `with_the_module_off_no_port_is_called` asserted that an
  unreadable configuration makes the two verify passes ask the database nothing. Each now reads
  the configuration once (and only that: the `Flaky` repository counts `load_config` calls
  separately and the test checks both counts). With a readable configuration, or a module never
  enabled, nothing is read, as before.

### Page

- `NofapPage.tsx`: while `status.issues` holds `config_unreadable`, the page shows its header (with
  "Falar com alguém"), a card with the overall state ("Proteção parcial") and the issue as one
  sentence with its one action, and nothing else: not the wizard of a module nobody enabled, not
  the page of a module that is on, since every write would be refused.
- `issue.config_unreadable`: "Não consegui ler as configurações salvas: o que já estava aplicado
  no sistema continua valendo, e o bloqueio no app e a verificação por IA ficam parados até você
  restaurar os padrões." / "I couldn't read the saved settings: what was already applied to the
  system still holds, and the in-app block and the AI check stay off until you restore the
  defaults."
- The action `restore_defaults` ("Restaurar padrões" / "Restore defaults", `IssueList`) opens a
  dialog before the reset, because `reset` also erases the lists, the sentence, the plans and the
  counts: `config.reset_body` says so and says that what is applied to the system stays. The
  button is a danger button like "Apagar tudo"; "Cancelar" comes first and takes the keyboard.
  Confirmed: `reset`, the toast `config.restored`, then the wizard.
- `store.ts`: a `status` event now also reloads the view when the issue appears or goes with the
  same `overall` (a stored module that is on with a tier missing is `reduced` before and after;
  without this the page would have kept the defaults and shown the wizard).
- Mock: scenario `unreadable` (`?nofap=unreadable`): the status above, the defaults, and the
  engine's refusals (`request`, `consent_vision`, `confirm_pending`, `cancel_pending`,
  `system_apply`, `restore_network`, `export`); `reset` and `enable` clear it.
- Tests (`NofapPage.test.tsx`, "the stored settings could not be read"): the sentence, the single
  button, no wizard, no status card, the talk button, no write (PT-BR); the sentence, the button
  and the dialog in English; the dialog's text, nothing sent before the confirmation, then exactly
  one `reset` and the wizard; cancel changes nothing; the view is read again on a status event
  with the same `overall` (fails without the `store.ts` change, checked); the mock's refusals.

## 3. Commands run on the final state

| Command | Result |
|---|---|
| `UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden` | passed; `golden.ts` and `views.json` regenerated (+27 lines each), not edited by hand |
| `cargo test -p ubiqx-core nofap` | 34 passed |
| `cargo test -p ubiqx-nofap` | 302 + 22 + 12 + 0 passed |
| `cargo test -p ubiqx-engine` | 20 + 12 + 77 + 2 + 28 + 6 + 0 passed |
| `cargo clippy -p ubiqx-core -p ubiqx-engine -p ubiqx-nofap --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace --all-targets` | passes |
| `pnpm typecheck` (apps/desktop) | clean |
| `pnpm test -- --run` (apps/desktop), whole suite | 35 files, 478 tests passed: once before the last change of the page text, twice on the final state |
| `pnpm build`, `pnpm build:nofap` | both build |
| Playwright screenshots, mock mode (`vite` on 1420 and `vite --mode nofap` on 1430), not in git | wizard steps 2 and 3, the Help tab and its AI section, the AI exit dialog, the unreadable state and its dialog; ubiqX and the standalone app, PT-BR and EN, 1180 x 820. Read one by one. The script also checked every page's text: no unresolved key or placeholder, no page or console error, and no "ubiqX", "modo só local", "local-only", "modo privado" or "private mode" on any standalone screen. Fixed after looking: the Portuguese AI exit read two ways, and the reset button of the dialog was primary |

Not run: the Tauri shells' own tests, anything on macOS or Windows, and no CI run was read.

## 4. Open points

- **Design 2.3 still quotes the old disclosure** ("O ubiqX não recebe nada", line 415 of
  `DESIGN.md`, a quotation in the resolver section, outside section 5.6 which this unit owns). The
  table of 5.6 is the text that ships; 2.3 should follow it.
- **The engine's error text** for a write while the configuration cannot be read
  (`unreadable_config()` in `crates/ubiqx-engine/src/nofap/mod.rs`) says "run the set-up again".
  The page no longer offers a write in that state, so the text is not shown; if it ever is,
  "restore the defaults" is the action the page names. `mod.rs` was left alone (another unit's
  decision 37 changes it at the same time).
- **"Keeps the last settings it knew"** (decision 33's wording, from decision 10): the service
  never holds a good configuration when this state begins, because the row is read only at the
  start. What holds is what is applied to the system (the hosts block, the resolver, the policies
  stay on the machine), and that is what the sentence says. The in-app block and the AI check do
  not run in this state, as before this unit.
- **"Restaurar padrões" erases the lists and the history** (it is `reset`, "Apagar tudo"). A
  reset that keeps the lists would need a new service command; the dialog says what goes.
- **The wizard is also a way out** (the engine allows `enable` over an unreadable row, and keeps
  the lists); the page does not offer it in this state, to keep one action per screen.
- `vision.managed` and `vision.managed_tick` name ubiqX's server and are kept out of the
  standalone app by its refusal of the managed provider, not by a check of `view.product`
  (a check there would leave the second tick required and hidden).

## Review

Independent review of this unit, after commit 772ef22. Fixes are in commit 0996501.

### Reproduced

| Command | Result |
|---|---|
| `cargo test -p ubiqx-core nofap` | 34 passed |
| `cargo test -p ubiqx-nofap` | 302 + 22 + 12 + 0 passed |
| `cargo test -p ubiqx-engine` | 20 + 12 + 77 + 2 + 28 + 6 + 0 passed before the review; the nofap file has 78 with the review test |
| `UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden` | passed, and `git status` shows no change to the fixtures afterwards: the committed fixtures are what the bless writes. Their diff in 84d8f59 is only `config_unreadable` in `enums.issue` and the fourteenth status of `statuses` |
| `cargo clippy -p ubiqx-core -p ubiqx-nofap -p ubiqx-engine --all-targets -- -D warnings`, `cargo fmt --all -- --check` | clean |
| `pnpm typecheck`, `pnpm test -- --run` (apps/desktop) | clean; 35 files, 478 tests before the review, 480 after |
| `pnpm build`, `pnpm build:nofap` | both build |
| `cargo test -p ubiqx-app nofap` | passed |

Not run: the Tauri shells' own tests, `cargo check --workspace`, anything on macOS or Windows, Playwright screenshots (the copy was read in the source and checked through the page tests), CI.

### Bundle and i18n grep

`dist` and `dist-nofap` each hold every i18n namespace (`i18n/index.ts` imports all of them),
so both contain "ubiqX" (63 and 50 times), most of them in ubiqX's own namespaces (licence,
tracking, updates, onboarding, the engine error of `AppShell`) that no component of the
standalone frame imports. In the module's namespaces the remaining hits are exactly
`UBIQX_ONLY`: `vision.pause_note` and `helptab.ai.local_only` (gated on `view.product` in
`labels.ts::withUbiqxLine`), `vision.managed` and `vision.managed_tick` (managed provider
only; the standalone refuses it), `vision.off.local_only` (`AiReadiness::LocalOnly` is set
only by ubiqX's `engine/src/nofap/env.rs`; `ubiqx-app/src/nofap_app.rs` never produces it).
"Pausar"/"pause" elsewhere in the namespaces is the AI check's own pause or the rule snoozes.
"UBI" (`block.discreet`, `card.window_label`) is the mascot's discreet name chosen by design
5.6 and 0.1 for both products, not ubiqX's name; left as it is.

### What was wrong, and what changed

- **`issue.config_unreadable` spoke in the first person** ("Não consegui ler…", "I couldn't
  read…"), while every other issue speaks of the app and the system. It also said the in-app
  block and the AI check stay off "até você restaurar os padrões", which a reset does not do
  (it leaves the module off; protection returns after the set-up), and it left out the other
  way back that this unit built (a verify pass that reads the row again). New text:
  "As configurações salvas não puderam ser lidas. O que está aplicado no sistema continua
  valendo; o bloqueio no app e a verificação por IA ficam parados até elas voltarem a ser lidas
  ou até você restaurar os padrões e configurar de novo." / "The saved settings could not be
  read. What is applied to the system still holds; the in-app block and the AI check stay
  stopped until they can be read again or until you restore the defaults and go through the
  set-up again." (The first clause follows `quit.confirm_system` of 5.6.)
- Tests added: `copy.test.ts` "says every issue in the voice of the others" (fails on the old
  Portuguese sentence, checked) and a pin of the two ways back; `crates/ubiqx-engine/tests/nofap.rs`
  `d33_review_only_the_verify_pass_reads_again_and_the_person_s_own_settings_come_back`: guard
  passes, `view` and `status` never read the row, each verify pass reads it once, a pass that
  still fails emits no status event, and a read that works brings back the person's own
  sentence and switched-on state, with the issue gone from the view.

### Still open (outside this unit's files, or for the integrator)

- **After "Restaurar padrões" the system tiers have no in-app way out.** The reset leaves what
  is applied on the machine (the dialog says so), but the switched-off page offers "Remover do
  sistema" only when `config.self_use_at` is set, and a fresh configuration has none; the
  service also does not read the system for a module never enabled (`read_for_view`), so the
  page cannot know tiers were left. A person who restores the defaults to leave has the manual
  steps of the Help text (not reachable from the switched-off page), the uninstaller, or the
  wizard and then a waited "off". Either the reset keeps `enabled_at`/`self_use_at`, or the
  unreadable page also offers `system_remove` (which the service already allows in that
  state). Needs an engine or design decision (`service.rs` / `system.rs`, not this unit's).
- `unreadable_config()` in `crates/ubiqx-engine/src/nofap/mod.rs` still says "run the set-up
  again"; not shown by the page in this state today (open point of the engineer, confirmed).
- `DESIGN.md` line 415 (section 2.3) still quotes "O ubiqX não recebe nada".
- `helptab.ai.body` says the check stops when the AI key is removed; with ubiqX's managed
  provider there is no key, and only the ubiqX-only line (local-only mode) gives the true way.
  True as shown, but the first sentence alone would not be for that provider.
- Every i18n namespace ships in the standalone bundle; nothing of ubiqX's namespaces is shown
  there today, but a split per product (`i18n/index.ts`, not this unit's) would make that
  structural rather than a matter of which components are imported.
