import clsx from 'clsx';
import type { HTMLAttributes, ReactNode } from 'react';

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  padded?: boolean;
}

export function Card({ padded = true, className, children, ...rest }: CardProps) {
  return (
    <div className={clsx('card', padded && 'p-5', className)} {...rest}>
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
        <h2 className="text-sm font-semibold tracking-tight text-ink">{title}</h2>
        {subtitle && <p className="mt-0.5 text-xs text-ink-3">{subtitle}</p>}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
}

export function StatTile({ label, value, hint, accent, className }: { label: string; value: ReactNode; hint?: ReactNode; accent?: string; className?: string }) {
  return (
    <div className={clsx('card flex flex-col gap-1 p-4', className)}>
      <span className="text-[11px] font-medium tracking-wide text-ink-3 uppercase">{label}</span>
      <span className="text-xl font-semibold tracking-tight tabular-nums" style={accent ? { color: accent } : undefined}>
        {value}
      </span>
      {hint && <span className="text-xs text-ink-3">{hint}</span>}
    </div>
  );
}
