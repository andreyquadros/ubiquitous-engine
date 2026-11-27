//! The verdict for a host: blocked, allowed, or unknown with a class that tells the AI
//! check whether it may look.
//!
//! Semantics (design section 3.4):
//!
//! 1. **Normalise:** lower-case; strip a trailing dot, port, user-info, a leading `*.` or
//!    `.` (how other blockers write "and its subdomains") and `www.`. A single
//!    label, `localhost`, a loopback, private, link-local or unique-local IP literal, and a
//!    name under `.local`, `.localhost`, `.internal`, `.lan`, `.home.arpa`, `.test` or
//!    `.corp` is `Unknown(Local)`. A public IP literal is `Unknown(Plain { signal: false })`.
//! 2. **Suffix walk:** test the host, then drop the left-most label, down to two labels.
//!    Every entry covers itself and its subdomains.
//! 3. **Most specific match wins.** At equal specificity: user block, user allow, built-in
//!    never-block, learned, list. A top-level or second-level rule of the never-block file
//!    (`gov`, `gov.*`) is not a name of the walk: it protects everything under it from the
//!    list and from learned entries at any depth, and only the user's own entries beat it.
//!    A second-level rule reaches only under a country code ([`RuleSet`]): `gov.com` and
//!    `edu.xxx` are names anybody can register.
//! 4. **No match:** the class from never-capture and mixed (the longer rule wins, so a
//!    private web app can sit under a mixed platform); then an unexpired snooze; then an
//!    adult TLD and the keyword rules ([`crate::heuristics`]); then popular. A host under
//!    a never-capture or mixed rule is never blocked by a TLD or a keyword. A signal word
//!    in the name of a popular host makes it `Plain { signal: true }`: popular sites are
//!    looked at when a local signal fires, and that is one.
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
///   when that last label is a country code (`planalto.gov.br`, `my.gov.uz`): a country's
///   public sector. Under a generic or brand top-level domain (`gov.com`, `mil.win`,
///   `gov.porn`) and under [`OPEN_COUNTRY_CODES`] (`edu.eu`) such a name is a private
///   registration and the rule does not reach it; a public sector there is a name rule
///   (`gov.wales`). `gov` itself as a top-level domain is written `gov`;
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
            || (is_public_sector_country_code(last)
                && labels
                    .next()
                    .is_some_and(|second| self.wildcard_labels.contains(second)))
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

/// Country codes whose second level anybody may register and that have no public sector
/// of their own under it: `edu.eu` is a private name. (The European institutions are
/// `europa.eu`, a name rule.)
pub const OPEN_COUNTRY_CODES: &[&str] = &["eu"];

/// Whether a second-level rule reaches under the top-level domain `tld`: a country code
/// (two letters) that is not one of [`OPEN_COUNTRY_CODES`].
fn is_public_sector_country_code(tld: &str) -> bool {
    tld.len() == 2
        && tld.bytes().all(|b| b.is_ascii_lowercase())
        && !OPEN_COUNTRY_CODES.contains(&tld)
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
#[derive(Clone)]
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

/// Sizes only: the tables are megabytes and the tier is a list of adult sites.
impl std::fmt::Debug for MatcherData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatcherData")
            .field("core_bytes", &self.core.len())
            .field("popular_bytes", &self.popular.len())
            .field("never_block", &self.never_block.len())
            .field("mixed", &self.mixed.len())
            .field("never_capture", &self.never_capture.len())
            .field("hosts_tier", &self.hosts_tier.len())
            .finish()
    }
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

