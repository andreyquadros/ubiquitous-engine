# Code map: screenshots and vision AI (input for the nofap module)

Read-only mapping of the code as it stands on branch `claude/sharp-bardeen-sk27ae` (HEAD `410c3d6`).
Every statement below was read from the source; `file:line` references are to that tree. Nothing was
compiled or run for this report. Where a statement is an estimate or comes from outside the repository
it is marked as such.

Contents

1. The constraints that matter most
2. Layer overview
3. How a screenshot is taken
4. When a screenshot reaches the AI (the existing vision path)
5. The vision request: prompt, schema, parsing
6. How the request reaches each provider
7. The "IA do Ubi" proxy (`services/ubi-api`)
8. Failure modes: offline, no AI, refusals, budget, health
9. Cost per call
10. Privacy controls in force today
11. The "Dados enviados à IA" audit trail
12. What is persisted and for how long
13. Fakes and test doubles
14. What a single-purpose "explicit content?" call needs in each layer
15. Surprising, fragile or missing (including docs that the code does not match)
16. File index

---

## 1. The constraints that matter most

1. **The existing vision path is post-hoc and slow by design.** A screenshot is only sent after the
   block has *closed*, the local chain failed, the text model answered "not sure" or `needs_vision`,
   and the batching threshold was met (8 pending blocks or the oldest waiting 240 s; worker tick 30 s).
   It can never drive a real-time intervention. nofap needs its own capture-and-ask loop on the
   foreground window; it can reuse the capturer, the `LlmClient` stack, the gates and the fakes, but not
   `classify::run_once`.
2. **Incognito / private windows are deliberately never captured.** A window title containing an
   incognito marker turns the sample into a `[privado]` block with `window_id: None`, and
   `should_capture` refuses `sys-private` blocks. Private mode, blocked apps, `vision_policy = Never`,
   `vision_denied_apps`, `local_only`, pausing tracking and "a blocked app is visible anywhere on
   screen" all stop capture too. For nofap each of these is an escape hatch the design has to decide
   about explicitly.
3. **A capture is already in-memory.** `ScreenCapturer::capture(target, max_edge)` returns an
   `EncodedImage` (JPEG bytes); writing to disk is done one layer up in
   `screenshots::capture_for_block`. A nofap check can capture, send and drop the bytes without a file
   or a `screenshots` row ever existing.
4. **One abstraction covers all four providers.** A new call is one `LlmRequest` with
   `LlmContent::Image` + `LlmContent::Text`, sent through the same `Arc<dyn LlmClient>`
   (`RoutingLlmClient`). No provider code has to change for the wire format. The Ubi proxy forwards
   Anthropic-shaped bodies verbatim (only `model` is rewritten), so images already pass through it.
5. **A model refusal is an error today, and usage is recorded before it is returned.**
   `stop_reason = refusal` (Anthropic) and `message.refusal` / `finish_reason = content_filter`
   (OpenAI, xAI) become `Err(CoreError::AiRefused)`; the answer text is discarded. A detector for
   explicit content must treat `AiRefused` as an outcome of the check, not as a failure.
6. **Cost and budget are shared with the productivity pipeline.** Every call lands in the same
   `ai_usage` ledger and the same monthly budget (`ai_monthly_budget_usd`, default US$ 5; the managed
   plan's proxy budget defaults to US$ 6). At the default image size one vision call is roughly
   US$ 0.002 on the default Anthropic model (estimate, section 9); a naive one-check-per-minute loop
   exhausts either budget in days.
7. **AI health is shared too.** `AiHealth::Paused` / `Degraded` set by a classification failure stops
   every remote call, and a nofap failure routed through `handle_remote_failure` would pause
   productivity classification. No AI, offline, budget spent and "onboarding not done" all mean: no
   vision check. The DNS layer has to carry the product in those states.
8. **Retries are per client, not per request.** A request can shorten its `timeout`, but the 4
   attempts with 2 s → 30 s backoff come from the client config. A real-time check has to bound total
   time itself (for example with `tokio::time::timeout` around `complete`).

---

## 2. Layer overview

```
ubiqx-platform                      ubiqx-engine                          ubiqx-ai                     network
---------------                     ------------                          --------                     -------
ActivitySource::foreground()  -->   sampler (OS thread, every 5 s)
  ForegroundWindow{window_id}        ActivitySample{window_id}
                                    tracker::handle_sample
                                      Segmenter::feed -> open block
                                      screenshots::should_capture
ScreenCapturer::capture       <--     screenshots::capture_for_block
  -> EncodedImage (JPEG q60)          -> <data_dir>/screenshots.noindex/YYYY-MM-DD/<id>.jpg
                                      -> screenshots row (sent_to_ai = 0)
                                    classify::run_once (every 30 s)
                                      1 local chain  2 text LLM (batch <= 25)
                                      3 vision, block by block  ------->  LlmVisionClassifier
                                                                            build_request -> LlmRequest
                                                                          RoutingLlmClient
                                                                            AnthropicClient ----------> api.anthropic.com/v1/messages
                                                                            AnthropicClient(for_ubi) -> <UBIQX_API_BASE>/v1/messages -> Anthropic
                                                                            OpenAiCompatClient -------> api.openai.com/v1/chat/completions
                                                                            OpenAiCompatClient -------> api.x.ai/v1/chat/completions
                                      mark_sent + unlink file  <--------  parse_vision -> Classification
                                    retention loop (hourly) -> screenshots::cleanup
```

Composition root: `crates/ubiqx-app/src/lib.rs` (`App::start`, `build_ai`, `RemoteClients::build`).
The desktop shell (`apps/desktop/src-tauri`) and the CLI both go through it.

---

## 3. How a screenshot is taken

### 3.1 Trigger

Screenshots are **block-driven, not timer-driven**. The only call site is
`crates/ubiqx-engine/src/tracker.rs:113-139`, inside `handle_sample`, which runs once per sample
(`Settings.sample_interval_secs`, default `5`, `crates/ubiqx-core/src/model.rs:980`) on a
`spawn_blocking` thread (`tracker.rs:25`). The capture is synchronous inside that call, so a slow
capture delays the next sample.

```rust
// tracker.rs:114-117
if let Some(open) = outcome.open {
    if !idle && screenshots::should_capture(state, &open, state.now()) {
        let bounds = None; // bounds are not carried on samples; capturer falls back to the window id
        match screenshots::capture_for_block(state, &open, sample.window_id, bounds) {
```

Before that, `handle_sample` returns early (no capture) when tracking is paused, disabled or onboarding
is not done (`tracker.rs:53-60`); the sampler thread does not even sample in those states
(`sampler.rs:29-33`).

### 3.2 `should_capture` (`crates/ubiqx-engine/src/screenshots.rs:20-45`)

`pub fn should_capture(state: &EngineState, block: &ActivityBlock, now: DateTime<Utc>) -> bool`
returns `false` when any of these holds, in order:

| # | Condition | Source |
|---|---|---|
| 1 | `settings.screenshot_interval_secs == 0` (screenshots disabled) | `screenshots.rs:22` |
| 2 | `settings.is_private(now)` (private mode, timed or indefinite) | `screenshots.rs:22`, `model.rs:1046-1048` |
| 3 | `block.category_id == Some(system_categories::PRIVATE)` (`"sys-private"`): blocked app, blocked domain, incognito window | `screenshots.rs:25-27` |
| 4 | `!vision_policy.allows(app_id, app_name, blocked_apps, vision_denied_apps)` **and** `!keep_screenshots_for_review` | `screenshots.rs:28-36` |
| 5 | block younger than `FIRST_CAPTURE_AFTER_SECS` (`pub const … : i64 = 20`, `screenshots.rs:13`) | `screenshots.rs:37-39` |
| 6 | last capture of this block less than `screenshot_interval_secs` ago (`state.last_capture: Mutex<HashMap<Id, DateTime<Utc>>>`, `state.rs:26`) | `screenshots.rs:40-44` |

Consequences: at most one screenshot per `screenshot_interval_secs` (default `120`, UI minimum 30,
`apps/desktop/src/pages/Settings.tsx:856`) per open block, none for blocks shorter than 20 s, and with
`VisionPolicy::Never` plus `keep_screenshots_for_review = false` no screenshot is ever taken.

The idle check is the caller's (`!idle`, `tracker.rs:115`).

### 3.3 `capture_for_block` (`screenshots.rs:50-117`)

```rust
pub fn capture_for_block(
    state: &Arc<EngineState>,
    block: &ActivityBlock,
    window_id: Option<u32>,
    bounds: Option<(i32, i32, u32, u32)>,
) -> CoreResult<Screenshot>
```

Steps:

1. `platform.capturer.visible_apps()`; if any entry equals (ASCII case-insensitive) an entry of
   `settings.blocked_apps`, return `Err(CoreError::Invalid("blocked app visible"))`
   (`screenshots.rs:60-69`). This is "a blocked app has a window anywhere on screen", not "is
   frontmost". `vision_denied_apps` is not consulted here.
2. Target (`screenshots.rs:71-78`):
   `(Some(id), _)` → `CaptureTarget::Window(id)`;
   `(None, Some(bounds))` → `CaptureTarget::DisplayAt { centre of bounds }`;
   otherwise → `CaptureTarget::PrimaryDisplay`.
   The tracker always passes `bounds = None`, so the two reachable cases are **window** (when the
   sample has a window id) and **whole primary display** (when it does not). See 15.1.
3. `platform.capturer.capture(target, settings.screenshot_max_edge)` (`screenshots.rs:82-84`).
4. Write `<data_dir>/screenshots.noindex/<YYYY-MM-DD>/<id>.jpg` (`screenshots.rs:86-98`). The day
   folder is named from `state.now()` formatted in UTC. On Unix the root `screenshots.noindex` is
   chmod `0700`; nothing equivalent on Windows.
5. Insert a `Screenshot` row with `sent_to_ai: false` (`screenshots.rs:100-110`), remember the capture
   time in `state.last_capture`, emit `EngineEvent::ScreenshotTaken { screenshot_id, block_id }`.
6. The tracker then links it: `repos.blocks.set_screenshot(&open.id, &shot.id)` (`tracker.rs:121`).

Errors: `CoreError::Permission` and any other error only set `last_capture` so the capture is not
retried on every sample (`tracker.rs:123-136`); nothing is surfaced beyond a `debug!` line.

### 3.4 Ports (`crates/ubiqx-core/src/ports.rs:36-67`)

```rust
pub struct EncodedImage { pub bytes: Vec<u8>, pub mime: String, pub width: u32, pub height: u32 }

pub enum CaptureTarget { Window(u32), DisplayAt { x: i32, y: i32 }, PrimaryDisplay }

pub trait ScreenCapturer: Send + Sync {
    fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage>;
    fn visible_apps(&self) -> CoreResult<Vec<String>>;
}
```

`ForegroundWindow` (`model.rs:20-33`) carries `window_id: Option<u32>` and
`bounds: Option<(i32, i32, u32, u32)>`; `ActivitySample` (`model.rs:37-47`) carries only `window_id`.

### 3.5 Per-OS capture

| OS | Implementation | Window capture | Fallback | Permission | File |
|---|---|---|---|---|---|
| macOS | `CgScreenCapturer` over `xcap` (CoreGraphics `CGWindowListCreateImage`) | `xcap::Window::all()` → window with that id → `capture_image()` | none inside the capturer; error "window {id} not found" | `CGPreflightScreenCaptureAccess()`; without it `Err(CoreError::Permission("screen recording"))` before any capture | `crates/ubiqx-platform/src/macos/capture.rs` |
| Windows | `WindowsScreenCapturer` over `xcap` | xcap window capture by id; if xcap cannot see the window or the image is empty, captures the monitor under the window and crops it to the DWM frame (`crop_to`) | the cropped display capture: other windows leak in where they overlap | none (`PermissionState::NotApplicable`) | `crates/ubiqx-platform/src/windows/capture.rs` |
| Linux | `X11ScreenCapturer` over `x11rb` `GetImage` (ZPixmap, depth 24/32) | window drawable clipped to the screen; falls back to reading the root window under the window's rectangle | `DisplayAt` and `PrimaryDisplay` both mean the whole X screen | none; **no capture under Wayland** (`"capture: no X11 display (Wayland needs XWayland)"`) | `crates/ubiqx-platform/src/linux/capture.rs`, `linux/x11.rs:289-322` |
| mock | `SyntheticCapturer` (1440×900 checkerboard), `DeniedCapturer` (always `Permission`) | — | — | — | `crates/ubiqx-platform/src/mock.rs:275-312` |

All three native capturers reject a near-uniform frame: `is_uniform(&img)` →
`Err(CoreError::Platform("capture is blank"))` (`image_util.rs:46-68`: sample ≈4096 pixels, fewer than
1 % differing from the first by more than 24 in summed RGB distance). That was written for "wallpaper
only / black frame without permission"; a legitimately black frame (a dark scene, a protected video
surface) is reported as an error, not as an image.

