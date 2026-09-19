#!/usr/bin/env bash
# CI step shared by the three app jobs: stops the build, with a message naming what is missing, when the
# updater signing secrets are not configured.
#
# tauri-plugin-updater only installs an artifact whose minisign signature matches the public key in
# apps/desktop/src-tauri/tauri.conf.json (plugins > updater > pubkey). The Tauri CLI would refuse to bundle
# anyway ("A public key has been found, but no private key"), but that message is buried in the build log:
# this step puts the failure at the top of the job and says exactly which secrets to create.
#
# Usage: bash scripts/ci-require-signing-key.sh
set -uo pipefail

missing=()
[[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]] || missing+=("TAURI_SIGNING_PRIVATE_KEY")
[[ -n "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]] || missing+=("TAURI_SIGNING_PRIVATE_KEY_PASSWORD")

if [[ "${#missing[@]}" -gt 0 ]]; then
  joined="$(printf '%s, ' "${missing[@]}")"
  joined="${joined%, }"
  echo "::error title=Chave de assinatura do updater ausente::Faltam os secrets ${joined}. Cadastre-os em Settings -> Secrets and variables -> Actions (docs/MACOS-TESTING.md 3.2). Sem eles o build sairia sem assinatura e o ubiqX recusaria a atualizacao, entao este job para aqui."
  echo "Faltam os secrets: ${joined}" >&2
  exit 1
fi

echo "chave de assinatura do updater presente (TAURI_SIGNING_PRIVATE_KEY + senha)"
