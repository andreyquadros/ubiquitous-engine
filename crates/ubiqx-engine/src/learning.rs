//! Corrections are the learning signal: they retrain memory, demote wrong rules and
//! propose new ones.

use std::sync::Arc;

use chrono::{Duration, Local, NaiveDate};
use ubiqx_core::learning::{suggest_rules, SUGGESTION_MIN_SUPPORT};
use ubiqx_core::rules::best_rule;
use ubiqx_core::scheduler::day_range;
use ubiqx_core::*;

use crate::handle::ReclassifyScope;
use crate::state::EngineState;

/// What a correction produced, for the UI to show ("Sempre: X → categoria?").
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CorrectionOutcome {
    pub block_ids: Vec<Id>,
    pub backfilled: u64,
    pub suggestions: Vec<RuleSuggestion>,
    pub auto_rules: Vec<Rule>,
    pub disabled_rules: Vec<Rule>,
}

pub fn reclassify(
    state: &Arc<EngineState>,
    block_id: &str,
    category_id: &str,
    note: Option<String>,
    scope: ReclassifyScope,
) -> CoreResult<CorrectionOutcome> {
    let repos = state.deps.repos.clone();
    let now = state.now();
    let block = repos
        .blocks
        .get(block_id)?
        .ok_or_else(|| CoreError::NotFound(format!("block {block_id}")))?;
    if repos.categories.get(category_id)?.is_none() {
        return Err(CoreError::NotFound(format!("category {category_id}")));
    }
    let from = block.category_id.clone();
    let mut outcome = CorrectionOutcome {
        block_ids: vec![block.id.clone()],
        backfilled: 0,
        suggestions: vec![],
        auto_rules: vec![],
        disabled_rules: vec![],
    };

    // 1. Apply to the block itself.
    repos.blocks.set_classification(
        &block.id,
        Some(category_id),
        1.0,
        ClassificationSource::User,
        note.as_deref(),
    )?;
    repos
        .blocks
        .record_attempt(&block.id, block.classify_attempts, None, false)?;

    // 2. Store the correction.
    if from.as_deref() != Some(category_id) {
        repos.corrections.insert(&Correction {
            id: new_id(),
            block_id: block.id.clone(),
            from_category_id: from.clone(),
            to_category_id: category_id.into(),
            app_id: block.app_id.clone(),
            app_name: block.app_name.clone(),
            title_key: block.title_key.clone(),
            domain: block.domain.clone(),
            note,
            at: now,
        })?;
    }

    // 3. Demote a rule that produced the wrong answer.
    if block.source == Some(ClassificationSource::Rule) {
        let rules = repos.rules.list()?;
        if let Some(rule) = best_rule(&rules, &block) {
            if rule.category_id != category_id {
                repos.rules.record_miss(&rule.id, now)?;
                let mut updated = rule.clone();
                updated.miss_count += 1;
                updated.last_contradicted_at = Some(now);
                if updated.should_auto_disable() {
                    updated.enabled = false;
                    repos.rules.upsert(&updated)?;
                    outcome.disabled_rules.push(updated);
                }
            }
        }
    }

    // 4. Backfill siblings.
    let range = match scope {
        ReclassifyScope::Block => None,
        ReclassifyScope::Day => Some(day_range(
            block.started_at.with_timezone(&Local).date_naive(),
        )),
        ReclassifyScope::Month => Some(TimeRange::new(
            now - Duration::days(30),
            now + Duration::days(1),
        )),
    };
    // A browser block without a domain (URL not resolvable, e.g. Automation not granted yet)
    // has no trustworthy key: spreading it would rewrite every block of the browser.
    let browser_without_domain =
        block.domain.is_none() && state.deps.platform.urls.supports(&block.app_id);
    if let Some(range) = range.filter(|_| !browser_without_domain) {
        outcome.backfilled = repos.blocks.backfill_category(
            &block.app_id,
            block.domain.as_deref(),
            range,
            category_id,
            ClassificationSource::User,
        )?;
    }

    // 5. Suggest / auto-create rules.
    let corrections = repos.corrections.list_recent(200)?;
    let existing = repos.rules.list()?;
    let lang = state.settings.read().ui_language();
    let suggestions = suggest_rules(&corrections, &existing, SUGGESTION_MIN_SUPPORT, lang);
    for s in suggestions {
        if s.auto_apply_safe {
            let rule = Rule {
                id: new_id(),
                category_id: s.category_id.clone(),
                matcher: s.matcher,
                pattern: s.pattern.clone(),
                priority: 0,
                origin: RuleOrigin::Learned,
                enabled: true,
                created_at: now,
                hit_count: 0,
                miss_count: 0,
                last_contradicted_at: None,
            };
            repos.rules.upsert(&rule)?;
            outcome.auto_rules.push(rule);
        } else {
            outcome.suggestions.push(s);
        }
    }
    // Always offer the immediate "sempre" shortcut for this block's own key.
    let own_key = block
        .domain
        .clone()
        .map(|d| (RuleMatcher::Domain, d))
        .unwrap_or((RuleMatcher::App, block.app_id.clone()));
    let covered = existing
        .iter()
        .any(|r| r.enabled && r.matcher == own_key.0 && r.pattern.eq_ignore_ascii_case(&own_key.1))
        || outcome
            .auto_rules
            .iter()
            .any(|r| r.matcher == own_key.0 && r.pattern.eq_ignore_ascii_case(&own_key.1))
        || outcome
            .suggestions
            .iter()
            .any(|s| s.matcher == own_key.0 && s.pattern.eq_ignore_ascii_case(&own_key.1));
    if !covered && !own_key.1.is_empty() {
        outcome.suggestions.push(RuleSuggestion {
            category_id: category_id.into(),
            matcher: own_key.0,
            pattern: own_key.1.clone(),
            support: 1,
            rationale: match lang {
                UiLanguage::PtBr => format!("Sempre classificar {} nesta categoria?", own_key.1),
                UiLanguage::En => format!("Always file {} under this category?", own_key.1),
            },
            auto_apply_safe: false,
        });
    }

    crate::reports::mark_day_stale(state, block.started_at);
    state.deps.sink.emit(EngineEvent::BlocksClassified {
        block_ids: outcome.block_ids.clone(),
    });
    Ok(outcome)
}