`visible_apps()` returns app ids *and* names of on-screen windows (macOS: owner names and bundle ids of
layer-0 windows; Windows: app id and app name of titled top-level windows; Linux: app id and `WM_CLASS`
of viewable client-list windows; empty without an X display).

### 3.6 Size and format (`crates/ubiqx-platform/src/image_util.rs`)

```rust
pub const JPEG_QUALITY: u8 = 60;   // "60 keeps text legible at ~100 KB"
pub fn downscale_and_encode(img: RgbaImage, max_edge: u32) -> CoreResult<EncodedImage>
```

- Longest edge scaled down to `max_edge.max(64)`; never upscaled; `FilterType::Triangle`.
- Alpha dropped, encoded as **JPEG quality 60**, `mime = "image/jpeg"`. There is no other encoder
  path; `EncodedImage.mime` documents webp/png but nothing produces them.
- `max_edge` comes from `Settings.screenshot_max_edge`, default `1280` (`model.rs:984`; UI minimum
  256, step 64, `Settings.tsx:858`). A 2560×1600 capture becomes 1280×800.
- Size: the code comment says ≈100 KB; the unit test only asserts `< 400_000` bytes for a synthetic
  gradient (`image_util.rs:84`). Base64 adds one third.

The quality is a constant, not a parameter. A nofap capture at a smaller `max_edge` reuses the function
unchanged.

---

## 4. When a screenshot reaches the AI (the existing vision path)

`crates/ubiqx-engine/src/classify.rs`, `pub async fn run_once(state: &Arc<EngineState>, force: bool)`,
called by `classify_loop` every `LoopConfig::classify_every` (30 s, `engine.rs:42`) and by
`EngineHandle::classify_now` (`handle.rs:232-235`).

1. `repos.blocks.list_pending_remote(now, 200)`: **closed** blocks without a category, not flagged for
   review, past their `next_attempt_at` (`classify.rs:327`).
2. Local chain: user rules → memory → learned rules (`classify_locally`, `classify.rs:71-88`).
3. Remote text, only if `force` or `remaining.len() >= classify_batch_min` (default `8`) or the oldest
   block has waited `classify_max_wait_secs` (default `240`) (`classify.rs:363-365`), and only if
   `state.deps.ai.remote` is `Some`, `budget_allows(state)?` and `state.remote_allowed()`
   (`classify.rs:366-374`). Chunks of 25.
4. A block goes to the vision list when the text answer is not confident (`min_confidence`, default
   `0.6`) **or** has `needs_vision = true` (`classify.rs:392-408`).
5. Vision loop (`classify.rs:463-532`), for each such block, sequentially:
   - `settings.vision_policy.allows(...)` must hold and
     `sent_this_hour < settings.max_vision_per_hour` (default `20`, UI range 0–60), where
     `sent_this_hour = repos.screenshots.count_sent_since(now - 1h)`; otherwise `mark_unresolved`.
   - `repos.screenshots.latest_for_block(&b.id)`: no screenshot → `mark_unresolved`.
   - `screenshots::load(&shot)` reads the file back into an `EncodedImage` with
     `mime = "image/jpeg"` (`screenshots.rs:120-129`).
   - `vision.classify_with_image(b, &image, &ctx).await`.
   - `Ok` → `repos.screenshots.mark_sent(&shot.id)`, counter + 1, and, unless
     `keep_screenshots_for_review`, `screenshots::unlink(&shot)` deletes the **file** (the row stays
     until retention). Confident → `apply`; otherwise the description is kept and the block goes to
     review.
   - `Err(CoreError::AiRefused)` → block to review, loop continues.
   - any other `Err` → `handle_remote_failure`, `break` (remaining candidates wait for the next pass).

So: vision is asked about a block at the earliest some tens of seconds after the user left that
window, usually minutes; it sees the *latest* screenshot of the block; at most 20 images per hour by
default.

`VisionPolicy` (`model.rs:796-826`):

