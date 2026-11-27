#!/usr/bin/env python3
"""Fetch the third-party logos and fonts for the launch video and build the ubiqX brand files.

Everything lands in video/launch/public/brand/. Sources are pinned (npm versions plus GitHub
commits), so a re-run reproduces the same files. See public/brand/CREDITS.md for licenses.

    python3 tools/build-brand-assets.py [--cache DIR]

Needs: npm, curl, python3 with fontTools (+ brotli), uharfbuzz and skia-pathops
(`pip install uharfbuzz skia-pathops`).
Reachable hosts only: registry.npmjs.org, github.com, raw.githubusercontent.com.
"""

from __future__ import annotations

import argparse
import io
import json
import re
import shutil
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]          # video/launch
REPO = ROOT.parents[1]                              # repository root
OUT = ROOT / "public" / "brand"

# ---- pinned sources ---------------------------------------------------------------------------
NPM = {
    "simple-icons": "16.33.0",                       # CC0-1.0
    "@lobehub/icons-static-svg": "1.95.1",           # MIT (lobehub/lobe-icons)
    "bootstrap-icons": "1.13.1",                     # MIT (twbs/icons)
    "devicon": "2.17.0",                             # MIT (devicons/devicon)
    "@fontsource-variable/sora": "5.3.0",            # OFL-1.1
    "@fontsource-variable/inter": "5.3.0",           # OFL-1.1
}
GOOGLE_FONTS_SHA = "23e54b51ddffbc7713c583748e3bd86f62b1fa4a"   # google/fonts main @ 2026-09-29
INTER_SHA = "e3a3d4c57d5ecc01453a575621882a384c1995a3"          # rsms/inter tag v4.1
RAW = "https://raw.githubusercontent.com"
GH_BLOB = {  # npm package -> (GitHub blob prefix at the published commit, path prefix inside the repo)
    "simple-icons": ("https://github.com/simple-icons/simple-icons/blob/5828a6df55f0afded88fce0b94ce801903ddefe8", "icons"),
    "@lobehub/icons-static-svg": ("https://github.com/lobehub/lobe-icons/blob/49a2130df7bfa5eb1b088261bff20a37e2967789", "packages/static-svg/icons"),
    "bootstrap-icons": ("https://github.com/twbs/icons/blob/ce0e49dd063243118a115f17ad1fe1fe7576d552", "icons"),
    "devicon": ("https://github.com/devicons/devicon/blob/54cfe13ac10eaa1ef817a343ab0a9437eb3c2e08", "icons"),
}

# ---- palette (project contract) ---------------------------------------------------------------
INK = "#e8edf9"
VOLT = "#4d8dff"
PANEL_2 = "#172033"
LINE_2 = "rgba(126,158,214,0.28)"                   # app dark theme --line-2 (tile border)

SVG_NS = "http://www.w3.org/2000/svg"
ET.register_namespace("", SVG_NS)
ET.register_namespace("xlink", "http://www.w3.org/1999/xlink")


def run(cmd: list[str], cwd: Path) -> str:
    return subprocess.run(cmd, cwd=cwd, check=True, capture_output=True, text=True).stdout


def npm_unpack(name: str, version: str, cache: Path) -> Path:
    dest = cache / (name.replace("/", "__").lstrip("@") + "-" + version)
    if (dest / "package").is_dir():
        return dest / "package"
    dest.mkdir(parents=True, exist_ok=True)
    tgz = run(["npm", "pack", f"{name}@{version}", "--silent"], dest).strip().splitlines()[-1]
    with tarfile.open(dest / tgz) as t:
        t.extractall(dest, filter="data")
    (dest / tgz).unlink()
    return dest / "package"


def curl(url: str, dest: Path) -> Path:
    if not dest.exists():
        dest.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["curl", "-sS", "-f", "-m", "120", "-o", str(dest), url], check=True)
    return dest


