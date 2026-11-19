//! The matcher against the data this build embeds: every row of design section 3.6, the
//! integrity of the embedded files, and the integrator's decisions on single names.
//!
//! Counts are read from `data/core.meta.json`, which the build script writes together with
//! the tables; nothing here is a number copied from a build.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use ubiqx_core::nofap::{NofapSite, SiteList, SiteOrigin};
use ubiqx_nofap::list::{self, HashTable, CORE_TAG, HEADER_LEN, POPULAR_TAG};
use ubiqx_nofap::matcher::{
    never_block, site_key, AllowSource, BlockSource, HostClass, Matcher, MatcherData, RuleSet,
    Verdict,
};

const SALT: [u8; 16] = [3; 16];
const LIST: Verdict = Verdict::Blocked(BlockSource::List);
const BUILT_IN: Verdict = Verdict::Allowed(AllowSource::BuiltIn);
const MIXED: Verdict = Verdict::Unknown(HostClass::Mixed);
const NEVER_CAPTURE: Verdict = Verdict::Unknown(HostClass::NeverCapture);
const POPULAR: Verdict = Verdict::Unknown(HostClass::Popular);
const LOCAL: Verdict = Verdict::Unknown(HostClass::Local);
const PLAIN: Verdict = Verdict::Unknown(HostClass::Plain { signal: false });

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap()
}

fn matcher(sites: &[NofapSite]) -> Matcher {
    Matcher::new(MatcherData::embedded(), sites, &SALT, now()).unwrap()
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

fn meta() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/core.meta.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn count(value: &Value) -> usize {
    usize::try_from(value.as_u64().expect("a count in core.meta.json")).unwrap()
}

/// `hosts::is_valid_hostname` as design 2.5 words it, written out here because the hosts
/// module is another unit's: ASCII letters, digits, hyphen and dots; 4 to 253 bytes; at
/// least two labels, none empty, longer than 63 bytes or with a hyphen at an edge; not an
/// IP literal.
fn is_valid_hostname(name: &str) -> bool {
    let labels: Vec<&str> = name.split('.').collect();
    (4..=253).contains(&name.len())
        && labels.len() >= 2
        && labels.iter().all(|label| {
            (1..=63).contains(&label.len())
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
        && name.parse::<std::net::IpAddr>().is_err()
        && !labels
            .last()
            .is_some_and(|tld| tld.bytes().all(|b| b.is_ascii_digit()))
}

// --- design 3.6, row by row -------------------------------------------------------------

#[test]
fn canary_block_names_are_blocked_by_the_list() {
    let m = matcher(&[]);
    let canaries = list::canary_block();
    assert_eq!(canaries.len(), count(&meta()["canaries"]["block"]));
    for name in canaries {
        assert_eq!(m.verdict(name), LIST, "{name}");
        assert_eq!(m.verdict(&format!("www.{name}")), LIST, "www.{name}");
        assert_eq!(m.verdict(&format!("a.b.{name}")), LIST, "a.b.{name}");
        assert_eq!(
            m.verdict(&format!("{}.", name.to_uppercase())),
            LIST,
            "{name} in capitals"
        );
    }
    assert_eq!(m.verdict("PT.PornHub.com."), LIST);
    assert_eq!(m.verdict("www.xvideos.com:443"), LIST);
    assert_eq!(
        m.verdict_for_url("https://www.xvideos.com:443/new/1"),
        (LIST, false)
    );
}

#[test]
fn canary_allow_names_are_not_blocked() {
    let m = matcher(&[]);
    let canaries = list::canary_allow();
    assert_eq!(canaries.len(), count(&meta()["canaries"]["allow"]));
    for name in canaries {
        let verdict = m.verdict(name);
        assert!(!verdict.is_blocked(), "{name}: {verdict:?}");
        let verdict = m.verdict(&format!("www.{name}"));
        assert!(!verdict.is_blocked(), "www.{name}: {verdict:?}");
    }
    for name in [
        "wikipedia.org",
        "youtube.com",
        "apple.com",
        "uol.com.br",
        "globo.com",
    ] {
        assert!(canaries.contains(&name), "{name} is a canary of the design");
    }
}

#[test]
fn an_entry_is_kept_at_its_listed_depth() {
    let m = matcher(&[]);
    assert_eq!(m.verdict("adult.contents.fc2.com"), LIST);
    assert_eq!(m.verdict("fc2.com"), MIXED);
    assert_eq!(m.verdict("blog.fc2.com"), MIXED);
    // Other listed names below a platform that is itself mixed.
    for (listed, platform) in [
        ("nsfw.reddit.com", "reddit.com"),
        ("img.2chan.net", "2chan.net"),
        ("video.fc2.com", "fc2.com"),
    ] {
        assert_eq!(m.verdict(listed), LIST, "{listed}");
        assert_eq!(m.verdict(platform), MIXED, "{platform}");
    }
}

/// Section 2 of the review file the build writes: every listed name that sits below a name
/// rule of a rule file, with that rule. The longest match keeps each of them blocked, with
/// its subdomains, while the name above it and its other subdomains stay what the rule
/// file says.
#[test]
fn every_listed_name_below_a_rule_file_name_stays_blocked() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/nofap/research/blocklist-top10k-review.txt"
    );
    let review = std::fs::read_to_string(path).unwrap();
    let (_, section) = review
        .split_once("## 2. Listed names kept below a protected name (")
        .expect("section 2 of the review file");
    let (announced, section) = section.split_once(')').unwrap();
    let section = section.split("\n## ").next().unwrap();
    let rows: Vec<(&str, &str, &str)> = section
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut columns = line.split('\t');
            let listed = columns.next().unwrap();
            let (file, rule) = columns.next().unwrap().split_once(": ").unwrap();
            (listed, file, rule)
        })
        .collect();
    assert!(!rows.is_empty());
    assert_eq!(rows.len(), announced.parse::<usize>().unwrap());

    let m = matcher(&[]);
    for (listed, file, rule) in rows {
        assert!(
            listed.ends_with(&format!(".{rule}")),
            "{listed} under {rule}"
        );
        for host in [
            listed.to_owned(),
            format!("www.{listed}"),
            format!("a.b.{listed}"),
        ] {
            assert_eq!(m.verdict(&host), LIST, "{host}");
        }
        let above = match file {
            "mixed.txt" => MIXED,
            "never-capture.txt" => NEVER_CAPTURE,
            "never-block.txt" => BUILT_IN,
            other => panic!("{listed}: unknown rule file {other}"),
        };
        for host in [rule.to_owned(), format!("unlisted-sibling.{rule}")] {
            assert_eq!(m.verdict(&host), above, "{host}");
        }
    }
}

