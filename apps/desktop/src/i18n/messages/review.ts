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

    // Shown under an empty queue: every review surface is scoped to one day, so what is flagged
    // on another day would have no way of being found.
    'backlog.cta_one': 'Ver {count} bloco sinalizado em {day}',
    'backlog.cta_other': 'Ver {count} blocos sinalizados em {day}',

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
    'keys.details': 'detalhes',

    // Row details (every block of the group, so the user can decide with context)
    'details.open': 'Ver detalhes do grupo',
    'details.close': 'Ocultar detalhes do grupo',
    'details.loading': 'Carregando os blocos do grupo',
    'details.blocks_one': '{count} bloco neste grupo',
    'details.blocks_other': '{count} blocos neste grupo',
    'block.range': '{from} até {to}, {duration}',
    'block.no_title': 'Sem título',
    'block.ai_payload': 'Dados enviados à IA',
    'block.ai_payload_hide': 'Ocultar dados enviados à IA',
    'block.not_sent': 'Nada foi enviado à IA para este bloco.',
    'block.no_screenshot': 'Sem captura de tela para este bloco.',
    'block.screenshot_gone': 'A captura foi apagada depois da classificação.',
    'block.keep_hint': 'Para ver a imagem ao revisar, ative “Manter screenshots para revisão” nas configurações.',
    'block.screenshot_alt': 'Captura de {app} às {time}',
    'block.screenshot_open': 'Ampliar captura',
    'block.screenshot_loading': 'Carregando captura',
    'block.screenshot_dialog': '{app}, {range}',

    // Everything already answered -- by the Ubi or by the user -- collects at the bottom.
    'reviewed.title': 'Classificados neste dia ({count})',
    'reviewed.hint': 'A etiqueta de cada linha diz quem decidiu. Clique para reabrir e reatribuir.',
    'reviewed.show': 'Mostrar os grupos classificados',
    'reviewed.hide': 'Ocultar os grupos classificados',
    'reviewed.empty': 'Nenhum grupo classificado ainda.',
    'suggestions.after': 'Da última decisão',

    // One click turns the Ubi's own answers into the user's, which is what feeds the memory.
    'confirm.hint_one': '{count} grupo foi o Ubi que decidiu.',
    'confirm.hint_other': '{count} grupos foram o Ubi que decidiu.',
    'confirm.cta_one': 'Confirmar {count}',
    'confirm.cta_other': 'Confirmar os {count}',

    // Queue cleared: nothing was left in doubt
    'done.speech': 'Nada em dúvida!',
    'done.title': 'Nada esperando por você',
    'done.body_one': 'O Ubi resolveu {count} grupo neste dia. Abra a lista abaixo se quiser conferir.',
    'done.body_other': 'O Ubi resolveu {count} grupos neste dia. Abra a lista abaixo se quiser conferir.',

    // Toasts
    'toast.classified_one': '{count} bloco classificado',
    'toast.classified_other': '{count} blocos classificados',
    'toast.backfilled_one': '{count} bloco anterior foi preenchido retroativamente.',
    'toast.backfilled_other': '{count} blocos anteriores foram preenchidos retroativamente.',
    'toast.rule_disabled_title': 'Regra desativada',
    'toast.rule_disabled_body': 'A regra “{pattern}” foi contradita e desativada.',
    'toast.assign_failed': 'Não foi possível classificar',
    'toast.confirmed_one': '{count} grupo confirmado',
    'toast.confirmed_other': '{count} grupos confirmados',
    'toast.confirmed_body_one': '{count} bloco virou memória: o Ubi resolve sozinho da próxima vez.',
    'toast.confirmed_body_other': '{count} blocos viraram memória: o Ubi resolve sozinho da próxima vez.',
    'toast.confirm_failed': 'Não foi possível confirmar',
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

    // Shown under an empty queue: every review surface is scoped to one day, so what is flagged
    // on another day would have no way of being found.
    'backlog.cta_one': 'See {count} flagged block from {day}',
    'backlog.cta_other': 'See {count} flagged blocks from {day}',

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
    'keys.details': 'details',

    // Row details (every block of the group, so the user can decide with context)
    'details.open': 'Show group details',
    'details.close': 'Hide group details',
    'details.loading': 'Loading the blocks in this group',
    'details.blocks_one': '{count} block in this group',
    'details.blocks_other': '{count} blocks in this group',
    'block.range': '{from} to {to}, {duration}',
    'block.no_title': 'No title',
    'block.ai_payload': 'Data sent to the AI',
    'block.ai_payload_hide': 'Hide data sent to the AI',
    'block.not_sent': 'Nothing was sent to the AI for this block.',
    'block.no_screenshot': 'No screenshot for this block.',
    'block.screenshot_gone': 'The screenshot was deleted after classification.',
    'block.keep_hint': 'To see the image while reviewing, turn on “Keep screenshots for review” in Settings.',
    'block.screenshot_alt': 'Screenshot of {app} at {time}',
    'block.screenshot_open': 'Enlarge screenshot',
    'block.screenshot_loading': 'Loading screenshot',
    'block.screenshot_dialog': '{app}, {range}',

    // Everything already answered -- by the Ubi or by the user -- collects at the bottom.
    'reviewed.title': 'Classified on this day ({count})',
    'reviewed.hint': 'The badge on each row says who decided. Click one to reopen it and reassign.',
    'reviewed.show': 'Show classified groups',
    'reviewed.hide': 'Hide classified groups',
    'reviewed.empty': 'No group classified yet.',
    'suggestions.after': 'From your last decision',

    // One click turns the Ubi's own answers into the user's, which is what feeds the memory.
    'confirm.hint_one': '{count} group was decided by the Ubi.',
    'confirm.hint_other': '{count} groups were decided by the Ubi.',
    'confirm.cta_one': 'Confirm {count}',
    'confirm.cta_other': 'Confirm all {count}',

    // Queue cleared: nothing was left in doubt
    'done.speech': 'Nothing uncertain!',
    'done.title': 'Nothing waiting on you',
    'done.body_one': 'The Ubi settled {count} group on this day. Open the list below to check its work.',
    'done.body_other': 'The Ubi settled {count} groups on this day. Open the list below to check its work.',

    // Toasts
    'toast.classified_one': '{count} block classified',
    'toast.classified_other': '{count} blocks classified',
    'toast.backfilled_one': '{count} earlier block was filled in retroactively.',
    'toast.backfilled_other': '{count} earlier blocks were filled in retroactively.',
    'toast.rule_disabled_title': 'Rule disabled',
    'toast.rule_disabled_body': 'The rule “{pattern}” was contradicted and has been disabled.',
    'toast.assign_failed': "Couldn't classify",
    'toast.confirmed_one': '{count} group confirmed',
    'toast.confirmed_other': '{count} groups confirmed',
    'toast.confirmed_body_one': '{count} block became memory: the Ubi will settle it on its own next time.',
    'toast.confirmed_body_other': '{count} blocks became memory: the Ubi will settle them on its own next time.',
    'toast.confirm_failed': "Couldn't confirm",
    'toast.classify_done_title': 'Classification complete',
    'toast.classify_done_body': '{local} by rules, {remote} by AI, {vision} by vision and {needs_review} to review.',
    'toast.classify_skipped_remote': 'AI was skipped (local-only mode).',
    'toast.classify_failed': 'Classification failed',
    'toast.rule_created': 'Rule created',
    'toast.rule_failed': "Couldn't create the rule",
  },
} satisfies NamespaceMessages;

export default messages;
