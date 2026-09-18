// Every visible string of the site, pt-BR first, then en. Components read it through useCopy().
import { createContext, useContext } from 'react';

export type Locale = 'pt-BR' | 'en';
export const LOCALES: readonly Locale[] = ['pt-BR', 'en'];
export const STORAGE_KEY = 'ubiqx-site.lang';

export interface Feature {
  title: string;
  text: string;
}
export interface Step {
  title: string;
  text: string;
}
export interface Shot {
  file: string;
  title: string;
  caption: string;
}
export interface Plan {
  badge?: string;
  name: string;
  price: string;
  period: string;
  alt?: string;
  lead: string;
  bullets: string[];
  highlight: string;
  cta: string;
}
export interface OsCard {
  id: 'macos' | 'windows' | 'linux';
  name: string;
  requires: string;
  cta: string;
}
export interface Faq {
  q: string;
  a: string;
}

export interface Copy {
  meta: { title: string; description: string; ogTitle: string; ogDescription: string };
  nav: {
    brand: string;
    how: string;
    features: string;
    screens: string;
    pricing: string;
    download: string;
    faq: string;
    cta: string;
    menu: string;
    close: string;
    language: string;
    skip: string;
    main: string;
    footerLinks: string;
  };
  lang: { 'pt-BR': string; en: string; switchTo: string };
  hero: {
    eyebrow: string;
    title: string;
    subtitle: string;
    primary: string;
    secondary: string;
    trust: string;
    mascotAlt: string;
    platforms: string;
  };
  how: { eyebrow: string; title: string; intro: string; steps: Step[] };
  features: { eyebrow: string; title: string; intro: string; items: Feature[] };
  screens: { eyebrow: string; title: string; intro: string; shots: Shot[]; open: string; close: string; prev: string; next: string; theme: string };
  pricing: {
    eyebrow: string;
    title: string;
    intro: string;
    plans: [Plan, Plan];
    soon: string;
    soonHint: string;
    note: string;
    footnote: string;
  };
  downloads: {
    eyebrow: string;
    title: string;
    intro: string;
    os: OsCard[];
    soon: string;
    releases: string;
    macos: { title: string; steps: string[]; damaged: string; copy: string; copied: string; guide: string };
  };
  faq: { eyebrow: string; title: string; items: Faq[] };
  footer: { tagline: string; repo: string; releases: string; docs: string; contact: string; rights: string; source: string };
  fallback: { webgl: string };
}

