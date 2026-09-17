//! The classification worker: local classifiers first, then the remote text model in batches,
//! then vision for blocks that need it. Handles retries, poison blocks, budget and health.

use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use ubiqx_core::learning::{select_examples, MemoryClassifier};
use ubiqx_core::ports::*;
use ubiqx_core::rules::RuleClassifier;
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

fn month_range(now: DateTime<Utc>) -> TimeRange {
    let start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now);
    TimeRange::new(start, now + Duration::seconds(1))
}

/// Checks the monthly budget; updates health and returns whether remote calls may proceed.
pub fn budget_allows(state: &EngineState) -> CoreResult<bool> {
    let settings = state.settings();
    if settings.ai_monthly_budget_usd <= 0.0 {
        return Ok(true);
    }
    let totals = state.deps.repos.usage.totals(month_range(state.now()))?;
    let ratio = totals.cost_usd / settings.ai_monthly_budget_usd;
    if ratio >= 1.0 {
        state.set_ai_health(AiHealth::Paused {
            reason: format!(
                "Orçamento mensal de IA atingido (US$ {:.2} de US$ {:.2}).",
                totals.cost_usd, settings.ai_monthly_budget_usd
            ),
        });
        return Ok(false);
    }
    if ratio >= 0.8 {
        let key = format!("budget_warned_{}", state.now().format("%Y-%m"));
        if state.deps.repos.kv.get(&key)?.is_none() {
            state.deps.repos.kv.set(&key, "1")?;
            crate::nudges::emit_attention(
                state,
                "Orçamento de IA quase no limite",
                &format!(
                    "Você já usou {:.0}% do orçamento mensal de IA (US$ {:.2}).",
                    ratio * 100.0,
                    totals.cost_usd
                ),
            );
        }
    }
    Ok(true)
}

/// Maps a remote failure onto health state and per-block backoff.
fn handle_remote_failure(state: &EngineState, blocks: &[ActivityBlock], err: &CoreError) {
    let now = state.now();
    match err {
        CoreError::AiNotConfigured => state.set_ai_health(AiHealth::NotConfigured),
        CoreError::RateLimited { retry_after_secs } => state.set_ai_health(AiHealth::Degraded {
            reason: "limite de requisições da API".into(),
            until: now + Duration::seconds((*retry_after_secs).max(30) as i64),
        }),
        CoreError::Ai(msg)
            if msg.to_lowercase().contains("api key")
                || msg.contains("401")
                || msg.contains("403") =>
        {
            state.set_ai_health(AiHealth::Paused {
                reason: "Chave de API inválida ou sem permissão.".into(),
            });
            crate::nudges::emit_attention(
                state,
                "IA pausada",
                "A chave de API foi recusada. Verifique em Configurações → IA.",
            );
        }
        CoreError::Ai(msg) => state.set_ai_health(AiHealth::Degraded {
            reason: msg.clone(),
            until: now + Duration::minutes(15),
        }),
        CoreError::Invalid(_) => {
            // Our request was rejected: do not hammer the API with the same batch.
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
            return;
        }
        _ => {}
    }
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
    if !due || !state.remote_allowed() || !budget_allows(state)? {
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
                handle_remote_failure(state, chunk, &e);
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
}
