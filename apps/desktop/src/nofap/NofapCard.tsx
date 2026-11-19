import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Ubi } from '../components/ubi/Ubi';
import { Button } from '../components/ui/Button';
import { hasKey, useT } from '../i18n';
import { isTauri } from '../lib/ipc';
import { PanicSheet, type PanicView } from './components/PanicSheet';
import { ResourceList } from './components/ResourceList';
import { onNofapEvent } from './events';
import { nofapIpc } from './ipc';
import { cardSize, fmtDelay } from './labels';
import type { CardAction, CardKind, NofapCard, NofapView, SupportView, UserEvent } from './types';

/** What the window shows: the card itself, or one of the flows a button opens inside it. */
type Mode = 'card' | 'support' | PanicView | 'help' | 'how_off' | 'done';

/** How long an acknowledgement ("Anotado.") stays before the window closes. */
export const CARD_DONE_CLOSE_MS = 6000;

/** The sentences a discreet block card rotates through once the user opens support. */
const SUPPORT_MESSAGES = ['block.breathe', 'block.choice', 'block.next'] as const;

/** The button that carries the weight on each card; the rest are quiet. */
const PRIMARY: Record<CardKind, CardAction> = {
  blocked: 'ok',
  blocked_repeat: 'surf',
  vision_closed: 'ok',
  vision_ask: 'close_window',
  drift: 'restore',
  pending_ready: 'keep_protection',
  resolver_unreachable: 'restore_network',
  reminder: 'ok',
  help: 'help',
};

/** The neutral card: what an unknown id, or a request that could not be answered, shows. Never a sample. */
const neutral = (id: string): NofapCard => ({
  id,
  kind: 'blocked',
  message_key: 'block.discreet',
  note_key: null,
  discreet: true,
  actions: ['support', 'ok'],
  auto_close_secs: 12,
  expires_at: new Date(Date.now() + 600_000).toISOString(),
});

/** The buttons that correct a verdict (design 5.4). */
const isCorrection = (action: CardAction): boolean => action === 'not_it' || action === 'request_exception';

function actionLabel(card: NofapCard, action: CardAction): string {
  if (action === 'surf' && card.kind === 'blocked_repeat') return 'nofap.btn.surf_wave';
  if (action === 'help' && card.kind === 'reminder') return 'nofap.btn.how_off';
  return `nofap.btn.${action}`;
}

/** Closes the card window in Tauri; in a browser only a popup opened by script can close itself. */
async function closeCardWindow(): Promise<void> {
  if (isTauri()) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
    return;
  }
  if (typeof window !== 'undefined' && window.opener) window.close();
}

/** Asks for the size the content needs. A shell that sizes the window itself may refuse; the content then scrolls. */
async function resizeCardWindow(width: number, height: number): Promise<void> {
  try {
    if (isTauri()) {
      const { getCurrentWindow, LogicalSize } = await import('@tauri-apps/api/window');
      await getCurrentWindow().setSize(new LogicalSize(width, height));
    } else if (typeof window !== 'undefined' && window.opener) {
      window.resizeTo(width + (window.outerWidth - window.innerWidth), height + (window.outerHeight - window.innerHeight));
    }
  } catch {
    // The window keeps its size.
  }
}

/**
 * The floating card window (second Tauri window, label `shield-card`, 420 px wide, no decorations, always on
 * top): UBI calm on the left, one or two sentences, and the card's buttons. A card carries no host, title or
 * application name, and neither does anything this page adds. Renders standalone in the browser at
 * `#/nofap-card?id=<id>`.
 */
