use thiserror::Error;

/// Errors that can be produced by any port implementation or core service.
///
/// Adapters map their own error types (SQLite, HTTP, OS) onto these variants so that the
/// application layer can react uniformly (retry, surface to the UI, degrade gracefully).
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("platform error: {0}")]
    Platform(String),

    #[error("permission missing: {0}")]
    Permission(String),

    #[error("AI provider error: {0}")]
    Ai(String),

    #[error("AI provider is not configured (missing API key)")]
    AiNotConfigured,

    #[error("rate limited by AI provider; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Other(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

impl CoreError {
    /// Whether the operation that produced this error is worth retrying later.
    pub fn is_transient(&self) -> bool {
        matches!(self, CoreError::RateLimited { .. } | CoreError::Ai(_))
    }
}
