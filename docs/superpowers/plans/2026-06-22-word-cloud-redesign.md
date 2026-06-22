# Word Cloud Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the flat bar-list Word Cloud tab with a three-section panel: commit-type breakdown, top two-word phrases, and a brightness-tiered word cloud — backed by two new pure stats functions.

**Architecture:** Two new pure functions in `stats/words.rs` (`commit_types`, `top_bigrams`), wired into `Scoreboard` in `scoreboard.rs`, rendered by a rewritten `words_widget` in `tui/panels.rs`. A new `dim_style()` theme helper supports the dim tier. Layers stay separated: stats are pure and unit-tested first; the TUI only renders.

**Tech Stack:** Rust, ratatui 0.30, rayon (existing analyzer scope). No new dependencies — conventional-commit prefixes are parsed manually (no `regex`).

**Working agreement:** the `rust-best-practices` skill MUST be active while writing code. TDD for the pure functions. `anyhow` errors are not involved here (pure, infallible functions).

Spec: `docs/superpowers/specs/2026-06-22-word-cloud-redesign-design.md`

---

## File Structure

- **Modify** `src/stats/words.rs` — add `TypeCount`, `commit_types`, `top_bigrams`, and their unit tests. `top_words` and `STOPWORDS` stay as-is and are reused.
- **Modify** `src/scoreboard.rs` — add `types` and `bigrams` fields, populate them in the rayon scope, update the import and the `analyze_populates_all_panels` test.
- **Modify** `src/tui/theme.rs` — add `dim_style()` helper + its no-color test assertion.
- **Modify** `src/tui/panels.rs` — rewrite `words_widget` to render three sections.

---

## Task 1: `commit_types` stats function

