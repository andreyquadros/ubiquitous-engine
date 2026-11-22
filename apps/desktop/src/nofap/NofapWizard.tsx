import { useState } from 'react';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { Field, Textarea } from '../components/ui/Field';
import { Progress } from '../components/ui/misc';
import { Toggle } from '../components/ui/Toggle';
import { useLocale, useT } from '../i18n';
import { ipcErrorMessage } from '../lib/ipc';
import { DelayPicker } from './components/DelayPicker';
import { ResolverChoice } from './components/ResolverChoice';
import { SampleFrameView } from './components/SampleFrameView';
import { consentComplete, VisionConsent } from './components/VisionConsent';
import { RemovalHelp } from './components/RemovalHelp';
import { VisionScope } from './components/VisionScope';
import { nofapIpc } from './ipc';
import { fmtDelay, fmtMoney, reasonText, VALUES_MAX_CHARS, type Translate } from './labels';
import type { AiReadiness, EnableOptions, NofapView, Tier, TierCapability, VisionOff, VisionScope as Scope } from './types';

/** What the host app lends the module: things that live outside it. */
export interface NofapHost {
  /**
   * "Abrir junto com o computador": the host owns the setting. The page changes it at once; the wizard shows the
   * switch on by default, never forces it, and hands the choice over only with its last button.
   */
  autostart?: { value: boolean; onChange: (value: boolean) => void };
  /** Opens the host's own settings (AI provider and key, local-only mode). */
  openSettings?: () => void;
}

const STEPS = 5;
const TIERS: Tier[] = ['hosts', 'resolver', 'policies'];

/** Why the AI check cannot run, in the words of the status row. */
const READINESS_REASON: Record<Exclude<AiReadiness, 'ready'>, VisionOff> = { no_key: 'no_key', local_only: 'local_only', not_onboarded: 'not_onboarded', licence_required: 'licence' };

const DISCLOSURE: Partial<Record<Tier, string>> = { resolver: 'nofap.dns.disclosure', policies: 'nofap.policies.disclosure' };

function capabilityNote(capability: TierCapability, t: Translate): string | null {
  if (capability.state === 'gated') return t('nofap.system.gated');
  if (capability.state === 'unsupported') return reasonText(capability.reason, t);
  return null;
}

interface Props {
  view: NofapView;
  host?: NofapHost;
  /** Called with the fresh view once the last button has switched the module on. */
  onEnabled: (view: NofapView) => void;
}

/**
 * The consent wizard (design 5.2). Five steps, and nothing is applied or stored before the last button: the
 * choices live in this component until then, and that button sends exactly one `enable` request.
 */
