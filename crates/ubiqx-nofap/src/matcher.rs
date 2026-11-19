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
//! 3. **Most specific match wins.** At equal specificity: user block, user allow, built-in
//!    never-block, learned, list. A top-level or second-level rule of the never-block file
//!    (`gov`, `gov.*`) is not a name of the walk: it protects everything under it from the
//!    list and from learned entries at any depth, and only the user's own entries beat it.
//! 4. **No match:** the class from never-capture and mixed (the longer rule wins, so a
//!    private web app can sit under a mixed platform); then an unexpired snooze; then an
//!    adult TLD and the keyword rules ([`crate::heuristics`]); then popular. A host under
//!    a never-capture or mixed rule is never blocked by a TLD or a keyword.
//! 5. Learned sites and snoozes are matched by [`site_key`] for the host and each parent.
//!
//! Cost: a verdict hashes the host and each parent once per table that can hold it and
//! runs a binary search over the embedded bytes; nothing is indexed when a matcher is
//! built except the user's own sites, and the hosts tier on its first use.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use ubiqx_core::nofap::{NofapSite, SiteList, SiteOrigin};
use ubiqx_core::CoreResult;

use crate::heuristics::{self, HostRule};
use crate::list::{self, HashTable};

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
    /// "Não era isso", not yet expired. Exempts the host from keyword and TLD rules only:
    /// the list, learned sites and the user's entries still decide first, and the AI check
    /// keeps looking at the host (at its signal cadence).
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
    exact: HashSet<String>,
    wildcard_labels: HashSet<String>,
}

/// How specific the rule is that covers a host: a top-level or second-level rule decides
/// at every depth, a name rule at its own number of labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Coverage {
    Name(usize),
    Suffix,
}

impl RuleSet {
    /// Builds the set from rule lines (already without comments and blanks).
    pub fn parse<'a>(rules: impl IntoIterator<Item = &'a str>) -> Self {
        let mut set = Self::default();
        for rule in rules {
            let rule = rule.trim().trim_end_matches('.').to_ascii_lowercase();
            if rule.is_empty() {
                continue;
            }
            match rule.strip_suffix(".*") {
                Some(label) => set.wildcard_labels.insert(label.to_owned()),
                None => set.exact.insert(rule),
            };
        }
        set
    }

    /// Whether `host` (normalised) falls under any rule, itself or through a parent.
    pub fn covers(&self, host: &str) -> bool {
        self.coverage(host).is_some()
    }

    pub fn len(&self) -> usize {
        self.exact.len() + self.wildcard_labels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.wildcard_labels.is_empty()
    }

    /// Whether a top-level (`gov`) or second-level (`gov.*`) rule covers `host`.
    fn suffix_covers(&self, host: &str) -> bool {
        let mut labels = host.rsplit('.');
        let last = labels.next().unwrap_or_default();
        self.exact.contains(last)
            || labels
                .next()
                .is_some_and(|second| self.wildcard_labels.contains(second))
    }

    /// Whether `name` itself is a name rule (no parent walk).
    fn has_name(&self, name: &str) -> bool {
        name.contains('.') && self.exact.contains(name)
    }

    fn coverage(&self, host: &str) -> Option<Coverage> {
        if self.suffix_covers(host) {
            return Some(Coverage::Suffix);
        }
        parents(host)
            .into_iter()
            .find(|name| self.exact.contains(*name))
            .map(|name| Coverage::Name(name.split('.').count()))
    }
}

/// The built-in never-block list (`data/never-block.txt`): operating-system vendors, update
/// and sign-in hosts, payment and banks, AI providers, `gov`, `gov.*`, `edu`, `edu.*`,
/// `ac.*`, `mil`, `jus.br`, sexual-health and education sites, known keyword collisions. It
/// is both the matcher's built-in allowlist and the helper's refusal list for user names.
pub fn never_block() -> &'static RuleSet {
    static RULES: OnceLock<RuleSet> = OnceLock::new();
    RULES.get_or_init(|| RuleSet::parse(list::never_block_rules().iter().copied()))
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
        Self {
            core: list::embedded_core(),
            popular: list::embedded_popular(),
            never_block: never_block().clone(),
            mixed: RuleSet::parse(list::mixed_rules().iter().copied()),
            never_capture: RuleSet::parse(list::never_capture_rules().iter().copied()),
            hosts_tier: list::hosts_tier(),
        }
    }
}

/// Decides hosts. Rebuilt only when `nofap_sites` changes; the list never enters SQLite.
#[derive(Debug)]
pub struct Matcher {
    data: MatcherData,
    core: HashTable<'static>,
    popular: HashTable<'static>,
    salt: [u8; 16],
    user_block: HashSet<String>,
    user_allow: HashSet<String>,
    learned: HashSet<[u8; 16]>,
    snoozed: HashSet<[u8; 16]>,
    /// The hosts tier without its `www.` twins, indexed on the first bypass check.
    hosts_tier: OnceLock<HashSet<&'static str>>,
}

