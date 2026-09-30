import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { ArrowDownToLine, X } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useT } from '../../i18n';
import { fmtDateTime } from '../../lib/format';
import { useAppStore } from '../../lib/store';
import { useToast } from '../../lib/toast';
import type { AssetKind, UpdateStatus } from '../../lib/types';
import { losesMacPermissions } from '../../lib/updater';
import { Button } from '../ui/Button';
import { InstallUpdateButton, UpdateInstallStatus } from './UpdateInstall';

/** Settings section the "how to install" link jumps to (`id="sec-atualizacoes"` in pages/Settings.tsx). */
export const UPDATES_SECTION_ID = 'atualizacoes';

/** pt "18/09 15:04" / en "09/18 15:04" for a build epoch (unix seconds); 0 = development build. */
export const fmtBuildDate = (epoch: number, devLabel: string): string => (epoch > 0 ? fmtDateTime(new Date(epoch * 1000).toISOString()) : devLabel);

/** Label of the download button for an installer kind: "Baixar (.dmg)", "Baixar (.exe)", "Baixar (.AppImage)"… (keys `updates.download.<kind>`). */
export const downloadLabel = (t: (key: string) => string, kind: AssetKind): string => t(`updates.download.${kind}`);

/** True when the banner and the Settings dot should show: an update the user has not dismissed, on a CI build. */
export const hasPendingUpdate = (status: UpdateStatus | null): status is UpdateStatus & { available: NonNullable<UpdateStatus['available']> } =>
  !!status && status.enabled && status.available !== null && !status.dismissed;

/**
 * Compact strip above every page: "new build available", update in place (download, install and reopen, with a
 * progress bar), or take the manual route — download the installer of this OS (the button names its extension),
 * dismiss, or jump to the install notes. Hidden when there is no update, when the user dismissed this build, or
 * on development builds.
 */
export function UpdateBanner() {
  const t = useT();
  const status = useAppStore((s) => s.updateStatus);
  const dismissUpdate = useAppStore((s) => s.dismissUpdate);
  const openUpdate = useAppStore((s) => s.openUpdate);
  const navigate = useNavigate();
  const toast = useToast();
  const reduce = useReducedMotion();

  const show = hasPendingUpdate(status);
  const release = show ? status.available : null;

  const download = () => {
    openUpdate().catch((e: unknown) => toast.error(t('updates.toast.open_failed'), e instanceof Error ? e.message : String(e)));
  };
  const howTo = () => navigate('/settings', { state: { section: UPDATES_SECTION_ID } });

  return (
    <AnimatePresence initial={false}>
      {release && (
        <motion.div
          key={release.build.epoch}
          role="status"
          data-testid="update-banner"
          initial={reduce ? { opacity: 0 } : { opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={reduce ? { opacity: 0 } : { opacity: 0, y: -8 }}
          transition={{ duration: reduce ? 0 : 0.18, ease: [0.16, 1, 0.3, 1] }}
          className="mb-4 flex flex-wrap items-center gap-x-4 gap-y-2 rounded-card border border-volt/30 bg-volt-soft px-4 py-2"
        >
          <ArrowDownToLine className="size-4 shrink-0 text-volt" strokeWidth={1.75} aria-hidden />
          <p className="min-w-0 flex-1 basis-64 text-sm leading-5 text-ink">
            {t('updates.banner.title', { date: fmtBuildDate(release.build.epoch, t('updates.dev_build')), sha: release.build.sha })}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <button type="button" onClick={howTo} className="mr-2 text-xs font-medium text-volt underline-offset-4 hover:underline">
              {t('updates.how_to_install')}
            </button>
            <Button size="sm" variant="ghost" icon={<X className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void dismissUpdate()}>
              {t('updates.later')}
            </Button>
            <Button size="sm" icon={<ArrowDownToLine className="size-3.5" strokeWidth={2} aria-hidden />} onClick={download}>
              {downloadLabel(t, release.kind)}
            </Button>
            <InstallUpdateButton size="sm" macWarning={losesMacPermissions(release.kind)} />
          </div>
          <UpdateInstallStatus className="basis-full" />
        </motion.div>
      )}
    </AnimatePresence>
  );
}