/// Counts only: a matcher printed into a log must not name the sites the user typed, nor
/// show the salt their learned sites are hashed with.
impl std::fmt::Debug for Matcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Matcher")
            .field("data", &self.data)
            .field("core", &self.core)
            .field("popular", &self.popular)
            .field("user_block", &self.user_block.len())
            .field("user_allow", &self.user_allow.len())
            .field("learned", &self.learned.len())
            .field("snoozed", &self.snoozed.len())
            .finish_non_exhaustive()
    }
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
        match heuristics::host_rule(&host) {
            HostRule::Block(source) => Verdict::Blocked(source),
            // A signal word in the name is a local signal, and a popular site with a local
            // signal is looked at like any other (design 4.2): the class says so.
            HostRule::Signal => Verdict::Unknown(HostClass::Plain { signal: true }),
            HostRule::None if self.popular.contains(&host) => Verdict::Unknown(HostClass::Popular),
            HostRule::None => Verdict::Unknown(HostClass::Plain { signal: false }),
        }
    }

    /// The verdict for a URL, and whether its path or query carries a block-grade word (a
    /// signal, never a block). A string that is not a URL with a host is `Unknown(Local)`.
    /// An address typed without its scheme (`example.com/page`) is read as `https`. The
    /// source view and the reader views of the browsers show a page under their own
    /// scheme, and a `blob:` or `filesystem:` address carries the origin of the site that
    /// made it: the verdict is the one of that page. A `file:` address of a picture or a
    /// video ([`MEDIA_EXTENSIONS`]) is `Unknown(Plain)`; any other `file:` address is
    /// `Unknown(Local)`.
    pub fn verdict_for_url(&self, url: &str) -> (Verdict, bool) {
        let url = url.trim();
        let shown = shown_page(url);
        let url = shown.as_deref().unwrap_or(url);
        let parsed = if scheme_end(url).is_some() {
            url::Url::parse(url)
        } else {
            url::Url::parse(&format!("https://{url}"))
        };
        let Ok(parsed) = parsed else {
            return (Verdict::Unknown(HostClass::Local), false);
        };
        let signal = || {
            heuristics::path_signal(parsed.path())
                || parsed.query().is_some_and(heuristics::path_signal)
        };
        if parsed.scheme() == "file" {
            // A picture or a video opened from the disk is shown as a player shows it, and
            // is looked at like an unknown site. A document, a page or a folder is not.
            return if is_media_file(parsed.path()) {
                (
                    Verdict::Unknown(HostClass::Plain { signal: false }),
                    signal(),
                )
            } else {
                (Verdict::Unknown(HostClass::Local), false)
            };
        }
        let Some(host) = parsed.host_str() else {
            return (Verdict::Unknown(HostClass::Local), false);
        };
        (self.verdict(host), signal())
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

/// The longest name the DNS carries, and the longest label of one.
const MAX_NAME_LEN: usize = 253;
const MAX_LABEL_LEN: usize = 63;

/// Where the `://` of a leading scheme ends (`https://example.com` → 8). A `://` further
/// in, as in `example.com/go?to=https://other.example`, is part of a path or a query and
/// not a scheme.
fn scheme_end(text: &str) -> Option<usize> {
    let (scheme, _) = text.split_once("://")?;
    let mut bytes = scheme.bytes();
    let valid = bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
    valid.then_some(scheme.len() + 3)
}

/// The address of the page behind a browser's wrapper, when `url` is one: the source view
/// (`view-source:https://…`), a `blob:` or `filesystem:` address (`blob:https://…/<id>`,
/// whose origin is the site that made it) and the reader views of Firefox
/// (`about:reader?url=…`), Edge (`read://…/?url=…`) and Chrome
/// (`chrome-distiller://…/?url=…`), which show the text and the pictures of the page named
/// in `url`.
fn shown_page(url: &str) -> Option<String> {
    for wrapper in ["view-source:", "blob:", "filesystem:"] {
        if url
            .get(..wrapper.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(wrapper))
        {
            return Some(url[wrapper.len()..].to_owned());
        }
    }
    let parsed = url::Url::parse(url).ok()?;
    let reader = match parsed.scheme() {
        "about" => parsed.path() == "reader",
        "read" | "chrome-distiller" => true,
        _ => false,
    };
    if !reader {
        return None;
    }
    parsed
        .query_pairs()
        .find(|(key, _)| key == "url")
        .map(|(_, page)| page.into_owned())
}

/// The extensions of the pictures and videos a browser shows by itself.
pub const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "jfif", "pjpeg", "png", "apng", "gif", "webp", "avif", "bmp", "ico", "tif",
    "tiff", "heic", "heif", "jxl", "mp4", "m4v", "webm", "mkv", "mov", "ogv", "ogg", "avi", "wmv",
    "flv", "mpg", "mpeg", "3gp", "3g2",
];

/// Whether the last segment of a `file:` path ends with one of [`MEDIA_EXTENSIONS`].
fn is_media_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or_default();
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty()
            && MEDIA_EXTENSIONS
                .iter()
                .any(|media| media.eq_ignore_ascii_case(extension))
    })
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

