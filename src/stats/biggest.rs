use crate::model::CommitRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiggestCommit {
    pub sha: String,
    pub author: String,
    pub lines: u64,
    pub summary: String,
}

pub fn biggest_commit(records: &[CommitRecord]) -> Option<BiggestCommit> {
    records
        .iter()
        .max_by_key(|r| r.lines_changed())
        .map(|r| BiggestCommit {
            sha: r.sha.clone(),
            author: r.author_name.clone(),
            lines: r.lines_changed(),
            summary: r.message.lines().next().unwrap_or("").to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    #[test]
    fn picks_commit_with_most_lines() {
        let records = vec![rec("a", 1, &[("x", 2, 2)]), rec("b", 2, &[("y", 50, 10)])];
        let big = biggest_commit(&records).unwrap();
        assert_eq!(big.author, "b");
        assert_eq!(big.lines, 60);
    }
    #[test]
    fn none_when_empty() {
        assert!(biggest_commit(&[]).is_none());
    }
}
