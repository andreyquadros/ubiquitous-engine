# ubiqX — desktop (Tauri 2 + React 19)

Frontend do ubiqX: rastreador de produtividade para macOS com IA nativa e o mascote UBI.

## Scripts

| Comando | O que faz |
|---------|-----------|
| `pnpm dev` | Vite em `http://localhost:1420` com dados **mock** (fora do Tauri). Use `?onboarding=1` para ver o onboarding (`&step=N` abre o passo N) e `?theme=dark` para forçar o tema. |
| `pnpm tauri dev` | App nativo (macOS) com o backend Rust. |
| `pnpm typecheck` | `tsc --noEmit`. |
| `pnpm test -- --run` | Testes (Vitest + Testing Library, jsdom). |
| `pnpm build` | Typecheck + build do Vite em `dist/`. |
| `pnpm screenshots` | Gera as capturas em `docs/screenshots/` (precisa do `pnpm dev` rodando). |
| `pnpm gen-icons` | Regenera `src-tauri/icons/tray.png`, `tray@2x.png` e `app-icon.png`. |

## Estrutura

```
src/
├── lib/         ipc.ts (comandos), types.ts (DTOs), mock.ts (dados fake), store.ts (Zustand), format.ts, theme.ts
├── components/  layout/ (sidebar, shell), ui/ (botões, diálogos…), charts/ (Recharts), ubi/ (mascote: PNG, 3D e SVG)
└── pages/       Hoje, Timeline, Revisão, Relatórios, Categorias, Insights, Configurações, Onboarding
```

## Design

O sistema de design ("painel de instrumentos à noite") está em [`DESIGN.md`](DESIGN.md): cores, tipografia
(Sora + Inter, auto-hospedadas), raios, motion, gráficos e o inventário de componentes. Os tokens vivem em
`src/index.css`; o tema escuro é o padrão e `.dark` no `<html>` é o interruptor. Para revisar uma tela:
`pnpm dev` e abra `http://localhost:1420/?theme=dark` (ou `light`); o onboarding aceita `?onboarding=1&step=N`
(1 a 7). `pnpm screenshots` captura as oito páginas nos dois temas em `docs/screenshots/`.

## Mascote (UBI)

Ordem de preferência: **`public/ubi/Ubi.glb`** (o modelo 3D, carregado direto com o `GLTFLoader` num canvas
transparente sobre a interface; luz de ambiente local, brilho no chão na cor do humor, flutuação, paralaxe e
piscada) → `public/ubi/ubi.png` (a ilustração; o fundo branco é removido pelo app) → SVG interno. Sem WebGL, ou se o
modelo falhar, o app cai para o PNG e depois para o SVG. Com o `ubi.glb` (e/ou o PNG) em `~/Downloads`, rode
`scripts/install-ubi-model.sh` na raiz do repositório (ele copia o `ubi*.glb` e o `ubi*.png` mais recentes) e
reinicie o `pnpm dev`. **Commite os dois arquivos**: os builds na nuvem (GitHub Actions e Codemagic) só empacotam
o que está no repositório. `pnpm hero` regenera `docs/ubi-hero.png` a partir da arte instalada. Detalhes (dicas de
export, `ROTATION_Y`, Draco) em `public/ubi/README.md`.
