// In-memory implementation of the standalone product's own command (`nofap_host`), of its window-only actions
// (quit, launch at login, opening a link, the in-app update) and of the module's command, used whenever the page is
// not running inside Tauri (`vite dev --mode nofap` in a browser, Vitest, Playwright screenshots).
//
// In the standalone build `vite.config.ts` resolves every import of `src/lib/mock` to this file, so the module's
// `ipc.ts` reaches `handle` below and ubiqX's own mock never enters the bundle (`NOFAP_MOCK_REDIRECT`).
//
// It keeps the module's mock in step with what a real host would tell the service: the provider and whether the AI
// check is ready (onboarding done and a key stored).

import { __nofapMock, handle as handleNofap } from '../nofap/mock';
import type { AiProvider, AiReadiness, NofapCommand } from '../nofap/types';
import { initialLocale } from '../i18n/locale';
import type { BuildInfo, HostAbout, HostCommand, HostSettings, PermissionStatus, UpdateStatus } from './types';

/** Model ids the mock offers per provider; the first is the default (`AiModels::for_provider`). */
export const MOCK_MODELS: Record<AiProvider, string[]> = {
  anthropic: ['claude-haiku-4-5', 'claude-sonnet-4-5'],
  openai: ['gpt-5-mini', 'gpt-5'],
  xai: ['grok-4-1-fast-non-reasoning', 'grok-4'],
  ubi: ['ubi-fast'],
};

const DEV_BUILD: BuildInfo = { version: '0.1.0', epoch: 0, number: 0, sha: 'dev', branch: '' };

interface HostState {
  settings: HostSettings;
  keys: Set<AiProvider>;
  permissions: PermissionStatus;
  autostart: boolean;
  update: UpdateStatus;
  /** Set by the window's quit button; the page cannot end a browser tab. */
  quitRequested: boolean;
  opened: string[];
}

/** A key that the pretend vendor refuses, for the error path. */
export const MOCK_BAD_KEY = 'sk-invalid';

function fresh(onboarded: boolean): HostState {
  return {
    settings: { ui_language: initialLocale(), ai_provider: 'anthropic', vision_model: MOCK_MODELS.anthropic[0]!, check_updates: true, onboarding_done: onboarded },
    keys: new Set(onboarded ? (['anthropic'] as AiProvider[]) : []),
    permissions: { screen_recording: 'not_applicable', automation: 'not_applicable', accessibility: 'not_applicable' },
    autostart: false,
    update: { current: DEV_BUILD, feed_url: 'https://example.com/latest.json', enabled: false, available: null, dismissed: false, last_check: null, last_error: null, checking: false },
    quitRequested: false,
    opened: [],
  };
}

/** A preview that names a module scenario (`?nofap=on`) starts past the welcome; a bare address starts fresh. */
const ONBOARDED_FROM_QUERY: boolean = (() => {
  try {
    if (typeof window === 'undefined') return false;
    const q = new URLSearchParams(window.location.search);
    return q.get('host') === 'onboarded' || (q.has('nofap') && q.get('host') !== 'fresh');
  } catch {
    return false;
  }
})();

let H: HostState = fresh(ONBOARDED_FROM_QUERY);
let calls: HostCommand[] = [];

const readiness = (): AiReadiness => (!H.settings.onboarding_done ? 'not_onboarded' : H.settings.ai_provider === 'ubi' ? 'licence_required' : H.keys.has(H.settings.ai_provider) ? 'ready' : 'no_key');

/** Tells the module's mock what the real host's environment would answer. */
const sync = (): void => __nofapMock.setAi(H.settings.ai_provider, readiness());
sync();

const refuseManaged = (provider: AiProvider): void => {
  if (provider === 'ubi') throw new Error('invalid: the managed provider is not offered in this product');
};

