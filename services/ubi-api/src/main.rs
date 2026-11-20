//! `ubi-api` server binary: reads the configuration from the environment and serves.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
    let config = ubi_api::Config::from_env()?;
    ubi_api::serve(config).await
}
