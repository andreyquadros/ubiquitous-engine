#!/usr/bin/env python3
"""Build the adult-content blocklist data that `crates/ubiqx-nofap` embeds.

Run by a maintainer before a release, never by `cargo build` and never by CI
(CI runs only `--check`, which needs no network). Python 3.11 standard library
only. The outputs are committed, so builds need no network and are reproducible.

    python3 scripts/nofap/build_blocklist.py             build, run every gate, write
    python3 scripts/nofap/build_blocklist.py --check     verify the committed outputs
    python3 scripts/nofap/build_blocklist.py --selftest  unit tests of this script
    python3 scripts/nofap/build_blocklist.py --pin       re-pin every source at its
                                                         upstream HEAD (network)

Design: docs/nofap/DESIGN.md section 3. Research: docs/nofap/research/research-blocklists.md.

===========================================================================
CONTRACT WITH THE RUST MATCHER (`ubiqx_nofap::list`, `ubiqx_nofap::matcher`)
===========================================================================

All files live in `crates/ubiqx-nofap/data/`.

1. Hash tables: `core.bin`, `popular.bin`

   offset  size  field
   0       8     magic         the bytes "UBXBL1" followed by two NUL bytes
   8       2     format        u16, = 1
   10      2     flags         u16, = 0 (reserved; a reader rejects any other value)
   12      4     count         u32, number of keys
   16      8     list_version  u64, UTC timestamp written as the decimal number
                               YYYYMMDDHHMM (for example 202611191200)
   24      8*n   keys          `count` u64 values, strictly ascending (sorted, unique)

   Every integer is big-endian. File length is exactly 24 + 8 * count.

   Key of a name:
       key = first 8 bytes of SHA-256(tag || name), read as a big-endian u64
   where `tag` is the ASCII tag INCLUDING its trailing NUL byte:
       core.bin     b"ubiqx-nofap-blocklist-v1\\0"   (25 bytes)
       popular.bin  b"ubiqx-nofap-popular-v1\\0"     (23 bytes)
   and `name` is the normalised host name in ASCII (see 2).
   `core.meta.json` field `vector` holds test vectors for both tags.

2. Normalised name

   Lower-case ASCII; IDN labels in punycode (`xn--...`); labels joined by single
   dots; no trailing dot, no port, no user-info, no brackets. A leading `www.` is
   removed when at least two labels remain (`www.example.com` -> `example.com`).

3. Semantics of `core.bin`

   Every entry covers itself and all of its subdomains. A host is covered when
   the key of the host, or of the host with its left-most labels removed one at a
   time DOWN TO TWO LABELS, is in the table (`a.b.example.co.uk` tests
   `a.b.example.co.uk`, `b.example.co.uk`, `example.co.uk`, `co.uk`). A single
   label is never tested. The table holds no public suffix (ICANN or private
   section of the Public Suffix List), so the walk needs no suffix list. Entries
   are suffix-deduplicated: no entry has a listed ancestor. At build time the
   leading labels `www.`, `m.` and `mobile.` were removed from listed names
   (while a registrable name remains), so those hosts match through the walk.
   An entry is never collapsed to its registrable domain: a name listed as
   `adult.contents.fc2.com` stays at that depth.

4. Semantics of `popular.bin`

   The normalised host (rule 2) of every Chrome UX Report origin with rank
   bucket <= 100,000, except hosts covered by `core.bin`, hosts under a rule of
   `mixed.txt`, and public suffixes. Intended lookup: the EXACT normalised host,
   no parent walk. A subdomain of a popular site is not known to be popular
   (user pages on hosting platforms), so it must not inherit the class.

5. `hosts-tier.txt`

   One host name per line, LF line ends, no comments, no blank lines, most
   visited first (so the first N lines are the best N). These are exact names
   for the hosts file: every Chrome UX Report origin with rank bucket <= 500,000
   that `core.bin` covers, each followed by its registrable domain and
   `www.` + registrable domain when the registrable domain itself is listed.
   Lower rank buckets come first; inside a bucket the order is a hash of the
   name (the report gives no finer rank). At most 20,000 lines. No line is under a rule of `never-block.txt`,
   `mixed.txt` or `never-capture.txt`. Every line has 4 to 253 bytes, at least
   two labels, only `a-z 0-9 - .`.

6. Rule files: `never-block.txt`, `mixed.txt`, `never-capture.txt`

   UTF-8 text, LF. A line that is empty or starts with `#` is ignored (there are
   no end-of-line comments). Every other line is one rule, lower-case ASCII, in
   one of three forms:

       example.com   NAME: two or more labels, no `*`. Covers the name and
                     every subdomain.
       gov           TLD: one label. Covers every name whose last label is it.
       gov.*         SECOND-LEVEL FORM: one label followed by `.*`. Covers every
                     name that has that label immediately before its last
                     label: `gov.br`, `x.gov.br`, `a.b.gov.uk`, `x.gov.wales`.
                     It does not cover `gov` as a top-level domain (that is the
                     TLD form) and not `xgov.br` or `gov.example.br`.
                     This is the reading of `ubiqx_nofap::matcher::RuleSet`.

   A TLD or SECOND-LEVEL rule decides at every depth: nothing under it is ever
   blocked by the list or by a keyword. For NAME rules the matcher's longest
   match applies (design 3.4): `fc2.com` on never-block does not release a
   listed `adult.contents.fc2.com`. The build follows the same reading: it
   removes a listed name that EQUALS a NAME rule, and every listed name under a
   TLD or SECOND-LEVEL rule, and it fails when a listed name is an ancestor of a
   NAME rule. Names removed, and listed names kept below a NAME rule, are printed in
   the review file.

7. `canary-block.txt`, `canary-allow.txt`, `extra-ptbr.txt`

   One host name per line; `#` lines and empty lines ignored. Every
   canary-block name must be covered by `core.bin`; no canary-allow name may be.

===========================================================================
PIPELINE (design 3.2)
===========================================================================

fetch at the pinned commits, verify SHA-256 -> parse (drop comments and the
address column; lower-case; strip trailing dots; drop IP literals and invalid
names: a label is `a-z 0-9 -` only, so the few upstream names with `_` go) ->
strip `www.`, `m.`, `mobile.` while a registrable name remains -> drop public suffixes
(ICANN and private) -> drop `*.tumblr.com` and `*.blogspot.*` ->
4skinSkywalker: keep only names with a block-grade keyword -> drop names that a
rule file protects and the names in UPSTREAM_MISTAKES -> never collapse to the
registrable domain -> add `extra-ptbr.txt` -> suffix-deduplicate -> gates ->
write.

`list_version` is read from `sources.lock.json`, never from the clock, so the
same inputs give the same bytes. `--pin` sets it; raise it by hand when only a
hand-maintained file changed.

Gates (any failure writes nothing): canaries; no public suffix, single label or
IP literal; count within -10% / +25% of the previous `core.meta.json`; absolute
bounds; no listed name covers a Chrome UX Report top-1,000 origin unless it is
in REVIEWED_TOP_1K below; no listed name at or above a NAME rule; the
false-positive traps of design 3.6 looked up in the built table; the upstream
lists inside StevenBlack's file and their licences are the ones the notices
name; THIRD-PARTY-NOTICES carries every copyright line; a second run in a
fresh interpreter with another hash seed gives the same bytes.

The build also writes docs/nofap/research/blocklist-top10k-review.txt, the list
a person reads before a release.
"""

from __future__ import annotations

import argparse
import bisect
import csv
import gzip
import hashlib
import io
import json
import os
import re
import ssl
import struct
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "crates" / "ubiqx-nofap" / "data"
CACHE = Path(__file__).resolve().parent / ".cache"
REVIEW = ROOT / "docs" / "nofap" / "research" / "blocklist-top10k-review.txt"
THIRD_PARTY = ROOT / "THIRD-PARTY-NOTICES"

MAGIC = b"UBXBL1\x00\x00"
FORMAT = 1
FLAGS = 0
HEADER = struct.Struct(">8sHHIQ")
TAG_CORE = b"ubiqx-nofap-blocklist-v1\x00"
TAG_POPULAR = b"ubiqx-nofap-popular-v1\x00"

# Bounds of design section 13 (U1).
CORE_MIN, CORE_MAX = 215_000, 300_000
CORE_MAX_BYTES = 2_400_000
POPULAR_MAX_BYTES = 1_000_000
HOSTS_TIER_CAP = 20_000
HOSTS_TIER_MAX_RANK = 500_000
POPULAR_MAX_RANK = 100_000
REVIEW_MAX_RANK = 10_000
GATE_MAX_RANK = 1_000