/// Creates a rule from a suggestion the user confirmed.
pub fn accept_suggestion(state: &EngineState, s: &RuleSuggestion) -> CoreResult<Rule> {
    ubiqx_core::rules::validate_rule(s.matcher, &s.pattern).map_err(CoreError::Invalid)?;
    let rule = Rule {
        id: new_id(),
        category_id: s.category_id.clone(),
        matcher: s.matcher,
        pattern: s.pattern.clone(),
        priority: 0,
        origin: RuleOrigin::Learned,
        enabled: true,
        created_at: state.now(),
        hit_count: 0,
        miss_count: 0,
        last_contradicted_at: None,
    };
    state.deps.repos.rules.upsert(&rule)?;
    Ok(rule)
}

/// Adds a manual entry (e.g. an in-person meeting) as a user-classified block.
pub fn add_manual_entry(
    state: &EngineState,
    started_at: chrono::DateTime<chrono::Utc>,
    ended_at: chrono::DateTime<chrono::Utc>,
    category_id: &str,
    note: Option<String>,
) -> CoreResult<ActivityBlock> {
    if ended_at <= started_at {
        return Err(CoreError::Invalid("end must be after start".into()));
    }
    if state.deps.repos.categories.get(category_id)?.is_none() {
        return Err(CoreError::NotFound(format!("category {category_id}")));
    }
    let block = ActivityBlock::new_manual(started_at, ended_at, category_id.into(), note);
    state.deps.repos.blocks.insert(&block)?;
    crate::reports::mark_day_stale(state, started_at);
    state.deps.sink.emit(EngineEvent::BlockClosed {
        block: block.clone(),
    });
    Ok(block)
}

/// Splits a block in two at `at`; both halves keep the category.
pub fn split_block(
    state: &EngineState,
    block_id: &str,
    at: chrono::DateTime<chrono::Utc>,
) -> CoreResult<Id> {
    let id = state.deps.repos.blocks.split(block_id, at)?;
    crate::reports::mark_day_stale(state, at);
    Ok(id)
}

pub fn local_date(at: chrono::DateTime<chrono::Utc>) -> NaiveDate {
    at.with_timezone(&Local).date_naive()
}
