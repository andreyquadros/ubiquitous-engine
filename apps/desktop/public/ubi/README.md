# Arte do UBI

O mascote é renderizado, nesta ordem de preferência:

1. **`Ubi.glb`** — o modelo 3D, carregado direto com o `GLTFLoader` do Three.js (React Three Fiber) num canvas
   transparente (`alpha: true`, alpha pré-multiplicado + antialias), então ele se sobrepõe à interface sem contorno
   pixelizado. Ganha luz de ambiente (RoomEnvironment, sem rede), luz de preenchimento e brilho no chão na cor do
   humor, flutuação com o ritmo do humor, paralaxe leve ao passar o mouse e uma piscada nos materiais emissivos
   (`eye`, `visor`, `crest`, `glow`). Sem WebGL (ou se o arquivo não carregar) cai para o item seguinte.
2. **`ubi.png`** — a ilustração oficial (robô ninja branco com visor preto). O app remove o fundo branco
   sozinho (preenchimento a partir das bordas, com 1–2 px de suavização), então pode ser o PNG "como veio".
   Também é o avatar da cabeça no pill "Rastreando" (variante `flat`, sem WebGL).
3. **SVG** (`src/components/ubi/UbiSvg.tsx`) — desenho interno, usado quando nada foi instalado.

Na raiz do repositório há um script que instala os dois arquivos de uma vez:

```bash
scripts/install-ubi-model.sh                 # procura o ubi*.glb e o ubi*.png mais recentes em ~/Downloads
scripts/install-ubi-model.sh ~/Downloads/ubi.glb ~/Downloads/ubi.png
```

**Os dois arquivos devem ser commitados** (`public/ubi/Ubi.glb` e `public/ubi/ubi.png`): são eles que os builds na
nuvem (GitHub Actions e Codemagic) empacotam no `.app`. Depois de instalar, reinicie o `pnpm dev` (ou gere o app de
novo). `pnpm hero` regenera `docs/ubi-hero.png` a partir da arte instalada.

## Rig e animações

O `Ubi.glb` do app não é o export bruto: ele passa por `scripts/ubi-rig/rig_ubi.py`, que lê o export (uma malha
rígida) e devolve o modelo com esqueleto (21 ossos: `Root`, `Hips`, `Spine`, `Chest`, `Neck`, `Head`, braços e pernas
`.L`/`.R`, `Orb`) e oito clipes (`Idle`, `Yes`, `No`, `Wave`, `Jump`, `Excited`, `Worried`, `Sleep`), já otimizado
(texturas 512 px, geometria quantizada). O app toca os clipes por humor e vira a cabeça para o mouse e para o balão de fala. Para
gerar de novo a partir de um export novo, siga `scripts/ubi-rig/README.md` (Blender como módulo Python, sem
interface). Sem esqueleto no arquivo o app volta ao movimento de corpo inteiro.

Dicas para exportar o modelo (glTF binário, `.glb`):

- personagem centralizado na origem e em pé sobre o chão — o app recalcula a caixa, centraliza, apoia os pés em
  y = 0 e escala a maior dimensão para ~2,4 unidades, então o tamanho do export não importa;
- olhando para +Z (de frente para a câmera). Se o export olhar para outro lado, ajuste `ROTATION_Y` em
  `src/components/ubi/Ubi3d.tsx` (`Math.PI` para um modelo de costas);
- texturas embutidas no `.glb` (nada é buscado na rede: o CSP do app bloqueia);
- **não use Draco nem meshopt**: os dois exigem um decodificador que o three.js roda num Worker criado a
  partir de uma URL `blob:`, e o WebKit (o motor do app no macOS) recusa isso pela política de conteúdo do
  app — o modelo não carregava e o mascote caía no desenho achatado em todo Mac. Use quantização
  (`KHR_mesh_quantization`), que o three.js lê sozinho; ela preserva `JOINTS_0`/`WEIGHTS_0`, o
  esqueleto e as animações);
- mantenha o arquivo abaixo de ~10 MB (ele vai no bundle do app e no repositório). Um export grande passa por
  `scripts/optimize-ubi-model.sh export.glb`: texturas em 512 px e geometria quantizada (o `Ubi.glb` atual veio de
  um export de 34 MB com texturas 4K e ficou com ~3 MB, sem diferença visível no tamanho em que o UBI aparece);
- materiais de olhos/visor/crista com nome contendo `eye`, `visor`, `crest`, `glow` ou `emiss` recebem a cor do humor
  como emissiva e piscam; os demais ficam como exportados.
