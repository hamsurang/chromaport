use crate::cli::Target;
use crate::ir::{AnsiPalette, HexColor, ThemeIR, ThemeType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Convert a HexColor to a ratatui Color.
pub fn to_color(hex: &HexColor) -> Color {
    let (r, g, b) = hex.to_rgb();
    Color::Rgb(r, g, b)
}

/// Token types for syntax highlighting simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    String,
    Function,
    Comment,
    Type,
    Punctuation,
    Variable,
    Plain,
}

impl TokenKind {
    /// Map token kind to a theme color using fixed ANSI mapping.
    pub fn color(self, ir: &ThemeIR) -> Color {
        match self {
            TokenKind::Keyword => to_color(&ir.terminal.normal.blue),
            TokenKind::String => to_color(&ir.terminal.normal.green),
            TokenKind::Function => to_color(&ir.terminal.normal.yellow),
            TokenKind::Comment => to_color(&ir.terminal.bright.black),
            TokenKind::Type => to_color(&ir.terminal.normal.cyan),
            TokenKind::Punctuation => to_color(&ir.terminal.foreground),
            TokenKind::Variable => to_color(&ir.terminal.normal.red),
            TokenKind::Plain => to_color(&ir.terminal.foreground),
        }
    }
}

/// Returns a TypeScript code snippet as tokenized lines.
pub fn typescript_snippet() -> Vec<Vec<(TokenKind, &'static str)>> {
    use TokenKind::*;
    vec![
        vec![
            (Keyword, "const"),
            (Plain, " "),
            (Variable, "greet"),
            (Plain, " "),
            (Punctuation, "="),
            (Plain, " "),
            (Punctuation, "("),
            (Variable, "name"),
            (Punctuation, ":"),
            (Plain, " "),
            (Type, "string"),
            (Punctuation, "):"),
            (Plain, " "),
            (Type, "void"),
            (Plain, " "),
            (Punctuation, "=>"),
            (Plain, " "),
            (Punctuation, "{"),
        ],
        vec![
            (Plain, "  "),
            (Variable, "console"),
            (Punctuation, "."),
            (Function, "log"),
            (Punctuation, "("),
            (String, "`Hello, "),
            (Punctuation, "${"),
            (Variable, "name"),
            (Punctuation, "}"),
            (String, "!`"),
            (Punctuation, ")"),
            (Punctuation, ";"),
        ],
        vec![(Punctuation, "}"), (Punctuation, ";")],
        vec![],
        vec![(Comment, "// Call the function")],
        vec![
            (Function, "greet"),
            (Punctuation, "("),
            (String, "\"World\""),
            (Punctuation, ")"),
            (Punctuation, ";"),
        ],
    ]
}

/// Build a Line of colored palette swatches.
fn palette_line(palette: &AnsiPalette, label: &str) -> Line<'static> {
    let colors = [
        &palette.black,
        &palette.red,
        &palette.green,
        &palette.yellow,
        &palette.blue,
        &palette.magenta,
        &palette.cyan,
        &palette.white,
    ];
    let mut spans = vec![Span::raw(format!(" {label}: "))];
    for (i, c) in colors.iter().enumerate() {
        spans.push(Span::styled("██", Style::default().fg(to_color(c))));
        if i < 7 {
            spans.push(Span::raw(" "));
        }
    }
    Line::from(spans)
}

/// Render the full preview pane for a theme.
pub fn render_preview(f: &mut Frame, area: Rect, ir: &ThemeIR, target: &Target) {
    let bg = to_color(&ir.background);
    let fg = to_color(&ir.foreground);
    let type_label = match ir.theme_type {
        ThemeType::Dark => "Dark",
        ThemeType::Light => "Light",
    };
    let title = format!(" Preview: {} ({type_label}) ", ir.name);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(fg))
        .title(title)
        .style(Style::default().bg(bg));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let show_chart = matches!(target, Target::Superset);
    let constraints = if show_chart {
        vec![
            Constraint::Length(2), // meta
            Constraint::Length(4), // palette
            Constraint::Length(3), // chart colors
            Constraint::Min(3),    // code snippet
        ]
    } else {
        vec![
            Constraint::Length(2), // meta
            Constraint::Length(4), // palette
            Constraint::Min(3),    // code snippet
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    render_meta(f, chunks[0], ir, bg);
    render_palette(f, chunks[1], ir, bg);

    if show_chart {
        render_chart_colors(f, chunks[2], ir, bg);
        render_code(f, chunks[3], ir, bg);
    } else {
        render_code(f, chunks[2], ir, bg);
    }
}

fn render_meta(f: &mut Frame, area: Rect, ir: &ThemeIR, bg: Color) {
    let lines = vec![Line::from(vec![
        Span::raw(" bg:"),
        Span::styled(
            ir.background.as_str().to_string(),
            Style::default().fg(to_color(&ir.background)),
        ),
        Span::raw("  fg:"),
        Span::styled(
            ir.foreground.as_str().to_string(),
            Style::default().fg(to_color(&ir.foreground)),
        ),
        Span::raw("  accent:"),
        Span::styled(
            ir.accent.as_str().to_string(),
            Style::default().fg(to_color(&ir.accent)),
        ),
    ])];
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(bg)), area);
}

