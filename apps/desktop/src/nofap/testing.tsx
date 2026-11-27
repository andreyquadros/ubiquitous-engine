// Helpers shared by the module's tests: a fresh mock in a named scenario, a clean store, and the page in a router.

import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { NofapCardPage } from './NofapCard';
import { NofapPage } from './NofapPage';
import type { NofapHost } from './NofapWizard';
import { __nofapMock, type Scenario } from './mock';
import { useNofapStore } from './store';
import type { NofapCmd, NofapCommand, ProductId } from './types';

/** A fresh mock in `scenario`, hosted by ubiqX unless `product` says otherwise, and a clean store. */
export function resetNofap(scenario: Scenario, product: ProductId = 'ubiqx'): void {
  __nofapMock.reset(scenario);
  __nofapMock.setProduct(product);
  useNofapStore.setState({ view: null, error: null, sites: null, support: null, supportFailed: false });
}

export const renderPage = (host?: NofapHost, entry = '/nofap') =>
  render(
    <MemoryRouter initialEntries={[entry]}>
      <NofapPage host={host} />
    </MemoryRouter>,
  );

export const renderCard = (id: string) =>
  render(
    <MemoryRouter initialEntries={[`/nofap-card?id=${id}`]}>
      <NofapCardPage />
    </MemoryRouter>,
  );

/** The requests the mock received, optionally only those of one command. */
export function calls<C extends NofapCmd>(cmd?: C): Extract<NofapCommand, { cmd: C }>[] {
  const all = __nofapMock.calls();
  return (cmd ? all.filter((c) => c.cmd === cmd) : all) as Extract<NofapCommand, { cmd: C }>[];
}

/** Commands that only read. Everything else changes something. */
export const READS: NofapCmd[] = ['view', 'sites', 'support', 'system_preview', 'card', 'sample_frame'];
