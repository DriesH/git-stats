# git-stats: more stats — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. The `rust-best-practices` skill MUST be active while writing Rust code.

**Goal:** Add author identity collapsing, a reusable generated-file filter, four improvements to existing stats (night-owl rank, churn filtering, commit-word cleanup, author collapse), and four new stats (chronotype, oops, busiest day, file battlefield) to the git-stats scoreboard.

**Architecture:** Two new pure foundations (`stats/identity.rs`, `stats/filters.rs`) plus four new pure analyzers (`stats/oops.rs`, `stats/busiest.rs`, `stats/battlefield.rs`, and chronotype folded into `stats/nightowl.rs`). Identity collapse runs once over the records in `main.rs` before `analyze()`, so every author-keyed stat benefits automatically. `scoreboard.rs` and `tui/` are wired last. Layer separation preserved: `stats/` stay pure and unit-tested first (TDD); `tui/` only renders.

**Tech Stack:** Rust, `chrono` (already a dep), `clap`, `ratatui`, `anyhow`. Errors bubble via `?`. Tests via `cargo test`.

**Spec:** `docs/superpowers/specs/2026-06-26-git-stats-more-stats-design.md`

---

## File Structure

- **Create** `src/stats/filters.rs` — `is_generated_path(path) -> bool`.
- **Create** `src/stats/identity.rs` — `collapse_identities(Vec<CommitRecord>) -> Vec<CommitRecord>`.
- **Create** `src/stats/oops.rs` — `oops_board(&[CommitRecord]) -> OopsBoard`.
- **Create** `src/stats/busiest.rs` — `busiest_day(&[CommitRecord]) -> Option<BusiestDay>`.
- **Create** `src/stats/battlefield.rs` — `file_battlefield(&[CommitRecord], bool) -> Vec<Battlefield>`.
- **Modify** `src/stats/mod.rs` — register the 5 new modules.
- **Modify** `src/stats/churn.rs` — add `include_generated: bool` param + filtering.
- **Modify** `src/stats/words.rs` — URL/domain/number/hex stripping + new stopwords.
- **Modify** `src/stats/nightowl.rs` — add `Chronotype` + night-owl/early-bird leaders.
- **Modify** `src/scoreboard.rs` — `analyze` signature, new fields, new spawns.
- **Modify** `src/cli.rs` — `--include-generated` flag.
- **Modify** `src/main.rs` — collapse identities, pass the flag to `analyze`.
- **Modify** `src/tui/panels.rs` + `src/tui/app.rs` — new panels/tabs, extend night-owl panel.

---

## Task 1: `stats/filters.rs` — generated-file predicate

**Files:**
- Create: `src/stats/filters.rs`
- Modify: `src/stats/mod.rs:1-8` (add `pub mod filters;`)

- [ ] **Step 1: Register the module**

In `src/stats/mod.rs`, add the line (keep the list alphabetical):

```rust
pub mod filters;
```

- [ ] **Step 2: Write the failing tests**

Create `src/stats/filters.rs`:

```rust
//! Shared predicate for skipping lock / generated / vendored files in
//! churn-style leaderboards.

/// Exact basenames of dependency lock files.
const LOCK_BASENAMES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "composer.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Pipfile.lock",
    "go.sum",
    "flake.lock",
];

/// Path suffixes for minified / generated artifacts.
const GENERATED_SUFFIXES: &[&str] = &[".min.js", ".min.css", ".map"];

/// Vendored directory names; matched against any `/`-separated path component.
const VENDOR_DIRS: &[&str] = &["vendor", "node_modules", "dist", "build"];

/// True when `path` is a lock file, minified/generated artifact, or lives in a
/// vendored directory — churn noise rather than authored code. `path` is the
/// repo-relative, `/`-separated string stored in `FileChurn.path`.
pub fn is_generated_path(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    if LOCK_BASENAMES.contains(&basename) {
        return true;
    }
    if GENERATED_SUFFIXES.iter().any(|s| path.ends_with(s)) {
        return true;
    }
    path.split('/').any(|c| VENDOR_DIRS.contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_lock_basenames_anywhere_in_tree() {
        assert!(is_generated_path("Cargo.lock"));
        assert!(is_generated_path("frontend/package-lock.json"));
        assert!(is_generated_path("a/b/go.sum"));
    }

    #[test]
    fn matches_minified_and_map_suffixes() {
        assert!(is_generated_path("static/app.min.js"));
        assert!(is_generated_path("static/app.min.css"));
        assert!(is_generated_path("bundle.js.map"));
    }

    #[test]
    fn matches_vendored_directory_components() {
        assert!(is_generated_path("node_modules/left-pad/index.js"));
        assert!(is_generated_path("dist/main.js"));
        assert!(is_generated_path("go/vendor/foo/bar.go"));
    }

    #[test]
    fn keeps_authored_source_files() {
        assert!(!is_generated_path("src/main.rs"));
        assert!(!is_generated_path("README.md"));
        // "rebuild" contains "build" as a substring but not as a component.
        assert!(!is_generated_path("src/rebuild/mod.rs"));
    }
}
```

