//! The embedded data: two hash tables (`core.bin`, `popular.bin`) and the hand-maintained
//! text lists. This is the only module that reads the files under `data/`; every other
//! module receives its data as arguments.
//!
//! Table format (all integers big-endian): magic `"UBXBL1\0\0"` (8 bytes), `format: u16`,
//! `flags: u16`, `count: u32`, `list_version: u64` (UTC `YYYYMMDDHHMM`), then `count` sorted
//! unique `u64` keys. A key is the first 8 bytes of SHA-256 over the table's tag followed by
//! the name. The same format is written by `scripts/nofap/build_blocklist.py`, whose
//! docstring is the full contract.
//!
//! A table is never copied or indexed: [`HashTable`] is a view over the embedded bytes, a
//! lookup hashes one name and runs a binary search, and parsing only walks the keys once to
//! check their order.

use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use ubiqx_core::{CoreError, CoreResult};

/// First 8 bytes of every table.
pub const MAGIC: [u8; 8] = *b"UBXBL1\0\0";
/// The only table format this build reads.
pub const FORMAT: u16 = 1;
/// Bytes before the first key: magic, format, flags, count, list version.
pub const HEADER_LEN: usize = 24;
/// Hash tag of `core.bin` (the blocklist).
pub const CORE_TAG: &[u8] = b"ubiqx-nofap-blocklist-v1\0";
/// Hash tag of `popular.bin` (well-known sites that are on no list).
pub const POPULAR_TAG: &[u8] = b"ubiqx-nofap-popular-v1\0";

/// The only value of the reserved `flags` field this build reads.
const FLAGS: u16 = 0;
const KEY_LEN: usize = 8;

static CORE_BIN: &[u8] = include_bytes!("../data/core.bin");
static POPULAR_BIN: &[u8] = include_bytes!("../data/popular.bin");
static HOSTS_TIER_TXT: &str = include_str!("../data/hosts-tier.txt");
static NEVER_BLOCK_TXT: &str = include_str!("../data/never-block.txt");
static MIXED_TXT: &str = include_str!("../data/mixed.txt");
static NEVER_CAPTURE_TXT: &str = include_str!("../data/never-capture.txt");
static CANARY_BLOCK_TXT: &str = include_str!("../data/canary-block.txt");
static CANARY_ALLOW_TXT: &str = include_str!("../data/canary-allow.txt");

/// A parsed table: a view over the bytes it was parsed from (memory equals file size; a
/// lookup is a binary search).
#[derive(Debug, Clone, Copy)]
pub struct HashTable<'a> {
    tag: &'static [u8],
    keys: &'a [u8],
    count: usize,
    list_version: u64,
}

impl<'a> HashTable<'a> {
    /// Validates the header and the keys: magic, format, reserved flags, `count` equal to
    /// the number of keys present, keys strictly increasing. `tag` is the hash tag lookups
    /// use ([`CORE_TAG`] or [`POPULAR_TAG`]).
    pub fn parse(bytes: &'a [u8], tag: &'static [u8]) -> CoreResult<Self> {
        let invalid = |what: &str| CoreError::Invalid(format!("blocklist table: {what}"));
        if bytes.len() < HEADER_LEN {
            return Err(invalid("shorter than its header"));
        }
        let (header, keys) = bytes.split_at(HEADER_LEN);
        if header[..8] != MAGIC {
            return Err(invalid("bad magic"));
        }
        let format = u16::from_be_bytes([header[8], header[9]]);
        let flags = u16::from_be_bytes([header[10], header[11]]);
        if format != FORMAT || flags != FLAGS {
            return Err(invalid("unsupported format or flags"));
        }
        let count = u32::from_be_bytes([header[12], header[13], header[14], header[15]]) as usize;
        let list_version = read_u64(&header[16..]);
        if count.checked_mul(KEY_LEN) != Some(keys.len()) {
            return Err(invalid("length does not match count"));
        }
        let mut previous = None;
        for chunk in keys.chunks_exact(KEY_LEN) {
            let key = read_u64(chunk);
            if previous.is_some_and(|before| before >= key) {
                return Err(invalid("keys are not strictly ascending"));
            }
            previous = Some(key);
        }
        Ok(Self {
            tag,
            keys,
            count,
            list_version,
        })
    }

