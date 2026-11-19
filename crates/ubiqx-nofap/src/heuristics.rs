//! Keyword and TLD rules, applied only to hosts that no list, user entry or never-block
//! rule decides (design section 3.5).
//!
//! | Grade | Effect |
//! |---|---|
//! | Block | an adult TLD, a block-grade word where it stands as a word, or a brand stem |
//! | Signal | a signal-grade whole token or TLD, or a block-grade word in a place that does not prove it: raises vision priority, never blocks |
//! | Never | words that are not used at all (`sex`, `anal`, `gay`, `teen`, …) |
//!
//! A plain substring test blocks ordinary names: `putas` is in "disputas", `porn` in the
//! town of Pornichet, in the Polish "odporność" and in Thai given names, `xxx` in roman
//! numerals and clothing sizes, `erome` in "aeromexico"; "cabin porn" is about cabins and
//! "porn free" about quitting. So every word carries the place
//! where it counts, and the rule is the same everywhere: **when the letters around a word
//! do not tell, it does not block**; the screen check is the layer for doubt, and the
//! word then only raises its priority.
//!
//! Vocabulary: a host is split into *labels* at dots, and a label into *tokens*, the runs
//! of letters between hyphens and digits (`porno365` holds the token `porno`). The *tail*
//! of a word is what follows it inside its token.
//!
//! * Long, unambiguous words (`hentai`, `putaria`, …) count anywhere in a label.
//! * `porn` and `xxx` count when the rest of their token reads as the words adult sites
//!   are named with: nothing or [`HEAD_WORDS`] before, nothing or [`SITE_WORDS`] after.
//!   `freeporn`, `pornovideos` and `xxxtube` block; `pornichet`, `odpornosc`, `siriporn`,
//!   `cabinporn`, `quitporn`, `terminalxxxix` and `mixxx` do not.
//! * Brand stems are matched on the name as written and again after look-alike folding
//!   ([`fold_lookalikes`]) with repeated letters collapsed, so `xvv1deos` is `xvideos`.
//!   A stem with a digit is matched as written only: folded, `cam4` would be "cama".
//! * Stems that are also ordinary letter sequences count only as a whole label (`erome`,
//!   `cam4`) or at the start of a token (`xvideos`, `redtube`, `hanime`: "maxvideos",
//!   "infraredtube" and "watchanime" are ordinary names).
//!
//! The build script filters one public list with the plain substring form of these words
//! (`scripts/nofap/build_blocklist.py`, `block_grade`). That is deliberate: a name on an
//! adult-site list that carries such a word needs no further proof; a name seen in a
//! browser does.

use crate::matcher::BlockSource;

/// Top-level domains that block.
pub const BLOCK_TLDS: &[&str] = &["xxx", "porn", "sex", "sexy", "adult"];

/// Signal-grade whole tokens: they raise vision priority and never block. `pussy` counts
/// only when it is not followed by digits.
pub const SIGNAL_TOKENS: &[&str] = &[
    "milf",
    "sexy",
    "adult",
    "escort",
    "naked",
    "nude",
    "pussy",
    "sexo",
    "bdsm",
    "webcam",
    "cams",
    "erotic",
    "erotica",
    "fetish",
    "voyeur",
    "fuck",
    "amadoras",
    "acompanhantes",
    // Block-grade words and brand names that, alone, are also ordinary names.
    "escorts",
    "kemono",
    "coomer",
];

/// Words that are a signal wherever they stand in a label and never block: each is also an
/// ordinary word (a cream pie is a dessert, "peladas" are street football matches in Brazil
/// and "gozando" is Spanish for "enjoying").
pub const SIGNAL_SUBSTRINGS: &[&str] = &["creampie", "peladas", "gozando"];

/// Signal-grade top-level domains.
pub const SIGNAL_TLDS: &[&str] = &["cam", "webcam", "tube"];

/// Words that are not used at all, neither to block nor as a signal. `gay` is never a
/// signal: an orientation is not a risk marker.
pub const NEVER_WORDS: &[&str] = &[
    "sex", "anal", "cum", "ass", "cock", "dick", "tits", "cam", "jav", "gay", "teen", "incest",
    "pau", "rola", "pica", "puta", "bunda", "pelada", "foda",
];

