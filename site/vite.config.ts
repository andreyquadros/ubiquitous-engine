import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vite';

/**
 * SITE_BASE is the public path the site is served from: "/" for a custom domain (ubiqx.ai),
 * "/ubiquitous-engine/" on GitHub Pages under the repository name. SITE_URL is the absolute origin
 * (plus base) used for the Open Graph image and the canonical link.
 */
const base = normaliseBase(process.env.SITE_BASE ?? '/');
const siteUrl = (process.env.SITE_URL ?? 'https://andreyquadros.github.io').replace(/\/+$/, '') + base;

function normaliseBase(b: string): string {
  let s = b.trim();
  if (!s.startsWith('/')) s = `/${s}`;
  if (!s.endsWith('/')) s = `${s}/`;
  return s;
}

/** Replaces %SITE_URL% in index.html so absolute meta tags follow the deployment. */
function siteUrlPlugin(): Plugin {
  return {
    name: 'ubiqx-site-url',
    transformIndexHtml(html) {
      return html.replaceAll('%SITE_URL%', siteUrl);
    },
  };
}

export default defineConfig({
  base,
  plugins: [react(), tailwindcss(), siteUrlPlugin()],
  define: {
    __SITE_URL__: JSON.stringify(siteUrl),
  },
  server: { port: 5173, strictPort: false },
  preview: { port: 4173, strictPort: false },
  build: {
    target: 'es2022',
    sourcemap: false,
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        manualChunks: (id: string) => {
          // three only enters through the lazy import of components/UbiHero3d, so it gets its own chunk.
          if (/node_modules\/three\//.test(id)) return 'three';
          return undefined;
        },
      },
    },
  },
});
