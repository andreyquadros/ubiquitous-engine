import { useEffect } from 'react';
import { Navigate, Outlet, useLocation } from 'react-router-dom';
import { useEngineEvents } from '../../lib/engine';
import { useT } from '../../i18n';
import { useAppStore } from '../../lib/store';
import { Toaster } from '../../lib/toast';
import { Spinner } from '../ui/misc';
import { Sidebar } from './Sidebar';
import { LicenseBanner } from './LicenseBanner';
import { UpdateBanner } from './UpdateBanner';

/** Loads settings once and redirects to the onboarding while it is not done. */
export function Gate() {
  const settingsView = useAppStore((s) => s.settingsView);
  const settingsError = useAppStore((s) => s.settingsError);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const location = useLocation();
  const t = useT();

  useEffect(() => {
    if (!settingsView) void loadSettings();
  }, [settingsView, loadSettings]);

  if (settingsError) {
    return (
      <div className="flex h-screen items-center justify-center p-8 text-center">
        <div className="panel max-w-md p-6">
          <h1 className="display text-lg">{t('nav.engine_error_title')}</h1>
          <p className="mt-2 text-sm text-ink-2">{settingsError}</p>
          <p className="mt-3 text-xs text-ink-3">{t('nav.engine_error_hint')}</p>
        </div>
      </div>
    );
  }
  if (!settingsView) {
    return (
      <div className="flex h-screen items-center justify-center">
        <Spinner />
      </div>
    );
  }
  if (!settingsView.settings.onboarding_done && location.pathname !== '/onboarding') return <Navigate to="/onboarding" replace />;
  return <Outlet />;
}

/** Left rail + scrolling content column (max 1280 px, 24 px gutters) + toasts. */
export function AppShell() {
  useEngineEvents();
  const date = useAppStore((s) => s.date);
  const needsReview = useAppStore((s) => s.dashboards[date]?.needs_review ?? 0);
  const loadDashboard = useAppStore((s) => s.loadDashboard);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const loadCategories = useAppStore((s) => s.loadCategories);
  const loadUpdateStatus = useAppStore((s) => s.loadUpdateStatus);

  useEffect(() => {
    void loadDashboard(date);
  }, [date, dataVersion, loadDashboard]);

  useEffect(() => {
    void loadCategories();
  }, [loadCategories]);

  // Once per shell mount; afterwards the `update_available` event keeps it fresh.
  useEffect(() => {
    void loadUpdateStatus();
  }, [loadUpdateStatus]);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-canvas">
      <Sidebar needsReview={needsReview} />
      <main className="relative flex min-w-0 flex-1 flex-col">
        <div data-tauri-drag-region className="h-[38px] shrink-0" />
        <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-6 pb-10">
          <div className="mx-auto w-full max-w-[1280px]">
            <UpdateBanner />
            <LicenseBanner />
            <Outlet />
          </div>
        </div>
      </main>
      <Toaster />
    </div>
  );
}
