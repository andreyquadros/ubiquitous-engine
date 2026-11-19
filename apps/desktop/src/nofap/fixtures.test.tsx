import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { hasKey, setLocale, translate } from '../i18n';
import type { Locale } from '../i18n/types';
import { IssueList } from './components/IssueList';
import { SystemPreview } from './components/SystemPreview';
import { TierRow } from './components/TierRow';
import commandsJson from './fixtures/commands.json';
import { commands, views } from './fixtures/golden';
import viewsJson from './fixtures/views.json';
import { LAYERS, PREVIEW_PLACEHOLDER, reasonText, tierKey } from './labels';
import { __nofapMock, handle } from './mock';
import type { NofapCommand, NofapStatus, PlannedChange } from './types';

const LOCALES: Locale[] = ['pt-BR', 'en'];
const thaw = <T,>(value: unknown): T => structuredClone(value) as T;

const TAGS = ['state', 'kind', 'outcome', 'type', 'event'];

/**
 * True when `value` has exactly the fields of `sample`, all the way down (arrays: their first elements). A tagged
 * union is compared by its tag only: its other fields depend on the variant, and `contract.ts` checks those.
 */
function sameShape(value: unknown, sample: unknown): boolean {
  if (Array.isArray(sample)) return Array.isArray(value) && (value.length === 0 || sample.length === 0 || sameShape(value[0], sample[0]));
  if (sample && typeof sample === 'object') {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
    const tag = TAGS.find((t) => t in sample && !('id' in sample));
    if (tag) return tag in value;
    const a = Object.keys(value).sort();
    const b = Object.keys(sample).sort();
    return a.join() === b.join() && b.every((k) => sample[k as keyof typeof sample] === null || value[k as keyof typeof value] === null || sameShape(value[k as keyof typeof value], sample[k as keyof typeof sample]));
  }
  return typeof value === typeof sample;
}

describe('the IPC fixtures', () => {
  it('are the same values in JSON and in the typed module', () => {
    expect(commandsJson).toEqual(commands);
    expect(viewsJson).toEqual(views);
  });

  it('cover every command name', () => {
    expect(new Set(commands.map((c) => c.cmd)).size).toBe(20);
  });
});

describe('the mock backend answers every fixture request in the fixture shape', () => {
  beforeEach(() => __nofapMock.reset('on'));

  const replyOf: Record<NofapCommand['cmd'], unknown> = {
    view: views.view,
    enable: views.view,
    consent_vision: views.view,
    request: null,
    confirm_pending: views.view,
    cancel_pending: views.view,
    system_preview: views.planned_changes,
    system_apply: null,
    system_remove: null,
    restore_network: null,
    sites: views.sites,
    support: views.support,
    record: null,
    card: views.cards[0],
    card_action: null,
    sample_frame: views.sample_frames[0],
    erase_history: null,
    reset: null,
    export: views.export,
    test_card: null,
  };

  for (const [i, fixture] of commands.entries()) {
    const command = thaw<NofapCommand>(fixture);
    it(`${i}: ${command.cmd}${command.cmd === 'request' ? ` ${command.args.kind}` : command.cmd === 'record' ? ` ${command.args.event}` : ''}`, () => {
      // The ids of the fixtures name things this fresh state does not have: the documented not-found case.
      if (command.cmd === 'confirm_pending' || command.cmd === 'cancel_pending') {
        expect(() => handle(command)).toThrow(/^not_found: /);
        return;
      }
      // Two commands are refused while the module is on.
      if (command.cmd === 'system_remove' || command.cmd === 'reset') {
        expect(() => handle(command)).toThrow(/^invalid: /);
        __nofapMock.reset('left');
      }
      const reply = handle(command);
      const sample = replyOf[command.cmd];
      if (command.cmd === 'request') {
        const outcome = reply as Record<string, unknown>;
        const applied = 'applied' in outcome;
        expect(sameShape(reply, views.change_outcomes[applied ? 0 : 1])).toBe(true);
      } else if (command.cmd === 'system_apply' || command.cmd === 'system_remove' || command.cmd === 'restore_network') {
        expect(views.apply_outcomes.map((o) => o.outcome)).toContain((reply as { outcome: string }).outcome);
      } else if (sample === null) {
        expect(reply).toBeNull();
      } else {
        expect(sameShape(reply, sample), JSON.stringify(reply)).toBe(true);
      }
    });
  }

  it('gives the neutral discreet card for an id it does not know', () => {
    expect(handle({ cmd: 'card', args: { id: 'card-1' } })).toMatchObject({ id: 'card-1', kind: 'blocked', message_key: 'block.discreet', discreet: true, actions: ['support', 'ok'] });
  });

  it('answers each kind of prompt outcome in the fixture shape', () => {
    for (const [answer, sample] of [['accept', 0], ['partial', 1], ['decline', 2], ['unavailable', 3]] as const) {
      __nofapMock.reset('reduced');
      __nofapMock.setPrompt(answer);
      expect(sameShape(handle({ cmd: 'system_apply' }), views.apply_outcomes[sample])).toBe(true);
    }
  });
});

