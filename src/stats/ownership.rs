use std::collections::HashMap;
use crate::model::CommitRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipStat { pub path: String, pub top_author: String, pub author_count: usize, pub commits: usize }

pub fn ownership(records: &[CommitRecord]) -> Vec<OwnershipStat> {
    let mut by: HashMap<&str, HashMap<&str, usize>> = HashMap::new();
    for r in records {
        for f in &r.files {
            *by.entry(f.path.as_str()).or_default().entry(r.author_name.as_str()).or_default() += 1;
        }
    }
    let mut out: Vec<OwnershipStat> = by.into_iter()
        .map(|(path, authors)| {
            let commits: usize = authors.values().sum();
            // Top author = most commits; tie broken by name ASCENDING.
            // max_by keeps the "greatest", so the name comparator is reversed
            // (b vs a) to make the alphabetically-first name rank highest.
            let (top, _) = authors.iter()
                .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
                .map(|(n, c)| (n.to_string(), *c))
                .unwrap_or_default();
            OwnershipStat { path: path.to_string(), top_author: top, author_count: authors.len(), commits }
        })
        .collect();
    out.sort_by(|a, b| b.commits.cmp(&a.commits).then(a.path.cmp(&b.path)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    #[test]
    fn top_author_and_distinct_count_per_file() {
        let records = vec![
            rec("alice", 1, &[("core.rs", 1, 0)]),
            rec("alice", 2, &[("core.rs", 1, 0)]),
            rec("bob",   3, &[("core.rs", 1, 0)]),
            rec("alice", 4, &[("solo.rs", 1, 0)]),
        ];
        let o = ownership(&records);
        let core = o.iter().find(|x| x.path == "core.rs").unwrap();
        assert_eq!(core.top_author, "alice");
        assert_eq!(core.author_count, 2);
        assert_eq!(core.commits, 3);
        let solo = o.iter().find(|x| x.path == "solo.rs").unwrap();
        assert_eq!(solo.author_count, 1);
    }
}
