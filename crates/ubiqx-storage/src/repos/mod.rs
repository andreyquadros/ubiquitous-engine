//! One module per persistence port. Each implements the corresponding `ubiqx_core::ports`
//! trait for [`crate::SqliteStore`].

mod blocks;
mod categories;
mod corrections;
mod focus;
mod kv;
mod maintenance;
mod nudges;
mod reports;
mod rules;
mod screenshots;
mod settings;
mod usage;
