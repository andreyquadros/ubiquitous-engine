#!/usr/bin/env bash
# System smoke test of the ubiqx-shield helper on Linux and macOS (docs/nofap/DESIGN.md 9.3).
#
# THIS SCRIPT CHANGES THE MACHINE IT RUNS ON. It runs the real helper with administrator rights:
# it writes the marked block into /etc/hosts and, for the resolver tier, points system DNS at the
# family resolver; it checks that both hold; then it takes everything off again and checks that the
# machine is as it was. It is meant for a CI runner (the job `nofap-system-smoke`) and for the one
# run in the build sandbox that opens the `(Linux, hosts)` gate. It is not for a machine in use.
#
# It therefore refuses to start unless ALL of this is true:
#   - it runs as root;
#   - `CI=true` or `UBIQX_SHIELD_SMOKE=1` is in the environment (so nobody starts it by accident;
#     `sudo` drops the environment, so say `sudo CI=true scripts/nofap/system-smoke.sh`);
#   - the machine carries nothing of the product yet: no marked block in the hosts file, no state
#     directory, no resolver drop-in. A machine where the product is in use would lose its
#     protection to the removal step, so the script does not touch it.
#
# What a run does:
#   1. Save: a copy of the hosts file with its owner and mode, the DNS configuration, what
#      `status` prints, and whether this machine resolves a name at all.
#   2. `apply` a plan for the chosen tiers (owner `shield`, no extra names), passing
#      `--unverified <tiers>` because the gate is closed until a run like this one is green.
#   3. Check: the block is in the file, intact, with the hash and the count it declares; every
#      byte in front of it is the saved file; the state file agrees; the canary and a name of the
#      list resolve to the null address through the operating system; another name still
#      resolves. Resolver tier: DNS is the provider's set (the drop-in on Linux, every enabled
#      network service on macOS); both IPv4 resolvers answer, and refuse a listed name.
#   4. `remove --owner all --scope all`.
#   5. Check the restore: the hosts file is byte for byte the saved copy, with the same owner and
#      mode; DNS is what it was; the state directory is gone; the canary no longer resolves.
#
# If anything fails after step 1, or the run is interrupted, the script puts the saved state back
# itself (the bytes, owner and mode of the hosts file, DNS, the state directory) and says so; an
# interrupt during that restore is ignored. It exits 0 only when every check passed and the
# helper's own removal restored the machine; 1 on any mismatch, also when the fallback restore
# worked; 2 when it refused to run. After a run that was not green, the work directory named in
# the last line keeps the saved hosts file and the saved DNS configuration.
#
# The expectations below (marker lines, hash rule, paths, the resolver's addresses, the plan's
# shape) are written out again here on purpose and not read from the helper: the script checks
# the helper against the design, not against itself.
#
# Usage:
#   sudo CI=true scripts/nofap/system-smoke.sh [--tiers hosts[,resolver]] [--helper <path>]
#
#   --tiers   tiers to exercise, comma separated: hosts, resolver (default: hosts,resolver)
#   --helper  the helper binary (default: <target dir>/debug/ubiqx-shield-helper, built with
#             `cargo build -p ubiqx-platform --bin ubiqx-shield-helper`); the app's own
#             executable works too, it is then run as `<exe> --shield-helper <verb>`
#
# Needs bash (3.2 is enough, as on macOS) and python3. The Windows twin is system-smoke.ps1.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

TIERS="hosts,resolver"
HELPER="${CARGO_TARGET_DIR:-$ROOT/target}/debug/ubiqx-shield-helper"

usage() {
  sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed -e 's/^# \{0,1\}//'
}

refuse() {
  echo "system-smoke: refusing to run: $*" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --tiers)
      [ $# -ge 2 ] || refuse "--tiers needs a value"
      TIERS="$2"
      shift 2
      ;;
    --tiers=*)
      TIERS="${1#--tiers=}"
      shift
      ;;
    --helper)
      [ $# -ge 2 ] || refuse "--helper needs a value"
      HELPER="$2"
      shift 2
      ;;
    --helper=*)
      HELPER="${1#--helper=}"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      refuse "unknown option: $1"
      ;;
  esac
done

# ---------------------------------------------------------------------------------------------
# Refusals. Nothing above or in this section changes anything.

case "$(uname -s)" in
  Linux) OS="linux" ;;
  Darwin) OS="macos" ;;
  *) refuse "this script is for Linux and macOS (Windows: system-smoke.ps1)" ;;
esac

case ",$TIERS," in
  ,hosts, | ,resolver, | ,hosts,resolver, | ,resolver,hosts,) ;;
  *) refuse "--tiers takes hosts, resolver or hosts,resolver (got: $TIERS)" ;;
esac

[ "$(id -u)" -eq 0 ] || refuse "it must run as root (it runs the helper's privileged verbs)"

