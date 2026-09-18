//! # ubi-api
//!
//! The Ubi proxy behind the "ubiqX Mensal, com a IA do Ubi" plan. Desktop apps holding a
//! `monthly_managed` license send Anthropic-shaped `POST /v1/messages` calls here with their
//! license key instead of a vendor key; the proxy verifies the key offline, maps the public
//! model aliases (`ubi-fast`, `ubi-smart`) to real vendor models, enforces a per-subscriber
//! monthly budget, forwards the call with the operator's vendor key (non-streaming and SSE
//! alike) and records the cost in a SQLite ledger.
//!
//! Modules:
//! * [`config`] — environment configuration and the price table;
//! * [`db`] — the SQLite ledger (usage, revocations, issued licenses);
//! * [`issue`] — key pairs and key issuing (the verify side lives in `ubiqx_core::license`);
//! * [`sse`] — usage extraction from JSON and SSE vendor responses;
//! * [`routes`] — the HTTP surface (public, admin and the payment webhook).
//!
//! It is a standalone Cargo project (not a member of the ubiqX workspace) so that desktop
//! builds never compile the server stack.

pub mod config;
pub mod db;
pub mod issue;
pub mod routes;
pub mod sse;

pub use config::Config;
pub use db::Db;
pub use routes::{router, AppState};

use anyhow::Context;
use tracing::info;

/// Binds `config.listen` and serves until SIGINT / SIGTERM.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let listen = config.listen;
    let state = AppState::new(config)?;
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("binding {listen}"))?;
    info!(
        %listen,
        vendor = %state.config.vendor,
        fast = %state.config.model_fast,
        smart = %state.config.model_smart,
        "ubi-api listening"
    );
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("shutting down");
}
