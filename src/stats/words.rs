use crate::model::CommitRecord;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCount {
    pub word: String,
    pub count: usize,
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "this", "that", "from", "into", "are", "was", "but", "not", "you",
    "your", "all", "can", "use", "add", "now", "out", "merge", "branch", "pull", "request", "pr",
    "wip", "via", "git",
];

/// Git trailer keys (lowercased) whose lines carry metadata, not prose, and so
/// are excluded from word/phrase analysis.
const TRAILER_KEYS: &[&str] = &[
    "co-authored-by",
    "signed-off-by",
    "reviewed-by",
    "acked-by",
    "tested-by",
    "reported-by",
    "suggested-by",
    "refs",
    "ref",
    "cc",
];

/// True when a commit-message line is a git trailer or tool-generated
/// boilerplate (e.g. "🤖 Generated with ..."), which would otherwise flood the
/// word cloud with noise like author emails and tool names.
fn is_noise_line(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with('🤖') || t.to_lowercase().contains("generated with") {
        return true;
    }
    // A `Key: value` trailer whose key is one of the known metadata keys. The
    // key check (alphabetic/hyphen only) keeps conventional-commit subjects
    // like "feat: add x" from being mistaken for trailers.
    if let Some((key, _)) = t.split_once(':') {
        let key = key.trim();
        if !key.is_empty()
            && key.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
            && TRAILER_KEYS.contains(&key.to_lowercase().as_str())
        {
            return true;
        }
    }
    false
}

/// Remove `http(s)://…` and `www.…` URLs and bare `host.tld` domains from a
/// line before tokenization, so the word cloud is not flooded with `https`,
/// `github`, `com`, etc. Replaces each match with a space to preserve adjacency
/// breaks.
fn strip_urls(line: &str) -> String {
    const TLDS: &[&str] = &[".com", ".org", ".io", ".net", ".dev", ".gov", ".edu"];
    let mut out = String::with_capacity(line.len());
    for token in line.split_whitespace() {
        let lower = token.to_lowercase();
        let is_url = lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("www.")
            || TLDS.iter().any(|t| lower.contains(t));
        if is_url {
            out.push(' ');
        } else {
            out.push_str(token);
        }
        out.push(' ');
    }
    out
}

