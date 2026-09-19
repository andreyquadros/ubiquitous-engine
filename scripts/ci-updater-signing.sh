#!/usr/bin/env bash
# CI step shared by the three app jobs: decides whether this build can produce a self-installable
# update, and says so loudly either way.
#
# tauri-plugin-updater only installs an artifact whose minisign signature matches the public key in
# apps/desktop/src-tauri/tauri.conf.json (plugins > updater > pubkey), so the private key has to be
# in the environment at build time. When it is not:
#
#   - the build goes on with `--no-sign`, so the DMG, the installers and the release keep coming;
#   - no updater artifact is published, so nobody is offered an update this build cannot prove;
#   - the job log carries a warning naming the two secrets to create.
#
# Nothing is ever published unsigned: an unsigned updater artifact is deleted before upload
# (see the "stable names" step in each app job) and updater.json is simply not written.
#
# Exports for the steps that follow (via $GITHUB_ENV):
#   UBIQX_TAURI_SIGN_FLAG  `--no-sign` when there is no key, empty when there is one
#   UBIQX_UPDATER_SIGNED   1 or 0
#
# Usage: bash scripts/ci-updater-signing.sh
set -uo pipefail

missing=()
[[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]] || missing+=("TAURI_SIGNING_PRIVATE_KEY")
[[ -n "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]] || missing+=("TAURI_SIGNING_PRIVATE_KEY_PASSWORD")

env_file="${GITHUB_ENV:-/dev/null}"

if [[ "${#missing[@]}" -gt 0 ]]; then
  joined="$(printf '%s, ' "${missing[@]}")"
  joined="${joined%, }"
  echo "::warning title=Atualização pelo app desligada neste build::Faltam os secrets ${joined}. O build continua e publica os instaladores, mas sem artefato de atualização assinado: quem já tem o ubiqX vai precisar instalar à mão. Cadastre os dois em Settings -> Secrets and variables -> Actions (docs/MACOS-TESTING.md 3.2) e o próximo build já se atualiza sozinho."
  echo "sem chave de assinatura: build com --no-sign, sem artefato de atualização (faltam ${joined})"
  {
    echo "UBIQX_TAURI_SIGN_FLAG=--no-sign"
    echo "UBIQX_UPDATER_SIGNED=0"
  } >> "$env_file"
  exit 0
fi

echo "chave de assinatura do updater presente: este build se instala sozinho"
{
  echo "UBIQX_TAURI_SIGN_FLAG="
  echo "UBIQX_UPDATER_SIGNED=1"
} >> "$env_file"
