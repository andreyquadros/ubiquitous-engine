// @ts-expect-error this package has no @types/node (vite.config.ts does the same for node:process)
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
// The word list the Rust tests read, as it is in the repository.
import forbidden from '../../../../crates/ubiqx-nofap/data/forbidden-words.json';
import tauriConf from '../../src-tauri/tauri.conf.json?raw';
import { hasKey, NAMESPACES, translate } from '../i18n';
import { PANIC_CRISIS, UBIQX_ONLY } from '../i18n/messages/nofap';
import type { Locale } from '../i18n/types';
import { aiExitText, consentKey, consentText, providerName } from './labels';

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
    expect(rows).toHaveLength(69);
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

describe('copy for two products (integrator decision 32)', () => {
  /** ubiqX by name (not the `ubiqx-shield` marker of the hosts block), its pause, its private mode, its local mode. */
  const UBIQX_WORDS = [/ubiqx(?!-shield)/i, /\bpausar o\b|\bnão pausa o\b|\bpausing\b|\bdoes not pause\b/i, /modo privado|private mode/i, /modo só local|local-only/i];
  const namesUbiqx = (text: string): boolean => UBIQX_WORDS.some((word) => word.test(text));

  it('says nothing of ubiqX in a string both products show', () => {
    for (const locale of LOCALES) {
      for (const [key, text] of strings(locale)) {
        const bare = key.replace(/^nofap\./, '');
        if (key.startsWith('nofap.') && bare in UBIQX_ONLY) continue;
        expect(namesUbiqx(text), `${key} (${locale}): ${text}`).toBe(false);
      }
    }
  });

  it('keeps the list of ubiqX-only strings honest: each exists in both languages and does speak of ubiqX', () => {
    expect(Object.keys(UBIQX_ONLY).sort()).toEqual(['helptab.ai.local_only', 'vision.managed', 'vision.managed_tick', 'vision.off.local_only', 'vision.pause_note']);
    for (const key of Object.keys(UBIQX_ONLY)) {
      for (const locale of LOCALES) {
        expect(hasKey(`nofap.${key}`, locale), `${key} (${locale})`).toBe(true);
        expect(namesUbiqx(nofap[locale][key]!), `${key} (${locale})`).toBe(true);
      }
    }
  });

  it('the old sentences are gone from the shared strings, in both languages', () => {
    expect(nofap['pt-BR']['dns.disclosure']).toContain('Este app não recebe nada.');
    expect(nofap.en['dns.disclosure']).toContain('This app receives nothing.');
    for (const key of ['vision.consent', 'vision.consent_ubi']) {
      expect(nofap['pt-BR'][key]!.endsWith('Janelas anônimas são verificadas.'), key).toBe(true);
      expect(nofap.en[key]!.endsWith('Private-browsing windows are checked.'), key).toBe(true);
    }
    expect(nofap['pt-BR']['helptab.ai.body']).toBe('A verificação para na hora, sem espera, quando você remove a chave de IA nas configurações do app.');
    expect(nofap.en['helptab.ai.body']).toBe("The check stops at once, with no wait, when you remove the AI key in the app's settings.");
  });

  it('adds the ubiqX-only line in ubiqX and nothing in the standalone app', () => {
    for (const locale of LOCALES) {
      const t = (key: string, vars?: Record<string, string | number>) => translate(locale, key, vars);
      for (const provider of ['anthropic', 'openai', 'xai', 'ubi'] as const) {
        const standalone = consentText(provider, 'NoFap', 'shield', t);
        const ubiqx = consentText(provider, 'NoFap', 'ubiqx', t);
        expect(standalone).toBe(t(consentKey(provider), { name: 'NoFap', provider: providerName(provider, t) }));
        expect(ubiqx).toBe(`${standalone} ${t('nofap.vision.pause_note')}`);
        expect(namesUbiqx(standalone), `${locale} ${provider}`).toBe(false);
      }
      expect(aiExitText('shield', t)).toBe(t('nofap.helptab.ai.body'));
      expect(aiExitText('ubiqx', t)).toBe(`${t('nofap.helptab.ai.body')} ${t('nofap.helptab.ai.local_only')}`);
    }
    // In ubiqX the consent reads as it did before the split.
    expect(consentText('anthropic', 'NoFap', 'ubiqx', (key, vars) => translate('pt-BR', key, vars)).endsWith('Janelas anônimas são verificadas. Pausar o ubiqX ou usar o modo privado não pausa esta verificação.')).toBe(true);
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
