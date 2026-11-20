//! # ubiqx-nofap
//!
//! The pure half of the `nofap` module: every decision, and the embedded data those
//! decisions read. No I/O, no clock, no operating-system API. The runtime that drives it is
//! `ubiqx_engine::nofap`; the adapters that touch the system are in `ubiqx-platform`
//! (`sysblock`, `frame`); the shared vocabulary (configuration, status, ports) is
//! `ubiqx_core::nofap`.
//!
//! | Module | What it decides |
//! |---|---|
//! | [`list`] | the embedded hash tables and data files (the only module that reads them) |
//! | [`matcher`] | the verdict for a host: blocked, allowed or unknown with a class |
//! | [`heuristics`] | keyword and TLD rules for hosts no list decides; signals |
//! | [`hosts`] | the marked block of the hosts file: render, splice, inspect |
//! | [`resolvers`] | the compiled table of filtering resolvers |
//! | [`policies`] | the browser policy values and conflict detection |
//! | [`plan`] | the system plan: build, encode, validate, union, assess, preview |
//! | [`gate`] | which system tier is open on which operating system |
//! | [`state_dir`] | whether the helper's state directory can be trusted |
//! | [`commitment`] | which change waits the delay, and when a waiting change is ready |
//! | [`sampling`] | when the AI check looks at a window, and what its answer means |
//! | [`episode`] | grouping hits into episodes; the two-pass confirmation |
//! | [`exclusions`] | when an entry of ubiqX's privacy lists becomes effective for vision |
//! | [`primary`] | which of two products on one machine runs the passes |
//! | [`messages`] | the few lines the engine and the shells need in Rust |
//! | [`help`] | when the professional-help card is due; the help resources |
//! | [`progress`] | "days your way" and the streak |
//!
//! Rules every module follows:
//!
//! * Pure functions receive their data as arguments; only [`list`] reads the embedded
//!   files, and assertions about the real embedded data live in `tests/embedded.rs`.
//! * Nothing here formats a sentence for the user except [`messages`], whose strings come
//!   from the reviewed copy bank.
//! * No function takes or returns an idle time: watching video produces no input, so
//!   idleness must never stop a check.

pub mod commitment;
pub mod episode;
pub mod exclusions;
pub mod gate;
pub mod help;
pub mod heuristics;
pub mod hosts;
pub mod list;
pub mod matcher;
pub mod messages;
pub mod plan;
pub mod policies;
pub mod primary;
pub mod progress;
pub mod resolvers;
pub mod sampling;
pub mod state_dir;

pub use matcher::{Matcher, Verdict};
