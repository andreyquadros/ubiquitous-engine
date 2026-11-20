//! ubiqX as the module reads it: the language, the AI provider and whether it is usable,
//! and the privacy lists. Read-only, and the only file of this module that knows the
//! engine's state; the standalone app has its own, smaller reading of its settings row.
//!
//! Nothing here reads the productivity AI's health or its budget: the AI check has its
//! own readiness, and neither side can pause the other.

use std::sync::{OnceLock, Weak};

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use ubiqx_core::nofap::{AiReadiness, NofapEnv, NofapEnvSnapshot};
use ubiqx_core::{AiProvider, UiLanguage};

use crate::state::EngineState;

/// How long an answer of the secret store is reused. The guard asks every couple of
/// seconds, and the store is the operating system's keychain; a removed key stops the
/// check at once all the same, because the AI client reads the key for every call.
const KEY_ANSWER_FOR_SECS: i64 = 15;

/// [`NofapEnv`] over the engine's state. It is created before the state exists (the
/// service is a field of the state) and bound to it right after.
#[derive(Default)]
pub struct EngineEnv {
    state: OnceLock<Weak<EngineState>>,
    key: Mutex<Option<(AiProvider, bool, DateTime<Utc>)>>,
}

impl EngineEnv {
    pub fn new() -> Self {
        Self::default()
    }

    /// Points the environment at the state it reads. Only the first call counts.
    pub fn bind(&self, state: Weak<EngineState>) {
        let _ = self.state.set(state);
    }

    fn has_key(&self, state: &EngineState, provider: AiProvider) -> bool {
        let now = state.now();
        let mut cached = self.key.lock();
        if let Some((of, present, at)) = *cached {
            // An answer from the future (the clock was set back) is asked for again.
            if of == provider && (0..KEY_ANSWER_FOR_SECS).contains(&(now - at).num_seconds()) {
                return present;
            }
        }
        let present = matches!(
            state.deps.platform.secrets.get(provider.secret_key()),
            Ok(Some(key)) if !key.trim().is_empty()
        );
        *cached = Some((provider, present, now));
        present
    }

    fn readiness(&self, state: &EngineState) -> AiReadiness {
        let settings = state.settings.read().clone();
        if settings.local_only {
            AiReadiness::LocalOnly
        } else if !settings.onboarding_done {
            AiReadiness::NotOnboarded
        } else if state.license.read().blocks_ai() {
            AiReadiness::LicenceRequired
        } else if settings.ai_provider.is_managed() {
            // The managed provider's "key" is the licence.
            if state.license.read().allows_managed_ai() {
                AiReadiness::Ready
            } else {
                AiReadiness::LicenceRequired
            }
        } else if !state.deps.ai.requires_api_key || self.has_key(state, settings.ai_provider) {
            AiReadiness::Ready
        } else {
            AiReadiness::NoKey
        }
    }
}

impl NofapEnv for EngineEnv {
    fn snapshot(&self) -> NofapEnvSnapshot {
        let Some(state) = self.state.get().and_then(Weak::upgrade) else {
            // The engine is gone or not there yet: nothing may be sent anywhere.
            return NofapEnvSnapshot {
                language: UiLanguage::default(),
                provider: AiProvider::default(),
                vision_model: String::new(),
                ai: AiReadiness::NotOnboarded,
                excluded_apps: Vec::new(),
                excluded_domains: Vec::new(),
            };
        };
        let ai = self.readiness(&state);
        let settings = state.settings.read();
        let mut excluded_apps = settings.blocked_apps.clone();
        excluded_apps.extend(settings.vision_denied_apps.iter().cloned());
        NofapEnvSnapshot {
            language: settings.ui_language(),
            provider: settings.ai_provider,
            vision_model: settings.models.vision.clone(),
            ai,
            excluded_apps,
            excluded_domains: settings.blocked_domains.clone(),
        }
    }
}
