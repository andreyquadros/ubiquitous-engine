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
        'relative inline-flex shrink-0 items-center rounded-pill border transition-[background-color,border-color,box-shadow] duration-150 disabled:cursor-not-allowed disabled:opacity-50',
        w,
        checked ? 'border-volt bg-volt shadow-[0_0_12px_rgb(77_141_255/.35)]' : 'border-line-2 bg-panel-3',
      )}
    >
      <span className={clsx('inline-block rounded-full bg-white shadow-[0_1px_2px_rgb(0_0_0/.35)] transition-transform duration-150', knob, checked ? move : 'translate-x-0.5')} />
    </button>
  );
}
