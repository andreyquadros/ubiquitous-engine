import { motion, useMotionValue, useReducedMotion, useSpring } from 'framer-motion';
import { useEffect, useState, type PointerEvent } from 'react';
import clsx from 'clsx';
import type { Mood } from '../../lib/types';
import { MOOD_GLOW } from './moods';

/** The user's own UBI art, installed by scripts/install-ubi-model.sh. */
export const PNG_URL = '/ubi/ubi.png';

let probe: Promise<boolean> | null = null;

/** Checks once (HEAD) whether /ubi/ubi.png exists. In jsdom the relative fetch throws → false. */
export function probePng(): Promise<boolean> {
  if (!probe) {
    probe = (async () => {
      try {
        if (typeof window === 'undefined') return false;
        const res = await fetch(PNG_URL, { method: 'HEAD' });
        if (!res.ok) return false;
        const ct = res.headers.get('content-type') ?? '';
        return ct.startsWith('image/');
      } catch {
        return false;
      }
    })();
  }
  return probe;
}

/** Test hook. */
export function __resetPngProbe(): void {
  probe = null;
  cutout = null;
}

interface Cutout {
  url: string;
  width: number;
  height: number;
}

let cutout: Promise<Cutout | null> | null = null;

const MAX_EDGE = 1024;
const WHITE_DIST = 40; // RGB distance to white below which a pixel counts as background

/**
 * Removes a plain white background: flood-fills from the four edges over near-white pixels, so the
 * white armour inside UBI (surrounded by darker outlines) stays opaque, then feathers the boundary.
 */
function knockoutWhite(img: HTMLImageElement): Cutout | null {
  const scale = Math.min(1, MAX_EDGE / Math.max(img.naturalWidth, img.naturalHeight));
  const w = Math.max(1, Math.round(img.naturalWidth * scale));
  const h = Math.max(1, Math.round(img.naturalHeight * scale));
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, w, h);
  let image: ImageData;
  try {
    image = ctx.getImageData(0, 0, w, h);
  } catch {
    return null;
  }
  const px = image.data;
  const corner = (x: number, y: number) => px[(y * w + x) * 4 + 3] ?? 255;
  // already transparent → nothing to do
  if (corner(0, 0) < 16 && corner(w - 1, 0) < 16 && corner(0, h - 1) < 16 && corner(w - 1, h - 1) < 16) return null;

  const n = w * h;
  const dist = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const o = i * 4;
    const dr = 255 - (px[o] ?? 0);
    const dg = 255 - (px[o + 1] ?? 0);
    const db = 255 - (px[o + 2] ?? 0);
    dist[i] = Math.sqrt(dr * dr + dg * dg + db * db);
  }
  const removed = new Uint8Array(n);
  const stack: number[] = [];
  const push = (i: number) => {
    if (!removed[i] && (dist[i] ?? 999) < WHITE_DIST) {
      removed[i] = 1;
      stack.push(i);
    }
  };
  for (let x = 0; x < w; x++) {
    push(x);
    push((h - 1) * w + x);
  }
  for (let y = 0; y < h; y++) {
    push(y * w);
    push(y * w + w - 1);
  }
  while (stack.length) {
    const i = stack.pop() as number;
    const x = i % w;
    if (x > 0) push(i - 1);
    if (x < w - 1) push(i + 1);
    if (i >= w) push(i - w);
    if (i + w < n) push(i + w);
  }
  // feather: kept pixels touching the removed region fade by how close to white they are
  const ring = new Uint8Array(n);
  for (let i = 0; i < n; i++) {
    if (removed[i]) {
      px[i * 4 + 3] = 0;
      continue;
    }
    const x = i % w;
    const touches = (x > 0 && removed[i - 1]) || (x < w - 1 && removed[i + 1]) || (i >= w && removed[i - w]) || (i + w < n && removed[i + w]);
    if (touches) ring[i] = 1;
  }
  for (let i = 0; i < n; i++) {
    if (!ring[i]) continue;
    const d = dist[i] ?? 0;
    const k = Math.max(0.2, Math.min(1, (d - WHITE_DIST) / 60));
    px[i * 4 + 3] = Math.round((px[i * 4 + 3] ?? 255) * k);
    const x = i % w;
    for (const j of [x > 0 ? i - 1 : -1, x < w - 1 ? i + 1 : -1, i >= w ? i - w : -1, i + w < n ? i + w : -1]) {
      if (j >= 0 && !removed[j] && !ring[j]) {
        const dj = dist[j] ?? 0;
        const kj = Math.max(0.6, Math.min(1, (dj - WHITE_DIST) / 40));
        px[j * 4 + 3] = Math.round((px[j * 4 + 3] ?? 255) * kj);
      }
    }
  }
  ctx.putImageData(image, 0, 0);
  try {
    return { url: canvas.toDataURL('image/png'), width: w, height: h };
  } catch {
    return null;
  }
}

