// Typed wrapper around the module's one Tauri command, `nofap`, whose single argument `command` is
// `{ cmd, args? }` (`NofapCommand` in ubiqx-core). Outside Tauri (plain `vite dev`, tests, screenshots) the
// same request goes to the in-memory mock through `lib/mock`, like every other command of the app.

import { isTauri } from '../lib/ipc';
import type {
  ApplyOutcome,
  CardAction,
  ChangeOutcome,
  ConsentArgs,
  EnableOptions,
  ExportResult,
  Id,
  NofapCard,
  NofapChange,
  NofapCommand,
  NofapView,
  PlannedChange,
  SampleFrame,
  SitesView,
  SupportView,
  UserEvent,
} from './types';

/** Name of the Tauri command and of the Tauri event. */
export const NOFAP_COMMAND = 'nofap';
export const NOFAP_EVENT = 'nofap';

async function send<T>(command: NofapCommand): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(NOFAP_COMMAND, { command });
  }
  const mock = await import('../lib/mock');
  return mock.handle<T>(NOFAP_COMMAND, { command });
}

export const nofapIpc = {
  view: () => send<NofapView>({ cmd: 'view' }),
  /** The wizard's last button; may raise the OS prompt. */
  enable: (options: EnableOptions) => send<NofapView>({ cmd: 'enable', args: options }),
  consentVision: (consent: ConsentArgs) => send<NofapView>({ cmd: 'consent_vision', args: consent }),
  /** Takes effect now (`applied`) or waits out the delay (`pending`). */
  request: (change: NofapChange) => send<ChangeOutcome>({ cmd: 'request', args: change }),
  confirmPending: (id: Id) => send<NofapView>({ cmd: 'confirm_pending', args: { id } }),
  /** Always immediate. */
  cancelPending: (id: Id) => send<NofapView>({ cmd: 'cancel_pending', args: { id } }),
  systemPreview: () => send<PlannedChange[]>({ cmd: 'system_preview' }),
  /** "Aplicar", "Restaurar", "Religar"; raises the OS prompt. */
  systemApply: () => send<ApplyOutcome>({ cmd: 'system_apply' }),
  /** Only while the module is off; raises the OS prompt. */
  systemRemove: () => send<ApplyOutcome>({ cmd: 'system_remove' }),
  /** "Restaurar minha rede": never delayed; raises the OS prompt. */
  restoreNetwork: () => send<ApplyOutcome>({ cmd: 'restore_network' }),
  sites: () => send<SitesView>({ cmd: 'sites' }),
  support: () => send<SupportView>({ cmd: 'support' }),
  /** Counters only. */
  record: (event: UserEvent) => send<null>({ cmd: 'record', args: event }),
  /** An unknown id gives the neutral discreet card. */
  card: (id: Id) => send<NofapCard>({ cmd: 'card', args: { id } }),
  cardAction: (id: Id, action: CardAction) => send<null>({ cmd: 'card_action', args: { id, action } }),
  /** The last candidate window, never sent, never stored; `null` when none is remembered. */
  sampleFrame: () => send<SampleFrame | null>({ cmd: 'sample_frame' }),
  /** Never delayed. */
  eraseHistory: () => send<null>({ cmd: 'erase_history' }),
  /** Only while the module is off. */
  reset: () => send<null>({ cmd: 'reset' }),
  export: () => send<ExportResult>({ cmd: 'export' }),
  /** A preview card that counts nothing. */
  testCard: () => send<null>({ cmd: 'test_card' }),
};
