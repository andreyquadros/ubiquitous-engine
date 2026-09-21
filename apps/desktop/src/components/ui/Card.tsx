import clsx from 'clsx';
import type { HTMLAttributes, ReactNode } from 'react';

type Tone = 'panel' | 'raised' | 'plain';

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  padded?: boolean;
  /** `panel` (default, hairline card), `raised` (nested panel-2 surface), `plain` (no border/background). */
  tone?: Tone;
}

const TONES: Record<Tone, string> = { panel: 'panel', raised: 'panel-raised', plain: 'rounded-card' };

export function Card({ padded = true, tone = 'panel', className, children, ...rest }: CardProps) {
  return (
    <div className={clsx(TONES[tone], padded && 'p-5', className)} {...rest}>
      {children}
    </div>
  );
}

interface CardHeaderProps {
  title: ReactNode;
  subtitle?: ReactNode;
  action?: ReactNode;
  className?: string;
}

export function CardHeader({ title, subtitle, action, className }: CardHeaderProps) {
  return (
    <div className={clsx('mb-4 flex items-start justify-between gap-3', className)}>
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-ink">{title}</h2>
        {subtitle && <p className="mt-0.5 text-xs text-ink-3">{subtitle}</p>}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
}

interface StatTileProps {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
  /** Colour for the value (a semantic token such as `var(--signal)` or a category colour). */
  accent?: string;
  icon?: ReactNode;
  className?: string;
}

/** A quiet number: sentence-case label, Sora value, optional hint. No caps, no shadow. */
export function StatTile({ label, value, hint, accent, icon, className }: StatTileProps) {
  return (
    <div className={clsx('panel flex flex-col gap-1 p-4', className)}>
      <span className="flex items-center gap-1.5 text-xs font-medium text-ink-3">
        {icon && (
          <span className="text-ink-4" aria-hidden>
            {icon}
          </span>
        )}
        {label}
      </span>
      <span className="display num text-[22px] leading-7" style={accent ? { color: accent } : undefined}>
        {value}
      </span>
      {hint && <span className="text-xs text-ink-3">{hint}</span>}
    </div>
  );
}
