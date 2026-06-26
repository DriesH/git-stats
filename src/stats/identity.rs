//! Collapse near-duplicate author identities into one canonical identity.
//!
//! Runs once over the full record set before any analyzer, so every
//! author-keyed stat sees canonical names/emails. Two raw `(name, email)`
//! identities merge when ANY of: same email (A), same name (B), or a GitHub
//! `<digits>+<handle>@users.noreply.github.com` handle matching another
//! identity's spaceless name or email local-part (C).

use crate::model::CommitRecord;
use std::collections::HashMap;

/// Disjoint-set over identity indices.
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect() }
    }

    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // Path compression.
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

/// Spaceless, lowercased name used as a Rule-C lookup key.
fn name_key(name: &str) -> String {
    name.trim().to_lowercase().replace(' ', "")
}

/// Lowercased local-part (before `@`) of a non-empty email.
fn local_part(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return None;
    }
    Some(email.split('@').next().unwrap_or(&email).to_string())
}

/// Extract the handle from `<digits>+<handle>@users.noreply.github.com` or
/// `<handle>@users.noreply.github.com`.
fn noreply_handle(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    let local = email.strip_suffix("@users.noreply.github.com")?;
    let handle = local.split_once('+').map(|(_, h)| h).unwrap_or(local);
    (!handle.is_empty()).then(|| handle.to_string())
}

/// Pick the highest-voted string; ties broken by lexicographically smallest.
fn pick(votes: &HashMap<String, usize>) -> String {
    votes
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(v, _)| v.clone())
        .unwrap_or_default()
}

