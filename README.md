<p align="center">
  <img src="docs/ubi-hero.png" alt="UBI, o mascote do ubiqX" width="180"/>
</p>

<h1 align="center">ubiqX</h1>
<p align="center"><strong>Rastreamento inteligente de atividade para macOS, com IA nativa e um mascote que cuida do seu foco.</strong></p>
<p align="center">
  <a href="https://github.com/andreyquadros/ubiquitous-engine/actions/workflows/ci.yml"><img src="https://github.com/andreyquadros/ubiquitous-engine/actions/workflows/ci.yml/badge.svg" alt="CI"/></a>
  <img src="https://img.shields.io/badge/Rust-1.85%2B-orange" alt="Rust"/>
  <img src="https://img.shields.io/badge/Tauri-2-blue" alt="Tauri 2"/>
  <img src="https://img.shields.io/badge/macOS-13%2B-black" alt="macOS 13+"/>
</p>

O ubiqX roda em segundo plano na barra de menus, observa **em qual app, janela e site você está**, tira
prints esparsos **só da janela ativa**, e enquadra cada bloco de tempo nas **categorias que você cria**
(IFRO, Incubadora, Cidades Inteligentes…). No horário que você escolher, gera um **relatório diário por
categoria** pronto para virar o relatório mensal de cada instituição. Se ele errar, você corrige em um clique —
e ele aprende. O **UBI**, o mascote, mostra seu humor de foco e avisa quando você está se distraindo.

