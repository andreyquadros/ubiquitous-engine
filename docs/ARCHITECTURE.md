# ubiqX — Arquitetura

> Rastreamento inteligente de atividade para macOS, com IA nativa, mascote (UBI) e relatórios por categoria.
> Concorrente do Rize (rize.io) com classificação, relatórios e recomendações geradas por IA.

## 1. Objetivos

| # | Objetivo | Como |
|---|----------|------|
| O1 | Registrar tudo que o usuário faz no macOS sem incomodar | Daemon em segundo plano (ícone na barra de menus), amostragem leve de app/janela/URL + screenshots esparsos |
| O2 | Enquadrar automaticamente cada atividade em categorias definidas pelo usuário (IFRO, Incubadora, Cidades Inteligentes…) | Pipeline de classificação em cadeia: regras → memória de correções → LLM (texto) → LLM (visão) |
| O3 | Aprender com correções | Cada correção vira regra sugerida + exemplo few-shot no prompt; similaridade com blocos já corrigidos |
| O4 | Relatórios diários por categoria em horário configurável | Agendador interno; LLM redige o relatório a partir dos blocos classificados |
| O5 | Recomendações de produtividade e alertas de improdutividade | Motor de insights local (heurísticas) + recomendações semanais por LLM; UBI comunica |
| O6 | Custo de API mínimo | Só texto vai ao LLM por padrão; imagens só em blocos ambíguos, com limite/hora; modelo barato para classificação e modelo melhor só para relatórios, em qualquer provedor |
| O6b | Provedor à escolha | Anthropic Claude, OpenAI ou xAI Grok, cada um com a própria chave; troca em tempo de execução sem reiniciar |
| O7 | Privacidade | Tudo local (SQLite + imagens no disco do usuário); lista de apps bloqueados; modo privado; retenção configurável; chave no Keychain |
| O8 | Portabilidade (macOS, Windows, Linux) | Toda dependência de SO isolada atrás de traits (`ActivitySource`, `ScreenCapturer`, `IdleDetector`); um adaptador por SO em `ubiqx-platform` |

## 2. Stack

| Camada | Escolha | Motivo |
|--------|---------|--------|
| Núcleo / daemon | **Rust** (edition 2021, MSRV 1.85) | Performance, baixo consumo em background, segurança de memória, um único binário |
| Desktop shell | **Tauri 2** | App nativo macOS pequeno (~10 MB), tray/menubar, notificações, autostart; webview do sistema |
| UI | **React 19 + TypeScript + Vite + Tailwind CSS 4** | UI rica estilo Rize; Recharts (gráficos), Framer Motion (animação do UBI), Zustand (estado), lucide-react (ícones) |
| Banco | **SQLite (rusqlite, bundled) + rusqlite_migration** | Zero-config, local, rápido; WAL |
| Captura macOS | `active-win-pos-rs` (app/janela ativa), `xcap` (screenshot), CoreGraphics (idle), `osascript` (URL do navegador) | Crates maduras; tudo por trás de traits |
| IA | **Anthropic Messages API** e **API compatível com OpenAI** (OpenAI, xAI) via `reqwest`, sem SDKs | Claude: `claude-haiku-4-5` para classificação (US$1/US$5 por MTok), `claude-sonnet-5` para relatórios, `output_config.format = json_schema`, prompt caching no system prompt. OpenAI: `gpt-5-mini` / `gpt-5`, `response_format = json_schema (strict)`, `max_completion_tokens`, `reasoning_effort = minimal` na classificação. xAI: `grok-4-1-fast-*`, mesmo formato com `max_tokens`. Um `RoutingLlmClient` escolhe o cliente pelo `ai_provider` das configurações |
| Segredos | `keyring` (macOS Keychain) | Chave de API nunca em texto plano no disco |
| Logs | `tracing` + `tauri-plugin-log` | Arquivo rotativo em `~/Library/Logs/ubiqX` |

Modelo default de custo (uso típico 8 h/dia): ~100 blocos/dia classificados por texto (~300 tokens in / 40 out cada) ≈ US$0,05/dia;
≤ 30 imagens/dia (1024 px, ~1.100 tokens cada) ≈ US$0,04/dia; 3 relatórios/dia no Sonnet 5 (~6 k in / 1,5 k out) ≈ US$0,08/dia.
**Estimativa: US$3–6/mês.** Todos os limites são configuráveis na UI.