#[test]
fn block_grade_rules_and_look_alikes_block() {
    let m = matcher(&[]);
    let core = HashTable::parse(list::embedded_core(), CORE_TAG).unwrap();
    for (host, rule) in [
        ("porno365.golf", BlockSource::Keyword),
        ("missav123.com", BlockSource::Keyword),
        ("xvv1deos.com", BlockSource::Keyword),
        ("anything.xxx", BlockSource::Tld),
        ("videosdeputaria.com.br", BlockSource::Keyword),
    ] {
        // A public list may come to name one of these; until then the rule decides.
        let expected = if core.contains(host) {
            BlockSource::List
        } else {
            rule
        };
        assert_eq!(m.verdict(host), Verdict::Blocked(expected), "{host}");
    }
    // The rules themselves are exercised: not every row is on the list.
    assert!(!core.contains("xvv1deos.com"));
    assert!(!core.contains("anything.xxx"));
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
        let verdict = m.verdict(host);
        assert!(!verdict.is_blocked(), "{host}: {verdict:?}");
    }
    for host in [
        "essex.gov.uk",
        "sussex.ac.uk",
        "planalto.gov.br",
        "iowasexoffender.gov",
    ] {
        assert_eq!(m.verdict(host), BUILT_IN, "{host}");
    }
}

