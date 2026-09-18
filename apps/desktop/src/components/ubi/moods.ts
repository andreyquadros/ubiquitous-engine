import type { Mood } from '../../lib/types';

/** Glow colour per mood: volt by default, UBI's sash ember when excited, rose when worried. */
export const MOOD_GLOW: Record<Mood, string> = {
  sleeping: '#7487a6',
  calm: '#4d8dff',
  focused: '#2ee6a6',
  excited: '#ff7a1f',
  worried: '#ff5c7a',
};

export const MOOD_TIP: Record<Mood, string> = {
  sleeping: 'Tudo quieto por aqui. Quando começar, eu registro.',
  calm: 'Dia tranquilo. Que tal um bloco de foco de 45 minutos?',
  focused: 'Você está no ritmo. Eu cuido do registro — segue o jogo.',
  excited: 'Que dia! Foco alto e poucas distrações. Orgulho de você.',
  worried: 'Muitas trocas de contexto hoje. Vamos fechar uma coisa de cada vez?',
};
