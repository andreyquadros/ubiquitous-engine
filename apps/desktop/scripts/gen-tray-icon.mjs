#!/usr/bin/env node
// Generates UBI icons without any dependency beyond Node:
//   src-tauri/icons/tray.png      22x22  black-on-transparent macOS template icon (UBI head silhouette)
//   src-tauri/icons/tray@2x.png   44x44  same, retina
//   src-tauri/icons/app-icon.png  1024x1024 full-colour app icon (feed it to `pnpm tauri icon`)
//
// Shapes are signed-distance functions rasterised with supersampling; the PNG encoder is hand-written
// (zlib from node:zlib + CRC32).

import { deflateSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const outDir = resolve(here, '../src-tauri/icons');

/* ---------------------------------------------------------------- PNG */

const CRC_TABLE = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crc]);
}

/** rgba: Uint8Array of w*h*4 */
function encodePng(w, h, rgba) {
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0; // filter: none
    rgba.copy ? rgba.copy(raw, y * (w * 4 + 1) + 1, y * w * 4, (y + 1) * w * 4) : raw.set(rgba.subarray(y * w * 4, (y + 1) * w * 4), y * (w * 4 + 1) + 1);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

/* ---------------------------------------------------------------- SDF */

const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
const len = (x, y) => Math.hypot(x, y);

const sdRoundRect = (px, py, cx, cy, hw, hh, r) => {
  const qx = Math.abs(px - cx) - (hw - r);
  const qy = Math.abs(py - cy) - (hh - r);
  return len(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
};
const sdCircle = (px, py, cx, cy, r) => len(px - cx, py - cy) - r;
const sdEllipse = (px, py, cx, cy, rx, ry) => {
  // cheap approximation good enough for icons
  const k = len((px - cx) / rx, (py - cy) / ry);
  return (k - 1) * Math.min(rx, ry);
};
// Inigo Quilez' sdTriangle
const sdTriangle = (px, py, [ax, ay], [bx, by], [cx, cy]) => {
  const e0x = bx - ax, e0y = by - ay, e1x = cx - bx, e1y = cy - by, e2x = ax - cx, e2y = ay - cy;
  const v0x = px - ax, v0y = py - ay, v1x = px - bx, v1y = py - by, v2x = px - cx, v2y = py - cy;
  const d0 = clamp((v0x * e0x + v0y * e0y) / (e0x * e0x + e0y * e0y), 0, 1);
  const d1 = clamp((v1x * e1x + v1y * e1y) / (e1x * e1x + e1y * e1y), 0, 1);
  const d2 = clamp((v2x * e2x + v2y * e2y) / (e2x * e2x + e2y * e2y), 0, 1);
  const p0x = v0x - e0x * d0, p0y = v0y - e0y * d0;
  const p1x = v1x - e1x * d1, p1y = v1y - e1y * d1;
  const p2x = v2x - e2x * d2, p2y = v2y - e2y * d2;
  const s = Math.sign(e0x * e2y - e0y * e2x);
  const dx = Math.min(p0x * p0x + p0y * p0y, p1x * p1x + p1y * p1y, p2x * p2x + p2y * p2y);
  const dy = Math.min(s * (v0x * e0y - v0y * e0x), s * (v1x * e1y - v1y * e1x), s * (v2x * e2y - v2y * e2x));
  return -Math.sqrt(dx) * Math.sign(dy);
};
/** Upward crescent (a "smile" eye): circle minus a circle shifted down. */
const sdCrescent = (px, py, cx, cy, r, shift) => Math.max(sdCircle(px, py, cx, cy, r), -sdCircle(px, py, cx, cy + shift, r * 1.05));

/* ---------------------------------------------------------------- raster */

/** layers: [{ sdf(x,y) -> d, color(x,y) -> [r,g,b,a], soft? }] painted in order with "over". */
function raster(w, h, layers, ss = 3) {
  const out = Buffer.alloc(w * h * 4);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      let R = 0, G = 0, B = 0, A = 0;
      for (let sy = 0; sy < ss; sy++) {
        for (let sx = 0; sx < ss; sx++) {
          const px = x + (sx + 0.5) / ss;
          const py = y + (sy + 0.5) / ss;
          let r = 0, g = 0, b = 0, a = 0; // premultiplied
          for (const L of layers) {
            const d = L.sdf(px, py);
            let cov = L.soft ? (d <= 0 ? 1 : Math.exp(-d / L.soft)) : clamp(0.5 - d, 0, 1);
            if (cov <= 0) continue;
            const [cr, cg, cb, ca] = L.color(px, py);
            const alpha = cov * ca;
            r = cr * alpha + r * (1 - alpha);
            g = cg * alpha + g * (1 - alpha);
            b = cb * alpha + b * (1 - alpha);
            a = alpha + a * (1 - alpha);
          }
          R += r; G += g; B += b; A += a;
        }
      }
      const n = ss * ss;
      const i = (y * w + x) * 4;
      const a = A / n;
      out[i] = a > 0 ? Math.round((R / n / a) * 255) : 0;
      out[i + 1] = a > 0 ? Math.round((G / n / a) * 255) : 0;
      out[i + 2] = a > 0 ? Math.round((B / n / a) * 255) : 0;
      out[i + 3] = Math.round(a * 255);
    }
  }
  return out;
}

