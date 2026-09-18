#!/usr/bin/env bash
# Installs UBI's art into the desktop app so the mascot uses the real assets:
#   *.png → apps/desktop/public/ubi/ubi.png   (the picture; a plain white background is removed by the app)
#   *.glb → apps/desktop/public/ubi/Ubi.glb   (the 3D model, used when no PNG is installed)
#
# Usage:
#   scripts/install-ubi-model.sh                       # looks in ~/Downloads for Ubi.glb and the newest ubi*.png (else the newest .png)
#   scripts/install-ubi-model.sh ~/Downloads/ubi.png   # any number of .png / .glb paths
#
# ubi.png is meant to be committed (cloud builds on GitHub Actions/Codemagic bundle it); Ubi.glb stays
# git-ignored and each machine installs its own. Without them the UI draws UBI as SVG.
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
      ;;
    *)
      echo "skip: $src (only .png and .glb are UBI assets)" >&2
      return 1
      ;;
  esac
}

installed=0
if [[ $# -gt 0 ]]; then
  for p in "$@"; do
    install_one "$p" && installed=$((installed + 1)) || true
  done
else
  DL="$HOME/Downloads"
  if [[ -f "$DL/Ubi.glb" ]]; then
    install_one "$DL/Ubi.glb" && installed=$((installed + 1)) || true
  fi
  # newest ubi*.png (case-insensitive), e.g. "UBI.png", "ubi (1).png", "Ubi-final.png"
  newest=""
  while IFS= read -r f; do
    if [[ -z "$newest" || "$f" -nt "$newest" ]]; then newest="$f"; fi
  done < <(find "$DL" -maxdepth 1 -type f \( -iname 'ubi*.png' \) 2>/dev/null)
  if [[ -z "$newest" ]]; then
    # no ubi*.png: fall back to the newest .png in ~/Downloads and say which one was taken
    while IFS= read -r f; do
      if [[ -z "$newest" || "$f" -nt "$newest" ]]; then newest="$f"; fi
    done < <(find "$DL" -maxdepth 1 -type f -iname '*.png' 2>/dev/null)
    if [[ -n "$newest" ]]; then
      echo "Nenhum ubi*.png em $DL; usando o PNG mais recente: $(basename "$newest")" >&2
    fi
  fi
  if [[ -n "$newest" ]]; then
    install_one "$newest" && installed=$((installed + 1)) || true
  fi
fi

if [[ $installed -eq 0 ]]; then
  echo "Nothing installed. Put ubi.png and/or Ubi.glb in ~/Downloads, or pass the paths as arguments." >&2
  exit 1
fi
echo "Done: $installed asset(s). Restart 'pnpm dev' or rebuild the app to see UBI."
if [[ -f "$DEST_DIR/ubi.png" ]]; then
  echo "Commit apps/desktop/public/ubi/ubi.png so cloud builds (GitHub Actions, Codemagic) include the art."
fi
