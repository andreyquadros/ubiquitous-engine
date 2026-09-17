#!/usr/bin/env bash
# Copies the UBI 3D model (Ubi.glb) into the desktop app's public folder so the mascot
# renders in 3D. Without it the UI falls back to the built-in SVG UBI.
#
# Usage: scripts/install-ubi-model.sh [path/to/Ubi.glb]   (default: ~/Downloads/Ubi.glb)
set -euo pipefail
SRC="${1:-$HOME/Downloads/Ubi.glb}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/apps/desktop/public/ubi/Ubi.glb"
if [[ ! -f "$SRC" ]]; then
  echo "Model not found at $SRC" >&2
  exit 1
fi
mkdir -p "$(dirname "$DEST")"
cp "$SRC" "$DEST"
echo "UBI model installed at $DEST ($(du -h "$DEST" | cut -f1))."
