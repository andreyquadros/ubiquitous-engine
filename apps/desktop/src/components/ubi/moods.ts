import type { Mood } from '../../lib/types';

/** Glow colour per mood: brand blue by default, UBI's sash orange when excited. */
export const MOOD_GLOW: Record<Mood, string> = {
  sleeping: '#94a3b8',
  calm: '#3b82f6',
  focused: '#2563eb',
  excited: '#f97316',
  worried: '#ef4444',
};

export const MOOD_TIP: Record<Mood, string> = {
  sleeping: 'Tudo quieto por aqui. Quando começar, eu registro.',
  calm: 'Dia tranquilo. Que tal um bloco de foco de 45 minutos?',
  focused: 'Você está no ritmo. Eu cuido do registro — segue o jogo.',
  excited: 'Que dia! Foco alto e poucas distrações. Orgulho de você.',
  worried: 'Muitas trocas de contexto hoje. Vamos fechar uma coisa de cada vez?',
};