## 3. Estrutura do repositório

```
ubiqx/
├── Cargo.toml                    # workspace
├── crates/
│   ├── ubiqx-core/               # DOMÍNIO: modelos, ports (traits), segmentação, regras, aprendizado,
│   │                             #   insights, agendamento, redação de dados, renderização de relatórios — sem I/O
│   │                             #   (+ nofap.rs / nofap_ipc.rs / product.rs: tipos, ports e IPC do NoFap, as duas identidades)
│   ├── ubiqx-nofap/              # DOMÍNIO do NoFap: listas embutidas, matcher, plano do sistema, compromisso,
│   │                             #   amostragem da visão, mensagens — puro, sem I/O (§ 9)
│   ├── ubiqx-storage/            # ADAPTER: SQLite (rusqlite) — repositórios e migrações
│   ├── ubiqx-platform/           # ADAPTER: macOS, Windows e Linux (janela ativa, título, idle, print por janela,
│   │                             #   URL, permissões, segredos) + plataforma roteirizada (mock) para testes/CI
│   ├── ubiqx-ai/                 # ADAPTER: clientes Anthropic e OpenAI-compatível (OpenAI, xAI) + roteador,
│   │                             #   tabela de preços por provedor, prompts, classificadores texto/visão,
│   │                             #   redator de relatórios, conselheiro, fakes para testes
│   ├── ubiqx-engine/             # APLICAÇÃO: amostrador, tracker, worker de classificação, agendador de
│   │                             #   relatórios, nudges, retenção, correções (aprendizado), fachada de leitura
│   ├── ubiqx-app/                # COMPOSIÇÃO: monta engine + SQLite + plataforma + IA (usado pelo desktop e CLI);
│   │                             #   nofap_app.rs monta o app separado (NofapApp, sem Engine)
│   └── ubiqx-cli/                # CLI headless: simulação, status, classificação, relatórios (roda em Linux)
├── apps/desktop/
│   ├── src/                      # React 19 + TypeScript + Tailwind 4 (UI); src/nofap/ = páginas do módulo,
│   │                             #   src/nofap-app/ = moldura do app separado (entrada index.nofap.html, dist-nofap)
│   └── src-tauri/                # Tauri 2: tray de barra de menus, comandos IPC, eventos, notificações
├── apps/nofap/src-tauri/         # Tauri 2: o app separado NoFap (binário ubiqx-shield, id ai.ubiqx.shield)
├── docs/                         # Arquitetura, guia de teste no macOS, screenshots
├── scripts/                      # assinatura de dev, instalação do modelo 3D do UBI, publicação das releases
│                                 #   (publish-release.mjs --product ubiqx|shield), scripts/nofap/ (lista, smoke)
└── .github/workflows/            # CI: testes Linux + type-check macOS/Windows + frontend + builds dos dois apps nos
                                  #   três SOs + as duas releases; nofap-smoke.yml aplica e remove as camadas do sistema
```

Regra de dependência (hexagonal): `core` ← `storage | platform | ai` ← `engine` ← `app` ← `desktop | nofap-desktop | cli`.
O crate puro `nofap` depende só de `core` e é usado por `platform` (o auxiliar do sistema), `engine`, `app` e pelo
shell do app separado; `storage` e `ai` não o usam.
`core` não depende de nenhum adapter; adapters dependem só de `core`; `engine` usa apenas as traits de `core`
e recebe as implementações por injeção (`EngineDeps`), montadas em `ubiqx-app`.

## 4. Domínio (`ubiqx-core`)