export function NofapCardPage() {
  const t = useT();
  const [params] = useSearchParams();
  const id = params.get('id') ?? '';
  const [card, setCard] = useState<NofapCard | null>(null);
  const [mode, setMode] = useState<Mode>('card');
  const [done, setDone] = useState<string | null>(null);
  const [closed, setClosed] = useState(false);
  const [view, setView] = useState<NofapView | null>(null);
  const [support, setSupport] = useState<SupportView | null>(null);
  const [busy, setBusy] = useState(false);
  const cardId = useRef(id);
  const shown = useRef(false);
  /** A button of this card was pressed (its request may still be on its way). */
  const touched = useRef(false);
  /** The mode of the latest render, for the timer that outlives a render. */
  const modeNow = useRef<Mode>(mode);
  modeNow.current = mode;

  const show = useCallback((next: NofapCard) => {
    shown.current = true;
    touched.current = false;
    cardId.current = next.id;
    setCard(next);
    // A block card with detailed messages on opens directly as the support card.
    setMode(next.kind === 'blocked' && !next.discreet ? 'support' : 'card');
    setDone(null);
    setClosed(false);
  }, []);

  const close = useCallback(() => {
    setClosed(true);
    void closeCardWindow();
  }, []);

  useEffect(() => {
    let alive = true;
    // The answer may arrive after the shell's `card` event already showed this card and the person opened a
    // flow in it: only the first card of an id comes from here, so a late answer never resets the flow. A new id
    // in the address (the shell reusing the window) starts over.
    shown.current = false;
    void nofapIpc
      .card(id)
      .then((c) => alive && !shown.current && show(c))
      .catch(() => alive && !shown.current && show(neutral(id)));
    void nofapIpc
      .view()
      .then((v) => alive && setView(v))
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [id, show]);

  // The shell may re-show the window with a new card, or take the card away, through events.
  useEffect(() => {
    let unsubscribe: (() => void) | null = null;
    let disposed = false;
    void onNofapEvent((event) => {
      if (event.type === 'card') show(event.card);
      if (event.type === 'card_closed' && event.id === cardId.current) close();
    }).then((u) => {
      if (disposed) u();
      else unsubscribe = u;
    });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [show, close]);

  const expanded = mode !== 'card' && mode !== 'done';
  const cardKind = card?.kind;

  // The values sentence, the plans and the help resources are needed only once a flow opens.
  useEffect(() => {
    if ((expanded || cardKind === 'blocked_repeat') && !support) void nofapIpc.support().then(setSupport).catch(() => undefined);
  }, [expanded, cardKind, support]);

  useEffect(() => {
    if (!card) return;
    const size = cardSize(card, expanded);
    void resizeCardWindow(size.width, size.height);
  }, [card, expanded]);

  // The card closes by itself only while nobody touched it. Clearing the timer is not enough: React runs
  // this effect's cleanup a moment after the click is handled and painted, and a timer that falls due in that
  // moment would close a card the person has just opened. So the timer asks, when it fires, whether the card
  // is still untouched.
  useEffect(() => {
    if (!card || mode !== 'card' || !card.auto_close_secs) return;
    const timer = setTimeout(() => {
      if (!touched.current && modeNow.current === 'card') close();
    }, card.auto_close_secs * 1000);
    return () => clearTimeout(timer);
  }, [card, mode, close]);

  // The same question at firing time as above: a new card may have replaced the acknowledgement in a render
  // whose effects have not run yet.
  useEffect(() => {
    if (mode !== 'done') return;
    const timer = setTimeout(() => {
      if (modeNow.current === 'done') close();
    }, CARD_DONE_CLOSE_MS);
    return () => clearTimeout(timer);
  }, [mode, close]);

  const record = useCallback((event: UserEvent) => void nofapIpc.record(event).catch(() => undefined), []);

  const acknowledge = (text: string) => {
    setDone(text);
    setMode('done');
  };

  /** Every button tells the engine first (it holds what the button needs); the flows then run in this window. */
  const act = async (action: CardAction) => {
    if (!card) return;
    touched.current = true;
    setBusy(true);
    try {
      const before = action === 'not_it' ? await nofapIpc.view().catch(() => null) : null;
      await nofapIpc.cardAction(card.id, action).catch(() => undefined);
      switch (action) {
        case 'support':
          setMode('support');
          break;
        case 'surf':
          setMode('surf');
          break;
        case 'help':
          setMode(card.kind === 'reminder' ? 'how_off' : 'help');
          break;
        case 'not_it': {
          // The engine decides whether this was a pass for now or a request that waits; the pending list tells.
          const after = await nofapIpc.view().catch(() => null);
          const known = new Set(before?.status.pending.map((p) => p.id));
          const requested = !!after && after.status.pending.some((p) => p.kind === 'add_allow' && !known.has(p.id));
          acknowledge(requested ? t('nofap.notit.request', { delay: fmtDelay(after.config.delay_secs, t) }) : t('nofap.notit.snoozed'));
          break;
        }
        case 'request_exception':
          acknowledge(t('nofap.notit.request', { delay: fmtDelay(view?.config.delay_secs ?? 600, t) }));
          break;
        case 'add_to_list':
          acknowledge(t('nofap.card.added'));
          break;
        case 'ok':
        case 'later':
        case 'close_window':
        case 'restore':
        case 'restore_network':
        case 'keep_protection':
        case 'turn_off':
          close();
          break;
      }
    } finally {
      setBusy(false);
    }
  };

  const message = useMemo(() => {
    if (!card) return '';
    const vars = { name: view?.display_name ?? '', delay: fmtDelay(view?.config.delay_secs ?? 600, t) };
    // An opened discreet card gets one of the support sentences, the same one for as long as the card lives.
    const rotation = [...card.id].reduce((sum, ch) => sum + ch.charCodeAt(0), 0) % SUPPORT_MESSAGES.length;
    const key = mode === 'support' && card.message_key === 'block.discreet' ? (SUPPORT_MESSAGES[rotation] ?? SUPPORT_MESSAGES[0]) : card.message_key;
    return hasKey(`nofap.${key}`) ? t(`nofap.${key}`, vars) : t('nofap.block.discreet');
  }, [card, mode, view, t]);

  if (closed) return <div data-testid="card-closed" />;

  const plan = support?.plans[0];
  const withReasons = mode === 'support' || (mode === 'card' && card?.kind === 'blocked_repeat');
  const note = card && mode === 'card' && card.note_key && hasKey(`nofap.${card.note_key}`) ? t(`nofap.${card.note_key}`, { name: view?.display_name ?? '' }) : null;
  const panic: PanicView | null = mode === 'surf' || mode === 'away' || mode === 'talk' ? mode : null;
  const showsMessage = mode === 'card' || mode === 'support' || mode === 'done';

  return (
    <div className="fixed inset-0 bg-transparent" data-testid="page-nofap-card">
      <section data-tauri-drag-region aria-label={t('nofap.card.window_label')} className="relative flex h-full w-full flex-col overflow-hidden rounded-[16px] border border-line-2 bg-panel shadow-float">
        <div className="pointer-events-none absolute inset-0" style={{ background: 'var(--hero-glow)' }} aria-hidden />
        {card && (
          <div className="scroll-thin relative flex min-h-0 flex-1 flex-col gap-2.5 overflow-y-auto p-3.5">
            <div className="flex items-start gap-3" data-tauri-drag-region>
              <div className="flex shrink-0 items-center justify-center overflow-visible" style={{ width: 48, height: 48 }} aria-hidden>
                <Ubi variant="flat" size={48} mood="calm" />
              </div>
              {showsMessage ? (
                <div className="min-w-0 flex-1 self-center">
                  <p role="status" className="text-[14px] leading-5 text-ink" data-testid="card-message">
                    {mode === 'done' ? done : message}
                  </p>
                  {note && (
                    <p className="mt-1 text-xs leading-5 text-ink-2" data-testid="card-note">
                      {note}
                    </p>
                  )}
                </div>
              ) : (
                <p className="min-w-0 flex-1 self-center text-sm font-medium text-ink">UBI</p>
              )}
            </div>

            {withReasons && support?.values && (
              <div className="panel-raised p-3">
                <p className="text-xs font-medium text-ink-3">{t('nofap.card.values')}</p>
                <p className="mt-0.5 text-[13px] leading-5 text-ink">{support.values}</p>
              </div>
            )}
            {withReasons && plan && (
              <div className="panel-raised p-3">
                <p className="text-xs font-medium text-ink-3">{t('nofap.card.plan')}</p>
                <p className="mt-0.5 text-[13px] leading-5 text-ink">{t('nofap.plans.item', { when: plan.when, then: plan.then })}</p>
              </div>
            )}

            {mode === 'card' && (
              <div className="mt-auto flex flex-wrap justify-end gap-2" data-testid="card-actions">
                {/* Quiet buttons first, the one that carries the weight last, on the right. */}
                {[...card.actions.filter((a) => a !== PRIMARY[card.kind]), ...card.actions.filter((a) => a === PRIMARY[card.kind])].map((action) => (
                  <Button key={action} size="sm" variant={action === PRIMARY[card.kind] ? 'primary' : 'ghost'} disabled={busy} autoFocus={action === PRIMARY[card.kind]} onClick={() => void act(action)}>
                    {t(actionLabel(card, action))}
                  </Button>
                ))}
              </div>
            )}

            {mode === 'support' && (
              <div className="mt-auto flex flex-col gap-2" data-testid="card-support">
                <Button variant="primary" onClick={() => setMode('surf')}>
                  {t('nofap.btn.surf')}
                </Button>
                <Button
                  onClick={() => {
                    record({ event: 'step_away' });
                    setMode('away');
                  }}
                >
                  {t('nofap.btn.step_away')}
                </Button>
                <Button onClick={() => setMode('talk')}>{t('nofap.btn.talk')}</Button>
                {/* A correction the engine offers on this card stays reachable on its support view. */}
                {card.actions.filter(isCorrection).map((action) => (
                  <Button key={action} variant="ghost" disabled={busy} onClick={() => void act(action)}>
                    {t(actionLabel(card, action))}
                  </Button>
                ))}
                <Button variant="ghost" onClick={close}>
                  {t('nofap.btn.close')}
                </Button>
              </div>
            )}

            {panic && <PanicSheet key={panic} start={panic} countPanic={false} values={support?.values ?? ''} plans={support?.plans ?? []} resources={support?.resources ?? []} record={record} onClose={close} />}

            {mode === 'help' && (
              <div className="flex flex-col gap-3" data-testid="card-help">
                <p className="text-[13px] leading-5 text-ink">{t('nofap.help.how')}</p>
                <ResourceList resources={support?.resources ?? []} />
                <p className="text-xs leading-5 text-ink-3">{t('nofap.help.limits')}</p>
                <div className="flex justify-end">
                  <Button size="sm" onClick={close}>
                    {t('nofap.btn.close')}
                  </Button>
                </div>
              </div>
            )}

            {mode === 'how_off' && (
              <div className="flex flex-col gap-3" data-testid="card-how-off">
                <p className="text-[13px] leading-5 text-ink">{t('nofap.helptab.off.body')}</p>
                <div className="flex justify-end">
                  <Button size="sm" onClick={close}>
                    {t('nofap.btn.close')}
                  </Button>
                </div>
              </div>
            )}

            {mode === 'done' && (
              <div className="mt-auto flex justify-end">
                <Button size="sm" variant="primary" onClick={close}>
                  {t('nofap.btn.ok')}
                </Button>
              </div>
            )}
          </div>
        )}
      </section>
    </div>
  );
}
