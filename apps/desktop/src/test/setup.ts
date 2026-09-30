import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import { setLocale } from '../i18n';

// Tests assert Portuguese copy: pin the locale (jsdom reports navigator.language = en-US) before and after every test.
setLocale('pt-BR');
beforeEach(() => setLocale('pt-BR'));
afterEach(() => {
  cleanup();
  setLocale('pt-BR');
});

// jsdom lacks a few browser APIs used by charts, theme and the mascot probe.
class RO {
  observe() {}
  unobserve() {}
  disconnect() {}
}
(globalThis as unknown as { ResizeObserver: typeof RO }).ResizeObserver = RO;

if (!window.matchMedia) {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}

HTMLCanvasElement.prototype.getContext = (() => null) as unknown as typeof HTMLCanvasElement.prototype.getContext;
Element.prototype.scrollIntoView = vi.fn();
