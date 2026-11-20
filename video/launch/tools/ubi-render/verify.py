#!/usr/bin/env python3
"""Checks the UBI PNG sequences rendered by render.mjs and writes contact sheets.

    python3 tools/ubi-render/verify.py [--sheets DIR] [--update-manifest]

Per sequence: frame count and naming, RGBA mode, alpha bounding box per frame (union + minimum margin to the frame
edge, so nothing is clipped), the edge-fringe ratio (luminance of anti-aliased edge pixels vs the opaque pixels next to
them: a black matte shows as a ratio well below 1), and for loops the seam (last -> first frame difference compared with
the typical frame-to-frame difference). With --update-manifest the measured boxes are written into ubi-manifest.json.
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
PUBLIC = HERE.parent.parent / "public"
CANVAS = (10, 13, 22)


def load(p):
    im = Image.open(p)
    if im.mode != "RGBA":
        raise SystemExit(f"{p}: mode {im.mode}, expected RGBA")
    return np.asarray(im)


def bbox(a):
    al = a[..., 3] > 8
    ys, xs = np.where(al)
    if not len(xs):
        return None
    return [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())]


def fringe(a):
    """Mean luminance of straight-alpha edge pixels / mean luminance of the opaque pixels within 2 px."""
    al = a[..., 3].astype(np.int32)
    rgb = a[..., :3].astype(np.float32)
    lum = rgb @ np.array([0.2126, 0.7152, 0.0722], np.float32)
    edge = (al > 16) & (al < 240)
    opaque = al == 255
    # dilate the opaque mask's luminance into the neighbourhood (mean of opaque neighbours in a 5x5 box)
    pad = 2
    op = np.pad(opaque.astype(np.float32), pad)
    lu = np.pad(np.where(opaque, lum, 0), pad)
    s = np.zeros_like(lum)
    c = np.zeros_like(lum)
    h, w = lum.shape
    for dy in range(-pad, pad + 1):
        for dx in range(-pad, pad + 1):
            s += lu[pad + dy : pad + dy + h, pad + dx : pad + dx + w]
            c += op[pad + dy : pad + dy + h, pad + dx : pad + dx + w]
    m = edge & (c > 0)
    if not m.any():
        return None
    neigh = s[m] / c[m]
    return float(lum[m].mean() / max(neigh.mean(), 1e-3)), int(m.sum())


def over(a, bg=CANVAS):
    im = Image.fromarray(a, "RGBA")
    base = Image.new("RGBA", im.size, bg + (255,))
    base.alpha_composite(im)
    return base.convert("RGB")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sheets", default=None)
    ap.add_argument("--update-manifest", action="store_true")
    ap.add_argument("--full", action="store_true", help="fringe metric on every frame (default: first/middle/last)")
    args = ap.parse_args()
    mpath = PUBLIC / "ubi" / "ubi-manifest.json"
    man = json.loads(mpath.read_text())
    W, H = man["size"]
    ok = True
    report = []
    sheet_rows = []
    for clip in man["clips"]:
        d = PUBLIC / clip["dir"]
        files = sorted(d.glob("*.png"))
        names_ok = [f.name for f in files] == [f"{i:04d}.png" for i in range(len(files))]
        n = len(files)
        if n == 0:
            print(f"{clip['name']}: NO FRAMES")
            ok = False
            continue
        union = None
        min_margin = 10**9
        diffs = []
        prev = None
        first = None
        sample_idx = sorted({0, n // 2, n - 1})
        fr = []
        for i, f in enumerate(files):
            a = load(f)
            if a.shape[:2] != (H, W):
                print(f"{f}: size {a.shape[1]}x{a.shape[0]} != {W}x{H}")
                ok = False
            b = bbox(a)
            if b:
                union = b if union is None else [min(union[0], b[0]), min(union[1], b[1]), max(union[2], b[2]), max(union[3], b[3])]
                min_margin = min(min_margin, b[0], b[1], W - 1 - b[2], H - 1 - b[3])
            if i == 0:
                first = a
            if prev is not None:
                diffs.append(float(np.abs(a.astype(np.int16) - prev.astype(np.int16)).mean()))
            if args.full or i in sample_idx:
                r = fringe(a)
                if r:
                    fr.append(r[0])
            prev = a
        seam = float(np.abs(first.astype(np.int16) - prev.astype(np.int16)).mean())
        typical = float(np.median(diffs)) if diffs else 0.0
        peak = float(np.max(diffs)) if diffs else 0.0
        size_mb = sum(f.stat().st_size for f in files) / 1e6
        # a loop is clean when the wrap (last -> first) is no bigger than the steps around it
        near = max(diffs[:3] + diffs[-3:]) if diffs else 0.0
        loop_ok = seam <= max(near, typical, 1e-6) * 1.35 if clip["loopable"] else None
        # a first step far above every other one means frame 0 is not posed like the rest (stale/unapplied channels)
        rest_max = max(diffs[1:]) if len(diffs) > 1 else 0.0
        first_outlier = bool(diffs) and diffs[0] > 2.0 * max(rest_max, 1e-6) and diffs[0] > 3.0 * max(typical, 1e-6)
        row = {
            "name": clip["name"],
            "frames": n,
            "names_ok": names_ok,
            "bbox_union_px": union,
            "min_margin_px": min_margin,
            "fringe_ratio": round(float(np.mean(fr)), 3) if fr else None,
            "step_diff_median": round(typical, 3),
            "step_diff_max": round(peak, 3),
            "loop_seam_diff": round(seam, 3),
            "loop_ok": loop_ok,
            "first_step_outlier": first_outlier,
            "mb": round(size_mb, 1),
        }
        if not names_ok or min_margin <= 2 or (clip["loopable"] and not loop_ok) or n != clip["frames"] or first_outlier:
            ok = False
        report.append(row)
        print(json.dumps(row))
        if args.update_manifest:
            clip["bytes"] = sum(f.stat().st_size for f in files)
            clip["bbox_union_px"] = union
            clip["min_margin_px"] = min_margin
            if clip["loopable"]:
                clip["loop_seam_check"] = {"last_to_first_diff": round(seam, 3), "max_step_diff": round(peak, 3)}
        if args.sheets:
            tiles = []
            for i in sample_idx:
                t = over(load(files[i])).resize((300, 300), Image.LANCZOS)
                tiles.append(t)
            sheet_rows.append((clip["name"], tiles, sample_idx))
    if args.update_manifest and man["clips"]:
        idle = PUBLIC / man["clips"][0]["dir"] / "0000.png"
        b = bbox(load(idle))
        man["idle_frame0_bbox_px"] = b
        man["feet_bottom_y_px"] = b[3] if b else None
        # which sequences can be cut into each other without a pop (mean abs RGBA difference, 0-255)
        def seq_frame(name, idx):
            c = next((c for c in man["clips"] if c["name"] == name), None)
            if not c:
                return None
            fs = sorted((PUBLIC / c["dir"]).glob("*.png"))
            return load(fs[idx]).astype(np.int16) if fs else None

        idle0 = seq_frame("idle", 0)
        cont = {}
        for name in ("yes", "no", "wave", "jump", "turntable", "look", "excited", "worried", "sleep"):
            for idx, tag in ((0, "first"), (-1, "last")):
                f = seq_frame(name, idx)
                if f is not None and idle0 is not None:
                    cont[f"idle[0] vs {name}[{tag}]"] = round(float(np.abs(f - idle0).mean()), 4)
        man["continuity_vs_idle_frame0"] = cont
        for st in man.get("stills", []):
            sp = PUBLIC / st["file"]
            if sp.exists():
                st["bytes"] = sp.stat().st_size
                st["bbox_px"] = bbox(load(sp))
        man["total_bytes"] = sum(c.get("bytes", 0) for c in man["clips"]) + sum(s.get("bytes", 0) for s in man.get("stills", []))
        mpath.write_text(json.dumps(man, indent=2, ensure_ascii=False) + "\n")
    if args.sheets:
        from PIL import ImageDraw

        out = Path(args.sheets)
        out.mkdir(parents=True, exist_ok=True)
        per = 5
        for k in range(0, len(sheet_rows), per):
            chunk = sheet_rows[k : k + per]
            sheet = Image.new("RGB", (900 + 160, 300 * len(chunk)), (30, 30, 30))
            dr = ImageDraw.Draw(sheet)
            for r, (name, tiles, idx) in enumerate(chunk):
                dr.text((8, 300 * r + 8), name, fill=(255, 255, 255))
                for c, t in enumerate(tiles):
                    sheet.paste(t, (160 + 300 * c, 300 * r))
                    dr.text((160 + 300 * c + 6, 300 * r + 6), f"#{idx[c]}", fill=(200, 200, 200))
            sheet.save(out / f"sheet-{k // per}.png")
    print("OK" if ok else "PROBLEMS FOUND")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