- [ ] **Step 3: Run the tests to verify they pass**

Run: `cargo test --lib filters`
Expected: PASS (4 tests).

- [ ] **Step 4: Commit**

```bash
git add src/stats/filters.rs src/stats/mod.rs
git commit -m "feat(stats): add reusable is_generated_path filter"
```

---

## Task 2: `stats/identity.rs` — author identity collapse

**Files:**
- Create: `src/stats/identity.rs`
- Modify: `src/stats/mod.rs` (add `pub mod identity;`)

- [ ] **Step 1: Register the module**

In `src/stats/mod.rs`, add:

```rust
pub mod identity;
```

- [ ] **Step 2: Write the failing tests**

Create `src/stats/identity.rs` with the test module only (so it fails to compile / fails the asserts):

```rust
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
        // Same email, two name spellings; "Dries Heyninck" appears twice.
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
        // Both collapse to a single display name.
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
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib identity`
Expected: FAIL to compile (`collapse_identities` not found).

- [ ] **Step 4: Write the implementation**

Prepend to `src/stats/identity.rs` (above the test module):

```rust
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

    // Rule B — same name (case-insensitive, trimmed).
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for (i, (name, _)) in identities.iter().enumerate() {
        let n = name.trim().to_lowercase();
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --lib identity`
Expected: PASS (6 tests).

- [ ] **Step 6: Commit**

```bash
git add src/stats/identity.rs src/stats/mod.rs
git commit -m "feat(stats): collapse near-duplicate author identities"
```

---

## Task 3: Churn ignores generated files

**Files:**
- Modify: `src/stats/churn.rs:17-36` (signature + filtering)
- Modify: `src/stats/churn.rs` test (existing call site)

- [ ] **Step 1: Update the existing test and add a new one**

In `src/stats/churn.rs`, replace the whole `#[cfg(test)] mod tests { ... }` block with:

```rust
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
```

- [ ] **Step 2: Run the new test to verify it fails**

Run: `cargo test --lib churn`
Expected: FAIL to compile (`churn_hotspots` takes 1 arg, not 2).

- [ ] **Step 3: Update the implementation**

In `src/stats/churn.rs`, add the import at the top (below `use crate::model::CommitRecord;`):

```rust
use crate::stats::filters::is_generated_path;
```

Replace the `churn_hotspots` function signature and the file loop:

```rust
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
    // ... rest unchanged (build out, sort, return)
```

