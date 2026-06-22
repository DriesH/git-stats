# git-stats

A Rust CLI that pulls fun stats out of the git repository in the current
directory and shows them in a colorful, interactive ratatui scoreboard.

## Goal

Point the tool at a repo and get an entertaining leaderboard: top committers,
churn hotspots, the biggest commit, night owls, streaks, word frequency,
file ownership / bus factor, and repo vitals. Fun and visual polish on top of
accurate git analysis.

## Working agreement

- **When writing Rust code, the `rust-best-practices` skill MUST be active.**
- Layers stay separated: `git/` does I/O, `stats/` are pure functions over
  `&[CommitRecord]`, `tui/` only renders. Analyzers never touch git; the TUI
  never computes stats.
- TDD: analyzers are pure and unit-tested first.
- Errors: `anyhow` end-to-end; library errors bubble via `?`, context added at the binary.

## Design & plan

- Spec: `docs/superpowers/specs/2026-06-22-git-stats-design.md`
- Plan: `docs/superpowers/plans/2026-06-22-git-stats.md`
