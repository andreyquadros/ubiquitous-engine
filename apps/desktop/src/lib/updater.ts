// In-app update: `tauri-plugin-updater` downloads and installs the signed artifact listed by the
// updater feed (`plugins.updater.endpoints` in src-tauri/tauri.conf.json) and `tauri-plugin-process`
// relaunches the app. The announcement feed (`latest.json`, `ipc.getUpdateStatus`) is a different
// thing and stays as it is: it is what draws the banner, the tray item and the Settings section.
//
// When the page is not running inside Tauri (plain `vite dev` in a browser, tests, screenshots) the
// whole flow is served by `./mock`, exactly like `./ipc`.

import { isTauri } from './ipc';

/** Bytes already downloaded and, when the server sent a `Content-Length`, the total. */
export interface UpdateProgress {
  downloaded: number;
  /** `null` when the server did not say how big the artifact is. */
  total: number | null;
}

/** An update the plugin is willing to install, already matched against the running version. */
export interface InAppUpdate {
  /** Version of the build being installed, as the updater feed names it. */
  version: string;
  /** Release notes of that build, when the feed carries them. */
  notes: string | null;
  /** Downloads (reporting progress) and installs. On macOS and Linux the app must then relaunch. */
  downloadAndInstall: (onProgress: (p: UpdateProgress) => void) => Promise<void>;
  /** Frees the handle when the user walks away without installing. */
  close: () => Promise<void>;
}

/**
 * What the in-app install is doing right now.
 *
 * `unavailable` is not a failure: it means the plugin found nothing to install for this build and
 * platform (an unsigned build, a feed without this platform), so the UI offers the manual installer.
 */
export type InstallPhase = 'idle' | 'checking' | 'downloading' | 'installing' | 'relaunching' | 'unavailable' | 'failed';

export interface UpdateInstall {
  phase: InstallPhase;
  downloaded: number;
  total: number | null;
  /** Message of the failure behind `phase: 'failed'`. */
  error: string | null;
}

export const IDLE_INSTALL: UpdateInstall = { phase: 'idle', downloaded: 0, total: null, error: null };

/** True while the install is running, so the UI keeps the button busy and the bar on screen. */
export const isInstalling = (i: UpdateInstall): boolean => i.phase === 'checking' || i.phase === 'downloading' || i.phase === 'installing' || i.phase === 'relaunching';

/**
 * Asks the plugin what it would install, or `null` when there is nothing for this build. Throws when
 * the feed cannot be read (offline, malformed JSON, bad signature).
 */
export async function checkInAppUpdate(): Promise<InAppUpdate | null> {
  if (!isTauri()) {
    const mock = await import('./mock');
    return mock.mockUpdater.check();
  }
  const { check } = await import('@tauri-apps/plugin-updater');
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    notes: update.body ?? null,
    async downloadAndInstall(onProgress) {
      let downloaded = 0;
      let total: number | null = null;
      await update.downloadAndInstall((e) => {
        if (e.event === 'Started') {
          total = e.data.contentLength ?? null;
          downloaded = 0;
        } else if (e.event === 'Progress') {
          downloaded += e.data.chunkLength;
        } else {
          downloaded = total ?? downloaded;
        }
        onProgress({ downloaded, total });
      });
    },
    close: () => update.close(),
  };
}

/** Quits and reopens the app on the freshly installed version (a no-op on Windows, which already exited). */
export async function relaunchApp(): Promise<void> {
  if (!isTauri()) {
    const mock = await import('./mock');
    return mock.mockUpdater.relaunch();
  }
  const { relaunch } = await import('@tauri-apps/plugin-process');
  await relaunch();
}

/**
 * Whether installing this build replaces a bundle macOS ties permissions to. CI signs the macOS app
 * ad hoc (no stable Developer ID), so the new bundle is a different app to the system and Screen
 * Recording and Automation have to be granted again — see docs/MACOS-TESTING.md § 3.2. The UI warns
 * before installing; `kind` is the installer of the feed entry this machine would take.
 */
export const losesMacPermissions = (kind: string | null | undefined): boolean => kind === 'dmg';

/** "1,2 MB" / "820 kB" for the progress line; `Intl` gives the decimal comma in pt-BR. */
export function fmtBytes(bytes: number, locale: string): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  if (bytes < 1000) return `${Math.round(bytes)} B`;
  const units = ['kB', 'MB', 'GB'];
  let value = bytes / 1000;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  return `${value.toLocaleString(locale, { maximumFractionDigits: value < 10 ? 1 : 0 })} ${units[unit]}`;
}

/** 0…1 for the progress bar; `null` while the total is unknown (an indeterminate bar). */
export const installRatio = (i: UpdateInstall): number | null => {
  if (i.total === null || i.total <= 0) return null;
  return Math.min(1, Math.max(0, i.downloaded / i.total));
};
