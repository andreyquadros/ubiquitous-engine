//! The verdict for a host: blocked, allowed, or unknown with a class that tells the AI
//! check whether it may look.
//!
//! Semantics (design section 3.4):
//!
//! 1. **Normalise:** lower-case; strip a trailing dot, port, user-info and `www.`. A single
//!    label, `localhost`, a loopback, private, link-local or unique-local IP literal, and a
//!    name under `.local`, `.localhost`, `.internal`, `.lan`, `.home.arpa`, `.test` or
//!    `.corp` is `Unknown(Local)`. A public IP literal is `Unknown(Plain { signal: false })`.
//! 2. **Suffix walk:** test the host, then drop the left-most label, down to two labels.
//!    Every entry covers itself and its subdomains.
//! 3. **Most specific match wins.** At equal specificity: user block, user allow (including
//!    an unexpired snooze), built-in never-block, learned, list.
//! 4. **No match:** adult TLD, then keyword rules ([`crate::heuristics`]), then the class
//!    from never-capture, mixed, popular.
//! 5. Learned sites and snoozes are matched by [`site_key`] for the host and each parent.
//!
//! Seeded by the foundation unit: complete types, signatures only.
#![allow(unused_variables, dead_code)]

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::NofapSite;
use ubiqx_core::CoreResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Blocked(BlockSource),
    Allowed(AllowSource),
    Unknown(HostClass),
}

impl Verdict {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Verdict::Blocked(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockSource {
    /// The user's own list.
    User,
    /// Learned from a vision positive (hash only).
    Learned,
    /// The embedded blocklist.
    List,
    /// An adult top-level domain.
    Tld,
    /// A block-grade keyword or brand stem.
    Keyword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllowSource {
    /// "Nunca bloquear".
    User,
    /// "Não era isso", not yet expired. Exempts the host from keyword and TLD rules only.
    Snooze,
    /// The built-in never-block list.
    BuiltIn,
}

/// What an undecided host is, for the AI check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostClass {
    /// A router page, a development server, an intranet: never blocked by a rule, never
    /// captured.
    Local,
    /// Banks, webmail, health, government, password managers: never captured.
    NeverCapture,
    /// A platform that cannot be split by domain: captured at signal cadence.
    Mixed,
    /// A well-known site that is on no list: captured only when a local signal fires.
    Popular,
    /// Anything else. `signal` is true when a signal-grade word is in the host.
    Plain { signal: bool },
}

/// A set of suffix rules read from a hand-maintained data file. One rule per line:
///
/// * `example.com` covers `example.com` and every subdomain;
/// * `gov.*` covers any name that has the label `gov` immediately before its last label
///   (`planalto.gov.br`), and `gov` itself as a top-level domain when written `gov`;
/// * a single label (`mil`, `edu`) covers that top-level domain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleSet {
    exact: std::collections::HashSet<String>,
    wildcard_labels: std::collections::HashSet<String>,
}

impl RuleSet {
    /// Builds the set from rule lines (already without comments and blanks).
    pub fn parse<'a>(rules: impl IntoIterator<Item = &'a str>) -> Self {
        todo!()
    }

    /// Whether `host` (normalised) falls under any rule, itself or through a parent.
    pub fn covers(&self, host: &str) -> bool {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

/// The built-in never-block list (`data/never-block.txt`): operating-system vendors, update
/// and sign-in hosts, payment and banks, AI providers, `gov`, `gov.*`, `edu`, `edu.*`,
/// `ac.*`, `mil`, `jus.br`, sexual-health and education sites, known keyword collisions. It
/// is both the matcher's built-in allowlist and the helper's refusal list for user names.
pub fn never_block() -> &'static RuleSet {
    todo!()
}

/// Everything a [`Matcher`] reads besides the user's sites.
#[derive(Debug, Clone)]
pub struct MatcherData {
    /// A table in the format of [`crate::list`], tag [`crate::list::CORE_TAG`].
    pub core: &'static [u8],
    /// A table with tag [`crate::list::POPULAR_TAG`].
    pub popular: &'static [u8],
    pub never_block: RuleSet,
    pub mixed: RuleSet,
    pub never_capture: RuleSet,
    /// The names of the hosts tier, for [`Matcher::in_hosts_tier`].
    pub hosts_tier: &'static [&'static str],
}

impl MatcherData {
    /// The data embedded in this build.
    pub fn embedded() -> Self {
        todo!()
    }
}

/// Decides hosts. Rebuilt only when `nofap_sites` changes; the list never enters SQLite.
#[derive(Debug)]
pub struct Matcher {
    data: MatcherData,
    salt: [u8; 16],
    user_block: std::collections::HashSet<String>,
    user_allow: std::collections::HashSet<String>,
    learned: std::collections::HashSet<[u8; 16]>,
    snoozed: std::collections::HashSet<[u8; 16]>,
    hosts_tier: std::collections::HashSet<&'static str>,
}

impl Matcher {
    /// Validates both tables (bad magic, unsorted or duplicate keys and a wrong count are
    /// errors) and indexes `sites`; a snooze whose `expires_at` is at or before `now` is
    /// ignored.
    pub fn new(
        data: MatcherData,
        sites: &[NofapSite],
        salt: &[u8; 16],
        now: DateTime<Utc>,
    ) -> CoreResult<Self> {
        todo!()
    }

    /// The verdict for a host name (any case, with or without port, user-info, `www.` and a
    /// trailing dot).
    pub fn verdict(&self, host: &str) -> Verdict {
        todo!()
    }

    /// The verdict for a URL, and whether its path carries a block-grade word (a signal,
    /// never a block). A string that is not a URL with a host is `Unknown(Local)`.
    pub fn verdict_for_url(&self, url: &str) -> (Verdict, bool) {
        todo!()
    }

    /// Whether the host, or a parent of it, is one of the names of the hosts tier (seen in
    /// front while the hosts block is applied, it means DNS is being bypassed).
    pub fn in_hosts_tier(&self, host: &str) -> bool {
        todo!()
    }
}

/// Normalises a host as rule 1 says; `None` for an empty or unusable string.
pub fn normalize_host(host: &str) -> Option<String> {
    todo!()
}

/// The host and each parent of it down to two labels, most specific first
/// (`a.b.example.com` → `a.b.example.com`, `b.example.com`, `example.com`).
pub fn parents(host: &str) -> Vec<&str> {
    todo!()
}

/// `SHA-256(salt ‖ 0x00 ‖ host)[..16]`: how a learned or snoozed site is stored.
pub fn site_key(salt: &[u8; 16], host: &str) -> [u8; 16] {
    todo!()
}
