//! The classification worker: local classifiers first, then the remote text model in batches,
//! then vision for blocks that need it. Handles retries, poison blocks, budget and health.

use std::sync::Arc;

use std::sync::atomic::Ordering;

use chrono::{DateTime, Duration, Utc};
use ubiqx_core::learning::{select_examples, MemoryClassifier};
use ubiqx_core::ports::*;
use ubiqx_core::rules::RuleClassifier;
use ubiqx_core::scheduler::month_range;
use ubiqx_core::*;

use crate::screenshots;
use crate::state::EngineState;

/// Outcome of one worker pass, for logs and the CLI.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClassifyReport {
    pub local: usize,
    pub remote: usize,
    pub vision: usize,
    pub needs_review: usize,
    pub skipped_remote: bool,
}

/// Backoff after the n-th failed remote attempt.
pub fn backoff_for(attempts: u32) -> Duration {
    match attempts {
        0 | 1 => Duration::minutes(1),
        2 => Duration::minutes(4),
        3 => Duration::minutes(16),
        4 => Duration::hours(1),
        _ => Duration::hours(4),
    }
}

pub const MAX_ATTEMPTS: u32 = 5;

/// Builds the shared classification context from the repositories.
pub fn build_context(
    state: &EngineState,
    blocks: &[ActivityBlock],
) -> CoreResult<ClassificationContext> {
    let repos = &state.deps.repos;
    let settings = state.settings();
    let categories = repos.categories.list(false)?;
    let rules: Vec<Rule> = repos
        .rules
        .list()?
        .into_iter()
        .filter(|r| r.enabled)
        .collect();
    let corrections = repos.corrections.list_recent(300)?;
    let examples = select_examples(&corrections, blocks, 12);
    let user_classified = repos.blocks.list_user_classified(400)?;
    Ok(ClassificationContext {
        categories,
        rules,
        examples,
        user_classified,
        language: settings.language.clone(),
        min_confidence: settings.min_confidence,
        models: settings.models.clone(),
        user_profile: settings.user_profile.clone(),
    })
}

/// Runs the local chain on one block: user rules → memory → learned rules.
pub fn classify_locally(
    block: &ActivityBlock,
    ctx: &ClassificationContext,
) -> Option<Classification> {
    let chain: [&dyn LocalClassifier; 3] = [
        &RuleClassifier::user_only(),
        &MemoryClassifier,
        &RuleClassifier::learned_only(),
    ];
    for c in chain {
        if let Some(cl) = c.classify(block, ctx) {
            if cl.is_confident(ctx.min_confidence) {
                return Some(cl);
            }
        }
    }
    None
}

/// Lifts a pause that this budget check itself imposed. A `Paused` caused by a rejected key
/// is left alone: only a new key clears that one.
fn clear_budget_pause(state: &EngineState) {
    if state.budget_paused.swap(false, Ordering::SeqCst) {
        state.set_ai_health(AiHealth::Ok);
    }
}

/// Checks the monthly (local calendar) budget; updates health and returns whether remote
/// calls may proceed. It is the single owner of budget pauses, so it must run before the
/// health gate: a month rollover or a raised budget resumes the AI on the next call.
///
/// With the managed provider the plan's budget is the proxy's, not the user's setting: the
/// usage last reported by the proxy decides (see [`crate::license::managed_budget_exhausted`]),
/// and a `402` from the proxy refreshes it ([`run_once`]).
pub fn budget_allows(state: &EngineState) -> CoreResult<bool> {
    let settings = state.settings();
    if settings.ai_provider.is_managed() {
        let usage = state.license.read().managed_usage.clone();
        return Ok(match usage {
            Some(u) if crate::license::managed_budget_exhausted(&u, state.now()) => {
                state.budget_paused.store(true, Ordering::SeqCst);
                state.set_ai_health(AiHealth::Paused {
                    reason: crate::license::managed_budget_reached_text(&u, settings.ui_language()),
                });
                false
            }
            _ => {
                clear_budget_pause(state);
                true
            }
        });
    }
    if settings.ai_monthly_budget_usd <= 0.0 {
        clear_budget_pause(state);
        return Ok(true);
    }
    let totals = state.deps.repos.usage.totals(month_range(state.now()))?;
    let ratio = totals.cost_usd / settings.ai_monthly_budget_usd;
    let lang = settings.ui_language();
    if ratio >= 1.0 {
        state.budget_paused.store(true, Ordering::SeqCst);
        state.set_ai_health(AiHealth::Paused {
            reason: budget_reached_text(totals.cost_usd, settings.ai_monthly_budget_usd, lang),
        });
        return Ok(false);
    }
    clear_budget_pause(state);
    if ratio >= 0.8 {
        let key = format!("budget_warned_{}", state.now().format("%Y-%m"));
        if state.deps.repos.kv.get(&key)?.is_none() {
            state.deps.repos.kv.set(&key, "1")?;
            let (title, body) = budget_warning_text(ratio, totals.cost_usd, lang);
            crate::nudges::emit_attention(state, &title, &body);
        }
    }
    Ok(true)
}

