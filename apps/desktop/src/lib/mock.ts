// In-memory mock of the Rust engine, used whenever the page is not running inside Tauri
// (browser `vite dev`, Vitest, Playwright screenshots). It mutates its own state for the
// write commands and emits fake engine events so the UI behaves like the real app.

import { getLocale } from '../i18n';
import { normalizeDomain } from './format';
import {
  SYSTEM_CATEGORIES,
  type ActivityBlock,
  type ActivityKind,
  type Advice,
  type AiHealth,
  type ApiKeyResult,
  type BlockGroup,
  type Category,
  type ClassificationSource,
  type ClassifyReport,
  type CorrectionOutcome,
  type DailyReport,
  type DashboardData,
  type EngineEvent,
  type FocusStats,
  type Id,
  type IsoDate,
  type IsoDateTime,
  type Mood,
  type Nudge,
  type PermissionStatus,
  type Platform,
  type ReportItem,
  type Rule,
  type RuleSuggestion,
  type ScreenshotData,
  type Settings,
  type SettingsView,
  type TrackerState,
  type AiUsageTotals,
  type AiModels,
  type AiProvider,
  type ApiKeyStatus,
  type ProviderInfo,
  type BuildInfo,
  type ReleaseInfo,
  type UpdateStatus,
  type FocusSession,
  type FocusStatus,
  type FocusTarget,
  type FocusTargetKind,
  type InstalledApp,
  type Intervention,
  type InterventionAction,
  type KnownDomain,
  type LicenseStatus,
  type Plan,
} from './types';
import { PROVIDER_IDS, SITE_URL, UBI_MODELS, licenseAllowsManaged, reconcileModels } from './providers';

/* ------------------------------------------------------------------ */
/* helpers                                                             */
/* ------------------------------------------------------------------ */

const LATENCY_MS = import.meta.env.MODE === 'test' ? 0 : 90;
const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

let counter = 1000;
const uid = (prefix: string): Id => `${prefix}-${(counter++).toString(36)}`;

