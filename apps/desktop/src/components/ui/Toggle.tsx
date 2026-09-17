import clsx from 'clsx';

interface Props {
  checked: boolean;
  onChange: (v: boolean) => void;
  id?: string;
  disabled?: boolean;
  label?: string;
  size?: 'sm' | 'md';
}

export function Toggle({ checked, onChange, id, disabled, label, size = 'md' }: Props) {
  const w = size === 'sm' ? 'h-5 w-9' : 'h-6 w-11';
  const knob = size === 'sm' ? 'size-4' : 'size-5';
  const move = size === 'sm' ? 'translate-x-4' : 'translate-x-5';
  return (
    <button
      id={id}
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={clsx(
        'relative inline-flex shrink-0 items-center rounded-full border border-transparent transition-colors disabled:opacity-50',
        w,
        checked ? 'bg-brand-600' : 'bg-surface-3',
      )}
    >
      <span className={clsx('inline-block rounded-full bg-white shadow-sm transition-transform', knob, checked ? move : 'translate-x-0.5')} />
    </button>
  );
}
