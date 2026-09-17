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
│   ├── ubiqx-core/               # DOMÍNIO: modelos, ports (traits), segmentação, insights, agendamento — sem I/O
│   ├── ubiqx-storage/            # ADAPTER: SQLite (repositórios), migrações
│   ├── ubiqx-platform/           # ADAPTER: macOS (janela ativa, screenshot, idle, URL) + implementação mock p/ outros SOs
│   ├── ubiqx-ai/                 # ADAPTER: cliente Anthropic, prompts, classificador LLM, redator de relatórios
│   ├── ubiqx-engine/             # APLICAÇÃO: orquestra tracker, pipeline de classificação, scheduler, nudges
│   └── ubiqx-cli/                # CLI headless (simulação, relatórios, debug) — roda em Linux/CI
├── apps/desktop/
│   ├── src/                      # React (UI)
│   └── src-tauri/                # Tauri: comandos IPC, tray, composição (DI) do engine
├── assets/ubi/                   # Mascote UBI (SVG + estados)
├── docs/                         # Arquitetura, decisões, screenshots
└── .github/workflows/            # CI (Linux: core/storage/ai/engine/cli + frontend; macOS: build do app)
```

Regra de dependência (hexagonal): `core` ← `storage | platform | ai` ← `engine` ← `desktop | cli`.
`core` não depende de nenhum adapter; adapters dependem só de `core`; `engine` usa apenas as traits de `core`
e recebe as implementações por injeção (composition root em `src-tauri/src/main.rs` e `ubiqx-cli`).

## 4. Domínio (`ubiqx-core`)

```
ActivitySample  { at, app_name, bundle_id, window_title, url, idle_secs }        // amostra bruta (a cada ~5 s)
ActivityBlock   { id, started_at, ended_at, app_name, bundle_id, title, url, domain,
                  category_id?, confidence, source(Rule|Memory|Llm|Vision|User), description?, screenshot_id? }
Category        { id, name, color, icon, description, keywords[], report_time (HH:MM)?, is_productive, archived }
Rule            { id, category_id, matcher(App|Domain|TitleContains|Regex), pattern, priority, origin(User|Learned) }
Correction      { id, block_id, from_category?, to_category, note?, at }          // alimenta aprendizado
Screenshot      { id, at, path, width, height, block_id?, sent_to_ai }
DailyReport     { id, date, category_id, generated_at, summary_md, items[], total_secs, model, tokens }
Nudge           { id, at, kind(Unproductive|Distracted|LongSession|BreakSuggested|Praise|Idle), message, seen }
Settings        { sample_interval, screenshot_interval, idle_threshold, blocked_apps[], private_mode,
                  retention_days, models{classify,report}, max_vision_per_hour, quiet_hours, report_default_time }
```

**Ports (traits)** — todas `Send + Sync`:

- `ActivitySource::current()` → `Option<ForegroundWindow>` (app, bundle id, título, pid)
- `BrowserUrlResolver::url_for(app) -> Option<Url>`
- `IdleDetector::idle_secs()`
- `ScreenCapturer::capture_primary() -> Image`
- `Repositories`: `BlockRepo`, `CategoryRepo`, `RuleRepo`, `CorrectionRepo`, `ScreenshotRepo`, `ReportRepo`, `NudgeRepo`, `SettingsRepo`
- `Classifier::classify(&[ActivityBlock], &Context) -> Vec<Classification>` (implementado por Rule, Memory, Llm; encadeados por `ClassifierChain`)
- `LlmClient::complete(request) -> response` (JSON estruturado; `AnthropicClient` é a implementação)
- `ReportWriter::write(date, category, blocks) -> DailyReport`
- `Notifier::notify(Nudge)`
- `Clock::now()` (permite testes determinísticos)

**Lógica pura em core (testável sem I/O):** `Segmenter` (amostras → blocos), `InsightEngine` (blocos → nudges/score de foco),
`ReportScheduler` (quando gerar cada relatório), `RuleMatcher`, `TitleNormalizer`.

## 5. Pipeline em tempo de execução (`ubiqx-engine`)

```
 ┌──────────┐ 5 s  ┌───────────┐       ┌──────────┐ blocos   ┌────────────────┐ pendentes ┌───────────────┐
 │ Sampler  │─────▶│ Segmenter │──────▶│ BlockRepo│─────────▶│ ClassifyWorker │──────────▶│ ClassifierChain│
 └──────────┘      └───────────┘       └──────────┘          └────────────────┘           │ Rule→Memory→LLM│
      │ a cada N min (se ativo e app permitido)                                            └───────┬───────┘
      ▼                                                                                            │ ambíguo
 ┌────────────┐  downscale 1024px, WebP q60                                               ┌────────▼──────┐
 │ Screenshot │────────────────────────────────▶ disco + ScreenshotRepo                     │ VisionClassif.│
 └────────────┘                                                                            └───────────────┘
 ┌──────────────┐ cron interno   ┌──────────────┐   ┌──────────┐      ┌────────────┐
 │ ReportSched. │───────────────▶│ ReportWriter │──▶│ReportRepo│ ───▶ │ Notifier+UI│
 └──────────────┘                └──────────────┘   └──────────┘      └────────────┘
 ┌──────────────┐ a cada 1 min   ┌────────┐
 │ InsightEngine│───────────────▶│ Nudges │──▶ UBI (UI) + notificação macOS
 └──────────────┘                └────────┘
