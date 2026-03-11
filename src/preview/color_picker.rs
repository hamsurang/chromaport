use crate::color::{hex_from_rgb, hsl_to_rgb};
use crate::ir::HexColor;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct ColorPicker {
    pub h: f64,
    pub s: f64,
    pub l: f64,
    pub active: usize,
}

impl ColorPicker {
    pub fn new(h: f64, s: f64, l: f64) -> Self {
        Self { h, s, l, active: 0 }
    }

    pub fn current_rgb(&self) -> (u8, u8, u8) {
        hsl_to_rgb(self.h, self.s, self.l)
    }

    pub fn current_hex(&self) -> HexColor {
        let (r, g, b) = self.current_rgb();
        hex_from_rgb(r, g, b)
    }

    pub fn move_up(&mut self) {
        self.active = self.active.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.active < 2 {
            self.active += 1;
        }
    }

    pub fn adjust(&mut self, delta: f64) {
        match self.active {
            0 => self.h = (self.h + delta).rem_euclid(360.0),
            1 => self.s = (self.s + delta).clamp(0.0, 100.0),
            2 => self.l = (self.l + delta).clamp(0.0, 100.0),
            _ => {}
        }
    }
}

pub fn render_picker(f: &mut Frame, area: Rect, label: &str, picker: &ColorPicker) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Pick {label} color "));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacer
            Constraint::Length(1), // H
            Constraint::Length(1), // S
            Constraint::Length(1), // L
            Constraint::Length(1), // spacer
            Constraint::Length(1), // preview
            Constraint::Min(1),    // spacer
            Constraint::Length(1), // help
        ])
        .split(inner);

    render_slider(
        f,
        chunks[1],
        "H",
        picker.h,
        0.0,
        360.0,
        picker.active == 0,
        |v| {
            let (r, g, b) = hsl_to_rgb(v, picker.s, picker.l);
            Color::Rgb(r, g, b)
        },
    );
    render_slider(
        f,
        chunks[2],
        "S",
        picker.s,
        0.0,
        100.0,
        picker.active == 1,
        |v| {
            let (r, g, b) = hsl_to_rgb(picker.h, v, picker.l);
            Color::Rgb(r, g, b)
        },
    );
    render_slider(
        f,
        chunks[3],
        "L",
        picker.l,
        0.0,
        100.0,
        picker.active == 2,
        |v| {
            let (r, g, b) = hsl_to_rgb(picker.h, picker.s, v);
            Color::Rgb(r, g, b)
        },
    );

    let (r, g, b) = picker.current_rgb();
    let hex = format!("#{r:02X}{g:02X}{b:02X}");
    let preview = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "\u{2588}\u{2588}\u{2588}\u{2588}",
            Style::default().fg(Color::Rgb(r, g, b)),
        ),
        Span::raw(format!("  {hex}")),
    ]);
    f.render_widget(Paragraph::new(preview), chunks[5]);

    let help = Line::from(vec![
        Span::styled(
            " \u{2190}/\u{2192} ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("adjust  "),
        Span::styled(
            "\u{2191}/\u{2193} ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("switch  "),
        Span::styled("Enter ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("confirm  "),
        Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("back"),
    ]);
    f.render_widget(Paragraph::new(help), chunks[7]);
}

fn render_slider(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    active: bool,
    color_fn: impl Fn(f64) -> Color,
) {
    let label_w: u16 = 5;
    let value_w: u16 = 6;
    let bar_w = area.width.saturating_sub(label_w + value_w) as usize;

    let mut spans = vec![];

    let label_style = if active {
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
    };
    spans.push(Span::styled(format!(" {label}: "), label_style));

    let pos = if bar_w > 0 {
        ((value - min) / (max - min) * (bar_w - 1) as f64)
            .round()
            .clamp(0.0, (bar_w - 1) as f64) as usize
    } else {
        0
    };

    for i in 0..bar_w {
        let v = min + (max - min) * (i as f64 / bar_w.max(1) as f64);
        let color = color_fn(v);
        if i == pos {
            spans.push(Span::styled(
                "\u{2503}",
                Style::default().fg(Color::White).bg(color),
            ));
        } else {
            spans.push(Span::styled("\u{2588}", Style::default().fg(color)));
        }
    }

    let value_str = if max > 200.0 {
        format!(" {value:3.0}\u{00B0}")
    } else {
        format!(" {value:3.0}%")
    };
    spans.push(Span::raw(value_str));

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