/// `AiHealth::Paused` reason once the monthly budget is spent.
pub fn budget_reached_text(spent_usd: f64, budget_usd: f64, lang: UiLanguage) -> String {
    match lang {
        UiLanguage::PtBr => {
            format!("Orçamento mensal de IA atingido (US$ {spent_usd:.2} de US$ {budget_usd:.2}).")
        }
        UiLanguage::En => {
            format!("Monthly AI budget reached (${spent_usd:.2} of ${budget_usd:.2}).")
        }
    }
}

/// Attention nudge at 80% of the monthly budget.
pub fn budget_warning_text(ratio: f64, spent_usd: f64, lang: UiLanguage) -> (String, String) {
    let pct = ratio * 100.0;
    match lang {
        UiLanguage::PtBr => (
            "Orçamento de IA quase no limite".into(),
            format!("Você já usou {pct:.0}% do orçamento mensal de IA (US$ {spent_usd:.2})."),
        ),
        UiLanguage::En => (
            "AI budget almost used up".into(),
            format!("You've used {pct:.0}% of this month's AI budget (${spent_usd:.2})."),
        ),
    }
}

/// Health reasons and attention nudges of the remote-failure path, in the UI language.
struct FailureText {
    rate_limited: &'static str,
    key_rejected_reason: &'static str,
    ai_paused: &'static str,
    key_rejected_body: &'static str,
    account_rejected_body: &'static str,
    unavailable_prefix: &'static str,
}

impl FailureText {
    fn for_language(lang: UiLanguage) -> Self {
        match lang {
            UiLanguage::PtBr => Self {
                rate_limited: "limite de requisições da API",
                key_rejected_reason: "Chave de API inválida ou sem permissão.",
                ai_paused: "IA pausada",
                key_rejected_body: "A chave de API foi recusada. Verifique em Configurações → IA.",
                account_rejected_body: "O provedor recusou a conta ou o modelo. Verifique créditos e modelo em Configurações → IA.",
                unavailable_prefix: "IA indisponível",
            },
            UiLanguage::En => Self {
                rate_limited: "API rate limit",
                key_rejected_reason: "API key is invalid or lacks permission.",
                ai_paused: "AI paused",
                key_rejected_body: "The API key was rejected. Check it under Settings → AI.",
                account_rejected_body: "The provider rejected the account or the model. Check credits and model under Settings → AI.",
                unavailable_prefix: "AI unavailable",
            },
        }
    }

    /// The same texts when the managed provider answers: its credential is the license.
    fn for_managed(lang: UiLanguage) -> Self {
        match lang {
            UiLanguage::PtBr => Self {
                key_rejected_reason: "Licença recusada pela IA do Ubi.",
                key_rejected_body:
                    "A IA do Ubi recusou a licença. Verifique em Configurações → Licença.",
                account_rejected_body:
                    "A IA do Ubi recusou a chamada. Verifique em Configurações → Licença.",
                ..Self::for_language(lang)
            },
            UiLanguage::En => Self {
                key_rejected_reason: "License rejected by Ubi's AI.",
                key_rejected_body:
                    "Ubi's AI rejected the license. Check it under Settings → License.",
                account_rejected_body:
                    "Ubi's AI rejected the call. Check it under Settings → License.",
                ..Self::for_language(lang)
            },
        }
    }
}

