use std::io::{self, IsTerminal, Stdout};
use std::sync::atomic::{AtomicBool, AtomicUsize};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use git2::Repository;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use git_stats::cli::{parse_since, Args};
use git_stats::git::collect::{collect, list_oids, CollectOpts};
use git_stats::scoreboard::analyze;
use git_stats::stats::identity::collapse_identities;
use git_stats::tui::theme::set_color_enabled;
use git_stats::tui::{app::run_scoreboard, loading::run_loading};

/// Restores the terminal on drop, even on panic.
struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Color on only when not suppressed AND stdout is a real terminal.
    set_color_enabled(!args.no_color && io::stdout().is_terminal());

    let since = match &args.since {
        Some(s) => Some(parse_since(s)?),
        None => None,
    };
    let opts = CollectOpts {
        limit: args.limit,
        since,
    };

    let repo =
        Repository::open(".").context("not a git repository (run git-stats inside a repo)")?;
    let repo_path = repo
        .path()
        .parent()
        .unwrap_or_else(|| repo.path())
        .to_path_buf();
    let oids = list_oids(&repo, &opts)?;
    drop(repo); // worker threads open their own handles

    if oids.is_empty() {
        println!("No commits found. Nothing to score yet!");
        return Ok(());
    }

    let total = oids.len();
    let done = AtomicUsize::new(0);
    let cancel = AtomicBool::new(false);

    let _guard = TermGuard;
    let mut term = setup_terminal()?;

    // Collection runs on rayon worker threads inside a scoped thread so the
    // main thread can render the loading screen and poll for cancel.
    let records = std::thread::scope(|s| -> Result<Option<Vec<_>>> {
        let handle = s.spawn(|| collect(&repo_path, &oids, &done, &cancel));
        let cancelled = run_loading(&mut term, &done, total, &cancel)?;
        let recs = handle.join().expect("collect thread panicked");
        if cancelled {
            Ok(None)
        } else {
            Ok(Some(recs))
        }
    })?;

    let Some(mut records) = records else {
        return Ok(()); // user cancelled; guard restores terminal
    };
    records.sort_by_key(|r| r.timestamp);
    let records = collapse_identities(records);

    let sb = analyze(&records, args.include_generated);
    run_scoreboard(&mut term, &sb)?;
    Ok(())
}
