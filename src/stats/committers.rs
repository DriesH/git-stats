use std::collections::HashMap;
use crate::model::CommitRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitterStat { pub name: String, pub commits: usize, pub lines: u64 }

pub fn top_committers(records: &[CommitRecord]) -> Vec<CommitterStat> {
    let mut by: HashMap<&str, (usize, u64)> = HashMap::new();
    for r in records {
        let e = by.entry(r.author_name.as_str()).or_default();
        e.0 += 1;
        e.1 += r.lines_changed();
    }
    let mut out: Vec<CommitterStat> = by.into_iter()
        .map(|(name, (commits, lines))| CommitterStat { name: name.to_string(), commits, lines })
        .collect();
    out.sort_by(|a, b| b.commits.cmp(&a.commits).then(a.name.cmp(&b.name)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    #[test]
    fn ranks_by_commit_count_desc() {
        let records = vec![
            rec("alice", 1, &[("a", 5, 0)]),
            rec("bob", 2, &[("b", 1, 0)]),
            rec("alice", 3, &[("a", 2, 1)]),
        ];
        let top = top_committers(&records);
        assert_eq!(top[0].name, "alice");
        assert_eq!(top[0].commits, 2);
        assert_eq!(top[0].lines, 8);
        assert_eq!(top[1].name, "bob");
        assert_eq!(top[1].commits, 1);
    }
}