/// Maps a remote failure onto health state and per-block backoff.
///
/// Only failures caused by the batch itself (`Invalid`, `AiRefused`) are charged to the
/// blocks. Everything else is infrastructure-level (offline, rate limit, rejected key or
/// account) and is throttled by the health state alone, so a couple of hours offline never
/// pushes blocks into the review queue.
fn handle_remote_failure(state: &EngineState, blocks: &[ActivityBlock], err: &CoreError) {
    let now = state.now();
    let (lang, managed) = {
        let s = state.settings.read();
        (s.ui_language(), s.ai_provider.is_managed())
    };
    let text = if managed {
        FailureText::for_managed(lang)
    } else {
        FailureText::for_language(lang)
    };
    match err {
        CoreError::AiNotConfigured => state.set_ai_health(AiHealth::NotConfigured),
        CoreError::RateLimited { retry_after_secs } => state.set_ai_health(AiHealth::Degraded {
            reason: text.rate_limited.into(),
            until: now + Duration::seconds((*retry_after_secs).max(30) as i64),
        }),
        CoreError::Ai(msg)
            if msg.to_lowercase().contains("api key")
                || msg.contains("401")
                || msg.contains("403") =>
        {
            state.set_ai_health(AiHealth::Paused {
                reason: text.key_rejected_reason.into(),
            });
            crate::nudges::emit_attention(state, text.ai_paused, text.key_rejected_body);
        }
        CoreError::AiRejected(msg) => {
            // Billing/model rejection: not transient and not the blocks' fault. Stays paused
            // until the user saves a key again (`EngineHandle::set_api_key` resets health).
            let already_paused = matches!(state.ai_health(), AiHealth::Paused { .. });
            state.set_ai_health(AiHealth::Paused {
                reason: format!("{}: {msg}", text.unavailable_prefix),
            });
            if !already_paused {
                crate::nudges::emit_attention(state, text.ai_paused, text.account_rejected_body);
            }
        }
        CoreError::AiRefused => {
            // Per-content decision, not an outage: retrying the same payload only bills it
            // again. The batch goes straight to review; AI health is untouched.
            for b in blocks {
                let _ = state
                    .deps
                    .repos
                    .blocks
                    .record_attempt(&b.id, MAX_ATTEMPTS, None, true);
            }
        }
        CoreError::Ai(msg) => state.set_ai_health(AiHealth::Degraded {
            reason: msg.clone(),
            until: now + Duration::minutes(15),
        }),
        CoreError::Invalid(_) => {
            // Our request was rejected: the batch itself is the problem, so back off per block
            // and eventually send it to review.
            for b in blocks {
                let attempts = b.classify_attempts + 1;
                let review = attempts >= MAX_ATTEMPTS;
                let _ = state.deps.repos.blocks.record_attempt(
                    &b.id,
                    attempts,
                    Some(now + backoff_for(attempts)),
                    review,
                );
            }
        }
        _ => {}
    }
}

fn apply(state: &EngineState, cl: &Classification) -> CoreResult<()> {
    state.deps.repos.blocks.set_classification(
        &cl.block_id,
        cl.category_id.as_deref(),
        cl.confidence,
        cl.source,
        cl.description.as_deref(),
    )?;
    if let Some(rule_id) = &cl.rule_id {
        let _ = state.deps.repos.rules.increment_hits(rule_id);
    }
    Ok(())
}

