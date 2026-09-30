#!/usr/bin/env node
// A version number per build, so the in-app updater has something to compare.
//
// tauri-plugin-updater compares semver: the running app's version (apps/desktop/src-tauri/tauri.conf.json,
// which is also what Settings shows and what `app.package_info().version` returns) against the `version` of
// the updater feed. A version frozen at 0.1.0 would never look newer, so CI stamps a monotonic number into
// tauri.conf.json before building:
//
//   <major>.<minor>.<UBIQX_BUILD_NUMBER>     e.g. 0.1.0 + build 128 -> 0.1.128
//
// The build number is the commit count of the built commit (`git rev-list --count HEAD`, what
// scripts/build-info.sh exports and what the app already shows as "Número"). It only grows, and it is the
// same number for every machine that builds that commit, so the GitHub Actions jobs and a Codemagic build of
// the same push agree on the version instead of racing. `GITHUB_RUN_NUMBER` is a fallback for a checkout
// without history. The three app jobs and the publish job all go through scripts/ci-app-version.sh, so the
// number the user sees, the number in both feeds and the number the updater compares are the same one.
//
// Usage:
//   node scripts/app-version.mjs            # prints the version of this build
//   node scripts/app-version.mjs --write    # also writes it into apps/desktop/src-tauri/tauri.conf.json
//
// In CI, call it through scripts/ci-app-version.sh: that one fills UBIQX_BUILD_NUMBER from
// scripts/build-info.sh first and exports the result as UBIQX_VERSION.
//
// Node 22, ESM, no dependencies. Importable: `appVersion(env, root)` and `writeAppVersion(root, version)`.

import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');

/** Where the app version lives; `tauri build` reads it and `build.rs` bakes it into the binary. */
export const TAURI_CONF = 'apps/desktop/src-tauri/tauri.conf.json';

export const USAGE = `Uso: node scripts/app-version.mjs [--write] [--root <dir>]

Imprime a versão deste build (<major>.<minor>.<número do build>) e, com --write, grava-a em
${TAURI_CONF}. Sem um número de build (fora da CI) imprime a versão base, sem alterar nada.

Ambiente:
  UBIQX_BUILD_NUMBER  contagem de commits do build (eval "$(bash scripts/build-info.sh)"); vira o patch
  GITHUB_RUN_NUMBER   reserva, quando o checkout não tem histórico
  UBIQX_VERSION       versão explícita, tem precedência sobre tudo
`;

/** The `<major>.<minor>` this product counts from: `[workspace.package] version` in Cargo.toml. */
export function baseVersion(root) {
  try {
    const cargo = readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
    const section = cargo.split(/^\[workspace\.package\]\s*$/m)[1] ?? '';
    const m = section.match(/^version\s*=\s*"([^"]+)"/m);
    if (m) return m[1];
  } catch {
    /* fall through */
  }
  return tauriConfVersion(root) ?? '0.1.0';
}

/** The `version` currently written in tauri.conf.json, or null when it cannot be read. */
export function tauriConfVersion(root) {
  try {
    const conf = JSON.parse(readFileSync(path.join(root, TAURI_CONF), 'utf8'));
    return conf.version ? String(conf.version) : null;
  } catch {
    return null;
  }
}

/**
 * The monotonic number the patch comes from: the commit count of the built commit
 * (`UBIQX_BUILD_NUMBER`, exported by scripts/build-info.sh), or the CI run number when the checkout has no
 * history. `null` outside CI, where nothing is stamped.
 */
export function buildNumber(env = process.env) {
  for (const name of ['UBIQX_BUILD_NUMBER', 'GITHUB_RUN_NUMBER']) {
    const n = Number(String(env[name] ?? '').trim());
    if (Number.isInteger(n) && n > 0) return n;
  }
  return null;
}

/**
 * The version of this build: `UBIQX_VERSION` when set, else `<major>.<minor>.<build number>`, else the base
 * version unchanged (a local build keeps 0.1.0 and never pretends to be a published one).
 */
export function appVersion(env = process.env, root = REPO_ROOT) {
  const explicit = String(env.UBIQX_VERSION ?? '').trim();
  if (explicit) return explicit;
  const base = baseVersion(root);
  const number = buildNumber(env);
  if (number === null) return base;
  const [major = '0', minor = '1'] = base.split('.');
  return `${major}.${minor}.${number}`;
}

/**
 * Rewrites the top-level `"version"` of tauri.conf.json in place, leaving every other byte of the file
 * alone (it is hand-formatted and reviewed). Returns true when the file changed.
 */
export function writeAppVersion(root, version) {
  const file = path.join(root, TAURI_CONF);
  const raw = readFileSync(file, 'utf8');
  let replaced = false;
  const next = raw.replace(/^(\s*"version"\s*:\s*")([^"]*)(")/m, (_all, head, current, tail) => {
    replaced = true;
    return current === version ? `${head}${current}${tail}` : `${head}${version}${tail}`;
  });
  if (!replaced) throw new Error(`Não achei o campo "version" em ${TAURI_CONF}`);
  if (next === raw) return false;
  writeFileSync(file, next);
  return true;
}

export function main(argv = process.argv.slice(2), env = process.env, { log = console.log, error = console.error } = {}) {
  let write = false;
  let root = REPO_ROOT;
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '-h' || arg === '--help') {
      log(USAGE);
      return 0;
    }
    if (arg === '--write') {
      write = true;
    } else if (arg === '--root') {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) {
        error('Falta o valor de --root');
        return 2;
      }
      root = value;
      i += 1;
    } else {
      error(`Opção desconhecida: ${arg}`);
      error(USAGE);
      return 2;
    }
  }
  const version = appVersion(env, root);
  if (write) {
    try {
      writeAppVersion(root, version);
    } catch (err) {
      error(err.message);
      return 1;
    }
  }
  log(version);
  return 0;
}

const invokedDirectly = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) process.exit(main());