```

- **Sampler**: lê `ActivitySource` + `IdleDetector` (+ `BrowserUrlResolver` quando o app é navegador e o título mudou).
- **Segmenter**: agrupa amostras consecutivas iguais (app + título normalizado/domínio); fecha bloco em troca de contexto,
  idle > limiar ou pausa; descarta blocos < 15 s (configurável) mesclando ao vizinho.
- **ClassifyWorker**: a cada 60 s pega blocos `category_id = NULL` fechados; roda a cadeia. LLM recebe **lotes** (até 25 blocos) em texto.
  Bloco é "ambíguo" se confiança < 0,6 ou título genérico (WhatsApp, Mail, Finder…) → um screenshot associado vai ao modelo de visão,
  que devolve `{category, description}`; `description` enriquece o relatório ("respondeu mensagens sobre edital X").
- **Aprendizado**: `Correction` → (a) `Rule` sugerida (domínio/app/título) com `origin=Learned` (usuário confirma ou o sistema aplica
  automaticamente após 2 correções coerentes); (b) top-K exemplos semelhantes (Jaccard sobre tokens do título + app) entram no prompt.
- **ReportScheduler**: para cada categoria com `report_time` (ou default), gera o relatório do dia; regenera sob demanda.
- **InsightEngine** (local, sem custo): score de foco (0–100) por hora e por dia, taxa de troca de contexto, tempo em apps
  "distração" (categorias não produtivas), sessões > 90 min sem pausa, ociosidade longa → `Nudge`. UBI muda de humor conforme o score.

## 6. Interface (React)

Páginas: **Dashboard** (hoje: tempo por categoria, timeline, score de foco, UBI com dica), **Timeline/Revisão** (blocos com
reclassificação inline em 1 clique — é aqui que o sistema aprende), **Relatórios** (por dia/categoria, Markdown, copiar/exportar),
**Categorias & Regras**, **Insights** (tendências, recomendações), **Configurações** (chave de API, modelos, intervalos,
apps bloqueados, horários de relatório, privacidade, iniciar com o sistema).
Design: tema claro/escuro, cards arredondados, paleta azul/laranja do UBI, tipografia Inter, gráficos suaves (Recharts).

## 7. Segurança & privacidade

- Chave de API só no Keychain (`keyring`), nunca em arquivo. Nada sai da máquina além das chamadas à API configurada.
- Screenshots: JPEG/WebP reduzido, sem OCR local por padrão, retenção de 7 dias (configurável), apagados após uso.
- Apps bloqueados (1Password, bancos…) nunca são capturados; "Modo privado" pausa tudo com 1 clique no tray.
- Permissões macOS necessárias: **Gravação de Tela** (títulos de janela + screenshots), **Automação** (URL do navegador) —
  a UI guia o usuário na primeira execução.

## 8. Decisões (ADRs resumidas)

1. **Rust + Tauri em vez de Electron/Swift**: menor consumo em background, um core reutilizável em outros SOs, UI web bonita.
2. **Texto antes de imagem**: 90 % dos blocos são classificáveis pelo título/URL; visão só em ambíguos → custo baixo.
3. **Cadeia de classificadores**: regras (determinístico, grátis) > memória (grátis) > LLM. Facilita testes e troca de provedor.
4. **Saída estruturada** (`json_schema`) elimina parsing frágil; **prompt caching** no system prompt (categorias + regras + exemplos).
5. **rusqlite** em vez de sqlx: sem runtime async no banco, mais simples para um app desktop; acesso serializado por `Mutex`.
6. **CLI headless** para desenvolver/testar o core em Linux/CI com uma fonte de atividade simulada.
