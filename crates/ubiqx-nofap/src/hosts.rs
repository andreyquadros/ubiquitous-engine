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
//! # What "the block" is, for every input
//!
//! [`splice`] runs with administrator rights on a file the user also edits by hand, so what
//! it may touch is defined for damaged files too. A file is read as lines (`\n` or `\r\n`
//! ended; a UTF-8 byte-order mark in front belongs to the file, not to a line). Ours are:
//!
//! * a BEGIN line (the prefix alone, or followed by a space), every line up to the next END
//!   line, and that END line, when no other BEGIN line lies in between;
//! * a BEGIN line without such an END (the END line was deleted, the file was cut short),
//!   together with the lines right after it that are provably of our making: the undo
//!   hint, then pairs of a `0.0.0.0` line and a `::` line carrying the same names;
//! * an END line without a BEGIN, together with such pairs and the hint right before it.
//!
//! Nothing else is ever removed or rewritten: a block with a similar marker, a single
//! `0.0.0.0 name` line of somebody's ad list, a pair that stands next to no marker. One
//! convention makes the round trip exact: when the block is the last thing in a file that
//! does not end with a line break, the line break in front of the block is the block's
//! (it was added to append the block, and goes when the block goes).

use std::collections::HashSet;

use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use ubiqx_core::nofap::{HostsStamp, Os};

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
/// Longest label of a hostname, in bytes.
pub const LABEL_MAX_LEN: usize = 63;

/// Suffixes of names that never leave the machine or the local network. Pointing one of
/// them at the null address could break the machine's own services, and the in-app guard
/// never blocks them either.
const LOCAL_SUFFIXES: &[&str] = &[
    "local",
    "localhost",
    "localdomain",
    "internal",
    "lan",
    "home.arpa",
    "test",
    "corp",
];

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const LF: &[u8] = b"\n";
const CRLF: &[u8] = b"\r\n";

/// [`CAP_WINDOWS`] or [`CAP_UNIX`].
pub fn cap_for(os: Os) -> usize {
    match os {
        Os::Windows => CAP_WINDOWS,
        Os::Macos | Os::Linux => CAP_UNIX,
    }
}

/// Lower-case ASCII letters, digits, hyphen and dots; 4 to 253 bytes; at least two labels,
/// none empty or longer than 63 bytes or starting or ending with a hyphen; not an IP
/// literal (no numeric last label, decimal or `0x` hexadecimal); not a name that stays on
/// the machine or the local network (`localhost`, `.local`, `.lan`, `.internal`, …).
///
/// This is the last check before a name is written to the hosts file with administrator
/// rights. Upper case is refused rather than folded, so exactly one spelling of a name can
/// reach the file and the never-block rules are always asked about that spelling.
pub fn is_valid_hostname(name: &str) -> bool {
    if !(HOSTNAME_MIN_LEN..=HOSTNAME_MAX_LEN).contains(&name.len()) {
        return false;
    }
    let mut labels = 0usize;
    let mut last = "";
    for label in name.split('.') {
        if !is_valid_label(label) {
            return false;
        }
        labels += 1;
        last = label;
    }
    labels >= 2 && !is_numeric_label(last) && !is_local_name(name)
}

fn is_valid_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= LABEL_MAX_LEN
        && bytes[0] != b'-'
        && bytes[bytes.len() - 1] != b'-'
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

/// A label an address parser reads as a number: decimal digits, or `0x` and hexadecimal
/// digits. A name whose last label is one is an IPv4 literal to a browser.
fn is_numeric_label(label: &str) -> bool {
    match label.strip_prefix("0x") {
        Some(hex) => hex.bytes().all(|b| b.is_ascii_hexdigit()),
        None => label.bytes().all(|b| b.is_ascii_digit()),
    }
}

fn is_local_name(name: &str) -> bool {
    LOCAL_SUFFIXES.iter().any(|suffix| {
        name.strip_suffix(suffix)
            .is_some_and(|rest| rest.is_empty() || rest.ends_with('.'))
    })
}

/// The names of a block, in order and without duplicates: [`CANARY_NAME`], then each valid
/// name of `extras` followed by its `www.` twin, then `tier` (most visited first), never
/// more than the cap of `os` in all. Names that fail [`is_valid_hostname`] are dropped,
/// wherever they come from.
pub fn names_for(tier: &[&str], extras: &[String], os: Os) -> Vec<String> {
    let cap = cap_for(os);
    let mut names: Vec<String> = Vec::with_capacity(cap.min(tier.len() + 2 * extras.len() + 1));
    let mut seen: HashSet<String> = HashSet::new();
    let mut add = |name: &str| {
        if names.len() < cap && seen.insert(name.to_owned()) {
            names.push(name.to_owned());
        }
    };
    add(CANARY_NAME);
    for extra in extras {
        if !is_valid_hostname(extra) {
            continue;
        }
        add(extra);
        if !extra.starts_with("www.") {
            let twin = format!("www.{extra}");
            if is_valid_hostname(&twin) {
                add(&twin);
            }
        }
    }
    for name in tier {
        if is_valid_hostname(name) {
            add(name);
        }
    }
    names
}

/// Whether a name may appear on a line of the block at all: the characters of a hostname
/// and nothing else. [`render_block`] writes no other name, whatever it is handed.
fn is_writable(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= HOSTNAME_MAX_LEN
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
}

/// The lines between the markers for `names`, and how many names they carry.
fn body_lines(names: &[String]) -> (Vec<String>, u32) {
    let writable: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| is_writable(name))
        .collect();
    let mut lines = Vec::with_capacity(1 + 2 * writable.len().div_ceil(NAMES_PER_LINE));
    lines.push(UNDO_HINT.to_owned());
    for group in writable.chunks(NAMES_PER_LINE) {
        let joined = group.join(" ");
        lines.push(format!("{NULL_V4} {joined}"));
        lines.push(format!("{NULL_V6} {joined}"));
    }
    (lines, u32::try_from(writable.len()).unwrap_or(u32::MAX))
}

/// The whole block for `names`, markers included, with `\n` line endings and a final
/// newline. One `0.0.0.0` line and one `::` line per group of [`NAMES_PER_LINE`]. A name
/// with any character outside `a-z 0-9 - .` is left out, so a line of the block can only
/// ever hold an address and names.
pub fn render_block(names: &[String]) -> String {
    let (lines, count) = body_lines(names);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let sha256 = body_sha256(&refs);
    let mut block = format!("{BEGIN_PREFIX} sha256={sha256} names={count}\n");
    for line in &lines {
        block.push_str(line);
        block.push('\n');
    }
    block.push_str(END_MARKER);
    block.push('\n');
    block
}

