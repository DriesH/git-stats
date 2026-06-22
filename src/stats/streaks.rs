use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Datelike};
use crate::model::CommitRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreakStat { pub name: String, pub longest_days: u32 }

fn local_day_number(r: &CommitRecord) -> i64 {
    let shifted = r.timestamp + i64::from(r.tz_offset_minutes) * 60;
    let dt = DateTime::<chrono::Utc>::from_timestamp(shifted, 0)
        .unwrap_or_else(|| DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap());
    dt.date_naive().num_days_from_ce() as i64
}

pub fn longest_streaks(records: &[CommitRecord]) -> Vec<StreakStat> {
    let mut days: HashMap<&str, HashSet<i64>> = HashMap::new();
    for r in records {
        days.entry(r.author_name.as_str()).or_default().insert(local_day_number(r));
    }
    let mut out: Vec<StreakStat> = days.into_iter()
        .map(|(name, set)| {
            let mut ds: Vec<i64> = set.into_iter().collect();
            ds.sort_unstable();
            let mut best = 1u32;
            let mut cur = 1u32;
            for w in ds.windows(2) {
                if w[1] == w[0] + 1 { cur += 1; best = best.max(cur); } else { cur = 1; }
            }
            StreakStat { name: name.to_string(), longest_days: if ds.is_empty() { 0 } else { best } }
        })
        .collect();
    out.sort_by(|a, b| b.longest_days.cmp(&a.longest_days).then(a.name.cmp(&b.name)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    const DAY: i64 = 86_400;
    const D0: i64 = 1_704_067_200; // 2024-01-01T00:00:00Z
    #[test]
    fn longest_run_of_consecutive_days() {
        let records = vec![
            rec("a", D0, &[("x",1,0)]),
            rec("a", D0 + DAY, &[("x",1,0)]),
            rec("a", D0 + 2*DAY, &[("x",1,0)]),
            rec("a", D0 + 5*DAY, &[("x",1,0)]),
        ];
        let s = longest_streaks(&records);
        let a = s.iter().find(|x| x.name == "a").unwrap();
        assert_eq!(a.longest_days, 3);
    }
}
