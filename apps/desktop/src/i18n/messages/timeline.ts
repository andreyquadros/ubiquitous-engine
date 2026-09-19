import type { NamespaceMessages } from '../types';

// 'timeline' namespace: the day's block list, the split dialog and the manual-entry dialog. Keys are used as t('timeline.<key>').
// Shared words (Cancelar, Adicionar, Categoria, Sem categoria, agora) come from common.*.
// Where the Portuguese copy uses "(s)" or a fixed plural, the _one form repeats it verbatim so pt-BR stays byte-identical while English gets a real singular.
const messages = {
  'pt-BR': {
    title: 'Timeline',
    subtitle: '{date}: {summary}',

    // header sentence written from the numbers
    summary_empty: 'Nenhum bloco registrado neste dia.',
    summary_total_one: '{duration} em {count} bloco',
    summary_total_other: '{duration} em {count} blocos',
    summary_all_classified: '{total}; tudo classificado.',
    summary_pending_one: '{total}; {count} ainda espera uma categoria.',
    summary_pending_other: '{total}; {count} ainda esperam uma categoria.',

    add_manual: 'Adicionar atividade manual',

    // macOS Screen Recording needs a relaunch
    restart_notice: 'Os títulos das janelas estão chegando vazios. O macOS só aplica a Gravação de Tela depois que o app reinicia.',
    restart_button: 'Reiniciar o ubiqX',

    // filters
    filter_label: 'Filtrar por categoria',
    filter_all: 'Todas as categorias',
    only_unclassified: 'Somente não classificados',
    shown_count_one: '{shown} de {count} blocos',
    shown_count_other: '{shown} de {count} blocos',

    // empty states
    empty_filtered_title: 'Nenhum bloco combina com os filtros',
    empty_filtered_desc: 'Limpe os filtros para ver o dia inteiro.',
    empty_title: 'Nenhum bloco neste dia',
    empty_desc: 'Quando o rastreador registrar algo, os blocos aparecem aqui. Você também pode adicionar uma atividade manual.',
    clear_filters: 'Limpar filtros',

    // block rows
    list_label: 'Blocos do dia',
    block_label: '{start} a {end}, {app}',
    block_range: '{start} até {end}',
    in_progress: 'em andamento',
    private_mode: 'Modo privado',
    idle: 'Ocioso',
    manual_badge: 'manual',
    needs_review: 'precisa de revisão',
    reclassify_heading: 'Reclassificar este bloco',

    // reclassify and rule toasts
    toast_reclassified: 'Categoria atualizada',
    toast_backfilled_one: '{count} bloco(s) semelhante(s) também foram ajustados.',
    toast_backfilled_other: '{count} bloco(s) semelhante(s) também foram ajustados.',
    toast_rule_disabled: 'Regra desativada',
    toast_rule_disabled_body: 'A regra “{pattern}” foi contradita e desativada.',
    toast_reclassify_failed: 'Não foi possível reclassificar',
    toast_rule_created: 'Regra criada',
    toast_rule_created_body: '{pattern} → sempre a mesma categoria.',
    toast_rule_failed: 'Não foi possível criar a regra',

    // split dialog
    split_action: 'Dividir bloco',
    split_description: '{app}, das {start} às {end}',
    split_confirm: 'Dividir',
    split_at: 'Dividir em',
    split_hint: 'O bloco será separado neste horário; as duas partes mantêm a categoria.',
    toast_split: 'Bloco dividido',
    toast_split_failed: 'Não foi possível dividir',

    // manual activity dialog
    manual_description: 'Para reuniões presenciais, leituras no papel ou qualquer coisa fora do computador.',
    manual_start: 'Início',
    manual_end: 'Fim',
    manual_note: 'Observação',
    manual_note_hint: 'Vai para o relatório do dia.',
    manual_note_placeholder: 'Ex.: Banca de TCC presencial',
    toast_invalid_time: 'Horário inválido',
    toast_invalid_time_body: 'O fim precisa ser depois do início.',
    toast_manual_added: 'Atividade adicionada',
    toast_manual_failed: 'Não foi possível adicionar',
  },
  en: {
    title: 'Timeline',
    subtitle: '{date}: {summary}',

    // header sentence written from the numbers
    summary_empty: 'No blocks recorded on this day.',
    summary_total_one: '{duration} across {count} block',
    summary_total_other: '{duration} across {count} blocks',
    summary_all_classified: '{total}; everything classified.',
    summary_pending_one: '{total}; {count} still needs a category.',
    summary_pending_other: '{total}; {count} still need a category.',

    add_manual: 'Add manual activity',

    // macOS Screen Recording needs a relaunch
    restart_notice: 'Window titles are coming through empty. macOS only applies Screen Recording after the app restarts.',
    restart_button: 'Restart ubiqX',

    // filters
    filter_label: 'Filter by category',
    filter_all: 'All categories',
    only_unclassified: 'Unclassified only',
    shown_count_one: '{shown} of {count} block',
    shown_count_other: '{shown} of {count} blocks',

    // empty states
    empty_filtered_title: 'No blocks match these filters',
    empty_filtered_desc: 'Clear the filters to see the whole day.',
    empty_title: 'No blocks on this day',
    empty_desc: 'Blocks show up here as soon as the tracker records something. You can also add an activity manually.',
    clear_filters: 'Clear filters',

    // block rows
    list_label: 'Blocks for the day',
    block_label: '{start} to {end}, {app}',
    block_range: '{start} to {end}',
    in_progress: 'in progress',
    private_mode: 'Private mode',
    idle: 'Idle',
    manual_badge: 'manual',
    needs_review: 'needs review',
    reclassify_heading: 'Reclassify this block',

    // reclassify and rule toasts
    toast_reclassified: 'Category updated',
    toast_backfilled_one: '{count} similar block was updated too.',
    toast_backfilled_other: '{count} similar blocks were updated too.',
    toast_rule_disabled: 'Rule disabled',
    toast_rule_disabled_body: 'The rule “{pattern}” was contradicted and has been disabled.',
    toast_reclassify_failed: "Couldn't reclassify",
    toast_rule_created: 'Rule created',
    toast_rule_created_body: '{pattern} → always this category.',
    toast_rule_failed: "Couldn't create the rule",

    // split dialog
    split_action: 'Split block',
    split_description: '{app}, {start} to {end}',
    split_confirm: 'Split',
    split_at: 'Split at',
    split_hint: 'The block is cut at this time; both halves keep the category.',
    toast_split: 'Block split',
    toast_split_failed: "Couldn't split the block",

    // manual activity dialog
    manual_description: 'For in-person meetings, reading on paper or anything else away from the computer.',
    manual_start: 'Start',
    manual_end: 'End',
    manual_note: 'Note',
    manual_note_hint: "Goes into the day's report.",
    manual_note_placeholder: 'E.g. In-person thesis defense',
    toast_invalid_time: 'Invalid time range',
    toast_invalid_time_body: 'The end time must be after the start time.',
    toast_manual_added: 'Activity added',
    toast_manual_failed: "Couldn't add the activity",
  },
} satisfies NamespaceMessages;

export default messages;
