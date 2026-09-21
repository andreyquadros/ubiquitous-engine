import type { NamespaceMessages } from '../types';

// Shared ui components (Dialog, DayNav, TagInput, BlockBits, CategoryChip, misc, IconPicker, LanguageSelect, toasts). t('ui.<key>').
const messages = {
  'pt-BR': {
    // DayNav
    previous_day: 'Dia anterior',
    next_day: 'Próximo dia',
    pick_date: 'Escolher data ({date})',
    today: 'Hoje',

    // TagInput
    tag_placeholder: 'Adicionar…',
    remove_tag: 'Remover {tag}',

    // misc
    loading: 'Carregando',
    close: 'Fechar',

    // CategoryChip / picker
    uncategorized: 'Sem categoria',
    pick_category: 'Escolher categoria',

    // BlockBits
    pending: 'pendente',
    classified_by: 'Classificado por: {source}',
    sent_to_ai_at: 'Enviado à IA em {at}',
    sent_to_ai: 'enviado à IA',
    confidence: 'Confiança: {value}',
    create_rule: 'Criar regra:',
    rule_created: 'Regra criada',
    always: 'Sempre',

    // IconPicker
    icon_picker: 'Ícone',

    // LanguageSelect
    language: 'Idioma',

    // engine event toasts (lib/engine.ts)
    report_ready_title: 'Relatório pronto',
    report_ready_body: 'Relatório de {date} gerado.',
    permission_required_title: 'Permissão necessária',
    permission_required_body: 'O ubiqX precisa da permissão: {permission}.',
  },
  en: {
    // DayNav
    previous_day: 'Previous day',
    next_day: 'Next day',
    pick_date: 'Pick a date ({date})',
    today: 'Today',

    // TagInput
    tag_placeholder: 'Add…',
    remove_tag: 'Remove {tag}',

    // misc
    loading: 'Loading',
    close: 'Close',

    // CategoryChip / picker
    uncategorized: 'Uncategorized',
    pick_category: 'Choose a category',

    // BlockBits
    pending: 'pending',
    classified_by: 'Classified by: {source}',
    sent_to_ai_at: 'Sent to AI on {at}',
    sent_to_ai: 'sent to AI',
    confidence: 'Confidence: {value}',
    create_rule: 'Create rule:',
    rule_created: 'Rule created',
    always: 'Always',

    // IconPicker
    icon_picker: 'Icon',

    // LanguageSelect
    language: 'Language',

    // engine event toasts (lib/engine.ts)
    report_ready_title: 'Report ready',
    report_ready_body: 'The report for {date} is ready.',
    permission_required_title: 'Permission required',
    permission_required_body: 'ubiqX needs the {permission} permission.',
  },
} satisfies NamespaceMessages;

export default messages;
