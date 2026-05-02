use crate::app::{App, InputMode, Mode, Overlay};
use crate::editor::EditorMode;
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
const LINE_NUM: Color = Color::Rgb(55, 55, 55);
const LINE_NUM_CUR: Color = Color::Rgb(120, 120, 120);
const CURSOR_LINE: Color = Color::Rgb(18, 18, 18);

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    // Editor takes full screen
    if let Some(ref ed) = app.editor {
        draw_editor(f, ed, area);
        return;
    }

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

    match app.overlay {
        Overlay::CommandPalette => draw_command_palette(f, app, area),
        Overlay::FileOpen => draw_file_open(f, app, area),
        Overlay::None => {}
    }
}

// ─── Editor ──────────────────────────────────────────────────────────────────

fn draw_editor(f: &mut Frame, ed: &crate::editor::EditorState, area: Rect) {
    // Layout: content | statusbar (1) | cmdline (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let content_area = chunks[0];
    let status_area = chunks[1];
    let cmd_area = chunks[2];

    let visible_h = content_area.height as usize;
    let gutter_w = (ed.lines.len().to_string().len() + 2).max(4) as u16;

    let text_area = Rect {
        x: content_area.x + gutter_w,
        y: content_area.y,
        width: content_area.width.saturating_sub(gutter_w),
        height: content_area.height,
    };
    let gutter_area = Rect {
        x: content_area.x,
        y: content_area.y,
        width: gutter_w,
        height: content_area.height,
    };

    // Compute scroll (mutable clone trick: we read scroll_row directly)
    let scroll = {
        let mut sr = ed.scroll_row;
        if ed.cursor_row < sr {
            sr = ed.cursor_row;
        } else if ed.cursor_row >= sr + visible_h {
            sr = ed.cursor_row + 1 - visible_h;
        }
        sr
    };

    // Gutter (line numbers)
    let mut gutter_lines: Vec<Line> = Vec::new();
    for i in 0..visible_h {
        let line_idx = scroll + i;
        if line_idx < ed.lines.len() {
            let num = line_idx + 1;
            let (color, bold) = if line_idx == ed.cursor_row {
                (LINE_NUM_CUR, true)
            } else {
                (LINE_NUM, false)
            };
            let mut style = Style::default().fg(color);
            if bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            gutter_lines.push(Line::from(vec![
                Span::styled(format!("{:>width$} ", num, width = (gutter_w - 1) as usize), style),
            ]));
        } else {
            gutter_lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>width$} ", "~", width = (gutter_w - 1) as usize),
                    Style::default().fg(DIM),
                ),
            ]));
        }
    }
    f.render_widget(
        Paragraph::new(Text::from(gutter_lines)).style(Style::default().bg(BG)),
        gutter_area,
    );

    // Content lines
    let mut content_lines: Vec<Line> = Vec::new();
    for i in 0..visible_h {
        let line_idx = scroll + i;
        if line_idx < ed.lines.len() {
            let text = &ed.lines[line_idx];
            let is_cursor_line = line_idx == ed.cursor_row;
            let bg = if is_cursor_line { CURSOR_LINE } else { BG };
            let style = Style::default().fg(FG).bg(bg);
            content_lines.push(Line::from(Span::styled(
                format!("{:<width$}", text, width = text_area.width as usize),
                style,
            )));
        } else {
            content_lines.push(Line::from(""));
        }
    }
    f.render_widget(
        Paragraph::new(Text::from(content_lines)).style(Style::default().bg(BG)),
        text_area,
    );

    // Status bar
    let fname = ed
        .file_path
        .as_deref()
        .unwrap_or("[no name]");
    let dirty = if ed.dirty { " [+]" } else { "" };
    let mode_label = match ed.mode {
        EditorMode::Normal => " NORMAL ",
        EditorMode::Insert => " INSERT ",
        EditorMode::Command => " COMMAND ",
    };
    let mode_color = match ed.mode {
        EditorMode::Normal => ACCENT,
        EditorMode::Insert => GREEN,
        EditorMode::Command => AMBER,
    };
    let pos = format!(" {}:{} ", ed.cursor_row + 1, ed.cursor_col + 1);

    let status_msg_text = ed.status_msg.as_deref().unwrap_or("");

    let status_line = Line::from(vec![
        Span::styled(mode_label, Style::default().fg(BG).bg(mode_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}{}", fname, dirty), Style::default().fg(MUTED)),
        Span::styled(format!("  {}", status_msg_text), Style::default().fg(DIM).add_modifier(Modifier::ITALIC)),
        Span::styled(pos, Style::default().fg(MUTED)),
    ]);
    f.render_widget(
        Paragraph::new(status_line).style(Style::default().bg(OVERLAY_BG)),
        status_area,
    );

    // Command line
    let cmd_content = match ed.mode {
        EditorMode::Command => {
            Line::from(vec![
                Span::styled(":", Style::default().fg(FG)),
                Span::styled(&ed.command_buf, Style::default().fg(FG)),
            ])
        }
        _ => Line::from(vec![
            Span::styled("  :w  save    :wq  save & quit    :q  quit    :q!  discard & quit", Style::default().fg(DIM)),
        ]),
    };
    f.render_widget(
        Paragraph::new(cmd_content).style(Style::default().bg(BG)),
        cmd_area,
    );

    // Place terminal cursor
    match ed.mode {
        EditorMode::Insert | EditorMode::Normal => {
            let visible_col = ed.cursor_col.min(text_area.width.saturating_sub(1) as usize);
            let visible_row = ed.cursor_row.saturating_sub(scroll);
            if visible_row < visible_h {
                f.set_cursor_position((
                    text_area.x + visible_col as u16,
                    text_area.y + visible_row as u16,
                ));
            }
        }
        EditorMode::Command => {
            f.set_cursor_position((
                cmd_area.x + 1 + ed.command_buf.len() as u16,
                cmd_area.y,
            ));
        }
    }
}