Leave the `let mut out: Vec<ChurnStat> = by ...` block and the sort/return below it exactly as they are.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib churn`
Expected: PASS (2 tests). (`scoreboard` will not compile yet — that is fixed in Task 9.)

- [ ] **Step 5: Commit**

```bash
git add src/stats/churn.rs
git commit -m "feat(stats): churn skips lock/generated files by default"
```

---

## Task 4: Commit-word cleanup

**Files:**
- Modify: `src/stats/words.rs:10-13` (stopwords), `:53-108` (token filtering)

- [ ] **Step 1: Add failing tests**

In `src/stats/words.rs`, inside `mod tests`, add these tests (the existing `msg` helper is already in that module):

```rust
    #[test]
    fn drops_urls_domains_numbers_and_hashes() {
        let records = vec![msg(
            "see https://github.com/foo/bar and www.example.org for 12345 abcdef1",
        )];
        let w = top_words(&records, 50);
        for noise in ["github", "com", "https", "www", "example", "org", "12345", "abcdef1"] {
            assert!(!w.iter().any(|x| x.word == noise), "leaked: {noise}");
        }
        // Plain prose survives.
        assert!(w.iter().any(|x| x.word == "see"));
    }

    #[test]
    fn drops_new_mechanical_stopwords_but_keeps_update() {
        let records = vec![msg("merge branch update parser")];
        let w = top_words(&records, 50);
        assert!(!w.iter().any(|x| x.word == "merge"));
        assert!(!w.iter().any(|x| x.word == "branch"));
        assert!(w.iter().any(|x| x.word == "update"));
        assert!(w.iter().any(|x| x.word == "parser"));
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --lib words`
Expected: FAIL (noise tokens leak; `merge`/`branch` present).

- [ ] **Step 3: Expand stopwords**

In `src/stats/words.rs`, replace the `STOPWORDS` constant with:

```rust
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "this", "that", "from", "into", "are", "was", "but", "not", "you",
    "your", "all", "can", "use", "add", "now", "out", "merge", "branch", "pull", "request", "pr",
    "wip", "via", "git",
];
```

- [ ] **Step 4: Add URL/domain stripping and number/hex token rejection**

In `src/stats/words.rs`, add these helpers above `top_words`:

```rust
/// Remove `http(s)://…` and `www.…` URLs and bare `host.tld` domains from a
/// line before tokenization, so the word cloud is not flooded with `https`,
/// `github`, `com`, etc. Replaces each match with a space to preserve adjacency
/// breaks.
fn strip_urls(line: &str) -> String {
    const TLDS: &[&str] = &[".com", ".org", ".io", ".net", ".dev", ".gov", ".edu"];
    let mut out = String::with_capacity(line.len());
    for token in line.split_whitespace() {
        let lower = token.to_lowercase();
        let is_url = lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("www.")
            || TLDS.iter().any(|t| lower.contains(t));
        if is_url {
            out.push(' ');
        } else {
            out.push_str(token);
        }
        out.push(' ');
    }
    out
}

/// True when a token is a pure number or a short-SHA-looking hex string
/// (length >= 7, all hex digits) — noise rather than a word.
fn is_number_or_hash(word: &str) -> bool {
    if word.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    word.len() >= 7 && word.chars().all(|c| c.is_ascii_hexdigit())
}
```

- [ ] **Step 5: Apply the filtering in `top_words` and `top_bigrams`**

In `top_words`, change the line loop body so each line is passed through `strip_urls`, and reject number/hash tokens. Replace the inner loop:

```rust
    for r in records {
        for line in r.message.lines().filter(|l| !is_noise_line(l)) {
            let cleaned = strip_urls(line);
            for raw in cleaned.split(|c: char| !c.is_alphanumeric()) {
                let w = raw.to_lowercase();
                if w.len() < 3 || STOPWORDS.contains(&w.as_str()) || is_number_or_hash(&w) {
                    continue;
                }
                *by.entry(w).or_default() += 1;
            }
        }
    }
```

In `top_bigrams`, mirror it — `strip_urls` the line, and treat a number/hash token like a dropped stopword (break adjacency):

```rust
    for r in records {
        for line in r.message.lines().filter(|l| !is_noise_line(l)) {
            let cleaned = strip_urls(line);
            let mut prev: Option<String> = None;
            for raw in cleaned.split(|c: char| !c.is_alphanumeric()) {
                if raw.is_empty() {
                    continue;
                }
                let w = raw.to_lowercase();
                if w.len() < 3 || STOPWORDS.contains(&w.as_str()) || is_number_or_hash(&w) {
                    prev = None;
                    continue;
                }
                if let Some(p) = &prev {
                    *by.entry(format!("{p} {w}")).or_default() += 1;
                }
                prev = Some(w);
            }
        }
    }
