// @ts-expect-error this package has no @types/node (vite.config.ts does the same for node:process)
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
// The word list the Rust tests read, as it is in the repository.
import forbidden from '../../../../crates/ubiqx-nofap/data/forbidden-words.json';
import tauriConf from '../../src-tauri/tauri.conf.json?raw';
import { hasKey, NAMESPACES, translate } from '../i18n';
import { PANIC_CRISIS } from '../i18n/messages/nofap';
import type { Locale } from '../i18n/types';
import { consentKey, providerName } from './labels';

// The design document itself. Vite refuses `?raw` for a file outside the app, so the test reads it from disk.
const here = (import.meta as unknown as { dirname: string }).dirname;
const design: string = readFileSync(`${here}/../../../../docs/nofap/DESIGN.md`, 'utf8');

const LOCALES: Locale[] = ['pt-BR', 'en'];
const nofap = NAMESPACES.nofap!;

/** Every string the module can show, per language: both namespaces and the crisis sentence. */
function strings(locale: Locale): [string, string][] {
  const out: [string, string][] = [];
  for (const ns of ['nofap', 'nofap_app']) for (const [key, text] of Object.entries(NAMESPACES[ns]![locale])) out.push([`${ns}.${key}`, text]);
  out.push(['nofap.panic.crisis', PANIC_CRISIS[locale]]);
  return out;
}

/** The rows of the tables of section 5.6: key, PT-BR, English. */
function copyBank(): [string, string, string][] {
  const section = design.slice(design.indexOf('### 5.6 Copy bank'), design.indexOf('### 5.7 Help resources'));
  const rows: [string, string, string][] = [];
  for (const line of section.split('\n')) {
    const m = /^\| `([a-z_.0-9]+)`[^|]*\| (.+) \| (.+) \|$/.exec(line);
    if (m) rows.push([m[1]!, m[2]!.trim(), m[3]!.trim()]);
  }
  return rows;
}

describe('the copy bank (design 5.6)', () => {
  const rows = copyBank();

  it('is read from the design in full', () => {
    expect(rows).toHaveLength(68);
    expect(rows.map((r) => r[0])).toContain('block.discreet');
  });

  it('has every string with the exact text in both languages', () => {
    for (const [key, pt, en] of rows) {
      const found = key === 'panic.crisis' ? PANIC_CRISIS : { 'pt-BR': nofap['pt-BR'][key], en: nofap.en[key] };
      expect(found['pt-BR'], `${key} (pt-BR)`).toBe(pt);
      expect(found.en, `${key} (en)`).toBe(en);
    }
  });

  it('names the product only through the {name} placeholder', () => {
    const withName = rows.filter(([, pt]) => pt.includes('{name}')).map((r) => r[0]);
    expect(withName).toEqual(expect.arrayContaining(['wizard.what', 'vision.consent', 'exclusion.delay', 'drift.notice', 'reminder.on', 'privacy.at_rest']));
  });
});

