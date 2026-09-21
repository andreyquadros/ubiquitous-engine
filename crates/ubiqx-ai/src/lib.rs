//! # ubiqx-ai
//!
//! The AI adapter of ubiqX: small, retrying HTTP clients for the hosted LLM vendors the user
//! can choose from (Anthropic Claude, OpenAI, xAI Grok), for the managed option (the Ubi
//! proxy, paid by the monthly plan) and the port implementations built on them.
//!
//! * [`client`] — [`AnthropicClient`] (`POST /v1/messages`, `GET /v1/models`), the
//!   vendor-neutral request/response types ([`LlmRequest`], [`LlmResponse`], [`LlmClient`])
//!   and usage recording. With [`AnthropicConfig::for_ubi`] the same client talks to the Ubi
//!   proxy (license key as credential, fixed model aliases, cost reported by the proxy).
//! * [`ubi`] — [`ubi::UbiLicenseClient`], the proxy's `GET /v1/license/status` (monthly
//!   usage of a managed subscription).
//! * [`openai`] — [`OpenAiCompatClient`] (`POST /chat/completions`, `GET /models`) for
//!   OpenAI and xAI, with the per-vendor request policy documented in the module.
//! * [`router`] — [`RoutingLlmClient`], an `LlmClient` that forwards to the client of the
//!   provider a [`ProviderSource`] currently selects (the user's setting).
//! * [`retry`] — the backoff/retry policy shared by the HTTP clients.
//! * [`prompts`] — prompt builders that only ever see redacted views of a block
//!   ([`prompts::PromptBlock`]) and the exact per-block line the engine stores as `ai_payload`
//!   ([`prompts::render_block_line`]).
//! * [`classifier`] — [`classifier::LlmTextClassifier`] (`RemoteClassifier`, batched text).
//! * [`vision`] — [`vision::LlmVisionClassifier`] (`VisionClassifier`, one screenshot).
//! * [`report`] — [`report::LlmReportWriter`] (`ReportWriter`, daily institutional report).
//! * [`advisor`] — [`advisor::LlmAdvisor`] (`Advisor`, weekly recommendations from numbers).
//! * [`pricing`] — per-vendor price tables and the cost estimate recorded with every call.
//! * [`fake`] — deterministic, network-free implementations for tests and the Linux CLI.
//!
//! Nothing in this crate reads a raw window title: every prompt goes through
//! [`ubiqx_core::redact`], enforced by construction.

pub mod advisor;
pub mod classifier;
pub mod client;
pub mod fake;
pub mod openai;
pub mod pricing;
pub mod prompts;
pub mod report;
pub mod retry;
pub mod router;
pub mod ubi;
pub mod vision;

pub use advisor::LlmAdvisor;
pub use classifier::LlmTextClassifier;
pub use client::{
    AnthropicClient, AnthropicConfig, ApiKeySource, LlmClient, LlmContent, LlmMessage, LlmRequest,
    LlmResponse, LlmRole, StaticApiKey, StopReason, SystemBlock, UBI_MODEL_ALIASES,
};
pub use openai::{OpenAiCompatClient, OpenAiCompatConfig};
pub use report::LlmReportWriter;
pub use router::{FixedProvider, ProviderSource, RoutingLlmClient};
pub use ubi::{RemoteLicenseStatus, UbiLicenseClient};
pub use vision::LlmVisionClassifier;
