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
| O6 | Custo de API mínimo | Só texto vai ao LLM por padrão; imagens só em blocos ambíguos, com limite/hora; modelo barato para classificação (Haiku 4.5) e modelo melhor só para relatórios (Sonnet 5) |
| O7 | Privacidade | Tudo local (SQLite + imagens no disco do usuário); lista de apps bloqueados; modo privado; retenção configurável; chave no Keychain |
| O8 | Portabilidade futura (Windows/Linux) | Toda dependência de SO isolada atrás de traits (`ActivitySource`, `ScreenCapturer`, `IdleDetector`) |

## 2. Stack

| Camada | Escolha | Motivo |
|--------|---------|--------|
| Núcleo / daemon | **Rust** (edition 2021, MSRV 1.85) | Performance, baixo consumo em background, segurança de memória, um único binário |
| Desktop shell | **Tauri 2** | App nativo macOS pequeno (~10 MB), tray/menubar, notificações, autostart; webview do sistema |
| UI | **React 19 + TypeScript + Vite + Tailwind CSS 4** | UI rica estilo Rize; Recharts (gráficos), Framer Motion (animação do UBI), Zustand (estado), lucide-react (ícones) |
| Banco | **SQLite (rusqlite, bundled) + rusqlite_migration** | Zero-config, local, rápido; WAL |
| Captura macOS | `active-win-pos-rs` (app/janela ativa), `xcap` (screenshot), CoreGraphics (idle), `osascript` (URL do navegador) | Crates maduras; tudo por trás de traits |
| IA | **Anthropic Messages API** via `reqwest` (sem SDK oficial em Rust) | `claude-haiku-4-5` para classificação (US$1/US$5 por MTok), `claude-sonnet-5` para relatórios; saída estruturada (`output_config.format = json_schema`); prompt caching no system prompt |
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
│   ├── ubiqx-storage/            # ADAPTER: SQLite (rusqlite) — repositórios e migrações
│   ├── ubiqx-platform/           # ADAPTER: macOS (janela ativa, título, idle, print por janela, URL, permissões,
│   │                             #   Keychain) + plataforma roteirizada (mock) para Linux/CI
│   ├── ubiqx-ai/                 # ADAPTER: cliente Anthropic (HTTP), prompts, classificadores texto/visão,
│   │                             #   redator de relatórios, conselheiro, fakes para testes
│   ├── ubiqx-engine/             # APLICAÇÃO: amostrador, tracker, worker de classificação, agendador de
│   │                             #   relatórios, nudges, retenção, correções (aprendizado), fachada de leitura
│   ├── ubiqx-app/                # COMPOSIÇÃO: monta engine + SQLite + plataforma + IA (usado pelo desktop e CLI)
│   └── ubiqx-cli/                # CLI headless: simulação, status, classificação, relatórios (roda em Linux)
├── apps/desktop/
│   ├── src/                      # React 19 + TypeScript + Tailwind 4 (UI)
│   └── src-tauri/                # Tauri 2: tray de barra de menus, comandos IPC, eventos, notificações
├── docs/                         # Arquitetura, guia de teste no macOS, screenshots
├── scripts/                      # assinatura de dev, instalação do modelo 3D do UBI
└── .github/workflows/            # CI: testes Linux + type-check macOS + frontend + build do app no macOS
```

Regra de dependência (hexagonal): `core` ← `storage | platform | ai` ← `engine` ← `app` ← `desktop | cli`.
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
- **Saúde da IA / custo**: 401/403 → pausa até trocar a chave; 429/5xx → degradado por 15 min; orçamento mensal
  (`ai_monthly_budget_usd`, padrão US$ 5) — aviso a 80 %, parada a 100 %. Sem chave, tudo funciona em modo local.
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
UBI: modelo 3D (`public/ubi/Ubi.glb`, react-three-fiber) com fallback SVG; humor derivado do score de foco.

## 7. Segurança & privacidade

- Chave de API só no Keychain (`keyring`, serviço `ai.ubiqx`), nunca em arquivo; validada com `GET /v1/models` e
  uma mensagem de 1 token, para que conta sem créditos ou modelo indisponível apareçam como erro claro
  (`CoreError::AiRejected`), não como chave "válida" que falha em silêncio depois.
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
4. **Saída estruturada** (`output_config.format = json_schema`) elimina parsing frágil. **Prompt caching** só onde
   vale: no redator (Sonnet 5, prefixo ≥ 1024 tokens); no Haiku 4.5 o mínimo é 4096 tokens, então o prompt de
   classificação é mantido enxuto e sem `cache_control`. Sonnet 5 recebe `thinking: disabled` para não cobrar raciocínio.
5. **rusqlite** atrás de `Mutex` com chamadas em `spawn_blocking`; nunca se segura o lock entre `await`s.
6. **Plataforma roteirizada + CLI** para desenvolver e testar o pipeline inteiro em Linux/CI; o módulo macOS é
   type-checked contra `aarch64-apple-darwin` na CI.
7. **Tempo nunca some**: bloqueios e modo privado redigem, não descartam.
