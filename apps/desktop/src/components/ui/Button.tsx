import clsx from 'clsx';
import { Loader2 } from 'lucide-react';
import type { ButtonHTMLAttributes, ReactNode } from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'accent';
type Size = 'sm' | 'md' | 'lg';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  icon?: ReactNode;
}

const VARIANTS: Record<Variant, string> = {
  primary: 'bg-brand-600 text-white hover:bg-brand-700 shadow-sm disabled:bg-brand-600/60',
  accent: 'bg-accent-500 text-white hover:bg-accent-600 shadow-sm',
  secondary: 'border border-line-strong bg-surface text-ink hover:bg-surface-2',
  ghost: 'text-ink-2 hover:bg-surface-2 hover:text-ink',
  danger: 'border border-red-200 bg-red-50 text-red-700 hover:bg-red-100 dark:border-red-900/50 dark:bg-red-950/40 dark:text-red-300 dark:hover:bg-red-950/60',
};

const SIZES: Record<Size, string> = {
  sm: 'h-8 px-2.5 text-xs gap-1.5 rounded-lg',
  md: 'h-9 px-3.5 text-sm gap-2 rounded-xl',
  lg: 'h-11 px-5 text-sm gap-2 rounded-xl',
};

export function Button({ variant = 'secondary', size = 'md', loading, icon, className, children, disabled, ...rest }: Props) {
  return (
    <button
      type="button"
      className={clsx(
        'inline-flex items-center justify-center font-medium transition-colors select-none disabled:cursor-not-allowed disabled:opacity-60',
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      disabled={disabled || loading}
      {...rest}
    >
      {loading ? <Loader2 className="size-4 animate-spin" /> : icon}
      {children}
    </button>
  );
}

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  size?: Size;
}

export function IconButton({ label, size = 'md', className, children, ...rest }: IconButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={clsx(
        'inline-flex items-center justify-center rounded-lg text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink disabled:opacity-50',
        size === 'sm' ? 'size-7' : size === 'lg' ? 'size-10' : 'size-8',
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
