# git-stats: more stats — design spec

Date: 2026-06-26

Extends the existing git-stats scoreboard with author identity collapsing, a
reusable generated-file filter, four improvements to existing stats, and four
new fun stats. The architecture's layer separation is preserved: `git/` does
I/O, `stats/` are pure functions over `&[CommitRecord]`, `tui/` only renders.
All new analyzers are pure and unit-tested first (TDD). Errors bubble via
`anyhow`.

## Scope

**Improvements to existing stats**

1. Night owl: add a per-author "biggest night owl" (and early bird) ranking.
2. Churn: ignore lock/dependency/generated files by default.
3. Commit words: drop URL/domain/number noise and expand stopwords.
4. Author collapse: merge near-duplicate author identities into one.

**New stats**

5. Chronotype rank (folded into night owl — the night-owl/early-bird leaders).
6. Oops counter — per-author leaderboard of "oops"-style commits.
7. Busiest day — the single calendar day with the most commits.
8. File battlefield — files touched by the most distinct authors (excluding
   generated files).

## Foundation 1: author identity collapse

A normalization pass that runs **once over `Vec<CommitRecord>` before**
`analyze()`, so every existing and new stat keyed by author benefits with no
per-stat changes.

**Module:** `stats/identity.rs` (pure, unit-tested first).

**Public API:**

```rust
/// Rewrite each record's author_name / author_email to its cluster's canonical
/// identity. Order of records is preserved.
pub fn collapse_identities(records: Vec<CommitRecord>) -> Vec<CommitRecord>;
```

**Clustering rules.** Two raw identities (a `(name, email)` pair) are merged
when **any** of:

- **A — same email:** emails equal after `trim().to_lowercase()` (ignore empty
  emails; an empty email never matches another).
- **B — same name:** names equal after `trim().to_lowercase()`.
- **C — GitHub noreply handle:** an email of the form
  `<digits>+<handle>@users.noreply.github.com` (or `<handle>@users.noreply.github.com`)
  contributes its `<handle>` (lowercased). A handle matches another identity
  when it equals that identity's lowercased name (with spaces removed) or its
  email local-part (the part before `@`, lowercased).

Implementation: union-find (disjoint set) over the list of distinct raw
identities. For each rule, union the identities that share a key. Rule C is
applied by indexing identities by their derived handle keys (name-without-
spaces, email local-part, and extracted noreply handle) and unioning identities
that collide on any key.

**Canonical identity per cluster:**

- **Display name** = the `author_name` that appears on the most commits in the
  cluster. Ties broken by the lexicographically smallest name (deterministic).
- **Display email** = the `author_email` that appears on the most commits in
  the cluster, ties broken lexicographically. Empty emails only win if the
  cluster has no non-empty email.

**Edge cases:**

- Empty email: does not participate in rule A; the identity can still merge via
  B or C.
- Single-commit identities with a unique name and email form their own cluster.
- Determinism: given the same input multiset, output is identical regardless of
  input order (canonical selection and tie-breaks are order-independent).

**Wiring:** `main.rs` (or wherever records are assembled) calls
`collapse_identities(records)` before `analyze(&records)`.

## Foundation 2: reusable generated-file filter

**Module:** `stats/filters.rs` (pure, unit-tested first).

**Public API:**

```rust
/// True when `path` is a lock file, minified/generated artifact, or lives in a
/// vendored directory — i.e. churn/ownership noise rather than authored code.
pub fn is_generated_path(path: &str) -> bool;
```

**Match set:**

- **Lock files (exact basename):** `Cargo.lock`, `package-lock.json`,
  `yarn.lock`, `pnpm-lock.yaml`, `composer.lock`, `Gemfile.lock`,
  `poetry.lock`, `Pipfile.lock`, `go.sum`, `flake.lock`.
- **Minified / map (suffix):** `*.min.js`, `*.min.css`, `*.map`.
- **Vendored directories (any path component):** `vendor/`, `node_modules/`,
  `dist/`, `build/`.