```
ActivitySample  { at, app_name, app_id, window_title, url, idle_secs, window_id }        // amostra bruta (~5 s)
ActivityBlock   { id, started_at, ended_at, app_name, app_id, title, title_key, url, domain,
                  category_id?, confidence, source(Rule|Memory|Llm|Vision|User), description?, screenshot_id?,
                  classify_attempts, next_attempt_at?, needs_review, ai_payload?, ai_sent_at?, is_manual, note? }
Category        { id, name, color, icon, description, keywords[], report_time?, report_template?,
                  is_productive, is_system, archived }
Rule            { id, category_id, matcher(App|Domain|TitleContains|Regex), pattern, priority,
                  origin(User|Learned), enabled, hit_count, miss_count, last_contradicted_at? }
Correction      { block_id, from_category?, to_category, app_id, title_key, domain?, at }   // sinal de aprendizado
Screenshot      { id, at, path, width, height, app_id, block_id?, sent_to_ai }
DailyReport     { id, date, category_id, items[{activity, kind, minutes, evidence[], time_range, continuation_of?}],
                  highlights[], summary_md (renderizado localmente), stale, edited, model, tokens }
Nudge           { kind(Unproductive|Distracted|BreakSuggested|Praise|Idle|ReportReady|Attention), title, message }
Settings        { intervalos, limiar de idle, apps/domínios bloqueados, modo privado com prazo, vision_policy
                  (Never|OnlyApps|AllExceptBlocked), modelos, orçamento mensal de IA, lote/espera de classificação,
                  local_only, horário padrão de relatório, perfil do usuário, horário silencioso, nudges }
EngineEvent     { BlockOpened, BlockClosed, BlocksClassified, ReportReady, Nudge, TrackerState, AiHealth,
                  PermissionRequired, ScreenshotTaken }                                       // engine → UI
```

**Ports (traits)** — todas `Send + Sync`:

- Plataforma: `ActivitySource::foreground()`, `BrowserUrlResolver`, `IdleDetector`, `ScreenCapturer::capture(target, max_edge)`
  (por janela, display ou primário) + `visible_apps()`, `PermissionChecker`, `SecretStore`, `Notifier`.
- Persistência (síncronas; o engine as chama em `spawn_blocking`): `BlockRepo`, `CategoryRepo`, `RuleRepo`,
  `CorrectionRepo`, `ScreenshotRepo`, `ReportRepo`, `NudgeRepo`, `SettingsRepo`, `UsageRepo`, `KvRepo`.
- Classificação: `LocalClassifier` (síncrono, infalível: regras, memória) e `RemoteClassifier` (assíncrono, em lote),
  `VisionClassifier`, `ReportWriter`, `Advisor`. O formato de requisição HTTP fica **dentro** de `ubiqx-ai`.
- `EventSink` (engine → shell) e `Clock` (testes determinísticos).

**Lógica pura em core (testada sem I/O):** `Segmenter`, `RuleClassifier`/`best_rule`, `MemoryClassifier`,
`suggest_rules`/`select_examples`, `compute_stats`/`NudgePolicy`, `due_reports`, `redact_*`,
`render_summary_md`/`render_monthly_md`.

## 5. Pipeline em tempo de execução (`ubiqx-engine`)

```
 thread "ubiqx-sampler" (bloqueante)        tarefas tokio
 ┌──────────┐ 5 s  mpsc  ┌───────────┐ blocos ┌──────────┐   30 s  ┌────────────────────────────────────┐
 │ Sampler  │───────────▶│ Tracker   │───────▶│ BlockRepo│────────▶│ ClassifyWorker                     │
 │ app/título│           │ Segmenter │        └──────────┘         │ regras do usuário → memória →      │
 │ idle, URL │           │ + print   │ (janela ativa, ≥20 s,       │ regras aprendidas → LLM (lote,     │
 └──────────┘           │ por bloco)│  política de visão)         │ texto redigido) → visão (print)    │
                        └───────────┘                             │ backoff 1m→4h, 5 tentativas → revisão│
 ┌──────────────┐ 30 s (relógio de parede, last_check persistido)  └────────────────────────────────────┘
 │ ReportSched. │──▶ due_reports → ReportWriter (Sonnet, thinking off) ou template local → ReportRepo → evento
 ├──────────────┤ 60 s
 │ Nudges       │──▶ NudgePolicy (cooldown, teto diário, silencioso, apps de reunião, snooze) → UBI + notificação
 ├──────────────┤ 1 h
 │ Retenção     │──▶ apaga prints além de `screenshot_retention_hours`
 └──────────────┘
```

- **Sampler**: lê `ActivitySource` + `IdleDetector`; URL só para navegadores e só quando o título muda
  (negações de Automação são lembradas por 30 min). Cada amostra é carimbada com relógio de parede, então
  suspensão/tampa fechada vira apenas um *gap* (bloco fechado), nunca tempo inflado.
- **Segmenter**: agrupa amostras consecutivas iguais (app + título normalizado, ou app + domínio no navegador);
  fecha em troca de contexto, ociosidade ou gap > 6 intervalos; blocos < `min_block_secs` são descartados.
  Apps bloqueados, janelas anônimas e modo privado geram blocos **"[privado]"** na categoria `sys-private`:
  o tempo continua contando, o conteúdo não.
