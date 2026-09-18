import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

import { Ubi, __resetProbe } from './Ubi';

describe('Ubi', () => {
  beforeEach(() => {
    __resetProbe();
    vi.stubGlobal('fetch', vi.fn(async () => new Response(null, { status: 404 })));
  });
  afterEach(() => vi.unstubAllGlobals());

  it('falls back to the inline SVG when the GLB is missing', async () => {
    render(<Ubi mood="focused" speaking="Olá!" />);
    expect(screen.getByTestId('ubi-svg')).toBeInTheDocument();
    expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', 'focused');
    expect(screen.getByText('Olá!')).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByTestId('ubi-3d')).not.toBeInTheDocument());
  });

  it('prefers the PNG when /ubi/ubi.png exists', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string) => (String(url).endsWith('.png') ? new Response(null, { status: 200, headers: { 'content-type': 'image/png' } }) : new Response(null, { status: 404 }))),
    );
    render(<Ubi mood="calm" />);
    await waitFor(() => expect(screen.getByTestId('ubi')).toHaveAttribute('data-ubi-mode', 'png'));
  });

  it('renders every mood', () => {
    for (const mood of ['sleeping', 'calm', 'focused', 'excited', 'worried'] as const) {
      const { unmount } = render(<Ubi mood={mood} variant="svg" />);
      expect(screen.getByTestId('ubi-svg')).toHaveAttribute('data-mood', mood);
      unmount();
    }
  });
});