É um concorrente do [Rize](https://rize.io) com IA embarcada de fábrica, local-first e de código aberto.

## Como funciona

```
 a cada 5 s          blocos de contexto           classificação em cadeia               relatórios
 app · janela · URL ─▶ Segmenter ─▶ SQLite ─▶ regras ▸ memória ▸ IA (texto) ▸ IA (visão) ─▶ diário/mensal
 print da janela ativa (quando permitido)        ▲ correções do usuário ═ aprendizado ═▶ regras sugeridas
```

- **Captura leve**: amostra o app/janela/URL ativo (sem teclas, sem conteúdo). Prints só depois de 20 s no mesmo
  contexto, só da janela em foco, nunca se um app bloqueado estiver visível.
- **Classificação em cadeia, custo mínimo**: regras e memória de correções resolvem a maioria dos blocos de graça;
  o restante vai ao modelo de texto em lote; prints só para blocos ambíguos e sob sua política.
  Custo típico estimado: **US$ 3–6/mês** com Claude, menos com OpenAI ou Grok; orçamento mensal configurável
  que pausa a IA ao ser atingido.
- **Você escolhe a IA**: Anthropic Claude, OpenAI ou xAI Grok, cada uma com a sua própria chave de API guardada no
  Keychain. Troque quando quiser em Configurações → IA; as chaves das outras ficam guardadas.
- **Privacidade por padrão**: URLs sem query string, e-mails/telefones/CPF/CNPJ mascarados, títulos de apps de
  mensagens reduzidos ao nome do app; "Dados enviados à IA" mostra exatamente o que saiu da máquina. Modo privado
  com prazo, apps e domínios bloqueados e janelas anônimas viram blocos "[privado]" — o tempo conta, o conteúdo não.
- **Aprende com você**: cada correção reclassifica o bloco, alimenta a memória, penaliza a regra que errou e sugere
  "Sempre: sei.ifro.edu.br → IFRO".
- **Relatórios que viram entregas**: itens no passado com tipo (reunião, desenvolvimento, ensino…), minutos e
  evidências; visão mensal por categoria que agrupa continuações ("continuou X — 3 dias, 7 h 20") e exporta Markdown.
- **UBI**: humor derivado do score de foco; avisos com teto diário, cooldown, horário silencioso e silêncio em reuniões.

## Telas

| Hoje | Timeline |
|---|---|
| ![Dashboard](docs/screenshots/dashboard.png) | ![Timeline](docs/screenshots/timeline.png) |

| Revisão (é aqui que ele aprende) | Relatórios |
|---|---|
| ![Revisão](docs/screenshots/review.png) | ![Relatórios](docs/screenshots/reports.png) |

| Categorias & regras | Insights |
|---|---|
| ![Categorias](docs/screenshots/categories.png) | ![Insights](docs/screenshots/insights.png) |

| Configurações | Onboarding |
|---|---|
| ![Configurações](docs/screenshots/settings.png) | ![Onboarding](docs/screenshots/onboarding.png) |

## Stack

| Camada | Tecnologia |
|---|---|
| Núcleo / daemon | **Rust** — workspace hexagonal (`core` → `storage` / `platform` / `ai` → `engine` → `app` → `desktop` / `cli`) |
| Desktop | **Tauri 2** (tray na barra de menus, notificações, iniciar com o sistema) |
| UI | **React 19 + TypeScript + Vite + Tailwind 4**, Recharts, Framer Motion, react-three-fiber (UBI em 3D) |
| Dados | **SQLite** local (`rusqlite`, WAL) |
| IA | **Anthropic Messages API**, **OpenAI Chat Completions** ou **xAI Grok** (API compatível com OpenAI) via HTTP, escolhida pelo usuário; saída estruturada JSON em todas |
| Segredos | Keychain do macOS (`keyring`) |

Detalhes em [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Rodando no seu Mac

Guia completo (assinatura estável, permissões, onde ficam os dados, problemas comuns):
[`docs/MACOS-TESTING.md`](docs/MACOS-TESTING.md). Resumo:

```bash
git clone https://github.com/andreyquadros/ubiquitous-engine ubiqx && cd ubiqx
scripts/install-ubi-model.sh                       # opcional: instala a arte do UBI (ubi.png e/ou Ubi.glb de ~/Downloads)
cd apps/desktop && pnpm install && pnpm tauri build
cd ../.. && scripts/codesign-dev.sh                # identidade "ubiqX Dev" — permissões sobrevivem a rebuilds
open target/release/bundle/macos/ubiqX.app
```

Na primeira execução o onboarding pede para **escolher a IA** e colar a chave correspondente, a permissão de
**Gravação de Tela** (reinicie o app depois de conceder) e a **Automação** para o navegador; depois você cria as
categorias e escolhe o horário dos relatórios.

| Provedor | Modelos padrão (classificação / visão / relatórios) | Estimativa com 8 h/dia | Onde criar a chave |
|---|---|---|---|
| Anthropic Claude | `claude-haiku-4-5` / `claude-haiku-4-5` / `claude-sonnet-5` | US$ 3–6/mês | [console.anthropic.com](https://console.anthropic.com/settings/keys) |
| OpenAI | `gpt-5-mini` / `gpt-5-mini` / `gpt-5` | US$ 1–3/mês | [platform.openai.com](https://platform.openai.com/api-keys) |
| xAI Grok | `grok-4-1-fast-non-reasoning` / idem / `grok-4-1-fast-reasoning` | US$ 0,50–2/mês | [console.x.ai](https://console.x.ai) |

Os ids de modelo são editáveis e "Listar modelos da conta" mostra o que a sua chave pode usar. Crie a chave num
projeto/workspace com limite de gasto: é a rede de segurança real. "Entrar com ChatGPT" (OAuth) hoje só identifica o
usuário em apps parceiros da OpenAI e não dá acesso aos modelos; por isso a OpenAI entra por chave de API.

Sem chave, tudo continua funcionando em modo local (regras + memória + relatórios por template).

## Desenvolvimento

```bash
# testes e lint dos crates portáveis (Linux/macOS)
cargo test --workspace --exclude ubiqx-desktop
cargo clippy --workspace --all-targets -- -D warnings

# simulação ponta a ponta de um dia de trabalho com IA falsa (não precisa de Mac nem de chave)
cargo run -p ubiqx-cli -- --ephemeral --fake-ai simulate --seconds 60 --events

# interface no navegador com dados simulados (sem Tauri)
cd apps/desktop && pnpm install && pnpm dev        # http://localhost:1420  (?onboarding=1 mostra o onboarding)

# app desktop em modo dev (macOS)
cd apps/desktop && pnpm tauri dev
```

Variáveis úteis: `UBIQX_LOG=debug` (log), `UBIQX_FAKE_AI=1` (IA falsa no app desktop), `UBIQX_SCRIPTED=1`
(plataforma roteirizada — o app "trabalha sozinho" para demonstração, sempre com IA falsa a menos que
`UBIQX_ALLOW_REAL_AI=1`), `UBIQX_ANTHROPIC_API_KEY` / `UBIQX_OPENAI_API_KEY` / `UBIQX_XAI_API_KEY` (CLI, junto com
`--provider anthropic|openai|xai`).

CLI: `ubiqx status [data]`, `ubiqx classify`, `ubiqx report <data> --category <id>`, `ubiqx monthly --category <id> --year 2026 --month 9`, `ubiqx export`.

## Estado do projeto

MVP v0.1 — pronto para o primeiro teste local no macOS. Ver [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) §8 para as
decisões e os próximos passos (ScreenCaptureKit, eventos de sono/bloqueio de tela, exportação DOCX, Windows/Linux).

## Licença

MIT
