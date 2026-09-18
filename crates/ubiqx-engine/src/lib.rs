//! # ubiqx-engine
//!
//! The application layer. It owns the runtime loops and exposes two things to the shells:
//!
//! * [`Engine::start`] — wires the ports from [`EngineDeps`], spawns the sampler thread and the
//!   tokio tasks (tracker, classifier, report scheduler, nudges, retention, update checks)
//!   and returns an [`EngineHandle`].
//! * [`EngineHandle`] — the API the desktop app and the CLI call: pause/resume, private mode,
//!   corrections (the learning entry point), manual entries, report generation, settings,
//!   update checks ([`update`]) and read models for the UI ([`service`]).
//!
//! The engine never talks to an OS API, a database driver or an HTTP client directly — only to
//! the traits in `ubiqx_core::ports`.

pub mod classify;
pub mod deps;
pub mod engine;
pub mod handle;
pub mod learning;
pub mod nudges;
pub mod reports;
pub mod sampler;
pub mod screenshots;
pub mod service;
pub mod state;
pub mod tracker;
pub mod update;

pub use deps::{AiPorts, EngineDeps, PlatformPorts, Repos};
pub use engine::{Engine, LoopConfig};
pub use handle::{EngineHandle, PrivateModeDuration, ReclassifyScope};
pub use service::*;
pub use state::EngineState;
pub use update::UpdateChecker;
