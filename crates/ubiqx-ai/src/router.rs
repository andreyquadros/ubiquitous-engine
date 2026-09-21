//! Routes every [`LlmRequest`] to the client of the provider the user selected.
//!
//! The port implementations ([`crate::classifier`], [`crate::vision`], [`crate::report`],
//! [`crate::advisor`]) hold one `Arc<dyn LlmClient>`; wrapping it in a [`RoutingLlmClient`]
//! lets the user switch vendors in settings without rebuilding the engine. The selected
//! provider is read from a [`ProviderSource`] on every call (the desktop app backs it with the
//! settings store, tests and the CLI use [`FixedProvider`]).

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;
use ubiqx_core::{AiProvider, CoreError, CoreResult};

use crate::client::{LlmClient, LlmRequest, LlmResponse};

/// Where the selected provider comes from. Read on every call so a settings change applies
/// to the next request.
pub trait ProviderSource: Send + Sync {
    fn provider(&self) -> AiProvider;
}

/// A [`ProviderSource`] that never changes (CLI, tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedProvider(pub AiProvider);

impl ProviderSource for FixedProvider {
    fn provider(&self) -> AiProvider {
        self.0
    }
}

/// An [`LlmClient`] that forwards each call to the client registered for the provider that
/// [`ProviderSource::provider`] currently returns.
pub struct RoutingLlmClient {
    clients: HashMap<AiProvider, Arc<dyn LlmClient>>,
    source: Arc<dyn ProviderSource>,
}

impl fmt::Debug for RoutingLlmClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut providers: Vec<&str> = self.clients.keys().map(|p| p.id()).collect();
        providers.sort_unstable();
        f.debug_struct("RoutingLlmClient")
            .field("providers", &providers)
            .field("selected", &self.source.provider().id())
            .finish()
    }
}

impl RoutingLlmClient {
    /// Builds the router. A provider listed twice keeps the last client given for it;
    /// providers without a client are allowed and yield [`CoreError::AiNotConfigured`] when
    /// selected.
    pub fn new(
        clients: Vec<(AiProvider, Arc<dyn LlmClient>)>,
        source: Arc<dyn ProviderSource>,
    ) -> Self {
        Self {
            clients: clients.into_iter().collect(),
            source,
        }
    }

    /// The provider requests are currently routed to.
    pub fn selected(&self) -> AiProvider {
        self.source.provider()
    }

    /// Providers that have a client registered (unordered).
    pub fn providers(&self) -> Vec<AiProvider> {
        AiProvider::ALL
            .into_iter()
            .filter(|p| self.clients.contains_key(p))
            .collect()
    }

    /// The client registered for `provider`, if any.
    pub fn client_for(&self, provider: AiProvider) -> Option<Arc<dyn LlmClient>> {
        self.clients.get(&provider).cloned()
    }

    /// The client for the selected provider, or [`CoreError::AiNotConfigured`] when none is
    /// registered for it.
    pub fn current(&self) -> CoreResult<Arc<dyn LlmClient>> {
        let provider = self.selected();
        self.client_for(provider).ok_or_else(|| {
            debug!(
                provider = provider.id(),
                "router: no client registered for the selected provider"
            );
            CoreError::AiNotConfigured
        })
    }
}

#[async_trait]
impl LlmClient for RoutingLlmClient {
    async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse> {
        self.current()?.complete(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::ScriptedLlmClient;

    #[test]
    fn providers_and_lookup() {
        let a: Arc<dyn LlmClient> = Arc::new(ScriptedLlmClient::new(vec![]));
        let x: Arc<dyn LlmClient> = Arc::new(ScriptedLlmClient::new(vec![]));
        let router = RoutingLlmClient::new(
            vec![(AiProvider::Anthropic, a), (AiProvider::Xai, x)],
            Arc::new(FixedProvider(AiProvider::Xai)),
        );
        assert_eq!(
            router.providers(),
            vec![AiProvider::Anthropic, AiProvider::Xai]
        );
        assert_eq!(router.selected(), AiProvider::Xai);
        assert!(router.client_for(AiProvider::OpenAi).is_none());
        assert!(router.current().is_ok());
        let dbg = format!("{router:?}");
        assert!(dbg.contains("xai"), "{dbg}");
    }

    #[test]
    fn unregistered_selection_is_not_configured() {
        let router = RoutingLlmClient::new(vec![], Arc::new(FixedProvider(AiProvider::OpenAi)));
        assert!(matches!(router.current(), Err(CoreError::AiNotConfigured)));
        assert!(router.providers().is_empty());
    }
}
