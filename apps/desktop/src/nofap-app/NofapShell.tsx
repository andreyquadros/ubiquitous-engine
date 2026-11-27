// The standalone product's frame: a narrow rail (the module, its settings, the theme, the way out) and the
// scrolling content column. Nothing of the tracker: no day, timeline, review or report.

import clsx from 'clsx';
import { LogOut, Moon, Settings, ShieldCheck, Sun, type LucideIcon } from 'lucide-react';
import { useState } from 'react';
import { NavLink, Outlet } from 'react-router-dom';
import { Button, IconButton } from '../components/ui/Button';
import { Dialog } from '../components/ui/Dialog';
import { useT } from '../i18n';
import { useNofapStore } from '../nofap/store';
import type { NofapView, TierState } from '../nofap/types';
import { useAutostart } from './autostart';
import { hostIpc } from './hostIpc';
import { useHostStore } from './hostStore';

/**
 * A tier stays after the quit while it is in place on the machine (the engine's `system_applied`): this app's, or
 * the other app's (`in_use_by_other`), which quitting this one does not take away.
 */
const inPlace = (tier: TierState): boolean => tier.state === 'applied' || tier.state === 'update_pending' || tier.state === 'in_use_by_other';

/** Whether something stays on the system after the app quits: picks the quit sentence that is true. */
export const systemApplied = (view: NofapView | null): boolean => !!view && [view.status.hosts, view.status.resolver, view.status.policies].some(inPlace);

/**
 * The window's way out (design 6.3): never hidden and never delayed. It says first what keeps holding once the app is
 * closed; the primary button is "Manter aberto".
 */
export function QuitDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const t = useT();
  const view = useNofapStore((s) => s.view);
  const [autostart] = useAutostart();
  const [busy, setBusy] = useState(false);
  const quit = async () => {
    setBusy(true);
    try {
      await hostIpc.quit();
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={t('nofap_app.quit.title')}
      width="sm"
      footer={
        <>
          {/* First in the document, so the dialog puts the keyboard on it; shown last, where a primary button goes. */}
          <Button variant="primary" className="order-last" onClick={onClose} data-testid="quit-keep-open">
            {t('nofap_app.quit.keep_open')}
          </Button>
          <Button variant="ghost" loading={busy} onClick={() => void quit()} data-testid="quit-confirm">
            {t('nofap_app.quit.quit')}
          </Button>
        </>
      }
    >
      <p className="text-sm leading-6 text-ink-2" data-testid="quit-message">
        {t(systemApplied(view) ? 'nofap.quit.confirm_system' : 'nofap.quit.confirm_none')}
      </p>
      {view?.config.enabled && autostart && <p className="mt-2 text-xs leading-5 text-ink-3">{t('nofap_app.quit.autostart')}</p>}
    </Dialog>
  );
}

/** The product's mark: a shield in UBI's volt. */
function Mark() {
  return (
    <span className="flex size-8 shrink-0 items-center justify-center rounded-control border border-line-2 bg-panel-2" aria-hidden>
      <ShieldCheck className="size-[18px] text-volt" strokeWidth={1.75} />
    </span>
  );
}

const NAV: { to: string; key: string; Icon: LucideIcon }[] = [
  { to: '/', key: 'nofap_app.nav.protection', Icon: ShieldCheck },
  { to: '/settings', key: 'nofap_app.nav.settings', Icon: Settings },
];

export function NofapShell() {
  const t = useT();
  const name = useNofapStore((s) => s.view?.display_name);
  const theme = useHostStore((s) => s.theme);
  const toggleTheme = useHostStore((s) => s.toggleTheme);
  const [quitting, setQuitting] = useState(false);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-canvas">
      <aside className="flex h-full w-[200px] shrink-0 flex-col border-r border-line bg-panel/70 backdrop-blur-xl" aria-label={t('nofap_app.nav.main')}>
        <div data-tauri-drag-region className="h-[38px] shrink-0" />
        <div className="flex items-center gap-2.5 px-5 pt-1 pb-5">
          <Mark />
          {name && <span className="display truncate text-[17px]">{name}</span>}
        </div>
        <nav className="flex flex-1 flex-col gap-0.5 px-3">
          {NAV.map(({ to, key, Icon }) => (
            <NavLink
              key={to}
              to={to}
              end={to === '/'}
              className={({ isActive }) =>
                clsx('group relative flex h-10 items-center gap-3 rounded-control px-3 text-sm font-medium transition-colors duration-150', isActive ? 'bg-volt-soft text-ink' : 'text-ink-2 hover:bg-panel-2 hover:text-ink')
              }
            >
              {({ isActive }) => (
                <>
                  {isActive && <span className="absolute top-2 bottom-2 -left-3 w-[3px] rounded-r-full bg-volt shadow-[0_0_12px_2px_rgb(77_141_255/.55)]" aria-hidden />}
                  <Icon className={clsx('size-[18px] shrink-0', isActive ? 'text-volt' : 'text-ink-3 group-hover:text-ink-2')} strokeWidth={1.75} aria-hidden />
                  <span className="truncate">{t(key)}</span>
                </>
              )}
            </NavLink>
          ))}
        </nav>
        <div className="flex items-center justify-between gap-2 border-t border-line p-3">
          <IconButton label={theme === 'dark' ? t('nofap_app.nav.theme_light') : t('nofap_app.nav.theme_dark')} onClick={toggleTheme}>
            {theme === 'dark' ? <Sun className="size-4" strokeWidth={1.75} /> : <Moon className="size-4" strokeWidth={1.75} />}
          </IconButton>
          <Button variant="ghost" size="sm" icon={<LogOut className="size-4" strokeWidth={1.75} aria-hidden />} onClick={() => setQuitting(true)} data-testid="nav-quit">
            {t('nofap_app.nav.quit')}
          </Button>
        </div>
      </aside>
      <main className="relative flex min-w-0 flex-1 flex-col">
        <div data-tauri-drag-region className="h-[38px] shrink-0" />
        <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-6 pb-10">
          <div className="mx-auto w-full max-w-[1120px]">
            <Outlet />
          </div>
        </div>
      </main>
      <QuitDialog open={quitting} onClose={() => setQuitting(false)} />
    </div>
  );
}