// ─── File-open modal ─────────────────────────────────────────────────────────

fn draw_file_open(f: &mut Frame, app: &App, area: Rect) {
    let modal_w = 60u16.min(area.width.saturating_sub(4));
    let modal_h = 5u16;
    let modal_x = area.x + (area.width.saturating_sub(modal_w)) / 2;
    let modal_y = area.y + (area.height.saturating_sub(modal_h)) / 2;

    let modal_area = Rect { x: modal_x, y: modal_y, width: modal_w, height: modal_h };

    f.render_widget(Clear, modal_area);
    f.render_widget(
        Block::default()
            .style(Style::default().bg(OVERLAY_BG))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(OVERLAY_BORDER))
            .title(Span::styled(" open file ", Style::default().fg(MUTED))),
        modal_area,
    );

    let inner = Rect {
        x: modal_area.x + 2,
        y: modal_area.y + 1,
        width: modal_area.width.saturating_sub(4),
        height: modal_area.height.saturating_sub(2),
    };

    let hint = Line::from(Span::styled(
        "path to open or create  (esc to cancel)",
        Style::default().fg(DIM),
    ));
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(ACCENT)),
        Span::styled(&app.file_open_buf, Style::default().fg(FG)),
        Span::styled("_", Style::default().fg(ACCENT).add_modifier(Modifier::SLOW_BLINK)),
    ]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(hint), chunks[0]);
    f.render_widget(Paragraph::new(input_line), chunks[1]);
}

// ─── Welcome screen ───────────────────────────────────────────────────────────

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
        Span::styled("otask", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
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

    let mode_color = match app.mode {
        Mode::Plan => ACCENT,
        Mode::Edit => GREEN,
    };
    let mode_switch_hint = match app.mode {
        Mode::Plan => "  [e] edit",
        Mode::Edit => "  [p] plan",
    };

    let mode_line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.mode),
            Style::default().fg(BG).bg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(mode_switch_hint, Style::default().fg(DIM)),
        Span::styled("  ·  ", Style::default().fg(DIM)),
        Span::styled(provider_text, Style::default().fg(pcolor)),
    ]);
    f.render_widget(
        Paragraph::new(mode_line).alignment(Alignment::Center),
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
        Span::styled("  commands  ", Style::default().fg(DIM)),
        Span::styled("ctrl+v", Style::default().fg(MUTED)),
        Span::styled("  editor", Style::default().fg(DIM)),
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
        let status_color = status_color(&app.status);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("· {}", app.status),
                Style::default().fg(status_color),
            ))
            .alignment(Alignment::Center),
            status_area,
        );
    }
}

// ─── Messages ────────────────────────────────────────────────────────────────

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
                lines.push(Line::from(Span::styled("you  ", Style::default().fg(DIM))));
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
                lines.push(Line::from(Span::styled(
                    format!("{:<5}", label),
                    Style::default().fg(label_color),
                )));
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
            Span::styled(
                "thinking…",
                Style::default().fg(DIM).add_modifier(Modifier::ITALIC),
            ),
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

    let (status_text, sc) = if !app.status.is_empty() {
        (format!("· {}  ", app.status), status_color(&app.status))
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
        Span::styled(status_text, Style::default().fg(sc)),
        Span::styled("ctrl+k commands  ctrl+v editor  ", Style::default().fg(DIM)),
    ]);
    f.render_widget(Paragraph::new(footer).alignment(Alignment::Left), footer_area);
}