```rust
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum VisionPolicy { Never, OnlyApps { apps: Vec<String> }, AllExceptBlocked }

pub fn allows(&self, app_id: &str, app_name: &str, blocked: &[String], denied: &[String]) -> bool
// Never => false
// OnlyApps => in apps && !in blocked
// AllExceptBlocked => !in blocked && !in denied      (default)
```

Matching is exact, ASCII case-insensitive, against app id or app name.

---

## 5. The vision request: prompt, schema, parsing

`crates/ubiqx-ai/src/vision.rs`.

```rust
pub const MAX_TOKENS: u32 = 400;
pub const SUPPORTED_MEDIA_TYPES: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/gif"];
pub const CONFIDENCE_HIGH: f32 = 0.9;  pub const CONFIDENCE_MEDIUM: f32 = 0.7;  pub const CONFIDENCE_LOW: f32 = 0.4;

pub struct LlmVisionClassifier { client: Arc<dyn LlmClient> }
pub fn build_request(block: &ActivityBlock, image: &EncodedImage, ctx: &ClassificationContext) -> CoreResult<LlmRequest>
```

The request built at `vision.rs:60-92`:

| Field | Value |
|---|---|
| `model` | `model_or_default(&ctx.models.vision, &AiModels::default().vision)`: the user's setting, else `claude-haiku-4-5` |
| `system` | one block, `prompts::vision_system_prompt(ctx)`, `cache = false` |
| `messages` | one user message: `[LlmContent::Image { media_type, base64 }, LlmContent::Text(prompts::vision_user_text(block))]` (image first) |
| `max_tokens` | `400` |
| `json_schema` | `vision_schema()`: `{category_id: string|null, description: string, evidence: string, confidence: "high"|"medium"|"low"}`, all required, `additionalProperties: false` |
| `disable_thinking` | `thinking_disabled_for(&model)` (true for every model whose id does not contain `haiku`) |
| `usage_kind` | `AiUsageKind::Vision` |
| `timeout` | `Some(LONG_TIMEOUT)` = 120 s |

Validation: unsupported media type or empty bytes → `CoreError::Invalid`.

`classify_with_image` (`vision.rs:96-115`): one call; if `resp.is_truncated()` (`StopReason::MaxTokens`)
retry once with `max_tokens * 2`; truncated twice → `CoreError::Ai("vision output truncated twice")`.

Prompt (`crates/ubiqx-ai/src/prompts.rs:549-581`): English instructions, the user's category catalogue
(`render_catalogue`: ids, names, descriptions, keywords), rules that forbid transcribing or naming
anything personal, and the user's free-text profile (`profile_section(ctx.user_profile, "About the
user")`, capped at `MAX_PROFILE_CHARS = 800`). The text part is

```
Block metadata:
- id=<id> | app=<app> | title=<title> [| domain=<d>] [| path=<p>] | min=<n> | time=<HH:MM–HH:MM>

Look at the screenshot and answer with the JSON object only.
```

where the line is `render_block_line(block)`, i.e. `PromptBlock::from_block` → `redact_block`
(section 10.5). A `PromptBlock` cannot be built without redaction (private fields, one constructor).

`parse_vision` (`vision.rs:131-159`): invalid JSON → `CoreError::Ai`; unknown category id → `None`;
`low` confidence → `category_id = None`; description passes through `clean_description`
(`redact_text` + whitespace collapse). `evidence` is only logged at `debug`.

The port (`ports.rs:514-522`):

```rust
#[async_trait]
pub trait VisionClassifier: Send + Sync {
    async fn classify_with_image(&self, block: &ActivityBlock, image: &EncodedImage, ctx: &ClassificationContext)
        -> CoreResult<Classification>;
}
```

It is tied to a block, a category catalogue and a `Classification` result. It is the wrong shape for a
yes/no/unsure question; section 14 lists what a sibling port needs.

---

## 6. How the request reaches each provider

### 6.1 Vendor-neutral types (`crates/ubiqx-ai/src/client.rs:181-307`)

```rust
pub enum LlmContent { Text(String), Image { media_type: String, base64: String } }
pub struct LlmMessage { pub role: LlmRole, pub content: Vec<LlmContent> }
pub struct SystemBlock { pub text: String, pub cache: bool }
pub struct LlmRequest {
    pub model: String, pub system: Vec<SystemBlock>, pub messages: Vec<LlmMessage>, pub max_tokens: u32,
    pub json_schema: Option<Value>, pub disable_thinking: bool,
    pub usage_kind: AiUsageKind, pub timeout: Option<Duration>,
}
pub enum StopReason { EndTurn, MaxTokens, Refusal, Other(String) }
pub struct LlmResponse { pub text: String, pub model: String, pub input_tokens: u32, pub output_tokens: u32,
                         pub cache_read_tokens: u32, pub cache_write_tokens: u32, pub stop_reason: StopReason }
#[async_trait] pub trait LlmClient: Send + Sync { async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse>; }
```

`validate_request` (`client.rs:389-409`): non-empty model, `max_tokens > 0`, at least one message, last
message from the user, no empty message.

Constants (`client.rs:38-53`): `DEFAULT_BASE_URL = "https://api.anthropic.com"`,
`API_VERSION = "2023-06-01"`, `DEFAULT_TIMEOUT = 60 s`, `LONG_TIMEOUT = 120 s`,
`DEFAULT_MAX_ATTEMPTS = 4`, `DEFAULT_BACKOFF_BASE = 2 s`, `DEFAULT_BACKOFF_CAP = 30 s`.

### 6.2 Routing and model selection

- `RoutingLlmClient` (`crates/ubiqx-ai/src/router.rs:37-104`) holds one client per `AiProvider` and
  reads `ProviderSource::provider()` on **every call**. In the app that is `StoredProvider`
  (`crates/ubiqx-app/src/lib.rs:162-195`), which loads the settings row from SQLite each time. No
  client for the selected provider → `CoreError::AiNotConfigured`.
- Keys: `SecretStoreKey` reads `AiProvider::secret_key()` from the secret store on every call
  (`ubiqx-app/src/lib.rs:148-158`). Secret names: `anthropic_api_key`, `openai_api_key`,
  `xai_api_key`, and for Ubi the license key under `ubiqx.license` (`model.rs:572`, `654-661`).
- Model ids come from `Settings.models: AiModels { classify, vision, report }` (`model.rs:696-760`).
  Defaults per provider (`AiModels::for_provider`):

  | Provider (`AiProvider::id()`) | classify | vision | report |
  |---|---|---|---|
  | `anthropic` | `claude-haiku-4-5` | `claude-haiku-4-5` | `claude-sonnet-5` |
  | `openai` | `gpt-5-mini` | `gpt-5-mini` | `gpt-5` |
  | `xai` | `grok-4-1-fast-non-reasoning` | `grok-4-1-fast-non-reasoning` | `grok-4-1-fast-reasoning` |
  | `ubi` | `ubi-fast` | `ubi-fast` | `ubi-smart` |

  `Settings::sanitize()` → `AiModels::reconciled_with(provider)` replaces blank ids and ids that
  visibly belong to another vendor; for `ubi` it always forces the two aliases. Custom ids are kept.
  The vision model id is user-editable; the UI hint is only "Precisa aceitar imagens".

### 6.3 Anthropic (`AnthropicClient`, `client.rs:640-955`)

`POST {base}/v1/messages`, headers `x-api-key`, `anthropic-version: 2023-06-01`,
`content-type: application/json`, user agent `ubiqx/<version>`. Body from `request_body`
(`client.rs:346-388`):

```json
{
  "model": "claude-haiku-4-5",
  "max_tokens": 400,
  "system": [{"type": "text", "text": "..."}],
  "messages": [{"role": "user", "content": [
    {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": "<base64>"}},
    {"type": "text", "text": "Block metadata: ..."}
  ]}],
  "output_config": {"format": {"type": "json_schema", "schema": { ... }}}
}
```

plus `"thinking": {"type": "disabled"}` when `disable_thinking` is true (never for an id containing
`haiku`), and `cache_control: {"type": "ephemeral"}` on system blocks marked `cache`. Never
`temperature`, never an assistant prefill.

Response handling (`client.rs:727-833`): first `text` content block is the answer; usage from
`usage.{input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens}`; cost from
the `x-ubiqx-cost-usd` response header when present, else `pricing::estimate_cost_usd_for_provider`;
an `AiUsage` row is recorded (failure to record is only logged); then `stop_reason == refusal` →
`Err(CoreError::AiRefused)`.

