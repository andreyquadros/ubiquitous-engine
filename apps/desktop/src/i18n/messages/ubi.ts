import type { NamespaceMessages } from '../types';

// UBI the mascot: speech lines per mood, the assistant strip, AI status. t('ubi.<key>').
const messages = {
  'pt-BR': {
    mascot: 'UBI, o mascote ({mood})',
    assistant: 'Assistente',
    says: 'UBI diz',

    'tip.sleeping': 'Tudo quieto por aqui. Quando começar, eu registro.',
    'tip.calm': 'Dia tranquilo. Que tal um bloco de foco de 45 minutos?',
    'tip.focused': 'Você está no ritmo. Eu cuido do registro — segue o jogo.',
    'tip.excited': 'Que dia! Foco alto e poucas distrações. Orgulho de você.',
    'tip.worried': 'Muitas trocas de contexto hoje. Vamos fechar uma coisa de cada vez?',

    'ai.ok': 'IA ativa',
    'ai.not_configured': 'IA não configurada',
    'ai.not_configured_hint': 'Adicione sua chave em Configurações, na aba IA.',
    'ai.paused': 'IA pausada',
    'ai.degraded': 'IA instável',
    'ai.degraded_until': '{reason} até {time}',

    usage_month: 'Uso da IA no mês',
    usage_of: 'de {budget}',
    usage_detail: '{calls} chamadas, {tokens}k tokens de entrada',
  },
  en: {
    mascot: 'UBI, the mascot ({mood})',
    assistant: 'Assistant',
    says: 'UBI says',

    'tip.sleeping': "All quiet here. As soon as you start, I'll keep track.",
    'tip.calm': 'A calm day. How about a 45-minute focus block?',
    'tip.focused': "You're in the zone. I've got the tracking covered, keep going.",
    'tip.excited': 'What a day! High focus and hardly any distractions. Proud of you.',
    'tip.worried': "A lot of context switching today. Let's finish one thing at a time?",

    'ai.ok': 'AI active',
    'ai.not_configured': 'AI not set up',
    'ai.not_configured_hint': 'Add your key in Settings, under the AI tab.',
    'ai.paused': 'AI paused',
    'ai.degraded': 'AI unstable',
    'ai.degraded_until': '{reason} until {time}',

    usage_month: 'AI usage this month',
    usage_of: 'of {budget}',
    usage_detail: '{calls} calls, {tokens}k input tokens',
  },
} satisfies NamespaceMessages;

export default messages;
