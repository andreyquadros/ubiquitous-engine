import { useEffect } from 'react';
import { UbiHero } from '../components/UbiHero';

/** Renders the still 3D mascot alone on a transparent page (scripts/images.mjs turns it into public/ubi-hero.png). */
export function HeroCapture() {
  useEffect(() => {
    // the page paints the canvas colour by default; the capture needs real transparency around the mascot
    for (const node of [document.documentElement, document.body]) node.style.background = 'transparent';
  }, []);
  return (
    <div className="flex min-h-screen items-center justify-center" data-testid="hero-capture">
      <UbiHero size={440} capture />
    </div>
  );
}
