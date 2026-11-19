//! The embedded data: two hash tables (`core.bin`, `popular.bin`) and the hand-maintained
//! text lists. This is the only module that reads the files under `data/`; every other
//! module receives its data as arguments.
//!
//! Table format (all integers big-endian): magic `"UBXBL1\0\0"` (8 bytes), `format: u16`,
//! `flags: u16`, `count: u32`, `list_version: u64` (UTC `YYYYMMDDHHMM`), then `count` sorted
//! unique `u64` keys. A key is the first 8 bytes of SHA-256 over the table's tag followed by
//! the name. The same format is written by `scripts/nofap/build_blocklist.py`.
//!
//! Seeded by the foundation unit: signatures only. The accessors below are functions (not
//! constants) so that the owner of this file decides how the files are embedded.
#![allow(unused_variables, dead_code)]

use ubiqx_core::CoreResult;

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
    /// Validates the header and the keys: magic, format, `count` equal to the number of
    /// keys present, keys strictly increasing. `tag` is the hash tag lookups use
    /// ([`CORE_TAG`] or [`POPULAR_TAG`]).
    pub fn parse(bytes: &'a [u8], tag: &'static [u8]) -> CoreResult<Self> {
        todo!()
    }

    /// Whether the table holds exactly `name` (already normalised; no parent walk here).
    pub fn contains(&self, name: &str) -> bool {
        todo!()
    }

    /// Whether the table holds this key.
    pub fn contains_key(&self, key: u64) -> bool {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// UTC `YYYYMMDDHHMM` of the build that wrote the table.
    pub fn list_version(&self) -> u64 {
        todo!()
    }
}

/// The key of `name` under `tag`: the first 8 bytes of `SHA-256(tag ‖ name)`, big-endian.
pub fn key_of(tag: &[u8], name: &str) -> u64 {
    todo!()
}

/// The non-empty lines of a data file that are not `#` comments, trimmed.
pub fn lines(text: &'static str) -> Vec<&'static str> {
    todo!()
}

/// `data/core.bin`.
pub fn embedded_core() -> &'static [u8] {
    todo!()
}

/// `data/popular.bin`.
pub fn embedded_popular() -> &'static [u8] {
    todo!()
}

/// `data/hosts-tier.txt`: plain hostnames for the hosts block, most visited first.
pub fn hosts_tier() -> &'static [&'static str] {
    todo!()
}

/// `data/never-block.txt`: rules in the syntax of [`crate::matcher::RuleSet`].
pub fn never_block_rules() -> &'static [&'static str] {
    todo!()
}

/// `data/mixed.txt`: platforms that cannot be split by domain.
pub fn mixed_rules() -> &'static [&'static str] {
    todo!()
}

/// `data/never-capture.txt`: hosts the AI check never looks at.
pub fn never_capture_rules() -> &'static [&'static str] {
    todo!()
}

/// `data/canary-block.txt`: names the embedded table must cover.
pub fn canary_block() -> &'static [&'static str] {
    todo!()
}

/// `data/canary-allow.txt`: names the embedded table must never cover.
pub fn canary_allow() -> &'static [&'static str] {
    todo!()
}
