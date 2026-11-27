import type { Mood } from '../../lib/types';

/** How the mascot is being drawn right now. `3d` is the model, the others are fallbacks. */
export type UbiMode = 'svg' | 'png' | '3d';

export interface UbiStatus {
  mode: UbiMode;
  /** Why the model was not used, when it was not: shown in Settings → Sobre. */
  reason: string | null;
}

/**
 * The mascot's presentation, published for Settings → Sobre.
 *
 * The 3D model degrades silently by design (a lost WebGL context or a bad export must never take the
 * app down), which once cost a long blind hunt: the packaged app showed the flat drawing and nothing
 * anywhere said why. Release builds have no devtools, so the reason has to reach the UI.
 */
let status: UbiStatus = { mode: 'svg', reason: null };
const listeners = new Set<() => void>();

export const getUbiStatus = (): UbiStatus => status;

export function setUbiStatus(next: UbiStatus): void {
  if (next.mode === status.mode && next.reason === status.reason) return;
  status = next;
  listeners.forEach((l) => l());
}

export function subscribeUbiStatus(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Test hook. */
export const __resetUbiStatus = (): void => setUbiStatus({ mode: 'svg', reason: null });

/** The label shown in Settings → Sobre, e.g. "3D (modelo)" or "SVG · WebGL indisponível". */
export function ubiStatusLabel(s: UbiStatus, t: (key: string) => string): string {
  const mode = t(`settings.about.ubi.${s.mode}`);
  return s.reason ? `${mode} · ${s.reason}` : mode;
}

export type { Mood };