```

- [ ] **Step 6: Run all word tests to verify they pass**

Run: `cargo test --lib words`
Expected: PASS (all existing tests plus the 2 new ones).

- [ ] **Step 7: Commit**

```bash
git add src/stats/words.rs
git commit -m "feat(stats): strip URLs/domains/numbers and expand commit-word stopwords"
```

---

## Task 5: Chronotype — night-owl & early-bird leaders

**Files:**
- Modify: `src/stats/nightowl.rs` (new `Chronotype` type, new fields, computation)

- [ ] **Step 1: Add a failing test**

In `src/stats/nightowl.rs`, inside `mod tests`, add (the `SAT_3AM` / `MON_10AM` consts already exist; add a morning const and a helper to repeat a commit):

```rust
    const MON_7AM: i64 = 1_704_697_200; // 2024-01-08T07:00:00Z (Monday, morning)

    #[test]
    fn night_owls_ranked_by_share_with_min_five_commits() {
        // Owl: 5 commits, all at 03:00 -> 100% night, eligible.
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
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib nightowl`
Expected: FAIL to compile (no `night_owls` / `early_birds` fields, no `Chronotype`).

- [ ] **Step 3: Add the `Chronotype` type and fields**

In `src/stats/nightowl.rs`, add the struct (near `WeekendWarrior`):

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Chronotype {
    pub name: String,
    pub night_pct: f64,
    pub morning_pct: f64,
    pub total: usize,
}
```

Add fields to `NightOwlStats`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct NightOwlStats {
    pub histogram: HourHistogram,
    pub warriors: Vec<WeekendWarrior>,
    pub night_owls: Vec<Chronotype>,
    pub early_birds: Vec<Chronotype>,
}
```

- [ ] **Step 4: Compute chronotypes**

In `night_owls()`, after the existing histogram/warrior loop but before building `NightOwlStats`, accumulate per-author night/morning/total counts and derive the two leaderboards. Add this computation:

```rust
    // Chronotype: night = 22:00-04:59, morning = 05:00-08:59 (local hour).
    let mut chrono: HashMap<&str, (usize, usize, usize)> = HashMap::new(); // (night, morning, total)
    for r in records {
        let hour = local(r).hour();
        let e = chrono.entry(r.author_name.as_str()).or_default();
        if matches!(hour, 22 | 23 | 0 | 1 | 2 | 3 | 4) {
            e.0 += 1;
        }
        if matches!(hour, 5 | 6 | 7 | 8) {
            e.1 += 1;
        }
        e.2 += 1;
    }
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
    let mut night_owls: Vec<Chronotype> = chronotypes.clone();
    night_owls.sort_by(|a, b| {
        b.night_pct
            .partial_cmp(&a.night_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total.cmp(&a.total))
            .then(a.name.cmp(&b.name))
    });
    let mut early_birds: Vec<Chronotype> = chronotypes;
    early_birds.sort_by(|a, b| {
        b.morning_pct
            .partial_cmp(&a.morning_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total.cmp(&a.total))
            .then(a.name.cmp(&b.name))
    });
```

Then update the returned struct:

```rust
    NightOwlStats {
        histogram: HourHistogram { hours },
        warriors,
        night_owls,
        early_birds,
    }
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --lib nightowl`
Expected: PASS (existing 2 tests + 2 new ones).

- [ ] **Step 6: Commit**

```bash
git add src/stats/nightowl.rs
git commit -m "feat(stats): add night-owl and early-bird chronotype leaders"
```

---

## Task 6: `stats/oops.rs` — oops counter

**Files:**
- Create: `src/stats/oops.rs`
- Modify: `src/stats/mod.rs` (add `pub mod oops;`)

- [ ] **Step 1: Register the module**

In `src/stats/mod.rs`, add:

```rust
pub mod oops;
```

- [ ] **Step 2: Write the failing tests**

Create `src/stats/oops.rs`:

```rust
//! "Oops" counter — commits whose first line confesses a mistake.

use crate::model::CommitRecord;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OopsStat {
    pub name: String,
    pub oops: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OopsBoard {
    pub total_oops: usize,
    pub leaders: Vec<OopsStat>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileChurn;

    fn msg(author: &str, m: &str) -> CommitRecord {
        CommitRecord {
            sha: "s".into(),
            author_name: author.into(),
            author_email: "a@x".into(),
            timestamp: 0,
            tz_offset_minutes: 0,
            message: m.into(),
            files: vec![FileChurn { path: "x".into(), added: 1, removed: 0 }],
        }
    }

    #[test]
    fn counts_keyword_and_fix_fix_phrase_per_author() {
        let records = vec![
            msg("alice", "oops, forgot the import"),
            msg("alice", "fix fix the build"),
            msg("alice", "add real feature"),
            msg("bob", "Revert \"add feature\""),
        ];
        let board = oops_board(&records);
        assert_eq!(board.total_oops, 3);
        let alice = board.leaders.iter().find(|s| s.name == "alice").unwrap();
        assert_eq!(alice.oops, 2);
        assert_eq!(alice.total, 3);
        assert!(board.leaders.iter().any(|s| s.name == "bob" && s.oops == 1));
    }

    #[test]
    fn whole_word_only_no_substring_false_positives() {
        // "reverted" / "actuator" should NOT match "revert" / "actual"... but
        // "actually" IS in the keyword set, so test a non-keyword substring.
        let records = vec![msg("carol", "improve actuator timing")];
        let board = oops_board(&records);
        assert_eq!(board.total_oops, 0);
        assert!(board.leaders.is_empty());
    }

    #[test]
    fn empty_input_is_empty() {
        let board = oops_board(&[]);
        assert_eq!(board.total_oops, 0);
        assert!(board.leaders.is_empty());
    }
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test --lib oops`
Expected: FAIL to compile (`oops_board` not found).

- [ ] **Step 4: Write the implementation**

In `src/stats/oops.rs`, insert above the test module (below the struct definitions):

```rust
/// Whole-word keywords that mark a commit as an "oops".
const OOPS_WORDS: &[&str] = &[
    "oops", "whoops", "typo", "wip", "revert", "fixup", "argh", "damn", "nvm", "broken", "forgot",
    "accidentally", "actually",
];

/// True when the commit's first line confesses a mistake: it contains the
/// phrase `fix fix`, or any `OOPS_WORDS` keyword as a whole (alphanumeric-
/// delimited) word.
fn is_oops(message: &str) -> bool {
    let first = message.lines().next().unwrap_or("");
    let lower = first.to_lowercase();
    if lower.contains("fix fix") {
        return true;
    }
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|tok| OOPS_WORDS.contains(&tok))
}

/// Per-author "oops" leaderboard plus the repo-wide total.
pub fn oops_board(records: &[CommitRecord]) -> OopsBoard {
    let mut by: HashMap<&str, (usize, usize)> = HashMap::new(); // (oops, total)
    let mut total_oops = 0;
    for r in records {
        let e = by.entry(r.author_name.as_str()).or_default();
        e.1 += 1;
        if is_oops(&r.message) {
            e.0 += 1;
            total_oops += 1;
        }
    }
    let mut leaders: Vec<OopsStat> = by
        .into_iter()
        .filter(|(_, (oops, _))| *oops >= 1)
        .map(|(name, (oops, total))| OopsStat {
            name: name.to_string(),
            oops,
            total,
        })
        .collect();
    leaders.sort_by(|a, b| b.oops.cmp(&a.oops).then(a.name.cmp(&b.name)));
    OopsBoard { total_oops, leaders }
}
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --lib oops`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add src/stats/oops.rs src/stats/mod.rs
git commit -m "feat(stats): add per-author oops counter"
```

---

## Task 7: `stats/busiest.rs` — busiest day

**Files:**
- Create: `src/stats/busiest.rs`
- Modify: `src/stats/mod.rs` (add `pub mod busiest;`)

- [ ] **Step 1: Register the module**

In `src/stats/mod.rs`, add:

```rust
pub mod busiest;
```

- [ ] **Step 2: Write the failing tests**

Create `src/stats/busiest.rs`:

```rust
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
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test --lib busiest`
Expected: FAIL to compile (`busiest_day` not found).

- [ ] **Step 4: Write the implementation**

In `src/stats/busiest.rs`, insert above the test module (below the struct):

```rust
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
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --lib busiest`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add src/stats/busiest.rs src/stats/mod.rs
git commit -m "feat(stats): add busiest-day stat"
```

---

## Task 8: `stats/battlefield.rs` — file battlefield

**Files:**
- Create: `src/stats/battlefield.rs`
- Modify: `src/stats/mod.rs` (add `pub mod battlefield;`)

- [ ] **Step 1: Register the module**

In `src/stats/mod.rs`, add:

```rust
pub mod battlefield;
```

- [ ] **Step 2: Write the failing tests**

Create `src/stats/battlefield.rs`:

```rust
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
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test --lib battlefield`
Expected: FAIL to compile (`file_battlefield` not found).

- [ ] **Step 4: Write the implementation**

In `src/stats/battlefield.rs`, insert above the test module (below the struct):

```rust
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
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --lib battlefield`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add src/stats/battlefield.rs src/stats/mod.rs
git commit -m "feat(stats): add file-battlefield stat"
```

---

## Task 9: Wire scoreboard, CLI, and main

**Files:**
- Modify: `src/scoreboard.rs` (imports, struct fields, `analyze` signature, spawns, test)
- Modify: `src/cli.rs` (new flag)
- Modify: `src/main.rs:87-93` (collapse + flag)

- [ ] **Step 1: Update the scoreboard test for the new signature/fields**

In `src/scoreboard.rs`, replace the `#[cfg(test)] mod tests { ... }` block with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;

    #[test]
    fn analyze_populates_all_panels() {
        let records = vec![
            rec("alice", 1_704_067_200, &[("a.rs", 5, 1)]),
            rec("bob", 1_704_153_600, &[("a.rs", 2, 0)]),
        ];
        let sb = analyze(&records, false);
        assert_eq!(sb.committers.len(), 2);
        assert_eq!(sb.churn[0].path, "a.rs");
        assert!(sb.biggest.is_some());
        assert_eq!(sb.types.iter().map(|t| t.count).sum::<usize>(), 2);
        assert!(sb.bigrams.is_empty());
        assert!(sb.vitals.is_some());
        assert_eq!(sb.vitals.unwrap().total_commits, 2);
        // New panels exist (a.rs has 2 authors -> battlefield).
        assert_eq!(sb.oops.total_oops, 0);
        assert!(sb.busiest.is_some());
        assert!(sb.battlefield.iter().any(|b| b.path == "a.rs"));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib scoreboard`
Expected: FAIL to compile (`analyze` takes 1 arg; no `oops`/`busiest`/`battlefield` fields).

- [ ] **Step 3: Update imports and struct**

In `src/scoreboard.rs`, extend the `use crate::stats::{…}` block with:

```rust
    battlefield::{file_battlefield, Battlefield},
    busiest::{busiest_day, BusiestDay},
    oops::{oops_board, OopsBoard},
```

(Keep the existing imports; add these alphabetically within the brace list.)

Add fields to `Scoreboard`:

```rust
    pub oops: OopsBoard,
    pub busiest: Option<BusiestDay>,
    pub battlefield: Vec<Battlefield>,
```

- [ ] **Step 4: Update `analyze`**

Change the signature and add the three spawns. Replace the function:

```rust
/// Run all independent analyzers concurrently over the same commit slice.
pub fn analyze(records: &[CommitRecord], include_generated: bool) -> Scoreboard {
    let mut committers = None;
    let mut churn = None;
    let mut biggest = None;
    let mut nightowls = None;
    let mut streaks = None;
    let mut words = None;
    let mut types = None;
    let mut bigrams = None;
    let mut ownership_ = None;
    let mut vitals_ = None;
    let mut oops_ = None;
    let mut busiest_ = None;
    let mut battlefield_ = None;

    rayon::scope(|s| {
        s.spawn(|_| committers = Some(top_committers(records)));
        s.spawn(|_| churn = Some(churn_hotspots(records, include_generated)));
        s.spawn(|_| biggest = Some(biggest_commit(records)));
        s.spawn(|_| nightowls = Some(night_owls(records)));
        s.spawn(|_| streaks = Some(longest_streaks(records)));
        s.spawn(|_| words = Some(top_words(records, 30)));
        s.spawn(|_| types = Some(commit_types(records)));
        s.spawn(|_| bigrams = Some(top_bigrams(records, 6)));
        s.spawn(|_| ownership_ = Some(ownership(records)));
        s.spawn(|_| vitals_ = Some(vitals(records)));
        s.spawn(|_| oops_ = Some(oops_board(records)));
        s.spawn(|_| busiest_ = Some(busiest_day(records)));
        s.spawn(|_| battlefield_ = Some(file_battlefield(records, include_generated)));
    });

    Scoreboard {
        committers: committers.unwrap(),
        churn: churn.unwrap(),
        biggest: biggest.unwrap(),
        nightowls: nightowls.unwrap(),
        streaks: streaks.unwrap(),
        words: words.unwrap(),
        types: types.unwrap(),
        bigrams: bigrams.unwrap(),
        ownership: ownership_.unwrap(),
        vitals: vitals_.unwrap(),
        oops: oops_.unwrap(),
        busiest: busiest_.unwrap(),
        battlefield: battlefield_.unwrap(),
    }
}
```

- [ ] **Step 5: Add the CLI flag**

In `src/cli.rs`, add a field to `Args` (after `no_color`):

```rust
    /// Include lock / generated / vendored files in churn & battlefield stats
    #[arg(long)]
    pub include_generated: bool,
```

And update the existing `parses_limit_and_no_color` test to assert the default:

```rust
        assert!(!a.include_generated);
```

(Add that line inside the existing test body.)

- [ ] **Step 6: Wire `main.rs`**

In `src/main.rs`, add the import near the other `git_stats::` uses:

```rust
use git_stats::stats::identity::collapse_identities;
```

Replace lines around the sort + analyze (currently `records.sort_by_key(...)` then `let sb = analyze(&records);`):

```rust
    records.sort_by_key(|r| r.timestamp);
    let records = collapse_identities(records);

    let sb = analyze(&records, args.include_generated);
    run_scoreboard(&mut term, &sb)?;
```

- [ ] **Step 7: Run the full test + build to verify**

Run: `cargo test --lib scoreboard cli` then `cargo build`
Expected: PASS, and `cargo build` succeeds (the `tui` panels test still compiles because it calls `analyze` — fixed in Task 10 if needed; if `cargo build` for the lib passes but tui test fails to compile, proceed to Task 10).

Note: the `tui/panels.rs` test calls `analyze(&[...])` and will fail to compile until Task 10. If running the whole `cargo test` now, expect that one compile error; `cargo test --lib scoreboard cli` isolates this task.

- [ ] **Step 8: Commit**

```bash
git add src/scoreboard.rs src/cli.rs src/main.rs
git commit -m "feat: collapse identities and wire oops/busiest/battlefield + --include-generated"
```

---

## Task 10: TUI panels & tabs

**Files:**
- Modify: `src/tui/panels.rs` (extend night-owl panel; add oops/busiest/battlefield widgets; fix test call)
- Modify: `src/tui/app.rs` (tab titles + match arms)

- [ ] **Step 1: Fix the existing panels test call site**

In `src/tui/panels.rs`, in the test `renders_committers_without_panic_and_shows_name`, change:

```rust
        let sb = analyze(&[rec("alice", 1_704_067_200, &[("a.rs", 5, 1)])]);
```

to:

```rust
        let sb = analyze(&[rec("alice", 1_704_067_200, &[("a.rs", 5, 1)])], false);
```

- [ ] **Step 2: Extend the night-owl widget with chronotype leaders**

In `src/tui/panels.rs`, in `nightowls_widget`, after the weekend-warrior loop (after the `for w in sb.nightowls.warriors...` block) and before `Paragraph::new(lines)...`, add:

```rust
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Biggest night owls", header_style())));
    for c in sb.nightowls.night_owls.iter().take(3) {
        lines.push(Line::from(format!(
            "{:<20} {:.0}% night ({} commits)",
            c.name, c.night_pct, c.total
        )));
    }
    lines.push(Line::from(Span::styled("Earliest birds", header_style())));
    for c in sb.nightowls.early_birds.iter().take(3) {
        lines.push(Line::from(format!(
            "{:<20} {:.0}% morning ({} commits)",
            c.name, c.morning_pct, c.total
        )));
    }
```

- [ ] **Step 3: Add the three new widgets**

In `src/tui/panels.rs`, add these functions (after `vitals_widget`):

```rust
pub fn oops_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("{} oops commits total", sb.oops.total_oops),
        accent_style(),
    ))];
    for (i, o) in sb.oops.leaders.iter().take(10).enumerate() {
        lines.push(Line::from(format!(
            "{} {:<20} {} oops / {} commits",
            medal(i),
            o.name,
            o.oops,
            o.total
        )));
    }
    Paragraph::new(lines).block(panel_block(" Oops Counter "))
}

