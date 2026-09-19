#!/usr/bin/env node
// Publishes the rolling "continuous" GitHub release the desktop app polls for updates.
//
// Each CI build of a commit calls this script with the installers it just produced (one or several
// --asset, platform and kind inferred from the file name). The script moves the release tag to the
// built commit, rewrites the release name and body from the commit message, replaces the assets
// under stable names and uploads latest.json, the feed the app reads (apps/desktop/src-tauri reads
// UBIQX_UPDATE_FEED_URL, whose default points here):
//
//   https://github.com/<owner>/<repo>/releases/download/continuous/latest.json
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-macos-<arch>.dmg
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-macos-<arch>.app.zip
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-windows-<arch>-setup.exe
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-windows-<arch>.msi
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-linux-<arch>.AppImage
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-linux-<arch>.deb
//
// latest.json keeps one entry per platform key (darwin-<arch>, windows-<arch>, linux-<arch>). Several
// publishers may run for the same commit (GitHub Actions with the three OSes at once, Codemagic with
// macOS only): when the release already carries a latest.json of the SAME build epoch, its platforms are
// merged with ours (ours win per key); when ours is newer it replaces the feed; when it is older nothing
// is touched.
//
// A SECOND feed, updater.json, is published next to it whenever the build produced updater artifacts
// (`bundle.createUpdaterArtifacts` in tauri.conf.json). It is the static JSON tauri-plugin-updater reads —
// {version, notes, pub_date, platforms: {"darwin-aarch64": {signature, url}, …}} — and it points at the
// artifacts the plugin installs by itself, each with the minisign signature Tauri wrote next to it:
//
//   https://github.com/<owner>/<repo>/releases/download/continuous/updater.json
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-macos-<arch>.app.tar.gz (+ .sig)
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-windows-<arch>-setup.nsis.zip (+ .sig)
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-linux-<arch>.AppImage.tar.gz (+ .sig)
//
// latest.json and its schema never change: the Rust checker, the banner and the tray still read that one,
// and the manual installers stay published under their stable names.
//
// Node 22, ESM, no dependencies. Importable: `main(argv, env)` returns the exit code and only
// runs when the file is executed directly, so scripts/publish-release.test.mjs can drive it
// against an in-process mock of the GitHub API through --api and --uploads.

import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');

const DEFAULTS = Object.freeze({
  tag: 'continuous',
  /** Used only when neither --asset nor --dmg is given (the historical macOS-only call). */
  dmg: 'target/release/bundle/dmg/*.dmg',
  out: 'target/release/bundle',
  api: 'https://api.github.com',
  uploads: 'https://uploads.github.com',
  releaseName: 'ubiqX (build contínuo)',
  product: 'ubiqX',
  feedSchema: 1,
  notesMaxLines: 40,
  retries: 3,
  retryDelayMs: 1500,
});

export const USAGE = `Uso: node scripts/publish-release.mjs [opções]

Publica (ou atualiza) a release contínua do ubiqX no GitHub com os instaladores recém-compilados e o
latest.json que o app lê para avisar que há uma atualização.

Opções:
  --repo <owner/repo>   repositório (padrão: GITHUB_REPOSITORY, CM_REPO_SLUG ou o remote origin)
  --tag <tag>           tag da release rolante (padrão: ${DEFAULTS.tag})
  --asset <caminho|glob>
                        instalador a publicar; repita para vários. Plataforma e tipo vêm do nome do
                        arquivo: ubiqX-macos-<arch>.dmg, ubiqX-macos-<arch>.app.zip,
                        ubiqX-windows-<arch>.msi, ubiqX-windows-<arch>-setup.exe,
                        ubiqX-linux-<arch>.AppImage, ubiqX-linux-<arch>.deb (outros nomes com a mesma
                        extensão também servem: o tipo vem da extensão e a arquitetura de --arch).
                        Artefatos de atualização (.app.tar.gz, -setup.nsis.zip, .AppImage.tar.gz) vão
                        para o updater.json e exigem o <arquivo>.sig que o Tauri grava ao lado
  --dmg <caminho|glob>  atalho para --asset de um .dmg (padrão, sem --asset: ${DEFAULTS.dmg})
  --app-zip <caminho>   atalho para --asset de um .app.zip feito com ditto
  --out <dir>           onde escrever latest.json e updater.json (padrão: ${DEFAULTS.out})
  --arch <arch>         aarch64 ou x86_64 para nomes sem arquitetura (padrão: a desta máquina)
  --version <x.y.z>     versão do produto (padrão: UBIQX_VERSION, depois a de tauri.conf.json)
  --api <url>           base da API do GitHub (padrão: ${DEFAULTS.api})
  --uploads <url>       base de upload de assets (padrão: ${DEFAULTS.uploads})
  --strict              falha (exit 1) quando GITHUB_TOKEN está ausente, em vez de pular
  --root <dir>          raiz do repositório para git e caminhos relativos (padrão: a do script)
  -h, --help            mostra esta ajuda

Ambiente:
  GITHUB_TOKEN          obrigatório; sem ele a publicação é pulada com exit 0 (ou 1 com --strict)
  UBIQX_VERSION         versão do build (node scripts/app-version.mjs); sem ela vale a de tauri.conf.json
  UBIQX_BUILD_EPOCH, UBIQX_BUILD_NUMBER, UBIQX_BUILD_SHA, UBIQX_BUILD_BRANCH
                        identidade do build (eval "$(bash scripts/build-info.sh)"); sem elas o
                        script consulta o git
`;