- **Prints**: dirigidos pelo bloco (nunca por timer cego): só depois de 20 s no mesmo bloco, só da **janela ativa**
  (`CGWindowListCreateImage` via xcap), nunca se um app bloqueado estiver visível, repetidos a cada
  `screenshot_interval_secs` enquanto o bloco continua. Reduzidos a 1280 px e JPEG q60; apagados após a
  classificação (ou após `screenshot_retention_hours` se guardados para revisão).
- **ClassifyWorker**: pega blocos pendentes (`next_attempt_at` respeitado); cadeia local é grátis; o LLM só é
  chamado quando há ≥ `classify_batch_min` blocos ou o mais antigo espera ≥ `classify_max_wait_secs`; lote máximo 25;
  antes de enviar, cada bloco passa por `redact_block` (sem query string, sem e-mail/telefone/CPF/CNPJ; títulos de
  apps de mensagens viram só o nome do app) e o texto exato enviado fica em `ai_payload` para auditoria na UI.
  Visão só para blocos ambíguos com print, dentro de `vision_policy` e `max_vision_per_hour`.
- **Saúde da IA / custo**: 401/403 → pausa até trocar a chave; conta sem créditos ou modelo inexistente
  (`AiRejected`) → pausa com o motivo; 429/5xx → degradado por 15 min; orçamento mensal (`ai_monthly_budget_usd`,
  padrão US$ 5) — aviso a 80 %, parada a 100 %. A saúde é avaliada para o provedor selecionado; trocar de provedor
  reavalia pela chave dele. Sem chave, tudo funciona em modo local.
- **Aprendizado**: uma correção (1) reclassifica o bloco (`source=User`, definitivo), (2) grava `Correction`,
  (3) penaliza a regra que errou (`miss_count`; auto-desativa com 2 erros ou >30 % de erro), (4) opcionalmente
  reaplica ao dia/mês (mesmo app+domínio), (5) sugere "Sempre: X → categoria" — aplicado automaticamente apenas para
  domínios específicos (nunca para apps ou serviços multi-inquilino como Gmail/WhatsApp/Docs), senão pede confirmação.
  A memória compara com blocos já corrigidos (similaridade app/domínio/título ≥ 0,85) e ignora títulos genéricos.
- **Relatórios**: o produto são os `items` (atividade no passado, tipo, minutos, evidências, continuação); o
  Markdown diário e o **relatório mensal por categoria** são renderizados localmente e exportáveis. O redator recebe
  perfil do usuário, descrição/template da categoria e itens dos 5 dias anteriores. Sem IA, um template local
  agrupa por app/domínio. Relatórios ficam `stale` quando blocos do dia mudam e podem ser regenerados; editados à
  mão nunca são sobrescritos.

## 6. Interface (React)

Páginas: **Hoje** (score de foco, tempo por categoria, linha do tempo 24 h, foco por hora, UBI com dica, saúde da IA
e orçamento), **Timeline** (blocos com reclassificação em 1 clique, divisão, entrada manual, badge "enviado à IA"),
**Revisão** (grupos por app/domínio ordenados por duração × incerteza, atalhos de teclado, sugestões "Sempre"),
**Relatórios** (diário editável + mensal exportável), **Categorias & Regras**, **Insights** (semana, recomendações do
UBI), **Configurações** (IA, rastreamento, privacidade, permissões, UBI) e **Onboarding** em 7 passos.
UBI: modelo 3D (`public/ubi/Ubi.glb`, GLTFLoader via react-three-fiber, canvas transparente) com fallback para o PNG
(`public/ubi/ubi.png`) e depois SVG; humor derivado do score de foco.

## 7. Segurança & privacidade

- Chaves de API só no Keychain (`keyring`, serviço `ai.ubiqx`, uma conta por provedor: `anthropic_api_key`,
  `openai_api_key`, `xai_api_key`), nunca em arquivo; cada uma é validada com `GET /models` e uma mensagem de
  1 token, para que conta sem créditos ou modelo indisponível apareçam como erro claro (`CoreError::AiRejected`),
  não como chave "válida" que falha em silêncio depois. Só o provedor selecionado recebe dados.