# Listed names that cover a Chrome UX Report top-1,000 origin and were judged
# to be dedicated adult sites, not general platforms. A new name that reaches
# the top 1,000 fails the build until someone adds it here, or moves it to
# mixed.txt, never-block.txt or UPSTREAM_MISTAKES when it does not belong on
# the list. (The design's wording of this gate, "no entry at or above the
# registrable domain of a top-1,000 origin", cannot be met as written: about
# 220 of the 1,000 most visited origins are adult sites.)
#
# First reading, 2026-11-19: by the engineer who wrote this script, from the
# names and from general knowledge; the build sandbox cannot open the sites.
# Moved out in that reading: ekstrabladet.dk, mangago.me, manhwaweb.com,
# suruga-ya.jp (mixed.txt). A person should read the list again before a
# release; docs/nofap/research/blocklist-top10k-review.txt shows which public
# list names each one.
REVIEWED_TOP_1K: frozenset[str] = frozenset(
    """
    18comic.vip annunci69.it antarvasna3.com anybunny.org arabtnt.com arabx.cam
    av19t.com aznude.com beeg.porn besthindiporn.com bhojpurisex.site bookmark.xxx
    boyfriendtv.com chaturbate.com cityheaven.net colliderporn.com darkero.com
    desitales2.com devil69porn.tv dmm.co.jp donmai.us dto.jp e-hentai.org e621.net
    eporner.com erome.com eroterest.net erotik.markt.de escort.club exhentai.org
    f95zone.to fabswingers.com fapello.com faphouse.com faphouse2.com fatalmodel.com
    fetlife.com fpo.xxx freepornvideo.sex freesexkahani.com fuq.com hanime1.me
    happy-porn.com hdtube.porn hentai.name heovl.im hitomi.la hotxv.com ijavhd.com
    imagefap.com indiansexstories3.com indoav.com inxxx.com iporntv.net ixxx.com
    jable.tv javsub.blog javwow.com joyclub.de kamareels2.com kir2kos.net ladies.de
    leakgallery.com letsporn.com listcrawler.eu luticlip.com luxuretv.com
    mileroticos.com milfporn.tv missav.ai missav.ws missav123.com momon-ga.com
    myreadingmanga.info njavtv.com noodlemagazine.com okxxx1.com onlyfans.com
    pimpbunny.com porn300.com pornhat.com pornhat.one pornhub.com pornhub.org
    pornoborshch.com pornobrasil.com pornone.com pornpics.com pussyboy.net
    pussyspace.com qorno.com rat.xxx redtube.com redwap.sex rule34.xxx rule34video.com
    serviporno.com sexalarab.com sexmupxinh.net sexvid.xxx sexvid1.com sexwabs.com
    shahvani.com simpcity.cr skipthegames.com skokka.com spankbang.com strip2.co
    stripchat.com superporn.com supjav.com sxyprn.com taktube.net tamilsexzone.com
    theblowers.com thisvid.com tnaflix.com tokyomotion.net topgirl.co tube8.fr
    twpornstars.com ukdevilz.com upskirt.tv videoxx.com vnsextop1.com wnacg.com
    wonporn.com xhaccess.com xham.live xhamster.com xhamster.desi xhamster1.desi
    xhamster19.com xhamster2.com xhamster46.desi xhfanclub.com xhfuck.com xhname.com
    xhopen.com xhsocial.com xhspot.com xhstream.com xnxx-arabic.com xnxx.com xnxx.es
    xnxx.health xnxx.tv xnxx2.com xorgasmo.com xpaja.net xvideos-ar.com xvideos.com
    xvideos.es xvideos2.com xvideos3.com xvideosbr.blog xxx18hot.com xxxbp.tv
    xxxbule.com xxxhindi.to xxxpostpic.org xxxvideo.link youjizz.com youporn.com
    zbporn.tv zebawy.com
    """.split()
)

# General-purpose sites that a public list names by mistake and that belong on
# no rule file (a rule file would also switch the screen check off for them or
# put them under it all the time). They are taken off the list and are then
# ordinary sites. They were found in the review file and among the covered
# Chrome UX Report origins of rank bucket 100,000 or better, and judged from
# the name and from general knowledge: the build sandbox cannot open the
# sites, so a person should confirm them before a release.
UPSTREAM_MISTAKES: frozenset[str] = frozenset(
    """
    yandex.by startpagina.nl
    kinopoisk.ru mafab.hu tvigle.ru podimo.com sportlemon.net
    hmv.co.jp tia.com.ec articulo.mercadolibre.com.co cafepress.com yellowkorner.com
    more.com freetour.com reservestock.jp
    marieclaire.it refinery29.com fark.com teenee.com yorkbbs.ca
    thechive.com ebaumsworld.com acidcow.com
    perezhilton.com laineygossip.com heatworld.com crazydaysandnights.net
    knuddels.de grindr.com policies.tinder.com
    fairfaxcryobank.com
    my-cima.net v5.my-cima.net v21.my-cima.net v23.my-cima.net w26.my-cima.net
    igg-games.com steamunlocked.net
    b-cdn.net
    """.split()
)
# By line: search and start-page portals; film, video, podcast and sport
# streaming services; shops and a marketplace; ticket and booking services;
# magazines, news portals and community forums; humour sites; celebrity gossip;
# chat and dating services without an adult catalogue (the lists name no other
# mainstream dating app); a fertility clinic; a film streaming site with the
# four mirror hosts a list names; game download sites; and b-cdn.net, a
# content-delivery suffix shared by unrelated sites that is not on the Public
# Suffix List (the subdomains a list names stay listed one by one).

# ---------------------------------------------------------------------------
# Block-grade keywords (design 3.5). Used here only to filter 4skinSkywalker,
# whose list also names ad trackers and mainstream sites. The run-time rules
# live in `ubiqx_nofap::heuristics`; the two sets are kept equal by hand.
# ---------------------------------------------------------------------------

ADULT_TLDS = frozenset({"xxx", "porn", "sex", "sexy", "adult"})
BLOCK_SUBSTRINGS = (
    "porn", "xxx", "hentai", "blowjob", "gangbang", "cumshot", "handjob", "bukkake",
    "deepthroat", "creampie", "camgirl", "shemale", "upskirt", "masturbat", "bokep",
    "phimsex", "nudes", "putas", "escorts",
    "putaria", "buceta", "bucetinha", "siririca", "contoseroticos", "brasileirinhas",
    "pornozao", "safadas", "novinhas", "gostosas", "peladas", "transando", "gozando",
)
# Brand stems of research-blocklists.md 9.1. A stem with a digit or a hyphen is
# matched only on the unfolded name: folding `cam4` gives `cama` ("bed").
BRAND_STEMS = (
    "pornhub", "xvideos", "xnxx", "xhamster", "redtube", "youporn", "spankbang",
    "eporner", "chaturbate", "stripchat", "bongacams", "livejasmin", "cam4",
    "myfreecams", "onlyfans", "fansly", "brazzers", "missav", "nhentai", "hanime",
    "rule34", "e-hentai", "fapello", "erome", "coomer", "kemono", "simpcity",
    "noodlemagazine", "sxyprn", "faphouse", "javhd", "supjav",
)
# Stems that are also ordinary letter sequences (`aeromexico`, `kemonofriends`,
# `chanime`): not used by the 4skinSkywalker filter.
AMBIGUOUS_STEMS = frozenset({"erome", "kemono", "coomer", "hanime", "cam4"})
_FOLD = str.maketrans("013457", "oieast")


def fold_lookalikes(label: str) -> str:
    """`0->o 1->i 3->e 4->a 5->s 7->t`, `vv->w`, digits at the edges removed."""
    return label.strip("0123456789").translate(_FOLD).replace("vv", "w")


def block_grade(name: str, suffix_labels: int) -> str | None:
    """The block-grade rule that `name` carries, or None.

    `suffix_labels` is the number of labels of the public suffix; keywords are
    searched to the left of it, the TLD rule looks at the last label.
    """
    labels = name.split(".")
    if labels[-1] in ADULT_TLDS:
        return "tld:" + labels[-1]
    left = ".".join(labels[: len(labels) - suffix_labels])
    for word in BLOCK_SUBSTRINGS:
        if word in left:
            return "word:" + word
    folded = ".".join(fold_lookalikes(part) for part in left.split("."))
    for stem in BRAND_STEMS:
        if stem in AMBIGUOUS_STEMS:
            continue
        if stem in left:
            return "brand:" + stem
        if stem.isalpha() and stem in folded:
            return "brand:" + stem
    return None


# ---------------------------------------------------------------------------
# Names
# ---------------------------------------------------------------------------

_LABEL = re.compile(r"^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$")
_IPV4 = re.compile(r"^\d{1,3}(\.\d{1,3}){3}$")
_LOCAL_NAMES = frozenset(
    {"localhost", "localhost.localdomain", "local", "broadcasthost", "ip6-localhost",
     "ip6-loopback", "ip6-localnet", "ip6-mcastprefix", "ip6-allnodes", "ip6-allrouters",
     "ip6-allhosts"}
)
_STRIP_PREFIXES = ("www.", "m.", "mobile.")
_STALE = re.compile(r"\.tumblr\.com$|\.blogspot\.[a-z]{2,3}(\.[a-z]{2})?$")


def clean_name(raw: str) -> str | None:
    """Lower-case, strip trailing dots; None for anything that is not a host name."""
    name = raw.strip().lower().rstrip(".")
    if not name or name in _LOCAL_NAMES or len(name) > 253:
        return None
    if _IPV4.match(name) or ":" in name:
        return None
    labels = name.split(".")
    if len(labels) < 2 or not all(_LABEL.match(part) for part in labels):
        return None
    if labels[-1].isdigit():
        return None
    return name


def is_hosts_name(name: str) -> bool:
    """Mirror of `ubiqx_nofap::hosts::is_valid_hostname` (design 2.5)."""
    return clean_name(name) == name and 4 <= len(name) <= 253