# ---- SVG normalisation ------------------------------------------------------------------------
def q(tag: str) -> str:
    return f"{{{SVG_NS}}}{tag}"


def tight_viewbox(root: ET.Element, pad: float) -> str:
    """Exact ink bounds of every <path d> (no transforms in the files we crop), padded."""
    from fontTools.pens.boundsPen import BoundsPen
    from fontTools.svgLib.path import parse_path

    bp = BoundsPen(None)
    for el in root.iter(q("path")):
        parse_path(el.get("d"), bp)
    x0, y0, x1, y1 = bp.bounds
    r = lambda v: f"{v:.3f}".rstrip("0").rstrip(".")
    return f"{r(x0 - pad)} {r(y0 - pad)} {r(x1 - x0 + 2 * pad)} {r(y1 - y0 + 2 * pad)}"


def normalise_svg(src: Path, title: str, *, mono: bool, crop: float | None = None) -> str:
    """Clean root attributes, add a <title>, and (mono) make every fill follow currentColor."""
    tree = ET.parse(src)
    root = tree.getroot()
    vb = root.get("viewBox")
    if not vb:
        raise SystemExit(f"{src}: no viewBox")
    if crop is not None:
        vb = tight_viewbox(root, crop)
    keep = {k: v for k, v in root.attrib.items() if k in ("fill-rule", "clip-rule")}
    root.attrib.clear()
    root.set("viewBox", vb)
    w, h = [float(x) for x in vb.split()[2:]]
    root.set("width", f"{w:g}")
    root.set("height", f"{h:g}")
    root.attrib.update(keep)
    root.set("role", "img")
    root.set("aria-label", title)
    for t in root.findall(q("title")):
        root.remove(t)
    if mono:
        root.set("fill", "currentColor")
        for el in root.iter():
            if el is root:
                continue
            f = el.get("fill")
            if f and f.lower() not in ("none", "currentcolor"):
                del el.attrib["fill"]
            el.attrib.pop("class", None)
    t = ET.Element(q("title"))
    t.text = title
    root.insert(0, t)
    body = ET.tostring(root, encoding="unicode", short_empty_elements=True)
    return body + "\n"


# ---- ubiqX brand files ------------------------------------------------------------------------
def brand_mark(tile: bool) -> str:
    """UBI's head, 1:1 with BrandMark in apps/desktop/src/components/layout/Sidebar.tsx."""
    glyph = (
        '<defs><linearGradient id="ubiqx-mark-visor" x1="0" y1="0" x2="0" y2="1">'
        '<stop offset="0" stop-color="#1b2540"/><stop offset="1" stop-color="#05070f"/>'
        "</linearGradient></defs>"
        '<path d="M11 9 L13 3.5 L15 9 Z M14.5 8.5 L16 2 L17.5 8.5 Z M17 9 L19 3.5 L21 9 Z" fill="#f4f7ff"/>'
        '<path d="M13.2 8.4 L13.2 5.5 M16 7.6 L16 4 M18.8 8.4 L18.8 5.5" stroke="#4d8dff" stroke-width="1" stroke-linecap="round"/>'
        '<rect x="4" y="8" width="24" height="20" rx="8" fill="#f4f7ff"/>'
        '<rect x="6.5" y="11" width="19" height="13" rx="6.5" fill="url(#ubiqx-mark-visor)"/>'
        '<path d="M10 19.5 Q12.5 15.5 15 19.5" stroke="#4d8dff" stroke-width="1.8" fill="none" stroke-linecap="round"/>'
        '<path d="M17 19.5 Q19.5 15.5 22 19.5" stroke="#4d8dff" stroke-width="1.8" fill="none" stroke-linecap="round"/>'
    )
    if not tile:
        return (
            f'<svg xmlns="{SVG_NS}" viewBox="0 0 32 32" width="32" height="32" role="img" aria-label="ubiqX">'
            f"<title>ubiqX</title>{glyph}</svg>\n"
        )
    # The sidebar tile: 32px box, radius-control 10px, 1px line-2 border, panel-2 fill, glyph at 24px.
    glyph = glyph.replace("ubiqx-mark-visor", "ubiqx-tile-visor")
    return (
        f'<svg xmlns="{SVG_NS}" viewBox="0 0 32 32" width="32" height="32" role="img" aria-label="ubiqX">'
        f"<title>ubiqX</title>"
        f'<rect x="0.5" y="0.5" width="31" height="31" rx="9.5" fill="{PANEL_2}" stroke="{LINE_2}" stroke-width="1"/>'
        f'<g transform="translate(4 4) scale(0.75)">{glyph}</g></svg>\n'
    )


