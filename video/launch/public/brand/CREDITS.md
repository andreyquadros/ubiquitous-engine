# Brand and third-party assets: credits

Everything in `public/brand/` is produced by `tools/build-brand-assets.py`, apart from this file and the grain
tiles another step adds. The script pins every source (npm version plus the byte-identical file on GitHub at that
release's commit) and rebuilds byte-for-byte:

```
python3 tools/build-brand-assets.py            # needs npm, curl, fontTools+brotli, uharfbuzz, skia-pathops
```

`brand-manifest.json` lists every asset as `{file, kind, name, colored, license, source, note}`. Wordmark entries
also carry `metrics` (viewBox, baselineY, capHeight, xHeight, unitsPerEm).

## Usage rules for the film (from `brief/style.md` and `brief/facts.md`)

| Rule | Source |
|---|---|
| **No AI-provider logos on screen.** Show Claude, OpenAI and Grok as plain text only. No "powered by" and no "em parceria". The files in `ai/` are for reference or a future brief change only. | style.md S20; facts.md §5.18 |
| OS row: "macOS · Windows · Linux" as **monochrome** glyphs (`os/*.svg`, tinted via `color`) or as Inter 600 words. | style.md S20, S22 |
| Use `ubiqx-wordmark-bold.svg` (Sora 700, −0.03em) for the film. `ubiqx-wordmark.svg` (Sora 600, −0.02em) matches the app sidebar. | style.md §4.2; `apps/desktop/src/index.css` `.display` |
| **Do not show `og.png` on screen.** Its fine print says "Tudo fica no seu Mac", which is macOS-only and contradicts cloud AI. | facts.md §3.16, §5.3 |
| `social/x.svg` is the X social network logo. It is **not** the xAI or Grok mark. | – |
| The "IA do Ubi" provider has no separate logo. Use `ubiqx-mark.svg` (UBI's head). | `apps/desktop/src/lib/providers.ts` |

## Third-party logos

All logos are trademarks of their owners. They are used **nominatively**, only to say which services and systems
ubiqX works with. They imply no partnership, sponsorship or endorsement. The icon-set licenses below cover the
drawings (the SVG files). They grant no trademark rights.

| File | Source (version / commit) | License | Trademark |
|---|---|---|---|
| `ai/anthropic.svg` | npm `simple-icons@16.33.0` `icons/anthropic.svg` = https://github.com/simple-icons/simple-icons/blob/5828a6df55f0afded88fce0b94ce801903ddefe8/icons/anthropic.svg | CC0-1.0 (collection) | Anthropic, PBC |
| `ai/claude.svg` | same package, `icons/claude.svg` (same commit) | CC0-1.0 (collection) | Claude is a trademark of Anthropic, PBC |
| `ai/claude-color.svg` | npm `@lobehub/icons-static-svg@1.95.1` `icons/claude-color.svg` = https://github.com/lobehub/lobe-icons/blob/49a2130df7bfa5eb1b088261bff20a37e2967789/packages/static-svg/icons/claude-color.svg | MIT, © 2023 LobeHub | Anthropic, PBC |
| `ai/openai.svg` | `@lobehub/icons-static-svg@1.95.1` `icons/openai.svg` (same commit). simple-icons no longer ships OpenAI. | MIT, © 2023 LobeHub | OpenAI |
| `ai/xai.svg` | `@lobehub/icons-static-svg@1.95.1` `icons/xai.svg` (same commit). The upstream `<title>` reads "Grok", but the drawing is the xAI slashed-X mark. | MIT, © 2023 LobeHub | xAI (X.AI Corp.) |
| `ai/grok.svg` | `@lobehub/icons-static-svg@1.95.1` `icons/grok.svg` (same commit) | MIT, © 2023 LobeHub | Grok is a trademark of xAI |
| `os/apple.svg` | `simple-icons@16.33.0` `icons/apple.svg` (commit 5828a6d…) | CC0-1.0 (collection) | Apple and macOS are trademarks of Apple Inc. |
| `os/macos-wordmark.svg` | `simple-icons@16.33.0` `icons/macos.svg`. The viewBox is cropped to the ink; the drawing is unchanged. | CC0-1.0 (collection) | Apple Inc. |
| `os/windows.svg` | npm `bootstrap-icons@1.13.1` `icons/windows.svg` = https://github.com/twbs/icons/blob/ce0e49dd063243118a115f17ad1fe1fe7576d552/icons/windows.svg | MIT, © 2019–2024 The Bootstrap Authors | Windows is a trademark of Microsoft Corporation |
| `os/windows11.svg`, `os/windows11-color.svg` | npm `devicon@2.17.0` `icons/windows11/windows11-original.svg` = https://github.com/devicons/devicon/blob/54cfe13ac10eaa1ef817a343ab0a9437eb3c2e08/icons/windows11/windows11-original.svg. The mono file is recoloured to `currentColor`. | MIT, © 2015 konpa | Microsoft Corporation |
| `os/linux.svg` | `simple-icons@16.33.0` `icons/linux.svg` (commit 5828a6d…) | CC0-1.0 (collection) | Linux® is the registered trademark of Linus Torvalds |
| `os/linux-color.svg` | `devicon@2.17.0` `icons/linux/linux-original.svg` (commit 54cfe13…) | MIT, © 2015 konpa | Tux is based on the original by Larry Ewing (lewing@isc.tamu.edu), made with The GIMP |
| `social/x.svg` | `simple-icons@16.33.0` `icons/x.svg` (commit 5828a6d…) | CC0-1.0 (collection) | X Corp. |

Notes:
- simple-icons is CC0 as a collection, and none of the slugs used here carries its own license entry in
  `data/simple-icons.json`. Its `DISCLAIMER.md` (copied to `licenses/`) asks users to respect each brand's
  guidelines.
- simple-icons 16.33.0 has no `openai`, `xai`, `grok`, `windows` or `microsoft` icon, so those come from the MIT
  sets above.
- Upstream license texts: `licenses/simple-icons-LICENSE.md`, `licenses/simple-icons-DISCLAIMER.md`,
  `licenses/lobe-icons-LICENSE.txt`, `licenses/bootstrap-icons-LICENSE.txt`, `licenses/devicon-LICENSE.txt`.

**What normalisation changed:** root `width`/`height` are set to the viewBox size, and `style`, `class` and `role`
are removed. Each file gets `role="img"`, an `aria-label` and a `<title>`. Monochrome files set
`fill="currentColor"` on the root and drop the fills on child elements. Coloured files keep their original fills.
No path data was edited.

## Fonts (SIL Open Font License 1.1)

| File | Source | License |
|---|---|---|
| `fonts/sora-latin-wght-normal.woff2`, `fonts/sora-latin-ext-wght-normal.woff2` | npm `@fontsource-variable/sora@5.3.0` `files/` | OFL-1.1, © 2019 The Sora Project Authors (https://github.com/sora-xor/sora-font) |
| `fonts/inter-latin-wght-normal.woff2`, `fonts/inter-latin-ext-wght-normal.woff2` | npm `@fontsource-variable/inter@5.3.0` `files/` | OFL-1.1, © 2020 The Inter Project Authors (https://github.com/rsms/inter) |
| `fonts/InterVariable.woff2` (full glyph set, including → ✓) | https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/docs/font-files/InterVariable.woff2 (tag v4.1) | OFL-1.1 (see `fonts/OFL-Inter-rsms.txt`) |
| `fonts/SoraVariable.woff2` (full glyph set, 513 glyphs) | https://raw.githubusercontent.com/google/fonts/23e54b51ddffbc7713c583748e3bd86f62b1fa4a/ofl/sora/Sora%5Bwght%5D.ttf, re-wrapped as WOFF2 with fontTools (outlines and names unchanged) | OFL-1.1 |
| `fonts/OFL-Sora.txt`, `fonts/OFL-Inter.txt` | google/fonts at the commit above, `ofl/sora/OFL.txt` and `ofl/inter/OFL.txt` | – |
| `fonts/fonts.css` | written here: `@font-face` rules for HTML previews. Remotion scenes load the woff2 files with `@remotion/fonts`. | – |

The fontsource `latin` subset covers every PT-BR character (U+0000–00FF, plus punctuation in U+2000–206F). It does
**not** include → (U+2192) or ✓. Use `InterVariable.woff2` if a scene needs them. Sora has no arrows or check marks
in any file.

## ubiqX brand files (first-party, this repository, MIT)

| File | Made from |
|---|---|
| `ubiqx-mark.svg` | `BrandMark` in `apps/desktop/src/components/layout/Sidebar.tsx`, converted from JSX to a standalone SVG. The gradient id is renamed to `ubiqx-mark-visor` so it can't collide when inlined. |
| `ubiqx-mark-tile.svg` | The same mark on the sidebar tile: panel-2 `#172033`, 1 px app `--line-2` border, radius 10 on 32, glyph at 24/32. |
| `ubiqx-wordmark.svg` / `-mono` | "ubiqX" in Sora 600, −0.02em, kerned with HarfBuzz, **converted to outlines** (fontTools, overlaps removed with skia-pathops). "ubiq" is ink `#e8edf9` and "X" is volt `#4d8dff`. There is one `<path id="wm-{char}">` per letter for staggered animation. The `-mono` variant uses `currentColor`. |
| `ubiqx-wordmark-bold.svg` / `-bold-mono` | The same, in Sora 700 at −0.03em (the film's wordmark spec). |
| `favicon-128.png`, `apple-touch-icon.png`, `og.png`, `ubi-hero.png` | Copied unchanged from `site/public/`. |

Wordmark geometry: the viewBox is the ink box in font units (UPM 1000), with the baseline at `metrics.baselineY`.
To match live Sora at font-size S px, render it at height `viewBox[3] / 1000 × S`. A canvas comparison against live
Sora text at 240 px gave pixel IoU 0.982 (600) and 0.988 (700), with ink widths within 1.1 px.

## Hosts

- Used: `registry.npmjs.org` (npm pack), `github.com` (git ls-remote to resolve commits), and
  `raw.githubusercontent.com` (files at pinned commits).
- `api.github.com` answered "GitHub access to this repository is not enabled for this session". The commits were
  resolved with `git ls-remote` and npm `gitHead` instead.
- Not tried, because they are blocked for this project: unsplash, pexels, pixabay, wikimedia (simple-icons' own
  macOS source link points there), freesound, openverse, huggingface, jsdelivr, unpkg, cdnjs, `*.cloudfront.net`
  and runwayml.com.