def strip_prefix(name: str, psl: "Psl") -> str:
    """Remove leading `www.`, `m.`, `mobile.` labels while what remains is still a
    registrable name (`www.example.com` -> `example.com`; `mobile.co.uk` stays)."""
    while name.startswith(_STRIP_PREFIXES):
        rest = name.split(".", 1)[1]
        if psl.registrable(rest) is None:
            break
        name = rest
    return name


def normalize_host(raw: str) -> str | None:
    """Rule 2 of the contract, for one host or `host:port` as typed or as in a URL."""
    host = raw.strip().lower()
    if "://" in host:
        host = host.split("://", 1)[1]
    host = host.split("/", 1)[0].split("?", 1)[0].split("#", 1)[0]
    host = host.rsplit("@", 1)[-1]
    if host.startswith("["):
        return None
    host = host.split(":", 1)[0].rstrip(".")
    name = clean_name(host)
    if name is None:
        return None
    if name.startswith("www.") and name.count(".") >= 2:
        name = name[4:]
    return name


def parents(name: str):
    """The name, then the name without its left-most labels, down to two labels."""
    labels = name.split(".")
    for start in range(len(labels) - 1):
        yield ".".join(labels[start:])


def parse_list(text: str, fmt: str) -> tuple[list[str], dict[str, int]]:
    """Names of a `hosts` or `plain` list, in file order, with what was dropped."""
    names: list[str] = []
    stats = {"lines": 0, "invalid": 0}
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        stats["lines"] += 1
        tokens = line.split()
        if fmt == "hosts":
            if len(tokens) < 2:
                stats["invalid"] += 1
                continue
            tokens = tokens[1:]
        else:
            tokens = tokens[:1]
        for token in tokens:
            name = clean_name(token) if token.isascii() else None
            if name is None:
                stats["invalid"] += 1
            else:
                names.append(name)
    return names, stats


def read_names(path: Path) -> list[str]:
    """A hand-maintained list of host names; an invalid line is an error."""
    out: list[str] = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if clean_name(line) != line:
            raise BuildError(f"{path.name}:{number}: not a normalised host name: {line!r}")
        out.append(line)
    if len(set(out)) != len(out):
        dup = sorted({n for n in out if out.count(n) > 1})
        raise BuildError(f"{path.name}: duplicate names: {dup}")
    return out


# ---------------------------------------------------------------------------
# Public Suffix List
# ---------------------------------------------------------------------------


class Psl:
    def __init__(self, text: str):
        self.exact: set[str] = set()
        self.wild: set[str] = set()
        self.exceptions: set[str] = set()
        self.private: set[str] = set()
        section_private = False
        for line in text.splitlines():
            line = line.strip()
            if line.startswith("//"):
                if "===BEGIN PRIVATE DOMAINS===" in line:
                    section_private = True
                continue
            if not line:
                continue
            rule = line.split()[0].lower()
            kind = self.exact
            if rule.startswith("!"):
                rule, kind = rule[1:], self.exceptions
            elif rule.startswith("*."):
                rule, kind = rule[2:], self.wild
            try:
                rule = rule.encode("idna").decode("ascii")
            except UnicodeError:
                continue
            kind.add(rule)
            if section_private:
                self.private.add(rule)
        if len(self.exact) < 5_000 or "com" not in self.exact or "co.uk" not in self.exact:
            raise BuildError("the Public Suffix List did not parse")

    def suffix_labels(self, name: str, private: bool = True) -> int:
        """Number of labels of the public suffix of `name` (1 for an unknown TLD)."""
        labels = name.split(".")
        for start in range(len(labels)):
            candidate = ".".join(labels[start:])
            rest = ".".join(labels[start + 1:])
            if candidate in self.exceptions and (private or candidate not in self.private):
                return len(labels) - start - 1
            if candidate in self.exact and (private or candidate not in self.private):
                return len(labels) - start
            if rest and rest in self.wild and (private or rest not in self.private):
                return len(labels) - start
        return 1

    def registrable(self, name: str, private: bool = True) -> str | None:
        """eTLD+1 of `name`, or None when `name` is itself a public suffix."""
        labels = name.split(".")
        count = self.suffix_labels(name, private)
        if len(labels) <= count:
            return None
        return ".".join(labels[len(labels) - count - 1:])


# ---------------------------------------------------------------------------
# Rule files
# ---------------------------------------------------------------------------


class Rules:
    """`never-block.txt`, `mixed.txt`, `never-capture.txt` (contract, rule 6)."""

    def __init__(self, path: Path):
        self.file = path.name
        self.names: set[str] = set()
        self.tlds: set[str] = set()
        self.second_level: set[str] = set()
        self.count = 0
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if line != line.strip():
                raise BuildError(f"{self.file}:{number}: leading or trailing space")
            if not line or line.startswith("#"):
                continue
            where = f"{self.file}:{number}: {line!r}"
            if not line.isascii() or line != line.lower() or " " in line or "\t" in line:
                raise BuildError(f"{where}: a rule is one lower-case ASCII token")
            if line.endswith(".*"):
                head = line[:-2]
                if not _LABEL.match(head):
                    raise BuildError(f"{where}: the second-level form is one label and '.*'")
                target = self.second_level
                key = head
            elif "." not in line:
                if not _LABEL.match(line) or line.isdigit():
                    raise BuildError(f"{where}: bad TLD rule")
                target, key = self.tlds, line
            else:
                if clean_name(line) != line:
                    raise BuildError(f"{where}: not a normalised host name")
                target, key = self.names, line
            if key in target:
                raise BuildError(f"{where}: duplicate rule")
            target.add(key)
            self.count += 1

    def suffix_rule(self, name: str) -> str | None:
        """The TLD or SECOND-LEVEL rule that decides `name` at any depth, or None."""
        labels = name.split(".")
        if labels[-1] in self.tlds:
            return labels[-1]
        if len(labels) >= 2 and labels[-2] in self.second_level:
            return labels[-2] + ".*"
        return None

    def name_rule(self, name: str) -> str | None:
        """The most specific NAME rule at or above `name`, or None."""
        for candidate in parents(name):
            if candidate in self.names:
                return candidate
        return None

    def covers(self, name: str) -> str | None:
        """The rule that `name` is under (at any depth), or None."""
        return self.suffix_rule(name) or self.name_rule(name)

    def removes(self, name: str) -> str | None:
        """The rule that takes a listed `name` off the list at build time, or None."""
        return self.suffix_rule(name) or (name if name in self.names else None)


# ---------------------------------------------------------------------------
# Hash tables
# ---------------------------------------------------------------------------


def key_of(tag: bytes, name: str) -> int:
    return int.from_bytes(hashlib.sha256(tag + name.encode("ascii")).digest()[:8], "big")


def write_table(keys: list[int], list_version: int) -> bytes:
    if keys != sorted(set(keys)):
        raise BuildError("table keys must be sorted and unique")
    head = HEADER.pack(MAGIC, FORMAT, FLAGS, len(keys), list_version)
    return head + struct.pack(f">{len(keys)}Q", *keys)


def read_table(data: bytes) -> tuple[int, list[int]]:
    """(list_version, keys) of a table; raises on any malformed input."""
    if len(data) < HEADER.size:
        raise BuildError("table: shorter than its header")
    magic, fmt, flags, count, list_version = HEADER.unpack_from(data)
    if magic != MAGIC:
        raise BuildError("table: bad magic")
    if fmt != FORMAT or flags != FLAGS:
        raise BuildError(f"table: unsupported format {fmt} or flags {flags}")
    if len(data) != HEADER.size + 8 * count:
        raise BuildError("table: length does not match count")
    keys = list(struct.unpack_from(f">{count}Q", data, HEADER.size))
    if any(a >= b for a, b in zip(keys, keys[1:])):
        raise BuildError("table: keys are not strictly ascending")
    return list_version, keys


def table_has(keys: list[int], key: int) -> bool:
    at = bisect.bisect_left(keys, key)
    return at < len(keys) and keys[at] == key


def table_covers(keys: list[int], tag: bytes, name: str) -> str | None:
    """The listed ancestor-or-self of a normalised `name`, by key lookup only."""
    for candidate in parents(name):
        if table_has(keys, key_of(tag, candidate)):
            return candidate
    return None


# ---------------------------------------------------------------------------
# Sources
# ---------------------------------------------------------------------------


class BuildError(Exception):
    pass


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def source_url(src: dict) -> str:
    return f"https://raw.githubusercontent.com/{src['repo']}/{src['commit']}/{src['path']}"


def http_get(url: str, ca_bundle: str | None) -> bytes:
    context = ssl.create_default_context(cafile=ca_bundle)
    request = urllib.request.Request(url, headers={"User-Agent": "blocklist-build"})
    with urllib.request.urlopen(request, timeout=120, context=context) as reply:
        return reply.read()


