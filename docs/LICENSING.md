# ubiqX — Licenciamento

> Como os planos, as chaves de licença, a aplicação (soft/hard), o proxy do Ubi e a
> integração com a plataforma de pagamento funcionam. Código: `crates/ubiqx-core/src/license.rs`
> (formato e verificação, compartilhados) e `services/ubi-api/` (proxy, emissão, CLI).

## 1. Planos

| Plano | Id (`plan`) | Preço | IA |
|---|---|---|---|
| **ubiqX Anual, com a sua IA** | `annual_own_key` | R$ 197/ano ou 10x de R$ 25 | O usuário traz a própria chave de API (Anthropic, OpenAI ou xAI). A chave de licença só destrava o app. |
| **ubiqX Mensal, com a IA do Ubi** | `monthly_managed` | R$ 49/mês | Sem chave de API: o app fala com o **proxy do Ubi** (`services/ubi-api`) usando a chave de licença. O proxy paga o provedor e limita o gasto mensal por assinante (`UBI_MONTHLY_BUDGET_USD`, padrão US$ 6). |

Os preços vivem no site (`site/`) e nos textos da interface; nunca no código de licença.
Não há trial nem desconto embutido: uma chave é válida ou não.

O que **sempre funciona**, com ou sem licença: rastreamento, linha do tempo, categorização
manual, regras, relatórios já gerados. O que depende da licença (só no modo *hard*, ver §4):
classificação por IA, visão, relatórios e recomendações.

## 2. Chaves de licença

Uma chave é um texto que o usuário cola em *Configurações › Licença* (ou na etapa
"Escolha sua IA" do onboarding):

```
UBIQX-<base32(claims em JSON canônico)>-<base32(assinatura Ed25519, 64 bytes)>
```

* base32 RFC 4648 sem padding, maiúsculas — sobrevive a e-mail, chat e digitação; o app
  aceita minúsculas e quebras de linha (normaliza antes de verificar);
* os *claims* são JSON canônico (chaves em ordem alfabética, sem espaços): sempre os mesmos
  bytes para os mesmos dados, e a assinatura cobre exatamente esses bytes;
* a assinatura é Ed25519 com a **chave privada do ubiqX**; o app embute só a **chave pública**
  (`UBIQX_LICENSE_PUBKEY_HEX` em `license.rs`).

Claims (`v = 1`):

| Campo | Significado |
|---|---|
| `plan` | `annual_own_key` \| `monthly_managed` |
| `sub` | id opaco do assinante (estável entre renovações da mesma assinatura) |
| `email_hash` | SHA-256 hex do e-mail em minúsculas — o e-mail em si nunca viaja |
| `issued_at`, `expires_at` | segundos Unix; a chave vale enquanto `agora < expires_at` |
| `seats` | 1 |

A verificação é **100 % offline**: assinatura + validade. Nenhuma chamada é necessária para
saber se uma chave é genuína. O plano mensal, além disso, consulta
`GET {UBIQX_API_BASE}/v1/license/status` para mostrar o consumo do mês (`managed_usage`); se a
rede falhar, o veredito local prevalece e `managed_usage` fica `null`.

O que a interface mostra (`LicenseStatus`, snake_case):

```json
{ "state": "valid", "plan": "monthly_managed", "expires_at": "2027-09-18T00:00:00+00:00",
  "days_left": 365, "key_hint": "PIBY", "enforcement": "soft",
  "managed_usage": { "month": "2026-09", "spent_usd": 1.25, "budget_usd": 6.0 } }
```

`state` ∈ `unlicensed` | `valid` | `expired` | `invalid`. A chave fica no cofre de segredos do
sistema (Keychain) sob `ubiqx.license`, nunca no JSON de configurações.

### Par de chaves

```sh
cargo run --manifest-path services/ubi-api/Cargo.toml --bin ubi-license -- keygen
# private key (hex, keep secret): …   → guarde como segredo (Fly secrets, 1Password); NUNCA no repositório
# public key  (hex, commit):      …   → UBIQX_LICENSE_PUBKEY_HEX em crates/ubiqx-core/src/license.rs
```