Matching is on the path string as stored in `FileChurn.path` (repo-relative,
`/`-separated). Basename match uses the final path component; directory match
checks whether any `/`-separated component equals a vendored dir name.

**CLI flag:** `--include-generated` (bool, default false). When set, the
analyzers that use the filter behave as if `is_generated_path` always returns
false (i.e. nothing is excluded).

**Threading the flag:** the two affected analyzers take an `include_generated:
bool` parameter. `scoreboard::analyze` takes the flag and forwards it.

## Improvement 1 / Stat 5: night owl rank + chronotype

**Module:** extend `stats/nightowl.rs`. Keep the existing `HourHistogram` and
weekend-warrior ranking unchanged; **add** chronotype leaders.

Using each commit's **local** hour (existing `local(r)` helper that applies
`tz_offset_minutes`):

- **Night window:** hours 22, 23, 0, 1, 2, 3, 4 (22:00–04:59).
- **Morning window:** hours 5, 6, 7, 8 (05:00–08:59).

**New types & fields:**

```rust
pub struct Chronotype {
    pub name: String,
    pub night_pct: f64,    // share of this author's commits in the night window
    pub morning_pct: f64,  // share in the morning window
    pub total: usize,
}
```

`NightOwlStats` gains:

```rust
pub night_owls: Vec<Chronotype>,   // sorted by night_pct desc
pub early_birds: Vec<Chronotype>,  // sorted by morning_pct desc
```

**Eligibility:** only authors with `total >= 5` commits appear in the
chronotype rankings (avoids 1-commit 100% artifacts). Sort ties broken by
`total` desc then name asc.

Computed over identity-collapsed records (collapse runs upstream, so this is
automatic).

## Improvement 2: churn ignores generated files

`stats/churn.rs::churn_hotspots` gains an `include_generated: bool` parameter.
When false, files where `is_generated_path(path)` is true are skipped during
aggregation. Behavior otherwise unchanged (aggregate added/removed per file,
sort by total desc then path).

## Improvement 3: commit-word cleanup

`stats/words.rs` `top_words` and `top_bigrams` gain extra token filtering on top
of the existing stopword/trailer handling:

- **Drop URL tokens:** when a line contains a URL (`http://`, `https://`, or a
  `www.` host), the URL substring's tokens are excluded. Simplest robust
  approach: before splitting a line into tokens, strip substrings matching a URL
  pattern (scheme `http(s)://` up to whitespace, and `www.` up to whitespace).
  This removes `https`, `github`, `com`, path segments, etc.
- **Drop bare domain/host tokens:** a raw token containing a dot followed by a
  known TLD-ish tail is dropped. Practically, since tokenization splits on
  non-alphanumerics, handle this at the line-strip stage together with URLs
  (strip `\b[\w.-]+\.(com|org|io|net|dev|gov|edu)\b`).
- **Drop pure-number and hex-hash tokens:** a token that is all digits, or all
  hex digits with length ≥ 7 (looks like a short SHA), is dropped.
- **Expanded stopwords:** add `merge`, `branch`, `pull`, `request`, `pr`,
  `wip`, `via`, `git` to the existing `STOPWORDS`. `update` is intentionally
  **kept** as a real word.

These rules apply identically to `top_words` and `top_bigrams` (a dropped token
breaks bigram adjacency, consistent with existing stopword handling).

## Stat 6: oops counter

**Module:** `stats/oops.rs` (pure, unit-tested first).

A commit is an "oops" when its **first line**, lowercased, contains any of these
as a whole word (word-boundary match, case-insensitive):

`oops`, `whoops`, `typo`, `wip`, `revert`, `fixup`, `argh`, `damn`, `nvm`,
`broken`, `forgot`, `accidentally`, `actually`

Plus the literal phrase `fix fix`.

**Public API & types:**

```rust
pub struct OopsStat {
    pub name: String,
    pub oops: usize,    // count of oops commits by this author
    pub total: usize,   // total commits by this author
}

pub struct OopsBoard {
    pub total_oops: usize,        // repo-wide oops commit count
    pub leaders: Vec<OopsStat>,   // authors with >=1 oops, sorted oops desc
}

pub fn oops_board(records: &[CommitRecord]) -> OopsBoard;
```