const ptBR: Copy = {
  meta: {
    title: 'ubiqX AI',
    description:
      'Rastreamento automático de tempo para o seu Mac, com IA que classifica cada bloco nas suas categorias e um mascote que cuida do seu foco. Tudo fica na sua máquina.',
    ogTitle: 'ubiqX AI. Seu tempo, entendido.',
    ogDescription: 'Um mascote que cuida do seu foco. Rastreamento automático, categorias suas, relatórios prontos.',
  },
  nav: {
    brand: 'ubiqX AI',
    how: 'Como funciona',
    features: 'Recursos',
    screens: 'Telas',
    pricing: 'Planos',
    download: 'Baixar',
    faq: 'Perguntas',
    cta: 'Baixar para macOS',
    menu: 'Abrir menu',
    close: 'Fechar menu',
    language: 'Idioma',
    skip: 'Ir para o conteúdo',
    main: 'Principal',
    footerLinks: 'Links do rodapé',
  },
  lang: { 'pt-BR': 'PT', en: 'EN', switchTo: 'Mudar idioma para {lang}' },
  hero: {
    eyebrow: 'Para quem trabalha em mais de uma frente',
    title: 'Seu tempo, entendido. Um mascote que cuida do seu foco.',
    subtitle:
      'O ubiqX registra sozinho em qual app, janela e site você está, classifica cada bloco com IA nas categorias que você cria (IFRO, Incubadora, Cidades Inteligentes) e entrega relatórios diários e mensais prontos. O UBI, o mascote, avisa quando você se distrai e bloqueia o que rouba atenção.',
    primary: 'Baixar para macOS',
    secondary: 'Ver os planos',
    trust: 'Tudo fica no seu Mac. Sem contas, sem telemetria.',
    mascotAlt: 'UBI, o mascote do ubiqX: um robô branco com visor escuro, crista azul e faixa laranja',
    platforms: 'macOS agora. Windows e Linux em breve.',
  },
  how: {
    eyebrow: 'Como funciona',
    title: 'Três passos, nenhum deles seu.',
    intro: 'Você instala uma vez, cria suas categorias e segue trabalhando. O resto acontece em segundo plano.',
    steps: [
      {
        title: 'Registra',
        text: 'A cada poucos segundos o ubiqX anota o app, a janela e o site ativos, sem teclas nem conteúdo. Blocos de contexto nascem sozinhos.',
      },
      {
        title: 'Classifica com IA e aprende com você',
        text: 'Regras e memória resolvem a maioria dos blocos de graça; o resto vai para a IA com o mínimo de dados. Corrigiu um bloco? Ele aprende e sugere a regra.',
      },
      {
        title: 'Relata e protege o foco',
        text: 'No horário que você escolher, um relatório diário por categoria, pronto para virar o mensal. Enquanto isso o UBI mede seu foco e intervém quando precisa.',
      },
    ],
  },
  features: {
    eyebrow: 'Recursos',
    title: 'Um painel de instrumentos para a sua atenção.',
    intro: 'Feito para quem precisa prestar contas do tempo em várias frentes e não quer preencher planilha nenhuma.',
    items: [
      {
        title: 'Rastreamento automático',
        text: 'App, janela e o site do navegador, registrados sozinhos. Prints esparsos só da janela ativa, e só quando você permite.',
      },
      {
        title: 'Categorias suas',
        text: 'IFRO, Incubadora, Cidades Inteligentes, o que você quiser. Cada uma com uma descrição do que conta como trabalho; a IA usa exatamente esse texto.',
      },
      {
        title: 'Relatórios diário e mensal',
        text: 'Itens no passado com tipo, minutos e evidências. A visão mensal agrupa continuações e exporta em Markdown.',
      },
      {
        title: 'Revisão com teclado e capturas',
        text: 'Os blocos incertos esperam numa fila. Você confirma ou corrige com uma tecla, olhando a captura quando houver. Cada correção ensina.',
      },
      {
        title: 'Foco',
        text: 'Bloqueio de apps e sites, sessões de foco e intervenções do UBI quando as trocas de contexto disparam. Com teto diário e horário silencioso.',
      },
      {
        title: 'Privacidade',
        text: 'Redação antes de enviar à IA: URLs sem query, e-mails, telefones e documentos mascarados. Apps bloqueados nunca são registrados. Modo privado com prazo.',
      },
      {
        title: 'UBI em 3D, com humores',
        text: 'O mascote reflete o seu foco: calmo, concentrado, empolgado ou preocupado. Ele fala pouco e no momento certo.',
      },
      {
        title: 'Português e inglês',
        text: 'Interface, relatórios e o próprio UBI nos dois idiomas. Troque quando quiser nas configurações.',
      },
    ],
  },
  screens: {
    eyebrow: 'Telas',
    title: 'O app, como ele é.',
    intro: 'Capturas reais da interface com dados de demonstração.',
    shots: [
      { file: 'dashboard', title: 'Hoje', caption: 'O dial de foco, a frase do dia escrita a partir dos dados e o UBI no canto.' },
      { file: 'timeline', title: 'Timeline', caption: 'Cada bloco de contexto do dia, com categoria, app e duração.' },
      { file: 'review', title: 'Revisão', caption: 'A fila do que a IA não teve certeza. Uma tecla confirma, outra corrige.' },
      { file: 'reports', title: 'Relatórios', caption: 'Diário por categoria, com itens, minutos e evidências. Vira o mensal.' },
      { file: 'categories', title: 'Categorias e regras', caption: 'Suas frentes de trabalho e as regras que o app sugeriu a partir das suas correções.' },
      { file: 'insights', title: 'Insights', caption: 'Tendência de foco, distrações e o que mudou na semana.' },
      { file: 'dashboard-light', title: 'Tema claro', caption: 'A mesma tela de dia: o instrumento, não uma inversão.' },
    ],
    open: 'Ampliar a captura de {title}',
    close: 'Fechar',
    prev: 'Captura anterior',
    next: 'Próxima captura',
    theme: 'tema claro',
  },
  pricing: {
    eyebrow: 'Planos',
    title: 'Dois jeitos de pagar. O mesmo app.',
    intro: 'Escolha entre trazer a sua própria chave de IA ou deixar o Ubi cuidar de tudo.',
    plans: [
      {
        name: 'Anual, com a sua IA',
        price: 'R$ 197',
        period: '/ano',
        alt: 'ou 10x de R$ 25',
        lead: 'Você traz a sua chave de API (Anthropic, OpenAI ou xAI). Para quem já tem conta num provedor.',
        bullets: [
          'Tudo incluso: rastreamento, revisão, relatórios, foco e UBI',
          'Atualizações por um ano',
          'macOS agora, Windows e Linux em breve',
          'Você paga a IA direto ao provedor (centavos por dia)',
        ],
        highlight: 'Você paga a IA direto ao provedor (centavos por dia)',
        cta: 'Assinar o plano anual',
      },
      {
        badge: 'Mais simples',
        name: 'Mensal, com a IA do Ubi',
        price: 'R$ 49',
        period: '/mês',
        lead: 'Sem chave de API: o Ubi gerencia a IA por você. Recomendado para quem não quer mexer com contas de provedor.',
        bullets: [
          'Chave, modelos, orçamento e upgrades por nossa conta',
          'Tudo incluso: rastreamento, revisão, relatórios, foco e UBI',
          'Cancele quando quiser',
          'Inclui a IA, sem surpresas',
        ],
        highlight: 'Inclui a IA, sem surpresas',
        cta: 'Assinar o plano mensal',
      },
    ],
    soon: 'Em breve',
    soonHint: 'Pagamento em configuração',
    note: 'Nos dois planos o app é o mesmo e os seus dados ficam na sua máquina.',
    footnote: 'Preços em reais.',
  },
  downloads: {
    eyebrow: 'Baixar',
    title: 'Instale no seu computador.',
    intro: 'O ubiqX roda na barra de menus e começa a registrar assim que você permite.',
    os: [
      { id: 'macos', name: 'macOS', requires: 'macOS 13 ou mais recente, Apple Silicon', cta: 'Baixar .dmg' },
      { id: 'windows', name: 'Windows', requires: 'Windows 10 e 11', cta: 'Baixar instalador' },
      { id: 'linux', name: 'Linux', requires: 'AppImage ou .deb', cta: 'Baixar' },
    ],
    soon: 'Em breve',
    releases: 'Todas as versões no GitHub',
    macos: {
      title: 'Primeira abertura no macOS',
      steps: [
        'Abra o .dmg e arraste o ubiqX para a pasta Aplicativos.',
        'Conceda Gravação de Tela quando o app pedir e reinicie o app: o macOS só aplica a permissão num processo novo.',
        'Abra o navegador que você usa antes de pedir a permissão de Automação, para o app ler a URL ativa.',
      ],
      damaged:
        'Se o macOS disser que o app está danificado ou vem de um desenvolvedor não identificado, é a quarentena do Gatekeeper. Rode estes dois comandos no Terminal e abra de novo:',
      copy: 'Copiar comandos',
      copied: 'Copiado',
      guide: 'Guia completo de instalação e permissões',
    },
  },
  faq: {
    eyebrow: 'Perguntas frequentes',
    title: 'O que as pessoas perguntam antes de instalar.',
    items: [
      {
        q: 'O que sai do meu Mac?',
        a: 'Só o necessário para classificar um bloco: nome do app, título da janela e domínio do site, já com e-mails, telefones, CPF e CNPJ mascarados e sem a URL completa. Em blocos ambíguos, e só se você permitir, um print reduzido da janela ativa. A tela "Dados enviados à IA" mostra exatamente o que saiu. Sem contas, sem telemetria, sem sincronização.',
      },
      {
        q: 'Quais permissões o app pede e por quê?',
        a: 'Gravação de Tela, para ler o título da janela ativa e tirar os prints esparsos. Automação, para perguntar ao navegador qual URL está aberta. Nenhuma das duas lê teclas nem conteúdo, e você pode negar a segunda: fica só o título.',
      },
      {
        q: 'Chave própria ou IA do Ubi: qual escolho?',
        a: 'Se você já tem conta na Anthropic, OpenAI ou xAI e sabe criar uma chave com limite de gasto, o plano anual sai mais barato: a IA custa centavos por dia e você paga direto ao provedor. Se não quer mexer com isso, o plano mensal inclui a IA e o Ubi cuida de chave, modelos, orçamento e upgrades.',
      },
      {
        q: 'Posso cancelar?',
        a: 'Sim. O plano mensal pode ser cancelado a qualquer momento e vale até o fim do mês pago. O anual vale por um ano com todas as atualizações do período.',
      },
      {
        q: 'Consigo exportar os meus dados?',
        a: 'Sim. Configurações, aba Sobre: "Exportar meus dados" gera um arquivo com blocos, categorias e relatórios. Os relatórios também saem em Markdown. O banco é um SQLite na sua pasta de suporte de aplicativos e pode ser apagado quando quiser.',
      },
      {
        q: 'Funciona sem internet ou sem chave?',
        a: 'Funciona. Sem IA, o ubiqX continua registrando e classifica com as suas regras e a memória das correções; os relatórios saem de um modelo por template. Quando a conexão volta, a fila segue de onde parou.',
      },
    ],
  },
  footer: {
    tagline: 'Rastreamento inteligente de atividade, com IA e um mascote que cuida do seu foco.',
    repo: 'Código no GitHub',
    releases: 'Versões',
    docs: 'Documentação',
    contact: 'Contato',
    rights: 'ubiqX AI. Código aberto sob licença MIT.',
    source: 'Este site também é código aberto.',
  },
  fallback: { webgl: 'Renderização estática do UBI (WebGL indisponível).' },
};

