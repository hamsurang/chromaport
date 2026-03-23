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
use super::PAGE_SIZE;

fn build_label(ir: &ThemeIR) -> String {
    let type_tag = match ir.theme_type {
        crate::ir::ThemeType::Dark => "Dark",
        crate::ir::ThemeType::Light => "Light",
    };
    match &ir.created_at {
        Some(date) => format!("{} [{}] {}", ir.name, type_tag, date),
        None => format!("{} [{}]", ir.name, type_tag),
    }
}

fn compute_filtered(themes: &[ThemeIR], filter: &str) -> Vec<(usize, String)> {
    let filter_lower = filter.to_lowercase();
    themes
        .iter()
        .enumerate()
        .filter(|(_, ir)| filter.is_empty() || ir.name.to_lowercase().contains(&filter_lower))
        .map(|(i, ir)| (i, build_label(ir)))
        .collect()
}

/// Lightweight TUI for selecting a theme from stored IRs with live preview.
///
/// Returns `Some(ThemeIR)` on selection, or `None` if cancelled.
pub fn select_ir_with_preview(themes: Vec<ThemeIR>, target: &Target) -> Result<Option<ThemeIR>> {
    let _guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut selected: usize = 0;
    let mut filter = String::new();
    let mut filtered = compute_filtered(&themes, &filter);

    loop {
        if selected >= filtered.len() {
            selected = filtered.len().saturating_sub(1);
        }

        let current_ir = filtered.get(selected).map(|(i, _)| &themes[*i]);

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

            render_ir_list(f, panes[0], &filtered, selected, &filter);

            if let Some(ir) = current_ir {
                ui::render_preview(f, panes[1], ir, target);
            }

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
                        if selected < filtered.len().saturating_sub(1) {
                            selected += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        selected = selected.saturating_sub(PAGE_SIZE);
                    }
                    KeyCode::PageDown => {
                        if !filtered.is_empty() {
                            selected = (selected + PAGE_SIZE).min(filtered.len().saturating_sub(1));
                        }
                    }
                    KeyCode::Home => {
                        selected = 0;
                    }
                    KeyCode::End => {
                        selected = filtered.len().saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        if let Some((orig_idx, _)) = filtered.get(selected) {
                            return Ok(Some(themes[*orig_idx].clone()));
                        }
                    }
                    KeyCode::Esc => {
                        if !filter.is_empty() {
                            filter.clear();
                            selected = 0;
                            filtered = compute_filtered(&themes, &filter);
                        } else {
                            return Ok(None);
                        }
                    }
                    KeyCode::Char('q') if filter.is_empty() => {
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        filter.pop();
                        filtered = compute_filtered(&themes, &filter);
                    }
                    KeyCode::Char(c) => {
                        filter.push(c);
                        filtered = compute_filtered(&themes, &filter);
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
    filtered: &[(usize, String)],
    selected: usize,
    filter: &str,
) {
    use ratatui::widgets::{Block, Borders};

    let position = ui::format_position(selected, filtered.len());

    let title = if filter.is_empty() {
        format!(" Saved Themes {position}")
    } else {
        format!(" Saved Themes {position}[{}] ", filter)
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if filtered.is_empty() {
        let msg = Paragraph::new(" No matching themes");
        f.render_widget(msg, inner);
        return;
    }

    let visible_height = inner.height as usize;
    let max_offset = filtered.len().saturating_sub(visible_height);
    let offset = selected.saturating_sub(visible_height / 2).min(max_offset);

    let items: Vec<Line> = filtered
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_height)
        .map(|(i, (_, label))| {
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
        Span::styled("PgUp/PgDn ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("page  "),
        Span::styled("Enter ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("select  "),
        Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("quit  "),
        Span::raw("Type to filter  "),
        Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("clear"),
    ]);
    f.render_widget(Paragraph::new(help), area);
}