### 6.4 OpenAI and xAI (`OpenAiCompatClient`, `crates/ubiqx-ai/src/openai.rs`)

`POST {base}/chat/completions` with `Authorization: Bearer <key>`; bases
`https://api.openai.com/v1` and `https://api.x.ai/v1`. Body from `request_body(provider, req)`
(`openai.rs:209-252`):

```json
{
  "model": "gpt-5-mini",
  "messages": [
    {"role": "system", "content": "<system blocks joined with blank lines>"},
    {"role": "user", "content": [
      {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,<base64>"}},
      {"type": "text", "text": "Block metadata: ..."}
    ]}
  ],
  "max_completion_tokens": 400,
  "reasoning_effort": "minimal",
  "response_format": {"type": "json_schema", "json_schema": {"name": "ubiqx_output", "strict": true, "schema": { ... }}}
}
```

- Token cap field: `max_completion_tokens` for OpenAI, `max_tokens` for xAI (`max_tokens_field`,
  `openai.rs:162-167`).
- `reasoning_effort` is sent **only when `usage_kind` is `Classify` or `Vision`**
  (`reasoning_effort_for`, `openai.rs:176-192`): `gpt-5*` → `"minimal"`, OpenAI `o<digit>*` → `"low"`,
  xAI `grok-3-mini*` → `"low"`, everything else omitted.
- No `detail` field on `image_url` (the vendor default applies).
- `SystemBlock::cache` and `disable_thinking` are ignored.
- Usage: `input_tokens = prompt_tokens − cached_tokens`, `output_tokens = completion_tokens`
  (includes reasoning tokens).
- Refusal: non-empty `message.refusal` or `finish_reason = "content_filter"` → `StopReason::Refusal` →
  `Err(CoreError::AiRefused)` after the usage row is written (`openai.rs:336-346`, `759`).

### 6.5 Ubi (managed)

`AnthropicConfig::for_ubi(api_base)` (`client.rs:143-154`): same `AnthropicClient`, base URL = the
proxy, `provider: AiProvider::Ubi`, extra header `x-ubiqx-plan: monthly_managed`; the license key
travels as `x-api-key`. Base URL: `UBIQX_API_BASE` at build time or
`UBIQX_API_BASE_DEFAULT = "https://api.ubiqx.ai"` (`crates/ubiqx-core/src/license.rs:57`). Models are
the aliases `ubi-fast` / `ubi-smart` (`license.rs:60-62`). Cost comes from the proxy's
`x-ubiqx-cost-usd` header (`cost_from_headers`, `client.rs:166-175`).

### 6.6 Retry and status mapping (`crates/ubiqx-ai/src/retry.rs`, `client.rs:592-632`, `openai.rs:488-527`)

`send_with_retry`: up to `max_attempts` (4) tries; network errors and timeouts are always retried;
wait is the server's `retry-after` when given, else `backoff_delay` = `base · 2^(attempt−1)` capped at
30 s, times a jitter factor in `[0.75, 1.25)`; a `retry-after` above the cap aborts.

| HTTP | Anthropic / Ubi | OpenAI / xAI |
|---|---|---|
| 401, 403 | `Ai("invalid api key …")` (Ubi: `"license rejected as api key by the Ubi proxy …"`) | `Ai("invalid api key …")` |
| billing / unknown model | `AiRejected(...)` (Anthropic: 400 credit balance, 402 `billing_error`, 404 `model:`; Ubi: 402 or `budget_exhausted`, 400 mentioning "model") | `AiRejected(...)` (402, `insufficient_quota`, `billing_hard_limit_reached`, 404, `model_not_found`) |
| 400, 404, 413, 422 (other) | `Invalid("HTTP <code>: …")` | 400, 413, 422 → `Invalid` |
| 429 | retried, then `RateLimited { retry_after_secs }` | same, also parses "try again in Ns" |
| 408, 500, 502, 503, 504, 529 | retried, then `Ai(...)` | 408, 409, 5xx retried, then `Ai(...)` |
| no key | `AiNotConfigured` before any request | same |

A content-policy rejection that a vendor returns as HTTP 400 would surface as `CoreError::Invalid`,
not as `AiRefused`; the code has no special case for it.

---

## 7. The "IA do Ubi" proxy (`services/ubi-api`)

Standalone Cargo project (axum 0.7, reqwest 0.12), not a workspace member.

`POST /v1/messages` (`services/ubi-api/src/routes.rs:324-491`):

1. `authorize_managed`: `x-api-key` must be a license key that verifies against the public key, is not
   expired, not revoked and has plan `monthly_managed` (401 / 403 otherwise).
2. `x-ubiqx-plan`, when present, must be `monthly_managed` (400 otherwise).
3. The body is parsed as JSON; `model` must be `ubi-fast` or `ubi-smart`
   (`Config::resolve_model_alias`, `config.rs:255-261`), else 400 "unknown model". It is replaced by
   `UBI_MODEL_FAST` / `UBI_MODEL_SMART` env values, defaults
   `claude-haiku-4-5-20251001` / `claude-sonnet-5` (`config.rs:15-16`).
4. Budget: `spent_usd(sub, YYYY-MM UTC) >= monthly_budget_usd` (default `6.0`, env
   `UBI_MONTHLY_BUDGET_USD`) → 402 `budget_exhausted` with `{month, spent_usd, budget_usd}`. The check
   is before the call, so the last call can overshoot.
5. The **whole body is forwarded unchanged apart from `model`** to
   `{vendor_base_url}/v1/messages` with the operator's `ANTHROPIC_API_KEY`, the client's
   `anthropic-version` (default `2023-06-01`) and `anthropic-beta` if sent. Image blocks,
   `output_config` and `thinking` therefore pass through as they are.
6. Non-streaming success: usage is read from the vendor's JSON, cost computed with the proxy's
   `PriceTable` (Haiku 4.5 $1/$5, Sonnet 5 $2/$10 per MTok; cache read ×0.1, write ×1.25), a row
   `usage(sub, month, model, input_tokens, output_tokens, cost_usd, at)` is inserted, and the response
   carries `x-ubiqx-cost-usd`, `x-ubiqx-spent-usd`, `x-ubiqx-budget-usd`. Vendor errors are passed
   through with their status and body.

What the proxy keeps: the ledger row above (subscriber id, month, model, token counts, cost, time).
It does not store or log request or response bodies; the log lines are `sub`, `model`, `cost_usd`
(`routes.rs:482`) and a warning with status on vendor errors. Images transit the operator's server in
memory and go on to Anthropic under the operator's key.

Limits worth knowing:

- The handler takes `body: Bytes` and the router sets no `DefaultBodyLimit`, so axum's default request
  body limit applies (2 MiB in axum 0.7). A 1280 px JPEG q60 in base64 is well under it; a larger or
  PNG image may get 413, which the app maps to `CoreError::Invalid`.
- The proxy's `reqwest::Client` has no timeout (`routes.rs:51-53`); the app's per-request timeout is
  the only bound.
- Only Anthropic is supported upstream (`Vendor::Anthropic`).
- `services/ubi-api/tests/proxy.rs` has no test that sends an image block.

`GET /v1/license/status` feeds `ManagedUsage { month, spent_usd, budget_usd }` to the app
(`crates/ubiqx-ai/src/ubi.rs`, `crates/ubiqx-engine/src/license.rs:91-124`), refreshed 5 s after start
and hourly (`engine.rs:49-50`).

---

## 8. Failure modes: offline, no AI, refusals, budget, health

### 8.1 Gates before any remote call

`EngineState::remote_allowed()` (`crates/ubiqx-engine/src/state.rs:228-243`) is `false` when:

- `settings.local_only` ("Somente local"),
- `!settings.onboarding_done` (consent not given yet; a leftover key must not send anything),
- `license.blocks_ai()` (only under `LicenseEnforcement::Hard`; the compiled constant is
  `LICENSE_ENFORCEMENT = Soft`, `license.rs:53`),
- `ai_health` is `NotConfigured` or `Paused`, or `Degraded` with `until` still in the future.

`key_health` (`state.rs:251-272`) sets the initial health: `NotConfigured` when there is neither a
remote classifier nor a report writer; `Ok` when `local_only` or `!deps.ai.requires_api_key` (fakes);
for `ubi`, `Ok` only with a valid `monthly_managed` license; otherwise `Ok` iff the selected
provider's key exists in the secret store.

### 8.2 Budget (`classify::budget_allows`, `classify.rs:105-147`)