    /// Whether the table holds exactly `name` (already normalised; no parent walk here).
    pub fn contains(&self, name: &str) -> bool {
        self.contains_key(key_of(self.tag, name))
    }

    /// Whether the table holds this key.
    pub fn contains_key(&self, key: u64) -> bool {
        let (mut low, mut high) = (0, self.count);
        while low < high {
            let middle = low + (high - low) / 2;
            let found = read_u64(&self.keys[middle * KEY_LEN..]);
            match found.cmp(&key) {
                std::cmp::Ordering::Equal => return true,
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
            }
        }
        false
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// UTC `YYYYMMDDHHMM` of the build that wrote the table.
    pub fn list_version(&self) -> u64 {
        self.list_version
    }
}

/// The big-endian `u64` at the start of `bytes` (callers pass at least 8 bytes).
fn read_u64(bytes: &[u8]) -> u64 {
    let mut raw = [0u8; KEY_LEN];
    raw.copy_from_slice(&bytes[..KEY_LEN]);
    u64::from_be_bytes(raw)
}

/// The key of `name` under `tag`: the first 8 bytes of `SHA-256(tag ‖ name)`, big-endian.
pub fn key_of(tag: &[u8], name: &str) -> u64 {
    let digest = Sha256::new()
        .chain_update(tag)
        .chain_update(name.as_bytes())
        .finalize();
    read_u64(&digest)
}

/// The non-empty lines of a data file that are not `#` comments, trimmed.
pub fn lines(text: &'static str) -> Vec<&'static str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// The lines of one embedded text file, split once per process.
fn cached(
    cell: &'static OnceLock<Vec<&'static str>>,
    text: &'static str,
) -> &'static [&'static str] {
    cell.get_or_init(|| lines(text))
}

/// `data/core.bin`.
pub fn embedded_core() -> &'static [u8] {
    CORE_BIN
}

/// `data/popular.bin`.
pub fn embedded_popular() -> &'static [u8] {
    POPULAR_BIN
}

/// `data/hosts-tier.txt`: plain hostnames for the hosts block, most visited first.
pub fn hosts_tier() -> &'static [&'static str] {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    cached(&LINES, HOSTS_TIER_TXT)
}

/// `data/never-block.txt`: rules in the syntax of [`crate::matcher::RuleSet`].
pub fn never_block_rules() -> &'static [&'static str] {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    cached(&LINES, NEVER_BLOCK_TXT)
}

/// `data/mixed.txt`: platforms that cannot be split by domain.
pub fn mixed_rules() -> &'static [&'static str] {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    cached(&LINES, MIXED_TXT)
}

/// `data/never-capture.txt`: hosts the AI check never looks at.
pub fn never_capture_rules() -> &'static [&'static str] {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    cached(&LINES, NEVER_CAPTURE_TXT)
}

/// `data/canary-block.txt`: names the embedded table must cover.
pub fn canary_block() -> &'static [&'static str] {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    cached(&LINES, CANARY_BLOCK_TXT)
}