/// Integrator decision 12: the ordinary names the hand-offs of the foundation and blocklist
/// units list, and the ones found while the rules were tuned, against the real data (the
/// rule files protect some of them, the placement rules of the heuristics the rest).
#[test]
fn ordinary_names_are_not_blocked() {
    let m = matcher(&[]);
    for host in [
        "disputas.com.br",
        "resolucaodedisputas.com.br",
        "computas.com",
        "atputasvakars.lv",
        "denudes.com",
        "apeladas.es",
        "peladasdobairro.com.br",
        "pornichet.fr",
        "ville-pornichet.fr",
        "pornic.com",
        "pornainen.inschool.fi",
        "uspornybalik.cz",
        "zdrowie.odpornosc.org.pl",
        "odporno.si",
        "vodootporno.rs",
        "app.pornuestralarasegura.com",
        "buscardnipornombre.pe",
        "pornosotros.org",
        "sompornlotto.online",
        "siripornautocar.com",
        "siriporn.com",
        "pornthip.co.th",
        "fersensporn.de",
        "sporn.de",
        "cabinporn.com",
        "foodporn.com",
        "bikeporn.cz",
        "quitporn.org",
        "pornfree.org",
        "pornaddictionhelp.org",
        "pornographyharms.org",
        "terminalxxxix.com.br",
        "superbowlxxxiv.com",
        "xxxl-shop.de",
        "mixxx.org",
        "xxxx.com.au",
        "xxxtentacion.com",
        "aeromexico.com",
        "jeromes.com",
        "queromeuape.com.br",
        "evchanime.ru",
        "watchanimeworld.one",
        "pokemonoriginheartgold.com",
        "kemonofriends.jp",
        "kemono.cafe",
        "coomer.com.co",
        "camara.leg.br",
        "camarote.com.br",
        "pijama.com.br",
        "dashcam4k.com",
        "webcam4you.com",
        "maxvideos.com",
        "relaxvideos.com",
        "infraredtube.com",
        "fanslyrics.com",
        "missavery.com",
        "bostoncreampie.com",
        "gozandolavida.com",
        "receitasgostosas.com.br",
        "escorts.co.in",
        "escortskubota.com",
        "escortsgroup.com",
        "ford-escorts.co.uk",
        // Found by the review of this unit.
        "menudesign.com",
        "venudesign.com.br",
        "nudesign.com.br",
        "nudestix.com",
        "chentai.com",
        "zhentai.com.cn",
        "computaria.com.br",
        "disputaria.com",
        "seminovinhas.com.br",
        "funeralescorts.com",
        "pilotcarescorts.com",
        "securityescorts.co.za",
        "medicalescorts.org",
        "motorcycleescorts.com",
        "xxx.fandom.com",
        "www.1337xxx.to",
        "supaporn.net",
        "kittiporn.com",
        "vencerapornografia.com.br",
        "yourbrainonporn.com",
        "xxxchurch.com",
        "superbowlxxx.com",
        "mmxxx.org",
        "xxxl.de",
        "tamanhoxxxg.com.br",
        "exameanal.com.br",
        "cockermouth.org.uk",
        "analandia.sp.gov.br",
        "picapau.com.br",
        "dickssportinggoods.com",
        "expertsexchange.com",
        "adultswim.com",
    ] {
        let verdict = m.verdict(host);
        assert!(!verdict.is_blocked(), "{host}: {verdict:?}");
    }
    // Two of them by data: the film wiki is a known keyword collision, the torrent index
    // a mixed platform like its siblings.
    assert_eq!(m.verdict("xxx.fandom.com"), BUILT_IN);
    assert_eq!(m.verdict("1337xxx.to"), MIXED);
}

/// Adult sites the public lists do not name: the Brazilian supplement and the keyword
/// rules cover them, the first also at the DNS level.
#[test]
fn unlisted_adult_sites_are_blocked() {
    let m = matcher(&[]);
    for host in [
        "pornodoido.com.br",
        "www.xvideoporno.com.br",
        "fotosporno.blog.br",
        "videospornosbr.com.br",
        "privacy.com.br",
        "contoseroticos.com.br",
    ] {
        assert_eq!(m.verdict(host), LIST, "{host}");
    }
    assert!(m.in_hosts_tier("pornodoido.com.br"));
    let core = HashTable::parse(list::embedded_core(), CORE_TAG).unwrap();
    for host in [
        "pornoespanol.es",
        "ruporno.tv",
        "incestoporno.com",
        "fansporno.com",
        "multporn.com",
        "viralxxx.hair",
        "www.hdporner.org",
        "pornhubb.com",
        "xvide0s.com",
        "0nlyf4ns.com",
        "ch4turbate.com",
        "sp4nkbang.com",
        "brazzzers.net",
        "missav01.com",
        "bucetas.net.br",
        "putariabrasileira.com",
        "gostosasnozap.com.br",
    ] {
        let expected = if parents_listed(&core, host) {
            LIST
        } else {
            Verdict::Blocked(BlockSource::Keyword)
        };
        assert_eq!(m.verdict(host), expected, "{host}");
    }
    // The rules themselves are exercised: these are on no list.
    for host in ["ruporno.tv", "pornhubb.com", "0nlyf4ns.com"] {
        assert!(!parents_listed(&core, host), "{host}");
    }
}