def load_lock() -> dict:
    lock = json.loads((DATA / "sources.lock.json").read_text(encoding="utf-8"))
    if lock.get("schema") != 1:
        raise BuildError("sources.lock.json: unknown schema")
    version = lock.get("list_version")
    if not isinstance(version, int) or not re.fullmatch(r"20\d{10}", str(version)):
        raise BuildError("sources.lock.json: list_version must be YYYYMMDDHHMM")
    seen = set()
    for src in lock["sources"]:
        for field in ("id", "repo", "commit", "path", "url", "sha256", "bytes", "licence", "role"):
            if field not in src:
                raise BuildError(f"sources.lock.json: {src.get('id')}: missing {field}")
        if not re.fullmatch(r"[0-9a-f]{40}", src["commit"]):
            raise BuildError(f"sources.lock.json: {src['id']}: commit is not a full SHA-1")
        if src["url"] != source_url(src):
            raise BuildError(f"sources.lock.json: {src['id']}: url does not match repo/commit/path")
        if src["id"] in seen or not re.fullmatch(r"[a-z0-9-]+", src["id"]):
            raise BuildError(f"sources.lock.json: bad or duplicate id {src['id']}")
        seen.add(src["id"])
    return lock


def cache_path(src: dict) -> Path:
    return CACHE / f"{src['id']}.{src['sha256'][:16]}"


def fetch(src: dict, offline: bool, ca_bundle: str | None) -> bytes | None:
    """The pinned bytes of a source, from the cache or the network; None when
    offline and not cached. A hash mismatch is always an error."""
    path = cache_path(src)
    if path.is_file():
        data = path.read_bytes()
        if sha256_hex(data) == src["sha256"]:
            return data
        path.unlink()
    if offline:
        return None
    try:
        data = http_get(src["url"], ca_bundle)
    except Exception as error:  # noqa: BLE001 - every network failure ends the build
        raise BuildError(
            f"{src['id']}: download failed ({error}). If the pinned commit is gone "
            f"(ShadowWhisperer squashes its history), re-pin with --pin."
        ) from error
    if sha256_hex(data) != src["sha256"] or len(data) != src["bytes"]:
        raise BuildError(
            f"{src['id']}: SHA-256 {sha256_hex(data)} ({len(data)} bytes) does not match the pin"
        )
    CACHE.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(".part")
    temp.write_bytes(data)
    temp.replace(path)
    return data


def fetch_all(lock: dict, offline: bool, ca_bundle: str | None) -> dict[str, bytes] | None:
    blobs: dict[str, bytes] = {}
    missing = []
    for src in lock["sources"]:
        data = fetch(src, offline, ca_bundle)
        if data is None:
            missing.append(src["id"])
        else:
            blobs[src["id"]] = data
    if missing:
        if offline:
            return None
        raise BuildError(f"sources not available: {missing}")
    return blobs


def pin(ca_bundle: str | None, now_version: int) -> None:
    """Move every source to its repository's HEAD and record hashes."""
    lock = load_lock()
    heads: dict[str, str] = {}
    for src in lock["sources"]:
        repo = src["repo"]
        if repo not in heads:
            refs = http_get(f"https://github.com/{repo}.git/info/refs?service=git-upload-pack", ca_bundle)
            found = re.search(rb"([0-9a-f]{40}) HEAD", refs)
            if not found:
                raise BuildError(f"{repo}: could not read HEAD")
            heads[repo] = found.group(1).decode()
        src["commit"] = heads[repo]
        src["url"] = source_url(src)
        data = http_get(src["url"], ca_bundle)
        src["sha256"], src["bytes"] = sha256_hex(data), len(data)
        CACHE.mkdir(parents=True, exist_ok=True)
        cache_path(src).write_bytes(data)
        print(f"pinned {src['id']}: {src['commit'][:12]} sha256 {src['sha256'][:16]} {len(data)} bytes")
    lock["list_version"] = max(now_version, lock["list_version"] + 1)
    write_atomic(DATA / "sources.lock.json", dump_json(lock))
    print(f"list_version {lock['list_version']}; now run the build")


# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

HAND_INPUTS = (
    "never-block.txt", "mixed.txt", "never-capture.txt", "extra-ptbr.txt",
    "canary-block.txt", "canary-allow.txt", "sources.lock.json",
)
OUTPUTS = ("core.bin", "popular.bin", "hosts-tier.txt", "NOTICE.txt", "core.meta.json")

# Design 3.6. (input, must be covered by core.bin)
LIST_CASES = (
    ("PT.PornHub.com.", True),
    ("www.xvideos.com:443", True),
    ("https://www.reddit.com/r/EarthPorn/", False),
    ("essex.gov.uk", False), ("sussex.ac.uk", False), ("sussexexpress.co.uk", False),
    ("planalto.gov.br", False), ("canaltech.com.br", False), ("motorola.com.br", False),
    ("drogariasaopaulo.com.br", False), ("peacocktv.com", False), ("emilfrey.de", False),
    ("milfordasset.com", False), ("lichngaytot.com", False),
    ("coroasparavelorio.com.br", False), ("nakedwines.com", False),
    ("skylinewebcams.com", False), ("iowasexoffender.gov", False), ("pussy888.com", False),
    ("cambridge.org", False), ("xxxlutz.de", False), ("scunthorpe.co.uk", False),
    ("penistone.co.uk", False), ("ninjavan.co", False), ("fapesp.br", False),
    ("fc2.com", False), ("blog.fc2.com", False),
)
# Design 3.6: names a rule file must cover, whatever the keyword rules say.
NEVER_BLOCK_CASES = (
    "essex.gov.uk", "sussex.ac.uk", "planalto.gov.br", "iowasexoffender.gov", "xxxlutz.de",
    "fapesp.br", "scarleteen.com", "plannedparenthood.org", "apple.com",
)
# Design 3.6: (name, carries a block-grade keyword or TLD)
KEYWORD_CASES = (
    ("porno365.golf", True), ("missav123.com", True), ("anything.xxx", True),
    ("videosdeputaria.com.br", True),
    ("sussexexpress.co.uk", False), ("canaltech.com.br", False), ("motorola.com.br", False),
    ("drogariasaopaulo.com.br", False), ("peacocktv.com", False), ("emilfrey.de", False),
    ("milfordasset.com", False), ("lichngaytot.com", False),
    ("coroasparavelorio.com.br", False), ("nakedwines.com", False),
    ("skylinewebcams.com", False), ("pussy888.com", False), ("cambridge.org", False),
    ("scunthorpe.co.uk", False), ("penistone.co.uk", False), ("ninjavan.co", False),
    ("aeromexico.com", False),
)
VECTOR_NAMES = ("example.com", "sub.example.co.uk", "xn--bcher-kva.example", "a.b")


def load_crux(blob: bytes) -> dict[str, int]:
    """host -> best rank bucket."""
    ranks: dict[str, int] = {}
    with gzip.open(io.BytesIO(blob), "rt", encoding="utf-8", newline="") as handle:
        reader = csv.reader(handle)
        if next(reader) != ["origin", "rank"]:
            raise BuildError("CrUX: unexpected header")
        for origin, rank in reader:
            host = origin.split("://", 1)[-1].split("/", 1)[0].split(":", 1)[0]
            name = clean_name(host) if host.isascii() else None
            if name is None:
                continue
            bucket = int(rank)
            if bucket < ranks.get(name, 1 << 62):
                ranks[name] = bucket
    if len(ranks) < 500_000:
        raise BuildError("CrUX: fewer than 500,000 hosts")
    return ranks


def suffix_dedup(names: set[str]) -> set[str]:
    out = set()
    for name in names:
        labels = name.split(".")
        if not any(".".join(labels[start:]) in names for start in range(1, len(labels) - 1)):
            out.add(name)
    return out


def copyright_lines(text: str) -> list[str]:
    """The copyright notices of a licence file, exactly as written."""
    return [line.strip() for line in text.splitlines()
            if re.match(r"\s*Copyright\s*(\(c\)|\u00a9)", line, re.IGNORECASE)]


