import type { NamespaceMessages } from '../types';

// 'insights' namespace (the weekly page): header, the four stat tiles, the two charts, UBI's advice card and the
// nudge history. Keys are used as t('insights.<key>'). Shared verbs (Gerar) come from common.*.
const messages = {
  'pt-BR': {
    title: 'Insights',
    subtitle: 'Últimos 7 dias, de {from} a {to}',

    // stat tiles
    'stat.productive': 'Tempo produtivo',
    'stat.productive_hint': 'na semana',
    'stat.avg_score': 'Score médio',
    'stat.active_days_one': '{count} dia com atividade',
    'stat.active_days_other': '{count} dias com atividade',
    'stat.calculating': 'calculando',
    'stat.longest_focus': 'Maior foco contínuo',
    'stat.longest_focus_on': 'em {date}',
    'stat.no_streaks': 'sem sequências ainda',
    'stat.switches_per_hour': 'Trocas por hora',
    'stat.switches_hint': 'média dos dias ativos',

    // charts
    'hours.title': 'Horas por categoria',
    'hours.subtitle': 'Empilhadas por dia',
    'score.title': 'Score de foco',
    'score.subtitle': 'De 0 a 100, por dia',

    // UBI's advice card
    'advice.title': 'Recomendações do UBI',
    'advice.subtitle': 'Uma leitura semanal dos seus padrões. Custa uma chamada ao modelo de relatórios.',
    'advice.generate_again': 'Gerar novamente',
    'advice.analysing': 'Analisando sua semana…',
    'advice.list_label': 'Recomendações',
    'advice.written_by': 'Escrito por {model}',
    'advice.empty_title': 'Sem recomendações ainda',
    'advice.empty_description': 'Toque em Gerar para o UBI ler seus últimos dias e sugerir ajustes concretos de rotina.',
    'advice.error': 'Não foi possível gerar recomendações',

    // nudge history
    'nudges.title': 'O que o UBI já disse',
    'nudges.subtitle': 'Avisos recentes',
    'nudges.new': 'novo',
    'nudges.empty_title': 'Nenhum aviso ainda',
    'nudges.empty_description': 'O UBI fala quando nota algo: muitas trocas, tempo demais sem pausa, um bloco longo de foco.',
  },
  en: {
    title: 'Insights',
    subtitle: 'Last 7 days, {from} to {to}',

    // stat tiles
    'stat.productive': 'Productive time',
    'stat.productive_hint': 'this week',
    'stat.avg_score': 'Average score',
    'stat.active_days_one': '{count} active day',
    'stat.active_days_other': '{count} active days',
    'stat.calculating': 'calculating',
    'stat.longest_focus': 'Longest focus streak',
    'stat.longest_focus_on': 'on {date}',
    'stat.no_streaks': 'no streaks yet',
    'stat.switches_per_hour': 'Switches per hour',
    'stat.switches_hint': 'average across active days',

    // charts
    'hours.title': 'Hours by category',
    'hours.subtitle': 'Stacked by day',
    'score.title': 'Focus score',
    'score.subtitle': '0 to 100, per day',

    // UBI's advice card
    'advice.title': "UBI's recommendations",
    'advice.subtitle': 'A weekly read on your patterns. Uses one call to the reports model.',
    'advice.generate_again': 'Generate again',
    'advice.analysing': 'Reading your week…',
    'advice.list_label': 'Recommendations',
    'advice.written_by': 'Written by {model}',
    'advice.empty_title': 'No recommendations yet',
    'advice.empty_description': 'Hit Generate and UBI will read your last few days and suggest concrete changes to your routine.',
    'advice.error': "Couldn't generate recommendations",

    // nudge history
    'nudges.title': 'What UBI has said',
    'nudges.subtitle': 'Recent nudges',
    'nudges.new': 'new',
    'nudges.empty_title': 'No nudges yet',
    'nudges.empty_description': 'UBI speaks up when it notices something: too much switching, too long without a break, a long stretch of focus.',
  },
} satisfies NamespaceMessages;

export default messages;