fn render_palette(f: &mut Frame, area: Rect, ir: &ThemeIR, bg: Color) {
    let lines = vec![
        Line::raw(""),
        palette_line(&ir.terminal.normal, "Normal"),
        palette_line(&ir.terminal.bright, "Bright"),
    ];
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(bg)), area);
}

fn render_chart_colors(f: &mut Frame, area: Rect, ir: &ThemeIR, bg: Color) {
    let mut spans = vec![Span::raw(" Chart: ")];
    for (i, c) in ir.chart_colors.iter().enumerate() {
        spans.push(Span::styled("██", Style::default().fg(to_color(c))));
        if i < 4 {
            spans.push(Span::raw(" "));
        }
    }
    let lines = vec![Line::raw(""), Line::from(spans)];
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(bg)), area);
}

fn render_code(f: &mut Frame, area: Rect, ir: &ThemeIR, bg: Color) {
    let snippet = typescript_snippet();
    let mut lines: Vec<Line> = vec![Line::raw("")];
    for token_line in &snippet {
        let spans: Vec<Span> = std::iter::once(Span::raw(" "))
            .chain(token_line.iter().map(|(kind, text)| {
                Span::styled((*text).to_string(), Style::default().fg(kind.color(ir)))
            }))
            .collect();
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(bg)), area);
}

/// Render the theme list pane.
pub fn render_theme_list(
    f: &mut Frame,
    area: Rect,
    labels: &[String],
    selected: usize,
    filter: &str,
    active_id: Option<&str>,
    settings_ids: &[String],
    saved_flags: &[bool],
) {
    let title = if filter.is_empty() {
        " Select Theme ".to_string()
    } else {
        format!(" Select Theme [{}] ", filter)
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if labels.is_empty() {
        let msg = Paragraph::new(" No matching themes");
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
            let is_active = active_id
                .map(|id| settings_ids.get(i).map(|s| s.as_str()) == Some(id))
                .unwrap_or(false);
            let is_saved = saved_flags.get(i).copied().unwrap_or(false);
            let display = match (is_active, is_saved) {
                (true, true) => format!("{label} [active] (saved)"),
                (true, false) => format!("{label} [active]"),
                (false, true) => format!("{label} (saved)"),
                (false, false) => label.clone(),
            };
            if i == selected {
                Line::from(Span::styled(
                    format!(" > {display}"),
                    Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
                ))
            } else {
                Line::from(format!("   {display}"))
            }
        })
        .collect();

    f.render_widget(Paragraph::new(items), inner);
}

/// Render the help bar at the bottom.
pub fn render_help_bar(f: &mut Frame, area: Rect) {
    let help = Line::from(vec![
        Span::styled(
            " \u{2191}/\u{2193} ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("navigate  "),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Target;
    use crate::ir::test_fixtures::make_test_ir;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn to_color_converts_hex() {
        let hex = HexColor::parse("#FF8040").unwrap();
        assert_eq!(to_color(&hex), Color::Rgb(255, 128, 64));
    }

    #[test]
    fn token_kind_maps_to_ansi() {
        let ir = make_test_ir();
        let color = TokenKind::Keyword.color(&ir);
        assert_eq!(color, to_color(&ir.terminal.normal.blue));
    }

    #[test]
    fn typescript_snippet_is_non_empty() {
        let snippet = typescript_snippet();
        assert!(!snippet.is_empty());
        assert!(!snippet[0].is_empty());
    }

    #[test]
    fn render_preview_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let ir = make_test_ir();
        terminal
            .draw(|f| {
                render_preview(f, f.area(), &ir, &Target::Ghostty);
            })
            .unwrap();
    }

    #[test]
    fn render_preview_superset_shows_chart() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let ir = make_test_ir();
        terminal
            .draw(|f| {
                render_preview(f, f.area(), &ir, &Target::Superset);
            })
            .unwrap();
    }

    #[test]
    fn render_theme_list_no_panic() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let labels = vec!["One Dark".to_string(), "Dracula".to_string()];
        let ids = vec!["one-dark".to_string(), "dracula".to_string()];
        terminal
            .draw(|f| {
                render_theme_list(f, f.area(), &labels, 0, "", Some("one-dark"), &ids, &[]);
            })
            .unwrap();
    }

    #[test]
    fn render_empty_list_no_panic() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_theme_list(f, f.area(), &[], 0, "xyz", None, &[], &[]);
            })
            .unwrap();
    }
}
