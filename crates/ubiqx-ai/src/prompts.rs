//! Prompt construction.
//!
//! Everything here works on *redacted* views: a [`PromptBlock`] can only be created through
//! [`ubiqx_core::redact::redact_block`] (its fields are private and the sole constructor
//! redacts), so a raw window title or a URL with its query string cannot end up in a prompt
//! by construction. [`render_block_line`] is the exact per-block line the classifier sends;
//! the engine stores it as `ActivityBlock::ai_payload` so what is stored equals what was sent.
//!
//! Classification instructions are written in English (compact and stable, good for caching);
//! the report writer and the advisor get instructions written in the language they must
//! answer in, with examples in that language. The language comes from the request
//! (`Settings.language`, `pt-BR` by default).

use std::collections::HashSet;

use chrono::{DateTime, Datelike, FixedOffset, Local, Offset, TimeZone, Utc, Weekday};
use ubiqx_core::ports::{
    AdviceRequest, ClassificationContext, ClassificationExample, ReportRequest,
};
use ubiqx_core::redact::{redact_block, redact_text, RedactedBlock};
use ubiqx_core::report::format_minutes;
use ubiqx_core::{
    system_categories, ActivityBlock, Category, Mood, NudgeKind, ReportItem, UiLanguage,
};

/// Language used when the request does not name one.
pub const DEFAULT_LANGUAGE: &str = "pt-BR";
/// Upper bound of few-shot examples in a classification prompt.
pub const MAX_EXAMPLES: usize = 12;
/// Upper bound of previous items shown to the report writer.
pub const MAX_PREVIOUS_ITEMS: usize = 20;
const MAX_TITLE_CHARS: usize = 120;
const MAX_PATH_CHARS: usize = 80;
const MAX_DESCRIPTION_CHARS: usize = 200;
const MAX_PROFILE_CHARS: usize = 800;
const MAX_TEMPLATE_CHARS: usize = 3000;

// ---------------------------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------------------------

/// `lang` unless blank, else [`DEFAULT_LANGUAGE`].
pub fn language_or_default(lang: &str) -> &str {
    let l = lang.trim();
    if l.is_empty() {
        DEFAULT_LANGUAGE
    } else {
        l
    }
}

/// The UI language named by a request's `language` tag (unknown tags mean pt-BR).
pub fn ui_language(lang: &str) -> UiLanguage {
    UiLanguage::from_tag(language_or_default(lang))
}

/// How an English instruction names the answer language.
fn language_name_en(lang: UiLanguage) -> &'static str {
    match lang {
        UiLanguage::PtBr => "Brazilian Portuguese",
        UiLanguage::En => "English",
    }
}

/// Offset of the machine's local time zone at `at`, in seconds east of UTC.
pub fn local_offset_secs(at: DateTime<Utc>) -> i32 {
    Local
        .offset_from_utc_datetime(&at.naive_utc())
        .local_minus_utc()
}

fn fixed_offset(secs: i32) -> FixedOffset {
    FixedOffset::east_opt(secs).unwrap_or_else(|| Utc.fix())
}

/// `HH:MM` of `at` in the given offset.
pub fn format_local_time(at: DateTime<Utc>, utc_offset_secs: i32) -> String {
    at.with_timezone(&fixed_offset(utc_offset_secs))
        .format("%H:%M")
        .to_string()
}

/// `HH:MM–HH:MM` in the given offset.
pub fn format_time_range(start: DateTime<Utc>, end: DateTime<Utc>, utc_offset_secs: i32) -> String {
    format!(
        "{}–{}",
        format_local_time(start, utc_offset_secs),
        format_local_time(end, utc_offset_secs)
    )
}

/// `UTC-04:00` style label.
fn format_offset(utc_offset_secs: i32) -> String {
    let sign = if utc_offset_secs < 0 { '-' } else { '+' };
    let abs = utc_offset_secs.unsigned_abs();
    format!("UTC{sign}{:02}:{:02}", abs / 3600, (abs % 3600) / 60)
}

/// Collapses whitespace, replaces the field separator and truncates to `max_chars`.
fn squash(text: &str, max_chars: usize) -> String {
    let joined = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "/");
    if joined.chars().count() > max_chars {
        let mut cut: String = joined.chars().take(max_chars.saturating_sub(1)).collect();
        cut.push('…');
        cut
    } else {
        joined
    }
}

/// Path component of an (already redacted) URL, or `None` when it is just `/`.
fn path_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let start = rest.find('/')?;
    let path = rest[start..].trim_end_matches('/');
    if path.is_empty() {
        None
    } else {
        Some(squash(path, MAX_PATH_CHARS))
    }
}

fn weekday_name(w: Weekday, lang: UiLanguage) -> &'static str {
    match (w, lang) {
        (Weekday::Mon, UiLanguage::PtBr) => "segunda-feira",
        (Weekday::Tue, UiLanguage::PtBr) => "terça-feira",
        (Weekday::Wed, UiLanguage::PtBr) => "quarta-feira",
        (Weekday::Thu, UiLanguage::PtBr) => "quinta-feira",
        (Weekday::Fri, UiLanguage::PtBr) => "sexta-feira",
        (Weekday::Sat, UiLanguage::PtBr) => "sábado",
        (Weekday::Sun, UiLanguage::PtBr) => "domingo",
        (Weekday::Mon, UiLanguage::En) => "Monday",
        (Weekday::Tue, UiLanguage::En) => "Tuesday",
        (Weekday::Wed, UiLanguage::En) => "Wednesday",
        (Weekday::Thu, UiLanguage::En) => "Thursday",
        (Weekday::Fri, UiLanguage::En) => "Friday",
        (Weekday::Sat, UiLanguage::En) => "Saturday",
        (Weekday::Sun, UiLanguage::En) => "Sunday",
    }
}

fn mood_label(mood: Mood) -> &'static str {
    match mood {
        Mood::Sleeping => "sleeping",
        Mood::Calm => "calm",
        Mood::Focused => "focused",
        Mood::Excited => "excited",
        Mood::Worried => "worried",
    }
}

fn secs_to_minutes(secs: i64) -> u32 {
    u32::try_from(secs.max(0) / 60).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------------------------
// PromptBlock
// ---------------------------------------------------------------------------------------------

/// The view of a block a model is allowed to see. Only constructible from a redacted block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptBlock {
    block_id: String,
    app_name: String,
    title: String,
    domain: Option<String>,
    path: Option<String>,
    duration_minutes: u32,
    time_range: String,
    description: Option<String>,
}

