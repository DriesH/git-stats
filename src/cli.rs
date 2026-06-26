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
    }

    #[test]
    fn since_absolute_date_parses_to_timestamp() {
        // 2024-01-01T00:00:00Z == 1704067200
        let ts = parse_since("2024-01-01").unwrap();
        assert_eq!(ts, 1_704_067_200);
    }
}
