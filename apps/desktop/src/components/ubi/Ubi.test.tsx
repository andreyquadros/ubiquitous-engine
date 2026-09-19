import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Mood } from '../../lib/types';

// The lazy 3D chunk is replaced by a swappable stand-in: a canvas-free box, or a component that throws
// (a broken export / a blocked decoder / a lost WebGL context) so the fallback chain can be asserted.
const ubi3d = vi.hoisted(() => ({
  impl: (props: { mood: Mood; size: number }) => <div data-testid="ubi-3d" data-mood={props.mood} />,
  /** The last `bubbleRef` the 3D component received (the glance target). */
  bubbleRef: null as null | { current: HTMLElement | null },
}));
vi.mock('./Ubi3d', () => ({
  default: (props: { mood: Mood; size: number; bubbleRef?: { current: HTMLElement | null }; onReady?: () => void }) => {
    ubi3d.bubbleRef = props.bubbleRef ?? null;
    props.onReady?.();
    return ubi3d.impl(props);
  },
}));

import { Ubi, __resetProbe } from './Ubi';
import { getUbiStatus, __resetUbiStatus } from './status';

/**
 * jsdom loads no images, so `probePng` would hang: this stand-in answers whether the optional
 * `/ubi/ubi.png` is installed. The model itself is never probed — it ships inside the app.
 */
const servePng = (present: boolean) =>
  vi.stubGlobal(
    'Image',
    class {
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      naturalWidth = present ? 512 : 0;
      set src(_value: string) {
        queueMicrotask(() => (present ? this.onload?.() : this.onerror?.()));
      }
    },
  );

describe('Ubi', () => {
  const realGetContext = HTMLCanvasElement.prototype.getContext;
  const noWebgl = () => {
    HTMLCanvasElement.prototype.getContext = (() => null) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  };
  beforeEach(() => {
    __resetProbe();
    __resetUbiStatus();
    ubi3d.impl = (props) => <div data-testid="ubi-3d" data-mood={props.mood} />;
    servePng(false);
    // the setup file makes getContext return null (no WebGL in jsdom); the 3D tests pretend it works
    HTMLCanvasElement.prototype.getContext = (() => ({})) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    HTMLCanvasElement.prototype.getContext = realGetContext;
  });

  it('tries the model whenever WebGL is there, even with the PNG installed', async () => {
    servePng(true);
    render(<Ubi mood="calm" speaking="Olá!" />);
    expect(screen.getByText('Olá!')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', '3d'));
    await waitFor(() => expect(screen.getByTestId('ubi-3d')).toHaveAttribute('data-mood', 'calm'));
    expect(getUbiStatus()).toEqual({ mode: '3d', reason: null });
  });

  it('takes the PNG without WebGL, and says so', async () => {
    noWebgl();
    servePng(true);
    render(<Ubi mood="calm" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
    expect(getUbiStatus()).toEqual({ mode: 'png', reason: 'WebGL unavailable' });
  });

  it('draws the SVG without WebGL and without a PNG', async () => {
    noWebgl();
    render(<Ubi mood="focused" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'svg'));
    expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', 'focused');
    expect(screen.queryByTestId('ubi-3d')).not.toBeInTheDocument();
    expect(getUbiStatus().reason).toBe('WebGL unavailable');
  });

  it('falls back to the PNG when the model fails to load, keeping the reason', async () => {
    const quiet = vi.spyOn(console, 'error').mockImplementation(() => {});
    ubi3d.impl = () => {
      throw new Error('Could not load /ubi/Ubi.glb');
    };
    servePng(true);
    render(<Ubi mood="excited" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
    expect(screen.queryByTestId('ubi-3d')).not.toBeInTheDocument();
    expect(getUbiStatus()).toEqual({ mode: 'png', reason: 'Could not load /ubi/Ubi.glb' });
    quiet.mockRestore();
  });

  it('falls back to the SVG when the model fails and there is no PNG', async () => {
    const quiet = vi.spyOn(console, 'error').mockImplementation(() => {});
    ubi3d.impl = () => {
      throw new Error('WebGL context lost');
    };
    render(<Ubi mood="worried" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'svg'));
    expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', 'worried');
    expect(getUbiStatus()).toEqual({ mode: 'svg', reason: 'WebGL context lost' });
    quiet.mockRestore();
  });

  it('keeps the bubble ref on the mounted bubble across a speech change (the exiting bubble never nulls it)', async () => {
    const { rerender } = render(<Ubi mood="calm" speaking="Primeira frase" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', '3d'));
    await waitFor(() => expect(ubi3d.bubbleRef?.current).toBe(screen.getByText('Primeira frase')));
    // `key={speaking}` remounts the bubble: the new one takes the ref at once…
    rerender(<Ubi mood="calm" speaking="Segunda frase" />);
    expect(ubi3d.bubbleRef?.current).toBe(screen.getByText('Segunda frase'));
    // …and keeps it once the old one has finished its exit animation and unmounted
    await waitFor(() => expect(screen.queryByText('Primeira frase')).not.toBeInTheDocument(), { timeout: 4000 });
    expect(ubi3d.bubbleRef?.current).toBe(screen.getByText('Segunda frase'));
    // no bubble at all → null
    rerender(<Ubi mood="calm" />);
    await waitFor(() => expect(screen.queryByText('Segunda frase')).not.toBeInTheDocument(), { timeout: 4000 });
    expect(ubi3d.bubbleRef?.current).toBeNull();
  });

  it('keeps the flat variant on the PNG and never reports for the whole app', async () => {
    servePng(true);
    render(<Ubi mood="calm" size={26} variant="flat" crop="head" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
    expect(screen.queryByTestId('ubi-3d')).not.toBeInTheDocument();
    // the rail avatar is flat on purpose: it must not claim the mascot degraded
    expect(getUbiStatus()).toEqual({ mode: 'svg', reason: null });
  });

  it('renders every mood', () => {
    for (const mood of ['sleeping', 'calm', 'focused', 'excited', 'worried'] as const) {
      const { unmount } = render(<Ubi mood={mood} variant="svg" />);
      expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', mood);
      unmount();
    }
  });
});
