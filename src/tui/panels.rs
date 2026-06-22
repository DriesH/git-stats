use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::scoreboard::Scoreboard;
use crate::tui::theme::{accent_style, header_style, medal};

fn panel_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title.to_string(), header_style()))
}

pub fn committers_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let lines: Vec<Line> = sb
        .committers
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, c)| {
            Line::from(vec![
                Span::raw(format!("{} ", medal(i))),
                Span::styled(format!("{:<20}", c.name), accent_style()),
                Span::raw(format!("{:>5} commits  {:>7} lines", c.commits, c.lines)),
            ])
        })
        .collect();
    Paragraph::new(lines).block(panel_block(" Top Committers "))
}

pub fn churn_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let lines: Vec<Line> = sb
        .churn
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, c)| {
            Line::from(format!(
                "{} {:<30} +{:<6} -{:<6} (Δ{})",
                medal(i),
                c.path,
                c.added,
                c.removed,
                c.total()
            ))
        })
        .collect();
    Paragraph::new(lines).block(panel_block(" Churn Hotspots "))
}

pub fn biggest_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let text = match &sb.biggest {
        Some(b) => format!(
            "🏆 {} lines\nby {}\n{}\n{}",
            b.lines,
            b.author,
            &b.sha[..b.sha.len().min(10)],
            b.summary
        ),
        None => "no commits".to_string(),
    };
    Paragraph::new(text).block(panel_block(" Biggest Commit "))
}

pub fn nightowls_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let max = sb
        .nightowls
        .histogram
        .hours
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    let mut lines: Vec<Line> = (0..24)
        .map(|h| {
            let count = sb.nightowls.histogram.hours[h];
            let bar_len = (count * 20 / max).max(if count > 0 { 1 } else { 0 });
            let bar = "█".repeat(bar_len);
            Line::from(format!("{:02}h {:<20} {}", h, bar, count))
        })
        .collect();
    lines.push(Line::from(""));
    for w in sb.nightowls.warriors.iter().take(5) {
        lines.push(Line::from(format!(
            "{:<20} {:.0}% weekend ({} commits)",
            w.name, w.weekend_pct, w.total
        )));
    }
    Paragraph::new(lines).block(panel_block(" Night Owls & Weekend Warriors "))
}

pub fn streaks_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let lines: Vec<Line> = sb
        .streaks
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, s)| {
            Line::from(format!(
                "{} {:<20} {} day streak",
                medal(i),
                s.name,
                s.longest_days
            ))
        })
        .collect();
    Paragraph::new(lines).block(panel_block(" Longest Streaks "))
}

pub fn words_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let lines: Vec<Line> = sb
        .words
        .iter()
        .take(15)
        .map(|w| Line::from(format!("{:<15} {}", w.word, "▪".repeat(w.count.min(40)))))
        .collect();
    Paragraph::new(lines).block(panel_block(" Commit Word Cloud "))
}

pub fn ownership_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let lines: Vec<Line> = sb
        .ownership
        .iter()
        .take(10)
        .map(|o| {
            let factor = if o.author_count == 1 {
                " ⚠ bus factor 1"
            } else {
                ""
            };
            Line::from(format!(
                "{:<30} {:<15} {} authors{}",
                o.path, o.top_author, o.author_count, factor
            ))
        })
        .collect();
    Paragraph::new(lines).block(panel_block(" File Ownership "))
}

pub fn vitals_widget(sb: &Scoreboard) -> Paragraph<'_> {
    let text = match &sb.vitals {
        Some(v) => format!(
            "Total commits: {}\nAuthors: {}\nAge: {} days\nPace: {:.2} commits/day",
            v.total_commits, v.authors, v.age_days, v.commits_per_day
        ),
        None => "no commits".to_string(),
    };
    Paragraph::new(text).block(panel_block(" Repo Vitals "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoreboard::analyze;
    use crate::stats::rec;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_committers_without_panic_and_shows_name() {
        let sb = analyze(&[rec("alice", 1_704_067_200, &[("a.rs", 5, 1)])]);
        let backend = TestBackend::new(60, 20);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let w = committers_widget(&sb);
            f.render_widget(w, f.area());
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("alice"));
    }
}