if [ "${CI:-}" != "true" ] && [ "${UBIQX_SHIELD_SMOKE:-}" != "1" ]; then
  refuse "neither CI=true nor UBIQX_SHIELD_SMOKE=1 is set. This script changes /etc/hosts and system DNS; it is for a CI runner, not for a machine in use"
fi

command -v python3 > /dev/null 2>&1 || refuse "python3 is not on PATH"
if [ ! -f "$HELPER" ] || [ ! -x "$HELPER" ]; then
  refuse "no helper at $HELPER (build it: cargo build -p ubiqx-platform --bin ubiqx-shield-helper)"
fi

HOSTS="/etc/hosts"
DROPIN="/etc/systemd/resolved.conf.d/90-ubiqx-shield.conf"
if [ "$OS" = "macos" ]; then
  STATE_DIR="/Library/Application Support/ubiqx-shield"
else
  STATE_DIR="/var/lib/ubiqx-shield"
fi

[ -f "$HOSTS" ] || refuse "$HOSTS is not a regular file"
if grep -q -e '>>> ubiqx-shield' -e '<<< ubiqx-shield' "$HOSTS"; then
  refuse "$HOSTS already carries a ubiqx-shield block. The product is in use on this machine, and this test would remove its protection"
fi
if [ -e "$STATE_DIR" ] || [ -L "$STATE_DIR" ]; then
  refuse "$STATE_DIR exists. The product is in use on this machine, and this test would remove its protection"
fi
if [ -e "$DROPIN" ] || [ -L "$DROPIN" ]; then
  refuse "$DROPIN exists. The product is in use on this machine, and this test would remove its protection"
fi

# How this binary takes a verb: the app's executable needs the helper flag in front, the small
# helper binary takes the verb directly. `status` is unprivileged and only reads. The bare form
# is tried only for a file named like the helper binary, so an app executable is never started
# as an app.
HELPER_FORM=""
if "$HELPER" --shield-helper status > /dev/null 2>&1; then
  HELPER_FORM="flag"
elif [ "$(basename "$HELPER")" = "ubiqx-shield-helper" ] && "$HELPER" status > /dev/null 2>&1; then
  HELPER_FORM="bare"
else
  refuse "$HELPER does not answer the status verb"
fi

helper() {
  if [ "$HELPER_FORM" = "flag" ]; then
    "$HELPER" --shield-helper "$@"
  else
    "$HELPER" "$@"
  fi
}

# ---------------------------------------------------------------------------------------------
# From here on the script may leave something behind, so everything goes through `finish`.

WORK="$(mktemp -d "${TMPDIR:-/tmp}/ubiqx-shield-smoke.XXXXXX")" || refuse "cannot create a work directory"
chmod 700 "$WORK"
CHECK="$WORK/check.py"

FAILED=0     # a check did not pass
CHANGED=0    # the machine may differ from the saved state
RESTORED=0   # the helper's removal was verified

say() {
  echo "system-smoke: $*"
}

mismatch() {
  FAILED=1
  echo "system-smoke: MISMATCH: $*" >&2
}

# Puts the saved state back without the helper. Only reached when the helper's own removal was
# not verified. Every step is tried, whatever the one before it did.
# shellcheck disable=SC2329  # called from `finish`, which the EXIT trap calls
fallback_restore() {
  say "restoring the saved state without the helper"
  if [ -f "$WORK/hosts.before" ]; then
    if ! cmp -s "$WORK/hosts.before" "$HOSTS"; then
      # In place: a bind-mounted hosts file (containers) cannot be renamed over.
      if cat "$WORK/hosts.before" > "$HOSTS" && cmp -s "$WORK/hosts.before" "$HOSTS"; then
        say "  $HOSTS: the saved bytes are back"
      else
        echo "system-smoke: COULD NOT RESTORE $HOSTS. The saved copy is $WORK/hosts.before" >&2
      fi
    fi
    # The file may have been replaced by one with another owner or mode; the saved copy
    # (`cp -p`) carries the ones it had.
    python3 "$CHECK" restore-meta --saved "$WORK/hosts.before" --hosts "$HOSTS" \
      || echo "system-smoke: COULD NOT RESTORE the owner and mode of $HOSTS. Compare with $WORK/hosts.before" >&2
  fi
  if [ -f "$WORK/dns.before.json" ]; then
    python3 "$CHECK" restore-dns --os "$OS" --saved "$WORK/dns.before.json" --dropin "$DROPIN" \
      || echo "system-smoke: COULD NOT RESTORE DNS. The saved state is $WORK/dns.before.json" >&2
  fi
  # The state directory did not exist before the run (checked above), and its path is a constant.
  if [ -e "$STATE_DIR" ] || [ -L "$STATE_DIR" ]; then
    rm -rf -- "$STATE_DIR" && say "  $STATE_DIR: removed"
  fi
  if [ "$OS" = "macos" ]; then
    /usr/bin/dscacheutil -flushcache > /dev/null 2>&1
    /usr/bin/killall -HUP mDNSResponder > /dev/null 2>&1
  else
    /usr/bin/resolvectl flush-caches > /dev/null 2>&1
  fi
  return 0
}

