//! Busiest single calendar day (in commit-local time).

use crate::model::CommitRecord;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusiestDay {
    pub date: String,
    pub commits: usize,
    pub top_author: String,
    pub top_author_commits: usize,
}

/// Local calendar date ("YYYY-MM-DD") of a commit, applying its tz offset.
fn local_date(r: &CommitRecord) -> String {
    let shifted = r.timestamp + i64::from(r.tz_offset_minutes) * 60;
    let dt = DateTime::<Utc>::from_timestamp(shifted, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    dt.format("%Y-%m-%d").to_string()
}

/// The single local calendar day with the most commits. Ties break to the most
/// recent date; the day's top author breaks ties by name ascending. `None` for
/// empty input.
pub fn busiest_day(records: &[CommitRecord]) -> Option<BusiestDay> {
    if records.is_empty() {
        return None;
    }
    // date -> (total commits, per-author counts)
    let mut by_date: HashMap<String, (usize, HashMap<&str, usize>)> = HashMap::new();
    for r in records {
        let entry = by_date.entry(local_date(r)).or_default();
        entry.0 += 1;
        *entry.1.entry(r.author_name.as_str()).or_default() += 1;
    }
    // Most commits, ties -> lexicographically largest date (= most recent, since
    // YYYY-MM-DD sorts chronologically).
    let (date, (commits, authors)) = by_date
        .into_iter()
        .max_by(|a, b| a.1 .0.cmp(&b.1 .0).then_with(|| a.0.cmp(&b.0)))?;
    // Top author that day: most commits, ties by name ascending.
    let (top_author, top_author_commits) = authors
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(n, c)| (n.to_string(), c))
        .unwrap_or_default();
    Some(BusiestDay {
        date,
        commits,
        top_author,
        top_author_commits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;

    // 2024-01-06T12:00:00Z and 2024-01-08T12:00:00Z (UTC, tz offset 0).
    const JAN6: i64 = 1_704_542_400;
    const JAN8: i64 = 1_704_715_200;

    #[test]
    fn picks_day_with_most_commits_and_top_author() {
        let records = vec![
            rec("alice", JAN6, &[("x", 1, 0)]),
            rec("alice", JAN6, &[("x", 1, 0)]),
            rec("bob", JAN6, &[("x", 1, 0)]),
            rec("bob", JAN8, &[("x", 1, 0)]),
        ];
        let d = busiest_day(&records).unwrap();
        assert_eq!(d.date, "2024-01-06");
        assert_eq!(d.commits, 3);
        assert_eq!(d.top_author, "alice");
        assert_eq!(d.top_author_commits, 2);
    }

    #[test]
    fn ties_break_to_most_recent_date() {
        let records = vec![
            rec("alice", JAN6, &[("x", 1, 0)]),
            rec("bob", JAN8, &[("x", 1, 0)]),
        ];
        let d = busiest_day(&records).unwrap();
        assert_eq!(d.date, "2024-01-08");
    }

    #[test]
    fn empty_input_is_none() {
        assert!(busiest_day(&[]).is_none());
    }
}