/// The words adult sites are named with. A tail that reads as these words confirms `porn`
/// and `xxx`; any other tail leaves the word a signal. "free" is deliberately absent:
/// "porn free" is what people who quit call themselves.
#[rustfmt::skip]
pub const SITE_WORDS: &[&str] = &[
    // What the site offers.
    "video", "videos", "videoz", "vid", "vids", "vidz", "tube", "tubes", "tub", "movie", "movies",
    "movs", "mov", "film", "films", "filme", "filmes", "filmy", "kino", "clip", "clips", "pic",
    "pics", "picture", "pictures", "photo", "photos", "foto", "fotos", "image", "images", "gif",
    "gifs", "gallery", "galleries", "collection", "comic", "comics", "comix", "game", "games",
    "story", "stories", "chat", "cam", "cams", "live", "stream", "show", "tv", "hd", "hq", "vr",
    "ai", "bot", "generator", "torrent", "download", "downloader", "file", "files", "load",
    "leak", "leaks", "archive", "search", "finder", "list", "directory", "link", "links",
    "review", "reviews", "guide", "news", "forum", "blog", "izle", "bilder",
    // What the site calls itself.
    "hub", "hubs", "site", "sites", "seiten", "web", "net", "org", "com", "app", "apps", "club",
    "zone", "world", "land", "city", "planet", "house", "home", "room", "space", "box", "base",
    "lab", "labs", "cloud", "page", "store", "shop", "pass", "access", "proxy", "flix", "king",
    "guru", "master", "dude", "mania", "party", "life", "trends", "deals", "discounts",
    "premium", "plus", "vip", "pro", "max", "mix", "top", "best", "new", "now", "today", "daily",
    "time", "here", "online", "mobile", "media", "gratis", "gratuit", "gratuito", "kostenlos",
    "besplatno", "full", "hits", "play", "fun", "fan", "fans", "love", "hot", "classic", "one",
    "go", "up", "ok", "x", "z",
    // What it shows.
    "porn", "porno", "xxx", "sex", "sexy", "fuck", "anal", "milf", "teen", "teens", "mature",
    "girl", "girls", "babes", "sluts", "pussy", "star", "stars", "gay", "lesbian", "shemale",
    "hentai", "anime", "cartoon", "amateur", "amador", "caseiro", "legendado", "wife", "incest",
    "zoo", "cum", "mms", "bluefilm",
    // Where it is from.
    "asian", "indian", "india", "desi", "hindi", "tamil", "arab", "japan", "viet", "phim",
    "ebony", "black", "latina", "latino", "brasil", "brasileiro", "tuga", "italiani", "francais",
    "deutsch", "rus", "russkoe",
    // Brands that are built on the word.
    "hat", "hwa", "xp", "dig", "db", "sub", "trex", "pornhub", "xvideos", "xnxx", "xhamster",
];

/// The words that stand before `porn` and `xxx` in the names of adult sites. A head made of
/// them confirms the word; any other head leaves it a signal ("cabinporn", "foodporn" and
/// "bikeporn" are about cabins, food and bicycles; "quitporn" and "noporn" are about
/// stopping).
#[rustfmt::skip]
pub const HEAD_WORDS: &[&str] = &[
    // Generic.
    "free", "hd", "hq", "best", "top", "new", "hot", "full", "real", "only", "just", "all", "any",
    "my", "your", "you", "the", "its", "mr", "go", "get", "try", "find", "save", "more", "one",
    "yes", "wow", "ok", "good", "great", "nice", "cool", "sweet", "crazy", "awesome", "pure",
    "super", "mega", "ultra", "epic", "elite", "prime", "gold", "pro", "max", "vip", "big",
    "daily", "fresh", "premium", "private", "gratis", "kostenlose", "ingyen", "online", "mobile",
    "live", "watch", "smotret", "skachat", "download", "play", "stream", "tube", "hub", "tv",
    "dvd", "app", "www", "video", "videos", "movies", "pics", "film", "filme", "filmes", "cam",
    "webcam", "virtual", "vr", "ai", "deepfake", "leak", "leaked",
    // What it shows.
    "sex", "sexy", "seks", "xxx", "porn", "porno", "adult", "erotic", "fuck", "anal", "gay",
    "twink", "boy", "teen", "teens", "young", "old", "mature", "milf", "milfs", "mom", "moms",
    "granny", "wife", "cuckold", "amateur", "amature", "homemade", "domaci", "reality", "pov",
    "public", "solo", "lesbian", "shemale", "tranny", "trans", "bbw", "fat", "chubby", "hairy",
    "tits", "bigtits", "bigass", "feet", "naked", "nude", "horny", "nasty", "geile", "babe",
    "girl", "girls", "pussy", "creampie", "massage", "interracial", "hentai", "anime", "toon",
    "cartoon", "comics", "cosplay", "furry", "bdsm", "bondage", "femdom", "fetish", "voyeur",
    "incest", "taboo", "family", "forced", "rape", "scat", "extreme", "zoo", "animal", "hard",
    "hardcore", "dirty", "vintage", "retro", "classic", "jav", "av",
    // Where it is from.
    "asian", "asians", "asia", "indian", "india", "desi", "hindi", "tamil", "bangla", "mallu",
    "pakistani", "pinay", "indo", "malay", "arab", "arabic", "african", "japan", "japanese",
    "tokyo", "china", "chinese", "korean", "thai", "viet", "rus", "russian", "russkoe", "novoe",
    "prosto", "turk", "uzbek", "azeri", "magyar", "deutsch", "deutsche", "euro", "ebony",
    "black", "latina", "brasil",
    // Brands.
    "xnxx", "xvideos", "xhamster",
];

