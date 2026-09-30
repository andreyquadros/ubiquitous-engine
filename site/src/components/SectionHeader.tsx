import type { ReactNode } from 'react';
import { cx } from './cx';

export function SectionHeader({ id, eyebrow, title, intro, align = 'left', className }: { id: string; eyebrow: string; title: string; intro?: ReactNode; align?: 'left' | 'center'; className?: string }) {
  return (
    <div className={cx('max-w-2xl', align === 'center' && 'mx-auto text-center', className)}>
      <p className="eyebrow">{eyebrow}</p>
      <h2 id={id} className="display mt-2 text-[28px] leading-[1.15] sm:text-[34px] lg:text-[40px]">{title}</h2>
      {intro && <p className="mt-4 text-[15px] leading-6 text-ink-2 sm:text-base sm:leading-7">{intro}</p>}
    </div>
  );
}
