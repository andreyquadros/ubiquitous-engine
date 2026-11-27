import { useT } from '../../i18n';
import { REMOVAL_URL } from '../labels';
import type { Os } from '../types';

/**
 * "Como remover sem o app": what to undo by hand, where each thing is on this system, and the full address of the
 * removal guide as text that can be selected and copied (neither app ships the guide).
 */
export function RemovalHelp({ os }: { os: Os | null }) {
  const t = useT();
  return (
    <div className="flex flex-col gap-1.5 text-[13px] leading-5 text-ink-2">
      <p>
        {t('nofap.helptab.remove.body')}
        {os ? ` ${t(`nofap.helptab.remove.where.${os}`)}` : ''}
      </p>
      <p>
        {t('nofap.helptab.remove.doc')} <span className="select-all break-all font-mono text-xs text-ink">{REMOVAL_URL}</span>
      </p>
    </div>
  );
}
