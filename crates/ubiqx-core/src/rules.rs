//! Deterministic rule matching — the first, free link of the classifier chain.

use async_trait::async_trait;

use crate::error::CoreResult;
use crate::model::*;
use crate::normalize::domain_matches;
use crate::ports::{Classification, ClassificationContext, Classifier};

/// Evaluates one rule against a block.
pub fn rule_matches(rule: &Rule, block: &ActivityBlock) -> bool {
    if !rule.enabled {
        return false;
    }
    let pattern = rule.pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    match rule.matcher {
        RuleMatcher::App => {
            pattern.eq_ignore_ascii_case(&block.app_id)
                || pattern.eq_ignore_ascii_case(&block.app_name)
        }
        RuleMatcher::Domain => block
            .domain
            .as_deref()
            .map(|d| domain_matches(d, pattern))
            .unwrap_or(false),
        RuleMatcher::TitleContains => {
            let p = pattern.to_lowercase();
            block.title.to_lowercase().contains(&p) || block.title_key.contains(&p)
        }
        RuleMatcher::Regex => match regex::RegexBuilder::new(pattern)
            .case_insensitive(true)
            .size_limit(1 << 20)
            .build()
        {
            Ok(re) => {
                let hay = format!(
                    "{} | {} | {}",
                    block.app_name,
                    block.title,
                    block.url.as_deref().unwrap_or("")
                );
                re.is_match(&hay)
            }
            Err(_) => false,
        },
    }
}

/// Finds the best matching rule (highest priority, then most specific matcher, then newest).
pub fn best_rule<'a>(rules: &'a [Rule], block: &ActivityBlock) -> Option<&'a Rule> {
    rules
        .iter()
        .filter(|r| rule_matches(r, block))
        .max_by_key(|r| (r.priority, specificity(r.matcher), r.created_at))
}

fn specificity(m: RuleMatcher) -> u8 {
    match m {
        RuleMatcher::Regex => 4,
        RuleMatcher::TitleContains => 3,
        RuleMatcher::Domain => 2,
        RuleMatcher::App => 1,
    }
}

/// Classifier backed by the user's rules. Never guesses: a block without a matching rule is
/// returned as unknown.
#[derive(Debug, Default, Clone, Copy)]
pub struct RuleClassifier;

#[async_trait]
impl Classifier for RuleClassifier {
    fn name(&self) -> &'static str {
        "rules"
    }

    async fn classify(
        &self,
        blocks: &[ActivityBlock],
        ctx: &ClassificationContext,
    ) -> CoreResult<Vec<Classification>> {
        Ok(blocks
            .iter()
            .map(|b| match best_rule(&ctx.rules, b) {
                Some(rule) => Classification {
                    block_id: b.id.clone(),
                    category_id: Some(rule.category_id.clone()),
                    confidence: 1.0,
                    source: ClassificationSource::Rule,
                    description: None,
                    needs_vision: false,
                },
                None => Classification::unknown(&b.id, ClassificationSource::Rule),
            })
            .collect())
    }
}

/// Validates a rule pattern before it is stored.
pub fn validate_rule(matcher: RuleMatcher, pattern: &str) -> Result<(), String> {
    let p = pattern.trim();
    if p.is_empty() {
        return Err("pattern must not be empty".into());
    }
    if matcher == RuleMatcher::Regex {
        regex::RegexBuilder::new(p)
            .size_limit(1 << 20)
            .build()
            .map_err(|e| format!("invalid regular expression: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn block(app: &str, title: &str, url: Option<&str>) -> ActivityBlock {
        ActivityBlock {
            id: new_id(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            app_name: app.into(),
            app_id: format!("com.x.{}", app.to_lowercase()),
            title: title.into(),
            title_key: title.to_lowercase(),
            url: url.map(String::from),
            domain: url.and_then(crate::normalize::domain_of),
            category_id: None,
            confidence: 0.0,
            source: None,
            description: None,
            screenshot_id: None,
            sample_count: 1,
            is_open: false,
        }
    }

    fn rule(m: RuleMatcher, p: &str, prio: i32) -> Rule {
        Rule {
            id: new_id(),
            category_id: format!("cat-{p}"),
            matcher: m,
            pattern: p.into(),
            priority: prio,
            origin: RuleOrigin::User,
            enabled: true,
            created_at: Utc::now(),
            hit_count: 0,
        }
    }

    #[test]
    fn matchers() {
        let b = block("Google Chrome", "SEI · Processo", Some("https://sei.ifro.edu.br/x"));
        assert!(rule_matches(&rule(RuleMatcher::App, "google chrome", 0), &b));
        assert!(rule_matches(&rule(RuleMatcher::App, "com.x.google chrome", 0), &b));
        assert!(rule_matches(&rule(RuleMatcher::Domain, "ifro.edu.br", 0), &b));
        assert!(rule_matches(&rule(RuleMatcher::TitleContains, "processo", 0), &b));
        assert!(rule_matches(&rule(RuleMatcher::Regex, r"sei\.ifro", 0), &b));
        assert!(!rule_matches(&rule(RuleMatcher::Regex, r"(unclosed", 0), &b));
        let mut disabled = rule(RuleMatcher::App, "google chrome", 0);
        disabled.enabled = false;
        assert!(!rule_matches(&disabled, &b));
    }

    #[test]
    fn priority_then_specificity() {
        let b = block("Google Chrome", "SEI", Some("https://sei.ifro.edu.br/x"));
        let rules = vec![
            rule(RuleMatcher::App, "google chrome", 0),
            rule(RuleMatcher::Domain, "ifro.edu.br", 0),
        ];
        assert_eq!(best_rule(&rules, &b).unwrap().pattern, "ifro.edu.br");
        let rules = vec![
            rule(RuleMatcher::App, "google chrome", 10),
            rule(RuleMatcher::Domain, "ifro.edu.br", 0),
        ];
        assert_eq!(best_rule(&rules, &b).unwrap().pattern, "google chrome");
    }

    #[tokio::test]
    async fn classifier_returns_unknown_without_match() {
        let ctx = ClassificationContext {
            categories: vec![],
            rules: vec![rule(RuleMatcher::App, "Xcode", 0)],
            examples: vec![],
            user_classified: vec![],
            language: "pt-BR".into(),
            min_confidence: 0.6,
        };
        let out = RuleClassifier
            .classify(&[block("Xcode", "a", None), block("Slack", "b", None)], &ctx)
            .await
            .unwrap();
        assert_eq!(out[0].category_id.as_deref(), Some("cat-Xcode"));
        assert!(out[1].category_id.is_none());
    }

    #[test]
    fn validation() {
        assert!(validate_rule(RuleMatcher::Regex, "(").is_err());
        assert!(validate_rule(RuleMatcher::App, "  ").is_err());
        assert!(validate_rule(RuleMatcher::Domain, "ifro.edu.br").is_ok());
    }
}