/// Whether the table names `host` or a parent of it.
fn parents_listed(core: &HashTable<'_>, host: &str) -> bool {
    let host = host.strip_prefix("www.").unwrap_or(host);
    ubiqx_nofap::matcher::parents(host)
        .into_iter()
        .any(|name| core.contains(name))
}

#[test]
fn a_keyword_in_a_path_is_a_signal_never_a_block() {
    let m = matcher(&[]);
    assert_eq!(
        m.verdict_for_url("https://www.reddit.com/r/EarthPorn/"),
        (MIXED, true)
    );
    assert_eq!(
        m.verdict_for_url("https://www.reddit.com/r/aww/"),
        (MIXED, false)
    );
    assert_eq!(
        m.verdict_for_url("https://www.google.com/search?q=hentai"),
        (POPULAR, true)
    );
    assert_eq!(
        m.verdict_for_url("https://www.google.com/search?q=weather"),
        (POPULAR, false)
    );
}

#[test]
fn an_address_without_its_scheme_and_a_reader_view_are_judged_by_their_page() {
    let m = matcher(&[]);
    // As a browser shows an address: no scheme, and another address in the query.
    assert_eq!(
        m.verdict_for_url("www.xvideos.com/out?to=https://www.wikipedia.org/"),
        (LIST, false)
    );
    assert_eq!(
        m.verdict_for_url("www.wikipedia.org/wiki/Special:Search?go=https://www.xvideos.com/"),
        (POPULAR, true)
    );
    // The reader and source views show the page under another scheme.
    for url in [
        "view-source:https://www.xvideos.com/",
        "about:reader?url=https%3A%2F%2Fwww.xvideos.com%2Fnew%2F1",
        "read://https_www.xvideos.com/?url=https%3A%2F%2Fwww.xvideos.com%2Fnew%2F1",
        "chrome-distiller://0a1b_2c3d/?url=https%3A%2F%2Fwww.xvideos.com%2Fnew%2F1",
    ] {
        assert_eq!(m.verdict_for_url(url), (LIST, false), "{url}");
    }
    assert_eq!(
        m.verdict_for_url("about:reader?url=https%3A%2F%2Fwww.wikipedia.org%2Fwiki%2FEssex"),
        (POPULAR, false)
    );
    assert_eq!(m.verdict_for_url("about:blank"), (LOCAL, false));
}

#[test]
fn local_hosts_are_local_and_a_public_address_is_plain() {
    let m = matcher(&[]);
    for host in [
        "192.168.0.1",
        "[::1]",
        "localhost",
        "nas.local",
        "intranet",
        "",
    ] {
        assert_eq!(m.verdict(host), LOCAL, "{host:?}");
    }
    assert_eq!(
        m.verdict_for_url("http://192.168.0.1/admin"),
        (LOCAL, false)
    );
    assert_eq!(m.verdict_for_url("http://localhost:5173/"), (LOCAL, false));
    assert_eq!(m.verdict("93.184.216.34"), PLAIN);
    assert_eq!(m.verdict_for_url("http://93.184.216.34/"), (PLAIN, false));
}

#[test]
fn user_entries_take_precedence() {
    let m = matcher(&[
        site(SiteList::Block, SiteOrigin::User, "example.com"),
        site(SiteList::Allow, SiteOrigin::User, "example.com"),
        site(SiteList::Allow, SiteOrigin::User, "pt.pornhub.com"),
    ]);
    assert_eq!(
        m.verdict("example.com"),
        Verdict::Blocked(BlockSource::User)
    );
    // An allow below a listed parent: the longer name wins, and only for itself.
    assert_eq!(
        m.verdict("pt.pornhub.com"),
        Verdict::Allowed(AllowSource::User)
    );
    assert_eq!(m.verdict("pornhub.com"), LIST);
    assert_eq!(m.verdict("es.pornhub.com"), LIST);
}

