import { AppWindow, Globe, Shield } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Ubi } from '../components/ubi/Ubi';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { useT } from '../i18n';
import { ipc, isTauri } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import type { Intervention } from '../lib/types';

/** How long the panel stays up before closing by itself. */
export const INTERVENTION_AUTO_CLOSE_MS = 7000;

/** Closes the intervention window in Tauri; in the browser only a popup opened by script can close itself. */
export async function closeInterventionWindow(): Promise<void> {
  if (isTauri()) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
    return;
  }
  if (typeof window !== 'undefined' && window.opener) window.close();
}

/** Sample shown for `?id=test` (the "Testar aviso" button). */
function sample(t: (key: string) => string): Intervention {
  return {
    id: 'test',
    at: new Date().toISOString(),
    target_id: null,
    kind: 'site',
    name: t('focus.intervention.sample_name'),
    key: 'youtube.com',
    action: 'tab_closed',
    session_id: null,
    message: t('focus.message.0'),
  };
}

/**
 * The floating intervention panel (second Tauri window, 460×188, no decorations): UBI worried on the left,
 * what he said as his bubble, the target as a chip and "Ok, foco!". Closes after 7 s or on the button.
 * Renders standalone in the browser at #/intervention?id=test.
 */
export function InterventionPage() {
  const t = useT();
  const [params] = useSearchParams();
  const id = params.get('id') ?? 'test';
  const loadSettings = useAppStore((s) => s.loadSettings);
  const settingsView = useAppStore((s) => s.settingsView);
  const [data, setData] = useState<Intervention | null>(null);

  // The shell language: Settings.language (applied by loadSettings) unless `?lang=` overrides it.
  useEffect(() => {
    if (!settingsView) void loadSettings();
  }, [settingsView, loadSettings]);

  useEffect(() => {
    let alive = true;
    if (id === 'test') {
      setData(sample(t));
      return;
    }
    void ipc
      .listInterventions(50)
      .then((list) => alive && setData(list.find((i) => i.id === id) ?? sample(t)))
      .catch(() => alive && setData(sample(t)));
    return () => {
      alive = false;
    };
  }, [id, t]);

  // The shell may re-show the window with a new intervention through an event instead of a navigation.
  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void import('@tauri-apps/api/event').then(({ listen }) =>
      listen<Intervention>('intervention', (ev) => setData(ev.payload)).then((u) => {
        if (disposed) u();
        else unlisten = u;
      }),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const close = useCallback(() => void closeInterventionWindow(), []);

  useEffect(() => {
    if (!data) return;
    const timer = setTimeout(close, INTERVENTION_AUTO_CLOSE_MS);
    return () => clearTimeout(timer);
  }, [data, close]);

  const KindIcon = data?.kind === 'app' ? AppWindow : Globe;

  return (
    <div className="fixed inset-0 bg-transparent" data-testid="page-intervention">
      <section
        data-tauri-drag-region
        aria-label={t('focus.intervention.window_label')}
        className="relative flex h-full w-full select-none items-center gap-4 overflow-hidden rounded-[16px] border border-line-2 bg-panel px-4 shadow-float"
      >
        <div className="pointer-events-none absolute inset-0" style={{ background: 'var(--hero-glow)' }} aria-hidden data-tauri-drag-region />
        <div className="relative shrink-0">
          <Ubi variant="auto" size={120} mood="worried" />
        </div>
        <div className="relative flex min-w-0 flex-1 flex-col gap-2.5">
          {data && (
            <>
              <div role="status" className="glass relative px-3 py-2 text-[13px] leading-5 text-ink" data-testid="intervention-message">
                {data.message}
                <span className="absolute top-1/2 -left-1.5 size-3 -translate-y-1/2 rotate-45 border-b border-l border-line-2 bg-[color-mix(in_oklab,var(--panel)_86%,transparent)]" aria-hidden />
              </div>
              <div className="flex items-center justify-between gap-3">
                <Badge tone="rose" title={data.key}>
                  <KindIcon className="size-3" strokeWidth={1.75} aria-hidden />
                  {data.name}
                </Badge>
                <Button variant="primary" size="sm" icon={<Shield className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={close} autoFocus>
                  {t('focus.intervention.ok')}
                </Button>
              </div>
            </>
          )}
        </div>
      </section>
    </div>
  );
}