def wordmark(sora_ttf: Path, weight: int, tracking_em: float, *, mono: bool) -> tuple[str, dict]:
    """'ubiqX' shaped with HarfBuzz at the given weight, outlined with fontTools, one path per glyph."""
    import uharfbuzz as hb
    from fontTools.pens.boundsPen import BoundsPen
    from fontTools.pens.svgPathPen import SVGPathPen
    from fontTools.pens.transformPen import TransformPen
    from fontTools.ttLib import TTFont
    from fontTools.varLib.instancer import OverlapMode, instantiateVariableFont

    # REMOVE merges overlapping contours (needs skia-pathops), so a stroke draw-on shows no seams.
    font = instantiateVariableFont(TTFont(sora_ttf), {"wght": weight}, inplace=False, overlap=OverlapMode.REMOVE)
    buf = io.BytesIO()
    font.save(buf)
    data = buf.getvalue()
    font = TTFont(io.BytesIO(data))
    upm = font["head"].unitsPerEm

    hb_font = hb.Font(hb.Face(hb.Blob(data)))
    text = "ubiqX"
    b = hb.Buffer()
    b.add_str(text)
    b.guess_segment_properties()
    hb.shape(hb_font, b, {"kern": True, "liga": False})
    order = font.getGlyphOrder()
    gs = font.getGlyphSet()
    track = tracking_em * upm

    placed = []                                           # (char, glyph name, x offset)
    pen_x = 0.0
    for i, (info, pos) in enumerate(zip(b.glyph_infos, b.glyph_positions)):
        name = order[info.codepoint]
        placed.append((text[info.cluster], name, pen_x + pos.x_offset, pos.y_offset))
        pen_x += pos.x_advance + (track if i < len(text) - 1 else 0)

    # Bounds in font units (y up).
    bp = BoundsPen(gs)
    for _, name, dx, dy in placed:
        gs[name].draw(TransformPen(bp, (1, 0, 0, 1, dx, dy)))
    xmin, ymin, xmax, ymax = bp.bounds
    pad = 0
    w, h = xmax - xmin + 2 * pad, ymax - ymin + 2 * pad

    paths = []
    for ch, name, dx, dy in placed:
        sp = SVGPathPen(gs, ntos=lambda v: f"{v:.1f}".rstrip("0").rstrip("."))
        # flip y and move the ink box to the origin
        gs[name].draw(TransformPen(sp, (1, 0, 0, -1, dx - xmin + pad, ymax + pad - dy)))
        colour = "currentColor" if mono else (VOLT if ch == "X" else INK)
        paths.append(f'<path id="wm-{ch}" data-char="{ch}" fill="{colour}" d="{sp.getCommands()}"/>')

    baseline = ymax + pad                                # y of the baseline inside the viewBox
    os2 = font["OS/2"]
    meta = {
        "unitsPerEm": upm,
        "viewBox": [0, 0, round(w, 1), round(h, 1)],
        "baselineY": round(baseline, 1),
        "capHeight": os2.sCapHeight,
        "xHeight": os2.sxHeight,
        "note": f"Sora {weight}, tracking {tracking_em:+g}em, kerned with HarfBuzz; ink box, "
                f"baseline at y={baseline:.1f} of {h:.1f}. To match Sora at font-size S px, "
                f"render at height {h / upm:.4f}*S px.",
    }
    fill_attr = ' fill="currentColor"' if mono else ""
    svg = (
        f'<svg xmlns="{SVG_NS}" viewBox="0 0 {w:.1f} {h:.1f}" width="{w / upm * 100:.1f}" '
        f'height="{h / upm * 100:.1f}" role="img" aria-label="ubiqX"{fill_attr}>'
        f"<title>ubiqX</title>{''.join(paths)}</svg>\n"
    )
    return svg, meta


