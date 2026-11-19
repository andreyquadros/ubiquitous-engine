import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Field } from '../../components/ui/Field';
import { Toggle } from '../../components/ui/Toggle';
import { useLocale, useT } from '../../i18n';
import { DelayPicker } from '../components/DelayPicker';
import { ResolverChoice } from '../components/ResolverChoice';
import { SampleFrameView } from '../components/SampleFrameView';
import { VisionCaps } from '../components/VisionCaps';
import { VisionScope } from '../components/VisionScope';
import { delayVariant, fmtDelay, fmtMoney, providerName, reasonText } from '../labels';
import type { NofapDialog } from '../NofapPage';
import type { NofapHost } from '../NofapWizard';
import type { NofapChange, NofapView, Tier } from '../types';

interface Props {
  view: NofapView;
  host?: NofapHost;
  onRequest: (change: NofapChange) => void;
  onOpen: (dialog: NofapDialog) => void;
}

const TIERS: Tier[] = ['hosts', 'resolver', 'policies'];
const DISCLOSURE: Partial<Record<Tier, string>> = { resolver: 'nofap.dns.disclosure', policies: 'nofap.policies.disclosure' };

/** "Proteção": which layers are chosen, the AI check, and the agreement about turning things off. */
export function ProtectionTab({ view, host, onRequest, onOpen }: Props) {
  const t = useT();
  const locale = useLocale();
  const name = view.display_name;
  const { system, vision, ui } = view.config;
  const delay = fmtDelay(view.config.delay_secs, t);
  const chosen: Record<Tier, boolean> = { hosts: system.hosts, resolver: system.resolver !== null, policies: system.policies };
  const change = (tier: 'hosts' | 'policies', on: boolean): NofapChange => (tier === 'hosts' ? { kind: 'set_hosts', on } : { kind: 'set_policies', on });
  const provider = providerName(view.provider, t);
  const consentCurrent = vision.consent?.provider === view.provider;
  const visionOn = view.status.vision.state !== 'off';

  const toggleVision = (on: boolean) => {
    // Switching on needs a consent that names the provider now selected.
    if (on && !consentCurrent) onOpen('consent');
    else onRequest({ kind: 'set_vision', on });
  };

  return (
    <div className="flex flex-col gap-5" data-testid="tab-protection">
      <Card>
        <CardHeader title={t('nofap.system.title')} subtitle={t('nofap.system.subtitle')} />
        <ul className="flex flex-col divide-y divide-line">
          {TIERS.map((tier) => {
            const disclosure = DISCLOSURE[tier];
            const capability = view.capabilities[tier];
            // A tier this system cannot have is shown as such: no switch, no choice, and no disclosure of
            // something that is not happening.
            if (capability.state !== 'available') {
              return (
                <li key={tier} className="flex items-start justify-between gap-4 py-3 first:pt-0" data-testid={`protection-tier-${tier}`}>
                  <div className="min-w-0">
                    <p className="text-[13px] font-medium text-ink">{t(`nofap.layer.${tier}`)}</p>
                    <p className="text-xs leading-5 text-ink-3">{capability.state === 'gated' ? t('nofap.system.gated') : reasonText(capability.reason, t)}</p>
                  </div>
                  <Badge tone={capability.state === 'gated' ? 'violet' : 'neutral'}>{t(capability.state === 'gated' ? 'nofap.tier.gated' : 'nofap.tier.unsupported')}</Badge>
                </li>
              );
            }
            return (
              <li key={tier} className="flex flex-col gap-2 py-3 first:pt-0" data-testid={`protection-tier-${tier}`}>
                {tier === 'resolver' ? (
                  <div className="flex flex-col gap-2">
                    <div>
                      <p className="text-[13px] font-medium text-ink">{t('nofap.layer.resolver')}</p>
                      <p className="text-xs text-ink-3">{t('nofap.layer.resolver.hint')}</p>
                    </div>
                    <ResolverChoice value={system.resolver} onChange={(resolver) => onRequest({ kind: 'set_resolver', resolver })} />
                  </div>
                ) : (
                  <Field label={t(`nofap.layer.${tier}`)} hint={t(`nofap.layer.${tier}.hint`)} inline>
                    {(id) => <Toggle id={id} checked={chosen[tier]} onChange={(on) => onRequest(change(tier, on))} />}
                  </Field>
                )}
                {disclosure && chosen[tier] && <p className="panel-raised p-3 text-xs leading-5 text-ink-2">{t(disclosure)}</p>}
              </li>
            );
          })}
        </ul>
        {host?.autostart && (
          <div className="border-t border-line pt-3">
            <Field label={t('nofap.system.autostart')} hint={t('nofap.system.autostart_hint')} inline>
              {(id) => <Toggle id={id} checked={host.autostart?.value ?? false} onChange={(v) => host.autostart?.onChange(v)} />}
            </Field>
          </div>
        )}
        <p className="mt-3 text-xs text-ink-3">{t(delayVariant('nofap.system.loosen_note', view.config.delay_secs), { delay })}</p>
      </Card>

      <Card data-testid="vision-card">
        <CardHeader title={t('nofap.vision.title')} subtitle={t('nofap.layer.vision.hint')} action={<Toggle label={t('nofap.vision.title')} checked={vision.enabled} onChange={toggleVision} />} />
        {vision.enabled ? (
          <div className="flex flex-col gap-4">
            {visionOn ? (
              <p className="text-[13px] leading-5 text-ink-2">{t('nofap.vision.today', { n: view.today.checks, provider })}</p>
            ) : (
              view.status.vision.state === 'off' && <p className="text-[13px] leading-5 text-ink-2">{t(`nofap.vision.off.${view.status.vision.reason}`)}</p>
            )}
            {!consentCurrent && (
              <div>
                <Button size="sm" variant="primary" onClick={() => onOpen('consent')}>
                  {t('nofap.btn.review_consent')}
                </Button>
              </div>
            )}
            <p className="num text-xs leading-5 text-ink-3">
              {t('nofap.vision.cost', { per_day: vision.max_per_day, cap: fmtMoney(vision.monthly_cap_usd, locale) })}{' '}
              {t('nofap.vision.month', { checks: view.cost_line.month_checks, usd: fmtMoney(view.cost_line.month_usd, locale), cap: fmtMoney(view.cost_line.cap_usd, locale) })}
            </p>
            <div className="border-t border-line pt-4">
              <h3 className="mb-3 text-[13px] font-medium text-ink">{t('nofap.vision.scope_title')}</h3>
              <VisionScope scope={vision} onChange={(scope) => onRequest({ kind: 'set_vision_scope', unknown_sites: scope.unknown_sites, media_apps: scope.media_apps, messaging_apps: scope.messaging_apps, private_windows: scope.private_windows })} />
            </div>
            <SampleFrameView />
            <div className="border-t border-line pt-4">
              <VisionCaps caps={vision} delaySecs={view.config.delay_secs} onSave={(caps) => onRequest({ kind: 'set_caps', ...caps })} />
            </div>
          </div>
        ) : (
          <p className="text-xs leading-5 text-ink-3">{t('nofap.vision.off_note')}</p>
        )}
      </Card>

      <Card data-testid="commit-card">
        <CardHeader title={t('nofap.commit.title')} subtitle={t('nofap.commit.subtitle')} />
        <div className="flex flex-col gap-2">
          <span className="text-[13px] font-medium text-ink">{t('nofap.delay.label')}</span>
          <DelayPicker value={view.config.delay_secs} onChange={(secs) => secs !== view.config.delay_secs && onRequest({ kind: 'set_delay', secs })} />
          <p className="text-xs text-ink-3">{t('nofap.delay.lower_note')}</p>
        </div>
        <div className="mt-4 border-t border-line pt-4">
          <Field label={t('nofap.prefs.detailed')} hint={t('nofap.prefs.detailed_hint')} inline>
            {(id) => <Toggle id={id} checked={!ui.discreet} onChange={(detailed) => onRequest({ kind: 'set_prefs', prefs: { ...ui, discreet: !detailed } })} />}
          </Field>
        </div>
        <div className="mt-4 flex justify-end border-t border-line pt-4">
          <Button variant="ghost" size="sm" onClick={() => onOpen('off')} data-testid="nofap-off-button">
            {t('nofap.off.button', { name })}
          </Button>
        </div>
      </Card>
    </div>
  );
}
