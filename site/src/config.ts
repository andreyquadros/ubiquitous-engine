/**
 * Site configuration. Empty strings mean "not available yet": the matching button renders as
 * "Em breve" (disabled) with a hint. Fill these in when checkout and the other builds exist.
 */
export const CHECKOUT_ANNUAL_URL = '';
export const CHECKOUT_MONTHLY_URL = '';

export const DOWNLOAD_URLS = {
  macos: 'https://github.com/andreyquadros/ubiquitous-engine/releases/download/continuous/ubiqX-macos-aarch64.dmg',
  windows: '',
  linux: '',
} as const;

export const REPO_URL = 'https://github.com/andreyquadros/ubiquitous-engine';
export const RELEASES_URL = 'https://github.com/andreyquadros/ubiquitous-engine/releases';
export const DOCS_URL = 'https://github.com/andreyquadros/ubiquitous-engine/tree/main/docs';
export const MACOS_GUIDE_URL = 'https://github.com/andreyquadros/ubiquitous-engine/blob/main/docs/MACOS-TESTING.md';

/** Shown in the footer and the FAQ when set. */
export const CONTACT_EMAIL = '';

/** Absolute site URL (origin + base), injected at build time from SITE_URL / SITE_BASE. */
declare const __SITE_URL__: string;
export const SITE_URL: string = typeof __SITE_URL__ === 'string' ? __SITE_URL__ : '/';

/** Terminal commands for a macOS download that Gatekeeper reports as damaged (docs/MACOS-TESTING.md, 3.1). */
export const MACOS_FIX_COMMANDS = [
  'xattr -dr com.apple.quarantine /Applications/ubiqX.app',
  'codesign --force --deep --options runtime --sign "ubiqX Dev" /Applications/ubiqX.app',
];
