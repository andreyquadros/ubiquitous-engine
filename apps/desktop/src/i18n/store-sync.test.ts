import { beforeEach, describe, expect, it } from 'vitest';
import { __mock } from '../lib/mock';
import { useAppStore } from '../lib/store';
import { getLocale, setLocale } from './index';

describe('app store locale sync', () => {
  beforeEach(async () => {
    __mock.reset();
    setLocale('pt-BR');
    useAppStore.setState({ settingsView: null });
    await useAppStore.getState().loadSettings();
  });

  it('applies Settings.language once settings load', () => {
    expect(useAppStore.getState().settingsView?.settings.language).toBe('pt-BR');
    expect(getLocale()).toBe('pt-BR');
    useAppStore.getState().applySettingsView({ ...useAppStore.getState().settingsView!, settings: { ...useAppStore.getState().settingsView!.settings, language: 'en-US' } });
    expect(getLocale()).toBe('en');
  });

  it('setLanguage switches now and persists through the backend', async () => {
    const view = await useAppStore.getState().setLanguage('en');
    expect(getLocale()).toBe('en');
    expect(document.documentElement.lang).toBe('en');
    expect(view?.settings.language).toBe('en');
    // the (mock) backend echoes the language back and a reload keeps it
    useAppStore.setState({ settingsView: null });
    setLocale('pt-BR');
    await useAppStore.getState().loadSettings();
    expect(useAppStore.getState().settingsView?.settings.language).toBe('en');
    expect(getLocale()).toBe('en');
  });
});