pub fn busiest_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let text = match &sb.busiest {
        Some(b) => format!(
            "📅 {}\n{} commits\ntop: {} ({} commits)",
            b.date, b.commits, b.top_author, b.top_author_commits
        ),
        None => "no commits".to_string(),
    };
    Paragraph::new(text).block(panel_block(" Busiest Day "))
}

pub fn battlefield_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let lines: Vec<Line> = sb
        .battlefield
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, b)| {
            Line::from(format!(
                "{} {:<30} {} authors, {} commits",
                medal(i),
                b.path,
                b.authors,
                b.commits
            ))
        })
        .collect();
    Paragraph::new(lines).block(panel_block(" File Battlefield "))
}
```

- [ ] **Step 4: Add tabs and route them**

In `src/tui/app.rs`, extend `TAB_TITLES`:

```rust
pub const TAB_TITLES: &[&str] = &[
    "Committers",
    "Churn",
    "Biggest",
    "Night Owls",
    "Streaks",
    "Words",
    "Ownership",
    "Vitals",
    "Oops",
    "Busiest",
    "Battlefield",
];
```

In the `match state.tab` block, replace the `_ => vitals` arm with explicit arms for vitals + the three new panels:

```rust
                7 => f.render_widget(panels::vitals_widget(sb), chunks[2]),
                8 => f.render_widget(panels::oops_widget(sb), chunks[2]),
                9 => f.render_widget(panels::busiest_widget(sb), chunks[2]),
                _ => f.render_widget(panels::battlefield_widget(sb), chunks[2]),