/** Answers one `nofap_host` request the way `NofapApp::handle_host` does. */
export async function handleHost(command: HostCommand): Promise<unknown> {
  calls.push(structuredClone(command));
  switch (command.cmd) {
    case 'settings_get':
      return { ...H.settings };
    case 'settings_set': {
      const patch = command.args;
      const next = { ...H.settings };
      if (patch.ui_language) next.ui_language = patch.ui_language;
      if (patch.ai_provider && patch.ai_provider !== next.ai_provider) {
        refuseManaged(patch.ai_provider);
        next.ai_provider = patch.ai_provider;
        next.vision_model = MOCK_MODELS[patch.ai_provider][0]!;
      }
      if (patch.vision_model !== undefined) {
        const model = patch.vision_model.trim();
        if (!model) throw new Error('invalid: vision model: empty');
        if (!MOCK_MODELS[next.ai_provider].includes(model)) throw new Error(`invalid: vision model ${model} is not a model of ${next.ai_provider}`);
        next.vision_model = model;
      }
      if (patch.check_updates !== undefined) next.check_updates = patch.check_updates;
      if (patch.onboarding_done !== undefined) next.onboarding_done = patch.onboarding_done;
      H.settings = next;
      sync();
      return { ...next };
    }
    case 'api_key_set': {
      refuseManaged(command.args.provider);
      const key = command.args.key.trim();
      if (!key) throw new Error('invalid: api key: empty');
      if (key === MOCK_BAD_KEY) throw new Error('ai_rejected: the provider refused this key');
      H.keys.add(command.args.provider);
      sync();
      return null;
    }
    case 'api_key_clear':
      refuseManaged(command.args.provider);
      H.keys.delete(command.args.provider);
      sync();
      return null;
    case 'models':
      refuseManaged(command.args.provider);
      if (!H.keys.has(command.args.provider)) throw new Error('ai_not_configured: AI provider is not configured (missing API key)');
      return [...MOCK_MODELS[command.args.provider]];
    case 'permissions':
      return { ...H.permissions };
    case 'permission_request':
      H.permissions = { ...H.permissions, [command.args.kind]: 'granted' };
      return null;
    case 'update_check':
      H.update = { ...H.update, last_check: new Date().toISOString(), last_error: null };
      return structuredClone(H.update);
    case 'about':
      return { display_name: displayName(), identifier: 'ai.ubiqx.shield', build: DEV_BUILD } satisfies HostAbout;
  }
}

/** The display name the module's mock sends, the same one the real app has from the engine. */
function displayName(): string {
  return (handleNofap({ cmd: 'view' }) as { display_name: string }).display_name;
}

/** The shape of `lib/mock`'s `handle`, which the module's `ipc.ts` calls outside Tauri (see the header). */
export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  if (cmd === 'nofap') return handleNofap(args.command as NofapCommand) as T;
  if (cmd === 'nofap_host') return (await handleHost(args.command as HostCommand)) as T;
  throw new Error(`other: unknown command ${cmd}`);
}

/** The window-only actions of the shell and its plugins. */
export const mockShell = {
  quit(): void {
    H.quitRequested = true;
  },
  autostart: {
    get: async (): Promise<boolean> => H.autostart,
    set: async (on: boolean): Promise<void> => {
      H.autostart = on;
    },
  },
  open(url: string): void {
    H.opened.push(url);
  },
};

export const __hostMock = {
  /** A fresh host: onboarded or not, with a stored Anthropic key or none (by default, one when onboarded). */
  reset(onboarded = ONBOARDED_FROM_QUERY, withKey = onboarded): void {
    H = fresh(onboarded);
    if (!withKey) H.keys.clear();
    calls = [];
    sync();
  },
  setPermissions(patch: Partial<PermissionStatus>): void {
    H.permissions = { ...H.permissions, ...patch };
  },
  setUpdate(patch: Partial<UpdateStatus>): void {
    H.update = { ...H.update, ...structuredClone(patch) };
  },
  calls: (): HostCommand[] => structuredClone(calls),
  state: (): Readonly<HostState> => H,
};

declare global {
  interface Window {
    __hostMock?: typeof __hostMock;
  }
}

if (typeof window !== 'undefined') window.__hostMock = __hostMock;