impl Matcher {
    /// Validates both tables (bad magic, unsorted or duplicate keys and a wrong count are
    /// errors) and indexes `sites`; a snooze whose `expires_at` is at or before `now` is
    /// ignored, and so is one without an expiry (a snooze is never permanent).
    pub fn new(
        data: MatcherData,
        sites: &[NofapSite],
        salt: &[u8; 16],
        now: DateTime<Utc>,
    ) -> CoreResult<Self> {
        let core = HashTable::parse(data.core, list::CORE_TAG)?;
        let popular = HashTable::parse(data.popular, list::POPULAR_TAG)?;
        let mut matcher = Self {
            data,
            core,
            popular,
            salt: *salt,
            user_block: HashSet::new(),
            user_allow: HashSet::new(),
            learned: HashSet::new(),
            snoozed: HashSet::new(),
            hosts_tier: OnceLock::new(),
        };
        for site in sites {
            let typed = site.host.as_deref().and_then(normalize_host);
            match (site.list, site.origin) {
                (SiteList::Block, SiteOrigin::User) => matcher.user_block.extend(typed),
                (SiteList::Allow, SiteOrigin::User) => matcher.user_allow.extend(typed),
                (SiteList::Block, SiteOrigin::Learned) => {
                    matcher.learned.insert(site.host_key);
                }
                (SiteList::Allow, SiteOrigin::Snooze) => {
                    if site.expires_at.is_some_and(|expires| expires > now) {
                        matcher.snoozed.insert(site.host_key);
                    }
                }
                // A learned allow and a snoozed block are not things the app writes.
                (SiteList::Allow, SiteOrigin::Learned) | (SiteList::Block, SiteOrigin::Snooze) => {}
            }
        }
        Ok(matcher)
    }

    /// The verdict for a host name (any case, with or without port, user-info, `www.` and a
    /// trailing dot).
    pub fn verdict(&self, host: &str) -> Verdict {
        let Some(host) = normalize_host(host) else {
            return Verdict::Unknown(HostClass::Local);
        };
        if let Some(address) = ip_literal(&host) {
            let class = if is_local_address(address) {
                HostClass::Local
            } else {
                HostClass::Plain { signal: false }
            };
            return Verdict::Unknown(class);
        }
        if is_local_name(&host) {
            return Verdict::Unknown(HostClass::Local);
        }

        let names = parents(&host);
        let protected = self.data.never_block.suffix_covers(&host);
        let keys: Vec<[u8; 16]> = if self.learned.is_empty() && self.snoozed.is_empty() {
            Vec::new()
        } else {
            names
                .iter()
                .map(|name| site_key(&self.salt, name))
                .collect()
        };
        for (depth, name) in names.iter().enumerate() {
            if self.user_block.contains(*name) {
                return Verdict::Blocked(BlockSource::User);
            }
            if self.user_allow.contains(*name) {
                return Verdict::Allowed(AllowSource::User);
            }
            if protected {
                continue;
            }
            if self.data.never_block.has_name(name) {
                return Verdict::Allowed(AllowSource::BuiltIn);
            }
            if keys
                .get(depth)
                .is_some_and(|key| self.learned.contains(key))
            {
                return Verdict::Blocked(BlockSource::Learned);
            }
            if self.core.contains(name) {
                return Verdict::Blocked(BlockSource::List);
            }
        }
        if protected {
            return Verdict::Allowed(AllowSource::BuiltIn);
        }

        let never_capture = self.data.never_capture.coverage(&host);
        let mixed = self.data.mixed.coverage(&host);
        match (never_capture, mixed) {
            (Some(private), Some(platform)) if platform > private => {
                return Verdict::Unknown(HostClass::Mixed);
            }
            (Some(_), _) => return Verdict::Unknown(HostClass::NeverCapture),
            (None, Some(_)) => return Verdict::Unknown(HostClass::Mixed),
            (None, None) => {}
        }
        if keys.iter().any(|key| self.snoozed.contains(key)) {
            return Verdict::Allowed(AllowSource::Snooze);
        }
        let signal = match heuristics::host_rule(&host) {
            HostRule::Block(source) => return Verdict::Blocked(source),
            HostRule::Signal => true,
            HostRule::None => false,
        };
        if self.popular.contains(&host) {
            return Verdict::Unknown(HostClass::Popular);
        }
        Verdict::Unknown(HostClass::Plain { signal })
    }

