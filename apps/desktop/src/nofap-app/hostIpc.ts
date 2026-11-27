// Typed wrapper around the standalone product's own command, `nofap_host` (whose single argument `command` is
// `{ cmd, args? }`), the window's quit, and the plugins the window uses (launch at login, opening a link, the in-app
// update). Outside Tauri every call goes to `./hostMock`.

import { isTauri } from '../lib/ipc';
import type { AiProvider } from '../nofap/types';
import { wireLanguage, type HostAbout, type HostCommand, type HostSettings, type PermissionKind, type PermissionStatus, type SettingsPatch, type UpdateStatus } from './types';

/** Name of the Tauri command (`apps/nofap/src-tauri/src/app.rs`). */
export const HOST_COMMAND = 'nofap_host';
/** The window's own way out: the shell counts the quit, gives up its lock and ends the process. */
export const QUIT_COMMAND = 'quit_app';
/** The event a newer build is announced under. */
export const UPDATE_EVENT = 'update';

async function send<T>(command: HostCommand): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(HOST_COMMAND, { command });
  }
  const mock = await import('./hostMock');
  return (await mock.handleHost(command)) as T;
}

export const hostIpc = {
  settings: () => send<HostSettings>({ cmd: 'settings_get' }),
  /** Only the fields given change; another provider brings its own default model. Returns the settings after the write. */
  saveSettings: ({ ui_language, ...rest }: SettingsPatch) => send<HostSettings>({ cmd: 'settings_set', args: { ...rest, ...(ui_language ? { ui_language: wireLanguage(ui_language) } : {}) } }),
  /** Checked with the vendor first; the key goes to the system's keychain, never into the settings. */
  setKey: (provider: AiProvider, key: string) => send<null>({ cmd: 'api_key_set', args: { provider, key } }),
  /** Stops the AI check at once (design 6.3). */
  clearKey: (provider: AiProvider) => send<null>({ cmd: 'api_key_clear', args: { provider } }),
  models: (provider: AiProvider) => send<string[]>({ cmd: 'models', args: { provider } }),
  permissions: () => send<PermissionStatus>({ cmd: 'permissions' }),
  requestPermission: (kind: PermissionKind) => send<null>({ cmd: 'permission_request', args: { kind } }),
  checkUpdates: () => send<UpdateStatus>({ cmd: 'update_check' }),
  about: () => send<HostAbout>({ cmd: 'about' }),

  /** Quits the app (after the page's confirmation). */
  async quit(): Promise<void> {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke(QUIT_COMMAND);
      return;
    }
    (await import('./hostMock')).mockShell.quit();
  },

  /** "Abrir junto com o computador": the operating system's login item is the setting itself. */
  async autostart(): Promise<boolean> {
    if (isTauri()) return (await import('@tauri-apps/plugin-autostart')).isEnabled();
    return (await import('./hostMock')).mockShell.autostart.get();
  },
  async setAutostart(on: boolean): Promise<void> {
    if (isTauri()) {
      const plugin = await import('@tauri-apps/plugin-autostart');
      await (on ? plugin.enable() : plugin.disable());
      return;
    }
    await (await import('./hostMock')).mockShell.autostart.set(on);
  },

  /** Opens a web address in the browser (a vendor's key page, a download). */
  async openUrl(url: string): Promise<void> {
    if (!/^https:\/\//.test(url)) return;
    if (isTauri()) {
      await (await import('@tauri-apps/plugin-opener')).openUrl(url);
      return;
    }
    (await import('./hostMock')).mockShell.open(url);
  },

  /**
   * Downloads and installs the build the updater feed lists, then reopens the app. `false` when the updater has
   * nothing for this build and platform (an unsigned build, a platform missing from the feed): the page then offers
   * the download instead.
   */
  async installUpdate(onProgress: (ratio: number | null) => void): Promise<boolean> {
    if (!isTauri()) return false;
    const { check } = await import('@tauri-apps/plugin-updater');
    const update = await check();
    if (!update) return false;
    let total: number | null = null;
    let done = 0;
    await update.downloadAndInstall((e) => {
      if (e.event === 'Started') total = e.data.contentLength ?? null;
      else if (e.event === 'Progress') done += e.data.chunkLength;
      onProgress(total ? Math.min(1, done / total) : null);
    });
    await (await import('@tauri-apps/plugin-process')).relaunch();
    return true;
  },

  /** Subscribes to the announcement of a newer build. Returns an unsubscribe function. */
  async onUpdate(handler: () => void): Promise<() => void> {
    if (!isTauri()) return () => undefined;
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen(UPDATE_EVENT, () => handler());
    return () => unlisten();
  },
};
