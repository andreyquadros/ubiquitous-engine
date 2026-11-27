#!/usr/bin/env bash
# CI step shared by the three app jobs, by the publish jobs and by Codemagic: stamps the version of this
# build into the product's tauri.conf.json and exports it as UBIQX_VERSION.
#
# tauri-plugin-updater compares semver, so the version has to grow with every build; scripts/app-version.mjs
# turns the commit count (UBIQX_BUILD_NUMBER, from scripts/build-info.sh) into <major>.<minor>.<number>.
# Running this in every job that builds or publishes is what keeps three numbers equal: the one Settings
# shows, the one in latest.json/updater.json and the one the updater compares. Outside CI, in a checkout
# without git history, it prints the base version and changes nothing.
#
# Two products are built from this workspace. Without an option the script stamps ubiqX
# (apps/desktop/src-tauri/tauri.conf.json), as it always did; `--product shield` stamps the standalone app
# (apps/nofap/src-tauri/tauri.conf.json). Both get the same number for the same commit, so a job that
# builds both calls the script once per product and UBIQX_VERSION is the same after either call.
#
# Usage: bash scripts/ci-app-version.sh [--product ubiqx|shield]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

product="ubiqx"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --product)
      if [[ $# -lt 2 ]]; then
        echo "ci-app-version.sh: --product needs a value (ubiqx or shield)" >&2
        exit 2
      fi
      product="$2"
      shift 2
      ;;
    --product=*)
      product="${1#--product=}"
      shift
      ;;
    *)
      echo "ci-app-version.sh: unknown option: $1" >&2
      exit 2
      ;;
  esac
done

# The name the scripts print for each product (`label` of PRODUCTS in scripts/app-version.mjs).
case "$product" in
  ubiqx) label="ubiqX" ;;
  shield) label="ubiqx-shield" ;;
  *)
    echo "ci-app-version.sh: unknown product: $product (use ubiqx or shield)" >&2
    exit 2
    ;;
esac

# The build identity may already be in the environment (the "Build identity" step of each job); when it is
# not, ask git for the same numbers, so this script never depends on the step order.
if [[ -z "${UBIQX_BUILD_NUMBER:-}" ]]; then
  eval "$(bash "$ROOT/scripts/build-info.sh")"
fi
export UBIQX_BUILD_NUMBER="${UBIQX_BUILD_NUMBER:-0}"

version="$(node "$ROOT/scripts/app-version.mjs" --write --product "$product")"
echo "$label $version (build $UBIQX_BUILD_NUMBER)"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "UBIQX_VERSION=$version" >> "$GITHUB_ENV"
fi
if [[ -n "${CM_ENV:-}" ]]; then
  echo "UBIQX_VERSION=$version" >> "$CM_ENV"
fi
