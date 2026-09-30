import clsx from 'clsx';
import { Loader2 } from 'lucide-react';
import type { ButtonHTMLAttributes, ReactNode } from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'accent';
type Size = 'sm' | 'md' | 'lg';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** `primary` = volt fill with glow (one per view), `accent` = ember fill (attention), `danger` = rose outline. */
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  icon?: ReactNode;
}

const VARIANTS: Record<Variant, string> = {
  primary: 'bg-volt text-on-volt glow-volt hover:brightness-110 disabled:shadow-none',
  accent: 'bg-ember text-on-ember hover:brightness-110',
  secondary: 'border border-line-2 bg-panel text-ink hover:bg-panel-2',
  ghost: 'text-ink-2 hover:bg-panel-2 hover:text-ink',
  danger: 'border border-rose/40 bg-rose/10 text-rose hover:bg-rose/15',
};

const SIZES: Record<Size, string> = {
  sm: 'h-8 px-2.5 text-xs gap-1.5 rounded-[8px]',
  md: 'h-9 px-3.5 text-sm gap-2 rounded-control',
  lg: 'h-11 px-5 text-sm gap-2 rounded-control',
};

export function Button({ variant = 'secondary', size = 'md', loading, icon, className, children, disabled, ...rest }: Props) {
  return (
    <button
      type="button"
      className={clsx(
        'inline-flex items-center justify-center font-medium whitespace-nowrap transition-[background-color,color,box-shadow,filter] duration-150 select-none disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:brightness-100',
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      disabled={disabled || loading}
      {...rest}
    >
      {loading ? <Loader2 className="size-4 animate-spin" strokeWidth={1.75} /> : icon}
      {children}
    </button>
  );
}

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  size?: Size;
  /** Toggled state (e.g. an active filter). */
  active?: boolean;
}

export function IconButton({ label, size = 'md', active, className, children, ...rest }: IconButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={active}
      title={label}
      className={clsx(
        'inline-flex items-center justify-center rounded-[8px] transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-50',
        active ? 'bg-volt-soft text-volt' : 'text-ink-2 hover:bg-panel-2 hover:text-ink',
        size === 'sm' ? 'size-7' : size === 'lg' ? 'size-11' : 'size-8',
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
