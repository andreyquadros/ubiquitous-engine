# Rig e animações do UBI

`rig_ubi.py` transforma o export bruto do UBI (uma malha rígida, sem esqueleto) no `Ubi.glb` que o app usa: uma
malha com esqueleto, oito clipes de animação e texturas otimizadas. Tudo roda sem interface (Blender como módulo
Python), é determinístico e reprodutível: a mesma entrada gera a mesma saída.

## O que o script faz

1. **Importa** o `.glb` bruto e aplica a transformação do objeto (Blender usa Z para cima: o +Y do glTF vira +Z e o
   +Z do glTF, a frente do personagem, vira −Y). A malha fica em unidades de mundo, pés em z = 0.
2. **Acha os pontos de referência (landmarks) na geometria** com numpy: fatias ao longo da altura para achar o
   pescoço (a fatia mais estreita da coluna central entre 45 % e 75 % da altura), a virilha (onde as duas pernas se
   separam), o centro do tronco (meio da extensão da fatia a 30 % da altura), os ombros (borda do tronco a 75 % do tronco); os braços são
   os vértices fora da coluna do tronco (cotovelo = parte mais baixa do braço; mão = extremidade mais distante do
   cotovelo, com o dedo apontando no braço direito e a palma aberta no esquerdo); as pernas são separadas pelo maior
   vão em x abaixo da virilha (quadril, joelho no meio, tornozelo a 7,5 % da altura, dedos na frente do pé). O orbe é a
   segunda ilha da malha (componentes conexos após soldar vértices por posição).
3. **Monta a armadura** com os ossos do contrato e a pose de descanso igual à do export.
4. **Pesos**: rígido-com-mistura por **segmento de osso mais próximo**, calculado em numpy. Cada vértice cai numa
   região (tronco/cabeça, braço L/R, perna L/R, orbe) pelos mesmos cortes que acharam os landmarks e recebe o osso cujo
   segmento está mais perto dentro daquela região — é isso que garante que a mão levantada, colada na cabeça, nunca
   recebe peso de `Head`, e que a faixa laranja não vai para o braço. Perto de cada junta o peso é dividido com o osso
   pai/filho por uma rampa suave (`smoothstep`) dentro do raio `BLEND_RADIUS` (0,08–0,16 unidades, ~3–5 % da altura).
   O orbe é 100 % `Orb`. Os pesos automáticos do Blender (bone heat) foram testados como comparação (veja abaixo).
5. **Clipes**: os oito clipes são criados como *actions* com chaves em quaternion nos pose bones (Bézier com
   `AUTO_CLAMPED`, loops com o modificador *Cycles* e última chave = primeira), 24 fps, cada uma empurrada para uma
   faixa NLA para que o exportador (modo `ACTIONS`) exporte todas como animações glTF separadas.
6. **Exporta** o GLB (`export_apply`, skins, animações amostradas, Y para cima), remove os canais que só repetem a pose
   de descanso (o exportador do Blender chaveia todos os ossos; assim cada clipe contém só os ossos que move e `Root`
   nunca é animado), torna as chaves de rotação contínuas em sinal (o exportador grava `repouso · pose` por amostra
   com `w ≥ 0`; num osso cujo repouso está perto de `w = 0`, como as pernas, a pose cruza esse ponto e as amostras
   viram `-q` — a mesma rotação, mas uma mistura aditiva ou outro motor mostraria um giro; chaves com produto escalar
   negativo em relação à anterior são negadas, a primeira mantém o sinal do repouso do nó) e otimiza com
   `gltf-transform` (`prune`, `dedup`, `weld`, texturas em 1024 px, Draco *edgebreaker*). Uma cópia sem Draco fica em
   `<saída>-nodraco.glb`. Depois verifica: 1 skin com os 21 ossos do contrato, as 8 animações, `JOINTS_0`/`WEIGHTS_0`
   presentes, nenhuma inversão de sinal entre chaves consecutivas, tamanho ≤ 8 MB.
7. Com `--renders DIR`, renderiza com Cycles (CPU, 24 amostras, 512 px) a pose de descanso, a cabeça virada
   (yaw 35°, pitch −20°, repartido 70 % `Head` / 30 % `Neck` como o app faz), o pico do `Wave`, `Sleep` no quadro 48,
   o meio do `Excited`, o ápice do `Jump` e `Worried`, em vista inteira e em close.

Um teste em Chromium (three.js + `DRACOLoader` local) confirmou que o arquivo Draco decodifica com os 21 ossos, os
8 clipes e os mesmos pesos da cópia sem Draco (render idêntico com a cabeça virada e o `Wave` tocando).

## Ossos

Nomes exatos (o app os encontra com `scene.getObjectByName`). `.L`/`.R` são o lado do **personagem** (esquerdo = +X
no glTF, porque ele olha para +Z).

```
Root (na origem, no chão)
└─ Hips
   ├─ Spine ─ Chest
   │          ├─ Neck ─ Head
   │          ├─ Shoulder.L ─ UpperArm.L ─ LowerArm.L ─ Hand.L   (braço esquerdo, estendido, palma sob o orbe)
   │          └─ Shoulder.R ─ UpperArm.R ─ LowerArm.R ─ Hand.R   (braço direito, dobrado, dedo apontando para cima)
   ├─ UpperLeg.L ─ LowerLeg.L ─ Foot.L
   └─ UpperLeg.R ─ LowerLeg.R ─ Foot.R
└─ Orb (rígido, peso 1,0)
```

