import clsx from 'clsx';
import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes, TextareaHTMLAttributes } from 'react';
import { useId } from 'react';

const base =
  'w-full rounded-control border border-line-2 bg-panel-2 px-3 text-sm text-ink placeholder:text-ink-4 transition-[border-color,box-shadow] duration-150 focus:border-volt focus:outline-none focus:ring-2 focus:ring-volt/25 disabled:cursor-not-allowed disabled:opacity-50';

export function Input({ className, ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={clsx(base, 'h-9', className)} {...rest} />;
}

export function Textarea({ className, ...rest }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={clsx(base, 'min-h-20 py-2 leading-5', className)} {...rest} />;
}

export function Select({ className, children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select className={clsx(base, 'h-9 appearance-none bg-no-repeat pr-8', className)} style={{ backgroundImage: CHEVRON, backgroundPosition: 'right 10px center', backgroundSize: '14px' }} {...rest}>
      {children}
    </select>
  );
}

const CHEVRON =
  "url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='%237487a6' stroke-width='1.75' stroke-linecap='round' stroke-linejoin='round'><path d='m6 9 6 6 6-6'/></svg>\")";

interface FieldProps {
  label: ReactNode;
  hint?: ReactNode;
  children: (id: string) => ReactNode;
  className?: string;
  inline?: boolean;
}

/** Label + control + hint. `children` receives the generated id to bind the control. */
export function Field({ label, hint, children, className, inline }: FieldProps) {
  const id = useId();
  return (
    <div className={clsx(inline ? 'flex items-center justify-between gap-4' : 'flex flex-col gap-1.5', className)}>
      <div className={clsx(inline && 'min-w-0')}>
        <label htmlFor={id} className="text-[13px] font-medium text-ink">
          {label}
        </label>
        {hint && inline && <p className="text-xs text-ink-3">{hint}</p>}
      </div>
      {children(id)}
      {hint && !inline && <p className="text-xs text-ink-3">{hint}</p>}
    </div>
  );
}
