# ubi-api — the Ubi proxy

The service behind the **"ubiqX Mensal, com a IA do Ubi"** plan. Desktop apps that hold a
`monthly_managed` license send their AI calls here with the **license key** instead of a
vendor API key. The proxy:

1. verifies the key **offline** (Ed25519 signature, expiry, plan, revocation list);
2. maps the public model aliases `ubi-fast` / `ubi-smart` to real vendor models (any other
   model id is a `400`);
3. enforces a per-subscriber **monthly budget** (`402 budget_exhausted` once reached);
4. forwards the request unchanged to the vendor (Anthropic Messages API, non-streaming and
   SSE alike) with the operator's key;
5. prices the response's `usage` block and records it in a SQLite ledger, returning the cost
   in `x-ubiqx-cost-usd`.

It is a **standalone Cargo project**: not a member of the root workspace, so desktop builds
never compile the server stack. Every cargo command takes `--manifest-path`:

```sh
cargo run   --manifest-path services/ubi-api/Cargo.toml                  # the server
cargo test  --manifest-path services/ubi-api/Cargo.toml                  # unit + e2e (mock vendor)
cargo run   --manifest-path services/ubi-api/Cargo.toml --bin ubi-license -- keygen
```

The key format and verification live in `crates/ubiqx-core/src/license.rs` (shared with the
app); this project only adds issuing, the ledger and the HTTP surface. The product-level
description (plans, enforcement, payment flow) is in `docs/LICENSING.md` (Portuguese).

## Environment

| Variable | Default | Meaning |
|---|---|---|
| `UBI_API_LISTEN` | `0.0.0.0:8080` | Bind address. |
| `UBI_LICENSE_PUBKEY_HEX` | the key compiled into `ubiqx-core` | Ed25519 public key license keys must verify against. Set it only to rotate. |
| `UBI_LICENSE_PRIVKEY_HEX` | — | Ed25519 private key used by the payment webhook to **issue** keys. Without it `subscription.created/renewed` answer `503`. Secret. |
| `UBI_ADMIN_TOKEN` | — | Bearer token for every `/admin` route, and what you log into `/panel` with. Without it they answer `503` and the panel answers `404`. Secret. |
| `UBI_WEBHOOK_SECRET` | — | HMAC-SHA256 secret of `/admin/webhooks/generic`. Without it the webhook answers `503`. Secret. |
| `UBI_VENDOR` | `anthropic` | Upstream vendor (only `anthropic` today). |
| `ANTHROPIC_API_KEY` | — | The operator's vendor key. Required. Secret. |
| `UBI_VENDOR_BASE_URL` | `https://api.anthropic.com` | Where `/v1/messages` is forwarded (tests point it at a mock). |
| `UBI_MONTHLY_BUDGET_USD` | `6.0` | Budget per managed subscriber per calendar month (UTC). |
| `UBI_DB` | `ubi-api.sqlite` | SQLite ledger path (`:memory:` for throw-away runs). |
| `UBI_MODEL_FAST` | `claude-haiku-4-5-20251001` | What `ubi-fast` maps to (classification, vision). |
| `UBI_MODEL_SMART` | `claude-sonnet-5` | What `ubi-smart` maps to (reports, advice). |
| `UBI_PRICES_JSON` | haiku 4.5 = 1/5, sonnet 5 = 2/10 | Extra or overriding prices, USD per million tokens: `{"claude-sonnet-5":{"input_per_mtok":2.0,"output_per_mtok":10.0}}`. Cache reads are billed at 10 % of input, cache writes at 125 %. Unknown ids are billed at the most expensive configured price. |
| `RUST_LOG` | `info` | Log filter. |

## Routes

### Public

| Route | Auth | Behaviour |
|---|---|---|
| `GET /healthz` | none | `{"ok":true,"service":"ubi-api","version":…}` |
| `GET /admin/models` | `Authorization: Bearer $UBI_ADMIN_TOKEN` | `{"vendor":…,"models":{"ubi-fast":…,"ubi-smart":…}}` |
| `GET /v1/license/status` | `x-api-key: <license>` | `200 {plan, expires_at, days_left, key_hint, month, spent_usd, budget_usd, state}` for any genuine key; `state` is `valid`, `expired` or `revoked`. `401 {state:"invalid", …}` when the key does not verify. The app merges `month/spent_usd/budget_usd` into its local verdict. `budget_usd` is `0` for annual keys. |
| `POST /v1/messages` | `x-api-key: <license>` (or `Authorization: Bearer`) | Anthropic Messages API passthrough, see below. The optional `x-ubiqx-plan` header must be `monthly_managed`. |

`POST /v1/messages` in order:

1. `401 authentication_error` — no key; `401 license_invalid` — bad signature / malformed;
2. `403 license_expired` / `403 license_revoked` / `403 plan_not_managed` (an annual key);
3. `400 invalid_request_error` — body not a JSON object or `model` not `ubi-fast` / `ubi-smart`;
4. `402 budget_exhausted` — the month's spend already reached the budget (the body also carries
   `month`, `spent_usd`, `budget_usd`);
5. the request is forwarded with `x-api-key` = the vendor key, `anthropic-version` (the client's or
   `2023-06-01`) and `anthropic-beta` if the client sent one. Vendor errors are passed through
   with their status and body; `502 upstream_error` when the vendor is unreachable.

Successful responses carry `x-ubiqx-cost-usd` (this call), `x-ubiqx-spent-usd` (the month so
far) and `x-ubiqx-budget-usd`. **Streaming** (`"stream": true`) is a byte-for-byte SSE
passthrough; the cost is only known when the stream ends, so the usage is recorded then and
`x-ubiqx-cost-usd` is absent (`x-ubiqx-spent-usd` is the total *before* the call).

Every error body is Anthropic-shaped so the app's client reads it unchanged:

```json
{"type":"error","error":{"type":"budget_exhausted","message":"…"},"month":"2026-09","spent_usd":6.1,"budget_usd":6.0}
```

### Admin (`Authorization: Bearer $UBI_ADMIN_TOKEN`)

| Route | Body / query | Result |
|---|---|---|
| `GET /admin/subscribers?q=` | `q` matches the subscriber id, the platform's id, the e-mail hash or the key hint | `{month, count, subscribers:[{sub, plan, email_hash, external_id, key_hint, issued_at, expires_at, events, revoked_at, revoked_reason, expired, month_cost_usd}]}` |
| `GET /admin/subscribers/{sub}` | — | `{subscriber, expired, licenses:[…every key ever issued…], usage:[{month, requests, cost_usd}], budget_usd}` |
| `GET /admin/stats` | — | `{stats:{subscribers, active, expired, revoked, expiring_soon, annual, monthly}, month, month_cost_usd, month_requests, budget_usd}` |
| `POST /admin/licenses/issue` | `{"plan":"monthly_managed","email":"…","months":6,"external_id":null}` | the same shape the webhook returns, key included — courtesy, support, a sale from outside the platform |
| `POST /admin/licenses/revoke` | `{"sub":"sub_…","reason":"chargeback"}` | `{"sub":…,"revoked":true}` — takes effect on the next call. |
| `POST /admin/licenses/unrevoke` | `{"sub":"sub_…"}` | `{"sub":…,"revoked":false,"lifted":true}` |
| `GET /admin/usage?month=YYYY-MM` | month defaults to the current one | `{month, budget_usd, total_cost_usd, subscribers:[{sub, requests, input_tokens, output_tokens, cost_usd}]}` |

`expiring_soon` reads against the plan, not the calendar: seven days for a monthly licence,
thirty for an annual one. A month is the whole life of a monthly key, so a thirty-day window
would flag every one of them, for ever.

### Panel — `GET /panel`

The operator's page, served by this service: one HTML file, no build step, no CDN, nothing
fetched from the network. It lists subscribers with their state and this month's spend, opens
one to show every key ever issued for it and the spend month by month, and carries the levers —
issue by hand, revoke, put back. It holds no credential: you paste `UBI_ADMIN_TOKEN` into it,
it keeps it in `sessionStorage` for the tab, and every figure comes from the `/admin` routes
above, which still check the bearer. Without `UBI_ADMIN_TOKEN` configured the route answers
`404`: a service with no way in has no panel.

Serving it from here rather than from the site is deliberate — one deploy, one secret store, no
CORS, and the private key never leaves the process that already holds it.

### Payment webhook — `POST /admin/webhooks/generic`