export function NofapWizard({ view, host, onEnabled }: Props) {
  const t = useT();
  const locale = useLocale();
  const name = view.display_name;
  const caps = view.capabilities;
  const aiReady = view.ai === 'ready';

  const [step, setStep] = useState(1);
  const [selfUse, setSelfUse] = useState(false);
  const [tiers, setTiers] = useState<Record<Tier, boolean>>({
    hosts: caps.hosts.state === 'available',
    resolver: caps.resolver.state === 'available',
    policies: caps.policies.state === 'available',
  });
  const [vision, setVision] = useState(aiReady);
  const [consent, setConsent] = useState(false);
  const [managed, setManaged] = useState(false);
  const [scope, setScope] = useState<Scope>({
    unknown_sites: view.config.vision.unknown_sites,
    media_apps: view.config.vision.media_apps,
    messaging_apps: view.config.vision.messaging_apps,
    private_windows: view.config.vision.private_windows,
  });
  const [values, setValues] = useState(view.config.values);
  const [delay, setDelay] = useState(view.config.delay_secs);
  const [detailed, setDetailed] = useState(!view.config.ui.discreet);
  // On by default (design 5.2), whatever the host has now; nothing is told to the host before the last button.
  const [autostart, setAutostart] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const chosen = TIERS.filter((tier) => tiers[tier] && caps[tier].state === 'available');
  const visionOn = aiReady && vision;
  const visionBlocked = visionOn && !consentComplete(view.provider, consent, managed);
  const canContinue = step === 1 ? selfUse : step === 3 ? !visionBlocked : true;
  const layers = [t('nofap.layer.guard'), ...chosen.map((tier) => t(`nofap.layer.${tier}`)), ...(visionOn ? [t('nofap.layer.vision')] : [])].join(', ');

  const enable = async () => {
    const options: EnableOptions = {
      self_use: selfUse,
      hosts: chosen.includes('hosts'),
      resolver: chosen.includes('resolver') ? 'cloudflare_family' : null,
      policies: chosen.includes('policies'),
      vision: visionOn,
      vision_consent: visionOn ? { provider: view.provider, managed: view.provider === 'ubi' && managed } : null,
      vision_scope: scope,
      delay_secs: delay,
      values: values.trim(),
      discreet: !detailed,
    };
    setBusy(true);
    setError(null);
    try {
      const enabled = await nofapIpc.enable(options);
      if (host?.autostart && host.autostart.value !== autostart) host.autostart.onChange(autostart);
      onEnabled(enabled);
    } catch (e) {
      setError(ipcErrorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="mx-auto w-full max-w-[720px]" data-testid="nofap-wizard">
      <header className="mb-5">
        <p className="eyebrow">{t('nofap.wizard.step', { step, total: STEPS })}</p>
        <h2 className="display mt-1 text-lg">{t(`nofap.wizard.step${step}`)}</h2>
        <Progress className="mt-3" value={step} max={STEPS} height="h-1" />
      </header>

      {step === 1 && (
        <div className="flex flex-col gap-3" data-testid="wizard-step-1">
          <p className="text-[15px] leading-6 text-ink">{t('nofap.wizard.what', { name })}</p>
          <p className="text-sm leading-6 text-ink-2">{t('nofap.wizard.limits')}</p>
          <p className="text-sm leading-6 text-ink-2">{t('nofap.wizard.adult')}</p>
          <label className="mt-1 flex cursor-pointer items-center gap-2.5 rounded-control border border-line-2 bg-panel-2 px-3 py-2.5 text-sm font-medium text-ink">
            <input type="checkbox" checked={selfUse} onChange={(e) => setSelfUse(e.target.checked)} className="size-4 shrink-0 accent-[var(--volt)]" />
            {t('nofap.wizard.self_use')}
          </label>
        </div>
      )}

      {step === 2 && (
        <div className="flex flex-col gap-4" data-testid="wizard-step-2">
          <ul className="flex flex-col divide-y divide-line">
            {TIERS.map((tier) => {
              const capability = caps[tier];
              const note = capabilityNote(capability, t);
              const disclosure = DISCLOSURE[tier];
              return (
                <li key={tier} className="flex flex-col gap-2 py-3 first:pt-0" data-testid={`wizard-tier-${tier}`}>
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                      <p id={`wizard-tier-label-${tier}`} className="text-sm font-medium text-ink">
                        {t(`nofap.layer.${tier}`)}
                      </p>
                      <p className="mt-0.5 text-xs leading-5 text-ink-3">{note ?? t(`nofap.layer.${tier}.hint`)}</p>
                    </div>
                    {capability.state !== 'available' ? (
                      <Badge tone={capability.state === 'gated' ? 'violet' : 'neutral'}>{t(capability.state === 'gated' ? 'nofap.tier.gated' : 'nofap.tier.unsupported')}</Badge>
                    ) : tier === 'resolver' ? null : (
                      <Toggle label={t(`nofap.layer.${tier}`)} checked={tiers[tier]} onChange={(on) => setTiers((v) => ({ ...v, [tier]: on }))} />
                    )}
                  </div>
                  {tier === 'resolver' && capability.state === 'available' && <ResolverChoice value={tiers.resolver ? 'cloudflare_family' : null} onChange={(resolver) => setTiers((v) => ({ ...v, resolver: resolver !== null }))} />}
                  {disclosure && capability.state === 'available' && tiers[tier] && <p className="panel-raised p-3 text-xs leading-5 text-ink-2">{t(disclosure)}</p>}
                </li>
              );
            })}
          </ul>

          {host?.autostart && (
            <Field label={t('nofap.system.autostart')} hint={t('nofap.system.autostart_hint')} inline>
              {(id) => <Toggle id={id} checked={autostart} onChange={setAutostart} />}
            </Field>
          )}

          <div className="rounded-control border border-line px-4 py-3" data-testid="wizard-changes">
            <h3 className="text-[13px] font-medium text-ink">{t('nofap.wizard.changes_title')}</h3>
            <ul className="mt-2 flex flex-col gap-1.5">
              {chosen.length ? (
                chosen.map((tier) => (
                  <li key={tier} className="flex gap-2.5 text-[13px] leading-5 text-ink-2">
                    <span className="mt-2 size-1.5 shrink-0 rounded-full bg-volt" aria-hidden />
                    {t(`nofap.wizard.change.${tier}`)}
                  </li>
                ))
              ) : (
                <li className="text-[13px] leading-5 text-ink-2">{t('nofap.wizard.change.none')}</li>
              )}
            </ul>
            {chosen.length > 0 && <p className="mt-2 text-xs text-ink-3">{t('nofap.system.prompt_next')}</p>}
          </div>
        </div>
      )}

      {step === 3 && (
        <div className="flex flex-col gap-4" data-testid="wizard-step-3">
          {view.ai === 'ready' ? (
            <>
              <Field label={t('nofap.layer.vision')} hint={t('nofap.layer.vision.hint')} inline>
                {(id) => <Toggle id={id} checked={vision} onChange={setVision} />}
              </Field>
              {vision ? (
                <>
                  <VisionConsent provider={view.provider} name={name} product={view.product} privateWindows={scope.private_windows} consent={consent} onConsent={setConsent} managed={managed} onManaged={setManaged} />
                  {visionBlocked && (
                    <p className="text-xs text-ink-2" role="status">
                      {t('nofap.vision.need_tick')}
                    </p>
                  )}
                  <p className="text-xs leading-5 text-ink-3">{t('nofap.vision.cost', { per_day: view.config.vision.max_per_day, cap: fmtMoney(view.config.vision.monthly_cap_usd, locale) })}</p>
                  <div className="border-t border-line pt-4">
                    <h3 className="mb-3 text-[13px] font-medium text-ink">{t('nofap.vision.scope_title')}</h3>
                    <VisionScope scope={scope} onChange={setScope} />
                  </div>
                  <SampleFrameView />
                </>
              ) : (
                <p className="text-xs leading-5 text-ink-3">{t('nofap.vision.off_note')}</p>
              )}
            </>
          ) : (
            <>
              <p className="text-sm leading-6 text-ink-2">{t('nofap.vision.unavailable', { reason: t(`nofap.vision.off.${READINESS_REASON[view.ai]}`) })}</p>
              <p className="text-xs leading-5 text-ink-3">{t('nofap.vision.off_note')}</p>
              {host?.openSettings && (
                <div>
                  <Button size="sm" onClick={host.openSettings}>
                    {t('nofap.btn.open_settings')}
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {step === 4 && (
        <div className="flex flex-col gap-5" data-testid="wizard-step-4">
          <Field label={t('nofap.values.label')} hint={t('nofap.checkin.values')}>
            {(id) => <Textarea id={id} value={values} maxLength={VALUES_MAX_CHARS} placeholder={t('nofap.values.placeholder')} onChange={(e) => setValues(e.target.value)} />}
          </Field>
          <div className="flex flex-col gap-2">
            <span className="text-[13px] font-medium text-ink">{t('nofap.delay.label')}</span>
            <DelayPicker value={delay} onChange={setDelay} />
            <p className="text-xs leading-5 text-ink-3">{t('nofap.commit.subtitle')}</p>
          </div>
          <Field label={t('nofap.prefs.detailed')} hint={t('nofap.prefs.detailed_hint')} inline>
            {(id) => <Toggle id={id} checked={detailed} onChange={setDetailed} />}
          </Field>
        </div>
      )}

      {step === 5 && (
        <div className="flex flex-col gap-4" data-testid="wizard-step-5">
          <section>
            <h3 className="text-[13px] font-medium text-ink">{t('nofap.wizard.summary.choices')}</h3>
            <p className="mt-1 text-[13px] leading-5 text-ink-2">{t('nofap.wizard.summary.layers', { layers })}</p>
            <p className="text-[13px] leading-5 text-ink-2">{t('nofap.wizard.summary.delay', { delay: fmtDelay(delay, t) })}</p>
            {host?.autostart && <p className="text-[13px] leading-5 text-ink-2">{t(autostart ? 'nofap.wizard.summary.autostart_on' : 'nofap.wizard.summary.autostart_off')}</p>}
          </section>
          {(['data', 'off', 'remove'] as const).map((part) => (
            <section key={part}>
              <h3 className="text-[13px] font-medium text-ink">{t(`nofap.helptab.${part}.title`)}</h3>
              {part === 'remove' ? (
                <div className="mt-1">
                  <RemovalHelp os={view.capabilities.os} />
                </div>
              ) : (
                <p className="mt-1 text-[13px] leading-5 text-ink-2">{t(`nofap.helptab.${part}.body`)}</p>
              )}
            </section>
          ))}
          <p className="text-xs leading-5 text-ink-3">{t('nofap.privacy.at_rest', { name })}</p>
          {chosen.length > 0 && <p className="text-xs leading-5 text-ink-3">{t('nofap.system.prompt_next')}</p>}
          {error && (
            <p role="alert" className="rounded-control border border-rose/40 bg-rose/10 px-3 py-2 text-[13px] leading-5 text-ink">
              {t('nofap.wizard.enable_failed')} {error}
            </p>
          )}
        </div>
      )}

      <footer className="mt-6 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-4">
        <p className="text-xs text-ink-3">{step < STEPS ? t('nofap.wizard.nothing_yet') : ''}</p>
        <div className="flex items-center gap-2">
          {step > 1 && (
            <Button variant="ghost" onClick={() => setStep((s) => s - 1)} disabled={busy}>
              {t('nofap.btn.back')}
            </Button>
          )}
          {step < STEPS ? (
            <Button variant="primary" onClick={() => setStep((s) => s + 1)} disabled={!canContinue}>
              {t('nofap.btn.next')}
            </Button>
          ) : (
            <Button variant="primary" onClick={() => void enable()} loading={busy} data-testid="wizard-enable">
              {t('nofap.wizard.enable', { name })}
            </Button>
          )}
        </div>
      </footer>
    </Card>
  );
}
