import type { NamespaceMessages } from '../types';

// 'reports' namespace: page chrome for Reports (daily and monthly). Report bodies come from the backend
// already in the chosen language. Keys are used as t('reports.<key>').
const messages = {
  'pt-BR': {
    // header
    title: 'Relatórios',
    subtitle: 'Gerados no horário de cada categoria e editáveis a qualquer momento.',
    tab_daily: 'Diário',
    tab_monthly: 'Mensal',

    // daily: status line and empty states
    status_none: 'nenhum relatório gerado ainda',
    status_all: 'todos os relatórios prontos',
    status_some: '{ready} de {total} relatórios prontos',
    empty_categories_title: 'Nenhuma categoria com relatório',
    empty_categories_description: 'Crie categorias produtivas em Categorias para receber relatórios diários.',
    empty_report_title: 'Nenhum relatório de {category} para este dia',
    empty_report_description: 'Ele será escrito às {time} a partir dos blocos desta categoria. Se preferir, gere agora.',

    // daily: report card
    badge_stale: 'Desatualizado',
    badge_edited: 'Editado',
    generated_meta: 'Gerado {time} com {model}',
    tracked_meta: '{duration} registradas',
    tokens_meta: '{input} tokens de entrada, {output} de saída',
    auto_at: 'Gera automaticamente às {time}',
    copy_markdown: 'Copiar Markdown',
    regenerate: 'Regenerar',
    generate_now: 'Gerar agora',
    highlights: 'Destaques',
    col_activity: 'Atividade',
    col_minutes: 'Minutos',
    col_kind: 'Tipo',
    col_time: 'Horário',
    items_label: 'Atividades do relatório',
    continues_from: 'Continua de {activity}',
    no_items: 'Sem atividades neste relatório.',

    // toasts
    toast_generated: 'Relatório gerado',
    toast_generated_body: '{category}, com {model}.',
    toast_generate_failed: 'Não foi possível gerar',
    toast_saved: 'Relatório salvo',
    toast_save_failed: 'Não foi possível salvar',
    toast_copied: 'Markdown copiado',
    toast_monthly_failed: 'Não foi possível gerar o relatório mensal',

    // monthly
    month: 'Mês',
    generate_monthly: 'Gerar relatório mensal',
    monthly_report: 'Relatório mensal',
    monthly_meta: '{month}, consolidado a partir dos relatórios diários',
    download_md: 'Baixar .md',
    empty_monthly_title: 'Nenhum mês gerado ainda',
    empty_monthly_description: 'Escolha a categoria e o mês acima e gere o relatório: um único Markdown por categoria, pronto para o relatório de atividades docentes ou da incubadora.',

    // exports (file name and Markdown heading)
    file_name: 'relatorio-{category}-{month}.md',
    file_category_fallback: 'categoria',
    md_activities: 'Atividades',
  },
  en: {
    // header
    title: 'Reports',
    subtitle: "Written on each category's schedule and editable at any time.",
    tab_daily: 'Daily',
    tab_monthly: 'Monthly',

    // daily: status line and empty states
    status_none: 'no reports generated yet',
    status_all: 'all reports ready',
    status_some: '{ready} of {total} reports ready',
    empty_categories_title: 'No categories with reports',
    empty_categories_description: 'Create productive categories under Categories to receive daily reports.',
    empty_report_title: 'No {category} report for this day',
    empty_report_description: "It will be written at {time} from this category's blocks. You can also generate it now.",

    // daily: report card
    badge_stale: 'Out of date',
    badge_edited: 'Edited',
    generated_meta: 'Generated {time} with {model}',
    tracked_meta: '{duration} tracked',
    tokens_meta: '{input} tokens in, {output} out',
    auto_at: 'Generates automatically at {time}',
    copy_markdown: 'Copy Markdown',
    regenerate: 'Regenerate',
    generate_now: 'Generate now',
    highlights: 'Highlights',
    col_activity: 'Activity',
    col_minutes: 'Minutes',
    col_kind: 'Type',
    col_time: 'Time',
    items_label: 'Report activities',
    continues_from: 'Continues from {activity}',
    no_items: 'No activities in this report.',

    // toasts
    toast_generated: 'Report generated',
    toast_generated_body: '{category}, using {model}.',
    toast_generate_failed: "Couldn't generate the report",
    toast_saved: 'Report saved',
    toast_save_failed: "Couldn't save the report",
    toast_copied: 'Markdown copied',
    toast_monthly_failed: "Couldn't generate the monthly report",

    // monthly
    month: 'Month',
    generate_monthly: 'Generate monthly report',
    monthly_report: 'Monthly report',
    monthly_meta: '{month}, consolidated from the daily reports',
    download_md: 'Download .md',
    empty_monthly_title: 'No month generated yet',
    empty_monthly_description: 'Choose a category and month above, then generate the report: a single Markdown file per category, ready for the faculty activity report or the incubator.',

    // exports (file name and Markdown heading)
    file_name: 'report-{category}-{month}.md',
    file_category_fallback: 'category',
    md_activities: 'Activities',
  },
} satisfies NamespaceMessages;

export default messages;
