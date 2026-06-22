# git-stats — Design

A Rust CLI that pulls fun statistics out of a git repository and presents them
in a colorful, interactive terminal scoreboard.

## Goal

Point the tool at a git repository and get an entertaining, leaderboard-style
view of who did what: top committers, churn hotspots, the biggest commit, night
owls, streaks, and more. The emphasis is on fun and visual polish (color, ASCII
art, medals) on top of accurate git analysis.

## Scope

In scope for v1:

- Analyze the git repository in the current working directory.
- Eight stat panels (see Panels).
- Interactive full-screen TUI (ratatui) with tab navigation between panels.
- A loading screen with progress bar, estimated time remaining, and graceful
  cancellation while the commit data is being built.
- Parallel commit analysis across all CPU cores.

Out of scope for v1:

- Analyzing arbitrary repository paths (cwd only).
- Remote/host integrations (GitHub, GitLab).
- Exporting to JSON/HTML.
- A profanity/swear-word panel (explicitly dropped).

## CLI

clap (derive API).

```
git-stats [OPTIONS]

OPTIONS:
  --limit <N>     Analyze at most N most-recent commits (default: all)
  --since <DATE>  Only commits on/after DATE (e.g. 2024-01-01, "3 months ago")
  --no-color      Disable colors (also auto-disabled when stdout is not a TTY)
  -h, --help
  -V, --version
```

Rayon uses all cores by default; there is intentionally no `--jobs` flag (YAGNI).

## Architecture

Three layers with strict separation: git I/O, pure analysis, and rendering.
Analyzers never touch git; the TUI never computes statistics.

```
main.rs          wire: CLI -> collect (with loading TUI) -> analyze -> scoreboard TUI
cli.rs           clap args, parse --since into a timestamp filter
model.rs         CommitRecord, FileChurn, Scoreboard, per-panel result structs

git/
  collect.rs     open cwd repo, revwalk -> Vec<Oid>, parallel diff -> Vec<CommitRecord>
                 takes a progress handle (AtomicUsize) and a cancel flag (AtomicBool)

stats/           pure functions: &[CommitRecord] -> typed result
  committers.rs    top committers by commit count
  churn.rs         files by lines added+removed
  biggest.rs       single commit with most lines changed
  nightowl.rs      commits by hour-of-day + weekend % per author
  streaks.rs       longest consecutive-day commit streak per author
  words.rs         most common words in commit messages (no profanity tally)
  ownership.rs     file ownership / bus factor over hotspots
  vitals.rs        repo age, total commits, first/last commit, commits/day

tui/
  theme.rs       colors, medals, ASCII banner art
  loading.rs     progress bar + spinner + ETA + cancel handling
  app.rs         scoreboard state: active tab, per-panel scroll
  panels.rs      one render function per panel
```

### Data model

```rust
struct FileChurn { path: String, added: u32, removed: u32 }

struct CommitRecord {
    sha: String,
    author_name: String,
    author_email: String,
    timestamp: i64,        // commit time, unix seconds
    tz_offset_minutes: i32,
    message: String,
    files: Vec<FileChurn>, // first-parent diff vs parent
}

struct Scoreboard {       // bundle of all panel results, owned by the TUI
    committers: ...,
    churn: ...,
    biggest: ...,
    nightowls: ...,
    streaks: ...,
    words: ...,
    ownership: ...,
    vitals: ...,
}
```

## Data flow

```
1. Parse CLI.
2. Open repo in cwd. If not a repo -> friendly error + exit.
3. revwalk (newest-first, first-parent) -> Vec<Oid>.
   Apply --limit and --since here. This is cheap and gives the total for ETA.
4. Spawn worker: rayon par_iter over the Oids computes CommitRecord per commit.
   Main thread runs the loading TUI.
5. On completion -> sort records by timestamp -> run the 8 analyzers in parallel
   -> assemble Scoreboard.
6. Launch the scoreboard TUI. User tabs between panels; q quits.
7. Restore terminal on every exit path (RAII guard).
```

## Parallelism

The expensive work is computing a diff per commit. Plan:

1. revwalk collects all `Oid`s on a single thread (cheap).
2. `oids.par_iter().map_init(|| Repository::open("."), |repo, oid| ...)` fans the
   diff work across all cores. `git2::Repository` is not `Sync`, so each rayon
   worker opens its own handle once via `map_init` and reuses it across its
   chunk. libgit2 is thread-safe when each thread uses a separate handle.
3. Progress: a shared `AtomicUsize` is incremented per finished commit; the
   loading screen reads it for the bar and ETA.
4. Cancellation: the parallel pass checks a shared `AtomicBool` each commit and
   bails early when set.
5. Each diff result carries its source index; results are sorted once after the
   pass so commit/time order is deterministic (streaks and vitals need it).

The 8 analyzers are independent pure functions over the same slice and run
concurrently (rayon join/scope). Cheap relative to collection, but free.

## Loading screen

Runs while step 4 collects records.

- Spinner + progress bar: `[████████░░░░] 1240 / 5000 commits`.
- ETA: `elapsed / done * remaining`, shown only after ~1% is done so the early
  estimate is not noise.
- Non-blocking key poll on a ~16ms tick. `q`, `Esc`, or `Ctrl-C` set the cancel
  flag, join the worker, restore the terminal, and exit cleanly (no panic, no
  broken terminal).
- Total comes from the revwalk count in step 3, so the bar is accurate from the
  start.

## TUI (scoreboard)

- ratatui + crossterm, full screen, alternate buffer.
- Tabs across the top: Committers · Churn · Biggest · Night Owls · Streaks ·
  Words · Ownership · Vitals.
- Navigation: `Left`/`Right` (or `Tab`/`h`/`l`) switch panels; `Up`/`Down`
  scroll within a panel; `q` quits.
- Theme: bold colors, 🥇🥈🥉 medals for top-3 rows, an ASCII banner header.
- Colors auto-disable when stdout is not a TTY or `--no-color` is set.

## Error handling

- `anyhow` with `.context(...)` end-to-end; library-module errors bubble up via
  `?` and gain context at the binary. (No `thiserror` in v1 — no public typed
  error enum is needed yet; add it if/when one is.)
- Not a git repo -> clear message, non-zero exit, no backtrace shown to user.
- Empty repo (no commits) -> "no commits yet" screen rather than a crash.
- A diff that fails for a single commit is logged and skipped; the run
  continues. (e.g. unusual root/merge cases.)
- A RAII terminal guard restores the terminal on any exit, including panic.

## Testing

- Analyzers (the bulk of the logic) are pure functions over `Vec<CommitRecord>`
  and are unit-tested with hand-built records. Built test-first (TDD).
- One integration test builds a temporary repo with git2 (a few commits across
  authors/times) and asserts `collect` produces the expected records.
- TUI panels render against ratatui's `TestBackend` as a smoke test (no panic,
  expected headings present).
- Cancellation: a unit test sets the cancel flag and asserts the parallel pass
  returns early.

## CLAUDE.md

The project root gets a `CLAUDE.md` stating the project goal and one hard rule:
**when writing Rust code, the `rust-best-practices` skill must be active.**

## Dependencies (anticipated)

- `clap` (derive) — CLI
- `git2` — libgit2 bindings
- `ratatui` + `crossterm` — TUI
- `rayon` — data parallelism
- `anyhow` — errors
- `chrono` — timestamp/`--since` handling and hour/weekday math
- a figlet/banner crate (e.g. `figlet-rs`) or an embedded banner string
```
