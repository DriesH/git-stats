use std::io::Stdout;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Terminal;

use crate::tui::theme::{title_style, BANNER};

/// Estimate seconds remaining.
///
/// Returns `None` until at least 1 % of work is done to avoid early noise
/// from a near-zero denominator. Once past that threshold the estimate is a
/// simple linear extrapolation: per-item time × items remaining.
pub fn eta_seconds(done: usize, total: usize, elapsed_secs: f64) -> Option<f64> {
    if total == 0 || done == 0 || (done as f64) / (total as f64) < 0.01 {
        return None;
    }
    let per = elapsed_secs / done as f64;
    Some(per * (total - done) as f64)
}

/// Render the loading screen until `done == total`.
///
/// Returns `Ok(true)` when the user pressed `q` or `Esc` (cancelled),
/// `Ok(false)` when all work finished normally.
pub fn run_loading(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    done: &AtomicUsize,
    total: usize,
    cancel: &AtomicBool,
) -> Result<bool> {
    let start = Instant::now();
    let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut frame = 0usize;

    loop {
        let d = done.load(Ordering::Relaxed);
        if d >= total {
            return Ok(false);
        }

        let elapsed = start.elapsed().as_secs_f64();
        let eta = eta_seconds(d, total, elapsed);
        let ratio = if total == 0 { 0.0 } else { d as f64 / total as f64 };
        frame = (frame + 1) % spinner.len();

        term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(f.area());

            f.render_widget(Paragraph::new(BANNER).style(title_style()), chunks[0]);

            let label = match eta {
                Some(s) => format!("{} {}/{} commits  ~{:.0}s left", spinner[frame], d, total, s),
                None => format!("{} {}/{} commits  estimating…", spinner[frame], d, total),
            };
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(" Reading history "))
                .ratio(ratio.clamp(0.0, 1.0))
                .label(label);
            f.render_widget(gauge, chunks[1]);

            f.render_widget(Paragraph::new("q / Esc to cancel"), chunks[2]);
        })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(k) = event::read()? {
                if matches!(k.code, KeyCode::Char('q') | KeyCode::Esc) {
                    cancel.store(true, Ordering::Relaxed);
                    return Ok(true);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eta_none_before_first_percent() {
        assert!(eta_seconds(0, 1000, 5.0).is_none());
    }

    #[test]
    fn eta_estimates_remaining() {
        // 100/1000 done in 10s -> 0.1s each -> 900 remaining ~= 90s
        let eta = eta_seconds(100, 1000, 10.0).unwrap();
        assert!((eta - 90.0).abs() < 1.0);
    }
}
