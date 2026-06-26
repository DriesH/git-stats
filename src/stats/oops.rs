//! "Oops" counter — commits whose first line confesses a mistake.

use crate::model::CommitRecord;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OopsStat {
    pub name: String,
    pub oops: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OopsBoard {
    pub total_oops: usize,
    pub leaders: Vec<OopsStat>,
}

/// Whole-word keywords that mark a commit as an "oops".
const OOPS_WORDS: &[&str] = &[
    "oops", "whoops", "typo", "wip", "revert", "fixup", "argh", "damn", "nvm", "broken", "forgot",
    "accidentally", "actually",
];

/// True when the commit's first line confesses a mistake: it contains the
/// phrase `fix fix`, or any `OOPS_WORDS` keyword as a whole (alphanumeric-
/// delimited) word.
fn is_oops(message: &str) -> bool {
    let first = message.lines().next().unwrap_or("");
    let lower = first.to_lowercase();
    if lower.contains("fix fix") {
        return true;
    }
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|tok| OOPS_WORDS.contains(&tok))
}

/// Per-author "oops" leaderboard plus the repo-wide total.
pub fn oops_board(records: &[CommitRecord]) -> OopsBoard {
    let mut by: HashMap<&str, (usize, usize)> = HashMap::new(); // (oops, total)
    let mut total_oops = 0;
    for r in records {
        let e = by.entry(r.author_name.as_str()).or_default();
        e.1 += 1;
        if is_oops(&r.message) {
            e.0 += 1;
            total_oops += 1;
        }
    }
    let mut leaders: Vec<OopsStat> = by
        .into_iter()
        .filter(|(_, (oops, _))| *oops >= 1)
        .map(|(name, (oops, total))| OopsStat {
            name: name.to_string(),
            oops,
            total,
        })
        .collect();
    leaders.sort_by(|a, b| b.oops.cmp(&a.oops).then(a.name.cmp(&b.name)));
    OopsBoard { total_oops, leaders }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileChurn;

    fn msg(author: &str, m: &str) -> CommitRecord {
        CommitRecord {
            sha: "s".into(),
            author_name: author.into(),
            author_email: "a@x".into(),
            timestamp: 0,
            tz_offset_minutes: 0,
            message: m.into(),
            files: vec![FileChurn { path: "x".into(), added: 1, removed: 0 }],
        }
    }

    #[test]
    fn counts_keyword_and_fix_fix_phrase_per_author() {
        let records = vec![
            msg("alice", "oops, forgot the import"),
            msg("alice", "fix fix the build"),
            msg("alice", "add real feature"),
            msg("bob", "Revert \"add feature\""),
        ];
        let board = oops_board(&records);
        assert_eq!(board.total_oops, 3);
        let alice = board.leaders.iter().find(|s| s.name == "alice").unwrap();
        assert_eq!(alice.oops, 2);
        assert_eq!(alice.total, 3);
        assert!(board.leaders.iter().any(|s| s.name == "bob" && s.oops == 1));
    }

    #[test]
    fn whole_word_only_no_substring_false_positives() {
        // "actuator" must NOT match the keyword "actually".
        let records = vec![msg("carol", "improve actuator timing")];
        let board = oops_board(&records);
        assert_eq!(board.total_oops, 0);
        assert!(board.leaders.is_empty());
    }

    #[test]
    fn empty_input_is_empty() {
        let board = oops_board(&[]);
        assert_eq!(board.total_oops, 0);
        assert!(board.leaders.is_empty());
    }
}
