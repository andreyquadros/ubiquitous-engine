#!/usr/bin/env bash
# Installs UBI's art into the desktop app so the mascot uses the real assets:
#   *.glb → apps/desktop/public/ubi/Ubi.glb   (the 3D model — the primary UBI, loaded with GLTFLoader)
#   *.png → apps/desktop/public/ubi/ubi.png   (the picture; a plain white background is removed by the app)
#
# Usage:
#   scripts/install-ubi-model.sh                       # looks in ~/Downloads for the newest ubi*.glb (ubi.glb, Ubi.glb, "ubi (1).glb"…)
#                                                      # and the newest ubi*.png (else the newest .png)
#   scripts/install-ubi-model.sh ~/Downloads/ubi.glb   # any number of .glb / .png paths
#
# Both files are meant to be committed: cloud builds on GitHub Actions/Codemagic bundle whatever is in
# apps/desktop/public/ubi. Without them the UI draws UBI as SVG.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST_DIR="$ROOT/apps/desktop/public/ubi"
mkdir -p "$DEST_DIR"

install_one() {
  local src="$1"
  if [[ ! -f "$src" ]]; then
    echo "skip: $src is not a file" >&2
    return 1
  fi
  local lower
  lower="$(printf '%s' "${src##*.}" | tr '[:upper:]' '[:lower:]')"
  case "$lower" in
    png)
      cp "$src" "$DEST_DIR/ubi.png"
      echo "✓ picture  → $DEST_DIR/ubi.png ($(du -h "$DEST_DIR/ubi.png" | cut -f1))"
      ;;
    glb)
      cp "$src" "$DEST_DIR/Ubi.glb"
      echo "✓ 3D model → $DEST_DIR/Ubi.glb ($(du -h "$DEST_DIR/Ubi.glb" | cut -f1))"
      if [[ $(stat -f %z "$src" 2>/dev/null || stat -c %s "$src") -gt 10000000 ]]; then
        echo "  The model is over 10 MB: shrink it with scripts/optimize-ubi-model.sh \"$src\" (textures 1024 px + Draco, ~3 MB)."
      fi
      ;;
    *)
      echo "skip: $src (only .glb and .png are UBI assets)" >&2
      return 1
      ;;
  esac
}

# newest file in ~/Downloads matching a case-insensitive glob (prints nothing when there is none)
newest_matching() {
  local dir="$1" pattern="$2" newest=""
  while IFS= read -r f; do
    if [[ -z "$newest" || "$f" -nt "$newest" ]]; then newest="$f"; fi
  done < <(find "$dir" -maxdepth 1 -type f -iname "$pattern" 2>/dev/null)
  printf '%s' "$newest"
}

installed=0
if [[ $# -gt 0 ]]; then
  for p in "$@"; do
    install_one "$p" && installed=$((installed + 1)) || true
  done
else
  DL="$HOME/Downloads"
  # newest ubi*.glb (case-insensitive), e.g. "ubi.glb", "Ubi.glb", "ubi (1).glb", "UBI-final.glb"
  glb="$(newest_matching "$DL" 'ubi*.glb')"
  if [[ -n "$glb" ]]; then
    install_one "$glb" && installed=$((installed + 1)) || true
  fi
  # newest ubi*.png (case-insensitive), e.g. "UBI.png", "ubi (1).png", "Ubi-final.png"
  png="$(newest_matching "$DL" 'ubi*.png')"
  if [[ -z "$png" ]]; then
    # no ubi*.png: fall back to the newest .png in ~/Downloads and say which one was taken
    png="$(newest_matching "$DL" '*.png')"
    if [[ -n "$png" ]]; then
      echo "Nenhum ubi*.png em $DL; usando o PNG mais recente: $(basename "$png")" >&2
    fi
  fi
  if [[ -n "$png" ]]; then
    install_one "$png" && installed=$((installed + 1)) || true
  fi
fi

if [[ $installed -eq 0 ]]; then
  echo "Nothing installed. Put ubi.glb and/or ubi.png in ~/Downloads, or pass the paths as arguments." >&2
  exit 1
fi
echo "Done: $installed asset(s). Restart 'pnpm dev' or rebuild the app to see UBI."
if [[ -f "$DEST_DIR/Ubi.glb" ]]; then
  echo "Commit apps/desktop/public/ubi/Ubi.glb so cloud builds (GitHub Actions, Codemagic) include the 3D UBI."
fi
if [[ -f "$DEST_DIR/ubi.png" ]]; then
  echo "Commit apps/desktop/public/ubi/ubi.png so cloud builds (GitHub Actions, Codemagic) include the art."
fi