- OAuth: "Entrar com ChatGPT" é, hoje, apenas identidade para apps parceiros da OpenAI, sem acesso aos modelos
  pela assinatura; por isso todos os provedores entram por chave. A credencial é um `Bearer` genérico
  (`ApiKeySource`), então um token OAuth pode ser plugado sem mudar o cliente quando isso existir oficialmente.
- Nada é registrado nem enviado antes do consentimento: rastreamento, prints e toda chamada remota exigem
  `onboarding_done`; até lá o tracker aparece como pausado.
- Só sai da máquina o que a UI mostra em "Dados enviados à IA": linhas redigidas por bloco e, quando permitido,
  o print da janela ativa. `local_only` desliga qualquer chamada remota.
- Prints em `~/Library/Application Support/ai.ubiqx.app/screenshots.noindex/` (0700), apagados após uso.
- Apps bloqueados (gerenciadores de senha por padrão), domínios bloqueados, janelas anônimas e modo privado
  (30 min / 1 h / até amanhã / indefinido, pelo tray) viram blocos "[privado]".
- "Exportar meus dados" (JSON) e "Apagar todos os dados": blocos, prints, correções, relatórios, avisos, uso de IA,
  cache de conselhos, regras aprendidas e a pasta de exportações, em uma transação (`MaintenanceRepo::wipe_user_data`).
  Configurações, categorias, regras criadas pelo usuário e a chave de API permanecem.
- O tracker persiste só as colunas que o segmentador possui (`BlockRepo::touch` / `set_screenshot`); uma
  reclassificação feita pelo usuário no bloco em andamento nunca é sobrescrita.
- Cmd+Q apenas esconde a janela; o único caminho para encerrar o rastreador é "Sair" no tray, que espera o
  bloco aberto ser gravado.
- Permissões macOS: **Gravação de Tela** (títulos + prints), **Automação** (URL do navegador). Ver `docs/MACOS-TESTING.md`
  — inclusive a assinatura estável do binário para as permissões sobreviverem a rebuilds.

## 8. Decisões (ADRs resumidas)

1. **Rust + Tauri em vez de Electron/Swift**: menor consumo em background, core reutilizável em outros SOs, UI web bonita.
2. **Texto antes de imagem**: a maioria dos blocos é classificável por título/URL; visão só em ambíguos e sob política.
3. **Cadeia de classificadores** com split local/remoto: regras e memória são síncronas, grátis e testáveis; o LLM é
   assíncrono, em lote, com backoff e orçamento.
4. **Saída estruturada** (`output_config.format = json_schema` na Anthropic, `response_format = json_schema`
   estrito na OpenAI/xAI) elimina parsing frágil. **Prompt caching** só onde vale: no redator (Sonnet 5, prefixo
   ≥ 1024 tokens); no Haiku 4.5 o mínimo é 4096 tokens, então o prompt de classificação é mantido enxuto e sem
   `cache_control`. Sonnet 5 recebe `thinking: disabled` e os `gpt-5*` recebem `reasoning_effort: minimal` na
   classificação para não cobrar raciocínio; os esquemas JSON são os mesmos para todos os provedores.
5. **rusqlite** atrás de `Mutex` com chamadas em `spawn_blocking`; nunca se segura o lock entre `await`s.
6. **Plataforma roteirizada + CLI** para desenvolver e testar o pipeline inteiro em Linux/CI; os módulos macOS e
   Windows são type-checked contra `aarch64-apple-darwin` e `x86_64-pc-windows-msvc` na CI.
7. **Tempo nunca some**: bloqueios e modo privado redigem, não descartam.
8. **Um adaptador por SO, uma identidade por SO**: bundle id no macOS, nome do executável no Windows e no Linux;
   ver [`WINDOWS-LINUX.md`](WINDOWS-LINUX.md). O que ainda falta: Wayland pelos portais (`xdg-desktop-portal`;
   hoje só X11/XWayland), URL do navegador no Linux por acessibilidade (AT-SPI; hoje só pelo título), eventos de
   sono/bloqueio de tela e exportação DOCX.

## 9. O módulo NoFap e o segundo produto