/// Single letters that stand alone before `porn` and `xxx` (`eporn`, `xporn`). Never `s`:
/// "Sporn" is a German spur.
const HEAD_LETTERS: &[u8] = b"adeikuvx";

/// A head or a tail longer than this is never read as words.
const MAX_AFFIX: usize = 40;

/// What must stand before a word.
#[derive(Debug, Clone, Copy)]
enum Before {
    Any,
    /// The word starts its token.
    TokenStart,
    /// The word starts its token or directly follows one of these words.
    TokenStartOrAfter(&'static [&'static str]),
    /// The word starts its label.
    LabelStart,
    /// The head is empty or made of [`HEAD_WORDS`].
    HeadWords,
}

/// What must stand after a word.
#[derive(Debug, Clone, Copy)]
enum After {
    Any,
    /// The word ends its token.
    TokenEnd,
    /// The tail is empty or reads as [`SITE_WORDS`], after one optional linking ending
    /// (`porn-o-videos`, `porn-os`).
    SiteWords(&'static [&'static str]),
    /// The word ends its label.
    LabelEnd,
}

/// A block-grade word or brand stem and the place where it counts.
#[derive(Debug, Clone, Copy)]
struct Rule {
    word: &'static str,
    before: Before,
    after: After,
    /// The match may not be continued by the same letter on either side (`xxx` in `xxxx`).
    exact_run: bool,
    /// Tokens that are ordinary words: the rule ignores them altogether.
    innocent_tokens: &'static [&'static str],
    /// Token beginnings that are ordinary words.
    innocent_starts: &'static [&'static str],
    /// A match in a place that does not block is still a signal.
    weak_signal: bool,
    /// Also matched after look-alike folding (brand stems made of letters only).
    folded: bool,
}

impl Rule {
    const fn anywhere(word: &'static str) -> Self {
        Self {
            word,
            before: Before::Any,
            after: After::Any,
            exact_run: false,
            innocent_tokens: &[],
            innocent_starts: &[],
            weak_signal: false,
            folded: false,
        }
    }

    const fn placed(word: &'static str, before: Before, after: After) -> Self {
        Self {
            before,
            after,
            ..Self::anywhere(word)
        }
    }

    const fn weak_signal(self) -> Self {
        Self {
            weak_signal: true,
            ..self
        }
    }

    const fn folded(self) -> Self {
        Self {
            folded: true,
            ..self
        }
    }
}

/// Block-grade words (design 3.5) with the place where each counts.
const BLOCK_WORDS: &[Rule] = &[
    Rule::placed(
        "porn",
        Before::HeadWords,
        After::SiteWords(&["o", "os", "s"]),
    )
    .weak_signal(),
    Rule {
        exact_run: true,
        ..Rule::placed("xxx", Before::HeadWords, After::SiteWords(&[]))
    }
    .weak_signal(),
    Rule::anywhere("hentai"),
    Rule::anywhere("blowjob"),
    Rule::anywhere("gangbang"),
    Rule::anywhere("cumshot"),
    Rule::anywhere("handjob"),
    Rule::anywhere("bukkake"),
    Rule::anywhere("deepthroat"),
    Rule::anywhere("camgirl"),
    Rule::anywhere("shemale"),
    Rule::anywhere("upskirt"),
    Rule::anywhere("masturbat"),
    Rule::anywhere("bokep"),
    Rule::anywhere("phimsex"),
    // "denudes" is an English verb.
    Rule {
        innocent_starts: &["denudes"],
        ..Rule::anywhere("nudes")
    },
    // "disputas", "computas", "imputas" and the Latvian "atputas" are ordinary words.
    Rule::placed(
        "putas",
        Before::TokenStartOrAfter(&["de", "las", "as"]),
        After::Any,
    ),
    // A token that is the word alone is also a tractor maker and a car model: it is a
    // signal token instead.
    Rule {
        innocent_tokens: &["escorts"],
        ..Rule::anywhere("escorts")
    },
    // PT-BR
    Rule::anywhere("putaria"),
    Rule::anywhere("buceta"),
    Rule::anywhere("bucetinha"),
    Rule::anywhere("siririca"),
    Rule::anywhere("contoseroticos"),
    Rule::anywhere("brasileirinhas"),
    Rule::anywhere("pornozao"),
    Rule::anywhere("safadas"),
    Rule::anywhere("novinhas"),
    // "receitas gostosas" are tasty recipes: the adjective follows its noun.
    Rule::placed("gostosas", Before::LabelStart, After::Any).weak_signal(),
    Rule::anywhere("transando"),
];

