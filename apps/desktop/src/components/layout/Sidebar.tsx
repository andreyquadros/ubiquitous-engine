import clsx from 'clsx';
import { Clock, FileText, LayoutDashboard, ListChecks, Moon, Settings, Sparkles, Sun, Tags, type LucideIcon } from 'lucide-react';
import { NavLink } from 'react-router-dom';
import { useAppStore } from '../../lib/store';
import { IconButton } from '../ui/Button';
import { TrackerPill } from './TrackerPill';

interface Item {
  to: string;
  label: string;
  Icon: LucideIcon;
  badge?: number;
}

export function Sidebar({ needsReview }: { needsReview: number }) {
  const theme = useAppStore((s) => s.theme);
  const toggleTheme = useAppStore((s) => s.toggleTheme);

  const items: Item[] = [
    { to: '/', label: 'Hoje', Icon: LayoutDashboard },
    { to: '/timeline', label: 'Timeline', Icon: Clock },
    { to: '/review', label: 'Revisão', Icon: ListChecks, badge: needsReview },
    { to: '/reports', label: 'Relatórios', Icon: FileText },
    { to: '/categories', label: 'Categorias', Icon: Tags },
    { to: '/insights', label: 'Insights', Icon: Sparkles },
    { to: '/settings', label: 'Configurações', Icon: Settings },
  ];

  return (
    <aside className="flex h-full w-[72px] shrink-0 flex-col border-r border-line bg-surface/70 backdrop-blur min-[1180px]:w-[220px]" aria-label="Navegação principal">
      {/* macOS overlay title bar: draggable strip; traffic lights live in the top-left 80px */}
      <div data-tauri-drag-region className="h-[38px] shrink-0" />
      <div className="flex items-center gap-2 px-4 pt-1 pb-4 min-[1180px]:px-5">
        <span className="flex size-8 items-center justify-center rounded-xl bg-brand-600 text-white shadow-sm" aria-hidden>
          <svg viewBox="0 0 64 64" className="size-5">
            <polygon points="22,20 27,10 32,20" fill="#fff" />
            <polygon points="29,19 34,7 39,19" fill="#fff" />
            <polygon points="36,20 41,10 46,20" fill="#fff" />
            <rect x="12" y="18" width="40" height="34" rx="13" fill="#fff" />
            <rect x="17" y="24" width="30" height="22" rx="10" fill="#0f172a" />
            <path d="M22 38 Q26 31 30 38" stroke="#60a5fa" strokeWidth="2.5" fill="none" strokeLinecap="round" />
            <path d="M34 38 Q38 31 42 38" stroke="#60a5fa" strokeWidth="2.5" fill="none" strokeLinecap="round" />
          </svg>
        </span>
        <span className="hidden text-base font-semibold tracking-tight min-[1180px]:inline">
          ubiq<span className="text-brand-600">X</span>
        </span>
      </div>

      <nav className="flex flex-1 flex-col gap-0.5 px-3">
        {items.map(({ to, label, Icon, badge }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            title={label}
            className={({ isActive }) =>
              clsx(
                'group relative flex h-10 items-center gap-3 rounded-xl px-2.5 text-sm font-medium transition-colors min-[1180px]:px-3',
                isActive ? 'bg-brand-50 text-brand-700 dark:bg-brand-900/30 dark:text-brand-300' : 'text-ink-2 hover:bg-surface-2 hover:text-ink',
              )
            }
          >
            <Icon className="size-[18px] shrink-0" />
            <span className="hidden truncate min-[1180px]:inline">{label}</span>
            {badge ? (
              <span className="absolute top-1.5 right-1.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-accent-500 px-1 text-[10px] font-semibold text-white min-[1180px]:static min-[1180px]:ml-auto">
                {badge}
              </span>
            ) : null}
          </NavLink>
        ))}
      </nav>

      <div className="flex flex-col gap-2 border-t border-line p-3">
        <IconButton label={theme === 'dark' ? 'Tema claro' : 'Tema escuro'} onClick={toggleTheme} className="self-start">
          {theme === 'dark' ? <Sun className="size-4" /> : <Moon className="size-4" />}
        </IconButton>
        <TrackerPill />
      </div>
    </aside>
  );
}