#[test]
fn an_expired_snooze_is_ignored() {
    let snooze = |expires_at| NofapSite {
        expires_at: Some(expires_at),
        ..site(SiteList::Allow, SiteOrigin::Snooze, "porno365.golf")
    };
    let live = matcher(&[snooze(now() + chrono::Duration::hours(24))]);
    assert_eq!(
        live.verdict("porno365.golf"),
        Verdict::Allowed(AllowSource::Snooze)
    );
    let expired = matcher(&[snooze(now() - chrono::Duration::minutes(1))]);
    assert_eq!(
        expired.verdict("porno365.golf"),
        Verdict::Blocked(BlockSource::Keyword)
    );
    // A snooze never lifts the list.
    let listed = matcher(&[NofapSite {
        expires_at: Some(now() + chrono::Duration::hours(24)),
        ..site(SiteList::Allow, SiteOrigin::Snooze, "pornhub.com")
    }]);
    assert_eq!(listed.verdict("pornhub.com"), LIST);
}

#[test]
fn a_learned_host_is_blocked_by_its_hash() {
    let learned = site(SiteList::Block, SiteOrigin::Learned, "a-quiet-name.example");
    assert_eq!(learned.host, None, "a learned row carries no name");
    assert_eq!(matcher(&[]).verdict("a-quiet-name.example"), PLAIN);
    let m = matcher(&[learned]);
    assert_eq!(
        m.verdict("a-quiet-name.example"),
        Verdict::Blocked(BlockSource::Learned)
    );
    assert_eq!(
        m.verdict("www.a-quiet-name.example"),
        Verdict::Blocked(BlockSource::Learned)
    );
    assert!(!format!("{m:?}").contains("a-quiet-name"));
}

#[test]
fn a_damaged_table_fails_the_matcher() {
    let good = list::embedded_core();
    let leak = |bytes: Vec<u8>| -> &'static [u8] { Box::leak(bytes.into_boxed_slice()) };

    let mut bad_magic = good.to_vec();
    bad_magic[0] ^= 0xff;
    let mut unsorted = good.to_vec();
    let (first, second) = unsorted[HEADER_LEN..HEADER_LEN + 16].split_at_mut(8);
    first.swap_with_slice(second);
    let mut duplicate = good.to_vec();
    duplicate.copy_within(HEADER_LEN..HEADER_LEN + 8, HEADER_LEN + 8);
    let mut wrong_count = good.to_vec();
    wrong_count[15] = wrong_count[15].wrapping_add(1);
    let mut truncated = good.to_vec();
    truncated.truncate(good.len() - 8);

    for (what, bytes) in [
        ("bad magic", bad_magic),
        ("unsorted", unsorted),
        ("duplicate keys", duplicate),
        ("wrong count", wrong_count),
        ("a key missing at the end", truncated),
    ] {
        let mut data = MatcherData::embedded();
        data.core = leak(bytes);
        assert!(Matcher::new(data, &[], &SALT, now()).is_err(), "{what}");
    }
    // The popular table is checked the same way.
    let mut data = MatcherData::embedded();
    data.popular = &list::embedded_popular()[..HEADER_LEN + 4];
    assert!(Matcher::new(data, &[], &SALT, now()).is_err());
}

#[test]
fn the_hashing_vector_of_the_build_script_matches() {
    let meta = meta();
    let vector = meta["vector"].as_array().unwrap();
    assert!(vector.len() >= 8, "both tags, four names each");
    let mut tags = HashSet::new();
    for row in vector {
        let tag = row["tag"].as_str().unwrap().as_bytes();
        let name = row["name"].as_str().unwrap();
        let expected = u64::from_str_radix(row["key"].as_str().unwrap(), 16).unwrap();
        assert_eq!(list::key_of(tag, name), expected, "{name}");
        tags.insert(tag.to_vec());
    }
    assert_eq!(
        tags,
        HashSet::from([CORE_TAG.to_vec(), POPULAR_TAG.to_vec()])
    );
}

// --- the embedded files ------------------------------------------------------------------

