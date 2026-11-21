import { describe, expect, it } from 'vitest';
import viteConfig from '../../vite.config.ts?raw';
import standaloneConf from '../../../nofap/src-tauri/tauri.conf.json?raw';
import ubiqxConf from '../../src-tauri/tauri.conf.json?raw';

// Every source file of the project, as text: the walk below reads the imports the bundler would follow.
const SOURCES = import.meta.glob<string>('/src/**/*.{ts,tsx}', { query: '?raw', import: 'default', eager: true });

/** The substitution `vite.config.ts` makes in the standalone build (ubiqX's mock → the standalone's). */
const REDIRECT: Record<string, string> = { '/src/lib/mock.ts': '/src/nofap-app/hostMock.ts' };

const SPECIFIERS = [/\b(?:import|export)\s[^'";]*?\bfrom\s*['"]([^'"]+)['"]/g, /\bimport\s*['"]([^'"]+)['"]/g, /\bimport\(\s*['"]([^'"]+)['"]\s*\)/g];

function join(dir: string, relative: string): string {
  const parts = dir.split('/');
  for (const part of relative.split('/')) {
    if (part === '..') parts.pop();
    else if (part !== '.') parts.push(part);
  }
  return parts.join('/');
}

/** The source file an import names, or `null` for a package, an asset or a file outside `src/`. */
function resolve(importer: string, specifier: string): string | null {
  if (!specifier.startsWith('.')) return null;
  const base = join(importer.slice(0, importer.lastIndexOf('/')), specifier.split('?')[0]!);
  const found = [base, `${base}.ts`, `${base}.tsx`, `${base}/index.ts`, `${base}/index.tsx`].find((candidate) => candidate in SOURCES);
  return found ? (REDIRECT[found] ?? found) : null;
}

/** Every source file reachable from `entry`, static and dynamic imports alike (type imports too, to be strict). */
function graph(entry: string): Set<string> {
  const seen = new Set<string>();
  const queue = [entry];
  while (queue.length) {
    const file = queue.pop()!;
    if (seen.has(file)) continue;
    seen.add(file);
    const text = SOURCES[file] ?? '';
    for (const pattern of SPECIFIERS) {
      for (const match of text.matchAll(pattern)) {
        const next = resolve(file, match[1]!);
        if (next && !seen.has(next)) queue.push(next);
      }
    }
  }
  return seen;
}

const TRACKER_PAGES = ['Dashboard', 'Timeline', 'Review', 'Reports', 'Categories', 'Insights', 'Focus', 'Onboarding', 'Settings', 'Intervention'];

describe('the standalone module graph (design 8.4)', () => {
  const standalone = graph('/src/nofap-app/main.tsx');

  it('reaches the module and the standalone frame', () => {
    for (const file of ['/src/nofap/NofapPage.tsx', '/src/nofap/NofapWizard.tsx', '/src/nofap/NofapCard.tsx', '/src/nofap-app/Welcome.tsx', '/src/nofap-app/NofapSettings.tsx', '/src/nofap-app/hostMock.ts']) {
      expect(standalone, file).toContain(file);
    }
  });

  it('contains no page of ubiqX, nothing of its frame, store or mock', () => {
    const tracker = [...standalone].filter(
      (file) =>
        file.startsWith('/src/pages/') ||
        file.startsWith('/src/components/layout/') ||
        file.startsWith('/src/components/focus/') ||
        ['/src/App.tsx', '/src/main.tsx', '/src/lib/store.ts', '/src/lib/engine.ts', '/src/lib/mock.ts', '/src/nofap/ubiqx.tsx'].includes(file),
    );
    expect(tracker).toEqual([]);
  });

  it('is a walk that can fail: ubiqX\'s own entry reaches every page of the tracker', () => {
    const ubiqx = graph('/src/main.tsx');
    for (const page of TRACKER_PAGES) expect(ubiqx, page).toContain(`/src/pages/${page}.tsx`);
    expect(ubiqx).toContain('/src/lib/store.ts');
  });

  it('applies the same substitution as the standalone build', () => {
    for (const [from, to] of Object.entries(REDIRECT)) expect(viteConfig).toContain(`{ from: '${from}', to: '${to}' }`);
  });
});

describe('the standalone shell configuration', () => {
  const standalone = JSON.parse(standaloneConf);
  const ubiqx = JSON.parse(ubiqxConf);

  it('keeps the webview off the network, with the same policy as ubiqX', () => {
    expect(standalone.app.security.csp).toBe(ubiqx.app.security.csp);
  });

  it('builds this entry and checks its own update channel', () => {
    expect(standalone.build.beforeBuildCommand).toContain('build:nofap');
    expect(standalone.build.frontendDist).toBe('../../desktop/dist-nofap');
    expect(standalone.plugins.updater.endpoints).toEqual(['https://github.com/andreyquadros/ubiquitous-engine/releases/download/shield-continuous/updater.json']);
    expect(standalone.plugins.updater.endpoints).not.toEqual(ubiqx.plugins.updater.endpoints);
  });
});
