#!/usr/bin/env node
// A version number per build, so the in-app updater has something to compare.
//
// tauri-plugin-updater compares semver: the running app's version (the product's tauri.conf.json, which is
// also what Settings shows and what `app.package_info().version` returns) against the `version` of the
// updater feed. A version frozen at 0.1.0 would never look newer, so CI stamps a monotonic number into
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
// Two products are built from this workspace (crates/ubiqx-core/src/product.rs): ubiqX and the standalone
// app whose machine id is `ubiqx-shield`. Both count from the same `[workspace.package] version` and the
// same commit count, so one commit gives both the same version; what differs is the tauri.conf.json the
// number is written into. `--product ubiqx|shield` picks it (default `ubiqx`, so a call without the option
// does what it always did). The table of the two products, [`PRODUCTS`], lives here and is what
// scripts/publish-release.mjs publishes by.
//
// Usage:
//   node scripts/app-version.mjs                            # prints the version of this build
//   node scripts/app-version.mjs --write                    # also writes it into apps/desktop/src-tauri/tauri.conf.json
//   node scripts/app-version.mjs --write --product shield   # … into apps/nofap/src-tauri/tauri.conf.json
//
// In CI, call it through scripts/ci-app-version.sh: that one fills UBIQX_BUILD_NUMBER from
// scripts/build-info.sh first and exports the result as UBIQX_VERSION.
//
// Node 22, ESM, no dependencies. Importable: `appVersion(env, root)`, `writeAppVersion(root, version, product)`
// and `PRODUCTS` / `resolveProduct(id)`.

import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');

/**
 * The two products built from this workspace, by the id of `ubiqx_core::product::ProductId`. Everything the
 * release scripts need to keep them apart is one row, and scripts/publish-release.test.mjs compares the rows
 * with crates/ubiqx-core/src/product.rs and with each shell's tauri.conf.json.
 *
 * - `label`: how the scripts name the product in what they print.
 * - `tauriConf`: where its version lives; `tauri build` reads it and `build.rs` bakes it into the binary.
 * - `assetPrefix`: the start of every published file name (`<prefix>-<os>-<arch>.<ext>`). The standalone
 *   app's is its machine id, never its display name (docs/nofap/DESIGN.md 0.1 and 8.6).
 * - `feedProduct`: the `product` field of its latest.json. The app ignores a feed that names another
 *   product (`ubiqx_core::update::feed_matches_product`), so this is `ProductIdentity::feed_product`.
 * - `tag`: its rolling release. The updater endpoint and the default feed URL of each shell point there.
 * - `releaseName`: the title of that release on GitHub.
 * - `defaultDmg`: what a call without --asset and --dmg publishes (the historical macOS-only call of
 *   ubiqX); `null` where there is no such call.
 */
export const PRODUCTS = Object.freeze({
  ubiqx: Object.freeze({
    id: 'ubiqx',
    label: 'ubiqX',
    tauriConf: 'apps/desktop/src-tauri/tauri.conf.json',
    assetPrefix: 'ubiqX',
    feedProduct: 'ubiqX',
    tag: 'continuous',
    releaseName: 'ubiqX (build contínuo)',
    defaultDmg: 'target/release/bundle/dmg/ubiqX*.dmg',
  }),
  shield: Object.freeze({
    id: 'shield',
    label: 'ubiqx-shield',
    tauriConf: 'apps/nofap/src-tauri/tauri.conf.json',
    assetPrefix: 'ubiqx-shield',
    feedProduct: 'ubiqx-shield',
    tag: 'shield-continuous',
    releaseName: 'ubiqx-shield (build contínuo)',
    defaultDmg: null,
  }),
});

export const DEFAULT_PRODUCT = 'ubiqx';

/** The row of `id`; throws for anything that is not a product id (`ubiqx`, `shield`). */
export function resolveProduct(id = DEFAULT_PRODUCT) {
  if (Object.hasOwn(PRODUCTS, id)) return PRODUCTS[id];
  throw new Error(`Produto desconhecido: "${id}" (use ${Object.keys(PRODUCTS).join(' ou ')})`);
}

