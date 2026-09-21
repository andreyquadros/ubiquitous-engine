import { LogoMark } from '../components/LogoMark';
import { useCopy } from '../i18n';

const HERO_PNG = `${import.meta.env.BASE_URL}ubi-hero.png`;

/** 1200x630 composition for the Open Graph image (rendered by scripts/images.mjs at /og). */
export function OgCard() {
  const { copy } = useCopy();
  return (
    <div className="relative flex h-[630px] w-[1200px] overflow-hidden bg-canvas text-ink" data-testid="og-card">
      <div className="hairline-grid pointer-events-none absolute inset-0" aria-hidden="true" />
      <div className="pointer-events-none absolute inset-0" style={{ background: 'radial-gradient(60% 80% at 25% 40%, rgb(77 141 255 / 0.2), transparent 70%)' }} aria-hidden="true" />
      <div className="relative flex w-[700px] flex-col justify-center pl-20">
        <span className="flex items-center gap-3">
          <LogoMark size={44} />
          <span className="display text-[28px]">
            ubiq<span className="text-volt">X</span> <span className="font-medium text-ink-2">AI</span>
          </span>
        </span>
        <h1 className="display mt-8 text-[64px] leading-[1.05]">{copy.hero.title}</h1>
        <p className="mt-6 text-[22px] leading-8 text-ink-2">{copy.hero.trust}</p>
      </div>
      <div className="relative flex flex-1 items-end justify-center pb-6">
        <span className="absolute bottom-6 left-1/2 h-10 w-[300px] -translate-x-1/2 rounded-[50%] opacity-70 blur-xl" style={{ background: 'radial-gradient(closest-side, #4d8dff, transparent)' }} aria-hidden="true" />
        <img src={HERO_PNG} alt="" width={440} height={528} className="relative" />
      </div>
    </div>
  );
}