const pad = (n: number) => String(n).padStart(2, '0');
const isoDate = (d: Date): IsoDate => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const today = (): IsoDate => isoDate(new Date());
const atLocal = (date: IsoDate, h: number, m: number): IsoDateTime => {
  const [y, mo, d] = date.split('-').map(Number);
  return new Date(y ?? 2026, (mo ?? 1) - 1, d ?? 1, h, m, 0, 0).toISOString();
};
const localDateOf = (iso: IsoDateTime): IsoDate => isoDate(new Date(iso));
const addMinutes = (iso: IsoDateTime, mins: number): IsoDateTime => new Date(new Date(iso).getTime() + mins * 60_000).toISOString();
const secsBetween = (a: IsoDateTime, b: IsoDateTime): number => Math.max(0, (new Date(b).getTime() - new Date(a).getTime()) / 1000);
const fmtHM = (iso: IsoDateTime): string => {
  const d = new Date(iso);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
const shiftDays = (date: IsoDate, days: number): IsoDate => {
  const [y, m, d] = date.split('-').map(Number);
  return isoDate(new Date(y ?? 2026, (m ?? 1) - 1, (d ?? 1) + days));
};
const weekday = (date: IsoDate): number => {
  const [y, m, d] = date.split('-').map(Number);
  return new Date(y ?? 2026, (m ?? 1) - 1, d ?? 1).getDay();
};

/** Tiny deterministic PRNG so other days look stable across reloads. */
function rng(seed: string): () => number {
  let h = 2166136261;
  for (const ch of seed) {
    h ^= ch.charCodeAt(0);
    h = Math.imul(h, 16777619);
  }
  return () => {
    h += 0x6d2b79f5;
    let t = h;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const domainOf = (url: string | null): string | null => {
  if (!url) return null;
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return null;
  }
};

const titleKey = (title: string): string =>
  title
    .toLowerCase()
    .replace(/\(\d+\)/g, '')
    .replace(/[0-9]+/g, '')
    .replace(/[^\p{L}\s]/gu, ' ')
    .replace(/\s+/g, ' ')
    .trim();

const APP_IDS: Record<string, string> = {
  'Google Chrome': 'com.google.Chrome',
  'Visual Studio Code': 'com.microsoft.VSCode',
  WhatsApp: 'net.whatsapp.WhatsApp',
  'Microsoft Teams': 'com.microsoft.teams2',
  Terminal: 'com.apple.Terminal',
  Preview: 'com.apple.Preview',
  Finder: 'com.apple.finder',
  Calendário: 'com.apple.iCal',
  Notas: 'com.apple.Notes',
  Ocioso: 'idle',
  'Modo privado': 'private',
  Manual: 'manual',
};

/* ------------------------------------------------------------------ */
/* seed data                                                           */
/* ------------------------------------------------------------------ */

const CAT_IFRO = 'cat-ifro';
const CAT_INCUB = 'cat-incubadora';
const CAT_CIDADES = 'cat-cidades';

const seedCategories = (): Category[] => {
  const created = atLocal(shiftDays(today(), -40), 9, 0);
  const cat = (partial: Partial<Category> & Pick<Category, 'id' | 'name' | 'color' | 'icon'>): Category => ({
    description: '',
    keywords: [],
    report_time: null,
    report_template: null,
    is_productive: true,
    is_system: false,
    archived: false,
    sort_order: 0,
    created_at: created,
    ...partial,
  });
  return [
    cat({
      id: CAT_IFRO,
      name: 'IFRO',
      color: '#2563EB',
      icon: 'graduation-cap',
      description:
        'Docência no IFRO Campus Porto Velho Calama: aulas de Programação Web, orientação de TCC, reuniões de colegiado, SEI/SUAP, editais PROEX e e-mails institucionais.',
      keywords: ['ifro', 'sei', 'suap', 'aula', 'tcc', 'colegiado', 'edital', 'proex', 'plano de ensino'],
      report_time: '18:00:00',
      report_template: null,
      sort_order: 0,
    }),
    cat({
      id: CAT_INCUB,
      name: 'Incubadora',
      color: '#F97316',
      icon: 'rocket',
      description:
        'Incubadora de startups do IFRO: mentoria das startups incubadas, desenvolvimento da API de inscrições, Demo Day e roadmap no Notion.',
      keywords: ['incubadora', 'startup', 'mentoria', 'demo day', 'agrotech', 'api-incubadora', 'notion'],
      report_time: '18:30:00',
      report_template: '## Resumo\n\n{resumo}\n\n## Atividades\n\n{itens}\n\n## Próximos passos\n\n{proximos}',
      sort_order: 1,
    }),
    cat({
      id: CAT_CIDADES,
      name: 'Cidades Inteligentes',
      color: '#10B981',
      icon: 'building-2',
      description:
        'Projeto de pesquisa em cidades inteligentes com a Prefeitura de Porto Velho: sensores IoT (LoRaWAN/MQTT), ingestão de dados, dashboards e dados abertos.',
      keywords: ['cidades', 'sensor', 'iot', 'lorawan', 'mqtt', 'prefeitura', 'dados abertos', 'qualidade do ar'],
      report_time: '18:00:00',
      sort_order: 2,
    }),
    cat({
      id: SYSTEM_CATEGORIES.uncategorized,
      name: 'Sem categoria',
      color: '#94A3B8',
      icon: 'circle-dashed',
      description: 'Blocos ainda não classificados.',
      is_productive: false,
      is_system: true,
      sort_order: 100,
    }),
    cat({
      id: SYSTEM_CATEGORIES.distraction,
      name: 'Distração',
      color: '#EF4444',
      icon: 'tv',
      description: 'Redes sociais, vídeos e sites de entretenimento.',
      keywords: ['youtube', 'instagram', 'twitter', 'reddit', 'netflix'],
      is_productive: false,
      is_system: true,
      sort_order: 101,
    }),
    cat({
      id: SYSTEM_CATEGORIES.break,
      name: 'Pausa',
      color: '#A78BFA',
      icon: 'coffee',
      description: 'Tempo ocioso ou pausas deliberadas.',
      is_productive: false,
      is_system: true,
      sort_order: 102,
    }),
    cat({
      id: SYSTEM_CATEGORIES.private,
      name: 'Privado',
      color: '#64748B',
      icon: 'eye-off',
      description: 'Modo privado: nada é registrado além da duração.',
      is_productive: false,
      is_system: true,
      sort_order: 103,
    }),
  ];
};

const seedRules = (): Rule[] => {
  const mk = (p: Partial<Rule> & Pick<Rule, 'category_id' | 'matcher' | 'pattern'>): Rule => ({
    id: uid('rule'),
    priority: 10,
    origin: 'user',
    enabled: true,
    created_at: atLocal(shiftDays(today(), -30), 10, 0),
    hit_count: 0,
    miss_count: 0,
    last_contradicted_at: null,
    ...p,
  });
  return [
    mk({ category_id: CAT_IFRO, matcher: 'domain', pattern: 'sei.ifro.edu.br', hit_count: 34 }),
    mk({ category_id: CAT_IFRO, matcher: 'domain', pattern: 'suap.ifro.edu.br', hit_count: 27 }),
    mk({ category_id: SYSTEM_CATEGORIES.distraction, matcher: 'domain', pattern: 'youtube.com', hit_count: 12, miss_count: 2, last_contradicted_at: atLocal(today(), 18, 12) }),
    mk({ category_id: SYSTEM_CATEGORIES.distraction, matcher: 'domain', pattern: 'instagram.com', hit_count: 9 }),
    mk({ category_id: CAT_INCUB, matcher: 'title_contains', pattern: 'incubadora', origin: 'learned', hit_count: 21, miss_count: 1, priority: 5 }),
    mk({ category_id: CAT_INCUB, matcher: 'domain', pattern: 'github.com', origin: 'learned', hit_count: 8, miss_count: 3, last_contradicted_at: atLocal(shiftDays(today(), -2), 15, 40), priority: 5 }),
    mk({ category_id: CAT_CIDADES, matcher: 'regex', pattern: 'cidades|sensor|lorawan', origin: 'learned', hit_count: 5, enabled: false, priority: 5 }),
    mk({ category_id: CAT_CIDADES, matcher: 'domain', pattern: 'portovelho.ro.gov.br', hit_count: 4 }),
  ];
};

type Extra = Partial<{ review: boolean; desc: string; sent: boolean; shot: boolean }>;
type Spec = [mins: number, app: string, title: string, url: string | null, cat: Id | null, conf: number, source: ClassificationSource | null, extra?: Extra];

const DAY_SPECS: Spec[] = [
  [10, 'Google Chrome', 'Caixa de entrada (12) - andrey.quadros@ifro.edu.br - Gmail', 'https://mail.google.com/mail/u/0/#inbox', CAT_IFRO, 0.72, 'llm', { desc: 'Leitura de e-mails institucionais: edital PROEX e convocação do colegiado', sent: true }],
  [4, 'WhatsApp', 'WhatsApp', null, CAT_IFRO, 0.55, 'llm', { review: true, desc: 'Mensagens de alunos sobre a entrega do projeto integrador', sent: true, shot: true }],
  [3, 'Finder', 'Downloads', null, null, 0, null],
  [45, 'Google Chrome', 'SEI - Processo 23243.001234/2026-11 - Ofício', 'https://sei.ifro.edu.br/sei/controlador.php?acao=procedimento_trabalhar', CAT_IFRO, 1, 'rule'],
  [3, 'Calendário', 'Calendário — setembro de 2026', null, null, 0, null],
  [45, 'Microsoft Teams', 'Reunião do Colegiado de ADS | Microsoft Teams', null, CAT_IFRO, 0.88, 'llm', { desc: 'Reunião do colegiado: aprovação de ementas e calendário de TCC', sent: true }],
  [10, 'Google Chrome', 'lofi hip hop radio - beats to relax/study to - YouTube', 'https://www.youtube.com/watch?v=jfKfPfyJRdk', SYSTEM_CATEGORIES.distraction, 1, 'rule'],
  [15, 'Google Chrome', 'Plano de ensino - Programação Web II - Google Docs', 'https://docs.google.com/document/d/1AbCdEf/edit', CAT_IFRO, 0.9, 'memory'],
  [5, 'Google Chrome', 'Caixa de entrada (9) - andrey.quadros@ifro.edu.br - Gmail', 'https://mail.google.com/mail/u/0/#inbox', CAT_IFRO, 0.7, 'llm', { sent: true }],
  [2, 'Finder', 'api-incubadora', null, null, 0, null],
  [50, 'Visual Studio Code', 'server.ts — api-incubadora', null, CAT_INCUB, 0.93, 'memory', { desc: 'Endpoint de inscrição de startups no edital 03/2026' }],
  [8, 'Terminal', 'pnpm test — api-incubadora — 80×24', null, CAT_INCUB, 0.85, 'llm', { sent: true }],
  [12, 'Visual Studio Code', 'startup.repository.ts — api-incubadora', null, CAT_INCUB, 0.95, 'memory'],
  [12, 'Google Chrome', 'Pull Request #42 · incubadora-ifro/api · GitHub', 'https://github.com/incubadora-ifro/api/pull/42', CAT_INCUB, 0.95, 'rule'],
  [3, 'Google Chrome', 'Notificações · GitHub', 'https://github.com/notifications', CAT_INCUB, 0.9, 'rule'],
  [5, 'WhatsApp', 'WhatsApp', null, null, 0.4, 'llm', { review: true, desc: 'Conversa com a equipe da AgroTech', sent: true, shot: true }],
  [25, 'Modo privado', '', null, SYSTEM_CATEGORIES.private, 1, 'rule'],
  [50, 'Ocioso', '', null, SYSTEM_CATEGORIES.break, 1, 'rule'],
  [15, 'Google Chrome', 'Caixa de entrada (3) - andrey.quadros@ifro.edu.br - Gmail', 'https://mail.google.com/mail/u/0/#inbox', CAT_INCUB, 0.62, 'llm', { review: true, desc: 'Respostas aos mentores sobre a agenda do Demo Day', sent: true, shot: true }],
  [52, 'Visual Studio Code', 'ingest_sensores.py — cidades-inteligentes', null, CAT_CIDADES, 0.91, 'memory', { desc: 'Ingestão dos sensores de qualidade do ar (MQTT → TimescaleDB)' }],
  [3, 'Google Chrome', 'python - paho-mqtt reconnect after broker restart - Stack Overflow', 'https://stackoverflow.com/questions/12345', CAT_CIDADES, 0.75, 'llm', { sent: true }],
  [6, 'Terminal', 'python ingest_sensores.py — cidades-inteligentes', null, CAT_CIDADES, 0.8, 'llm', { sent: true }],
  [20, 'Google Chrome', 'Dados Abertos — Prefeitura de Porto Velho', 'https://dadosabertos.portovelho.ro.gov.br/', CAT_CIDADES, 0.78, 'llm', { sent: true }],
  [10, 'Google Chrome', 'LoRaWAN® Specification — The Things Network', 'https://www.thethingsnetwork.org/docs/lorawan/', CAT_CIDADES, 0.7, 'llm', { sent: true }],
  [10, 'Preview', 'relatorio-calibracao-sensores.pdf', null, CAT_CIDADES, 0.66, 'vision', { desc: 'Leitura do relatório de calibração dos sensores de PM2.5', sent: true, shot: true }],
  [12, 'Google Chrome', 'Instagram', 'https://www.instagram.com/', SYSTEM_CATEGORIES.distraction, 1, 'rule'],
  [3, 'Finder', 'cidades-inteligentes', null, null, 0, null],
  [51, 'Microsoft Teams', 'Mentoria — AgroTech | Microsoft Teams', null, CAT_INCUB, 0.9, 'llm', { desc: 'Mentoria com a AgroTech: modelo de receita e pitch para o Demo Day', sent: true }],
  [4, 'Preview', 'pitch-agrotech-v3.pdf', null, CAT_INCUB, 0.7, 'vision', { desc: 'Revisão do pitch da AgroTech', sent: true, shot: true }],
  [4, 'Notas', 'Notas — anotações da mentoria', null, CAT_INCUB, 0.6, 'llm', { review: true, sent: true }],
  [15, 'Google Chrome', 'Edital PROEX 2026 - Google Docs', 'https://docs.google.com/document/d/1XyZ/edit', CAT_IFRO, 0.74, 'llm', { sent: true }],
  [15, 'Google Chrome', 'Roadmap Incubadora — Notion', 'https://www.notion.so/incubadora/roadmap', CAT_INCUB, 0.81, 'llm', { sent: true }],
  [5, 'Google Chrome', 'Caixa de entrada (1) - andrey.quadros@ifro.edu.br - Gmail', 'https://mail.google.com/mail/u/0/#inbox', CAT_IFRO, 0.7, 'llm', { sent: true }],
  [33, 'Google Chrome', 'SUAP - Diário: Programação Web II - Lançar notas', 'https://suap.ifro.edu.br/edu/meu_diario/1234/', CAT_IFRO, 1, 'rule'],
  [2, 'Calendário', 'Calendário — setembro de 2026', null, null, 0, null],
  [6, 'Google Chrome', 'WhatsApp Web', 'https://web.whatsapp.com/', null, 0.45, 'llm', { review: true, desc: 'Combinando o horário da aula de sábado com a turma', sent: true }],
  [4, 'Terminal', 'git push — tcc-orientacao', null, CAT_IFRO, 0.68, 'llm', { sent: true }],
  [25, 'Visual Studio Code', 'revisao-cap3.md — tcc-orientacao', null, CAT_IFRO, 0.68, 'llm', { review: true, desc: 'Revisão do capítulo 3 do TCC da orientanda (metodologia)', sent: true }],
  [8, 'Google Chrome', 'Caixa de entrada (5) - andrey.quadros@ifro.edu.br - Gmail', 'https://mail.google.com/mail/u/0/#inbox', CAT_INCUB, 0.58, 'llm', { review: true, sent: true }],
  [20, 'Google Chrome', 'TEDx: Cidades inteligentes de verdade — YouTube', 'https://www.youtube.com/watch?v=dQw4w9WgXcQ', CAT_CIDADES, 0.58, 'llm', { review: true, desc: 'Palestra sobre sensoriamento urbano (referência para o projeto)', sent: true, shot: true }],
];

function specToBlock(date: IsoDate, spec: Spec, startIso: IsoDateTime, mins: number): ActivityBlock {
  const [, app, title, url, cat, conf, source, extra] = spec;
  const endIso = addMinutes(startIso, mins);
  const domain = domainOf(url);
  const sent = !!extra?.sent;
  const isPrivate = app === 'Modo privado';
  return {
    id: uid('blk'),
    started_at: startIso,
    ended_at: endIso,
    app_name: app,
    app_id: APP_IDS[app] ?? app.toLowerCase().replace(/\s+/g, '.'),
    title: isPrivate ? '' : title,
    title_key: isPrivate ? '' : titleKey(title),
    url: isPrivate ? null : url,
    domain: isPrivate ? null : domain,
    category_id: cat,
    confidence: conf,
    source,
    description: extra?.desc ?? null,
    screenshot_id: extra?.shot ? uid('shot') : null,
    sample_count: Math.max(1, Math.round((mins * 60) / 5)),
    is_open: false,
    classify_attempts: source ? 1 : 0,
    next_attempt_at: !source && !cat ? addMinutes(endIso, 1) : null,
    needs_review: !!extra?.review,
    ai_payload: sent
      ? JSON.stringify({ app, title, domain, hints: extra?.desc ? [extra.desc] : [], date, samples: Math.round((mins * 60) / 5) })
      : null,
    ai_sent_at: sent ? addMinutes(endIso, 2) : null,
    is_manual: false,
    note: null,
  };
}

function buildDay(date: IsoDate): ActivityBlock[] {
  const t = today();
  if (date > t) return [];
  const dow = weekday(date);
  const r = rng(date);
  const blocks: ActivityBlock[] = [];

  if (date === t) {
    let cursor = atLocal(date, 8, 0);
    for (const spec of DAY_SPECS) {
      const b = specToBlock(date, spec, cursor, spec[0]);
      cursor = b.ended_at;
      blocks.push(b);
    }
    const last = blocks[blocks.length - 1];
    if (last) last.is_open = true;
    return blocks;
  }

  if (dow === 0) return [];
  const specs = dow === 6 ? DAY_SPECS.slice(9, 16) : DAY_SPECS;
  let cursor = atLocal(date, dow === 6 ? 9 : 8, Math.floor(r() * 40));
  for (const spec of specs) {
    if (r() < 0.22) continue;
    const mins = Math.max(2, Math.round(spec[0] * (0.7 + r() * 0.7)));
    const b = specToBlock(date, spec, cursor, mins);
    // past days were already reviewed and most things settled
    if (r() < 0.85) {
      b.needs_review = false;
      if (!b.category_id && b.app_name !== 'Ocioso' && b.app_name !== 'Modo privado') {
        b.category_id = r() < 0.5 ? CAT_IFRO : CAT_INCUB;
        b.source = 'user';
        b.confidence = 1;
      }
    }
    cursor = b.ended_at;
    blocks.push(b);
  }
  return blocks;
}

const seedReports = (date: IsoDate): DailyReport[] => [
  {
    id: uid('rep'),
    date,
    category_id: CAT_IFRO,
    generated_at: atLocal(date, 18, 0),
    summary_md:
      '## Resumo do dia — IFRO\n\nDia dividido entre **gestão acadêmica** e **ensino**. A manhã foi dominada pela reunião do colegiado de ADS (aprovação de ementas e calendário de TCC) e pelo despacho do processo SEI 23243.001234/2026-11. No fim da tarde, lançamento de notas de Programação Web II no SUAP e revisão do capítulo 3 do TCC da orientanda.\n\n**Destaques**\n\n- Processo SEI despachado (45 min).\n- Notas do 2º bimestre lançadas no SUAP.\n- Plano de ensino de Programação Web II atualizado.\n',
    items: [
      { activity: 'Reunião do Colegiado de ADS: ementas e calendário de TCC', kind: 'reuniao', minutes: 45, evidence: ['Microsoft Teams · Reunião do Colegiado de ADS'], time_range: '09:10–09:55', continuation_of: null },
      { activity: 'Despacho do processo SEI 23243.001234/2026-11 (ofício)', kind: 'gestao', minutes: 45, evidence: ['sei.ifro.edu.br · Processo 23243.001234/2026-11'], time_range: '08:17–09:02', continuation_of: null },
      { activity: 'Lançamento de notas de Programação Web II no SUAP', kind: 'ensino', minutes: 33, evidence: ['suap.ifro.edu.br · Diário: Programação Web II'], time_range: '16:44–17:17', continuation_of: null },
      { activity: 'Revisão do capítulo 3 (metodologia) do TCC da orientanda', kind: 'ensino', minutes: 29, evidence: ['Visual Studio Code · revisao-cap3.md', 'Terminal · git push — tcc-orientacao'], time_range: '17:25–17:54', continuation_of: null },
      { activity: 'Atualização do plano de ensino de Programação Web II', kind: 'documentacao', minutes: 15, evidence: ['docs.google.com · Plano de ensino - Programação Web II'], time_range: '10:05–10:20', continuation_of: null },
      { activity: 'E-mails institucionais (edital PROEX, convocações)', kind: 'comunicacao', minutes: 35, evidence: ['mail.google.com · Caixa de entrada', 'docs.google.com · Edital PROEX 2026'], time_range: '08:00–16:35', continuation_of: null },
    ],
    highlights: ['Processo SEI despachado', 'Notas lançadas no SUAP', 'Plano de ensino atualizado'],
    total_secs: 202 * 60,
    model: 'claude-sonnet-5',
    input_tokens: 6120,
    output_tokens: 1480,
    stale: true,
    edited: false,
  },
  {
    id: uid('rep'),
    date,
    category_id: CAT_INCUB,
    generated_at: atLocal(date, 18, 30),
    summary_md:
      '## Resumo do dia — Incubadora\n\nForte bloco de **desenvolvimento** na API de inscrições (endpoint de inscrição de startups no edital 03/2026, repositório e testes) seguido de uma **mentoria** de quase uma hora com a AgroTech sobre modelo de receita e pitch para o Demo Day.\n\n**Destaques**\n\n- PR #42 (inscrições) revisado e pronto para merge.\n- Pitch v3 da AgroTech revisado com a equipe.\n- Roadmap atualizado no Notion.\n',
    items: [
      { activity: 'Endpoint de inscrição de startups (edital 03/2026) na api-incubadora', kind: 'desenvolvimento', minutes: 70, evidence: ['Visual Studio Code · server.ts — api-incubadora', 'Terminal · pnpm test — api-incubadora'], time_range: '10:22–11:32', continuation_of: null },
      { activity: 'Mentoria com a AgroTech: modelo de receita e pitch para o Demo Day', kind: 'reuniao', minutes: 55, evidence: ['Microsoft Teams · Mentoria — AgroTech', 'Preview · pitch-agrotech-v3.pdf'], time_range: '15:25–16:20', continuation_of: null },
      { activity: 'Revisão do PR #42 (incubadora-ifro/api)', kind: 'desenvolvimento', minutes: 15, evidence: ['github.com · Pull Request #42'], time_range: '11:32–11:47', continuation_of: null },
      { activity: 'Roadmap da incubadora no Notion', kind: 'gestao', minutes: 15, evidence: ['notion.so · Roadmap Incubadora'], time_range: '16:35–16:50', continuation_of: null },
      { activity: 'Agenda do Demo Day com mentores (e-mail)', kind: 'comunicacao', minutes: 15, evidence: ['mail.google.com · Caixa de entrada'], time_range: '13:25–13:40', continuation_of: null },
    ],
    highlights: ['PR #42 pronto para merge', 'Pitch v3 da AgroTech revisado', 'Roadmap atualizado'],
    total_secs: 170 * 60,
    model: 'claude-sonnet-5',
    input_tokens: 5480,
    output_tokens: 1210,
    stale: false,
    edited: false,
  },
];

/** The real engine writes nudges, advice and reports in Settings.language; the mock follows the UI locale instead. */
const en = (): boolean => getLocale() === 'en';
/** Picks the Portuguese or English variant of a mock text for the current locale. */
const pick = (pt: string, english: string): string => (en() ? english : pt);

const seedNudges = (date: IsoDate): Nudge[] => [
  {
    id: uid('ndg'),
    at: atLocal(date, 10, 2),
    kind: 'distracted',
    title: pick('YouTube de novo?', 'YouTube again?'),
    message: pick('Você passou 10 min no YouTube. Quer voltar ao plano de ensino de Programação Web II?', 'You spent 10 min on YouTube. Want to get back to the Web Programming II lesson plan?'),
    seen: true,
  },
  {
    id: uid('ndg'),
    at: atLocal(date, 11, 30),
    kind: 'praise',
    title: pick('Foco de 70 minutos!', '70 minutes of focus!'),
    message: pick('Belo bloco de código na API da Incubadora. Continue assim — mas lembre de beber água.', 'Nice stretch of coding on the Incubator API. Keep it up, and remember to drink some water.'),
    seen: true,
  },
  {
    id: uid('ndg'),
    at: atLocal(date, 16, 25),
    kind: 'break_suggested',
    title: pick('Hora de uma pausa', 'Time for a break'),
    message: pick('Você está há 1h50 sem pausa. Que tal esticar as pernas por 5 minutos antes de lançar as notas?', "You've been going for 1h50 without a break. How about stretching your legs for 5 minutes before entering the grades?"),
    seen: false,
  },
];

/* ------------------------------------------------------------------ */
/* AI providers                                                        */
/* ------------------------------------------------------------------ */

/** Mirrors `AiProvider::{label, console_url, key_prefix}` and `AiModels::for_provider` in ubiqx-core. */
const PROVIDERS: ProviderInfo[] = [
  {
    id: 'anthropic',
    label: 'Anthropic Claude',
    console_url: 'https://console.anthropic.com/settings/keys',
    key_prefix: 'sk-ant-',
    default_models: { classify: 'claude-haiku-4-5', vision: 'claude-haiku-4-5', report: 'claude-sonnet-5' },
  },
  {
    id: 'openai',
    label: 'OpenAI',
    console_url: 'https://platform.openai.com/api-keys',
    key_prefix: 'sk-',
    default_models: { classify: 'gpt-5-mini', vision: 'gpt-5-mini', report: 'gpt-5' },
  },
  {
    id: 'xai',
    label: 'xAI Grok',
    console_url: 'https://console.x.ai',
    key_prefix: 'xai-',
    default_models: { classify: 'grok-4-1-fast-non-reasoning', vision: 'grok-4-1-fast-non-reasoning', report: 'grok-4-1-fast-reasoning' },
  },
  {
    id: 'ubi',
    label: 'IA do Ubi',
    console_url: SITE_URL,
    key_prefix: 'UBIQX-',
    default_models: { ...UBI_MODELS },
  },
];

/** What `GET /models` of each account would return (chat-capable ids only), sorted. */
const ACCOUNT_MODELS: Record<AiProvider, string[]> = {
  anthropic: ['claude-haiku-4-5', 'claude-sonnet-5', 'claude-opus-5'],
  ubi: [UBI_MODELS.classify, UBI_MODELS.report],
  openai: ['gpt-5', 'gpt-5-mini', 'gpt-5-nano', 'gpt-5.1', 'gpt-5.4', 'gpt-5.4-mini', 'gpt-4.1-mini', 'gpt-4o-mini', 'o4-mini'],
  xai: ['grok-4', 'grok-4-1-fast-reasoning', 'grok-4-1-fast-non-reasoning', 'grok-4-fast-reasoning', 'grok-4-fast-non-reasoning', 'grok-3-mini', 'grok-4.6'],
};
for (const list of Object.values(ACCOUNT_MODELS)) list.sort((a, b) => a.localeCompare(b));

const providerInfo = (id: AiProvider): ProviderInfo => PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[0]!;
const isProvider = (v: unknown): v is AiProvider => typeof v === 'string' && (PROVIDER_IDS as string[]).includes(v);

/** Simulates the key validation the real backend does with a probe call. */
function keyLooksValid(provider: AiProvider, key: string): boolean {
  if (key.length < 20) return false;
  const prefixes = provider === 'openai' ? ['sk-proj-', 'sk-'] : [providerInfo(provider).key_prefix];
  return prefixes.some((p) => key.startsWith(p)) && (provider !== 'openai' || !key.startsWith('sk-ant-'));
}

const defaultSettings = (): Settings => ({
  tracking_enabled: true,
  sample_interval_secs: 5,
  idle_threshold_secs: 180,
  min_block_secs: 15,
  screenshot_interval_secs: 300,
  screenshot_max_edge: 1024,
  screenshot_retention_hours: 48,
  keep_screenshots_for_review: true,
  vision_policy: { mode: 'all_except_blocked' },
  vision_denied_apps: ['1Password', 'Banco do Brasil'],
  blocked_apps: ['1Password', 'Sicoob'],
  blocked_domains: ['bb.com.br', 'nubank.com.br', 'sicoob.com.br'],
  private_mode: false,
  private_until: null,
  ai_provider: 'anthropic',
  models: { ...PROVIDERS[0]!.default_models },
  max_vision_per_hour: 6,
  ai_monthly_budget_usd: 5,
  classify_batch_min: 5,
  classify_max_wait_secs: 120,
  local_only: false,
  min_confidence: 0.6,
  report_default_time: '18:00:00',
  language: 'pt-BR',
  user_profile:
    'Professor de informática no IFRO (Campus Porto Velho Calama), coordenador da incubadora de startups do campus e pesquisador no projeto Cidades Inteligentes com a Prefeitura de Porto Velho.',
  quiet_hours: { enabled: true, start: '20:00:00', end: '07:00:00' },
  nudges: {
    enabled: true,
    unproductive: true,
    distracted: true,
    break_suggested: true,
    praise: true,
    idle: false,
    max_per_day: 8,
    cooldown_mins: 20,
    silent_apps: ['Microsoft Teams', 'zoom.us', 'Keynote'],
    snoozed_until: null,
  },
  launch_at_login: true,
  onboarding_done: true,
  check_updates: true,
  focus: {
    guard_enabled: true,
    block_distraction_in_session: true,
    hide_others_on_start: true,
    session_minutes: 45,
    intervention_cooldown_secs: 20,
    macos_focus_shortcut_on: null,
    macos_focus_shortcut_off: null,
  },
});

/* ------------------------------------------------------------------ */
/* focus guard (blocked apps and sites, interventions, sessions)       */
/* ------------------------------------------------------------------ */

/** What `list_installed_apps` finds on this Mac (sorted by name, like the Rust side). */
const INSTALLED_APPS: InstalledApp[] = [
  { name: 'Discord', bundle_id: 'com.hnc.Discord', path: '/Applications/Discord.app' },
  { name: 'Google Chrome', bundle_id: 'com.google.Chrome', path: '/Applications/Google Chrome.app' },
  { name: 'Slack', bundle_id: 'com.tinyspeck.slackmacgap', path: '/Applications/Slack.app' },
  { name: 'Steam', bundle_id: 'com.valvesoftware.steam', path: '/Applications/Steam.app' },
  { name: 'WhatsApp', bundle_id: 'net.whatsapp.WhatsApp', path: '/Applications/WhatsApp.app' },
  { name: 'Xcode', bundle_id: 'com.apple.dt.Xcode', path: '/Applications/Xcode.app' },
];

const minutesAgo = (mins: number): IsoDateTime => new Date(Date.now() - mins * 60_000).toISOString();

/** The engine rotates these in order per process (same list for apps and sites); the mock follows the UI locale. */
const interventionMessage = (i: number): string => {
  const pt = ['Não! Foque na sua produtividade.', 'Esse app fica pra depois. Sua meta agradece.', 'Você bloqueou isso por um motivo. Volta pro que importa.', 'Não hoje. Que tal mais 20 minutos de foco?', 'Eu seguro a distração; você segura o foco.'];
  const english = ['No! Focus on your productivity.', 'That one can wait. Your goal says thanks.', 'You blocked this for a reason. Back to what matters.', 'Not today. How about 20 more minutes of focus?', 'I hold the distraction; you hold the focus.'];
  const list = en() ? english : pt;
  return list[((i % list.length) + list.length) % list.length] as string;
};

const TARGET_YOUTUBE = 'tgt-youtube';
const TARGET_INSTAGRAM = 'tgt-instagram';
const TARGET_DISCORD = 'tgt-discord';

/** Three targets; YouTube was blocked 3 minutes ago (so disabling it triggers the three warnings). */
const seedTargets = (): FocusTarget[] => {
  const created = atLocal(shiftDays(today(), -12), 9, 30);
  return [
    { id: TARGET_YOUTUBE, kind: 'site', name: 'YouTube', key: 'youtube.com', enabled: true, created_at: created, last_blocked_at: minutesAgo(3), blocked_count: 7 },
    { id: TARGET_INSTAGRAM, kind: 'site', name: 'Instagram', key: 'instagram.com', enabled: true, created_at: created, last_blocked_at: minutesAgo(130), blocked_count: 3 },
    { id: TARGET_DISCORD, kind: 'app', name: 'Discord', key: 'com.hnc.Discord', enabled: false, created_at: atLocal(shiftDays(today(), -5), 14, 0), last_blocked_at: atLocal(shiftDays(today(), -1), 16, 20), blocked_count: 1 },
  ];
};

/** The oldest seeded intervention, in minutes. */
const SEED_SPAN_MINS = 190;

/**
 * `mins` ago, but never earlier than today's local midnight: the Focus page counts the
 * interventions whose local date is today, and this seed promises four of them. Before 03:10 a
 * plain `minutesAgo(190)` lands on yesterday and the count drops, so early in the day the four
 * are compressed into the part of the day that has already elapsed, keeping their order.
 */
const minutesAgoToday = (mins: number): IsoDateTime => {
  const now = Date.now();
  const midnight = new Date(now);
  midnight.setHours(0, 0, 0, 0);
  const elapsed = (now - midnight.getTime()) / 60_000;
  const back = elapsed >= SEED_SPAN_MINS ? mins : (mins / SEED_SPAN_MINS) * elapsed;
  return new Date(now - back * 60_000).toISOString();
};

/** Four interventions today, newest first. */
const seedInterventions = (sessionId: Id | null): Intervention[] => {
  const mk = (i: number, at: IsoDateTime, target: FocusTarget, action: InterventionAction, session: Id | null): Intervention => ({
    id: `itv-${i}`,
    at,
    target_id: target.id,
    kind: target.kind,
    name: target.name,
    key: target.key,
    action,
    session_id: session,
    message: interventionMessage(i),
  });
  const [yt, ig, dc] = seedTargets() as [FocusTarget, FocusTarget, FocusTarget];
  return [mk(3, minutesAgoToday(3), yt, 'tab_closed', sessionId), mk(2, minutesAgoToday(48), ig, 'tab_closed', null), mk(1, minutesAgoToday(125), dc, 'app_quit', null), mk(0, minutesAgoToday(SEED_SPAN_MINS), yt, 'tab_blanked', null)];
};

/** A 45-minute session started 12 minutes ago (`?session=active`, `__mock.setFocus({ session: 'active' })`). */
const sampleSession = (): FocusSession => ({
  id: 'fs-active',
  task: pick('terminar o relatório do IFRO', 'finish the IFRO report'),
  started_at: minutesAgo(12),
  ends_at: new Date(Date.now() + 33 * 60_000).toISOString(),
  ended_at: null,
  interventions: 1,
  hid_windows: true,
  ran_shortcut: false,
});

/** The "a lot of windows" nudge the engine stores after 10 app switches in 15 minutes (`?nudge=focus_prompt`). */
const focusPromptNudge = (): Nudge => ({
  id: uid('ndg'),
  at: new Date().toISOString(),
  kind: 'focus_prompt',
  title: pick('Muitas janelas', 'A lot of windows'),
  message: pick(
    'Você tem alternado entre muitas janelas, que tal focar mais? Que tarefa você precisa fazer agora e quer que eu te ajude com um foco maior?',
    "You've been switching between a lot of windows. How about focusing more? What do you need to get done right now, and shall I help you focus on it?",
  ),
  seen: false,
});

/** `?session=active` and `?nudge=focus_prompt` are read once at module load (like `?lang=`), so a reset keeps them. */
const FOCUS_FROM_QUERY: { session: boolean; nudge: boolean } = (() => {
  try {
    if (typeof window === 'undefined') return { session: false, nudge: false };
    const q = new URLSearchParams(window.location.search);
    return { session: q.get('session') === 'active', nudge: q.get('nudge') === 'focus_prompt' };
  } catch {
    return { session: false, nudge: false };
  }
})();

interface FocusState {
  targets: FocusTarget[];
  interventions: Intervention[];
  session: FocusSession | null;
  /** Timer that ends the active session at `ends_at` (browser only). */
  timer: ReturnType<typeof setTimeout> | null;
  /** Index of the next intervention message (the engine rotates per process). */
  nextMessage: number;
}

const freshFocusState = (): FocusState => {
  const session = FOCUS_FROM_QUERY.session ? sampleSession() : null;
  return { targets: seedTargets(), interventions: seedInterventions(session?.id ?? null), session, timer: null, nextMessage: 4 };
};

/* ------------------------------------------------------------------ */
/* platform (macOS by default; ?platform=windows|linux for the UI)     */
/* ------------------------------------------------------------------ */

const isPlatform = (v: unknown): v is Platform => v === 'macos' || v === 'windows' || v === 'linux';

/** `?platform=windows|linux` is read once at module load (like `?lang=`), so a reset keeps it; anything else is macOS. */
const PLATFORM_FROM_QUERY: Platform = (() => {
  try {
    if (typeof window === 'undefined') return 'macos';
    const q = new URLSearchParams(window.location.search).get('platform');
    return isPlatform(q) ? q : 'macos';
  } catch {
    return 'macos';
  }
})();

/** What the real engine reports per OS: the data dir, and macOS-only permissions as "not applicable" elsewhere. */
const platformFacts = (platform: Platform): { data_dir: string; permissions: PermissionStatus } => {
  switch (platform) {
    case 'windows':
      return { data_dir: 'C:\\Users\\andrey\\AppData\\Roaming\\ai.ubiqx.app', permissions: { screen_recording: 'not_applicable', automation: 'not_applicable', accessibility: 'not_applicable' } };
    case 'linux':
      return { data_dir: '/home/andrey/.local/share/ai.ubiqx.app', permissions: { screen_recording: 'not_applicable', automation: 'not_applicable', accessibility: 'not_applicable' } };
    default:
      return { data_dir: '/Users/andrey/Library/Application Support/ai.ubiqx.app', permissions: { screen_recording: 'granted', automation: 'granted', accessibility: 'unknown' } };
  }
};

/* ------------------------------------------------------------------ */
/* updates (rolling "continuous" GitHub release)                       */
/* ------------------------------------------------------------------ */

const RELEASE_BASE = 'https://github.com/andreyquadros/ubiquitous-engine/releases';
const UPDATE_FEED_URL = `${RELEASE_BASE}/download/continuous/latest.json`;

/** The build this mock pretends to be running (a CI build, so automatic checks are on). */
const CURRENT_BUILD: BuildInfo = { version: '0.1.0', epoch: 1758200000, number: 26, sha: '14c6e7f', branch: 'main' };

/** The installer of the feed entry the engine would pick on each OS (`platforms["<os>-<arch>"]` of latest.json). */
const platformDownload = (platform: Platform): Pick<ReleaseInfo, 'download_url' | 'app_zip_url' | 'kind'> => {
  const download = `${RELEASE_BASE}/download/continuous`;
  switch (platform) {
    case 'windows':
      return { download_url: `${download}/ubiqX-windows-x86_64-setup.exe`, app_zip_url: null, kind: 'exe' };
    case 'linux':
      return { download_url: `${download}/ubiqX-linux-x86_64.AppImage`, app_zip_url: null, kind: 'appimage' };
    default:
      return { download_url: `${download}/ubiqX-macos-aarch64.dmg`, app_zip_url: `${download}/ubiqX-macos-aarch64.app.zip`, kind: 'dmg' };
  }
};

/** A plausible newer build of the same version, served after `?update=available` or `__mock.setUpdate()`, with the installer of `platform`. */
const sampleRelease = (platform: Platform = S.platform): ReleaseInfo => ({
  version: '0.1.0',
  build: { version: '0.1.0', epoch: 1758221040, number: 27, sha: 'a1b2c3d', branch: 'main' },
  published_at: new Date().toISOString(),
  notes: pick(
    'Aviso de nova versão dentro do app\n\n- Faixa no topo e seção Atualizações em Configurações\n- Notificação do sistema uma vez por build\n- Instaladores para macOS, Windows e Linux no mesmo feed',
    'In-app new version notice\n\n- Top banner and an Updates section in Settings\n- One system notification per build\n- macOS, Windows and Linux installers in the same feed',
  ),
  ...platformDownload(platform),
  release_url: `${RELEASE_BASE}/tag/continuous`,
});

/** `?update=available` is read once at module load (like `?lang=`), so a reset keeps the flag. */
const UPDATE_FROM_QUERY: boolean = (() => {
  try {
    return typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('update') === 'available';
  } catch {
    return false;
  }
})();

interface UpdateState {
  available: ReleaseInfo | null;
  dismissedEpoch: number | null;
  lastCheck: IsoDateTime | null;
  lastError: string | null;
  checking: boolean;
  /** Epoch already pushed as an `update_available` event (the engine emits once per build). */
  announcedEpoch: number | null;
}

const freshUpdateState = (platform: Platform): UpdateState => ({
  available: UPDATE_FROM_QUERY ? sampleRelease(platform) : null,
  dismissedEpoch: null,
  lastCheck: UPDATE_FROM_QUERY ? new Date().toISOString() : null,
  lastError: null,
  checking: false,
  announcedEpoch: null,
});


/* ------------------------------------------------------------------ */
/* license (ubiqX Anual / Mensal)                                      */
/* ------------------------------------------------------------------ */

/** Where the mock pretends the Ubi proxy lives (`UBIQX_API_BASE` of the real build). */
const UBI_API_BASE = 'https://api.ubiqx.ai';
/** Monthly AI budget of a managed subscriber, as the proxy would report it. */
const MANAGED_BUDGET_USD = 6;
const MANAGED_SPENT_USD = 2.35;

const B32 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

/** RFC 4648 base32, upper-case, no padding (the license key's segments). */
function base32Encode(bytes: Uint8Array): string {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += B32[(value << (5 - bits)) & 31];
  return out;
}

function base32Decode(text: string): Uint8Array | null {
  let bits = 0;
  let value = 0;
  const out: number[] = [];
  for (const ch of text.toUpperCase()) {
    const idx = B32.indexOf(ch);
    if (idx < 0) return null;
    value = (value << 5) | idx;
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 255);
      bits -= 8;
    }
  }
  return Uint8Array.from(out);
}

