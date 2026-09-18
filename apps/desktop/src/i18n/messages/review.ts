import type { NamespaceMessages } from '../types';

// 'review' namespace (the Revisão page): header sentence, queue rows, the assign card, toasts and the empty state. t('review.<key>').
const messages = {
  'pt-BR': {
    title: 'Revisão',
    classify_now: 'Classificar agora',

    // Header subtitle, one sentence from the numbers: '{day}: {first}, {second}.'
    'subtitle.empty': '{day}: a fila está vazia.',
    'subtitle.groups_one': '{count} grupo espera sua decisão',
    'subtitle.groups_other': '{count} grupos esperam sua decisão',
    'subtitle.uncategorized': '{duration} ainda sem categoria',
    'subtitle.nothing_uncategorized': 'nada sem categoria',
    'subtitle.sentence': '{day}: {first}, {second}.',

    // Empty queue
    'empty.speech': 'Fila vazia. Bom trabalho!',
    'empty.title': 'Nada para revisar',
    'empty.body': 'Todos os blocos do dia estão classificados com boa confiança. Quando algo ficar em dúvida, aparece aqui.',
    'empty.cta': 'Ver a Timeline',

    // Queue rows
    list_label: 'Grupos para revisão',
    blocks_in_group: 'Blocos no grupo',
    needs_review: 'Precisa de revisão',
    confidence: 'Confiança: {value}',
    started_at: 'Começou às',
    min_confidence: 'Confiança mínima',
    // '{k1}' and '{k9}' are rendered as keyboard keys by the page.
    hint_one: 'Pressione {k1} a {k9} ou escolha a categoria ao lado; a decisão vale para este bloco e ensina o classificador.',
    hint_other: 'Pressione {k1} a {k9} ou escolha a categoria ao lado; a decisão vale para os {count} blocos e ensina o classificador.',

    // Assign card
    assign_label: 'Atribuir categoria',
    assign_title: 'Atribuir ao grupo selecionado',
    selected: '{app}, {duration}',
    selected_with_domain: '{app} em {domain}, {duration}',
    select_a_group: 'Selecione um grupo na lista',
    'keys.move': 'mover',
    'keys.assign': 'atribuir',

    // Toasts
    'toast.classified_one': '{count} bloco classificado',
    'toast.classified_other': '{count} blocos classificados',
    'toast.backfilled_one': '{count} bloco anterior foi preenchido retroativamente.',
    'toast.backfilled_other': '{count} blocos anteriores foram preenchidos retroativamente.',
    'toast.rule_disabled_title': 'Regra desativada',
    'toast.rule_disabled_body': 'A regra “{pattern}” foi contradita e desativada.',
    'toast.assign_failed': 'Não foi possível classificar',
    'toast.classify_done_title': 'Classificação concluída',
    'toast.classify_done_body': '{local} pelas regras, {remote} pela IA, {vision} por visão e {needs_review} para revisar.',
    'toast.classify_skipped_remote': 'A IA foi ignorada (modo somente local).',
    'toast.classify_failed': 'Falha ao classificar',
    'toast.rule_created': 'Regra criada',
    'toast.rule_failed': 'Não foi possível criar a regra',
  },
  en: {
    title: 'Review',
    classify_now: 'Classify now',

    // Header subtitle, one sentence from the numbers: '{day}: {first}, {second}.'
    'subtitle.empty': '{day}: the queue is empty.',
    'subtitle.groups_one': '{count} group awaits your decision',
    'subtitle.groups_other': '{count} groups await your decision',
    'subtitle.uncategorized': '{duration} still uncategorized',
    'subtitle.nothing_uncategorized': 'nothing uncategorized',
    'subtitle.sentence': '{day}: {first}, {second}.',

    // Empty queue
    'empty.speech': "Queue's empty. Nice work!",
    'empty.title': 'Nothing to review',
    'empty.body': 'Every block from this day is classified with good confidence. Anything uncertain will show up here.',
    'empty.cta': 'Go to Timeline',

    // Queue rows
    list_label: 'Groups to review',
    blocks_in_group: 'Blocks in this group',
    needs_review: 'Needs review',
    confidence: 'Confidence: {value}',
    started_at: 'Started at',
    min_confidence: 'Lowest confidence',
    // '{k1}' and '{k9}' are rendered as keyboard keys by the page.
    hint_one: 'Press {k1} to {k9} or pick a category on the right; your choice applies to this block and teaches the classifier.',
    hint_other: 'Press {k1} to {k9} or pick a category on the right; your choice applies to all {count} blocks and teaches the classifier.',

    // Assign card
    assign_label: 'Assign a category',
    assign_title: 'Assign to the selected group',
    selected: '{app}, {duration}',
    selected_with_domain: '{app} on {domain}, {duration}',
    select_a_group: 'Select a group from the list',
    'keys.move': 'move',
    'keys.assign': 'assign',

    // Toasts
    'toast.classified_one': '{count} block classified',
    'toast.classified_other': '{count} blocks classified',
    'toast.backfilled_one': '{count} earlier block was filled in retroactively.',
    'toast.backfilled_other': '{count} earlier blocks were filled in retroactively.',
    'toast.rule_disabled_title': 'Rule disabled',
    'toast.rule_disabled_body': 'The rule “{pattern}” was contradicted and has been disabled.',
    'toast.assign_failed': "Couldn't classify",
    'toast.classify_done_title': 'Classification complete',
    'toast.classify_done_body': '{local} by rules, {remote} by AI, {vision} by vision and {needs_review} to review.',
    'toast.classify_skipped_remote': 'AI was skipped (local-only mode).',
    'toast.classify_failed': 'Classification failed',
    'toast.rule_created': 'Rule created',
    'toast.rule_failed': "Couldn't create the rule",
  },
} satisfies NamespaceMessages;

export default messages;
