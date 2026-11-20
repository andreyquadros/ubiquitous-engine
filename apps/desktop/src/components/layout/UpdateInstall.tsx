import { AlertTriangle, Download, RefreshCw } from 'lucide-react';
import { useState } from 'react';
import { useT } from '../../i18n';
import { intlLocale } from '../../lib/format';
import { useAppStore } from '../../lib/store';
import { fmtBytes, installRatio, isInstalling } from '../../lib/updater';
import { Button } from '../ui/Button';

/**
 * The in-app update, shared by the top banner and Settings → Atualizações: one button that downloads the
 * signed build, installs it and reopens the app (`lib/updater.ts` over tauri-plugin-updater), plus the
 * progress bar and the outcome. The manual installer stays one click away in both places, for the builds
 * the plugin cannot install and for whoever prefers it.
 *
 * On macOS the button asks for one confirmation first: CI signs the bundle ad hoc, so the updated app is a
 * different app to the system and the permissions have to be granted again (docs/MACOS-TESTING.md § 3.2).
 */
export function InstallUpdateButton({ size = 'md', macWarning }: { size?: 'sm' | 'md'; macWarning: boolean }) {
  const t = useT();
  const install = useAppStore((s) => s.updateInstall);
  const installUpdate = useAppStore((s) => s.installUpdate);
  const [armed, setArmed] = useState(false);

  const busy = isInstalling(install);
  const start = () => {
    setArmed(false);
    void installUpdate();
  };

  if (armed && !busy) {
    return (
      <div className="flex flex-wrap items-center gap-2" data-testid="update-macos-warning">
        <p className="flex min-w-0 basis-full items-start gap-2 text-xs leading-5 text-ink-2">
          <AlertTriangle className="mt-0.5 size-4 shrink-0 text-ember" strokeWidth={1.75} aria-hidden />
          {t('updates.macos.warning')}
        </p>
        <Button size={size} variant="ghost" onClick={() => setArmed(false)}>
          {t('updates.macos.cancel')}
        </Button>
        <Button size={size} variant="primary" onClick={start}>
          {t('updates.macos.confirm')}
        </Button>
      </div>
    );
  }

  return (
    <Button
      size={size}
      variant="primary"
      loading={busy}
      icon={<Download className={size === 'sm' ? 'size-3.5' : 'size-4'} strokeWidth={2} aria-hidden />}
      onClick={() => (macWarning ? setArmed(true) : start())}
      data-testid="update-install"
    >
      {t('updates.install_now')}
    </Button>
  );
}

/** Progress bar, status line and outcome of the in-app install. Renders nothing while it has not started. */
export function UpdateInstallStatus({ className }: { className?: string }) {
  const t = useT();
  const install = useAppStore((s) => s.updateInstall);
  const installUpdate = useAppStore((s) => s.installUpdate);

  if (install.phase === 'idle') return null;

  if (install.phase === 'failed' || install.phase === 'unavailable') {
    const message = install.phase === 'failed' ? t('updates.install.failed', { error: install.error ?? '' }) : t('updates.install.unavailable');
    return (
      <div className={className} data-testid="update-install-error">
        <p className="flex items-start gap-2 rounded-control border border-rose/40 bg-rose/10 px-3 py-2 text-xs leading-5 text-rose" role="alert">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" strokeWidth={1.75} aria-hidden />
          <span className="min-w-0">{message}</span>
        </p>
        {install.phase === 'failed' && (
          <Button size="sm" className="mt-2" icon={<RefreshCw className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void installUpdate()}>
            {t('updates.install.retry')}
          </Button>
        )}
      </div>
    );
  }

  const ratio = installRatio(install);
  const locale = intlLocale();
  const label =
    install.phase === 'checking'
      ? t('updates.install.checking')
      : install.phase === 'installing'
        ? t('updates.install.installing')
        : install.phase === 'relaunching'
          ? t('updates.install.relaunching')
          : install.total !== null
            ? t('updates.install.downloading', { done: fmtBytes(install.downloaded, locale), total: fmtBytes(install.total, locale) })
            : t('updates.install.downloading_unknown', { done: fmtBytes(install.downloaded, locale) });

  return (
    <div className={className} data-testid="update-install-progress">
      <div
        className="h-1.5 w-full overflow-hidden rounded-full bg-panel-2"
        role="progressbar"
        aria-label={t('updates.install.progress_label')}
        aria-valuemin={0}
        aria-valuemax={100}
        {...(ratio === null ? {} : { 'aria-valuenow': Math.round(ratio * 100) })}
      >
        <div className={ratio === null ? 'h-full w-1/3 animate-pulse rounded-full bg-volt' : 'h-full rounded-full bg-volt transition-[width] duration-200'} style={ratio === null ? undefined : { width: `${Math.round(ratio * 100)}%` }} />
      </div>
      <p className="num mt-1.5 text-xs text-ink-2" aria-live="polite">
        {label}
      </p>
    </div>
  );
}
