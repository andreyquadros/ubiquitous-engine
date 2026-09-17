import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it } from 'vitest';

import { __mock } from '../lib/mock';
import { useAppStore } from '../lib/store';
import { SettingsPage } from './Settings';

describe('Settings · IA section (mock backend)', () => {
  beforeEach(async () => {
    __mock.reset();
    useAppStore.setState({ settingsView: null });
    await useAppStore.getState().loadSettings();
  });

  it('picks the provider, lists the account models and restores the defaults', async () => {
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );
    const group = await screen.findByRole('radiogroup', { name: 'Provedor de IA' });
    expect(within(group).getByRole('radio', { name: /Anthropic Claude/ })).toHaveAttribute('aria-checked', 'true');
    // only anthropic has a key in the mock → one key icon
    expect(within(group).getAllByLabelText('chave configurada')).toHaveLength(1);
    expect(screen.getByText('Chave configurada')).toBeInTheDocument();
    expect(screen.getByText('…f3a9')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Listar modelos da conta/ }));
    await screen.findByText(/3 modelos disponíveis na conta/);
    expect(document.querySelectorAll('datalist option')).toHaveLength(3);

    // free-text model field
    const report = screen.getByLabelText('Modelo de relatórios') as HTMLInputElement;
    fireEvent.change(report, { target: { value: 'claude-opus-5' } });
    expect(report.value).toBe('claude-opus-5');
    fireEvent.click(screen.getByRole('button', { name: /Padrões do provedor/ }));
    expect((screen.getByLabelText('Modelo de relatórios') as HTMLInputElement).value).toBe('claude-sonnet-5');

    // switching to xAI: no key yet → listing disabled, defaults adopted, key form for xAI
    fireEvent.click(within(group).getByRole('radio', { name: /xAI Grok/ }));
    expect(screen.getByRole('button', { name: /Listar modelos da conta/ })).toBeDisabled();
    expect((screen.getByLabelText('Modelo de classificação') as HTMLInputElement).value).toBe('grok-4-1-fast-non-reasoning');
    expect(screen.getByTestId('api-key-form-xai')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('xai-…')).toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('xai'), { timeout: 3000 });
    expect(useAppStore.getState().settingsView?.ai_health).toEqual({ state: 'not_configured' });

    // saving an xAI key enables the listing again
    fireEvent.change(screen.getByPlaceholderText('xai-…'), { target: { value: 'xai-abcdefghijklmnopqrstuvwxyz9f2c' } });
    fireEvent.click(screen.getByRole('button', { name: 'Validar e salvar' }));
    await screen.findByText('…9f2c');
    expect(screen.getByRole('button', { name: /Listar modelos da conta/ })).toBeEnabled();
    expect(screen.getByText(/Para a IA escolhida \(Anthropic, OpenAI ou xAI\) vão apenas/)).toBeInTheDocument();
  });
});
