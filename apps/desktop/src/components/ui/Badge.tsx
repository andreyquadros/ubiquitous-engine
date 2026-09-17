import clsx from 'clsx';
import type { ReactNode } from 'react';

type Tone = 'neutral' | 'brand' | 'accent' | 'success' | 'warning' | 'danger' | 'violet';

const TONES: Record<Tone, string> = {
  neutral: 'bg-surface-2 text-ink-2 border-line',
  brand: 'bg-brand-50 text-brand-700 border-brand-100 dark:bg-brand-900/30 dark:text-brand-300 dark:border-brand-900/60',
  accent: 'bg-accent-50 text-accent-700 border-accent-100 dark:bg-accent-900/20 dark:text-accent-300 dark:border-accent-700/40',
  success: 'bg-emerald-50 text-emerald-700 border-emerald-100 dark:bg-emerald-900/30 dark:text-emerald-300 dark:border-emerald-900/60',
  warning: 'bg-amber-50 text-amber-700 border-amber-100 dark:bg-amber-900/30 dark:text-amber-300 dark:border-amber-900/60',
  danger: 'bg-red-50 text-red-700 border-red-100 dark:bg-red-900/30 dark:text-red-300 dark:border-red-900/60',
  violet: 'bg-violet-50 text-violet-700 border-violet-100 dark:bg-violet-900/30 dark:text-violet-300 dark:border-violet-900/60',
};

export function Badge({ tone = 'neutral', children, className, title }: { tone?: Tone; children: ReactNode; className?: string; title?: string }) {
  return (
    <span title={title} className={clsx('inline-flex h-5 items-center gap-1 rounded-md border px-1.5 text-[11px] font-medium whitespace-nowrap', TONES[tone], className)}>
      {children}
    </span>
  );
}

/** Coloured status dot + text. */
export function StatusPill({ color, children, className }: { color: string; children: ReactNode; className?: string }) {
  return (
    <span className={clsx('inline-flex items-center gap-1.5 rounded-full border border-line bg-surface px-2.5 py-1 text-xs font-medium text-ink-2', className)}>
      <span className="size-2 rounded-full" style={{ background: color, boxShadow: `0 0 0 3px ${color}22` }} />
      {children}
    </span>
  );
}
