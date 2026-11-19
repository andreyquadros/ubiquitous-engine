//! The marked block of the hosts file (L2): what goes in it, how it is rendered, and how
//! it is spliced into and out of a file without touching anything else.
//!
//! ```text
//! # >>> ubiqx-shield v1 BEGIN sha256=<hex of the lines between the markers> names=<n>
//! # Added by the ubiqx-shield app on this computer. To undo, delete every line from BEGIN to END.
//! 0.0.0.0 name1 name2 name3 name4 name5 name6
//! :: name1 name2 name3 name4 name5 name6
//! # <<< ubiqx-shield END
//! ```
//!
//! The hash covers the lines strictly between the two markers (the undo hint included),
//! joined with `\n`, so converting the block to the file's line endings does not change it.
//!
//! Seeded by the foundation unit: complete types and constants, signatures only.
#![allow(unused_variables)]

use ubiqx_core::nofap::Os;

/// Most names in the block on Windows (the DNS Client re-parses the file) and elsewhere.
pub const CAP_WINDOWS: usize = 8_000;
pub const CAP_UNIX: usize = 20_000;
/// Names per `0.0.0.0` / `::` line.
pub const NAMES_PER_LINE: usize = 6;
/// The first name of every block: a fixed reserved name, the same on every machine.
pub const CANARY_NAME: &str = "canary.ubiqx-shield.invalid";
/// Addresses a blocked name resolves to.
pub const NULL_V4: &str = "0.0.0.0";
pub const NULL_V6: &str = "::";
/// Start of the first marker line (followed by ` sha256=<hex> names=<n>`).
pub const BEGIN_PREFIX: &str = "# >>> ubiqx-shield v1 BEGIN";
/// The last marker line.
pub const END_MARKER: &str = "# <<< ubiqx-shield END";
/// The comment line after BEGIN (inside the hash): the way out, written in the file.
pub const UNDO_HINT: &str =
    "# Added by the ubiqx-shield app on this computer. To undo, delete every line from BEGIN to END.";
/// Shortest and longest hostname accepted, in bytes.
pub const HOSTNAME_MIN_LEN: usize = 4;
pub const HOSTNAME_MAX_LEN: usize = 253;

/// [`CAP_WINDOWS`] or [`CAP_UNIX`].
pub fn cap_for(os: Os) -> usize {
    todo!()
}

/// ASCII letters, digits, hyphen and dots; 4 to 253 bytes; at least two labels, none empty
/// or longer than 63 bytes or starting or ending with a hyphen; not an IP literal.
pub fn is_valid_hostname(name: &str) -> bool {
    todo!()
}

/// The names of a block, in order and without duplicates: [`CANARY_NAME`], then each valid
/// name of `extras` followed by its `www.` twin, then `tier` (most visited first) until the
/// cap of `os`. Invalid extras are dropped.
pub fn names_for(tier: &[&str], extras: &[String], os: Os) -> Vec<String> {
    todo!()
}

/// The whole block for `names`, markers included, with `\n` line endings and a final
/// newline. One `0.0.0.0` line and one `::` line per group of [`NAMES_PER_LINE`].
pub fn render_block(names: &[String]) -> String {
    todo!()
}

/// The hash a block declares for its body: SHA-256 (lower-case hex) over the lines between
/// the markers joined with `\n`. `lines` are those lines, without line endings.
pub fn body_sha256(lines: &[&str]) -> String {
    todo!()
}

/// Replaces the block in `existing` with `block` when the markers are present, appends it
/// otherwise; `None` removes exactly the block. Every byte outside the markers, the file's
/// line-ending convention (the block is converted to it) and a missing final newline are
/// preserved, so `splice(splice(x, Some(b)), None) == x`. A foreign block with a similar
/// marker is untouched.
pub fn splice(existing: &[u8], block: Option<&str>) -> Vec<u8> {
    todo!()
}

/// What a hosts file holds between our markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockInspection {
    /// No BEGIN marker.
    Absent,
    /// Markers present and the declared hash equals the hash of the lines.
    Intact { sha256: String, names: u32 },
    /// Markers present, but the lines are not what the marker declares (or END is missing).
    Changed,
}

/// Reads the block out of a hosts file.
pub fn inspect(existing: &[u8]) -> BlockInspection {
    todo!()
}

/// The names of the block in `existing`, in file order and without duplicates (empty when
/// there is no block).
pub fn block_names(existing: &[u8]) -> Vec<String> {
    todo!()
}