// ---------------------------------------------------------------------------------------------
// Arguments and environment

export function parseArgs(argv) {
  const opts = { ...DEFAULTS, dmg: null, appZip: null, assets: [], repo: null, arch: null, version: null, strict: false, help: false, root: REPO_ROOT };
  const takesValue = { '--repo': 'repo', '--tag': 'tag', '--dmg': 'dmg', '--app-zip': 'appZip', '--asset': 'assets', '--out': 'out', '--arch': 'arch', '--version': 'version', '--api': 'api', '--uploads': 'uploads', '--root': 'root' };
  const set = (key, value) => {
    if (key === 'assets') opts.assets.push(value);
    else opts[key] = value;
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '-h' || arg === '--help') {
      opts.help = true;
    } else if (arg === '--strict') {
      opts.strict = true;
    } else if (arg in takesValue) {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) throw new Error(`Falta o valor de ${arg}`);
      set(takesValue[arg], value);
      i += 1;
    } else if (arg.startsWith('--') && arg.includes('=')) {
      const [key, ...rest] = arg.split('=');
      if (!(key in takesValue)) throw new Error(`Opção desconhecida: ${key}`);
      set(takesValue[key], rest.join('='));
    } else {
      throw new Error(`Opção desconhecida: ${arg}`);
    }
  }
  opts.api = opts.api.replace(/\/+$/, '');
  opts.uploads = opts.uploads.replace(/\/+$/, '');
  return opts;
}

function git(root, args) {
  try {
    return execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  } catch {
    return '';
  }
}

/** Build identity from UBIQX_BUILD_* (scripts/build-info.sh) or, failing that, from git. */
export function buildInfo(env, root) {
  const fromEnv = (name) => (env[name] !== undefined && env[name] !== '' ? String(env[name]) : null);
  const epoch = Number(fromEnv('UBIQX_BUILD_EPOCH') ?? git(root, ['log', '-1', '--format=%ct']) ?? 0) || 0;
  const number = Number(fromEnv('UBIQX_BUILD_NUMBER') ?? git(root, ['rev-list', '--count', 'HEAD']) ?? 0) || 0;
  const sha = fromEnv('UBIQX_BUILD_SHA') ?? (git(root, ['rev-parse', '--short=7', 'HEAD']) || 'dev');
  const branch = fromEnv('UBIQX_BUILD_BRANCH') ?? fromEnv('GITHUB_REF_NAME') ?? fromEnv('CM_BRANCH') ?? (git(root, ['symbolic-ref', '--short', '-q', 'HEAD']) || '');
  return { epoch, number, sha, branch };
}

/** The 40-char sha the tag must point at. Falls back to the short sha when nothing longer is known. */
export function fullSha(env, root, short) {
  const candidates = [git(root, ['rev-parse', 'HEAD']), env.GITHUB_SHA, env.CM_COMMIT].filter(Boolean);
  const match = candidates.find((c) => short === 'dev' || c.startsWith(short));
  return match ?? short;
}

/** Subject and body of the built commit, without git trailers, capped to a few dozen lines. */
export function commitNotes(root, maxLines = DEFAULTS.notesMaxLines) {
  return trimNotes(git(root, ['log', '-1', '--format=%B']), maxLines);
}

export function trimNotes(raw, maxLines = DEFAULTS.notesMaxLines) {
  const lines = String(raw ?? '').replace(/\r\n/g, '\n').split('\n');
  while (lines.length && lines[lines.length - 1].trim() === '') lines.pop();
  // Drop the trailer block at the end (Signed-off-by: and similar "Key: value" lines): metadata, not release notes.
  let end = lines.length;
  while (end > 0 && /^[A-Za-z][A-Za-z0-9-]*: \S/.test(lines[end - 1])) end -= 1;
  const kept = end > 0 && lines.slice(0, end).some((l) => l.trim() !== '') ? lines.slice(0, end) : lines;
  while (kept.length && kept[kept.length - 1].trim() === '') kept.pop();
  return kept.slice(0, maxLines).join('\n');
}