- Managed provider: uses `license.managed_usage`; if `managed_budget_exhausted(&u, now)` (same UTC
  month, `budget_usd > 0`, `spent >= budget`) → `AiHealth::Paused { reason }`, returns `false`.
- Own key: `ai_monthly_budget_usd <= 0` means no limit. Otherwise
  `ratio = usage.totals(month_range(now)).cost_usd / budget` (local calendar month):
  `>= 1.0` → `budget_paused = true`, `AiHealth::Paused { "Orçamento mensal de IA atingido (US$ x de US$ y)." }`,
  `false`; `>= 0.8` → one attention nudge per month (kv key `budget_warned_YYYY-MM`).
- The check is the single owner of budget pauses and lifts them (`clear_budget_pause`) when the month
  rolls over, the budget is raised, or data is wiped.
- It runs once at the top of the remote phase of `run_once`, in `reports::generate` and in
  `EngineHandle::advice`; it is not re-checked between vision calls.

### 8.3 What each failure does (`handle_remote_failure`, `classify.rs:237-306`, private to the module)

| Error | Effect on health | Effect on blocks |
|---|---|---|
| `AiNotConfigured` | `NotConfigured` | none |
| `RateLimited { retry_after_secs }` | `Degraded { until: now + max(retry_after, 30 s) }` | none |
| `Ai(msg)` containing "api key" / `401` / `403` | `Paused` + attention nudge "IA pausada" | none |
| `AiRejected(msg)` (billing, model) | `Paused { "IA indisponível: …" }` + one nudge | none; managed: refresh usage, set `budget_paused` if spent |
| `AiRefused` | untouched | batch/block sent straight to review (`record_attempt(id, MAX_ATTEMPTS, None, true)`) |
| other `Ai(msg)` (network, 5xx, unparseable) | `Degraded { until: now + 15 min }` | none |
| `Invalid(_)` | untouched | per-block backoff (`backoff_for`: 1 m, 4 m, 16 m, 1 h, 4 h), review after `MAX_ATTEMPTS = 5` |

### 8.4 Scenarios

| Situation | Screenshot taken? | Sent? | Result |
|---|---|---|---|
| Offline | yes | attempted: 4 tries with backoff, each up to the request timeout, then `Ai("network error…")` | AI `Degraded` for 15 min; blocks stay pending; screenshots stay on disk until retention |
| No key / no remote AI (`AiBackend::None`) | yes (if policy allows) | no | `run_once` returns `skipped_remote` at `classify.rs:366-369`; nothing is classified beyond rules and memory |
| `local_only` | yes | no | same |
| Budget exhausted | yes | no | `Paused`; resumes on rollover or a raised budget |
| Managed plan budget exhausted (402) | yes | the call that got 402 | `AiRejected` → `Paused`, usage refreshed |
| Vision model refuses | — | yes, and billed | `AiRefused`; the screenshot is **not** marked sent and **not** deleted (15.6) |
| Wayland / no Screen Recording permission | no | no | text-only classification continues |
| Vision hourly cap reached | yes | no | block `mark_unresolved` (review after 2 attempts) |

---

## 9. Cost per call

Price table used for the ledger (`crates/ubiqx-ai/src/pricing.rs`, USD per million tokens, input /
output): `claude-haiku-4-5` 1 / 5 (`HAIKU_4_5`), `claude-sonnet-5` 2 / 10 (`SONNET_5`, also the
fallback for unknown Anthropic ids), `claude-opus-5` 5 / 25, `gpt-5-mini` 0.25 / 2, `gpt-5` 1.25 / 10,
`grok-4-1-fast*` 0.20 / 0.50, `ubi-fast` priced as Haiku, `ubi-smart` as Sonnet. Family matching is by
substring; any Anthropic id containing `haiku` is priced 1 / 5. Cached reads ×0.10, cache writes ×1.25.

The code does not estimate tokens before a call; cost is computed from the usage the vendor returns.
The numbers below are therefore **estimates**, and the image-token rule is Anthropic's published
approximation for models of the Haiku 4.5 generation (about width × height / 750, at most about 1,600
tokens per image), not something in this repository. Newer high-resolution Claude models can use up to
about three times as many image tokens. Measure with real calls before fixing a budget.

