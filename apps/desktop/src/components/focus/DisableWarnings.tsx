import { useEffect, useState } from 'react';
import { useT } from '../../i18n';
import type { FocusTarget } from '../../lib/types';
import { Ubi } from '../ubi/Ubi';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';

export type PendingAction = 'disable' | 'remove';

export interface PendingChange {
  target: FocusTarget;
  action: PendingAction;
}

/** A target held in the last 15 minutes gets the three warnings instead of one confirmation. */
export const RECENT_BLOCK_MS = 15 * 60_000;
export const wasBlockedRecently = (target: FocusTarget, now = Date.now()): boolean => !!target.last_blocked_at && now - new Date(target.last_blocked_at).getTime() < RECENT_BLOCK_MS;

const TOTAL_STEPS = 3;

interface Props {
  pending: PendingChange | null;
  /** Keep the block (also Escape and the backdrop). */
  onClose: () => void;
  /** Only after the last step (or the single confirmation). */
  onConfirm: (pending: PendingChange) => void;
}

/**
 * Disabling or removing a target: one confirmation normally; when it was blocked in the last 15 minutes, UBI gives three
 * warnings in a row ("don't give up on your dreams") and only the third "Disable anyway" goes through.
 */
export function DisableWarnings({ pending, onClose, onConfirm }: Props) {
  const t = useT();
  const [step, setStep] = useState(1);
  const recent = pending ? wasBlockedRecently(pending.target) : false;

  useEffect(() => {
    if (pending) setStep(1);
  }, [pending]);

  const name = pending?.target.name ?? '';
  const isRemove = pending?.action === 'remove';
  const anyway = isRemove ? t('focus.warnings.remove_anyway') : t('focus.warnings.disable_anyway');
  const confirmLabel = isRemove ? t('focus.confirm.remove') : t('focus.confirm.disable');

  if (recent) {
    const last = step >= TOTAL_STEPS;
    return (
      <Dialog
        open={!!pending}
        onClose={onClose}
        title={t('focus.warnings.title')}
        description={t('focus.warnings.step', { step, total: TOTAL_STEPS })}
        width="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => (last && pending ? onConfirm(pending) : setStep((s) => s + 1))} data-testid="focus-warning-continue">
              {last ? anyway : t('focus.warnings.continue')}
            </Button>
            <Button variant="primary" onClick={onClose}>
              {t('focus.warnings.keep')}
            </Button>
          </>
        }
      >
        <div className="flex flex-col items-center py-2" data-testid={`focus-warning-step-${step}`}>
          <Ubi variant="flat" size={96} mood="worried" speaking={t(`focus.warnings.${step}`)} />
          <p className="mt-3 text-center text-xs text-ink-3">{name}</p>
        </div>
      </Dialog>
    );
  }

  return (
    <Dialog
      open={!!pending}
      onClose={onClose}
      title={isRemove ? t('focus.confirm.remove_title', { name }) : t('focus.confirm.disable_title', { name })}
      width="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button variant={isRemove ? 'danger' : 'secondary'} onClick={() => pending && onConfirm(pending)} data-testid="focus-confirm">
            {confirmLabel}
          </Button>
        </>
      }
    >
      <p className="text-sm text-ink-2">{t('focus.confirm.body')}</p>
    </Dialog>
  );
}
