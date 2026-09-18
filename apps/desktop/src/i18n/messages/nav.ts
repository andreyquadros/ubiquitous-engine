import type { NamespaceMessages } from '../types';

// Left rail, tracker pill and the app shell (gate). t('nav.<key>').
const messages = {
  'pt-BR': {
    main_navigation: 'Navegação principal',
    today: 'Hoje',
    timeline: 'Timeline',
    review: 'Revisão',
    reports: 'Relatórios',
    categories: 'Categorias',
    insights: 'Insights',
    settings: 'Configurações',
    to_review_one: '{count} bloco para revisar',
    to_review_other: '{count} blocos para revisar',
    theme_light: 'Tema claro',
    theme_dark: 'Tema escuro',

    // tracker pill
    pause_tracking: 'Pausar rastreamento',
    resume_tracking: 'Retomar rastreamento',
    tracking_change_failed: 'Não foi possível alterar o rastreamento',

    // gate
    engine_error_title: 'Não foi possível conectar ao motor do ubiqX',
    engine_error_hint: 'Feche e abra o app de novo. Se continuar, veja os logs em Configurações.',
  },
  en: {
    main_navigation: 'Main navigation',
    today: 'Today',
    timeline: 'Timeline',
    review: 'Review',
    reports: 'Reports',
    categories: 'Categories',
    insights: 'Insights',
    settings: 'Settings',
    to_review_one: '{count} block to review',
    to_review_other: '{count} blocks to review',
    theme_light: 'Light theme',
    theme_dark: 'Dark theme',

    // tracker pill
    pause_tracking: 'Pause tracking',
    resume_tracking: 'Resume tracking',
    tracking_change_failed: "Couldn't change tracking",

    // gate
    engine_error_title: "Couldn't connect to the ubiqX engine",
    engine_error_hint: 'Close and reopen the app. If it keeps happening, check the logs in Settings.',
  },
} satisfies NamespaceMessages;

export default messages;
