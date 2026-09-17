//! # ubiqx-ai
//!
//! The AI adapter of ubiqX: a small, retrying client for the Anthropic Messages API and the
//! port implementations built on it.
//!
//! * [`client`] — [`AnthropicClient`] (`POST /v1/messages`, `GET /v1/models`), request/response
//!   types, retry policy and usage recording.
//! * [`prompts`] — prompt builders that only ever see redacted views of a block
//!   ([`prompts::PromptBlock`]) and the exact per-block line the engine stores as `ai_payload`
//!   ([`prompts::render_block_line`]).
//! * [`classifier`] — [`classifier::LlmTextClassifier`] (`RemoteClassifier`, batched text).
//! * [`vision`] — [`vision::LlmVisionClassifier`] (`VisionClassifier`, one screenshot).
//! * [`report`] — [`report::LlmReportWriter`] (`ReportWriter`, daily institutional report).
//! * [`advisor`] — [`advisor::LlmAdvisor`] (`Advisor`, weekly recommendations from numbers).
//! * [`pricing`] — price table and cost estimate per call.
//! * [`fake`] — deterministic, network-free implementations for tests and the Linux CLI.
//!
//! Nothing in this crate reads a raw window title: every prompt goes through
//! [`ubiqx_core::redact`], enforced by construction.

pub mod advisor;
pub mod classifier;
pub mod client;
pub mod fake;
pub mod pricing;
pub mod prompts;
pub mod report;
pub mod vision;

pub use advisor::LlmAdvisor;
pub use classifier::LlmTextClassifier;
pub use client::{
    AnthropicClient, AnthropicConfig, ApiKeySource, LlmClient, LlmContent, LlmMessage, LlmRequest,
    LlmResponse, LlmRole, StaticApiKey, StopReason, SystemBlock,
};
pub use report::LlmReportWriter;
pub use vision::LlmVisionClassifier;
