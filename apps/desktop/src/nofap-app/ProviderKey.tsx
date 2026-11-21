// The AI provider and the user's own key, in the welcome and in the settings. Only Anthropic, OpenAI and xAI are
// offered: "IA do Ubi" and licence keys are not in this product (design 8.5). The key is checked with the vendor
// and goes to the system's keychain; the page never reads it back.

import clsx from 'clsx';
import { ExternalLink, KeyRound } from 'lucide-react';
import { useState } from 'react';
import { Button } from '../components/ui/Button';
import { Field, Input } from '../components/ui/Field';
import { useT } from '../i18n';
import { ipcErrorMessage } from '../lib/ipc';
import { useToast } from '../lib/toast';
import { providerName } from '../nofap/labels';
import { useNofapStore } from '../nofap/store';
import type { AiProvider } from '../nofap/types';
import { hostIpc } from './hostIpc';
import { useHostStore } from './hostStore';
import { OWN_PROVIDERS, type OwnProvider } from './types';

/** Where each vendor's console creates a key (`AiProvider::console_url`). */
export const KEY_PAGES: Record<OwnProvider, string> = {
  anthropic: 'https://console.anthropic.com/settings/keys',
  openai: 'https://platform.openai.com/api-keys',
  xai: 'https://console.x.ai',
};

/** What the page knows about the key of the chosen provider. */
type KeyState = 'saved' | 'missing' | 'unknown';

export function ProviderKey({ onSaved }: { onSaved?: () => void }) {
  const t = useT();
  const toast = useToast();
  const settings = useHostStore((s) => s.settings);
  const save = useHostStore((s) => s.save);
  const ai = useNofapStore((s) => s.view?.ai);
  const loadNofap = useNofapStore((s) => s.load);
  const [key, setKey] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** A key saved from this page while the module cannot tell yet (before the welcome ends). */
  const [savedFor, setSavedFor] = useState<AiProvider | null>(null);

  if (!settings) return null;
  const provider = settings.ai_provider;
  const state: KeyState = savedFor === provider || ai === 'ready' ? 'saved' : ai === 'no_key' ? 'missing' : 'unknown';
  const name = providerName(provider, t);

  const choose = async (next: OwnProvider) => {
    if (next === provider) return;
    setError(null);
    try {
      await save({ ai_provider: next });
    } catch (e) {
      setError(ipcErrorMessage(e));
    }
  };

  const store = async () => {
    setBusy(true);
    setError(null);
    try {
      await hostIpc.setKey(provider, key);
      setKey('');
      setSavedFor(provider);
      await loadNofap();
      toast.success(t('nofap_app.ai.saved', { provider: name }));
      onSaved?.();
    } catch (e) {
      setError(ipcErrorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    setError(null);
    try {
      await hostIpc.clearKey(provider);
      setSavedFor(null);
      await loadNofap();
      toast.info(t('nofap_app.ai.cleared'));
    } catch (e) {
      setError(ipcErrorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-4" data-testid="provider-key">
      <div className="flex flex-col gap-1.5">
        <span className="text-[13px] font-medium text-ink" id="provider-label">
          {t('nofap_app.ai.provider')}
        </span>
        <div role="radiogroup" aria-labelledby="provider-label" className="inline-flex flex-wrap gap-1 self-start rounded-control border border-line bg-panel-2 p-1">
          {OWN_PROVIDERS.map((id) => (
            <button
              key={id}
              type="button"
              role="radio"
              aria-checked={id === provider}
              onClick={() => void choose(id)}
              className={clsx('h-7 rounded-[7px] px-3 text-sm font-medium transition-colors duration-150', id === provider ? 'bg-panel text-ink shadow-[inset_0_0_0_1px_var(--line-2)]' : 'text-ink-2 hover:text-ink')}
            >
              {providerName(id, t)}
            </button>
          ))}
        </div>
        <p className="text-xs text-ink-3">{t('nofap_app.ai.own_key_only')}</p>
      </div>

      <Field label={t('nofap_app.ai.key', { provider: name })} hint={t('nofap_app.ai.key_hint')}>
        {(id) => (
          <div className="flex flex-wrap gap-2">
            <Input
              id={id}
              type="password"
              autoComplete="off"
              spellCheck={false}
              value={key}
              placeholder={t('nofap_app.ai.key_placeholder')}
              onChange={(e) => setKey(e.target.value)}
              className="min-w-0 flex-1 basis-56"
            />
            <Button variant="primary" icon={<KeyRound className="size-4" strokeWidth={1.75} aria-hidden />} loading={busy} disabled={!key.trim()} onClick={() => void store()} data-testid="key-save">
              {t('nofap_app.ai.save')}
            </Button>
          </div>
        )}
      </Field>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-xs text-ink-2" role="status" data-testid="key-state">
          {state === 'saved' ? t('nofap_app.ai.state_saved', { provider: name }) : state === 'missing' ? t('nofap_app.ai.state_missing', { provider: name }) : ''}
        </p>
        <div className="flex flex-wrap gap-2">
          {provider !== 'ubi' && (
            <Button size="sm" variant="ghost" icon={<ExternalLink className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void hostIpc.openUrl(KEY_PAGES[provider])}>
              {t('nofap_app.ai.get_key', { provider: name })}
            </Button>
          )}
          {state === 'saved' && (
            <Button size="sm" onClick={() => void remove()} disabled={busy} data-testid="key-clear">
              {t('nofap_app.ai.clear')}
            </Button>
          )}
        </div>
      </div>

      {error && (
        <p role="alert" className="rounded-control border border-rose/40 bg-rose/10 px-3 py-2 text-[13px] leading-5 text-ink">
          {error}
        </p>
      )}
    </div>
  );
}
