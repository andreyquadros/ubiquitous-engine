//! The license as the engine sees it: the stored key's verdict, the managed plan's monthly
//! usage and what the license gates.
//!
//! * The key lives in the secret store under [`LICENSE_SECRET_KEY`], never in the settings
//!   row. It is verified offline against the public key in [`crate::LicenseDeps`] at start
//!   and whenever it is set ([`set_key`]).
//! * A valid `monthly_managed` key also asks the proxy for the month's usage
//!   ([`refresh_managed_usage`]): at start, when the key is set, periodically and after the
//!   proxy refused a call for budget reasons. A proxy that cannot be reached leaves the local
//!   verdict as it is, without usage numbers.
//! * Gating: the managed provider ([`AiProvider::Ubi`]) can only be selected with a valid
//!   `monthly_managed` license ([`CoreError::LicenseRequired`], checked in
//!   `EngineState::apply_settings`); under [`ubiqx_core::LicenseEnforcement::Hard`] an
//!   unlicensed app gets the same error from every AI entry point and never calls the AI in
//!   the background (`EngineState::license_gate` / `remote_allowed`). Tracking, the timeline
//!   and manual categorisation are never gated.

use std::sync::Arc;

use ubiqx_core::*;

use crate::deps::EngineDeps;
use crate::state::EngineState;

/// The stored license key, if any (blank counts as none).
pub fn stored_key(deps: &EngineDeps) -> CoreResult<Option<String>> {
    Ok(deps
        .platform
        .secrets
        .get(LICENSE_SECRET_KEY)?
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty()))
}

/// The local verdict on the stored key at the engine's current time. A secret store that
/// cannot be read counts as "no key" (logged), so the engine still starts.
pub fn evaluate(deps: &EngineDeps) -> LicenseStatus {
    let key = match stored_key(deps) {
        Ok(k) => k,
        Err(e) => {
            tracing::warn!(error = %e, "could not read the license key; treating as unlicensed");
            None
        }
    };
    let status =
        LicenseStatus::evaluate_with(key.as_deref(), &deps.license.pubkey_hex, deps.clock.now());
    tracing::info!(
        state = ?status.state,
        plan = ?status.plan,
        days_left = ?status.days_left,
        "license evaluated"
    );
    status
}

/// Re-evaluates the stored key (expiry moves with the clock) without touching the proxy.
/// Keeps the managed usage already known when the key is still a valid managed one.
pub fn reevaluate(state: &EngineState) -> LicenseStatus {
    let mut status = evaluate(&state.deps);
    {
        let current = state.license.read();
        if status.allows_managed_ai() && current.key_hint == status.key_hint {
            status.managed_usage = current.managed_usage.clone();
        }
    }
    *state.license.write() = status.clone();
    status
}

/// The last verdict without touching the secret store: the cached status, unless its
/// expiry has passed since it was computed, in which case the key is re-verified (so an
/// expiry is never reported late, and the common path costs no Keychain read).
pub fn current(state: &EngineState) -> LicenseStatus {
    let cached = state.license.read().clone();
    let expired = cached.state == LicenseState::Valid
        && cached
            .expires_at
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .is_some_and(|exp| state.deps.clock.now() >= exp);
    if expired {
        reevaluate(state)
    } else {
        cached
    }
}

/// Asks the proxy for the month's usage when the license is a valid `monthly_managed` one
/// and stores the answer in the status (`None` when the proxy did not answer). Any other
/// license keeps `managed_usage = None`.
pub async fn refresh_managed_usage(state: &Arc<EngineState>) -> LicenseStatus {
    let (allows, key) = {
        let status = state.license.read();
        (
            status.allows_managed_ai(),
            stored_key(&state.deps).ok().flatten(),
        )
    };
    let usage = match (allows, key) {
        (true, Some(key)) => match state.deps.license.server.managed_usage(&key).await {
            Ok(usage) => {
                tracing::info!(
                    month = %usage.month,
                    spent_usd = usage.spent_usd,
                    budget_usd = usage.budget_usd,
                    "managed AI usage refreshed"
                );
                Some(usage)
            }
            Err(e) => {
                tracing::warn!(error = %e, "could not fetch the managed AI usage; keeping the local verdict");
                None
            }
        },
        _ => None,
    };
    let mut status = state.license.write();
    status.managed_usage = usage;
    status.clone()
}

