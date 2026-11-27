import clsx from 'clsx';
import type { ReactNode } from 'react';
import { Progress } from './misc';

/** Colour of a spend bar by how much of the budget is gone: signal → amber (70 %) → rose (90 %). */
export const usageColor = (spent: number, budget: number): string => {
  const ratio = budget > 0 ? spent / budget : 0;
  return ratio >= 0.9 ? 'var(--rose)' : ratio >= 0.7 ? 'var(--amber)' : 'var(--signal)';
};

/**
 * A budget bar with its label and the "spent of budget" figure on one line. Used for the managed plan's monthly AI
 * allowance (Settings › Licença); `value` is the text to show on the right (already formatted).
 */
export function UsageBar({ label, value, spent, budget, hint, className }: { label: ReactNode; value: ReactNode; spent: number; budget: number; hint?: ReactNode; className?: string }) {
  return (
    <div className={clsx('flex flex-col gap-1.5', className)} data-testid="usage-bar">
      <div className="flex items-baseline justify-between gap-3">
        <span className="text-[13px] font-medium text-ink">{label}</span>
        <span className="num text-xs text-ink-2">{value}</span>
      </div>
      <Progress value={spent} max={budget} color={usageColor(spent, budget)} height="h-2" />
      {hint && <p className="text-xs leading-5 text-ink-3">{hint}</p>}
    </div>
  );
}
