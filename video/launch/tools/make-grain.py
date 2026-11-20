#!/usr/bin/env python3
"""Generate public/fx/grain.png — a 512x512 tileable gaussian film-grain tile.

Used by src/components/Grain.tsx (tiled, re-offset every frame, blended with
`overlay`). Pure numpy + zlib (no Pillow needed). Deterministic (seed 7).
Run from video/launch/:  python3 tools/make-grain.py
"""
import struct
import zlib

import numpy as np

rng = np.random.default_rng(7)
a = np.clip(rng.normal(128, 42, (512, 512)), 0, 255).astype(np.uint8)


def chunk(tag: bytes, data: bytes) -> bytes:
    return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data) & 0xFFFFFFFF)


h, w = a.shape
raw = b''.join(b'\x00' + a[y].tobytes() for y in range(h))
png = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 0, 0, 0, 0))
       + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))
with open('public/fx/grain.png', 'wb') as f:
    f.write(png)
print('wrote public/fx/grain.png', a.shape, round(float(a.std()), 1))
