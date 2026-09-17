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
├── components/  layout/ (sidebar, shell), ui/ (botões, diálogos…), charts/ (Recharts), ubi/ (mascote 3D + SVG)
└── pages/       Hoje, Timeline, Revisão, Relatórios, Categorias, Insights, Configurações, Onboarding
```

## Mascote 3D

Coloque `Ubi.glb` em `public/ubi/` (ou rode `scripts/install-ubi-model.sh` na raiz do repositório).
Sem o arquivo, o UBI é desenhado em SVG.
