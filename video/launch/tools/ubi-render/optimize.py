#!/usr/bin/env python3
"""Losslessly recompresses the UBI PNGs (oxipng level 2; pixels stay bit-identical, ~20 % smaller).

    pip install pyoxipng && python3 tools/ubi-render/optimize.py [--jobs 2] [dir ...]
"""
import argparse
import os
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import oxipng

ROOT = Path(__file__).resolve().parent.parent.parent / "public" / "ubi"


def one(p):
    before = p.stat().st_size
    oxipng.optimize(p, level=2, strip=oxipng.StripChunks.safe())
    return before, p.stat().st_size


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("dirs", nargs="*", help="sub-directories of public/ubi to optimise (default: all)")
    a = ap.parse_args()
    roots = [ROOT / d for d in a.dirs] if a.dirs else [ROOT]
    files = sorted(f for r in roots for f in r.rglob("*.png"))
    with ThreadPoolExecutor(a.jobs) as ex:
        res = list(ex.map(one, files))
    b = sum(r[0] for r in res)
    after = sum(r[1] for r in res)
    print(f"{len(files)} PNGs: {b / 1e6:.1f} MB -> {after / 1e6:.1f} MB ({after / max(b, 1):.0%})")


if __name__ == "__main__":
    main()
