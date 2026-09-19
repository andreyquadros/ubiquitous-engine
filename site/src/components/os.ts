/**
 * Which desktop the visitor is on, so the primary CTA offers the right installer instead of always the .dmg.
 * Uses the User-Agent Client Hints platform when the browser exposes it (Chromium) and falls back to the
 * user agent string. An unknown platform (or SSR/capture) keeps macOS, the project's reference build.
 */
export type Os = 'macos' | 'windows' | 'linux';

export const DEFAULT_OS: Os = 'macos';

/** Parses a platform/user-agent pair; exported so the mapping is testable without touching `navigator`. */
export function osFrom(platform: string, userAgent: string): Os {
  const s = `${platform} ${userAgent}`.toLowerCase();
  // Android reports "linux" in the UA but is not a desktop target: it falls through to the default.
  if (s.includes('android')) return DEFAULT_OS;
  if (s.includes('win')) return 'windows';
  if (s.includes('mac') || s.includes('iphone') || s.includes('ipad')) return 'macos';
  if (s.includes('linux') || s.includes('x11') || s.includes('ubuntu')) return 'linux';
  return DEFAULT_OS;
}

interface UaData {
  platform?: string;
}

export function detectOs(): Os {
  if (typeof navigator === 'undefined') return DEFAULT_OS;
  const data = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  return osFrom(data?.platform ?? '', navigator.userAgent ?? '');
}