describe('every state of the fixtures has words and renders', () => {
  let warn: ReturnType<typeof vi.spyOn>;
  beforeEach(() => {
    warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
  });
  afterEach(() => warn.mockRestore());

  it('has a label in both languages for every value of every closed set', () => {
    const e = views.enums;
    const keys = [
      ...e.overall.map((v) => `overall.${v}`),
      ...e.guard_limit.map((v) => `guard.limit.${v}`),
      ...e.vision_off.map((v) => `vision.off.${v}`),
      ...e.change_kind.map((v) => `pending.kind.${v}`),
      ...e.card_action.map((v) => `btn.${v}`),
      ...e.mood.map((v) => `mood.${v}`),
      ...e.trigger.map((v) => `trigger.${v}`),
      ...e.tier.flatMap((v) => [`layer.${v}`, `layer.${v}.hint`, `wizard.change.${v}`]),
      ...e.resolver.map((v) => `resolver.${v}`),
      ...e.reason.map((v) => `reason.${v}`),
      ...views.preview_lines.map((l) => l.key),
      ...views.cards.flatMap((c) => [c.message_key, ...(c.note_key ? [c.note_key] : [])]),
      ...(['off', 'applied', 'update_pending', 'changed', 'missing', 'not_applied', 'gated', 'unsupported', 'in_use_by_other'] as const).map((s) => `tier.${tierKey(s)}`),
    ];
    for (const locale of LOCALES) for (const key of keys) expect(hasKey(`nofap.${key}`, locale), `nofap.${key} (${locale})`).toBe(true);
    // Issues: two reuse a sentence of the copy bank.
    for (const issue of e.issue) expect(hasKey(`nofap.issue.${issue}`) || issue === 'resolver_unreachable' || issue === 'other_product_primary', issue).toBe(true);
  });

  it('fills the placeholder of each preview line with its detail', () => {
    for (const line of views.preview_lines) {
      expect(PREVIEW_PLACEHOLDER[line.key]).toBe(line.placeholder);
      for (const locale of LOCALES) {
        const text = translate(locale, `nofap.${line.key}`, line.placeholder ? { [line.placeholder]: 'VALUE' } : undefined);
        expect(text.includes('{'), `${line.key} (${locale})`).toBe(false);
        expect(text.includes('VALUE')).toBe(line.placeholder !== null);
      }
    }
  });

  it('gives a code it does not know the neutral sentence', () => {
    expect(reasonText('written_by_a_newer_build', (k) => translate('pt-BR', k))).toBe('Não foi possível neste computador.');
    expect(reasonText('no_resolved', (k) => translate('pt-BR', k))).toContain('systemd-resolved');
  });

  for (const locale of LOCALES) {
    it(`renders every status of the fixtures (${locale}): one row per layer, every tier state and every issue`, () => {
      setLocale(locale);
      const seenStates = new Set<string>();
      const seenIssues = new Set<string>();
      for (const fixture of views.statuses) {
        const status = thaw<NofapStatus>(fixture);
        const { unmount } = render(
          <>
            <ul>
              {LAYERS.map((layer) => (
                <TierRow key={layer} layer={layer} status={status} capabilities={thaw(views.view.capabilities)} />
              ))}
            </ul>
            <IssueList issues={status.issues} onAction={() => undefined} />
          </>,
        );
        for (const tier of ['hosts', 'resolver', 'policies'] as const) {
          seenStates.add(status[tier].state);
          expect(screen.getByTestId(`tier-row-${tier}`)).toHaveTextContent(translate(locale, `nofap.tier.${tierKey(status[tier].state)}`));
        }
        for (const issue of status.issues) {
          seenIssues.add(issue);
          expect(screen.getByTestId(`issue-${issue}`).textContent?.length).toBeGreaterThan(20);
        }
        // No key leaks into the page, and nothing is left unfilled.
        expect(document.body.textContent).not.toMatch(/nofap\.|\{\w+\}/);
        unmount();
      }
      expect(seenStates.size).toBe(9);
      expect(seenIssues.size).toBe(views.enums.issue.length);
      expect(warn).not.toHaveBeenCalled();
    });
  }

  it('labels a tier a tester opened and says why a tier is closed', () => {
    const status = thaw<NofapStatus>(views.statuses[1]);
    render(
      <ul>
        <TierRow layer="hosts" status={status} capabilities={thaw(views.view.capabilities)} />
        <TierRow layer="policies" status={status} capabilities={thaw(views.view.capabilities)} />
        <TierRow layer="resolver" status={thaw<NofapStatus>(views.statuses[3])} />
      </ul>,
    );
    expect(screen.getByTestId('tier-row-hosts')).toHaveTextContent('Em teste');
    expect(screen.getByTestId('tier-row-hosts')).toHaveTextContent('1 site espera a próxima vez que você aplicar.');
    expect(screen.getByTestId('tier-row-policies')).toHaveTextContent('O sistema não deixou escrever o arquivo.');
    expect(screen.getByTestId('tier-row-resolver')).toHaveTextContent('Em verificação neste sistema. Por enquanto valem o bloqueio no app e a verificação por IA.');
  });

  it('gives an issue at most one action', () => {
    const onAction = vi.fn();
    render(<IssueList issues={thaw(views.enums.issue)} onAction={onAction} />);
    for (const issue of views.enums.issue) expect(screen.getByTestId(`issue-${issue}`).querySelectorAll('button').length).toBeLessThanOrEqual(1);
    screen.getByRole('button', { name: 'Restaurar minha rede' }).click();
    expect(onAction).toHaveBeenCalledWith('restore_network');
  });

  it('renders the planned changes of the fixtures as sentences', () => {
    render(<SystemPreview lines={thaw<PlannedChange[]>(views.planned_changes)} />);
    const preview = screen.getByTestId('system-preview');
    expect(preview).toHaveTextContent('Adiciona 8003 sites à lista do sistema');
    expect(preview).toHaveTextContent('Seu DNS atual (192.168.0.2) será substituído.');
    expect(preview).toHaveTextContent('Escreve as regras em Chrome, Firefox');
    expect(preview).toHaveTextContent('O sistema vai pedir sua senha de administrador para aplicar isso.');
  });
});