Cada osso aponta da sua junta para a junta seguinte (convenção do Blender): `Head` vai do pescoço até ~75 % da
altura da cabeça, `Hand.R` vai do pulso à ponta do dedo, `Foot` do tornozelo aos dedos do pé. O app não assume eixos:
o look-at da cabeça é calculado a partir da orientação de mundo do osso.

## Clipes

| Nome | Tipo | Duração | Ossos | Movimento |
| --- | --- | --- | --- | --- |
| `Idle` | loop | 4 s | Chest, Spine, Hips, Neck, Head, UpperArm.L/R, Orb | respiração, leve transferência de peso, micro-balanço da cabeça (≤ 4°), orbe subindo/descendo e orbitando ~0,05 |
| `Yes` | único | 1,2 s | Head, Neck | dois acenos de sim (pitch) |
| `No` | único | 1,2 s | Head, Neck | dois "não" (yaw ±12°) |
| `Wave` | único | 2 s | LowerArm.R, Hand.R, UpperArm.R, Head, Neck | antebraço direito abana para fora (nunca cruza a cara), cabeça inclina |
| `Jump` | único | 1 s | Hips (translação), Spine (escala squash/stretch), Chest, Head, pernas, UpperArm.L/R, Orb | agacha, pula 0,25, aterrissa com squash |
| `Excited` | loop | 1,2 s | Hips (translação), Spine, Chest, Neck, Head, UpperArm.L/R, LowerArm.R, Hand.L, Orb | pulinhos, braços bombeando, orbe orbitando rápido |
| `Worried` | loop | 1,5 s | Shoulder.L/R, Chest, Neck, Head, UpperArm.L/R, Orb | ombros erguidos, tremor rápido (4 Hz) na cabeça e no peito, orbe puxado para perto |
| `Sleep` | loop | 4 s | Spine, Chest, Neck, Head, Shoulder.L/R, UpperArm.L/R, Orb | peito caído ~12°, cabeça baixa e de lado, respiração lenta, orbe baixo |

Os clipes únicos começam e terminam na pose de descanso; os loops fecham sem salto. A 24 fps, 1,2 s vira 29 quadros
(1,208 s). `Root` não tem canal em clipe nenhum; a única translação é em `Hips` (`Jump`, `Excited`). O look-at da
cabeça é procedural no app, aplicado depois do `mixer.update` — por isso os clipes só põem micro-movimentos em
`Head`/`Neck`.

## Como rodar num export novo

Precisa de Python 3.11 (o `bpy` 5.0 só existe para ele), Node 22 + pnpm (para o `gltf-transform`) e ~2 GB livres.

```bash
# uma vez, na raiz do repositório
python3.11 -m venv .venv-bpy
.venv-bpy/bin/pip install -r scripts/ubi-rig/requirements.txt      # baixa o Blender inteiro (~300 MB)

# a cada export novo do UBI (o .glb bruto que sai do Blender/da ferramenta de modelagem)
.venv-bpy/bin/python scripts/ubi-rig/rig_ubi.py ~/Downloads/Ubi.glb apps/desktop/public/ubi/Ubi.glb \
  --renders /tmp/ubi-renders
open /tmp/ubi-renders            # confira as poses antes de commitar
git add apps/desktop/public/ubi/Ubi.glb
```

No Mac com Homebrew: `brew install python@3.11 pnpm`. Se o `pip install bpy` reclamar da versão do Python, confira
`python3.11 --version`. O script imprime o resumo do arquivo final (ossos, clipes, tamanho) e falha se algo do
contrato faltar. Outras opções: `--no-optimize` (só o export do Blender, para depurar), `--texture-size 2048`,
`--blend cena.blend` (salva a cena para abrir no Blender e inspecionar pesos e clipes), `--keep-temp`.

O export bruto deve ser um único `.glb` com o personagem em pé, olhando para +Z, o orbe como ilha separada da malha
(sem tocar o corpo) e o braço direito dobrado com o dedo para cima — é assim que a análise geométrica acha as juntas.

## Ajustando os landmarks

Cada execução grava `<saída>.landmarks.json` com todas as juntas encontradas (em unidades de mundo do Blender:
x = esquerda do personagem, −y = frente, z = para cima), os metadados dos cortes e o mapa osso → (junta inicial,
junta final). Se algum render sair errado (pescoço rasgando, mão deformando com a cabeça, braço dobrando na junta
errada), copie o JSON, edite só as juntas que precisam mudar e rode de novo com `--landmarks meu.json`: as juntas
listadas substituem as calculadas, o resto continua automático.

```json
{ "joints": { "elbow_R": [-0.56, -0.12, 1.21], "wrist_R": [-0.66, -0.53, 1.47] } }
```

Os raios de mistura por junta (`BLEND_RADIUS`) e as proporções dos cortes (altura do pescoço, dos ombros, do
tornozelo) estão no topo do script. Os landmarks usados no `Ubi.glb` atual (export de 3,0 unidades de altura) estão
em `landmarks.json` ao lado deste README.

## Comparação com os pesos automáticos do Blender

`bpy.ops.object.parent_set(type='ARMATURE_AUTO')` (bone heat) foi testado no mesmo esqueleto e **falhou neste
export**: o Blender avisa `Bone Heat Weighting: failed to find solution for one or more bones` e deixa os 89 443
vértices do corpo sem peso nenhum (a malha vem com vértices duplicados nas costuras de UV e várias cascas que se
interpenetram — capacete, visor, anéis, faixa — o que quebra a difusão de calor). Os pesos por segmento não dependem
da topologia, são determinísticos, rápidos (< 1 s) e respeitam as regiões (a mão levantada nunca recebe `Head`), por
isso são os únicos usados.