const hex = (h, a = 1) => {
  const n = parseInt(h.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255, a];
};
const solid = (h, a = 1) => () => hex(h, a);

/* ---------------------------------------------------------------- UBI head geometry (unit: 22px box) */

function headLayers(s, { template }) {
  // s = scale factor from the 22-unit design space; template = black silhouette with holes
  const S = (v) => v * s;
  const black = solid('#000000');
  const spikes = [
    [[S(5.2), S(6.4)], [S(7.2), S(2.2)], [S(9.2), S(6.4)]],
    [[S(9.0), S(6.0)], [S(11), S(0.8)], [S(13), S(6.0)]],
    [[S(12.8), S(6.4)], [S(14.8), S(2.2)], [S(16.8), S(6.4)]],
  ];
  if (template) {
    return [
      ...spikes.map((t) => ({ sdf: (x, y) => sdTriangle(x, y, ...t), color: black })),
      { sdf: (x, y) => sdRoundRect(x, y, S(11), S(13.2), S(9.4), S(7.6), S(5.6)), color: black },
      // visor hole
      { sdf: (x, y) => sdRoundRect(x, y, S(11), S(12.6), S(6.6), S(4.2), S(3.6)), color: solid('#000000', 0), holeOf: true },
      // eyes inside the visor
      { sdf: (x, y) => sdCrescent(x, y, S(8.2), S(13.2), S(2.0), S(1.25)), color: black },
      { sdf: (x, y) => sdCrescent(x, y, S(13.8), S(13.2), S(2.0), S(1.25)), color: black },
    ];
  }
  const white = solid('#ffffff');
  const line = solid('#cbd5e1');
  return [
    // spikes with outline
    ...spikes.map((t) => ({ sdf: (x, y) => sdTriangle(x, y, ...t) - S(0.25), color: line })),
    ...spikes.map((t) => ({ sdf: (x, y) => sdTriangle(x, y, ...t), color: white })),
    // blue cores of the spikes
    ...spikes.map(([, tip]) => ({ sdf: (x, y) => sdRoundRect(x, y, tip[0], tip[1] + S(2.4), S(0.28), S(1.5), S(0.28)), color: solid('#60a5fa'), soft: S(0.35) })),
    // head
    { sdf: (x, y) => sdRoundRect(x, y, S(11), S(13.2), S(9.4), S(7.6), S(5.6)) - S(0.25), color: line },
    { sdf: (x, y) => sdRoundRect(x, y, S(11), S(13.2), S(9.4), S(7.6), S(5.6)), color: white },
    // side pods
    { sdf: (x, y) => sdRoundRect(x, y, S(1.4), S(13.6), S(0.9), S(2.4), S(0.9)), color: white },
    { sdf: (x, y) => sdRoundRect(x, y, S(20.6), S(13.6), S(0.9), S(2.4), S(0.9)), color: white },
    { sdf: (x, y) => sdRoundRect(x, y, S(1.4), S(13.6), S(0.75), S(0.35), S(0.3)), color: solid('#2563eb') },
    { sdf: (x, y) => sdRoundRect(x, y, S(20.6), S(13.6), S(0.75), S(0.35), S(0.3)), color: solid('#2563eb') },
    // visor (dark gradient) + highlight
    {
      sdf: (x, y) => sdRoundRect(x, y, S(11), S(12.6), S(6.8), S(4.4), S(3.7)),
      color: (x, y) => {
        const t = clamp((y - S(8.2)) / S(8.8), 0, 1);
        return [0.03 + (0.12 - 0.03) * (1 - t), 0.06 + (0.16 - 0.06) * (1 - t), 0.15 + (0.23 - 0.15) * (1 - t), 1];
      },
    },
    { sdf: (x, y) => sdEllipse(x, y, S(8.6), S(9.6), S(3.2), S(0.9)), color: solid('#ffffff', 0.12) },
    // eye glow + eyes
    { sdf: (x, y) => sdCrescent(x, y, S(8.2), S(13.2), S(2.0), S(1.25)), color: solid('#3b82f6', 0.55), soft: S(0.9) },
    { sdf: (x, y) => sdCrescent(x, y, S(13.8), S(13.2), S(2.0), S(1.25)), color: solid('#3b82f6', 0.55), soft: S(0.9) },
    { sdf: (x, y) => sdCrescent(x, y, S(8.2), S(13.2), S(2.0), S(1.25)), color: solid('#7dd3fc') },
    { sdf: (x, y) => sdCrescent(x, y, S(13.8), S(13.2), S(2.0), S(1.25)), color: solid('#7dd3fc') },
  ];
}