A chave pública comprometida no repositório é a do par de **desenvolvimento**
(`ba31f587…2282`). Para produção: gere um par novo, troque a constante, publique um build do
app e configure o proxy com a chave privada (`UBI_LICENSE_PRIVKEY_HEX`). Trocar a chave pública
invalida todas as chaves já emitidas — planeje a rotação junto com uma reemissão.

### Emitir e verificar à mão

```sh
ubi-license issue --plan annual_own_key  --email cliente@exemplo.com --months 12 --privkey HEX
ubi-license issue --plan monthly_managed --email cliente@exemplo.com --months 1  --privkey HEX
ubi-license verify --key UBIQX-…           # usa a chave pública embutida; --pubkey HEX para outra
```

## 3. O proxy do Ubi (`services/ubi-api`)

Projeto Cargo **independente** (não é membro do workspace: o build do desktop nunca o compila).

```sh
cargo run  --manifest-path services/ubi-api/Cargo.toml
cargo test --manifest-path services/ubi-api/Cargo.toml
```

Fluxo de `POST /v1/messages` (formato Anthropic Messages, sem alterações no corpo):

1. `x-api-key` = chave de licença → verifica assinatura, validade, plano `monthly_managed` e
   lista de revogação (`401 license_invalid`, `403 license_expired | license_revoked | plan_not_managed`);
2. `model` deve ser `ubi-fast` (classificação/visão) ou `ubi-smart` (relatórios/recomendações);
   o proxy troca pelo modelo real (`UBI_MODEL_FAST` / `UBI_MODEL_SMART`); qualquer outro id → `400`;
3. gasto do mês ≥ orçamento → `402 {"error":{"type":"budget_exhausted"}}` (o mês vira em UTC);
4. encaminha ao provedor com a chave do operador (`ANTHROPIC_API_KEY`), streaming (SSE) ou não;
5. calcula o custo pela tabela de preços (`UBI_PRICES_JSON`) a partir do bloco `usage` da
   resposta, grava no SQLite (`usage(sub, month, model, input_tokens, output_tokens, cost_usd, at)`)
   e devolve `x-ubiqx-cost-usd` (no streaming o custo só é conhecido no fim: é gravado, mas o
   cabeçalho não existe). O app registra esse custo no seu próprio livro-razão como faz com
   qualquer provedor.

Erros seguem o formato da Anthropic (`{"type":"error","error":{"type":…,"message":…}}`).

Variáveis de ambiente, rotas de administração e detalhes: `services/ubi-api/README.md`.

### Publicar

* **Docker** (a partir da raiz do repositório): `docker build -f services/ubi-api/Dockerfile -t ubi-api .`
  Imagem distroless, usuário não-root, banco em `/data` (monte um volume).
* **Fly.io**: `services/ubi-api/fly.toml` é um exemplo completo (uma máquina, volume de 1 GB para o
  SQLite, health check em `/healthz`). Segredos via `fly secrets set ANTHROPIC_API_KEY=… UBI_ADMIN_TOKEN=…
  UBI_WEBHOOK_SECRET=… UBI_LICENSE_PRIVKEY_HEX=…`.
* **Lado do app**: a URL base vem de `UBIQX_API_BASE` **em tempo de build** (padrão
  `https://api.ubiqx.ai`). Ex.: `UBIQX_API_BASE=https://ubi-api-staging.fly.dev pnpm tauri build`.

Rode uma única instância: o livro-razão é um arquivo SQLite. Faça backup do volume — ele guarda
a lista de revogação e o histórico de chaves emitidas.

## 4. Aplicação: *soft* × *hard*

`LICENSE_ENFORCEMENT` é uma **constante de compilação** em `license.rs`:

| Modo | Sem licença válida… |
|---|---|
| `Soft` (**atual**) | Tudo funciona. Uma vez a cada 7 dias o app mostra uma linha discreta ("O ubiqX está sem licença. Planos: anual R$ 197 (ou 10x de R$ 25) com a sua IA, ou R$ 49/mês com a IA do Ubi.") com "Ver planos" e "Já tenho uma chave". No onboarding dá para "Continuar sem licença por enquanto". |
| `Hard` | Os recursos de IA (classificação, visão, relatórios, recomendações) ficam bloqueados com um diálogo explicativo até uma chave válida ser informada. Rastreamento, linha do tempo e categorização manual continuam funcionando. |