interface MockClaims {
  v: number;
  plan: Plan;
  sub: string;
  email_hash: string;
  issued_at: number;
  expires_at: number;
  seats: number;
}

/** A deterministic 64-byte stand-in for the Ed25519 signature (the mock cannot verify a real one). */
const fakeSignature = (seed: string): Uint8Array => {
  const out = new Uint8Array(64);
  let h = 2166136261;
  for (let i = 0; i < out.length; i++) {
    h ^= seed.charCodeAt(i % seed.length);
    h = Math.imul(h, 16777619) >>> 0;
    out[i] = h & 255;
  }
  return out;
};

/** Builds a key in the real format: `UBIQX-<base32 claims json>-<base32 signature>`. */
function buildLicenseKey(claims: MockClaims): string {
  const json = JSON.stringify(claims);
  const payload = new TextEncoder().encode(json);
  return `UBIQX-${base32Encode(payload)}-${base32Encode(fakeSignature(json))}`;
}

const DAY = 86_400;
/** Claims expiring `daysLeft` days from module load, plus half a day so `days_left` (floored, like the engine) reads `daysLeft` all session long. */
const sampleClaims = (plan: Plan, daysLeft: number): MockClaims => {
  const now = Math.floor(Date.now() / 1000);
  return { v: 1, plan, sub: `sub_${plan === 'monthly_managed' ? 'm' : 'a'}_7f3c21`, email_hash: '4c2b1e9a0d7f6e5c3b2a19080706050403020100ffeeddccbbaa99887766554433', issued_at: now - 30 * DAY, expires_at: now + daysLeft * DAY + DAY / 2, seats: 1 };
};