`leaders` includes only authors with `oops >= 1`, sorted by `oops` desc, ties by
name asc. Computed over collapsed records.

## Stat 7: busiest day

**Module:** `stats/busiest.rs` (pure, unit-tested first).

The single calendar **date** (in each commit's local time, via the same
`tz_offset_minutes` shift) with the most commits.

```rust
pub struct BusiestDay {
    pub date: String,            // "YYYY-MM-DD" (local)
    pub commits: usize,
    pub top_author: String,      // most-committing author that day
    pub top_author_commits: usize,
}

pub fn busiest_day(records: &[CommitRecord]) -> Option<BusiestDay>;
```

Returns `None` for empty input. Group commits by local date; pick the date with
the most commits, ties broken by the **most recent** date. Within that date,
`top_author` is the author with the most commits, ties by name asc. Computed
over collapsed records.

## Stat 8: file battlefield

**Module:** `stats/battlefield.rs` (pure, unit-tested first).

Files touched by the most **distinct** authors, excluding generated files.

```rust
pub struct Battlefield {
    pub path: String,
    pub authors: usize,   // distinct authors who touched the file
    pub commits: usize,   // total commits touching the file
}

pub fn file_battlefield(records: &[CommitRecord], include_generated: bool)
    -> Vec<Battlefield>;
```

For each file (skipping `is_generated_path` unless `include_generated`), count
distinct author names and total touching commits. Only files with `authors >= 2`
are returned. Sorted by `authors` desc, then `commits` desc, then path asc.
Computed over collapsed records (so distinct-author counts use canonical
identities).

## Wiring: scoreboard & TUI

**`scoreboard.rs`:**

- `analyze(records: &[CommitRecord], include_generated: bool) -> Scoreboard`.
- `Scoreboard` gains: `oops: OopsBoard`, `busiest: Option<BusiestDay>`,
  `battlefield: Vec<Battlefield>`. (`night_owls`/`early_birds` ride inside the
  existing `NightOwlStats`.)
- New `rayon::scope` spawns for `oops_board`, `busiest_day`, and
  `file_battlefield` (the latter receiving `include_generated`); `churn_hotspots`
  call updated to pass `include_generated`.

**`main.rs`:** collapse identities and read the `--include-generated` flag
before analyzing:

```rust
let records = collapse_identities(records);
let scoreboard = analyze(&records, args.include_generated);
```

**`cli.rs`:** add `--include-generated` bool flag.

**`tui/panels.rs`:** add panels rendering night-owl/early-bird leaders, the
oops leaderboard (with repo total in the header), busiest day, and file
battlefield. TUI only renders; no computation in the TUI layer.

## Testing

Each pure module is unit-tested first (TDD):

- `identity`: rules A/B/C independently; canonical-name selection; determinism
  across input order; empty-email handling; noreply-handle extraction.
- `filters`: lock basenames, `*.min.js`/`*.map` suffixes, vendored-dir
  components, and non-matching authored paths.
- `nightowl`: night/morning window bucketing; min-5 eligibility; sort order.
- `churn`: generated files excluded when flag false, included when true.
- `words`: URL/domain/number/hex stripping; new stopwords; `update` survives;
  bigram adjacency broken by dropped tokens.
- `oops`: keyword/word-boundary matches, `fix fix` phrase, per-author counts,
  repo total.
- `busiest`: local-date grouping, tie-break to most recent date, top author.
- `battlefield`: distinct-author counting, `>=2` threshold, generated exclusion,
  sort order.
- `scoreboard`: `analyze` populates all new panels.

## Non-goals (YAGNI)

- No fuzzy/edit-distance name matching (rule D "same local-part" was considered
  and rejected as too aggressive).
- No config file for the generated-file set or oops keywords; the in-code
  defaults plus `--include-generated` are sufficient.
- Biggest commit and ownership are **not** changed to exclude generated files
  (intentionally show reality).
