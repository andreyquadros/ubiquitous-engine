import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Mood } from '../../lib/types';

// The lazy 3D chunk is replaced by a swappable stand-in: a canvas-free box, or a component that throws
// (a broken export / a lost WebGL context) so the fallback chain can be asserted.
const ubi3d = vi.hoisted(() => ({
  impl: (props: { mood: Mood; size: number }) => <div data-testid="ubi-3d" data-mood={props.mood} />,
}));
vi.mock('./Ubi3d', () => ({ default: (props: { mood: Mood; size: number }) => ubi3d.impl(props) }));

import { Ubi, __resetProbe } from './Ubi';

const ok = (type: string) => new Response(null, { status: 200, headers: { 'content-type': type } });
const missing = () => new Response(null, { status: 404 });
/** HEAD responses per asset: the GLB, the PNG, both or none. */
const serve = (assets: { glb?: boolean; png?: boolean }) =>
  vi.fn(async (url: string) => {
    const u = String(url);
    if (u.endsWith('.glb')) return assets.glb ? ok('model/gltf-binary') : missing();
    if (u.endsWith('.png')) return assets.png ? ok('image/png') : missing();
    return missing();
  });

describe('Ubi', () => {
  const realGetContext = HTMLCanvasElement.prototype.getContext;
  beforeEach(() => {
    __resetProbe();
    ubi3d.impl = (props) => <div data-testid="ubi-3d" data-mood={props.mood} />;
    vi.stubGlobal('fetch', serve({}));
    // the setup file makes getContext return null (no WebGL in jsdom); the 3D tests pretend it works
    HTMLCanvasElement.prototype.getContext = (() => ({})) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    HTMLCanvasElement.prototype.getContext = realGetContext;
  });

  it('falls back to the inline SVG when nothing is installed', async () => {
    render(<Ubi mood="focused" speaking="Olá!" />);
    expect(screen.getByTestId('ubi-svg')).toBeInTheDocument();
    expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', 'focused');
    expect(screen.getByText('Olá!')).toBeInTheDocument();
    await waitFor(() => expect(fetch).toHaveBeenCalled());
    expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'svg');
    expect(screen.queryByTestId('ubi-3d')).not.toBeInTheDocument();
  });

  it('prefers the 3D model when /ubi/Ubi.glb exists (even with the PNG installed)', async () => {
    vi.stubGlobal('fetch', serve({ glb: true, png: true }));
    render(<Ubi mood="calm" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', '3d'));
    await waitFor(() => expect(screen.getByTestId('ubi-3d')).toHaveAttribute('data-mood', 'calm'));
  });

  it('uses the PNG when only /ubi/ubi.png exists', async () => {
    vi.stubGlobal('fetch', serve({ png: true }));
    render(<Ubi mood="calm" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
  });

  it('skips the model without WebGL and takes the PNG', async () => {
    HTMLCanvasElement.prototype.getContext = (() => null) as unknown as typeof HTMLCanvasElement.prototype.getContext;
    vi.stubGlobal('fetch', serve({ glb: true, png: true }));
    render(<Ubi mood="calm" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
  });

  it('falls back to the PNG when the model fails to load', async () => {
    const quiet = vi.spyOn(console, 'error').mockImplementation(() => {});
    ubi3d.impl = () => {
      throw new Error('Could not load /ubi/Ubi.glb');
    };
    vi.stubGlobal('fetch', serve({ glb: true, png: true }));
    render(<Ubi mood="excited" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
    expect(screen.queryByTestId('ubi-3d')).not.toBeInTheDocument();
    quiet.mockRestore();
  });

  it('falls back to the SVG when the model fails and there is no PNG', async () => {
    const quiet = vi.spyOn(console, 'error').mockImplementation(() => {});
    ubi3d.impl = () => {
      throw new Error('WebGL context lost');
    };
    vi.stubGlobal('fetch', serve({ glb: true }));
    render(<Ubi mood="worried" />);
    await waitFor(() => expect(fetch).toHaveBeenCalled());
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'svg'));
    expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', 'worried');
    quiet.mockRestore();
  });

  it('keeps the flat variant on the PNG (no WebGL for the rail avatar)', async () => {
    vi.stubGlobal('fetch', serve({ glb: true, png: true }));
    render(<Ubi mood="calm" size={26} variant="flat" crop="head" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
    expect(screen.queryByTestId('ubi-3d')).not.toBeInTheDocument();
  });

  it('renders every mood', () => {
    for (const mood of ['sleeping', 'calm', 'focused', 'excited', 'worried'] as const) {
      const { unmount } = render(<Ubi mood={mood} variant="svg" />);
      expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', mood);
      unmount();
    }
  });
});
