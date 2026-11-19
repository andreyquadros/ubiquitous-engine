//! Parsers of what the system tools print. None depends on the system language: a line
//! is used only for what can be recognised without reading words (an IP address, a
//! registry type, JSON), and `netsh` and `ipconfig /all` are never parsed.

use std::net::IpAddr;

use serde::Deserialize;
use serde_json::Value;
use ubiqx_core::nofap::PolicyValue;

/// The services of `networksetup -listallnetworkservices` as `(name, enabled)`. The first
/// line is a header in the system language and is skipped; a leading `*` marks a disabled
/// service.
pub fn network_services(output: &str) -> Vec<(String, bool)> {
    output
        .lines()
        .skip(1)
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.strip_prefix('*') {
            Some(name) => (name.to_owned(), false),
            None => (line.to_owned(), true),
        })
        .filter(|(name, _)| !name.is_empty())
        .collect()
}

/// The DNS servers in the output of `networksetup -getdnsservers <service>`: every line
/// that is an IP address. A sentence saying there are none (in any language) has no such
/// line, which means "automatic".
pub fn ip_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| is_address(line))
        .map(str::to_owned)
        .collect()
}

/// Whether `text` is an IP address, with or without an IPv6 zone (`fe80::1%en0`).
fn is_address(text: &str) -> bool {
    let address = text.split_once('%').map_or(text, |(address, _)| address);
    address.parse::<IpAddr>().is_ok()
}

/// The entries of a registry `NameServer` value: separated by commas or blanks. Every
/// entry is kept, address or not, so a list with something else in it never reads as
/// exactly a provider's set.
pub fn name_servers(data: &str) -> Vec<String> {
    data.split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// One value printed by `reg.exe query`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegValue {
    /// The key the value is under, as printed (`HKEY_LOCAL_MACHINE\…`).
    pub key: String,
    pub name: String,
    /// `REG_SZ`, `REG_DWORD`, …
    pub kind: String,
    pub data: String,
}

/// The values in the output of `reg.exe query`. A key is a line that starts at the first
/// column with `HKEY_`; a value is an indented line of name, type and data separated by
/// four blanks. Anything else (the localised summary of a search, blank lines) is
/// skipped, and both line endings are accepted.
pub fn reg_values(output: &str) -> Vec<RegValue> {
    const GAP: &str = "    ";
    let mut values = Vec::new();
    let mut key = String::new();
    for line in output.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("HKEY_") {
            key = line.trim_end().to_owned();
            continue;
        }
        let Some(rest) = line.strip_prefix(GAP) else {
            continue;
        };
        let mut fields = rest.splitn(3, GAP);
        let (Some(name), Some(kind)) = (fields.next(), fields.next()) else {
            continue;
        };
        if !kind.starts_with("REG_") || key.is_empty() {
            continue;
        }
        values.push(RegValue {
            key: key.clone(),
            name: name.trim().to_owned(),
            kind: kind.trim().to_owned(),
            data: fields.next().unwrap_or_default().trim().to_owned(),
        });
    }
    values
}

/// A registry value as JSON: `REG_DWORD` and `REG_QWORD` (`0x1`) are numbers, the string
/// types are strings, anything else is not a policy value this tier reads.
pub fn reg_json(value: &RegValue) -> Option<Value> {
    match value.kind.as_str() {
        "REG_DWORD" | "REG_QWORD" => {
            let digits = value.data.strip_prefix("0x")?;
            u64::from_str_radix(digits, 16).ok().map(Value::from)
        }
        "REG_SZ" | "REG_EXPAND_SZ" => Some(Value::from(value.data.as_str())),
        _ => None,
    }
}

/// The last component of a registry key path (an interface GUID under `Interfaces`).
pub fn key_leaf(key: &str) -> &str {
    key.rsplit('\\').next().unwrap_or(key)
}

/// One network adapter of the full Windows read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub index: u32,
    /// `{XXXXXXXX-…}`, upper case.
    pub guid: String,
    pub name: String,
    pub servers: Vec<String>,
}

#[derive(Deserialize)]
struct RawAdapter {
    index: u32,
    #[serde(default)]
    guid: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    servers: Value,
}

/// The adapters in the JSON the full-read script prints. `ConvertTo-Json` writes a single
/// item as an object and several as an array, and a single server as a string, so both
/// shapes are read. `None` when the text is not that JSON.
pub fn adapters_json(text: &str) -> Option<Vec<Adapter>> {
    let text = text.trim().trim_start_matches('\u{feff}');
    if text.is_empty() {
        return Some(Vec::new());
    }
    let rows = match serde_json::from_str::<Value>(text).ok()? {
        Value::Array(rows) => rows,
        Value::Null => Vec::new(),
        row @ Value::Object(_) => vec![row],
        _ => return None,
    };
    let mut adapters = Vec::new();
    for row in rows {
        let raw: RawAdapter = serde_json::from_value(row).ok()?;
        let servers = match raw.servers {
            Value::Array(items) => items,
            Value::Null => Vec::new(),
            single => vec![single],
        };
        adapters.push(Adapter {
            index: raw.index,
            guid: raw.guid.trim().to_ascii_uppercase(),
            name: raw.name,
            // Every entry is kept, address or not (see `name_servers`).
            servers: servers
                .iter()
                .map(|server| match server {
                    Value::String(text) => text.trim().to_owned(),
                    other => other.to_string(),
                })
                .filter(|server| !server.is_empty())
                .collect(),
        });
    }
    Some(adapters)
}

/// The top-level keys of a preferences domain as `defaults read <domain>` prints it (an
/// old-style property list): `0` and `1` are numbers, other scalars are strings, and a
/// nested dictionary or array is `Null` (present, not read).
pub fn defaults_domain(output: &str) -> Vec<(String, Value)> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    for line in output.lines() {
        let line = line.trim();
        if depth == 1 {
            if let Some((key, value)) = line.split_once(" = ") {
                let key = key.trim().trim_matches('"').to_owned();
                let scalar = value.strip_suffix(';').map(|v| v.trim().trim_matches('"'));
                let value = match scalar {
                    Some(text) => match text.parse::<i64>() {
                        Ok(number) => Value::from(number),
                        Err(_) => Value::from(text),
                    },
                    None => Value::Null,
                };
                found.push((key, value));
            }
        }
        if line.ends_with('{') || line.ends_with('(') {
            depth += 1;
        } else if line.starts_with('}') || line.starts_with(')') {
            depth = depth.saturating_sub(1);
        }
    }
    found
}

/// The address systemd-resolved's stub listens on.
pub const RESOLVED_STUB: &str = "127.0.0.53";

/// Whether `/etc/resolv.conf` hands lookups to systemd-resolved: a `nameserver` line with
/// its stub address. Without it the service may be running and unused.
pub fn resolv_conf_uses_stub(text: &str) -> bool {
    text.lines().any(|line| {
        let mut words = line.split_whitespace();
        words.next() == Some("nameserver") && words.next() == Some(RESOLVED_STUB)
    })
}

/// The values of `keys` in a Chromium-family policy file (a JSON object). `None` when the
/// text is not a JSON object.
pub fn policy_file_values(text: &str, keys: &[String]) -> Option<Vec<PolicyValue>> {
    let Value::Object(members) = serde_json::from_str::<Value>(text).ok()? else {
        return None;
    };
    Some(
        keys.iter()
            .filter_map(|key| {
                members.get(key).map(|value| PolicyValue {
                    key: key.clone(),
                    value: value.clone(),
                })
            })
            .collect(),
    )
}