#[test]
fn the_tables_parse_and_match_the_meta_file() {
    let meta = meta();
    for (section, bytes, tag) in [
        ("core", list::embedded_core(), CORE_TAG),
        ("popular", list::embedded_popular(), POPULAR_TAG),
    ] {
        let table = HashTable::parse(bytes, tag).unwrap();
        assert_eq!(table.len(), count(&meta[section]["count"]), "{section}");
        assert_eq!(bytes.len(), count(&meta[section]["bytes"]), "{section}");
        assert_eq!(bytes.len(), HEADER_LEN + 8 * table.len(), "{section}");
        assert_eq!(
            Some(table.list_version()),
            meta["list_version"].as_u64(),
            "{section}"
        );

        // Sorted and unique, checked here without the parser.
        let keys: Vec<u64> = bytes[HEADER_LEN..]
            .chunks_exact(8)
            .map(|chunk| u64::from_be_bytes(chunk.try_into().unwrap()))
            .collect();
        assert_eq!(keys.len(), table.len(), "{section}");
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{section}");
        for key in [keys[0], keys[keys.len() / 2], keys[keys.len() - 1]] {
            assert!(table.contains_key(key), "{section}");
        }
    }
    // The bounds of the work breakdown.
    let core = HashTable::parse(list::embedded_core(), CORE_TAG).unwrap();
    assert!((215_000..=300_000).contains(&core.len()), "{}", core.len());
    assert!(list::embedded_core().len() <= 2_400_000);
    assert!(list::embedded_popular().len() <= 1_000_000);
    // A table read with the other table's tag finds nothing: the tag is part of the key.
    let crossed = HashTable::parse(list::embedded_core(), POPULAR_TAG).unwrap();
    assert!(!crossed.contains("pornhub.com"));
    assert!(core.contains("pornhub.com"));
}

#[test]
fn the_hosts_tier_holds_valid_listed_names_under_no_rule_file() {
    let meta = meta();
    let tier = list::hosts_tier();
    assert_eq!(tier.len(), count(&meta["hosts_tier"]["lines"]));
    assert!(tier.len() <= 20_000, "{}", tier.len());
    assert_eq!(
        tier.iter().collect::<HashSet<_>>().len(),
        tier.len(),
        "no duplicate line"
    );

    let m = matcher(&[]);
    let data = MatcherData::embedded();
    for name in tier {
        assert!(is_valid_hostname(name), "{name}");
        assert_eq!(*name, name.to_ascii_lowercase(), "{name}");
        assert!(!never_block().covers(name), "{name} is under never-block");
        assert!(!data.mixed.covers(name), "{name} is under mixed");
        assert!(
            !data.never_capture.covers(name),
            "{name} is under never-capture"
        );
        assert_eq!(m.verdict(name), LIST, "{name}");
        assert!(m.in_hosts_tier(name), "{name}");
    }
    assert!(m.in_hosts_tier("www.pornhub.com"));
    assert!(m.in_hosts_tier("anything.below.pornhub.com"));
    assert!(!m.in_hosts_tier("wikipedia.org"));
    assert!(!m.in_hosts_tier("example.com"));
}

#[test]
fn the_rule_files_match_the_meta_file() {
    let meta = meta();
    let data = MatcherData::embedded();
    for (file, lines, rules) in [
        (
            "never-block.txt",
            list::never_block_rules(),
            &data.never_block,
        ),
        ("mixed.txt", list::mixed_rules(), &data.mixed),
        (
            "never-capture.txt",
            list::never_capture_rules(),
            &data.never_capture,
        ),
    ] {
        assert_eq!(lines.len(), count(&meta["rules"][file]), "{file}");
        assert_eq!(rules.len(), lines.len(), "{file}: a rule is written twice");
        for line in lines {
            let name = line.strip_suffix(".*").unwrap_or(line);
            let plain = name
                .bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.'));
            let one_label_before_the_star = !(line.ends_with(".*") && name.contains('.'));
            assert!(
                !name.is_empty() && plain && one_label_before_the_star,
                "{file}: {line}"
            );
        }
        assert_eq!(*rules, RuleSet::parse(lines.iter().copied()), "{file}");
    }
    assert_eq!(*never_block(), data.never_block);
    // The forms of the syntax, on the real file.
    for host in [
        "agency.gov",
        "planalto.gov.br",
        "a.b.sussex.ac.uk",
        "banco.bradesco",
        "apple.com",
    ] {
        assert!(never_block().covers(host), "{host}");
    }
    for host in ["gov.example.com", "example.com", "bradesco"] {
        assert!(!never_block().covers(host), "{host}");
    }
}

