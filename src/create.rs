use crate::cli::Target;
use crate::color::derive_palette;
use crate::ir::{ThemeIR, ThemeType};
use crate::preview::color_picker::{render_picker, ColorPicker};
use crate::preview::ui::render_preview;
use crate::preview::TerminalGuard;
use crate::store;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use inquire::InquireError;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
};
use std::io;

#[derive(Clone, Copy)]
enum Phase {
    PickBg,
    PickFg,
    PickAccent,
    Preview,
}

pub fn run_create() -> Result<()> {
    // 1. Dark/Light selection
    let options = vec!["Dark", "Light"];
    let theme_type = match inquire::Select::new("Theme type:", options).prompt() {
        Ok("Light") => ThemeType::Light,
        Ok(_) => ThemeType::Dark,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    let is_dark = matches!(theme_type, ThemeType::Dark);

    let mut bg_picker = if is_dark {
        ColorPicker::new(220.0, 13.0, 18.0)
    } else {
        ColorPicker::new(0.0, 0.0, 97.0)
    };
    let mut fg_picker = if is_dark {
        ColorPicker::new(220.0, 9.0, 73.0)
    } else {
        ColorPicker::new(0.0, 0.0, 30.0)
    };
    let mut accent_picker = ColorPicker::new(210.0, 82.0, 66.0);

    // 2-5. TUI session for color picking + preview
    let ir: Option<ThemeIR> = {
        let _guard = TerminalGuard::new()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        let mut phase = Phase::PickBg;
        let mut cached_ir: Option<ThemeIR> = None;

        loop {
            terminal.draw(|f| {
                let size = f.area();
                match phase {
                    Phase::PickBg => render_picker(f, size, "background", &bg_picker),
                    Phase::PickFg => render_picker(f, size, "foreground", &fg_picker),
                    Phase::PickAccent => render_picker(f, size, "accent", &accent_picker),
                    Phase::Preview => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Min(3), Constraint::Length(1)])
                            .split(size);
                        if let Some(ir) = &cached_ir {
                            render_preview(f, chunks[0], ir, &Target::Superset);
                        }
                        let help = Line::from(vec![
                            Span::styled(" Enter ", Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw("confirm  "),
                            Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw("re-pick colors  "),
                            Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw("quit"),
                        ]);
                        f.render_widget(Paragraph::new(help), chunks[1]);
                    }
                }
            })?;

            if !event::poll(std::time::Duration::from_millis(50))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                break None;
            }

            let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
                5.0
            } else {
                1.0
            };

            // Handle picker input for non-preview phases
            if !matches!(phase, Phase::Preview) {
                let picker = match phase {
                    Phase::PickBg => &mut bg_picker,
                    Phase::PickFg => &mut fg_picker,
                    Phase::PickAccent => &mut accent_picker,
                    Phase::Preview => unreachable!(),
                };
                match key.code {
                    KeyCode::Up => picker.move_up(),
                    KeyCode::Down => picker.move_down(),
                    KeyCode::Left => picker.adjust(-step),
                    KeyCode::Right => picker.adjust(step),
                    KeyCode::Enter => {
                        phase = match phase {
                            Phase::PickBg => Phase::PickFg,
                            Phase::PickFg => Phase::PickAccent,
                            Phase::PickAccent => {
                                cached_ir = Some(derive_palette(
                                    &bg_picker.current_hex(),
                                    &fg_picker.current_hex(),
                                    &accent_picker.current_hex(),
                                    &theme_type,
                                ));
                                Phase::Preview
                            }
                            Phase::Preview => unreachable!(),
                        };
                    }
                    KeyCode::Esc => {
                        phase = match phase {
                            Phase::PickBg => break None,
                            Phase::PickFg => Phase::PickBg,
                            Phase::PickAccent => Phase::PickFg,
                            Phase::Preview => unreachable!(),
                        };
                    }
                    _ => {}
                }
            } else {
                // Preview phase input
                match key.code {
                    KeyCode::Enter => break cached_ir,
                    KeyCode::Esc => {
                        cached_ir = None;
                        phase = Phase::PickBg;
                    }
                    KeyCode::Char('q') => break None,
                    _ => {}
                }
            }
        }
    };

    // Guard dropped — back to normal terminal

    let Some(mut ir) = ir else {
        return Ok(());
    };

    // 6. Name input
    let name = match inquire::Text::new("Theme name:").prompt() {
        Ok(n) if !n.trim().is_empty() => n.trim().to_string(),
        Ok(_) => {
            eprintln!("Name cannot be empty.");
            return Ok(());
        }
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    ir.name = name;
    ir.id = store::theme_slug(&ir.name);

    // 7. Save
    let path = store::save_ir(&ir)?;
    println!("\n  \u{2714} Saved to {}", path.display());
    println!("  Run `chromaport apply` to apply this theme to your targets.");

    Ok(())
}
