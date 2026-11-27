/// <reference types="vitest/config" />
import { defineConfig, searchForWorkspaceRoot, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
// @ts-expect-error type error without @types/node package
import process from 'node:process';

const host = process.env.TAURI_DEV_HOST as string | undefined;

/**
 * The standalone product (`apps/nofap`, design 8.1) is a second entry of this project: `vite build --mode nofap`
 * (`pnpm build:nofap`) builds `index.nofap.html` → `src/nofap-app/main.tsx` into `dist-nofap`, and
 * `vite --mode nofap` (`pnpm dev:nofap`) serves it on its own port. A mode rather than an environment variable, so
 * the same script runs in every shell, Windows' included.
 */
const NOFAP_MODE = 'nofap';
const NOFAP_ENTRY = 'index.nofap.html';
/**
 * In the standalone build every import of ubiqX's mock (`src/lib/mock.ts`, which the module's `ipc.ts` reaches
 * outside Tauri) resolves to the standalone's own (`src/nofap-app/hostMock.ts`), so nothing of ubiqX's tracker mock
 * enters `dist-nofap`. `src/nofap-app/graph.test.ts` applies the same rule when it walks the module graph.
 */
const NOFAP_MOCK_REDIRECT = { from: '/src/lib/mock.ts', to: '/src/nofap-app/hostMock.ts' } as const;

/** Resolves ubiqX's mock to the standalone's (before Vite's own resolver, which would otherwise answer first). */
function nofapMock(): Plugin {
  return {
    name: 'ubiqx-nofap-mock',
    enforce: 'pre',
    async resolveId(source, importer) {
      if (!importer || !/(^|\/)mock(\.ts)?$/.test(source)) return null;
      const resolved = await this.resolve(source, importer, { skipSelf: true });
      if (!resolved || !resolved.id.replace(/\\/g, '/').endsWith(NOFAP_MOCK_REDIRECT.from)) return null;
      return this.resolve(NOFAP_MOCK_REDIRECT.to, importer, { skipSelf: true });
    },
  };
}

/**
 * The shell loads `index.html` (its window has no `url`, which would put the source name into the shell's
 * configuration): the built page takes that name, and the dev server answers `/` with the standalone entry. After
 * Vite's own HTML plugin, which emits the page.
 */
function nofapPage(): Plugin {
  return {
    name: 'ubiqx-nofap-page',
    enforce: 'post',
    configureServer(server) {
      server.middlewares.use((req, _res, next) => {
        const [path, query] = (req.url ?? '/').split('?', 2);
        if (path === '/' || path === '/index.html') req.url = `/${NOFAP_ENTRY}${query === undefined ? '' : `?${query}`}`;
        next();
      });
    },
    generateBundle(_options, bundle) {
      const page = bundle[NOFAP_ENTRY];
      if (!page || page.type !== 'asset') return;
      delete bundle[NOFAP_ENTRY];
      this.emitFile({ type: 'asset', fileName: 'index.html', source: page.source });
    },
  };
}

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const nofap = mode === NOFAP_MODE;
  const port = nofap ? 1430 : 1420;
  return {
    plugins: [react(), tailwindcss(), ...(nofap ? [nofapMock(), nofapPage()] : [])],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: 'ws',
            host,
            port: port + 1,
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ['**/src-tauri/**'],
      },
      // The repository, not only this project: the standalone frontend reads files outside it as text (the
      // third-party notices of its about box; in a test, the standalone shell's configuration), and Vite refuses
      // `?raw` outside the allowed roots (a single file cannot be allowed: the check also sees the `?raw` suffix).
      fs: { allow: [searchForWorkspaceRoot(process.cwd()), '../..'] },
    },
    build: {
      target: 'es2022',
      chunkSizeWarningLimit: 1500,
      ...(nofap ? { outDir: 'dist-nofap' } : {}),
      rollupOptions: {
        ...(nofap ? { input: NOFAP_ENTRY } : {}),
        output: {
          manualChunks: (id: string) => {
            // three + @react-three/fiber are NOT a manual chunk on purpose: they are only reached through the lazy
            // import of components/ubi/Ubi3d, so the bundler splits them into that chunk by itself, and the modules
            // they share with the app (zustand, scheduler) stay in the entry instead of dragging three into it.
            if (/node_modules\/(recharts|d3-|victory-vendor)/.test(id)) return 'charts';
            return undefined;
          },
        },
      },
    },
    test: {
      environment: 'jsdom',
      setupFiles: ['./src/test/setup.ts'],
      include: ['src/**/*.test.tsx', 'src/**/*.test.ts'],
      css: false,
    },
  };
});