# ---- main -------------------------------------------------------------------------------------
def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", type=Path, default=None, help="download cache (default: a temp dir)")
    args = ap.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp(prefix="brand-cache-"))
    cache.mkdir(parents=True, exist_ok=True)

    pk = {n: npm_unpack(n, v, cache) for n, v in NPM.items()}

    def src_url(path: Path) -> str:
        """npm spec + the byte-identical file on GitHub at the package's gitHead."""
        for name, root in pk.items():
            if root in path.parents:
                rel = path.relative_to(root / "icons").as_posix()
                blob, prefix = GH_BLOB[name]
                return f"npm:{name}@{NPM[name]} icons/{rel} = {blob}/{prefix}/{rel}"
        raise KeyError(path)

    def src_url_root(name: str, file: str) -> str:
        blob = GH_BLOB[name][0]
        return f"npm:{name}@{NPM[name]} {file} = {blob}/{file}"

    si, lobe, bi, dev = (pk["simple-icons"] / "icons", pk["@lobehub/icons-static-svg"] / "icons",
                         pk["bootstrap-icons"] / "icons", pk["devicon"] / "icons")
    sora_ttf = curl(f"{RAW}/google/fonts/{GOOGLE_FONTS_SHA}/ofl/sora/Sora%5Bwght%5D.ttf", cache / "gf" / "Sora[wght].ttf")
    sora_ofl = curl(f"{RAW}/google/fonts/{GOOGLE_FONTS_SHA}/ofl/sora/OFL.txt", cache / "gf" / "sora-OFL.txt")
    inter_ofl = curl(f"{RAW}/google/fonts/{GOOGLE_FONTS_SHA}/ofl/inter/OFL.txt", cache / "gf" / "inter-OFL.txt")
    inter_full = curl(f"{RAW}/rsms/inter/{INTER_SHA}/docs/font-files/InterVariable.woff2", cache / "inter" / "InterVariable.woff2")

    lobe_license = curl(f"{RAW}/lobehub/lobe-icons/49a2130df7bfa5eb1b088261bff20a37e2967789/LICENSE", cache / "lobe-LICENSE")
    inter_license = curl(f"{RAW}/rsms/inter/{INTER_SHA}/LICENSE.txt", cache / "inter" / "LICENSE.txt")

    for sub in ("ai", "os", "social", "fonts", "licenses"):
        (OUT / sub).mkdir(parents=True, exist_ok=True)
    manifest: list[dict] = []

    def emit(rel: str, content: str | bytes, *, listed: bool = True, **entry) -> None:
        p = OUT / rel
        if isinstance(content, str):
            p.write_text(content, encoding="utf-8")
        else:
            p.write_bytes(content)
        if listed:                                    # license texts are written but not listed as assets
            manifest.append({"file": rel, **entry})

    SI = "CC0-1.0 (simple-icons 16.33.0)"
    LOBE = "MIT (lobehub/lobe-icons 1.95.1)"
    BI = "MIT (twbs/icons 1.13.1)"
    DEV = "MIT (devicons/devicon 2.17.0)"
    NOMINATIVE = "Trademark of its owner; used only nominatively to state compatibility."
    NO_PROVIDER_LOGOS = ("style.md S20 and facts.md 5.18 forbid provider logos on screen (names as plain text only). "
                         "Kept for reference or a possible brief change; do not place in scenes without sign-off.")

    # AI providers (mono = currentColor) + colour versions where the brand has one.
    ai = [
        ("ai/anthropic.svg", si / "anthropic.svg", "Anthropic", True, SI, "Anthropic wordmark-A glyph (brand colour #191919: monochrome brand)."),
        ("ai/claude.svg", si / "claude.svg", "Claude", True, SI, "Claude spark."),
        ("ai/claude-color.svg", lobe / "claude-color.svg", "Claude", False, LOBE, "Claude spark in brand terracotta #D97757."),
        ("ai/openai.svg", lobe / "openai.svg", "OpenAI", True, LOBE, "OpenAI blossom (absent from simple-icons since v11; monochrome brand)."),
        ("ai/xai.svg", lobe / "xai.svg", "xAI", True, LOBE, "xAI slashed-X mark (the upstream file's <title> says 'Grok'; it is the xAI mark)."),
        ("ai/grok.svg", lobe / "grok.svg", "Grok", True, LOBE, "Grok orbit mark (monochrome brand)."),
    ]
    for rel, src, name, mono, lic, note in ai:
        emit(rel, normalise_svg(src, name, mono=mono), kind="logo", name=name, colored=not mono,
             license=lic, source=src_url(src), note=f"{note} {NOMINATIVE} {NO_PROVIDER_LOGOS}")

    # Operating systems.
    win11_mono = dev / "windows11" / "windows11-original.svg"
    os_ = [
        ("os/apple.svg", si / "apple.svg", "Apple (macOS)", True, SI, "Apple logo — use for macOS."),
        ("os/macos-wordmark.svg", si / "macos.svg", "macOS wordmark", True, SI, "Lower-case 'macOS' wordmark; viewBox cropped to the ink."),
        ("os/windows.svg", bi / "windows.svg", "Windows", True, BI, "Windows 10-style perspective four-pane flag (simple-icons removed Microsoft/Windows logos)."),
        ("os/windows11.svg", win11_mono, "Windows 11", True, DEV, "Windows 11 flat four-square mark, recoloured to currentColor."),
        ("os/windows11-color.svg", win11_mono, "Windows 11", False, DEV, "Windows 11 mark in #0078D4."),
        ("os/linux.svg", si / "linux.svg", "Linux (Tux)", True, SI, "Tux silhouette."),
        ("os/linux-color.svg", dev / "linux" / "linux-original.svg", "Linux (Tux)", False, DEV, "Full-colour Tux (gradients, ~190 KB)."),
    ]
    for rel, src, name, mono, lic, note in os_:
        crop = 0.05 if rel == "os/macos-wordmark.svg" else None      # 24x24 box is mostly empty
        emit(rel, normalise_svg(src, name, mono=mono, crop=crop), kind="os", name=name, colored=not mono,
             license=lic, source=src_url(src), note=f"{note} {NOMINATIVE}")

    emit("social/x.svg", normalise_svg(si / "x.svg", "X", mono=True), kind="logo", name="X (social network)",
         colored=False, license=SI, source=src_url(si / "x.svg"),
         note="The X social network logo — NOT xAI or Grok. No ubiqX handle is documented in facts.md; no current use. " + NOMINATIVE)

    # Fonts.
    FS = "npm:@fontsource-variable/{}@5.3.0 files/{}"
    fonts = [
        ("fonts/sora-latin-wght-normal.woff2", pk["@fontsource-variable/sora"] / "files" / "sora-latin-wght-normal.woff2", "Sora Variable (latin, wght 100–800)"),
        ("fonts/sora-latin-ext-wght-normal.woff2", pk["@fontsource-variable/sora"] / "files" / "sora-latin-ext-wght-normal.woff2", "Sora Variable (latin-ext, wght 100–800)"),
        ("fonts/inter-latin-wght-normal.woff2", pk["@fontsource-variable/inter"] / "files" / "inter-latin-wght-normal.woff2", "Inter Variable (latin, wght 100–900)"),
        ("fonts/inter-latin-ext-wght-normal.woff2", pk["@fontsource-variable/inter"] / "files" / "inter-latin-ext-wght-normal.woff2", "Inter Variable (latin-ext, wght 100–900)"),
        ("fonts/InterVariable.woff2", inter_full, "Inter Variable 4.1, full glyph set (opsz 14–32, wght 100–900; arrows, check marks)"),
    ]
    for rel, src, name in fonts:
        source = (f"{RAW}/rsms/inter/{INTER_SHA}/docs/font-files/InterVariable.woff2 (tag v4.1)" if src == inter_full
                  else FS.format("sora" if "sora" in src.name else "inter", src.name))
        emit(rel, src.read_bytes(), kind="font", name=name, colored=False, license="OFL-1.1", source=source)

    # Full Sora variable as woff2 (all 513 glyphs, one file) from the google/fonts TTF.
    from fontTools.ttLib import TTFont
    f = TTFont(sora_ttf, recalcTimestamp=False)             # keep head.modified: reproducible bytes
    f.flavor = "woff2"
    b = io.BytesIO()
    f.save(b)
    gf = f"{RAW}/google/fonts/{GOOGLE_FONTS_SHA}/ofl"
    emit("fonts/SoraVariable.woff2", b.getvalue(), kind="font", name="Sora Variable, full glyph set (wght 100–800)",
         colored=False, license="OFL-1.1", source=f"{gf}/sora/Sora%5Bwght%5D.ttf, re-wrapped as woff2 (fontTools; outlines unchanged)")
    emit("fonts/OFL-Sora.txt", sora_ofl.read_text(), kind="font", name="Sora license (SIL OFL 1.1)", colored=False, license="OFL-1.1",
         source=f"{gf}/sora/OFL.txt")
    emit("fonts/OFL-Inter.txt", inter_ofl.read_text(), kind="font", name="Inter license (SIL OFL 1.1)", colored=False, license="OFL-1.1",
         source=f"{gf}/inter/OFL.txt")
    emit("fonts/fonts.css", FONTS_CSS, kind="font", name="@font-face declarations for the files above", colored=False, license="OFL-1.1 (fonts)")

    # Upstream license texts (MIT asks for the notice to travel with the files).
    for rel, src, name, lic, source in [
        ("licenses/simple-icons-LICENSE.md", pk["simple-icons"] / "LICENSE.md", "simple-icons license (CC0 1.0)", "CC0-1.0", src_url_root("simple-icons", "LICENSE.md")),
        ("licenses/simple-icons-DISCLAIMER.md", pk["simple-icons"] / "DISCLAIMER.md", "simple-icons trademark disclaimer", "CC0-1.0", src_url_root("simple-icons", "DISCLAIMER.md")),
        ("licenses/lobe-icons-LICENSE.txt", lobe_license, "lobe-icons license (MIT)", "MIT", "https://github.com/lobehub/lobe-icons/blob/49a2130df7bfa5eb1b088261bff20a37e2967789/LICENSE"),
        ("licenses/bootstrap-icons-LICENSE.txt", pk["bootstrap-icons"] / "LICENSE", "Bootstrap Icons license (MIT)", "MIT", src_url_root("bootstrap-icons", "LICENSE")),
        ("licenses/devicon-LICENSE.txt", pk["devicon"] / "LICENSE", "devicon license (MIT)", "MIT", src_url_root("devicon", "LICENSE")),
        ("fonts/OFL-Inter-rsms.txt", inter_license, "Inter 4.1 license as shipped by rsms/inter (SIL OFL 1.1)", "OFL-1.1", f"{RAW}/rsms/inter/{INTER_SHA}/LICENSE.txt"),
    ]:
        emit(rel, src.read_text(encoding="utf-8"), listed=rel.startswith("fonts/"), kind="font", name=name,
             colored=False, license=lic, source=source)

    # ubiqX brand (first-party, from this repository).
    FIRST = "First-party ubiqX asset (this repository, MIT; see LICENSE)"
    emit("ubiqx-mark.svg", brand_mark(False), kind="brand", name="ubiqX mark (UBI head)", colored=True, license=FIRST,
         source="apps/desktop/src/components/layout/Sidebar.tsx BrandMark",
         note="Also the mark for the managed 'IA do Ubi' provider option.")
    emit("ubiqx-mark-tile.svg", brand_mark(True), kind="brand", name="ubiqX mark on the sidebar tile", colored=True, license=FIRST,
         source="Sidebar.tsx BrandMark wrapper (panel-2 fill, line-2 border, radius 10)")
    for rel, weight, track, mono, label in [
        ("ubiqx-wordmark.svg", 600, -0.02, False, "semibold, as in the app sidebar"),
        ("ubiqx-wordmark-mono.svg", 600, -0.02, True, "semibold, monochrome currentColor"),
        ("ubiqx-wordmark-bold.svg", 700, -0.03, False, "bold, style.md §4.2 wordmark spec"),
        ("ubiqx-wordmark-bold-mono.svg", 700, -0.03, True, "bold, monochrome currentColor"),
    ]:
        svg, meta = wordmark(sora_ttf, weight, track, mono=mono)
        emit(rel, svg, kind="brand", name=f"ubiqX wordmark ({label})", colored=not mono, license=FIRST + "; glyph outlines from Sora (OFL-1.1)",
             source=f"outlined from {RAW}/google/fonts/{GOOGLE_FONTS_SHA}/ofl/sora/Sora%5Bwght%5D.ttf", note=meta.pop("note"), metrics=meta)
    png_notes = {
        "og.png": "Reference only: its fine print says 'Tudo fica no seu Mac', which is macOS-only and contradicts "
                  "facts.md (three platforms §3.16; cloud AI §5.3). Do not show it on screen.",
    }
    for name, label in [("favicon-128.png", "App icon 128 px"), ("apple-touch-icon.png", "App icon 180 px"),
                        ("og.png", "Open Graph card 1200×630"), ("ubi-hero.png", "UBI hero render 880×1056")]:
        extra = {"note": png_notes[name]} if name in png_notes else {}
        emit(name, (REPO / "site" / "public" / name).read_bytes(), kind="brand", name=label, colored=True,
             license=FIRST, source=f"site/public/{name}", **extra)

    (OUT / "brand-manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {len(manifest)} assets to {OUT}")


