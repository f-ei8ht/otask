use crate::app::{App, InputMode, Mode, Overlay};
use crate::markdown::md_to_text;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

const BG: Color = Color::Rgb(0, 0, 0);
const FG: Color = Color::Rgb(220, 220, 220);
const MUTED: Color = Color::Rgb(100, 100, 100);
const DIM: Color = Color::Rgb(60, 60, 60);
const ACCENT: Color = Color::Rgb(100, 160, 255);
const AMBER: Color = Color::Rgb(255, 180, 50);
const GREEN: Color = Color::Rgb(80, 200, 120);
const RED: Color = Color::Rgb(220, 80, 60);
const OVERLAY_BG: Color = Color::Rgb(15, 15, 15);
const OVERLAY_BORDER: Color = Color::Rgb(45, 45, 45);

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    if app.messages.is_empty() {
        draw_welcome(f, app, area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        draw_messages(f, app, chunks[0]);
        draw_input(f, app, chunks[1]);
    }

    if app.overlay == Overlay::CommandPalette {
        draw_command_palette(f, app, area);
    }
}

fn draw_welcome(f: &mut Frame, app: &App, area: Rect) {
    let center_y = area.height / 2;
    let input_y = center_y.saturating_add(4).min(area.height.saturating_sub(6));

    let title_area = Rect {
        x: area.x,
        y: area.y + center_y.saturating_sub(5),
        width: area.width,
        height: 3,
    };

    let title_line = Line::from(vec![
        Span::styled(
            "otask",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(title_line).alignment(Alignment::Center),
        title_area,
    );

    let provider_area = Rect {
        x: area.x,
        y: area.y + center_y.saturating_sub(2),
        width: area.width,
        height: 1,
    };

    let (provider_text, pcolor) = if let Some(ref name) = app.provider_name {
        (format!("{}", name), GREEN)
    } else {
        ("no provider connected".to_string(), MUTED)
    };

    let mode_indicator = format!("{}  ·  {}", app.mode, provider_text);
    f.render_widget(
        Paragraph::new(Span::styled(mode_indicator, Style::default().fg(pcolor)))
            .alignment(Alignment::Center),
        provider_area,
    );

    let input_area = Rect {
        x: area.x + area.width / 5,
        y: area.y + input_y,
        width: area.width * 3 / 5,
        height: 3,
    };

    draw_input(f, app, input_area);

    let hints_area = Rect {
        x: area.x,
        y: area.y + input_y + 4,
        width: area.width,
        height: 1,
    };

    let hints = Line::from(vec![
        Span::styled("ctrl+k", Style::default().fg(MUTED)),
        Span::styled("  commands", Style::default().fg(DIM)),
    ]);
    f.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        hints_area,
    );

    if !app.status.is_empty() {
        let status_area = Rect {
            x: area.x,
            y: area.y + input_y + 6,
            width: area.width,
            height: 1,
        };
        let status_color = if app.status.contains("connected") || app.status.contains("saved") || app.status.contains("copied") {
            GREEN
        } else if app.status.contains("failed") || app.status.contains("error") || app.status.contains("unknown") {
            RED
        } else {
            AMBER
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!("· {}", app.status), Style::default().fg(status_color)))
                .alignment(Alignment::Center),
            status_area,
        );
    }
}

