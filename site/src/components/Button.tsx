import type { AnchorHTMLAttributes, ButtonHTMLAttributes, ReactNode } from 'react';
import { cx } from './cx';

type Variant = 'primary' | 'secondary' | 'ghost';
type Size = 'md' | 'lg';

const base =
  'inline-flex items-center justify-center gap-2 rounded-control font-medium transition-colors duration-150 ease-[var(--ease-standard)] disabled:cursor-not-allowed disabled:opacity-60';
const variants: Record<Variant, string> = {
  primary: 'bg-volt text-on-volt glow-volt hover:bg-[#6aa0ff]',
  secondary: 'border border-line-2 bg-panel-2 text-ink hover:border-volt/60 hover:bg-panel-3',
  ghost: 'text-ink-2 hover:bg-panel-2 hover:text-ink',
};
const sizes: Record<Size, string> = {
  md: 'h-10 px-4 text-sm',
  lg: 'h-12 px-5 text-[15px]',
};

interface Common {
  variant?: Variant;
  size?: Size;
  icon?: ReactNode;
  className?: string;
  children: ReactNode;
}

export function ButtonLink({ variant = 'primary', size = 'md', icon, className, children, ...rest }: Common & AnchorHTMLAttributes<HTMLAnchorElement>) {
  return (
    <a className={cx(base, variants[variant], sizes[size], className)} {...rest}>
      {icon}
      {children}
    </a>
  );
}

export function Button({ variant = 'primary', size = 'md', icon, className, children, ...rest }: Common & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button type="button" className={cx(base, variants[variant], sizes[size], className)} {...rest}>
      {icon}
      {children}
    </button>
  );
}