```

- [ ] **Step 5: Add a render smoke test for a new panel**

In `src/tui/panels.rs` `mod tests`, add:

```rust
    #[test]
    fn renders_oops_panel_without_panic() {
        let sb = analyze(
            &[
                rec("alice", 1_704_067_200, &[("a.rs", 5, 1)]),
                rec("bob", 1_704_153_600, &[("a.rs", 2, 0)]),
            ],
            false,
        );
        let backend = TestBackend::new(60, 20);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let w = oops_widget(&sb);
            f.render_widget(w, f.area());
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("oops commits total"));
    }
```

- [ ] **Step 6: Run the full test suite**

Run: `cargo test`
Expected: PASS (all tests across all modules).

- [ ] **Step 7: Clippy + build**

Run: `cargo clippy --all-targets --all-features --locked -- -D warnings`
Expected: no warnings. Fix any that appear, then re-run.

- [ ] **Step 8: Commit**

```bash
git add src/tui/panels.rs src/tui/app.rs
git commit -m "feat(tui): add oops/busiest/battlefield panels and chronotype leaders"
```

---

## Task 11: Manual smoke test & docs

**Files:**
- Modify: `CLAUDE.md` (optional — note the new `--include-generated` flag if a flags list exists)

- [ ] **Step 1: Run against this repo**

Run: `cargo run --release` and tab through to **Night Owls** (chronotype leaders visible), **Oops**, **Busiest**, **Battlefield**. Confirm churn no longer shows `Cargo.lock`. Quit with `q`.

- [ ] **Step 2: Run with the escape hatch**

Run: `cargo run --release -- --include-generated` and confirm `Cargo.lock` reappears in Churn / Battlefield.

- [ ] **Step 3: Final full verification**

Run: `cargo test && cargo clippy --all-targets --all-features --locked -- -D warnings`
Expected: all green.

- [ ] **Step 4: Commit any doc tweaks (if made)**

```bash
git add -A
git commit -m "docs: note --include-generated flag"
```

---

## Self-Review notes

- **Spec coverage:** identity collapse (Task 2), generated-file filter (Task 1), churn filtering (Task 3), word cleanup (Task 4), chronotype/night-owl rank (Task 5), oops (Task 6), busiest (Task 7), battlefield (Task 8), wiring + flag + main (Task 9), TUI (Task 10). All spec sections map to a task.
- **Type consistency:** `analyze(records, include_generated)`, `churn_hotspots(records, include_generated)`, `file_battlefield(records, include_generated)`, `oops_board(records)`, `busiest_day(records)`, `collapse_identities(records)`, `Chronotype { name, night_pct, morning_pct, total }`, `OopsBoard { total_oops, leaders }`, `BusiestDay { date, commits, top_author, top_author_commits }`, `Battlefield { path, authors, commits }` — names used identically in defining and consuming tasks.
- **Non-goals honored:** biggest/ownership unchanged; no config file; no fuzzy name matching.
