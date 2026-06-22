use crate::model::CommitRecord;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCount {
    pub word: String,
    pub count: usize,
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "this", "that", "from", "into", "are", "was", "but", "not", "you",
    "your", "all", "can", "use", "now", "out",
];

pub fn top_words(records: &[CommitRecord], limit: usize) -> Vec<WordCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        for raw in r.message.split(|c: char| !c.is_alphanumeric()) {
            let w = raw.to_lowercase();
            if w.len() < 3 || STOPWORDS.contains(&w.as_str()) {
                continue;
            }
            *by.entry(w).or_default() += 1;
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
/// runs of punctuation) are skipped without breaking adjacency.
pub fn top_bigrams(records: &[CommitRecord], limit: usize) -> Vec<WordCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        let mut prev: Option<String> = None;
        for raw in r.message.split(|c: char| !c.is_alphanumeric()) {
            if raw.is_empty() {
                continue;
            }
            let w = raw.to_lowercase();
            if w.len() < 3 || STOPWORDS.contains(&w.as_str()) {
                prev = None;
                continue;
            }
            if let Some(p) = &prev {
                *by.entry(format!("{p} {w}")).or_default() += 1;
            }
            prev = Some(w);
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
        let records = vec![msg("add login support"), msg("add login support")];
        let b = top_bigrams(&records, 10);
        assert!(b.iter().any(|x| x.word == "add login" && x.count == 2));
        assert!(b.iter().any(|x| x.word == "login support" && x.count == 2));
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
}
