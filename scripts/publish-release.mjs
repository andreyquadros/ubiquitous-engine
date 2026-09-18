#!/usr/bin/env node
// Publishes the rolling "continuous" GitHub release the desktop app polls for updates.
//
// Each CI build of a commit calls this script with the DMG it just produced. The script moves
// the release tag to the built commit, rewrites the release name and body from the commit
// message, replaces the assets under stable names and uploads latest.json, the feed the app
// reads (apps/desktop/src-tauri reads UBIQX_UPDATE_FEED_URL, whose default points here):
//
//   https://github.com/<owner>/<repo>/releases/download/continuous/latest.json
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-macos-<arch>.dmg
//   https://github.com/<owner>/<repo>/releases/download/continuous/ubiqX-macos-<arch>.app.zip
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

Publica (ou atualiza) a release contínua do ubiqX no GitHub com o DMG recém-compilado e o
latest.json que o app lê para avisar que há uma atualização.

Opções:
  --repo <owner/repo>   repositório (padrão: GITHUB_REPOSITORY, CM_REPO_SLUG ou o remote origin)
  --tag <tag>           tag da release rolante (padrão: ${DEFAULTS.tag})
  --dmg <caminho|glob>  DMG a publicar (padrão: ${DEFAULTS.dmg})
  --app-zip <caminho>   zip do .app feito com ditto (opcional)
  --out <dir>           onde escrever latest.json (padrão: ${DEFAULTS.out})
  --arch <arch>         aarch64 ou x86_64 (padrão: a arquitetura desta máquina)
  --version <x.y.z>     versão do produto (padrão: [workspace.package] em Cargo.toml)
  --api <url>           base da API do GitHub (padrão: ${DEFAULTS.api})
  --uploads <url>       base de upload de assets (padrão: ${DEFAULTS.uploads})
  --strict              falha (exit 1) quando GITHUB_TOKEN está ausente, em vez de pular
  --root <dir>          raiz do repositório para git e caminhos relativos (padrão: a do script)
  -h, --help            mostra esta ajuda

Ambiente:
  GITHUB_TOKEN          obrigatório; sem ele a publicação é pulada com exit 0 (ou 1 com --strict)
  UBIQX_BUILD_EPOCH, UBIQX_BUILD_NUMBER, UBIQX_BUILD_SHA, UBIQX_BUILD_BRANCH
                        identidade do build (eval "$(bash scripts/build-info.sh)"); sem elas o
                        script consulta o git
`;

// ---------------------------------------------------------------------------------------------
// Arguments and environment

export function parseArgs(argv) {
  const opts = { ...DEFAULTS, appZip: null, repo: null, arch: null, version: null, strict: false, help: false, root: REPO_ROOT };
  const takesValue = { '--repo': 'repo', '--tag': 'tag', '--dmg': 'dmg', '--app-zip': 'appZip', '--out': 'out', '--arch': 'arch', '--version': 'version', '--api': 'api', '--uploads': 'uploads', '--root': 'root' };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '-h' || arg === '--help') {
      opts.help = true;
    } else if (arg === '--strict') {
      opts.strict = true;
    } else if (arg in takesValue) {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) throw new Error(`Falta o valor de ${arg}`);
      opts[takesValue[arg]] = value;
      i += 1;
    } else if (arg.startsWith('--') && arg.includes('=')) {
      const [key, ...rest] = arg.split('=');
      if (!(key in takesValue)) throw new Error(`Opção desconhecida: ${key}`);
      opts[takesValue[key]] = rest.join('=');
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

export function productVersion(opts, root) {
  if (opts.version) return opts.version;
  try {
    const cargo = readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
    const section = cargo.split(/^\[workspace\.package\]\s*$/m)[1] ?? '';
    const m = section.match(/^version\s*=\s*"([^"]+)"/m);
    if (m) return m[1];
  } catch { /* fall through */ }
  try {
    const conf = JSON.parse(readFileSync(path.join(root, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8'));
    if (conf.version) return String(conf.version);
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
// Feed

export function makeFeed({ owner, repo, tag, version, build, arch, dmgSize, hasAppZip, publishedAt, notes, product = DEFAULTS.product }) {
  const download = `https://github.com/${owner}/${repo}/releases/download/${tag}`;
  const platformKey = `darwin-${arch}`;
  return {
    schema: DEFAULTS.feedSchema,
    product,
    version,
    build: { epoch: build.epoch, number: build.number, sha: build.sha, branch: build.branch },
    published_at: publishedAt,
    notes,
    release_url: `https://github.com/${owner}/${repo}/releases/tag/${tag}`,
    platforms: {
      [platformKey]: {
        url: `${download}/${dmgAssetName(arch)}`,
        kind: 'dmg',
        size: dmgSize,
        app_zip_url: hasAppZip ? `${download}/${appZipAssetName(arch)}` : null,
      },
    },
  };
}

