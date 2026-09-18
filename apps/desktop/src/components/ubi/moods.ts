import type { Mood } from '../../lib/types';
import { t } from '../../i18n';
import { MOODS } from '../../lib/format';

/** Glow colour per mood: volt by default, UBI's sash ember when excited, rose when worried. */
export const MOOD_GLOW: Record<Mood, string> = {
  sleeping: '#7487a6',
  calm: '#4d8dff',
  focused: '#2ee6a6',
  excited: '#ff7a1f',
  worried: '#ff5c7a',
};

/** UBI's line for a mood, in the current language (`t('ubi.tip.<mood>')`). */
export const moodTip = (mood: Mood): string => t(`ubi.tip.${mood}`);

/**
 * Live map kept for existing callers (`MOOD_TIP[mood]` follows the current locale on every read).
 * Inside components prefer `const t = useT(); t(`ubi.tip.${mood}`)` so they re-render on a language change.
 */
export const MOOD_TIP: Record<Mood, string> = (() => {
  const o = {} as Record<Mood, string>;
  for (const m of MOODS) Object.defineProperty(o, m, { get: () => moodTip(m), enumerable: true });
  return o;
})();
