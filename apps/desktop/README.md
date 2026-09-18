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

Ordem de preferência: **`public/ubi/ubi.png`** (a ilustração oficial; o fundo branco é removido pelo app e ele
ganha flutuação, brilho e paralaxe) → `public/ubi/Ubi.glb` (3D) → SVG interno. Com o PNG em `~/Downloads`, rode
`scripts/install-ubi-model.sh` na raiz do repositório (ele copia o `ubi*.png` mais recente, ou o último `.png`
da pasta) e reinicie o `pnpm dev`. `pnpm hero` regenera `docs/ubi-hero.png` a partir da arte instalada.
Detalhes em `public/ubi/README.md`.
