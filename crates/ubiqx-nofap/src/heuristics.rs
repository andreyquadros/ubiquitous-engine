//! Keyword and TLD rules, applied only to hosts that no list, user entry or never-block
//! rule decides (design section 3.5).
//!
//! | Grade | Effect |
//! |---|---|
//! | Block | an adult TLD, a block-grade substring, or a brand stem after look-alike folding |
//! | Signal | a signal-grade whole token or TLD: raises vision priority, never blocks |
//! | Never | words that are not used at all (`sex`, `anal`, `gay`, `teen`, …) |
//!
//! Seeded by the foundation unit: the word lists of the design's table and of
//! `docs/nofap/research/research-blocklists.md` section 9.1, and the signatures. The owner
//! of this file tunes the lists against the false-positive traps of design section 3.6.
#![allow(unused_variables)]

use crate::matcher::BlockSource;

/// Top-level domains that block.
pub const BLOCK_TLDS: &[&str] = &["xxx", "porn", "sex", "sexy", "adult"];

/// Block-grade substrings, safe anywhere in a name.
pub const BLOCK_SUBSTRINGS: &[&str] = &[
    "porn",
    "xxx",
    "hentai",
    "blowjob",
    "gangbang",
    "cumshot",
    "handjob",
    "bukkake",
    "deepthroat",
    "creampie",
    "camgirl",
    "shemale",
    "upskirt",
    "masturbat",
    "bokep",
    "phimsex",
    "nudes",
    "putas",
    "escorts",
    // PT-BR
    "putaria",
    "buceta",
    "bucetinha",
    "siririca",
    "contoseroticos",
    "brasileirinhas",
    "pornozao",
    "safadas",
    "novinhas",
    "gostosas",
    "peladas",
    "transando",
    "gozando",
];

/// Brand stems, block-grade as substrings after [`fold_lookalikes`].
pub const BRAND_STEMS: &[&str] = &[
    "pornhub",
    "xvideos",
    "xnxx",
    "xhamster",
    "redtube",
    "youporn",
    "spankbang",
    "eporner",
    "chaturbate",
    "stripchat",
    "bongacams",
    "livejasmin",
    "cam4",
    "myfreecams",
    "onlyfans",
    "fansly",
    "brazzers",
    "missav",
    "nhentai",
    "hanime",
    "rule34",
    "e-hentai",
    "fapello",
    "erome",
    "coomer",
    "kemono",
    "simpcity",
    "noodlemagazine",
    "sxyprn",
    "faphouse",
    "javhd",
    "supjav",
];

/// Signal-grade whole tokens: they raise vision priority and never block. `pussy` counts
/// only when it is not followed by digits.
pub const SIGNAL_TOKENS: &[&str] = &[
    "milf",
    "sexy",
    "adult",
    "escort",
    "naked",
    "nude",
    "pussy",
    "sexo",
    "bdsm",
    "webcam",
    "cams",
    "erotic",
    "erotica",
    "fetish",
    "voyeur",
    "fuck",
    "amadoras",
    "acompanhantes",
];

/// Signal-grade top-level domains.
pub const SIGNAL_TLDS: &[&str] = &["cam", "webcam", "tube"];

/// Words that are not used at all, neither to block nor as a signal. `gay` is never a
/// signal: an orientation is not a risk marker.
pub const NEVER_WORDS: &[&str] = &[
    "sex", "anal", "cum", "ass", "cock", "dick", "tits", "cam", "jav", "gay", "teen", "incest",
    "pau", "rola", "pica", "puta", "bunda", "pelada", "foda",
];

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
