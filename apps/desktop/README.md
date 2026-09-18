# ubiqX — desktop (Tauri 2 + React 19)

Frontend do ubiqX: rastreador de produtividade para macOS com IA nativa e o mascote UBI.

## Scripts

| Comando | O que faz |
|---------|-----------|
| `pnpm dev` | Vite em `http://localhost:1420` com dados **mock** (fora do Tauri). Use `?onboarding=1` para ver o onboarding (`&step=N` abre o passo N), `?theme=dark` para forçar o tema, `?update=available` para ver o banner de atualização com uma release fictícia (o build real vem da release `continuous`; ver `docs/MACOS-TESTING.md` § 3.2), `?session=active` para uma sessão de foco em andamento e `?nudge=focus_prompt` para o convite ao foco na bolha do UBI. |
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
└── pages/       Hoje, Timeline, Revisão, Relatórios, Categorias, Insights, Configurações, Foco, Onboarding, Intervention (janela flutuante)
```

## Design

O sistema de design ("painel de instrumentos à noite") está em [`DESIGN.md`](DESIGN.md): cores, tipografia
(Sora + Inter, auto-hospedadas), raios, motion, gráficos e o inventário de componentes. Os tokens vivem em
`src/index.css`; o tema escuro é o padrão e `.dark` no `<html>` é o interruptor. Para revisar uma tela:
`pnpm dev` e abra `http://localhost:1420/?theme=dark` (ou `light`); o onboarding aceita `?onboarding=1&step=N`
(1 a 7). `pnpm screenshots` captura as oito páginas nos dois temas em `docs/screenshots/`.

## Foco

A página **Foco** (`#/focus`, item abaixo de Configurações) reúne três coisas:

- **Bloqueios**: uma busca que encontra apps instalados no Mac (`list_installed_apps`), domínios que você mesmo
  visitou (`list_known_domains`) e, quando o texto parece um domínio, a entrada "Bloquear o site …". Cada resultado
  vira um alvo (`FocusTarget`, chave = bundle id ou domínio). Enquanto o guarda estiver ligado, o motor fecha o app
  ou a aba assim que aparecer na frente e a janela de intervenção mostra o UBI dizendo "Não! Foque na sua
  produtividade." Desativar ou remover um alvo bloqueado nos últimos 15 minutos passa por três avisos do UBI; só o
  terceiro "Desativar mesmo assim" desativa. Fora disso, uma confirmação só.
- **Sessão de foco**: "Que tarefa você precisa fazer agora?", 25 / 45 / 90 min e "Focar". Enquanto ela roda, o item
  Foco ganha um ponto na barra lateral, o Hoje mostra a tarefa com a contagem regressiva e "Encerrar", e o motor
  também segura o que cair em Distração (opcional), esconde as outras janelas e roda um atalho do macOS, se houver.
- **Opções**: guarda ligado/desligado, o que a sessão segura, duração padrão, intervalo entre avisos, os nomes dos
  atalhos e "Testar aviso".

Quando o motor percebe muitas trocas de janela, ele guarda um nudge `focus_prompt` e a bolha do UBI no Hoje vira um
mini formulário ("Me ajude a focar") que inicia a sessão dali mesmo.

No mock: `?session=active` abre com uma sessão de 45 min começada há 12 min; `?nudge=focus_prompt` mostra o convite
no Hoje; `#/intervention?id=test` renderiza a janela de intervenção (460×188) no navegador; no console,
`__ubiqxMock.setFocus({ session: 'active' | null, nudge: true })` e `__ubiqxMock.intervene()` (registra uma
intervenção no primeiro alvo ativo e abre o painel em um popup). "Testar aviso" abre o mesmo popup.

## Mascote (UBI)

Ordem de preferência: **`public/ubi/Ubi.glb`** (o modelo 3D, carregado direto com o `GLTFLoader` num canvas
transparente sobre a interface; luz de ambiente local, brilho no chão na cor do humor, flutuação, paralaxe e
piscada) → `public/ubi/ubi.png` (a ilustração; o fundo branco é removido pelo app) → SVG interno. Sem WebGL, ou se o
modelo falhar, o app cai para o PNG e depois para o SVG. Com o `ubi.glb` (e/ou o PNG) em `~/Downloads`, rode
`scripts/install-ubi-model.sh` na raiz do repositório (ele copia o `ubi*.glb` e o `ubi*.png` mais recentes) e
reinicie o `pnpm dev`. **Commite os dois arquivos**: os builds na nuvem (GitHub Actions e Codemagic) só empacotam
o que está no repositório. `pnpm hero` regenera `docs/ubi-hero.png` a partir da arte instalada. Detalhes (dicas de
export, `ROTATION_Y`, Draco) em `public/ubi/README.md`.

O modelo é **rigado e animado**: o `Ubi.glb` traz um esqueleto (`Root > Hips > Spine > Chest > Neck > Head`, braços,
pernas e o osso `Orb` da esfera) e os clipes `Idle`, `Excited`, `Worried`, `Sleep` (um por humor, em loop) e `Yes`,
`No`, `Wave`, `Jump` (curtos). `src/components/ubi/Ubi3d.tsx` toca o clipe do humor com `AnimationMixer` (crossfade
de 0,35 s), dispara `Yes` quando a fala do balão muda, `Wave` quando o humor vira "empolgado" ou ao clicar no mascote
(`No` se estiver preocupado) e, por cima dos clipes, gira a cabeça (70 % `Head`, 30 % `Neck`) para seguir o mouse em
qualquer ponto da janela e para olhar o balão de dica a cada nova frase e a cada ~10 s. A lógica pura (clipe por
humor, ângulos com limites e amortecimento, agenda das olhadas) fica em `src/components/ubi/rig.ts`. Se o clipe de um
humor faltar, vale o `Idle`; se o export não tiver esqueleto, o app mantém a flutuação do corpo inteiro. Para depurar,
a caixa `data-testid="ubi-3d"` expõe `data-ubi-rig` (1 = esqueleto com `Head`), `data-ubi-clip` (clipe base) e
`data-ubi-look` (yaw,pitch em graus); em `pnpm dev`, `window.__ubiqxUbi.set({ mood: 'worried', speaking: 'Oi' })`
força humor e fala do mascote da página (`reset()` desfaz).
