import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { setLocale } from '../../i18n';
import { __mock } from '../../lib/mock';
import { useAppStore } from '../../lib/store';
import { UpdateBanner, fmtBuildDate, hasPendingUpdate } from './UpdateBanner';

function Where() {
  const loc = useLocation();
  return <output data-testid="where">{loc.pathname + ':' + String((loc.state as { section?: string } | null)?.section ?? '')}</output>;
}

const renderBanner = () =>
  render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route
          path="*"
          element={
            <>
              <UpdateBanner />
              <Where />
            </>
          }
        />
      </Routes>
    </MemoryRouter>,
  );

describe('UpdateBanner (mock backend)', () => {
  beforeEach(() => {
    __mock.reset();
    useAppStore.setState({ updateStatus: null });
  });
  afterEach(() => vi.restoreAllMocks());

  it('is hidden while the mock serves no update', async () => {
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    expect(useAppStore.getState().updateStatus?.available).toBeNull();
    expect(screen.queryByTestId('update-banner')).not.toBeInTheDocument();
  });

  it('shows the new build, downloads the dmg and hides after "Depois"', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    __mock.setUpdate(true);
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    const banner = await screen.findByTestId('update-banner');
    expect(banner).toHaveTextContent('Nova versão do ubiqX disponível: build');
    expect(banner).toHaveTextContent('(a1b2c3d)');
    expect(banner).toHaveTextContent(fmtBuildDate(1758221040, 'dev'));

    fireEvent.click(screen.getByRole('button', { name: 'Baixar (.dmg)' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith(expect.stringMatching(/ubiqX-macos-aarch64\.dmg$/), '_blank', 'noopener'));

    fireEvent.click(screen.getByRole('button', { name: 'Depois' }));
    await waitFor(() => expect(screen.queryByTestId('update-banner')).not.toBeInTheDocument());
    await waitFor(() => expect(useAppStore.getState().updateStatus?.dismissed).toBe(true));
    expect(hasPendingUpdate(useAppStore.getState().updateStatus)).toBe(false);
  });

  it('"Como instalar" jumps to the updates section of Settings', async () => {
    __mock.setUpdate(true);
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    fireEvent.click(await screen.findByRole('button', { name: 'Como instalar' }));
    expect(screen.getByTestId('where')).toHaveTextContent('/settings:atualizacoes');
  });

  it('appears when the engine pushes update_available and follows the locale', async () => {
    setLocale('en');
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    expect(screen.queryByTestId('update-banner')).not.toBeInTheDocument();
    // Like the engine: the status carries the release before the event goes out.
    __mock.setUpdate(true);
    act(() => {
      useAppStore.getState().applyEvent({ type: 'update_available', release: __mock.sampleRelease() });
    });
    expect(await screen.findByTestId('update-banner')).toHaveTextContent('New ubiqX version available: build');
    expect(screen.getByRole('button', { name: 'Download (.dmg)' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Later' })).toBeInTheDocument();
  });

  it('does not come back for a dismissed build when the engine announces it again', async () => {
    __mock.setUpdate(true);
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    await screen.findByTestId('update-banner');
    await act(async () => {
      await useAppStore.getState().dismissUpdate();
    });
    await waitFor(() => expect(screen.queryByTestId('update-banner')).not.toBeInTheDocument());

    const shown: boolean[] = [];
    const unsubscribe = useAppStore.subscribe((s) => shown.push(hasPendingUpdate(s.updateStatus)));
    // The scheduled check after a relaunch and every manual check re-emit the event for the same build.
    await act(async () => {
      useAppStore.getState().applyEvent({ type: 'update_available', release: __mock.sampleRelease() });
      await useAppStore.getState().checkForUpdates();
      await new Promise((r) => setTimeout(r, 50));
    });
    unsubscribe();
    expect(shown.every((v) => !v)).toBe(true);
    expect(screen.queryByTestId('update-banner')).not.toBeInTheDocument();
    expect(useAppStore.getState().updateStatus?.dismissed).toBe(true);
  });

  it('stays hidden on development builds', async () => {
    __mock.setUpdate(true);
    renderBanner();
    await act(async () => {
      const s = await useAppStore.getState().loadUpdateStatus();
      useAppStore.setState({ updateStatus: s ? { ...s, enabled: false, current: { ...s.current, epoch: 0, number: 0, sha: 'dev' } } : null });
    });
    await waitFor(() => expect(screen.queryByTestId('update-banner')).not.toBeInTheDocument());
    expect(fmtBuildDate(0, 'dev')).toBe('dev');
  });
});
