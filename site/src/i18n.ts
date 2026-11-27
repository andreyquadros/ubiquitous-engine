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
  /** Text link to the secondary installer (.msi, .deb), when the OS has one. */
  alt?: string;
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
  /** Rótulo do botão principal por sistema operacional detectado. */
  downloadFor: { macos: string; windows: string; linux: string };
  hero: {
    eyebrow: string;
    title: string;
    subtitle: string;
    primary: string;
    secondary: string;
    trust: string;
    mascotAlt: string;
    platforms: string;
    /** Falas curtas que giram no balão ao lado do UBI. */
    bubbles: string[];
    /** Botão que liga a animação para quem tem "reduzir movimento" no sistema. */
    enableMotion: string;
    /** Balão clicável ao lado do UBI que abre o trailer. */
    video: { bubble: string; hint: string; title: string; caption: string; close: string; unsupported: string };
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
    windows: { title: string; steps: string[] };
    linux: { title: string; steps: string[]; terminal: string; copy: string; copied: string };
    /** Link to docs/WINDOWS-LINUX.md, under the Windows and Linux panels. */
    otherGuide: string;
  };
  faq: { eyebrow: string; title: string; items: Faq[] };
  footer: { tagline: string; repo: string; releases: string; docs: string; contact: string; rights: string; source: string };
  fallback: { webgl: string };
}

