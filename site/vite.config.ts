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

/**
 * Emits robots.txt and sitemap.xml pointing at the deployment's own origin, so they never drift from
 * SITE_URL. llms.txt is a hand-written file in public/ (it is content, not plumbing).
 */
function crawlerFilesPlugin(): Plugin {
  const today = new Date().toISOString().slice(0, 10);
  return {
    name: 'ubiqx-crawler-files',
    apply: 'build',
    generateBundle() {
      this.emitFile({
        type: 'asset',
        fileName: 'robots.txt',
        source: [
          'User-agent: *',
          'Allow: /',
          '',
          '# AI crawlers and answer engines are welcome: see llms.txt for a plain-text summary.',
          `Sitemap: ${siteUrl}sitemap.xml`,
          '',
        ].join('\n'),
      });
      this.emitFile({
        type: 'asset',
        fileName: 'sitemap.xml',
        source: [
          '<?xml version="1.0" encoding="UTF-8"?>',
          '<urlset xmlns="http://www.sitemap.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">'.replace(
            'www.sitemap.org',
            'www.sitemaps.org',
          ),
          '  <url>',
          `    <loc>${siteUrl}</loc>`,
          `    <lastmod>${today}</lastmod>`,
          '    <changefreq>weekly</changefreq>',
          '    <priority>1.0</priority>',
          `    <xhtml:link rel="alternate" hreflang="pt-BR" href="${siteUrl}"/>`,
          `    <xhtml:link rel="alternate" hreflang="en" href="${siteUrl}?lang=en"/>`,
          `    <xhtml:link rel="alternate" hreflang="x-default" href="${siteUrl}"/>`,
          '  </url>',
          '</urlset>',
          '',
        ].join('\n'),
      });
    },
  };
}

export default defineConfig({
  base,
  plugins: [react(), tailwindcss(), siteUrlPlugin(), crawlerFilesPlugin()],
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
