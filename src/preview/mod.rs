mod app;
mod ui;

use crate::cli::Target;
use crate::reader::{ThemeEntry, ThemeReader};
use anyhow::Result;
use app::PreviewApp;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io;

/// RAII guard for terminal state restoration.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        // Defensive: reset terminal state before entering raw mode.
        // inquire uses crossterm 0.25 while ratatui uses 0.28; both modify the
        // same termios flags. This call clears any stale raw-mode state left by
        // the earlier inquire prompts (editor/target selection).
        let _ = disable_raw_mode();
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Interactive theme selection with live preview.
///
/// Returns `Some(ThemeEntry)` on selection, or `None` if the user cancelled.
pub fn select_theme_with_preview(
    themes: &[ThemeEntry],
    active_id: Option<&str>,
    reader: &ThemeReader,
    target: &Target,
) -> Result<Option<ThemeEntry>> {
    // Sort: active theme first, then alphabetical
    let mut sorted = themes.to_vec();
    if let Some(active) = active_id {
        sorted.sort_by(|a, b| {
            let a_active = a.settings_id == active;
            let b_active = b.settings_id == active;
            b_active.cmp(&a_active).then(a.label.cmp(&b.label))
        });
    } else {
        sorted.sort_by(|a, b| a.label.cmp(&b.label));
    }

    let _guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = PreviewApp::new(sorted, active_id.map(str::to_string), reader, target);

    loop {
        app.ensure_current_cached();

        terminal.draw(|f| {
            let size = f.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(1)])
                .split(size);

            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(chunks[0]);

            let labels = app.filtered_labels();
            let settings_ids = app.filtered_settings_ids();

            ui::render_theme_list(
                f,
                panes[0],
                &labels,
                app.selected_index(),
                app.filter(),
                app.active_id(),
                &settings_ids,
            );

            if let Some(ir) = app.cached_current_ir() {
                ui::render_preview(f, panes[1], ir, app.target());
            } else if let Some(err) = app.cached_current_error() {
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(" Preview ");
                let inner = block.inner(panes[1]);
                f.render_widget(block, panes[1]);
                let msg = ratatui::widgets::Paragraph::new(format!(" Error: {err}"));
                f.render_widget(msg, inner);
            } else {
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(" Preview ");
                f.render_widget(block, panes[1]);
            }

            ui::render_help_bar(f, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Ctrl+C → return None (exit code 130 handled by caller)
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(None);
                }

                match key.code {
                    KeyCode::Up => app.move_up(),
                    KeyCode::Down => app.move_down(),
                    KeyCode::Enter => {
                        if let Some(entry) = app.select() {
                            return Ok(Some(entry));
                        }
                    }
                    KeyCode::Esc => {
                        if !app.filter().is_empty() {
                            app.clear_filter();
                        } else {
                            return Ok(None);
                        }
                    }
                    KeyCode::Char('q') if app.filter().is_empty() => {
                        return Ok(None);
                    }
                    KeyCode::Backspace => app.delete_filter_char(),
                    KeyCode::Char(c) => app.add_filter_char(c),
                    _ => {}
                }
            }
        }
    }
}
