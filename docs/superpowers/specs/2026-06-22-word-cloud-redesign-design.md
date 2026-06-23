# Word Cloud Redesign — Design

Date: 2026-06-22
Status: Approved (pending spec review)

## Problem

The Commit Word Cloud tab renders each top word followed by a raw run of `▪`
glyphs (`{:<15} {}`, `"▪".repeat(count.min(40))`). It reads as a flat bar
list with no scale, no color, and no semantic depth — single tokens like `fix`
and `add` carry little meaning. It is the least informative tab.

## Goal

Turn the tab into an informative summary of *what the project's commits are
about*, addressing four itches:

- **A — Scale:** show counts and percentages, bars normalized to a real max.
- **B — Color:** top entries pop; degrades cleanly under `--no-color`.
- **C — Depth:** add commit-type breakdown and two-word phrases.
- **D — Layout:** the single-word view reads like a cloud, not a chart.

## Layered changes

Layers stay separated per the working agreement: `stats/` pure, `scoreboard`
wires, `tui/` renders.

### stats/ (pure, TDD first)

Two new functions in `src/stats/words.rs` (alongside existing `top_words`,
which is unchanged and feeds section 3).

**`commit_types(&[CommitRecord]) -> Vec<TypeCount>`**

```rust
pub struct TypeCount {
    pub kind: String,   // e.g. "feat", "fix", "other"
    pub count: usize,
}
```

Classification, per commit, from the first line of `message`:

1. **Conventional prefix.** Match `^\s*(type)(\(scope\))?!?\s*:` case-insensitively
   where `type` is one of the known set below. The captured `type` (lowercased)
   is the kind.
2. **Keyword inference** when no prefix matches. Scan the lowercased first line
   for the first hit:
   - `fix`, `bug`, `patch`, `hotfix` → `fix`
   - `add`, `new`, `introduce`, `feature` → `feat`
   - `doc`, `docs`, `readme` → `docs`
   - `refactor`, `cleanup`, `rename` → `refactor`
   - `test`, `tests` → `test`
   - `chore`, `bump`, `deps`, `dependency` → `chore`
3. **Fallback** → `other`.

Known conventional set: `feat fix docs refactor test chore style perf build ci
revert`. Plus `other` as a catch-all. `revert` is recognized as a prefix only
(no keyword rule).

Returns every kind that occurred, sorted by `count` desc then `kind` asc. Empty
input → empty vec.

**`top_bigrams(&[CommitRecord], limit) -> Vec<WordCount>`**

Adjacent token pairs from each commit message. Reuses the existing tokenization
(`split` on non-alphanumeric, lowercase, `len < 3` and `STOPWORDS` filtered).
A bigram is two *consecutive surviving tokens within the same message* — a
filtered/dropped token breaks adjacency (no pairing across a removed stopword).
`word` field holds `"first second"`. Sorted by `count` desc then `word` asc,
truncated to `limit`. Reuses `WordCount`.

### scoreboard.rs

Add two fields to `Scoreboard`, populated where `words` is built today:

```rust
pub types: Vec<TypeCount>,    // commit_types(records)
pub bigrams: Vec<WordCount>,  // top_bigrams(records, 6)
```

`words` continues to be `top_words(records, 15)`.

### tui/panels.rs — `words_widget`

One `Paragraph`, three labeled sections separated by blank lines. Section
headers use `header_style()`.

1. **By type** — every entry in `sb.types`. Per row:
   `{kind:<9} {bar} {count:>4} {pct:>3}%`. Bar is `█` repeated, length
   `count * 16 / max` (min 1 when count > 0), `max` = the largest count in
   `sb.types`. `pct` = `count * 100 / total` (integer), where `total` is the
   sum of all type counts — every commit is classified into exactly one kind,
   so this equals the commit count (no extra field needed). Rank-0 row uses
   `accent_style()`, rest plain.

2. **Top phrases** — `sb.bigrams` (≤ 6). Per row:
   `{phrase:<16} {bar} {count:>3}`. Bar is `▇` repeated, length normalized to
   the bigram max. Rank-0 uses `accent_style()`.

3. **Top words** — `sb.words` rendered inline as a compact cloud:
   `word·count  word·count  …` wrapped across lines. Brightness tiers fake
   "size = frequency": rank 0–2 `accent_style()` (bold for 0), ranks 3–7 plain,
   ranks 8+ dim (`Color::DarkGray`). All color goes through the theme's
   `maybe()` path so `--no-color` collapses to plain text.

No new layout split — the tab already gets full height (`Constraint::Min(0)`).

## Color & no-color

All styling routes through existing `theme` helpers (`header_style`,
`accent_style`) or `maybe(...)`-wrapped styles, so `--no-color` / non-TTY
produces plain, aligned text. A dim tier needs a new helper:

```rust
/// Dim gray — used for low-rank, low-emphasis entries.
pub fn dim_style() -> Style  // maybe(Style::default().fg(Color::DarkGray))
```

## Testing

Pure functions get unit tests first (TDD):

- `commit_types`: prefix match (`feat:`, `Fix(scope):`, `feat!:`), case
  insensitivity, keyword inference, `other` fallback, sort order, empty input.
- `top_bigrams`: adjacency, stopword breaks adjacency, lowercasing, sort +
  truncation, fewer-than-limit input.

TUI rendering is not unit-tested (consistent with current `panels.rs`); verified
by running the app.

## Trailer & boilerplate filtering (added during implementation)

Rendering against real repos showed that git trailers and tool boilerplate
dominated the phrases and word cloud (e.g. `noreply anthropic`, `authored`,
`generated`). `top_words` and `top_bigrams` therefore exclude noise lines
before tokenizing:

- Lines starting with `🤖` or containing `generated with` (case-insensitive).
- `Key: value` trailer lines whose key (alphabetic/hyphen only) is one of:
  `co-authored-by`, `signed-off-by`, `reviewed-by`, `acked-by`, `tested-by`,
  `reported-by`, `suggested-by`, `refs`, `ref`, `cc`. The key-shape check keeps
  conventional-commit subjects like `feat: add x` from being treated as
  trailers.

Both functions iterate per line (skipping noise lines), so phrases also never
bridge a line break. `commit_types` is unaffected — it reads only the subject
(first) line.

## Note on `top_words` limit

The scoreboard keeps the pre-existing `top_words(records, 30)`; the widget
slices `take(15)`. Output is identical to a `15` limit (only 15 render); the
larger fetch is harmless and left unchanged to avoid touching unrelated
behavior.

## Out of scope

- Trigrams / n>2.
- Configurable type rules or stopwords.
- Font-size scaling (terminals can't; brightness tiers approximate it).
