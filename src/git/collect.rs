use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use git2::{Diff, DiffOptions, Oid, Repository};
use rayon::prelude::*;

use crate::model::{CommitRecord, FileChurn};

/// Options controlling which commits are walked.
#[derive(Debug, Default, Clone)]
pub struct CollectOpts {
    /// Stop after this many commits (newest-first).
    pub limit: Option<usize>,
    /// Skip commits whose timestamp (Unix seconds) is older than this.
    pub since: Option<i64>,
}

/// Walk history newest-first along the FIRST-PARENT line only.
///
/// Applies `--limit` / `--since` from `opts`. Returns an empty `Vec` for an
/// unborn HEAD (freshly `git init`, no commits yet) so callers can display a
/// "no commits yet" message without treating the condition as an error.
pub fn list_oids(repo: &Repository, opts: &CollectOpts) -> anyhow::Result<Vec<Oid>> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(e)
            if e.code() == git2::ErrorCode::UnbornBranch
                || e.code() == git2::ErrorCode::NotFound =>
        {
            return Ok(Vec::new())
        }
        Err(e) => return Err(e.into()),
    };

    let mut current = match head.peel_to_commit() {
        Ok(c) => Some(c),
        Err(e)
            if e.code() == git2::ErrorCode::UnbornBranch
                || e.code() == git2::ErrorCode::NotFound =>
        {
            return Ok(Vec::new())
        }
        Err(e) => return Err(e.into()),
    };

    let mut out = Vec::new();
    while let Some(commit) = current {
        if let Some(since) = opts.since {
            if commit.time().seconds() < since {
                break;
            }
        }
        out.push(commit.id());
        if let Some(limit) = opts.limit {
            if out.len() >= limit {
                break;
            }
        }
        // First parent only; returns None at the root commit.
        current = commit.parent(0).ok();
    }
    Ok(out)
}

/// Compute a [`CommitRecord`] for every OID in `oids`, in parallel via rayon.
///
/// Each rayon worker opens its own `Repository` handle because `git2::Repository`
/// is not `Sync`. `done` is incremented for every OID that is actually processed
/// (whether or not the result is `Some`). When `cancel` is set, workers return
/// early without incrementing `done`, so the caller's completion counter never
/// stalls.
pub fn collect(
    repo_path: &Path,
    oids: &[Oid],
    done: &AtomicUsize,
    cancel: &AtomicBool,
) -> Vec<CommitRecord> {
    oids.par_iter()
        .map_init(
            || Repository::open(repo_path).ok(),
            |repo, oid| {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let rec = repo.as_ref().and_then(|r| record_for(r, *oid));
                done.fetch_add(1, Ordering::Relaxed);
                rec
            },
        )
        .flatten()
        .collect()
}

fn record_for(repo: &Repository, oid: Oid) -> Option<CommitRecord> {
    let commit = repo.find_commit(oid).ok()?;
    let tree = commit.tree().ok()?;
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let mut opts = DiffOptions::new();
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))
        .ok()?;
    let files = file_churn(&diff);
    let author = commit.author();
    Some(CommitRecord {
        sha: oid.to_string(),
        author_name: author.name().unwrap_or("unknown").to_string(),
        author_email: author.email().unwrap_or("").to_string(),
        timestamp: commit.time().seconds(),
        tz_offset_minutes: commit.time().offset_minutes(),
        message: commit.message().unwrap_or("").to_string(),
        files,
    })
}

/// Build per-file churn stats from a diff by walking every line callback.
///
/// The linear `find` over `files` is intentional: a commit touches few files,
/// so the constant-factor of a `HashMap` is not worth it here.
fn file_churn(diff: &Diff<'_>) -> Vec<FileChurn> {
    let mut files: Vec<FileChurn> = Vec::new();
    let _ = diff.foreach(
        &mut |_delta, _progress| true,
        None,
        None,
        Some(&mut |delta, _hunk, line| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let entry = match files.iter_mut().find(|f| f.path == path) {
                Some(e) => e,
                None => {
                    files.push(FileChurn {
                        path,
                        added: 0,
                        removed: 0,
                    });
                    files.last_mut().unwrap()
                }
            };
            match line.origin() {
                '+' => entry.added += 1,
                '-' => entry.removed += 1,
                _ => {}
            }
            true
        }),
    );
    files
}
