use std::sync::atomic::{AtomicBool, Ordering};
use ratatui::style::{Color, Modifier, Style};

pub const BANNER: &str = r#"
  ____ ___ _____   ____ _____ _  _____ ____
 / ___|_ _|_   _| / ___|_   _/ \|_   _/ ___|
| |  _ | |  | |   \___ \ | |/ _ \ | | \___ \
| |_| || |  | |    ___) || / ___ \| |  ___) |
 \____|___| |_|   |____/ |_/_/   \_\_| |____/
"#;

/// Global color switch. Defaults on; main() turns it off for `--no-color`
/// or when stdout is not a TTY. Every `*_style()` collapses to a plain
/// `Style::default()` when off, so callers never branch on color themselves.
static COLOR_ENABLED: AtomicBool = AtomicBool::new(true);

/// Enable or disable colored output globally.
pub fn set_color_enabled(on: bool) {
    COLOR_ENABLED.store(on, Ordering::Relaxed);
}

/// Returns the style unchanged when color is enabled, or `Style::default()`
/// (no color, no modifiers) when color is disabled.
fn maybe(style: Style) -> Style {
    if COLOR_ENABLED.load(Ordering::Relaxed) {
        style
    } else {
        Style::default()
    }
}

/// Returns the medal string for a given zero-based rank.
///
/// Ranks 0–2 get medal emoji; higher ranks get a right-aligned two-digit
/// human rank (1-indexed).
pub fn medal(rank: usize) -> String {
    match rank {
        0 => "🥇".to_string(),
        1 => "🥈".to_string(),
        2 => "🥉".to_string(),
        n => format!("{:>2}", n + 1),
    }
}

/// Bold magenta — used for top-level titles.
pub fn title_style() -> Style {
    maybe(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
}

/// Bold cyan — used for table headers and section headings.
pub fn header_style() -> Style {
    maybe(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
}

/// Yellow — used for highlighted values and accents.
pub fn accent_style() -> Style {
    maybe(Style::default().fg(Color::Yellow))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medal_for_rank() {
        assert_eq!(medal(0), "🥇");
        assert_eq!(medal(1), "🥈");
        assert_eq!(medal(2), "🥉");
        assert_eq!(medal(3), " 4");
        assert_eq!(medal(9), "10");
    }

    #[test]
    fn banner_nonempty() {
        assert!(!BANNER.trim().is_empty());
    }

    #[test]
    fn no_color_yields_plain_style() {
        set_color_enabled(false);
        assert_eq!(title_style(), ratatui::style::Style::default());
        set_color_enabled(true); // restore for other tests in the binary
        assert_ne!(title_style(), ratatui::style::Style::default());
    }
}