// ─── Input ───────────────────────────────────────────────────────────────────

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let is_typing = app.input_mode == InputMode::Typing;
    let border_color = if is_typing { ACCENT } else { DIM };

    let placeholder = if app.messages.is_empty() {
        "Ask anything…  \"What is the tech stack of this project?\""
    } else {
        "Ask anything…"
    };

    let display = if app.input.is_empty() && !is_typing {
        Line::from(Span::styled(
            placeholder,
            Style::default().fg(DIM).add_modifier(Modifier::ITALIC),
        ))
    } else {
        Line::from(Span::styled(app.input.as_str(), Style::default().fg(FG)))
    };

    let mode_color = match app.mode {
        Mode::Plan => ACCENT,
        Mode::Edit => GREEN,
    };

    let provider_info = if let Some(ref name) = app.provider_name {
        format!(" {} ", name)
    } else {
        " no provider ".to_string()
    };

    let subtitle = Line::from(vec![
        Span::styled(
            format!(" {} ", app.mode),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
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
        f.set_cursor_position((inner.x + app.cursor_pos as u16, inner.y));
    }
}

// ─── Command palette ─────────────────────────────────────────────────────────

fn draw_command_palette(f: &mut Frame, app: &App, area: Rect) {
    let modal_w = (area.width * 2 / 3).max(50).min(area.width.saturating_sub(4));
    let modal_h = 30u16.min(area.height.saturating_sub(4));
    let modal_x = area.x + (area.width.saturating_sub(modal_w)) / 2;
    let modal_y = area.y + (area.height.saturating_sub(modal_h)) / 2;
    let modal_area = Rect { x: modal_x, y: modal_y, width: modal_w, height: modal_h };

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
        ("i",                "start typing",       "focus the input box"),
        ("/",                "command mode",        "prefix for slash commands"),
        ("enter",            "send message",        "send your message to the AI"),
        ("esc",              "normal mode / close", "exit typing or close this panel"),
        ("p",                "plan mode",           "switch to planning mode"),
        ("e",                "edit / build mode",   "switch to edit mode"),
        ("j / ↓",            "scroll down",         "scroll chat"),
        ("k / ↑",            "scroll up",           "scroll chat"),
        ("y",                "copy response",       "copy last response to clipboard"),
        ("s",                "save response",       "save last response to response_N.md"),
        ("ctrl+v",           "open editor",         "open built-in code editor"),
        ("ctrl+n / /new",    "new session",         "clear chat, keep provider"),
        ("ctrl+k",           "command palette",     "open / close this panel"),
        ("ctrl+c / q",       "quit",                "exit the application"),
        ("", "", ""),
        ("── editor (ctrl+v) ──", "", ""),
        ("h j k l",          "navigate",            "move cursor"),
        ("i",                "insert mode",         "start typing"),
        ("a",                "append",              "insert after cursor"),
        ("o / O",            "new line",            "open line below / above"),
        ("dd",               "delete line",         ""),
        ("x",                "delete char",         ""),
        ("0 / $",            "line start / end",    ""),
        ("g / G",            "file start / end",    ""),
        (":w",               "save",                "write file to disk"),
        (":q",               "quit editor",         "return to AI agent"),
        (":wq",              "save & quit",         "write then return"),
        (":q!",              "discard & quit",      "force quit without saving"),
        ("", "", ""),
        ("/connect cerebras <key>",           "", "connect cerebras (gpt-oss-120b)"),
        ("/connect cerebras <key> llama3.1-8b", "", ""),
        ("/connect anthropic <key>",           "", "connect anthropic (claude-opus-4-5)"),
        ("/connect codex <key>",               "", "connect openai codex (gpt-4o)"),
        ("/edit <path>",                       "", "open file directly in editor"),
    ];

    let visible_h = inner.height as usize;
    let scroll = app.palette_scroll.min(entries.len().saturating_sub(visible_h));

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("commands", Style::default().fg(FG).add_modifier(Modifier::BOLD)),
            Span::styled("  ·  esc to close", Style::default().fg(DIM)),
        ]),
        Line::from(Span::styled(
            "─".repeat(inner.width as usize),
            Style::default().fg(OVERLAY_BORDER),
        )),
    ];

    for (key, label, desc) in &entries {
        if key.is_empty() {
            lines.push(Line::from(""));
        } else if label.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<38}", key), Style::default().fg(MUTED)),
                Span::styled(*desc, Style::default().fg(DIM)),
            ]));
        } else if key.starts_with("──") {
            lines.push(Line::from(Span::styled(
                format!("  {}", key),
                Style::default().fg(DIM).add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<14}", key), Style::default().fg(ACCENT)),
                Span::styled(format!("{:<22}", label), Style::default().fg(FG)),
                Span::styled(*desc, Style::default().fg(DIM)),
            ]));
        }
    }

    let para = Paragraph::new(Text::from(lines))
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);

    // scrollbar
    if entries.len() > visible_h {
        let sb_x = modal_area.x + modal_area.width.saturating_sub(2);
        let sb_h = modal_area.height.saturating_sub(2);
        let ratio =
            scroll as f32 / entries.len().saturating_sub(visible_h).max(1) as f32;
        let thumb_y = (ratio * sb_h as f32) as u16;
        for dy in 0..sb_h {
            let ch = if dy == thumb_y { "█" } else { "░" };
            f.render_widget(
                Paragraph::new(Span::styled(ch, Style::default().fg(DIM))),
                Rect { x: sb_x, y: modal_area.y + 1 + dy, width: 1, height: 1 },
            );
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn status_color(s: &str) -> Color {
    if s.contains("connected") || s.contains("saved") || s.contains("copied") || s.contains("written") {
        GREEN
    } else if s.contains("failed") || s.contains("error") || s.contains("unknown") || s.contains("E:") {
        RED
    } else {
        AMBER
    }
}