/** Keys the mock accepts as signed (`__mock.setLicense('annual' | 'managed' | 'expired')`, `?license=`). Any other `UBIQX-…` string is `invalid`. */
export const SAMPLE_LICENSE_KEYS = {
  /** ubiqX Anual, com a sua IA: 335 days left. */
  annual: buildLicenseKey(sampleClaims('annual_own_key', 335)),
  /** ubiqX Mensal, com a IA do Ubi: 22 days left. */
  managed: buildLicenseKey(sampleClaims('monthly_managed', 22)),
  /** An annual key that ran out 3 days ago. */
  expired: buildLicenseKey(sampleClaims('annual_own_key', -3)),
} as const;

export type SampleLicense = keyof typeof SAMPLE_LICENSE_KEYS;

const isPlan = (v: unknown): v is Plan => v === 'annual_own_key' || v === 'monthly_managed';

/** Parses a key the way the real verifier does, minus the signature check (a 64-byte signature segment is enough here). */
function parseLicenseKey(raw: string): MockClaims | null {
  const key = raw.trim();
  if (!key.startsWith('UBIQX-')) return null;
  const parts = key.slice('UBIQX-'.length).split('-');
  if (parts.length !== 2 || !parts[0] || !parts[1]) return null;
  const payload = base32Decode(parts[0]);
  const signature = base32Decode(parts[1]);
  if (!payload || !signature || signature.length !== 64) return null;
  try {
    const claims = JSON.parse(new TextDecoder().decode(payload)) as Partial<MockClaims>;
    if (claims.v !== 1 || !isPlan(claims.plan) || typeof claims.sub !== 'string' || typeof claims.expires_at !== 'number' || typeof claims.issued_at !== 'number') return null;
    return { v: 1, plan: claims.plan, sub: claims.sub, email_hash: String(claims.email_hash ?? ''), issued_at: claims.issued_at, expires_at: claims.expires_at, seats: Number(claims.seats ?? 1) };
  } catch {
    return null;
  }
}

/** `?license=annual|managed|expired` is read once at module load (like `?lang=`), so a reset keeps it. */
const LICENSE_FROM_QUERY: SampleLicense | null = (() => {
  try {
    if (typeof window === 'undefined') return null;
    const v = new URLSearchParams(window.location.search).get('license');
    return v && v in SAMPLE_LICENSE_KEYS ? (v as SampleLicense) : null;
  } catch {
    return null;
  }
})();

interface LicenseState {
  /** The stored key (secret store), or null. Kept even when invalid, like the engine does. */
  key: string | null;
  /** Whether the proxy answers (`false` = network failure: the local verdict stays, usage is null). */
  proxyReachable: boolean;
}

const freshLicenseState = (): LicenseState => ({ key: LICENSE_FROM_QUERY ? SAMPLE_LICENSE_KEYS[LICENSE_FROM_QUERY] : null, proxyReachable: true });

const unlicensed = (): LicenseStatus => ({ state: 'unlicensed', plan: null, expires_at: null, days_left: null, key_hint: null, enforcement: 'soft', managed_usage: null });

/** The local verdict on the stored key at `now` (mirrors `LicenseStatus::evaluate`). */
function evaluateLicense(key: string | null, now = new Date()): LicenseStatus {
  const raw = key?.trim();
  if (!raw) return unlicensed();
  const key_hint = raw.slice(-4);
  const claims = parseLicenseKey(raw);
  if (!claims) return { ...unlicensed(), state: 'invalid', key_hint };
  const secsLeft = claims.expires_at - Math.floor(now.getTime() / 1000);
  return {
    state: secsLeft <= 0 ? 'expired' : 'valid',
    plan: claims.plan,
    expires_at: new Date(claims.expires_at * 1000).toISOString(),
    days_left: Math.max(0, Math.floor(secsLeft / DAY)),
    key_hint,
    enforcement: 'soft',
    managed_usage: null,
  };
}