O NoFap é uma ferramenta de autocontrole contra a pornografia que uma pessoa adulta instala no próprio computador:
visível, desligada até a pessoa concluir o assistente, sem relatório para ninguém. O projeto completo está em
[`nofap/DESIGN.md`](nofap/DESIGN.md) (as decisões tomadas durante a construção, que valem sobre ele, em
[`nofap/handoff/INTEGRATOR.md`](nofap/handoff/INTEGRATOR.md)); a privacidade em [`nofap/PRIVACY.md`](nofap/PRIVACY.md);
a remoção em [`nofap/REMOVAL.md`](nofap/REMOVAL.md).

### 9.1 Camadas

| Camada | Onde está | O que faz |
|---|---|---|
| L1 guarda | `ubiqx_engine::nofap::guard` + `ubiqx_nofap::matcher` | a cada 2 s, o site da aba em frente contra a lista embutida (~240 mil entradas como hashes; as 20 mil mais visitadas também por extenso), as regras de palavras e terminações (`heuristics`) e a lista da pessoa; um acerto fecha a aba e mostra um cartão |
| L2 `hosts` | `ubiqx_nofap::hosts` + `ubiqx_platform::sysblock` | um bloco marcado (`# >>> ubiqx-shield v1 BEGIN …` / `# <<< ubiqx-shield END`) com até 20.000 nomes (8.000 no Windows) |
| L3 DNS filtrado | `ubiqx_nofap::resolvers` | DNS do sistema na Cloudflare for Families (`1.1.1.3`, `1.0.0.3` e IPv6) |
| L4 políticas | `ubiqx_nofap::policies` | DNS próprio do navegador desligado e SafeSearch forçado (Chrome, Chromium, Edge, Brave, Firefox) |
| L5 visão | `ubiqx_engine::nofap::vision` + `ubiqx_ai::explicit` + `ubiqx_nofap::sampling` | uma imagem da janela em frente, reduzida a 640 px, ao provedor de IA escolhido quando as regras locais não decidem; resposta fechada (explícito, sugestivo, nada, não sei) |

Cada camada tem o seu estado (`TierState`) e a página mostra o que está protegendo e o que não está, sem silêncio.
L2 a L4 passam pela trava por sistema `ubiqx_nofap::gate::TIER_GATE`: uma camada só abre num sistema depois de uma
execução verde do job `nofap-system-smoke` nele (hoje `hosts` e `resolver` abertos em macOS, Windows e Linux;
`policies` fechado nos três até a conferência manual dos navegadores).

### 9.2 Onde fica a lógica

- **`ubiqx-core`** (`nofap.rs`, `nofap_ipc.rs`, `product.rs`): tipos, ports (`SystemShield`, `FrameSource`,
  `ExplicitDetector`, `NofapPresenter`, `NofapSink`, `NofapRepo`, `NofapEnv`), comandos e respostas do IPC, e as
  duas identidades de produto (`UBIQX`, `SHIELD`: bundle id, banco, serviço do cofre, feed).
- **`ubiqx-nofap`**: as decisões e os dados embutidos, puros e testados sem I/O: listas e matcher, plano e avaliação
  do sistema (`plan`), compromisso (pedidos que enfraquecem a proteção esperam o tempo que a pessoa escolheu e
  depois pedem confirmação), amostragem e limites da visão, episódios, exclusões, papel primário entre os dois
  produtos, mensagens e recursos de ajuda.
- **`ubiqx-engine::nofap`**: `NofapService`, construído só de ports (a forma do `UpdateChecker`), com dois laços:
  guarda (2 s) e verificação do sistema (120 s). Ele nunca referencia `EngineState`, e é isso que deixa o app
  separado hospedá-lo. No ubiqX, um acerto também faz o rastreador esquecer a página (`tracker::scrub_open_block`,
  `redacts`, `capture_veto`).
- **`ubiqx-platform::sysblock`**: um adaptador `NativeSystemShield` com os três back-ends (macOS, Windows, Linux)
  escritos como montadores de comando e leitores de saída, compilados em todos os sistemas atrás de `Runner`, `Fs`
  e `Elevator`. A elevação é o próprio executável do app chamado como `--shield-helper <verbo>` por `osascript`,
  `Start-Process -Verb RunAs` ou `pkexec` (`elevate.rs`); os verbos são só `status`, `elevate-argv`, `apply` e
  `remove` (`helper.rs`). Não há serviço privilegiado em segundo plano. O mesmo código existe como binário de linha
  de comando, `ubiqx-shield-helper`, para a CI e para a remoção sem o app.