const en: Copy = {
  meta: {
    title: 'ubiqX AI',
    description:
      'Automatic time tracking for your Mac, with AI that files every block into your own categories and a mascot that guards your focus. Everything stays on your machine.',
    ogTitle: 'ubiqX AI. Your time, understood.',
    ogDescription: 'A mascot that guards your focus. Automatic tracking, your own categories, reports ready to send.',
  },
  nav: {
    brand: 'ubiqX AI',
    how: 'How it works',
    features: 'Features',
    screens: 'Screens',
    pricing: 'Plans',
    download: 'Download',
    faq: 'FAQ',
    cta: 'Download for macOS',
    menu: 'Open menu',
    close: 'Close menu',
    language: 'Language',
    skip: 'Skip to content',
    main: 'Main',
    footerLinks: 'Footer links',
  },
  lang: { 'pt-BR': 'PT', en: 'EN', switchTo: 'Switch language to {lang}' },
  hero: {
    eyebrow: 'For people who work on more than one front',
    title: 'Your time, understood. A mascot that guards your focus.',
    subtitle:
      'ubiqX quietly records which app, window and site you are in, files every block with AI into the categories you create (IFRO, Incubadora, Cidades Inteligentes) and delivers daily and monthly reports ready to send. UBI, the mascot, nudges you when you drift and blocks what steals your attention.',
    primary: 'Download for macOS',
    secondary: 'See the plans',
    trust: 'Everything stays on your Mac. No accounts, no telemetry.',
    mascotAlt: 'UBI, the ubiqX mascot: a white robot with a dark visor, a blue crest and an orange sash',
    platforms: 'macOS today. Windows and Linux soon.',
  },
  how: {
    eyebrow: 'How it works',
    title: 'Three steps, none of them yours.',
    intro: 'Install once, create your categories and keep working. The rest happens in the background.',
    steps: [
      {
        title: 'Records',
        text: 'Every few seconds ubiqX notes the active app, window and site, with no keystrokes and no content. Context blocks build themselves.',
      },
      {
        title: 'Classifies with AI and learns from you',
        text: 'Rules and memory settle most blocks for free; the rest goes to the AI with the least data possible. Corrected a block? It learns and proposes the rule.',
      },
      {
        title: 'Reports and protects your focus',
        text: 'At the time you pick, a daily report per category, ready to become the monthly one. Meanwhile UBI measures your focus and steps in when needed.',
      },
    ],
  },
  features: {
    eyebrow: 'Features',
    title: 'An instrument panel for your attention.',
    intro: 'Made for people who have to account for their time on several fronts and refuse to fill in a spreadsheet.',
    items: [
      {
        title: 'Automatic tracking',
        text: 'App, window and the browser site, recorded on their own. Sparse screenshots of the active window only, and only when you allow them.',
      },
      {
        title: 'Your own categories',
        text: 'IFRO, Incubadora, Cidades Inteligentes, whatever you need. Each one carries a description of what counts as its work; the AI uses exactly that text.',
      },
      {
        title: 'Daily and monthly reports',
        text: 'Items in the past tense with type, minutes and evidence. The monthly view groups continuations and exports Markdown.',
      },
      {
        title: 'Keyboard review with captures',
        text: 'Uncertain blocks wait in a queue. Confirm or correct with one key, looking at the capture when there is one. Every correction teaches.',
      },
      {
        title: 'Focus',
        text: 'App and site blocking, focus sessions and UBI interventions when context switching spikes. With a daily cap and quiet hours.',
      },
      {
        title: 'Privacy',
        text: 'Redaction before anything reaches the AI: URLs without queries, e-mails, phones and IDs masked. Blocked apps are never recorded. Private mode with a timer.',
      },
      {
        title: 'UBI in 3D, with moods',
        text: 'The mascot mirrors your focus: calm, focused, excited or worried. He speaks little and at the right moment.',
      },
      {
        title: 'Portuguese and English',
        text: 'Interface, reports and UBI himself in both languages. Switch whenever you like in the settings.',
      },
    ],
  },
  screens: {
    eyebrow: 'Screens',
    title: 'The app, as it is.',
    intro: 'Real captures of the interface with demo data.',
    shots: [
      { file: 'dashboard', title: 'Today', caption: 'The focus dial, a sentence written from the data and UBI in the corner.' },
      { file: 'timeline', title: 'Timeline', caption: 'Every context block of the day, with category, app and duration.' },
      { file: 'review', title: 'Review', caption: 'The queue of what the AI was unsure about. One key confirms, another corrects.' },
      { file: 'reports', title: 'Reports', caption: 'Daily per category, with items, minutes and evidence. It becomes the monthly.' },
      { file: 'categories', title: 'Categories and rules', caption: 'Your fronts of work and the rules the app proposed from your corrections.' },
      { file: 'insights', title: 'Insights', caption: 'Focus trend, distractions and what changed this week.' },
      { file: 'dashboard-light', title: 'Light theme', caption: 'The same screen by day: the instrument, not an inversion.' },
    ],
    open: 'Enlarge the {title} capture',
    close: 'Close',
    prev: 'Previous capture',
    next: 'Next capture',
    theme: 'light theme',
  },
  pricing: {
    eyebrow: 'Plans',
    title: 'Two ways to pay. The same app.',
    intro: 'Bring your own AI key or let Ubi handle everything.',
    plans: [
      {
        name: 'Annual, with your AI',
        price: 'R$ 197',
        period: '/year',
        alt: 'or 10x R$ 25',
        lead: 'You bring your own API key (Anthropic, OpenAI or xAI). For people who already have a provider account.',
        bullets: [
          'Everything included: tracking, review, reports, focus and UBI',
          'Updates for a year',
          'macOS now, Windows and Linux soon',
          'You pay the AI directly to the provider (cents a day)',
        ],
        highlight: 'You pay the AI directly to the provider (cents a day)',
        cta: 'Get the annual plan',
      },
      {
        badge: 'Simplest',
        name: 'Monthly, with Ubi AI',
        price: 'R$ 49',
        period: '/month',
        lead: 'No API key: Ubi manages the AI for you. Recommended if you would rather not deal with provider accounts.',
        bullets: [
          'Key, models, budget and upgrades on us',
          'Everything included: tracking, review, reports, focus and UBI',
          'Cancel anytime',
          'AI included, no surprises',
        ],
        highlight: 'AI included, no surprises',
        cta: 'Get the monthly plan',
      },
    ],
    soon: 'Coming soon',
    soonHint: 'Checkout being set up',
    note: 'Both plans ship the same app and your data stays on your machine.',
    footnote: 'Prices in Brazilian reais.',
  },
  downloads: {
    eyebrow: 'Download',
    title: 'Install it on your computer.',
    intro: 'ubiqX lives in the menu bar and starts recording as soon as you allow it.',
    os: [
      { id: 'macos', name: 'macOS', requires: 'macOS 13 or later, Apple Silicon', cta: 'Download .dmg' },
      { id: 'windows', name: 'Windows', requires: 'Windows 10 and 11', cta: 'Download installer' },
      { id: 'linux', name: 'Linux', requires: 'AppImage or .deb', cta: 'Download' },
    ],
    soon: 'Coming soon',
    releases: 'All releases on GitHub',
    macos: {
      title: 'First launch on macOS',
      steps: [
        'Open the .dmg and drag ubiqX to the Applications folder.',
        'Grant Screen Recording when asked and restart the app: macOS only applies the permission to a new process.',
        'Open the browser you use before requesting the Automation permission, so the app can read the active URL.',
      ],
      damaged:
        'If macOS says the app is damaged or comes from an unidentified developer, that is Gatekeeper quarantine. Run these two commands in Terminal and open it again:',
      copy: 'Copy commands',
      copied: 'Copied',
      guide: 'Full installation and permissions guide',
    },
  },
  faq: {
    eyebrow: 'Frequently asked questions',
    title: 'What people ask before installing.',
    items: [
      {
        q: 'What leaves my Mac?',
        a: 'Only what is needed to classify a block: app name, window title and site domain, with e-mails, phones and IDs masked and without the full URL. For ambiguous blocks, and only if you allow it, a downscaled screenshot of the active window. The "Data sent to the AI" screen shows exactly what went out. No accounts, no telemetry, no sync.',
      },
      {
        q: 'Which permissions does it ask for, and why?',
        a: 'Screen Recording, to read the active window title and take the sparse screenshots. Automation, to ask the browser which URL is open. Neither reads keystrokes or content, and you can deny the second one: you keep the title only.',
      },
      {
        q: 'Own key or Ubi AI: which one should I pick?',
        a: 'If you already have an Anthropic, OpenAI or xAI account and know how to create a key with a spending limit, the annual plan is cheaper: the AI costs cents a day and you pay the provider directly. If you would rather not deal with that, the monthly plan includes the AI and Ubi handles the key, models, budget and upgrades.',
      },
      {
        q: 'Can I cancel?',
        a: 'Yes. The monthly plan can be cancelled anytime and runs until the end of the paid month. The annual one is valid for a year with every update released in that period.',
      },
      {
        q: 'Can I export my data?',
        a: 'Yes. Settings, About tab: "Export my data" produces a file with blocks, categories and reports. Reports also export as Markdown. The database is a SQLite file in your application support folder and can be deleted whenever you want.',
      },
      {
        q: 'Does it work offline or without a key?',
        a: 'It does. Without AI, ubiqX keeps recording and classifies with your rules and the memory of your corrections; reports come from a template. When the connection is back, the queue resumes where it stopped.',
      },
    ],
  },
  footer: {
    tagline: 'Smart activity tracking, with AI and a mascot that guards your focus.',
    repo: 'Code on GitHub',
    releases: 'Releases',
    docs: 'Documentation',
    contact: 'Contact',
    rights: 'ubiqX AI. Open source under the MIT licence.',
    source: 'This site is open source too.',
  },
  fallback: { webgl: 'Static render of UBI (WebGL unavailable).' },
};

