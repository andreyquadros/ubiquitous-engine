#!/usr/bin/env bash
# Prints the build identity of the checked-out commit as shell exports, so CI and the
# desktop build (apps/desktop/src-tauri/build.rs) stamp the very same numbers:
#
#   UBIQX_BUILD_EPOCH   unix seconds of the commit (git log -1 --format=%ct); 0 when unknown
#   UBIQX_BUILD_NUMBER  commits reachable from HEAD (git rev-list --count HEAD); 0 when unknown
#   UBIQX_BUILD_SHA     short 7-char sha; "dev" when unknown
#   UBIQX_BUILD_BRANCH  GITHUB_REF_NAME, CM_BRANCH or the current git branch; "" when unknown
#
# Usage:
#   eval "$(bash scripts/build-info.sh)"
#
# A shallow clone (fetch-depth 1) would count a single commit, so the script first tries to
# unshallow it; when that is not possible the numbers still come out, just smaller.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

epoch=0
number=0
sha=dev
branch=""

if command -v git >/dev/null 2>&1 && git rev-parse --git-dir >/dev/null 2>&1; then
  if [[ "$(git rev-parse --is-shallow-repository 2>/dev/null || echo false)" == "true" ]]; then
    git fetch --unshallow --quiet >/dev/null 2>&1 || true
  fi
  epoch="$(git log -1 --format=%ct 2>/dev/null || echo 0)"
  number="$(git rev-list --count HEAD 2>/dev/null || echo 0)"
  sha="$(git rev-parse --short=7 HEAD 2>/dev/null || echo dev)"
  branch="$(git symbolic-ref --short -q HEAD 2>/dev/null || true)"
fi

# CI checkouts are usually detached; the platform knows the branch better than git does.
if [[ -n "${GITHUB_REF_NAME:-}" ]]; then
  branch="$GITHUB_REF_NAME"
elif [[ -n "${CM_BRANCH:-}" ]]; then
  branch="$CM_BRANCH"
fi

[[ "$epoch" =~ ^[0-9]+$ ]] || epoch=0
[[ "$number" =~ ^[0-9]+$ ]] || number=0
[[ -n "$sha" ]] || sha=dev

printf 'export UBIQX_BUILD_EPOCH=%s\n' "$epoch"
printf 'export UBIQX_BUILD_NUMBER=%s\n' "$number"
printf 'export UBIQX_BUILD_SHA=%q\n' "$sha"
printf 'export UBIQX_BUILD_BRANCH=%q\n' "$branch"
