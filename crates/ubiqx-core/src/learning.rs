//! Learning from corrections.
//!
//! Two complementary mechanisms, both free of API calls:
//!
//! 1. **Memory classifier** — nearest-neighbour lookup over blocks the user classified by hand.
//!    If a new block looks like something the user already labelled, reuse that label.
//! 2. **Rule suggestions** — when several corrections agree (same domain → same category),
//!    propose (or auto-apply) a deterministic rule so the block never reaches the AI again.
//!
//! In addition, [`select_examples`] picks the corrections most similar to the blocks being
//! classified so they can be shown to the LLM as few-shot examples.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::error::CoreResult;
use crate::model::*;
use crate::normalize::{activity_similarity, is_generic_title};
use crate::ports::{Classification, ClassificationContext, ClassificationExample, Classifier};

/// Similarity above which a user-labelled block is trusted as a match.
pub const MEMORY_MATCH_THRESHOLD: f32 = 0.85;

#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryClassifier;

#[async_trait]
impl Classifier for MemoryClassifier {
    fn name(&self) -> &'static str {
        "memory"
    }

    async fn classify(
        &self,
        blocks: &[ActivityBlock],
        ctx: &ClassificationContext,
    ) -> CoreResult<Vec<Classification>> {
        Ok(blocks
            .iter()
            .map(|b| {
                let best = ctx
                    .user_classified
                    .iter()
                    .filter(|u| u.category_id.is_some())
                    .map(|u| {
                        (
                            activity_similarity(
                                &b.app_id,
                                &b.title_key,
                                b.domain.as_deref(),
                                &u.app_id,
                                &u.title_key,
                                u.domain.as_deref(),
                            ),
                            u,
                        )
                    })
                    .filter(|(s, _)| *s >= MEMORY_MATCH_THRESHOLD)
                    .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                match best {
                    // Generic titles (e.g. "WhatsApp") match each other trivially; a memory
                    // hit there would just replay the last label forever, so skip them.
                    Some((score, u)) if !is_generic_title(&b.title_key) => Classification {
                        block_id: b.id.clone(),
                        category_id: u.category_id.clone(),
                        confidence: score.min(0.95),
                        source: ClassificationSource::Memory,
                        description: None,
                        needs_vision: false,
                    },
                    _ => Classification::unknown(&b.id, ClassificationSource::Memory),
                }
            })
            .collect())
    }
}

/// Minimum number of agreeing corrections before a rule is suggested.
pub const SUGGESTION_MIN_SUPPORT: u32 = 2;

/// Derives rule suggestions from corrections. A suggestion is emitted when at least
/// `min_support` corrections map the same key (domain, or app for non-browser apps) to the same
/// category and no correction maps that key elsewhere. Keys already covered by an existing
/// enabled rule are skipped.
pub fn suggest_rules(
    corrections: &[Correction],
    existing: &[Rule],
    min_support: u32,
) -> Vec<RuleSuggestion> {
    #[derive(Hash, PartialEq, Eq, Clone)]
    struct Key {
        matcher: RuleMatcher,
        pattern: String,
    }
    let mut votes: HashMap<Key, HashMap<Id, u32>> = HashMap::new();

    for c in corrections {
        let key = match c.domain.as_deref().filter(|d| !d.is_empty()) {
            Some(d) => Key { matcher: RuleMatcher::Domain, pattern: d.to_lowercase() },
            None => {
                if c.app_id.is_empty() {
                    continue;
                }
                Key { matcher: RuleMatcher::App, pattern: c.app_id.clone() }
            }
        };
        *votes.entry(key).or_default().entry(c.to_category_id.clone()).or_default() += 1;
    }

    let mut out: Vec<RuleSuggestion> = votes
        .into_iter()
        .filter_map(|(key, by_cat)| {
            if by_cat.len() != 1 {
                return None; // conflicting corrections → not a rule
            }
            let (cat, n) = by_cat.into_iter().next()?;
            if n < min_support {
                return None;
            }
            let covered = existing.iter().any(|r| {
                r.enabled
                    && r.matcher == key.matcher
                    && r.pattern.eq_ignore_ascii_case(&key.pattern)
            });
            if covered {
                return None;
            }
            let rationale = match key.matcher {
                RuleMatcher::Domain => format!(
                    "Você reclassificou {n} vezes atividades em {} para esta categoria.",
                    key.pattern
                ),
                _ => format!(
                    "Você reclassificou {n} vezes atividades do app {} para esta categoria.",
                    key.pattern
                ),
            };
            Some(RuleSuggestion {
                category_id: cat,
                matcher: key.matcher,
                pattern: key.pattern,
                support: n,
                rationale,
            })
        })
        .collect();
    out.sort_by(|a, b| b.support.cmp(&a.support).then(a.pattern.cmp(&b.pattern)));
    out
}

