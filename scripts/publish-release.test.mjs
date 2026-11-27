// Tests for scripts/publish-release.mjs against an in-process mock of the GitHub API.
// Run with: node --test scripts/publish-release.test.mjs
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { after, before, beforeEach, describe, test } from 'node:test';

import {
  buildPlatforms,
  buildUpdaterPlatforms,
  collectAssets,
  compareFeeds,
  feedBelongsTo,
  foreignAssets,
  inferAsset,
  main,
  makeFeed,
  makeUpdaterFeed,
  mergeFeeds,
  mergeUpdaterFeeds,
  parseArgs,
  parseRepo,
  productOf,
  productStems,
  productVersion,
  trimNotes,
  resolveGlob,
  UPDATER_ASSET_NAME,
} from './publish-release.mjs';
import { appVersion, baseVersion, buildNumber, main as appVersionMain, PRODUCTS, resolveProduct, writeAppVersion } from './app-version.mjs';

const OWNER = 'andreyquadros';
const REPO = 'ubiquitous-engine';
const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const readRepoFile = (relative) => readFileSync(path.join(REPO_ROOT, relative), 'utf8');
/** The standalone app's display name, as its shell is configured today (Tauri names its output after it). */
const SHIELD_DISPLAY = JSON.parse(readRepoFile(PRODUCTS.shield.tauriConf)).productName;

// ---------------------------------------------------------------------------------------------
// Mock GitHub: the routes the script uses, with a request log to assert on.

function createMockGitHub() {
  const state = {
    releases: [], // { id, tag_name, name, body, target_commitish, assets: [{ id, name, content, contentType }] }
    refs: {}, // 'refs/tags/x' -> sha
    nextId: 100,
    log: [], // 'METHOD path'
    failNext: [], // status codes to return before answering normally (retry tests)
  };

  const server = createServer((req, res) => {
    const url = new URL(req.url, 'http://mock');
    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => {
      const body = Buffer.concat(chunks);
      state.log.push(`${req.method} ${url.pathname}${url.search}`);
      if (state.failNext.length) {
        const status = state.failNext.shift();
        res.writeHead(status, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ message: 'simulated failure' }));
        return;
      }
      if (!req.headers.authorization?.startsWith('Bearer ')) {
        res.writeHead(401);
        res.end('{"message":"no token"}');
        return;
      }
      const json = (status, data) => {
        res.writeHead(status, { 'Content-Type': 'application/json' });
        res.end(data === undefined ? '' : JSON.stringify(data));
      };
      const releaseJson = (r) => ({
        id: r.id,
        tag_name: r.tag_name,
        name: r.name,
        body: r.body,
        target_commitish: r.target_commitish,
        html_url: `https://github.com/${OWNER}/${REPO}/releases/tag/${r.tag_name}`,
        assets: r.assets.map((a) => ({ id: a.id, name: a.name, browser_download_url: `https://github.com/${OWNER}/${REPO}/releases/download/${r.tag_name}/${a.name}` })),
      });
      const p = url.pathname;
      const base = `/repos/${OWNER}/${REPO}`;
      let m;

      if (req.method === 'GET' && (m = p.match(new RegExp(`^${base}/releases/tags/([^/]+)$`)))) {
        const r = state.releases.find((x) => x.tag_name === decodeURIComponent(m[1]));
        return r ? json(200, releaseJson(r)) : json(404, { message: 'Not Found' });
      }
      if (req.method === 'POST' && p === `${base}/releases`) {
        const data = JSON.parse(body.toString());
        const r = { id: state.nextId++, tag_name: data.tag_name, name: data.name, body: data.body, target_commitish: data.target_commitish, assets: [] };
        // Kept only when the request carried it, so a test can tell "not sent" from "sent".
        if ('make_latest' in data) r.make_latest = data.make_latest;
        state.releases.push(r);
        if (!state.refs[`refs/tags/${data.tag_name}`]) state.refs[`refs/tags/${data.tag_name}`] = data.target_commitish;
        return json(201, releaseJson(r));
      }
      if (req.method === 'PATCH' && (m = p.match(new RegExp(`^${base}/releases/(\\d+)$`)))) {
        const r = state.releases.find((x) => x.id === Number(m[1]));
        if (!r) return json(404, { message: 'Not Found' });
        Object.assign(r, JSON.parse(body.toString()));
        return json(200, releaseJson(r));
      }
      if (req.method === 'GET' && (m = p.match(new RegExp(`^${base}/releases/assets/(\\d+)$`)))) {
        for (const r of state.releases) {
          const a = r.assets.find((x) => x.id === Number(m[1]));
          if (a) {
            res.writeHead(200, { 'Content-Type': a.contentType });
            res.end(a.content);
            return;
          }
        }
        return json(404, { message: 'Not Found' });
      }
      if (req.method === 'DELETE' && (m = p.match(new RegExp(`^${base}/releases/assets/(\\d+)$`)))) {
        for (const r of state.releases) {
          const i = r.assets.findIndex((x) => x.id === Number(m[1]));
          if (i >= 0) {
            r.assets.splice(i, 1);
            res.writeHead(204);
            res.end();
            return;
          }
        }
        return json(404, { message: 'Not Found' });
      }
      if (req.method === 'PATCH' && (m = p.match(new RegExp(`^${base}/git/refs/tags/([^/]+)$`)))) {
        const ref = `refs/tags/${decodeURIComponent(m[1])}`;
        if (!state.refs[ref]) return json(404, { message: 'Not Found' });
        const data = JSON.parse(body.toString());
        assert.equal(data.force, true, 'moving the rolling tag must be forced');
        state.refs[ref] = data.sha;
        return json(200, { ref, object: { sha: data.sha } });
      }
      if (req.method === 'POST' && p === `${base}/git/refs`) {
        const data = JSON.parse(body.toString());
        state.refs[data.ref] = data.sha;
        return json(201, { ref: data.ref, object: { sha: data.sha } });
      }
      if (req.method === 'POST' && (m = p.match(new RegExp(`^/uploads${base}/releases/(\\d+)/assets$`)))) {
        const r = state.releases.find((x) => x.id === Number(m[1]));
        if (!r) return json(404, { message: 'Not Found' });
        const name = url.searchParams.get('name');
        if (r.assets.some((a) => a.name === name)) return json(422, { message: 'already_exists' });
        const asset = { id: state.nextId++, name, content: body, contentType: req.headers['content-type'] };
        r.assets.push(asset);
        return json(201, { id: asset.id, name, browser_download_url: `https://github.com/${OWNER}/${REPO}/releases/download/${r.tag_name}/${name}` });
      }
      json(404, { message: `unhandled ${req.method} ${p}` });
    });
  });

  return {
    state,
    listen: () =>
      new Promise((resolve) => {
        server.listen(0, '127.0.0.1', () => resolve(`http://127.0.0.1:${server.address().port}`));
      }),
    close: () => new Promise((resolve) => server.close(resolve)),
    reset() {
      state.releases = [];
      state.refs = {};
      state.log = [];
      state.failNext = [];
    },
    // `product: null` seeds a feed without the field, as feeds were written before it existed.
    seedRelease({ feedEpoch, assets = ['ubiqX-macos-aarch64.dmg', 'ubiqX-macos-aarch64.app.zip'], platforms = {}, tag = 'continuous', product = 'ubiqX' }) {
      const r = { id: state.nextId++, tag_name: tag, name: 'old', body: 'old notes', target_commitish: 'a'.repeat(40), assets: [] };
      for (const name of assets) r.assets.push({ id: state.nextId++, name, content: Buffer.from('old'), contentType: 'application/octet-stream' });
      if (feedEpoch !== undefined) {
        const feed = { schema: 1, ...(product === null ? {} : { product }), version: '0.1.0', build: { epoch: feedEpoch, number: 1, sha: 'aaaaaaa', branch: 'main' }, platforms };
        r.assets.push({ id: state.nextId++, name: 'latest.json', content: Buffer.from(JSON.stringify(feed)), contentType: 'application/json' });
      }
      state.releases.push(r);
      state.refs[`refs/tags/${tag}`] = r.target_commitish;
      return r;
    },
  };
}

// ---------------------------------------------------------------------------------------------

const BUILD_ENV = {
  UBIQX_BUILD_EPOCH: '1758221040',
  UBIQX_BUILD_NUMBER: '27',
  UBIQX_BUILD_SHA: '14c6e7f',
  UBIQX_BUILD_BRANCH: 'main',
  GITHUB_REPOSITORY: `${OWNER}/${REPO}`,
};