/// One pass of the worker. `force` ignores the batching thresholds (used by the CLI/UI).
pub async fn run_once(state: &Arc<EngineState>, force: bool) -> CoreResult<ClassifyReport> {
    let mut report = ClassifyReport::default();
    let now = state.now();
    let repos = state.deps.repos.clone();
    let pending = repos.blocks.list_pending_remote(now, 200)?;
    if pending.is_empty() {
        return Ok(report);
    }
    let ctx = build_context(state, &pending)?;

    // 1. Local chain.
    let mut remaining: Vec<ActivityBlock> = Vec::new();
    let mut classified_ids = Vec::new();
    for b in pending {
        match classify_locally(&b, &ctx) {
            Some(cl) => {
                apply(state, &cl)?;
                classified_ids.push(b.id.clone());
                report.local += 1;
            }
            None => remaining.push(b),
        }
    }
    if !classified_ids.is_empty() {
        state.deps.sink.emit(EngineEvent::BlocksClassified {
            block_ids: classified_ids.clone(),
        });
        classified_ids.clear();
    }
    if remaining.is_empty() {
        return Ok(report);
    }

    // 2. Remote text model, batched.
    let settings = state.settings();
    let oldest_age = remaining
        .iter()
        .map(|b| now - b.ended_at)
        .max()
        .unwrap_or(Duration::zero());
    let due = force
        || remaining.len() >= settings.classify_batch_min as usize
        || oldest_age >= Duration::seconds(settings.classify_max_wait_secs as i64);
    let Some(remote) = state.deps.ai.remote.clone() else {
        report.skipped_remote = true;
        return Ok(report);
    };
    // The budget check runs before the health gate: it is what lifts a budget pause.
    if !due || !budget_allows(state)? || !state.remote_allowed() {
        report.skipped_remote = true;
        return Ok(report);
    }

    let mut needs_vision: Vec<ActivityBlock> = Vec::new();
    for chunk in remaining.chunks(25) {
        // Record the redacted payload that will be sent, for the transparency view.
        for b in chunk {
            let _ = repos
                .blocks
                .set_ai_payload(&b.id, &remote.describe_payload(b), now);
        }
        match remote.classify_batch(chunk, &ctx).await {
            Ok(results) => {
                state.set_ai_health(AiHealth::Ok);
                let mut answered = std::collections::HashSet::new();
                for cl in &results {
                    answered.insert(cl.block_id.clone());
                    let block = chunk.iter().find(|b| b.id == cl.block_id);
                    let Some(block) = block else { continue };
                    if cl.is_confident(ctx.min_confidence) && !cl.needs_vision {
                        apply(state, cl)?;
                        classified_ids.push(cl.block_id.clone());
                        report.remote += 1;
                    } else {
                        // Keep the description even when the category is uncertain.
                        if let Some(d) = &cl.description {
                            let _ = repos.blocks.set_classification(
                                &cl.block_id,
                                None,
                                cl.confidence,
                                cl.source,
                                Some(d),
                            );
                        }
                        needs_vision.push(block.clone());
                    }
                }
                for b in chunk.iter().filter(|b| !answered.contains(&b.id)) {
                    let attempts = b.classify_attempts + 1;
                    let review = attempts >= MAX_ATTEMPTS;
                    repos.blocks.record_attempt(
                        &b.id,
                        attempts,
                        Some(now + backoff_for(attempts)),
                        review,
                    )?;
                    if review {
                        report.needs_review += 1;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "remote classification failed");
                // The proxy refused for the plan's sake (budget spent, revoked key…): learn
                // the month's usage now, so the budget check owns the pause and lifts it
                // when the month rolls over instead of retrying against a closed door.
                if settings.ai_provider.is_managed() && matches!(e, CoreError::AiRejected(_)) {
                    // Learn the month's usage now. When the refusal was the plan's budget, the
                    // budget check owns the pause and lifts it at the month rollover instead of
                    // retrying against a closed door; a refusal for any other reason (a model the
                    // plan does not offer, a revoked key) leaves the budget gate open, since
                    // pausing on it would hide the real reason behind "budget spent".
                    let status = crate::license::refresh_managed_usage(state).await;
                    let spent = status
                        .managed_usage
                        .as_ref()
                        .is_some_and(|u| crate::license::managed_budget_exhausted(u, state.now()));
                    if spent {
                        state.budget_paused.store(true, Ordering::SeqCst);
                    }
                }
                handle_remote_failure(state, chunk, &e);
                if matches!(e, CoreError::AiRefused) {
                    // Only this batch is affected; the remaining chunks are still worth sending.
                    report.needs_review += chunk.len();
                    continue;
                }
                report.skipped_remote = true;
                return Ok(report);
            }
        }
    }
    if !classified_ids.is_empty() {
        state.deps.sink.emit(EngineEvent::BlocksClassified {
            block_ids: classified_ids.clone(),
        });
        classified_ids.clear();
    }

    // 3. Vision for ambiguous blocks that have a screenshot and are allowed.
    if let Some(vision) = state.deps.ai.vision.clone() {
        let hour_ago = now - Duration::hours(1);
        let mut sent_this_hour = repos.screenshots.count_sent_since(hour_ago)?;
        for b in &needs_vision {
            let allowed = settings.vision_policy.allows(
                &b.app_id,
                &b.app_name,
                &settings.blocked_apps,
                &settings.vision_denied_apps,
            );
            if !allowed || sent_this_hour >= settings.max_vision_per_hour as u64 {
                mark_unresolved(state, b, now)?;
                continue;
            }
            let Some(shot) = repos.screenshots.latest_for_block(&b.id)? else {
                mark_unresolved(state, b, now)?;
                continue;
            };
            let image = match screenshots::load(&shot) {
                Ok(i) => i,
                Err(e) => {
                    tracing::debug!(error = %e, "screenshot unreadable");
                    mark_unresolved(state, b, now)?;
                    continue;
                }
            };
            match vision.classify_with_image(b, &image, &ctx).await {
                Ok(cl) => {
                    repos.screenshots.mark_sent(&shot.id)?;
                    sent_this_hour += 1;
                    if !settings.keep_screenshots_for_review {
                        screenshots::unlink(&shot);
                    }
                    if cl.is_confident(ctx.min_confidence) {
                        apply(state, &cl)?;
                        classified_ids.push(cl.block_id.clone());
                        report.vision += 1;
                    } else {
                        if let Some(d) = &cl.description {
                            let _ = repos.blocks.set_classification(
                                &b.id,
                                None,
                                cl.confidence,
                                cl.source,
                                Some(d),
                            );
                        }
                        repos
                            .blocks
                            .record_attempt(&b.id, MAX_ATTEMPTS, None, true)?;
                        report.needs_review += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "vision classification failed");
                    handle_remote_failure(state, std::slice::from_ref(b), &e);
                    if matches!(e, CoreError::AiRefused) {
                        // Only this screenshot was refused; the other candidates still get theirs.
                        report.needs_review += 1;
                        continue;
                    }
                    break;
                }
            }
        }
    } else {
        for b in &needs_vision {
            mark_unresolved(state, b, now)?;
        }
    }
    if !classified_ids.is_empty() {
        state.deps.sink.emit(EngineEvent::BlocksClassified {
            block_ids: classified_ids,
        });
    }
    Ok(report)
}

/// A block the text model could not decide and vision cannot help with: it goes to review
/// after the usual attempts, but is retried a couple of times first (context may improve).
fn mark_unresolved(state: &EngineState, b: &ActivityBlock, now: DateTime<Utc>) -> CoreResult<()> {
    let attempts = b.classify_attempts + 1;
    let review = attempts >= 2;
    state.deps.repos.blocks.record_attempt(
        &b.id,
        attempts,
        Some(now + backoff_for(attempts + 2)),
        review,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows() {
        assert!(backoff_for(1) < backoff_for(2));
        assert!(backoff_for(4) < backoff_for(5));
        assert_eq!(backoff_for(9), Duration::hours(4));
    }

    #[test]
    fn budget_texts_follow_the_language() {
        assert_eq!(
            budget_reached_text(5.0, 5.0, UiLanguage::PtBr),
            "Orçamento mensal de IA atingido (US$ 5.00 de US$ 5.00)."
        );
        assert_eq!(
            budget_reached_text(5.0, 5.0, UiLanguage::En),
            "Monthly AI budget reached ($5.00 of $5.00)."
        );
        let (title, body) = budget_warning_text(0.85, 4.25, UiLanguage::PtBr);
        assert_eq!(title, "Orçamento de IA quase no limite");
        assert_eq!(
            body,
            "Você já usou 85% do orçamento mensal de IA (US$ 4.25)."
        );
        let (title, body) = budget_warning_text(0.85, 4.25, UiLanguage::En);
        assert_eq!(title, "AI budget almost used up");
        assert_eq!(body, "You've used 85% of this month's AI budget ($4.25).");
    }
}
