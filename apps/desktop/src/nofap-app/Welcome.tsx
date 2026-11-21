// The first start: the language, what this app is and is not, and the optional AI key. Its last button marks the
// onboarding done and leads straight into the module's consent wizard (design 5.2), which is the module page while
// protection is off. Nothing is enabled here.

import { ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { LanguageSelect } from '../components/ui/LanguageSelect';
import { Progress } from '../components/ui/misc';
import { setLocale, useLocale, useT, type Locale } from '../i18n';
import { ipcErrorMessage } from '../lib/ipc';
import { useNofapStore } from '../nofap/store';
import { useHostStore } from './hostStore';
import { ProviderKey } from './ProviderKey';

const STEPS = 2;

export function Welcome() {
  const t = useT();
  const locale = useLocale();
  const navigate = useNavigate();
  const save = useHostStore((s) => s.save);
  const name = useNofapStore((s) => s.view?.display_name);
  const [step, setStep] = useState(1);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const chooseLanguage = (next: Locale) => {
    setLocale(next);
    void save({ ui_language: next }).catch(() => undefined);
  };

  const finish = async () => {
    setBusy(true);
    setError(null);
    try {
      await save({ ui_language: locale, onboarding_done: true });
      navigate('/', { replace: true });
    } catch (e) {
      setError(ipcErrorMessage(e));
      setBusy(false);
    }
  };

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-canvas">
      <div data-tauri-drag-region className="h-[38px] shrink-0" />
      <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-6 pb-10">
        <Card className="mx-auto mt-4 w-full max-w-[640px]" data-testid="welcome">
          <header className="mb-5">
            <div className="flex items-center gap-3">
              <span className="flex size-10 shrink-0 items-center justify-center rounded-control border border-line-2 bg-panel-2" aria-hidden>
                <ShieldCheck className="size-5 text-volt" strokeWidth={1.75} />
              </span>
              <div className="min-w-0">
                <p className="eyebrow">{t('nofap_app.welcome.step', { step, total: STEPS })}</p>
                <h1 className="display mt-0.5 text-xl">{step === 1 ? (name ? t('nofap_app.welcome.title', { name }) : t('nofap_app.welcome.title_plain')) : t('nofap_app.welcome.ai_title')}</h1>
              </div>
            </div>
            <Progress className="mt-4" value={step} max={STEPS} height="h-1" />
          </header>

          {step === 1 && (
            <div className="flex flex-col gap-4" data-testid="welcome-step-1">
              <p className="text-[15px] leading-6 text-ink">{t('nofap.page.subtitle')}</p>
              <p className="text-sm leading-6 text-ink-2">{t('nofap_app.welcome.no_tracker')}</p>
              <p className="text-sm leading-6 text-ink-2">{t('nofap_app.welcome.next_steps')}</p>
              <div className="flex flex-col gap-1.5 border-t border-line pt-4">
                <span className="text-[13px] font-medium text-ink">{t('nofap_app.welcome.language')}</span>
                <LanguageSelect value={locale} onChange={chooseLanguage} className="self-start" />
              </div>
            </div>
          )}

          {step === 2 && (
            <div className="flex flex-col gap-4" data-testid="welcome-step-2">
              <p className="text-sm leading-6 text-ink-2">{t('nofap_app.welcome.ai_body')}</p>
              <ProviderKey />
              <p className="text-xs leading-5 text-ink-3">{t('nofap_app.welcome.ai_later')}</p>
            </div>
          )}

          {error && (
            <p role="alert" className="mt-4 rounded-control border border-rose/40 bg-rose/10 px-3 py-2 text-[13px] leading-5 text-ink">
              {error}
            </p>
          )}

          <footer className="mt-6 flex flex-wrap items-center justify-end gap-2 border-t border-line pt-4">
            {step > 1 && (
              <Button variant="ghost" onClick={() => setStep(1)} disabled={busy}>
                {t('nofap_app.welcome.back')}
              </Button>
            )}
            {step < STEPS ? (
              <Button variant="primary" onClick={() => setStep(2)}>
                {t('nofap_app.welcome.next')}
              </Button>
            ) : (
              <Button variant="primary" loading={busy} onClick={() => void finish()} data-testid="welcome-finish">
                {t('nofap_app.welcome.finish')}
              </Button>
            )}
          </footer>
        </Card>
      </div>
    </div>
  );
}