/// Brand stems of `research-blocklists.md` 9.1. Of its 32, `e-hentai` is covered by
/// `hentai`, and `kemono` and `coomer` are signal tokens: as whole labels they are also a
/// webcomic community and a cooperative, and nothing in a name tells which.
const BRAND_STEMS: &[Rule] = &[
    Rule::anywhere("pornhub").folded(),
    Rule::placed("xvideos", Before::TokenStart, After::Any).folded(),
    Rule::anywhere("xnxx"),
    Rule::anywhere("xhamster").folded(),
    Rule::placed("redtube", Before::TokenStart, After::Any).folded(),
    Rule::anywhere("youporn").folded(),
    Rule::anywhere("spankbang").folded(),
    Rule::anywhere("eporner").folded(),
    Rule::anywhere("chaturbate").folded(),
    Rule::anywhere("stripchat").folded(),
    Rule::anywhere("bongacams").folded(),
    Rule::anywhere("livejasmin").folded(),
    Rule::placed("cam4", Before::LabelStart, After::LabelEnd),
    Rule::anywhere("myfreecams").folded(),
    Rule::anywhere("onlyfans").folded(),
    // "fanslyrics" are lyrics.
    Rule::placed("fansly", Before::Any, After::SiteWords(&[])),
    Rule::anywhere("brazzers").folded(),
    // "missavery" and "missavenue" are ordinary names.
    Rule::placed("missav", Before::Any, After::TokenEnd),
    Rule::anywhere("nhentai").folded(),
    Rule::placed("hanime", Before::TokenStart, After::Any).folded(),
    Rule::anywhere("rule34"),
    Rule::anywhere("fapello").folded(),
    Rule::placed("erome", Before::LabelStart, After::LabelEnd),
    Rule::anywhere("simpcity").folded(),
    Rule::anywhere("noodlemagazine").folded(),
    Rule::anywhere("sxyprn"),
    Rule::anywhere("faphouse").folded(),
    Rule::anywhere("javhd").folded(),
    Rule::anywhere("supjav").folded(),
];

/// What the rules say about a host that nothing else decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostRule {
    /// [`BlockSource::Tld`] or [`BlockSource::Keyword`].
    Block(BlockSource),
    /// A signal-grade word or TLD.
    Signal,
    None,
}

/// What one piece of text (a label, a word of a title, a segment of a path) carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Grade {
    None,
    Signal,
    Block,
}

/// The rule for a normalised host.
pub fn host_rule(host: &str) -> HostRule {
    let host = host.to_ascii_lowercase();
    let mut labels: Vec<&str> = host.split('.').filter(|label| !label.is_empty()).collect();
    let tld = if labels.len() >= 2 {
        labels.pop()
    } else {
        None
    };
    if tld.is_some_and(|tld| BLOCK_TLDS.contains(&tld)) {
        return HostRule::Block(BlockSource::Tld);
    }
    let mut grade = Grade::None;
    for label in &labels {
        grade = grade.max(grade_of(label.as_bytes()));
        if grade == Grade::Block {
            return HostRule::Block(BlockSource::Keyword);
        }
    }
    if grade == Grade::Signal || tld.is_some_and(|tld| SIGNAL_TLDS.contains(&tld)) {
        HostRule::Signal
    } else {
        HostRule::None
    }
}

/// Whether a window title carries a block-grade word (a signal for the AI check). A signal
/// costs one look, so the word counts wherever it stands: "EarthPorn" is a signal here and
/// would not be a block in a host.
pub fn title_signal(title: &str) -> bool {
    text_has_block_word(title)
}

/// Whether a URL path (with or without its query) carries a block-grade word (a signal on a
/// mixed host, never a block). Read like a title.
pub fn path_signal(path: &str) -> bool {
    text_has_block_word(path)
}

/// Look-alike folding for brand stems: `0→o 1→i 3→e 4→a 5→s 7→t vv→w`, digits at the edges
/// of a label stripped.
pub fn fold_lookalikes(label: &str) -> String {
    letters_for_digits(label.trim_matches(|c: char| c.is_ascii_digit())).replace("vv", "w")
}

/// The digit half of [`fold_lookalikes`]: `0→o 1→i 3→e 4→a 5→s 7→t`.
fn letters_for_digits(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '0' => 'o',
            '1' => 'i',
            '3' => 'e',
            '4' => 'a',
            '5' => 's',
            '7' => 't',
            other => other,
        })
        .collect()
}

/// `text` with every run of one repeated letter reduced to a single letter.
fn collapse_repeats(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if !(c.is_ascii_lowercase() && out.ends_with(c)) {
            out.push(c);
        }
    }
    out
}

/// Free text (a title, a path) is lower-cased, the accented letters of Portuguese and
/// Spanish lose their accent, and every other character separates words.
fn text_has_block_word(text: &str) -> bool {
    let plain: String = text
        .chars()
        .flat_map(char::to_lowercase)
        .map(unaccent)
        .collect();
    plain
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
        .filter(|piece| !piece.is_empty())
        .any(|piece| word_grade(piece.as_bytes()) != Grade::None)
}

fn unaccent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        other => other,
    }
}

/// The grade of one label of a host (lower-case ASCII letters, digits and hyphens).
fn grade_of(label: &[u8]) -> Grade {
    match word_grade(label) {
        Grade::None if has_signal_token(label) => Grade::Signal,
        grade => grade,
    }
}