def build(lock: dict, blobs: dict[str, bytes]) -> tuple[dict[str, bytes], list[str], list[str]]:
    """Every output as bytes, the failures of the gates (empty when all pass),
    and notes for the maintainer that fail nothing."""
    failures: list[str] = []
    notes: list[str] = []
    by_id = {src["id"]: src for src in lock["sources"]}
    list_version = lock["list_version"]

    psl = Psl(blobs["public-suffix-list"].decode("utf-8"))
    never_block = Rules(DATA / "never-block.txt")
    mixed = Rules(DATA / "mixed.txt")
    never_capture = Rules(DATA / "never-capture.txt")
    protected = (never_block, mixed, never_capture)
    extra = read_names(DATA / "extra-ptbr.txt")
    canary_block = read_names(DATA / "canary-block.txt")
    canary_allow = read_names(DATA / "canary-allow.txt")

    def protecting(name: str) -> str | None:
        for rules in protected:
            rule = rules.covers(name)
            if rule:
                return f"{rules.file}: {rule}"
        return None

    # --- the three lists -------------------------------------------------
    origin_of: dict[str, set[str]] = {}
    removed: dict[str, tuple[str, set[str]]] = {}
    stats = []
    union: set[str] = set()
    for src in lock["sources"]:
        if src["role"] != "list":
            continue
        names, parse_stats = parse_list(blobs[src["id"]].decode("utf-8", "replace"), src["format"])
        unique = set(names)
        stripped = {strip_prefix(name, psl) for name in unique}
        no_suffix = {name for name in stripped if psl.registrable(name) is not None}
        live = {name for name in no_suffix if not _STALE.search(name)}
        if src.get("filter") == "block-grade-keyword":
            kept = {name for name in live if block_grade(name, psl.suffix_labels(name))}
        elif src.get("filter") is None:
            kept = live
        else:
            raise BuildError(f"{src['id']}: unknown filter {src['filter']!r}")
        final = set()
        for name in kept:
            hit = "build_blocklist.py: UPSTREAM_MISTAKES" if name in UPSTREAM_MISTAKES else None
            for rules in () if hit else protected:
                rule = rules.removes(name)
                if rule:
                    hit = f"{rules.file}: {rule}"
                    break
            if hit:
                removed.setdefault(name, (hit, set()))[1].add(src["id"])
            else:
                final.add(name)
        for name in final:
            origin_of.setdefault(name, set()).add(src["id"])
        union |= final
        stats.append({
            "id": src["id"],
            "lines": parse_stats["lines"],
            "invalid": parse_stats["invalid"],
            "unique": len(unique),
            "after_prefix_strip": len(stripped),
            "public_suffixes_dropped": len(stripped) - len(no_suffix),
            "tumblr_blogspot_dropped": len(no_suffix) - len(live),
            "keyword_filter_dropped": len(live) - len(kept),
            "protected_dropped": len(kept) - len(final),
            "contributed": len(final),
        })

    # --- our supplement --------------------------------------------------
    extra_new = 0
    for name in extra:
        why = None
        if strip_prefix(name, psl) != name:
            why = "carries a www./m./mobile. prefix"
        elif psl.registrable(name) is None:
            why = "is a public suffix"
        elif protecting(name):
            why = f"is protected ({protecting(name)})"
        if why:
            failures.append(f"extra-ptbr.txt: {name} {why}")
            continue
        if not any(parent in union for parent in parents(name)):
            extra_new += 1
        origin_of.setdefault(name, set()).add("extra-ptbr")
        union.add(name)

    core = suffix_dedup(union)
    core_keys = sorted({key_of(TAG_CORE, name) for name in core})
    if len(core_keys) != len(core):
        failures.append("core: two names share a key")
    core_bin = write_table(core_keys, list_version)

    def covering(name: str) -> str | None:
        for candidate in parents(name):
            if candidate in core:
                return candidate
        return None

    # --- gates on the list itself ----------------------------------------
    for name in sorted(core):
        if psl.registrable(name) is None or clean_name(name) != name:
            failures.append(f"core: {name} is a public suffix or not a host name")
        elif strip_prefix(name, psl) != name:
            failures.append(f"core: {name} kept a www./m./mobile. prefix")
    for rules in protected:
        for rule_name in sorted(rules.names):
            above = covering(rule_name)
            if above:
                failures.append(
                    f"core: listed name {above} is at or above the protected name "
                    f"{rule_name} ({rules.file})"
                )
    if not CORE_MIN <= len(core) <= CORE_MAX:
        failures.append(f"core: {len(core)} entries, outside {CORE_MIN}..{CORE_MAX}")
    if len(core_bin) > CORE_MAX_BYTES:
        failures.append(f"core.bin: {len(core_bin)} bytes, above {CORE_MAX_BYTES}")
    previous = None
    meta_path = DATA / "core.meta.json"
    if meta_path.is_file():
        previous = json.loads(meta_path.read_text(encoding="utf-8")).get("core", {}).get("count")
    if previous and not previous * 0.90 <= len(core) <= previous * 1.25:
        failures.append(
            f"core: {len(core)} entries, outside -10% / +25% of the previous {previous}"
        )

    # Canaries and the cases of design 3.6, by key lookup in the built table
    # (the same path the Rust matcher takes).
    _, table_keys = read_table(core_bin)

    def in_table(raw: str) -> str | None:
        name = normalize_host(raw)
        return table_covers(table_keys, TAG_CORE, name) if name else None

    for name in canary_block:
        if not in_table(name):
            failures.append(f"canary-block: {name} is not covered")
    for name in canary_allow:
        if in_table(name):
            failures.append(f"canary-allow: {name} is covered by {in_table(name)}")
    for raw, expected in LIST_CASES:
        if bool(in_table(raw)) != expected:
            failures.append(f"design 3.6: {raw} covered={bool(in_table(raw))}, expected {expected}")
    for name in NEVER_BLOCK_CASES:
        if not never_block.covers(name):
            failures.append(f"design 3.6: {name} is not under a never-block rule")
    if not mixed.covers("reddit.com"):
        failures.append("design 3.6: reddit.com is not on mixed.txt")
    for name, expected in KEYWORD_CASES:
        got = block_grade(name, psl.suffix_labels(name))
        if bool(got) != expected:
            failures.append(f"design 3.6: block_grade({name}) = {got}, expected {expected}")

    # --- Chrome UX Report -------------------------------------------------
    ranks = load_crux(blobs["crux-top-1m"])
    ordered = sorted(ranks, key=lambda host: (ranks[host], host))

    review_rows = []
    for host in ordered:
        if ranks[host] > REVIEW_MAX_RANK:
            break
        entry = covering(host)
        if not entry:
            continue
        review_rows.append((ranks[host], host, entry, ",".join(sorted(origin_of[entry]))))
        if ranks[host] <= GATE_MAX_RANK and entry not in REVIEWED_TOP_1K:
            failures.append(
                f"top-1,000: {host} is covered by {entry}, which is not in REVIEWED_TOP_1K"
            )
    # A listed name has no listed ancestor (suffix de-duplication), so the name
    # found above is the only one at or above the origin's registrable domain.
    top_1k = {row[2] for row in review_rows if row[0] <= GATE_MAX_RANK}
    stale = sorted(REVIEWED_TOP_1K - top_1k)
    if stale:
        notes.append(f"REVIEWED_TOP_1K: no longer cover a top-1,000 origin: {' '.join(stale)}")
    unused = sorted(name for name in UPSTREAM_MISTAKES if name not in removed)
    if unused:
        notes.append(f"UPSTREAM_MISTAKES: no longer on any list: {' '.join(unused)}")

    # Hosts tier. The Chrome UX Report gives a rank bucket, not a rank, so the
    # order inside a bucket carries no information; a hash of the name makes the
    # cut at a cap (20,000 here, fewer on Windows) an even sample of the bucket
    # instead of the start of the alphabet.
    tier_order = sorted(
        (host for host in ranks if ranks[host] <= HOSTS_TIER_MAX_RANK),
        key=lambda host: (ranks[host], hashlib.sha256(host.encode("ascii")).digest()),
    )
    tier_buckets: dict[str, int] = {}
    tier: list[str] = []
    tier_seen: set[str] = set()
    tier_skipped_protected = 0
    for host in tier_order:
        if len(tier) >= HOSTS_TIER_CAP:
            break
        if not covering(host):
            continue
        candidates = [host]
        apex = psl.registrable(host)
        if apex and apex in core:
            candidates += [apex, "www." + apex]
        for name in candidates:
            if name in tier_seen:
                continue
            tier_seen.add(name)
            if not is_hosts_name(name):
                continue
            if protecting(name):
                tier_skipped_protected += 1
                continue
            if len(tier) < HOSTS_TIER_CAP:
                tier.append(name)
                bucket = str(ranks[host])
                tier_buckets[bucket] = tier_buckets.get(bucket, 0) + 1
    hosts_tier = ("\n".join(tier) + "\n").encode("ascii")
    if not 1_000 <= len(tier) <= HOSTS_TIER_CAP or len(set(tier)) != len(tier):
        failures.append(f"hosts-tier: {len(tier)} lines")
    for name in tier:
        if not covering(name):
            failures.append(f"hosts-tier: {name} is not covered by core")

    # Popular table.
    popular: set[str] = set()
    for host in ordered:
        if ranks[host] > POPULAR_MAX_RANK:
            break
        name = normalize_host(host)
        if name is None or psl.registrable(name) is None:
            continue
        if covering(name) or mixed.covers(name):
            continue
        popular.add(name)
    popular_keys = sorted({key_of(TAG_POPULAR, name) for name in popular})
    popular_bin = write_table(popular_keys, list_version)
    if len(popular_bin) > POPULAR_MAX_BYTES or len(popular) < 50_000:
        failures.append(f"popular.bin: {len(popular)} keys, {len(popular_bin)} bytes")
    for name in canary_allow:
        if mixed.covers(name) or never_block.covers(name):
            continue
        if ranks.get(name, ranks.get("www." + name, 1 << 62)) <= POPULAR_MAX_RANK \
                and not table_has(popular_keys, key_of(TAG_POPULAR, name)):
            failures.append(f"popular: {name} is in the top 100,000 but not in the table")

    # --- notice, review list, meta ------------------------------------------
    notice = render_notice(lock, blobs, by_id)
    failures += notices_out_of_date(notice)
    failures += stevenblack_upstreams_changed(blobs["stevenblack-sources"])
    review = render_review(lock, review_rows, core, origin_of, removed, protected)
    counts = {
        "union_before_dedup": len(union),
        "extra_ptbr_names": len(extra),
        "extra_ptbr_not_covered_by_public_lists": extra_new,
        "removed_by_rule_files": len(removed),
        "top10k_hosts_covered": len(review_rows),
        "top1k_hosts_covered": sum(1 for row in review_rows if row[0] <= GATE_MAX_RANK),
        "crux_hosts": len(ranks),
        "hosts_tier_skipped_protected": tier_skipped_protected,
        "br_entries": sum(1 for name in core if name.endswith(".br")),
    }
    outputs = {
        "core.bin": core_bin,
        "popular.bin": popular_bin,
        "hosts-tier.txt": hosts_tier,
        "NOTICE.txt": notice.encode("utf-8"),
    }
    meta = {
        "schema": 1,
        "generated_by": "scripts/nofap/build_blocklist.py",
        "list_version": list_version,
        "format": {
            "magic_hex": MAGIC.hex(),
            "format": FORMAT,
            "flags": FLAGS,
            "header_bytes": HEADER.size,
            "layout": "magic[8] | format u16 | flags u16 | count u32 | list_version u64 | "
                      "count * key u64, strictly ascending; every integer big-endian",
            "key": "first 8 bytes of SHA-256(tag || name) as a big-endian u64; the tag "
                   "ends with one NUL byte, written in this file as the JSON escape \\u0000",
            "tags": {"core.bin": TAG_CORE.decode(), "popular.bin": TAG_POPULAR.decode()},
            "name": "lower-case ASCII, punycode for IDN, no trailing dot, no port, a "
                    "leading 'www.' removed when two labels remain",
            "core_lookup": "the host, then the host without its left-most labels, down to "
                           "two labels; an entry covers itself and its subdomains",
            "popular_lookup": "the exact normalised host; no parent walk",
        },
        "core": {"count": len(core_keys), "bytes": len(core_bin), "sha256": sha256_hex(core_bin)},
        "popular": {
            "count": len(popular_keys), "bytes": len(popular_bin),
            "sha256": sha256_hex(popular_bin),
        },
        "hosts_tier": {
            "lines": len(tier), "bytes": len(hosts_tier), "sha256": sha256_hex(hosts_tier),
            "lines_by_rank_bucket": tier_buckets,
        },
        "notice": {"sha256": sha256_hex(outputs["NOTICE.txt"])},
        "rules": {rules.file: rules.count for rules in protected},
        "canaries": {"block": len(canary_block), "allow": len(canary_allow)},
        "sources": stats,
        "counts": counts,
        "inputs": {name: sha256_hex((DATA / name).read_bytes()) for name in HAND_INPUTS},
        "vector": [
            {"table": table, "tag": tag.decode(), "name": name, "key": f"{key_of(tag, name):016x}"}
            for table, tag in (("core.bin", TAG_CORE), ("popular.bin", TAG_POPULAR))
            for name in VECTOR_NAMES
        ],
    }
    outputs["core.meta.json"] = dump_json(meta)
    outputs["blocklist-top10k-review.txt"] = review.encode("utf-8")
    return outputs, failures, notes