impl PromptBlock {
    /// Redacts `block` and renders its local times with `utc_offset_secs`.
    pub fn from_block(block: &ActivityBlock, utc_offset_secs: i32) -> Self {
        let redacted = redact_block(
            &block.app_id,
            &block.app_name,
            &block.title,
            block.url.as_deref(),
            block.domain.as_deref(),
        );
        Self::from_redacted(
            &block.id,
            redacted,
            block.started_at,
            block.ended_at,
            utc_offset_secs,
            block.description.as_deref(),
        )
    }

    fn from_redacted(
        block_id: &str,
        redacted: RedactedBlock,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
        utc_offset_secs: i32,
        description: Option<&str>,
    ) -> Self {
        let secs = (ended_at - started_at).num_seconds().max(0);
        let duration_minutes = u32::try_from((secs + 30) / 60).unwrap_or(u32::MAX);
        let title = squash(&redacted.title, MAX_TITLE_CHARS);
        Self {
            block_id: block_id.to_string(),
            app_name: squash(&redacted.app_name, 60),
            title: if title.is_empty() {
                "(sem título)".to_string()
            } else {
                title
            },
            domain: redacted
                .domain
                .as_deref()
                .map(|d| squash(d, 80))
                .filter(|d| !d.is_empty()),
            path: redacted.url.as_deref().and_then(path_of),
            duration_minutes,
            time_range: format_time_range(started_at, ended_at, utc_offset_secs),
            // Descriptions come from our own classifiers or from a manual note; mask anyway.
            description: description
                .map(|d| squash(&redact_text(d), MAX_DESCRIPTION_CHARS))
                .filter(|d| !d.is_empty()),
        }
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn duration_minutes(&self) -> u32 {
        self.duration_minutes
    }

    pub fn time_range(&self) -> &str {
        &self.time_range
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The classification line:
    /// `- id=<id> | app=<app> | title=<title> [| domain=<d>] [| path=<p>] | min=<n> | time=<HH:MM–HH:MM>`.
    pub fn render_line(&self) -> String {
        let mut parts = vec![
            format!("id={}", self.block_id),
            format!("app={}", self.app_name),
            format!("title={}", self.title),
        ];
        if let Some(d) = &self.domain {
            parts.push(format!("domain={d}"));
        }
        if let Some(p) = &self.path {
            parts.push(format!("path={p}"));
        }
        parts.push(format!("min={}", self.duration_minutes));
        parts.push(format!("time={}", self.time_range));
        format!("- {}", parts.join(" | "))
    }

    /// The report line (no id, includes the description when present):
    /// `- <HH:MM–HH:MM> | <n> min | app=<app> | title=<title> [| domain=<d>] [| path=<p>] [| descrição=<d>]`.
    pub fn render_report_line(&self) -> String {
        let mut parts = vec![
            self.time_range.clone(),
            format!("{} min", self.duration_minutes),
            format!("app={}", self.app_name),
            format!("title={}", self.title),
        ];
        if let Some(d) = &self.domain {
            parts.push(format!("domain={d}"));
        }
        if let Some(p) = &self.path {
            parts.push(format!("path={p}"));
        }
        if let Some(d) = &self.description {
            parts.push(format!("descrição={d}"));
        }
        format!("- {}", parts.join(" | "))
    }
}

/// The exact per-block line used in classification prompts, with times in the machine's local
/// zone. The engine stores this as `ai_payload`.
pub fn render_block_line(block: &ActivityBlock) -> String {
    render_block_line_with_offset(block, local_offset_secs(block.started_at))
}

/// [`render_block_line`] with an explicit UTC offset (deterministic; used by tests and by
/// callers that already know the user's zone).
pub fn render_block_line_with_offset(block: &ActivityBlock, utc_offset_secs: i32) -> String {
    PromptBlock::from_block(block, utc_offset_secs).render_line()
}

// ---------------------------------------------------------------------------------------------
// Category catalogue
// ---------------------------------------------------------------------------------------------

/// Category ids a model may answer with: the (non-archived) context categories plus the
/// `Distraction` and `Break` system categories. `Uncategorized` and `Private` are never
/// valid answers (`null` means uncategorised; private blocks are never sent).
pub fn allowed_category_ids(categories: &[Category]) -> HashSet<String> {
    let mut ids: HashSet<String> = categories
        .iter()
        .filter(|c| !c.archived)
        .map(|c| c.id.clone())
        .collect();
    ids.remove(system_categories::UNCATEGORIZED);
    ids.remove(system_categories::PRIVATE);
    ids.insert(system_categories::DISTRACTION.to_string());
    ids.insert(system_categories::BREAK.to_string());
    ids
}

/// Field labels of a catalogue line, per language.
struct CatalogueLabels {
    name: &'static str,
    productive: &'static str,
    yes: &'static str,
    no: &'static str,
    description: &'static str,
    keywords: &'static str,
    distraction: (&'static str, &'static str),
    brk: (&'static str, &'static str),
}

impl CatalogueLabels {
    fn for_language(lang: UiLanguage) -> Self {
        match lang {
            UiLanguage::PtBr => Self {
                name: "nome",
                productive: "produtiva",
                yes: "sim",
                no: "não",
                description: "descrição",
                keywords: "palavras-chave",
                distraction: (
                    "Distração",
                    "Entretenimento, redes sociais, vídeos, jogos ou compras sem relação com o trabalho",
                ),
                brk: (
                    "Pausa",
                    "Descanso, refeição ou pausa deliberada longe do trabalho",
                ),
            },
            UiLanguage::En => Self {
                name: "name",
                productive: "productive",
                yes: "yes",
                no: "no",
                description: "description",
                keywords: "keywords",
                distraction: (
                    "Distraction",
                    "Entertainment, social media, videos, games or shopping unrelated to work",
                ),
                brk: ("Break", "Rest, a meal or a deliberate pause away from work"),
            },
        }
    }
}

/// One line per category: `- id=… | nome=… | produtiva=sim | descrição=… | palavras-chave=…`
/// (labels in the UI language), always ending with the `Distraction` and `Break` system
/// categories when the caller did not include them.
pub fn render_catalogue(categories: &[Category], lang: UiLanguage) -> String {
    let l = CatalogueLabels::for_language(lang);
    let mut lines = Vec::new();
    let mut has_distraction = false;
    let mut has_break = false;
    for c in categories.iter().filter(|c| {
        !c.archived
            && c.id != system_categories::UNCATEGORIZED
            && c.id != system_categories::PRIVATE
    }) {
        has_distraction |= c.id == system_categories::DISTRACTION;
        has_break |= c.id == system_categories::BREAK;
        let mut line = format!(
            "- id={} | {}={} | {}={}",
            c.id,
            l.name,
            squash(&c.name, 60),
            l.productive,
            if c.is_productive { l.yes } else { l.no }
        );
        let desc = squash(&c.description, 300);
        if !desc.is_empty() {
            line.push_str(&format!(" | {}={desc}", l.description));
        }
        let keywords: Vec<String> = c
            .keywords
            .iter()
            .map(|k| squash(k, 40))
            .filter(|k| !k.is_empty())
            .take(20)
            .collect();
        if !keywords.is_empty() {
            line.push_str(&format!(" | {}={}", l.keywords, keywords.join(", ")));
        }
        lines.push(line);
    }
    if !has_distraction {
        lines.push(format!(
            "- id={} | {}={} | {}={} | {}={}",
            system_categories::DISTRACTION,
            l.name,
            l.distraction.0,
            l.productive,
            l.no,
            l.description,
            l.distraction.1
        ));
    }
    if !has_break {
        lines.push(format!(
            "- id={} | {}={} | {}={} | {}={}",
            system_categories::BREAK,
            l.name,
            l.brk.0,
            l.productive,
            l.no,
            l.description,
            l.brk.1
        ));
    }
    lines.join("\n")
}

fn profile_section(profile: Option<&str>, heading: &str) -> String {
    match profile
        .map(|p| squash(&redact_text(p), MAX_PROFILE_CHARS))
        .filter(|p| !p.is_empty())
    {
        Some(p) => format!("\n## {heading}\n{p}\n"),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------------------------

/// Lean, stable system prompt for batch classification (role, catalogue, rules, profile).
pub fn classification_system_prompt(ctx: &ClassificationContext) -> String {
    let lang = ui_language(&ctx.language);
    let language = language_name_en(lang);
    let example = lang.pick(
        "Editou a planilha de orçamento da incubadora",
        "Edited the incubator's budget spreadsheet",
    );
    let mut s = String::with_capacity(4096);
    s.push_str("You classify blocks of desktop activity into the user's categories.\n");
    s.push_str("A block is one stretch of time in one application/window, described by: app name, window title, web domain, URL path (query strings removed), duration in minutes and local time range. Titles were anonymised before reaching you (e-mail addresses, phone and document numbers are masked) and the titles of messaging or e-mail apps were replaced by the app name.\n\n");
    s.push_str("## Categories\n");
    s.push_str(&render_catalogue(&ctx.categories, lang));
    s.push_str("\n\n## Rules\n");
    s.push_str("- `category_id` must be one of the ids listed above, or null when the block cannot be placed with reasonable confidence. Never invent ids.\n");
    s.push_str("- Judge by app, title, domain and path together; match keywords loosely (synonyms, abbreviations, Portuguese and English variants). Neighbouring blocks of the same batch are context: work usually continues across apps.\n");
    s.push_str("- Messaging and e-mail apps (WhatsApp, Mail, Outlook, Slack, Teams, Telegram…) carry no topic in the title: decide by the app, the examples and the neighbouring blocks; when nothing indicates the topic answer null.\n");
    s.push_str("- `needs_vision` is true only when the text is insufficient AND a screenshot would probably settle it (generic titles such as \"Nova guia\", \"Finder\", \"Terminal\", messaging apps, file dialogs). Otherwise false.\n");
    s.push_str("- `confidence`: 0.9–1.0 for an obvious match, 0.6–0.8 when likely, below 0.5 for a guess (prefer null).\n");
    s.push_str(&format!("- `description`: one short sentence in {language}, past tense, about the topic or task only, e.g. \"{example}\". Never include names of people, message or e-mail contents, quoted text or identifiers. Use null when nothing useful can be said.\n"));
    s.push_str(
        "- Answer with the JSON object only: one result per block id, every block exactly once.\n",
    );
    s.push_str(&profile_section(
        ctx.user_profile.as_deref(),
        "About the user",
    ));
    s
}

/// Few-shot examples (at most [`MAX_EXAMPLES`], redacted, only with valid category ids).
pub fn render_examples(examples: &[ClassificationExample], categories: &[Category]) -> String {
    let allowed = allowed_category_ids(categories);
    examples
        .iter()
        .filter(|e| allowed.contains(&e.category_id))
        .take(MAX_EXAMPLES)
        .map(|e| {
            let r = redact_block(&e.app_id, &e.app_name, &e.title, None, e.domain.as_deref());
            let mut parts = vec![
                format!("app={}", squash(&r.app_name, 60)),
                format!("title={}", squash(&r.title, MAX_TITLE_CHARS)),
            ];
            if let Some(d) = r
                .domain
                .as_deref()
                .map(|d| squash(d, 80))
                .filter(|d| !d.is_empty())
            {
                parts.push(format!("domain={d}"));
            }
            format!("- {} → category_id={}", parts.join(" | "), e.category_id)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The volatile part of a classification request: `<examples>` then `<blocks>`.
pub fn classification_user_message(
    blocks: &[ActivityBlock],
    ctx: &ClassificationContext,
) -> String {
    let mut s = String::with_capacity(256 * blocks.len() + 512);
    let examples = render_examples(&ctx.examples, &ctx.categories);
    if !examples.is_empty() {
        s.push_str(
            "<examples>\nBlocks the user classified by hand earlier (follow these preferences):\n",
        );
        s.push_str(&examples);
        s.push_str("\n</examples>\n\n");
    }
    s.push_str("<blocks>\n");
    for b in blocks {
        s.push_str(&render_block_line(b));
        s.push('\n');
    }
    s.push_str("</blocks>\n\nClassify every block in <blocks>.");
    s
}

// ---------------------------------------------------------------------------------------------
// Vision
// ---------------------------------------------------------------------------------------------

/// System prompt for the single-screenshot classifier.
pub fn vision_system_prompt(ctx: &ClassificationContext) -> String {
    let lang = ui_language(&ctx.language);
    let language = language_name_en(lang);
    let mut s = String::with_capacity(3072);
    s.push_str("You look at one screenshot of the user's desktop, describe the task or topic it shows, and classify it into one of the user's categories.\n\n");
    s.push_str("## Categories\n");
    s.push_str(&render_catalogue(&ctx.categories, lang));
    s.push_str("\n\n## Rules\n");
    s.push_str("- Describe only the task or topic (e.g. \"editing a spreadsheet about scholarship budgets\"). Never mention names of people, message or e-mail contents, addresses, numbers, credentials or anything personal, and never transcribe text from the screen.\n");
    s.push_str(
        "- `category_id`: one of the ids above, or null when the screen does not settle it.\n",
    );
    s.push_str("- `confidence`: \"high\" when the screen makes the category obvious, \"medium\" when likely, \"low\" when guessing.\n");
    s.push_str(&format!(
        "- `description`: one sentence in {language}, past tense, topic only.\n"
    ));
    s.push_str("- `evidence`: which kind of on-screen element supports the choice (a document title bar, a code editor, a video player…), never its content.\n");
    s.push_str("- Answer with the JSON object only.\n");
    s.push_str(&profile_section(
        ctx.user_profile.as_deref(),
        "About the user",
    ));
    s
}

/// Text that accompanies the screenshot.
pub fn vision_user_text(block: &ActivityBlock) -> String {
    format!(
        "Block metadata:\n{}\n\nLook at the screenshot and answer with the JSON object only.",
        render_block_line(block)
    )
}

// ---------------------------------------------------------------------------------------------
// Daily report
// ---------------------------------------------------------------------------------------------

/// Stable instructions of the report writer (first system block), written in the language
/// the report must be in, with examples in that language. The JSON schema (field names, `kind`
/// ids) is the same in both languages.
pub fn report_instructions(language: &str) -> String {
    let mut s = String::with_capacity(3072);
    match ui_language(language) {
        UiLanguage::PtBr => {
            s.push_str("Você redige relatórios diários de atividades profissionais a partir de blocos de uso do computador (aplicativo, título da janela, domínio, caminho, duração e horário local), já filtrados para a categoria informada. O texto compõe um relatório institucional mensal e precisa ser preciso, sóbrio e verificável.\n\n");
            s.push_str("## Regras\n");
            s.push_str("- Escreva em português do Brasil, em voz institucional na terceira pessoa e no passado (\"Elaborou o parecer…\", \"Participou de reunião…\", \"Atualizou o sistema…\"), sem pronomes pessoais e sem juízo de valor.\n");
            s.push_str("- Agrupe os blocos em atividades significativas: uma atividade reúne todos os blocos sobre o mesmo assunto, mesmo em aplicativos diferentes. Não liste aplicativos nem blocos individualmente. Prefira nomes de documentos, projetos, sistemas, disciplinas e processos.\n");
            s.push_str(
                "- `minutes`: soma dos blocos da atividade, arredondada para múltiplos de 5.\n",
            );
            s.push_str("- Nunca cite nomes de pessoas, conteúdo de mensagens ou e-mails, nem reproduza títulos de janela entre aspas. O campo `descrição` dos blocos é a fonte principal do tema.\n");
            s.push_str("- Não invente atividades, resultados ou detalhes ausentes dos blocos. Sem blocos: `items` vazio e `highlights` igual a [\"Sem atividade registrada\"].\n");
            s.push_str("- `continuation_of`: quando a atividade continua um item de dias anteriores (lista fornecida), copie exatamente o texto daquele item; caso contrário null.\n");
            s.push_str("- `kind`: desenvolvimento (código, sistemas, infraestrutura), reuniao (videoconferência, agenda), comunicacao (e-mail, mensagens, atendimento), documentacao (documentos, planilhas, pareceres, apresentações), ensino (aulas, materiais, avaliações), pesquisa (leitura, artigos, dados), extensao (projetos com a comunidade, eventos), gestao (SEI, processos, planejamento, orçamento), outro.\n");
            s.push_str("- `evidence`: aplicativos, documentos, domínios, sistemas ou projetos que sustentam o item (nunca pessoas), até 4 por item.\n");
            s.push_str("- `time_range`: \"HH:MM–HH:MM\" do primeiro ao último bloco da atividade; \"\" quando espalhada pelo dia.\n");
            s.push_str(
                "- `highlights`: até 3 frases curtas com o que foi mais relevante no dia.\n",
            );
            s.push_str("- Responda apenas com o objeto JSON.\n");
        }
        UiLanguage::En => {
            s.push_str("You write daily reports of professional activity from blocks of computer usage (application, window title, domain, path, duration and local time), already filtered to the given category. The text feeds a monthly institutional report and must be precise, sober and verifiable.\n\n");
            s.push_str("## Rules\n");
            s.push_str("- Write in English, in an institutional third-person voice and in the past tense (\"Drafted the opinion…\", \"Attended the planning meeting…\", \"Updated the system…\"), with no personal pronouns and no value judgements.\n");
            s.push_str("- Group the blocks into meaningful activities: one activity gathers every block about the same subject, even across different applications. Do not list applications or individual blocks. Prefer the names of documents, projects, systems, courses and processes.\n");
            s.push_str(
                "- `minutes`: the sum of the activity's blocks, rounded to a multiple of 5.\n",
            );
            s.push_str("- Never name people, quote message or e-mail contents, or reproduce window titles in quotation marks. The blocks' `descrição` field is the main source for the topic.\n");
            s.push_str("- Do not invent activities, outcomes or details absent from the blocks. With no blocks: `items` empty and `highlights` equal to [\"No activity recorded\"].\n");
            s.push_str("- `continuation_of`: when the activity continues an item from previous days (list provided), copy that item's text exactly; otherwise null.\n");
            s.push_str("- `kind`: desenvolvimento (code, systems, infrastructure), reuniao (video calls, calendar), comunicacao (e-mail, messaging, support), documentacao (documents, spreadsheets, opinions, slides), ensino (classes, course material, grading), pesquisa (reading, papers, data), extensao (community projects, events), gestao (administrative systems, processes, planning, budget), outro. Keep these ids exactly as written.\n");
            s.push_str("- `evidence`: applications, documents, domains, systems or projects that support the item (never people), up to 4 per item.\n");
            s.push_str("- `time_range`: \"HH:MM–HH:MM\" from the first to the last block of the activity; \"\" when it is spread across the day.\n");
            s.push_str(
                "- `highlights`: up to 3 short sentences with what mattered most that day.\n",
            );
            s.push_str("- Answer with the JSON object only.\n");
        }
    }
    s
}

/// Per-user/per-category context of the report writer (second system block, cached on Sonnet):
/// profile, category and the institution's report template.
pub fn report_context(req: &ReportRequest) -> String {
    let lang = ui_language(&req.language);
    let (profile, category, name, description, keywords, template) = match lang {
        UiLanguage::PtBr => (
            "Perfil do usuário",
            "Categoria",
            "Nome",
            "Descrição",
            "Palavras-chave",
            "Modelo de relatório da instituição",
        ),
        UiLanguage::En => (
            "About the user",
            "Category",
            "Name",
            "Description",
            "Keywords",
            "The institution's report template",
        ),
    };
    let mut s = String::with_capacity(2048);
    s.push_str(&profile_section(req.user_profile.as_deref(), profile));
    s.push_str(&format!("\n## {category}\n"));
    s.push_str(&format!("{name}: {}\n", squash(&req.category.name, 60)));
    let desc = squash(&req.category.description, 600);
    if !desc.is_empty() {
        s.push_str(&format!("{description}: {desc}\n"));
    }
    let kw: Vec<String> = req
        .category
        .keywords
        .iter()
        .map(|k| squash(k, 40))
        .filter(|k| !k.is_empty())
        .take(20)
        .collect();
    if !kw.is_empty() {
        s.push_str(&format!("{keywords}: {}\n", kw.join(", ")));
    }
    if let Some(t) = req
        .category
        .report_template
        .as_deref()
        .map(|t| redact_text(t.trim()))
        .filter(|t| !t.is_empty())
    {
        let tpl: String = t.chars().take(MAX_TEMPLATE_CHARS).collect();
        s.push_str(&format!("\n## {template}\n"));
        s.push_str(tpl.trim());
        s.push('\n');
    }
    s
}

fn render_previous_item(item: &ReportItem, lang: UiLanguage) -> String {
    let mut line = format!(
        "- [{}] {} ({})",
        item.kind.as_str(),
        squash(&item.activity, 200),
        format_minutes(item.minutes)
    );
    if let Some(c) = item
        .continuation_of
        .as_deref()
        .map(|c| squash(c, 200))
        .filter(|c| !c.is_empty())
    {
        line.push_str(&format!(
            " — {}: {c}",
            lang.pick("continuação de", "continuation of")
        ));
    }
    line
}

/// The day: date, previous items, redacted block lines with local times, total time.
pub fn report_user_message(req: &ReportRequest) -> String {
    let lang = ui_language(&req.language);
    let mut s = String::with_capacity(256 * req.blocks.len() + 1024);
    s.push_str(&format!(
        "{}: {}, {} ({})\n",
        lang.pick("Data", "Date"),
        weekday_name(req.date.weekday(), lang),
        ubiqx_core::report::format_date(req.date, lang),
        format_offset(req.utc_offset_secs)
    ));
    let total_secs: i64 = req.blocks.iter().map(ActivityBlock::duration_secs).sum();
    s.push_str(&format!(
        "{}: {}\n\n",
        lang.pick("Tempo total registrado", "Total time recorded"),
        format_minutes(secs_to_minutes(total_secs))
    ));

    if !req.previous_items.is_empty() {
        match lang {
            UiLanguage::PtBr => s.push_str("<itens_anteriores>\nItens dos dias anteriores nesta categoria (use em continuation_of quando a atividade continuar):\n"),
            UiLanguage::En => s.push_str("<previous_items>\nItems from previous days in this category (use them in continuation_of when the activity continues):\n"),
        }
        for item in req.previous_items.iter().take(MAX_PREVIOUS_ITEMS) {
            s.push_str(&render_previous_item(item, lang));
            s.push('\n');
        }
        s.push_str(lang.pick("</itens_anteriores>\n\n", "</previous_items>\n\n"));
    }

    s.push_str(lang.pick("<blocos>\n", "<blocks>\n"));
    if req.blocks.is_empty() {
        s.push_str(lang.pick("(nenhum bloco registrado)\n", "(no blocks recorded)\n"));
    }
    for b in &req.blocks {
        s.push_str(&PromptBlock::from_block(b, req.utc_offset_secs).render_report_line());
        s.push('\n');
    }
    s.push_str(lang.pick(
        "</blocos>\n\nRedija o relatório do dia em JSON.",
        "</blocks>\n\nWrite the day's report as JSON.",
    ));
    s
}

// ---------------------------------------------------------------------------------------------
// Advisor
// ---------------------------------------------------------------------------------------------

/// System prompt of the advisor (stable, cached on Sonnet), written in the language of the
/// answer.
pub fn advisor_system_prompt(req: &AdviceRequest) -> String {
    let mut s = String::with_capacity(2048);
    match ui_language(&req.language) {
        UiLanguage::PtBr => {
            s.push_str("Você é o UBI, o assistente de produtividade do ubiqX. A partir de números agregados sobre o uso do computador na última semana, você escreve um título curto e recomendações práticas.\n\n");
            s.push_str("## Regras\n");
            s.push_str("- Escreva em português do Brasil, com tom gentil, direto e específico.\n");
            s.push_str("- `headline`: uma frase curta que resume a semana (por exemplo, \"Semana de foco excelente\").\n");
            s.push_str("- `recommendations`: no máximo 5, cada uma com uma única ação concreta em uma frase, baseada nos números fornecidos (trocas de contexto, tempo de foco, distrações, categorias, aplicativos, alertas). Nada genérico.\n");
            s.push_str("- Não invente dados, não mencione pessoas e não faça juízo moral. Se os números forem bons, reconheça e sugira como manter.\n");
            s.push_str("- Responda apenas com o objeto JSON.\n");
            s.push_str(&profile_section(
                req.user_profile.as_deref(),
                "Sobre o usuário",
            ));
        }
        UiLanguage::En => {
            s.push_str("You are UBI, the productivity assistant in ubiqX. From aggregated numbers about the past week's computer usage, you write a short headline and practical recommendations.\n\n");
            s.push_str("## Rules\n");
            s.push_str("- Write in English, in a kind, direct and specific tone.\n");
            s.push_str("- `headline`: one short sentence that sums up the week (for example, \"A week of excellent focus\").\n");
            s.push_str("- `recommendations`: at most 5, each a single concrete action in one sentence, grounded in the numbers provided (context switches, focus time, distractions, categories, applications, alerts). Nothing generic.\n");
            s.push_str("- Do not invent data, do not mention people and do not moralise. When the numbers are good, say so and suggest how to keep it up.\n");
            s.push_str("- Answer with the JSON object only.\n");
            s.push_str(&profile_section(
                req.user_profile.as_deref(),
                "About the user",
            ));
        }
    }
    s
}

/// The aggregated numbers (never block titles).
pub fn advisor_user_message(req: &AdviceRequest) -> String {
    let lang = ui_language(&req.language);
    let st = &req.stats;
    let mut s = String::with_capacity(1024);
    s.push_str(lang.pick(
        "Período: última semana (números agregados)\n",
        "Period: past week (aggregated numbers)\n",
    ));
    s.push_str(&format!(
        "{}: {}/100 ({}: {})\n",
        lang.pick("Score de foco", "Focus score"),
        st.focus_score,
        lang.pick("humor", "mood"),
        mood_label(st.mood)
    ));
    s.push_str(&format!(
        "{}: {} | {}: {} | {}: {} | {}: {} | {}: {}\n",
        lang.pick("Tempo produtivo", "Productive time"),
        format_minutes(secs_to_minutes(st.productive_secs)),
        lang.pick("distrações", "distractions"),
        format_minutes(secs_to_minutes(st.distraction_secs)),
        lang.pick("sem categoria", "uncategorized"),
        format_minutes(secs_to_minutes(st.uncategorized_secs)),
        lang.pick("ocioso", "idle"),
        format_minutes(secs_to_minutes(st.idle_secs)),
        lang.pick("total", "total"),
        format_minutes(secs_to_minutes(st.total_secs))
    ));
    s.push_str(&format!(
        "{}: {:.1} | {}: {}\n",
        lang.pick("Trocas de contexto por hora", "Context switches per hour"),
        st.switches_per_hour,
        lang.pick("maior sequência de foco", "longest focus streak"),
        format_minutes(secs_to_minutes(st.longest_focus_secs))
    ));
    if !req.category_totals.is_empty() {
        s.push_str(lang.pick("Tempo por categoria:\n", "Time per category:\n"));
        for (name, secs) in req.category_totals.iter().take(20) {
            s.push_str(&format!(
                "- {}: {}\n",
                squash(name, 60),
                format_minutes(secs_to_minutes(*secs))
            ));
        }
    }
    if !req.top_apps.is_empty() {
        s.push_str(lang.pick("Aplicativos mais usados:\n", "Most used applications:\n"));
        for app in req.top_apps.iter().take(10) {
            s.push_str(&format!(
                "- {}: {}\n",
                squash(&app.app_name, 60),
                format_minutes(secs_to_minutes(app.secs))
            ));
        }
    }
    if !req.recent_nudges.is_empty() {
        let kinds: Vec<&str> = req.recent_nudges.iter().map(NudgeKind::as_str).collect();
        s.push_str(&format!(
            "{}: {}\n",
            lang.pick("Alertas recentes", "Recent alerts"),
            kinds.join(", ")
        ));
    }
    s.push_str(lang.pick(
        "\nEscreva o título e as recomendações em JSON.",
        "\nWrite the headline and the recommendations as JSON.",
    ));
    s
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use chrono::TimeZone;
    use ubiqx_core::AiModels;

    pub(crate) fn block(
        id: &str,
        app_id: &str,
        app: &str,
        title: &str,
        url: Option<&str>,
    ) -> ActivityBlock {
        let started_at = Utc.with_ymd_and_hms(2026, 9, 17, 13, 10, 0).unwrap();
        ActivityBlock {
            id: id.into(),
            started_at,
            ended_at: started_at + chrono::Duration::minutes(25),
            app_name: app.into(),
            app_id: app_id.into(),
            title: title.into(),
            title_key: title.to_lowercase(),
            url: url.map(str::to_string),
            domain: url.and_then(ubiqx_core::normalize::domain_of),
            category_id: None,
            confidence: 0.0,
            source: None,
            description: None,
            screenshot_id: None,
            sample_count: 5,
            is_open: false,
            classify_attempts: 0,
            next_attempt_at: None,
            needs_review: false,
            ai_payload: None,
            ai_sent_at: None,
            is_manual: false,
            note: None,
        }
    }

    pub(crate) fn category(id: &str, name: &str) -> Category {
        Category {
            id: id.into(),
            name: name.into(),
            color: "#000".into(),
            icon: "x".into(),
            description: format!("Tudo sobre {name}"),
            keywords: vec![name.to_lowercase(), "edital".into()],
            report_time: None,
            report_template: None,
            is_productive: true,
            is_system: false,
            archived: false,
            sort_order: 0,
            created_at: Utc::now(),
        }
    }

    pub(crate) fn ctx(categories: Vec<Category>) -> ClassificationContext {
        ClassificationContext {
            categories,
            rules: vec![],
            examples: vec![],
            user_classified: vec![],
            language: "pt-BR".into(),
            min_confidence: 0.6,
            models: AiModels::default(),
            user_profile: Some("Servidor do IFRO, coordena a incubadora".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{block, category, ctx};
    use super::*;
    use ubiqx_core::ClassificationSource;

    #[test]
    fn block_line_is_redacted_and_deterministic() {
        let b = block(
            "b1",
            "com.google.Chrome",
            "Google Chrome",
            "Re: reunião com joao.silva@ifro.edu.br (69) 99999-1234",
            Some("https://docs.google.com/document/d/abc/edit?usp=sharing&token=SECRET#heading"),
        );
        let line = render_block_line_with_offset(&b, -4 * 3600);
        assert_eq!(
            line,
            "- id=b1 | app=Google Chrome | title=Re: reunião com [email] [telefone] | domain=docs.google.com | path=/document/d/abc/edit | min=25 | time=09:10–09:35"
        );
        assert!(!line.contains("joao.silva"));
        assert!(!line.contains("SECRET"));
        assert!(!line.contains("usp="));
        // The convenience variant uses the same renderer (only the zone differs).
        let local = render_block_line(&b);
        assert!(local.starts_with("- id=b1 | app=Google Chrome | title=Re: reunião com [email] [telefone] | domain=docs.google.com | path=/document/d/abc/edit | min=25 | time="));
    }

    #[test]
    fn messaging_titles_are_reduced_to_app_name() {
        let b = block(
            "b2",
            "net.whatsapp.WhatsApp",
            "WhatsApp",
            "Maria Souza",
            None,
        );
        let line = render_block_line_with_offset(&b, 0);
        assert_eq!(
            line,
            "- id=b2 | app=WhatsApp | title=WhatsApp | min=25 | time=13:10–13:35"
        );
        assert!(!line.contains("Maria"));
    }

    #[test]
    fn prompt_block_squashes_and_truncates() {
        let long = "x".repeat(300);
        let b = block(
            "b3",
            "app",
            "App",
            &format!("a |  b\n{long}"),
            Some("https://example.com/"),
        );
        let pb = PromptBlock::from_block(&b, 0);
        assert!(pb.title().starts_with("a / b x"));
        assert_eq!(pb.title().chars().count(), MAX_TITLE_CHARS);
        assert!(pb.title().ends_with('…'));
        assert_eq!(pb.path(), None);
        assert_eq!(pb.domain(), Some("example.com"));
        let empty = block("b4", "app", "App", "   ", None);
        assert_eq!(PromptBlock::from_block(&empty, 0).title(), "(sem título)");
    }

    #[test]
    fn classification_prompts_contain_catalogue_examples_and_blocks() {
        let mut c = ctx(vec![
            category("cat-ifro", "IFRO"),
            category("cat-inc", "Incubadora"),
        ]);
        c.categories[1].archived = true;
        c.examples = (0..15)
            .map(|i| ClassificationExample {
                app_id: "com.google.Chrome".into(),
                app_name: "Google Chrome".into(),
                title: format!("edital {i} contato pessoa{i}@ifro.edu.br"),
                domain: Some("sei.ifro.edu.br".into()),
                category_id: if i == 0 {
                    "stale-id".into()
                } else {
                    "cat-ifro".into()
                },
            })
            .collect();
        let system = classification_system_prompt(&c);
        assert!(system.contains("- id=cat-ifro | nome=IFRO | produtiva=sim | descrição=Tudo sobre IFRO | palavras-chave=ifro, edital"));
        assert!(
            !system.contains("cat-inc"),
            "archived categories are hidden"
        );
        assert!(system.contains("id=sys-distraction"));
        assert!(system.contains("id=sys-break"));
        assert!(system.contains("one short sentence in Brazilian Portuguese"));
        assert!(system.contains("\"Editou a planilha de orçamento da incubadora\""));
        assert!(system.contains("coordena a incubadora"));
        assert!(
            system.len() < 6000,
            "system prompt must stay lean: {}",
            system.len()
        );

        let blocks = vec![block(
            "b1",
            "com.google.Chrome",
            "Google Chrome",
            "Edital 12/2026 - SEI",
            Some("https://sei.ifro.edu.br/sei/x?y=1"),
        )];
        let user = classification_user_message(&blocks, &c);
        assert!(user.starts_with("<examples>"));
        assert_eq!(user.matches("→ category_id=cat-ifro").count(), MAX_EXAMPLES);
        assert!(!user.contains("stale-id"));
        assert!(!user.contains("@ifro.edu.br"), "{user}");
        assert!(user.contains("[email]"));
        assert!(user.contains("<blocks>\n- id=b1 | app=Google Chrome | title=Edital 12/2026 - SEI | domain=sei.ifro.edu.br | path=/sei/x | min=25 | time="));
        assert!(user.ends_with("</blocks>\n\nClassify every block in <blocks>."));
        assert!(!user.contains("y=1"));
    }

    #[test]
    fn classification_prompt_in_english() {
        let mut c = ctx(vec![category("cat-ifro", "IFRO")]);
        c.language = "en-US".into();
        let system = classification_system_prompt(&c);
        assert!(system.contains("- id=cat-ifro | name=IFRO | productive=yes | description=Tudo sobre IFRO | keywords=ifro, edital"), "{system}");
        assert!(
            system.contains("| name=Distraction | productive=no |"),
            "{system}"
        );
        assert!(
            system.contains("| name=Break | productive=no |"),
            "{system}"
        );
        assert!(system.contains("one short sentence in English"), "{system}");
        assert!(system.contains("\"Edited the incubator's budget spreadsheet\""));
        assert!(!system.contains("produtiva="), "{system}");
        let vision = vision_system_prompt(&c);
        assert!(vision.contains("one sentence in English"), "{vision}");
        assert!(vision.contains("name=Break"), "{vision}");
        c.language = String::new();
        assert!(vision_system_prompt(&c).contains("one sentence in Brazilian Portuguese"));
    }

    #[test]
    fn allowed_ids_exclude_uncategorized_and_private() {
        let ids = allowed_category_ids(&[
            category("a", "A"),
            category(system_categories::PRIVATE, "P"),
        ]);
        assert!(ids.contains("a"));
        assert!(ids.contains(system_categories::DISTRACTION));
        assert!(ids.contains(system_categories::BREAK));
        assert!(!ids.contains(system_categories::PRIVATE));
        assert!(!ids.contains(system_categories::UNCATEGORIZED));
    }

    #[test]
    fn report_prompts() {
        let mut b = block(
            "b1",
            "com.google.Chrome",
            "Google Chrome",
            "Parecer edital 12/2026 - ana@ifro.edu.br",
            Some("https://sei.ifro.edu.br/sei/controlador.php?acao=1"),
        );
        b.description = Some("Revisou o parecer do edital".into());
        b.category_id = Some("cat-ifro".into());
        b.source = Some(ClassificationSource::Llm);
        let mut category = category("cat-ifro", "IFRO");
        category.report_template = Some("Use seções por projeto.".into());
        let req = ReportRequest {
            date: chrono::NaiveDate::from_ymd_opt(2026, 9, 17).unwrap(),
            category,
            blocks: vec![b],
            previous_items: vec![ReportItem {
                activity: "Iniciou o parecer do edital 12/2026".into(),
                kind: ubiqx_core::ActivityKind::Documentacao,
                minutes: 40,
                evidence: vec![],
                time_range: String::new(),
                continuation_of: None,
            }],
            language: String::new(),
            utc_offset_secs: -4 * 3600,
            user_profile: Some("Coordenador".into()),
            model: "claude-sonnet-5".into(),
        };
        let instructions = report_instructions(&req.language);
        assert!(instructions.contains("- Escreva em português do Brasil,"));
        assert!(instructions.contains("\"Elaborou o parecer…\""));
        assert!(instructions.contains("[\"Sem atividade registrada\"]"));
        let context = report_context(&req);
        assert!(context.contains("## Perfil do usuário\nCoordenador"));
        assert!(context.contains("Nome: IFRO"));
        assert!(context.contains("## Modelo de relatório da instituição\nUse seções por projeto."));
        let user = report_user_message(&req);
        assert!(
            user.starts_with(
                "Data: quinta-feira, 17/09/2026 (UTC-04:00)\nTempo total registrado: 25 min\n"
            ),
            "{user}"
        );
        assert!(user.contains("- [documentacao] Iniciou o parecer do edital 12/2026 (40 min)"));
        assert!(user.contains("- 09:10–09:35 | 25 min | app=Google Chrome | title=Parecer edital 12/2026 - [email] | domain=sei.ifro.edu.br | path=/sei/controlador.php | descrição=Revisou o parecer do edital"), "{user}");
        assert!(!user.contains("ana@"));
        assert!(!user.contains("acao=1"));

        // The same request in English: instructions, context and message change language, the
        // schema vocabulary (`kind` ids, field names) and the redacted payload do not.
        let mut en = req.clone();
        en.language = "en".into();
        let instructions = report_instructions(&en.language);
        assert!(
            instructions.contains("- Write in English,"),
            "{instructions}"
        );
        assert!(instructions.contains("\"Drafted the opinion…\""));
        assert!(instructions.contains("[\"No activity recorded\"]"));
        assert!(instructions
            .contains("`kind`: desenvolvimento (code, systems, infrastructure), reuniao"));
        assert!(!instructions.contains("Escreva"));
        let context = report_context(&en);
        assert!(
            context.contains("## About the user\nCoordenador"),
            "{context}"
        );
        assert!(context.contains("## Category\nName: IFRO"), "{context}");
        assert!(context.contains("## The institution's report template\nUse seções por projeto."));
        let user = report_user_message(&en);
        assert!(
            user.starts_with(
                "Date: Thursday, 2026-09-17 (UTC-04:00)\nTotal time recorded: 25 min\n"
            ),
            "{user}"
        );
        assert!(user.contains("<previous_items>\n"), "{user}");
        assert!(user.contains("- [documentacao] Iniciou o parecer do edital 12/2026 (40 min)"));
        assert!(user.contains("<blocks>\n- 09:10–09:35 | 25 min | app=Google Chrome | title=Parecer edital 12/2026 - [email] | domain=sei.ifro.edu.br | path=/sei/controlador.php | descrição=Revisou o parecer do edital"), "{user}");
        assert!(
            user.ends_with("</blocks>\n\nWrite the day's report as JSON."),
            "{user}"
        );
    }

    #[test]
    fn advisor_prompts_only_carry_numbers() {
        let req = AdviceRequest {
            language: "pt-BR".into(),
            stats: ubiqx_core::FocusStats {
                focus_score: 72,
                productive_secs: 18 * 3600 + 20 * 60,
                distraction_secs: 2 * 3600,
                uncategorized_secs: 3600,
                idle_secs: 0,
                total_secs: 21 * 3600 + 20 * 60,
                switches_per_hour: 14.26,
                longest_focus_secs: 95 * 60,
                mood: Mood::Focused,
            },
            category_totals: vec![("IFRO".into(), 10 * 3600)],
            top_apps: vec![ubiqx_core::AppTotal {
                app_id: "com.google.Chrome".into(),
                app_name: "Google Chrome".into(),
                secs: 8 * 3600,
            }],
            recent_nudges: vec![NudgeKind::Distracted],
            user_profile: None,
            model: "claude-sonnet-5".into(),
        };
        let system = advisor_system_prompt(&req);
        assert!(system.contains("- Escreva em português do Brasil,"));
        assert!(system.contains("no máximo 5"));
        let user = advisor_user_message(&req);
        assert!(user.contains("Score de foco: 72/100 (humor: focused)"));
        assert!(user.contains("Tempo produtivo: 18 h 20 min | distrações: 2 h | sem categoria: 1 h | ocioso: 0 min | total: 21 h 20 min"));
        assert!(user
            .contains("Trocas de contexto por hora: 14.3 | maior sequência de foco: 1 h 35 min"));
        assert!(user.contains("- IFRO: 10 h"));
        assert!(user.contains("- Google Chrome: 8 h"));
        assert!(user.contains("Alertas recentes: distracted"));

        let mut en = req.clone();
        en.language = "en-GB".into();
        let system = advisor_system_prompt(&en);
        assert!(system.contains("- Write in English,"), "{system}");
        assert!(system.contains("at most 5"));
        assert!(!system.contains("Escreva"));
        let user = advisor_user_message(&en);
        assert!(
            user.starts_with("Period: past week (aggregated numbers)\n"),
            "{user}"
        );
        assert!(user.contains("Focus score: 72/100 (mood: focused)"));
        assert!(user.contains("Productive time: 18 h 20 min | distractions: 2 h | uncategorized: 1 h | idle: 0 min | total: 21 h 20 min"), "{user}");
        assert!(user.contains("Context switches per hour: 14.3 | longest focus streak: 1 h 35 min"));
        assert!(user.contains("Time per category:\n- IFRO: 10 h"));
        assert!(user.contains("Most used applications:\n- Google Chrome: 8 h"));
        assert!(user.contains("Recent alerts: distracted"));
        assert!(user.ends_with("\nWrite the headline and the recommendations as JSON."));
    }

    #[test]
    fn examples_from_messaging_apps_are_redacted_by_bundle_id() {
        // The macOS adapter reports localized names ("Mensagens" for Messages on pt-BR), so
        // the redaction must key on the bundle id carried by the example, not on the name.
        let cats = vec![category("cat-ifro", "IFRO")];
        let examples = vec![ClassificationExample {
            app_id: "com.apple.MobileSMS".into(),
            app_name: "Mensagens".into(),
            title: "maria souza".into(),
            domain: None,
            category_id: "cat-ifro".into(),
        }];
        let rendered = render_examples(&examples, &cats);
        assert_eq!(
            rendered,
            "- app=Mensagens | title=Mensagens → category_id=cat-ifro"
        );
        assert!(!rendered.contains("maria"), "{rendered}");
    }

    #[test]
    fn helpers() {
        assert_eq!(language_or_default(""), "pt-BR");
        assert_eq!(language_or_default(" en "), "en");
        assert_eq!(ui_language(""), UiLanguage::PtBr);
        assert_eq!(ui_language("en-US"), UiLanguage::En);
        assert_eq!(ui_language("pt_BR"), UiLanguage::PtBr);
        assert_eq!(format_offset(-4 * 3600), "UTC-04:00");
        assert_eq!(format_offset(5 * 3600 + 1800), "UTC+05:30");
        assert_eq!(path_of("https://a.b/c/d/"), Some("/c/d".into()));
        assert_eq!(path_of("https://a.b"), None);
        assert_eq!(path_of("https://a.b/"), None);
        assert_eq!(squash("  a   b\tc ", 10), "a b c");
        assert_eq!(squash("abcdef", 3), "ab…");
    }
}