/// The hash and the size [`render_block`] declares for `names`: what the state file
/// records after a write, and what an intact block is compared with.
pub fn stamp_of(names: &[String]) -> HostsStamp {
    let (lines, count) = body_lines(names);
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    HostsStamp {
        sha256: body_sha256(&refs),
        names: count,
    }
}

/// The hash a block declares for its body: SHA-256 (lower-case hex) over the lines between
/// the markers joined with `\n`. `lines` are those lines, without line endings.
pub fn body_sha256(lines: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            hasher.update(LF);
        }
        hasher.update(line.as_bytes());
    }
    HEXLOWER.encode(&hasher.finalize())
}

// ---------------------------------------------------------------------------------------------
// Reading a hosts file
// ---------------------------------------------------------------------------------------------

/// One line of the file: its bytes without the line ending, and the line ending (`\n`,
/// `\r\n`, or nothing for a last line that is not ended).
#[derive(Clone, Copy)]
struct Line<'a> {
    text: &'a [u8],
    eol: &'a [u8],
}

fn split_bom(bytes: &[u8]) -> (&[u8], &[u8]) {
    if bytes.starts_with(BOM) {
        bytes.split_at(BOM.len())
    } else {
        (&[], bytes)
    }
}

fn split_lines(bytes: &[u8]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let text_end = if index > start && bytes[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        lines.push(Line {
            text: &bytes[start..text_end],
            eol: &bytes[text_end..=index],
        });
        start = index + 1;
    }
    if start < bytes.len() {
        lines.push(Line {
            text: &bytes[start..],
            eol: &[],
        });
    }
    lines
}

/// The line without the blanks an editor may have left at its end.
fn trim_end(text: &[u8]) -> &[u8] {
    let end = text
        .iter()
        .rposition(|b| *b != b' ' && *b != b'\t')
        .map_or(0, |last| last + 1);
    &text[..end]
}

fn is_begin(text: &[u8]) -> bool {
    let text = trim_end(text);
    match text.strip_prefix(BEGIN_PREFIX.as_bytes()) {
        Some(rest) => rest.is_empty() || rest[0] == b' ',
        None => false,
    }
}

fn is_end(text: &[u8]) -> bool {
    trim_end(text) == END_MARKER.as_bytes()
}

fn is_hint(text: &[u8]) -> bool {
    trim_end(text) == UNDO_HINT.as_bytes()
}

/// The names of an address line exactly as [`render_block`] writes one: the address, one
/// space, one to [`NAMES_PER_LINE`] names separated by single spaces.
fn rendered_names<'a>(text: &'a [u8], address: &str) -> Option<&'a [u8]> {
    let names = text
        .strip_prefix(address.as_bytes())?
        .strip_prefix(b" ".as_slice())?;
    let mut count = 0;
    for name in names.split(|b| *b == b' ') {
        count += 1;
        if count > NAMES_PER_LINE || !std::str::from_utf8(name).is_ok_and(is_writable) {
            return None;
        }
    }
    Some(names)
}