/** Loads the PNG once and caches the background-free version (object URL or data URL). */
export function loadCutout(src = PNG_URL): Promise<Cutout | null> {
  if (!cutout) {
    cutout = new Promise<Cutout | null>((resolve) => {
      if (typeof Image === 'undefined') return resolve(null);
      const img = new Image();
      img.onload = () => {
        const raw = { url: src, width: img.naturalWidth || 1, height: img.naturalHeight || 1 };
        try {
          const cut = knockoutWhite(img);
          if (!cut) return resolve(raw);
          // prefer an object URL (cheaper to paint than a huge data URL)
          fetch(cut.url)
            .then((r) => r.blob())
            .then((b) => resolve({ ...cut, url: URL.createObjectURL(b) }))
            .catch(() => resolve(cut));
        } catch {
          resolve(raw);
        }
      };
      img.onerror = () => resolve(null);
      img.src = src;
    });
  }
  return cutout;
}

export interface UbiImageProps {
  mood: Mood;
  /** Width in px; the box is size × 1.2 like the other UBI variants. */
  size?: number;
  /** `head` shows a circular head crop (rail avatar). */
  crop?: 'full' | 'head';
  parallax?: boolean;
  glowFloor?: boolean;
  className?: string;
  src?: string;
}

function Sparkle({ x, y, s, delay, reduce }: { x: number; y: number; s: number; delay: number; reduce: boolean }) {
  return (
    <motion.svg
      viewBox="0 0 24 24"
      width={s}
      height={s}
      className="absolute"
      style={{ left: `${x}%`, top: `${y}%` }}
      animate={reduce ? undefined : { scale: [0.7, 1.15, 0.7], opacity: [0.5, 1, 0.5], rotate: [0, 20, 0] }}
      transition={{ duration: 1.6, repeat: Infinity, delay, ease: 'easeInOut' }}
      aria-hidden
    >
      <path d="M12 2 L14.2 9.8 L22 12 L14.2 14.2 L12 22 L9.8 14.2 L2 12 L9.8 9.8 Z" fill="#ffc24d" />
    </motion.svg>
  );
}

/** Small mood props drawn over the picture; positions assume the head fills the top ~40 %. */
function MoodOverlay({ mood, reduce }: { mood: Mood; reduce: boolean }) {
  switch (mood) {
    case 'excited':
      return (
        <>
          <Sparkle x={6} y={8} s={22} delay={0} reduce={reduce} />
          <Sparkle x={82} y={4} s={16} delay={0.5} reduce={reduce} />
          <Sparkle x={88} y={30} s={12} delay={0.9} reduce={reduce} />
        </>
      );
    case 'sleeping':
      return (
        <motion.svg viewBox="0 0 48 32" width="22%" className="absolute" style={{ left: '74%', top: '2%' }} animate={reduce ? undefined : { y: [6, -8], x: [0, 6], opacity: [0, 1, 0] }} transition={{ duration: 2.6, repeat: Infinity, ease: 'easeOut' }} aria-hidden>
          <text x="0" y="26" fill="#9daec7" fontSize="22" fontWeight="700" fontFamily="Sora Variable, Inter Variable, sans-serif">
            zzz
          </text>
        </motion.svg>
      );
    case 'worried':
      return (
        <motion.svg viewBox="0 0 16 22" width="7%" className="absolute" style={{ left: '80%', top: '14%' }} animate={reduce ? undefined : { y: [0, 6, 0], opacity: [0.85, 1, 0.85] }} transition={{ duration: 1.6, repeat: Infinity, ease: 'easeInOut' }} aria-hidden>
          <path d="M8 1 C8 1 1 11 1 14.5 A7 7 0 0 0 15 14.5 C15 11 8 1 8 1Z" fill="#7dd3fc" stroke="#38bdf8" strokeWidth="1" />
          <ellipse cx="5.5" cy="14" rx="1.6" ry="2.4" fill="#e0f2fe" opacity="0.8" />
        </motion.svg>
      );
    default:
      return null;
  }
}

