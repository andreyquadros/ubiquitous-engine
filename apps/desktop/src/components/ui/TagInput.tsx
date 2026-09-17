import { X } from 'lucide-react';
import { useState, type KeyboardEvent } from 'react';
import clsx from 'clsx';

interface Props {
  value: string[];
  onChange: (v: string[]) => void;
  placeholder?: string;
  id?: string;
  disabled?: boolean;
  className?: string;
}

export function TagInput({ value, onChange, placeholder = 'Adicionar…', id, disabled, className }: Props) {
  const [draft, setDraft] = useState('');

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
        'flex min-h-9 flex-wrap items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-2 py-1.5 focus-within:border-brand-500 focus-within:ring-2 focus-within:ring-brand-500/25',
        disabled && 'opacity-60',
        className,
      )}
    >
      {value.map((tag) => (
        <span key={tag} className="inline-flex h-6 items-center gap-1 rounded-md bg-surface-2 pr-1 pl-2 text-xs text-ink">
          {tag}
          <button
            type="button"
            aria-label={`Remover ${tag}`}
            className="rounded p-0.5 text-ink-3 hover:bg-surface-3 hover:text-ink"
            disabled={disabled}
            onClick={() => onChange(value.filter((t) => t !== tag))}
          >
            <X className="size-3" />
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
        placeholder={value.length ? '' : placeholder}
        className="h-6 min-w-24 flex-1 bg-transparent text-sm outline-none placeholder:text-ink-3"
      />
    </div>
  );
}