describe('the consent sentence with the managed provider (integrator decision 24)', () => {
  /** The sentences of a text, each without its closing full stop. */
  const sentences = (text: string): string[] => text.split(/\. (?=[A-ZÀ-Ý{])/u);
  const filled = (locale: Locale, provider: 'anthropic' | 'ubi'): string => translate(locale, consentKey(provider), { name: 'NoFap', provider: providerName(provider, (key, vars) => translate(locale, key, vars)) });

  it('uses the copy-bank sentence for a vendor and its own for "IA do Ubi"', () => {
    expect(consentKey('anthropic')).toBe('nofap.vision.consent');
    expect(consentKey('openai')).toBe('nofap.vision.consent');
    expect(consentKey('xai')).toBe('nofap.vision.consent');
    expect(consentKey('ubi')).toBe('nofap.vision.consent_ubi');
    for (const locale of LOCALES) expect(hasKey('nofap.vision.consent_ubi', locale), locale).toBe(true);
  });

  it('reads correctly in Portuguese: the article, the gender, and whose rules apply', () => {
    const text = filled('pt-BR', 'ubi');
    expect(text).toContain('o NoFap tira uma foto reduzida só dessa janela e pergunta à IA do Ubi se há pornografia nela.');
    expect(text).toContain('A IA do Ubi envia a imagem à Anthropic, que aplica as próprias regras ao que recebe e guarda pelo prazo que os termos dela definem.');
    expect(text).not.toMatch(/\bdele\b/);
    expect(text).not.toContain('pergunta a IA');
    expect(text).not.toMatch(/[{}]/);
  });

  it('reads correctly in English', () => {
    const text = filled('en', 'ubi');
    expect(text).toContain('NoFap takes a reduced picture of that window only and asks Ubi AI whether it shows pornography.');
    expect(text).toContain('Ubi AI sends the image on to Anthropic, which applies its own rules to what it receives and keeps it for the period its terms state.');
    expect(text).not.toMatch(/[{}]/);
  });

  it('differs from the copy-bank sentence only where the provider is named', () => {
    for (const locale of LOCALES) {
      const bank = sentences(nofap[locale]['vision.consent']!);
      const managed = sentences(nofap[locale]['vision.consent_ubi']!);
      expect(managed, locale).toHaveLength(bank.length);
      const differing = bank.map((sentence, i) => (sentence === managed[i] ? null : i)).filter((i) => i !== null);
      // The first sentence (who is asked) and the fourth (who receives the image).
      expect(differing, locale).toEqual(locale === 'pt-BR' ? [0, 3] : [3]);
      for (const i of differing) expect(bank[i], `${locale} sentence ${i}`).toContain('{provider}');
    }
  });
});

const fold = (text: string): string =>
  text
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase();

/** A forbidden entry as a whole-word pattern; an entry ending in `*` is a stem. */
function pattern(entry: string): RegExp {
  const stem = entry.endsWith('*');
  const body = fold(stem ? entry.slice(0, -1) : entry)
    .replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    .replace(/\s+/g, '\\s+');
  return new RegExp(`(^|[^a-z0-9])${body}${stem ? '[a-z]*' : ''}(?![a-z0-9])`);
}

describe('tone rules (design 5.1)', () => {
  const words = [...forbidden.pt, ...forbidden.en];

  it('reads the shared word list', () => {
    expect(forbidden.pt).toContain('vergonha');
    expect(forbidden.en).toContain('shame');
    expect(pattern('fracass*').test(fold('Você fracassou'))).toBe(true);
    expect(pattern('sin').test(fold('single'))).toBe(false);
    expect(pattern('vício').test(fold('Um VICIO antigo'))).toBe(true);
  });

  it('uses no forbidden word in any string of either language', () => {
    for (const locale of LOCALES) {
      for (const [key, text] of strings(locale)) {
        const folded = fold(text);
        for (const word of words) expect(pattern(word).test(folded), `${key} (${locale}) contains "${word}"`).toBe(false);
      }
    }
  });

  it('puts no host, title or application name into a card: only the name and the delay are filled in', () => {
    const cardKeys = Object.keys(nofap['pt-BR']).filter((k) => /^(block|vision\.(soft|closed|added|wrong|after)|notit|drift\.card|pending\.ready|resolver\.unreachable|reminder|help\.(pattern|how|limits)|card)\b/.test(k));
    expect(cardKeys.length).toBeGreaterThan(15);
    for (const locale of LOCALES) {
      for (const key of cardKeys) {
        const placeholders = [...nofap[locale][key]!.matchAll(/\{(\w+)\}/g)].map((m) => m[1]);
        expect(placeholders.filter((p) => p !== 'name' && p !== 'delay'), `${key} (${locale})`).toEqual([]);
      }
    }
  });

  it('keeps the first thing on screen after a block free of the subject', () => {
    expect(nofap['pt-BR']['block.discreet']).toBe('Página bloqueada pelo UBI.');
    expect(nofap.en['block.discreet']).toBe('Page blocked by UBI.');
  });
});

describe('naming (design 0.1)', () => {
  it('never writes the source name into a string: the display name arrives from the engine', () => {
    for (const locale of LOCALES) for (const [key, text] of strings(locale)) expect(/nofap/i.test(text), `${key} (${locale})`).toBe(false);
  });

  it('keeps the source name out of the shell configuration', () => {
    expect(/nofap/i.test(tauriConf)).toBe(false);
  });

  it('says "barrados", never the label Settings uses for "never recorded"', () => {
    expect(nofap['pt-BR']['lists.blocked.title']).toBe('Sites barrados');
    for (const [key, text] of Object.entries(nofap['pt-BR'])) if (key.startsWith('lists.')) expect(/bloquead/i.test(text), key).toBe(false);
  });
});