/// Rewrite each record's author to its cluster's canonical identity. Record
/// order is preserved; output is independent of input order.
pub fn collapse_identities(records: Vec<CommitRecord>) -> Vec<CommitRecord> {
    // 1. Distinct raw identities and their commit counts.
    let mut index_of: HashMap<(String, String), usize> = HashMap::new();
    let mut identities: Vec<(String, String)> = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    for r in &records {
        let key = (r.author_name.clone(), r.author_email.clone());
        let idx = *index_of.entry(key.clone()).or_insert_with(|| {
            identities.push(key);
            counts.push(0);
            identities.len() - 1
        });
        counts[idx] += 1;
    }

    let mut uf = UnionFind::new(identities.len());

    // Rule A — same non-empty email.
    let mut by_email: HashMap<String, usize> = HashMap::new();
    for (i, (_, email)) in identities.iter().enumerate() {
        let e = email.trim().to_lowercase();
        if e.is_empty() {
            continue;
        }
        match by_email.get(&e) {
            Some(&j) => uf.union(i, j),
            None => {
                by_email.insert(e, i);
            }
        }
    }

    // Rule B — same name (case-insensitive, trimmed); skip empty names so blank
    // authors are not all collapsed into one.
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for (i, (name, _)) in identities.iter().enumerate() {
        let n = name.trim().to_lowercase();
        if n.is_empty() {
            continue;
        }
        match by_name.get(&n) {
            Some(&j) => uf.union(i, j),
            None => {
                by_name.insert(n, i);
            }
        }
    }

    // Rule C — a noreply handle matching another identity's spaceless name or
    // email local-part. Build the lookup keys, then fire only on noreply emails.
    let mut by_key: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, (name, email)) in identities.iter().enumerate() {
        by_key.entry(name_key(name)).or_default().push(i);
        if let Some(lp) = local_part(email) {
            by_key.entry(lp).or_default().push(i);
        }
    }
    for (i, (_, email)) in identities.iter().enumerate() {
        if let Some(handle) = noreply_handle(email) {
            if let Some(ids) = by_key.get(&handle) {
                for &j in ids {
                    uf.union(i, j);
                }
            }
        }
    }

    // 2. Canonical name/email per cluster root, weighted by commit count.
    let mut name_votes: HashMap<usize, HashMap<String, usize>> = HashMap::new();
    let mut email_votes: HashMap<usize, HashMap<String, usize>> = HashMap::new();
    for (i, (name, email)) in identities.iter().enumerate() {
        let root = uf.find(i);
        *name_votes.entry(root).or_default().entry(name.clone()).or_default() += counts[i];
        if !email.trim().is_empty() {
            *email_votes.entry(root).or_default().entry(email.clone()).or_default() += counts[i];
        }
    }
    let mut canonical: HashMap<usize, (String, String)> = HashMap::new();
    for (&root, votes) in &name_votes {
        let email = email_votes.get(&root).map(pick).unwrap_or_default();
        canonical.insert(root, (pick(votes), email));
    }

    // 3. Map every raw identity to its canonical pair, then rewrite records.
    let mut id_to_canon: HashMap<usize, (String, String)> = HashMap::new();
    for i in 0..identities.len() {
        let root = uf.find(i);
        id_to_canon.insert(i, canonical[&root].clone());
    }
    records
        .into_iter()
        .map(|mut r| {
            let idx = index_of[&(r.author_name.clone(), r.author_email.clone())];
            let (name, email) = id_to_canon[&idx].clone();
            r.author_name = name;
            r.author_email = email;
            r
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CommitRecord, FileChurn};

    /// Build a record with explicit name + email; message/files are filler.
    fn rec(name: &str, email: &str) -> CommitRecord {
        CommitRecord {
            sha: format!("{name}-{email}"),
            author_name: name.into(),
            author_email: email.into(),
            timestamp: 0,
            tz_offset_minutes: 0,
            message: "m".into(),
            files: vec![FileChurn { path: "x".into(), added: 1, removed: 0 }],
        }
    }

    fn names(records: &[CommitRecord]) -> Vec<String> {
        records.iter().map(|r| r.author_name.clone()).collect()
    }

    #[test]
    fn rule_a_same_email_merges_and_picks_majority_name() {
        let recs = vec![
            rec("Dries Heyninck", "d@x.com"),
            rec("Dries Heyninck", "d@x.com"),
            rec("dries", "d@x.com"),
        ];
        let out = collapse_identities(recs);
        assert_eq!(names(&out), vec!["Dries Heyninck"; 3]);
    }

    #[test]
    fn rule_b_same_name_case_insensitive_merges_emails() {
        let recs = vec![
            rec("Alice", "alice@home.com"),
            rec("alice", "alice@work.com"),
        ];
        let out = collapse_identities(recs);
        assert_eq!(out[0].author_name, out[1].author_name);
    }

    #[test]
    fn rule_c_github_noreply_handle_matches_name() {
        let recs = vec![
            rec("driesheyninck", "real@x.com"),
            rec("Dries via GitHub", "12345+driesheyninck@users.noreply.github.com"),
        ];
        let out = collapse_identities(recs);
        assert_eq!(out[0].author_name, out[1].author_name);
    }

    #[test]
    fn rule_c_plain_noreply_handle_matches_name() {
        let recs = vec![
            rec("driesheyninck", "real@x.com"),
            rec("via GitHub", "driesheyninck@users.noreply.github.com"),
        ];
        let out = collapse_identities(recs);
        assert_eq!(out[0].author_name, out[1].author_name);
    }

    #[test]
    fn distinct_people_stay_separate() {
        let recs = vec![rec("Alice", "alice@x.com"), rec("Bob", "bob@x.com")];
        let out = collapse_identities(recs);
        assert_eq!(out[0].author_name, "Alice");
        assert_eq!(out[1].author_name, "Bob");
    }

    #[test]
    fn empty_email_does_not_merge_unrelated_identities() {
        let recs = vec![rec("Alice", ""), rec("Bob", "")];
        let out = collapse_identities(recs);
        assert_eq!(out[0].author_name, "Alice");
        assert_eq!(out[1].author_name, "Bob");
    }

    #[test]
    fn result_is_order_independent() {
        let a = vec![
            rec("Dries Heyninck", "d@x.com"),
            rec("Dries Heyninck", "d@x.com"),
            rec("dries", "d@x.com"),
        ];
        let mut b = a.clone();
        b.reverse();
        let ra: Vec<String> = collapse_identities(a).iter().map(|r| r.author_name.clone()).collect();
        let mut rb: Vec<String> = collapse_identities(b).iter().map(|r| r.author_name.clone()).collect();
        rb.reverse();
        assert_eq!(ra, rb);
    }
}
