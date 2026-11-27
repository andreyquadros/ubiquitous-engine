import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { setLocale } from '../../i18n';
import { __mock } from '../../lib/mock';
import { useAppStore } from '../../lib/store';
import type { LicenseStatus } from '../../lib/types';
import { LICENSE_NAG_KEY, LicenseBanner, LicenseRequiredDialog, nagDismissedRecently, shouldNag, useLicenseGate } from './LicenseBanner';

function Where() {
  const loc = useLocation();
  return <output data-testid="where">{loc.pathname + ':' + String((loc.state as { section?: string } | null)?.section ?? '')}</output>;
}

const renderBanner = (extra?: React.ReactNode) =>
  render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route
          path="*"
          element={
            <>
              <LicenseBanner />
              {extra}
              <Where />
            </>
          }
        />
      </Routes>
    </MemoryRouter>,
  );

const loadLicense = () =>
  act(async () => {
    await useAppStore.getState().loadSettings();
  });

const DAY = 24 * 60 * 60 * 1000;

describe('LicenseBanner (soft enforcement, mock backend)', () => {
  beforeEach(() => {
    __mock.reset();
    localStorage.removeItem(LICENSE_NAG_KEY);
    useAppStore.setState({ settingsView: null, license: null });
  });
  afterEach(() => vi.restoreAllMocks());

  it('shows nothing until the license is known, then the reminder while unlicensed', async () => {
    renderBanner();
    expect(screen.queryByTestId('license-banner')).not.toBeInTheDocument();
    await loadLicense();
    const banner = await screen.findByTestId('license-banner');
    expect(banner).toHaveTextContent('O ubiqX está sem licença. Planos: anual R$ 197 (ou 10x de R$ 25) com a sua IA, ou R$ 49/mês com a IA do Ubi.');
    expect(banner).toHaveAttribute('data-mode', 'soft');
    expect(screen.getByRole('button', { name: 'Ver planos' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Já tenho uma chave' })).toBeInTheDocument();
  });

  it('"Ver planos" opens the site and "Já tenho uma chave" jumps to Settings › Licença', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    renderBanner();
    await loadLicense();
    fireEvent.click(await screen.findByRole('button', { name: 'Ver planos' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith('https://andreyquadros.github.io/ubiquitous-engine/#planos', '_blank', 'noopener'));
    fireEvent.click(screen.getByRole('button', { name: 'Já tenho uma chave' }));
    expect(screen.getByTestId('where')).toHaveTextContent('/settings:licenca');
  });

  it('dismissing hides it for 7 days (localStorage ubiqx.license_nag) and it comes back afterwards', async () => {
    renderBanner();
    await loadLicense();
    await screen.findByTestId('license-banner');
    fireEvent.click(screen.getByRole('button', { name: 'Dispensar por 7 dias' }));
    await waitFor(() => expect(screen.queryByTestId('license-banner')).not.toBeInTheDocument());
    const stored = Number(localStorage.getItem(LICENSE_NAG_KEY));
    expect(stored).toBeGreaterThan(Date.now() - 5000);
    expect(nagDismissedRecently()).toBe(true);
    expect(shouldNag(useAppStore.getState().license, stored, stored + 6 * DAY)).toBe(false);
    expect(shouldNag(useAppStore.getState().license, stored, stored + 7 * DAY)).toBe(true);
    // a stale dismissal no longer hides it
    localStorage.setItem(LICENSE_NAG_KEY, String(Date.now() - 8 * DAY));
    expect(nagDismissedRecently()).toBe(false);
  });

  it('disappears once a valid key is stored and says so when the license expired', async () => {
    renderBanner();
    await loadLicense();
    await screen.findByTestId('license-banner');
    await act(async () => {
      await useAppStore.getState().setLicenseKey(__mock.sampleLicenseKeys.annual);
    });
    await waitFor(() => expect(screen.queryByTestId('license-banner')).not.toBeInTheDocument());
    await act(async () => {
      await useAppStore.getState().setLicenseKey(__mock.sampleLicenseKeys.expired);
    });
    expect(await screen.findByTestId('license-banner')).toHaveTextContent('A licença do ubiqX expirou.');
  });

  it('follows the locale', async () => {
    renderBanner();
    await loadLicense();
    // the store applies Settings.language (pt-BR) on every get_settings: switch afterwards
    act(() => setLocale('en'));
    expect(await screen.findByTestId('license-banner')).toHaveTextContent('ubiqX has no license. Plans: yearly R$ 197 (or 10x R$ 25) with your AI, or R$ 49/month with the Ubi AI.');
    expect(screen.getByRole('button', { name: 'See plans' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'I already have a key' })).toBeInTheDocument();
  });
});

describe('LicenseBanner (hard enforcement branch)', () => {
  const hard: LicenseStatus = { state: 'unlicensed', plan: null, expires_at: null, days_left: null, key_hint: null, enforcement: 'hard', managed_usage: null };

  beforeEach(() => {
    __mock.reset();
    localStorage.setItem(LICENSE_NAG_KEY, String(Date.now()));
    useAppStore.setState({ settingsView: null, license: hard });
  });

  it('ignores the dismissal, explains that the AI is off and offers no close button', async () => {
    renderBanner();
    const banner = await screen.findByTestId('license-banner');
    expect(banner).toHaveAttribute('data-mode', 'hard');
    expect(banner).toHaveTextContent('A IA fica desligada até uma licença válida ser informada.');
    expect(screen.queryByRole('button', { name: 'Dispensar por 7 dias' })).not.toBeInTheDocument();
    expect(shouldNag(hard, Date.now())).toBe(true);
  });

  it('useLicenseGate replaces an AI action by the dialog while blocked, and runs it under soft enforcement', async () => {
    const run = vi.fn();
    function Action() {
      const { guard, dialog, blocked } = useLicenseGate();
      return (
        <>
          <button type="button" onClick={guard(run)} data-blocked={blocked}>
            gerar
          </button>
          {dialog}
        </>
      );
    }
    renderBanner(<Action />);
    fireEvent.click(await screen.findByRole('button', { name: 'gerar' }));
    expect(run).not.toHaveBeenCalled();
    const dialog = await screen.findByRole('dialog', { name: 'Licença necessária' });
    expect(dialog).toHaveTextContent('Esta ação usa a IA e, nesta versão, precisa de uma licença válida.');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Já tenho uma chave' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.getByTestId('where')).toHaveTextContent('/settings:licenca');

    act(() => useAppStore.setState({ license: { ...hard, enforcement: 'soft' } }));
    fireEvent.click(screen.getByRole('button', { name: 'gerar' }));
    expect(run).toHaveBeenCalledTimes(1);
  });

  it('LicenseRequiredDialog renders on its own', async () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <LicenseRequiredDialog open onClose={onClose} />
      </MemoryRouter>,
    );
    expect(await screen.findByRole('dialog', { name: 'Licença necessária' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Fechar' }));
    expect(onClose).toHaveBeenCalled();
  });
});