/// What the block-grade words say about one label or one word of free text: a block where
/// a word or a brand stem stands as the rules ask, a signal where a word stands in a place
/// that does not prove it.
fn word_grade(label: &[u8]) -> Grade {
    let mut grade = Grade::None;
    for rule in BLOCK_WORDS {
        grade = grade.max(rule.grade_in(label, rule.word.as_bytes()));
        if grade == Grade::Block {
            return grade;
        }
    }
    if brand_stem_in(label) {
        return Grade::Block;
    }
    if SIGNAL_SUBSTRINGS
        .iter()
        .any(|word| find(label, word.as_bytes(), 0).is_some())
    {
        return Grade::Signal;
    }
    grade
}

/// Whether a brand stem stands in the label, as written or as a look-alike. The look-alike
/// forms are the folded label, and the label with its digits read as letters (edge digits
/// kept, so `0nlyfans` reads "onlyfans", and stripped) and repeated letters collapsed, so
/// `xvv1deos` and `brazzzers` read as their brands.
fn brand_stem_in(label: &[u8]) -> bool {
    if BRAND_STEMS
        .iter()
        .any(|rule| rule.grade_in(label, rule.word.as_bytes()) == Grade::Block)
    {
        return true;
    }
    let Ok(text) = std::str::from_utf8(label) else {
        return false;
    };
    if !text.bytes().any(|b| b.is_ascii_digit()) && collapse_repeats(text) == text {
        // No digit and no repeated letter: every look-alike form is the label itself.
        return false;
    }
    let folded = fold_lookalikes(text);
    let whole = collapse_repeats(&letters_for_digits(text));
    let trimmed = collapse_repeats(&letters_for_digits(
        text.trim_matches(|c: char| c.is_ascii_digit()),
    ));
    BRAND_STEMS.iter().filter(|rule| rule.folded).any(|rule| {
        let collapsed = collapse_repeats(rule.word);
        rule.grade_in(folded.as_bytes(), rule.word.as_bytes()) == Grade::Block
            || rule.grade_in(whole.as_bytes(), collapsed.as_bytes()) == Grade::Block
            || rule.grade_in(trimmed.as_bytes(), collapsed.as_bytes()) == Grade::Block
    })
}

/// Whether a token of the label is one of [`SIGNAL_TOKENS`].
fn has_signal_token(label: &[u8]) -> bool {
    let mut start = 0;
    while start < label.len() {
        if !label[start].is_ascii_lowercase() {
            start += 1;
            continue;
        }
        let end = token_end(label, start);
        let token = &label[start..end];
        let followed_by_digit = label.get(end).is_some_and(u8::is_ascii_digit);
        if SIGNAL_TOKENS.iter().any(|word| word.as_bytes() == token)
            && !(token == b"pussy" && followed_by_digit)
        {
            return true;
        }
        start = end;
    }
    false
}

impl Rule {
    /// The grade `needle` (this rule's word, or its collapsed form) earns in `label`.
    fn grade_in(&self, label: &[u8], needle: &[u8]) -> Grade {
        let mut grade = Grade::None;
        let mut from = 0;
        while let Some(at) = find(label, needle, from) {
            from = at + 1;
            let end = at + needle.len();
            let token_from = token_start(label, at);
            let token_to = token_end(label, end);
            let token = &label[token_from..token_to];
            if self
                .innocent_tokens
                .iter()
                .any(|word| word.as_bytes() == token)
                || self
                    .innocent_starts
                    .iter()
                    .any(|word| token.starts_with(word.as_bytes()))
            {
                continue;
            }
            let head = &label[token_from..at];
            let tail = &label[end..token_to];
            let run_broken = self.exact_run
                && (label[..at].ends_with(&needle[..1])
                    || label[end..].starts_with(&needle[needle.len() - 1..]));
            let before = match self.before {
                Before::Any => true,
                Before::TokenStart => head.is_empty(),
                Before::TokenStartOrAfter(words) => {
                    head.is_empty() || words.iter().any(|word| head.ends_with(word.as_bytes()))
                }
                Before::LabelStart => at == 0,
                Before::HeadWords => head_is_site_words(head),
            };
            let after = match self.after {
                After::Any => true,
                After::TokenEnd => tail.is_empty(),
                After::SiteWords(links) => tail_is_site_words(tail, links),
                After::LabelEnd => end == label.len(),
            };
            if before && after && !run_broken {
                return Grade::Block;
            }
            if self.weak_signal {
                grade = Grade::Signal;
            }
        }
        grade
    }
}

/// The first position of `needle` in `haystack` at or after `from`.
fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&at| haystack[at..].starts_with(needle))
}

/// Where the run of letters around `at` begins.
fn token_start(label: &[u8], at: usize) -> usize {
    let mut start = at;
    while start > 0 && label[start - 1].is_ascii_lowercase() {
        start -= 1;
    }
    start
}

/// Where the run of letters that reaches `at` ends.
fn token_end(label: &[u8], at: usize) -> usize {
    let mut end = at;
    while end < label.len() && label[end].is_ascii_lowercase() {
        end += 1;
    }
    end
}

