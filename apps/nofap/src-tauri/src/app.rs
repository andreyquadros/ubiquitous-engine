//! The standalone shell: plugins, tray, the main window, the two commands (`nofap`,
//! `nofap_host`), the presenter and the sink. It starts `ubiqx_app::nofap_app::NofapApp`,
//! which never constructs an `Engine`: no sampler, tracker, classifier or report exists in
//! this process.
//!
//! Seeded by the foundation unit so that `main.rs` compiles; the shell itself is written
//! by the owner of this file.

pub fn run() {}
