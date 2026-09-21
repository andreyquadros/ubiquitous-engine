// Drifting motes behind the mascot: a cheap 2D canvas (no three, no extra chunk) that reads as "the app is
// quietly working in the background". Pauses off screen and with a hidden tab, and never runs under reduced
// motion — the field is decorative.
import { useEffect, useRef } from 'react';
import { prefersReducedMotion } from './webgl';

const COUNT = 46;
const COLOR = '77, 141, 255';
/** Motes rise this many canvas heights per second, and the pointer pushes them this far sideways. */
const RISE = 0.05;
const PARALLAX = 14;

interface Mote {
  x: number;
  y: number;
  r: number;
  speed: number;
  drift: number;
  phase: number;
  alpha: number;
}

const make = (): Mote => ({
  x: Math.random(),
  y: Math.random(),
  r: 0.7 + Math.random() * 1.9,
  speed: 0.55 + Math.random() * 1.1,
  drift: 0.25 + Math.random() * 0.8,
  phase: Math.random() * Math.PI * 2,
  alpha: 0.18 + Math.random() * 0.42,
});

export function Particles({ className }: { className?: string }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas || prefersReducedMotion()) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let disposed = false;
    let raf = 0;
    let active = true;
    let visible = document.visibilityState !== 'hidden';
    let w = 1;
    let h = 1;
    let last = 0;
    const pointer = { x: 0.5, y: 0.5 };
    const motes = Array.from({ length: COUNT }, make);

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = canvas.clientWidth || 1;
      h = canvas.clientHeight || 1;
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);

    const onMove = (e: PointerEvent) => {
      const box = canvas.getBoundingClientRect();
      pointer.x = (e.clientX - box.left) / Math.max(box.width, 1);
      pointer.y = (e.clientY - box.top) / Math.max(box.height, 1);
    };
    window.addEventListener('pointermove', onMove, { passive: true });

    const frame = (now: number) => {
      raf = 0;
      if (disposed) return;
      const dt = Math.min((now - (last || now)) / 1000, 0.05);
      last = now;
      ctx.clearRect(0, 0, w, h);
      for (const m of motes) {
        m.y -= RISE * m.speed * dt;
        m.phase += dt * m.drift;
        if (m.y < -0.05) {
          m.y = 1.05;
          m.x = Math.random();
        }
        // sideways sway plus a gentle pull towards the pointer, so the field feels aware of the cursor
        const sway = Math.sin(m.phase) * 10;
        const pull = (pointer.x - 0.5) * PARALLAX * m.drift;
        const x = m.x * w + sway + pull;
        const y = m.y * h;
        const twinkle = 0.75 + Math.sin(m.phase * 2.1) * 0.25;
        ctx.beginPath();
        ctx.arc(x, y, m.r, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${COLOR}, ${m.alpha * twinkle})`;
        ctx.fill();
      }
      if (active && visible) raf = requestAnimationFrame(frame);
    };
    const kick = () => {
      if (!raf && !disposed) {
        last = 0;
        raf = requestAnimationFrame(frame);
      }
    };

    const io = new IntersectionObserver((entries) => {
      active = entries.some((e) => e.isIntersecting);
      if (active) kick();
    });
    io.observe(canvas);
    const onVis = () => {
      visible = document.visibilityState !== 'hidden';
      if (visible) kick();
    };
    document.addEventListener('visibilitychange', onVis);
    kick();

    return () => {
      disposed = true;
      if (raf) cancelAnimationFrame(raf);
      ro.disconnect();
      io.disconnect();
      window.removeEventListener('pointermove', onMove);
      document.removeEventListener('visibilitychange', onVis);
    };
  }, []);

  return <canvas ref={ref} className={className} aria-hidden="true" />;
}
