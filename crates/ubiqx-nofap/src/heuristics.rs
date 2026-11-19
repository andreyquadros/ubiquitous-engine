//! Keyword and TLD rules, applied only to hosts that no list, user entry or never-block
//! rule decides (design section 3.5).
//!
//! | Grade | Effect |
//! |---|---|
//! | Block | an adult TLD, a block-grade substring, or a brand stem after look-alike folding |
//! | Signal | a signal-grade whole token or TLD: raises vision priority, never blocks |
//! | Never | words that are not used at all (`sex`, `anal`, `gay`, `teen`, …) |
//!
//! Seeded by the foundation unit: signatures only. The word lists are data of this file
//! and are written by its owner from the design's table and
//! `docs/nofap/research/research-blocklists.md` section 9.
#![allow(unused_variables)]

use crate::matcher::BlockSource;

/// What the rules say about a host that nothing else decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostRule {
    /// [`BlockSource::Tld`] or [`BlockSource::Keyword`].
    Block(BlockSource),
    /// A signal-grade word or TLD.
    Signal,
    None,
}

/// The rule for a normalised host.
pub fn host_rule(host: &str) -> HostRule {
    todo!()
}

/// Whether a window title carries a block-grade word (a signal for the AI check).
pub fn title_signal(title: &str) -> bool {
    todo!()
}

/// Whether a URL path carries a block-grade word (a signal on a mixed host, never a block).
pub fn path_signal(path: &str) -> bool {
    todo!()
}

/// Look-alike folding for brand stems: `0→o 1→i 3→e 4→a 5→s 7→t vv→w`, digits at the edges
/// of a label stripped.
pub fn fold_lookalikes(label: &str) -> String {
    todo!()
}
