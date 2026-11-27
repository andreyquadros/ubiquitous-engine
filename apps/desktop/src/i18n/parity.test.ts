import { describe, expect, it } from 'vitest';
import { NAMESPACES } from './index';
import type { Locale } from './types';

const LOCALES: Locale[] = ['pt-BR', 'en'];
const placeholders = (s: string): string[] => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]!).sort();

describe('i18n message parity', () => {
  for (const [ns, messages] of Object.entries(NAMESPACES)) {
    describe(ns, () => {
      it('has both locales', () => {
        for (const l of LOCALES) expect(messages[l], `${ns} is missing locale ${l}`).toBeTypeOf('object');
      });

      it('has the same keys in pt-BR and en', () => {
        const pt = Object.keys(messages['pt-BR']).sort();
        const en = Object.keys(messages.en).sort();
        const missingInEn = pt.filter((k) => !en.includes(k));
        const missingInPt = en.filter((k) => !pt.includes(k));
        expect(missingInEn, `${ns}: keys missing in en`).toEqual([]);
        expect(missingInPt, `${ns}: keys missing in pt-BR`).toEqual([]);
      });

      it('uses the same placeholders in both languages', () => {
        for (const key of Object.keys(messages['pt-BR'])) {
          const pt = messages['pt-BR'][key];
          const en = messages.en[key];
          if (pt === undefined || en === undefined) continue;
          expect(placeholders(en), `${ns}.${key}: placeholders differ`).toEqual(placeholders(pt));
        }
      });

      it('pairs every plural form', () => {
        for (const l of LOCALES) {
          const keys = Object.keys(messages[l]);
          for (const k of keys) {
            if (k.endsWith('_one')) expect(keys, `${ns}.${k} has no _other form (${l})`).toContain(k.replace(/_one$/, '_other'));
            if (k.endsWith('_other')) expect(keys, `${ns}.${k} has no _one form (${l})`).toContain(k.replace(/_other$/, '_one'));
          }
        }
      });

      it('has no empty strings and keeps the design rules (no ALL-CAPS labels)', () => {
        for (const l of LOCALES) {
          for (const [k, v] of Object.entries(messages[l])) {
            expect(v.trim(), `${ns}.${k} (${l}) is empty`).not.toBe('');
            const letters = v.replace(/[^A-Za-zÀ-ÿ]/g, '');
            if (letters.length > 3) expect(letters === letters.toUpperCase(), `${ns}.${k} (${l}) is ALL-CAPS`).toBe(false);
          }
        }
      });
    });
  }
});
