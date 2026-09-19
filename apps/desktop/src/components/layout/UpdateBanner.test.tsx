import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { setLocale } from '../../i18n';
import { __mock } from '../../lib/mock';
import { useAppStore } from '../../lib/store';
import { IDLE_INSTALL } from '../../lib/updater';
import { UpdateBanner, downloadLabel, fmtBuildDate, hasPendingUpdate } from './UpdateBanner';

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
    useAppStore.setState({ updateStatus: null, updateInstall: IDLE_INSTALL });
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

  it('names the installer of the running OS on the download button', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    __mock.setPlatform('windows');
    __mock.setUpdate(true);
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    expect(useAppStore.getState().updateStatus?.available?.kind).toBe('exe');
    fireEvent.click(await screen.findByRole('button', { name: 'Baixar (.exe)' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith(expect.stringMatching(/ubiqX-windows-x86_64-setup\.exe$/), '_blank', 'noopener'));
    expect(screen.queryByRole('button', { name: 'Baixar (.dmg)' })).not.toBeInTheDocument();

    // Linux: the AppImage is the primary installer of the feed entry
    __mock.setPlatform('linux');
    act(() => {
      useAppStore.getState().applyEvent({ type: 'update_available', release: __mock.sampleRelease() });
    });
    expect(await screen.findByRole('button', { name: 'Baixar (.AppImage)' })).toBeInTheDocument();
    expect(downloadLabel((k) => k, 'deb')).toBe('updates.download.deb');
  });

  it('"Atualizar agora" warns about the macOS permissions, then downloads, installs and reopens the app', async () => {
    __mock.setUpdate(true);
    __mock.setUpdateInstall({ totalBytes: 24_000_000 });
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    await screen.findByTestId('update-banner');

    // macOS (the feed entry is a .dmg): the first click only arms the warning, nothing is installed yet.
    fireEvent.click(screen.getByRole('button', { name: 'Atualizar agora' }));
    const warning = await screen.findByTestId('update-macos-warning');
    expect(warning).toHaveTextContent('Gravação de Tela e Automação terão de ser concedidas de novo');
    expect(__mock.state().update.install.installed).toBe(0);

    // Cancelling puts the button back without touching anything.
    fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));
    expect(await screen.findByRole('button', { name: 'Atualizar agora' })).toBeInTheDocument();
    expect(__mock.state().update.install.installed).toBe(0);

    fireEvent.click(screen.getByRole('button', { name: 'Atualizar agora' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Atualizar mesmo assim' }));

    // The progress bar reports bytes over total while the plugin downloads…
    const bar = await screen.findByRole('progressbar', { name: 'Progresso da atualização' });
    expect(bar).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId('update-install-progress')).toHaveTextContent(/Baixando .* de 24 MB/));
    // …and the app installs and reopens itself, without any manual download.
    await waitFor(() => expect(__mock.state().update.install.installed).toBe(1));
    await waitFor(() => expect(__mock.state().update.install.relaunched).toBe(1));
    expect(useAppStore.getState().updateInstall.phase).toBe('relaunching');
    expect(screen.queryByTestId('update-install-error')).not.toBeInTheDocument();
  });

  it('installs straight away off macOS and shows a download without a known size', async () => {
    __mock.setPlatform('linux');
    __mock.setUpdate(true);
    __mock.setUpdateInstall({ totalBytes: null });
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    await screen.findByTestId('update-banner');
    fireEvent.click(await screen.findByRole('button', { name: 'Atualizar agora' }));
    expect(screen.queryByTestId('update-macos-warning')).not.toBeInTheDocument();
    const bar = await screen.findByRole('progressbar', { name: 'Progresso da atualização' });
    expect(bar).not.toHaveAttribute('aria-valuenow');
    await waitFor(() => expect(screen.getByTestId('update-install-progress')).toHaveTextContent('Baixando…'));
    await waitFor(() => expect(__mock.state().update.install.relaunched).toBe(1));
  });

  it('shows a failure and keeps the manual installer one click away', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    __mock.setPlatform('windows');
    __mock.setUpdate(true);
    __mock.setUpdateInstall({ failure: 'servidor fora do ar' });
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    await screen.findByTestId('update-banner');
    fireEvent.click(await screen.findByRole('button', { name: 'Atualizar agora' }));

    const error = await screen.findByTestId('update-install-error');
    expect(error).toHaveTextContent('A atualização automática falhou: servidor fora do ar');
    expect(useAppStore.getState().updateInstall.phase).toBe('failed');
    expect(__mock.state().update.install.relaunched).toBe(0);
    expect(screen.getByRole('button', { name: 'Tentar de novo' })).toBeInTheDocument();

    // The manual route is still right there.
    fireEvent.click(screen.getByRole('button', { name: 'Baixar (.exe)' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith(expect.stringMatching(/ubiqX-windows-x86_64-setup\.exe$/), '_blank', 'noopener'));
  });

  it('says so when this build cannot update itself', async () => {
    __mock.setUpdate(true);
    __mock.setUpdateInstall({ supported: false });
    renderBanner();
    await act(async () => {
      await useAppStore.getState().loadUpdateStatus();
    });
    await screen.findByTestId('update-banner');
    fireEvent.click(await screen.findByRole('button', { name: 'Atualizar agora' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Atualizar mesmo assim' }));
    const error = await screen.findByTestId('update-install-error');
    expect(error).toHaveTextContent('Este build não consegue se atualizar sozinho. Baixe o instalador e instale à mão.');
    expect(useAppStore.getState().updateInstall.phase).toBe('unavailable');
    expect(screen.queryByRole('button', { name: 'Tentar de novo' })).not.toBeInTheDocument();
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
