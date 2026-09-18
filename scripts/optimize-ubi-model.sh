#!/usr/bin/env bash
# Shrinks a UBI export for the app: textures resized to 1024 px (JPEG/PNG kept, no new extensions),
# geometry compressed with Draco (the decoder ships in apps/desktop/public/draco). A 34 MB Blender export
# with 4K textures becomes ~3 MB with no visible loss at the sizes the mascot is drawn (≤ 200 px).
#
# Usage:
#   scripts/optimize-ubi-model.sh ~/Downloads/Ubi.glb                 # writes apps/desktop/public/ubi/Ubi.glb
#   scripts/optimize-ubi-model.sh in.glb out.glb                      # explicit output
#   TEXTURE_SIZE=2048 scripts/optimize-ubi-model.sh in.glb            # sharper textures (bigger file)
#
# Needs Node 22 + pnpm (uses `pnpm dlx @gltf-transform/cli`). Commit the result: cloud builds bundle it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="${1:-}"
OUT="${2:-$ROOT/apps/desktop/public/ubi/Ubi.glb}"
SIZE="${TEXTURE_SIZE:-1024}"
if [[ -z "$IN" || ! -f "$IN" ]]; then
  echo "usage: $0 <model.glb> [out.glb]" >&2
  exit 1
fi
command -v pnpm >/dev/null || { echo "pnpm is required (https://pnpm.io)" >&2; exit 1; }

GT=(pnpm dlx @gltf-transform/cli@4)
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "→ $IN ($(du -h "$IN" | cut -f1))"
"${GT[@]}" prune "$IN" "$TMP/1.glb" >/dev/null
"${GT[@]}" dedup "$TMP/1.glb" "$TMP/2.glb" >/dev/null
"${GT[@]}" weld "$TMP/2.glb" "$TMP/3.glb" >/dev/null
"${GT[@]}" resize --width "$SIZE" --height "$SIZE" "$TMP/3.glb" "$TMP/4.glb" >/dev/null
"${GT[@]}" draco --method edgebreaker "$TMP/4.glb" "$OUT" >/dev/null
echo "✓ $OUT ($(du -h "$OUT" | cut -f1)); textures ${SIZE}px, Draco geometry"
if [[ "$OUT" == "$ROOT/apps/desktop/public/ubi/Ubi.glb" ]]; then
  echo "Commit apps/desktop/public/ubi/Ubi.glb so cloud builds (GitHub Actions, Codemagic) include the 3D UBI."
fi