Independentemente do modo, o provedor **IA do Ubi** só pode ser selecionado com uma licença
`monthly_managed` válida (`IpcError` `license_required`); com a licença expirada ou removida, o
app volta a pedir um provedor com chave própria.

Para mudar de modo: altere a constante, recompile, publique. Não há chave remota nem flag.

## 5. Conectar a plataforma de pagamento

O ponto de integração é o webhook genérico do proxy — `POST /admin/webhooks/generic` — pensado
para Mercado Pago, Stripe, Hotmart ou qualquer plataforma que chame uma URL quando uma
assinatura muda (diretamente ou por uma automação — Make, Zapier, n8n — que traduza o evento da
plataforma para este corpo):

```json
{ "event": "subscription.created", "plan": "monthly_managed",
  "email": "cliente@exemplo.com", "months": 1, "external_id": "mp_sub_123" }
```

Cabeçalho `x-ubi-signature` = HMAC-SHA256 (hex) do **corpo bruto** com `UBI_WEBHOOK_SECRET`.

| Evento | O proxy… | Devolve |
|---|---|---|
| `subscription.created` | emite a chave (assinada com `UBI_LICENSE_PRIVKEY_HEX`, válida por `months` meses; padrão 12 no anual, 1 no mensal) e a registra | `{ "sub", "plan", "expires_at", "key_hint", "key", … }` — **a automação envia `key` por e-mail ao cliente** |
| `subscription.renewed` | emite uma chave nova para o **mesmo** `sub` (derivado de `external_id`, ou do e-mail) e remove a revogação | idem |
| `subscription.cancelled` | revoga o `sub`: as chaves existentes deixam de funcionar no proxy imediatamente | `{ "sub", "revoked": true }` |

Passo a passo (exemplo com Mercado Pago):

1. Crie os dois produtos/assinaturas na plataforma com os preços do site.
2. Configure a notificação de assinatura da plataforma para chamar a automação; nela, monte o
   corpo acima (`plan` conforme o produto, `email` do comprador, `external_id` = id da assinatura
   na plataforma), assine com o segredo e chame o webhook.
3. Envie por e-mail ao cliente o campo `key` da resposta com o link para
   *Configurações › Licença* do app.
4. Na renovação, repita com `subscription.renewed`; no cancelamento/estorno, `subscription.cancelled`.
5. Para casos manuais (cortesia, suporte): `ubi-license issue …` no seu computador com a chave
   privada, ou `POST /admin/licenses/revoke {"sub"}` com o bearer `UBI_ADMIN_TOKEN`.
6. Acompanhe o consumo com `GET /admin/usage?month=YYYY-MM`.

Observações:

* o e-mail nunca é armazenado no proxy nem na chave — apenas o SHA-256;
* chaves do plano anual também são emitidas por esse webhook, mas não são aceitas em
  `/v1/messages` (o anual usa a chave de API do próprio cliente);
* a revogação vale só para o proxy (plano mensal). Uma chave anual é offline por natureza e
  expira sozinha em `expires_at`; a "revogação" de uma anual é não renová-la.

## 6. Chaves de desenvolvimento

O par de desenvolvimento tem a chave pública `ba31f58756b2ea408d3ea7fa8116ca3a802aff3986919492f482ea7bf73e2282`
comprometida em `license.rs`; a chave privada fica com o responsável pelo produto (fora do
repositório). A chave de exemplo assinada com esse par (`sub_dev_managed_0001`, mensal, válida até
2036-09-18) está nos testes do serviço (`services/ubi-api/tests/license_cli.rs`); emita outras
com `ubi-license issue`. O mock do frontend (`apps/desktop/src/lib/mock.ts`) não usa essas
chaves: ele monta as suas (`__mock.sampleLicenseKeys`, assinatura fictícia, validade relativa
ao carregamento) porque nada ali verifica assinatura.
