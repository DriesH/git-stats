use chrono::{NaiveDate, TimeZone, Utc};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "git-stats",
    version,
    about = "Fun git repo stats in a TUI scoreboard"
)]
pub struct Args {
    /// Analyze at most N most-recent commits
    #[arg(long)]
    pub limit: Option<usize>,

    /// Only commits on/after this date (YYYY-MM-DD)
    #[arg(long)]
    pub since: Option<String>,

    /// Disable colors
    #[arg(long)]
    pub no_color: bool,

    /// Include lock / generated / vendored files in churn & battlefield stats
    #[arg(long)]
    pub include_generated: bool,

    /// Worker threads for commit collection (default: min(cores, 8))
    #[arg(long)]
    pub jobs: Option<usize>,
}

/// Default cap on collection worker threads. Commit diffing stops scaling past
/// roughly this many threads because libgit2 serializes packfile access, and
/// oversubscribing beyond it regresses wall time.
const DEFAULT_JOBS_CAP: usize = 8;

/// Resolve the number of collection worker threads. An explicit `--jobs` wins
/// (floored at 1 so the pool is always valid); otherwise use the core count
/// capped at [`DEFAULT_JOBS_CAP`].
pub fn resolve_jobs(requested: Option<usize>, cores: usize) -> usize {
    match requested {
        Some(n) => n.max(1),
        None => cores.clamp(1, DEFAULT_JOBS_CAP),
    }
}

/// Parse a YYYY-MM-DD date string into a Unix timestamp (UTC midnight).
pub fn parse_since(s: &str) -> anyhow::Result<i64> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("invalid --since date '{s}': {e}"))?;
    let dt = date.and_hms_opt(0, 0, 0).expect("valid midnight");
    Ok(Utc.from_utc_datetime(&dt).timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_limit_and_no_color() {
        let a = Args::parse_from(["git-stats", "--limit", "50", "--no-color"]);
        assert_eq!(a.limit, Some(50));
        assert!(a.no_color);
        assert!(a.since.is_none());
        assert!(!a.include_generated);
        assert!(a.jobs.is_none());
    }

    #[test]
    fn parses_jobs_override() {
        let a = Args::parse_from(["git-stats", "--jobs", "4"]);
        assert_eq!(a.jobs, Some(4));
    }

    #[test]
    fn resolve_jobs_caps_core_count_by_default() {
        assert_eq!(resolve_jobs(None, 14), 8);
        assert_eq!(resolve_jobs(None, 4), 4);
        assert_eq!(resolve_jobs(None, 0), 1);
    }

    #[test]
    fn resolve_jobs_honors_explicit_override_above_and_below_cap() {
        assert_eq!(resolve_jobs(Some(12), 14), 12);
        assert_eq!(resolve_jobs(Some(0), 14), 1);
    }

    #[test]
    fn since_absolute_date_parses_to_timestamp() {
        // 2024-01-01T00:00:00Z == 1704067200
        let ts = parse_since("2024-01-01").unwrap();
        assert_eq!(ts, 1_704_067_200);
    }
}
