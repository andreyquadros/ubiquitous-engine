// Tests for scripts/publish-release.mjs against an in-process mock of the GitHub API.
// Run with: node --test scripts/publish-release.test.mjs
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { after, before, beforeEach, describe, test } from 'node:test';

import { main, makeFeed, parseArgs, parseRepo, trimNotes, resolveGlob } from './publish-release.mjs';

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
    seedRelease({ feedEpoch, assets = ['ubiqX-macos-aarch64.dmg', 'ubiqX-macos-aarch64.app.zip'] }) {
      const r = { id: state.nextId++, tag_name: 'continuous', name: 'old', body: 'old notes', target_commitish: 'a'.repeat(40), assets: [] };
      for (const name of assets) r.assets.push({ id: state.nextId++, name, content: Buffer.from('old'), contentType: 'application/octet-stream' });
      if (feedEpoch !== undefined) {
        const feed = { schema: 1, product: 'ubiqX', version: '0.1.0', build: { epoch: feedEpoch, number: 1, sha: 'aaaaaaa', branch: 'main' }, platforms: {} };
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
  let out;
  let lines;
  const log = (line) => lines.push(String(line));

  before(async () => {
    api = await gh.listen();
    work = mkdtempSync(path.join(tmpdir(), 'ubiqx-publish-'));
    dmg = path.join(work, 'ubiqX_0.1.0_aarch64.dmg');
    appZip = path.join(work, 'ubiqX-macos-aarch64.app.zip');
    out = path.join(work, 'out');
    writeFileSync(dmg, Buffer.alloc(2048, 1));
    writeFileSync(appZip, Buffer.alloc(512, 2));
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
    assert.equal(o.dmg, 'target/release/bundle/dmg/*.dmg');
    assert.equal(o.uploads, 'https://uploads.github.com');
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
    const feed = makeFeed({ owner: 'o', repo: 'r', tag: 'continuous', version: '0.2.0', build: { epoch: 1, number: 2, sha: 'abc1234', branch: 'main' }, arch: 'x86_64', dmgSize: 10, hasAppZip: false, publishedAt: '2026-01-01T00:00:00Z', notes: 'n' });
    assert.equal(feed.platforms['darwin-x86_64'].url, 'https://github.com/o/r/releases/download/continuous/ubiqX-macos-x86_64.dmg');
    assert.equal(feed.platforms['darwin-x86_64'].app_zip_url, null);
    assert.equal(feed.release_url, 'https://github.com/o/r/releases/tag/continuous');
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