/** Template icons need "holes": we render silhouette layers then subtract the visor, then add eyes. */
function templateRaster(size) {
  const s = size / 22;
  const layers = headLayers(s, { template: true });
  const solidLayers = layers.filter((l) => !l.holeOf);
  const hole = layers.find((l) => l.holeOf);
  const eyes = solidLayers.slice(-2);
  const body = solidLayers.slice(0, -2);
  // coverage = body AND NOT hole, OR eyes
  const composite = {
    sdf: (x, y) => {
      const bodyD = Math.min(...body.map((l) => l.sdf(x, y)));
      const carved = Math.max(bodyD, -hole.sdf(x, y));
      return Math.min(carved, ...eyes.map((l) => l.sdf(x, y)));
    },
    color: solid('#000000'),
  };
  return raster(size, size, [composite], 4);
}

function appIconRaster(size) {
  const s = size / 22;
  const S = (v) => v * s;
  const bg = {
    sdf: (x, y) => sdRoundRect(x, y, size / 2, size / 2, size / 2, size / 2, size * 0.225),
    color: (x, y) => {
      const t = clamp((x + y) / (2 * size), 0, 1);
      const a = hex('#3b82f6');
      const b = hex('#1e40af');
      return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, 1];
    },
  };
  // soft orange sash behind the head (UBI's accent) + glow under the head
  const sash = {
    sdf: (x, y) => {
      // diagonal band from bottom-left to right
      const nx = (x - size * 0.5) * 0.7071 + (y - size * 0.72) * 0.7071;
      return Math.abs(nx) - size * 0.075;
    },
    color: solid('#f97316', 0.9),
  };
  const glow = { sdf: (x, y) => sdCircle(x, y, size / 2, size * 0.55, size * 0.28), color: solid('#93c5fd', 0.35), soft: size * 0.08 };
  // head layers scaled to occupy the centre (design box 22 → fit ~64% of the icon)
  const k = 0.66;
  const off = { x: size * (1 - k) * 0.5, y: size * (1 - k) * 0.5 + size * 0.02 };
  const head = headLayers(s * k, { template: false }).map((l) => ({
    ...l,
    sdf: (x, y) => l.sdf(x - off.x, y - off.y),
    color: typeof l.color === 'function' ? (x, y) => l.color(x - off.x, y - off.y) : l.color,
  }));
  void S;
  // shadow under head
  const shadow = { sdf: (x, y) => sdEllipse(x, y, size / 2, size * 0.78, size * 0.26, size * 0.05), color: solid('#0f172a', 0.35), soft: size * 0.03 };
  return raster(size, size, [bg, sash, glow, shadow, ...head], 2);
}

mkdirSync(outDir, { recursive: true });
const t0 = Date.now();
writeFileSync(resolve(outDir, 'tray.png'), encodePng(22, 22, templateRaster(22)));
writeFileSync(resolve(outDir, 'tray@2x.png'), encodePng(44, 44, templateRaster(44)));
console.log(`tray.png / tray@2x.png written (${Date.now() - t0} ms)`);
const t1 = Date.now();
const big = 1024;
writeFileSync(resolve(outDir, 'app-icon.png'), encodePng(big, big, appIconRaster(big)));
console.log(`app-icon.png written (${Date.now() - t1} ms) → ${outDir}`);
