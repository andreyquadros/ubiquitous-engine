import { X } from 'lucide-react';
import { useState, type KeyboardEvent } from 'react';
import clsx from 'clsx';
import { useT } from '../../i18n';

interface Props {
  value: string[];
  onChange: (v: string[]) => void;
  placeholder?: string;
  id?: string;
  disabled?: boolean;
  className?: string;
}

export function TagInput({ value, onChange, placeholder, id, disabled, className }: Props) {
  const [draft, setDraft] = useState('');
  const t = useT();
  const hint = placeholder ?? t('ui.tag_placeholder');

  const commit = () => {
    const parts = draft
      .split(/[,\n]/)
      .map((s) => s.trim())
      .filter(Boolean);
    if (parts.length) onChange([...new Set([...value, ...parts])]);
    setDraft('');
  };

  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Backspace' && !draft && value.length) {
      onChange(value.slice(0, -1));
    }
  };

  return (
    <div
      className={clsx(
        'flex min-h-9 flex-wrap items-center gap-1.5 rounded-control border border-line-2 bg-panel-2 px-2 py-1.5 transition-[border-color,box-shadow] duration-150 focus-within:border-volt focus-within:ring-2 focus-within:ring-volt/25',
        disabled && 'opacity-50',
        className,
      )}
    >
      {value.map((tag) => (
        <span key={tag} className="inline-flex h-6 items-center gap-1 rounded-md border border-line bg-panel pr-1 pl-2 text-xs text-ink">
          {tag}
          <button
            type="button"
            aria-label={t('ui.remove_tag', { tag })}
            className="rounded p-0.5 text-ink-3 hover:bg-panel-3 hover:text-ink"
            disabled={disabled}
            onClick={() => onChange(value.filter((t) => t !== tag))}
          >
            <X className="size-3" strokeWidth={1.75} />
          </button>
        </span>
      ))}
      <input
        id={id}
        value={draft}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={onKey}
        onBlur={commit}
        placeholder={value.length ? '' : hint}
        className="h-6 min-w-24 flex-1 bg-transparent text-sm outline-none placeholder:text-ink-4"
      />
    </div>
  );
}