const currentMonth = (): string => {
  const d = new Date();
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}`;
};

/* ------------------------------------------------------------------ */
/* state                                                               */
/* ------------------------------------------------------------------ */

interface State {
  categories: Category[];
  rules: Rule[];
  blocks: Map<IsoDate, ActivityBlock[]>;
  reports: DailyReport[];
  nudges: Nudge[];
  settings: Settings;
  /** Last 4 chars of the stored key per provider (null = no key in the Keychain). */
  keyHints: Record<AiProvider, string | null>;
  permissions: PermissionStatus;
  aiHealth: AiHealth;
  trackerState: TrackerState;
  usage: AiUsageTotals;
  advices: number;
  update: UpdateState;
  focus: FocusState;
  /** OS the mock pretends to run on (`?platform=` or `__mock.setPlatform`). */
  platform: Platform;
  license: LicenseState;
}

const onboardingFromQuery = (): boolean => {
  try {
    return typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('onboarding') === '1';
  } catch {
    return false;
  }
};

function freshState(): State {
  counter = 1000;
  const t = today();
  const settings = defaultSettings();
  if (onboardingFromQuery()) settings.onboarding_done = false;
  return {
    categories: seedCategories(),
    rules: seedRules(),
    blocks: new Map<IsoDate, ActivityBlock[]>(),
    reports: seedReports(t),
    nudges: FOCUS_FROM_QUERY.nudge ? [focusPromptNudge(), ...seedNudges(t)] : seedNudges(t),
    settings,
    keyHints: { anthropic: 'f3a9', openai: null, xai: null, ubi: null },
    permissions: platformFacts(PLATFORM_FROM_QUERY).permissions,
    aiHealth: { state: 'ok' },
    trackerState: 'running',
    usage: { calls: 412, input_tokens: 1_234_567, output_tokens: 98_765, cost_usd: 1.37 },
    advices: 0,
    update: freshUpdateState(PLATFORM_FROM_QUERY),
    focus: freshFocusState(),
    platform: PLATFORM_FROM_QUERY,
    license: freshLicenseState(),
  };
}

let S: State = freshState();

const dayBlocks = (date: IsoDate): ActivityBlock[] => {
  let list = S.blocks.get(date);
  if (!list) {
    list = buildDay(date);
    S.blocks.set(date, list);
  }
  return list;
};

const allBlocks = (): ActivityBlock[] => [...S.blocks.values()].flat();

const findBlock = (id: Id): { block: ActivityBlock; date: IsoDate } | null => {
  for (const [date, list] of S.blocks) {
    const block = list.find((b) => b.id === id);
    if (block) return { block, date };
  }
  return null;
};

const catById = (id: Id | null): Category | undefined => (id ? S.categories.find((c) => c.id === id) : undefined);
const isProductive = (id: Id | null): boolean => !!catById(id)?.is_productive;
const isRealBlock = (b: ActivityBlock): boolean => b.app_id !== 'idle' && b.app_id !== 'private';
const groupKey = (b: ActivityBlock): string => `${b.app_id}|${b.domain ?? b.title_key}`;

/* ------------------------------------------------------------------ */
/* derived data                                                        */
/* ------------------------------------------------------------------ */

function computeStats(blocks: ActivityBlock[]): FocusStats {
  let productive = 0;
  let distraction = 0;
  let uncategorized = 0;
  let idle = 0;
  let total = 0;
  for (const b of blocks) {
    const secs = secsBetween(b.started_at, b.ended_at);
    if (b.app_id === 'idle' || b.category_id === SYSTEM_CATEGORIES.break) {
      idle += secs;
      continue;
    }
    total += secs;
    if (b.category_id === SYSTEM_CATEGORIES.private) continue;
    if (!b.category_id || b.category_id === SYSTEM_CATEGORIES.uncategorized) uncategorized += secs;
    else if (b.category_id === SYSTEM_CATEGORIES.distraction) distraction += secs;
    else if (isProductive(b.category_id)) productive += secs;
  }
  const real = blocks.filter(isRealBlock);
  const hours = Math.max(total / 3600, 0.25);
  const switches = Math.max(0, real.length - 1) / hours;

  let longest = 0;
  let run = 0;
  let runCat: Id | null = null;
  for (const b of blocks) {
    const secs = secsBetween(b.started_at, b.ended_at);
    if (b.category_id && isProductive(b.category_id)) {
      if (b.category_id === runCat) run += secs;
      else {
        runCat = b.category_id;
        run = secs;
      }
      longest = Math.max(longest, run);
    } else if (secs > 5 * 60 || b.category_id === SYSTEM_CATEGORIES.distraction) {
      run = 0;
      runCat = null;
    }
  }

  const base = total > 0 ? productive / total : 0;
  const penalty = Math.max(0, switches - 4) * 1.5;
  const score = total > 0 ? Math.max(0, Math.min(100, Math.round(base * 100 - penalty))) : 0;
  let mood: Mood = 'calm';
  if (S.trackerState === 'paused' || S.trackerState === 'idle' || total === 0) mood = 'sleeping';
  else if (score >= 80) mood = 'excited';
  else if (score >= 60) mood = 'focused';
  else if (score >= 40) mood = 'calm';
  else mood = 'worried';

  return {
    focus_score: score,
    productive_secs: Math.round(productive),
    distraction_secs: Math.round(distraction),
    uncategorized_secs: Math.round(uncategorized),
    idle_secs: Math.round(idle),
    total_secs: Math.round(total),
    switches_per_hour: Math.round(switches * 10) / 10,
    longest_focus_secs: Math.round(longest),
    mood,
  };
}

function hourlyFocus(date: IsoDate, blocks: ActivityBlock[]): (number | null)[] {
  const out: (number | null)[] = [];
  for (let h = 0; h < 24; h++) {
    const hs = new Date(atLocal(date, h, 0)).getTime();
    const he = hs + 3600_000;
    let prod = 0;
    let tot = 0;
    for (const b of blocks) {
      if (!isRealBlock(b)) continue;
      const s = Math.max(hs, new Date(b.started_at).getTime());
      const e = Math.min(he, new Date(b.ended_at).getTime());
      if (e <= s) continue;
      tot += e - s;
      if (isProductive(b.category_id)) prod += e - s;
    }
    out.push(tot < 60_000 ? null : Math.round((prod / tot) * 100));
  }
  return out;
}

function dashboard(date: IsoDate): DashboardData {
  const blocks = dayBlocks(date);
  const totals = new Map<Id | null, { secs: number; block_count: number }>();
  const apps = new Map<string, { app_name: string; secs: number }>();
  for (const b of blocks) {
    const secs = secsBetween(b.started_at, b.ended_at);
    const key = b.category_id ?? null;
    const t = totals.get(key) ?? { secs: 0, block_count: 0 };
    t.secs += secs;
    t.block_count += 1;
    totals.set(key, t);
    if (isRealBlock(b)) {
      const a = apps.get(b.app_id) ?? { app_name: b.app_name, secs: 0 };
      a.secs += secs;
      apps.set(b.app_id, a);
    }
  }
  const open = blocks.find((b) => b.is_open) ?? null;
  return {
    date,
    stats: computeStats(blocks),
    totals: [...totals.entries()]
      .map(([category_id, t]) => ({ category_id, secs: Math.round(t.secs), block_count: t.block_count }))
      .sort((a, b) => b.secs - a.secs),
    top_apps: [...apps.entries()]
      .map(([app_id, a]) => ({ app_id, app_name: a.app_name, secs: Math.round(a.secs) }))
      .sort((a, b) => b.secs - a.secs)
      .slice(0, 8),
    timeline: blocks,
    open_block: open,
    categories: S.categories.filter((c) => !c.archived),
    unseen_nudges: S.nudges.filter((n) => !n.seen),
    tracker_state: S.trackerState,
    ai_health: S.aiHealth,
    usage_month: S.usage,
    budget_usd: S.settings.ai_monthly_budget_usd,
    needs_review: blocks.filter((b) => b.needs_review).length,
    hourly_focus: hourlyFocus(date, blocks),
  };
}

function reviewGroups(date: IsoDate): BlockGroup[] {
  const groups = new Map<string, BlockGroup>();
  for (const b of dayBlocks(date)) {
    if (!isRealBlock(b)) continue;
    const key = groupKey(b);
    const secs = secsBetween(b.started_at, b.ended_at);
    const g = groups.get(key);
    if (!g) {
      groups.set(key, {
        key,
        app_id: b.app_id,
        app_name: b.app_name,
        domain: b.domain,
        title: b.title,
        total_secs: Math.round(secs),
        block_ids: [b.id],
        category_id: b.category_id,
        min_confidence: b.confidence,
        source: b.source,
        needs_review: b.needs_review,
        description: b.description,
        first_started_at: b.started_at,
      });
    } else {
      g.total_secs += Math.round(secs);
      g.block_ids.push(b.id);
      g.min_confidence = Math.min(g.min_confidence, b.confidence);
      g.needs_review = g.needs_review || b.needs_review;
      g.description = g.description ?? b.description;
      if (!g.category_id && b.category_id) g.category_id = b.category_id;
    }
  }
  const rank = (g: BlockGroup): number => {
    if (g.needs_review) return 0;
    if (!g.category_id) return 1;
    if (g.min_confidence < 0.8) return 2;
    return 3;
  };
  return [...groups.values()].sort((a, b) => rank(a) - rank(b) || a.min_confidence - b.min_confidence || b.total_secs - a.total_secs);
}

function suggestionsFor(block: ActivityBlock, categoryId: Id): RuleSuggestion[] {
  const out: RuleSuggestion[] = [];
  const catName = catById(categoryId)?.name ?? categoryId;
  if (block.domain) {
    const support = allBlocks().filter((b) => b.domain === block.domain).length;
    if (!S.rules.some((r) => r.matcher === 'domain' && r.pattern === block.domain && r.category_id === categoryId)) {
      out.push({
        category_id: categoryId,
        matcher: 'domain',
        pattern: block.domain,
        support,
        rationale: `${support} bloco(s) em ${block.domain} — sempre classificar como ${catName}?`,
        auto_apply_safe: support >= 3,
      });
    }
  } else if (block.app_name && block.app_id !== 'manual') {
    const support = allBlocks().filter((b) => b.app_id === block.app_id).length;
    out.push({
      category_id: categoryId,
      matcher: 'app',
      pattern: block.app_name,
      support,
      rationale: `${support} bloco(s) no app ${block.app_name}`,
      auto_apply_safe: false,
    });
  }
  const words = block.title_key.split(' ').filter((w) => w.length >= 6);
  if (words[0] && block.domain !== 'mail.google.com') {
    out.push({
      category_id: categoryId,
      matcher: 'title_contains',
      pattern: words[0],
      support: allBlocks().filter((b) => b.title_key.includes(words[0] ?? '')).length,
      rationale: `Títulos contendo “${words[0]}”`,
      auto_apply_safe: false,
    });
  }
  return out.slice(0, 2);
}

function applyCorrection(target: ActivityBlock, categoryId: Id, scope: 'block' | 'day' | 'month' | 'group', date: IsoDate): CorrectionOutcome {
  const key = groupKey(target);
  const changed: Id[] = [];
  const apply = (b: ActivityBlock) => {
    b.category_id = categoryId;
    b.confidence = 1;
    b.source = 'user';
    b.needs_review = false;
    changed.push(b.id);
  };
  apply(target);
  let backfilled = 0;
  const candidates =
    scope === 'block'
      ? []
      : scope === 'month'
        ? allBlocks().filter((b) => localDateOf(b.started_at).slice(0, 7) === date.slice(0, 7))
        : dayBlocks(date);
  for (const b of candidates) {
    if (b.id === target.id || groupKey(b) !== key) continue;
    if (scope === 'group' || b.source !== 'user') {
      apply(b);
      if (b.source !== 'user' || scope === 'group') backfilled += 1;
    }
  }
  // memory backfill: other low-confidence blocks with the same key today
  if (scope === 'block') {
    for (const b of dayBlocks(date)) {
      if (b.id !== target.id && groupKey(b) === key && (!b.category_id || b.confidence < S.settings.min_confidence)) {
        apply(b);
        backfilled += 1;
      }
    }
  }
  const suggestions = suggestionsFor(target, categoryId);
  const auto_rules: Rule[] = [];
  const disabled_rules: Rule[] = [];
  for (const r of S.rules) {
    if (!r.enabled) continue;
    const matches =
      (r.matcher === 'domain' && target.domain === r.pattern) ||
      (r.matcher === 'app' && target.app_name === r.pattern) ||
      (r.matcher === 'title_contains' && target.title.toLowerCase().includes(r.pattern.toLowerCase()));
    if (matches && r.category_id !== categoryId) {
      r.miss_count += 1;
      r.last_contradicted_at = new Date().toISOString();
      if (r.origin === 'learned' && r.miss_count > r.hit_count) {
        r.enabled = false;
        disabled_rules.push(r);
      }
    }
  }
  return { block_ids: changed, backfilled, suggestions, auto_rules, disabled_rules };
}

const kindForBlock = (b: ActivityBlock): ActivityKind => {
  if (b.app_name === 'Visual Studio Code' || b.app_name === 'Terminal' || b.domain === 'github.com') return 'desenvolvimento';
  if (b.app_name === 'Microsoft Teams') return 'reuniao';
  if (b.app_name === 'WhatsApp' || b.domain === 'mail.google.com' || b.domain === 'web.whatsapp.com') return 'comunicacao';
  if (b.domain === 'docs.google.com' || b.domain === 'notion.so' || b.app_name === 'Notas') return 'documentacao';
  if (b.domain === 'sei.ifro.edu.br') return 'gestao';
  if (b.domain === 'suap.ifro.edu.br') return 'ensino';
  if (b.domain?.endsWith('.gov.br') || b.app_name === 'Preview') return 'pesquisa';
  return 'outro';
};

function buildReport(date: IsoDate, categoryId: Id, previous?: DailyReport): DailyReport {
  const cat = catById(categoryId);
  const blocks = dayBlocks(date).filter((b) => b.category_id === categoryId);
  const byKey = new Map<string, ActivityBlock[]>();
  for (const b of blocks) {
    const k = b.description ?? b.title_key;
    byKey.set(k, [...(byKey.get(k) ?? []), b]);
  }
  const items: ReportItem[] = [...byKey.values()]
    .map((list) => {
      const first = list[0] as ActivityBlock;
      const minutes = Math.round(list.reduce((s, b) => s + secsBetween(b.started_at, b.ended_at), 0) / 60);
      const last = list[list.length - 1] as ActivityBlock;
      return {
        activity: first.description ?? first.title,
        kind: kindForBlock(first),
        minutes,
        evidence: [...new Set(list.map((b) => `${b.domain ?? b.app_name} · ${b.title}`))].slice(0, 3),
        time_range: `${fmtHM(first.started_at)}–${fmtHM(last.ended_at)}`,
        continuation_of: null,
      };
    })
    .filter((i) => i.minutes >= 2)
    .sort((a, b) => b.minutes - a.minutes);
  const total = blocks.reduce((s, b) => s + secsBetween(b.started_at, b.ended_at), 0);
  const top = items.slice(0, 3).map((i) => i.activity);
  const hours = (total / 3600).toLocaleString(en() ? 'en-US' : 'pt-BR', { maximumFractionDigits: 1 });
  const summary_md = en()
    ? `## Day summary — ${cat?.name ?? categoryId}\n\n` +
      (items.length
        ? `**${hours} h** across ${items.length} ${items.length === 1 ? 'activity' : 'activities'}. ${top.length ? `Main ones: ${top.map((t) => `_${t}_`).join('; ')}.` : ''}\n\n**Highlights**\n\n${top.map((t) => `- ${t}`).join('\n')}\n`
        : `No activity recorded in this category on ${date}.\n`)
    : `## Resumo do dia — ${cat?.name ?? categoryId}\n\n` +
      (items.length
        ? `Foram **${hours} h** em ${items.length} atividade(s). ${top.length ? `As principais: ${top.map((t) => `_${t}_`).join('; ')}.` : ''}\n\n**Destaques**\n\n${top.map((t) => `- ${t}`).join('\n')}\n`
        : `Nenhuma atividade registrada nesta categoria em ${date}.\n`);
  return {
    id: previous?.id ?? uid('rep'),
    date,
    category_id: categoryId,
    generated_at: new Date().toISOString(),
    summary_md,
    items,
    highlights: top,
    total_secs: Math.round(total),
    model: S.settings.models.report,
    input_tokens: 4200 + items.length * 310,
    output_tokens: 700 + items.length * 90,
    stale: false,
    edited: false,
  };
}