/// Whether `tail` is empty or reads as site words after one optional linking ending.
fn tail_is_site_words(tail: &[u8], links: &[&str]) -> bool {
    reads_as_words(tail, SITE_WORDS, true)
        || links.iter().any(|link| {
            tail.strip_prefix(link.as_bytes())
                .is_some_and(|rest| reads_as_words(rest, SITE_WORDS, true))
        })
}

/// Whether `head` is empty, one of [`HEAD_LETTERS`], or made of [`HEAD_WORDS`].
fn head_is_site_words(head: &[u8]) -> bool {
    match head {
        [letter] => HEAD_LETTERS.contains(letter),
        _ => reads_as_words(head, HEAD_WORDS, false),
    }
}

/// Whether `text` is empty or a chain of `words`. With `open_end`, a word of four letters
/// or more ends the reading wherever it stands ("videosgratuitos" reads as "videos"); a
/// shorter one must be followed by another word: "ai" does not confirm "pornainen".
/// One pass from the end, so a long run of letters costs no more than a short one.
fn reads_as_words(text: &[u8], words: &[&str], open_end: bool) -> bool {
    if text.len() > MAX_AFFIX {
        return false;
    }
    let mut reads = [false; MAX_AFFIX + 1];
    reads[text.len()] = true;
    for at in (0..text.len()).rev() {
        reads[at] = words.iter().any(|word| {
            text[at..].starts_with(word.as_bytes())
                && ((open_end && word.len() >= 4) || reads[at + word.len()])
        });
    }
    reads[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYWORD: HostRule = HostRule::Block(BlockSource::Keyword);
    const TLD: HostRule = HostRule::Block(BlockSource::Tld);

    fn blocks(host: &str) -> bool {
        matches!(host_rule(host), HostRule::Block(_))
    }

    #[test]
    fn block_rows_of_the_design() {
        assert_eq!(host_rule("porno365.golf"), KEYWORD);
        assert_eq!(host_rule("missav123.com"), KEYWORD);
        assert_eq!(host_rule("xvv1deos.com"), KEYWORD);
        assert_eq!(host_rule("videosdeputaria.com.br"), KEYWORD);
        assert_eq!(host_rule("anything.xxx"), TLD);
    }

    #[test]
    fn false_positive_traps_of_the_design_do_not_block() {
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
            assert!(!blocks(host), "{host}: {:?}", host_rule(host));
        }
        // And most of them carry no signal either.
        for host in [
            "emilfrey.de",
            "nakedwines.com",
            "skylinewebcams.com",
            "pussy888.com",
        ] {
            assert_eq!(host_rule(host), HostRule::None, "{host}");
        }
    }

    /// Integrator decision 12: every ordinary name the hand-offs of the foundation and
    /// blocklist units mention, and the ones found while these rules were tuned against
    /// the Chrome UX Report hosts that are on no list.
    #[test]
    fn ordinary_names_do_not_block() {
        for host in [
            // `putas`
            "disputas.com.br",
            "resolucaodedisputas.com.br",
            "computas.com",
            "atputasvakars.lv",
            // `nudes`, `peladas`
            "denudes.com",
            "apeladas.es",
            "peladasdobairro.com.br",
            // `porn`: a town, Slavic and Spanish words, Thai names, a German word, the
            // "porn" of enthusiasts and the "porn" of people who quit
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
            // `xxx`: roman numerals, sizes, brands
            "terminalxxxix.com.br",
            "superbowlxxxiv.com",
            "xxxl-shop.de",
            "xxxlutz.de",
            "mixxx.org",
            "xxxx.com.au",
            "xxxtentacion.com",
            // brand stems
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
            // words that moved to the signal grade
            "bostoncreampie.com",
            "gozandolavida.com",
            "receitasgostosas.com.br",
            "receitas-gostosas.com.br",
            // the tractor maker and the car
            "escorts.co.in",
            "erp.escorts.co.in",
            "ford-escorts.co.uk",
        ] {
            assert!(!blocks(host), "{host}: {:?}", host_rule(host));
        }
    }

    #[test]
    fn a_block_word_in_an_unproven_place_is_a_signal() {
        for host in [
            "pornichet.fr",
            "cabinporn.com",
            "pornfree.org",
            "xxxlutz.de",
            "xxxx.com.au",
            "bostoncreampie.com",
            "peladasdobairro.com.br",
            "gozandolavida.com",
            "receitasgostosas.com.br",
            "ford-escorts.co.uk",
            "kemono.cafe",
            "coomer.com.co",
        ] {
            assert_eq!(host_rule(host), HostRule::Signal, "{host}");
        }
        // Collisions that are plain words carry nothing at all.
        for host in [
            "disputas.com.br",
            "denudes.com",
            "aeromexico.com",
            "maxvideos.com",
            "watchanimeworld.one",
            "missavery.com",
            "fanslyrics.com",
            "dashcam4k.com",
        ] {
            assert_eq!(host_rule(host), HostRule::None, "{host}");
        }
    }

    #[test]
    fn porn_and_xxx_block_where_the_token_reads_as_site_words() {
        for host in [
            "porn.example.com",
            "porno.example",
            "pornos.example",
            "pornovideos.example",
            "pornosgratis.example",
            "freeporn.example",
            "bestfreeporn.example",
            "hdpornvideos.example",
            "gayporn.example",
            "eporn.example",
            "free-porn-videos.example",
            "porn4k.example",
            "24porn.example",
            "pornstars.example",
            "porntube.example",
            "pornhd.example",
            "pornxp.example",
            "xxx.example.com",
            "xxxvideos.example",
            "xxx-tube.example",
            "desixxx.example",
            "videosxxx.example",
            "xxx18.example",
            "sexxxx.example.xxx",
        ] {
            assert!(blocks(host), "{host}: {:?}", host_rule(host));
        }
        // Four letters in a row are not the word.
        assert_eq!(host_rule("xxxx.example"), HostRule::Signal);
        assert_eq!(host_rule("videoxxxx.example"), HostRule::Signal);
    }

    #[test]
    fn every_block_word_blocks_in_a_plain_name() {
        for host in [
            "hentaiworld.example",
            "myhentai.example",
            "blowjobs.example",
            "gangbangs.example",
            "cumshots.example",
            "handjobs.example",
            "bukkake.example",
            "deepthroat.example",
            "camgirls.example",
            "shemales.example",
            "upskirts.example",
            "masturbation.example",
            "bokepindo.example",
            "phimsexhay.example",
            "nudes.example",
            "darknudes.example",
            "putas.example",
            "putaslocas.example",
            "videosdeputas.example",
            "cityescorts.example",
            "escortsinlondon.example",
            "putaria.example",
            "bucetas.example",
            "bucetinhas.example",
            "siririca.example",
            "contoseroticos.example",
            "brasileirinhas.example",
            "pornozao.example",
            "safadas.example",
            "novinhas.example",
            "gostosas.example",
            "gostosasdozap.example",
            "transando.example",
        ] {
            assert_eq!(host_rule(host), KEYWORD, "{host}");
        }
    }

    #[test]
    fn every_brand_stem_blocks_as_its_own_site() {
        for rule in BRAND_STEMS {
            for host in [format!("{}.com", rule.word), format!("pt.{}.tv", rule.word)] {
                assert_eq!(host_rule(&host), KEYWORD, "{host}");
            }
        }
        assert_eq!(BRAND_STEMS.len(), 29);
    }

    #[test]
    fn brand_stems_block_as_look_alikes_and_mirrors() {
        for host in [
            "p0rnhub.example",
            "pornhubb.example",
            "xv1de0s.example",
            "xvv1deos.example",
            "xvideos2.example",
            "br-xvideos.example",
            "xnxx2.example",
            "freexnxx.example",
            "xhamster19.example",
            "xhamsterr.example",
            "redtube2.example",
            "y0uporn.example",
            "ch4turb4te.example",
            "0nlyfans.example",
            "onlyfansleaks.example",
            "brazzzers.example",
            "missav.example",
            "missav-tv.example",
            "hanime1.example",
            "rule34video.example",
            "fansly.example",
            "fanslyleaks.example",
            "erome.example",
            "cam4.example",
            "pt.cam4.example",
            "javhdporn.example",
            "5pankbang.example",
        ] {
            assert_eq!(host_rule(host), KEYWORD, "{host}");
        }
    }

    #[test]
    fn signal_words_are_whole_tokens() {
        for host in [
            "milf.example",
            "sexy-hair.example",
            "adult.example.com",
            "escort-advisor.example",
            "naked-science.example",
            "nude-project.example",
            "pussy.example",
            "pussy-cats.example",
            "sexo.example",
            "bdsm-test.example",
            "webcam.example",
            "cams.example",
            "erotic-art.example",
            "erotica.example",
            "fetish.example",
            "voyeur-house.example",
            "fuck-me.example",
            "amadoras10.example",
            "acompanhantes.example",
            "kemono.example",
            "coomer.example",
            "example.cam",
            "example.webcam",
            "example.tube",
        ] {
            assert_eq!(host_rule(host), HostRule::Signal, "{host}");
        }
        for host in [
            "emilfrey.example",
            "sexyhair.example",
            "adultswim.example",
            "nakedwines.example",
            "nudeproject.example",
            "pussy888.example",
            "skylinewebcams.example",
            "example.com",
            "example.camera",
        ] {
            assert_eq!(host_rule(host), HostRule::None, "{host}");
        }
    }

    #[test]
    fn never_words_do_nothing() {
        for word in NEVER_WORDS {
            for host in [
                format!("{word}.com"),
                format!("my{word}page.com"),
                format!("{word}-x.net"),
            ] {
                assert_eq!(host_rule(&host), HostRule::None, "{host}");
            }
        }
    }

    #[test]
    fn the_word_lists_are_consistent() {
        let block: Vec<&str> = BLOCK_WORDS
            .iter()
            .chain(BRAND_STEMS)
            .map(|rule| rule.word)
            .collect();
        let lists: [(&str, &[&str]); 8] = [
            ("block", &block),
            ("signal tokens", SIGNAL_TOKENS),
            ("signal substrings", SIGNAL_SUBSTRINGS),
            ("never", NEVER_WORDS),
            ("site words", SITE_WORDS),
            ("head words", HEAD_WORDS),
            ("block tlds", BLOCK_TLDS),
            ("signal tlds", SIGNAL_TLDS),
        ];
        for (name, list) in lists {
            let mut seen = std::collections::HashSet::new();
            for word in list {
                assert!(seen.insert(word), "{name}: {word} twice");
                assert!(
                    !word.is_empty()
                        && word
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
                    "{name}: {word}"
                );
            }
        }
        // A word that is never used is on no other list.
        for word in NEVER_WORDS {
            assert!(!block.contains(word), "{word}");
            assert!(!SIGNAL_TOKENS.contains(word), "{word}");
            assert!(!SIGNAL_SUBSTRINGS.contains(word), "{word}");
        }
        // A folded stem has no digit: digits are what folding rewrites.
        for rule in BRAND_STEMS.iter().filter(|rule| rule.folded) {
            assert!(
                rule.word.bytes().all(|b| b.is_ascii_lowercase()),
                "{}",
                rule.word
            );
        }
        // "porn free" must stay out of the tail words (see `SITE_WORDS`), and the letter
        // "s" out of the head letters.
        assert!(!SITE_WORDS.contains(&"free"));
        assert!(!HEAD_LETTERS.contains(&b's'));
    }

    #[test]
    fn folding_follows_the_table() {
        assert_eq!(fold_lookalikes("p0rnhub"), "pornhub");
        assert_eq!(fold_lookalikes("l33t-sp34k"), "leet-speak");
        assert_eq!(fold_lookalikes("x1337"), "x");
        assert_eq!(fold_lookalikes("123xv1de0s9"), "xvideos");
        assert_eq!(fold_lookalikes("vvebcam"), "webcam");
        assert_eq!(fold_lookalikes("s74r"), "star");
        assert_eq!(fold_lookalikes("cam4"), "cam");
        assert_eq!(fold_lookalikes("a2b6c8d9"), "a2b6c8d");
        assert_eq!(fold_lookalikes("plain"), "plain");
        assert_eq!(fold_lookalikes("2024"), "");
        assert_eq!(collapse_repeats("xvvideoss"), "xvideos");
        assert_eq!(collapse_repeats("a11b"), "a11b");
    }

    #[test]
    fn titles_and_paths_signal_on_block_words_wherever_they_stand() {
        for title in [
            "Free Porn Videos - Example",
            "PORNO GRÁTIS | Example",
            "Pornô brasileiro",
            "Vídeos de putaria - Example",
            "xHamster: Free Videos",
            "Hentai Haven",
            "r/EarthPorn",
            "XXX videos",
            "Mulheres peladas",
            // A signal is one look, not a block: these ordinary titles cost one too.
            "Pornichet - Wikipedia",
            "Odporność organizmu - Zdrowie",
            "How to stay porn-free: a guide",
        ] {
            assert!(title_signal(title), "{title}");
        }
        for title in [
            "",
            "Inbox (3) - Mail",
            "Essex County Council",
            "Disputas contratuais - Planalto",
            "Aeromexico: vuelos",
            "Sexy hair products",
            "Adult Swim",
            "Watch anime online",
            "Escorts Kubota tractors",
        ] {
            assert!(!title_signal(title), "{title}");
        }
        assert!(path_signal("/r/EarthPorn/"));
        assert!(path_signal("/r/EarthPorn/comments/abc/a_lake/"));
        assert!(path_signal("/search?q=porn+videos"));
        assert!(path_signal("/tag/hentai"));
        assert!(path_signal("/user/xvideos_fan"));
        assert!(path_signal("/r/pornfree/"));
        assert!(!path_signal("/"));
        assert!(!path_signal("/r/pics/"));
        assert!(!path_signal("/wiki/Essex"));
        assert!(!path_signal("/watch?v=dQw4w9WgXcQ"));
    }

    #[test]
    fn a_very_long_run_of_letters_is_read_in_one_pass() {
        // A recursive reader took exponential time on runs like these.
        let started = std::time::Instant::now();
        let mut title = "porn".to_owned();
        title.push_str(&"x".repeat(39));
        title.push('q');
        assert!(title_signal(&title));
        assert_eq!(host_rule(&format!("{title}.example")), HostRule::Signal);
        let label = format!("{}porn{}q", "go".repeat(19), "ok".repeat(19));
        assert_eq!(host_rule(&format!("{label}.example")), HostRule::Signal);
        let long = format!("{}porn{}", "a".repeat(5_000), "o".repeat(5_000));
        assert!(title_signal(&long));
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }
}
