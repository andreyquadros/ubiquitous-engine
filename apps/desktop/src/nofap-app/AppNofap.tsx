// The standalone product's frontend: its own frame (navigation, welcome, settings) around the module's pages from
// `src/nofap/`. It imports no page of ubiqX and nothing of the tracker (a test walks the module graph).

import { useEffect, useMemo } from 'react';
import { HashRouter, Navigate, Outlet, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Spinner } from '../components/ui/misc';
import { useT } from '../i18n';
import { Toaster } from '../lib/toast';
import { NofapCardPage } from '../nofap/NofapCard';
import { NofapPage } from '../nofap/NofapPage';
import type { NofapHost } from '../nofap/NofapWizard';
import { useNofapStore } from '../nofap/store';
import { useAutostart } from './autostart';
import { useHostStore } from './hostStore';
import { NofapSettings } from './NofapSettings';
import { NofapShell } from './NofapShell';
import { Welcome } from './Welcome';

/** Loads the settings row once; sends a first start to the welcome. */
function HostGate() {
  const t = useT();
  const settings = useHostStore((s) => s.settings);
  const error = useHostStore((s) => s.error);
  const load = useHostStore((s) => s.load);
  const loadNofap = useNofapStore((s) => s.load);
  const location = useLocation();

  useEffect(() => {
    void load();
    void loadNofap();
  }, [load, loadNofap]);

  let body;
  if (error && !settings) {
    body = (
      <div className="flex h-screen items-center justify-center p-8 text-center">
        <div className="panel max-w-md p-6" role="alert">
          <h1 className="display text-lg">{t('nofap_app.start.failed')}</h1>
          <p className="mt-2 text-sm text-ink-2">{error}</p>
          <div className="mt-4">
            <Button onClick={() => void load()}>{t('nofap_app.start.retry')}</Button>
          </div>
        </div>
      </div>
    );
  } else if (!settings) {
    body = (
      <div className="flex h-screen items-center justify-center">
        <Spinner />
      </div>
    );
  } else if (!settings.onboarding_done && location.pathname !== '/welcome') {
    body = <Navigate to="/welcome" replace />;
  } else if (settings.onboarding_done && location.pathname === '/welcome') {
    body = <Navigate to="/" replace />;
  } else {
    body = <Outlet />;
  }
  return (
    <>
      {body}
      <Toaster />
    </>
  );
}

/** The module's page with what this host lends it: launch at login and the way to its settings. */
function ModulePage() {
  const navigate = useNavigate();
  const [autostart, setAutostart] = useAutostart();
  const host = useMemo<NofapHost>(
    () => ({
      autostart: autostart === undefined ? undefined : { value: autostart, onChange: (on) => void setAutostart(on).catch(() => undefined) },
      openSettings: () => navigate('/settings'),
    }),
    [autostart, setAutostart, navigate],
  );
  return <NofapPage host={host} />;
}

/** The floating card window: outside the gate and the frame. It follows the app's language. */
function StandaloneCard() {
  const settings = useHostStore((s) => s.settings);
  const load = useHostStore((s) => s.load);
  useEffect(() => {
    if (!settings) void load();
  }, [settings, load]);
  return <NofapCardPage />;
}

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/nofap-card" element={<StandaloneCard />} />
      <Route element={<HostGate />}>
        <Route path="/welcome" element={<Welcome />} />
        <Route element={<NofapShell />}>
          <Route index element={<ModulePage />} />
          {/* The module's tabs, by name (the tray's "Preciso de um respiro" opens the support tab). */}
          <Route path="/support" element={<Navigate to="/?tab=support" replace />} />
          <Route path="/lists" element={<Navigate to="/?tab=lists" replace />} />
          <Route path="/help" element={<Navigate to="/?tab=help" replace />} />
          <Route path="/settings" element={<NofapSettings />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Route>
    </Routes>
  );
}

export default function AppNofap() {
  return (
    <HashRouter>
      <AppRoutes />
    </HashRouter>
  );
}
