import { Download, Lock } from 'lucide-react';
import { useEffect, useState } from 'react';
import { ButtonLink } from '../components/Button';
import { detectOs, DEFAULT_OS, type Os } from '../components/os';
import { Particles } from '../components/Particles';
import { UbiHero } from '../components/UbiHero';
import { DOWNLOAD_URLS } from '../config';
import { useCopy } from '../i18n';

export function Hero() {
  const { copy } = useCopy();
  const h = copy.hero;
  // detected after mount so the first paint is identical for every visitor (and for the capture scripts)
  const [os, setOs] = useState<Os>(DEFAULT_OS);
  useEffect(() => setOs(detectOs()), []);
  const cta = DOWNLOAD_URLS[os] || '#baixar';
  return (
    <section className="relative overflow-hidden" aria-labelledby="hero-title">
      <div className="hairline-grid pointer-events-none absolute inset-0" aria-hidden="true" />
      <div className="pointer-events-none absolute inset-0" style={{ background: 'var(--hero-glow)' }} aria-hidden="true" />
      <div className="container-site relative grid items-center gap-10 py-14 sm:py-20 lg:grid-cols-12 lg:gap-8 lg:py-24">
        <div className="min-w-0 lg:col-span-7">
          <p className="eyebrow">{h.eyebrow}</p>
          <h1 id="hero-title" className="display mt-3 text-[34px] leading-[1.08] sm:text-[46px] lg:text-[56px]">
            {h.title}
          </h1>
          <p className="mt-5 max-w-xl text-[15px] leading-6 text-ink-2 sm:text-[17px] sm:leading-7">{h.subtitle}</p>
          <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:items-center">
            <ButtonLink href={cta} size="lg" className="neon-cta" icon={<Download size={18} strokeWidth={1.75} aria-hidden="true" />}>
              {copy.downloadFor[os]}
            </ButtonLink>
            <ButtonLink href="#planos" size="lg" variant="secondary">
              {h.secondary}
            </ButtonLink>
          </div>
          <p className="mt-4 text-sm text-ink-3">{h.platforms}</p>
          <p className="mt-6 inline-flex items-center gap-2 rounded-pill border border-line bg-panel/70 px-3 py-1.5 text-sm text-ink-2">
            <Lock size={14} strokeWidth={1.75} className="text-signal" aria-hidden="true" />
            {h.trust}
          </p>
        </div>
        <div className="flex min-w-0 justify-center lg:col-span-5 lg:justify-end">
          <div className="relative">
            <Particles className="pointer-events-none absolute -inset-x-10 -top-8 bottom-0 h-[calc(100%+2rem)] w-[calc(100%+5rem)]" />
            <UbiHero size={400} className="relative w-[280px] sm:w-[360px] lg:w-[400px]" />
          </div>
        </div>
      </div>
    </section>
  );
}
