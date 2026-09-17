# Modelo 3D do UBI

Coloque o arquivo **`Ubi.glb`** nesta pasta (`apps/desktop/public/ubi/Ubi.glb`) para que o mascote
seja renderizado em 3D (React Three Fiber + drei `useGLTF`).

Na raiz do repositório há um script que copia o modelo de `~/Downloads`:

```bash
scripts/install-ubi-model.sh            # usa ~/Downloads/Ubi.glb
scripts/install-ubi-model.sh /caminho/para/Ubi.glb
```

Sem o arquivo (ou sem WebGL), a interface usa automaticamente a versão em **SVG** do UBI
(`src/components/ubi/UbiSvg.tsx`) — nada quebra. O `.glb` é ignorado pelo git; cada máquina precisa ter o seu.

Dicas para o modelo: exporte com o personagem centralizado na origem, olhando para +Z, tamanho ~2 unidades,
texturas embutidas (glTF binário) e, se possível, comprimido com Draco desativado (o loader padrão não usa Draco).
