use crate::model::CommitRecord;
use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourHistogram {
    pub hours: [usize; 24],
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeekendWarrior {
    pub name: String,
    pub weekend_pct: f64,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chronotype {
    pub name: String,
    pub night_pct: f64,
    pub morning_pct: f64,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NightOwlStats {
    pub histogram: HourHistogram,
    pub warriors: Vec<WeekendWarrior>,
    pub night_owls: Vec<Chronotype>,
    pub early_birds: Vec<Chronotype>,
}

fn local(r: &CommitRecord) -> DateTime<Utc> {
    let shifted = r.timestamp + i64::from(r.tz_offset_minutes) * 60;
    DateTime::<Utc>::from_timestamp(shifted, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
}

pub fn night_owls(records: &[CommitRecord]) -> NightOwlStats {
    let mut hours = [0usize; 24];
    let mut per_author: HashMap<&str, (usize, usize)> = HashMap::new(); // (weekend, total)
    // Chronotype: night = 22:00-04:59, morning = 05:00-08:59 (local hour).
    let mut chrono: HashMap<&str, (usize, usize, usize)> = HashMap::new(); // (night, morning, total)
    for r in records {
        let dt = local(r);
        let hour = dt.hour();
        hours[hour as usize] += 1;
        let is_weekend = matches!(dt.weekday(), Weekday::Sat | Weekday::Sun);
        let e = per_author.entry(r.author_name.as_str()).or_default();
        if is_weekend {
            e.0 += 1;
        }
        e.1 += 1;
        let c = chrono.entry(r.author_name.as_str()).or_default();
        if matches!(hour, 22 | 23 | 0 | 1 | 2 | 3 | 4) {
            c.0 += 1;
        }
        if matches!(hour, 5..=8) {
            c.1 += 1;
        }
        c.2 += 1;
    }
    let mut warriors: Vec<WeekendWarrior> = per_author
        .into_iter()
        .map(|(name, (w, t))| WeekendWarrior {
            name: name.to_string(),
            weekend_pct: if t == 0 {
                0.0
            } else {
                (w as f64) * 100.0 / (t as f64)
            },
            total: t,
        })
        .collect();
    warriors.sort_by(|a, b| {
        b.weekend_pct
            .partial_cmp(&a.weekend_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.name.cmp(&b.name))
    });
    let chronotypes: Vec<Chronotype> = chrono
        .into_iter()
        .filter(|(_, (_, _, total))| *total >= 5)
        .map(|(name, (night, morning, total))| Chronotype {
            name: name.to_string(),
            night_pct: (night as f64) * 100.0 / (total as f64),
            morning_pct: (morning as f64) * 100.0 / (total as f64),
            total,
        })
        .collect();
    let mut night_owl_list: Vec<Chronotype> = chronotypes.clone();
    night_owl_list.sort_by(|a, b| {
        b.night_pct
            .partial_cmp(&a.night_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total.cmp(&a.total))
            .then(a.name.cmp(&b.name))
    });
    let mut early_bird_list: Vec<Chronotype> = chronotypes;
    early_bird_list.sort_by(|a, b| {
        b.morning_pct
            .partial_cmp(&a.morning_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total.cmp(&a.total))
            .then(a.name.cmp(&b.name))
    });

    NightOwlStats {
        histogram: HourHistogram { hours },
        warriors,
        night_owls: night_owl_list,
        early_birds: early_bird_list,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    const SAT_3AM: i64 = 1_704_510_000; // 2024-01-06T03:00:00Z (Saturday)
    const MON_10AM: i64 = 1_704_708_000; // 2024-01-08T10:00:00Z (Monday)
    const MON_7AM: i64 = 1_704_697_200; // 2024-01-08T07:00:00Z (Monday, morning)

    #[test]
    fn histogram_counts_local_hour() {
        let records = vec![rec("a", SAT_3AM, &[("x", 1, 0)])];
        let s = night_owls(&records);
        assert_eq!(s.histogram.hours[3], 1);
    }
    #[test]
    fn weekend_pct_per_author() {
        let records = vec![
            rec("a", SAT_3AM, &[("x", 1, 0)]),
            rec("a", MON_10AM, &[("x", 1, 0)]),
        ];
        let s = night_owls(&records);
        let a = s.warriors.iter().find(|w| w.name == "a").unwrap();
        assert_eq!(a.total, 2);
        assert!((a.weekend_pct - 50.0).abs() < 1e-6);
    }

    #[test]
    fn night_owls_ranked_by_share_with_min_five_commits() {
        // Owl: 5 commits at 03:00 -> 100% night, eligible.
        // Newbie: 1 commit at night -> ineligible (< 5).
        let mut records: Vec<_> = (0..5).map(|_| rec("owl", SAT_3AM, &[("x", 1, 0)])).collect();
        records.push(rec("newbie", SAT_3AM, &[("x", 1, 0)]));
        let s = night_owls(&records);
        assert_eq!(s.night_owls[0].name, "owl");
        assert!((s.night_owls[0].night_pct - 100.0).abs() < 1e-6);
        assert!(!s.night_owls.iter().any(|c| c.name == "newbie"));
    }

    #[test]
    fn early_birds_ranked_by_morning_share() {
        let records: Vec<_> = (0..5).map(|_| rec("lark", MON_7AM, &[("x", 1, 0)])).collect();
        let s = night_owls(&records);
        assert_eq!(s.early_birds[0].name, "lark");
        assert!((s.early_birds[0].morning_pct - 100.0).abs() < 1e-6);
    }
}