fn draw_messages(f: &mut Frame, app: &App, area: Rect) {
    let content_area = Rect {
        x: area.x + (area.width / 6).min(8),
        y: area.y,
        width: area.width - 2 * (area.width / 6).min(8),
        height: area.height,
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));

    for (idx, msg) in app.messages.iter().enumerate() {
        match msg.role.as_str() {
            "user" => {
                lines.push(Line::from(vec![
                    Span::styled("you  ", Style::default().fg(DIM)),
                ]));
                for line in msg.content.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("     ", Style::default().fg(DIM)),
                        Span::styled(line, Style::default().fg(FG)),
                    ]));
                }
                lines.push(Line::from(""));
            }
            "assistant" => {
                let is_focused = app.focused_msg == Some(idx);
                let label_color = if is_focused { ACCENT } else { MUTED };
                let label = if is_focused { "ai ◀" } else { "ai" };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:<5}", label),
                        Style::default().fg(label_color),
                    ),
                ]));

                let md = md_to_text(&msg.content);
                for md_line in md.lines {
                    let mut spans = vec![Span::styled("     ", Style::default().fg(DIM))];
                    spans.extend(md_line.spans);
                    lines.push(Line::from(spans));
                }
                lines.push(Line::from(""));
            }
            "error" => {
                lines.push(Line::from(vec![
                    Span::styled("err  ", Style::default().fg(RED)),
                    Span::styled(&msg.content, Style::default().fg(RED)),
                ]));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    if app.is_loading {
        lines.push(Line::from(vec![
            Span::styled("ai   ", Style::default().fg(MUTED)),
            Span::styled("thinking…", Style::default().fg(DIM).add_modifier(Modifier::ITALIC)),
        ]));
    }

    let total = lines.len();
    let visible = content_area.height as usize;

    let scroll = if app.scroll == usize::MAX {
        total.saturating_sub(visible)
    } else {
        app.scroll.min(total.saturating_sub(visible))
    };

    let para = Paragraph::new(Text::from(lines))
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(para, content_area);

    draw_footer(f, app, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    if area.height < 4 {
        return;
    }

    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    let (status_text, status_color) = if !app.status.is_empty() {
        let color = if app.status.contains("connected") || app.status.contains("saved") || app.status.contains("copied") {
            GREEN
        } else if app.status.contains("failed") || app.status.contains("error") || app.status.contains("unknown") {
            RED
        } else {
            AMBER
        };
        (format!("· {}  ", app.status), color)
    } else if let Some(ref name) = app.provider_name {
        (format!("· {}  ", name), DIM)
    } else {
        (String::new(), DIM)
    };

    let mode_color = match app.mode {
        Mode::Plan => ACCENT,
        Mode::Edit => GREEN,
    };

    let footer = Line::from(vec![
        Span::styled(format!(" {}  ", app.mode), Style::default().fg(mode_color)),
        Span::styled(status_text, Style::default().fg(status_color)),
        Span::styled("ctrl+k commands  ", Style::default().fg(DIM)),
    ]);

    f.render_widget(
        Paragraph::new(footer).alignment(Alignment::Left),
        footer_area,
    );
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let is_typing = app.input_mode == InputMode::Typing;

    let border_color = if is_typing { ACCENT } else { DIM };

    let placeholder = if app.messages.is_empty() {
        "Ask anything…  \"What is the tech stack of this project?\""
    } else {
        "Ask anything…"
    };

    let display = if app.input.is_empty() && !is_typing {
        Line::from(Span::styled(placeholder, Style::default().fg(DIM).add_modifier(Modifier::ITALIC)))
    } else {
        Line::from(Span::styled(app.input.as_str(), Style::default().fg(FG)))
    };

    let mode_color = match app.mode {
        Mode::Plan => ACCENT,
        Mode::Edit => GREEN,
    };

    let label = format!(" {}  ", app.mode);

    let provider_info = if let Some(ref name) = app.provider_name {
        format!(" {} ", name)
    } else {
        " no provider ".to_string()
    };

    let subtitle = Line::from(vec![
        Span::styled(&label, Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        Span::styled("·", Style::default().fg(DIM)),
        Span::styled(&provider_info, Style::default().fg(MUTED)),
        Span::styled("·", Style::default().fg(DIM)),
        Span::styled(" max", Style::default().fg(AMBER)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(display), input_chunks[0]);
    f.render_widget(Paragraph::new(subtitle), input_chunks[1]);

    if is_typing {
        let cursor_x = inner.x + app.cursor_pos as u16;
        let cursor_y = inner.y;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_command_palette(f: &mut Frame, app: &App, area: Rect) {
    let modal_w = (area.width * 2 / 3).max(50).min(area.width.saturating_sub(4));
    let modal_h = 28u16.min(area.height.saturating_sub(4));
    let modal_x = area.x + (area.width.saturating_sub(modal_w)) / 2;
    let modal_y = area.y + (area.height.saturating_sub(modal_h)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_w,
        height: modal_h,
    };

    f.render_widget(Clear, modal_area);
    f.render_widget(
        Block::default()
            .style(Style::default().bg(OVERLAY_BG))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(OVERLAY_BORDER)),
        modal_area,
    );

    let inner = Rect {
        x: modal_area.x + 2,
        y: modal_area.y + 1,
        width: modal_area.width.saturating_sub(4),
        height: modal_area.height.saturating_sub(2),
    };

    let entries: Vec<(&str, &str, &str)> = vec![
        ("i", "start typing", "focus the input box"),
        ("/", "command mode", "prefix for slash commands"),
        ("enter", "send message", "send your message to the AI"),
        ("esc", "normal mode / close", "exit typing or close this panel"),
        ("p", "plan mode", "switch to planning mode"),
        ("b", "edit / build mode", "switch to edit mode"),
        ("j / ↓", "scroll down", "scroll chat or navigate responses"),
        ("k / ↑", "scroll up", "scroll chat or navigate responses"),
        ("y", "copy response", "copy last response to clipboard"),
        ("s", "save response", "save last response to response_N.md"),
        ("ctrl+k", "command palette", "open / close this panel"),
        ("ctrl+c / q", "quit", "exit the application"),
        ("", "", ""),
        ("/connect cerebras <key>", "", "connect cerebras (gpt-oss-120b)"),
        ("/connect cerebras <key> llama3.1-8b", "", ""),
        ("/connect anthropic <key>", "", "connect anthropic (claude-opus-4-5)"),
        ("/connect codex <key>", "", "connect openai codex (gpt-4o)"),
    ];

    let visible_h = inner.height as usize;
    let scroll = app.palette_scroll.min(entries.len().saturating_sub(visible_h));

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("commands", Style::default().fg(FG).add_modifier(Modifier::BOLD)),
            Span::styled("  ·  esc to close", Style::default().fg(DIM)),
        ]),
        Line::from(Span::styled("─".repeat(inner.width as usize), Style::default().fg(OVERLAY_BORDER))),
    ];

    for (key, label, desc) in &entries {
        if key.is_empty() {
            lines.push(Line::from(""));
            continue;
        }
        if label.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<35}", key), Style::default().fg(MUTED)),
                Span::styled(*desc, Style::default().fg(DIM)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<12}", key), Style::default().fg(ACCENT)),
                Span::styled(format!("{:<22}", label), Style::default().fg(FG)),
                Span::styled(*desc, Style::default().fg(DIM)),
            ]));
        }
    }

    let para = Paragraph::new(Text::from(lines))
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(para, inner);

    if entries.len() > visible_h {
        let scrollbar_area = Rect {
            x: modal_area.x + modal_area.width.saturating_sub(2),
            y: modal_area.y + 1,
            width: 1,
            height: modal_area.height.saturating_sub(2),
        };
        let ratio = scroll as f32 / entries.len().saturating_sub(visible_h).max(1) as f32;
        let thumb_y = (ratio * scrollbar_area.height as f32) as u16;
        for dy in 0..scrollbar_area.height {
            let ch = if dy == thumb_y { "█" } else { "░" };
            let row_area = Rect {
                x: scrollbar_area.x,
                y: scrollbar_area.y + dy,
                width: 1,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(Span::styled(ch, Style::default().fg(DIM))),
                row_area,
            );
        }
    }
}
