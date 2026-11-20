import { useEffect, useState } from 'react';
import { Button } from '../../components/ui/Button';
import { Field, Input } from '../../components/ui/Field';
import { useLocale, useT, type Locale } from '../../i18n';
import { delayVariant, fmtDelay, fmtMoney } from '../labels';
import type { VisionPrefs } from '../types';

/** The ceilings of the AI check and the range each may take (design 4.3). */
export const CAP_LIMITS = {
  max_per_hour: { min: 1, max: 60, step: 1 },
  max_per_day: { min: 1, max: 1000, step: 1 },
  monthly_cap_usd: { min: 0.25, max: 20, step: 0.25 },
} as const;

type CapKey = keyof typeof CAP_LIMITS;
const KEYS: CapKey[] = ['max_per_hour', 'max_per_day', 'monthly_cap_usd'];
export type Caps = Pick<VisionPrefs, CapKey>;

/** A typed value as a number inside the range of `key`; `null` when it is not one. Both decimal marks are read. */
function parse(key: CapKey, text: string): number | null {
  const value = Number(text.trim().replace(',', '.'));
  const { min, max } = CAP_LIMITS[key];
  if (!text.trim() || !Number.isFinite(value) || value < min || value > max) return null;
  return key === 'monthly_cap_usd' ? Math.round(value * 100) / 100 : Number.isInteger(value) ? value : null;
}

/** The values in force, as the fields show them. */
const stored = (caps: Caps, locale: Locale): Record<CapKey, string> => ({ max_per_hour: String(caps.max_per_hour), max_per_day: String(caps.max_per_day), monthly_cap_usd: fmtMoney(caps.monthly_cap_usd, locale) });

interface Props {
  caps: Caps;
  delaySecs: number;
  /** Sends the three values together. Raising applies at once; a lowered value waits like any loosening. */
  onSave: (caps: Caps) => void;
}

/**
 * How many checks an hour and a day, and how much a month, the AI check may spend. Fewer checks is less
 * protection, so lowering a ceiling waits out the delay; the note says so before the user types.
 */
export function VisionCaps({ caps, delaySecs, onSave }: Props) {
  const t = useT();
  const locale = useLocale();
  const [text, setText] = useState<Record<CapKey, string>>(() => stored(caps, locale));

  // A change that applied (or one made elsewhere) shows up in the fields.
  useEffect(() => {
    setText(stored({ max_per_hour: caps.max_per_hour, max_per_day: caps.max_per_day, monthly_cap_usd: caps.monthly_cap_usd }, locale));
  }, [caps.max_per_hour, caps.max_per_day, caps.monthly_cap_usd, locale]);

  const parsed = { max_per_hour: parse('max_per_hour', text.max_per_hour), max_per_day: parse('max_per_day', text.max_per_day), monthly_cap_usd: parse('monthly_cap_usd', text.monthly_cap_usd) };
  const valid = KEYS.every((key) => parsed[key] !== null);
  const changed = KEYS.some((key) => parsed[key] !== caps[key]);

  const save = () => {
    if (parsed.max_per_hour === null || parsed.max_per_day === null || parsed.monthly_cap_usd === null) return;
    onSave({ max_per_hour: parsed.max_per_hour, max_per_day: parsed.max_per_day, monthly_cap_usd: parsed.monthly_cap_usd });
    // The fields go back to what is in force: a lowered value waits (the banner says so), a raised one comes back
    // with the fresh state.
    setText(stored(caps, locale));
  };

  return (
    <div className="flex flex-col gap-3" data-testid="vision-caps">
      <h3 className="text-[13px] font-medium text-ink">{t('nofap.caps.title')}</h3>
      <div className="grid grid-cols-1 gap-3 min-[720px]:grid-cols-3">
        {KEYS.map((key) => {
          const { min, max } = CAP_LIMITS[key];
          const money = key === 'monthly_cap_usd';
          const range = t('nofap.caps.range', { min: money ? fmtMoney(min, locale) : min, max: money ? fmtMoney(max, locale) : max });
          return (
            <Field key={key} label={t(`nofap.caps.${key}`)} hint={range}>
              {(id) => <Input id={id} inputMode={money ? 'decimal' : 'numeric'} autoComplete="off" value={text[key]} aria-invalid={parsed[key] === null} onChange={(e) => setText((v) => ({ ...v, [key]: e.target.value }))} />}
            </Field>
          );
        })}
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="min-w-0 flex-1 basis-56 text-xs leading-5 text-ink-3">{valid ? t(delayVariant('nofap.caps.note', delaySecs), { delay: fmtDelay(delaySecs, t) }) : t('nofap.caps.invalid')}</p>
        <Button size="sm" disabled={!valid || !changed} onClick={save}>
          {t('nofap.caps.save')}
        </Button>
      </div>
    </div>
  );
}