FONTS_CSS = """/* Sora + Inter for the launch video. Load with @remotion/fonts loadFont() in scenes; this file is for
   plain HTML previews. The fontsource subsets (latin + latin-ext) cover PT-BR; the full files add arrows etc. */
@font-face { font-family: "Sora"; font-style: normal; font-display: block; font-weight: 100 800;
  src: url("./sora-latin-wght-normal.woff2") format("woff2-variations");
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD; }
@font-face { font-family: "Sora"; font-style: normal; font-display: block; font-weight: 100 800;
  src: url("./sora-latin-ext-wght-normal.woff2") format("woff2-variations");
  unicode-range: U+0100-02BA, U+02BD-02C5, U+02C7-02CC, U+02CE-02D7, U+02DD-02FF, U+0304, U+0308, U+0329, U+1D00-1DBF, U+1E00-1E9F, U+1EF2-1EFF, U+2020, U+20A0-20AB, U+20AD-20C0, U+2113, U+2C60-2C7F, U+A720-A7FF; }
@font-face { font-family: "Inter"; font-style: normal; font-display: block; font-weight: 100 900;
  src: url("./InterVariable.woff2") format("woff2-variations"); }
@font-face { font-family: "Sora Full"; font-style: normal; font-display: block; font-weight: 100 800;
  src: url("./SoraVariable.woff2") format("woff2-variations"); }
"""

if __name__ == "__main__":
    main()
