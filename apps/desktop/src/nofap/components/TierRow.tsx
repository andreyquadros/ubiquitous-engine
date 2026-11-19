import { Eye, FileText, Globe, Network, ShieldCheck, type LucideIcon } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { useT } from '../../i18n';
import { reasonText, tierKey, type Layer, type Translate } from '../labels';
import type { NofapStatus, SystemCapabilities, TierState } from '../types';

type Tone = 'neutral' | 'volt' | 'ember' | 'signal' | 'violet' | 'amber';

interface RowText {
  label: string;
  tone: Tone;
  details: string[];
}

const ICONS: Record<Layer, LucideIcon> = { guard: ShieldCheck, hosts: FileText, resolver: Network, policies: Globe, vision: Eye };

const TIER_TONE: Record<TierState['state'], Tone> = {
  off: 'neutral',
  applied: 'signal',
  update_pending: 'volt',
  changed: 'ember',
  missing: 'ember',
  not_applied: 'amber',
  gated: 'violet',
  unsupported: 'neutral',
  in_use_by_other: 'volt',
};

const fmtTime = (iso: string): string => new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

function tierText(layer: 'hosts' | 'resolver' | 'policies', status: NofapStatus, t: Translate): RowText {
  const state = status[layer];
  const details: string[] = [];
  switch (state.state) {
    case 'unsupported':
      details.push(reasonText(state.reason, t));
      break;
    case 'gated':
      details.push(t('nofap.system.gated'));
      break;
    case 'off':
    case 'applied':
      details.push(t(`nofap.layer.${layer}.hint`));
      break;
    case 'update_pending':
    case 'changed':
    case 'missing':
    case 'not_applied':
    case 'in_use_by_other':
      details.push(t(`nofap.tier.${tierKey(state.state)}.detail`));
      break;
  }
  if (layer === 'hosts' && status.hosts_waiting > 0) details.push(t('nofap.tier.waiting', { count: status.hosts_waiting }));
  if (layer === 'hosts' && status.extras_left_out > 0) details.push(t('nofap.tier.left_out', { count: status.extras_left_out }));
  return { label: t(`nofap.tier.${tierKey(state.state)}`), tone: TIER_TONE[state.state], details };
}

function rowText(layer: Layer, status: NofapStatus, t: Translate): RowText {
  if (layer === 'guard') {
    const guard = status.guard;
    if (guard.state === 'limited') return { label: t('nofap.guard.limited'), tone: 'amber', details: [t(`nofap.guard.limit.${guard.reason}`)] };
    return { label: t(`nofap.guard.${guard.state}`), tone: guard.state === 'on' ? 'signal' : 'neutral', details: [t('nofap.layer.guard.hint')] };
  }
  if (layer === 'vision') {
    const vision = status.vision;
    if (vision.state === 'on') return { label: t('nofap.vision.state.on'), tone: 'signal', details: [t('nofap.layer.vision.hint')] };
    if (vision.state === 'degraded') return { label: t('nofap.vision.state.degraded'), tone: 'amber', details: [t('nofap.vision.degraded.detail', { time: fmtTime(vision.until) })] };
    return { label: t('nofap.vision.state.off'), tone: vision.reason === 'disabled' ? 'neutral' : 'amber', details: [t(`nofap.vision.off.${vision.reason}`)] };
  }
  return tierText(layer, status, t);
}

/** One line of the status header: a layer, what it is doing and why. Never a site name. */
export function TierRow({ layer, status, capabilities }: { layer: Layer; status: NofapStatus; capabilities?: SystemCapabilities }) {
  const t = useT();
  const Icon = ICONS[layer];
  const { label, tone, details } = rowText(layer, status, t);
  const unverified = layer !== 'guard' && layer !== 'vision' && capabilities?.unverified.includes(layer);
  return (
    <li className="flex items-start gap-3 py-3" data-testid={`tier-row-${layer}`}>
      <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-[8px] bg-panel-2 text-ink-3" aria-hidden>
        <Icon className="size-4" strokeWidth={1.75} />
      </span>
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-ink">{t(`nofap.layer.${layer}`)}</p>
        {details.map((detail) => (
          <p key={detail} className="mt-0.5 text-xs leading-5 text-ink-3">
            {detail}
          </p>
        ))}
      </div>
      <div className="flex shrink-0 flex-col items-end gap-1">
        <Badge tone={tone}>{label}</Badge>
        {unverified && <Badge tone="violet">{t('nofap.tier.unverified')}</Badge>}
      </div>
    </li>
  );
}
