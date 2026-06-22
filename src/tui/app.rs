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
];

pub struct AppState {
    pub tab: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self { tab: 0 }
    }

    pub fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % TAB_TITLES.len();
    }

    pub fn prev_tab(&mut self) {
        self.tab = (self.tab + TAB_TITLES.len() - 1) % TAB_TITLES.len();
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

            match state.tab {
                0 => f.render_widget(panels::committers_widget(sb), chunks[2]),
                1 => f.render_widget(panels::churn_widget(sb), chunks[2]),
                2 => f.render_widget(panels::biggest_widget(sb), chunks[2]),
                3 => f.render_widget(panels::nightowls_widget(sb), chunks[2]),
                4 => f.render_widget(panels::streaks_widget(sb), chunks[2]),
                5 => f.render_widget(panels::words_widget(sb), chunks[2]),
                6 => f.render_widget(panels::ownership_widget(sb), chunks[2]),
                _ => f.render_widget(panels::vitals_widget(sb), chunks[2]),
            }

            f.render_widget(Paragraph::new("←/→ switch tab   q quit"), chunks[3]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => state.next_tab(),
                    KeyCode::Left | KeyCode::Char('h') => state.prev_tab(),
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
}