/** Where ubiqX's version lives (kept for callers that predate `--product`). */
export const TAURI_CONF = PRODUCTS.ubiqx.tauriConf;

export const USAGE = `Uso: node scripts/app-version.mjs [--write] [--product ubiqx|shield] [--root <dir>]

Imprime a versão deste build (<major>.<minor>.<número do build>) e, com --write, grava-a no
tauri.conf.json do produto (${PRODUCTS.ubiqx.tauriConf} para ubiqx, o padrão;
${PRODUCTS.shield.tauriConf} para shield). Os dois produtos têm a mesma versão no mesmo
commit. Sem um número de build (fora da CI) imprime a versão base, sem alterar nada.

Ambiente:
  UBIQX_BUILD_NUMBER  contagem de commits do build (eval "$(bash scripts/build-info.sh)"); vira o patch
  GITHUB_RUN_NUMBER   reserva, quando o checkout não tem histórico
  UBIQX_VERSION       versão explícita, tem precedência sobre tudo
`;

/** The `<major>.<minor>` both products count from: `[workspace.package] version` in Cargo.toml. */
export function baseVersion(root, product = DEFAULT_PRODUCT) {
  try {
    const cargo = readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
    const section = cargo.split(/^\[workspace\.package\]\s*$/m)[1] ?? '';
    const m = section.match(/^version\s*=\s*"([^"]+)"/m);
    if (m) return m[1];
  } catch {
    /* fall through */
  }
  return tauriConfVersion(root, product) ?? '0.1.0';
}

/** The `version` currently written in the product's tauri.conf.json, or null when it cannot be read. */
export function tauriConfVersion(root, product = DEFAULT_PRODUCT) {
  try {
    const conf = JSON.parse(readFileSync(path.join(root, resolveProduct(product).tauriConf), 'utf8'));
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
export function appVersion(env = process.env, root = REPO_ROOT, product = DEFAULT_PRODUCT) {
  const explicit = String(env.UBIQX_VERSION ?? '').trim();
  if (explicit) return explicit;
  const base = baseVersion(root, product);
  const number = buildNumber(env);
  if (number === null) return base;
  const [major = '0', minor = '1'] = base.split('.');
  return `${major}.${minor}.${number}`;
}

/**
 * Rewrites the top-level `"version"` of the product's tauri.conf.json in place, leaving every other byte of
 * the file alone (it is hand-formatted and reviewed). Returns true when the file changed.
 */
export function writeAppVersion(root, version, product = DEFAULT_PRODUCT) {
  const conf = resolveProduct(product).tauriConf;
  const file = path.join(root, conf);
  const raw = readFileSync(file, 'utf8');
  let replaced = false;
  const next = raw.replace(/^(\s*"version"\s*:\s*")([^"]*)(")/m, (_all, head, current, tail) => {
    replaced = true;
    return current === version ? `${head}${current}${tail}` : `${head}${version}${tail}`;
  });
  if (!replaced) throw new Error(`Não achei o campo "version" em ${conf}`);
  if (next === raw) return false;
  writeFileSync(file, next);
  return true;
}

export function main(argv = process.argv.slice(2), env = process.env, { log = console.log, error = console.error } = {}) {
  let write = false;
  let root = REPO_ROOT;
  let product = DEFAULT_PRODUCT;
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '-h' || arg === '--help') {
      log(USAGE);
      return 0;
    }
    if (arg === '--write') {
      write = true;
    } else if (arg === '--root' || arg === '--product') {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) {
        error(`Falta o valor de ${arg}`);
        return 2;
      }
      if (arg === '--root') root = value;
      else product = value;
      i += 1;
    } else if (arg.startsWith('--product=')) {
      product = arg.slice('--product='.length);
    } else {
      error(`Opção desconhecida: ${arg}`);
      error(USAGE);
      return 2;
    }
  }
  try {
    resolveProduct(product);
  } catch (err) {
    error(err.message);
    error(USAGE);
    return 2;
  }
  const version = appVersion(env, root, product);
  if (write) {
    try {
      writeAppVersion(root, version, product);
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