    /// The verdict for a URL, and whether its path or query carries a block-grade word (a
    /// signal, never a block). A string that is not a URL with a host is `Unknown(Local)`.
    /// An address typed without its scheme (`example.com/page`) is read as `https`.
    pub fn verdict_for_url(&self, url: &str) -> (Verdict, bool) {
        let url = url.trim();
        let parsed = if url.contains("://") {
            url::Url::parse(url)
        } else {
            url::Url::parse(&format!("https://{url}"))
        };
        let Ok(parsed) = parsed else {
            return (Verdict::Unknown(HostClass::Local), false);
        };
        let Some(host) = parsed.host_str() else {
            return (Verdict::Unknown(HostClass::Local), false);
        };
        let signal = heuristics::path_signal(parsed.path())
            || parsed.query().is_some_and(heuristics::path_signal);
        (self.verdict(host), signal)
    }

    /// Whether the host, or a parent of it, is one of the names of the hosts tier (seen in
    /// front while the hosts block is applied, it means DNS is being bypassed).
    pub fn in_hosts_tier(&self, host: &str) -> bool {
        let Some(host) = normalize_host(host) else {
            return false;
        };
        let tier = self.hosts_tier.get_or_init(|| {
            self.data
                .hosts_tier
                .iter()
                .map(|name| strip_www(name))
                .collect()
        });
        parents(&host).into_iter().any(|name| tier.contains(name))
    }
}

/// Names that never leave the machine or the local network.
const LOCAL_SUFFIXES: &[&str] = &[
    "local",
    "localhost",
    "internal",
    "lan",
    "home.arpa",
    "test",
    "corp",
];

/// A single label, or a name under one of [`LOCAL_SUFFIXES`].
fn is_local_name(host: &str) -> bool {
    !host.contains('.')
        || LOCAL_SUFFIXES.iter().any(|suffix| {
            host.strip_suffix(suffix)
                .is_some_and(|rest| rest.is_empty() || rest.ends_with('.'))
        })
}

/// The address a normalised host stands for, when it is an IP literal.
fn ip_literal(host: &str) -> Option<IpAddr> {
    host.parse().ok()
}

/// Loopback, private, link-local, unique-local, shared (carrier-grade NAT) and unspecified
/// addresses: never an unknown website.
fn is_local_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => is_local_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_local_v4(v4),
            None => is_local_v6(v6),
        },
    }
}

fn is_local_v4(address: Ipv4Addr) -> bool {
    let [first, second, ..] = address.octets();
    address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_broadcast()
        || (first == 100 && (64..128).contains(&second))
}

fn is_local_v6(address: Ipv6Addr) -> bool {
    let first = address.segments()[0];
    address.is_loopback()
        || address.is_unspecified()
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
}

/// `name` without a leading `www.` label, when at least two labels remain.
fn strip_www(name: &str) -> &str {
    match name.strip_prefix("www.") {
        Some(rest) if rest.contains('.') => rest,
        _ => name,
    }
}

/// Normalises a host as rule 1 says; `None` for an empty or unusable string. A scheme, a
/// path, a query, user-info and a port are dropped, an internationalised name becomes
/// punycode, and an IP literal comes back in its canonical form without brackets.
pub fn normalize_host(host: &str) -> Option<String> {
    let mut text = host.trim();
    if let Some((_, rest)) = text.split_once("://") {
        text = rest;
    }
    text = text.split(['/', '?', '#']).next().unwrap_or_default();
    if let Some((_, rest)) = text.rsplit_once('@') {
        text = rest;
    }
    if let Some(bracketed) = text.strip_prefix('[') {
        let (address, _) = bracketed.split_once(']')?;
        return address
            .parse::<Ipv6Addr>()
            .ok()
            .map(|address| address.to_string());
    }
    if let Ok(address) = text.parse::<Ipv6Addr>() {
        return Some(address.to_string());
    }
    if let Some((name, port)) = text.rsplit_once(':') {
        if !port.bytes().all(|b| b.is_ascii_digit()) || name.contains(':') {
            return None;
        }
        text = name;
    }
    let text = text.trim_end_matches('.');
    if text.is_empty() {
        return None;
    }
    match url::Host::parse(text).ok()? {
        url::Host::Domain(name) => {
            let name = name.trim_end_matches('.');
            if name.is_empty() || name.split('.').any(str::is_empty) {
                return None;
            }
            Some(strip_www(name).to_owned())
        }
        url::Host::Ipv4(address) => Some(address.to_string()),
        url::Host::Ipv6(address) => Some(address.to_string()),
    }
}

/// The host and each parent of it down to two labels, most specific first
/// (`a.b.example.com` → `a.b.example.com`, `b.example.com`, `example.com`). A single label
/// has no such name and gives nothing.
pub fn parents(host: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = host;
    while rest.contains('.') {
        names.push(rest);
        rest = rest.split_once('.').map_or("", |(_, parent)| parent);
    }
    names
}

/// `SHA-256(salt ‖ 0x00 ‖ host)[..16]`: how a learned or snoozed site is stored.
pub fn site_key(salt: &[u8; 16], host: &str) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(salt)
        .chain_update([0u8])
        .chain_update(host.as_bytes())
        .finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&digest[..16]);
    key
}
