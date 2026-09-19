import type { NamespaceMessages } from '../types';

// Chart labels, tooltips, legends and tick formats. t('charts.<key>').
const messages = {
  'pt-BR': {
    legend: 'Legenda',
    uncategorized: 'Sem categoria',
    nothing_recorded: 'Nada registrado neste dia ainda.',
    recorded: 'registradas',
    day_track: 'Linha do tempo do dia',
    hour_tick: '{hour}h',
    dial_label: 'de foco',
    dial_aria: 'Score de foco {score} de 100, {mood}',
    no_data: 'sem dados',
    focus_value: 'foco {score}',
    score_value: 'score {score}',
    hours_unit: 'h',
  },
  en: {
    legend: 'Legend',
    uncategorized: 'Uncategorized',
    nothing_recorded: 'Nothing recorded on this day yet.',
    recorded: 'recorded',
    day_track: 'Day timeline',
    hour_tick: '{hour}:00',
    dial_label: 'focus',
    dial_aria: 'Focus score {score} out of 100, {mood}',
    no_data: 'no data',
    focus_value: 'focus {score}',
    score_value: 'score {score}',
    hours_unit: 'h',
  },
} satisfies NamespaceMessages;

export default messages;
