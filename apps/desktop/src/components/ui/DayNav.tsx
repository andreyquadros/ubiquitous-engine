import { CalendarDays, ChevronLeft, ChevronRight } from 'lucide-react';
import { useRef } from 'react';
import { fmtDateNumeric, fmtWeekday, isToday, shiftDate, todayIso } from '../../lib/format';
import type { IsoDate } from '../../lib/types';
import { IconButton } from './Button';
import clsx from 'clsx';
import { useT } from '../../i18n';

/** Prev / next day, a locale-formatted date that opens the native picker, and a "Hoje" / "Today" shortcut. */
export function DayNav({ date, onChange, className }: { date: IsoDate; onChange: (d: IsoDate) => void; className?: string }) {
  const input = useRef<HTMLInputElement>(null);
  const t = useT();
  const openPicker = () => {
    const el = input.current;
    if (!el) return;
    if ('showPicker' in el && typeof el.showPicker === 'function') {
      try {
        el.showPicker();
        return;
      } catch {
        /* fall through */
      }
    }
    el.focus();
    el.click();
  };
  return (
    <div className={clsx('inline-flex h-9 items-center gap-0.5 rounded-control border border-line bg-panel p-0.5', className)}>
      <IconButton label={t('ui.previous_day')} size="sm" onClick={() => onChange(shiftDate(date, -1))}>
        <ChevronLeft className="size-4" strokeWidth={1.75} />
      </IconButton>
      <div className="relative">
        <button type="button" onClick={openPicker} className="num flex h-7 items-center gap-1.5 rounded-[8px] px-2 text-sm font-medium hover:bg-panel-2" aria-label={t('ui.pick_date', { date: fmtDateNumeric(date) })}>
          <CalendarDays className="size-3.5 text-ink-3" strokeWidth={1.75} />
          <span className="text-ink-3">{fmtWeekday(date)}</span>
          {fmtDateNumeric(date)}
        </button>
        <input
          ref={input}
          type="date"
          tabIndex={-1}
          aria-hidden
          value={date}
          max={todayIso()}
          onChange={(e) => e.target.value && onChange(e.target.value)}
          className="pointer-events-none absolute inset-0 h-full w-full opacity-0"
        />
      </div>
      <IconButton label={t('ui.next_day')} size="sm" disabled={isToday(date)} onClick={() => onChange(shiftDate(date, 1))}>
        <ChevronRight className="size-4" strokeWidth={1.75} />
      </IconButton>
      <button
        type="button"
        onClick={() => onChange(todayIso())}
        disabled={isToday(date)}
        className="h-7 rounded-[8px] px-2 text-xs font-medium text-volt transition-colors hover:bg-volt-soft disabled:text-ink-4 disabled:hover:bg-transparent"
      >
        {t('ui.today')}
      </button>
    </div>
  );
}
