use std::collections::HashMap;
use crate::model::CommitRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCount { pub word: String, pub count: usize }

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "this", "that", "from", "into", "are", "was",
    "but", "not", "you", "your", "all", "can", "use", "add", "now", "out",
];

pub fn top_words(records: &[CommitRecord], limit: usize) -> Vec<WordCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        for raw in r.message.split(|c: char| !c.is_alphanumeric()) {
            let w = raw.to_lowercase();
            if w.len() < 3 || STOPWORDS.contains(&w.as_str()) { continue; }
            *by.entry(w).or_default() += 1;
        }
    }
    let mut out: Vec<WordCount> = by.into_iter().map(|(word, count)| WordCount { word, count }).collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CommitRecord, FileChurn};
    fn msg(m: &str) -> CommitRecord {
        CommitRecord { sha: "s".into(), author_name: "a".into(), author_email: "a@x".into(),
            timestamp: 0, tz_offset_minutes: 0, message: m.into(),
            files: vec![FileChurn { path: "x".into(), added: 1, removed: 0 }] }
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
}