/// Picks up to `limit` corrections most similar to the given blocks, as few-shot examples.
/// Always includes the most recent corrections first so the model sees fresh preferences.
pub fn select_examples(
    corrections: &[Correction],
    blocks: &[ActivityBlock],
    limit: usize,
) -> Vec<ClassificationExample> {
    if limit == 0 || corrections.is_empty() {
        return vec![];
    }
    let mut scored: Vec<(f32, &Correction)> = corrections
        .iter()
        .map(|c| {
            let best = blocks
                .iter()
                .map(|b| {
                    activity_similarity(
                        &b.app_id,
                        &b.title_key,
                        b.domain.as_deref(),
                        &c.app_id,
                        &c.title_key,
                        c.domain.as_deref(),
                    )
                })
                .fold(0.0_f32, f32::max);
            (best, c)
        })
        .collect();
    // Similar first; ties broken by recency (corrections arrive newest first).
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut seen = std::collections::HashSet::new();
    scored
        .into_iter()
        .filter(|(_, c)| seen.insert((c.app_id.clone(), c.title_key.clone(), c.to_category_id.clone())))
        .take(limit)
        .map(|(_, c)| ClassificationExample {
            app_name: c.app_name.clone(),
            title: c.title_key.clone(),
            domain: c.domain.clone(),
            category_id: c.to_category_id.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn correction(app: &str, title: &str, domain: Option<&str>, to: &str) -> Correction {
        Correction {
            id: new_id(),
            block_id: new_id(),
            from_category_id: None,
            to_category_id: to.into(),
            app_id: app.into(),
            app_name: app.into(),
            title_key: title.into(),
            domain: domain.map(String::from),
            note: None,
            at: Utc::now(),
        }
    }

    #[test]
    fn suggests_domain_rule_after_two_agreeing_corrections() {
        let cs = vec![
            correction("chrome", "sei proc 1", Some("sei.ifro.edu.br"), "ifro"),
            correction("chrome", "sei proc 2", Some("sei.ifro.edu.br"), "ifro"),
            correction("chrome", "yt", Some("youtube.com"), "distraction"),
        ];
        let s = suggest_rules(&cs, &[], 2);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].matcher, RuleMatcher::Domain);
        assert_eq!(s[0].pattern, "sei.ifro.edu.br");
        assert_eq!(s[0].category_id, "ifro");
    }

    #[test]
    fn conflicting_corrections_do_not_suggest() {
        let cs = vec![
            correction("slack", "general", None, "ifro"),
            correction("slack", "general", None, "incubadora"),
        ];
        assert!(suggest_rules(&cs, &[], 1).is_empty());
    }

    #[test]
    fn existing_rule_suppresses_suggestion() {
        let cs = vec![
            correction("xcode", "a", None, "inc"),
            correction("xcode", "b", None, "inc"),
        ];
        let existing = vec![Rule {
            id: new_id(),
            category_id: "inc".into(),
            matcher: RuleMatcher::App,
            pattern: "XCODE".into(),
            priority: 0,
            origin: RuleOrigin::User,
            enabled: true,
            created_at: Utc::now(),
            hit_count: 0,
        }];
        assert!(suggest_rules(&cs, &existing, 2).is_empty());
    }

    #[test]
    fn examples_prefer_similar_and_dedupe() {
        let cs = vec![
            correction("chrome", "sei processo", Some("sei.ifro.edu.br"), "ifro"),
            correction("chrome", "sei processo", Some("sei.ifro.edu.br"), "ifro"),
            correction("figma", "landing page", None, "inc"),
        ];
        let b = ActivityBlock {
            id: new_id(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            app_name: "chrome".into(),
            app_id: "chrome".into(),
            title: "SEI processo 9".into(),
            title_key: "sei processo 9".into(),
            url: None,
            domain: Some("sei.ifro.edu.br".into()),
            category_id: None,
            confidence: 0.0,
            source: None,
            description: None,
            screenshot_id: None,
            sample_count: 1,
            is_open: false,
        };
        let ex = select_examples(&cs, &[b], 5);
        assert_eq!(ex.len(), 2);
        assert_eq!(ex[0].category_id, "ifro");
    }

    #[tokio::test]
    async fn memory_classifier_matches_similar_user_blocks() {
        let mut labelled = ActivityBlock {
            id: new_id(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            app_name: "Google Chrome".into(),
            app_id: "com.google.Chrome".into(),
            title: "SEI - Processo 1".into(),
            title_key: "sei - processo 1".into(),
            url: None,
            domain: Some("sei.ifro.edu.br".into()),
            category_id: Some("ifro".into()),
            confidence: 1.0,
            source: Some(ClassificationSource::User),
            description: None,
            screenshot_id: None,
            sample_count: 1,
            is_open: false,
        };
        let mut fresh = labelled.clone();
        fresh.id = new_id();
        fresh.category_id = None;
        fresh.source = None;
        fresh.title_key = "sei - processo 2".into();
        let ctx = ClassificationContext {
            categories: vec![],
            rules: vec![],
            examples: vec![],
            user_classified: vec![labelled.clone()],
            language: "pt-BR".into(),
            min_confidence: 0.6,
        };
        let out = MemoryClassifier.classify(&[fresh.clone()], &ctx).await.unwrap();
        assert_eq!(out[0].category_id.as_deref(), Some("ifro"));
        assert_eq!(out[0].source, ClassificationSource::Memory);

        // Generic titles never match through memory.
        labelled.title_key = "whatsapp".into();
        labelled.domain = None;
        fresh.title_key = "whatsapp".into();
        fresh.domain = None;
        let ctx = ClassificationContext { user_classified: vec![labelled], ..ctx };
        let out = MemoryClassifier.classify(&[fresh], &ctx).await.unwrap();
        assert!(out[0].category_id.is_none());
    }
}
