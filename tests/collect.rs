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