**Files:**
- Modify: `src/stats/words.rs`
- Test: `src/stats/words.rs` (inline `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `mod tests` block in `src/stats/words.rs` (the `msg` helper already exists there):

```rust
#[test]
fn commit_types_parses_conventional_prefix() {
    let records = vec![
        msg("feat: add login"),
        msg("Fix(auth): handle expiry"),
        msg("feat!: breaking change"),
        msg("docs: update readme"),
    ];
    let t = commit_types(&records);
    // feat appears twice -> first, sorted by count desc then kind asc
    assert_eq!(t[0].kind, "feat");
    assert_eq!(t[0].count, 2);
    assert!(t.iter().any(|x| x.kind == "fix" && x.count == 1));
    assert!(t.iter().any(|x| x.kind == "docs" && x.count == 1));
}

#[test]
fn commit_types_infers_from_keywords_then_other() {
    let records = vec![
        msg("squashed a nasty bug"), // -> fix (keyword "bug")
        msg("introduce dark mode"),  // -> feat (keyword "introduce")
        msg("rename the module"),    // -> refactor (keyword "rename")
        msg("merge branch main"),    // -> other (no rule)
    ];
    let t = commit_types(&records);
    assert!(t.iter().any(|x| x.kind == "fix" && x.count == 1));
    assert!(t.iter().any(|x| x.kind == "feat" && x.count == 1));
    assert!(t.iter().any(|x| x.kind == "refactor" && x.count == 1));
    assert!(t.iter().any(|x| x.kind == "other" && x.count == 1));
}

#[test]
fn commit_types_empty_input_is_empty() {
    assert!(commit_types(&[]).is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib commit_types`
Expected: FAIL — `cannot find function commit_types` / `cannot find type TypeCount`.

- [ ] **Step 3: Write the minimal implementation**

Add to `src/stats/words.rs` (after the existing `WordCount` struct / `STOPWORDS` const, before `top_words`):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCount {
    pub kind: String,
    pub count: usize,
}

/// Conventional-commit types recognized as an explicit `type:` prefix.
const KNOWN_TYPES: &[&str] = &[
    "feat", "fix", "docs", "refactor", "test", "chore", "style", "perf", "build", "ci", "revert",
];

/// Keyword → kind rules, tried in order when there is no conventional prefix.
/// First rule whose keyword appears as a substring of the lowercased first
/// line wins.
const KEYWORD_RULES: &[(&str, &str)] = &[
    ("fix", "fix"),
    ("bug", "fix"),
    ("patch", "fix"),
    ("hotfix", "fix"),
    ("add", "feat"),
    ("new", "feat"),
    ("introduce", "feat"),
    ("feature", "feat"),
    ("doc", "docs"),
    ("readme", "docs"),
    ("refactor", "refactor"),
    ("cleanup", "refactor"),
    ("rename", "refactor"),
    ("test", "test"),
    ("chore", "chore"),
    ("bump", "chore"),
    ("deps", "chore"),
    ("dependency", "chore"),
];

/// Recognize a leading `type:` / `type(scope):` / `type!:` prefix, returning
/// the lowercased type when it is one of `KNOWN_TYPES`.
fn conventional_prefix(line: &str) -> Option<String> {
    let line = line.trim_start();
    let colon = line.find(':')?;
    let head = &line[..colon];
    let typ = head
        .split('(')
        .next()
        .unwrap_or(head)
        .trim_end_matches('!')
        .trim()
        .to_lowercase();
    KNOWN_TYPES.contains(&typ.as_str()).then_some(typ)
}

/// Classify one commit message into a single kind.
fn classify(message: &str) -> String {
    let first = message.lines().next().unwrap_or("");
    if let Some(kind) = conventional_prefix(first) {
        return kind;
    }
    let lower = first.to_lowercase();
    for (key, kind) in KEYWORD_RULES {
        if lower.contains(key) {
            return (*kind).to_string();
        }
    }
    "other".to_string()
}

/// Count commits by inferred type. Sorted by count desc, then kind asc.
pub fn commit_types(records: &[CommitRecord]) -> Vec<TypeCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        *by.entry(classify(&r.message)).or_default() += 1;
    }
    let mut out: Vec<TypeCount> = by
        .into_iter()
        .map(|(kind, count)| TypeCount { kind, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.kind.cmp(&b.kind)));
    out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib commit_types`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/stats/words.rs
git commit -m "feat(stats): classify commits by conventional/keyword type"
```

---

## Task 2: `top_bigrams` stats function

**Files:**
- Modify: `src/stats/words.rs`
- Test: `src/stats/words.rs` (inline `mod tests`)

- [ ] **Step 1: Write the failing tests**

Add inside the existing `mod tests` block:

```rust
#[test]
fn bigrams_pair_adjacent_surviving_tokens() {
    let records = vec![msg("add login support"), msg("add login support")];
    let b = top_bigrams(&records, 10);
    assert!(b.iter().any(|x| x.word == "add login" && x.count == 2));
    assert!(b.iter().any(|x| x.word == "login support" && x.count == 2));
}

#[test]
fn bigrams_stopword_breaks_adjacency() {
    // "the" is a stopword and is dropped, so it must NOT bridge fix<->bug.
    let records = vec![msg("fix the bug")];
    let b = top_bigrams(&records, 10);
    assert!(!b.iter().any(|x| x.word == "fix bug"));
    assert!(b.is_empty());
}

#[test]
fn bigrams_sorted_and_truncated() {
    let records = vec![
        msg("alpha beta alpha beta"), // "alpha beta" x2, "beta alpha" x1
        msg("gamma delta"),           // "gamma delta" x1
    ];
    let b = top_bigrams(&records, 2);
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].word, "alpha beta");
    assert_eq!(b[0].count, 2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib bigrams`
Expected: FAIL — `cannot find function top_bigrams`.

- [ ] **Step 3: Write the minimal implementation**

Add to `src/stats/words.rs` (after `top_words`):

```rust
/// Count adjacent two-word phrases. A token is kept under the same rule as
/// `top_words` (length >= 3, not a stopword). A dropped token breaks
/// adjacency, so no phrase bridges a removed stopword. Empty splits (from
/// runs of punctuation) are skipped without breaking adjacency.
pub fn top_bigrams(records: &[CommitRecord], limit: usize) -> Vec<WordCount> {
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in records {
        let mut prev: Option<String> = None;
        for raw in r.message.split(|c: char| !c.is_alphanumeric()) {
            if raw.is_empty() {
                continue;
            }
            let w = raw.to_lowercase();
            if w.len() < 3 || STOPWORDS.contains(&w.as_str()) {
                prev = None;
                continue;
            }
            if let Some(p) = &prev {
                *by.entry(format!("{p} {w}")).or_default() += 1;
            }
            prev = Some(w);
        }
    }
    let mut out: Vec<WordCount> = by
        .into_iter()
        .map(|(word, count)| WordCount { word, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    out.truncate(limit);
    out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib bigrams`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/stats/words.rs
git commit -m "feat(stats): extract top two-word phrases (bigrams)"
```

---

## Task 3: Wire `types` and `bigrams` into the Scoreboard

**Files:**
- Modify: `src/scoreboard.rs`

- [ ] **Step 1: Update the failing test**

In `src/scoreboard.rs`, extend `analyze_populates_all_panels` with assertions that the new fields are populated. The two test commits use the default message `"msg"` (from the `rec` helper), which classifies as `other`:

```rust
        assert_eq!(sb.committers.len(), 2);
        assert_eq!(sb.churn[0].path, "a.rs");
        assert!(sb.biggest.is_some());
        assert_eq!(sb.types.iter().map(|t| t.count).sum::<usize>(), 2);
        assert!(sb.bigrams.is_empty()); // "msg" is a single token -> no pairs
        assert!(sb.vitals.is_some());
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib analyze_populates_all_panels`
Expected: FAIL — `no field types on type &Scoreboard`.

- [ ] **Step 3: Implement the wiring**

In `src/scoreboard.rs`:

a) Extend the `words` import line:

```rust
    words::{commit_types, top_bigrams, top_words, TypeCount, WordCount},
```

b) Add fields to the `Scoreboard` struct (after `words`):

```rust
    pub words: Vec<WordCount>,
    pub types: Vec<TypeCount>,
    pub bigrams: Vec<WordCount>,
    pub ownership: Vec<OwnershipStat>,
```

c) In `analyze`, add locals and rayon spawns alongside `words`:

```rust
    let mut words = None;
    let mut types = None;
    let mut bigrams = None;
```

```rust
        s.spawn(|_| words = Some(top_words(records, 30)));
        s.spawn(|_| types = Some(commit_types(records)));
        s.spawn(|_| bigrams = Some(top_bigrams(records, 6)));
```

d) Populate the returned struct (after `words`):

```rust
        words: words.unwrap(),
        types: types.unwrap(),
        bigrams: bigrams.unwrap(),
        ownership: ownership_.unwrap(),
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib analyze_populates_all_panels`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/scoreboard.rs
git commit -m "feat(scoreboard): expose commit types and bigrams"
```

---

## Task 4: Add `dim_style()` theme helper

**Files:**
- Modify: `src/tui/theme.rs`

- [ ] **Step 1: Add the no-color assertion to the existing test**

In `src/tui/theme.rs`, inside `no_color_yields_plain_style`, after the existing `title_style` assertions, add:

```rust
        set_color_enabled(false);
        assert_eq!(dim_style(), ratatui::style::Style::default());
        set_color_enabled(true);
        assert_ne!(dim_style(), ratatui::style::Style::default());
```

(Place these alongside the existing `title_style` checks, keeping the final `set_color_enabled(true)` restore at the end of the test.)

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib no_color_yields_plain_style`
Expected: FAIL — `cannot find function dim_style`.

- [ ] **Step 3: Add the helper**

In `src/tui/theme.rs`, after `accent_style()`:

```rust
/// Dim gray — used for low-rank, low-emphasis entries in the word cloud.
pub fn dim_style() -> Style {
    maybe(Style::default().fg(Color::DarkGray))
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib no_color_yields_plain_style`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/tui/theme.rs
git commit -m "feat(theme): add dim_style helper"
```

---

## Task 5: Rewrite `words_widget` with three sections

**Files:**
- Modify: `src/tui/panels.rs`

No unit test (consistent with the rest of `panels.rs`); verified by build + run in Task 6.

- [ ] **Step 1: Update imports**

At the top of `src/tui/panels.rs`, update the two `use` lines:

```rust
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
```

and extend the theme import to include `dim_style`:

```rust
use crate::tui::theme::{accent_style, dim_style, header_style, medal};
```

- [ ] **Step 2: Replace `words_widget`**

Replace the entire existing `words_widget` function with:

```rust
pub fn words_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let mut lines: Vec<Line> = Vec::new();

    // Section 1 — commit types: bar normalized to the largest type count,
    // plus raw count and percentage of all commits.
    lines.push(Line::from(Span::styled("By type", header_style())));
    let type_max = sb.types.iter().map(|t| t.count).max().unwrap_or(1).max(1);
    let type_total = sb.types.iter().map(|t| t.count).sum::<usize>().max(1);
    for (i, t) in sb.types.iter().enumerate() {
        let bar = "█".repeat((t.count * 16 / type_max).max(1));
        let pct = t.count * 100 / type_total;
        let text = format!("{:<9} {:<16} {:>4} {:>3}%", t.kind, bar, t.count, pct);
        let line = Line::from(text);
        lines.push(if i == 0 { line.style(accent_style()) } else { line });
    }

    lines.push(Line::from(""));

    // Section 2 — top two-word phrases, bar normalized to the largest count.
    lines.push(Line::from(Span::styled("Top phrases", header_style())));
    let bg_max = sb.bigrams.iter().map(|b| b.count).max().unwrap_or(1).max(1);
    for (i, b) in sb.bigrams.iter().enumerate() {
        let bar = "▇".repeat((b.count * 12 / bg_max).max(1));
        let text = format!("{:<16} {:<12} {:>3}", b.word, bar, b.count);
        let line = Line::from(text);
        lines.push(if i == 0 { line.style(accent_style()) } else { line });
    }

    lines.push(Line::from(""));

    // Section 3 — word cloud: `word·count`, brightness tiered by rank to fake
    // size = frequency. Wraps across lines via the Paragraph's Wrap.
    lines.push(Line::from(Span::styled("Top words", header_style())));
    let spans: Vec<Span> = sb
        .words
        .iter()
        .take(15)
        .enumerate()
        .map(|(i, w)| {
            let style = match i {
                0 => accent_style().add_modifier(Modifier::BOLD),
                1..=2 => accent_style(),
                3..=7 => Style::default(),
                _ => dim_style(),
            };
            Span::styled(format!("{}·{}  ", w.word, w.count), style)
        })
        .collect();
    lines.push(Line::from(spans));

    Paragraph::new(lines)
        .block(panel_block(" Commit Word Cloud "))
        .wrap(Wrap { trim: true })
}
```

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo build`
Expected: compiles with no errors. (If `Line::style` is flagged, it is the ratatui 0.30 builder `Line::style(self, Style) -> Line`; it exists.)

- [ ] **Step 4: Commit**

```bash
git add src/tui/panels.rs
git commit -m "feat(tui): render word cloud as type/phrase/word sections"
```

---

## Task 6: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full test suite**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: Run the app against this repo and view the tab**

Run: `cargo run -- .` then press `→` to reach the **Commit Word Cloud** tab.
Expected: three sections render — "By type" with bars/counts/percentages, "Top phrases" with up to 6 bigrams, "Top words" as a wrapped, brightness-tiered cloud. Top rows are colored (yellow); lower words are dim.

- [ ] **Step 4: Verify no-color path**

Run: `cargo run -- --no-color .` then navigate to the tab.
Expected: same layout, plain text, still aligned and readable.

- [ ] **Step 5: Final commit (if any cleanup was needed)**

```bash
git add -A
git commit -m "chore: word cloud redesign cleanup"
```

(Skip if nothing changed in Task 6.)

---

## Self-Review notes

- **Spec coverage:** A (scale: bars normalized + count + %) → Task 5 §1/§2. B (color, no-color safe) → Tasks 4 + 5. C1 (commit types) → Task 1. C2 (bigrams) → Task 2. D (cloud layout) → Task 5 §3. Scoreboard wiring → Task 3. `dim_style` helper from spec → Task 4. `--no-color` → Task 6 §4.
- **`top_words` limit:** scoreboard keeps the existing `top_words(records, 30)`; the widget takes 15. Left unchanged to avoid touching unrelated behavior; spec's "15" is satisfied at render time.
- **Type/name consistency:** `TypeCount { kind, count }`, `commit_types`, `top_bigrams` used identically across Tasks 1–3 and 5. `dim_style` defined in Task 4, used in Task 5.
- **Bigram adjacency:** the stopword-breaks-adjacency rule (spec) is enforced via the `prev = None` reset and covered by `bigrams_stopword_breaks_adjacency`.
