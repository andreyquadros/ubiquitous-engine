#!/usr/bin/env bash
# CI step shared by the three app jobs, by publish-continuous and by Codemagic: stamps the version of this
# build into apps/desktop/src-tauri/tauri.conf.json and exports it as UBIQX_VERSION.
#
# tauri-plugin-updater compares semver, so the version has to grow with every build; scripts/app-version.mjs
# turns the commit count (UBIQX_BUILD_NUMBER, from scripts/build-info.sh) into <major>.<minor>.<number>.
# Running this in every job that builds or publishes is what keeps three numbers equal: the one Settings
# shows, the one in latest.json/updater.json and the one the updater compares. Outside CI, in a checkout
# without git history, it prints the base version and changes nothing.
#
# Usage: bash scripts/ci-app-version.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The build identity may already be in the environment (the "Build identity" step of each job); when it is
# not, ask git for the same numbers, so this script never depends on the step order.
if [[ -z "${UBIQX_BUILD_NUMBER:-}" ]]; then
  eval "$(bash "$ROOT/scripts/build-info.sh")"
fi
export UBIQX_BUILD_NUMBER="${UBIQX_BUILD_NUMBER:-0}"

version="$(node "$ROOT/scripts/app-version.mjs" --write)"
echo "ubiqX $version (build $UBIQX_BUILD_NUMBER)"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "UBIQX_VERSION=$version" >> "$GITHUB_ENV"
fi
if [[ -n "${CM_ENV:-}" ]]; then
  echo "UBIQX_VERSION=$version" >> "$CM_ENV"
fi
