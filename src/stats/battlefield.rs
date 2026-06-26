//! "File battlefield" — files touched by the most distinct authors.

use crate::model::CommitRecord;
use crate::stats::filters::is_generated_path;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battlefield {
    pub path: String,
    pub authors: usize,
    pub commits: usize,
}

/// Files touched by the most distinct authors (>= 2), excluding generated files
/// unless `include_generated`. Sorted by distinct authors desc, then commits
/// desc, then path asc.
pub fn file_battlefield(records: &[CommitRecord], include_generated: bool) -> Vec<Battlefield> {
    // path -> (distinct authors, commit count)
    let mut by: HashMap<&str, (HashSet<&str>, usize)> = HashMap::new();
    for r in records {
        for f in &r.files {
            if !include_generated && is_generated_path(&f.path) {
                continue;
            }
            let e = by.entry(f.path.as_str()).or_default();
            e.0.insert(r.author_name.as_str());
            e.1 += 1;
        }
    }
    let mut out: Vec<Battlefield> = by
        .into_iter()
        .filter(|(_, (authors, _))| authors.len() >= 2)
        .map(|(path, (authors, commits))| Battlefield {
            path: path.to_string(),
            authors: authors.len(),
            commits,
        })
        .collect();
    out.sort_by(|a, b| {
        b.authors
            .cmp(&a.authors)
            .then(b.commits.cmp(&a.commits))
            .then(a.path.cmp(&b.path))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;

    #[test]
    fn ranks_files_by_distinct_authors_excluding_solo_and_generated() {
        let records = vec![
            rec("alice", 1, &[("core.rs", 1, 0), ("Cargo.lock", 9, 9)]),
            rec("bob", 2, &[("core.rs", 1, 0), ("Cargo.lock", 9, 9)]),
            rec("alice", 3, &[("solo.rs", 1, 0)]),
        ];
        let b = file_battlefield(&records, false);
        // core.rs: 2 authors, 2 commits. solo.rs dropped (1 author). Cargo.lock dropped (generated).
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].path, "core.rs");
        assert_eq!(b[0].authors, 2);
        assert_eq!(b[0].commits, 2);
    }

    #[test]
    fn include_generated_keeps_lock_files() {
        let records = vec![
            rec("alice", 1, &[("Cargo.lock", 1, 0)]),
            rec("bob", 2, &[("Cargo.lock", 1, 0)]),
        ];
        let b = file_battlefield(&records, true);
        assert!(b.iter().any(|x| x.path == "Cargo.lock" && x.authors == 2));
    }
}
