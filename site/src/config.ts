/**
 * Site configuration. Empty strings mean "not available yet": the matching button renders as
 * "Em breve" (disabled) with a hint. Fill these in when checkout exists.
 */
export const CHECKOUT_ANNUAL_URL = '';
export const CHECKOUT_MONTHLY_URL = '';

/** The rolling "continuous" release: stable names per OS, kept by scripts/publish-release.mjs. */
const CONTINUOUS = 'https://github.com/andreyquadros/ubiquitous-engine/releases/download/continuous';

export const DOWNLOAD_URLS = {
  macos: `${CONTINUOUS}/ubiqX-macos-aarch64.dmg`,
  windows: `${CONTINUOUS}/ubiqX-windows-x86_64-setup.exe`,
  linux: `${CONTINUOUS}/ubiqX-linux-x86_64.AppImage`,
} as const;

/** Secondary installers offered as text links under the cards. */
export const ALT_DOWNLOAD_URLS = {
  windows: `${CONTINUOUS}/ubiqX-windows-x86_64.msi`,
  linux: `${CONTINUOUS}/ubiqX-linux-x86_64.deb`,
} as const;

export const REPO_URL = 'https://github.com/andreyquadros/ubiquitous-engine';
export const RELEASES_URL = 'https://github.com/andreyquadros/ubiquitous-engine/releases';
export const DOCS_URL = 'https://github.com/andreyquadros/ubiquitous-engine/tree/main/docs';
export const MACOS_GUIDE_URL = 'https://github.com/andreyquadros/ubiquitous-engine/blob/main/docs/MACOS-TESTING.md';
export const WINDOWS_LINUX_GUIDE_URL = 'https://github.com/andreyquadros/ubiquitous-engine/blob/main/docs/WINDOWS-LINUX.md';

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

/** Terminal commands for the Linux download: run the AppImage, or install the .deb (docs/WINDOWS-LINUX.md). */
export const LINUX_INSTALL_COMMANDS = ['chmod +x ~/Downloads/ubiqX-linux-x86_64.AppImage && ~/Downloads/ubiqX-linux-x86_64.AppImage', 'sudo apt install ./ubiqX-linux-x86_64.deb'];
