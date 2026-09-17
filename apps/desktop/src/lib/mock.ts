// In-memory mock of the Rust engine, used whenever the page is not running inside Tauri
// (browser `vite dev`, Vitest, Playwright screenshots). It mutates its own state for the
// write commands and emits fake engine events so the UI behaves like the real app.

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
  type ReportItem,
  type Rule,
  type RuleSuggestion,
  type Settings,
  type SettingsView,
  type TrackerState,
  type AiUsageTotals,
} from './types';

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
  [15, 'Google Chrome', 'Caixa de entrada (3) - andrey.quadros@ifro.edu.br - Gmail', 'https://mail.google.com/mail/u/0/#inbox', CAT_INCUB, 0.62, 'llm', { review: true, desc: 'Respostas aos mentores sobre a agenda do Demo Day', sent: true }],
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
  [20, 'Google Chrome', 'TEDx: Cidades inteligentes de verdade — YouTube', 'https://www.youtube.com/watch?v=dQw4w9WgXcQ', CAT_CIDADES, 0.58, 'llm', { review: true, desc: 'Palestra sobre sensoriamento urbano (referência para o projeto)', sent: true }],
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

const seedNudges = (date: IsoDate): Nudge[] => [
  { id: uid('ndg'), at: atLocal(date, 10, 2), kind: 'distracted', title: 'YouTube de novo?', message: 'Você passou 10 min no YouTube. Quer voltar ao plano de ensino de Programação Web II?', seen: true },
  { id: uid('ndg'), at: atLocal(date, 11, 30), kind: 'praise', title: 'Foco de 70 minutos!', message: 'Belo bloco de código na API da Incubadora. Continue assim — mas lembre de beber água.', seen: true },
  { id: uid('ndg'), at: atLocal(date, 16, 25), kind: 'break_suggested', title: 'Hora de uma pausa', message: 'Você está há 1h50 sem pausa. Que tal esticar as pernas por 5 minutos antes de lançar as notas?', seen: false },
];

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
  models: { classify: 'claude-haiku-4-5', vision: 'claude-haiku-4-5', report: 'claude-sonnet-5' },
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
});

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
  apiKeyConfigured: boolean;
  apiKeyHint: string | null;
  permissions: PermissionStatus;
  aiHealth: AiHealth;
  trackerState: TrackerState;
  usage: AiUsageTotals;
  advices: number;
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
    nudges: seedNudges(t),
    settings,
    apiKeyConfigured: true,
    apiKeyHint: '…k3Qa',
    permissions: { screen_recording: 'granted', automation: 'granted', accessibility: 'unknown' },
    aiHealth: { state: 'ok' },
    trackerState: 'running',
    usage: { calls: 412, input_tokens: 1_234_567, output_tokens: 98_765, cost_usd: 1.37 },
    advices: 0,
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
  const hours = (total / 3600).toLocaleString('pt-BR', { maximumFractionDigits: 1 });
  const summary_md =
    `## Resumo do dia — ${cat?.name ?? categoryId}\n\n` +
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
    `_Gerado pelo ubiqX com ${S.settings.models.report}._\n`
  );
}

function settingsView(): SettingsView {
  return {
    settings: structuredClone(S.settings),
    api_key_configured: S.apiKeyConfigured,
    api_key_hint: S.apiKeyConfigured ? S.apiKeyHint : null,
    permissions: { ...S.permissions },
    ai_health: S.aiHealth,
    tracker_state: S.trackerState,
    data_dir: '/Users/andrey/Library/Application Support/ai.ubiqx.app',
    platform: 'macos',
    version: '0.1.0',
  };
}

function refreshAiHealth(): void {
  if (!S.apiKeyConfigured) S.aiHealth = { state: 'not_configured' };
  else if (S.settings.local_only) S.aiHealth = { state: 'paused', reason: 'Modo somente local ativado' };
  else if (S.usage.cost_usd >= S.settings.ai_monthly_budget_usd) S.aiHealth = { state: 'paused', reason: 'Orçamento mensal atingido' };
  else S.aiHealth = { state: 'ok' };
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
      title: tick % 4 === 0 ? 'Bom ritmo!' : 'Bloco novo aguardando revisão',
      message:
        tick % 4 === 0
          ? 'Você está mantendo um bom ritmo nesta última hora. Que tal fechar essa tarefa antes de trocar de contexto?'
          : 'Classifiquei um bloco com baixa confiança. Dê uma olhada na Revisão quando puder.',
      seen: false,
    };
    S.nudges.unshift(nudge);
    emit({ type: 'nudge', nudge });
  }
}

export function subscribe(handler: Handler): () => void {
  subscribers.add(handler);
  if (!ticker && import.meta.env.MODE !== 'test') ticker = setInterval(fakeTick, 20_000);
  return () => {
    subscribers.delete(handler);
    if (subscribers.size === 0 && ticker) {
      clearInterval(ticker);
      ticker = null;
    }
  };
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
      title: note ?? 'Atividade manual',
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
    const report: ClassifyReport = { local: 0, remote: 0, vision: 0, needs_review: 0, skipped_remote: S.settings.local_only || !S.apiKeyConfigured };
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
    S.settings = { ...S.settings, ...(a.settings as Settings) };
    if (!S.settings.tracking_enabled && S.trackerState === 'running') S.trackerState = 'paused';
    if (S.settings.tracking_enabled && S.trackerState === 'paused') S.trackerState = 'running';
    refreshAiHealth();
    return settingsView();
  },
  set_api_key: async (a): Promise<ApiKeyResult> => {
    await sleep(LATENCY_MS ? 700 : 0);
    const key = a.key as string | null;
    if (key === null || key === '') {
      S.apiKeyConfigured = false;
      S.apiKeyHint = null;
      refreshAiHealth();
      return { valid: true, message: 'Chave removida do Keychain.' };
    }
    if (key.startsWith('sk-ant-') && key.length >= 20) {
      S.apiKeyConfigured = true;
      S.apiKeyHint = `…${key.slice(-4)}`;
      refreshAiHealth();
      return { valid: true, message: `Chave válida — ${S.settings.models.classify} respondeu em 412 ms.` };
    }
    return { valid: false, message: 'Chave inválida: a API da Anthropic respondeu 401 (authentication_error).' };
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
  state(): State {
    return S;
  },
};

declare global {
  interface Window {
    __ubiqxMock?: { reset: () => void; setOnboardingDone: (done: boolean) => void };
  }
}

if (typeof window !== 'undefined') {
  window.__ubiqxMock = { reset: __mock.reset, setOnboardingDone: __mock.setOnboardingDone };
}