export const dmgAssetName = (arch) => `ubiqX-macos-${arch}.dmg`;
export const appZipAssetName = (arch) => `ubiqX-macos-${arch}.app.zip`;
export const FEED_ASSET_NAME = 'latest.json';

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

  const dmgPath = resolveGlob(opts.dmg, root);
  if (!dmgPath || !safeStat(dmgPath)) {
    error(`DMG não encontrado: ${opts.dmg} (compile com "pnpm tauri build --bundles app,dmg" antes de publicar)`);
    return 1;
  }
  const appZipPath = opts.appZip ? resolveGlob(opts.appZip, root) : null;
  if (opts.appZip && (!appZipPath || !safeStat(appZipPath))) {
    error(`Zip do app não encontrado: ${opts.appZip}`);
    return 1;
  }

  const build = buildInfo(env, root);
  const sha = fullSha(env, root, build.sha);
  const arch = normalizeArch(opts.arch || env.UBIQX_BUILD_ARCH || process.arch);
  const version = productVersion(opts, root);
  const notes = commitNotes(root);
  const publishedAt = now().toISOString().replace(/\.\d{3}Z$/, 'Z');
  const feed = makeFeed({ owner, repo, tag: opts.tag, version, build, arch, dmgSize: statSync(dmgPath).size, hasAppZip: Boolean(appZipPath), publishedAt, notes });

  const outDir = path.isAbsolute(opts.out) ? opts.out : path.join(root, opts.out);
  mkdirSync(outDir, { recursive: true });
  const feedPath = path.join(outDir, FEED_ASSET_NAME);
  const feedJson = `${JSON.stringify(feed, null, 2)}\n`;
  writeFileSync(feedPath, feedJson);

  log(`ubiqX ${version} build ${build.number} (${build.sha}${build.branch ? `, ${build.branch}` : ''}, epoch ${build.epoch}) -> ${owner}/${repo} tag "${opts.tag}"`);
  log(`  DMG: ${path.relative(root, dmgPath) || dmgPath} (${feed.platforms[`darwin-${arch}`].size} bytes)`);
  if (appZipPath) log(`  app zip: ${path.relative(root, appZipPath) || appZipPath}`);
  log(`  feed: ${path.relative(root, feedPath) || feedPath}`);

  const gh = createClient({ token, api: opts.api, uploads: opts.uploads, log, retryDelayMs: retryDelayMs ?? Number(env.UBIQX_PUBLISH_RETRY_MS ?? DEFAULTS.retryDelayMs) });

  let release = await gh.getReleaseByTag(owner, repo, opts.tag);
  const releaseData = { name: DEFAULTS.releaseName, body: notes, target_commitish: sha };

  if (release) {
    const existingFeed = release.assets?.find((a) => a.name === FEED_ASSET_NAME);
    if (existingFeed) {
      const existingEpoch = await readFeedEpoch(gh, owner, repo, existingFeed.id, log);
      if (existingEpoch !== null && existingEpoch > build.epoch) {
        log(`Nada a fazer: a release "${opts.tag}" já traz um build mais novo (epoch ${existingEpoch} > ${build.epoch}). Este build fica só como artefato.`);
        return 0;
      }
    }
    log(`  release existente #${release.id}: atualizando nome, notas e tag`);
    release = await gh.updateRelease(owner, repo, release.id, releaseData);
  } else {
    log('  release não existe ainda: criando');
    release = await gh.createRelease(owner, repo, { tag_name: opts.tag, ...releaseData, draft: false, prerelease: false });
  }

  const moved = await gh.updateTagRef(owner, repo, opts.tag, sha);
  if (moved === null) {
    await gh.createTagRef(owner, repo, opts.tag, sha);
    log(`  tag ${opts.tag} criada em ${sha}`);
  } else {
    log(`  tag ${opts.tag} movida para ${sha}`);
  }

  const uploads = [
    { name: dmgAssetName(arch), contentType: 'application/octet-stream', data: readFileSync(dmgPath) },
    ...(appZipPath ? [{ name: appZipAssetName(arch), contentType: 'application/octet-stream', data: readFileSync(appZipPath) }] : []),
    { name: FEED_ASSET_NAME, contentType: 'application/json', data: Buffer.from(feedJson, 'utf8') },
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

async function readFeedEpoch(gh, owner, repo, assetId, log) {
  try {
    const buf = await gh.downloadAsset(owner, repo, assetId);
    const parsed = JSON.parse(buf.toString('utf8'));
    const epoch = Number(parsed?.build?.epoch);
    return Number.isFinite(epoch) ? epoch : null;
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