/// Stores (or, with `None`/blank, deletes) the license key, verifies it and returns the new
/// status. Any non-blank key is stored, even an invalid or expired one, so the status the
/// UI shows (`invalid` / `expired` with the key's hint) matches what is in the store; the
/// user replaces or removes it from the same screen.
///
/// When the managed provider is selected, its health follows the verdict: `Ok` with a valid
/// `monthly_managed` key, `NotConfigured` otherwise (the app then skips remote calls until
/// the user picks another provider or fixes the key).
pub async fn set_key(state: &Arc<EngineState>, key: Option<&str>) -> CoreResult<LicenseStatus> {
    let secrets = &state.deps.platform.secrets;
    match key.map(license::normalize_key).filter(|k| !k.is_empty()) {
        Some(k) => secrets.set(LICENSE_SECRET_KEY, &k)?,
        None => secrets.delete(LICENSE_SECRET_KEY)?,
    }
    let status = evaluate(&state.deps);
    *state.license.write() = status;
    let status = refresh_managed_usage(state).await;
    let settings = state.settings();
    if settings.ai_provider.is_managed() {
        let health = crate::state::key_health(&state.deps, &settings, &status);
        state.set_ai_health(health);
    }
    Ok(status)
}

/// `AiHealth::Paused` reason once the managed plan's monthly budget is spent.
pub fn managed_budget_reached_text(usage: &ManagedUsage, lang: UiLanguage) -> String {
    match lang {
        UiLanguage::PtBr => format!(
            "Orçamento mensal da IA do Ubi atingido (US$ {:.2} de US$ {:.2}). A IA volta no próximo mês.",
            usage.spent_usd, usage.budget_usd
        ),
        UiLanguage::En => format!(
            "Monthly budget of Ubi's AI reached (${:.2} of ${:.2}). The AI resumes next month.",
            usage.spent_usd, usage.budget_usd
        ),
    }
}

/// Whether `usage` says the managed plan's budget for the current UTC month is spent. Usage
/// of another month is stale (the proxy resets on the 1st) and never pauses anything.
pub fn managed_budget_exhausted(usage: &ManagedUsage, now: chrono::DateTime<chrono::Utc>) -> bool {
    usage.month == now.format("%Y-%m").to_string()
        && usage.budget_usd > 0.0
        && usage.spent_usd >= usage.budget_usd
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn budget_exhaustion_is_per_current_month() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 9, 18, 12, 0, 0).unwrap();
        let usage = |month: &str, spent: f64| ManagedUsage {
            month: month.into(),
            spent_usd: spent,
            budget_usd: 6.0,
        };
        assert!(managed_budget_exhausted(&usage("2026-09", 6.0), now));
        assert!(managed_budget_exhausted(&usage("2026-09", 7.5), now));
        assert!(!managed_budget_exhausted(&usage("2026-09", 5.99), now));
        assert!(!managed_budget_exhausted(&usage("2026-08", 9.0), now));
        assert!(!managed_budget_exhausted(
            &ManagedUsage {
                budget_usd: 0.0,
                ..usage("2026-09", 1.0)
            },
            now
        ));
        let text = managed_budget_reached_text(&usage("2026-09", 6.0), UiLanguage::PtBr);
        assert!(text.contains("US$ 6.00 de US$ 6.00"), "{text}");
        let text = managed_budget_reached_text(&usage("2026-09", 6.0), UiLanguage::En);
        assert!(text.contains("$6.00 of $6.00"), "{text}");
    }
}