export function resolveRepo(opts, env, root) {
  const raw = opts.repo || env.GITHUB_REPOSITORY || env.CM_REPO_SLUG || git(root, ['remote', 'get-url', 'origin']);
  const parsed = parseRepo(raw);
  if (!parsed) throw new Error(`Não consegui descobrir owner/repo (use --repo owner/repo); valor lido: "${raw}"`);
  return parsed;
}

export function parseRepo(value) {
  if (!value) return null;
  let s = String(value).trim();
  s = s.replace(/\.git$/, '');
  const m = s.match(/(?:github\.com[/:])?([^/:\s]+)\/([^/\s]+)$/);
  if (!m) return null;
  return { owner: m[1], repo: m[2] };
}

/**
 * The version both feeds carry. tauri.conf.json comes first because that is the version the app was built
 * with — the one Settings shows and the one tauri-plugin-updater compares — and CI rewrites it per run
 * (`node scripts/app-version.mjs --write`, see scripts/app-version.mjs). `--version` and `UBIQX_VERSION`
 * override it; `[workspace.package]` in Cargo.toml is the last resort.
 */
export function productVersion(opts, root, env = {}) {
  if (opts.version) return opts.version;
  const fromEnv = String(env.UBIQX_VERSION ?? '').trim();
  if (fromEnv) return fromEnv;
  try {
    const conf = JSON.parse(readFileSync(path.join(root, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8'));
    if (conf.version) return String(conf.version);
  } catch { /* fall through */ }
  try {
    const cargo = readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
    const section = cargo.split(/^\[workspace\.package\]\s*$/m)[1] ?? '';
    const m = section.match(/^version\s*=\s*"([^"]+)"/m);
    if (m) return m[1];
  } catch { /* fall through */ }
  return '0.0.0';
}

export function normalizeArch(arch) {
  switch (arch) {
    case 'arm64':
    case 'aarch64':
      return 'aarch64';
    case 'x64':
    case 'x86_64':
    case 'amd64':
      return 'x86_64';
    default:
      return arch;
  }
}

/** Minimal glob: `dir/*.dmg`. Returns the newest match, or the path itself when there is no `*`. */
export function resolveGlob(pattern, root) {
  const abs = path.isAbsolute(pattern) ? pattern : path.join(root, pattern);
  const dir = path.dirname(abs);
  const base = path.basename(abs);
  if (!base.includes('*')) return abs;
  const re = new RegExp(`^${base.split('*').map((p) => p.replace(/[.+?^${}()|[\]\\]/g, '\\$&')).join('.*')}$`);
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return null;
  }
  const matches = entries
    .filter((name) => re.test(name))
    .map((name) => path.join(dir, name))
    .filter((p) => statSync(p).isFile())
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  return matches[0] ?? null;
}

// ---------------------------------------------------------------------------------------------
// Assets and feed

/**
 * Every installer kind the feed knows. `platform` is the prefix of the feed key (`<platform>-<arch>`, the
 * value crates/ubiqx-core/src/update.rs::current_target() produces), `os` the word in the stable asset name,
 * `rank` decides which kind becomes the platform's primary `url` when two are published (lower wins; the
 * other goes to `alternates`). `app_zip` is not an installer: it rides along as `app_zip_url` of the DMG.
 */
export const ASSET_KINDS = Object.freeze([
  { kind: 'app_zip', os: 'macos', platform: 'darwin', test: /\.app\.zip$/i, rank: 9, stableName: (arch) => `ubiqX-macos-${arch}.app.zip` },
  { kind: 'dmg', os: 'macos', platform: 'darwin', test: /\.dmg$/i, rank: 0, stableName: (arch) => `ubiqX-macos-${arch}.dmg` },
  { kind: 'exe', os: 'windows', platform: 'windows', test: /\.exe$/i, rank: 0, stableName: (arch) => `ubiqX-windows-${arch}-setup.exe` },
  { kind: 'msi', os: 'windows', platform: 'windows', test: /\.msi$/i, rank: 1, stableName: (arch) => `ubiqX-windows-${arch}.msi` },
  { kind: 'appimage', os: 'linux', platform: 'linux', test: /\.appimage$/i, rank: 0, stableName: (arch) => `ubiqX-linux-${arch}.AppImage` },
  { kind: 'deb', os: 'linux', platform: 'linux', test: /\.deb$/i, rank: 1, stableName: (arch) => `ubiqX-linux-${arch}.deb` },
  // Updater artifacts (`bundle.createUpdaterArtifacts: "v1Compatible"`): what tauri-plugin-updater downloads
  // and installs by itself. They never appear in latest.json — they are listed in updater.json, each with the
  // minisign signature Tauri wrote next to the file (`<arquivo>.sig`).
  { kind: 'app_tar_gz', updater: true, os: 'macos', platform: 'darwin', test: /\.app\.tar\.gz$/i, rank: 99, stableName: (arch) => `ubiqX-macos-${arch}.app.tar.gz` },
  { kind: 'nsis_zip', updater: true, os: 'windows', platform: 'windows', test: /\.nsis\.zip$/i, rank: 99, stableName: (arch) => `ubiqX-windows-${arch}-setup.nsis.zip` },
  { kind: 'appimage_tar_gz', updater: true, os: 'linux', platform: 'linux', test: /\.AppImage\.tar\.gz$/i, rank: 99, stableName: (arch) => `ubiqX-linux-${arch}.AppImage.tar.gz` },
]);

const STABLE_NAME = /^ubiqX-(macos|windows|linux)-(aarch64|x86_64)(?:-setup)?\.(?:app\.tar\.gz|AppImage\.tar\.gz|nsis\.zip|app\.zip|dmg|msi|exe|AppImage|deb)$/i;
const ARCH_TOKEN = /(?:^|[-_.])(aarch64|arm64|x86_64|x64|amd64)(?=[-_.]|$)/i;

/**
 * What a file name says about an asset: kind (by extension), os/platform and arch. The arch comes from the
 * stable name (`ubiqX-<os>-<arch>…`), else from a token Tauri puts in its output names (`ubiqX_0.1.0_aarch64.dmg`,
 * `ubiqX_0.1.0_x64_en-US.msi`, `ubiqX_0.1.0_amd64.AppImage`), else from `fallbackArch`. Returns null for an
 * extension the feed does not know.
 */
export function inferAsset(filePath, fallbackArch) {
  const base = path.basename(filePath);
  const rule = ASSET_KINDS.find((r) => r.test.test(base));
  if (!rule) return null;
  const stable = base.match(STABLE_NAME);
  let arch = null;
  if (stable) {
    if (stable[1].toLowerCase() !== rule.os) return null; // "ubiqX-linux-x86_64.msi" is a mistake, not an asset
    arch = normalizeArch(stable[2].toLowerCase());
  } else {
    const token = base.match(ARCH_TOKEN);
    arch = token ? normalizeArch(token[1].toLowerCase()) : normalizeArch(fallbackArch);
  }
  if (!arch) return null;
  return { path: filePath, kind: rule.kind, os: rule.os, platformKey: `${rule.platform}-${arch}`, arch, name: rule.stableName(arch), rank: rule.rank, updater: rule.updater === true };
}

/** Feed `platforms` for a list of inferred assets (with `size`): one entry per platform key, sorted by key. */
export function buildPlatforms(assets, download) {
  const byKey = new Map();
  for (const a of assets) {
    if (a.updater) continue; // updater artifacts live in updater.json, never in latest.json (schema 1)
    if (!byKey.has(a.platformKey)) byKey.set(a.platformKey, []);
    byKey.get(a.platformKey).push(a);
  }
  const platforms = {};
  for (const key of [...byKey.keys()].sort()) {
    const list = byKey.get(key);
    const installers = list.filter((a) => a.kind !== 'app_zip').sort((a, b) => a.rank - b.rank || a.name.localeCompare(b.name));
    const appZip = list.find((a) => a.kind === 'app_zip');
    if (installers.length === 0) throw new Error(`${key}: ${appZip?.name ?? '?'} sem o instalador correspondente (o .app.zip acompanha o .dmg)`);
    const [primary, ...rest] = installers;
    const entry = {
      url: `${download}/${primary.name}`,
      kind: primary.kind,
      size: primary.size,
      app_zip_url: appZip ? `${download}/${appZip.name}` : null,
    };
    if (rest.length) entry.alternates = rest.map((a) => ({ url: `${download}/${a.name}`, kind: a.kind, size: a.size }));
    platforms[key] = entry;
  }
  return platforms;
}

export function makeFeed({ owner, repo, tag, version, build, assets, publishedAt, notes, product = DEFAULTS.product }) {
  const download = `https://github.com/${owner}/${repo}/releases/download/${tag}`;
  return {
    schema: DEFAULTS.feedSchema,
    product,
    version,
    build: { epoch: build.epoch, number: build.number, sha: build.sha, branch: build.branch },
    published_at: publishedAt,
    notes,
    release_url: `https://github.com/${owner}/${repo}/releases/tag/${tag}`,
    platforms: buildPlatforms(assets, download),
  };
}

/**
 * How `ours` relates to the feed already published: 'newer' replaces it, 'same' merges the platforms
 * (ours win per key, the other publisher's entries stay), 'older' leaves the release alone. A missing or
 * unreadable feed counts as older than anything.
 */
export function compareFeeds(existing, ours) {
  const theirs = Number(existing?.build?.epoch);
  if (!Number.isFinite(theirs)) return 'newer';
  if (theirs > ours.build.epoch) return 'older';
  return theirs === ours.build.epoch ? 'same' : 'newer';
}

export function mergeFeeds(existing, ours) {
  const platforms = { ...(existing?.platforms ?? {}) };
  for (const [key, entry] of Object.entries(ours.platforms)) platforms[key] = entry;
  const sorted = {};
  for (const key of Object.keys(platforms).sort()) sorted[key] = platforms[key];
  return { ...ours, platforms: sorted };
}

export const dmgAssetName = (arch) => `ubiqX-macos-${arch}.dmg`;
export const appZipAssetName = (arch) => `ubiqX-macos-${arch}.app.zip`;
export const FEED_ASSET_NAME = 'latest.json';

// ---------------------------------------------------------------------------------------------
// The updater feed (updater.json)
//
// The second feed published next to latest.json, in the shape tauri-plugin-updater expects
// (https://v2.tauri.app/plugin/updater/#static-json-file). It is what `plugins.updater.endpoints` in
// apps/desktop/src-tauri/tauri.conf.json points at, and it lists only the updater artifacts:
//
//   { "version": "0.1.128", "notes": "…", "pub_date": "…Z",
//     "platforms": { "darwin-aarch64": { "signature": "<minisign>", "url": "https://…app.tar.gz" }, … } }
//
// latest.json stays exactly as it was (schema 1): the Rust checker, the banner and the tray read that one.

export const UPDATER_ASSET_NAME = 'updater.json';

/** `platforms` of the updater feed: one `{ signature, url }` per platform key, sorted by key. */
export function buildUpdaterPlatforms(assets, download) {
  const platforms = {};
  for (const a of assets.filter((x) => x.updater).sort((a, b) => a.platformKey.localeCompare(b.platformKey))) {
    if (platforms[a.platformKey]) throw new Error(`${a.platformKey}: dois artefatos de atualização (${platforms[a.platformKey].url} e ${a.name})`);
    if (!a.signature) throw new Error(`${a.name}: sem assinatura (${a.name}.sig)`);
    platforms[a.platformKey] = { signature: a.signature, url: `${download}/${a.name}` };
  }
  return platforms;
}

/** The updater feed, or null when this build produced no updater artifact (nothing to publish then). */
export function makeUpdaterFeed({ owner, repo, tag, version, assets, publishedAt, notes }) {
  const download = `https://github.com/${owner}/${repo}/releases/download/${tag}`;
  const platforms = buildUpdaterPlatforms(assets, download);
  if (Object.keys(platforms).length === 0) return null;
  return { version, notes, pub_date: publishedAt, platforms };
}

/**
 * Like [`mergeFeeds`] for the updater feed: the publishers of the same build (the three OS jobs, or Actions
 * and Codemagic) each bring their own platforms. Ours win per key; a feed of another version is replaced.
 */
export function mergeUpdaterFeeds(existing, ours) {
  if (!existing || typeof existing !== 'object' || existing.version !== ours.version) return ours;
  const platforms = { ...(existing.platforms ?? {}) };
  for (const [key, entry] of Object.entries(ours.platforms)) platforms[key] = entry;
  const sorted = {};
  for (const key of Object.keys(platforms).sort()) sorted[key] = platforms[key];
  return { ...ours, platforms: sorted };
}

// ---------------------------------------------------------------------------------------------
// GitHub client (fetch + retries)

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export class GitHubError extends Error {
  constructor(status, method, url, body) {
    super(`${method} ${url} -> HTTP ${status}${body ? `: ${String(body).slice(0, 300)}` : ''}`);
    this.status = status;
  }
}

export function createClient({ token, api, uploads, retries = DEFAULTS.retries, retryDelayMs = DEFAULTS.retryDelayMs, log = () => {}, fetchImpl = globalThis.fetch }) {
  const baseHeaders = {
    Authorization: `Bearer ${token}`,
    Accept: 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
    'User-Agent': 'ubiqx-publish-release',
  };

  async function request(method, url, { body, headers = {}, accept, raw = false, allow404 = false } = {}) {
    let attempt = 0;
    for (;;) {
      attempt += 1;
      let res;
      try {
        const init = { method, headers: { ...baseHeaders, ...headers } };
        if (accept) init.headers.Accept = accept;
        if (body !== undefined) {
          if (Buffer.isBuffer(body)) {
            init.body = body;
            init.headers['Content-Length'] = String(body.length);
          } else {
            init.body = JSON.stringify(body);
            init.headers['Content-Type'] = 'application/json';
          }
        }
        res = await fetchImpl(url, init);
      } catch (err) {
        if (attempt >= retries) throw new Error(`${method} ${url}: ${err.message}`);
        log(`  rede falhou (${err.message}); tentativa ${attempt + 1}/${retries} em ${retryDelayMs * attempt} ms`);
        await sleep(retryDelayMs * attempt);
        continue;
      }
      if (res.status >= 500 && attempt < retries) {
        log(`  HTTP ${res.status} em ${method} ${url}; tentativa ${attempt + 1}/${retries} em ${retryDelayMs * attempt} ms`);
        await sleep(retryDelayMs * attempt);
        continue;
      }
      if (res.status === 404 && allow404) return null;
      if (res.status === 204) return null;
      if (raw) {
        const buf = Buffer.from(await res.arrayBuffer());
        if (!res.ok) throw new GitHubError(res.status, method, url, buf.toString('utf8'));
        return buf;
      }
      const text = await res.text();
      if (!res.ok) throw new GitHubError(res.status, method, url, text);
      return text ? JSON.parse(text) : null;
    }
  }

  return {
    getReleaseByTag: (o, r, tag) => request('GET', `${api}/repos/${o}/${r}/releases/tags/${encodeURIComponent(tag)}`, { allow404: true }),
    createRelease: (o, r, data) => request('POST', `${api}/repos/${o}/${r}/releases`, { body: data }),
    updateRelease: (o, r, id, data) => request('PATCH', `${api}/repos/${o}/${r}/releases/${id}`, { body: data }),
    downloadAsset: (o, r, id) => request('GET', `${api}/repos/${o}/${r}/releases/assets/${id}`, { accept: 'application/octet-stream', raw: true }),
    deleteAsset: (o, r, id) => request('DELETE', `${api}/repos/${o}/${r}/releases/assets/${id}`),
    updateTagRef: (o, r, tag, sha) => request('PATCH', `${api}/repos/${o}/${r}/git/refs/tags/${encodeURIComponent(tag)}`, { body: { sha, force: true }, allow404: true }),
    createTagRef: (o, r, tag, sha) => request('POST', `${api}/repos/${o}/${r}/git/refs`, { body: { ref: `refs/tags/${tag}`, sha } }),
    uploadAsset: (o, r, id, name, contentType, data) =>
      request('POST', `${uploads}/repos/${o}/${r}/releases/${id}/assets?name=${encodeURIComponent(name)}`, { body: data, headers: { 'Content-Type': contentType } }),
  };
}

// ---------------------------------------------------------------------------------------------
// Main

export async function main(argv = process.argv.slice(2), env = process.env, { log = console.log, error = console.error, now = () => new Date(), retryDelayMs } = {}) {
  let opts;
  try {
    opts = parseArgs(argv);
  } catch (err) {
    error(err.message);
    error(USAGE);
    return 2;
  }
  if (opts.help) {
    log(USAGE);
    return 0;
  }

  const token = env.GITHUB_TOKEN;
  if (!token) {
    const msg = 'Publicação pulada: GITHUB_TOKEN não definido. Sem o token o build continua válido, só não vira uma atualização para quem já usa o app.';
    if (opts.strict) {
      error(msg.replace('Publicação pulada', 'Publicação abortada (--strict)'));
      return 1;
    }
    log(msg);
    return 0;
  }

  const root = path.resolve(opts.root);
  let owner;
  let repo;
  try {
    ({ owner, repo } = resolveRepo(opts, env, root));
  } catch (err) {
    error(err.message);
    return 2;
  }

  const arch = normalizeArch(opts.arch || env.UBIQX_BUILD_ARCH || process.arch);
  let assets;
  try {
    assets = collectAssets(opts, root, arch);
  } catch (err) {
    error(err.message);
    return 1;
  }

  const build = buildInfo(env, root);
  const sha = fullSha(env, root, build.sha);
  const version = productVersion(opts, root, env);
  const notes = commitNotes(root);
  const publishedAt = now().toISOString().replace(/\.\d{3}Z$/, 'Z');
  let feed;
  let updaterFeed;
  try {
    feed = makeFeed({ owner, repo, tag: opts.tag, version, build, assets, publishedAt, notes });
    updaterFeed = makeUpdaterFeed({ owner, repo, tag: opts.tag, version, assets, publishedAt, notes });
  } catch (err) {
    error(err.message);
    return 1;
  }

  log(`ubiqX ${version} build ${build.number} (${build.sha}${build.branch ? `, ${build.branch}` : ''}, epoch ${build.epoch}) -> ${owner}/${repo} tag "${opts.tag}"`);
  for (const a of assets) log(`  ${a.platformKey} ${a.kind}: ${path.relative(root, a.path) || a.path} -> ${a.name} (${a.size} bytes)`);

  const gh = createClient({ token, api: opts.api, uploads: opts.uploads, log, retryDelayMs: retryDelayMs ?? Number(env.UBIQX_PUBLISH_RETRY_MS ?? DEFAULTS.retryDelayMs) });

  let release = await gh.getReleaseByTag(owner, repo, opts.tag);
  const releaseData = { name: DEFAULTS.releaseName, body: notes, target_commitish: sha };

  if (release) {
    const existingFeed = release.assets?.find((a) => a.name === FEED_ASSET_NAME);
    const existing = existingFeed ? await readFeed(gh, owner, repo, existingFeed.id, log) : null;
    const relation = compareFeeds(existing, feed);
    if (relation === 'older') {
      log(`Nada a fazer: a release "${opts.tag}" já traz um build mais novo (epoch ${existing.build.epoch} > ${build.epoch}). Este build fica só como artefato.`);
      return 0;
    }
    if (relation === 'same') {
      const kept = Object.keys(existing.platforms ?? {}).filter((k) => !(k in feed.platforms));
      feed = mergeFeeds(existing, feed);
      log(`  latest.json atual é do mesmo build (epoch ${build.epoch}): plataformas mescladas${kept.length ? ` (mantidas: ${kept.join(', ')})` : ''}`);
      if (updaterFeed) {
        const existingUpdater = release.assets?.find((a) => a.name === UPDATER_ASSET_NAME);
        const other = existingUpdater ? await readFeed(gh, owner, repo, existingUpdater.id, log) : null;
        const keptUpdater = Object.keys(other?.platforms ?? {}).filter((k) => !(k in updaterFeed.platforms));
        updaterFeed = mergeUpdaterFeeds(other, updaterFeed);
        if (keptUpdater.length) log(`  updater.json atual é da mesma versão (${version}): plataformas mescladas (mantidas: ${keptUpdater.join(', ')})`);
      }
    } else if (existing) {
      log(`  latest.json atual é mais antigo (epoch ${existing.build.epoch}): substituído`);
    }
    log(`  release existente #${release.id}: atualizando nome, notas e tag`);
    release = await gh.updateRelease(owner, repo, release.id, releaseData);
  } else {
    log('  release não existe ainda: criando');
    release = await gh.createRelease(owner, repo, { tag_name: opts.tag, ...releaseData, draft: false, prerelease: false });
  }

  const outDir = path.isAbsolute(opts.out) ? opts.out : path.join(root, opts.out);
  mkdirSync(outDir, { recursive: true });
  const feedPath = path.join(outDir, FEED_ASSET_NAME);
  const feedJson = `${JSON.stringify(feed, null, 2)}\n`;
  writeFileSync(feedPath, feedJson);
  log(`  feed: ${path.relative(root, feedPath) || feedPath} (${Object.keys(feed.platforms).join(', ')})`);

  let updaterJson = null;
  if (updaterFeed) {
    const updaterPath = path.join(outDir, UPDATER_ASSET_NAME);
    updaterJson = `${JSON.stringify(updaterFeed, null, 2)}\n`;
    writeFileSync(updaterPath, updaterJson);
    log(`  feed do updater: ${path.relative(root, updaterPath) || updaterPath} (${Object.keys(updaterFeed.platforms).join(', ')})`);
  } else {
    log('  sem artefatos de atualização neste build: updater.json não foi tocado');
  }

  const moved = await gh.updateTagRef(owner, repo, opts.tag, sha);
  if (moved === null) {
    await gh.createTagRef(owner, repo, opts.tag, sha);
    log(`  tag ${opts.tag} criada em ${sha}`);
  } else {
    log(`  tag ${opts.tag} movida para ${sha}`);
  }

  const uploads = [
    ...assets.map((a) => ({ name: a.name, contentType: 'application/octet-stream', data: readFileSync(a.path) })),
    // The signature travels inside updater.json; the .sig next to the artifact is published too so anyone
    // can verify the download by hand (`minisign -Vm <artefato> -P <pubkey>`).
    ...assets.filter((a) => a.updater).map((a) => ({ name: `${a.name}.sig`, contentType: 'text/plain', data: Buffer.from(`${a.signature}\n`, 'utf8') })),
    { name: FEED_ASSET_NAME, contentType: 'application/json', data: Buffer.from(feedJson, 'utf8') },
    ...(updaterJson ? [{ name: UPDATER_ASSET_NAME, contentType: 'application/json', data: Buffer.from(updaterJson, 'utf8') }] : []),
  ];
  const names = new Set(uploads.map((u) => u.name));
  for (const asset of release.assets ?? []) {
    if (names.has(asset.name)) {
      await gh.deleteAsset(owner, repo, asset.id);
      log(`  asset antigo removido: ${asset.name}`);
    }
  }

  const urls = [];
  for (const upload of uploads) {
    const result = await gh.uploadAsset(owner, repo, release.id, upload.name, upload.contentType, upload.data);
    const url = result?.browser_download_url ?? `https://github.com/${owner}/${repo}/releases/download/${opts.tag}/${upload.name}`;
    urls.push(url);
    log(`  enviado: ${upload.name} (${upload.data.length} bytes)`);
  }

  log('Publicado:');
  for (const url of urls) log(`  ${url}`);
  log(`  ${feed.release_url}`);
  return 0;
}

/**
 * The files to publish, inferred from --asset (repeatable), --dmg and --app-zip. Without any of them the
 * historical default (the newest DMG under target/release/bundle/dmg) applies. Throws when a file is
 * missing, has an unknown extension or two files map to the same stable name.
 */
export function collectAssets(opts, root, arch) {
  const specs = [...opts.assets.map((p) => ({ pattern: p, what: 'asset' }))];
  if (opts.dmg) specs.push({ pattern: opts.dmg, what: 'dmg' });
  if (opts.appZip) specs.push({ pattern: opts.appZip, what: 'app.zip' });
  if (specs.length === 0) specs.push({ pattern: DEFAULTS.dmg, what: 'dmg' });

  const assets = [];
  for (const { pattern, what } of specs) {
    const file = resolveGlob(pattern, root);
    if (!file || !safeStat(file)) {
      const hint = what === 'dmg' ? ' (compile com "pnpm tauri build --bundles app,dmg" antes de publicar)' : '';
      throw new Error(`${what === 'dmg' ? 'DMG' : what === 'app.zip' ? 'Zip do app' : 'Asset'} não encontrado: ${pattern}${hint}`);
    }
    const inferred = inferAsset(file, arch);
    if (!inferred) throw new Error(`Não sei publicar ${path.basename(file)}: use .dmg, .app.zip, .msi, -setup.exe, .AppImage ou .deb`);
    if (what === 'dmg' && inferred.kind !== 'dmg') throw new Error(`--dmg espera um .dmg, recebeu ${path.basename(file)}`);
    if (what === 'app.zip' && inferred.kind !== 'app_zip') throw new Error(`--app-zip espera um .app.zip, recebeu ${path.basename(file)}`);
    if (assets.some((a) => a.name === inferred.name)) throw new Error(`Dois arquivos viram o mesmo asset ${inferred.name} (${pattern})`);
    assets.push({ ...inferred, size: statSync(file).size, signature: inferred.updater ? readSignature(file) : null });
  }
  return assets;
}

/**
 * The minisign signature Tauri writes next to an updater artifact (`<arquivo>.sig`). Without it the plugin
 * refuses the update, so a missing .sig is a build that ran without TAURI_SIGNING_PRIVATE_KEY: fail loudly
 * instead of publishing an artifact nobody can install.
 */
export function readSignature(file) {
  const sig = `${file}.sig`;
  let raw;
  try {
    raw = readFileSync(sig, 'utf8').trim();
  } catch {
    throw new Error(`Assinatura não encontrada: ${path.basename(sig)} (compile com TAURI_SIGNING_PRIVATE_KEY e TAURI_SIGNING_PRIVATE_KEY_PASSWORD definidos; sem ela o updater recusa a atualização)`);
  }
  if (!raw) throw new Error(`Assinatura vazia: ${path.basename(sig)}`);
  return raw;
}

async function readFeed(gh, owner, repo, assetId, log) {
  try {
    const buf = await gh.downloadAsset(owner, repo, assetId);
    const parsed = JSON.parse(buf.toString('utf8'));
    return parsed && typeof parsed === 'object' ? parsed : null;
  } catch (err) {
    log(`  latest.json atual ilegível (${err.message}); seguindo com a publicação`);
    return null;
  }
}

function safeStat(p) {
  try {
    return statSync(p).isFile();
  } catch {
    return false;
  }
}

const invokedDirectly = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  main().then(
    (code) => process.exit(code),
    (err) => {
      console.error(`Falha ao publicar: ${err.message}`);
      process.exit(1);
    },
  );
}