function monthlyReport(categoryId: Id, year: number, month: number): string {
  const cat = catById(categoryId);
  const prefix = `${year}-${pad(month)}`;
  const t = today();
  const rows: string[] = [];
  let totalSecs = 0;
  const kinds = new Map<string, number>();
  for (let d = 1; d <= 31; d++) {
    const date = `${prefix}-${pad(d)}`;
    if (date > t) break;
    const [, m] = date.split('-');
    if (m !== pad(month)) break;
    const blocks = dayBlocks(date).filter((b) => b.category_id === categoryId);
    if (!blocks.length) continue;
    const secs = blocks.reduce((s, b) => s + secsBetween(b.started_at, b.ended_at), 0);
    totalSecs += secs;
    for (const b of blocks) kinds.set(kindForBlock(b), (kinds.get(kindForBlock(b)) ?? 0) + secsBetween(b.started_at, b.ended_at));
    const main = blocks.sort((a, b) => secsBetween(b.started_at, b.ended_at) - secsBetween(a.started_at, a.ended_at))[0];
    rows.push(`| ${date.slice(8)}/${pad(month)} | ${(secs / 3600).toLocaleString('pt-BR', { maximumFractionDigits: 1 })} h | ${main?.description ?? main?.title ?? '—'} |`);
  }
  const kindRows = [...kinds.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([k, s]) => `- **${k}**: ${(s / 3600).toLocaleString('pt-BR', { maximumFractionDigits: 1 })} h`)
    .join('\n');
  return (
    `# ${cat?.name ?? categoryId} — ${pad(month)}/${year}\n\n` +
    `Total no mês: **${(totalSecs / 3600).toLocaleString('pt-BR', { maximumFractionDigits: 1 })} h** em ${rows.length} dia(s) com atividade.\n\n` +
    `## Por tipo de atividade\n\n${kindRows || '- (sem dados)'}\n\n` +
    `## Por dia\n\n| Dia | Horas | Atividade principal |\n|-----|-------|----------------------|\n${rows.join('\n') || '| — | — | — |'}\n\n` +
    `_Gerado pelo ubiqX com ${activeModel('report')}._\n`
  );
}

/** Last 4 chars of the stored key of `p`; for the managed provider that is the license key itself (`ubiqx.license`). */
const keyHint = (p: AiProvider): string | null => (p === 'ubi' ? (S.license.key ? S.license.key.trim().slice(-4) : null) : S.keyHints[p]);
const keyConfigured = (p: AiProvider = S.settings.ai_provider): boolean => keyHint(p) !== null;
const keyStatus = (p: AiProvider): ApiKeyStatus => ({ provider: p, configured: keyConfigured(p), hint: keyHint(p) ? `…${keyHint(p)}` : null });

/** The license as `get_license_status` reports it: the local verdict plus, for a valid managed key, the proxy's usage. */
function licenseStatus(): LicenseStatus {
  const status = evaluateLicense(S.license.key);
  if (licenseAllowsManaged(status) && S.license.proxyReachable) status.managed_usage = { month: currentMonth(), spent_usd: MANAGED_SPENT_USD, budget_usd: MANAGED_BUDGET_USD };
  return status;
}
/** Provider + model that would answer a remote call right now, for user-facing strings. */
const activeModel = (slot: keyof AiModels): string => `${providerInfo(S.settings.ai_provider).label} · ${S.settings.models[slot]}`;

function settingsView(): SettingsView {
  const selected = keyStatus(S.settings.ai_provider);
  return {
    settings: structuredClone(S.settings),
    api_key_configured: selected.configured,
    api_key_hint: selected.hint,
    api_keys: PROVIDER_IDS.map(keyStatus),
    providers: structuredClone(PROVIDERS),
    permissions: { ...S.permissions },
    ai_health: S.aiHealth,
    tracker_state: S.trackerState,
    license: licenseStatus(),
    ubi_api_base: UBI_API_BASE,
    data_dir: platformFacts(S.platform).data_dir,
    platform: S.platform,
    version: '0.1.0',
  };
}

function updateStatus(): UpdateStatus {
  const u = S.update;
  return {
    current: { ...CURRENT_BUILD },
    feed_url: UPDATE_FEED_URL,
    enabled: CURRENT_BUILD.epoch !== 0,
    available: u.available ? structuredClone(u.available) : null,
    dismissed: u.available !== null && u.dismissedEpoch === u.available.build.epoch,
    last_check: u.lastCheck,
    last_error: u.lastError,
    checking: u.checking,
  };
}

/** Pushes `update_available` for the current release once per build epoch, like the engine does. */
function announceUpdate(): void {
  const rel = S.update.available;
  if (!rel || S.update.announcedEpoch === rel.build.epoch) return;
  S.update.announcedEpoch = rel.build.epoch;
  emit({ type: 'update_available', release: structuredClone(rel) });
}

function refreshAiHealth(): void {
  // EngineState::key_health: the managed provider without a valid monthly license is "not configured" (no fallback).
  if (S.settings.ai_provider === 'ubi' && !licenseAllowsManaged(licenseStatus())) S.aiHealth = { state: 'not_configured' };
  else if (!keyConfigured()) S.aiHealth = { state: 'not_configured' };
  else if (S.settings.local_only) S.aiHealth = { state: 'paused', reason: 'Modo somente local ativado' };
  else if (S.usage.cost_usd >= S.settings.ai_monthly_budget_usd) S.aiHealth = { state: 'paused', reason: 'Orçamento mensal atingido' };
  else S.aiHealth = { state: 'ok' };
}

/* ------------------------------------------------------------------ */
/* focus guard: derived data and session lifecycle                     */
/* ------------------------------------------------------------------ */

const sortTargets = (list: FocusTarget[]): FocusTarget[] => [...list].sort((a, b) => Number(b.enabled) - Number(a.enabled) || a.name.localeCompare(b.name));

/** Domains of the last 7 days of blocks, most time first (`list_known_domains`). */
function knownDomains(limit: number): KnownDomain[] {
  const secs = new Map<string, number>();
  for (let i = 0; i < 7; i++) {
    for (const b of dayBlocks(shiftDays(today(), -i))) {
      if (!b.domain) continue;
      const d = normalizeDomain(b.domain);
      secs.set(d, (secs.get(d) ?? 0) + secsBetween(b.started_at, b.ended_at));
    }
  }
  return [...secs.entries()]
    .map(([domain, seconds]) => ({ domain, seconds: Math.round(seconds) }))
    .sort((a, b) => b.seconds - a.seconds)
    .slice(0, limit);
}

const activeSession = (): FocusSession | null => {
  const s = S.focus.session;
  return s && !s.ended_at ? s : null;
};

function focusStatus(): FocusStatus {
  const session = activeSession();
  const t = today();
  return {
    session: session ? structuredClone(session) : null,
    remaining_secs: session ? Math.max(0, Math.round((new Date(session.ends_at).getTime() - Date.now()) / 1000)) : null,
    targets_enabled: S.focus.targets.filter((x) => x.enabled).length,
    interventions_today: S.focus.interventions.filter((i) => localDateOf(i.at) === t).length,
    guard_enabled: S.settings.focus.guard_enabled,
  };
}

/** Ends the active session (timer or user): `ended_at`, the `focus_session` event and a Praise nudge when it lasted 5 min or more. */
function endSession(): FocusSession | null {
  const session = activeSession();
  if (!session) return null;
  if (S.focus.timer) {
    clearTimeout(S.focus.timer);
    S.focus.timer = null;
  }
  session.ended_at = new Date().toISOString();
  const minutes = Math.round(secsBetween(session.started_at, session.ended_at) / 60);
  if (minutes >= 5) {
    const nudge: Nudge = {
      id: uid('ndg'),
      at: session.ended_at,
      kind: 'praise',
      title: pick('Sessão de foco concluída', 'Focus session done'),
      message: pick(
        `Sessão de foco concluída: ${minutes} min em "${session.task}", ${session.interventions} distrações seguradas.`,
        `Focus session done: ${minutes} min on "${session.task}", ${session.interventions} distractions held.`,
      ),
      seen: false,
    };
    S.nudges.unshift(nudge);
    setTimeout(() => emit({ type: 'nudge', nudge }), 20);
  }
  const snapshot = structuredClone(session);
  setTimeout(() => emit({ type: 'focus_session', session: snapshot }), 10);
  return snapshot;
}

/** What the guard does when a listed app or site comes to the front: records the intervention and pushes the event. */
function intervene(target: FocusTarget): Intervention {
  const session = activeSession();
  const at = new Date().toISOString();
  const intervention: Intervention = {
    id: uid('itv'),
    at,
    target_id: target.id,
    kind: target.kind,
    name: target.name,
    key: target.key,
    action: target.kind === 'app' ? 'app_quit' : 'tab_closed',
    session_id: session?.id ?? null,
    message: interventionMessage(S.focus.nextMessage++),
  };
  S.focus.interventions.unshift(intervention);
  target.blocked_count += 1;
  target.last_blocked_at = at;
  emit({ type: 'intervention', intervention: structuredClone(intervention) });
  if (session) {
    session.interventions += 1;
    emit({ type: 'focus_session', session: structuredClone(session) });
  }
  return intervention;
}

/** The intervention window, previewed in the browser as a 460×188 popup (the Tauri shell opens a real window). */
function openInterventionPreview(id: string): void {
  if (typeof window === 'undefined' || !LATENCY_MS) return;
  const url = `${window.location.pathname}${window.location.search}#/intervention?id=${encodeURIComponent(id)}`;
  window.open(url, 'ubiqx-intervention', 'popup=yes,width=460,height=188,top=24');
}

/* ------------------------------------------------------------------ */
/* events                                                              */
/* ------------------------------------------------------------------ */

type Handler = (e: EngineEvent) => void;
const subscribers = new Set<Handler>();
let ticker: ReturnType<typeof setInterval> | null = null;
let tick = 0;

const emit = (e: EngineEvent): void => {
  for (const h of subscribers) h(e);
};

const FAKE_OPENS: [string, string, string | null, Id][] = [
  ['Google Chrome', 'Tauri 2 — Window customization', 'https://v2.tauri.app/learn/window-customization/', CAT_INCUB],
  ['Visual Studio Code', 'dashboard.py — cidades-inteligentes', null, CAT_CIDADES],
  ['Google Chrome', 'Caixa de entrada (2) - andrey.quadros@ifro.edu.br - Gmail', 'https://mail.google.com/mail/u/0/#inbox', CAT_IFRO],
];

function fakeTick(): void {
  tick += 1;
  const t = today();
  if (tick % 2 === 1) {
    const blocks = dayBlocks(t);
    const now = new Date().toISOString();
    for (const b of blocks) {
      if (b.is_open) {
        b.is_open = false;
        if (new Date(b.ended_at).getTime() < Date.now()) b.ended_at = now;
      }
    }
    const [app, title, url, cat] = FAKE_OPENS[(tick >> 1) % FAKE_OPENS.length] as [string, string, string | null, Id];
    const spec: Spec = [1, app, title, url, cat, 0.8, 'llm', { sent: true }];
    const b = specToBlock(t, spec, now, 1);
    b.is_open = true;
    blocks.push(b);
    emit({ type: 'block_opened', block: b });
  } else {
    const nudge: Nudge = {
      id: uid('ndg'),
      at: new Date().toISOString(),
      kind: tick % 4 === 0 ? 'praise' : 'attention',
      title: tick % 4 === 0 ? pick('Bom ritmo!', 'Good pace!') : pick('Bloco novo aguardando revisão', 'New block waiting for review'),
      message:
        tick % 4 === 0
          ? pick('Você está mantendo um bom ritmo nesta última hora. Que tal fechar essa tarefa antes de trocar de contexto?', "You've kept a good pace this past hour. How about wrapping up this task before switching context?")
          : pick('Classifiquei um bloco com baixa confiança. Dê uma olhada na Revisão quando puder.', 'I classified a block with low confidence. Take a look at Review when you get a chance.'),
      seen: false,
    };
    S.nudges.unshift(nudge);
    emit({ type: 'nudge', nudge });
  }
}

export function subscribe(handler: Handler): () => void {
  subscribers.add(handler);
  if (!ticker && import.meta.env.MODE !== 'test') ticker = setInterval(fakeTick, 20_000);
  // An update set before the UI subscribed (?update=available, __mock.setUpdate) is announced shortly after, once.
  if (S.update.available && S.update.announcedEpoch !== S.update.available.build.epoch) setTimeout(announceUpdate, LATENCY_MS ? 800 : 20);
  return () => {
    subscribers.delete(handler);
    if (subscribers.size === 0 && ticker) {
      clearInterval(ticker);
      ticker = null;
    }
  };
}

/* ------------------------------------------------------------------ */
/* screenshots                                                         */
/* ------------------------------------------------------------------ */