export const COPY: Record<Locale, Copy> = { 'pt-BR': ptBR, en };

export const isLocale = (v: unknown): v is Locale => v === 'pt-BR' || v === 'en';

/** Maps any BCP-47-ish tag to a supported locale; unknown values fall back to pt-BR. */
export const normaliseLocale = (v: string | null | undefined): Locale => {
  const s = (v ?? '').trim().toLowerCase().replace('_', '-');
  return s.startsWith('en') ? 'en' : 'pt-BR';
};

/** URL ?lang= > localStorage > navigator.language > pt-BR. */
export function initialLocale(): Locale {
  try {
    const q = new URLSearchParams(window.location.search).get('lang');
    if (q) return normaliseLocale(q);
  } catch {
    /* ignore */
  }
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLocale(stored)) return stored;
  } catch {
    /* ignore */
  }
  try {
    if (navigator.language) return normaliseLocale(navigator.language);
  } catch {
    /* ignore */
  }
  return 'pt-BR';
}

/** Persists the choice and reflects it in <html lang>, the title, the description and ?lang=. */
export function persistLocale(l: Locale): void {
  try {
    localStorage.setItem(STORAGE_KEY, l);
  } catch {
    /* ignore */
  }
  const copy = COPY[l];
  document.documentElement.lang = l;
  document.title = copy.meta.title;
  const desc = document.querySelector('meta[name="description"]');
  if (desc) desc.setAttribute('content', copy.meta.description);
  const og = document.querySelector('meta[property="og:description"]');
  if (og) og.setAttribute('content', copy.meta.ogDescription);
  try {
    const url = new URL(window.location.href);
    if (url.searchParams.has('lang') && url.searchParams.get('lang') !== l) {
      url.searchParams.set('lang', l);
      window.history.replaceState(null, '', url);
    }
  } catch {
    /* ignore */
  }
}

/** Fills {name} placeholders. */
export const fill = (text: string, vars: Record<string, string | number>): string =>
  text.replace(/\{(\w+)\}/g, (m, k: string) => (k in vars ? String(vars[k]) : m));

export interface I18n {
  locale: Locale;
  copy: Copy;
  setLocale: (l: Locale) => void;
}

export const I18nContext = createContext<I18n>({ locale: 'pt-BR', copy: ptBR, setLocale: () => {} });
export const useCopy = (): I18n => useContext(I18nContext);
