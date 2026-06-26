use crate::scoreboard::Scoreboard;
use crate::tui::panels;
use crate::tui::theme::{title_style, BANNER};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

pub const TAB_TITLES: &[&str] = &[
    "Committers",
    "Churn",
    "Biggest",
    "Night Owls",
    "Streaks",
    "Words",
    "Ownership",
    "Vitals",
    "Oops",
    "Busiest",
    "Battlefield",
];

pub struct AppState {
    pub tab: usize,
    /// Vertical scroll offset (in lines) for the current tab's panel. Reset to
    /// 0 whenever the tab changes; clamped to the content height at render time.
    pub scroll: u16,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self { tab: 0, scroll: 0 }
    }

    pub fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % TAB_TITLES.len();
        self.scroll = 0;
    }

    pub fn prev_tab(&mut self) {
        self.tab = (self.tab + TAB_TITLES.len() - 1) % TAB_TITLES.len();
        self.scroll = 0;
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}

pub fn run_scoreboard(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    sb: &Scoreboard,
) -> Result<()> {
    let mut state = AppState::new();
    loop {
        term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7),
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(f.area());

            f.render_widget(Paragraph::new(BANNER).style(title_style()), chunks[0]);

            let tabs = Tabs::new(
                TAB_TITLES
                    .iter()
                    .map(|t| Line::from(*t))
                    .collect::<Vec<_>>(),
            )
            .select(state.tab)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(title_style());
            f.render_widget(tabs, chunks[1]);

            let panel = match state.tab {
                0 => panels::committers_widget(sb),
                1 => panels::churn_widget(sb),
                2 => panels::biggest_widget(sb),
                3 => panels::nightowls_widget(sb),
                4 => panels::streaks_widget(sb),
                5 => panels::words_widget(sb),
                6 => panels::ownership_widget(sb),
                7 => panels::vitals_widget(sb),
                8 => panels::oops_widget(sb),
                9 => panels::busiest_widget(sb),
                _ => panels::battlefield_widget(sb),
            };
            // Clamp the scroll offset to the content so you cannot scroll past
            // the last line. `line_count` includes the block borders, matching
            // the panel area's height.
            let area = chunks[2];
            let content = panel.line_count(area.width) as u16;
            let max_scroll = content.saturating_sub(area.height);
            state.scroll = state.scroll.min(max_scroll);
            f.render_widget(panel.scroll((state.scroll, 0)), area);

            f.render_widget(
                Paragraph::new("←/→ tab   ↑/↓ scroll   q quit"),
                chunks[3],
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => state.next_tab(),
                    KeyCode::Left | KeyCode::Char('h') => state.prev_tab(),
                    KeyCode::Down | KeyCode::Char('j') => state.scroll_down(),
                    KeyCode::Up | KeyCode::Char('k') => state.scroll_up(),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_wrap_around() {
        let mut a = AppState::new();
        assert_eq!(a.tab, 0);
        a.prev_tab();
        assert_eq!(a.tab, TAB_TITLES.len() - 1);
        a.next_tab();
        assert_eq!(a.tab, 0);
        a.next_tab();
        assert_eq!(a.tab, 1);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        let mut a = AppState::new();
        a.scroll_up();
        assert_eq!(a.scroll, 0);
        a.scroll_down();
        a.scroll_down();
        assert_eq!(a.scroll, 2);
    }

    #[test]
    fn changing_tab_resets_scroll() {
        let mut a = AppState::new();
        a.scroll_down();
        a.scroll_down();
        a.next_tab();
        assert_eq!(a.scroll, 0);
        a.scroll_down();
        a.prev_tab();
        assert_eq!(a.scroll, 0);
    }
}