/// True when a token is a pure number or a short-SHA-looking hex string
/// (length >= 7, all hex digits) — noise rather than a word.
fn is_number_or_hash(word: &str) -> bool {
    if word.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    word.len() >= 7 && word.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn top_words(records: &[CommitRecord], limit: usize) -> Vec<WordCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        for line in r.message.lines().filter(|l| !is_noise_line(l)) {
            let cleaned = strip_urls(line);
            for raw in cleaned.split(|c: char| !c.is_alphanumeric()) {
                let w = raw.to_lowercase();
                if w.len() < 3 || STOPWORDS.contains(&w.as_str()) || is_number_or_hash(&w) {
                    continue;
                }
                *by.entry(w).or_default() += 1;
            }
        }
    }
    let mut out: Vec<WordCount> = by
        .into_iter()
        .map(|(word, count)| WordCount { word, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    out.truncate(limit);
    out
}

/// Count adjacent two-word phrases. A token is kept under the same rule as
/// `top_words` (length >= 3, not a stopword). A dropped token breaks
/// adjacency, so no phrase bridges a removed stopword. Empty splits (from
/// runs of punctuation) are skipped without breaking adjacency. Trailer and
/// boilerplate lines are excluded, and phrases never bridge a line break.
pub fn top_bigrams(records: &[CommitRecord], limit: usize) -> Vec<WordCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        for line in r.message.lines().filter(|l| !is_noise_line(l)) {
            let cleaned = strip_urls(line);
            let mut prev: Option<String> = None;
            for raw in cleaned.split(|c: char| !c.is_alphanumeric()) {
                if raw.is_empty() {
                    continue;
                }
                let w = raw.to_lowercase();
                if w.len() < 3 || STOPWORDS.contains(&w.as_str()) || is_number_or_hash(&w) {
                    prev = None;
                    continue;
                }
                if let Some(p) = &prev {
                    *by.entry(format!("{p} {w}")).or_default() += 1;
                }
                prev = Some(w);
            }
        }
    }
    let mut out: Vec<WordCount> = by
        .into_iter()
        .map(|(word, count)| WordCount { word, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    out.truncate(limit);
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCount {
    pub kind: String,
    pub count: usize,
}

/// Conventional-commit types recognized as an explicit `type:` prefix.
const KNOWN_TYPES: &[&str] = &[
    "feat", "fix", "docs", "refactor", "test", "chore", "style", "perf", "build", "ci", "revert",
];

/// Keyword → kind rules, tried in order when there is no conventional prefix.
/// First rule whose keyword appears as a substring of the lowercased first
/// line wins.
const KEYWORD_RULES: &[(&str, &str)] = &[
    ("fix", "fix"),
    ("bug", "fix"),
    ("patch", "fix"),
    ("hotfix", "fix"),
    ("add", "feat"),
    ("new", "feat"),
    ("introduce", "feat"),
    ("feature", "feat"),
    ("doc", "docs"),
    ("readme", "docs"),
    ("refactor", "refactor"),
    ("cleanup", "refactor"),
    ("rename", "refactor"),
    ("test", "test"),
    ("chore", "chore"),
    ("bump", "chore"),
    ("deps", "chore"),
    ("dependency", "chore"),
];

/// Recognize a leading `type:` / `type(scope):` / `type!:` prefix, returning
/// the lowercased type when it is one of `KNOWN_TYPES`.
fn conventional_prefix(line: &str) -> Option<String> {
    let line = line.trim_start();
    let colon = line.find(':')?;
    let head = &line[..colon];
    let typ = head
        .split('(')
        .next()
        .unwrap_or(head)
        .trim_end_matches('!')
        .trim()
        .to_lowercase();
    KNOWN_TYPES.contains(&typ.as_str()).then_some(typ)
}

/// Classify one commit message into a single kind.
fn classify(message: &str) -> String {
    let first = message.lines().next().unwrap_or("");
    if let Some(kind) = conventional_prefix(first) {
        return kind;
    }
    let lower = first.to_lowercase();
    for (key, kind) in KEYWORD_RULES {
        if lower.contains(key) {
            return (*kind).to_string();
        }
    }
    "other".to_string()
}

/// Count commits by inferred type. Sorted by count desc, then kind asc.
pub fn commit_types(records: &[CommitRecord]) -> Vec<TypeCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        *by.entry(classify(&r.message)).or_default() += 1;
    }
    let mut out: Vec<TypeCount> = by
        .into_iter()
        .map(|(kind, count)| TypeCount { kind, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.kind.cmp(&b.kind)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CommitRecord, FileChurn};
    fn msg(m: &str) -> CommitRecord {
        CommitRecord {
            sha: "s".into(),
            author_name: "a".into(),
            author_email: "a@x".into(),
            timestamp: 0,
            tz_offset_minutes: 0,
            message: m.into(),
            files: vec![FileChurn {
                path: "x".into(),
                added: 1,
                removed: 0,
            }],
        }
    }
    #[test]
    fn counts_words_lowercased_filters_stopwords_and_short() {
        let records = vec![msg("Fix the Login bug"), msg("fix login again")];
        let w = top_words(&records, 10);
        assert_eq!(w[0].word, "fix");
        assert_eq!(w[0].count, 2);
        assert!(w.iter().any(|x| x.word == "login" && x.count == 2));
        assert!(!w.iter().any(|x| x.word == "the"));
    }

    #[test]
    fn words_skip_trailers_and_boilerplate() {
        let records = vec![msg(
            "fix login bug\n\n🤖 Generated with Claude Code\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
        )];
        let w = top_words(&records, 50);
        // Real prose survives.
        assert!(w.iter().any(|x| x.word == "login"));
        // Trailer / boilerplate noise is gone.
        for noise in ["anthropic", "noreply", "authored", "claude", "generated"] {
            assert!(!w.iter().any(|x| x.word == noise), "leaked: {noise}");
        }
    }

    #[test]
    fn bigrams_skip_trailers() {
        let records = vec![msg(
            "add cache layer\n\nCo-authored-by: Claude <noreply@anthropic.com>",
        )];
        let b = top_bigrams(&records, 50);
        assert!(b.iter().any(|x| x.word == "cache layer"));
        assert!(!b.iter().any(|x| x.word == "noreply anthropic"));
        assert!(!b.iter().any(|x| x.word.contains("anthropic")));
    }

    #[test]
    fn commit_types_parses_conventional_prefix() {
        let records = vec![
            msg("feat: add login"),
            msg("Fix(auth): handle expiry"),
            msg("feat!: breaking change"),
            msg("docs: update readme"),
        ];
        let t = commit_types(&records);
        // feat appears twice -> first, sorted by count desc then kind asc
        assert_eq!(t[0].kind, "feat");
        assert_eq!(t[0].count, 2);
        assert!(t.iter().any(|x| x.kind == "fix" && x.count == 1));
        assert!(t.iter().any(|x| x.kind == "docs" && x.count == 1));
    }

    #[test]
    fn commit_types_infers_from_keywords_then_other() {
        let records = vec![
            msg("squashed a nasty bug"), // -> fix (keyword "bug")
            msg("introduce dark mode"),  // -> feat (keyword "introduce")
            msg("rename the module"),    // -> refactor (keyword "rename")
            msg("merge branch main"),    // -> other (no rule)
        ];
        let t = commit_types(&records);
        assert!(t.iter().any(|x| x.kind == "fix" && x.count == 1));
        assert!(t.iter().any(|x| x.kind == "feat" && x.count == 1));
        assert!(t.iter().any(|x| x.kind == "refactor" && x.count == 1));
        assert!(t.iter().any(|x| x.kind == "other" && x.count == 1));
    }

    #[test]
    fn commit_types_empty_input_is_empty() {
        assert!(commit_types(&[]).is_empty());
    }

    #[test]
    fn bigrams_pair_adjacent_surviving_tokens() {
        let records = vec![msg("refactor login parser"), msg("refactor login parser")];
        let b = top_bigrams(&records, 10);
        assert!(b.iter().any(|x| x.word == "refactor login" && x.count == 2));
        assert!(b.iter().any(|x| x.word == "login parser" && x.count == 2));
    }

    #[test]
    fn bigrams_stopword_breaks_adjacency() {
        // "the" is a stopword and is dropped, so it must NOT bridge fix<->bug.
        let records = vec![msg("fix the bug")];
        let b = top_bigrams(&records, 10);
        assert!(!b.iter().any(|x| x.word == "fix bug"));
        assert!(b.is_empty());
    }

    #[test]
    fn bigrams_sorted_and_truncated() {
        let records = vec![
            msg("alpha beta alpha beta"), // "alpha beta" x2, "beta alpha" x1
            msg("gamma delta"),           // "gamma delta" x1
        ];
        let b = top_bigrams(&records, 2);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].word, "alpha beta");
        assert_eq!(b[0].count, 2);
    }

    #[test]
    fn drops_urls_domains_numbers_and_hashes() {
        let records = vec![msg(
            "see https://github.com/foo/bar and www.example.org for 12345 abcdef1",
        )];
        let w = top_words(&records, 50);
        for noise in ["github", "com", "https", "www", "example", "org", "12345", "abcdef1"] {
            assert!(!w.iter().any(|x| x.word == noise), "leaked: {noise}");
        }
        // Plain prose survives.
        assert!(w.iter().any(|x| x.word == "see"));
    }

    #[test]
    fn drops_new_mechanical_stopwords_but_keeps_update() {
        let records = vec![msg("merge branch update parser")];
        let w = top_words(&records, 50);
        assert!(!w.iter().any(|x| x.word == "merge"));
        assert!(!w.iter().any(|x| x.word == "branch"));
        assert!(w.iter().any(|x| x.word == "update"));
        assert!(w.iter().any(|x| x.word == "parser"));
    }
}
