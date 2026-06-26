use crate::model::CommitRecord;
use crate::stats::filters::is_generated_path;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChurnStat {
    pub path: String,
    pub added: u64,
    pub removed: u64,
}

impl ChurnStat {
    pub fn total(&self) -> u64 {
        self.added + self.removed
    }
}

pub fn churn_hotspots(records: &[CommitRecord], include_generated: bool) -> Vec<ChurnStat> {
    let mut by: HashMap<&str, (u64, u64)> = HashMap::new();
    for r in records {
        for f in &r.files {
            if !include_generated && is_generated_path(&f.path) {
                continue;
            }
            let e = by.entry(f.path.as_str()).or_default();
            e.0 += u64::from(f.added);
            e.1 += u64::from(f.removed);
        }
    }
    let mut out: Vec<ChurnStat> = by
        .into_iter()
        .map(|(path, (added, removed))| ChurnStat {
            path: path.to_string(),
            added,
            removed,
        })
        .collect();
    out.sort_by(|a, b| b.total().cmp(&a.total()).then(a.path.cmp(&b.path)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;
    #[test]
    fn aggregates_per_file_sorted_by_total_desc() {
        let records = vec![
            rec("a", 1, &[("hot.rs", 10, 5), ("cold.rs", 1, 0)]),
            rec("b", 2, &[("hot.rs", 4, 2)]),
        ];
        let c = churn_hotspots(&records, false);
        assert_eq!(c[0].path, "hot.rs");
        assert_eq!(c[0].added, 14);
        assert_eq!(c[0].removed, 7);
        assert_eq!(c[0].total(), 21);
        assert_eq!(c[1].path, "cold.rs");
    }

    #[test]
    fn skips_generated_files_unless_included() {
        let records = vec![rec("a", 1, &[("Cargo.lock", 100, 50), ("src/main.rs", 3, 1)])];
        let filtered = churn_hotspots(&records, false);
        assert!(filtered.iter().all(|c| c.path != "Cargo.lock"));
        assert_eq!(filtered[0].path, "src/main.rs");
        let included = churn_hotspots(&records, true);
        assert!(included.iter().any(|c| c.path == "Cargo.lock"));
    }
}