const ptBR: Copy = {
  meta: {
    title: 'ubiqX AI — Controle de tempo automático com IA',
    description:
      'O ubiqX registra sozinho em que apps e sites você trabalha, classifica cada bloco com IA nas suas categorias e entrega o relatório de horas pronto. Sem planilha, sem cronômetro.',
    ogTitle: 'Trabalhe o dia inteiro. O relatório se escreve sozinho.',
    ogDescription: 'Controle de tempo automático com IA: para onde o seu dia foi, por categoria, em relatórios prontos para entregar.',
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
  downloadFor: { macos: 'Baixar para macOS', windows: 'Baixar para Windows', linux: 'Baixar para Linux' },
  hero: {
    eyebrow: 'Os dias não estão menores. O seu tempo é que está vazando.',
    title: 'Descubra para onde vão as suas horas. E retome o controle do dia.',
    subtitle:
      'Você senta às 8h, levanta às 18h e não sabe dizer o que rendeu. Não foi o dia que encurtou: foram as trocas de janela, as abas abertas e o \u201Csó um minuto\u201D que virou quarenta. O ubiqX mede tudo isso sozinho, mostra quanto foi foco e quanto foi distração, e ainda entrega o relatório por categoria pronto. Chega de procrastinar no escuro: o UBI chegou para te ajudar.',
    primary: 'Baixar para macOS',
    secondary: 'Ver os planos',
    trust: 'Tudo fica no seu computador. Sem contas, sem telemetria.',
    mascotAlt: 'UBI, o mascote do ubiqX: um robô branco com visor escuro, crista azul e faixa laranja',
    platforms: 'macOS, Windows e Linux.',
    bubbles: [
      'Oi! Eu sou o UBI. Eu conto as suas horas pra você não ter que contar.',
      'Você trocou de janela 21 vezes nos últimos 30 minutos. Quer fechar o que não é urgente?',
      '5h08 de foco hoje. Ontem foram 3h40 — tá subindo.',
      'Esse bloco eu não soube classificar. Me corrige uma vez e eu nunca mais erro.',
      'Relatório do dia pronto. É só conferir e enviar.',
      '1h50 sem pausa. Levanta 5 minutos, eu seguro o cronômetro.',
    ],
    enableMotion: 'Ativar animação',
    video: {
      bubble: 'Conheça mais no vídeo',
      hint: 'Menos de 1 minuto',
      title: 'ubiqX AI — o trailer',
      caption: 'Para onde vão as suas horas, em menos de um minuto.',
      close: 'Fechar o vídeo',
      unsupported: 'O seu navegador não reproduz este vídeo.',
    },
  },
  how: {
    eyebrow: 'Como funciona',
    title: 'Três passos. Nenhum deles é seu.',
    intro: 'Você instala uma vez, cria as suas categorias e volta a trabalhar. O resto roda em segundo plano, sem pedir sua atenção.',
    steps: [
      {
        title: 'Registra sozinho',
        text: 'A cada poucos segundos ele anota o app, a janela e o site ativos. Sem teclas, sem conteúdo e sem você lembrar de apertar play.',
      },
      {
        title: 'Classifica e aprende',
        text: 'Regras e memória resolvem a maior parte de graça; o resto vai para a IA com o mínimo de dados. Corrigiu um bloco, ele aprende e já sugere a regra.',
      },
      {
        title: 'Entrega o relatório',
        text: 'No horário que você escolher, um relatório por categoria com itens, minutos e evidências. Você confere, copia e entrega.',
      },
    ],
  },
  features: {
    eyebrow: 'Recursos',
    title: 'Menos tempo prestando contas do tempo.',
    intro: 'Feito para quem toca várias frentes ao mesmo tempo e precisa mostrar o que fez, sem que isso vire um segundo trabalho.',
    items: [
      {
        title: 'Zero esforço diário',
        text: 'Nada de apertar play, parar o cronômetro ou lembrar no fim do dia. Ele registra o app, a janela e o site ativos sozinho, o dia inteiro.',
      },
      {
        title: 'As suas categorias, não as dele',
        text: 'Cliente, projeto, disciplina, instituição: você cria as frentes e descreve o que conta como trabalho em cada uma. A IA segue exatamente essa descrição.',
      },
      {
        title: 'O relatório sai pronto',
        text: 'Diário e mensal, por categoria, com atividades, minutos e evidências. A visão mensal junta o que continuou de um dia para o outro e exporta em Markdown.',
      },
      {
        title: 'Cinco minutos de revisão',
        text: 'Só o que a IA não teve certeza entra na fila. Você confirma ou corrige com uma tecla, e cada correção vira regra para o mês seguinte.',
      },
      {
        title: 'Menos troca de contexto',
        text: 'Bloqueio de apps e sites, sessões de foco e um aviso discreto quando você começa a pular de janela em janela. Com teto diário e horário silencioso.',
      },
      {
        title: 'Seus dados não saem da máquina',
        text: 'Banco local, sem contas e sem telemetria. Antes de qualquer consulta à IA, URLs perdem a query e e-mail, telefone, CPF e CNPJ são mascarados.',
      },
      {
        title: 'Funciona sem internet',
        text: 'Sem conexão, ele continua registrando e classifica pelas suas regras e pela memória das correções. Quando a rede volta, a fila segue de onde parou.',
      },
      {
        title: 'macOS, Windows e Linux',
        text: 'O mesmo app e o mesmo feed de atualização nos três sistemas, em português e em inglês.',
      },
    ],
  },
  screens: {
    eyebrow: 'Telas',
    title: 'O app, como ele é.',
    intro: 'Capturas reais da interface com dados de demonstração.',
    shots: [
      { file: 'dashboard', title: 'Hoje', caption: 'O foco do dia em um número, o resumo escrito a partir dos seus próprios dados e o que ainda falta revisar.' },
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
          'macOS, Windows e Linux',
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
    intro: 'O ubiqX roda na barra de menus (ou na bandeja do sistema) e começa a registrar assim que você permite.',
    os: [
      { id: 'macos', name: 'macOS', requires: 'macOS 13 ou mais recente, Apple Silicon', cta: 'Baixar .dmg' },
      { id: 'windows', name: 'Windows', requires: 'Windows 10 e 11, 64 bits', cta: 'Baixar instalador (.exe)', alt: 'ou o pacote .msi' },
      { id: 'linux', name: 'Linux', requires: 'Sessão X11 (Xorg), 64 bits', cta: 'Baixar AppImage', alt: 'ou o pacote .deb' },
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
    windows: {
      title: 'Primeira abertura no Windows',
      steps: [
        'Abra o instalador e siga os passos; o ubiqX fica na bandeja do sistema, ao lado do relógio.',
        'Se o SmartScreen aparecer, clique em “Mais informações” e depois em “Executar assim mesmo”: o instalador ainda não tem assinatura de editor.',
        'Nenhuma permissão a conceder: título da janela e URL do navegador (Chrome, Edge, Brave, Opera, Vivaldi, Firefox) são lidos pela acessibilidade do próprio Windows.',
      ],
    },
    linux: {
      title: 'Primeira abertura no Linux',
      steps: [
        'Torne o AppImage executável e abra, ou instale o .deb; o ubiqX aparece na bandeja do sistema.',
        'Precisa de uma sessão X11 (“GNOME on Xorg”, “Plasma (X11)”): em sessão Wayland o app só vê os programas que rodam pelo XWayland; os nativos Wayland ficam invisíveis.',
        'A URL do navegador vem só do título da janela; sem ela, os sites entram pelo nome do app.',
      ],
      terminal: 'No terminal:',
      copy: 'Copiar comandos',
      copied: 'Copiado',
    },
    otherGuide: 'O que funciona no Windows e no Linux, limitações e como compilar',
  },
  faq: {
    eyebrow: 'Perguntas frequentes',
    title: 'O que as pessoas perguntam antes de instalar.',
    items: [
      {
        q: 'O que é o ubiqX?',
        a: 'O ubiqX é um app de controle de tempo automático para macOS, Windows e Linux. Ele registra sozinho em que apps, janelas e sites você trabalha, usa IA para classificar cada bloco de tempo nas categorias que você criar e gera relatórios diários e mensais por categoria, prontos para entregar.',
      },
      {
        q: 'Preciso apertar play para ele contar o tempo?',
        a: 'Não. Essa é a diferença principal para um cronômetro comum. O ubiqX roda na barra de menus e registra a sua atividade continuamente, sozinho. Você não inicia nem para nada: no fim do dia o tempo já está medido e separado por categoria.',
      },
      {
        q: 'Em que ele é diferente do Rize, do RescueTime ou do Toggl?',
        a: 'Três pontos. Os dados ficam na sua máquina, sem conta e sem servidor nosso. A classificação usa as categorias que você escreve, não uma taxonomia pronta. E a saída é um relatório por categoria pronto para entregar a um cliente ou instituição, não só um gráfico de produtividade.',
      },
      {
        q: 'O ubiqX substitui a minha planilha de horas?',
        a: 'Essa é a ideia. Em vez de lembrar no fim da semana o que você fez, o relatório diário já chega com as atividades, os minutos e as evidências de cada categoria. Você revisa o que ficou em dúvida, corrige com uma tecla e exporta em Markdown.',
      },
      {
        q: 'O que sai do meu computador?',
        a: 'Só o necessário para classificar um bloco: nome do app, título da janela e domínio do site, já com e-mails, telefones, CPF e CNPJ mascarados e sem a URL completa. Em blocos ambíguos, e só se você permitir, um print reduzido da janela ativa. A tela "Dados enviados à IA" mostra exatamente o que saiu. Sem contas, sem telemetria, sem sincronização.',
      },
      {
        q: 'Quais permissões o app pede e por quê?',
        a: 'No macOS: Gravação de Tela, para ler o título da janela ativa e tirar os prints esparsos, e Automação, para perguntar ao navegador qual URL está aberta. Nenhuma das duas lê teclas nem conteúdo, e você pode negar a segunda: fica só o título. No Windows e no Linux não há permissão a conceder.',
      },
      {
        q: 'Funciona no Windows e no Linux?',
        a: 'Sim, com o mesmo app e o mesmo feed de atualização. No Windows, a URL da aba ativa vem da acessibilidade do sistema (Chrome, Edge, Brave, Opera, Vivaldi e Firefox) e o Foco fecha apps, fecha abas e minimiza as outras janelas. No Linux, o app vê a janela ativa em sessões X11; numa sessão Wayland só os programas que rodam pelo XWayland são rastreados (os nativos Wayland ficam para uma versão futura, pelos portais) e a URL é lida só do título do navegador. A chave de API fica no Gerenciador de Credenciais ou no chaveiro do sistema.',
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
    tagline: 'Controle de tempo automático com IA. O seu dia, contado sem você anotar nada.',
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
    title: 'ubiqX AI — Automatic time tracking with AI',
    description:
      'ubiqX records which apps and sites you work in, files every block with AI into your own categories and hands you the timesheet already written. No spreadsheet, no stopwatch.',
    ogTitle: 'Work all day. The report writes itself.',
    ogDescription: 'Automatic time tracking with AI: where your day went, by category, in reports ready to hand in.',
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
  downloadFor: { macos: 'Download for macOS', windows: 'Download for Windows', linux: 'Download for Linux' },
  hero: {
    eyebrow: 'The days are not getting shorter. Your time is leaking.',
    title: 'Find out where your hours go. And take the day back.',
    subtitle:
      'You sit down at 8, get up at 6 and cannot say what you got done. The day did not shrink: the window switches, the open tabs and the \u201Cjust a minute\u201D that became forty did that. ubiqX measures all of it on its own, shows how much was focus and how much was distraction, and hands you the report per category. Stop guessing in the dark: UBI is here to help.',
    primary: 'Download for macOS',
    secondary: 'See the plans',
    trust: 'Everything stays on your computer. No accounts, no telemetry.',
    mascotAlt: 'UBI, the ubiqX mascot: a white robot with a dark visor, a blue crest and an orange sash',
    platforms: 'macOS, Windows and Linux.',
    bubbles: [
      'Hi! I am UBI. I count your hours so you do not have to.',
      'You switched windows 21 times in the last 30 minutes. Shall we close what is not urgent?',
      '5h08 of focus today. Yesterday it was 3h40 — it is going up.',
      'I was not sure about this block. Correct me once and I will not miss it again.',
      'Today\u2019s report is ready. Check it and send it.',
      '1h50 without a break. Stand up for five minutes, I will hold the clock.',
    ],
    enableMotion: 'Turn on animation',
    video: {
      bubble: 'See more in the video',
      hint: 'Under a minute',
      title: 'ubiqX AI — the trailer',
      caption: 'Where your hours go, in under a minute.',
      close: 'Close the video',
      unsupported: 'Your browser cannot play this video.',
    },
  },
  how: {
    eyebrow: 'How it works',
    title: 'Three steps. None of them yours.',
    intro: 'Install once, create your categories and go back to work. The rest runs in the background, without asking for your attention.',
    steps: [
      {
        title: 'It records by itself',
        text: 'Every few seconds it notes the active app, window and site. No keystrokes, no content, and nothing for you to press.',
      },
      {
        title: 'It classifies and learns',
        text: 'Rules and memory settle most of it for free; the rest goes to the AI with the least data possible. Correct a block and it learns, then proposes the rule.',
      },
      {
        title: 'It hands you the report',
        text: 'At the time you pick, a report per category with activities, minutes and evidence. You check it, copy it and hand it in.',
      },
    ],
  },
  features: {
    eyebrow: 'Features',
    title: 'Less time accounting for your time.',
    intro: 'Made for people running several fronts at once who still have to show what they did, without turning that into a second job.',
    items: [
      {
        title: 'Zero daily effort',
        text: 'No start button, no stopwatch to remember, no reconstructing the day at 6pm. It records the active app, window and site on its own, all day.',
      },
      {
        title: 'Your categories, not its own',
        text: 'Client, project, course, institution: you create the fronts and describe what counts as work in each. The AI follows exactly that description.',
      },
      {
        title: 'The report comes out written',
        text: 'Daily and monthly, per category, with activities, minutes and evidence. The monthly view joins what carried over between days and exports Markdown.',
      },
      {
        title: 'Five minutes of review',
        text: 'Only what the AI was unsure about reaches the queue. You confirm or correct with one key, and every correction becomes a rule for next month.',
      },
      {
        title: 'Fewer context switches',
        text: 'App and site blocking, focus sessions and a quiet nudge when you start hopping between windows. With a daily cap and quiet hours.',
      },
      {
        title: 'Your data never leaves the machine',
        text: 'Local database, no account, no telemetry. Before anything reaches the AI, URLs lose their query and e-mails, phones and ID numbers are masked.',
      },
      {
        title: 'Works offline',
        text: 'With no connection it keeps recording and classifies from your rules and the memory of your corrections. When the network is back, the queue resumes.',
      },
      {
        title: 'macOS, Windows and Linux',
        text: 'The same app and the same update feed on all three systems, in Portuguese and English.',
      },
    ],
  },
  screens: {
    eyebrow: 'Screens',
    title: 'The app, as it is.',
    intro: 'Real captures of the interface with demo data.',
    shots: [
      { file: 'dashboard', title: 'Today', caption: 'The focus of the day as a single number, a summary written from your own data, and what is still waiting for review.' },
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
          'macOS, Windows and Linux',
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
    intro: 'ubiqX lives in the menu bar (or the system tray) and starts recording as soon as you allow it.',
    os: [
      { id: 'macos', name: 'macOS', requires: 'macOS 13 or later, Apple Silicon', cta: 'Download .dmg' },
      { id: 'windows', name: 'Windows', requires: 'Windows 10 and 11, 64-bit', cta: 'Download installer (.exe)', alt: 'or the .msi package' },
      { id: 'linux', name: 'Linux', requires: 'X11 (Xorg) session, 64-bit', cta: 'Download AppImage', alt: 'or the .deb package' },
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
    windows: {
      title: 'First launch on Windows',
      steps: [
        'Open the installer and follow the steps; ubiqX sits in the system tray, next to the clock.',
        'If SmartScreen shows up, choose “More info” and then “Run anyway”: the installer has no publisher signature yet.',
        'Nothing to grant: the window title and the browser URL (Chrome, Edge, Brave, Opera, Vivaldi, Firefox) are read through Windows accessibility.',
      ],
    },
    linux: {
      title: 'First launch on Linux',
      steps: [
        'Make the AppImage executable and open it, or install the .deb; ubiqX shows up in the system tray.',
        'It needs an X11 session (“GNOME on Xorg”, “Plasma (X11)”): on a Wayland session the app only sees programs running through XWayland; Wayland-native ones stay invisible.',
        'The browser URL comes from the window title only; without it, sites count under the app name.',
      ],
      terminal: 'In a terminal:',
      copy: 'Copy commands',
      copied: 'Copied',
    },
    otherGuide: 'What works on Windows and Linux, limitations and how to build',
  },
  faq: {
    eyebrow: 'Frequently asked questions',
    title: 'What people ask before installing.',
    items: [
      {
        q: 'What is ubiqX?',
        a: 'ubiqX is an automatic time tracking app for macOS, Windows and Linux. It records which apps, windows and sites you work in, uses AI to file every block of time into the categories you create, and produces daily and monthly reports per category, ready to hand in.',
      },
      {
        q: 'Do I have to press start for it to count time?',
        a: 'No. That is the main difference from a regular stopwatch. ubiqX lives in the menu bar and records your activity continuously, on its own. You never start or stop anything: by the end of the day the time is already measured and split by category.',
      },
      {
        q: 'How is it different from Rize, RescueTime or Toggl?',
        a: 'Three things. Your data stays on your machine, with no account and no server of ours. Classification uses the categories you write, not a fixed taxonomy. And the output is a report per category ready to hand to a client or an institution, not only a productivity chart.',
      },
      {
        q: 'Does it replace my timesheet?',
        a: 'That is the idea. Instead of reconstructing the week from memory, the daily report already arrives with the activities, the minutes and the evidence for each category. You review whatever was uncertain, correct it with one key and export Markdown.',
      },
      {
        q: 'What leaves my computer?',
        a: 'Only what is needed to classify a block: app name, window title and site domain, with e-mails, phones and IDs masked and without the full URL. For ambiguous blocks, and only if you allow it, a downscaled screenshot of the active window. The "Data sent to the AI" screen shows exactly what went out. No accounts, no telemetry, no sync.',
      },
      {
        q: 'Which permissions does it ask for, and why?',
        a: 'On macOS: Screen Recording, to read the active window title and take the sparse screenshots, and Automation, to ask the browser which URL is open. Neither reads keystrokes or content, and you can deny the second one: you keep the title only. On Windows and Linux there is nothing to grant.',
      },
      {
        q: 'Does it work on Windows and Linux?',
        a: 'Yes, same app and same update feed. On Windows, the active tab URL comes from system accessibility (Chrome, Edge, Brave, Opera, Vivaldi and Firefox) and Focus quits apps, closes tabs and minimises the other windows. On Linux, the app sees the active window on X11 sessions; on a Wayland session only programs running through XWayland are tracked (Wayland-native ones are left for a future version, through the portals) and the URL is read from the browser title only. The API key lives in the Credential Manager or the system keyring.',
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
    tagline: 'Automatic time tracking with AI. Your day, accounted for without writing anything down.',
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
