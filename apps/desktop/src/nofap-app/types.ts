// Wire types of the standalone product's own command, `nofap_host` (`NofapHostCommand` in
// crates/ubiqx-app/src/nofap_app.rs). The module's own command and types are in `src/nofap/`.

import type { Locale } from '../i18n';
import type { BuildInfo, PermissionKind, PermissionStatus, UpdateStatus } from '../lib/types';
import type { AiProvider } from '../nofap/types';

export type { BuildInfo, PermissionKind, PermissionStatus, UpdateStatus };

/** The providers this product offers: the user's own key only, never "IA do Ubi" (design 8.5). */
export type OwnProvider = Exclude<AiProvider, 'ubi'>;
export const OWN_PROVIDERS: readonly OwnProvider[] = ['anthropic', 'openai', 'xai'];

/** `UiLanguage` on the wire (serde kebab-case): not the page's locale tag, which is `pt-BR`. */
export type UiLanguage = 'pt-br' | 'en';

export const wireLanguage = (locale: Locale): UiLanguage => (locale === 'en' ? 'en' : 'pt-br');
export const localeOf = (language: UiLanguage): Locale => (language === 'en' ? 'en' : 'pt-BR');

/** `settings_get`, and the reply of `settings_set`. */
export interface HostSettings {
  ui_language: UiLanguage;
  ai_provider: AiProvider;
  vision_model: string;
  check_updates: boolean;
  onboarding_done: boolean;
}

/** `settings_set`: only the fields given change. */
export type HostSettingsPatch = Partial<HostSettings>;

/** What the page writes: the language as a locale, turned into the wire form by `hostIpc`. */
export type SettingsPatch = Omit<HostSettingsPatch, 'ui_language'> & { ui_language?: Locale };

/** `about`. */
export interface HostAbout {
  display_name: string;
  identifier: string;
  build: BuildInfo;
}

export type HostCommand =
  | { cmd: 'settings_get' }
  | { cmd: 'settings_set'; args: HostSettingsPatch }
  | { cmd: 'api_key_set'; args: { provider: AiProvider; key: string } }
  | { cmd: 'api_key_clear'; args: { provider: AiProvider } }
  | { cmd: 'models'; args: { provider: AiProvider } }
  | { cmd: 'permissions' }
  | { cmd: 'permission_request'; args: { kind: PermissionKind } }
  | { cmd: 'update_check' }
  | { cmd: 'about' };

export type HostCmd = HostCommand['cmd'];
