import clsx from 'clsx';
import { AppWindow, Globe, X } from 'lucide-react';
import { useT } from '../../i18n';
import { fmtRelative } from '../../lib/format';
import type { FocusTarget } from '../../lib/types';
import { IconButton } from '../ui/Button';
import { Toggle } from '../ui/Toggle';

interface Props {
  targets: FocusTarget[];
  onToggle: (target: FocusTarget, enabled: boolean) => void;
  onRemove: (target: FocusTarget) => void;
}

/** The block list: kind icon, name, key in mono, how often it was held and when, a Toggle and a remove button. */
export function TargetList({ targets, onToggle, onRemove }: Props) {
  const t = useT();
  return (
    <ul className="divide-y divide-line" data-testid="focus-targets">
      {targets.map((x) => {
        const Icon = x.kind === 'app' ? AppWindow : Globe;
        return (
          <li key={x.id} className={clsx('flex items-center gap-3 py-2.5', !x.enabled && 'opacity-60')} data-testid={`focus-target-${x.id}`}>
            <span className="flex size-8 shrink-0 items-center justify-center rounded-control border border-line bg-panel-2 text-ink-3" title={x.kind === 'app' ? t('focus.targets.kind_app') : t('focus.targets.kind_site')}>
              <Icon className="size-4" strokeWidth={1.75} aria-hidden />
            </span>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm font-medium text-ink">{x.name}</span>
                {x.key !== x.name && <span className="truncate font-mono text-[11px] text-ink-3">{x.key}</span>}
              </div>
              <p className="num mt-0.5 text-[11px] text-ink-3">
                {x.blocked_count > 0 ? t('focus.targets.blocked_count', { count: x.blocked_count }) : t('focus.targets.never_blocked')}
                {x.last_blocked_at && <span>, {t('focus.targets.last_blocked', { when: fmtRelative(x.last_blocked_at) })}</span>}
              </p>
            </div>
            <Toggle size="sm" checked={x.enabled} onChange={(v) => onToggle(x, v)} label={t('focus.targets.toggle', { name: x.name })} />
            <IconButton size="sm" label={t('focus.targets.remove', { name: x.name })} onClick={() => onRemove(x)}>
              <X className="size-4" strokeWidth={1.75} />
            </IconButton>
          </li>
        );
      })}
    </ul>
  );
}
