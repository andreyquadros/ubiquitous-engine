import { useCallback, useEffect, useMemo, useState } from 'react';
import { COPY, I18nContext, initialLocale, persistLocale, type Locale } from './i18n';
import { Nav } from './sections/Nav';
import { Hero } from './sections/Hero';
import { HowItWorks } from './sections/HowItWorks';
import { Features } from './sections/Features';
import { Screens } from './sections/Screens';
import { Pricing } from './sections/Pricing';
import { Downloads } from './sections/Downloads';
import { Faq } from './sections/Faq';
import { Footer } from './sections/Footer';
import { OgCard } from './sections/OgCard';
import { HeroCapture } from './sections/HeroCapture';

/** Special routes used by the image scripts: /og (or ?og=1) and ?capture=hero. */
function specialView(): 'og' | 'hero' | null {
  try {
    const url = new URL(window.location.href);
    const path = url.pathname.replace(/\/+$/, '');
    if (path.endsWith('/og') || url.searchParams.get('og') === '1') return 'og';
    if (url.searchParams.get('capture') === 'hero') return 'hero';
  } catch {
    /* ignore */
  }
  return null;
}

export default function App() {
  const [locale, setLocaleState] = useState<Locale>(() => initialLocale());
  const setLocale = useCallback((l: Locale) => setLocaleState(l), []);
  useEffect(() => persistLocale(locale), [locale]);
  const value = useMemo(() => ({ locale, copy: COPY[locale], setLocale }), [locale, setLocale]);
  const view = useMemo(specialView, []);

  return (
    <I18nContext.Provider value={value}>
      {view === 'og' ? (
        <OgCard />
      ) : view === 'hero' ? (
        <HeroCapture />
      ) : (
        <>
          <a
            href="#conteudo"
            className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[100] focus:rounded-control focus:bg-volt focus:px-4 focus:py-2 focus:text-on-volt"
          >
            {value.copy.nav.skip}
          </a>
          <Nav />
          <main id="conteudo">
            <Hero />
            <HowItWorks />
            <Features />
            <Screens />
            <Pricing />
            <Downloads />
            <Faq />
          </main>
          <Footer />
        </>
      )}
    </I18nContext.Provider>
  );
}