# shellcheck disable=SC2329  # the EXIT trap
finish() {
  code=$?
  trap - EXIT
  # What follows may be the only way back: a second interrupt must not cut it short.
  trap '' INT TERM HUP
  if [ "$CHANGED" -eq 1 ] && [ "$RESTORED" -eq 0 ]; then
    FAILED=1
    fallback_restore
  fi
  if [ "$FAILED" -eq 0 ] && [ "$code" -eq 0 ]; then
    rm -rf -- "$WORK"
    exit 0
  fi
  echo "system-smoke: RED. What the run saved and saw is in $WORK" >&2
  exit 1
}
trap finish EXIT
trap 'exit 1' INT TERM HUP

# The checks are one Python program: file comparisons, the hash of the block, the JSON that
# `status` prints, lookups through the operating system and plain DNS questions to the resolver.
cat > "$CHECK" << 'PYTHON_CHECKS'
"""Checks of the ubiqx-shield system smoke run. Written to a work directory by system-smoke.sh."""

import argparse
import base64
import hashlib
import ipaddress
import json
import os
import random
import re
import socket
import stat
import struct
import subprocess
import sys
import time

# What the design says the helper writes (docs/nofap/DESIGN.md 2.2, 2.3, 2.5). Stated here
# again, not read from the helper.
CANARY = "canary.ubiqx-shield.invalid"
BEGIN_PREFIX = "# >>> ubiqx-shield v1 BEGIN"
END_MARKER = "# <<< ubiqx-shield END"
UNDO_HINT = "# Added by the ubiqx-shield app on this computer. To undo, delete every line from BEGIN to END."
BEGIN_LINE = re.compile(r"^# >>> ubiqx-shield v1 BEGIN sha256=([0-9a-f]{64}) names=([0-9]+)$")
NAMES_PER_LINE = 6
CAP_UNIX = 20000
NULL_ADDRESSES = {"0.0.0.0", "::"}
OWNER = "shield"
RESOLVER_ID = "cloudflare_family"
RESOLVER_V4 = ["1.1.1.3", "1.0.0.3"]
RESOLVER_V6 = ["2606:4700:4700::1113", "2606:4700:4700::1003"]
DROPIN_TEXT = (
    "# ubiqx-shield: delete this file and restart systemd-resolved to undo\n"
    "[Resolve]\n"
    "DNS=1.1.1.3 1.0.0.3 2606:4700:4700::1113 2606:4700:4700::1003\n"
    "Domains=~.\n"
)
# A name that has to keep resolving, and the name the resolver's operator publishes for testing
# its adult-content filter (answered with the null address by the family resolver only).
REACHABLE = "example.com"
FILTER_TEST_NAME = "nudity.testcategory.com"
TOOL_ENV = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LANG": "C"}
NETWORKSETUP = "/usr/sbin/networksetup"
RESOLVECTL = "/usr/bin/resolvectl"
SYSTEMCTL = "/usr/bin/systemctl"
RESOLV_CONF = "/etc/resolv.conf"


class Report:
    """Collects what was checked; one line per check, and the exit code at the end."""

    def __init__(self):
        self.failures = 0

    def ok(self, what):
        print("  ok    " + what)

    def note(self, what):
        print("  note  " + what)

    def fail(self, what):
        self.failures += 1
        print("  FAIL  " + what)

    def check(self, passed, what, found=None):
        if passed:
            self.ok(what)
        else:
            self.fail(what if found is None else "%s (found: %s)" % (what, found))
        return passed

    def exit_code(self):
        return 1 if self.failures else 0


def read_bytes(path):
    with open(path, "rb") as handle:
        return handle.read()


