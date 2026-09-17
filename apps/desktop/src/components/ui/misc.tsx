import clsx from 'clsx';
import { Loader2 } from 'lucide-react';
import type { ReactNode } from 'react';

export function Progress({ value, max = 1, color, className, height = 'h-1.5' }: { value: number; max?: number; color?: string; className?: string; height?: string }) {
  const pct = Math.max(0, Math.min(1, max ? value / max : 0));
  return (
    <div className={clsx('w-full overflow-hidden rounded-full bg-surface-3', height, className)} role="progressbar" aria-valuenow={Math.round(pct * 100)} aria-valuemin={0} aria-valuemax={100}>
      <div className="h-full rounded-full transition-[width] duration-500" style={{ width: `${pct * 100}%`, background: color ?? 'var(--color-brand-600)' }} />
    </div>
  );
}

export function Spinner({ className }: { className?: string }) {
  return <Loader2 className={clsx('size-5 animate-spin text-ink-3', className)} aria-label="Carregando" />;
}

export function EmptyState({ icon, title, description, action, className }: { icon?: ReactNode; title: string; description?: ReactNode; action?: ReactNode; className?: string }) {
  return (
    <div className={clsx('flex flex-col items-center justify-center gap-2 py-10 text-center', className)}>
      {icon && <div className="mb-1 flex size-11 items-center justify-center rounded-2xl bg-surface-2 text-ink-3">{icon}</div>}
      <p className="text-sm font-medium">{title}</p>
      {description && <p className="max-w-sm text-xs leading-5 text-ink-3">{description}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

export function Tabs<T extends string>({ value, onChange, items, className }: { value: T; onChange: (v: T) => void; items: { value: T; label: string }[]; className?: string }) {
  return (
    <div role="tablist" className={clsx('inline-flex items-center gap-0.5 rounded-xl bg-surface-2 p-1', className)}>
      {items.map((it) => (
        <button
          key={it.value}
          role="tab"
          type="button"
          aria-selected={value === it.value}
          onClick={() => onChange(it.value)}
          className={clsx(
            'h-8 rounded-lg px-3 text-sm font-medium transition-colors',
            value === it.value ? 'bg-surface text-ink shadow-sm' : 'text-ink-2 hover:text-ink',
          )}
        >
          {it.label}
        </button>
      ))}
    </div>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd>{children}</kbd>;
}

export function Skeleton({ className }: { className?: string }) {
  return <div className={clsx('animate-pulse rounded-lg bg-surface-3', className)} />;
}

export function SectionTitle({ children, description, action }: { children: ReactNode; description?: ReactNode; action?: ReactNode }) {
  return (
    <div className="mb-3 flex items-end justify-between gap-3">
      <div>
        <h3 className="text-sm font-semibold tracking-tight">{children}</h3>
        {description && <p className="mt-0.5 text-xs text-ink-3">{description}</p>}
      </div>
      {action}
    </div>
  );
}