// --- integrator decisions 7 and 8 --------------------------------------------------------

#[test]
fn bing_and_grok_are_ordinary_sites() {
    let m = matcher(&[]);
    for host in ["bing.com", "www.bing.com", "grok.com"] {
        let verdict = m.verdict(host);
        assert!(
            matches!(
                verdict,
                Verdict::Unknown(HostClass::Popular | HostClass::Plain { .. })
            ),
            "{host}: {verdict:?}"
        );
        assert!(!never_block().covers(host), "{host}");
    }
    // The screen check may look when a signal fires, and the user may block them.
    let (verdict, signal) = m.verdict_for_url("https://www.bing.com/images/search?q=porn");
    assert!(!verdict.is_blocked() && signal, "{verdict:?}");
    let m = matcher(&[
        site(SiteList::Block, SiteOrigin::User, "bing.com"),
        site(SiteList::Block, SiteOrigin::User, "grok.com"),
    ]);
    assert_eq!(
        m.verdict("www.bing.com"),
        Verdict::Blocked(BlockSource::User)
    );
    assert_eq!(m.verdict("grok.com"), Verdict::Blocked(BlockSource::User));
    // What stays protected: sign-in, update, store and API hosts, and the AI provider.
    let m = matcher(&[]);
    for host in [
        "login.live.com",
        "login.microsoftonline.com",
        "update.microsoft.com",
        "windowsupdate.com",
        "microsoftstore.com",
        "microsoft.com",
        "x.ai",
        "api.x.ai",
        "accounts.google.com",
    ] {
        assert_eq!(m.verdict(host), BUILT_IN, "{host}");
    }
    // google.com is the model: an ordinary popular site.
    assert_eq!(m.verdict("google.com"), POPULAR);
}

#[test]
fn private_messaging_on_the_web_is_never_captured() {
    let m = matcher(&[]);
    for host in [
        "web.whatsapp.com",
        "messenger.com",
        "web.telegram.org",
        "discord.com",
        "canary.discord.com",
        "discordapp.com",
        "web.snapchat.com",
    ] {
        assert_eq!(m.verdict(host), NEVER_CAPTURE, "{host}");
    }
    // Public platforms stay mixed, and so do the public pages of the messaging services.
    for host in [
        "reddit.com",
        "x.com",
        "tumblr.com",
        "imgur.com",
        "snapchat.com",
        "t.me",
    ] {
        assert_eq!(m.verdict(host), MIXED, "{host}");
    }
    let data = MatcherData::embedded();
    for host in [
        "web.telegram.org",
        "telegram.org",
        "discord.com",
        "discord.gg",
    ] {
        assert!(
            !data.mixed.covers(host) || data.never_capture.covers(host),
            "{host}"
        );
    }
    assert!(!data.mixed.covers("telegram.org"));
    assert!(!data.mixed.covers("discord.com"));
}

// --- cost --------------------------------------------------------------------------------

/// The guard asks for a verdict every two seconds and the matcher is rebuilt when the
/// user's sites change: building one walks the keys once and hashes nothing, a verdict
/// hashes a handful of names. The bounds are generous for a debug build on a slow machine;
/// a matcher that hashed every listed name when built would miss them by far.
#[test]
fn building_a_matcher_and_asking_it_are_cheap() {
    let sites = [site(
        SiteList::Block,
        SiteOrigin::Learned,
        "a-quiet-name.example",
    )];
    let started = Instant::now();
    for _ in 0..20 {
        let m = Matcher::new(MatcherData::embedded(), &sites, &SALT, now()).unwrap();
        assert_eq!(m.verdict("pornhub.com"), LIST);
    }
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );

    let m = matcher(&sites);
    let started = Instant::now();
    for round in 0..2_000 {
        let host = format!("cdn{round}.media.some-unknown-site{round}.example.com");
        assert_eq!(m.verdict(&host), PLAIN);
    }
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}