MIT_TEXT = """\
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""


# What StevenBlack's "porn-only" file unifies, as its own index states it
# (readmeData.json, key "porn-only"): project page -> licence. NOTICE.txt and
# THIRD-PARTY-NOTICES are written for exactly this set; when a re-pin changes
# it, the build stops until the notices and this table are brought up to date.
STEVENBLACK_UPSTREAMS = {
    "https://github.com/bigdargon/hostsVN/": "MIT",
    "https://github.com/brijrajparmar27/host-sources": "CC BY 4.0",
    "https://github.com/Clefspeare13/pornhosts": "MIT",
    "https://github.com/Sinfonietta/hostfiles": "MIT",
    "https://github.com/tiuxo/hosts": "CC BY 4.0",
}


def stevenblack_upstreams_changed(blob: bytes) -> list[str]:
    try:
        rows = json.loads(blob)["porn-only"]["sourcesdata"]
        found = {row["homeurl"]: row["license"] for row in rows}
    except (ValueError, KeyError, TypeError) as error:
        return [f"stevenblack-sources: cannot read the index ({error!r})"]
    if found != STEVENBLACK_UPSTREAMS:
        return [f"stevenblack-sources: the upstream lists or their licences changed: {found}; "
                "update STEVENBLACK_UPSTREAMS, render_notice and THIRD-PARTY-NOTICES"]
    return []


def render_notice(lock: dict, blobs: dict[str, bytes], by_id: dict[str, dict]) -> str:
    def licence(source_id: str) -> tuple[str, str]:
        text = blobs[source_id].decode("utf-8")
        lines = copyright_lines(text)
        if not lines:
            raise BuildError(f"{source_id}: no copyright line found")
        squeezed = " ".join(text.replace("\u201c", '"').replace("\u201d", '"').split())
        if " ".join(MIT_TEXT.split())[:200] not in squeezed:
            raise BuildError(f"{source_id}: the licence text is not the MIT licence")
        return "\n".join("       " + line for line in lines), by_id[source_id]["commit"]

    sb, sb_commit = licence("licence-stevenblack")
    sinf, sinf_commit = licence("licence-sinfonietta")
    bigd, bigd_commit = licence("licence-bigdargon")
    skin, skin_commit = licence("licence-4skinskywalker")
    shadow = by_id["shadowwhisperer-adult"]
    crux = by_id["crux-top-1m"]
    return f"""\
Adult-content blocklist data: third-party notices
==================================================

Generated by scripts/nofap/build_blocklist.py from the pinned commits named
below. Do not edit by hand. List version {lock['list_version']}.

The files core.bin and hosts-tier.txt in this directory are compiled from the
following lists.

1. StevenBlack/hosts, "porn" extension (alternates/porn-only/hosts)
   https://github.com/StevenBlack/hosts
   commit {sb_commit}
   MIT License. From license.txt at that commit:
{sb}
   The extension unifies these lists (licences as stated in readmeData.json
   of that commit, key "porn-only"):
   - Sinfonietta/hostfiles (pornography-hosts, snuff-hosts), MIT License.
     https://github.com/Sinfonietta/hostfiles
     From LICENSE at commit {sinf_commit}:
{sinf}
   - Clefspeare13/pornhosts, MIT License. The upstream repository no longer
     exists; StevenBlack/hosts keeps a frozen copy, and no copyright line
     could be copied.
   - bigdargon/hostsVN (extensions/adult), MIT License.
     https://github.com/bigdargon/hostsVN
     From LICENSE at commit {bigd_commit}:
{bigd}
   - brijrajparmar27/host-sources (Porn/hosts), Creative Commons Attribution
     4.0 International. https://github.com/brijrajparmar27/host-sources
   - tiuxo/hosts (porn), Creative Commons Attribution 4.0 International.
     https://github.com/tiuxo/hosts

2. ShadowWhisperer/BlockLists, "Adult" (Lists/Adult)
   https://github.com/ShadowWhisperer/BlockLists
   commit {shadow['commit']}
   Released into the public domain (The Unlicense).

3. 4skinSkywalker/Anti-Porn-HOSTS-File (HOSTS.txt)
   https://github.com/4skinSkywalker/Anti-Porn-HOSTS-File
   commit {skin_commit}
   MIT License. From LICENSE at that commit:
{skin}
   Only the entries whose name carries an adult keyword were used.

Changes made to the lists: they were merged; comments, address columns,
invalid names, "www.", "m." and "mobile." prefixes, public suffixes,
tumblr.com and blogspot subdomains and the names on our own allowlists were
removed; a short list of our own was added; names already covered by a listed
parent were dropped; the result was converted to a table of truncated SHA-256
hashes (core.bin). hosts-tier.txt is the part of the result that the Chrome UX
Report names among the most visited sites.

