import { Download, Menu, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { ButtonLink } from '../components/Button';
import { cx } from '../components/cx';
import { LangToggle } from '../components/LangToggle';
import { LogoMark } from '../components/LogoMark';
import { DOWNLOAD_URLS } from '../config';
import { useCopy } from '../i18n';

const LINKS: Array<['how' | 'features' | 'screens' | 'pricing' | 'download' | 'faq', string]> = [
  ['how', '#como-funciona'],
  ['features', '#recursos'],
  ['screens', '#telas'],
  ['pricing', '#planos'],
  ['download', '#baixar'],
  ['faq', '#perguntas'],
];

export function Nav() {
  const { copy } = useCopy();
  const [open, setOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const on = () => setScrolled(window.scrollY > 8);
    on();
    window.addEventListener('scroll', on, { passive: true });
    return () => window.removeEventListener('scroll', on);
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);

  const cta = DOWNLOAD_URLS.macos || '#baixar';

  return (
    <header className={cx('sticky top-0 z-50 transition-colors', scrolled || open ? 'border-b border-line bg-canvas/85 backdrop-blur-md' : 'bg-transparent')}>
      <nav className="container-site flex h-16 items-center justify-between gap-4" aria-label={copy.nav.main}>
        <a href="#" className="flex items-center gap-2.5 rounded-control" aria-label={copy.nav.brand}>
          <LogoMark size={30} />
          <span className="display text-[17px]">
            ubiq<span className="text-volt">X</span> <span className="font-medium text-ink-2">AI</span>
          </span>
        </a>
        <ul className="hidden items-center gap-1 lg:flex">
          {LINKS.map(([k, href]) => (
            <li key={k}>
              <a href={href} className="rounded-control px-3 py-2 text-sm text-ink-2 transition-colors hover:bg-panel-2 hover:text-ink">
                {copy.nav[k]}
              </a>
            </li>
          ))}
        </ul>
        <div className="hidden items-center gap-3 lg:flex">
          <LangToggle />
          <ButtonLink href={cta} icon={<Download size={16} strokeWidth={1.75} aria-hidden="true" />}>
            {copy.nav.cta}
          </ButtonLink>
        </div>
        <div className="flex items-center gap-2 lg:hidden">
          <LangToggle />
          <button
            type="button"
            className="inline-flex h-9 w-9 items-center justify-center rounded-control border border-line-2 bg-panel-2 text-ink"
            aria-expanded={open}
            aria-controls="menu-mobile"
            aria-label={open ? copy.nav.close : copy.nav.menu}
            onClick={() => setOpen((v) => !v)}
          >
            {open ? <X size={18} strokeWidth={1.75} aria-hidden="true" /> : <Menu size={18} strokeWidth={1.75} aria-hidden="true" />}
          </button>
        </div>
      </nav>
      {open && (
        <div id="menu-mobile" className="container-site border-t border-line pb-4 lg:hidden">
          <ul className="flex flex-col py-2">
            {LINKS.map(([k, href]) => (
              <li key={k}>
                <a href={href} onClick={() => setOpen(false)} className="block rounded-control px-3 py-2.5 text-[15px] text-ink-2 hover:bg-panel-2 hover:text-ink">
                  {copy.nav[k]}
                </a>
              </li>
            ))}
          </ul>
          <ButtonLink href={cta} className="w-full" size="lg" icon={<Download size={16} strokeWidth={1.75} aria-hidden="true" />} onClick={() => setOpen(false)}>
            {copy.nav.cta}
          </ButtonLink>
        </div>
      )}
    </header>
  );
}
