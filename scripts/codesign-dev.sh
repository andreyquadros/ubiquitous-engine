#!/usr/bin/env bash
# Signs a locally built ubiqX binary/app with a stable identity so macOS TCC grants
# (Screen Recording, Automation) and Keychain items survive rebuilds.
#
# Usage:
#   scripts/codesign-dev.sh                      # signs the release .app produced by `pnpm tauri build`
#   scripts/codesign-dev.sh path/to/ubiqX.app    # signs a specific bundle
#   UBIQX_SIGN_IDENTITY="Apple Development: ..." scripts/codesign-dev.sh
#
# Create the default self-signed identity once (Keychain Access → Certificate Assistant →
# Create a Certificate… → Name "ubiqX Dev", Type "Code Signing").
set -euo pipefail

IDENTITY="${UBIQX_SIGN_IDENTITY:-ubiqX Dev}"
TARGET="${1:-}"

if [[ -z "$TARGET" ]]; then
  ROOT="$(cd "$(dirname "$0")/.." && pwd)"
  TARGET="$(ls -d "$ROOT"/target/release/bundle/macos/*.app 2>/dev/null | head -n1 || true)"
fi

if [[ -z "$TARGET" || ! -e "$TARGET" ]]; then
  echo "Nothing to sign. Build first with: cd apps/desktop && pnpm tauri build" >&2
  exit 1
fi

if ! security find-identity -v -p codesigning | grep -q "$IDENTITY"; then
  echo "Signing identity '$IDENTITY' not found in your keychain." >&2
  echo "Create it in Keychain Access (Certificate Assistant → Create a Certificate…, type Code Signing)" >&2
  echo "or set UBIQX_SIGN_IDENTITY to an existing identity." >&2
  exit 1
fi

echo "Signing $TARGET with '$IDENTITY'…"
codesign --force --deep --options runtime --sign "$IDENTITY" "$TARGET"
codesign --verify --deep --strict "$TARGET"
echo "OK. Grant Screen Recording once to this app; the grant now survives rebuilds."
