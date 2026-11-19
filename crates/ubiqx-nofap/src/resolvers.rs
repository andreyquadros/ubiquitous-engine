//! The compiled table of filtering resolvers (L3). One provider row and the choice "none";
//! a second provider is one more row. The helper accepts a `ResolverId`, never an address,
//! and there are no custom addresses.
//!
//! Seeded by the foundation unit: the table is complete; the lookups are signatures.
#![allow(unused_variables)]

use ubiqx_core::nofap::ResolverId;

/// The addresses of one resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolverSet {
    pub id: ResolverId,
    pub v4: [&'static str; 2],
    pub v6: [&'static str; 2],
    /// Who operates it, as credited next to the toggle and in About.
    pub operator: &'static str,
}

impl ResolverSet {
    /// The four addresses, IPv4 first, in the order they are handed to the system.
    pub fn addresses(&self) -> [&'static str; 4] {
        todo!()
    }
}

/// Cloudflare for Families (malware and adult content).
pub const CLOUDFLARE_FAMILY: ResolverSet = ResolverSet {
    id: ResolverId::CloudflareFamily,
    v4: ["1.1.1.3", "1.0.0.3"],
    v6: ["2606:4700:4700::1113", "2606:4700:4700::1003"],
    operator: "Cloudflare for Families",
};

pub const RESOLVERS: &[ResolverSet] = &[CLOUDFLARE_FAMILY];

/// The name the helper resolves before and after a DNS change, and the verify pass
/// resolves while the tier is applied.
pub const PROBE_NAME: &str = "one.one.one.one";
/// How long the helper keeps trying the probe after the change before it rolls back.
pub const PROBE_WAIT_SECS: u64 = 8;
/// Consecutive failed probes before `Issue::ResolverUnreachable`.
pub const UNREACHABLE_AFTER_FAILS: u32 = 3;

/// The row of `id`.
pub fn lookup(id: ResolverId) -> &'static ResolverSet {
    todo!()
}

/// Whether `servers` (as configured on an interface, any order, any letter case, with or
/// without an IPv6 zone) is exactly the address set of `id`.
pub fn is_exactly(id: ResolverId, servers: &[String]) -> bool {
    todo!()
}

/// Path of the systemd-resolved drop-in the Linux back-end writes.
pub const RESOLVED_DROPIN_PATH: &str = "/etc/systemd/resolved.conf.d/90-ubiqx-shield.conf";
/// First line of the drop-in: the way out, written in the file.
pub const RESOLVED_DROPIN_HINT: &str =
    "# ubiqx-shield: delete this file and restart systemd-resolved to undo";

/// The exact content of the drop-in for `id`: the hint line, `[Resolve]`,
/// `DNS=<v4a> <v4b> <v6a> <v6b>`, `Domains=~.`, with a final newline. The verify pass
/// compares the file with this byte for byte.
pub fn resolved_dropin(id: ResolverId) -> String {
    todo!()
}
