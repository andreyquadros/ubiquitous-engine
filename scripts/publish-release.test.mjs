// Tests for scripts/publish-release.mjs against an in-process mock of the GitHub API.
// Run with: node --test scripts/publish-release.test.mjs
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { after, before, beforeEach, describe, test } from 'node:test';

import {
  buildPlatforms,
  buildUpdaterPlatforms,
  compareFeeds,
  inferAsset,
  main,
  makeFeed,
  makeUpdaterFeed,
  mergeFeeds,
  mergeUpdaterFeeds,
  parseArgs,
  parseRepo,
  productVersion,
  trimNotes,
  resolveGlob,
  UPDATER_ASSET_NAME,
} from './publish-release.mjs';
import { appVersion, baseVersion, buildNumber, main as appVersionMain, writeAppVersion } from './app-version.mjs';

const OWNER = 'andreyquadros';
const REPO = 'ubiquitous-engine';

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
    seedRelease({ feedEpoch, assets = ['ubiqX-macos-aarch64.dmg', 'ubiqX-macos-aarch64.app.zip'], platforms = {} }) {
      const r = { id: state.nextId++, tag_name: 'continuous', name: 'old', body: 'old notes', target_commitish: 'a'.repeat(40), assets: [] };
      for (const name of assets) r.assets.push({ id: state.nextId++, name, content: Buffer.from('old'), contentType: 'application/octet-stream' });
      if (feedEpoch !== undefined) {
        const feed = { schema: 1, product: 'ubiqX', version: '0.1.0', build: { epoch: feedEpoch, number: 1, sha: 'aaaaaaa', branch: 'main' }, platforms };
        r.assets.push({ id: state.nextId++, name: 'latest.json', content: Buffer.from(JSON.stringify(feed)), contentType: 'application/json' });
      }
      state.releases.push(r);
      state.refs['refs/tags/continuous'] = r.target_commitish;
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
    assert.ok(!JSON.stringify(feed).includes('tar.gz'), 'latest.json never mentions an updater artifact');
    assert.ok(!JSON.stringify(feed).includes('nsis.zip'));
    assert.ok(!JSON.stringify(feed).includes('signature'));

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
