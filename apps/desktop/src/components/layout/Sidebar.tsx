import clsx from 'clsx';
import { Clock, FileText, LayoutDashboard, ListChecks, Moon, Settings, Sparkles, Sun, Tags, type LucideIcon } from 'lucide-react';
import { NavLink } from 'react-router-dom';
import { useAppStore } from '../../lib/store';
import { useT } from '../../i18n';
import { IconButton } from '../ui/Button';
import { TrackerPill } from './TrackerPill';
import { hasPendingUpdate } from './UpdateBanner';

interface Item {
  to: string;
  label: string;
  Icon: LucideIcon;
  badge?: number;
  /** A quiet volt dot (an update is waiting in Settings). */
  dot?: boolean;
}

/** UBI's head as a mark: white shell, black visor, two volt crescents. */
export function BrandMark({ className }: { className?: string }) {
  return (
    <span className={clsx('flex size-8 items-center justify-center rounded-control border border-line-2 bg-panel-2', className)} aria-hidden>
      <svg viewBox="0 0 32 32" className="size-6">
        <defs>
          <linearGradient id="bm-visor" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#1b2540" />
            <stop offset="1" stopColor="#05070f" />
          </linearGradient>
        </defs>
        <path d="M11 9 L13 3.5 L15 9 Z M14.5 8.5 L16 2 L17.5 8.5 Z M17 9 L19 3.5 L21 9 Z" fill="#f4f7ff" />
        <path d="M13.2 8.4 L13.2 5.5 M16 7.6 L16 4 M18.8 8.4 L18.8 5.5" stroke="#4d8dff" strokeWidth="1" strokeLinecap="round" />
        <rect x="4" y="8" width="24" height="20" rx="8" fill="#f4f7ff" />
        <rect x="6.5" y="11" width="19" height="13" rx="6.5" fill="url(#bm-visor)" />
        <path d="M10 19.5 Q12.5 15.5 15 19.5" stroke="#4d8dff" strokeWidth="1.8" fill="none" strokeLinecap="round" />
        <path d="M17 19.5 Q19.5 15.5 22 19.5" stroke="#4d8dff" strokeWidth="1.8" fill="none" strokeLinecap="round" />
      </svg>
    </span>
  );
}

export function Sidebar({ needsReview }: { needsReview: number }) {
  const theme = useAppStore((s) => s.theme);
  const toggleTheme = useAppStore((s) => s.toggleTheme);
  const updatePending = useAppStore((s) => hasPendingUpdate(s.updateStatus));
  const t = useT();

  const items: Item[] = [
    { to: '/', label: t('nav.today'), Icon: LayoutDashboard },
    { to: '/timeline', label: t('nav.timeline'), Icon: Clock },
    { to: '/review', label: t('nav.review'), Icon: ListChecks, badge: needsReview },
    { to: '/reports', label: t('nav.reports'), Icon: FileText },
    { to: '/categories', label: t('nav.categories'), Icon: Tags },
    { to: '/insights', label: t('nav.insights'), Icon: Sparkles },
    { to: '/settings', label: t('nav.settings'), Icon: Settings, dot: updatePending },
  ];

  return (
    <aside
      className="flex h-full w-[72px] shrink-0 flex-col border-r border-line bg-panel/70 backdrop-blur-xl min-[1180px]:w-[232px]"
      aria-label={t('nav.main_navigation')}
    >
      {/* macOS overlay title bar: draggable strip; traffic lights live in the top-left 80px */}
      <div data-tauri-drag-region className="h-[38px] shrink-0" />
      <div className="flex items-center gap-2.5 px-4 pt-1 pb-5 min-[1180px]:px-5">
        <BrandMark />
        <span className="display hidden text-[17px] min-[1180px]:inline">
          ubiq<span className="text-volt">X</span>
        </span>
      </div>

      <nav className="flex flex-1 flex-col gap-0.5 px-3">
        {items.map(({ to, label, Icon, badge, dot }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            title={label}
            className={({ isActive }) =>
              clsx(
                'group relative flex h-10 items-center gap-3 rounded-control px-2.5 text-sm font-medium transition-colors duration-150 min-[1180px]:px-3',
                isActive ? 'bg-volt-soft text-ink' : 'text-ink-2 hover:bg-panel-2 hover:text-ink',
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && <span className="absolute top-2 bottom-2 -left-3 w-[3px] rounded-r-full bg-volt shadow-[0_0_12px_2px_rgb(77_141_255/.55)]" aria-hidden />}
                <Icon className={clsx('size-[18px] shrink-0', isActive ? 'text-volt' : 'text-ink-3 group-hover:text-ink-2')} strokeWidth={1.75} aria-hidden />
                <span className="hidden truncate min-[1180px]:inline">{label}</span>
                {badge ? (
                  <span
                    className="num absolute top-1.5 right-1.5 flex h-[18px] min-w-[18px] items-center justify-center rounded-pill bg-ember px-1.5 text-[11px] font-semibold text-on-ember min-[1180px]:static min-[1180px]:ml-auto"
                    aria-label={t('nav.to_review', { count: badge })}
                  >
                    {badge}
                  </span>
                ) : null}
                {dot && !badge ? (
                  <span className="absolute top-2.5 right-2.5 size-1.5 rounded-full bg-volt min-[1180px]:static min-[1180px]:ml-auto" data-testid="nav-update-dot">
                    <span className="sr-only">{t('updates.banner.badge')}</span>
                  </span>
                ) : null}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="flex flex-col gap-2 border-t border-line p-3">
        <IconButton label={theme === 'dark' ? t('nav.theme_light') : t('nav.theme_dark')} onClick={toggleTheme} className="self-start">
          {theme === 'dark' ? <Sun className="size-4" strokeWidth={1.75} /> : <Moon className="size-4" strokeWidth={1.75} />}
        </IconButton>
        <TrackerPill />
      </div>
    </aside>
  );
}
