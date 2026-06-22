use crate::model::CommitRecord;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Vitals {
    pub total_commits: usize,
    pub first: i64,
    pub last: i64,
    pub age_days: i64,
    pub commits_per_day: f64,
    pub authors: usize,
}

pub fn vitals(records: &[CommitRecord]) -> Option<Vitals> {
    if records.is_empty() {
        return None;
    }
    let first = records.iter().map(|r| r.timestamp).min().unwrap();
    let last = records.iter().map(|r| r.timestamp).max().unwrap();
    let age_days = (last - first) / 86_400;
    let authors = records
        .iter()
        .map(|r| r.author_name.as_str())
        .collect::<HashSet<_>>()
        .len();
    let total = records.len();
    let commits_per_day = if age_days > 0 {
        total as f64 / age_days as f64
    } else {
        total as f64
    };
    Some(Vitals {
        total_commits: total,
        first,
        last,
        age_days,
        commits_per_day,
        authors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    const DAY: i64 = 86_400;
    const D0: i64 = 1_704_067_200;
    #[test]
    fn computes_age_and_pace() {
        let records = vec![
            rec("a", D0, &[("x", 1, 0)]),
            rec("b", D0 + 10 * DAY, &[("x", 1, 0)]),
        ];
        let v = vitals(&records).unwrap();
        assert_eq!(v.total_commits, 2);
        assert_eq!(v.first, D0);
        assert_eq!(v.last, D0 + 10 * DAY);
        assert_eq!(v.age_days, 10);
        assert_eq!(v.authors, 2);
        assert!((v.commits_per_day - 0.2).abs() < 1e-6);
    }
    #[test]
    fn none_when_empty() {
        assert!(vitals(&[]).is_none());
    }
}