Site popularity (popular.bin, and the choice and order of hosts-tier.txt) is
derived from the Chrome UX Report by Google, licensed under Creative Commons
Attribution 4.0 International (https://developer.chrome.com/docs/crux), as
cached by https://github.com/zakird/crux-top-lists (data/global/current.csv.gz)
at commit {crux['commit']}. The origins were
reduced to host names, filtered and hashed.

The Public Suffix List (https://publicsuffix.org, Mozilla Public License 2.0)
was used at build time as a filter. It is not part of these files.

Creative Commons Attribution 4.0 International:
https://creativecommons.org/licenses/by/4.0/

The MIT License
---------------

{MIT_TEXT}"""


def notices_out_of_date(notice: str) -> list[str]:
    """The copyright lines of NOTICE.txt that the hand-written THIRD-PARTY-NOTICES
    at the repository root does not carry word for word."""
    shipped = THIRD_PARTY.read_text(encoding="utf-8") if THIRD_PARTY.is_file() else ""
    return [f"THIRD-PARTY-NOTICES: missing the line {line!r}"
            for line in copyright_lines(notice) if line not in shipped]


def render_review(lock, review_rows, core, origin_of, removed, protected) -> str:
    out = [
        "# Review list of the adult-content blocklist (core tier).",
        "# Generated by scripts/nofap/build_blocklist.py; do not edit by hand.",
        f"# List version {lock['list_version']}.",
        "#",
        "# For a person to read before a release. Three sections:",
        "#   1. every Chrome UX Report origin with rank bucket <= 10,000 that the list covers",
        "#      (a general-purpose site here is a false positive: add it to mixed.txt,",
        "#      to never-block.txt or to UPSTREAM_MISTAKES in the script, and rebuild);",
        "#   2. listed names kept below a NAME rule of a rule file (longest match: the",
        "#      listed subdomain stays blocked, its parent stays allowed);",
        "#   3. listed names removed by a rule file or by UPSTREAM_MISTAKES (a dedicated",
        "#      adult site here means a rule is too broad).",
        "# Columns are separated by tabs.",
        "",
        f"## 1. Covered origins in the top 10,000 ({len(review_rows)})",
        "# rank bucket\thost\tlisted name that covers it\tsources",
    ]
    out += ["\t".join(str(part) for part in row) for row in review_rows]
    kept = []
    for name in sorted(core):
        for rules in protected:
            rule = rules.name_rule(name)
            if rule and rule != name:
                kept.append(f"{name}\t{rules.file}: {rule}\t{','.join(sorted(origin_of[name]))}")
                break
    out += ["", f"## 2. Listed names kept below a protected name ({len(kept)})",
            "# listed name\trule above it\tsources"]
    out += kept
    out += ["", f"## 3. Listed names removed ({len(removed)})",
            "# name\trule\tsources"]
    out += [f"{name}\t{removed[name][0]}\t{','.join(sorted(removed[name][1]))}"
            for name in sorted(removed)]
    return "\n".join(out) + "\n"


def dump_json(value) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=True) + "\n").encode("ascii")


def write_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
    try:
        with os.fdopen(handle, "wb") as out:
            out.write(data)
        os.chmod(temp, 0o644)
        os.replace(temp, path)
    except BaseException:
        if os.path.exists(temp):
            os.unlink(temp)
        raise


def output_path(name: str) -> Path:
    return REVIEW if name == REVIEW.name else DATA / name


def digests(outputs: dict[str, bytes]) -> dict[str, str]:
    return {name: sha256_hex(data) for name, data in sorted(outputs.items())}


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------


def run_build(args) -> int:
    selftest()
    lock = load_lock()
    blobs = fetch_all(lock, args.offline, args.ca_bundle)
    if blobs is None:
        raise BuildError("--offline: the sources are not in the cache")
    outputs, failures, notes = build(lock, blobs)
    if args.digest:
        if failures:
            raise BuildError("gates failed in the second run: " + "; ".join(failures[:5]))
        print(json.dumps(digests(outputs)))
        return 0
    if not failures:
        env = dict(os.environ, PYTHONHASHSEED="20261119")
        if os.environ.get("PYTHONHASHSEED") == "20261119":
            env["PYTHONHASHSEED"] = "7"
        second = subprocess.run(
            [sys.executable, str(Path(__file__).resolve()), "--digest", "--offline"],
            env=env, capture_output=True, text=True, check=False,
        )
        if second.returncode != 0:
            failures.append(f"second run failed: {second.stderr.strip()[-400:]}")
        elif json.loads(second.stdout) != digests(outputs):
            first = digests(outputs)
            differing = [n for n, d in json.loads(second.stdout).items() if first.get(n) != d]
            failures.append(f"second run is not byte-identical: {differing}")
    if failures:
        print(f"{len(failures)} gate failure(s); nothing was written:", file=sys.stderr)
        for line in failures[:200]:
            print("  " + line, file=sys.stderr)
        if len(failures) > 200:
            print(f"  ... and {len(failures) - 200} more", file=sys.stderr)
        return 1
    changed = []
    for name, data in outputs.items():
        path = output_path(name)
        if not path.is_file() or path.read_bytes() != data:
            write_atomic(path, data)
            changed.append(name)
    meta = json.loads(outputs["core.meta.json"])
    print(f"list version   {meta['list_version']}")
    for row in meta["sources"]:
        print(f"source         {row['id']}: {row['unique']} unique names, {row['contributed']} used")
    print(f"core.bin       {meta['core']['count']} keys, {meta['core']['bytes']} bytes")
    print(f"popular.bin    {meta['popular']['count']} keys, {meta['popular']['bytes']} bytes")
    print(f"hosts-tier.txt {meta['hosts_tier']['lines']} lines, {meta['hosts_tier']['bytes']} bytes")
    print(f"review list    {meta['counts']['top10k_hosts_covered']} covered origins in the "
          f"top 10,000: {REVIEW.relative_to(ROOT)}")
    for note in notes:
        print("note: " + note)
    print("all gates passed; second run byte-identical")
    print("written: " + (", ".join(changed) if changed else "nothing (already up to date)"))
    return 0


def run_check(args) -> int:
    """Verify the committed outputs. Needs no network. With the downloads in the
    cache it also rebuilds everything and compares byte for byte."""
    selftest()
    problems: list[str] = []
    lock = load_lock()
    meta = json.loads((DATA / "core.meta.json").read_text(encoding="utf-8"))
    if meta.get("list_version") != lock["list_version"]:
        problems.append("core.meta.json and sources.lock.json disagree on list_version")
    for name in HAND_INPUTS:
        if sha256_hex((DATA / name).read_bytes()) != meta["inputs"].get(name):
            problems.append(f"{name} changed after the last build; run the build again")
    tables = {}
    for name, section, tag in (("core.bin", "core", TAG_CORE), ("popular.bin", "popular", TAG_POPULAR)):
        data = (DATA / name).read_bytes()
        version, keys = read_table(data)
        tables[name] = keys
        if version != lock["list_version"]:
            problems.append(f"{name}: list_version {version}")
        if (len(keys), len(data), sha256_hex(data)) != (
            meta[section]["count"], meta[section]["bytes"], meta[section]["sha256"]
        ):
            problems.append(f"{name}: does not match core.meta.json")
        for vector in meta["vector"]:
            if f"{key_of(vector['tag'].encode('ascii'), vector['name']):016x}" != vector["key"]:
                problems.append(f"vector {vector['name']}: key mismatch")
    core_keys, popular_keys = tables["core.bin"], tables["popular.bin"]
    if not CORE_MIN <= len(core_keys) <= CORE_MAX or (DATA / "core.bin").stat().st_size > CORE_MAX_BYTES:
        problems.append(f"core.bin: {len(core_keys)} keys, out of bounds")
    if (DATA / "popular.bin").stat().st_size > POPULAR_MAX_BYTES:
        problems.append("popular.bin: larger than 1 MB")
    notice = (DATA / "NOTICE.txt").read_bytes()
    if sha256_hex(notice) != meta["notice"]["sha256"]:
        problems.append("NOTICE.txt: does not match core.meta.json")
    if len(copyright_lines(notice.decode("utf-8"))) < 4:
        problems.append("NOTICE.txt: fewer than four copyright lines")
    problems += notices_out_of_date(notice.decode("utf-8"))

    never_block = Rules(DATA / "never-block.txt")
    mixed = Rules(DATA / "mixed.txt")
    never_capture = Rules(DATA / "never-capture.txt")

    def in_table(raw: str) -> str | None:
        name = normalize_host(raw)
        return table_covers(core_keys, TAG_CORE, name) if name else None

    for name in read_names(DATA / "canary-block.txt"):
        if not in_table(name):
            problems.append(f"canary-block: {name} is not covered")
    for name in read_names(DATA / "canary-allow.txt"):
        if in_table(name):
            problems.append(f"canary-allow: {name} is covered")
    for raw, expected in LIST_CASES:
        if bool(in_table(raw)) != expected:
            problems.append(f"design 3.6: {raw} covered={bool(in_table(raw))}")
    for name in NEVER_BLOCK_CASES:
        if not never_block.covers(name):
            problems.append(f"design 3.6: {name} is not under a never-block rule")
    for name in read_names(DATA / "extra-ptbr.txt"):
        if not in_table(name):
            problems.append(f"extra-ptbr: {name} is not covered")
    for rules in (never_block, mixed, never_capture):
        for name in sorted(rules.names):
            if in_table(name):
                problems.append(f"{rules.file}: {name} is covered by the list")

    tier_data = (DATA / "hosts-tier.txt").read_bytes()
    tier = tier_data.decode("ascii").split("\n")
    if tier[-1] != "" or sha256_hex(tier_data) != meta["hosts_tier"]["sha256"]:
        problems.append("hosts-tier.txt: does not match core.meta.json")
    tier = tier[:-1]
    if len(tier) > HOSTS_TIER_CAP or len(set(tier)) != len(tier):
        problems.append(f"hosts-tier.txt: {len(tier)} lines or duplicates")
    for name in tier:
        if not is_hosts_name(name):
            problems.append(f"hosts-tier.txt: invalid name {name!r}")
        elif never_block.covers(name) or mixed.covers(name) or never_capture.covers(name):
            problems.append(f"hosts-tier.txt: {name} is protected by a rule file")
        elif not in_table(name):
            problems.append(f"hosts-tier.txt: {name} is not covered by core.bin")

    blobs = fetch_all(lock, offline=True, ca_bundle=None)
    if blobs is None:
        print("sources not cached: the byte-for-byte rebuild was skipped")
    else:
        outputs, failures, _ = build(lock, blobs)
        problems += failures
        for name, data in outputs.items():
            path = output_path(name)
            if not path.is_file() or path.read_bytes() != data:
                problems.append(f"{name}: differs from a fresh build")
        if not failures:
            print("rebuilt from the cached sources: byte-identical")
    problems = list(dict.fromkeys(problems))
    if problems:
        print(f"{len(problems)} problem(s):", file=sys.stderr)
        for line in problems[:100]:
            print("  " + line, file=sys.stderr)
        return 1
    print(f"check passed: core {len(core_keys)} keys, popular {len(popular_keys)} keys, "
          f"hosts tier {len(tier)} lines, list version {lock['list_version']}")
    return 0


def selftest() -> None:
    """Unit tests of the pure functions. Raises AssertionError on a failure."""
    # Names.
    assert clean_name("Example.COM.") == "example.com"
    for bad in ("localhost", "com", "1.2.3.4", "::1", "a..b", "-a.com", "a_b.com", "x.123",
                "a." + "b" * 64 + ".com", ""):
        assert clean_name(bad) is None, bad
    assert normalize_host("PT.PornHub.com.") == "pt.pornhub.com"
    assert normalize_host("www.xvideos.com:443") == "xvideos.com"
    assert normalize_host("https://user:pw@WWW.Example.com:8443/a?b#c") == "example.com"
    assert normalize_host("[::1]") is None and normalize_host("192.168.0.1") is None
    assert normalize_host("intranet") is None and normalize_host("") is None
    assert list(parents("a.b.example.co.uk")) == [
        "a.b.example.co.uk", "b.example.co.uk", "example.co.uk", "co.uk"]
    assert list(parents("example.com")) == ["example.com"]
    assert is_hosts_name("a.io") and not is_hosts_name("a.b") and not is_hosts_name("A.io")
    # Parsing.
    hosts = "# c\n0.0.0.0 0.0.0.0\n0.0.0.0 A.com b.org # x\n127.0.0.1 localhost\n\nbad\n:: c.net\n"
    assert parse_list(hosts, "hosts")[0] == ["a.com", "b.org", "c.net"]
    assert parse_list("# h\nA.com\n\n1.2.3.4\nb.org extra\n", "plain")[0] == ["a.com", "b.org"]
    assert _STALE.search("x.tumblr.com") and _STALE.search("x.blogspot.com.br")
    assert _STALE.search("a.b.blogspot.co.uk") and not _STALE.search("tumblr.com")
    assert not _STALE.search("blogspot.com") and not _STALE.search("xblogspot.com")
    # Public suffixes.
    psl = Psl(
        "// ===BEGIN ICANN DOMAINS===\ncom\nuk\nco.uk\nbr\ncom.br\n*.ck\n!www.ck\n"
        + "".join(f"t{n}\n" for n in range(5000))
        + "// ===BEGIN PRIVATE DOMAINS===\nblogspot.com\ngithub.io\n"
    )
    assert psl.registrable("a.b.example.co.uk") == "example.co.uk"
    assert psl.registrable("co.uk") is None and psl.registrable("com") is None
    assert psl.registrable("x.blogspot.com") == "x.blogspot.com"
    assert psl.registrable("blogspot.com") is None
    assert psl.registrable("blogspot.com", private=False) == "blogspot.com"
    assert psl.registrable("a.x.github.io", private=False) == "github.io"
    assert psl.registrable("foo.ck") is None and psl.registrable("a.foo.ck") == "a.foo.ck"
    assert psl.registrable("www.ck") == "www.ck" and psl.registrable("x.unknowntld") == "x.unknowntld"
    assert psl.suffix_labels("a.uol.com.br") == 2
    for raw, want in (("www.x.com", "x.com"), ("m.x.com", "x.com"), ("mobile.x.com", "x.com"),
                      ("www.com", "www.com"), ("www.m.x.com", "x.com"), ("wwwx.com", "wwwx.com"),
                      ("m.www.com", "www.com"), ("mobile.co.uk", "mobile.co.uk"),
                      ("www.x.co.uk", "x.co.uk"), ("m.com.br", "m.com.br"),
                      ("www.x.blogspot.com", "x.blogspot.com"), ("www.blogspot.com", "www.blogspot.com")):
        assert strip_prefix(raw, psl) == want, raw
    # Rules.
    with tempfile.TemporaryDirectory() as folder:
        path = Path(folder) / "rules.txt"
        path.write_text("# c\n\ngov\ngov.*\nac.*\njus.br\nfc2.com\nlogin.example.org\n", "utf-8")
        rules = Rules(path)
        assert rules.count == 6
        assert rules.covers("iowasexoffender.gov") == "gov"
        assert rules.covers("essex.gov.uk") == "gov.*" and rules.covers("gov.br") == "gov.*"
        assert rules.covers("a.b.planalto.gov.br") == "gov.*"
        assert rules.covers("sussex.ac.uk") == "ac.*"
        assert rules.covers("x.gov.wales") == "gov.*" and rules.covers("gov.com") == "gov.*"
        assert rules.covers("government.uk") is None and rules.covers("xgov.br") is None
        assert rules.covers("gov.example.br") is None and rules.covers("ac.example.com") is None
        assert rules.covers("tjsp.jus.br") == "jus.br" and rules.covers("jus.br") == "jus.br"
        assert rules.covers("blog.fc2.com") == "fc2.com" and rules.covers("xfc2.com") is None
        assert rules.removes("fc2.com") == "fc2.com" and rules.removes("adult.contents.fc2.com") is None
        assert rules.removes("x.essex.gov.uk") == "gov.*" and rules.removes("example.org") is None
        assert rules.covers("example.org") is None
        for bad in ("Gov\n", "a b.com\n", "*.gov\n", "gov.*.br\n", " gov\n", "gov\ngov\n", "x.com # c\n",
                    "gov.co.*\n", ".*\n"):
            path.write_text(bad, "utf-8")
            try:
                Rules(path)
            except BuildError:
                continue
            raise AssertionError(f"rule file accepted {bad!r}")
    # Keys and tables.
    assert key_of(TAG_CORE, "example.com") == int.from_bytes(
        hashlib.sha256(b"ubiqx-nofap-blocklist-v1\x00example.com").digest()[:8], "big")
    assert key_of(TAG_CORE, "example.com") != key_of(TAG_POPULAR, "example.com")
    keys = sorted(key_of(TAG_CORE, name) for name in ("listed.example", "deep.sub.example.org"))
    table = write_table(keys, 202601020304)
    assert len(table) == 24 + 16 and table[:8] == b"UBXBL1\x00\x00"
    assert table[8:16] == bytes([0, 1, 0, 0, 0, 0, 0, 2])
    assert int.from_bytes(table[16:24], "big") == 202601020304
    assert read_table(table) == (202601020304, keys)
    assert table_covers(keys, TAG_CORE, "a.b.listed.example") == "listed.example"
    assert table_covers(keys, TAG_CORE, "sub.example.org") is None
    assert table_covers(keys, TAG_CORE, "x.deep.sub.example.org") == "deep.sub.example.org"
    assert table_covers(keys, TAG_CORE, "example") is None
    for broken in (b"", table[:-1], b"X" + table[1:], table[:24] + table[32:] + table[24:32],
                   table[:24] + table[24:32] * 2, table[:15] + b"\x03" + table[16:],
                   table[:9] + b"\x02" + table[10:], table[:11] + b"\x01" + table[12:]):
        try:
            read_table(broken)
        except BuildError:
            continue
        raise AssertionError("a malformed table was accepted")
    try:
        write_table([2, 1], 1)
    except BuildError:
        pass
    else:
        raise AssertionError("unsorted keys were accepted")
    # Suffix de-duplication.
    assert suffix_dedup({"a.com", "x.a.com", "y.x.a.com", "b.co.uk", "co.uk.b"}) == {
        "a.com", "b.co.uk", "co.uk.b"}
    # Keywords.
    assert block_grade("x.porn", 1) == "tld:porn" and block_grade("pornic.fr", 1) == "word:porn"
    assert block_grade("m1ssav.ws", 1) == "brand:missav" and block_grade("x0nlyfans7.to", 1)
    assert block_grade("sex.example.com", 1) is None and block_grade("camara.leg.br", 2) is None
    assert block_grade("porn.example", 2) is None
    assert fold_lookalikes("123xvv1d30s9") == "xwideos"
    index = {"porn-only": {"sourcesdata": [
        {"homeurl": url, "license": licence} for url, licence in STEVENBLACK_UPSTREAMS.items()]}}
    assert stevenblack_upstreams_changed(json.dumps(index).encode()) == []
    index["porn-only"]["sourcesdata"].append({"homeurl": "https://example.org/x", "license": "GPL-3.0"})
    assert len(stevenblack_upstreams_changed(json.dumps(index).encode())) == 1
    assert len(stevenblack_upstreams_changed(b"{}")) == 1
    assert copyright_lines(
        "MIT\n\n Copyright (c) 2016 X \nCopyright \u00a9 2026 Y\nCOPYRIGHT HOLDERS BE LIABLE\n"
    ) == ["Copyright (c) 2016 X", "Copyright \u00a9 2026 Y"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="verify the committed outputs (no network)")
    mode.add_argument("--selftest", action="store_true", help="run the unit tests of this script")
    mode.add_argument("--pin", action="store_true", help="re-pin every source at its upstream HEAD")
    parser.add_argument("--offline", action="store_true", help="never use the network")
    parser.add_argument("--ca-bundle", help="CA certificates for HTTPS (default: the system's; "
                        "SSL_CERT_FILE is honoured)")
    parser.add_argument("--list-version", type=int, help="with --pin: YYYYMMDDHHMM in UTC "
                        "(default: now)")
    parser.add_argument("--digest", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    try:
        if args.selftest:
            selftest()
            print("selftest passed")
            return 0
        if args.check:
            return run_check(args)
        if args.pin:
            import datetime
            now = int(datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%d%H%M"))
            pin(args.ca_bundle, args.list_version or now)
            return 0
        return run_build(args)
    except BuildError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
