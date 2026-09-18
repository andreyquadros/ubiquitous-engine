# Arte do UBI

O mascote é renderizado, nesta ordem de preferência:

1. **`ubi.png`** — a ilustração oficial (robô ninja branco com visor preto). O app remove o fundo branco
   sozinho (preenchimento a partir das bordas, com 1–2 px de suavização), então pode ser o PNG "como veio".
   Ganha flutuação, brilho no chão na cor do humor, sombra e um paralaxe leve ao passar o mouse.
2. **`Ubi.glb`** — o modelo 3D (React Three Fiber + drei `useGLTF`), usado quando não há PNG.
3. **SVG** (`src/components/ubi/UbiSvg.tsx`) — desenho interno, usado quando nada foi instalado ou não há WebGL.

Na raiz do repositório há um script que instala os dois arquivos de uma vez:

```bash
scripts/install-ubi-model.sh                 # procura ~/Downloads/Ubi.glb e o ubi*.png mais recente
scripts/install-ubi-model.sh ~/Downloads/ubi.png ~/Downloads/Ubi.glb
```

O `ubi.png` deve ser commitado: é ele que os builds na nuvem (GitHub Actions e Codemagic) empacotam no
`.app`. O `Ubi.glb` continua ignorado pelo git (cada máquina instala o seu). Depois de instalar, reinicie o
`pnpm dev` (ou gere o app de novo). `pnpm hero` regenera `docs/ubi-hero.png` a partir da arte instalada.

Dicas para o modelo: exporte com o personagem centralizado na origem, olhando para +Z, tamanho ~2 unidades,
texturas embutidas (glTF binário). O decoder Draco local está em `public/draco/`.
