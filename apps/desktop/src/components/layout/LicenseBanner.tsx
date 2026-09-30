import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { BadgeCheck, Lock, X } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useT } from '../../i18n';
import { ipc, ipcErrorMessage } from '../../lib/ipc';
import { SITE_URL, licenseBlocksAi } from '../../lib/providers';
import { useAppStore } from '../../lib/store';
import { useToast } from '../../lib/toast';
import type { LicenseStatus } from '../../lib/types';
import { Button, IconButton } from '../ui/Button';
import { Dialog } from '../ui/Dialog';

/** Settings section the "Já tenho uma chave" button jumps to (`id="sec-licenca"` in pages/Settings.tsx). */
export const LICENSE_SECTION_ID = 'licenca';

/** localStorage key holding the unix ms of the last dismissal; the reminder stays away for `LICENSE_NAG_DAYS` after it. */
export const LICENSE_NAG_KEY = 'ubiqx.license_nag';
export const LICENSE_NAG_DAYS = 7;
const NAG_MS = LICENSE_NAG_DAYS * 24 * 60 * 60 * 1000;

/** When the reminder was last dismissed (unix ms), or null. Storage failures read as "never". */
export function readNagDismissedAt(): number | null {
  try {
    const v = Number(localStorage.getItem(LICENSE_NAG_KEY));
    return Number.isFinite(v) && v > 0 ? v : null;
  } catch {
    return null;
  }
}

/** True while a dismissal younger than 7 days is stored. */
export const nagDismissedRecently = (now = Date.now()): boolean => {
  const at = readNagDismissedAt();
  return at !== null && now - at < NAG_MS && at <= now;
};

/**
 * Whether the reminder line should show: no valid license, and either hard enforcement (the line never goes away) or
 * no dismissal in the last 7 days. `null` (license unknown yet) shows nothing.
 */
export function shouldNag(license: LicenseStatus | null, dismissedAt: number | null, now = Date.now()): boolean {
  if (!license || license.state === 'valid') return false;
  if (licenseBlocksAi(license)) return true;
  return dismissedAt === null || now - dismissedAt >= NAG_MS || dismissedAt > now;
}

/**
 * Soft-enforcement reminder above every page: "O ubiqX está sem licença…", the plans and the two ways in. Dismissable
 * for 7 days (localStorage). Under hard enforcement it turns into a persistent line explaining that the AI is off.
 */
export function LicenseBanner() {
  const t = useT();
  const license = useAppStore((s) => s.license);
  const navigate = useNavigate();
  const toast = useToast();
  const reduce = useReducedMotion();
  const [dismissedAt, setDismissedAt] = useState<number | null>(readNagDismissedAt);

  const show = shouldNag(license, dismissedAt);
  const blocked = licenseBlocksAi(license);

  const dismiss = () => {
    const now = Date.now();
    try {
      localStorage.setItem(LICENSE_NAG_KEY, String(now));
    } catch {
      // private window or blocked storage: the line simply comes back on the next mount
    }
    setDismissedAt(now);
  };
  const seePlans = () => {
    ipc.openExternal(SITE_URL).catch((e: unknown) => toast.error(t('common.license.toast.open_failed'), ipcErrorMessage(e)));
  };
  const haveKey = () => navigate('/settings', { state: { section: LICENSE_SECTION_ID } });

  const text = blocked ? t('common.license.banner.blocked') : license?.state === 'expired' ? t('common.license.banner.expired') : t('common.license.banner.unlicensed');

  return (
    <AnimatePresence initial={false}>
      {show && license && (
        <motion.div
          key="license-banner"
          role="status"
          data-testid="license-banner"
          data-mode={blocked ? 'hard' : 'soft'}
          initial={reduce ? { opacity: 0 } : { opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={reduce ? { opacity: 0 } : { opacity: 0, y: -8 }}
          transition={{ duration: reduce ? 0 : 0.18, ease: [0.16, 1, 0.3, 1] }}
          className={blocked ? 'mb-4 flex flex-wrap items-center gap-x-4 gap-y-2 rounded-card border border-amber/40 bg-amber/10 px-4 py-2' : 'mb-4 flex flex-wrap items-center gap-x-4 gap-y-2 rounded-card border border-line bg-panel px-4 py-2'}
        >
          {blocked ? <Lock className="size-4 shrink-0 text-amber" strokeWidth={1.75} aria-hidden /> : <BadgeCheck className="size-4 shrink-0 text-volt" strokeWidth={1.75} aria-hidden />}
          <p className="num min-w-0 flex-1 basis-64 text-sm leading-5 text-ink">{text}</p>
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm" variant="ghost" onClick={haveKey}>
              {t('common.license.have_key')}
            </Button>
            <Button size="sm" variant="primary" onClick={seePlans}>
              {t('common.license.see_plans')}
            </Button>
            {!blocked && (
              <IconButton size="sm" label={t('common.license.dismiss')} onClick={dismiss}>
                <X className="size-3.5" strokeWidth={1.75} />
              </IconButton>
            )}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

/**
 * Hard-enforcement dialog: shown by `useLicenseGate()` in place of an AI action while the license blocks the AI.
 * Never opens under the (current) soft enforcement, since `licenseBlocksAi` is false there.
 */
export function LicenseRequiredDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const t = useT();
  const navigate = useNavigate();
  const toast = useToast();
  const seePlans = () => {
    ipc.openExternal(SITE_URL).catch((e: unknown) => toast.error(t('common.license.toast.open_failed'), ipcErrorMessage(e)));
  };
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={t('common.license.required_title')}
      width="sm"
      footer={
        <>
          <Button
            variant="ghost"
            onClick={() => {
              onClose();
              navigate('/settings', { state: { section: LICENSE_SECTION_ID } });
            }}
          >
            {t('common.license.have_key')}
          </Button>
          <Button variant="primary" onClick={seePlans}>
            {t('common.license.see_plans')}
          </Button>
        </>
      }
    >
      <p className="num text-sm leading-6 text-ink-2">{t('common.license.required_body')}</p>
    </Dialog>
  );
}

/**
 * Wraps an AI action with the license policy: under hard enforcement without a valid license the action is replaced
 * by the explanatory dialog; otherwise it runs. `const { guard, dialog } = useLicenseGate(); <button onClick={guard(run)} />{dialog}`.
 */
export function useLicenseGate() {
  const license = useAppStore((s) => s.license);
  const [open, setOpen] = useState(false);
  const blocked = licenseBlocksAi(license);
  const guard = useCallback(
    <A extends unknown[]>(action: (...args: A) => void) =>
      (...args: A) => {
        if (blocked) setOpen(true);
        else action(...args);
      },
    [blocked],
  );
  const close = useCallback(() => setOpen(false), []);
  return { blocked, guard, dialog: <LicenseRequiredDialog open={open} onClose={close} /> };
}