describe('publish-release', () => {
  const gh = createMockGitHub();
  let api;
  let work;
  let dmg;
  let appZip;
  let winExe;
  let winMsi;
  let linuxAppImage;
  let linuxDeb;
  // Updater artifacts (bundle.createUpdaterArtifacts) and the .sig Tauri writes next to each of them.
  let macUpdater;
  let winUpdater;
  let linuxUpdater;
  let out;
  let lines;
  const log = (line) => lines.push(String(line));

  before(async () => {
    api = await gh.listen();
    work = mkdtempSync(path.join(tmpdir(), 'ubiqx-publish-'));
    dmg = path.join(work, 'ubiqX_0.1.0_aarch64.dmg');
    appZip = path.join(work, 'ubiqX-macos-aarch64.app.zip');
    out = path.join(work, 'out');
    winExe = path.join(work, 'ubiqX-windows-x86_64-setup.exe');
    winMsi = path.join(work, 'ubiqX_0.1.0_x64_en-US.msi');
    linuxAppImage = path.join(work, 'ubiqX-linux-x86_64.AppImage');
    linuxDeb = path.join(work, 'ubiqX_0.1.0_amd64.deb');
    writeFileSync(dmg, Buffer.alloc(2048, 1));
    writeFileSync(appZip, Buffer.alloc(512, 2));
    writeFileSync(winExe, Buffer.alloc(3000, 3));
    writeFileSync(winMsi, Buffer.alloc(3100, 4));
    writeFileSync(linuxAppImage, Buffer.alloc(4000, 5));
    writeFileSync(linuxDeb, Buffer.alloc(4100, 6));
    macUpdater = path.join(work, 'ubiqX.app.tar.gz');
    winUpdater = path.join(work, 'ubiqX_0.1.0_x64-setup.nsis.zip');
    linuxUpdater = path.join(work, 'ubiqX_0.1.0_amd64.AppImage.tar.gz');
    writeFileSync(macUpdater, Buffer.alloc(5000, 7));
    writeFileSync(winUpdater, Buffer.alloc(5100, 8));
    writeFileSync(linuxUpdater, Buffer.alloc(5200, 9));
    writeFileSync(`${macUpdater}.sig`, 'dW50cnVzdGVkIG1hYw==\n');
    writeFileSync(`${winUpdater}.sig`, 'dW50cnVzdGVkIHdpbg==\n');
    writeFileSync(`${linuxUpdater}.sig`, 'dW50cnVzdGVkIGxpbnV4\n');
  });
  after(async () => {
    await gh.close();
    rmSync(work, { recursive: true, force: true });
  });
  beforeEach(() => {
    gh.reset();
    lines = [];
  });

  const argv = (...extra) => ['--dmg', path.join(work, '*.dmg'), '--app-zip', appZip, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`, ...extra];
  const run = (args, env = {}) => main(args, { ...BUILD_ENV, GITHUB_TOKEN: 'ghp_test', ...env }, { log, error: log, retryDelayMs: 5, now: () => new Date('2026-09-18T19:10:00Z') });

  test('creates the release, the tag and the three assets when nothing exists', async () => {
    const code = await run(argv());
    assert.equal(code, 0, lines.join('\n'));

    assert.equal(gh.state.releases.length, 1);
    const release = gh.state.releases[0];
    assert.equal(release.tag_name, 'continuous');
    assert.equal(release.name, 'ubiqX (build contínuo)');
    assert.match(release.target_commitish, /^[0-9a-f]{7,40}$/);
    assert.deepEqual(release.assets.map((a) => a.name), ['ubiqX-macos-aarch64.dmg', 'ubiqX-macos-aarch64.app.zip', 'latest.json']);
    assert.equal(release.assets[0].content.length, 2048);
    assert.equal(release.assets[0].contentType, 'application/octet-stream');
    assert.equal(release.assets[2].contentType, 'application/json');
    assert.equal(gh.state.refs['refs/tags/continuous'], release.target_commitish);

    assert.ok(gh.state.log.includes(`POST /repos/${OWNER}/${REPO}/releases`), 'creates through POST');
    assert.ok(!gh.state.log.some((l) => l.startsWith('PATCH /repos/andreyquadros/ubiquitous-engine/releases/')), 'no PATCH on a fresh release');
    assert.ok(!gh.state.log.some((l) => l.startsWith('DELETE')), 'nothing to delete');

    const feed = JSON.parse(readFileSync(path.join(out, 'latest.json'), 'utf8'));
    assert.equal(feed.schema, 1);
    assert.equal(feed.product, 'ubiqX');
    assert.equal(feed.version, '0.1.0');
    assert.deepEqual(feed.build, { epoch: 1758221040, number: 27, sha: '14c6e7f', branch: 'main' });
    assert.equal(feed.published_at, '2026-09-18T19:10:00Z');
    assert.equal(typeof feed.notes, 'string');
    assert.equal(feed.release_url, `https://github.com/${OWNER}/${REPO}/releases/tag/continuous`);
    assert.deepEqual(feed.platforms['darwin-aarch64'], {
      url: `https://github.com/${OWNER}/${REPO}/releases/download/continuous/ubiqX-macos-aarch64.dmg`,
      kind: 'dmg',
      size: 2048,
      app_zip_url: `https://github.com/${OWNER}/${REPO}/releases/download/continuous/ubiqX-macos-aarch64.app.zip`,
    });
    assert.deepEqual(JSON.parse(release.assets[2].content.toString()), feed, 'the uploaded feed is the one written to --out');

    const printed = lines.join('\n');
    assert.match(printed, /releases\/download\/continuous\/ubiqX-macos-aarch64\.dmg/);
    assert.match(printed, /releases\/download\/continuous\/latest\.json/);
  });

  test('updates an existing release: rewrites it, force-moves the tag and replaces same-named assets', async () => {
    const seeded = gh.seedRelease({ feedEpoch: 1758000000 });
    const code = await run(argv());
    assert.equal(code, 0, lines.join('\n'));

    assert.equal(gh.state.releases.length, 1, 'no second release');
    const release = gh.state.releases[0];
    assert.equal(release.id, seeded.id);
    assert.equal(release.name, 'ubiqX (build contínuo)');
    assert.notEqual(release.body, 'old notes');
    assert.notEqual(release.target_commitish, 'a'.repeat(40));
    assert.equal(gh.state.refs['refs/tags/continuous'], release.target_commitish, 'tag moved to the built commit');

    assert.ok(gh.state.log.includes(`PATCH /repos/${OWNER}/${REPO}/releases/${seeded.id}`));
    assert.ok(gh.state.log.includes(`PATCH /repos/${OWNER}/${REPO}/git/refs/tags/continuous`));
    assert.ok(!gh.state.log.includes(`POST /repos/${OWNER}/${REPO}/releases`), 'must not create a second release');
    assert.equal(gh.state.log.filter((l) => l.startsWith('DELETE')).length, 3, 'dmg, app zip and latest.json replaced');

    assert.deepEqual(release.assets.map((a) => a.name).sort(), ['latest.json', 'ubiqX-macos-aarch64.app.zip', 'ubiqX-macos-aarch64.dmg']);
    const feed = JSON.parse(release.assets.find((a) => a.name === 'latest.json').content.toString());
    assert.equal(feed.build.epoch, 1758221040);
    assert.equal(release.assets.find((a) => a.name === 'ubiqX-macos-aarch64.dmg').content.length, 2048);
  });

  test('leaves a release alone when its latest.json is newer than this build', async () => {
    const seeded = gh.seedRelease({ feedEpoch: 1758221040 + 3600 });
    const code = await run(argv());
    assert.equal(code, 0);
    assert.match(lines.join('\n'), /mais novo/);
    const release = gh.state.releases[0];
    assert.equal(release.name, 'old');
    assert.equal(release.body, 'old notes');
    assert.equal(gh.state.refs['refs/tags/continuous'], 'a'.repeat(40));
    assert.equal(release.assets.length, 3, 'assets untouched');
    assert.equal(release.assets.find((a) => a.name === 'ubiqX-macos-aarch64.dmg').content.toString(), 'old');
    const mutating = gh.state.log.filter((l) => !l.startsWith('GET '));
    assert.deepEqual(mutating, [], `no writes expected, got: ${mutating.join(', ')}`);
    assert.equal(seeded.id, release.id);
  });

  test('re-publishing the same epoch is allowed (same commit rebuilt)', async () => {
    gh.seedRelease({ feedEpoch: 1758221040 });
    const code = await run(argv());
    assert.equal(code, 0);
    assert.ok(gh.state.log.some((l) => l.startsWith('POST /uploads')), 'assets re-uploaded');
  });

  test('skips with exit 0 and a clear message when GITHUB_TOKEN is missing', async () => {
    const code = await run(argv(), { GITHUB_TOKEN: '' });
    assert.equal(code, 0);
    assert.match(lines.join('\n'), /Publicação pulada/);
    assert.match(lines.join('\n'), /GITHUB_TOKEN/);
    assert.deepEqual(gh.state.log, [], 'no request without a token');
  });

  test('--strict turns the missing token into a failure', async () => {
    const code = await run(argv('--strict'), { GITHUB_TOKEN: '' });
    assert.equal(code, 1);
    assert.deepEqual(gh.state.log, []);
  });

  test('retries on 5xx before giving up', async () => {
    gh.state.failNext.push(503, 502);
    const code = await run(argv());
    assert.equal(code, 0, lines.join('\n'));
    assert.equal(gh.state.log.filter((l) => l === `GET /repos/${OWNER}/${REPO}/releases/tags/continuous`).length, 3);
    assert.equal(gh.state.releases.length, 1);
  });

  test('fails with a clear error when the DMG is missing', async () => {
    const code = await run(argv('--dmg', path.join(work, 'nothing-*.dmg')));
    assert.equal(code, 1);
    assert.match(lines.join('\n'), /DMG não encontrado/);
    assert.deepEqual(gh.state.log, []);
  });

  test('omits app_zip_url when no app zip is given', async () => {
    const code = await run(['--dmg', dmg, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 0, lines.join('\n'));
    const feed = JSON.parse(readFileSync(path.join(out, 'latest.json'), 'utf8'));
    assert.equal(feed.platforms['darwin-aarch64'].app_zip_url, null);
    assert.deepEqual(gh.state.releases[0].assets.map((a) => a.name), ['ubiqX-macos-aarch64.dmg', 'latest.json']);
  });

  test('--asset publishes several platforms at once under their stable names', async () => {
    const code = await run(['--asset', dmg, '--asset', appZip, '--asset', winExe, '--asset', winMsi, '--asset', linuxAppImage, '--asset', linuxDeb, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 0, lines.join('\n'));
    const release = gh.state.releases[0];
    assert.deepEqual(release.assets.map((a) => a.name), [
      'ubiqX-macos-aarch64.dmg',
      'ubiqX-macos-aarch64.app.zip',
      'ubiqX-windows-x86_64-setup.exe',
      'ubiqX-windows-x86_64.msi',
      'ubiqX-linux-x86_64.AppImage',
      'ubiqX-linux-x86_64.deb',
      'latest.json',
    ]);
    assert.equal(release.assets.find((a) => a.name === 'ubiqX-windows-x86_64.msi').content.length, 3100);
    assert.equal(release.assets.find((a) => a.name === 'ubiqX-linux-x86_64.deb').content.length, 4100);

    const feed = JSON.parse(readFileSync(path.join(out, 'latest.json'), 'utf8'));
    assert.deepEqual(Object.keys(feed.platforms), ['darwin-aarch64', 'linux-x86_64', 'windows-x86_64']);
    const download = `https://github.com/${OWNER}/${REPO}/releases/download/continuous`;
    assert.deepEqual(feed.platforms['windows-x86_64'], {
      url: `${download}/ubiqX-windows-x86_64-setup.exe`,
      kind: 'exe',
      size: 3000,
      app_zip_url: null,
      alternates: [{ url: `${download}/ubiqX-windows-x86_64.msi`, kind: 'msi', size: 3100 }],
    });
    assert.deepEqual(feed.platforms['linux-x86_64'], {
      url: `${download}/ubiqX-linux-x86_64.AppImage`,
      kind: 'appimage',
      size: 4000,
      app_zip_url: null,
      alternates: [{ url: `${download}/ubiqX-linux-x86_64.deb`, kind: 'deb', size: 4100 }],
    });
    assert.equal(feed.platforms['darwin-aarch64'].kind, 'dmg');
    assert.equal(feed.platforms['darwin-aarch64'].app_zip_url, `${download}/ubiqX-macos-aarch64.app.zip`);
    assert.deepEqual(JSON.parse(release.assets.at(-1).content.toString()), feed);
  });

  test('same build epoch: merges the platforms already published (ours win per key, the others stay)', async () => {
    const download = `https://github.com/${OWNER}/${REPO}/releases/download/continuous`;
    const seeded = gh.seedRelease({
      feedEpoch: 1758221040,
      assets: ['ubiqX-windows-x86_64-setup.exe', 'ubiqX-macos-aarch64.dmg'],
      platforms: {
        'windows-x86_64': { url: `${download}/ubiqX-windows-x86_64-setup.exe`, kind: 'exe', size: 77, app_zip_url: null },
        'darwin-aarch64': { url: `${download}/ubiqX-macos-aarch64.dmg`, kind: 'dmg', size: 1, app_zip_url: null },
      },
    });
    const code = await run(argv());
    assert.equal(code, 0, lines.join('\n'));
    assert.match(lines.join('\n'), /mesclad/);
    const release = gh.state.releases[0];
    assert.equal(release.id, seeded.id);
    const feed = JSON.parse(release.assets.find((a) => a.name === 'latest.json').content.toString());
    assert.deepEqual(Object.keys(feed.platforms), ['darwin-aarch64', 'windows-x86_64']);
    assert.equal(feed.platforms['windows-x86_64'].size, 77, 'the other publisher\'s entry survives');
    assert.equal(feed.platforms['darwin-aarch64'].size, 2048, 'ours replaces the same key');
    assert.equal(feed.platforms['darwin-aarch64'].app_zip_url, `${download}/ubiqX-macos-aarch64.app.zip`);
    assert.deepEqual(JSON.parse(readFileSync(path.join(out, 'latest.json'), 'utf8')), feed, 'the merged feed is what --out gets');
    // the windows installer was not touched, only what we uploaded was replaced
    assert.equal(release.assets.find((a) => a.name === 'ubiqX-windows-x86_64-setup.exe').content.toString(), 'old');
    assert.equal(gh.state.log.filter((l) => l.startsWith('DELETE')).length, 2, 'dmg and latest.json replaced');
  });

  test('newer build: replaces the feed, the other platforms wait for their own publish', async () => {
    const download = `https://github.com/${OWNER}/${REPO}/releases/download/continuous`;
    gh.seedRelease({
      feedEpoch: 1758000000,
      assets: ['ubiqX-windows-x86_64-setup.exe'],
      platforms: { 'windows-x86_64': { url: `${download}/ubiqX-windows-x86_64-setup.exe`, kind: 'exe', size: 77, app_zip_url: null } },
    });
    const code = await run(argv());
    assert.equal(code, 0, lines.join('\n'));
    assert.match(lines.join('\n'), /mais antigo/);
    const feed = JSON.parse(gh.state.releases[0].assets.find((a) => a.name === 'latest.json').content.toString());
    assert.deepEqual(Object.keys(feed.platforms), ['darwin-aarch64']);
    assert.equal(feed.build.epoch, 1758221040);
  });

  test('refuses a file with an unknown extension and an --asset that is missing', async () => {
    const bogus = path.join(work, 'ubiqX-windows-x86_64.txt');
    writeFileSync(bogus, 'x');
    let code = await run(['--asset', bogus, '--out', out, '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 1);
    assert.match(lines.join('\n'), /Não sei publicar ubiqX-windows-x86_64\.txt/);
    lines = [];
    code = await run(['--asset', path.join(work, 'nothing-*.AppImage'), '--out', out, '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 1);
    assert.match(lines.join('\n'), /Asset não encontrado/);
    assert.deepEqual(gh.state.log, []);
  });

  test('publishes updater.json and the .sig next to the installers, leaving latest.json untouched', async () => {
    const code = await run([
      '--asset', dmg,
      '--asset', appZip,
      '--asset', macUpdater,
      '--asset', winExe,
      '--asset', winUpdater,
      '--asset', linuxAppImage,
      '--asset', linuxUpdater,
      '--out', out,
      '--arch', 'aarch64',
      '--api', api,
      '--uploads', `${api}/uploads`,
    ]);
    assert.equal(code, 0, lines.join('\n'));
    const release = gh.state.releases[0];
    const download = `https://github.com/${OWNER}/${REPO}/releases/download/continuous`;

    // Both feeds are uploaded, plus one .sig per updater artifact, and the installers keep their names.
    assert.deepEqual(release.assets.map((a) => a.name), [
      'ubiqX-macos-aarch64.dmg',
      'ubiqX-macos-aarch64.app.zip',
      'ubiqX-macos-aarch64.app.tar.gz',
      'ubiqX-windows-x86_64-setup.exe',
      'ubiqX-windows-x86_64-setup.nsis.zip',
      'ubiqX-linux-x86_64.AppImage',
      'ubiqX-linux-x86_64.AppImage.tar.gz',
      'ubiqX-macos-aarch64.app.tar.gz.sig',
      'ubiqX-windows-x86_64-setup.nsis.zip.sig',
      'ubiqX-linux-x86_64.AppImage.tar.gz.sig',
      'latest.json',
      UPDATER_ASSET_NAME,
    ]);

    // latest.json: schema 1, installers only, no trace of the updater artifacts.
    const feed = JSON.parse(readFileSync(path.join(out, 'latest.json'), 'utf8'));
    assert.equal(feed.schema, 1);
    assert.deepEqual(Object.keys(feed.platforms), ['darwin-aarch64', 'linux-x86_64', 'windows-x86_64']);
    assert.equal(feed.platforms['darwin-aarch64'].kind, 'dmg');
    assert.equal(feed.platforms['windows-x86_64'].url, `${download}/ubiqX-windows-x86_64-setup.exe`);
    assert.equal(feed.platforms['linux-x86_64'].url, `${download}/ubiqX-linux-x86_64.AppImage`);
    // Everything but `notes`, which carries the commit message verbatim: a commit that happens to
    // say "signature" or "tar.gz" is not a feed that points at an updater artifact, and the day one
    // did, this assertion failed for the wrong reason.
    const { notes: _notes, ...structure } = feed;
    assert.ok(!JSON.stringify(structure).includes('tar.gz'), 'latest.json never mentions an updater artifact');
    assert.ok(!JSON.stringify(structure).includes('nsis.zip'));
    assert.ok(!JSON.stringify(structure).includes('signature'));

    // updater.json: exactly the shape tauri-plugin-updater reads.
    const updater = JSON.parse(readFileSync(path.join(out, UPDATER_ASSET_NAME), 'utf8'));
    assert.deepEqual(Object.keys(updater).sort(), ['notes', 'platforms', 'pub_date', 'version']);
    assert.equal(updater.version, '0.1.0');
    assert.equal(updater.pub_date, '2026-09-18T19:10:00Z');
    assert.equal(typeof updater.notes, 'string');
    assert.deepEqual(updater.platforms, {
      'darwin-aarch64': { signature: 'dW50cnVzdGVkIG1hYw==', url: `${download}/ubiqX-macos-aarch64.app.tar.gz` },
      'linux-x86_64': { signature: 'dW50cnVzdGVkIGxpbnV4', url: `${download}/ubiqX-linux-x86_64.AppImage.tar.gz` },
      'windows-x86_64': { signature: 'dW50cnVzdGVkIHdpbg==', url: `${download}/ubiqX-windows-x86_64-setup.nsis.zip` },
    });
    assert.deepEqual(JSON.parse(release.assets.find((a) => a.name === UPDATER_ASSET_NAME).content.toString()), updater, 'the uploaded updater feed is the one written to --out');
    assert.equal(release.assets.find((a) => a.name === UPDATER_ASSET_NAME).contentType, 'application/json');
    assert.equal(release.assets.find((a) => a.name === 'ubiqX-macos-aarch64.app.tar.gz.sig').content.toString(), 'dW50cnVzdGVkIG1hYw==\n');
    assert.equal(release.assets.find((a) => a.name === 'ubiqX-macos-aarch64.app.tar.gz').content.length, 5000);
  });

  test('a build without updater artifacts publishes latest.json alone and leaves updater.json alone', async () => {
    const code = await run(argv());
    assert.equal(code, 0, lines.join('\n'));
    assert.ok(!gh.state.releases[0].assets.some((a) => a.name === UPDATER_ASSET_NAME));
    assert.match(lines.join('\n'), /sem artefatos de atualização/);
  });

  test('an updater artifact without its .sig is a named failure, not a silent unsigned publish', async () => {
    const unsigned = path.join(work, 'unsigned', 'ubiqX-macos-aarch64.app.tar.gz');
    mkdirSync(path.dirname(unsigned), { recursive: true });
    writeFileSync(unsigned, Buffer.alloc(16, 1));
    const code = await run(['--asset', dmg, '--asset', unsigned, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 1);
    const printed = lines.join('\n');
    assert.match(printed, /Assinatura não encontrada: ubiqX-macos-aarch64\.app\.tar\.gz\.sig/);
    assert.match(printed, /TAURI_SIGNING_PRIVATE_KEY/);
    assert.deepEqual(gh.state.log, [], 'nothing is published without the signature');
  });

  test('same build epoch: the updater feed merges the platforms another publisher already put there', async () => {
    const download = `https://github.com/${OWNER}/${REPO}/releases/download/continuous`;
    const seeded = gh.seedRelease({ feedEpoch: 1758221040, assets: ['ubiqX-macos-aarch64.dmg'], platforms: {} });
    seeded.assets.push({
      id: 9001,
      name: UPDATER_ASSET_NAME,
      content: Buffer.from(JSON.stringify({ version: '0.1.0', notes: 'old', pub_date: '2026-09-18T18:00:00Z', platforms: { 'windows-x86_64': { signature: 'antiga', url: `${download}/ubiqX-windows-x86_64-setup.nsis.zip` } } })),
      contentType: 'application/json',
    });
    const code = await run(['--asset', dmg, '--asset', macUpdater, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 0, lines.join('\n'));
    assert.match(lines.join('\n'), /updater\.json atual é da mesma versão/);
    const updater = JSON.parse(gh.state.releases[0].assets.find((a) => a.name === UPDATER_ASSET_NAME).content.toString());
    assert.deepEqual(Object.keys(updater.platforms), ['darwin-aarch64', 'windows-x86_64']);
    assert.equal(updater.platforms['windows-x86_64'].signature, 'antiga', "the other publisher's platform survives");
    assert.equal(updater.platforms['darwin-aarch64'].signature, 'dW50cnVzdGVkIG1hYw==');
    assert.equal(updater.pub_date, '2026-09-18T19:10:00Z', 'ours is the feed being written');
  });

  test('--version and UBIQX_VERSION put the same number in both feeds', async () => {
    let code = await run(['--asset', dmg, '--asset', macUpdater, '--out', out, '--arch', 'aarch64', '--version', '0.1.128', '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 0, lines.join('\n'));
    assert.equal(JSON.parse(readFileSync(path.join(out, 'latest.json'), 'utf8')).version, '0.1.128');
    assert.equal(JSON.parse(readFileSync(path.join(out, UPDATER_ASSET_NAME), 'utf8')).version, '0.1.128');

    gh.reset();
    lines = [];
    code = await run(['--asset', dmg, '--asset', macUpdater, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`], { UBIQX_VERSION: '0.1.129' });
    assert.equal(code, 0, lines.join('\n'));
    assert.equal(JSON.parse(readFileSync(path.join(out, 'latest.json'), 'utf8')).version, '0.1.129');
    assert.equal(JSON.parse(readFileSync(path.join(out, UPDATER_ASSET_NAME), 'utf8')).version, '0.1.129');
  });

  test('--help prints the usage and does nothing else', async () => {
    const code = await main(['--help'], { GITHUB_TOKEN: 'x' }, { log, error: log });
    assert.equal(code, 0);
    assert.match(lines.join('\n'), /^Uso: node scripts\/publish-release\.mjs/);
  });

  // -------------------------------------------------------------------------------------------
  // The second product: the standalone app, published under its machine id to its own release.

  /** The files a build of the standalone app leaves behind, named as Tauri names them, in their own folder. */
  function shieldBuild() {
    const dir = path.join(work, 'shield');
    mkdirSync(dir, { recursive: true });
    const files = {
      dmg: [`${SHIELD_DISPLAY}_0.1.0_aarch64.dmg`, 2100],
      appZip: ['ubiqx-shield-macos-aarch64.app.zip', 600],
      macUpdater: [`${SHIELD_DISPLAY}.app.tar.gz`, 5300],
      exe: [`${SHIELD_DISPLAY}_0.1.0_x64-setup.exe`, 3300],
      msi: [`${SHIELD_DISPLAY}_0.1.0_x64_pt-BR.msi`, 3400],
      appImage: [`${SHIELD_DISPLAY}_0.1.0_amd64.AppImage`, 4300],
      deb: ['ubiqx-shield-linux-x86_64.deb', 4400],
    };
    const paths = {};
    for (const [key, [name, size]] of Object.entries(files)) {
      paths[key] = path.join(dir, name);
      writeFileSync(paths[key], Buffer.alloc(size, 11));
    }
    writeFileSync(`${paths.macUpdater}.sig`, 'c2hpZWxkIG1hYw==\n');
    return paths;
  }
  const shieldArgv = (files, ...extra) => ['--product', 'shield', ...Object.values(files).flatMap((f) => ['--asset', f]), '--out', path.join(work, 'out-shield'), '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`, ...extra];
  /** Everything the mock holds about one release, to compare before and after. */
  const snapshot = (tag) => {
    const r = gh.state.releases.find((x) => x.tag_name === tag);
    return JSON.stringify({ name: r.name, body: r.body, target: r.target_commitish, ref: gh.state.refs[`refs/tags/${tag}`], assets: r.assets.map((a) => [a.name, a.content.toString('base64')]) });
  };

  test('--product shield publishes the standalone app to its own release, under the machine id', async () => {
    // ubiqX is already published: nothing of it may move.
    assert.equal(await run(argv()), 0, lines.join('\n'));
    const ubiqxBefore = snapshot('continuous');
    gh.state.log = [];
    lines = [];

    const files = shieldBuild();
    const code = await run(shieldArgv(files));
    assert.equal(code, 0, lines.join('\n'));

    assert.deepEqual(gh.state.releases.map((r) => r.tag_name), ['continuous', 'shield-continuous']);
    const release = gh.state.releases[1];
    assert.equal(release.name, 'ubiqx-shield (build contínuo)');
    assert.equal(gh.state.refs['refs/tags/shield-continuous'], release.target_commitish);
    assert.deepEqual(release.assets.map((a) => a.name), [
      'ubiqx-shield-macos-aarch64.dmg',
      'ubiqx-shield-macos-aarch64.app.zip',
      'ubiqx-shield-macos-aarch64.app.tar.gz',
      'ubiqx-shield-windows-x86_64-setup.exe',
      'ubiqx-shield-windows-x86_64.msi',
      'ubiqx-shield-linux-x86_64.AppImage',
      'ubiqx-shield-linux-x86_64.deb',
      'ubiqx-shield-macos-aarch64.app.tar.gz.sig',
      'latest.json',
      UPDATER_ASSET_NAME,
    ]);
    assert.equal(release.assets[0].content.length, 2100, 'the DMG Tauri named after the display name is the one uploaded');

    const download = `https://github.com/${OWNER}/${REPO}/releases/download/shield-continuous`;
    const feed = JSON.parse(readFileSync(path.join(work, 'out-shield', 'latest.json'), 'utf8'));
    assert.equal(feed.schema, 1);
    assert.equal(feed.product, 'ubiqx-shield', 'the name the standalone app accepts a feed under');
    assert.equal(feed.version, '0.1.0');
    assert.equal(feed.release_url, `https://github.com/${OWNER}/${REPO}/releases/tag/shield-continuous`);
    assert.deepEqual(feed.platforms, {
      'darwin-aarch64': { url: `${download}/ubiqx-shield-macos-aarch64.dmg`, kind: 'dmg', size: 2100, app_zip_url: `${download}/ubiqx-shield-macos-aarch64.app.zip` },
      'linux-x86_64': {
        url: `${download}/ubiqx-shield-linux-x86_64.AppImage`,
        kind: 'appimage',
        size: 4300,
        app_zip_url: null,
        alternates: [{ url: `${download}/ubiqx-shield-linux-x86_64.deb`, kind: 'deb', size: 4400 }],
      },
      'windows-x86_64': {
        url: `${download}/ubiqx-shield-windows-x86_64-setup.exe`,
        kind: 'exe',
        size: 3300,
        app_zip_url: null,
        alternates: [{ url: `${download}/ubiqx-shield-windows-x86_64.msi`, kind: 'msi', size: 3400 }],
      },
    });
    assert.deepEqual(JSON.parse(release.assets.find((a) => a.name === 'latest.json').content.toString()), feed);
    const updater = JSON.parse(release.assets.find((a) => a.name === UPDATER_ASSET_NAME).content.toString());
    assert.deepEqual(updater.platforms, { 'darwin-aarch64': { signature: 'c2hpZWxkIG1hYw==', url: `${download}/ubiqx-shield-macos-aarch64.app.tar.gz` } });

    // Nothing published carries the display name, and no request went to ubiqX's release.
    const { notes: _notes, ...structure } = feed;
    const published = [...release.assets.map((a) => a.name), release.name, JSON.stringify(structure), JSON.stringify(updater.platforms)].join('\n');
    assert.ok(!published.toLowerCase().includes(SHIELD_DISPLAY.toLowerCase()), `the display name leaked into what is published:\n${published}`);
    assert.ok(!published.includes('ubiqX-'), 'no file of the standalone app is named like one of ubiqX');
    assert.equal(snapshot('continuous'), ubiqxBefore, 'the ubiqX release is byte for byte what it was');
    assert.ok(!gh.state.log.some((l) => l.includes('/tags/continuous')), `a request went to the ubiqX tag: ${gh.state.log.join(', ')}`);
    assert.match(lines.join('\n'), /^ubiqx-shield 0\.1\.0 build 27 /m);
  });

  test('the standalone app\'s release never becomes the repository\'s "Latest" one; ubiqX\'s requests carry no such member', async () => {
    // GitHub serves one release at /releases/latest. Created: the request says "not this one".
    assert.equal(await run(shieldArgv(shieldBuild())), 0, lines.join('\n'));
    const shield = gh.state.releases.find((r) => r.tag_name === 'shield-continuous');
    assert.equal(shield.make_latest, 'false', 'the string GitHub expects, not a boolean');
    // Updated: said again, because an update without the member may hand the mark back.
    delete shield.make_latest;
    assert.equal(await run(shieldArgv(shieldBuild())), 0, lines.join('\n'));
    assert.equal(shield.make_latest, 'false');

    // ubiqX, created and then updated: the body of its requests is what it was before there were two products.
    assert.equal(await run(argv()), 0, lines.join('\n'));
    const ubiqx = gh.state.releases.find((r) => r.tag_name === 'continuous');
    assert.ok(!('make_latest' in ubiqx), 'creating the ubiqX release sends no make_latest');
    assert.equal(await run(argv()), 0, lines.join('\n'));
    assert.ok(!('make_latest' in ubiqx), 'updating the ubiqX release sends no make_latest');
    assert.deepEqual(Object.keys(ubiqx).sort(), ['assets', 'body', 'id', 'name', 'tag_name', 'target_commitish']);
  });

  test('publishing ubiqX leaves the standalone app\'s release alone, and its own names are what they always were', async () => {
    assert.equal(await run(shieldArgv(shieldBuild())), 0, lines.join('\n'));
    const shieldBefore = snapshot('shield-continuous');
    gh.state.log = [];
    lines = [];

    const code = await run(['--asset', dmg, '--asset', appZip, '--asset', macUpdater, '--asset', winExe, '--asset', winMsi, '--asset', linuxAppImage, '--asset', linuxDeb, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 0, lines.join('\n'));
    const release = gh.state.releases.find((r) => r.tag_name === 'continuous');
    assert.equal(release.name, 'ubiqX (build contínuo)');
    assert.deepEqual(release.assets.map((a) => a.name), [
      'ubiqX-macos-aarch64.dmg',
      'ubiqX-macos-aarch64.app.zip',
      'ubiqX-macos-aarch64.app.tar.gz',
      'ubiqX-windows-x86_64-setup.exe',
      'ubiqX-windows-x86_64.msi',
      'ubiqX-linux-x86_64.AppImage',
      'ubiqX-linux-x86_64.deb',
      'ubiqX-macos-aarch64.app.tar.gz.sig',
      'latest.json',
      UPDATER_ASSET_NAME,
    ]);
    const feed = JSON.parse(release.assets.find((a) => a.name === 'latest.json').content.toString());
    assert.equal(feed.product, 'ubiqX');
    assert.equal(feed.release_url, `https://github.com/${OWNER}/${REPO}/releases/tag/continuous`);
    assert.equal(feed.platforms['darwin-aarch64'].url, `https://github.com/${OWNER}/${REPO}/releases/download/continuous/ubiqX-macos-aarch64.dmg`);
    assert.equal(snapshot('shield-continuous'), shieldBefore);
    assert.ok(!gh.state.log.some((l) => l.includes('shield')), `a request went to the standalone release: ${gh.state.log.join(', ')}`);
    assert.match(lines.join('\n'), /^ubiqX 0\.1\.0 build 27 /m);
  });

  test('a file of the other product is refused before any request, in both directions', async () => {
    const files = shieldBuild();
    // The standalone app's installers handed to a ubiqX publish (the default product).
    for (const stray of [files.dmg, files.deb, files.macUpdater]) {
      lines = [];
      const code = await run(['--asset', dmg, '--asset', stray, '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`]);
      assert.equal(code, 1, lines.join('\n'));
      assert.match(lines.join('\n'), /é do produto shield, não do ubiqx: publique-o com --product shield/);
    }
    // The historical shortcut, pointed at a folder where the standalone DMG is the newest file.
    lines = [];
    assert.equal(await run(['--dmg', path.join(path.dirname(files.dmg), '*.dmg'), '--out', out, '--arch', 'aarch64', '--api', api, '--uploads', `${api}/uploads`]), 1);
    assert.match(lines.join('\n'), /é do produto shield/);
    // ubiqX's installers handed to a publish of the standalone app, by Tauri's name and by the stable one.
    for (const stray of [dmg, appZip, linuxDeb, macUpdater]) {
      lines = [];
      const code = await run(shieldArgv({ stray }));
      assert.equal(code, 1, lines.join('\n'));
      assert.match(lines.join('\n'), /é do produto ubiqx, não do shield: publique-o com --product ubiqx/);
    }
    assert.deepEqual(gh.state.log, [], 'nothing reached GitHub');
  });

  test('a release that carries the other product\'s feed is left alone', async () => {
    // A tag that is nobody's default, so the refusal can only come from the feed that is published there.
    const ubiqxRelease = gh.seedRelease({ feedEpoch: 1758000000, tag: 'nightly', product: 'ubiqX' });
    let code = await run(shieldArgv(shieldBuild(), '--tag', 'nightly'));
    assert.equal(code, 1, lines.join('\n'));
    assert.match(lines.join('\n'), /traz o latest\.json de outro produto \("ubiqX"\)/);
    assert.deepEqual(gh.state.log.filter((l) => !l.startsWith('GET ')), [], 'no write to a release of the other product');
    assert.equal(ubiqxRelease.name, 'old');
    assert.equal(ubiqxRelease.assets.find((a) => a.name === 'ubiqX-macos-aarch64.dmg').content.toString(), 'old');

    // A feed from before the field existed is ubiqX's: the standalone app may not replace it, ubiqX may.
    gh.reset();
    lines = [];
    gh.seedRelease({ feedEpoch: 1758000000, tag: 'nightly', product: null });
    code = await run(shieldArgv(shieldBuild(), '--tag', 'nightly'));
    assert.equal(code, 1, lines.join('\n'));
    assert.deepEqual(gh.state.log.filter((l) => !l.startsWith('GET ')), []);
    code = await run(argv('--tag', 'nightly'));
    assert.equal(code, 0, lines.join('\n'));
    assert.equal(gh.state.releases[0].name, 'ubiqX (build contínuo)');

    // The other direction: ubiqX never writes into a release of the standalone app.
    gh.reset();
    lines = [];
    const shieldRelease = gh.seedRelease({ feedEpoch: 1758000000, tag: 'nightly', product: 'ubiqx-shield', assets: ['ubiqx-shield-macos-aarch64.dmg'] });
    code = await run(argv('--tag', 'nightly'));
    assert.equal(code, 1, lines.join('\n'));
    assert.match(lines.join('\n'), /de outro produto \("ubiqx-shield"\)/);
    assert.deepEqual(gh.state.log.filter((l) => !l.startsWith('GET ')), []);
    assert.equal(shieldRelease.name, 'old');
  });

  test('a release that holds the other product\'s files is left alone, with no feed or with one that cannot be read', async () => {
    // No latest.json at all: only the files say whose release this is. updater.json has no product field,
    // and it is what ubiqX's installed apps would take a replaced file from.
    const ubiqxRelease = gh.seedRelease({ tag: 'nightly', assets: ['ubiqX-macos-aarch64.dmg', 'ubiqX-macos-aarch64.app.tar.gz', UPDATER_ASSET_NAME] });
    let code = await run(shieldArgv(shieldBuild(), '--tag', 'nightly'));
    assert.equal(code, 1, lines.join('\n'));
    assert.match(lines.join('\n'), /A release "nightly" traz arquivos de outro produto \(ubiqX-macos-aarch64\.dmg, ubiqX-macos-aarch64\.app\.tar\.gz\), não do ubiqx-shield/);
    assert.deepEqual(gh.state.log.filter((l) => !l.startsWith('GET ')), [], 'no write to a release of the other product');
    assert.deepEqual(ubiqxRelease.assets.map((a) => [a.name, a.content.toString()]), [['ubiqX-macos-aarch64.dmg', 'old'], ['ubiqX-macos-aarch64.app.tar.gz', 'old'], [UPDATER_ASSET_NAME, 'old']]);
    assert.equal(gh.state.refs['refs/tags/nightly'], 'a'.repeat(40), 'the tag did not move');

    // A latest.json that is there and cannot be read counts as no feed; the files still decide.
    gh.reset();
    lines = [];
    const damaged = gh.seedRelease({ tag: 'nightly', assets: ['ubiqX-linux-x86_64.deb', 'latest.json'] });
    code = await run(shieldArgv(shieldBuild(), '--tag', 'nightly'));
    assert.equal(code, 1, lines.join('\n'));
    assert.match(lines.join('\n'), /latest\.json atual ilegível/);
    assert.match(lines.join('\n'), /traz arquivos de outro produto \(ubiqX-linux-x86_64\.deb\)/);
    assert.deepEqual(gh.state.log.filter((l) => !l.startsWith('GET ')), []);
    assert.equal(damaged.name, 'old');

    // The other direction, by the name Tauri gives the standalone app's files as well as the published one.
    for (const name of ['ubiqx-shield-windows-x86_64-setup.exe', `${SHIELD_DISPLAY}_0.1.0_amd64.deb`]) {
      gh.reset();
      lines = [];
      const shieldRelease = gh.seedRelease({ tag: 'nightly', assets: [name] });
      code = await run(argv('--tag', 'nightly'));
      assert.equal(code, 1, lines.join('\n'));
      assert.match(lines.join('\n'), /traz arquivos de outro produto \(.+\), não do ubiqX\./);
      assert.deepEqual(gh.state.log.filter((l) => !l.startsWith('GET ')), []);
      assert.equal(shieldRelease.assets[0].content.toString(), 'old');
    }

    // A release without a feed that holds this product's own files, or files that are nobody's, is published to.
    gh.reset();
    lines = [];
    gh.seedRelease({ tag: 'nightly', assets: ['ubiqX-macos-aarch64.dmg', 'notes.txt'] });
    code = await run(argv('--tag', 'nightly'));
    assert.equal(code, 0, lines.join('\n'));
    assert.deepEqual(gh.state.releases[0].assets.map((a) => a.name).sort(), ['latest.json', 'notes.txt', 'ubiqX-macos-aarch64.app.zip', 'ubiqX-macos-aarch64.dmg']);
  });

  test('a product never publishes under the other product\'s tag, an unknown product is a usage error, and the standalone app has no default file', async () => {
    let code = await run(shieldArgv(shieldBuild(), '--tag', 'continuous'));
    assert.equal(code, 2, lines.join('\n'));
    assert.match(lines.join('\n'), /A tag "continuous" é a release do produto ubiqx/);
    lines = [];
    code = await run(argv('--tag', 'shield-continuous'));
    assert.equal(code, 2, lines.join('\n'));
    assert.match(lines.join('\n'), /A tag "shield-continuous" é a release do produto shield/);
    lines = [];
    code = await run(argv('--product', 'escudo'));
    assert.equal(code, 2, lines.join('\n'));
    assert.match(lines.join('\n'), /Produto desconhecido: "escudo"/);
    lines = [];
    code = await run(['--product', 'shield', '--out', out, '--api', api, '--uploads', `${api}/uploads`]);
    assert.equal(code, 1, lines.join('\n'));
    assert.match(lines.join('\n'), /--product shield precisa de --asset/);
    assert.deepEqual(gh.state.log, []);
  });
});

describe('helpers', () => {
  test('parseArgs takes defaults, --key value and --key=value', () => {
    const o = parseArgs(['--tag', 'nightly', '--api=http://x', '--strict']);
    assert.equal(o.tag, 'nightly');
    assert.equal(o.api, 'http://x');
    assert.equal(o.strict, true);
    assert.equal(o.dmg, null, 'the DMG default only applies when neither --asset nor --dmg is given');
    assert.deepEqual(o.assets, []);
    assert.equal(o.uploads, 'https://uploads.github.com');
    assert.deepEqual(parseArgs(['--asset', 'a.dmg', '--asset=b.deb']).assets, ['a.dmg', 'b.deb']);
    assert.throws(() => parseArgs(['--bogus']), /Opção desconhecida/);
    assert.throws(() => parseArgs(['--tag']), /Falta o valor/);
  });

  test('parseRepo accepts slugs and remote URLs', () => {
    assert.deepEqual(parseRepo('andreyquadros/ubiquitous-engine'), { owner: 'andreyquadros', repo: 'ubiquitous-engine' });
    assert.deepEqual(parseRepo('https://github.com/andreyquadros/ubiquitous-engine.git'), { owner: 'andreyquadros', repo: 'ubiquitous-engine' });
    assert.deepEqual(parseRepo('git@github.com:andreyquadros/ubiquitous-engine.git'), { owner: 'andreyquadros', repo: 'ubiquitous-engine' });
    assert.equal(parseRepo(''), null);
  });

  test('trimNotes drops git trailers and caps the length', () => {
    const raw = 'feat: thing\n\nBody line 1\nBody line 2\n\nSigned-off-by: Someone <s@example.com>\nReviewed-by: Other <o@example.com>\n\n';
    assert.equal(trimNotes(raw), 'feat: thing\n\nBody line 1\nBody line 2');
    assert.equal(trimNotes('Only-Trailer: x'), 'Only-Trailer: x', 'a lone trailer-looking subject is kept');
    const long = Array.from({ length: 60 }, (_, i) => `line ${i}`).join('\n');
    assert.equal(trimNotes(long, 40).split('\n').length, 40);
  });

  test('makeFeed builds stable download URLs', () => {
    const asset = { ...inferAsset('/x/ubiqX_0.2.0_x64.dmg', 'aarch64'), size: 10 };
    const feed = makeFeed({ owner: 'o', repo: 'r', tag: 'continuous', version: '0.2.0', build: { epoch: 1, number: 2, sha: 'abc1234', branch: 'main' }, assets: [asset], publishedAt: '2026-01-01T00:00:00Z', notes: 'n' });
    assert.equal(feed.platforms['darwin-x86_64'].url, 'https://github.com/o/r/releases/download/continuous/ubiqX-macos-x86_64.dmg');
    assert.equal(feed.platforms['darwin-x86_64'].kind, 'dmg');
    assert.equal(feed.platforms['darwin-x86_64'].app_zip_url, null);
    assert.equal(feed.platforms['darwin-x86_64'].alternates, undefined);
    assert.equal(feed.release_url, 'https://github.com/o/r/releases/tag/continuous');
  });

  test('inferAsset reads platform, kind and arch from the file name', () => {
    const pick = ({ kind, platformKey, name }) => ({ kind, platformKey, name });
    assert.deepEqual(pick(inferAsset('ubiqX-macos-aarch64.dmg', 'x86_64')), { kind: 'dmg', platformKey: 'darwin-aarch64', name: 'ubiqX-macos-aarch64.dmg' });
    assert.deepEqual(pick(inferAsset('dist/ubiqX-macos-aarch64.app.zip', 'x86_64')), { kind: 'app_zip', platformKey: 'darwin-aarch64', name: 'ubiqX-macos-aarch64.app.zip' });
    assert.deepEqual(pick(inferAsset('ubiqX-windows-x86_64.msi', 'aarch64')), { kind: 'msi', platformKey: 'windows-x86_64', name: 'ubiqX-windows-x86_64.msi' });
    assert.deepEqual(pick(inferAsset('ubiqX-windows-x86_64-setup.exe', 'aarch64')), { kind: 'exe', platformKey: 'windows-x86_64', name: 'ubiqX-windows-x86_64-setup.exe' });
    assert.deepEqual(pick(inferAsset('ubiqX-linux-x86_64.AppImage', 'aarch64')), { kind: 'appimage', platformKey: 'linux-x86_64', name: 'ubiqX-linux-x86_64.AppImage' });
    assert.deepEqual(pick(inferAsset('ubiqX-linux-aarch64.deb', 'x86_64')), { kind: 'deb', platformKey: 'linux-aarch64', name: 'ubiqX-linux-aarch64.deb' });
    // Tauri's own output names: the arch token wins over the fallback, the extension gives the kind
    assert.deepEqual(pick(inferAsset('ubiqX_0.1.0_x64_en-US.msi', 'aarch64')), { kind: 'msi', platformKey: 'windows-x86_64', name: 'ubiqX-windows-x86_64.msi' });
    assert.deepEqual(pick(inferAsset('ubiqX_0.1.0_x64-setup.exe', 'aarch64')), { kind: 'exe', platformKey: 'windows-x86_64', name: 'ubiqX-windows-x86_64-setup.exe' });
    assert.deepEqual(pick(inferAsset('ubiqx_0.1.0_amd64.AppImage', 'aarch64')), { kind: 'appimage', platformKey: 'linux-x86_64', name: 'ubiqX-linux-x86_64.AppImage' });
    assert.deepEqual(pick(inferAsset('ubiqx_0.1.0_amd64.deb', 'aarch64')), { kind: 'deb', platformKey: 'linux-x86_64', name: 'ubiqX-linux-x86_64.deb' });
    assert.deepEqual(pick(inferAsset('ubiqX_0.1.0_aarch64.dmg', 'x86_64')), { kind: 'dmg', platformKey: 'darwin-aarch64', name: 'ubiqX-macos-aarch64.dmg' });
    // no arch anywhere: --arch decides
    assert.deepEqual(pick(inferAsset('ubiqX.dmg', 'arm64')), { kind: 'dmg', platformKey: 'darwin-aarch64', name: 'ubiqX-macos-aarch64.dmg' });
    assert.equal(inferAsset('ubiqX-windows-x86_64.txt', 'x86_64'), null);
    assert.equal(inferAsset('ubiqX-linux-x86_64.msi', 'x86_64'), null, 'os and extension must agree');
  });

  test('buildPlatforms keeps one entry per key, the primary kind first and the rest as alternates', () => {
    const d = 'https://d';
    const a = (name, size) => ({ ...inferAsset(name, 'x86_64'), size });
    const platforms = buildPlatforms([a('ubiqX-windows-x86_64.msi', 2), a('ubiqX-windows-x86_64-setup.exe', 1), a('ubiqX-linux-x86_64.deb', 4), a('ubiqX-linux-x86_64.AppImage', 3)], d);
    assert.deepEqual(Object.keys(platforms), ['linux-x86_64', 'windows-x86_64']);
    assert.equal(platforms['windows-x86_64'].url, `${d}/ubiqX-windows-x86_64-setup.exe`);
    assert.deepEqual(platforms['windows-x86_64'].alternates, [{ url: `${d}/ubiqX-windows-x86_64.msi`, kind: 'msi', size: 2 }]);
    assert.equal(platforms['linux-x86_64'].kind, 'appimage');
    assert.deepEqual(platforms['linux-x86_64'].alternates, [{ url: `${d}/ubiqX-linux-x86_64.deb`, kind: 'deb', size: 4 }]);
    assert.throws(() => buildPlatforms([a('ubiqX-macos-x86_64.app.zip', 1)], d), /sem o instalador/);
  });

  test('compareFeeds and mergeFeeds', () => {
    const ours = { build: { epoch: 100 }, platforms: { 'darwin-aarch64': { size: 2 } } };
    assert.equal(compareFeeds(null, ours), 'newer');
    assert.equal(compareFeeds({ build: {} }, ours), 'newer');
    assert.equal(compareFeeds({ build: { epoch: 99 } }, ours), 'newer');
    assert.equal(compareFeeds({ build: { epoch: 100 } }, ours), 'same');
    assert.equal(compareFeeds({ build: { epoch: 101 } }, ours), 'older');
    const merged = mergeFeeds({ build: { epoch: 100 }, platforms: { 'windows-x86_64': { size: 7 }, 'darwin-aarch64': { size: 1 } } }, ours);
    assert.deepEqual(merged, { build: { epoch: 100 }, platforms: { 'darwin-aarch64': { size: 2 }, 'windows-x86_64': { size: 7 } } });
    assert.deepEqual(mergeFeeds(null, ours).platforms, ours.platforms);
  });

  test('inferAsset knows the updater artifacts and marks them as such', () => {
    const pick = ({ kind, platformKey, name, updater }) => ({ kind, platformKey, name, updater });
    // Tauri's own output names…
    assert.deepEqual(pick(inferAsset('bundle/macos/ubiqX.app.tar.gz', 'aarch64')), { kind: 'app_tar_gz', platformKey: 'darwin-aarch64', name: 'ubiqX-macos-aarch64.app.tar.gz', updater: true });
    assert.deepEqual(pick(inferAsset('bundle/nsis/ubiqX_0.1.128_x64-setup.nsis.zip', 'aarch64')), { kind: 'nsis_zip', platformKey: 'windows-x86_64', name: 'ubiqX-windows-x86_64-setup.nsis.zip', updater: true });
    assert.deepEqual(pick(inferAsset('bundle/appimage/ubiqX_0.1.128_amd64.AppImage.tar.gz', 'aarch64')), { kind: 'appimage_tar_gz', platformKey: 'linux-x86_64', name: 'ubiqX-linux-x86_64.AppImage.tar.gz', updater: true });
    // …and the stable names CI renames them to, which must round-trip.
    for (const name of ['ubiqX-macos-aarch64.app.tar.gz', 'ubiqX-windows-x86_64-setup.nsis.zip', 'ubiqX-linux-x86_64.AppImage.tar.gz']) {
      assert.equal(inferAsset(name, 'x86_64').name, name, `${name} must keep its name`);
      assert.equal(inferAsset(name, 'x86_64').updater, true);
    }
    // Installers are not updater artifacts, and an updater name of the wrong OS is still a mistake.
    assert.equal(inferAsset('ubiqX-linux-x86_64.AppImage', 'x86_64').updater, false);
    assert.equal(inferAsset('ubiqX-macos-aarch64.app.zip', 'x86_64').updater, false);
    assert.equal(inferAsset('ubiqX-linux-x86_64.app.tar.gz', 'x86_64'), null, 'os and extension must agree');
  });

  test('buildPlatforms ignores updater artifacts so latest.json keeps schema 1', () => {
    const d = 'https://d';
    const a = (name, size) => ({ ...inferAsset(name, 'x86_64'), size, signature: 'sig' });
    const platforms = buildPlatforms([a('ubiqX-linux-x86_64.AppImage', 3), a('ubiqX-linux-x86_64.AppImage.tar.gz', 9)], d);
    assert.deepEqual(platforms, { 'linux-x86_64': { url: `${d}/ubiqX-linux-x86_64.AppImage`, kind: 'appimage', size: 3, app_zip_url: null } });
    // An updater artifact alone is not an installer: it neither builds an entry nor trips the "sem o instalador" guard.
    assert.deepEqual(buildPlatforms([a('ubiqX-macos-aarch64.app.tar.gz', 9)], d), {});
  });

  test('buildUpdaterPlatforms, makeUpdaterFeed and mergeUpdaterFeeds', () => {
    const d = 'https://d';
    const a = (name, signature) => ({ ...inferAsset(name, 'x86_64'), size: 1, signature });
    const platforms = buildUpdaterPlatforms([a('ubiqX-windows-x86_64-setup.nsis.zip', 'w'), a('ubiqX-macos-aarch64.app.tar.gz', 'm'), a('ubiqX-linux-x86_64.AppImage', null)], d);
    assert.deepEqual(Object.keys(platforms), ['darwin-aarch64', 'windows-x86_64'], 'sorted, installers left out');
    assert.deepEqual(platforms['darwin-aarch64'], { signature: 'm', url: `${d}/ubiqX-macos-aarch64.app.tar.gz` });
    assert.throws(() => buildUpdaterPlatforms([a('ubiqX-macos-aarch64.app.tar.gz', '')], d), /sem assinatura/);

    const feed = makeUpdaterFeed({ owner: 'o', repo: 'r', tag: 'continuous', version: '0.1.7', assets: [a('ubiqX-macos-aarch64.app.tar.gz', 'm')], publishedAt: '2026-01-01T00:00:00Z', notes: 'n' });
    assert.deepEqual(feed, { version: '0.1.7', notes: 'n', pub_date: '2026-01-01T00:00:00Z', platforms: { 'darwin-aarch64': { signature: 'm', url: 'https://github.com/o/r/releases/download/continuous/ubiqX-macos-aarch64.app.tar.gz' } } });
    assert.equal(makeUpdaterFeed({ owner: 'o', repo: 'r', tag: 'continuous', version: '0.1.7', assets: [a('ubiqX-macos-aarch64.dmg', null)], publishedAt: 'p', notes: 'n' }), null, 'no updater artifact, no feed');

    const ours = { version: '0.1.7', platforms: { 'darwin-aarch64': { signature: 'new' } } };
    assert.deepEqual(mergeUpdaterFeeds(null, ours), ours);
    assert.deepEqual(mergeUpdaterFeeds({ version: '0.1.6', platforms: { 'windows-x86_64': { signature: 'old' } } }, ours), ours, 'another version is replaced, never merged');
    assert.deepEqual(mergeUpdaterFeeds({ version: '0.1.7', platforms: { 'windows-x86_64': { signature: 'old' }, 'darwin-aarch64': { signature: 'old' } } }, ours), {
      version: '0.1.7',
      platforms: { 'darwin-aarch64': { signature: 'new' }, 'windows-x86_64': { signature: 'old' } },
    });
  });

  test('productVersion prefers --version, then UBIQX_VERSION, then tauri.conf.json', () => {
    const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
    assert.equal(productVersion({ version: '9.9.9' }, root, { UBIQX_VERSION: '0.1.5' }), '9.9.9');
    assert.equal(productVersion({ version: null }, root, { UBIQX_VERSION: '0.1.5' }), '0.1.5');
    assert.equal(productVersion({ version: null }, root, {}), baseVersion(root), 'the checked-in tauri.conf.json version');
    assert.equal(productVersion({ version: null }, '/nowhere', {}), '0.0.0');
  });

  test('productVersion reads the configuration of the product being published', () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'ubiqx-products-'));
    try {
      for (const [product, version] of [[PRODUCTS.ubiqx, '0.1.128'], [PRODUCTS.shield, '0.1.640']]) {
        const conf = path.join(dir, product.tauriConf);
        mkdirSync(path.dirname(conf), { recursive: true });
        writeFileSync(conf, JSON.stringify({ version }));
      }
      assert.equal(productVersion({ version: null }, dir, {}), '0.1.128', 'ubiqX unless a product is given');
      assert.equal(productVersion({ version: null }, dir, {}, PRODUCTS.ubiqx), '0.1.128');
      assert.equal(productVersion({ version: null }, dir, {}, PRODUCTS.shield), '0.1.640');
      assert.equal(productVersion({ version: '2.0.0' }, dir, {}, PRODUCTS.shield), '2.0.0');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('inferAsset names the standalone app\'s files after its machine id', () => {
    const shield = PRODUCTS.shield;
    const pick = ({ kind, platformKey, name, updater }) => ({ kind, platformKey, name, updater });
    // Tauri's output names (after the display name) and the stable names CI renames them to.
    assert.deepEqual(pick(inferAsset(`${SHIELD_DISPLAY}_0.1.0_aarch64.dmg`, 'x86_64', shield)), { kind: 'dmg', platformKey: 'darwin-aarch64', name: 'ubiqx-shield-macos-aarch64.dmg', updater: false });
    assert.deepEqual(pick(inferAsset(`${SHIELD_DISPLAY}_0.1.0_x64-setup.exe`, 'aarch64', shield)), { kind: 'exe', platformKey: 'windows-x86_64', name: 'ubiqx-shield-windows-x86_64-setup.exe', updater: false });
    assert.deepEqual(pick(inferAsset(`${SHIELD_DISPLAY}_0.1.0_amd64.AppImage.tar.gz`, 'aarch64', shield)), { kind: 'appimage_tar_gz', platformKey: 'linux-x86_64', name: 'ubiqx-shield-linux-x86_64.AppImage.tar.gz', updater: true });
    for (const name of [
      'ubiqx-shield-macos-aarch64.dmg',
      'ubiqx-shield-macos-x86_64.app.zip',
      'ubiqx-shield-macos-aarch64.app.tar.gz',
      'ubiqx-shield-windows-x86_64-setup.exe',
      'ubiqx-shield-windows-x86_64.msi',
      'ubiqx-shield-windows-x86_64-setup.nsis.zip',
      'ubiqx-shield-linux-x86_64.AppImage',
      'ubiqx-shield-linux-aarch64.deb',
      'ubiqx-shield-linux-x86_64.AppImage.tar.gz',
    ]) {
      // The arch in a stable name wins over --arch, so a published name is a fixed point.
      const other = name.includes('aarch64') ? 'x86_64' : 'aarch64';
      assert.equal(inferAsset(name, other, shield)?.name, name, `${name} must keep its name`);
    }
    assert.equal(inferAsset('ubiqx-shield-linux-x86_64.msi', 'x86_64', shield), null, 'os and extension must agree');
    assert.equal(inferAsset('ubiqx-shield-linux-x86_64.txt', 'x86_64', shield), null);
    // The glob that collects ubiqX's files in CI (`ubiqX-*`, case-sensitive) matches none of these names.
    assert.ok(!/^ubiqX-/.test(inferAsset('x.dmg', 'aarch64', shield).name));
  });

  test('productOf tells whose file a name is, by the longest name it starts with', () => {
    const stems = productStems(REPO_ROOT);
    assert.ok(stems.shield.includes(SHIELD_DISPLAY.toLowerCase()), 'the display name is read from the shell\'s configuration');
    const shieldNames = ['ubiqx-shield-macos-aarch64.dmg', 'UBIQX-SHIELD-linux-x86_64.deb', 'ubiqx-shield_0.1.0_amd64.deb', `${SHIELD_DISPLAY}_0.1.0_aarch64.dmg`, `${SHIELD_DISPLAY.toLowerCase()}_0.1.0_amd64.deb`, `/a/b/${SHIELD_DISPLAY}.app.tar.gz`];
    for (const name of shieldNames) assert.equal(productOf(name, stems), 'shield', name);
    const ubiqxNames = ['ubiqX-macos-aarch64.dmg', 'ubiqX_0.1.0_x64_en-US.msi', 'ubiqx_0.1.0_amd64.deb', 'bundle/macos/ubiqX.app.tar.gz', 'ubiqX.dmg'];
    for (const name of ubiqxNames) assert.equal(productOf(name, stems), 'ubiqx', name);
    // A name that starts with neither is nobody's, and is published as before.
    for (const name of ['installer.dmg', 'ubiqXtra.dmg', `${SHIELD_DISPLAY}plus.dmg`, 'x-ubiqx-shield.deb']) assert.equal(productOf(name, stems), null, name);
    // Without a readable checkout the published prefixes still tell the two apart.
    const bare = productStems('/nowhere');
    assert.deepEqual(bare, { ubiqx: ['ubiqx'], shield: ['ubiqx-shield'] });
    assert.equal(productOf('ubiqx-shield-linux-x86_64.deb', bare), 'shield');
    assert.equal(productOf('ubiqX-linux-x86_64.deb', bare), 'ubiqx');
  });

  test('collectAssets refuses the other product\'s file and gives the standalone app no default', () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'ubiqx-collect-'));
    try {
      const mine = path.join(dir, 'ubiqX_0.1.0_aarch64.dmg');
      const theirs = path.join(dir, 'ubiqx-shield-macos-aarch64.dmg');
      writeFileSync(mine, 'x');
      writeFileSync(theirs, 'yy');
      const opts = (assets) => ({ assets, dmg: null, appZip: null });
      assert.deepEqual(collectAssets(opts([mine]), REPO_ROOT, 'aarch64').map((a) => a.name), ['ubiqX-macos-aarch64.dmg']);
      assert.deepEqual(collectAssets(opts([theirs]), REPO_ROOT, 'aarch64', PRODUCTS.shield).map((a) => [a.name, a.size]), [['ubiqx-shield-macos-aarch64.dmg', 2]]);
      assert.throws(() => collectAssets(opts([theirs]), REPO_ROOT, 'aarch64'), /é do produto shield, não do ubiqx/);
      assert.throws(() => collectAssets(opts([mine]), REPO_ROOT, 'aarch64', PRODUCTS.shield), /é do produto ubiqx, não do shield/);
      assert.throws(() => collectAssets(opts([]), REPO_ROOT, 'aarch64', PRODUCTS.shield), /precisa de --asset/);
      // ubiqX's bare call looks for its own DMG, not for whichever DMG is newest in the shared folder.
      assert.throws(() => collectAssets(opts([]), dir, 'aarch64'), /DMG não encontrado: target\/release\/bundle\/dmg\/ubiqX\*\.dmg/);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('foreignAssets names the files of a release that are the other product\'s', () => {
    const stems = productStems(REPO_ROOT);
    const assets = ['ubiqX-macos-aarch64.dmg', 'ubiqX-macos-aarch64.app.tar.gz.sig', 'ubiqx-shield-linux-x86_64.deb', `${SHIELD_DISPLAY}.app.tar.gz`, 'latest.json', UPDATER_ASSET_NAME, 'notes.txt'].map((name) => ({ name }));
    assert.deepEqual(foreignAssets(assets, PRODUCTS.shield, stems), ['ubiqX-macos-aarch64.dmg', 'ubiqX-macos-aarch64.app.tar.gz.sig']);
    assert.deepEqual(foreignAssets(assets, PRODUCTS.ubiqx, stems), ['ubiqx-shield-linux-x86_64.deb', `${SHIELD_DISPLAY}.app.tar.gz`]);
    assert.deepEqual(foreignAssets(undefined, PRODUCTS.ubiqx, stems), []);
    assert.deepEqual(foreignAssets([{}, { name: null }], PRODUCTS.shield, stems), []);
  });

  test('feedBelongsTo follows the rule of the app: the exact name, and a feed without one is ubiqX\'s', () => {
    const { ubiqx, shield } = PRODUCTS;
    assert.equal(feedBelongsTo({ product: 'ubiqX' }, ubiqx), true);
    assert.equal(feedBelongsTo({ product: 'ubiqX' }, shield), false);
    assert.equal(feedBelongsTo({ product: 'ubiqx-shield' }, shield), true);
    assert.equal(feedBelongsTo({ product: 'ubiqx-shield' }, ubiqx), false);
    assert.equal(feedBelongsTo({ product: 'UBIQX' }, ubiqx), false, 'case matters, as in feed_matches_product');
    for (const blank of [{}, { product: '' }, { product: '  ' }, { product: null }, { product: 7 }]) {
      assert.equal(feedBelongsTo(blank, ubiqx), true, JSON.stringify(blank));
      assert.equal(feedBelongsTo(blank, shield), false, JSON.stringify(blank));
    }
    assert.equal(makeFeed({ owner: 'o', repo: 'r', tag: 't', version: '1.0.0', build: { epoch: 1, number: 1, sha: 's', branch: 'b' }, assets: [], publishedAt: 'p', notes: 'n' }).product, 'ubiqX', 'the default is unchanged');
  });

  test('parseArgs gives each product its own tag and refuses the other one\'s', () => {
    assert.equal(parseArgs([]).product, 'ubiqx');
    assert.equal(parseArgs([]).tag, 'continuous');
    assert.equal(parseArgs(['--product', 'shield']).tag, 'shield-continuous');
    assert.equal(parseArgs(['--product=shield', '--tag', 'nightly']).tag, 'nightly');
    assert.throws(() => parseArgs(['--product', 'shield', '--tag', 'continuous']), /é a release do produto ubiqx/);
    assert.throws(() => parseArgs(['--tag=shield-continuous']), /é a release do produto shield/);
    assert.throws(() => parseArgs(['--product', 'nofap']), /Produto desconhecido/);
    assert.throws(() => parseArgs(['--product']), /Falta o valor/);
  });

  test('the product table agrees with the Rust identities and with both shells', () => {
    assert.deepEqual(Object.keys(PRODUCTS), ['ubiqx', 'shield']);
    assert.equal(resolveProduct(), PRODUCTS.ubiqx, 'ubiqX is the default');
    assert.throws(() => resolveProduct('toString'), /Produto desconhecido/, 'only the two ids are products');

    // crates/ubiqx-core/src/product.rs: the ids, the feed names the apps accept and the feed URLs they poll.
    const rust = readRepoFile('crates/ubiqx-core/src/product.rs');
    const feedUrl = {
      ubiqx: readRepoFile('crates/ubiqx-core/src/update.rs').match(/pub const DEFAULT_FEED_URL: &str =\s*"([^"]+)"/)?.[1],
      shield: rust.match(/pub const SHIELD_FEED_URL: &str =\s*"([^"]+)"/)?.[1],
    };
    for (const [id, constant] of [['ubiqx', 'UBIQX'], ['shield', 'SHIELD']]) {
      const product = PRODUCTS[id];
      assert.equal(product.id, id);
      assert.ok(rust.includes(`=> "${id}"`), `ProductId::as_str has no "${id}"`);
      const identity = rust.match(new RegExp(`pub static ${constant}: ProductIdentity = ProductIdentity \\{([\\s\\S]*?)\\n\\};`))?.[1];
      assert.ok(identity, `static ${constant} not found in product.rs`);
      assert.equal(identity.match(/feed_product: "([^"]+)"/)?.[1], product.feedProduct, `${id}: the feed name`);
      assert.equal(feedUrl[id], `https://github.com/${OWNER}/${REPO}/releases/download/${product.tag}/latest.json`, `${id}: the app polls the release this script publishes`);

      // The shell: its version file exists, and its updater endpoint (once it has one) is this tag's updater.json.
      const conf = JSON.parse(readRepoFile(product.tauriConf));
      assert.match(conf.version, /^\d+\.\d+\.\d+$/);
      for (const endpoint of conf.plugins?.updater?.endpoints ?? []) {
        assert.equal(endpoint, `https://github.com/${OWNER}/${REPO}/releases/download/${product.tag}/${UPDATER_ASSET_NAME}`, `${id}: updater endpoint`);
      }
    }
    assert.equal(JSON.parse(readRepoFile(PRODUCTS.ubiqx.tauriConf)).plugins.updater.endpoints.length, 1, 'ubiqX has its updater endpoint');
    assert.equal(JSON.parse(readRepoFile(PRODUCTS.ubiqx.tauriConf)).productName, PRODUCTS.ubiqx.assetPrefix);

    // The standalone app is published under its machine id (design 0.1 and 8.6): the binary name of its shell,
    // `SYSTEM_ID` in the core, and never the display name, which may still change.
    const shield = PRODUCTS.shield;
    const shieldConf = JSON.parse(readRepoFile(shield.tauriConf));
    assert.equal(shield.assetPrefix, shieldConf.mainBinaryName);
    assert.equal(shield.assetPrefix, readRepoFile('crates/ubiqx-core/src/nofap.rs').match(/pub const SYSTEM_ID: &str = "([^"]+)"/)?.[1]);
    for (const field of ['label', 'assetPrefix', 'feedProduct', 'tag', 'releaseName']) {
      assert.ok(!/nofap/i.test(shield[field]), `${field} carries the source name: ${shield[field]}`);
      assert.ok(!shield[field].toLowerCase().includes(shieldConf.productName.toLowerCase()), `${field} carries the display name: ${shield[field]}`);
    }
    // The names of the two products never collide, whatever the letter case of the file system.
    assert.notEqual(shield.tag, PRODUCTS.ubiqx.tag);
    assert.notEqual(shield.feedProduct.toLowerCase(), PRODUCTS.ubiqx.feedProduct.toLowerCase());
    assert.notEqual(inferAsset('a.dmg', 'aarch64', shield).name.toLowerCase(), inferAsset('a.dmg', 'aarch64').name.toLowerCase());
  });

  test('resolveGlob picks a matching file and returns null when none matches', () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'ubiqx-glob-'));
    try {
      writeFileSync(path.join(dir, 'ubiqX_0.1.0_aarch64.dmg'), 'x');
      assert.equal(resolveGlob(path.join(dir, '*.dmg'), '/'), path.join(dir, 'ubiqX_0.1.0_aarch64.dmg'));
      assert.equal(resolveGlob(path.join(dir, '*.zip'), '/'), null);
      assert.equal(resolveGlob('rel/file.dmg', '/base'), path.join('/base', 'rel/file.dmg'));
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

// ---------------------------------------------------------------------------------------------
// scripts/app-version.mjs: the version CI stamps into tauri.conf.json, which both feeds then carry.

describe('app-version', () => {
  const REAL_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const CONF = path.join(REAL_ROOT, 'apps/desktop/src-tauri/tauri.conf.json');

  test('the patch is the build number of the commit, on top of the major.minor of Cargo.toml', () => {
    const base = baseVersion(REAL_ROOT);
    const [major, minor] = base.split('.');
    // The commit count every job already computes (scripts/build-info.sh) is what grows the version, so two
    // machines building the same commit agree instead of racing with their own counters.
    assert.equal(appVersion({ UBIQX_BUILD_NUMBER: '128' }, REAL_ROOT), `${major}.${minor}.128`);
    assert.equal(appVersion({ UBIQX_BUILD_NUMBER: '1' }, REAL_ROOT), `${major}.${minor}.1`);
    assert.equal(appVersion({ UBIQX_BUILD_NUMBER: '128', GITHUB_RUN_NUMBER: '7' }, REAL_ROOT), `${major}.${minor}.128`, 'the commit count wins');
    // …and the CI run number stands in for a checkout without history.
    assert.equal(appVersion({ GITHUB_RUN_NUMBER: '7' }, REAL_ROOT), `${major}.${minor}.7`);
    assert.equal(appVersion({ UBIQX_BUILD_NUMBER: '0', GITHUB_RUN_NUMBER: '7' }, REAL_ROOT), `${major}.${minor}.7`);
    assert.equal(buildNumber({ UBIQX_BUILD_NUMBER: '9' }), 9);
    assert.equal(buildNumber({}), null);
    // A local build (no numbers at all) keeps the base version and never pretends to be a published one.
    assert.equal(appVersion({}, REAL_ROOT), base);
    assert.equal(appVersion({ UBIQX_BUILD_NUMBER: '' }, REAL_ROOT), base);
    assert.equal(appVersion({ UBIQX_BUILD_NUMBER: 'x' }, REAL_ROOT), base);
    assert.equal(appVersion({ UBIQX_BUILD_NUMBER: '0' }, REAL_ROOT), base);
    // UBIQX_VERSION wins, so one job can hand the number to the next.
    assert.equal(appVersion({ UBIQX_VERSION: '0.2.3', UBIQX_BUILD_NUMBER: '128' }, REAL_ROOT), '0.2.3');
  });

  test('--write rewrites only the version of tauri.conf.json and prints it', () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'ubiqx-version-'));
    try {
      const conf = path.join(dir, 'apps/desktop/src-tauri/tauri.conf.json');
      mkdirSync(path.dirname(conf), { recursive: true });
      writeFileSync(path.join(dir, 'Cargo.toml'), '[workspace.package]\nversion = "0.1.0"\n');
      const before = readFileSync(CONF, 'utf8');
      writeFileSync(conf, before);

      const printed = [];
      const io = { log: (l) => printed.push(String(l)), error: (l) => printed.push(String(l)) };
      assert.equal(appVersionMain(['--write', '--root', dir], { UBIQX_BUILD_NUMBER: '128' }, io), 0, printed.join('\n'));
      assert.deepEqual(printed, ['0.1.128']);

      const after = readFileSync(conf, 'utf8');
      assert.equal(JSON.parse(after).version, '0.1.128');
      assert.equal(after, before.replace('"version": "0.1.0"', '"version": "0.1.128"'), 'no other byte of the file moves');
      // The pubkey and the endpoint the in-app updater needs survive the rewrite.
      assert.ok(JSON.parse(after).plugins.updater.pubkey.length > 0);
      assert.match(JSON.parse(after).plugins.updater.endpoints[0], /updater\.json$/);

      // Idempotent: writing the same version again changes nothing.
      assert.equal(writeAppVersion(dir, '0.1.128'), false);
      assert.equal(readFileSync(conf, 'utf8'), after);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  /** A scratch checkout with both shells' configuration files and the workspace version. */
  function scratchRoot(prefix) {
    const dir = mkdtempSync(path.join(tmpdir(), prefix));
    writeFileSync(path.join(dir, 'Cargo.toml'), '[workspace.package]\nversion = "0.1.0"\n');
    const confs = {};
    for (const product of Object.values(PRODUCTS)) {
      confs[product.id] = path.join(dir, product.tauriConf);
      mkdirSync(path.dirname(confs[product.id]), { recursive: true });
      copyFileSync(path.join(REAL_ROOT, product.tauriConf), confs[product.id]);
    }
    return { dir, confs };
  }

  test('--product shield writes the standalone app\'s configuration and nothing else', () => {
    const { dir, confs } = scratchRoot('ubiqx-version-shield-');
    try {
      const before = { ubiqx: readFileSync(confs.ubiqx, 'utf8'), shield: readFileSync(confs.shield, 'utf8') };
      const printed = [];
      const io = { log: (l) => printed.push(String(l)), error: (l) => printed.push(String(l)) };

      assert.equal(appVersionMain(['--write', '--product', 'shield', '--root', dir], { UBIQX_BUILD_NUMBER: '128' }, io), 0, printed.join('\n'));
      assert.deepEqual(printed, ['0.1.128']);
      const after = readFileSync(confs.shield, 'utf8');
      assert.equal(after, before.shield.replace('"version": "0.1.0"', '"version": "0.1.128"'), 'no other byte of the file moves');
      assert.equal(JSON.parse(after).mainBinaryName, 'ubiqx-shield');
      assert.equal(readFileSync(confs.ubiqx, 'utf8'), before.ubiqx, 'ubiqX is not stamped by a call for the other product');

      // The default product is ubiqX, with or without the option; both products get the same number.
      assert.equal(appVersionMain(['--write', '--root', dir], { UBIQX_BUILD_NUMBER: '128' }, io), 0);
      assert.equal(JSON.parse(readFileSync(confs.ubiqx, 'utf8')).version, '0.1.128');
      assert.equal(readFileSync(confs.shield, 'utf8'), after);
      assert.equal(appVersionMain(['--product=ubiqx', '--root', dir], { UBIQX_BUILD_NUMBER: '129' }, io), 0);
      assert.equal(printed.at(-1), '0.1.129');
      assert.equal(appVersion({ UBIQX_BUILD_NUMBER: '7' }, dir, 'shield'), appVersion({ UBIQX_BUILD_NUMBER: '7' }, dir, 'ubiqx'));
      assert.equal(writeAppVersion(dir, '0.1.128', 'shield'), false, 'idempotent');

      // An unknown product is a usage error and writes nothing.
      const stamped = { ubiqx: readFileSync(confs.ubiqx, 'utf8'), shield: readFileSync(confs.shield, 'utf8') };
      assert.equal(appVersionMain(['--write', '--product', 'escudo', '--root', dir], { UBIQX_BUILD_NUMBER: '500' }, io), 2);
      assert.match(printed.join('\n'), /Produto desconhecido: "escudo"/);
      assert.equal(appVersionMain(['--product'], {}, io), 2);
      assert.match(printed.join('\n'), /Falta o valor de --product/);
      assert.equal(readFileSync(confs.ubiqx, 'utf8'), stamped.ubiqx);
      assert.equal(readFileSync(confs.shield, 'utf8'), stamped.shield);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  // The wrapper the CI jobs call. It is a bash script, so the test runs where bash is (every CI job that
  // runs this file is on Linux).
  test('ci-app-version.sh stamps the product it is asked for and exports UBIQX_VERSION', { skip: process.platform === 'win32' && 'bash script' }, () => {
    const { dir, confs } = scratchRoot('ubiqx-ci-version-');
    try {
      mkdirSync(path.join(dir, 'scripts'));
      for (const name of ['ci-app-version.sh', 'app-version.mjs']) copyFileSync(path.join(REAL_ROOT, 'scripts', name), path.join(dir, 'scripts', name));
      const before = { ubiqx: readFileSync(confs.ubiqx, 'utf8'), shield: readFileSync(confs.shield, 'utf8') };
      const githubEnv = path.join(dir, 'github-env');
      const run = (...args) =>
        spawnSync('bash', [path.join(dir, 'scripts', 'ci-app-version.sh'), ...args], {
          encoding: 'utf8',
          // The build number is given, so the script does not need scripts/build-info.sh or git.
          env: { PATH: `${path.dirname(process.execPath)}${path.delimiter}${process.env.PATH}`, UBIQX_BUILD_NUMBER: '128', GITHUB_ENV: githubEnv },
        });

      let result = run('--product', 'shield');
      assert.equal(result.status, 0, result.stderr);
      assert.equal(result.stdout, 'ubiqx-shield 0.1.128 (build 128)\n');
      assert.equal(JSON.parse(readFileSync(confs.shield, 'utf8')).version, '0.1.128');
      assert.equal(readFileSync(confs.ubiqx, 'utf8'), before.ubiqx);
      assert.equal(readFileSync(githubEnv, 'utf8'), 'UBIQX_VERSION=0.1.128\n');

      // Without an option it is the call the jobs have always made.
      result = run();
      assert.equal(result.status, 0, result.stderr);
      assert.equal(result.stdout, 'ubiqX 0.1.128 (build 128)\n');
      assert.equal(JSON.parse(readFileSync(confs.ubiqx, 'utf8')).version, '0.1.128');
      assert.equal(readFileSync(githubEnv, 'utf8'), 'UBIQX_VERSION=0.1.128\nUBIQX_VERSION=0.1.128\n', 'both calls export the same number');

      for (const bad of [['--product', 'escudo'], ['--product'], ['--bogus']]) {
        const stamped = readFileSync(confs.shield, 'utf8');
        result = run(...bad);
        assert.equal(result.status, 2, `${bad.join(' ')}: ${result.stdout}${result.stderr}`);
        assert.equal(result.stdout, '');
        assert.equal(readFileSync(confs.shield, 'utf8'), stamped);
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('without --write nothing is touched, and an unknown option fails with the usage', () => {
    const printed = [];
    const io = { log: (l) => printed.push(String(l)), error: (l) => printed.push(String(l)) };
    const before = readFileSync(CONF, 'utf8');
    assert.equal(appVersionMain([], { UBIQX_BUILD_NUMBER: '7' }, io), 0);
    assert.equal(readFileSync(CONF, 'utf8'), before, 'a plain run never edits the checked-in config');
    assert.equal(appVersionMain(['--bogus'], {}, io), 2);
    assert.match(printed.join('\n'), /Opção desconhecida/);
    assert.equal(appVersionMain(['--help'], {}, io), 0);
    assert.match(printed.join('\n'), /^Uso: node scripts\/app-version\.mjs/m);
  });
});

// ---------------------------------------------------------------------------------------------
// The CI workflow that feeds the publisher: .github/workflows/ci.yml builds both products in the same
// jobs, into the same target/release/bundle/<kind>/ directories, so what keeps their files, releases and
// feeds apart is a handful of globs and conditions. Node has no YAML parser, so the workflow is read with a
// small reader of the subset it uses (block mappings, step lists, `|` literal blocks).

describe('ci workflow', () => {
  const WORKFLOW = '.github/workflows/ci.yml';
  const text = readRepoFile(WORKFLOW).replace(/\r\n/g, '\n');
  const lines = text.split('\n');

  /** The keys of a block mapping at exactly `indent` spaces, each with its inline value and its child lines. */
  function keysAt(block, indent) {
    const out = new Map();
    const prefix = ' '.repeat(indent);
    for (let i = 0; i < block.length; i += 1) {
      const line = block[i];
      if (!line.startsWith(prefix) || line.startsWith(`${prefix} `)) continue;
      const m = /^([A-Za-z_][\w-]*):(?: (.*))?$/.exec(line.slice(indent));
      if (!m) continue;
      const children = [];
      for (let j = i + 1; j < block.length && (block[j].trim() === '' || block[j].startsWith(`${prefix} `)); j += 1) children.push(block[j]);
      out.set(m[1], { value: (m[2] ?? '').trim(), children });
    }
    return out;
  }

  /** A scalar: the inline value, or the dedented content of a `|` literal block. */
  function scalar(entry) {
    if (!entry) return undefined;
    if (entry.value !== '|') return entry.value;
    const body = [...entry.children];
    while (body.length && body.at(-1).trim() === '') body.pop();
    const indent = Math.min(...body.filter((l) => l.trim()).map((l) => /^ */.exec(l)[0].length));
    return `${body.map((l) => l.slice(indent)).join('\n')}\n`;
  }

  /** Every job: its job-level keys and its steps (each step's keys, `with` flattened into `with.<key>`). */
  function readJobs() {
    const start = lines.indexOf('jobs:');
    assert.ok(start >= 0, `${WORKFLOW} has no jobs: mapping`);
    const body = lines.slice(start + 1);
    const jobs = new Map();
    for (const [name, entry] of keysAt(body, 2)) {
      const keys = keysAt(entry.children, 4);
      const steps = [];
      for (const line of keys.get('steps')?.children ?? []) {
        if (line.startsWith('      - ')) steps.push([`        ${line.slice(8)}`]);
        else if (steps.length) steps.at(-1).push(line);
      }
      jobs.set(name, {
        keys,
        steps: steps.map((stepLines) => {
          const step = {};
          for (const [key, value] of keysAt(stepLines, 8)) step[key] = scalar(value);
          for (const [key, value] of keysAt(keysAt(stepLines, 8).get('with')?.children ?? [], 10)) step[`with.${key}`] = scalar(value);
          return step;
        }),
      });
    }
    return jobs;
  }

  const jobs = readJobs();
  const job = (name) => {
    assert.ok(jobs.has(name), `${WORKFLOW} has no job ${name}`);
    return jobs.get(name);
  };
  const conditionOf = (name) => scalar(job(name).keys.get('if'));
  const runs = (name) => job(name).steps.map((s) => s.run ?? '').join('\n');
  const UBIQX_CONF = PRODUCTS.ubiqx.tauriConf;
  const SHIELD_CONF = PRODUCTS.shield.tauriConf;
  const SHIELD_DIR = path.posix.dirname(SHIELD_CONF);
  const APP_JOBS = { macos: 'macos-app', windows: 'windows-app', linux: 'linux-app' };

  test('the reader sees the jobs the release depends on', () => {
    for (const name of ['rust-linux', 'frontend', ...Object.values(APP_JOBS), 'publish-continuous', 'publish-shield']) job(name);
    assert.ok(job('macos-app').steps.length > 10, 'the steps of a job are read');
  });

  test('only a push to main publishes ubiqX and the standalone app, and NOFAP_PUBLISH=false stops the standalone app', () => {
    assert.equal(conditionOf('publish-continuous'), "github.event_name == 'push' && github.ref == 'refs/heads/main'");
    const shield = conditionOf('publish-shield');
    assert.doesNotMatch(shield, /\|\||!(?!=)|\bcontains\(|\bstartsWith\(/, 'every part of the condition is required');
    assert.deepEqual(
      shield.split('&&').map((part) => part.trim()).sort(),
      ["github.event_name == 'push'", "github.ref == 'refs/heads/main'", "vars.NOFAP_PUBLISH != 'false'"],
    );
    for (const name of ['publish-continuous', 'publish-shield']) {
      assert.deepEqual(scalar(job(name).keys.get('needs')), '[macos-app, windows-app, linux-app]', `${name} waits for the three platforms`);
    }
  });

  test('each publish job takes only its own product\'s artefacts and files, under its own product', () => {
    const download = (name) => job(name).steps.find((s) => s.uses?.startsWith('actions/download-artifact'));
    assert.equal(download('publish-continuous')['with.pattern'], `${PRODUCTS.ubiqx.assetPrefix}-*-app`);
    assert.equal(download('publish-shield')['with.pattern'], `${PRODUCTS.shield.assetPrefix}-*-app`);

    const ubiqx = runs('publish-continuous');
    assert.match(ubiqx, new RegExp(`find dist -type f -name '${PRODUCTS.ubiqx.assetPrefix}-\\*' ! -name '\\*\\.sig'`));
    assert.match(ubiqx, /node scripts\/publish-release\.mjs --strict /);
    assert.doesNotMatch(ubiqx, /--product|--tag/, 'ubiqX publishes with the defaults it always had');
    assert.match(ubiqx, /bash scripts\/ci-app-version\.sh\n/);

    const shield = runs('publish-shield');
    assert.match(shield, new RegExp(`find dist -type f -name '${PRODUCTS.shield.assetPrefix}-\\*' ! -name '\\*\\.sig'`));
    assert.match(shield, /node scripts\/publish-release\.mjs --product shield --strict /);
    assert.doesNotMatch(shield, /--tag/, 'the tag comes from the product table');
    assert.match(shield, /bash scripts\/ci-app-version\.sh --product shield\n/);

    // The download pattern of each job, read the way a case-insensitive matcher would, still misses the
    // other product's artefacts: the case of `ubiqX` is not what keeps them apart.
    const artefacts = Object.keys(APP_JOBS).flatMap((os) => [`${PRODUCTS.ubiqx.assetPrefix}-${os}-app`, `${PRODUCTS.shield.assetPrefix}-${os}-app`]);
    const caseless = (pattern) => new RegExp(`^${pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*')}$`, 'i');
    assert.deepEqual(artefacts.filter((a) => caseless(download('publish-shield')['with.pattern']).test(a)), artefacts.filter((a) => a.startsWith(`${PRODUCTS.shield.assetPrefix}-`)));
  });

  test('no update feed variable is set where both shells build', () => {
    const feedVars = /^(UBIQX_UPDATE_FEED_URL|UBIQX_SHIELD_UPDATE_FEED_URL)$/;
    const topEnv = keysAt(lines, 0).get('env');
    for (const key of keysAt(topEnv?.children ?? [], 2).keys()) assert.doesNotMatch(key, feedVars, `workflow-level env sets ${key}`);
    for (const [name, { keys }] of jobs) {
      for (const key of keysAt(keys.get('env')?.children ?? [], 6).keys()) assert.doesNotMatch(key, feedVars, `job ${name} sets ${key}`);
    }
  });

  test('every app job builds the standalone app from its own shell, after ubiqX is uploaded, and Windows ships it without an MSI', () => {
    for (const [os, name] of Object.entries(APP_JOBS)) {
      const { steps } = job(name);
      const build = steps.findIndex((s) => s['working-directory'] === SHIELD_DIR && /tauri\.js build /.test(s.run ?? ''));
      assert.ok(build >= 0, `${name} builds the standalone app in ${SHIELD_DIR}`);
      const ubiqxUpload = steps.findIndex((s) => s['with.name'] === `${PRODUCTS.ubiqx.assetPrefix}-${os}-app`);
      const shieldUpload = steps.findIndex((s) => s['with.name'] === `${PRODUCTS.shield.assetPrefix}-${os}-app`);
      assert.ok(ubiqxUpload >= 0 && ubiqxUpload < build && build < shieldUpload, `${name}: ubiqX uploaded, then the standalone app built and uploaded`);
      const stamps = steps.map((s) => s.run ?? '').join('\n');
      assert.match(stamps, /bash scripts\/ci-app-version\.sh --product shield\n/, `${name} stamps the standalone app's version`);
      const bundles = /--bundles (\S+)/.exec(steps[build].run)?.[1];
      if (os === 'windows') assert.equal(bundles, 'nsis', 'the standalone app has no MSI on Windows (decision 39)');
      else assert.ok(bundles, `${name} names the bundles of the standalone app`);
    }
    const windows = runs('windows-app');
    assert.match(windows, /cargo clippy -p ubiqx-nofap-desktop --all-targets -- -D warnings/);
    assert.match(windows, /cargo test -p ubiqx-nofap-desktop\n/);
    assert.match(runs('frontend'), /pnpm build:nofap/);
    for (const command of ['cargo clippy --workspace', 'cargo test --workspace']) {
      const line = runs('rust-linux').split('\n').find((l) => l.startsWith(command));
      assert.match(line, /--exclude ubiqx-desktop --exclude ubiqx-nofap-desktop/, `rust-linux: ${command} leaves both shells out`);
    }
  });

  // The rename steps run for real, with bash, against a bundle tree holding both products' outputs under the
  // names Tauri gives them (from each productName), then the upload globs are matched against what is left.
  // Only the tools that do not exist everywhere (ditto, dpkg-deb) are stand-ins.
  describe('the rename steps and the upload globs, run against both products\' bundles', { skip: process.platform === 'win32' && 'bash steps' }, () => {
    const NAMES = {
      ubiqx: JSON.parse(readRepoFile(UBIQX_CONF)).productName,
      shield: JSON.parse(readRepoFile(SHIELD_CONF)).productName,
    };
    const BINARIES = { ubiqx: 'ubiqx-desktop', shield: 'ubiqx-shield' };
    const ARCH = { macos: 'aarch64', windows: 'x86_64', linux: 'x86_64' };

    /** What `tauri build` leaves for one product, by OS (paths under target/release). */
    function tauriOutputs(os, product) {
      const n = NAMES[product];
      switch (os) {
        case 'macos':
          return [`bundle/macos/${n}.app/Contents/MacOS/${BINARIES[product]}`, `bundle/macos/${n}.app.tar.gz`, `bundle/macos/${n}.app.tar.gz.sig`, `bundle/dmg/${n}_0.1.7_aarch64.dmg`];
        case 'windows':
          return [
            `bundle/msi/${n}_0.1.7_x64_pt-BR.msi`, `bundle/msi/${n}_0.1.7_x64_en-US.msi`,
            `bundle/nsis/${n}_0.1.7_x64-setup.exe`, `bundle/nsis/${n}_0.1.7_x64-setup.nsis.zip`, `bundle/nsis/${n}_0.1.7_x64-setup.nsis.zip.sig`,
          ];
        default:
          return [`bundle/appimage/${n}_0.1.7_amd64.AppImage`, `bundle/appimage/${n}_0.1.7_amd64.AppImage.tar.gz`, `bundle/appimage/${n}_0.1.7_amd64.AppImage.tar.gz.sig`, `bundle/deb/${n}_0.1.7_amd64.deb`];
      }
    }

    /** The stable names each product's upload must hold, signed or not. */
    function expected(os, product, signed) {
      const p = `${PRODUCTS[product].assetPrefix}-${os}-${ARCH[os]}`;
      switch (os) {
        case 'macos':
          return [`bundle/dmg/${p}.dmg`, `bundle/macos/${p}.app.zip`, ...(signed ? [`bundle/macos/${p}.app.tar.gz`, `bundle/macos/${p}.app.tar.gz.sig`] : [])];
        case 'windows':
          return [
            ...(product === 'ubiqx' ? [`bundle/msi/${p}.msi`] : []),
            `bundle/nsis/${p}-setup.exe`,
            ...(signed ? [`bundle/nsis/${p}-setup.nsis.zip`, `bundle/nsis/${p}-setup.nsis.zip.sig`] : []),
          ];
        default:
          return [`bundle/appimage/${p}.AppImage`, ...(signed ? [`bundle/appimage/${p}.AppImage.tar.gz`, `bundle/appimage/${p}.AppImage.tar.gz.sig`] : []), `bundle/deb/${p}.deb`];
      }
    }

    function filesUnder(dir, base = dir) {
      if (!existsSync(dir)) return [];
      return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
        const full = path.join(dir, entry.name);
        return entry.isDirectory() ? filesUnder(full, base) : [path.relative(base, full).split(path.sep).join('/')];
      });
    }

    /** upload-artifact's glob, for the `*` the workflow uses; case-insensitive on Windows, as it is there. */
    function globMatcher(pattern, os) {
      const source = pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '[^/]*');
      return new RegExp(`^${source}$`, os === 'windows' ? 'i' : '');
    }

    function renameStep(name, conf) {
      const found = job(name).steps.filter((s) => (s.run ?? '').includes(`require('./${conf}')`) && (s.run ?? '').includes('mv '));
      assert.equal(found.length, 1, `${name}: one rename step reads ${conf}`);
      return found[0].run;
    }

    function runStep(root, os, script, signed) {
      const file = path.join(root, 'zz-step.sh');
      writeFileSync(file, script);
      // The Windows job runs bash with pipefail (its defaults.run.shell); the others take GitHub's default.
      const flags = os === 'windows' ? ['--noprofile', '--norc', '-e', '-o', 'pipefail'] : ['-e'];
      const result = spawnSync('bash', [...flags, file], {
        cwd: root,
        encoding: 'utf8',
        env: {
          PATH: [path.join(root, 'zz-bin'), path.dirname(process.execPath), process.env.PATH].join(path.delimiter),
          UBIQX_ARCH: ARCH[os],
          UBIQX_UPDATER_SIGNED: signed ? '1' : '0',
        },
      });
      rmSync(file);
      assert.equal(result.status, 0, `${os} step failed:\n${result.stdout}\n${result.stderr}`);
    }

    function scratch(os) {
      const root = mkdtempSync(path.join(tmpdir(), 'ubiqx-ci-steps-'));
      for (const conf of [UBIQX_CONF, SHIELD_CONF]) {
        mkdirSync(path.join(root, path.dirname(conf)), { recursive: true });
        copyFileSync(path.join(REPO_ROOT, conf), path.join(root, conf));
      }
      const bin = path.join(root, 'zz-bin');
      mkdirSync(bin);
      // ditto writes the zip named by its last argument. dpkg-deb answers from zz-debroot/<deb name>, the
      // files that .deb would install: --contents lists them as dpkg-deb does, --fsys-tarfile tars them.
      writeFileSync(path.join(bin, 'ditto'), '#!/bin/sh\nfor last; do :; done\n: > "$last"\n');
      writeFileSync(
        path.join(bin, 'dpkg-deb'),
        [
          '#!/bin/sh',
          'for a; do case "$a" in *.deb) deb="$a" ;; esac; done',
          'tree="$(dirname "$0")/../zz-debroot/$(basename "$deb")"',
          'case "$1" in',
          '  --field) echo "Package: stand-in" ;;',
          '  --contents) (cd "$tree" && find . -type f | sort | sed "s|^|-rw-r--r-- root/root 0 2026-01-01 00:00 |") ;;',
          '  --fsys-tarfile) tar -cf - -C "$tree" . ;;',
          '  *) exit 2 ;;',
          'esac',
          '',
        ].join('\n'),
      );
      for (const tool of ['ditto', 'dpkg-deb']) chmodSync(path.join(bin, tool), 0o755);
      const release = path.join(root, 'target/release');
      const seed = (relative, content = relative) => {
        mkdirSync(path.dirname(path.join(release, relative)), { recursive: true });
        writeFileSync(path.join(release, relative), content);
      };
      for (const product of ['ubiqx', 'shield']) for (const f of tauriOutputs(os, product)) seed(f);
      if (os === 'windows') seed('nsis/x64/installer.nsi', `!define MAINBINARYNAME "${BINARIES.ubiqx}"\n`);
      if (os === 'linux') for (const product of ['ubiqx', 'shield']) debTree(root, product);
      return { root, release, seed };
    }

    /** What each product's .deb installs: its executable and the polkit action its tauri.conf.json ships. */
    function debTree(root, product, policy = (text) => text) {
      const deb = tauriOutputs('linux', product).find((f) => f.endsWith('.deb'));
      const tree = path.join(root, 'zz-debroot', path.posix.basename(deb));
      const conf = PRODUCTS[product].tauriConf;
      const files = JSON.parse(readRepoFile(conf)).bundle.linux.deb.files;
      rmSync(tree, { recursive: true, force: true });
      mkdirSync(path.join(tree, 'usr/bin'), { recursive: true });
      writeFileSync(path.join(tree, 'usr/bin', BINARIES[product]), '');
      for (const [installed, source] of Object.entries(files)) {
        const target = path.join(tree, installed.replace(/^\//, ''));
        mkdirSync(path.dirname(target), { recursive: true });
        writeFileSync(target, policy(readRepoFile(path.posix.join(path.posix.dirname(conf), source))));
      }
      return tree;
    }

    // The step extracts the action with GNU tar's --wildcards, as the Ubuntu runner has it.
    const gnuTar = /GNU tar/.test(spawnSync('tar', ['--version'], { encoding: 'utf8' }).stdout ?? '');

    for (const product of ['ubiqx', 'shield']) {
      test(`linux: the ${product} step fails when its .deb lacks its polkit action or the action names another executable`, { skip: !gnuTar && 'needs GNU tar' }, () => {
        const conf = PRODUCTS[product].tauriConf;
        const breakages = [
          (tree) => rmSync(path.join(tree, 'usr/share/polkit-1'), { recursive: true, force: true }),
          (tree) => debTree(path.dirname(path.dirname(tree)), product, (text) => text.replaceAll(`/usr/bin/${BINARIES[product]}<`, '/usr/bin/someone-else<')),
        ];
        for (const breakage of breakages) {
          const { root } = scratch('linux');
          try {
            const deb = tauriOutputs('linux', product).find((f) => f.endsWith('.deb'));
            breakage(path.join(root, 'zz-debroot', path.posix.basename(deb)));
            const file = path.join(root, 'zz-step.sh');
            writeFileSync(file, renameStep(APP_JOBS.linux, conf));
            const result = spawnSync('bash', ['-e', file], {
              cwd: root,
              encoding: 'utf8',
              env: { PATH: [path.join(root, 'zz-bin'), path.dirname(process.execPath), process.env.PATH].join(path.delimiter), UBIQX_ARCH: 'x86_64', UBIQX_UPDATER_SIGNED: '0' },
            });
            assert.notEqual(result.status, 0, `${product}: a broken .deb must fail the step`);
            assert.ok(existsSync(path.join(root, 'target/release', deb)), 'the .deb is not renamed after the check failed');
          } finally {
            rmSync(root, { recursive: true, force: true });
          }
        }
      });
    }

    for (const os of Object.keys(APP_JOBS)) {
      for (const signed of [true, false]) {
        test(`${os}, ${signed ? 'signed' : 'unsigned'}: each step renames only its own product's files, and each upload takes only them`, { skip: os === 'linux' && !gnuTar && 'needs GNU tar' }, () => {
          const name = APP_JOBS[os];
          const { root, release } = scratch(os);
          try {
            const bundle = path.join(release, 'bundle');
            const shieldBefore = tauriOutputs(os, 'shield').map((f) => [f, readFileSync(path.join(release, f), 'utf8')]);

            // ubiqX first, as in the job, but with the standalone app's files already there: its globs must
            // name ubiqX, not take the first .dmg, .msi or .deb they find.
            runStep(root, os, renameStep(name, UBIQX_CONF), signed);
            for (const [f, content] of shieldBefore) assert.equal(readFileSync(path.join(release, f), 'utf8'), content, `ubiqX's step left ${f} alone`);
            runStep(root, os, renameStep(name, SHIELD_CONF), signed);

            const left = filesUnder(bundle).map((f) => `bundle/${f}`);
            for (const product of ['ubiqx', 'shield']) {
              const upload = job(name).steps.find((s) => s['with.name'] === `${PRODUCTS[product].assetPrefix}-${os}-app`);
              assert.equal(upload['with.if-no-files-found'], 'error');
              const globs = upload['with.path'].trim().split('\n').map((g) => globMatcher(g.trim().replace(/^target\/release\//, ''), os));
              const taken = left.filter((f) => globs.some((g) => g.test(f))).sort();
              assert.deepEqual(taken, expected(os, product, signed).sort(), `${upload['with.name']} uploads exactly its own files`);
              for (const f of taken) assert.ok(path.posix.basename(f).startsWith(`${PRODUCTS[product].assetPrefix}-`), f);
              if (product === 'shield') assert.ok(!taken.some((f) => f.endsWith('.msi')), 'no MSI of the standalone app');
              // Each renamed file is the one Tauri built for that product, not the other product's.
              for (const f of taken.filter((t) => !t.endsWith('.app.zip'))) {
                assert.match(readFileSync(path.join(release, f), 'utf8'), new RegExp(`^bundle/[a-z]+/${NAMES[product]}[._]`), `${f} holds ${NAMES[product]}'s output`);
              }
            }
            if (!signed) {
              const updater = left.filter((f) => /\.(tar\.gz|nsis\.zip)(\.sig)?$/.test(f));
              assert.deepEqual(updater, [], 'an unsigned build leaves no updater artefact of either product');
            }
          } finally {
            rmSync(root, { recursive: true, force: true });
          }
        });
      }
    }
  });
});