| Image sent | ≈ image tokens | + prompt | Output | ≈ cost on `claude-haiku-4-5` / `ubi-fast` |
|---|---|---|---|---|
| 1280×800 (today's default) with the existing vision prompt | 1,365 | 400–900 (catalogue, profile) | ≈ 80 | US$ 0.0022–0.0027 |
| 1280×800 with a short yes/no prompt | 1,365 | ≈ 250 | ≈ 30 | ≈ US$ 0.0018 |
| 768×480 with a short prompt | ≈ 490 | ≈ 250 | ≈ 30 | ≈ US$ 0.0009 |
| 512×320 with a short prompt | ≈ 220 | ≈ 250 | ≈ 30 | ≈ US$ 0.0006 |

Against the budgets in the code:

- User default `ai_monthly_budget_usd = 5.0`; proxy default `6.0` per subscriber per month.
- The existing cap of 20 images per hour, used fully for 8 h on 22 days at US$ 0.0018, is about
  US$ 6.3 a month, already above both defaults.
- One check per minute for 6 active hours a day is 360 calls, about US$ 0.65 a day at 1280 px
  (≈ US$ 0.32 at 768 px): the monthly budget is gone in one to two weeks, and when it is gone the
  productivity classification pauses with it.

OpenAI and xAI default models are cheaper per token in the table, but their image tokenisation is not
represented anywhere in the code; the ledger will show the real cost after the first calls.

`README.md:141-145` quotes whole-product estimates per provider (Anthropic US$ 3–6 a month, OpenAI
US$ 1–3, xAI US$ 0.50–2 at 8 h a day); those were written for the current, sparse use of vision.

---

## 10. Privacy controls in force today

### 10.1 Consent

Nothing is sampled, captured or sent before `settings.onboarding_done`
(`sampler.rs:29-33`, `tracker.rs:53`, `state.rs:231`, `state.rs:189`).

### 10.2 Screenshot policy settings (`Settings`, `crates/ubiqx-core/src/model.rs:905-1019`)

| Field | Default | Meaning |
|---|---|---|
| `screenshot_interval_secs: u32` | `120` | seconds between screenshots of one block; `0` disables screenshots |
| `screenshot_max_edge: u32` | `1280` | longest edge in pixels |
| `screenshot_retention_hours: u32` | `24` | rows and files older than this are purged hourly |
| `keep_screenshots_for_review: bool` | `false` | keep the file after vision used it; also allows capture where vision is not allowed |
| `vision_policy: VisionPolicy` | `AllExceptBlocked` | which apps' screenshots may be sent |
| `vision_denied_apps: Vec<String>` | `[]` | excluded from vision under `AllExceptBlocked` |
| `blocked_apps: Vec<String>` | 1Password (2 ids), Keychain Access, Bitwarden, Apple Passwords, Passbook | never sampled for content nor captured |
| `blocked_domains: Vec<String>` | `[]` | content never recorded |
| `private_mode: bool`, `private_until: Option<DateTime<Utc>>` | `false`, `None` | pause content recording and screenshots |
| `max_vision_per_hour: u32` | `20` | cap on screenshots sent per hour |
| `ai_monthly_budget_usd: f64` | `5.0` | monthly cap on estimated spend |
| `local_only: bool` | `false` | never call a remote AI |
| `min_confidence: f32` | `0.6` | below this the next classifier is tried |

`Settings` is `#[serde(default)]` and stored as one JSON row; `Settings::sanitize()` runs on every
write (`EngineState::apply_settings`, `state.rs:120-168`).

UI: Settings → privacy section (`apps/desktop/src/pages/Settings.tsx:760-865`), onboarding "vision"
step (`apps/desktop/src/pages/Onboarding.tsx:636-680`). Copy:
`apps/desktop/src/i18n/messages/settings.ts:128-160`.

### 10.3 Blocked apps, blocked domains, incognito, private mode (`crates/ubiqx-core/src/segmenter.rs:101-150`)

`Segmenter::is_blocked(sample)` is true when the app id or name is in `blocked_apps`, **or**
`normalize::is_private_browsing_title(&sample.window_title)`, **or** the URL's domain matches a
`blocked_domains` pattern (`domain_matches`: suffix-aware, `*.` prefix ignored).

Incognito markers (`crates/ubiqx-core/src/normalize.rs:79-88`, substring match on the lower-cased
title): `(incognito)`, `(anônima)`, `(anonima)`, `navegação privada`, `navegacao privada`,
`private browsing`, `inprivate`, `navegação anônima`.

A blocked sample, or any sample while `cfg.private_mode`, is replaced by a redacted one:
`window_title = "[privado]"` (`PRIVATE_TITLE`), `url = None`, `window_id = None`; in private mode
also `app_name = "Privado"/"Private"` and `app_id = "privado"`. The resulting block gets
`category_id = "sys-private"`, `confidence = 1.0`, `source = Rule`. Such blocks are never pending for
the classifier and never captured (3.2 #3).

Private mode: `EngineHandle::set_private_mode(PrivateModeDuration::{Off, Minutes30, Hour1,
UntilTomorrow, Indefinite})` (`handle.rs:99-134`); expiry is applied by the nudges loop every 60 s
(`engine.rs:187-194`).

Pause: `EngineHandle::pause()` / `resume()` (`handle.rs:84-97`) stop sampling entirely.

### 10.4 Capture-time protections

- Window-only capture when a window id is known; no display fallback on macOS when the window capture
  fails (`screenshots.rs:79-84`).
- No capture while any `blocked_apps` entry has a window on screen (3.3 step 1).
- Files under a `0700` directory named `screenshots.noindex` (excluded from Spotlight).
- `service::screenshot_for_block` only reads files that canonicalise inside the screenshots directory
  (`service.rs:101-113`).

### 10.5 Redaction of text (`crates/ubiqx-core/src/redact.rs`)

```rust
pub fn redact_text(text: &str) -> String      // [email], [cnpj], [cpf], [número] (>= 9 digits), [telefone]
pub fn redact_url(url: &str) -> Option<String> // scheme + host + path only; no query, fragment, credentials
pub fn is_messaging_app(app_id: &str, app_name: &str) -> bool
pub fn redact_block(app_id, app_name, title, url, domain) -> RedactedBlock { app_name, title, url, domain }
```

Titles of messaging and mail apps (`MESSAGING_APPS`, `MESSAGING_BUNDLE_PREFIXES`: WhatsApp, Messages,
Telegram, Mail, Outlook, Slack, Discord, Messenger, Teams) are replaced by the app name. Prompt-side
limits: title 120 chars, path 80, description 200, profile 800 (`prompts.rs:32-36`).

**Redaction is text only. Nothing blurs, crops or masks the image.** The only protections for pixels
are the capture gates above and the instruction in the vision prompt not to transcribe.

### 10.6 Logging

`ubiqx_app::init_tracing` writes to stderr with `UBIQX_LOG` (default `info`)
(`crates/ubiqx-app/src/lib.rs:577-580`). At `info` each call logs vendor, model, kind, token counts,
cost and stop reason. At `trace` both clients log the **full request body, base64 image included**
(`client.rs:745`, `openai.rs:689`) and the answer text (`client.rs:811`, `openai.rs:757`).

---

## 11. The "Dados enviados à IA" audit trail

What exists:

- `ActivityBlock.ai_payload: Option<String>` and `ai_sent_at: Option<DateTime<Utc>>`
  (`model.rs:128-130`), written by `BlockRepo::set_ai_payload(id, payload, at)` (`ports.rs:199`).
- Text classification stamps `remote.describe_payload(b)` for every block of a chunk **before** the
  request is sent (`classify.rs:379-383`); for `LlmTextClassifier` that is exactly
  `prompts::render_block_line(block)` (`classifier.rs:154-156`).
- Reports append `"[relatório] <report line>"` per block (`reports.rs:152-165`).
- Read model: `service::ai_sent_blocks(state, date)` = that day's timeline filtered by
  `ai_sent_at.is_some()` (`service.rs:388-393`); Tauri command `get_ai_sent`
  (`apps/desktop/src-tauri/src/commands.rs:180-187`); UI `AiSentDialog`
  (`Settings.tsx:882-913`), the `AiSentBadge` on the timeline and block details.
- For images: `screenshots.sent_to_ai` (`0/1`), set by `mark_sent` after a successful vision call.

What it does not cover (all verified in code):

- The vision call writes nothing to `ai_payload`; the block already carries the text line from the
  text call, and the image is represented only by `sent_to_ai` on the screenshot row, which no IPC
  command exposes.
- The dialog's "com imagem" badge is shown when `b.screenshot_id` is set (`Settings.tsx:901`), i.e.
  when the block *has* a screenshot, whether or not it was sent.
- The system prompt content (category catalogue, user profile) and the advisor's aggregated numbers
  are not in the trail.
- A payload is stamped before the call, so a failed call still shows as sent.
- The trail is per block and per day. An AI call that is not about a closed block has nowhere to be
  recorded.

---

## 12. What is persisted and for how long

| Data | Where | Lifetime |
|---|---|---|
| Screenshot file (JPEG) | `<data_dir>/screenshots.noindex/<YYYY-MM-DD>/<id>.jpg`; `data_dir` = `directories::ProjectDirs::from("ai", "ubiqx", "ubiqX")` (`ubiqx-app/src/lib.rs:141-145`) | deleted right after a **successful** vision call unless `keep_screenshots_for_review`; otherwise until `screenshot_retention_hours` (default 24 h) |
| Screenshot row | `screenshots(id, at, path, width, height, app_id, block_id, sent_to_ai)` (`crates/ubiqx-storage/src/migrations.rs:98-109`) | `screenshots::cleanup` hourly (`retention_every = 3600 s`, first run at start): `delete_before(now − retention)`, then unlink files and remove empty day folders |
| Block text sent | `blocks.ai_payload`, `blocks.ai_sent_at` | as long as the block; no automatic block retention is wired (`BlockRepo::delete_before` exists but has no caller in the engine) |
| Vision result | `blocks.category_id`, `confidence`, `source = vision`, `description` | as long as the block |
| AI usage | `ai_usage(at, kind, model, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd)` (`migrations.rs:149-160`) | until "Apagar todos os dados" |
| Advice cache | `kv["advice_<date>"]` | until wipe |
| Budget warning flag | `kv["budget_warned_<YYYY-MM>"]` | until wipe |
| Settings | `settings` single JSON row | permanent |
| API keys, license key | OS secret store (Keychain / Credential Manager / Secret Service, file fallback on Windows and Linux) | until removed |
| Proxy ledger | `usage(sub, month, model, input_tokens, output_tokens, cost_usd, at)` on the operator's server | operator's policy; no bodies |

`EngineHandle::delete_all_data` (`handle.rs:452-479`) removes every screenshot row and file, the whole
screenshots directory, the exports folder, and runs `MaintenanceRepo::wipe_user_data` (blocks,
screenshots, corrections, reports, nudges, AI usage, kv, learned rules, interventions, focus
sessions). Settings, categories, user rules, focus targets and keys stay.

---

## 13. Fakes and test doubles

`crates/ubiqx-ai/src/fake.rs`:

| Double | What it does | Use for nofap |
|---|---|---|
| `ScriptedLlmClient::new(Vec<String>)` | answers scripted texts in order (1000 in / 100 out tokens, `EndTurn`), **records every `LlmRequest`** (`requests()`); `push_response(LlmResponse)` for a fully specified response; `truncate_response(i)` marks one as `MaxTokens`; fails with `CoreError::Ai` when the script runs out | unit-test a new detector end to end: assert the image block, model, `max_tokens`, schema and timeout, script yes / no / unsure / invalid JSON / truncated |
| `MemoryUsageRepo` | in-memory `UsageRepo` with `records()` | cost and budget assertions |
| `FakeVisionClassifier::new(Option<&str>, &str)` | always the same category and description; `calls()` | engine tests of the existing path |
| `FakeClassifier`, `FakeReportWriter`, `FakeAdvisor` | deterministic text doubles | unchanged |
| `FAKE_MODEL = "fake"` | model name reported | — |

`ScriptedLlmClient` cannot script an **error** (for example `AiRefused` or a network failure): it
returns `Ok` or "no more responses". A detector test for the refusal path needs a small custom
`LlmClient` or an extension of the double.

`crates/ubiqx-platform/src/mock.rs`: `SyntheticCapturer` (valid JPEG from a 1440×900 pattern),
`DeniedCapturer`, `ScriptedPlatform` with `Scenario` / `Step` (foreground window, URL, idle,
permissions; the scripted window has `window_id: Some(77)` or `None`), `MockEnforcer`,
`MockAppCatalog`, `StaticUpdateFeed`. `PlatformServices::scripted(scenario)` bundles them.

HTTP-level tests use `wiremock` 0.6 (`crates/ubiqx-ai/tests/http_client.rs`, `openai_client.rs`,
`ubi_client.rs`, `router.rs`); `openai_client.rs:67-68, 186, 284` already assert the `image_url` shape.
`request_body` is public in both clients so a body can be asserted without a server.

Engine tests (`crates/ubiqx-engine/tests/pipeline.rs`): `Harness` with an in-memory `SqliteStore`,
`ScriptedPlatform`, `FixedClock`, `CollectSink`, `MemorySecrets`, `LoopConfig { without_sampler: true,
.. }` and manual sample feeding. `AiPorts` is built with struct literals in
`pipeline.rs:97, 113, 1381`, `tests/focus.rs:220`, `tests/screenshot_read.rs:67` and
`ubiqx-app/src/lib.rs:463, 493`.

`AiBackend::Fake` (`ubiqx-app/src/lib.rs:62-72, 462-474`) wires the fakes for the CLI and demos with
`requires_api_key: false`.

There is no fake that inspects pixels and nothing that simulates "this screen shows X".

---

## 14. What a single-purpose "explicit content?" call needs in each layer

The call: one image in, `{verdict: yes | no | unsure, confidence}` out.

### 14.1 `ubiqx-core`

- **Port** next to `VisionClassifier` (`ports.rs:513-522`), without a block and without the
  classification context, for example

  ```rust
  pub enum ExplicitVerdict { Yes, No, Unsure }
  pub struct ExplicitCheck { pub verdict: ExplicitVerdict, pub confidence: f32, pub refused: bool }
  #[async_trait]
  pub trait ExplicitContentDetector: Send + Sync {
      async fn check(&self, image: &EncodedImage, model: &str, language: &str) -> CoreResult<ExplicitCheck>;
  }
  ```

  (names are suggestions; nothing like it exists).
- **Settings**: a nested `#[serde(default)]` struct like `FocusSettings` (`model.rs:871-905`), added
  as a field of `Settings`. `Settings` itself is `#[serde(default)]`, so old stored JSON keeps
  loading. Mirror it in `apps/desktop/src/lib/types.ts:259-290` and `src/lib/mock.ts`.
- **Model id**: reuse `settings.models.vision`. `AiModels` has **no** `#[serde(default)]`
  (`model.rs:697-705`): adding a field without a per-field default makes every existing settings row
  fail to deserialise, and `Engine::start_with` propagates that error (`engine.rs:69`).
- **`AiUsageKind`** (`ports.rs:347-375`): either reuse `Vision` or add a variant. A new variant needs
  `as_str` / `parse`, the `reasoning_effort_for` match in `openai.rs:181` (otherwise `gpt-5*` runs at
  default reasoning effort, slower and billed as output), and the tests that enumerate kinds. The
  `ai_usage.kind` column is free text, so no migration.
- **Events**: `EngineEvent` (`model.rs:1053-1097`, `#[serde(tag = "type", rename_all = "snake_case")]`)
  is the engine → UI channel; a new variant needs its TypeScript twin.
- **Errors**: `CoreError::AiRefused` exists (`error.rs:29-32`).

### 14.2 `ubiqx-platform`

Nothing new is required to obtain an image:

- `ActivitySource::foreground()` → `ForegroundWindow { app_id, app_name, window_title, window_id, bounds, pid }`.
- `ScreenCapturer::capture(CaptureTarget::Window(id), max_edge)` with any `max_edge >= 64`; returns
  JPEG q60 bytes in memory.
- `BrowserUrlResolver::resolve(&window)` for the active tab's URL (to skip known-safe or
  already-blocked domains).
- `ScreenCapturer::visible_apps()` for the blocked-app check.

Not available: capture under Wayland, capture without the macOS Screen Recording permission, any
on-device image analysis (no model, no perceptual hash, no change detection), a quality parameter, and
a way to capture a region of a window.

### 14.3 `ubiqx-ai`

A new module shaped like `vision.rs` (≈150 lines there):

- `build_request(image, model) -> CoreResult<LlmRequest>` with
  `system: vec![SystemBlock::new(prompt, false)]`,
  `messages: vec![LlmMessage::user(vec![LlmContent::Image { .. }, LlmContent::Text(..)])]`,
  a small `max_tokens`, `json_schema: Some(schema)` (closed object, all properties required: both
  vendors' strict modes need that), `disable_thinking: thinking_disabled_for(&model)`,
  `timeout: Some(short)`.
- A prompt function in `prompts.rs`. If any window metadata is included it must go through
  `PromptBlock` / `redact_block`; a pure image question needs none. The existing vision prompt carries
  the category catalogue and the user profile, neither of which this call needs.
- `parse_*` with the same "invalid JSON → `CoreError::Ai`" convention, mapping an unknown verdict
  string to `Unsure`.
- **Refusal mapping**: catch `Err(CoreError::AiRefused)` from `client.complete` and turn it into a
  result (`refused: true`), because the clients throw the answer away (6.3, 6.4).
- Truncation: follow `vision.rs:105-112` (retry once with doubled `max_tokens`) or size `max_tokens`
  generously; see 15.9 for models that think by default.
- No change in `client.rs`, `openai.rs` (apart from the usage-kind match above), `router.rs`,
  `retry.rs` or `pricing.rs`.
- A fake in `fake.rs` (scripted verdict queue with a call counter, in the style of
  `FakeVisionClassifier`).

### 14.4 `ubiqx-engine`

- `AiPorts` (`deps.rs:101-121`) gets an optional detector; update the seven struct literals listed in
  section 13 (or switch them to `..Default::default()`).
- A loop of its own in `engine.rs`, modelled on `focus_loop` (`engine.rs:218-234`: tick, skip the
  first tick, `spawn_blocking` for platform calls) with a `LoopConfig` field so tests can shorten or
  disable it. The focus guard already looks at the foreground window every 2 s (`focus_every`).
- Gates to call, all existing: `state.remote_allowed()`, `classify::budget_allows(state)?`,
  `state.settings().is_private(now)`, and whichever of `VisionPolicy::allows`, `blocked_apps` and the
  incognito rule the design decides to honour.
- `handle_remote_failure` is a private fn that takes blocks (`classify.rs:237`). Reusing it needs
  `pub(crate)` and an empty slice, and means a nofap failure changes the shared `AiHealth`.
- Hourly cap: `ScreenshotRepo::count_sent_since` counts `screenshots` rows. A check that never writes
  a row needs its own counter (in memory, `kv`, or a new table).
- Persistence of outcomes: a new table through a new repo trait added to `Repos` (`deps.rs:49-98`) and
  `Repos::from_store`'s bound list, or reuse of `interventions` (`FocusRepo::insert_intervention`).
  `MaintenanceRepo::wipe_user_data` has a fixed list of tables (`WIPE` in
  `crates/ubiqx-storage/src/repos/maintenance.rs`) that a new table must join.
- User-facing output channels that exist: `nudges::emit(state, kind, title, message, os_notification)`
  (`nudges.rs:16-40`), `InterventionPresenter::show(&Intervention)` (`ports.rs:134-136`),
  `Enforcer::{close_active_tab, blank_active_tab, quit_app, hide_others}` (`ports.rs:118-130`).

### 14.5 `ubiqx-app` and shells

- `build_ai` (`ubiqx-app/src/lib.rs:454-510`): construct the detector with the same
  `llm: Arc<dyn LlmClient>` (the `RoutingLlmClient`) in the `Remote` arm and the fake in the `Fake`
  arm.
- Tauri commands and TypeScript IPC (`apps/desktop/src-tauri/src/commands.rs`,
  `apps/desktop/src/lib/ipc.ts`, `types.ts`, `mock.ts`) for settings, status and the audit view.
- `App::start` always starts the whole engine (sampler, tracker, classify, reports, nudges, retention,
  update, focus, license loops; `engine.rs:75-107`). There is no switch to run only some loops; a
  standalone product either adds such switches to `LoopConfig` or composes its own smaller runtime
  from the same crates.

### 14.6 `services/ubi-api`

No change for the wire format. To decide: whether these calls share the US$ 6 plan budget, and the
privacy statement (images cross the operator's server in transit and reach Anthropic under the
operator's key). Add an image passthrough test; consider an explicit `DefaultBodyLimit`.

---

## 15. Surprising, fragile or missing

### 15.1 A missing window id silently becomes a full-display capture

`tracker.rs:116` always passes `bounds = None`, so `capture_for_block` maps `window_id = None` to
`CaptureTarget::PrimaryDisplay` (`screenshots.rs:71-78`). The comment directly below says a
whole-display grab "would include other apps' windows … which the 'active window only' promise
forbids". macOS can report a foreground app without a focused window
(`window_id: win.map(|w| w.id)`, `macos/frontmost.rs:174`). README and ARCHITECTURE promise "só da
janela ativa". On Linux, a failed window read also falls back to the root window region
(`linux/capture.rs:36-41`), and on Windows to a cropped display capture.

### 15.2 "Apagados após a classificação / após uso" is only true for the one image vision consumed

`screenshots::unlink` is called in exactly one place, after a successful vision call
(`classify.rs:493-495`). Screenshots of blocks classified by rules, memory or the text model, earlier
screenshots of the same block, and screenshots whose vision call failed all stay on disk for
`screenshot_retention_hours` (24 h by default). `docs/ARCHITECTURE.md:132-133, 177` and the settings
hint `screenshots.keep.hint` ("a imagem é apagada logo após o uso") describe more than the code does.

### 15.3 The hourly cap counts by capture time and only successes

`COUNT_SENT_SINCE = "… WHERE sent_to_ai = 1 AND at >= ?1"` uses `at`, the capture time
(`crates/ubiqx-storage/src/repos/screenshots.rs:40`). A screenshot taken two hours ago and sent now
does not count; neither do images whose call failed or was refused.

### 15.4 "Com imagem" in the audit dialog does not mean an image was sent

See section 11. `docs/ARCHITECTURE.md:175-176` says only what the UI shows under "Dados enviados à IA"
leaves the machine, "e, quando permitido, o print da janela ativa"; the UI cannot currently tell the
user which screenshots left.

### 15.5 Blocked apps: UI says "nem título, nem tempo", code keeps time and app name

`settings.ts:142`: "Não são registrados de forma alguma: nem título, nem tempo." The segmenter keeps
the block with the real `app_name` / `app_id` and `[privado]` as the title (`segmenter.rs:130-150`);
`docs/ARCHITECTURE.md:128-129` describes the code correctly.

### 15.6 A refused or failed vision call leaves no trace of the image having left

On `Err`, `mark_sent` is not called and the file is not deleted (`classify.rs:516-525`), although the
image was transmitted (certainly for a refusal, which is also billed). The same code path is the one
an explicit image is most likely to take.

### 15.7 No per-request retry control

`LlmRequest.timeout` is the only per-call knob. With the defaults a hanging network costs up to
4 × timeout plus backoff before the caller gets an error (with `LONG_TIMEOUT`, more than eight
minutes).

### 15.8 Shared health and budget

Described in sections 1 and 8. Also: `budget_allows` is not re-checked inside the vision loop, and for
own-key users with `ai_monthly_budget_usd = 0` there is no cap at all.

### 15.9 Model-policy heuristics assume `claude-haiku-4-5` and `claude-sonnet-5`

`is_haiku(model)` (`client.rs:313-315`) drives `thinking_disabled_for` and `prompt_cache_for`. For any
non-Haiku id the client sends `thinking: {"type": "disabled"}`. According to the Claude API reference
bundled with this session (model table cached 2026-11-03, not part of the repository), that field is
rejected with HTTP 400 by `claude-opus-5-5`, `claude-sonnet-5-5` and the Fable models, and
`claude-haiku-5-5` thinks by default when the field is omitted, which a `max_tokens` of a few dozen
tokens would not survive. The shipped defaults are unaffected; a user-edited vision model id is. The
price table also charges any `*haiku*` id at the Haiku 4.5 rate.

### 15.10 Usage timestamps use the wall clock

`AiUsage.at = Utc::now()` in both clients (`client.rs:785`, `openai.rs:733`), while the budget window
is computed from the injected clock (`month_range(state.now())`). Engine tests with `FixedClock` and a
real client would disagree; with `MemoryUsageRepo` or the fakes this does not show.

### 15.11 Trace logging contains the image

Section 10.6. `UBIQX_LOG=trace` writes every base64 screenshot to stderr.

### 15.12 Capture blocks the tracker thread

`capture_for_block` runs inside `handle_sample`; `xcap::Window::all()` enumerates every window on each
capture (macOS and Windows).

### 15.13 Smaller documentation drifts

- `docs/ARCHITECTURE.md:28` lists `active-win-pos-rs` for the foreground window; the crate is not a
  dependency (AppKit / CoreGraphics / Win32 / x11rb are used directly).
- `docs/WINDOWS-LINUX.md:99` lists "ScreenCaptureKit/CG" for macOS; the code uses `xcap`'s
  `CGWindowListCreateImage` and names ScreenCaptureKit only as an upgrade path
  (`macos/capture.rs:1-4`).
- `EncodedImage.mime` documents `image/webp`, `image/jpeg` or `image/png`; only JPEG is produced, and
  `screenshots::load` hard-codes `image/jpeg`.

### 15.14 Missing for nofap, as far as this area goes

- No real-time vision path, no capture outside a block, no in-memory-only capture helper in the engine
  (the pieces exist in the platform layer).
- No yes/no vision port, prompt or schema; no handling of refusal as a signal.
- No local prefilter of any kind (image similarity, change detection, on-device classifier), so every
  check is a paid network call.
- No audit record for AI calls that are not tied to a closed block.
- No image-level redaction.
- No proxy test with an image; no explicit body limit on the proxy.

---

## 16. File index

| Area | File |
|---|---|
| Screenshot policy, capture, load, unlink, retention | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/screenshots.rs` |
| Capture trigger | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/tracker.rs`, `sampler.rs` |
| Classification worker, vision loop, budget, failure mapping | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/classify.rs` |
| Loops and cadences | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/engine.rs` |
| State, `remote_allowed`, `key_health`, `apply_settings` | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/state.rs` |
| `AiPorts`, `Repos`, `EngineDeps` | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/deps.rs` |
| Audit read model, screenshot read | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/service.rs` |
| Handle (private mode, keys, wipe) | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/handle.rs` |
| Managed usage, license gate | `/home/user/ubiquitous-engine/crates/ubiqx-engine/src/license.rs` |
| Ports (`ScreenCapturer`, `VisionClassifier`, `AiUsageKind`, repos) | `/home/user/ubiquitous-engine/crates/ubiqx-core/src/ports.rs` |
| `Settings`, `VisionPolicy`, `AiProvider`, `AiModels`, `Screenshot`, `EngineEvent`, `AiHealth` | `/home/user/ubiquitous-engine/crates/ubiqx-core/src/model.rs` |
| Redaction | `/home/user/ubiquitous-engine/crates/ubiqx-core/src/redact.rs` |
| Blocked apps / incognito / private mode | `/home/user/ubiquitous-engine/crates/ubiqx-core/src/segmenter.rs`, `normalize.rs` |
| Errors | `/home/user/ubiquitous-engine/crates/ubiqx-core/src/error.rs` |
| License constants, proxy base, aliases | `/home/user/ubiquitous-engine/crates/ubiqx-core/src/license.rs` |
| Capturers | `/home/user/ubiquitous-engine/crates/ubiqx-platform/src/{macos,windows,linux}/capture.rs`, `image_util.rs`, `mock.rs` |
| Vision classifier | `/home/user/ubiquitous-engine/crates/ubiqx-ai/src/vision.rs` |
| Anthropic / Ubi client, neutral types | `/home/user/ubiquitous-engine/crates/ubiqx-ai/src/client.rs` |
| OpenAI / xAI client | `/home/user/ubiquitous-engine/crates/ubiqx-ai/src/openai.rs` |
| Router, retry, pricing, prompts, fakes, license endpoint, advisor | `/home/user/ubiquitous-engine/crates/ubiqx-ai/src/{router,retry,pricing,prompts,fake,ubi,advisor}.rs` |
| Composition root | `/home/user/ubiquitous-engine/crates/ubiqx-app/src/lib.rs` |
| Proxy | `/home/user/ubiquitous-engine/services/ubi-api/src/{routes,config,db,sse,lib}.rs` |
| Storage | `/home/user/ubiquitous-engine/crates/ubiqx-storage/src/migrations.rs`, `repos/screenshots.rs`, `repos/maintenance.rs` |
| UI settings and audit dialog | `/home/user/ubiquitous-engine/apps/desktop/src/pages/Settings.tsx`, `src/i18n/messages/settings.ts`, `src/lib/types.ts` |
| Engine test harness | `/home/user/ubiquitous-engine/crates/ubiqx-engine/tests/pipeline.rs` |