/**
 * UBI from the user's PNG: white background knocked out once, idle float, tiny tilt, floor glow in the
 * mood colour, drop-shadow glow and a subtle pointer parallax. Renders nothing until the picture is ready.
 */
export function UbiImage({ mood, size = 160, crop = 'full', parallax = true, glowFloor = true, className, src }: UbiImageProps) {
  const [cut, setCut] = useState<Cutout | null>(null);
  const reduce = !!useReducedMotion();
  const glow = MOOD_GLOW[mood];
  const rx = useSpring(useMotionValue(0), { stiffness: 120, damping: 14 });
  const ry = useSpring(useMotionValue(0), { stiffness: 120, damping: 14 });

  useEffect(() => {
    let alive = true;
    void loadCutout(src).then((c) => {
      if (alive) setCut(c);
    });
    return () => {
      alive = false;
    };
  }, [src]);

  if (!cut) return null;

  const onMove = (e: PointerEvent<HTMLDivElement>) => {
    if (!parallax || reduce || crop === 'head') return;
    const b = e.currentTarget.getBoundingClientRect();
    const dx = (e.clientX - b.left) / b.width - 0.5;
    const dy = (e.clientY - b.top) / b.height - 0.5;
    ry.set(dx * 10);
    rx.set(-dy * 8);
  };
  const onLeave = () => {
    rx.set(0);
    ry.set(0);
  };

  if (crop === 'head') {
    // the head sits in the top ~40 % of the picture, centred horizontally
    const W = size * 2.15;
    const H = (W * cut.height) / cut.width;
    return (
      <span
        className={clsx('relative inline-block shrink-0 overflow-hidden rounded-full', className)}
        style={{ width: size, height: size }}
        data-testid="ubi-png"
        data-mood={mood}
        role="img"
        aria-label={`UBI, o mascote (${mood})`}
      >
        <img src={cut.url} alt="" draggable={false} className="absolute max-w-none select-none" style={{ width: W, height: H, left: size / 2 - W / 2, top: size / 2 - H * 0.27 }} />
      </span>
    );
  }

  const boxH = size * 1.2;
  const speed = mood === 'sleeping' ? 5.6 : mood === 'excited' ? 3.2 : 4.2;
  return (
    <div
      className={clsx('relative select-none', className)}
      style={{ width: size, height: boxH, perspective: 900 }}
      data-testid="ubi-png"
      data-mood={mood}
      role="img"
      aria-label={`UBI, o mascote (${mood})`}
      onPointerMove={onMove}
      onPointerLeave={onLeave}
    >
      {glowFloor && (
        <motion.span
          className="pointer-events-none absolute bottom-0 left-1/2 -translate-x-1/2 rounded-[50%]"
          style={{ width: size * 0.62, height: size * 0.11, background: `radial-gradient(closest-side, ${glow}, transparent)`, filter: 'blur(6px)', opacity: 0.7 }}
          animate={reduce ? undefined : { scaleX: [1, 0.86, 1], opacity: [0.7, 0.45, 0.7] }}
          transition={{ duration: speed, repeat: Infinity, ease: 'easeInOut' }}
          aria-hidden
        />
      )}
      <motion.div
        className="absolute inset-x-0 top-0"
        style={{ height: boxH - size * 0.06 }}
        animate={reduce ? undefined : { y: [0, -8, 0], rotate: [0, 1.2, 0, -1.2, 0] }}
        transition={{ duration: speed, repeat: Infinity, ease: 'easeInOut' }}
      >
        <motion.div className="relative flex h-full w-full items-end justify-center" style={{ rotateX: rx, rotateY: ry, transformStyle: 'preserve-3d' }}>
          <img
            src={cut.url}
            alt=""
            draggable={false}
            className="max-h-full max-w-full object-contain"
            style={{ filter: `drop-shadow(0 ${Math.round(size * 0.05)}px ${Math.round(size * 0.12)}px color-mix(in oklab, ${glow} 45%, transparent))` }}
          />
          <MoodOverlay mood={mood} reduce={reduce} />
        </motion.div>
      </motion.div>
    </div>
  );
}
