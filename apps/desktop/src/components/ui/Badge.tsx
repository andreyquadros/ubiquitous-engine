import clsx from 'clsx';
import type { ReactNode } from 'react';

/** Semantic tones; `brand`/`accent`/`success`/`warning`/`danger` are aliases kept for existing pages. */
type Tone = 'neutral' | 'volt' | 'ember' | 'signal' | 'rose' | 'violet' | 'amber' | 'brand' | 'accent' | 'success' | 'warning' | 'danger';

const TONES: Record<Tone, string> = {
  neutral: 'bg-panel-2 text-ink-2 border-line',
  volt: 'bg-volt/12 text-volt border-volt/30',
  ember: 'bg-ember/12 text-ember border-ember/30',
  signal: 'bg-signal/12 text-signal border-signal/30',
  rose: 'bg-rose/12 text-rose border-rose/30',
  violet: 'bg-violet/12 text-violet border-violet/30',
  amber: 'bg-amber/12 text-amber border-amber/30',
  brand: 'bg-volt/12 text-volt border-volt/30',
  accent: 'bg-ember/12 text-ember border-ember/30',
  success: 'bg-signal/12 text-signal border-signal/30',
  warning: 'bg-amber/12 text-amber border-amber/30',
  danger: 'bg-rose/12 text-rose border-rose/30',
};

export function Badge({ tone = 'neutral', children, className, title }: { tone?: Tone; children: ReactNode; className?: string; title?: string }) {
  return (
    <span title={title} className={clsx('inline-flex h-5 items-center gap-1 rounded-md border px-1.5 text-[11px] font-medium whitespace-nowrap', TONES[tone], className)}>
      {children}
    </span>
  );
}

/** Coloured status dot + text. `pulse` adds a slow breathing halo (live states only). */
export function StatusPill({ color, children, className, pulse }: { color: string; children: ReactNode; className?: string; pulse?: boolean }) {
  return (
    <span className={clsx('inline-flex items-center gap-1.5 rounded-pill border border-line bg-panel px-2.5 py-1 text-xs font-medium text-ink-2', className)}>
      <span className="relative flex size-2">
        {pulse && <span className="absolute inset-0 animate-ping rounded-full opacity-40 motion-reduce:hidden" style={{ background: color, animationDuration: '2.4s' }} />}
        <span className="size-2 rounded-full" style={{ background: color, boxShadow: `0 0 0 3px color-mix(in oklab, ${color} 18%, transparent)` }} />
      </span>
      {children}
    </span>
  );
}
