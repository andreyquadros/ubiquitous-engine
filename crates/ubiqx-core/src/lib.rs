//! # ubiqx-core
//!
//! The domain layer of ubiqX. It contains:
//!
//! * [`model`] — plain data types shared by every other crate (activity samples and blocks,
//!   categories, rules, corrections, reports, nudges, settings).
//! * [`ports`] — the traits ("ports" in hexagonal-architecture terms) that adapters implement:
//!   platform capture, persistence, AI, notifications and time.
//! * Pure business logic that needs no I/O and is therefore fully unit-tested here:
//!   [`segmenter`] (samples → blocks), [`rules`] (deterministic classification),
//!   [`learning`] (turning corrections into rules and few-shot examples), [`insights`]
//!   (focus score and nudges), [`scheduler`] (when daily reports are due) and
//!   [`normalize`] (title/domain normalisation and similarity) and [`redact`] (what may leave
//!   the machine). [`lang`] names the UI language every generated text follows.
//!
//! This crate must never depend on an operating-system API, a database driver or an HTTP client.

pub mod clock;
pub mod error;
pub mod insights;
pub mod lang;
pub mod learning;
pub mod model;
pub mod normalize;
pub mod ports;
pub mod redact;
pub mod report;
pub mod rules;
pub mod scheduler;
pub mod segmenter;

pub use clock::{Clock, SystemClock};
pub use error::{CoreError, CoreResult};
pub use lang::UiLanguage;
pub use model::*;