def read_json(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def dig(value, *keys):
    """value[k1][k2]... or None when a step is missing."""
    for key in keys:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


# ----------------------------------------------------------------------------------------------
# The hosts block


class BlockError(Exception):
    pass


def parse_block(data):
    """The one block of `data` (bytes of a hosts file): what it declares and what it holds."""
    lines = data.split(b"\n")
    begins = [i for i, line in enumerate(lines) if line.startswith(BEGIN_PREFIX.encode())]
    ends = [i for i, line in enumerate(lines) if line.rstrip(b"\r") == END_MARKER.encode()]
    if len(begins) != 1 or len(ends) != 1:
        raise BlockError("expected one BEGIN and one END line, found %d and %d" % (len(begins), len(ends)))
    begin, end = begins[0], ends[0]
    if end <= begin:
        raise BlockError("the END line is not after the BEGIN line")
    raw = lines[begin : end + 1]
    # Every line of the block but the last is followed by "\n"; the END line is too unless
    # the file stops there. The block is written with one line ending throughout.
    ended = raw[:-1] + (raw[-1:] if end + 1 < len(lines) else [])
    with_cr = sum(1 for line in ended if line.endswith(b"\r"))
    if with_cr not in (0, len(ended)):
        raise BlockError("the block mixes LF and CRLF line endings (%d of %d lines end with CRLF)" % (with_cr, len(ended)))
    try:
        text = [(line[:-1] if line.endswith(b"\r") else line).decode("ascii") for line in raw]
    except UnicodeDecodeError:
        raise BlockError("the block holds bytes that are not ASCII")
    declared = BEGIN_LINE.match(text[0])
    if not declared:
        raise BlockError("the BEGIN line is not `%s sha256=<64 hex> names=<n>`: %r" % (BEGIN_PREFIX, text[0]))
    body = text[1:-1]
    return {
        "sha256": declared.group(1),
        "names": int(declared.group(2)),
        "body": body,
        "eol": b"\r\n" if with_cr else b"\n",
        # The rule of the design: SHA-256 over the lines between the markers, joined with "\n".
        "body_sha256": hashlib.sha256("\n".join(body).encode("ascii")).hexdigest(),
    }


def block_names(body):
    """The names of a block body, in order. Raises when a line is not one the design describes."""
    if not body or body[0] != UNDO_HINT:
        raise BlockError("the first line of the block is not the undo hint")
    rows = body[1:]
    if len(rows) % 2:
        raise BlockError("the block does not hold one `::` line per `0.0.0.0` line")
    names = []
    for index in range(0, len(rows), 2):
        v4, v6 = rows[index].split(" "), rows[index + 1].split(" ")
        if v4[0] != "0.0.0.0" or v6[0] != "::":
            raise BlockError("line %d of the block is not a `0.0.0.0` line followed by a `::` line" % (index + 2))
        if v4[1:] != v6[1:]:
            raise BlockError("the `::` line %d does not carry the names of its `0.0.0.0` line" % (index + 3))
        group = v4[1:]
        if not 1 <= len(group) <= NAMES_PER_LINE or "" in group:
            raise BlockError("line %d holds %d names (1 to %d expected)" % (index + 2, len(group), NAMES_PER_LINE))
        names.extend(group)
    if len(set(names)) != len(names):
        raise BlockError("a name is written twice")
    return names


def file_eol(saved):
    """The line ending a block added to `saved` has to use: the file's own. CRLF when most of
    its lines end so, and when it stops at a lone CR (a "\\n" right after that CR would join it
    into one line ending and change what the last line says)."""
    crlf = saved.count(b"\r\n")
    lf = saved.count(b"\n") - crlf
    return b"\r\n" if crlf > lf or saved.endswith(b"\r") else b"\n"


def has_marker(data):
    return b">>> ubiqx-shield" in data or b"<<< ubiqx-shield" in data


# ----------------------------------------------------------------------------------------------
# Lookups


def resolve(name):
    """Addresses `name` resolves to through the operating system; empty when it does not."""
    try:
        infos = socket.getaddrinfo(name, None)
    except (socket.gaierror, UnicodeError, OSError):
        return set()
    found = set()
    for info in infos:
        try:
            found.add(ipaddress.ip_address(info[4][0].split("%")[0]).compressed)
        except ValueError:
            pass
    return found


def eventually(look, wanted, seconds):
    """Calls `look` until `wanted(result)` holds or `seconds` passed; the last result."""
    deadline = time.monotonic() + seconds
    while True:
        result = look()
        if wanted(result) or time.monotonic() >= deadline:
            return result
        time.sleep(0.5)


def is_blocked(addresses):
    return bool(addresses) and addresses <= NULL_ADDRESSES


def is_reachable(addresses):
    return bool(addresses - NULL_ADDRESSES)


def _skip_name(packet, offset):
    while True:
        length = packet[offset]
        if length == 0:
            return offset + 1
        if length & 0xC0 == 0xC0:
            return offset + 2
        offset += 1 + length


def dns_a(server, name, timeout=3.0, tries=3):
    """The A records `server` gives for `name`, asked over UDP; None when it did not answer."""
    question = b"".join(bytes([len(label)]) + label.encode("ascii") for label in name.rstrip(".").split("."))
    for _ in range(tries):
        ident = random.randrange(65536)
        packet = struct.pack(">HHHHHH", ident, 0x0100, 1, 0, 0, 0) + question + b"\x00" + struct.pack(">HH", 1, 1)
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.settimeout(timeout)
        try:
            sock.sendto(packet, (server, 53))
            answer, _ = sock.recvfrom(4096)
        except OSError:
            continue
        finally:
            sock.close()
        try:
            got, flags, questions, answers = struct.unpack(">HHHH", answer[:8])
            if got != ident or not flags & 0x8000:
                continue
            offset = 12
            for _ in range(questions):
                offset = _skip_name(answer, offset) + 4
            records = set()
            for _ in range(answers):
                offset = _skip_name(answer, offset)
                kind, _, _, length = struct.unpack(">HHIH", answer[offset : offset + 10])
                offset += 10
                if kind == 1 and length == 4:
                    records.add(socket.inet_ntoa(answer[offset : offset + 4]))
                offset += length
            return records
        except (struct.error, IndexError):
            continue
    return None


# ----------------------------------------------------------------------------------------------
# The DNS configuration, as a value that can be saved, compared and put back


def run_tool(argv):
    try:
        done = subprocess.run(argv, env=TOOL_ENV, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return None
    if done.returncode != 0:
        return None
    return done.stdout.decode("utf-8", "replace")


def is_address(text):
    try:
        ipaddress.ip_address(text.split("%")[0])
        return True
    except ValueError:
        return False


def mac_services():
    listed = run_tool([NETWORKSETUP, "-listallnetworkservices"])
    if listed is None:
        return None
    # The first line is a note about the asterisk; a name that starts with one is disabled.
    return [line for line in listed.splitlines()[1:] if line.strip() and not line.startswith("*")]


def mac_servers(service):
    # A line that parses as an address is a server; no such line means "automatic".
    printed = run_tool([NETWORKSETUP, "-getdnsservers", service])
    if printed is None:
        return None
    return [line.strip() for line in printed.splitlines() if is_address(line.strip())]


def resolved_global():
    printed = run_tool([RESOLVECTL, "dns"])
    if printed is None:
        return None
    for line in printed.splitlines():
        if line.startswith("Global:"):
            return line[len("Global:") :].split()
    return []


def dns_state(target, dropin):
    if target == "macos":
        services = mac_services()
        if services is None:
            return {"services": None}
        return {"services": {service: mac_servers(service) for service in services}}
    try:
        resolv_conf = read_bytes(RESOLV_CONF).decode("utf-8", "replace")
    except OSError:
        resolv_conf = None
    return {
        "dropin": os.path.lexists(dropin),
        "dropin_dir": os.path.isdir(os.path.dirname(dropin)),
        "resolv_conf": resolv_conf,
        "resolved_global": resolved_global(),
        "resolved_active": (run_tool([SYSTEMCTL, "is-active", "systemd-resolved"]) or "").strip() == "active",
    }


def same_dns(now, was):
    """Whether two values of `dns_state` are the same configuration. The directory of the
    drop-in is not part of it: the design has the helper delete its file, not the directory
    it may have had to create."""

    def without_dir(state):
        return {key: value for key, value in state.items() if key != "dropin_dir"}

    return without_dir(now) == without_dir(was)


def dns_settles(target, dropin, was, seconds=15):
    """`dns_state` once it equals `was`, or what it is after `seconds`: systemd-resolved
    needs a moment after a restart before it answers `resolvectl`."""
    return eventually(lambda: dns_state(target, dropin), lambda now: same_dns(now, was), seconds)


def same_addresses(servers, wanted):
    """Whether `servers` is exactly the address set `wanted`: nothing missing, nothing more."""
    if not isinstance(servers, list) or len(servers) != len(wanted):
        return False
    try:
        found = {ipaddress.ip_address(str(server).split("%")[0]) for server in servers}
    except ValueError:
        return False
    return found == {ipaddress.ip_address(address) for address in wanted}


# ----------------------------------------------------------------------------------------------
# Sub-commands


def cmd_plan(args):
    tiers = args.tiers.split(",")
    plan = {
        "v": 1,
        "owner": OWNER,
        "hosts": "hosts" in tiers,
        "extra": [],
        "resolver": RESOLVER_ID if "resolver" in tiers else None,
        "policies": False,
    }
    text = json.dumps(plan, separators=(",", ":")).encode("ascii")
    print(base64.urlsafe_b64encode(text).decode("ascii").rstrip("="))
    return 0


def cmd_dns_state(args):
    json.dump(dns_state(args.os, args.dropin), sys.stdout, indent=2, sort_keys=True)
    print()
    return 0


def cmd_baseline(args):
    json.dump({"resolves": is_reachable(resolve(REACHABLE))}, sys.stdout)
    print()
    return 0


def cmd_restore_dns(args):
    saved = read_json(args.saved)
    if args.os == "macos":
        for service, servers in (saved.get("services") or {}).items():
            if servers is None or mac_servers(service) == servers:
                continue
            run_tool([NETWORKSETUP, "-setdnsservers", service] + (servers or ["empty"]))
            print("system-smoke:   DNS of %r: put back" % service)
    else:
        if not saved.get("dropin") and os.path.lexists(args.dropin):
            os.remove(args.dropin)
            run_tool([SYSTEMCTL, "restart", "systemd-resolved"])
            print("system-smoke:   %s: removed, systemd-resolved restarted" % args.dropin)
        # The directory, when it was not there before the run and holds nothing now.
        folder = os.path.dirname(args.dropin)
        if saved.get("dropin_dir") is False and os.path.isdir(folder) and not os.path.islink(folder):
            try:
                os.rmdir(folder)
                print("system-smoke:   %s: removed (it did not exist before the run)" % folder)
            except OSError:
                pass
    return 0 if same_dns(dns_settles(args.os, args.dropin, saved), saved) else 1


def cmd_restore_meta(args):
    """Owner, group and mode of the hosts file as the saved copy has them."""
    before, now = os.stat(args.saved), os.stat(args.hosts)
    if (before.st_uid, before.st_gid) != (now.st_uid, now.st_gid):
        os.chown(args.hosts, before.st_uid, before.st_gid)
        print("system-smoke:   %s: owner %d:%d is back" % (args.hosts, before.st_uid, before.st_gid))
    if stat.S_IMODE(before.st_mode) != stat.S_IMODE(now.st_mode):
        os.chmod(args.hosts, stat.S_IMODE(before.st_mode))
        print("system-smoke:   %s: mode %o is back" % (args.hosts, stat.S_IMODE(before.st_mode)))
    return 0


def reachable_still(report, baseline, when):
    if not baseline.get("resolves"):
        report.note("%s did not resolve on this machine before the run, so it is not asked for %s" % (REACHABLE, when))
        return
    found = eventually(lambda: resolve(REACHABLE), is_reachable, 15)
    report.check(is_reachable(found), "%s still resolves %s" % (REACHABLE, when), sorted(found))


def cmd_applied(args):
    report = Report()
    tiers = args.tiers.split(",")
    status = read_json(args.status)
    baseline = read_json(os.path.join(args.work, "baseline.json"))
    facts = {"listed": None, "filter_test": False}
    report.check(dig(status, "state_error") is None, "status: the state file is readable", dig(status, "state_error"))
    failed = [r for r in dig(status, "state", "report") or [] if not r.get("ok")]
    report.check(not failed, "state: no tier is reported as failed", failed)

    info = os.lstat(args.state_dir) if os.path.lexists(args.state_dir) else None
    report.check(info is not None and stat.S_ISDIR(info.st_mode), "the state directory exists and is a directory, not a link")
    if info is not None:
        report.check(info.st_uid == 0 and not info.st_mode & 0o022, "the state directory is root's and nobody else can write to it", "uid %d mode %o" % (info.st_uid, stat.S_IMODE(info.st_mode)))
        state_file = os.path.join(args.state_dir, "state.json")
        readable = os.path.isfile(state_file) and os.stat(state_file).st_mode & 0o004
        report.check(bool(readable), "the state file is readable by every account")

    if "hosts" in tiers:
        saved, current = read_bytes(args.saved), read_bytes(args.hosts)
        report.check(current.startswith(saved), "hosts: every byte of the saved file is still in front, unchanged")
        try:
            block = parse_block(current)
            names = block_names(block["body"])
        except BlockError as error:
            report.fail("hosts: %s" % error)
            block, names = None, []
        if block is not None:
            eol = file_eol(saved)
            report.check(block["eol"] == eol, "hosts: the block uses the line ending of the file (%s)" % ("CRLF" if eol == b"\r\n" else "LF"), "CRLF" if block["eol"] == b"\r\n" else "LF")
            # A saved file that does not end with a line break gets one in front of the block.
            ended = not saved or saved.endswith(b"\n")
            lead = b"" if ended else eol
            report.check(current[len(saved) :].startswith(lead + BEGIN_PREFIX.encode()), "hosts: the block starts right after the saved bytes, on a line of its own")
            # The file keeps its own ending: a line break after END exactly when the saved file had one.
            tail = END_MARKER.encode() + (eol if ended else b"")
            report.check(current.endswith(tail), "hosts: the END line closes the file, ended as the saved file was")
            report.note("hosts: the block declares sha256=%s names=%d" % (block["sha256"], block["names"]))
            report.check(block["body_sha256"] == block["sha256"], "hosts: the declared hash is the hash of the lines between the markers", block["body_sha256"])
            report.check(block["names"] == len(names), "hosts: the declared count is the number of names (%d)" % block["names"], len(names))
            report.check(2 <= len(names) <= CAP_UNIX, "hosts: between 2 and %d names" % CAP_UNIX, len(names))
            report.check(bool(names) and names[0] == CANARY, "hosts: the canary is the first name")
            report.check(dig(status, "state", "hosts", "sha256") == block["sha256"] and dig(status, "state", "hosts", "names") == block["names"], "state: records the hash and the count of the block", dig(status, "state", "hosts"))
        text = dig(status, "hosts", "text") or ""
        report.check(BEGIN_PREFIX in text and END_MARKER in text, "status: the unprivileged read sees the block")
        report.check(dig(status, "hosts", "error") is None, "status: no hosts error", dig(status, "hosts", "error"))
        report.check(dig(status, "state", "owners", OWNER, "hosts") is True, "state: the owner %r wants the hosts tier" % OWNER)
        # What the app's own check of the hosts tier reads: the helper asks for the canary
        # the way the app does and prints the answer.
        canary = dig(status, "canary")
        report.check(isinstance(canary, list) and bool(canary) and set(canary) <= NULL_ADDRESSES, "status: the canary reads as the null address, as the check in the app needs it", canary)

        found = eventually(lambda: resolve(CANARY), is_blocked, 10)
        report.check(is_blocked(found), "the canary resolves to the null address through the operating system", sorted(found))
        # A name of the list itself, taken from the file: the first one after the canary that
        # has two labels.
        listed = next((name for name in names[1:] if name.count(".") == 1), None)
        if listed is None and block is not None:
            report.fail("hosts: the block holds no name of the list after the canary")
        elif listed is not None:
            facts["listed"] = listed
            found = eventually(lambda: resolve(listed), is_blocked, 10)
            report.check(is_blocked(found), "a name of the list (%s) resolves to the null address" % listed, sorted(found))

    if "resolver" in tiers:
        report.check(dig(status, "state", "owners", OWNER, "resolver") == RESOLVER_ID, "state: the owner %r wants the resolver %s" % (OWNER, RESOLVER_ID), dig(status, "state", "owners", OWNER, "resolver"))
        wanted = RESOLVER_V4 + RESOLVER_V6
        now = dns_state(args.os, args.dropin)
        if args.os == "macos":
            services = now.get("services") or {}
            report.check(bool(services), "DNS: at least one enabled network service")
            for service, servers in sorted(services.items()):
                report.check(same_addresses(servers, wanted), "DNS: %r uses exactly the resolver's four addresses" % service, servers)
            active = [i for i in dig(status, "resolver", "interfaces") or [] if i.get("active")]
            report.check(bool(active) and all(same_addresses(i.get("servers"), wanted) for i in active), "status: every active interface reads as the resolver's set", [(i.get("name"), i.get("servers")) for i in active])
        else:
            try:
                dropin = read_bytes(args.dropin).decode("utf-8", "replace")
            except OSError:
                dropin = None
            report.check(dropin == DROPIN_TEXT, "DNS: the drop-in holds exactly the expected four lines", dropin)
            report.check(dig(status, "resolver", "dropin") == DROPIN_TEXT, "status: reads the same drop-in")
            report.check(dig(status, "resolver", "resolved_active") is True and dig(status, "resolver", "resolv_conf_uses_resolved") is True, "status: systemd-resolved is active and /etc/resolv.conf points at it")
            report.check(same_addresses(now.get("resolved_global"), wanted), "DNS: systemd-resolved's global servers are the resolver's four addresses", now.get("resolved_global"))
        for server in RESOLVER_V4:
            answer = dns_a(server, REACHABLE)
            report.check(answer is not None and is_reachable(answer), "the resolver %s answers for %s" % (server, REACHABLE), answer)
            if facts["listed"]:
                answer = dns_a(server, facts["listed"])
                report.check(answer is not None and is_blocked(answer), "the resolver %s refuses a listed name (%s) with the null address" % (server, facts["listed"]), answer)
        if not facts["listed"]:
            report.note("without the hosts tier in this run, no listed name was asked of the resolver")
        # Does the operating system really ask this resolver? Its operator's test name is
        # answered with the null address by the family resolver and by nobody else.
        direct = dns_a(RESOLVER_V4[0], FILTER_TEST_NAME)
        if direct is not None and is_blocked(direct):
            facts["filter_test"] = True
            found = eventually(lambda: resolve(FILTER_TEST_NAME), is_blocked, 15)
            report.check(is_blocked(found), "the operating system resolves through the family resolver (%s is refused)" % FILTER_TEST_NAME, sorted(found))
        else:
            report.note("the resolver did not refuse its operator's test name %s (answer: %s), so whether the operating system asks it is shown by the configuration only" % (FILTER_TEST_NAME, direct))

    reachable_still(report, baseline, "with the tiers applied")
    with open(os.path.join(args.work, "facts.json"), "w", encoding="utf-8") as handle:
        json.dump(facts, handle)
    return report.exit_code()


def cmd_removed(args):
    report = Report()
    tiers = args.tiers.split(",")
    status = read_json(args.status)
    baseline = read_json(os.path.join(args.work, "baseline.json"))
    try:
        facts = read_json(os.path.join(args.work, "facts.json"))
    except (OSError, ValueError):
        facts = {}
    saved, current = read_bytes(args.saved), read_bytes(args.hosts)
    report.check(current == saved, "hosts: byte for byte the saved file (%d bytes)" % len(saved), "%d bytes" % len(current))
    before, after = os.stat(args.saved), os.stat(args.hosts)
    report.check((before.st_uid, before.st_gid, stat.S_IMODE(before.st_mode)) == (after.st_uid, after.st_gid, stat.S_IMODE(after.st_mode)), "hosts: owner, group and mode are what they were", "%d:%d %o" % (after.st_uid, after.st_gid, stat.S_IMODE(after.st_mode)))
    report.check(not os.path.lexists(args.state_dir), "the state directory is gone")
    report.check(dig(status, "state") is None and dig(status, "state_error") is None, "status: no state file", dig(status, "state"))
    report.check(not has_marker((dig(status, "hosts", "text") or "").encode("utf-8", "replace")), "status: the unprivileged read sees no block")
    report.check(dig(status, "canary") is None, "status: the canary is not asked for without the block", dig(status, "canary"))
    was = read_json(os.path.join(args.work, "dns.before.json"))
    now = dns_settles(args.os, args.dropin, was)
    report.check(same_dns(now, was), "DNS: the configuration is what it was before the run", now)
    if was.get("dropin_dir") is False and now.get("dropin_dir"):
        report.note("%s did not exist before the run and stays, empty (the helper deletes its file, not the directory)" % os.path.dirname(args.dropin))

    if "hosts" in tiers:
        found = eventually(lambda: resolve(CANARY), lambda addresses: not addresses, 10)
        report.check(not found, "the canary no longer resolves", sorted(found))
        if facts.get("listed"):
            found = eventually(lambda: resolve(facts["listed"]), lambda addresses: not is_blocked(addresses), 10)
            report.check(not is_blocked(found), "the listed name (%s) is no longer pinned to the null address" % facts["listed"], sorted(found))
    if "resolver" in tiers and facts.get("filter_test"):
        found = eventually(lambda: resolve(FILTER_TEST_NAME), lambda addresses: not is_blocked(addresses), 15)
        report.check(not is_blocked(found), "the operating system no longer resolves through the family resolver", sorted(found))
    reachable_still(report, baseline, "after the removal")
    return report.exit_code()


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    for name, run in [("plan", cmd_plan), ("dns-state", cmd_dns_state), ("baseline", cmd_baseline), ("restore-dns", cmd_restore_dns), ("restore-meta", cmd_restore_meta), ("applied", cmd_applied), ("removed", cmd_removed)]:
        sub = commands.add_parser(name)
        sub.set_defaults(run=run)
        sub.add_argument("--os", choices=["linux", "macos"])
        sub.add_argument("--tiers")
        sub.add_argument("--hosts")
        sub.add_argument("--saved")
        sub.add_argument("--status")
        sub.add_argument("--state-dir", dest="state_dir")
        sub.add_argument("--dropin")
        sub.add_argument("--work")
    args = parser.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
PYTHON_CHECKS

check() {
  local phase="$1"
  python3 "$CHECK" "$phase" --os "$OS" --tiers "$TIERS" --hosts "$HOSTS" --saved "$WORK/hosts.before" \
    --status "$WORK/status.$phase.json" --state-dir "$STATE_DIR" --dropin "$DROPIN" --work "$WORK"
}

# ---------------------------------------------------------------------------------------------
# 1. Save.

say "os=$OS tiers=$TIERS helper=$HELPER ($HELPER_FORM form)"
cp -p "$HOSTS" "$WORK/hosts.before" || exit 1
python3 "$CHECK" dns-state --os "$OS" --dropin "$DROPIN" > "$WORK/dns.before.json" || exit 1
python3 "$CHECK" baseline > "$WORK/baseline.json" || exit 1
helper status > "$WORK/status.before.json" || exit 1
PLAN="$(python3 "$CHECK" plan --tiers "$TIERS")" || exit 1
[ -n "$PLAN" ] || exit 1
say "saved: $HOSTS ($(wc -c < "$WORK/hosts.before" | tr -d ' ') bytes), the DNS configuration and the status"

# ---------------------------------------------------------------------------------------------
# 2. Apply.

CHANGED=1
say "apply"
applied=0
helper apply --plan-b64 "$PLAN" --unverified "$TIERS" || applied=$?

# ---------------------------------------------------------------------------------------------
# 3. Check what was applied.

helper status > "$WORK/status.applied.json" || mismatch "status failed after apply"
if [ "$applied" -ne 0 ]; then
  # 2 invalid plan, 3 partial (the state file holds a report per tier), 4 failed, 5 tier gated.
  mismatch "apply exited $applied (0 expected); what status printed afterwards is in $WORK/status.applied.json"
else
  say "checking what was applied"
  check applied || mismatch "the applied state is not what the design describes"
fi

# ---------------------------------------------------------------------------------------------
# 4. Remove. Done whatever step 3 found, so the helper's own way back is always exercised.

say "remove --owner all --scope all"
removed=0
helper remove --owner all --scope all || removed=$?
[ "$removed" -eq 0 ] || mismatch "remove exited $removed (0 expected)"

# ---------------------------------------------------------------------------------------------
# 5. Check the restore.

helper status > "$WORK/status.removed.json" || mismatch "status failed after remove"
say "checking the restore"
if check removed; then
  RESTORED=1
else
  mismatch "the helper's removal did not put the machine back as it was"
fi

if [ "$FAILED" -ne 0 ]; then
  exit 1
fi

HELPER_SHA="$(python3 -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "$HELPER")"
COMMIT="$(git -C "$ROOT" rev-parse --short=7 HEAD 2> /dev/null || echo unknown)"
say "GREEN os=$OS tiers=$TIERS commit=$COMMIT helper_sha256=$HELPER_SHA"
exit 0
