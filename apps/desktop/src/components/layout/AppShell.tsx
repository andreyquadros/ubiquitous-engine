import { useEffect } from 'react';
import { Navigate, Outlet, useLocation } from 'react-router-dom';
import { useEngineEvents } from '../../lib/engine';
import { useAppStore } from '../../lib/store';
import { Toaster } from '../../lib/toast';
import { Spinner } from '../ui/misc';
import { Sidebar } from './Sidebar';

/** Loads settings once and redirects to the onboarding while it is not done. */
export function Gate() {
  const settingsView = useAppStore((s) => s.settingsView);
  const settingsError = useAppStore((s) => s.settingsError);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const location = useLocation();

  useEffect(() => {
    if (!settingsView) void loadSettings();
  }, [settingsView, loadSettings]);

  if (settingsError) {
    return (
      <div className="flex h-screen items-center justify-center p-8 text-center">
        <div className="card max-w-md p-6">
          <h1 className="text-lg font-semibold">Não foi possível conectar ao motor do ubiqX</h1>
          <p className="mt-2 text-sm text-ink-2">{settingsError}</p>
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

export function AppShell() {
  useEngineEvents();
  const date = useAppStore((s) => s.date);
  const needsReview = useAppStore((s) => s.dashboards[date]?.needs_review ?? 0);
  const loadDashboard = useAppStore((s) => s.loadDashboard);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const loadCategories = useAppStore((s) => s.loadCategories);

  useEffect(() => {
    void loadDashboard(date);
  }, [date, dataVersion, loadDashboard]);

  useEffect(() => {
    void loadCategories();
  }, [loadCategories]);

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar needsReview={needsReview} />
      <main className="relative flex min-w-0 flex-1 flex-col">
        <div data-tauri-drag-region className="h-[38px] shrink-0" />
        <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-6 pb-10 min-[1180px]:px-8">
          <div className="mx-auto w-full max-w-[1320px]">
            <Outlet />
          </div>
        </div>
      </main>
      <Toaster />
    </div>
  );
}
