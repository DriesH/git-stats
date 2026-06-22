use std::collections::HashMap;
use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};
use crate::model::CommitRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourHistogram { pub hours: [usize; 24] }

#[derive(Debug, Clone, PartialEq)]
pub struct WeekendWarrior { pub name: String, pub weekend_pct: f64, pub total: usize }

#[derive(Debug, Clone, PartialEq)]
pub struct NightOwlStats { pub histogram: HourHistogram, pub warriors: Vec<WeekendWarrior> }

fn local(r: &CommitRecord) -> DateTime<Utc> {
    let shifted = r.timestamp + i64::from(r.tz_offset_minutes) * 60;
    DateTime::<Utc>::from_timestamp(shifted, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
}

pub fn night_owls(records: &[CommitRecord]) -> NightOwlStats {
    let mut hours = [0usize; 24];
    let mut per_author: HashMap<&str, (usize, usize)> = HashMap::new(); // (weekend, total)
    for r in records {
        let dt = local(r);
        hours[dt.hour() as usize] += 1;
        let is_weekend = matches!(dt.weekday(), Weekday::Sat | Weekday::Sun);
        let e = per_author.entry(r.author_name.as_str()).or_default();
        if is_weekend { e.0 += 1; }
        e.1 += 1;
    }
    let mut warriors: Vec<WeekendWarrior> = per_author.into_iter()
        .map(|(name, (w, t))| WeekendWarrior {
            name: name.to_string(),
            weekend_pct: if t == 0 { 0.0 } else { (w as f64) * 100.0 / (t as f64) },
            total: t,
        })
        .collect();
    warriors.sort_by(|a, b| {
        b.weekend_pct.partial_cmp(&a.weekend_pct).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.name.cmp(&b.name))
    });
    NightOwlStats { histogram: HourHistogram { hours }, warriors }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    const SAT_3AM: i64 = 1_704_510_000; // 2024-01-06T03:00:00Z (Saturday)
    const MON_10AM: i64 = 1_704_708_000; // 2024-01-08T10:00:00Z (Monday)
    #[test]
    fn histogram_counts_local_hour() {
        let records = vec![rec("a", SAT_3AM, &[("x", 1, 0)])];
        let s = night_owls(&records);
        assert_eq!(s.histogram.hours[3], 1);
    }
    #[test]
    fn weekend_pct_per_author() {
        let records = vec![rec("a", SAT_3AM, &[("x", 1, 0)]), rec("a", MON_10AM, &[("x", 1, 0)])];
        let s = night_owls(&records);
        let a = s.warriors.iter().find(|w| w.name == "a").unwrap();
        assert_eq!(a.total, 2);
        assert!((a.weekend_pct - 50.0).abs() < 1e-6);
    }
}
