//! The compiled table of filtering resolvers (L3). One provider row and the choice "none";
//! a second provider is one more row. The helper accepts a `ResolverId`, never an address,
//! and there are no custom addresses.

use std::collections::BTreeSet;
use std::net::IpAddr;

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
        [self.v4[0], self.v4[1], self.v6[0], self.v6[1]]
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

/// The row of `id`. The `match` has no wildcard arm, so a new id does not compile until
/// it has a row.
pub fn lookup(id: ResolverId) -> &'static ResolverSet {
    match id {
        ResolverId::CloudflareFamily => &CLOUDFLARE_FAMILY,
    }
}

/// One configured server as an address: blanks, letter case and an IPv6 zone
/// (`fe80::1%en0`) do not matter. `None` for anything that is not an IP address.
fn parse_server(server: &str) -> Option<IpAddr> {
    let server = server.trim();
    let server = server
        .split_once('%')
        .map_or(server, |(address, _)| address);
    server.parse().ok()
}

/// Whether `servers` (as configured on an interface, any order, any letter case, with or
/// without an IPv6 zone) is exactly the address set of `id`: all four addresses and
/// nothing else. A list with anything that is not an address is never ours.
pub fn is_exactly(id: ResolverId, servers: &[String]) -> bool {
    let wanted: BTreeSet<IpAddr> = lookup(id)
        .addresses()
        .iter()
        .filter_map(|address| parse_server(address))
        .collect();
    let mut found = BTreeSet::new();
    for server in servers {
        match parse_server(server) {
            Some(address) => found.insert(address),
            None => return false,
        };
    }
    found == wanted
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
    let servers = lookup(id).addresses().join(" ");
    format!("{RESOLVED_DROPIN_HINT}\n[Resolve]\nDNS={servers}\nDomains=~.\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn servers(list: &[&str]) -> Vec<String> {
        list.iter().map(|server| (*server).to_owned()).collect()
    }

    #[test]
    fn every_resolver_id_has_exactly_one_row() {
        assert_eq!(RESOLVERS.len(), ResolverId::ALL.len());
        for id in ResolverId::ALL {
            assert_eq!(lookup(id).id, id);
            assert_eq!(RESOLVERS.iter().filter(|row| row.id == id).count(), 1);
            assert!(RESOLVERS.contains(lookup(id)));
        }
    }

    #[test]
    fn every_address_of_the_table_is_an_address_of_its_family() {
        for row in RESOLVERS {
            for address in row.v4 {
                assert!(
                    matches!(address.parse::<IpAddr>(), Ok(IpAddr::V4(_))),
                    "{address}"
                );
            }
            for address in row.v6 {
                assert!(
                    matches!(address.parse::<IpAddr>(), Ok(IpAddr::V6(_))),
                    "{address}"
                );
            }
            let all: BTreeSet<&str> = row.addresses().into_iter().collect();
            assert_eq!(all.len(), 4, "four distinct addresses");
            assert!(!row.operator.is_empty());
        }
    }

    #[test]
    fn cloudflare_family_is_the_documented_set() {
        assert_eq!(
            lookup(ResolverId::CloudflareFamily).addresses(),
            [
                "1.1.1.3",
                "1.0.0.3",
                "2606:4700:4700::1113",
                "2606:4700:4700::1003"
            ]
        );
    }

    #[test]
    fn is_exactly_accepts_the_set_in_any_spelling() {
        let id = ResolverId::CloudflareFamily;
        assert!(is_exactly(
            id,
            &servers(&[
                "1.1.1.3",
                "1.0.0.3",
                "2606:4700:4700::1113",
                "2606:4700:4700::1003"
            ])
        ));
        assert!(is_exactly(
            id,
            &servers(&[
                "2606:4700:4700:0:0:0:0:1003",
                " 1.0.0.3 ",
                "2606:4700:4700::1113%eth0",
                "1.1.1.3",
                "1.1.1.3",
            ])
        ));
        assert!(is_exactly(
            id,
            &servers(&[
                "1.1.1.3",
                "1.0.0.3",
                "2606:4700:4700::1113",
                "2606:4700:4700::1003".to_uppercase().as_str()
            ])
        ));
    }

    #[test]
    fn is_exactly_refuses_anything_else() {
        let id = ResolverId::CloudflareFamily;
        for other in [
            &[][..],
            &["1.1.1.3", "1.0.0.3"],
            &["2606:4700:4700::1113", "2606:4700:4700::1003"],
            &["1.1.1.3", "1.0.0.3", "2606:4700:4700::1113"],
            &[
                "1.1.1.3",
                "1.0.0.3",
                "2606:4700:4700::1113",
                "2606:4700:4700::1003",
                "192.168.0.1",
            ],
            &[
                "1.1.1.1",
                "1.0.0.1",
                "2606:4700:4700::1111",
                "2606:4700:4700::1001",
            ],
            &[
                "1.1.1.3",
                "1.0.0.3",
                "2606:4700:4700::1113",
                "2606:4700:4700::1003",
                "not an address",
            ],
            &[
                "1.1.1.3",
                "1.0.0.3",
                "2606:4700:4700::1113",
                "2606:4700:4700::1003",
                "",
            ],
            &["192.168.0.2"],
        ] {
            assert!(!is_exactly(id, &servers(other)), "{other:?}");
        }
    }

    #[test]
    fn resolved_dropin_is_the_documented_file() {
        assert_eq!(
            resolved_dropin(ResolverId::CloudflareFamily),
            "# ubiqx-shield: delete this file and restart systemd-resolved to undo\n\
             [Resolve]\n\
             DNS=1.1.1.3 1.0.0.3 2606:4700:4700::1113 2606:4700:4700::1003\n\
             Domains=~.\n"
        );
    }
}