/// A `0.0.0.0` line followed by a `::` line with the same names: the shape only
/// [`render_block`] produces.
fn is_pair(v4: &[u8], v6: &[u8]) -> bool {
    match (rendered_names(v4, NULL_V4), rendered_names(v6, NULL_V6)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// A run of lines that are ours (see the module documentation). `end` is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Region {
    start: usize,
    end: usize,
    /// BEGIN and END both present.
    complete: bool,
}

fn regions(lines: &[Line<'_>]) -> Vec<Region> {
    let mut found = Vec::new();
    // First line that no earlier region owns: a backward walk never crosses it.
    let mut floor = 0;
    let mut index = 0;
    while index < lines.len() {
        let text = lines[index].text;
        if is_begin(text) {
            let mut scan = index + 1;
            let mut end = None;
            while scan < lines.len() && !is_begin(lines[scan].text) {
                if is_end(lines[scan].text) {
                    end = Some(scan);
                    break;
                }
                scan += 1;
            }
            let region = match end {
                Some(end) => Region {
                    start: index,
                    end: end + 1,
                    complete: true,
                },
                None => Region {
                    start: index,
                    end: rendered_run_forward(lines, index + 1),
                    complete: false,
                },
            };
            index = region.end;
            floor = region.end;
            found.push(region);
        } else if is_end(text) {
            found.push(Region {
                start: rendered_run_backward(lines, index, floor),
                end: index + 1,
                complete: false,
            });
            index += 1;
            floor = index;
        } else {
            index += 1;
        }
    }
    found
}

/// Where the lines of our making end after an orphaned BEGIN at `from - 1`.
fn rendered_run_forward(lines: &[Line<'_>], from: usize) -> usize {
    let mut index = from;
    if index < lines.len() && is_hint(lines[index].text) {
        index += 1;
    }
    while index + 1 < lines.len() && is_pair(lines[index].text, lines[index + 1].text) {
        index += 2;
    }
    index
}

/// Where the lines of our making start before an orphaned END at `end`.
fn rendered_run_backward(lines: &[Line<'_>], end: usize, floor: usize) -> usize {
    let mut index = end;
    while index >= floor + 2 && is_pair(lines[index - 2].text, lines[index - 1].text) {
        index -= 2;
    }
    if index > floor && is_hint(lines[index - 1].text) {
        index -= 1;
    }
    index
}

/// The lines of a block handed to [`splice`], when it is one: a BEGIN line, an END line
/// and no marker in between.
fn block_lines(block: &str) -> Option<Vec<&str>> {
    let mut lines: Vec<&str> = block
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let (first, rest) = lines.split_first()?;
    let (last, middle) = rest.split_last()?;
    let well_formed = is_begin(first.as_bytes())
        && is_end(last.as_bytes())
        && middle
            .iter()
            .all(|line| !is_begin(line.as_bytes()) && !is_end(line.as_bytes()));
    well_formed.then_some(lines)
}

/// The line ending a block appended to this file gets: `\r\n` when most of the file's
/// lines end so, and when the file ends with a lone `\r` (a `\n` right after it would
/// turn the two into one line ending and change what the last line says).
fn append_eol(rest: &[u8], lines: &[Line<'_>]) -> &'static [u8] {
    let crlf = lines.iter().filter(|line| line.eol == CRLF).count();
    let lf = lines.iter().filter(|line| line.eol == LF).count();
    if crlf > lf || rest.last() == Some(&b'\r') {
        CRLF
    } else {
        LF
    }
}

fn static_eol(eol: &[u8]) -> &'static [u8] {
    if eol == CRLF {
        CRLF
    } else {
        LF
    }
}

fn write_block(out: &mut Vec<u8>, lines: &[&str], eol: &[u8], final_eol: bool) {
    for (index, line) in lines.iter().enumerate() {
        out.extend_from_slice(line.as_bytes());
        if index + 1 < lines.len() || final_eol {
            out.extend_from_slice(eol);
        }
    }
}

/// Replaces the block in `existing` with `block` when the markers are present, appends it
/// otherwise; `None` removes exactly the block. Every byte outside the block (as the
/// module documentation defines it), the file's line-ending convention (the block is
/// converted to it) and a missing final newline are preserved, so for a file without a
/// block `splice(splice(x, Some(b)), None) == x`, and splicing the same block twice
/// changes nothing. A foreign block with a similar marker is untouched. Should the file
/// hold more than one block of ours, the first is replaced and the others are removed.
///
/// `block` is the output of [`render_block`]. Anything that is not a BEGIN line, lines
/// without a marker and an END line is refused: the file comes back unchanged, because a
/// text written without its markers could never be found and removed again.
pub fn splice(existing: &[u8], block: Option<&str>) -> Vec<u8> {
    let new_lines = match block {
        Some(block) => match block_lines(block) {
            Some(lines) => Some(lines),
            None => return existing.to_vec(),
        },
        None => None,
    };
    let (bom, rest) = split_bom(existing);
    let lines = split_lines(rest);
    let found = regions(&lines);
    let mut out = Vec::with_capacity(existing.len() + block.map_or(0, |b| b.len() + 2));
    out.extend_from_slice(bom);

    if found.is_empty() {
        out.extend_from_slice(rest);
        if let Some(new_lines) = &new_lines {
            let eol = append_eol(rest, &lines);
            let ended = lines.last().is_none_or(|line| !line.eol.is_empty());
            if !ended {
                out.extend_from_slice(eol);
            }
            write_block(&mut out, new_lines, eol, ended);
        }
        return out;
    }

    let mut pending = found.iter().peekable();
    let mut written = false;
    // Length of the line ending `out` currently ends with, when that line ending may be
    // taken back (see the convention in the module documentation); 0 otherwise.
    let mut tail_eol = 0;
    let mut index = 0;
    while index < lines.len() {
        let region = match pending.peek() {
            Some(region) if region.start == index => **region,
            _ => {
                let line = lines[index];
                out.extend_from_slice(line.text);
                out.extend_from_slice(line.eol);
                tail_eol = line.eol.len();
                index += 1;
                continue;
            }
        };
        pending.next();
        let first = lines[region.start];
        let ended = !lines[region.end - 1].eol.is_empty();
        match &new_lines {
            Some(new_lines) if !written => {
                let eol = if first.eol.is_empty() {
                    append_eol(rest, &lines)
                } else {
                    static_eol(first.eol)
                };
                write_block(&mut out, new_lines, eol, ended);
                written = true;
                tail_eol = if ended { eol.len() } else { 0 };
            }
            _ => {
                if !ended {
                    // The block closes a file that has no final line break: the line
                    // break in front of it goes with it, as long as it is still there.
                    let own = if first.eol.is_empty() {
                        tail_eol
                    } else {
                        first.eol.len()
                    };
                    out.truncate(out.len() - own.min(tail_eol));
                }
                tail_eol = 0;
            }
        }
        index = region.end;
    }
    out
}

/// What a hosts file holds between our markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockInspection {
    /// No marker of ours.
    Absent,
    /// One block, BEGIN and END present, and the hash and the number of names the marker
    /// declares are those of the lines.
    Intact { sha256: String, names: u32 },
    /// A marker of ours is there, but the lines are not what the marker declares, a marker
    /// is missing, or there is more than one block.
    Changed,
}

/// Reads the block out of a hosts file.
pub fn inspect(existing: &[u8]) -> BlockInspection {
    let (_, rest) = split_bom(existing);
    let lines = split_lines(rest);
    match regions(&lines).as_slice() {
        [] => BlockInspection::Absent,
        [region] if region.complete => {
            let body = &lines[region.start + 1..region.end - 1];
            match declared(lines[region.start].text) {
                Some((sha256, names))
                    if sha256 == body_hash(body) && names == count_names(body) =>
                {
                    BlockInspection::Intact { sha256, names }
                }
                _ => BlockInspection::Changed,
            }
        }
        _ => BlockInspection::Changed,
    }
}

/// The hash and the number of names a BEGIN line declares.
fn declared(begin: &[u8]) -> Option<(String, u32)> {
    let rest = trim_end(begin).strip_prefix(BEGIN_PREFIX.as_bytes())?;
    let rest = std::str::from_utf8(rest).ok()?;
    let mut fields = rest.split(' ').filter(|field| !field.is_empty());
    let sha256 = fields.next()?.strip_prefix("sha256=")?;
    let names = fields.next()?.strip_prefix("names=")?;
    if fields.next().is_some()
        || sha256.len() != 64
        || !sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || names.is_empty()
        || !names.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((sha256.to_owned(), names.parse().ok()?))
}

fn body_hash(body: &[Line<'_>]) -> String {
    let mut hasher = Sha256::new();
    for (index, line) in body.iter().enumerate() {
        if index > 0 {
            hasher.update(LF);
        }
        hasher.update(line.text);
    }
    HEXLOWER.encode(&hasher.finalize())
}

/// The names on an address line of `family` (`0.0.0.0` or `::`), however it is spaced.
fn address_names<'a>(text: &'a [u8], family: &str) -> Option<impl Iterator<Item = &'a [u8]>> {
    let mut fields = text
        .split(|b| *b == b' ' || *b == b'\t')
        .filter(|field| !field.is_empty());
    (fields.next()? == family.as_bytes()).then_some(fields)
}

fn count_names(body: &[Line<'_>]) -> u32 {
    let count: usize = body
        .iter()
        .filter_map(|line| address_names(line.text, NULL_V4))
        .map(Iterator::count)
        .sum();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The names of the block in `existing`, in file order and without duplicates (empty when
/// there is no block). For a damaged block: the names on the lines that are still ours.
pub fn block_names(existing: &[u8]) -> Vec<String> {
    let (_, rest) = split_bom(existing);
    let lines = split_lines(rest);
    let mut names: Vec<String> = Vec::new();
    let mut seen: HashSet<&[u8]> = HashSet::new();
    for region in regions(&lines) {
        for line in &lines[region.start..region.end] {
            let fields = address_names(line.text, NULL_V4)
                .into_iter()
                .flatten()
                .chain(address_names(line.text, NULL_V6).into_iter().flatten());
            for field in fields {
                if let Ok(name) = std::str::from_utf8(field) {
                    if seen.insert(field) {
                        names.push(name.to_owned());
                    }
                }
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    fn sample_block() -> String {
        render_block(&names(&[CANARY_NAME, "example.com", "www.example.com"]))
    }

    fn other_block() -> String {
        render_block(&names(&[
            CANARY_NAME,
            "a.example",
            "b.example",
            "c.example",
            "d.example",
            "e.example",
            "f.example",
            "g.example",
        ]))
    }

    #[test]
    fn caps_per_os() {
        assert_eq!(cap_for(Os::Windows), 8_000);
        assert_eq!(cap_for(Os::Macos), 20_000);
        assert_eq!(cap_for(Os::Linux), 20_000);
    }

    #[test]
    fn valid_hostnames() {
        for name in [
            "example.com",
            "a.io",
            "sub.example.co.uk",
            "xn--bcher-kva.example",
            "my-site.example.com",
            "123.example.com",
            "1337x.to",
            "banco.bradesco",
            CANARY_NAME,
        ] {
            assert!(is_valid_hostname(name), "{name} should be valid");
        }
        let label = "a".repeat(63);
        assert!(is_valid_hostname(&format!("{label}.com")));
        let longest = format!("{}.{}.{}.{}", label, label, label, "a".repeat(61));
        assert_eq!(longest.len(), 253);
        assert!(is_valid_hostname(&longest));
    }

    #[test]
    fn hostile_or_broken_names_are_refused() {
        let long_label = format!("{}.com", "a".repeat(64));
        let too_long = format!(
            "{}.{}.{}.{}",
            "a".repeat(63),
            "a".repeat(63),
            "a".repeat(63),
            "a".repeat(62)
        );
        assert_eq!(too_long.len(), 254);
        for name in [
            "",
            "a.b",
            "com",
            "localhost",
            ".com",
            "example.",
            "example..com",
            "example.com.",
            "-example.com",
            "example-.com",
            "exa mple.com",
            "example.com ",
            " example.com",
            "example.com\n0.0.0.0 evil.example",
            "example.com\r",
            "example.com\t127.0.0.1",
            "example.com#comment",
            "exam_ple.com",
            "Example.com",
            "EXAMPLE.COM",
            "exämple.com",
            "example.com/path",
            "../etc/passwd",
            "..\\windows\\system32",
            "/etc/hosts",
            "C:\\hosts.txt",
            "--plan-b64=x.y",
            "-rf.example.com",
            "example.com;reboot",
            "$(reboot).example.com",
            "`id`.example.com",
            "example.com|sh",
            "example.com&calc",
            "a'b.example.com",
            "a\"b.example.com",
            "example.com\0",
            "user@example.com",
            "example.com:443",
            "http://example.com",
            "*.example.com",
            "1.2.3.4",
            "127.0.0.1",
            "0.0.0.0",
            "127.1",
            "0x7f.0.0.1",
            "0x7f.1",
            "017.0.0.1",
            "1.2.3.0x4",
            "example.0x",
            "::1",
            "[::1]",
            "fe80::1",
            "mypc.local",
            "printer.lan",
            "foo.localhost",
            "localhost.localdomain",
            "nas.home.arpa",
            "wiki.internal",
            "build.corp",
            "site.test",
            long_label.as_str(),
            too_long.as_str(),
        ] {
            assert!(!is_valid_hostname(name), "{name:?} should be refused");
        }
    }

    #[test]
    fn names_for_orders_canary_extras_twins_and_tier() {
        let tier = ["tier1.example", "example.com", "tier2.example", "bad name"];
        let extras = names(&[
            "example.com",
            "not valid",
            "www.second.example",
            "example.com",
            "localhost",
        ]);
        assert_eq!(
            names_for(&tier, &extras, Os::Linux),
            names(&[
                CANARY_NAME,
                "example.com",
                "www.example.com",
                "www.second.example",
                "tier1.example",
                "tier2.example",
            ])
        );
    }

    #[test]
    fn names_for_never_exceeds_the_cap_and_keeps_extras_first() {
        let tier_owned: Vec<String> = (0..25_000).map(|n| format!("site{n}.example")).collect();
        let tier: Vec<&str> = tier_owned.iter().map(String::as_str).collect();
        let extras = names(&["mine.example"]);
        let windows = names_for(&tier, &extras, Os::Windows);
        assert_eq!(windows.len(), CAP_WINDOWS);
        assert_eq!(
            windows[..3],
            names(&[CANARY_NAME, "mine.example", "www.mine.example"])[..]
        );
        assert_eq!(windows[3], "site0.example");
        assert_eq!(
            windows[CAP_WINDOWS - 1],
            format!("site{}.example", CAP_WINDOWS - 4)
        );
        assert_eq!(names_for(&tier, &extras, Os::Macos).len(), CAP_UNIX);
        assert_eq!(names_for(&tier, &extras, Os::Linux).len(), CAP_UNIX);
        // A short tier is taken whole.
        assert_eq!(names_for(&tier[..10], &[], Os::Windows).len(), 11);
    }

    #[test]
    fn render_block_is_the_documented_text() {
        let block = render_block(&names(&[
            CANARY_NAME,
            "a.example",
            "b.example",
            "c.example",
            "d.example",
            "e.example",
            "f.example",
        ]));
        let lines: Vec<&str> = block.lines().collect();
        assert_eq!(lines.len(), 7);
        assert!(block.ends_with("# <<< ubiqx-shield END\n"));
        assert_eq!(lines[1], UNDO_HINT);
        assert_eq!(
            lines[2],
            "0.0.0.0 canary.ubiqx-shield.invalid a.example b.example c.example d.example e.example"
        );
        assert_eq!(
            lines[3],
            ":: canary.ubiqx-shield.invalid a.example b.example c.example d.example e.example"
        );
        assert_eq!(lines[4], "0.0.0.0 f.example");
        assert_eq!(lines[5], ":: f.example");
        assert_eq!(lines[6], END_MARKER);
        let sha256 = body_sha256(&lines[1..6]);
        assert_eq!(lines[0], format!("{BEGIN_PREFIX} sha256={sha256} names=7"));
        assert_eq!(sha256.len(), 64);
        assert_eq!(
            stamp_of(&names(&[
                CANARY_NAME,
                "a.example",
                "b.example",
                "c.example",
                "d.example",
                "e.example",
                "f.example",
            ])),
            HostsStamp { sha256, names: 7 }
        );
    }

    #[test]
    fn body_hash_is_sha256_of_the_lines_joined_with_lf() {
        // SHA-256 of "a\nb".
        assert_eq!(
            body_sha256(&["a", "b"]),
            HEXLOWER.encode(&Sha256::digest(b"a\nb"))
        );
        // SHA-256 of the empty string.
        assert_eq!(
            body_sha256(&[]),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn render_block_writes_nothing_but_names() {
        let block = render_block(&names(&[
            "ok.example",
            "evil.example\n127.0.0.1 bank.example",
            "has space.example",
            "tab\t.example",
            "# comment",
            "",
            "UPPER.example",
        ]));
        assert_eq!(
            block.lines().collect::<Vec<_>>()[2..4],
            ["0.0.0.0 ok.example", ":: ok.example"]
        );
        assert_eq!(block.lines().count(), 5);
        assert!(block.lines().next().unwrap().ends_with(" names=1"));
        assert_eq!(
            inspect(block.as_bytes()),
            BlockInspection::Intact {
                sha256: stamp_of(&names(&["ok.example"])).sha256,
                names: 1
            }
        );
    }

    #[test]
    fn splice_appends_replaces_and_removes() {
        let block = sample_block();
        let original = b"127.0.0.1 localhost\n::1 localhost\n";
        let with = splice(original, Some(&block));
        assert_eq!(with, [original.as_slice(), block.as_bytes()].concat());
        assert_eq!(splice(&with, Some(&block)), with);
        assert_eq!(splice(&with, None), original);

        let other = other_block();
        let replaced = splice(&with, Some(&other));
        assert_eq!(replaced, [original.as_slice(), other.as_bytes()].concat());
        assert_eq!(splice(&replaced, None), original);

        // A block in the middle: what follows it stays where it is.
        let middle = [original.as_slice(), block.as_bytes(), b"10.0.0.5 nas\n"].concat();
        assert_eq!(
            splice(&middle, Some(&other)),
            [original.as_slice(), other.as_bytes(), b"10.0.0.5 nas\n"].concat()
        );
        assert_eq!(
            splice(&middle, None),
            b"127.0.0.1 localhost\n::1 localhost\n10.0.0.5 nas\n"
        );
    }

    #[test]
    fn splice_into_an_empty_file_and_out_again() {
        let block = sample_block();
        assert_eq!(splice(b"", Some(&block)), block.as_bytes());
        assert_eq!(splice(block.as_bytes(), None), b"");
        assert_eq!(splice(b"", None), b"");
    }

    #[test]
    fn splice_keeps_a_missing_final_newline() {
        let block = sample_block();
        let original = b"127.0.0.1 localhost";
        let with = splice(original, Some(&block));
        let expected = format!("127.0.0.1 localhost\n{}", block.trim_end_matches('\n'));
        assert_eq!(with, expected.as_bytes());
        assert_eq!(splice(&with, Some(&block)), with);
        assert_eq!(splice(&with, None), original);
        assert_eq!(inspect(&with), inspect(block.as_bytes()));
    }

    #[test]
    fn splice_uses_the_line_endings_of_the_file() {
        let block = sample_block();
        let original = b"127.0.0.1 localhost\r\n::1 localhost\r\n";
        let with = splice(original, Some(&block));
        let crlf_block = block.replace('\n', "\r\n");
        assert_eq!(with, [original.as_slice(), crlf_block.as_bytes()].concat());
        // The hash is over the lines, so the converted block is still intact.
        assert_eq!(inspect(&with), inspect(block.as_bytes()));
        assert!(matches!(
            inspect(&with),
            BlockInspection::Intact { names: 3, .. }
        ));
        assert_eq!(splice(&with, None), original);
        // A replacement keeps the endings the block already has.
        let replaced = splice(&with, Some(&other_block()));
        let other_crlf = other_block().replace('\n', "\r\n");
        assert_eq!(
            replaced,
            [original.as_slice(), other_crlf.as_bytes()].concat()
        );
    }

    #[test]
    fn splice_keeps_a_byte_order_mark_and_a_lone_carriage_return() {
        let block = sample_block();
        let bom_only = [0xEF, 0xBB, 0xBF];
        let with = splice(&bom_only, Some(&block));
        assert_eq!(with, [bom_only.as_slice(), block.as_bytes()].concat());
        assert_eq!(splice(&with, None), bom_only);

        let bom_file = [bom_only.as_slice(), b"127.0.0.1 localhost\n"].concat();
        let with = splice(&bom_file, Some(&block));
        assert!(with.starts_with(&bom_file));
        assert_eq!(splice(&with, None), bom_file);

        // A file that ends with a lone carriage return keeps it.
        for original in [&b"127.0.0.1 localhost\r"[..], b"a\nb\r", b"\r", b"x\r\r"] {
            let with = splice(original, Some(&block));
            assert!(with.starts_with(original));
            assert_eq!(splice(&with, Some(&block)), with);
            assert_eq!(splice(&with, None), original, "{original:?}");
        }
    }

    #[test]
    fn a_marker_followed_by_a_lone_carriage_return_is_not_a_marker() {
        // The last line is no marker of ours as long as the carriage return is part of
        // it. Appending must not turn that carriage return into half a line ending.
        let block = sample_block();
        for original in [
            &b"# <<< ubiqx-shield END\r"[..],
            b"127.0.0.1 localhost\n# >>> ubiqx-shield v1 BEGIN\r",
            b"0.0.0.0 a.example\n:: a.example\n# <<< ubiqx-shield END\r",
        ] {
            assert_eq!(inspect(original), BlockInspection::Absent);
            let with = splice(original, Some(&block));
            assert!(with.starts_with(original));
            assert_eq!(inspect(&with), inspect(block.as_bytes()));
            assert_eq!(splice(&with, None), original, "{original:?}");
        }
    }

    #[test]
    fn a_foreign_block_with_a_similar_marker_is_untouched() {
        let foreign = concat!(
            "# >>> ubiqx-shield v2 BEGIN sha256=00 names=1\n",
            "0.0.0.0 keep.example\n",
            ":: keep.example\n",
            "# <<< ubiqx-shield v2 END\n",
            "# >>> other-shield v1 BEGIN\n",
            "0.0.0.0 also.example\n",
            "# <<< other-shield END\n",
            " # >>> ubiqx-shield v1 BEGIN\n",
            "# >>> ubiqx-shield v1 BEGINNING\n",
            "#>>> ubiqx-shield v1 BEGIN\n",
            "# <<< ubiqx-shield ENDED\n",
            "## <<< ubiqx-shield END\n",
        );
        assert_eq!(inspect(foreign.as_bytes()), BlockInspection::Absent);
        assert_eq!(splice(foreign.as_bytes(), None), foreign.as_bytes());
        assert!(block_names(foreign.as_bytes()).is_empty());
        let block = sample_block();
        let with = splice(foreign.as_bytes(), Some(&block));
        assert_eq!(with, [foreign.as_bytes(), block.as_bytes()].concat());
        assert_eq!(splice(&with, None), foreign.as_bytes());
    }

    #[test]
    fn a_block_whose_end_marker_is_gone_is_replaced_not_duplicated() {
        let block = sample_block();
        let without_end = block.trim_end_matches('\n').rsplit_once('\n').unwrap().0;
        // The user's own lines follow; a single 0.0.0.0 line is not of our making.
        let damaged = format!("127.0.0.1 localhost\n{without_end}\n0.0.0.0 ads.example\n# mine\n");
        assert_eq!(inspect(damaged.as_bytes()), BlockInspection::Changed);
        let restored = splice(damaged.as_bytes(), Some(&block));
        assert_eq!(
            restored,
            format!("127.0.0.1 localhost\n{block}0.0.0.0 ads.example\n# mine\n").as_bytes()
        );
        assert!(matches!(inspect(&restored), BlockInspection::Intact { .. }));
        assert_eq!(
            splice(damaged.as_bytes(), None),
            b"127.0.0.1 localhost\n0.0.0.0 ads.example\n# mine\n"
        );
    }

    #[test]
    fn a_block_whose_begin_marker_is_damaged_is_cleaned_up() {
        let block = sample_block();
        let damaged_begin = block.replacen(
            "# >>> ubiqx-shield v1 BEGIN",
            "# >> ubiqx-shield v1 BEGIN",
            1,
        );
        let first_line = damaged_begin.lines().next().unwrap();
        let file = format!("# top\n0.0.0.0 ads.example\n{damaged_begin}# bottom\n");
        assert_eq!(inspect(file.as_bytes()), BlockInspection::Changed);
        // The line that no longer is a marker of ours stays; our lines and END go.
        let expected_rest = format!("# top\n0.0.0.0 ads.example\n{first_line}\n");
        assert_eq!(
            splice(file.as_bytes(), None),
            format!("{expected_rest}# bottom\n").as_bytes()
        );
        assert_eq!(
            splice(file.as_bytes(), Some(&block)),
            format!("{expected_rest}{block}# bottom\n").as_bytes()
        );
    }

    #[test]
    fn edits_inside_the_markers_are_replaced_and_everything_else_stays() {
        let block = sample_block();
        let edited = block.replace("0.0.0.0 canary", "0.0.0.0 added.example canary");
        let file = format!("# top\n{edited}# bottom");
        assert_eq!(inspect(file.as_bytes()), BlockInspection::Changed);
        assert_eq!(
            splice(file.as_bytes(), Some(&block)),
            format!("# top\n{block}# bottom").as_bytes()
        );
        assert_eq!(splice(file.as_bytes(), None), b"# top\n# bottom");
        assert_eq!(block_names(file.as_bytes())[0], "added.example");
    }

    #[test]
    fn two_blocks_become_one() {
        let block = sample_block();
        let file = format!("# top\n{block}# between\n{block}# bottom\n");
        assert_eq!(inspect(file.as_bytes()), BlockInspection::Changed);
        let other = other_block();
        assert_eq!(
            splice(file.as_bytes(), Some(&other)),
            format!("# top\n{other}# between\n# bottom\n").as_bytes()
        );
        assert_eq!(
            splice(file.as_bytes(), None),
            b"# top\n# between\n# bottom\n"
        );
        // The second block closes the file without a line break: it takes the line break
        // in front of it along, here the one that ends the block written first.
        let unended = format!("# top\n{block}{}", block.trim_end_matches('\n'));
        assert_eq!(
            splice(unended.as_bytes(), Some(&other)),
            format!("# top\n{}", other.trim_end_matches('\n')).as_bytes()
        );
        assert_eq!(splice(unended.as_bytes(), None), b"# top\n");
    }

    #[test]
    fn splice_refuses_a_text_that_is_not_a_block() {
        let file = b"127.0.0.1 localhost\n";
        for not_a_block in [
            "",
            "0.0.0.0 evil.example\n",
            "# >>> ubiqx-shield v1 BEGIN sha256=x names=1\n0.0.0.0 a.example\n",
            "0.0.0.0 a.example\n# <<< ubiqx-shield END\n",
            "# >>> ubiqx-shield v1 BEGIN\n# <<< ubiqx-shield END\n# <<< ubiqx-shield END\n",
            "# >>> ubiqx-shield v1 BEGIN\n# >>> ubiqx-shield v1 BEGIN\n# <<< ubiqx-shield END\n",
        ] {
            assert_eq!(splice(file, Some(not_a_block)), file, "{not_a_block:?}");
        }
    }

    #[test]
    fn inspect_tells_intact_from_changed() {
        let block = sample_block();
        let stamp = stamp_of(&names(&[CANARY_NAME, "example.com", "www.example.com"]));
        assert_eq!(inspect(b""), BlockInspection::Absent);
        assert_eq!(inspect(b"127.0.0.1 localhost\n"), BlockInspection::Absent);
        assert_eq!(
            inspect(block.as_bytes()),
            BlockInspection::Intact {
                sha256: stamp.sha256.clone(),
                names: 3
            }
        );
        // A name edited, a line deleted, the declared hash edited, the count edited.
        let line_gone: String = block
            .lines()
            .filter(|line| !line.starts_with("::"))
            .map(|l| format!("{l}\n"))
            .collect();
        for changed in [
            block.replace("example.com", "exemple.com"),
            line_gone,
            block.replacen("sha256=", "sha256=0", 1),
            block.replacen("names=3", "names=4", 1),
            block.replacen(" names=3", "", 1),
            block.replacen(UNDO_HINT, "# hint", 1),
        ] {
            assert_eq!(
                inspect(changed.as_bytes()),
                BlockInspection::Changed,
                "{changed}"
            );
        }
        // Trailing blanks on a marker line and CRLF endings do not change the block.
        let spaced = block
            .replacen("names=3\n", "names=3 \t\n", 1)
            .replace('\n', "\r\n");
        assert!(matches!(
            inspect(spaced.as_bytes()),
            BlockInspection::Intact { .. }
        ));
    }

    #[test]
    fn block_names_lists_each_name_once_in_file_order() {
        let file = format!("0.0.0.0 outside.example\n{}", other_block());
        assert_eq!(
            block_names(file.as_bytes()),
            names(&[
                CANARY_NAME,
                "a.example",
                "b.example",
                "c.example",
                "d.example",
                "e.example",
                "f.example",
                "g.example",
            ])
        );
        assert!(block_names(b"0.0.0.0 outside.example\n").is_empty());
    }

    // -----------------------------------------------------------------------------------------
    // Property test: generated files
    // -----------------------------------------------------------------------------------------

    /// A small deterministic generator (xorshift64*), so a failure names its seed.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn chance(&mut self, one_in: usize) -> bool {
            self.below(one_in) == 0
        }
    }

    const FOREIGN_LINES: &[&[u8]] = &[
        b"127.0.0.1 localhost",
        b"::1 localhost ip6-localhost ip6-loopback",
        b"255.255.255.255 broadcasthost",
        b"# a comment",
        b"",
        b"   ",
        b"\t# indented comment",
        b"0.0.0.0 ads.example.com",
        b":: ads.example.com",
        b"0.0.0.0\ttracker.example # blocked by me",
        b"10.0.0.5 nas nas.home",
        b"# >>> ubiqx-shield v2 BEGIN sha256=00 names=1",
        b"# <<< ubiqx-shield v2 END",
        b"# >>> other-shield v1 BEGIN",
        b"# <<< other-shield END",
        b" # >>> ubiqx-shield v1 BEGIN",
        b"# >>> ubiqx-shield v1 BEGINNING sha256=00 names=0",
        b"#>>> ubiqx-shield v1 BEGIN",
        b"# >> ubiqx-shield v1 BEGIN sha256=00 names=0",
        b"# <<< ubiqx-shield ENDED",
        b"## <<< ubiqx-shield END",
        b" # <<< ubiqx-shield END",
        b"# Added by the ubiqx-shield app on this computer.",
        b"caf\xC3\xA9 # utf-8",
        b"\xFF\xFE not utf-8 \x80",
        b"nul\0byte",
        b"carriage\rreturn inside",
        b"\xEF\xBB\xBF# a mark in the middle of the file",
    ];

    /// One line of somebody else's content, without its line ending.
    fn foreign_line(rng: &mut Rng) -> Vec<u8> {
        match rng.below(12) {
            0 => {
                // A very long line.
                let len = 5_000 + rng.below(60_000);
                let mut line = b"0.0.0.0".to_vec();
                while line.len() < len {
                    line.extend_from_slice(b" long.example");
                }
                line
            }
            1 => (0..rng.below(40))
                .map(|_| match rng.below(256) as u8 {
                    b'\n' => b' ',
                    byte => byte,
                })
                .collect(),
            _ => FOREIGN_LINES[rng.below(FOREIGN_LINES.len())].to_vec(),
        }
    }

    fn eol_for(rng: &mut Rng, style: usize) -> &'static [u8] {
        match style {
            0 => LF,
            1 => CRLF,
            _ if rng.chance(2) => LF,
            _ => CRLF,
        }
    }

    /// Lines of foreign content, each ended; `style` is LF, CRLF or mixed.
    fn foreign_text(rng: &mut Rng, style: usize, max_lines: usize) -> Vec<u8> {
        let mut text = Vec::new();
        for _ in 0..rng.below(max_lines + 1) {
            text.extend_from_slice(&foreign_line(rng));
            text.extend_from_slice(eol_for(rng, style));
        }
        text
    }

    /// A whole file that holds nothing of ours.
    fn foreign_file(rng: &mut Rng) -> Vec<u8> {
        let style = rng.below(3);
        let mut file = Vec::new();
        if rng.chance(4) {
            file.extend_from_slice(BOM);
        }
        file.extend_from_slice(&foreign_text(rng, style, 12));
        if rng.chance(2) {
            // No final line break; sometimes a lone carriage return instead.
            file.extend_from_slice(&foreign_line(rng));
            if rng.chance(5) {
                file.push(b'\r');
            }
        }
        file
    }

    fn generated_block(rng: &mut Rng) -> String {
        let mut list = vec![CANARY_NAME.to_owned()];
        for n in 0..rng.below(40) {
            list.push(format!("site{n}-{}.example", rng.below(1_000)));
        }
        render_block(&list)
    }

    fn with_eol(block: &str, eol: &[u8]) -> Vec<u8> {
        block
            .lines()
            .flat_map(|line| [line.as_bytes(), eol].concat())
            .collect()
    }

    #[test]
    fn property_splice_round_trips_on_files_without_a_block() {
        for seed in 1..=1_500u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let file = foreign_file(&mut rng);
            let block = generated_block(&mut rng);
            let other = generated_block(&mut rng);
            assert_eq!(inspect(&file), BlockInspection::Absent, "seed {seed}");
            assert_eq!(splice(&file, None), file, "seed {seed}: removing nothing");

            let with = splice(&file, Some(&block));
            assert!(with.starts_with(&file), "seed {seed}: the file is a prefix");
            assert_eq!(inspect(&with), inspect(block.as_bytes()), "seed {seed}");
            assert!(
                matches!(inspect(&with), BlockInspection::Intact { .. }),
                "seed {seed}"
            );
            assert_eq!(splice(&with, Some(&block)), with, "seed {seed}: idempotent");
            assert_eq!(splice(&with, None), file, "seed {seed}: round trip");

            let replaced = splice(&with, Some(&other));
            assert!(replaced.starts_with(&file), "seed {seed}: replaced prefix");
            assert_eq!(inspect(&replaced), inspect(other.as_bytes()), "seed {seed}");
            assert_eq!(
                splice(&replaced, None),
                file,
                "seed {seed}: round trip after replace"
            );
            assert_eq!(
                block_names(&replaced),
                block_names(other.as_bytes()),
                "seed {seed}"
            );

            // A final line break is neither added nor taken away.
            assert_eq!(
                with.ends_with(b"\n"),
                file.is_empty() || file == BOM || file.ends_with(b"\n"),
                "seed {seed}: final line break"
            );
        }
    }

    #[test]
    fn property_the_block_takes_the_line_endings_of_the_file() {
        for seed in 1..=300u64 {
            let mut rng = Rng(seed.wrapping_mul(0xD6E8_FEB8_6659_FD93) | 1);
            let block = generated_block(&mut rng);
            for (style, eol) in [(0, LF), (1, CRLF)] {
                let mut file = foreign_text(&mut rng, style, 8);
                file.extend_from_slice(b"# last");
                file.extend_from_slice(eol);
                let with = splice(&file, Some(&block));
                assert_eq!(
                    with,
                    [file.as_slice(), with_eol(&block, eol).as_slice()].concat(),
                    "seed {seed} style {style}"
                );
            }
        }
    }

    /// What stands in the file where a block of ours is, or was.
    enum Ours {
        /// BEGIN to END, as written.
        Whole,
        /// BEGIN to END with lines edited, added and deleted in between.
        Edited,
        /// The END line is gone.
        NoEnd,
        /// The BEGIN line is no longer a marker (it stays in the file as a foreign line).
        NoBegin,
    }

    #[test]
    fn property_everything_outside_the_block_is_byte_exact() {
        for seed in 1..=1_500u64 {
            let mut rng = Rng(seed.wrapping_mul(0xA24B_AED4_963E_E407) | 1);
            let style = rng.below(3);
            let block_eol = eol_for(&mut rng, style);
            let mut before = Vec::new();
            if rng.chance(4) {
                before.extend_from_slice(BOM);
            }
            before.extend_from_slice(&foreign_text(&mut rng, style, 8));
            let mut after = foreign_text(&mut rng, style, 8);
            // Lines of our making are told from foreign ones by the pair shape; keep a
            // plain comment next to the damaged ends so that the expectation is exact.
            before.extend_from_slice(b"# before");
            before.extend_from_slice(block_eol);
            after.splice(0..0, [b"# after".as_slice(), block_eol].concat());
            if rng.chance(3) {
                // The file ends without a line break, after foreign content.
                after.extend_from_slice(b"# no line break at the end");
            }

            let old = generated_block(&mut rng);
            let old_lines: Vec<&str> = old.lines().collect();
            let kind = match rng.below(4) {
                0 => Ours::Whole,
                1 => Ours::Edited,
                2 => Ours::NoEnd,
                _ => Ours::NoBegin,
            };
            let mut stays_before = before.clone();
            let mut ours: Vec<Vec<u8>> = old_lines
                .iter()
                .map(|line| line.as_bytes().to_vec())
                .collect();
            match kind {
                Ours::Whole => {}
                Ours::Edited => {
                    for _ in 0..1 + rng.below(4) {
                        // Index of a line strictly between BEGIN and END.
                        let end = ours.len() - 1;
                        let at = 1 + rng.below(end - 1);
                        match rng.below(3) {
                            0 => {
                                let line = foreign_line(&mut rng);
                                let marker = is_begin(&line) || is_end(&line);
                                ours.insert(
                                    at,
                                    if marker {
                                        b"# no marker".to_vec()
                                    } else {
                                        line
                                    },
                                );
                            }
                            1 if end > 2 => {
                                ours.remove(at);
                            }
                            _ => ours[at] = b"0.0.0.0 edited.example".to_vec(),
                        }
                    }
                }
                Ours::NoEnd => {
                    ours.pop();
                }
                Ours::NoBegin => {
                    let damaged = ours.remove(0)[1..].to_vec();
                    stays_before.extend_from_slice(&damaged);
                    stays_before.extend_from_slice(block_eol);
                }
            }
            let ours_bytes: Vec<u8> = ours
                .iter()
                .flat_map(|line| [line.as_slice(), block_eol].concat())
                .collect();
            let file = [stays_before.as_slice(), &ours_bytes, &after].concat();

            let removed = splice(&file, None);
            assert_eq!(
                removed,
                [stays_before.as_slice(), &after].concat(),
                "seed {seed}: removal leaves the rest"
            );
            assert_eq!(inspect(&removed), BlockInspection::Absent, "seed {seed}");

            let new = generated_block(&mut rng);
            let spliced = splice(&file, Some(&new));
            assert_eq!(
                spliced,
                [stays_before.as_slice(), &with_eol(&new, block_eol), &after].concat(),
                "seed {seed}: replacement leaves the rest"
            );
            assert_eq!(inspect(&spliced), inspect(new.as_bytes()), "seed {seed}");
            assert_eq!(
                splice(&spliced, Some(&new)),
                spliced,
                "seed {seed}: idempotent"
            );
            assert_eq!(splice(&spliced, None), removed, "seed {seed}: then removed");
        }
    }

    #[test]
    fn property_a_block_at_the_end_of_a_file_without_final_line_break() {
        for seed in 1..=300u64 {
            let mut rng = Rng(seed.wrapping_mul(0x8CB9_2BA7_2F3D_8DD7) | 1);
            let file = foreign_file(&mut rng);
            let block = generated_block(&mut rng);
            let other = generated_block(&mut rng);
            let with = splice(&file, Some(&block));
            // Replacing a block keeps whether the file ends with a line break.
            let replaced = splice(&with, Some(&other));
            assert_eq!(
                replaced.ends_with(b"\n"),
                with.ends_with(b"\n"),
                "seed {seed}"
            );
            assert_eq!(replaced, splice(&file, Some(&other)), "seed {seed}");
        }
    }
}