/// Normalises a host as rule 1 says; `None` for an empty or unusable string, for a label
/// with a character other than a letter, a digit, `-` or `_`, and for a name longer than
/// the DNS carries (253 bytes, 63 a label). A leading `*.` or `.` is dropped. A leading scheme, a path, a
/// query, user-info and a port are dropped, an internationalised name becomes punycode,
/// and an IP literal comes back in its canonical form without brackets.
pub fn normalize_host(host: &str) -> Option<String> {
    let mut text = host.trim();
    if let Some(end) = scheme_end(text) {
        text = &text[end..];
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
    // `*.example.com` and `.example.com` are how other blockers write "the name and its
    // subdomains", which is what every entry means here.
    let text = text
        .strip_prefix("*.")
        .or_else(|| text.strip_prefix('.'))
        .unwrap_or(text);
    if text.is_empty() {
        return None;
    }
    match url::Host::parse(text).ok()? {
        url::Host::Domain(name) => {
            let name = name.trim_end_matches('.');
            // A name the DNS cannot carry is not a website (and a verdict hashes the
            // name once per label, so its length is bounded here). A label is letters,
            // digits, hyphens and underscores: anything else (`*`, `!`, `$`) would be
            // stored and then never match a site.
            if name.is_empty()
                || name.len() > MAX_NAME_LEN
                || name.split('.').any(|label| {
                    label.is_empty()
                        || label.len() > MAX_LABEL_LEN
                        || !label
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                })
            {
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;
    use crate::list::{key_of, CORE_TAG, FORMAT, MAGIC, POPULAR_TAG};

    const SALT: [u8; 16] = [7; 16];

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap()
    }

    /// A table in the build script's format holding `names`, leaked: the matcher takes
    /// `'static` bytes, as the embedded tables are.
    fn table(tag: &[u8], names: &[&str]) -> &'static [u8] {
        let mut keys: Vec<u64> = names.iter().map(|name| key_of(tag, name)).collect();
        keys.sort_unstable();
        raw_table(&keys)
    }

    fn raw_table(keys: &[u64]) -> &'static [u8] {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&FORMAT.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&(keys.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&202611191200u64.to_be_bytes());
        for key in keys {
            bytes.extend_from_slice(&key.to_be_bytes());
        }
        Box::leak(bytes.into_boxed_slice())
    }

    /// A small world: a list, a popular table and the three rule files.
    fn data() -> MatcherData {
        MatcherData {
            core: table(
                CORE_TAG,
                &[
                    "pornhub.com",
                    "xvideos.com",
                    "listed.example",
                    "adult.contents.fc2.com",
                    "nsfw.reddit.com",
                    "spam.agency.gov.br",
                    "deep.one.two.listed-deep.example",
                ],
            ),
            popular: table(
                POPULAR_TAG,
                &["wikipedia.org", "bing.com", "popular.example"],
            ),
            never_block: RuleSet::parse([
                "gov",
                "gov.*",
                "ac.*",
                "edu",
                "edu.*",
                "mil.*",
                "apple.com",
                "fc2.com",
                "banco.bradesco",
                "xxxlutz.de",
            ]),
            mixed: RuleSet::parse([
                "reddit.com",
                "snapchat.com",
                "images.google.com",
                "tumblr.com",
            ]),
            never_capture: RuleSet::parse([
                "gov",
                "bank.example",
                "web.snapchat.com",
                "mail.google.com",
                "med.*",
            ]),
            hosts_tier: &[
                "pornhub.com",
                "www.pornhub.com",
                "www.sub.tier.example",
                "xvideos.com",
            ],
        }
    }

    fn site(list: SiteList, origin: SiteOrigin, host: &str) -> NofapSite {
        NofapSite {
            id: format!("site-{host}"),
            list,
            origin,
            host: (origin == SiteOrigin::User).then(|| host.to_owned()),
            host_key: site_key(&SALT, host),
            created_at: now(),
            expires_at: None,
        }
    }

    fn snooze(host: &str, expires_at: DateTime<Utc>) -> NofapSite {
        NofapSite {
            expires_at: Some(expires_at),
            ..site(SiteList::Allow, SiteOrigin::Snooze, host)
        }
    }

    fn matcher(sites: &[NofapSite]) -> Matcher {
        Matcher::new(data(), sites, &SALT, now()).unwrap()
    }

    const LIST: Verdict = Verdict::Blocked(BlockSource::List);
    const PLAIN: Verdict = Verdict::Unknown(HostClass::Plain { signal: false });
    const LOCAL: Verdict = Verdict::Unknown(HostClass::Local);

    #[test]
    fn listed_names_block_after_normalisation_and_through_the_parent_walk() {
        let m = matcher(&[]);
        for host in [
            "pornhub.com",
            "PT.PornHub.com.",
            "www.xvideos.com:443",
            "WWW.PORNHUB.COM",
            "a.b.c.pornhub.com",
            "user:secret@pornhub.com:8080",
            "https://pt.pornhub.com/view?x=1#top",
            "listed.example",
            "deep.one.two.listed-deep.example",
            "x.deep.one.two.listed-deep.example",
        ] {
            assert_eq!(m.verdict(host), LIST, "{host}");
            assert!(m.verdict(host).is_blocked());
        }
        // A parent of a listed name is not listed, nor is a name that only looks alike.
        for host in [
            "one.two.listed-deep.example",
            "listed-deep.example",
            "notlisted.example",
        ] {
            assert_eq!(m.verdict(host), PLAIN, "{host}");
        }
    }

    #[test]
    fn an_entry_stays_at_its_listed_depth_under_a_never_block_name() {
        let m = matcher(&[]);
        assert_eq!(m.verdict("adult.contents.fc2.com"), LIST);
        assert_eq!(m.verdict("x.adult.contents.fc2.com"), LIST);
        for host in ["fc2.com", "blog.fc2.com", "contents.fc2.com", "www.fc2.com"] {
            assert_eq!(
                m.verdict(host),
                Verdict::Allowed(AllowSource::BuiltIn),
                "{host}"
            );
        }
    }

    #[test]
    fn an_entry_stays_at_its_listed_depth_under_a_mixed_name() {
        let m = matcher(&[]);
        assert_eq!(m.verdict("nsfw.reddit.com"), LIST);
        assert_eq!(m.verdict("reddit.com"), Verdict::Unknown(HostClass::Mixed));
        assert_eq!(
            m.verdict("old.reddit.com"),
            Verdict::Unknown(HostClass::Mixed)
        );
    }

    #[test]
    fn keyword_and_tld_rules_decide_what_no_list_decides() {
        let m = matcher(&[]);
        for host in [
            "porno365.golf",
            "missav123.com",
            "xvv1deos.com",
            "videosdeputaria.com.br",
        ] {
            assert_eq!(
                m.verdict(host),
                Verdict::Blocked(BlockSource::Keyword),
                "{host}"
            );
        }
        assert_eq!(
            m.verdict("anything.xxx"),
            Verdict::Blocked(BlockSource::Tld)
        );
        assert_eq!(
            m.verdict("www.anything.sex"),
            Verdict::Blocked(BlockSource::Tld)
        );
        // A signal-grade word is a class, not a block.
        assert_eq!(
            m.verdict("nude-art.example"),
            Verdict::Unknown(HostClass::Plain { signal: true })
        );
        assert_eq!(
            m.verdict("cabinporn.example"),
            Verdict::Unknown(HostClass::Plain { signal: true })
        );
        assert_eq!(
            m.verdict("example.cam"),
            Verdict::Unknown(HostClass::Plain { signal: true })
        );
    }

    #[test]
    fn false_positive_traps_are_not_blocked() {
        let m = matcher(&[]);
        for host in [
            "essex.gov.uk",
            "sussex.ac.uk",
            "sussexexpress.co.uk",
            "planalto.gov.br",
            "canaltech.com.br",
            "motorola.com.br",
            "drogariasaopaulo.com.br",
            "peacocktv.com",
            "emilfrey.de",
            "milfordasset.com",
            "lichngaytot.com",
            "coroasparavelorio.com.br",
            "nakedwines.com",
            "skylinewebcams.com",
            "iowasexoffender.gov",
            "pussy888.com",
            "cambridge.org",
            "xxxlutz.de",
            "scunthorpe.co.uk",
            "penistone.co.uk",
            "ninjavan.co",
            "fapesp.br",
        ] {
            assert!(
                !m.verdict(host).is_blocked(),
                "{host}: {:?}",
                m.verdict(host)
            );
        }
        assert_eq!(
            m.verdict("essex.gov.uk"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        assert_eq!(
            m.verdict("sussex.ac.uk"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        assert_eq!(
            m.verdict("iowasexoffender.gov"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        assert_eq!(
            m.verdict("xxxlutz.de"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
    }

    #[test]
    fn a_suffix_rule_of_never_block_protects_at_every_depth() {
        let m = matcher(&[]);
        // Listed below a `gov.*` rule: the rule decides, also against a keyword and a TLD.
        assert_eq!(
            m.verdict("spam.agency.gov.br"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        assert_eq!(
            m.verdict("porn.agency.gov.br"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        assert_eq!(
            m.verdict("xxx.university.edu"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        // The shape is exact: the label must be the one before the last, or the last.
        assert_eq!(m.verdict("gov.example.com"), PLAIN);
        assert_eq!(m.verdict("xgov.br"), PLAIN);
        // A brand top-level domain is an ordinary name rule.
        assert_eq!(
            m.verdict("banco.bradesco"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        assert_eq!(
            m.verdict("www.banco.bradesco"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
        assert_eq!(m.verdict("outro.bradesco"), PLAIN);
        // Only the user's own entries beat the rule.
        let m = matcher(&[site(SiteList::Block, SiteOrigin::User, "agency.gov.br")]);
        assert_eq!(
            m.verdict("spam.agency.gov.br"),
            Verdict::Blocked(BlockSource::User)
        );
        assert_eq!(
            m.verdict("other.gov.br"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
    }

    /// `gov.com`, `mil.win`, `ac.cab` and `edu.eu` are names anybody can register, and
    /// `gov.porn` or `edu.xxx` would be too. A second-level rule means a country's public
    /// sector, so it reaches only under a country code that has one.
    #[test]
    fn a_second_level_rule_reaches_only_under_a_country_code() {
        let m = matcher(&[]);
        for host in [
            "videos.edu.xxx",
            "free.ac.sex",
            "gov.porn",
            "pornhub.gov.adult",
        ] {
            assert_eq!(
                m.verdict(host),
                Verdict::Blocked(BlockSource::Tld),
                "{host}"
            );
        }
        for host in [
            "hot.gov.com",
            "gov.com",
            "mil.win",
            "a.ac.cab",
            "campus.edu.eu",
        ] {
            let verdict = m.verdict(host);
            assert!(
                !matches!(verdict, Verdict::Allowed(_)),
                "{host}: {verdict:?}"
            );
        }
        assert_eq!(
            m.verdict("pornhub.gov.lol"),
            Verdict::Blocked(BlockSource::Keyword)
        );
        // A country's public sector is still protected, at every depth.
        for host in ["a.b.gov.br", "porn.agency.gov.br", "x.ac.uk", "gov.uz"] {
            assert_eq!(
                m.verdict(host),
                Verdict::Allowed(AllowSource::BuiltIn),
                "{host}"
            );
        }
    }

    /// Rules copied from another blocker are often written `*.example.com` or
    /// `.example.com`; an entry already covers its subdomains, so both mean the name.
    #[test]
    fn a_wildcard_typed_by_the_user_means_the_name() {
        let m = matcher(&[
            site(SiteList::Block, SiteOrigin::User, "*.adultsite-ana.example"),
            site(SiteList::Block, SiteOrigin::User, ".otheradult.example"),
            site(SiteList::Allow, SiteOrigin::User, "*.pornhub.com"),
        ]);
        for host in [
            "videos.adultsite-ana.example",
            "adultsite-ana.example",
            "a.otheradult.example",
        ] {
            assert_eq!(
                m.verdict(host),
                Verdict::Blocked(BlockSource::User),
                "{host}"
            );
        }
        assert_eq!(
            m.verdict("pt.pornhub.com"),
            Verdict::Allowed(AllowSource::User)
        );
    }

    /// A blob or filesystem address carries the origin of the site that made it; a local
    /// picture or video opened in a browser is shown as a player would show it. Any other
    /// local file (a document, a page, a folder) stays out of the check.
    #[test]
    fn wrapped_addresses_and_local_media_are_judged() {
        let m = matcher(&[]);
        for url in [
            "blob:https://pornhub.com/0b1c",
            "BLOB:https://pt.pornhub.com/0b1c",
            "filesystem:https://pornhub.com/temporary/a.mp4",
        ] {
            assert_eq!(m.verdict_for_url(url), (LIST, false), "{url}");
        }
        assert_eq!(
            m.verdict_for_url("blob:https://notlisted.example/0b1c"),
            (PLAIN, false)
        );
        assert_eq!(m.verdict_for_url("blob:null/0b1c"), (LOCAL, false));
        for url in [
            "file:///home/ana/Downloads/clip.mp4",
            "file:///home/ana/Pictures/IMG_0001.JPG",
            "file:///C:/Users/Ana/Videos/clip.webm",
            "file:///Users/ana/Movies/a%20b.mov",
        ] {
            assert_eq!(m.verdict_for_url(url), (PLAIN, false), "{url}");
        }
        assert_eq!(
            m.verdict_for_url("file:///home/ana/porn/clip.mkv"),
            (PLAIN, true)
        );
        for url in [
            "file:///home/ana/extrato.pdf",
            "file:///home/ana/notas.html",
            "file:///home/ana/Pictures/",
            "file:///home/ana/video.mp4.txt",
        ] {
            assert_eq!(m.verdict_for_url(url), (LOCAL, false), "{url}");
        }
    }

    #[test]
    fn a_url_gives_the_verdict_and_the_path_signal() {
        let m = matcher(&[]);
        assert_eq!(
            m.verdict_for_url("https://www.reddit.com/r/EarthPorn/"),
            (Verdict::Unknown(HostClass::Mixed), true)
        );
        assert_eq!(
            m.verdict_for_url("https://www.reddit.com/r/pics/"),
            (Verdict::Unknown(HostClass::Mixed), false)
        );
        assert_eq!(
            m.verdict_for_url("https://www.bing.com/images/search?q=hentai+wallpaper"),
            (Verdict::Unknown(HostClass::Popular), true)
        );
        assert_eq!(
            m.verdict_for_url("https://PT.PornHub.com./video"),
            (LIST, false)
        );
        // As a browser shows it: no scheme, perhaps a port.
        assert_eq!(m.verdict_for_url("www.xvideos.com:443/new"), (LIST, false));
        assert_eq!(
            m.verdict_for_url("reddit.com/r/EarthPorn"),
            (Verdict::Unknown(HostClass::Mixed), true)
        );
        assert_eq!(m.verdict_for_url("http://[::1]:8080/admin"), (LOCAL, false));
        assert_eq!(m.verdict_for_url("localhost:3000/porn"), (LOCAL, true));
        assert_eq!(m.verdict_for_url("https://bücher.example/"), (PLAIN, false));
        for not_a_site in [
            "",
            "   ",
            "about:blank",
            "file:///home/ana/notes.txt",
            "chrome://newtab/",
        ] {
            assert_eq!(
                m.verdict_for_url(not_a_site),
                (LOCAL, false),
                "{not_a_site:?}"
            );
        }
    }

    #[test]
    fn a_scheme_is_read_only_at_the_start_of_an_address() {
        let m = matcher(&[]);
        // As a browser shows it, without its scheme, with another address in the query.
        assert_eq!(
            m.verdict_for_url("pornhub.com/login?next=https://notlisted.example/"),
            (LIST, false)
        );
        assert_eq!(
            m.verdict_for_url("www.xvideos.com:443/out?to=http://notlisted.example"),
            (LIST, false)
        );
        // And the other way round: the page is the first host, not the one in the query
        // (whose name is still a signal).
        assert_eq!(
            m.verdict_for_url("notlisted.example/go?to=https://pornhub.com/"),
            (PLAIN, true)
        );
        assert_eq!(
            m.verdict_for_url("https://notlisted.example/go?to=https://listed.example/"),
            (PLAIN, false)
        );
        assert_eq!(
            m.verdict("pornhub.com/login?next=https://notlisted.example/"),
            LIST
        );
        assert_eq!(
            m.verdict("notlisted.example/go?to=https://pornhub.com/"),
            PLAIN
        );
        assert_eq!(
            normalize_host("example.com/r?u=https://other.example").as_deref(),
            Some("example.com")
        );
        assert_eq!(
            normalize_host("HTTPS://Example.com/r?u=ftp://other.example").as_deref(),
            Some("example.com")
        );
        assert_eq!(scheme_end("https://example.com"), Some(8));
        assert_eq!(scheme_end("chrome-extension://abc"), Some(19));
        assert_eq!(scheme_end("example.com/a?b=c://d"), None);
        assert_eq!(scheme_end("://example.com"), None);
        assert_eq!(scheme_end("1http://example.com"), None);
        assert_eq!(scheme_end("example.com"), None);
    }

    #[test]
    fn source_and_reader_views_get_the_verdict_of_the_page_they_show() {
        let m = matcher(&[]);
        for url in [
            "view-source:https://pt.pornhub.com/view",
            "VIEW-SOURCE:pt.pornhub.com/view",
            "about:reader?url=https%3A%2F%2Fpt.pornhub.com%2Fview",
            "read://https_pt.pornhub.com/?url=https%3A%2F%2Fpt.pornhub.com%2Fview",
            "chrome-distiller://0a1b_2c3d/?title=A+page&url=https%3A%2F%2Fpt.pornhub.com%2Fview",
        ] {
            assert_eq!(m.verdict_for_url(url), (LIST, false), "{url}");
        }
        // The class and the path signal are the page's too.
        assert_eq!(
            m.verdict_for_url("about:reader?url=https%3A%2F%2Fwww.reddit.com%2Fr%2FEarthPorn%2F"),
            (Verdict::Unknown(HostClass::Mixed), true)
        );
        assert_eq!(
            m.verdict_for_url("about:reader?url=https%3A%2F%2Fnotlisted.example%2Fnews"),
            (PLAIN, false)
        );
        // Anything else under those schemes is still not a site.
        for url in [
            "about:reader",
            "about:reader?url=",
            "about:blank?url=https%3A%2F%2Fpt.pornhub.com%2F",
            "about:config",
            "read://",
            "view-source:",
        ] {
            assert_eq!(m.verdict_for_url(url), (LOCAL, false), "{url:?}");
        }
    }

    #[test]
    fn a_name_the_dns_cannot_carry_is_not_a_site() {
        let m = matcher(&[]);
        let label63 = "a".repeat(63);
        let label64 = "a".repeat(64);
        assert_eq!(m.verdict(&format!("{label63}.pornhub.com")), LIST);
        assert_eq!(m.verdict(&format!("{label64}.pornhub.com")), LOCAL);
        assert_eq!(normalize_host(&format!("{label64}.example")), None);
        // 253 bytes pass, 254 do not.
        let name253 = format!("{}.pornhub.com", ["a"; 121].join("."));
        assert_eq!(name253.len(), 253);
        assert_eq!(m.verdict(&name253), LIST);
        assert_eq!(m.verdict(&format!("b{name253}")), LOCAL);
        // A verdict hashes a name once per label: a name of a hundred thousand labels
        // must be refused, not hashed a hundred thousand times.
        let started = std::time::Instant::now();
        let endless = format!("{}pornhub.com", "x.".repeat(100_000));
        assert_eq!(m.verdict(&endless), LOCAL);
        assert_eq!(m.verdict_for_url(&format!("https://{endless}/")).0, LOCAL);
        assert!(!m.in_hosts_tier(&endless));
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn local_names_and_addresses_are_local() {
        let m = matcher(&[site(SiteList::Block, SiteOrigin::User, "nas.local")]);
        for host in [
            "192.168.0.1",
            "192.168.0.1:8080",
            "10.1.2.3",
            "172.16.5.4",
            "169.254.10.10",
            "100.64.0.7",
            "127.0.0.1",
            "0.0.0.0",
            "[::1]",
            "[::1]:3000",
            "::1",
            "[fe80::1]",
            "[fd12:3456::1]",
            "[::ffff:192.168.1.1]",
            "localhost",
            "LOCALHOST.",
            "nas.local",
            "printer.lan",
            "app.localhost",
            "git.internal",
            "router.home.arpa",
            "site.test",
            "wiki.corp",
            "intranet",
            "",
            "   ",
            ".",
            "exa mple.com",
            "a..b.com",
            "host:port",
        ] {
            assert_eq!(m.verdict(host), LOCAL, "{host:?}");
        }
        // The suffix must be a whole label.
        assert_eq!(m.verdict("mylocal.example"), PLAIN);
        assert_eq!(m.verdict("example.corporate"), PLAIN);
    }

    #[test]
    fn a_public_address_is_an_unknown_site() {
        let m = matcher(&[]);
        for host in [
            "93.184.216.34",
            "93.184.216.34:443",
            "[2606:2800:220:1::1]",
            "172.32.0.1",
        ] {
            assert_eq!(m.verdict(host), PLAIN, "{host}");
        }
    }

    #[test]
    fn user_entries_follow_the_precedence() {
        // Equal specificity: block beats allow.
        let m = matcher(&[
            site(SiteList::Allow, SiteOrigin::User, "example.com"),
            site(SiteList::Block, SiteOrigin::User, "example.com"),
        ]);
        assert_eq!(
            m.verdict("example.com"),
            Verdict::Blocked(BlockSource::User)
        );
        assert_eq!(
            m.verdict("www.example.com"),
            Verdict::Blocked(BlockSource::User)
        );
        assert_eq!(
            m.verdict("sub.example.com"),
            Verdict::Blocked(BlockSource::User)
        );

        // The longer name wins, whichever way.
        let m = matcher(&[
            site(SiteList::Allow, SiteOrigin::User, "sub.listed.example"),
            site(SiteList::Block, SiteOrigin::User, "strict.example"),
            site(SiteList::Allow, SiteOrigin::User, "ok.strict.example"),
            site(SiteList::Allow, SiteOrigin::User, "Pornhub.com"),
            site(
                SiteList::Block,
                SiteOrigin::User,
                "https://www.Apple.com/shop",
            ),
            site(SiteList::Block, SiteOrigin::User, "tumblr.com"),
        ]);
        assert_eq!(
            m.verdict("sub.listed.example"),
            Verdict::Allowed(AllowSource::User)
        );
        assert_eq!(
            m.verdict("x.sub.listed.example"),
            Verdict::Allowed(AllowSource::User)
        );
        assert_eq!(m.verdict("listed.example"), LIST);
        assert_eq!(m.verdict("other.listed.example"), LIST);
        assert_eq!(
            m.verdict("strict.example"),
            Verdict::Blocked(BlockSource::User)
        );
        assert_eq!(
            m.verdict("ok.strict.example"),
            Verdict::Allowed(AllowSource::User)
        );
        // A user allow beats the list at the same name, and a user block beats never-block.
        assert_eq!(
            m.verdict("pornhub.com"),
            Verdict::Allowed(AllowSource::User)
        );
        assert_eq!(m.verdict("apple.com"), Verdict::Blocked(BlockSource::User));
        // "Modo estrito": a mixed platform blocked whole.
        assert_eq!(
            m.verdict("blog.tumblr.com"),
            Verdict::Blocked(BlockSource::User)
        );
        // Printed, a matcher counts the user's sites and names none of them.
        let printed = format!("{m:?}");
        assert!(
            printed.contains("user_block: 3") && printed.contains("user_allow: 3"),
            "{printed}"
        );
        assert!(
            !printed.contains("strict.example") && !printed.contains("tumblr"),
            "{printed}"
        );
        assert!(printed.len() < 600, "{printed}");
        // A user allow also lifts the keyword rules for its name.
        let m = matcher(&[site(SiteList::Allow, SiteOrigin::User, "porno365.golf")]);
        assert_eq!(
            m.verdict("porno365.golf"),
            Verdict::Allowed(AllowSource::User)
        );
    }

    #[test]
    fn a_learned_site_blocks_by_its_key_only() {
        let learned = site(SiteList::Block, SiteOrigin::Learned, "quiet-name.example");
        assert_eq!(learned.host, None);
        let m = matcher(std::slice::from_ref(&learned));
        assert_eq!(
            m.verdict("quiet-name.example"),
            Verdict::Blocked(BlockSource::Learned)
        );
        assert_eq!(
            m.verdict("www.quiet-name.example"),
            Verdict::Blocked(BlockSource::Learned)
        );
        assert_eq!(
            m.verdict("cdn.quiet-name.example"),
            Verdict::Blocked(BlockSource::Learned)
        );
        assert_eq!(m.verdict("other-name.example"), PLAIN);
        // The matcher holds the key, never the name.
        assert!(!format!("{m:?}").contains("quiet-name"));
        assert!(format!("{m:?}").contains("learned: 1"));
        // Another salt gives another key: the same row decides nothing.
        let other = Matcher::new(data(), &[learned], &[9; 16], now()).unwrap();
        assert_eq!(other.verdict("quiet-name.example"), PLAIN);
        // The built-in allowlist beats a learned key at the same name.
        let m = matcher(&[site(SiteList::Block, SiteOrigin::Learned, "apple.com")]);
        assert_eq!(
            m.verdict("apple.com"),
            Verdict::Allowed(AllowSource::BuiltIn)
        );
    }

    #[test]
    fn a_snooze_lifts_keyword_and_tld_rules_until_it_expires() {
        let later = now() + Duration::hours(24);
        let m = matcher(&[
            snooze("porno365.golf", later),
            snooze("anything.xxx", later),
        ]);
        assert_eq!(
            m.verdict("porno365.golf"),
            Verdict::Allowed(AllowSource::Snooze)
        );
        assert_eq!(
            m.verdict("www.porno365.golf"),
            Verdict::Allowed(AllowSource::Snooze)
        );
        assert_eq!(
            m.verdict("anything.xxx"),
            Verdict::Allowed(AllowSource::Snooze)
        );
        assert_eq!(m.verdict("other.xxx"), Verdict::Blocked(BlockSource::Tld));

        // Expired, expiring this very second, or without an expiry: ignored.
        for expired in [
            snooze("porno365.golf", now() - Duration::seconds(1)),
            snooze("porno365.golf", now()),
            site(SiteList::Allow, SiteOrigin::Snooze, "porno365.golf"),
        ] {
            let m = matcher(&[expired]);
            assert_eq!(
                m.verdict("porno365.golf"),
                Verdict::Blocked(BlockSource::Keyword)
            );
        }

        // It lifts nothing else: the list, a learned key and a user block still decide.
        let m = matcher(&[
            snooze("pornhub.com", later),
            snooze("learned.example", later),
            site(SiteList::Block, SiteOrigin::Learned, "learned.example"),
            snooze("mine.example", later),
            site(SiteList::Block, SiteOrigin::User, "mine.example"),
        ]);
        assert_eq!(m.verdict("pornhub.com"), LIST);
        assert_eq!(
            m.verdict("learned.example"),
            Verdict::Blocked(BlockSource::Learned)
        );
        assert_eq!(
            m.verdict("mine.example"),
            Verdict::Blocked(BlockSource::User)
        );
    }

    #[test]
    fn rows_the_app_never_writes_decide_nothing() {
        let m = matcher(&[
            site(SiteList::Allow, SiteOrigin::Learned, "pornhub.com"),
            NofapSite {
                expires_at: Some(now() + Duration::hours(1)),
                ..site(SiteList::Block, SiteOrigin::Snooze, "example.com")
            },
            NofapSite {
                host: None,
                ..site(SiteList::Block, SiteOrigin::User, "nameless.example")
            },
            NofapSite {
                host: Some("not a host".into()),
                ..site(SiteList::Block, SiteOrigin::User, "broken.example")
            },
        ]);
        assert_eq!(m.verdict("pornhub.com"), LIST);
        assert_eq!(m.verdict("example.com"), PLAIN);
        assert_eq!(m.verdict("nameless.example"), PLAIN);
        assert_eq!(m.verdict("broken.example"), PLAIN);
    }

    #[test]
    fn classes_of_undecided_hosts() {
        let m = matcher(&[]);
        assert_eq!(
            m.verdict("bank.example"),
            Verdict::Unknown(HostClass::NeverCapture)
        );
        assert_eq!(
            m.verdict("login.bank.example"),
            Verdict::Unknown(HostClass::NeverCapture)
        );
        assert_eq!(m.verdict("tumblr.com"), Verdict::Unknown(HostClass::Mixed));
        assert_eq!(
            m.verdict("wikipedia.org"),
            Verdict::Unknown(HostClass::Popular)
        );
        assert_eq!(
            m.verdict("www.wikipedia.org"),
            Verdict::Unknown(HostClass::Popular)
        );
        // Popular is the exact host: a page on a popular platform is not known to be popular.
        assert_eq!(m.verdict("someone.popular.example"), PLAIN);
        assert_eq!(m.verdict("unknown.example"), PLAIN);
        // A rule file protects from keyword and TLD rules.
        assert_eq!(
            m.verdict("pornhub-fans.tumblr.com"),
            Verdict::Unknown(HostClass::Mixed)
        );
        assert_eq!(
            m.verdict("xxx.bank.example"),
            Verdict::Unknown(HostClass::NeverCapture)
        );
        // The longer rule wins between the two files; never-capture wins a tie.
        assert_eq!(
            m.verdict("snapchat.com"),
            Verdict::Unknown(HostClass::Mixed)
        );
        assert_eq!(
            m.verdict("story.snapchat.com"),
            Verdict::Unknown(HostClass::Mixed)
        );
        assert_eq!(
            m.verdict("web.snapchat.com"),
            Verdict::Unknown(HostClass::NeverCapture)
        );
        assert_eq!(
            m.verdict("mail.google.com"),
            Verdict::Unknown(HostClass::NeverCapture)
        );
        assert_eq!(
            m.verdict("images.google.com"),
            Verdict::Unknown(HostClass::Mixed)
        );
        // A second-level rule of never-capture covers every depth below it.
        assert_eq!(
            m.verdict("clinic.med.br"),
            Verdict::Unknown(HostClass::NeverCapture)
        );
        assert_eq!(
            m.verdict("a.b.clinic.med.br"),
            Verdict::Unknown(HostClass::NeverCapture)
        );
        // A signal word in the name of a popular host is a local signal: the class says it.
        let mut with_popular = data();
        with_popular.popular = table(POPULAR_TAG, &["nude-project.example", "calm.example"]);
        let m = Matcher::new(with_popular, &[], &SALT, now()).unwrap();
        assert_eq!(
            m.verdict("nude-project.example"),
            Verdict::Unknown(HostClass::Plain { signal: true })
        );
        assert_eq!(
            m.verdict("calm.example"),
            Verdict::Unknown(HostClass::Popular)
        );
    }

    #[test]
    fn malformed_tables_fail_the_matcher() {
        assert!(Matcher::new(data(), &[], &SALT, now()).is_ok());
        let good = table(CORE_TAG, &["a.example", "b.example", "c.example"]);
        let mut bad_magic = good.to_vec();
        bad_magic[3] = b'?';
        let mut wrong_count = good.to_vec();
        wrong_count[15] = 2;
        let cases: [(&str, &'static [u8]); 6] = [
            ("bad magic", Box::leak(bad_magic.into_boxed_slice())),
            ("unsorted", raw_table(&[5, 4, 6])),
            ("duplicate keys", raw_table(&[4, 4, 6])),
            ("wrong count", Box::leak(wrong_count.into_boxed_slice())),
            ("truncated", &good[..good.len() - 3]),
            ("empty file", &[]),
        ];
        for (what, bytes) in cases {
            let mut broken_core = data();
            broken_core.core = bytes;
            assert!(
                Matcher::new(broken_core, &[], &SALT, now()).is_err(),
                "core: {what}"
            );
            let mut broken_popular = data();
            broken_popular.popular = bytes;
            assert!(
                Matcher::new(broken_popular, &[], &SALT, now()).is_err(),
                "popular: {what}"
            );
        }
    }

    #[test]
    fn the_hosts_tier_is_found_through_parents_and_www() {
        let m = matcher(&[]);
        for host in [
            "pornhub.com",
            "www.pornhub.com",
            "pt.pornhub.com",
            "PORNHUB.COM.",
            "xvideos.com:443",
            "sub.tier.example",
            "www.sub.tier.example",
            "x.sub.tier.example",
        ] {
            assert!(m.in_hosts_tier(host), "{host}");
        }
        for host in [
            "tier.example",
            "example.com",
            "",
            "localhost",
            "notpornhub.com",
        ] {
            assert!(!m.in_hosts_tier(host), "{host}");
        }
    }

    #[test]
    fn rule_sets_read_the_three_forms() {
        let rules = RuleSet::parse([
            "example.com",
            "gov",
            "gov.*",
            " Mixed.Example. ",
            "",
            "ac.*",
        ]);
        assert_eq!(rules.len(), 5);
        assert!(!rules.is_empty());
        for host in [
            "example.com",
            "a.b.example.com",
            "mixed.example",
            "x.mixed.example",
            "agency.gov",
            "a.b.agency.gov",
            "gov.br",
            "planalto.gov.br",
            "a.b.gov.uk",
            "sussex.ac.uk",
        ] {
            assert!(rules.covers(host), "{host}");
        }
        for host in [
            // Not under a country code: a name rule must say so (gov.wales).
            "x.gov.wales",
            "gov.com",
            "edu.eu",
            "x.ac.xxx",
            "example.org",
            "notexample.com",
            "com",
            "xgov.br",
            "gov.example.br",
            "governo.br",
            "ac.example.org",
            "mixed",
        ] {
            assert!(!rules.covers(host), "{host}");
        }
        let empty = RuleSet::parse([]);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(!empty.covers("example.com"));
        // The embedded never-block list is parsed once and shared.
        assert!(std::ptr::eq(never_block(), never_block()));
        assert!(!never_block().is_empty());
    }

    #[test]
    fn hosts_are_normalised() {
        for (raw, expected) in [
            ("Example.COM", "example.com"),
            ("example.com.", "example.com"),
            ("www.example.com", "example.com"),
            ("WWW.Example.Com.:8443", "example.com"),
            ("www.www.example.com", "www.example.com"),
            ("www.com", "www.com"),
            ("www", "www"),
            ("ana:secret@example.com", "example.com"),
            (
                "https://ana@www.example.com:443/path?q=1#frag",
                "example.com",
            ),
            ("  example.com  ", "example.com"),
            ("bücher.example", "xn--bcher-kva.example"),
            ("BÜCHER.example", "xn--bcher-kva.example"),
            ("xn--bcher-kva.example", "xn--bcher-kva.example"),
            ("192.168.0.1", "192.168.0.1"),
            ("192.168.0.1:8080", "192.168.0.1"),
            ("[::1]", "::1"),
            ("[::1]:8080", "::1"),
            ("::1", "::1"),
            ("[2001:DB8::1]", "2001:db8::1"),
            ("localhost", "localhost"),
            ("a_b.example.com", "a_b.example.com"),
            ("*.example.com", "example.com"),
            ("*.www.example.com", "example.com"),
            (".example.com", "example.com"),
            ("https://*.example.com/x", "example.com"),
            ("*.example.com:443", "example.com"),
        ] {
            assert_eq!(normalize_host(raw).as_deref(), Some(expected), "{raw}");
        }
        for raw in [
            "",
            "   ",
            ".",
            "...",
            ":80",
            "exa mple.com",
            "a..b",
            "[::1",
            "[nope]",
            "a:b:c",
            "x:port",
            "*.",
            "*",
            "*.*.example.com",
            "a.*.example.com",
            "ex*ample.com",
            "**.example.com",
            "example!.com",
            "exa$mple.com",
            "(example).com",
        ] {
            assert_eq!(normalize_host(raw), None, "{raw:?}");
        }
    }

    #[test]
    fn parents_stop_at_two_labels() {
        assert_eq!(
            parents("a.b.example.com"),
            ["a.b.example.com", "b.example.com", "example.com"]
        );
        assert_eq!(parents("example.com"), ["example.com"]);
        assert_eq!(
            parents("sub.example.co.uk"),
            ["sub.example.co.uk", "example.co.uk", "co.uk"]
        );
        assert!(parents("localhost").is_empty());
        assert!(parents("").is_empty());
    }

    #[test]
    fn the_site_key_is_a_salted_hash() {
        let salt: [u8; 16] = std::array::from_fn(|at| at as u8);
        // SHA-256(00 01 .. 0f ‖ 00 ‖ "example.com"), first 16 bytes.
        let expected = [
            0xd5, 0x90, 0x71, 0x42, 0xcc, 0x28, 0x7c, 0xed, 0x18, 0x31, 0xaf, 0x0e, 0x3b, 0xd5,
            0xf9, 0x70,
        ];
        assert_eq!(site_key(&salt, "example.com"), expected);
        assert_ne!(
            site_key(&salt, "example.com"),
            site_key(&salt, "example.org")
        );
        assert_ne!(
            site_key(&salt, "example.com"),
            site_key(&SALT, "example.com")
        );
    }
}
