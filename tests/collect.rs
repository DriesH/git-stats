use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use git_stats::git::collect::{list_oids, collect, CollectOpts};
use git2::Repository;
use tempfile::TempDir;

fn run(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git").current_dir(dir).args(args)
        .env("GIT_AUTHOR_DATE", "2024-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2024-01-01T00:00:00Z")
        .status().unwrap();
    assert!(status.success());
}

fn run_at(dir: &std::path::Path, date: &str, args: &[&str]) {
    let status = Command::new("git").current_dir(dir).args(args)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .status().unwrap();
    assert!(status.success());
}

#[test]
fn collects_records_from_temp_repo() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run(dir, &["init", "-q"]);
    run(dir, &["config", "user.name", "alice"]);
    run(dir, &["config", "user.email", "a@x"]);
    std::fs::write(dir.join("a.txt"), "line1\nline2\n").unwrap();
    run(dir, &["add", "."]);
    run(dir, &["commit", "-q", "-m", "first commit"]);
    std::fs::write(dir.join("a.txt"), "line1\nchanged\nline3\n").unwrap();
    run(dir, &["add", "."]);
    run(dir, &["commit", "-q", "-m", "second commit"]);

    let repo = Repository::open(dir).unwrap();
    let oids = list_oids(&repo, &CollectOpts::default()).unwrap();
    assert_eq!(oids.len(), 2);

    let done = AtomicUsize::new(0);
    let cancel = AtomicBool::new(false);
    let mut records = collect(dir, &oids, &done, &cancel);
    records.sort_by_key(|r| r.timestamp);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].author_name, "alice");
    assert!(records[0].files.iter().any(|f| f.path == "a.txt"));
    assert_eq!(done.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[test]
fn cancel_returns_early() {
    use std::sync::atomic::Ordering;
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    run(dir, &["init", "-q"]);
    run(dir, &["config", "user.name", "a"]);
    run(dir, &["config", "user.email", "a@x"]);
    for i in 0..3 {
        std::fs::write(dir.join("f"), format!("v{i}")).unwrap();
        run(dir, &["add", "."]);
        run(dir, &["commit", "-q", "-m", &format!("c{i}")]);
    }
    let repo = Repository::open(dir).unwrap();
    let oids = list_oids(&repo, &CollectOpts::default()).unwrap();
    assert_eq!(oids.len(), 3);

    let done = AtomicUsize::new(0);
    let cancel = AtomicBool::new(true); // pre-cancelled: every oid bails on entry
    let records = collect(dir, &oids, &done, &cancel);
    assert!(records.is_empty(), "cancelled run must produce no records");
    assert_eq!(done.load(Ordering::Relaxed), 0, "cancelled oids are not counted");
}

/// Verify that `list_oids` respects both the `--limit` break and the `--since` break.
///
/// Three commits on a linear history with distinct timestamps:
///   commit 1 – 2024-01-01 (oldest)
///   commit 2 – 2024-02-01
///   commit 3 – 2024-03-01 (newest / HEAD)
///
/// `list_oids` walks newest-first, so the order returned is [mar, feb, jan].
#[test]
fn list_oids_respects_limit_and_since() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    run_at(dir, "2024-01-01T00:00:00Z", &["init", "-q"]);
    run_at(dir, "2024-01-01T00:00:00Z", &["config", "user.name", "bob"]);
    run_at(dir, "2024-01-01T00:00:00Z", &["config", "user.email", "b@x"]);

    // Commit 1 – 2024-01-01
    std::fs::write(dir.join("f"), "v1").unwrap();
    run_at(dir, "2024-01-01T00:00:00Z", &["add", "."]);
    run_at(dir, "2024-01-01T00:00:00Z", &["commit", "-q", "-m", "jan"]);

    // Commit 2 – 2024-02-01
    std::fs::write(dir.join("f"), "v2").unwrap();
    run_at(dir, "2024-02-01T00:00:00Z", &["add", "."]);
    run_at(dir, "2024-02-01T00:00:00Z", &["commit", "-q", "-m", "feb"]);

    // Commit 3 – 2024-03-01
    std::fs::write(dir.join("f"), "v3").unwrap();
    run_at(dir, "2024-03-01T00:00:00Z", &["add", "."]);
    run_at(dir, "2024-03-01T00:00:00Z", &["commit", "-q", "-m", "mar"]);

    let repo = Repository::open(dir).unwrap();

    // --limit 2: should return the 2 newest commits (mar, feb); exercises the limit break.
    let limited = list_oids(&repo, &CollectOpts { limit: Some(2), since: None }).unwrap();
    assert_eq!(limited.len(), 2, "limit=2 must return exactly 2 oids");

    // --since 2024-02-01T00:00:00Z (unix 1706745600): should return mar + feb; exercises the since break.
    let since_feb: i64 = 1706745600;
    let since_filtered = list_oids(&repo, &CollectOpts { limit: None, since: Some(since_feb) }).unwrap();
    assert_eq!(since_filtered.len(), 2, "since=2024-02-01 must return 2 oids (feb + mar)");
}
