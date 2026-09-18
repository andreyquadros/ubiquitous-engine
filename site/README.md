# ubiqX AI, the site

The landing page that presents ubiqX AI as a product: hero with the 3D UBI, how it works, features, real
screenshots, plans, downloads, FAQ. Vite 8 + React 19 + TypeScript + Tailwind 4, with `three` only in a lazy
chunk for the mascot. Portuguese (pt-BR) is the default language, English the second; every visible string lives
in `src/i18n.ts`, both languages side by side.

## Development

```bash
cd site
pnpm install
pnpm dev            # http://localhost:5173  (?lang=en switches the language; /og shows the Open Graph card)
pnpm typecheck      # tsc --noEmit
pnpm build          # dist/ (SITE_BASE=/ubiquitous-engine/ pnpm build for GitHub Pages under the repo name)
pnpm preview        # serves dist/ on http://localhost:4173
```

`predev` and `prebuild` run `scripts/sync-assets.mjs`, which copies `Ubi.glb` from `../apps/desktop/public/ubi/`
into `public/ubi/` (git-ignored here, so the 3 MB model is not duplicated in the repository). The Draco decoder
is three's own copy, bundled by Vite as hashed assets; nothing is fetched from a CDN.

### Refreshing the images

```bash
pnpm screenshots                  # gallery in public/shots: starts ../apps/desktop in mock mode, captures 1440x900, optimises
pnpm screenshots --url http://localhost:1420   # against a desktop dev server you already started
pnpm images                       # favicons from the app icon, public/ubi-hero.png (2x, transparent) and public/og.png (1200x630)
pnpm captures --url http://localhost:4173      # QA: full-page captures at 1440x900 and 390x844 in both languages, fails on console errors or horizontal overflow
```

The screenshots need `pnpm install` in `apps/desktop`. Playwright uses the Chromium from `PLAYWRIGHT_BROWSERS_PATH`
(or `CHROMIUM_PATH`) with software WebGL flags so the 3D mascot renders headless.

## Configuration (`src/config.ts`)

| Field | Meaning |
|---|---|
| `CHECKOUT_ANNUAL_URL`, `CHECKOUT_MONTHLY_URL` | Checkout links of the two plans. Empty: the button reads "Em breve" (disabled) with the hint "Pagamento em configuração". |
| `DOWNLOAD_URLS.macos / windows / linux` | Installer links of the download cards and the primary CTA. Empty: "Em breve". |
| `REPO_URL`, `RELEASES_URL`, `DOCS_URL`, `MACOS_GUIDE_URL` | Footer and download links. |
| `CONTACT_EMAIL` | Adds a "Contato" link to the footer when set. |
| `MACOS_FIX_COMMANDS` | The two Terminal commands shown for a download that Gatekeeper reports as damaged (docs/MACOS-TESTING.md, 3.1). |

Prices and every other text live in `src/i18n.ts` (`pricing.plans`).

Build-time variables: `SITE_BASE` (public path, default `/`) and `SITE_URL` (origin used for the canonical link and
the Open Graph image, default `https://andreyquadros.github.io` + `SITE_BASE`).

## Deployment (GitHub Pages)

`.github/workflows/site.yml` builds `site/` on every push to `main` that touches `site/**` (and on
*workflow_dispatch*), uploads `site/dist` with `upload-pages-artifact` and publishes it with `deploy-pages` in the
`github-pages` environment.

1. In the repository: **Settings → Pages → Build and deployment → Source: GitHub Actions**.
2. Push to `main` (or run the workflow manually). The site appears at
   `https://<owner>.github.io/ubiquitous-engine/`; the workflow builds with `SITE_BASE=/ubiquitous-engine/` so
   every asset URL carries that prefix.
3. Custom domain (`ubiqx.ai`), later: add the domain in **Settings → Pages → Custom domain** (GitHub creates the
   `CNAME` file; keep it in `site/public/` so the build ships it), point the DNS at GitHub Pages, and change the
   build step to `SITE_BASE=/ SITE_URL=https://ubiqx.ai pnpm build`, since the site then lives at the root.

## Layout of `src/`

- `i18n.ts`: the dictionary (pt-BR, en), the locale resolution (`?lang=` > localStorage > browser language) and `useCopy()`.
- `config.ts`: links and switches described above.
- `components/UbiHero.tsx`: PNG first, then the lazy `UbiHero3d.tsx` (GLTFLoader + Draco, RoomEnvironment, slow turn, pointer parallax); reduced motion or no WebGL keeps the PNG.
- `sections/`: Nav, Hero, HowItWorks, Features, Screens (gallery with light-box), Pricing, Downloads, Faq, Footer, plus `OgCard` (`/og`) and `HeroCapture` (`?capture=hero`) used by the image scripts.