- **`ubiqx-storage`**: as tabelas `nofap_day` (contagens por dia e hora), `nofap_sites` (lista da pessoa, sites
  aprendidos como hash com sal, adiamentos) e `nofap_state` (a configuração, uma linha), na migração 3. Toda conexão
  abre com `secure_delete`, e o apagamento do histórico passa o WAL para o arquivo principal.
- **`ubiqx-ai::explicit`**: `LlmExplicitDetector` sobre o mesmo cliente roteado do ubiqX; para o tipo
  `nofap_vision`, nenhuma linha de log no caminho de sucesso e o custo gravado na hora cheia.

### 9.3 Dois produtos, um código

| | ubiqX | NoFap separado |
|---|---|---|
| Shell | `apps/desktop/src-tauri` (`ubiqx-desktop`) | `apps/nofap/src-tauri` (`ubiqx-nofap-desktop`, binário `ubiqx-shield`) |
| Composição | `App::start` monta o `Engine` com `EngineDeps.nofap` | `NofapApp::start` (`ubiqx_app::nofap_app`): só o serviço, sem `Engine`; nenhum amostrador, rastreador, classificador ou relatório existe no processo |
| Interface | página `/nofap` dentro do ubiqX | `src/nofap-app/` sobre as mesmas páginas de `src/nofap/`, construída com `pnpm build:nofap` (`vite --mode nofap`, saída `dist-nofap`) |
| Comandos | `nofap` | `nofap`, `nofap_host` (configurações, chave de IA, modelos, permissões, atualização, sobre) e `quit_app` |
| Identidade | `ai.ubiqx.app`, `ubiqx.db`, cofre `ai.ubiqx` | `ai.ubiqx.shield`, `shield.db`, cofre `ai.ubiqx.shield` |
| Atualização | release `continuous` (`ubiqX-*`), variável `UBIQX_UPDATE_FEED_URL` | release `shield-continuous` (`ubiqx-shield-*`), variável `UBIQX_SHIELD_UPDATE_FEED_URL`; publicação ligada; `NOFAP_PUBLISH=false` desliga |
| IA | as três com chave própria ou a IA do Ubi | só as três com chave própria |

Os dois podem estar instalados juntos. As camadas do sistema são de quem as pediu: o arquivo de estado do sistema
guarda o que cada produto quer, e `remove --owner ubiqx|shield` tira só o de um. Quando os dois estão ligados, só
um faz a guarda: o arquivo `primary.lock` na pasta `ubiqx-shield` dos dados locais diz qual (`ubiqx_nofap::primary`).
Cada app ignora o feed de atualização do outro (`ubiqx_core::update::feed_matches_product`).

### 9.4 Os três anéis de nomes

- **Fonte**: `nofap` (crates, módulos, tabelas, comandos, namespace de i18n).
- **Máquina**: `ubiqx-shield`, em tudo o que é escrito no computador ou publicado (bloco do `hosts`, pastas de
  estado, nomes dos arquivos de política, argumentos de processo, nomes dos instaladores do app separado).
- **Exibição**: `ubiqx_core::nofap::DISPLAY_NAME` ("NoFap"), o único lugar do nome visível.

`cargo test -p ubiqx-nofap --test naming` falha se a palavra da fonte aparecer no anel da máquina.

### 9.5 CI

`rust-linux` roda `python3 scripts/nofap/build_blocklist.py --check` (a lista embutida é a que as fontes fixadas
geram, sem rede) e o teste dos nomes; clippy e testes excluem os dois shells (sem webkit). `frontend` roda
`pnpm build:nofap`. Os três jobs de app compilam os dois produtos (o do NoFap com o CLI do Tauri a partir de
`apps/nofap/src-tauri`, cujo `beforeBuildCommand` gera `dist-nofap`) e renomeiam os arquivos de cada um pelo
`productName` do seu `tauri.conf.json`, porque os dois empacotam nas mesmas pastas de `target/release/bundle`. O
job do Windows roda clippy e testes do `ubiqx-nofap-desktop`. `publish-continuous` publica só `ubiqX-*`;
`publish-shield` publica só `ubiqx-shield-*`, só na `main`, a menos que `vars.NOFAP_PUBLISH` seja `false`. O workflow
`nofap-smoke.yml` aplica e remove as camadas do sistema de verdade nos três sistemas, com o auxiliar como
administrador.