/// `data/canary-allow.txt`: names the embedded table must never cover.
pub fn canary_allow() -> &'static [&'static str] {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    cached(&LINES, CANARY_ALLOW_TXT)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table in the build script's format, from keys given in file order.
    fn table(keys: &[u64], list_version: u64) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&FORMAT.to_be_bytes());
        bytes.extend_from_slice(&FLAGS.to_be_bytes());
        bytes.extend_from_slice(&(keys.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&list_version.to_be_bytes());
        for key in keys {
            bytes.extend_from_slice(&key.to_be_bytes());
        }
        bytes
    }

    fn sorted_keys(names: &[&str]) -> Vec<u64> {
        let mut keys: Vec<u64> = names.iter().map(|name| key_of(CORE_TAG, name)).collect();
        keys.sort_unstable();
        keys
    }

    #[test]
    fn key_is_the_first_eight_bytes_of_sha256_over_tag_and_name() {
        // The values scripts/nofap/build_blocklist.py writes to `core.meta.json.vector`
        // (tests/embedded.rs compares the whole vector of the embedded file).
        assert_eq!(key_of(CORE_TAG, "example.com"), 0xda53_0776_05c4_bf53);
        assert_eq!(key_of(POPULAR_TAG, "example.com"), 0x10e6_d953_785f_f652);
        assert_ne!(
            key_of(CORE_TAG, "example.com"),
            key_of(CORE_TAG, "example.org")
        );
    }

    #[test]
    fn a_parsed_table_finds_its_names_and_nothing_else() {
        let names = ["one.example", "two.example", "three.example", "a.b"];
        let bytes = table(&sorted_keys(&names), 202611191200);
        let parsed = HashTable::parse(&bytes, CORE_TAG).unwrap();
        assert_eq!(parsed.len(), 4);
        assert!(!parsed.is_empty());
        assert_eq!(parsed.list_version(), 202611191200);
        for name in names {
            assert!(parsed.contains(name), "{name}");
            assert!(parsed.contains_key(key_of(CORE_TAG, name)));
        }
        assert!(!parsed.contains("four.example"));
        // No parent walk and no normalisation at this level.
        assert!(!parsed.contains("sub.one.example"));
        assert!(!parsed.contains("ONE.example"));
        // The tag is part of the key.
        let other = HashTable::parse(&bytes, POPULAR_TAG).unwrap();
        assert!(!other.contains("one.example"));
    }

    #[test]
    fn the_first_and_the_last_key_are_found() {
        let keys = [1, 5, 9, 200, u64::MAX];
        let bytes = table(&keys, 1);
        let parsed = HashTable::parse(&bytes, CORE_TAG).unwrap();
        for key in keys {
            assert!(parsed.contains_key(key), "{key}");
        }
        for key in [0, 2, 8, 10, 199, 201, u64::MAX - 1] {
            assert!(!parsed.contains_key(key), "{key}");
        }
    }

    #[test]
    fn an_empty_table_parses_and_holds_nothing() {
        let bytes = table(&[], 7);
        let parsed = HashTable::parse(&bytes, CORE_TAG).unwrap();
        assert!(parsed.is_empty());
        assert_eq!(parsed.len(), 0);
        assert!(!parsed.contains("example.com"));
    }

    #[test]
    fn malformed_tables_are_rejected() {
        let good = table(&[1, 2, 3], 1);
        assert!(HashTable::parse(&good, CORE_TAG).is_ok());

        let mut bad_magic = good.clone();
        bad_magic[0] = b'X';
        let mut bad_format = good.clone();
        bad_format[9] = 2;
        let mut bad_flags = good.clone();
        bad_flags[11] = 1;
        let mut count_too_big = good.clone();
        count_too_big[15] = 4;
        let mut count_too_small = good.clone();
        count_too_small[15] = 2;
        let mut truncated_key = good.clone();
        truncated_key.pop();
        let cases: [(&str, Vec<u8>); 10] = [
            ("empty", Vec::new()),
            ("short header", good[..HEADER_LEN - 1].to_vec()),
            ("bad magic", bad_magic),
            ("unknown format", bad_format),
            ("unknown flags", bad_flags),
            ("count above the keys present", count_too_big),
            ("count below the keys present", count_too_small),
            ("a key cut short", truncated_key),
            ("unsorted keys", table(&[1, 3, 2], 1)),
            ("duplicate keys", table(&[1, 2, 2], 1)),
        ];
        for (what, bytes) in cases {
            let error = HashTable::parse(&bytes, CORE_TAG).expect_err(what);
            assert!(matches!(error, CoreError::Invalid(_)), "{what}: {error}");
        }
    }

    #[test]
    fn lines_drop_comments_blanks_and_surrounding_space() {
        let text = "# header\n\nexample.com\n  spaced.example  \r\n#another\ngov.*\n";
        assert_eq!(lines(text), ["example.com", "spaced.example", "gov.*"]);
        assert!(lines("").is_empty());
        assert!(lines("# only a comment").is_empty());
    }

    #[test]
    fn the_embedded_files_are_present() {
        assert!(embedded_core().starts_with(&MAGIC));
        assert!(embedded_popular().starts_with(&MAGIC));
        for (name, list) in [
            ("hosts-tier", hosts_tier()),
            ("never-block", never_block_rules()),
            ("mixed", mixed_rules()),
            ("never-capture", never_capture_rules()),
            ("canary-block", canary_block()),
            ("canary-allow", canary_allow()),
        ] {
            assert!(!list.is_empty(), "{name}");
            assert!(
                list.iter()
                    .all(|line| !line.is_empty() && !line.starts_with('#')),
                "{name}"
            );
        }
        // The accessors hand out the same split every time.
        assert!(std::ptr::eq(hosts_tier(), hosts_tier()));
    }
}
