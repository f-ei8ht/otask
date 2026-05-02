use crate::app::{App, InputMode, Mode};
use crate::markdown::md_to_text;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, BorderType, Padding, Paragraph, Wrap},
};

const BG: Color = Color::Rgb(22, 23, 35);
const FG: Color = Color::Rgb(250, 250, 252);
const CARD: Color = Color::Rgb(33, 34, 46);
const SIDEBAR: Color = Color::Rgb(33, 34, 46);
const MUTED_BG: Color = Color::Rgb(42, 43, 58);
const MUTED_FG: Color = Color::Rgb(168, 170, 185);
const PRIMARY: Color = Color::Rgb(75, 100, 220);
const PRIMARY_FG: Color = Color::Rgb(235, 238, 255);
const ACCENT: Color = Color::Rgb(80, 130, 230);
const DESTRUCTIVE: Color = Color::Rgb(220, 80, 55);
const BORDER: Color = Color::Rgb(50, 52, 68);
const PLAN_COLOR: Color = Color::Rgb(80, 130, 230);
const EDIT_COLOR: Color = Color::Rgb(100, 200, 150);
const SUCCESS: Color = Color::Rgb(100, 200, 150);

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    f.render_widget(
        Block::default().style(Style::default().bg(BG)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_messages(f, app, chunks[1]);
    draw_input(f, app, chunks[2]);
    draw_statusbar(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mode_color = match app.mode {
        Mode::Plan => PLAN_COLOR,
        Mode::Edit => EDIT_COLOR,
    };

    let mode_icon = match app.mode {
        Mode::Plan => "◆",
        Mode::Edit => "◇",
    };

    let provider_text = app
        .provider_name
        .as_deref()
        .unwrap_or("No Provider");

    let provider_color = if app.provider.is_some() {
        SUCCESS
    } else {
        MUTED_FG
    };

    let loading_indicator = if app.is_loading { " ⟳ " } else { "" };

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(1),
            Constraint::Length(30),
        ])
        .split(area);

    let mode_block = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            mode_icon,
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{} MODE", app.mode),
            Style::default()
                .fg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(mode_color))
            .style(Style::default().bg(CARD)),
    );

    let title_block = Paragraph::new(Line::from(vec![
        Span::styled(
            " AI Coding Agent",
            Style::default()
                .fg(PRIMARY_FG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(loading_indicator, Style::default().fg(ACCENT)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG)),
    );

    let provider_block = Paragraph::new(Line::from(vec![
        Span::styled(" Provider: ", Style::default().fg(MUTED_FG)),
        Span::styled(
            provider_text,
            Style::default()
                .fg(provider_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ]))
    .alignment(Alignment::Right)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(CARD)),
    );

    f.render_widget(mode_block, header_chunks[0]);
    f.render_widget(title_block, header_chunks[1]);
    f.render_widget(provider_block, header_chunks[2]);
}

fn draw_messages(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .padding(Padding::new(1, 1, 0, 0))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        match msg.role.as_str() {
            "system" => {
                lines.push(Line::from(vec![
                    Span::styled("  ╔ ", Style::default().fg(MUTED_FG)),
                    Span::styled(
                        "System",
                        Style::default().fg(MUTED_FG).add_modifier(Modifier::ITALIC),
                    ),
                ]));
                for line in msg.content.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("  ║ ", Style::default().fg(MUTED_BG)),
                        Span::styled(line, Style::default().fg(MUTED_FG)),
                    ]));
                }
                lines.push(Line::from(""));
            }
            "user" => {
                lines.push(Line::from(vec![
                    Span::styled("  ╔ ", Style::default().fg(ACCENT)),
                    Span::styled(
                        "You",
                        Style::default()
                            .fg(ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                for line in msg.content.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("  ║ ", Style::default().fg(ACCENT)),
                        Span::styled(line, Style::default().fg(FG)),
                    ]));
                }
                lines.push(Line::from(""));
            }
            "assistant" => {
                lines.push(Line::from(vec![
                    Span::styled("  ╔ ", Style::default().fg(PRIMARY)),
                    Span::styled(
                        "Assistant",
                        Style::default()
                            .fg(PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                let md = md_to_text(&msg.content);
                for md_line in md.lines {
                    let mut prefixed_spans = vec![
                        Span::styled("  ║ ", Style::default().fg(BORDER)),
                    ];
                    prefixed_spans.extend(md_line.spans);
                    lines.push(Line::from(prefixed_spans));
                }
                lines.push(Line::from(""));
            }
            "error" => {
                lines.push(Line::from(vec![
                    Span::styled("  ✗ ", Style::default().fg(DESTRUCTIVE)),
                    Span::styled(
                        &msg.content,
                        Style::default().fg(DESTRUCTIVE),
                    ),
                ]));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    if app.is_loading {
        lines.push(Line::from(vec![
            Span::styled("  ╔ ", Style::default().fg(PRIMARY)),
            Span::styled(
                "Assistant",
                Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ║ ", Style::default().fg(BORDER)),
            Span::styled(
                "Thinking...",
                Style::default()
                    .fg(MUTED_FG)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    let total_lines = lines.len();
    let visible_height = inner.height as usize;

    let scroll = if app.scroll == usize::MAX {
        total_lines.saturating_sub(visible_height)
    } else {
        app.scroll.min(total_lines.saturating_sub(visible_height))
    };

    let paragraph = Paragraph::new(Text::from(lines))
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, inner);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let is_typing = app.input_mode == InputMode::Typing;

    let (border_color, label, label_color) = if is_typing {
        (PRIMARY, " ❯ ", PRIMARY)
    } else {
        (BORDER, " › ", MUTED_FG)
    };

    let display_text = if app.input.is_empty() && !is_typing {
        Span::styled(
            "Press [i] to type, [/] for commands, [p] Plan, [b] Edit, [q] Quit",
            Style::default().fg(MUTED_FG).add_modifier(Modifier::ITALIC),
        )
    } else {
        Span::styled(app.input.as_str(), Style::default().fg(FG))
    };

    let input_line = Line::from(vec![
        Span::styled(label, Style::default().fg(label_color).add_modifier(Modifier::BOLD)),
        display_text,
    ]);

    let input_widget = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(CARD)),
    );

    f.render_widget(input_widget, area);

    if is_typing {
        let cursor_x = area.x + 1 + 3 + app.cursor_pos as u16;
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let mode_color = match app.mode {
        Mode::Plan => PLAN_COLOR,
        Mode::Edit => EDIT_COLOR,
    };

    let mode_hint = match app.mode {
        Mode::Plan => " [b] → Edit ",
        Mode::Edit => " [p] → Plan ",
    };

    let status_line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.mode),
            Style::default()
                .fg(BG)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(mode_hint, Style::default().fg(MUTED_FG)),
        Span::styled("│", Style::default().fg(BORDER)),
        Span::styled(" [i] Type  [/] Command  [s] Save  [↑↓] Scroll  [q] Quit ", Style::default().fg(MUTED_FG)),
        Span::styled("│", Style::default().fg(BORDER)),
        Span::styled(
            format!(" {} ", app.status),
            Style::default().fg(MUTED_FG).add_modifier(Modifier::ITALIC),
        ),
    ]);

    let statusbar = Paragraph::new(status_line)
        .style(Style::default().bg(SIDEBAR));

    f.render_widget(statusbar, area);
}
