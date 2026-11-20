import type { ReactNode } from 'react';

/** Page title in Sora, an optional one-sentence subtitle and right-aligned actions. */
export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: ReactNode; actions?: ReactNode }) {
  return (
    <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div className="min-w-0">
        <h1 className="display text-[26px] leading-8">{title}</h1>
        {subtitle && <p className="mt-1 text-sm text-ink-2 first-letter:uppercase">{subtitle}</p>}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </header>
  );
}
