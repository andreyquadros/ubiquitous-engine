import { useId } from 'react';
import { useT } from '../../i18n';
import { consentKey, providerName } from '../labels';
import type { AiProvider } from '../types';

interface Props {
  provider: AiProvider;
  name: string;
  consent: boolean;
  onConsent: (ticked: boolean) => void;
  /** The second tick, asked only when the checks go through the managed route ("IA do Ubi"). */
  managed: boolean;
  onManaged: (ticked: boolean) => void;
}

function Tick({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  const id = useId();
  return (
    <label htmlFor={id} className="flex cursor-pointer items-center gap-2.5 text-[13px] font-medium text-ink">
      <input id={id} type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} className="size-4 shrink-0 accent-[var(--volt)]" />
      {label}
    </label>
  );
}

/** What the AI check sends, to whom, and the box that has to be ticked by hand before it can be switched on. */
export function VisionConsent({ provider, name, consent, onConsent, managed, onManaged }: Props) {
  const t = useT();
  return (
    <div className="flex flex-col gap-3" data-testid="vision-consent">
      <p className="panel-raised p-3 text-[13px] leading-5 text-ink-2">{t(consentKey(provider), { name, provider: providerName(provider, t) })}</p>
      <Tick checked={consent} onChange={onConsent} label={t('nofap.vision.consent_tick')} />
      {provider === 'ubi' && (
        <>
          <p className="panel-raised p-3 text-[13px] leading-5 text-ink-2">{t('nofap.vision.managed')}</p>
          <Tick checked={managed} onChange={onManaged} label={t('nofap.vision.managed_tick')} />
        </>
      )}
    </div>
  );
}

/** Whether the ticks the selected provider needs are all there. */
export const consentComplete = (provider: AiProvider, consent: boolean, managed: boolean): boolean => consent && (provider !== 'ubi' || managed);
