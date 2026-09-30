import type { NamespaceMessages } from '../types';

// 'dashboard' namespace (the Hoje page): the hero headline, hero stats, card titles and empty states. t('dashboard.<key>').
const messages = {
  'pt-BR': {
    title_day: 'Dia',
    hero_label: 'Resumo do dia',

    // The hero headline: one sentence written from the numbers, '{first}; {second}.'
    'headline.empty_today': 'Nada registrado ainda. Quando você começar, eu registro.',
    'headline.empty_day': 'Nada foi registrado neste dia.',
    'headline.focus_since_lunch': '{duration} de foco desde o almoço',
    'headline.focus_morning': '{duration} de foco nesta manhã',
    'headline.focus_today': '{duration} de foco hoje',
    'headline.focus_day': '{duration} de foco neste dia',
    'headline.awaiting_review_one': '{count} bloco espera sua revisão',
    'headline.awaiting_review_other': '{count} blocos esperam sua revisão',
    'headline.distraction': '{duration} de distração',
    'headline.longest_streak': 'maior sequência de {duration}',
    'headline.all_sorted': 'tudo classificado',
    'headline.sentence': '{first}; {second}.',

    review_cta_one: 'Precisa de revisão: {count} bloco',
    review_cta_other: 'Precisa de revisão: {count} blocos',
    all_sorted: 'Tudo classificado',
    uncategorized_time: '{duration} sem categoria',

    'stat.productive': 'Tempo produtivo',
    'stat.distractions': 'Distrações',
    'stat.longest_focus': 'Maior foco contínuo',
    'stat.switches_per_hour': 'Trocas por hora',

    'timeline.title': 'Linha do tempo',
    'timeline.subtitle': 'Clique em um bloco para abrir na Timeline',
    'categories.title': 'Tempo por categoria',
    'categories.subtitle': 'Horas registradas no dia',
    'hourly.title': 'Foco por hora',
    'hourly.subtitle': 'Parte do tempo em categorias produtivas',
    'top_apps.title': 'Apps mais usados',
    'top_apps.empty': 'Nenhum app registrado ainda.',
  },
  en: {
    title_day: 'Day',
    hero_label: 'Day summary',

    'headline.empty_today': "Nothing recorded yet. Once you start, I'll keep track.",
    'headline.empty_day': 'Nothing was recorded that day.',
    'headline.focus_since_lunch': '{duration} of focus since lunch',
    'headline.focus_morning': '{duration} of focus this morning',
    'headline.focus_today': '{duration} of focus today',
    'headline.focus_day': '{duration} of focus that day',
    'headline.awaiting_review_one': '{count} block is waiting for your review',
    'headline.awaiting_review_other': '{count} blocks are waiting for your review',
    'headline.distraction': '{duration} of distractions',
    'headline.longest_streak': 'longest streak of {duration}',
    'headline.all_sorted': 'all caught up',
    'headline.sentence': '{first}; {second}.',

    review_cta_one: 'Needs review: {count} block',
    review_cta_other: 'Needs review: {count} blocks',
    all_sorted: 'All caught up',
    uncategorized_time: '{duration} uncategorized',

    'stat.productive': 'Productive time',
    'stat.distractions': 'Distractions',
    'stat.longest_focus': 'Longest focus streak',
    'stat.switches_per_hour': 'Switches per hour',

    'timeline.title': 'Timeline',
    'timeline.subtitle': 'Click a block to open it in Timeline',
    'categories.title': 'Time by category',
    'categories.subtitle': 'Hours logged for the day',
    'hourly.title': 'Focus by hour',
    'hourly.subtitle': 'Share of time in productive categories',
    'top_apps.title': 'Top apps',
    'top_apps.empty': 'No apps recorded yet.',
  },
} satisfies NamespaceMessages;

export default messages;