const escapeXml = (s: string): string => s.replace(/[<>&"']/g, (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&#39;' })[c] ?? c);
const toBase64 = (s: string): string => {
  const bytes = new TextEncoder().encode(s);
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
};

/** A 640×400 SVG "screenshot" showing the app name and window title, so the review panel can be exercised without real captures. */
export function placeholderScreenshot(app: string, title: string): ScreenshotData {
  const color = `hsl(${[...app].reduce((h, ch) => (h * 31 + ch.charCodeAt(0)) % 360, 0)} 60% 55%)`;
  const shortTitle = title.length > 64 ? `${title.slice(0, 63)}…` : title;
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400" viewBox="0 0 640 400">` +
    `<rect width="640" height="400" fill="#0c1220"/>` +
    `<rect x="0" y="0" width="640" height="36" fill="#121a2b"/>` +
    `<circle cx="18" cy="18" r="6" fill="#ff5c7a"/><circle cx="38" cy="18" r="6" fill="#ffc24d"/><circle cx="58" cy="18" r="6" fill="#2ee6a6"/>` +
    `<text x="320" y="23" fill="#9daec7" font-family="Inter, sans-serif" font-size="13" text-anchor="middle">${escapeXml(shortTitle)}</text>` +
    `<rect x="24" y="60" width="592" height="316" rx="10" fill="#121a2b" stroke="${color}" stroke-opacity=".35"/>` +
    `<text x="320" y="200" fill="${color}" font-family="Sora, Inter, sans-serif" font-size="28" font-weight="600" text-anchor="middle">${escapeXml(app)}</text>` +
    `<text x="320" y="236" fill="#7487a6" font-family="Inter, sans-serif" font-size="14" text-anchor="middle">${escapeXml(pick('captura simulada', 'simulated capture'))}</text>` +
    `</svg>`;
  return { mime: 'image/svg+xml', data_base64: toBase64(svg), width: 640, height: 400 };
}

/* ------------------------------------------------------------------ */
/* command handlers                                                    */
/* ------------------------------------------------------------------ */

const str = (v: unknown, name: string): string => {
  if (typeof v !== 'string') throw new Error(`mock: argumento ${name} inválido`);
  return v;
};
const num = (v: unknown, fallback: number): number => (typeof v === 'number' ? v : fallback);

type Cmd = (args: Record<string, unknown>) => unknown;

const commands: Record<string, Cmd> = {
  get_dashboard: (a) => dashboard(str(a.date, 'date')),
  get_timeline: (a) => [...dayBlocks(str(a.date, 'date'))].sort((x, y) => x.started_at.localeCompare(y.started_at)),
  get_review_groups: (a) => reviewGroups(str(a.date, 'date')),
  get_ai_sent: (a) => dayBlocks(str(a.date, 'date')).filter((b) => b.ai_sent_at),
  get_screenshot: (a) => {
    const found = findBlock(str(a.blockId, 'blockId'));
    if (!found) throw new Error('Bloco não encontrado');
    if (!found.block.screenshot_id) return null;
    return placeholderScreenshot(found.block.app_name, found.block.title);
  },

  reclassify: (a) => {
    const found = findBlock(str(a.blockId, 'blockId'));
    if (!found) throw new Error('Bloco não encontrado');
    const scope = (a.scope as 'block' | 'day' | 'month') ?? 'block';
    if (typeof a.note === 'string' && a.note) found.block.note = a.note;
    return applyCorrection(found.block, str(a.categoryId, 'categoryId'), scope, found.date);
  },
  reclassify_group: (a) => {
    const date = str(a.date, 'date');
    const key = str(a.key, 'key');
    const target = dayBlocks(date).find((b) => groupKey(b) === key);
    if (!target) throw new Error('Grupo não encontrado');
    return applyCorrection(target, str(a.categoryId, 'categoryId'), 'group', date);
  },
  accept_rule_suggestion: (a) => {
    const s = a.suggestion as RuleSuggestion;
    const rule: Rule = {
      id: uid('rule'),
      category_id: s.category_id,
      matcher: s.matcher,
      pattern: s.pattern,
      priority: 10,
      origin: 'learned',
      enabled: true,
      created_at: new Date().toISOString(),
      hit_count: s.support,
      miss_count: 0,
      last_contradicted_at: null,
    };
    S.rules.push(rule);
    return rule;
  },
  split_block: (a) => {
    const found = findBlock(str(a.blockId, 'blockId'));
    if (!found) throw new Error('Bloco não encontrado');
    const at = str(a.at, 'at');
    const { block, date } = found;
    if (at <= block.started_at || at >= block.ended_at) throw new Error('O horário precisa estar dentro do bloco');
    const second: ActivityBlock = { ...block, id: uid('blk'), started_at: at, is_open: block.is_open, sample_count: Math.max(1, Math.round(secsBetween(at, block.ended_at) / 5)) };
    block.ended_at = at;
    block.is_open = false;
    block.sample_count = Math.max(1, Math.round(secsBetween(block.started_at, at) / 5));
    const list = dayBlocks(date);
    list.splice(list.indexOf(block) + 1, 0, second);
    return second.id;
  },
  add_manual_entry: (a) => {
    const startedAt = str(a.startedAt, 'startedAt');
    const endedAt = str(a.endedAt, 'endedAt');
    const note = typeof a.note === 'string' ? a.note : null;
    const block: ActivityBlock = {
      id: uid('blk'),
      started_at: startedAt,
      ended_at: endedAt,
      app_name: 'Manual',
      app_id: 'manual',
      title: note ?? pick('Atividade manual', 'Manual activity'),
      title_key: titleKey(note ?? 'atividade manual'),
      url: null,
      domain: null,
      category_id: str(a.categoryId, 'categoryId'),
      confidence: 1,
      source: 'user',
      description: note,
      screenshot_id: null,
      sample_count: 0,
      is_open: false,
      classify_attempts: 0,
      next_attempt_at: null,
      needs_review: false,
      ai_payload: null,
      ai_sent_at: null,
      is_manual: true,
      note,
    };
    const list = dayBlocks(localDateOf(startedAt));
    list.push(block);
    list.sort((x, y) => x.started_at.localeCompare(y.started_at));
    return block;
  },
  classify_now: () => {
    const report: ClassifyReport = { local: 0, remote: 0, vision: 0, needs_review: 0, skipped_remote: S.settings.local_only || !keyConfigured() };
    for (const b of dayBlocks(today())) {
      if (b.category_id || !isRealBlock(b)) continue;
      const rule = S.rules.find((r) => r.enabled && ((r.matcher === 'domain' && r.pattern === b.domain) || (r.matcher === 'app' && r.pattern === b.app_name)));
      if (rule) {
        b.category_id = rule.category_id;
        b.confidence = 1;
        b.source = 'rule';
        rule.hit_count += 1;
        report.local += 1;
      } else if (!report.skipped_remote) {
        b.category_id = b.app_name === 'Finder' ? CAT_INCUB : CAT_IFRO;
        b.confidence = 0.55 + Math.random() * 0.2;
        b.source = 'llm';
        b.needs_review = b.confidence < S.settings.min_confidence;
        b.ai_sent_at = new Date().toISOString();
        b.ai_payload = JSON.stringify({ app: b.app_name, title: b.title, domain: b.domain });
        report.remote += 1;
        if (b.needs_review) report.needs_review += 1;
        S.usage.calls += 1;
        S.usage.input_tokens += 320;
        S.usage.output_tokens += 40;
        S.usage.cost_usd = Math.round((S.usage.cost_usd + 0.0005) * 10000) / 10000;
      }
    }
    if (report.local + report.remote > 0) {
      const ids = dayBlocks(today()).filter((b) => b.category_id).map((b) => b.id);
      setTimeout(() => emit({ type: 'blocks_classified', block_ids: ids }), 50);
    }
    return report;
  },

  list_categories: (a) => S.categories.filter((c) => a.includeArchived === true || !c.archived),
  save_category: (a) => {
    const cat = structuredClone(a.category as Category);
    if (!cat.id) cat.id = uid('cat');
    const idx = S.categories.findIndex((c) => c.id === cat.id);
    if (idx >= 0) {
      const existing = S.categories[idx] as Category;
      if (existing.is_system) {
        // only cosmetic edits are allowed on system categories
        S.categories[idx] = { ...existing, color: cat.color, icon: cat.icon };
      } else S.categories[idx] = cat;
    } else {
      if (!cat.created_at) cat.created_at = new Date().toISOString();
      S.categories.push(cat);
    }
    return S.categories.find((c) => c.id === cat.id);
  },
  delete_category: (a) => {
    const id = str(a.id, 'id');
    const cat = S.categories.find((c) => c.id === id);
    if (!cat) return;
    if (cat.is_system) throw new Error('Categorias do sistema não podem ser removidas');
    cat.archived = true;
    for (const b of allBlocks()) if (b.category_id === id) b.category_id = null;
  },
  list_rules: () => [...S.rules].sort((x, y) => x.priority - y.priority || y.hit_count - x.hit_count),
  save_rule: (a) => {
    const rule = structuredClone(a.rule as Rule);
    if (!rule.id) rule.id = uid('rule');
    const idx = S.rules.findIndex((r) => r.id === rule.id);
    if (idx >= 0) S.rules[idx] = rule;
    else S.rules.push(rule);
    return rule;
  },
  delete_rule: (a) => {
    S.rules = S.rules.filter((r) => r.id !== a.id);
  },

  get_reports: (a) => S.reports.filter((r) => r.date === a.date),
  list_reports_between: (a) => S.reports.filter((r) => r.date >= str(a.from, 'from') && r.date <= str(a.to, 'to')),
  generate_report: async (a) => {
    await sleep(LATENCY_MS ? 900 : 0);
    const date = str(a.date, 'date');
    const categoryId = str(a.categoryId, 'categoryId');
    const idx = S.reports.findIndex((r) => r.date === date && r.category_id === categoryId);
    const report = buildReport(date, categoryId, idx >= 0 ? S.reports[idx] : undefined);
    if (idx >= 0) S.reports[idx] = report;
    else S.reports.push(report);
    S.usage.calls += 1;
    S.usage.input_tokens += report.input_tokens;
    S.usage.output_tokens += report.output_tokens;
    S.usage.cost_usd = Math.round((S.usage.cost_usd + 0.04) * 100) / 100;
    setTimeout(() => emit({ type: 'report_ready', report }), 30);
    return report;
  },
  update_report: (a) => {
    const report = structuredClone(a.report as DailyReport);
    report.edited = true;
    const idx = S.reports.findIndex((r) => r.id === report.id);
    if (idx >= 0) S.reports[idx] = report;
    else S.reports.push(report);
    return report;
  },
  get_monthly_report: async (a) => {
    await sleep(LATENCY_MS ? 500 : 0);
    return monthlyReport(str(a.categoryId, 'categoryId'), num(a.year, new Date().getFullYear()), num(a.month, new Date().getMonth() + 1));
  },

  get_nudges: (a) => [...S.nudges].sort((x, y) => y.at.localeCompare(x.at)).slice(0, num(a.limit, 30)),
  mark_nudges_seen: () => {
    for (const n of S.nudges) n.seen = true;
  },
  snooze_nudges: (a) => {
    S.settings.nudges.snoozed_until = new Date(Date.now() + num(a.minutes, 30) * 60_000).toISOString();
  },
  get_advice: async (): Promise<Advice> => {
    await sleep(LATENCY_MS ? 1400 : 0);
    S.advices += 1;
    S.usage.calls += 1;
    S.usage.cost_usd = Math.round((S.usage.cost_usd + 0.03) * 100) / 100;
    if (en()) {
      return {
        headline: 'A productive week, but your afternoons were fragmented.',
        recommendations: [
          'Your mornings (8 to 11) hold your longest focus blocks: keep them for development (api-incubadora and the sensor ingestion) and push email and SEI to after 4 pm.',
          'Gmail showed up 6 times during the day in short blocks. Batching it into 2 fixed sessions would cut about 10 context switches a day.',
          'Tuesdays and Thursdays are packed with Teams meetings. Block 90 min before them to prepare mentoring sessions; AgroTech asked for pitch material twice.',
          'YouTube blocks were classified as Smart Cities in 1 of 3 cases: create a “TEDx → Smart Cities” rule to skip the manual review.',
        ],
        model: S.settings.models.report,
      };
    }
    return {
      headline: 'Sua semana foi produtiva, mas fragmentada nas tardes.',
      recommendations: [
        'Suas manhãs (08h–11h) têm os maiores blocos de foco: reserve-as para desenvolvimento (api-incubadora e ingestão de sensores) e deixe e-mails/SEI para depois das 16h.',
        'O Gmail apareceu 6 vezes ao longo do dia em blocos curtos. Agrupar em 2 sessões fixas reduziria ~10 trocas de contexto por dia.',
        'Terças e quintas concentram reuniões no Teams. Bloqueie 90 min antes delas para preparar mentorias — a AgroTech pediu material de pitch duas vezes.',
        'Blocos do YouTube foram classificados como Cidades Inteligentes em 1 de 3 casos: crie uma regra “TEDx → Cidades Inteligentes” para evitar revisão manual.',
      ],
      model: S.settings.models.report,
    };
  },

  get_settings: () => settingsView(),
  update_settings: (a) => {
    const incoming = a.settings as Settings;
    const provider = isProvider(incoming.ai_provider) ? incoming.ai_provider : S.settings.ai_provider;
    // EngineState::apply_settings: the managed provider needs a valid monthly_managed license; nothing is saved otherwise.
    if (provider === 'ubi' && !licenseAllowsManaged(licenseStatus())) throw new Error(pick('license_required: a IA do Ubi precisa de uma licença mensal válida', 'license_required: the Ubi AI needs a valid monthly license'));
    S.settings = { ...S.settings, ...incoming, ai_provider: provider };
    // Settings::reconcile_models(): ids of another vendor make no sense for the selected provider.
    S.settings.models = reconcileModels(S.settings.models, providerInfo(provider));
    if (!S.settings.tracking_enabled && S.trackerState === 'running') S.trackerState = 'paused';
    if (S.settings.tracking_enabled && S.trackerState === 'paused') S.trackerState = 'running';
    refreshAiHealth();
    return settingsView();
  },
  set_api_key: async (a): Promise<ApiKeyResult> => {
    await sleep(LATENCY_MS ? 700 : 0);
    const provider = isProvider(a.provider) ? a.provider : S.settings.ai_provider;
    if (provider === 'ubi') throw new Error(pick('invalid: a IA do Ubi usa a chave de licença (set_license_key)', 'invalid: the Ubi AI uses the license key (set_license_key)'));
    const info = providerInfo(provider);
    const key = typeof a.key === 'string' ? a.key.trim() : null;
    if (key === null || key === '') {
      S.keyHints[provider] = null;
      refreshAiHealth();
      return { valid: true, message: pick(`Chave da ${info.label} removida do Keychain.`, `${info.label} key removed from the Keychain.`) };
    }
    if (keyLooksValid(provider, key)) {
      S.keyHints[provider] = key.slice(-4);
      refreshAiHealth();
      const model = provider === S.settings.ai_provider ? S.settings.models.classify : info.default_models.classify;
      return { valid: true, message: pick(`Chave válida — ${info.label} (${model}) respondeu em 412 ms.`, `Valid key: ${info.label} (${model}) answered in 412 ms.`) };
    }
    const hint = provider === 'openai' ? 'sk-… ou sk-proj-…' : `${info.key_prefix}…`;
    return {
      valid: false,
      message: pick(
        `Chave inválida: a API da ${info.label} respondeu 401 (authentication_error). Chaves da ${info.label} começam com ${hint}.`,
        `Invalid key: the ${info.label} API answered 401 (authentication_error). ${info.label} keys start with ${hint}.`,
      ),
    };
  },
  list_models: async (a): Promise<string[]> => {
    await sleep(LATENCY_MS ? 500 : 0);
    const provider = isProvider(a.provider) ? a.provider : S.settings.ai_provider;
    if (!keyConfigured(provider)) throw new Error(`chave não configurada para ${providerInfo(provider).label}`);
    return [...ACCOUNT_MODELS[provider]];
  },
  get_license_status: () => licenseStatus(),
  set_license_key: async (a): Promise<LicenseStatus> => {
    await sleep(LATENCY_MS ? 600 : 0);
    const key = typeof a.key === 'string' ? a.key.trim() : '';
    S.license.key = key || null;
    // Like the engine (`license::set_key`): the managed provider stays selected; its health follows the verdict.
    refreshAiHealth();
    return licenseStatus();
  },
  set_tracking: (a) => {
    const enabled = a.enabled === true;
    S.settings.tracking_enabled = enabled;
    S.trackerState = enabled ? 'running' : 'paused';
    setTimeout(() => emit({ type: 'tracker_state', state: S.trackerState }), 10);
  },
  set_private_mode: (a) => {
    const d = a.duration as string;
    if (d === 'off') {
      S.settings.private_mode = false;
      S.settings.private_until = null;
      S.trackerState = S.settings.tracking_enabled ? 'running' : 'paused';
    } else {
      S.settings.private_mode = true;
      const mins = d === 'minutes30' ? 30 : d === 'hour1' ? 60 : d === 'until_tomorrow' ? 12 * 60 : null;
      S.settings.private_until = mins ? new Date(Date.now() + mins * 60_000).toISOString() : null;
      S.trackerState = 'private';
    }
    setTimeout(() => emit({ type: 'tracker_state', state: S.trackerState }), 10);
  },
  request_permission: async (a) => {
    await sleep(LATENCY_MS ? 600 : 0);
    const kind = a.kind as keyof PermissionStatus;
    S.permissions = { ...S.permissions, [kind]: 'granted' };
  },
  restart_app: () => {
    if (typeof window !== 'undefined' && LATENCY_MS) window.location.reload();
  },
  delete_all_data: () => {
    S.blocks = new Map();
    for (const d of [today()]) S.blocks.set(d, []);
    S.reports = [];
    S.nudges = [];
    S.rules = S.rules.filter((r) => r.origin === 'user');
  },
  export_data: async () => {
    await sleep(LATENCY_MS ? 500 : 0);
    return `/Users/andrey/Downloads/ubiqx-export-${today()}.json`;
  },
  open_external: (a) => {
    if (typeof window !== 'undefined') window.open(str(a.url, 'url'), '_blank', 'noopener');
  },

  get_update_status: () => updateStatus(),
  check_for_updates: async () => {
    S.update.checking = true;
    await sleep(LATENCY_MS ? 900 : 0);
    S.update.checking = false;
    S.update.lastCheck = new Date().toISOString();
    S.update.lastError = null;
    // Every manual check that finds an update re-emits the event, as the engine does.
    if (S.update.available) {
      S.update.announcedEpoch = null;
      setTimeout(announceUpdate, 10);
    }
    return updateStatus();
  },
  dismiss_update: (a) => {
    S.update.dismissedEpoch = num(a.epoch, S.update.available?.build.epoch ?? 0);
    return updateStatus();
  },
  open_update: () => {
    const rel = S.update.available;
    if (!rel) throw new Error('not_found: nenhuma atualização disponível');
    if (typeof window !== 'undefined') window.open(rel.download_url, '_blank', 'noopener');
  },

  list_installed_apps: () => [...INSTALLED_APPS].sort((a, b) => a.name.localeCompare(b.name)),
  list_known_domains: (a) => knownDomains(num(a.limit, 30)),
  list_focus_targets: () => sortTargets(S.focus.targets),
  add_focus_target: (a) => {
    const kind = (a.kind === 'app' ? 'app' : 'site') as FocusTargetKind;
    const name = str(a.name, 'name').trim();
    const rawKey = str(a.key, 'key').trim();
    const key = kind === 'site' ? normalizeDomain(rawKey) : rawKey;
    if (!key || !name) throw new Error('invalid: nome e chave são obrigatórios');
    const existing = S.focus.targets.find((x) => x.kind === kind && x.key === key);
    if (existing) {
      existing.enabled = true;
      return existing;
    }
    const target: FocusTarget = { id: uid('tgt'), kind, name, key, enabled: true, created_at: new Date().toISOString(), last_blocked_at: null, blocked_count: 0 };
    S.focus.targets.push(target);
    return target;
  },
  set_focus_target_enabled: (a) => {
    const target = S.focus.targets.find((x) => x.id === a.id);
    if (!target) throw new Error('not_found: bloqueio não encontrado');
    target.enabled = a.enabled === true;
    return target;
  },
  remove_focus_target: (a) => {
    S.focus.targets = S.focus.targets.filter((x) => x.id !== a.id);
  },
  list_interventions: (a) => [...S.focus.interventions].sort((x, y) => y.at.localeCompare(x.at)).slice(0, num(a.limit, 30)),
  get_focus_status: () => focusStatus(),
  start_focus_session: (a) => {
    const task = str(a.task, 'task').trim();
    const minutes = num(a.minutes, 0);
    if (!task) throw new Error('invalid: a tarefa está vazia');
    if (!Number.isInteger(minutes) || minutes < 5 || minutes > 240) throw new Error('invalid: a duração precisa estar entre 5 e 240 minutos');
    endSession();
    const now = Date.now();
    const session: FocusSession = {
      id: uid('fs'),
      task,
      started_at: new Date(now).toISOString(),
      ends_at: new Date(now + minutes * 60_000).toISOString(),
      ended_at: null,
      interventions: 0,
      hid_windows: S.settings.focus.hide_others_on_start,
      ran_shortcut: S.settings.focus.macos_focus_shortcut_on !== null,
    };
    S.focus.session = session;
    if (LATENCY_MS) S.focus.timer = setTimeout(endSession, minutes * 60_000);
    setTimeout(() => emit({ type: 'focus_session', session: structuredClone(session) }), 10);
    return session;
  },
  stop_focus_session: () => endSession(),
  test_intervention: () => {
    openInterventionPreview('test');
  },
};

export async function handle<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const fn = commands[cmd];
  if (!fn) throw new Error(`mock: comando desconhecido "${cmd}"`);
  if (LATENCY_MS) await sleep(LATENCY_MS + Math.random() * 60);
  const result = await fn(args);
  // return a copy so the UI can't mutate the mock state by accident
  return (result === undefined ? undefined : structuredClone(result)) as T;
}

/* ------------------------------------------------------------------ */
/* test / screenshot hooks                                             */
/* ------------------------------------------------------------------ */

export const __mock = {
  reset(): void {
    S = freshState();
    tick = 0;
  },
  setOnboardingDone(done: boolean): void {
    S.settings.onboarding_done = done;
  },
  /** Pretends to run on another OS: permissions, data dir and the installer of any served update follow it. */
  setPlatform(platform: Platform): void {
    S.platform = platform;
    S.permissions = platformFacts(platform).permissions;
    if (S.update.available) S.update.available = { ...S.update.available, ...platformDownload(platform) };
  },
  /** Serves `release` as the available update (`true` = the sample newer build) or clears it with `null`. Subscribers get one `update_available` event. */
  setUpdate(release: ReleaseInfo | true | null): void {
    S.update.available = release === true ? sampleRelease() : release ? structuredClone(release) : null;
    S.update.dismissedEpoch = null;
    S.update.announcedEpoch = null;
    S.update.lastCheck = new Date().toISOString();
    S.update.lastError = null;
    if (S.update.available && subscribers.size > 0) setTimeout(announceUpdate, LATENCY_MS ? 300 : 10);
  },
  /** The newer build `setUpdate(true)` serves, for assertions. */
  sampleRelease,
  /**
   * Focus guard state for tests and screenshots: `session: 'active'` = the 45-min sample started 12 min ago, `null` ends any
   * session silently; `nudge: true` stores an unseen `focus_prompt` nudge (and pushes it to subscribers); `targets`/`interventions`
   * replace the lists.
   */
  setFocus(patch: { session?: FocusSession | 'active' | null; nudge?: boolean; targets?: FocusTarget[]; interventions?: Intervention[] }): void {
    if (patch.session !== undefined) {
      if (S.focus.timer) clearTimeout(S.focus.timer);
      S.focus.timer = null;
      S.focus.session = patch.session === 'active' ? sampleSession() : patch.session ? structuredClone(patch.session) : null;
    }
    if (patch.targets) S.focus.targets = structuredClone(patch.targets);
    if (patch.interventions) S.focus.interventions = structuredClone(patch.interventions);
    if (patch.nudge) {
      const nudge = focusPromptNudge();
      S.nudges.unshift(nudge);
      if (subscribers.size > 0) setTimeout(() => emit({ type: 'nudge', nudge }), LATENCY_MS ? 300 : 10);
    }
  },
  /** Simulates the guard catching a listed target (the first enabled one by default): records an intervention, pushes the events and, in the browser, opens the panel. */
  intervene(targetId?: Id): Intervention | null {
    const target = targetId ? S.focus.targets.find((x) => x.id === targetId) : sortTargets(S.focus.targets).find((x) => x.enabled);
    if (!target) return null;
    const intervention = intervene(target);
    openInterventionPreview(intervention.id);
    return structuredClone(intervention);
  },
  /** The session `setFocus({ session: 'active' })` serves, for assertions. */
  sampleSession,
  /**
   * License for tests and screenshots: a sample name (`'annual' | 'managed' | 'expired'`), a raw key (`'UBIQX-…'`, anything the
   * mock cannot parse reads as `invalid`) or `null` (no key). `proxyReachable: false` simulates the Ubi proxy being offline
   * (the local verdict stays, `managed_usage` is null).
   */
  setLicense(license: SampleLicense | string | null, opts: { proxyReachable?: boolean } = {}): LicenseStatus {
    S.license.key = license === null ? null : license in SAMPLE_LICENSE_KEYS ? SAMPLE_LICENSE_KEYS[license as SampleLicense] : license;
    if (opts.proxyReachable !== undefined) S.license.proxyReachable = opts.proxyReachable;
    refreshAiHealth();
    return licenseStatus();
  },
  /** The keys `setLicense('annual' | 'managed' | 'expired')` store, for assertions. */
  sampleLicenseKeys: SAMPLE_LICENSE_KEYS,
  state(): State {
    return S;
  },
};

declare global {
  interface Window {
    __ubiqxMock?: {
      reset: () => void;
      setOnboardingDone: (done: boolean) => void;
      setPlatform: (platform: Platform) => void;
      setUpdate: (release: ReleaseInfo | true | null) => void;
      setFocus: (typeof __mock)['setFocus'];
      intervene: (typeof __mock)['intervene'];
      setLicense: (typeof __mock)['setLicense'];
    };
  }
}

if (typeof window !== 'undefined') {
  window.__ubiqxMock = { reset: __mock.reset, setOnboardingDone: __mock.setOnboardingDone, setPlatform: __mock.setPlatform, setUpdate: __mock.setUpdate, setFocus: __mock.setFocus, intervene: __mock.intervene, setLicense: __mock.setLicense };
}
