use crate::cli::Target;
use crate::ir::ThemeIR;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
};
use std::io;

use super::ui;
use super::TerminalGuard;

/// Lightweight TUI for selecting a theme from stored IRs with live preview.
///
/// Returns `Some(ThemeIR)` on selection, or `None` if cancelled.
pub fn select_ir_with_preview(themes: Vec<ThemeIR>, target: &Target) -> Result<Option<ThemeIR>> {
    let _guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut selected: usize = 0;

    loop {
        let labels: Vec<String> = themes.iter().map(|ir| ir.name.clone()).collect();
        let current_ir = &themes[selected];

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

            // Render theme list (simplified, no filter/active_id)
            render_ir_list(f, panes[0], &labels, selected);

            // Render preview
            ui::render_preview(f, panes[1], current_ir, target);

            // Help bar
            render_apply_help_bar(f, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(None);
                }

                match key.code {
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if selected < themes.len() - 1 {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        return Ok(Some(themes[selected].clone()));
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn render_ir_list(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    labels: &[String],
    selected: usize,
) {
    use ratatui::widgets::{Block, Borders};

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Saved Themes ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if labels.is_empty() {
        let msg = Paragraph::new(" No saved themes");
        f.render_widget(msg, inner);
        return;
    }

    let visible_height = inner.height as usize;
    let max_offset = labels.len().saturating_sub(visible_height);
    let offset = selected.saturating_sub(visible_height / 2).min(max_offset);

    let items: Vec<Line> = labels
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_height)
        .map(|(i, label)| {
            if i == selected {
                Line::from(Span::styled(
                    format!(" > {label}"),
                    Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
                ))
            } else {
                Line::from(format!("   {label}"))
            }
        })
        .collect();

    f.render_widget(Paragraph::new(items), inner);
}

fn render_apply_help_bar(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let help = Line::from(vec![
        Span::styled(
            " \u{2191}/\u{2193} ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("navigate  "),
        Span::styled("Enter ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("select  "),
        Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("quit"),
    ]);
    f.render_widget(Paragraph::new(help), area);
}