The integration point for **Mercado Pago, Stripe, Hotmart** or any platform that can call a
URL when a subscription changes (directly, or through a small automation such as Make/Zapier/n8n
that reshapes the platform's event into this body). It is authenticated by its **signature**,
not by the admin bearer: the header `x-ubi-signature` is the hex HMAC-SHA256 of the **raw
request body** with `UBI_WEBHOOK_SECRET` (an optional `sha256=` prefix is accepted).

```json
{"event":"subscription.created","plan":"monthly_managed","email":"cliente@exemplo.com","months":1,"external_id":"mp_sub_123"}
```

| `event` | Effect | Response |
|---|---|---|
| `subscription.created` | Issues a key signed with `UBI_LICENSE_PRIVKEY_HEX`, valid for `months` calendar months (default 12 for `annual_own_key`, 1 for `monthly_managed`). Records it in the ledger. | `{"sub","plan","email_hash","months","expires_at","key_hint","key"}` — **the automation e-mails `key` to the subscriber**; the proxy never sees the e-mail again (only its SHA-256 goes into the key). |
| `subscription.renewed` | Same as created, same `sub` (derived from `external_id`, or from the e-mail when absent), and lifts a revocation. | same |
| `subscription.cancelled` | Revokes the `sub` derived from `external_id` / `email`. Existing keys stop working on the proxy at once. | `{"sub","revoked":true}` |

Signing example (shell):

```sh
BODY='{"event":"subscription.created","plan":"monthly_managed","email":"cliente@exemplo.com","external_id":"mp_sub_123"}'
SIG=$(printf '%s' "$BODY" | openssl dgst -sha256 -hmac "$UBI_WEBHOOK_SECRET" | awk '{print $NF}')
curl -sS https://api.ubiqx.ai/admin/webhooks/generic -H "content-type: application/json" -H "x-ubi-signature: $SIG" -d "$BODY"
```

Note: `annual_own_key` keys are issued through the same webhook (so one payment flow serves both
plans) but are **not** accepted on `/v1/messages`: that plan uses the customer's own vendor key.

## `ubi-license` CLI

```sh
ubi-license keygen
#   private key (hex, keep secret): 75…
#   public key  (hex, commit):      de…
ubi-license issue --plan monthly_managed --email cliente@exemplo.com --months 1 --privkey HEX   # prints the key
ubi-license issue --plan annual_own_key  --email cliente@exemplo.com --months 12 --privkey HEX
ubi-license verify --key UBIQX-… [--pubkey HEX]       # defaults to the key compiled into ubiqx-core
```

`issue` also takes `--sub` (explicit subscriber id), `--external-id` (derive a stable id from
the payment platform's id) and `--issued-at RFC3339` (backdate; handy to mint an expired key
for tests). `--privkey` / `--pubkey` can come from `UBI_LICENSE_PRIVKEY_HEX` /
`UBI_LICENSE_PUBKEY_HEX`.

**Keys**: the *private* key is printed once by `keygen`; store it as a secret (Fly secrets, 1Password…)
and never commit it. The *public* key goes into `crates/ubiqx-core/src/license.rs`
(`UBIQX_LICENSE_PUBKEY_HEX`) — every build of the app trusts that key only — and, if you want the
proxy to trust a different one, into `UBI_LICENSE_PUBKEY_HEX`.

## Deployment

### Docker (build from the repository root)

```sh
docker build -f services/ubi-api/Dockerfile -t ubi-api .
docker run --rm -p 8080:8080 -v ubi_data:/data \
  -e ANTHROPIC_API_KEY=sk-ant-… -e UBI_ADMIN_TOKEN=… -e UBI_WEBHOOK_SECRET=… -e UBI_LICENSE_PRIVKEY_HEX=… \
  ubi-api
curl -s localhost:8080/healthz
```

The image is distroless (`gcr.io/distroless/cc-debian12`), runs as non-root and keeps the SQLite
ledger in `/data` (`UBI_DB=/data/ubi-api.sqlite`); mount a volume there.

### Fly.io

`fly.toml` is a working example (one shared-cpu machine, a 1 GB volume for the ledger, health
check on `/healthz`). From the repository root:

```sh
fly launch --no-deploy --copy-config --name ubi-api
fly volumes create ubi_data --size 1 --region gru
fly secrets set ANTHROPIC_API_KEY=… UBI_ADMIN_TOKEN=… UBI_WEBHOOK_SECRET=… UBI_LICENSE_PRIVKEY_HEX=…
fly deploy
fly certs add api.ubiqx.ai
```

Run it on a single machine: the ledger is one SQLite file. Back the volume up (`fly volumes
snapshots list`) — it holds the revocation list and the issued-key log.

### The app side

The desktop app reads the proxy's base URL from `UBIQX_API_BASE` **at build time** (default
`https://api.ubiqx.ai`, see `ubiqx_core::license::UBIQX_API_BASE_DEFAULT`). To point a build at a
staging proxy:

```sh
UBIQX_API_BASE=https://ubi-api-staging.fly.dev pnpm tauri build
```

The app sends `x-api-key: <license key>` and `x-ubiqx-plan: monthly_managed` with model ids
`ubi-fast` / `ubi-smart`, and reads `x-ubiqx-cost-usd` into its usage ledger like any vendor.

## Tests

`cargo test --manifest-path services/ubi-api/Cargo.toml` runs:

* unit tests for the price table, the ledger, SSE usage parsing, the webhook signature and
  subscriber ids;
* `tests/license_cli.rs` — key pair → issue → verify round trip through the real `ubi-license`
  binary and `ubiqx_core::license`, including an expired key and the committed dev public key;
* `tests/proxy.rs` — the whole HTTP surface against an **in-process mock vendor** on
  `127.0.0.1`: alias mapping, cost header and ledger, configured prices, unknown models, the
  budget cut-off, SSE passthrough with deferred accounting, vendor error passthrough, every
  license verdict, `/v1/license/status`, admin auth, revocation, and the webhook (bad
  signature, created → cancelled → renewed).

No test touches the network.
