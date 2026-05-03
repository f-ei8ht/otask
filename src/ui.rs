use crate::app::{App, InputMode, Mode, Overlay};
use crate::editor::EditorMode;
use crate::filetree::file_icon;
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
const DIM: Color = Color::Rgb(55, 55, 55);
const ACCENT: Color = Color::Rgb(100, 160, 255);
const AMBER: Color = Color::Rgb(255, 180, 50);
const GREEN: Color = Color::Rgb(80, 200, 120);
const RED: Color = Color::Rgb(220, 80, 60);
const OVERLAY_BG: Color = Color::Rgb(15, 15, 15);
const OVERLAY_BORDER: Color = Color::Rgb(45, 45, 45);
const LINE_NUM: Color = Color::Rgb(50, 50, 50);
const LINE_NUM_CUR: Color = Color::Rgb(110, 110, 110);
const CURSOR_LINE: Color = Color::Rgb(16, 16, 16);
const TREE_BG: Color = Color::Rgb(8, 8, 8);
const TREE_SEL: Color = Color::Rgb(28, 40, 60);
const TREE_SEL_FG: Color = Color::Rgb(140, 190, 255);

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    if let Some(ref ed) = app.editor {
        draw_editor_with_tree(f, app, ed, area);
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
        Overlay::FilePicker => draw_file_picker(f, app, area),
        Overlay::None => {}
    }
}

// ─── Editor + tree layout ────────────────────────────────────────────────────

fn draw_editor_with_tree(
    f: &mut Frame,
    app: &App,
    ed: &crate::editor::EditorState,
    area: Rect,
) {
    // Bottom rows: status bar + command line
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let main_area = rows[0];
    let status_area = rows[1];
    let cmd_area = rows[2];

    // Horizontal split: tree | editor
    let cols = if app.tree.is_some() {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(26), Constraint::Min(1)])
            .split(main_area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1)])
            .split(main_area)
    };

    if app.tree.is_some() {
        draw_tree_panel(f, app, cols[0]);
        draw_editor_content(f, ed, cols[1]);
    } else {
        draw_editor_content(f, ed, cols[0]);
    }

    draw_editor_statusbar(f, app, ed, status_area);
    draw_editor_cmdline(f, ed, cmd_area);
}

// ─── File tree panel ─────────────────────────────────────────────────────────

fn draw_tree_panel(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.tree_focused;
    let border_color = if focused { ACCENT } else { OVERLAY_BORDER };

    let title_span = Span::styled(
        " explorer ",
        Style::default().fg(if focused { ACCENT } else { MUTED }),
    );

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color))
        .title(title_span)
        .style(Style::default().bg(TREE_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tree = match app.tree.as_ref() {
        Some(t) => t,
        None => return,
    };

    if tree.visible.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("(empty)", Style::default().fg(DIM))),
            inner,
        );
        return;
    }

    let h = inner.height as usize;
    let sel = tree.selected;
    let scroll = if sel >= h { sel + 1 - h } else { 0 };

    let mut lines: Vec<Line> = Vec::new();
    for (i, entry) in tree.visible.iter().enumerate() {
        let indent = "  ".repeat(entry.depth);
        let icon = if entry.is_dir {
            if entry.expanded { "▾ " } else { "▸ " }
        } else {
            file_icon(&entry.name)
        };

        let is_sel = i == sel;
        let (fg, bg) = if is_sel {
            (TREE_SEL_FG, TREE_SEL)
        } else if entry.is_dir {
            (MUTED, TREE_BG)
        } else {
            (Color::Rgb(180, 180, 180), TREE_BG)
        };

        let icon_color = if entry.is_dir {
            if is_sel { TREE_SEL_FG } else { Color::Rgb(100, 130, 180) }
        } else {
            if is_sel { TREE_SEL_FG } else { DIM }
        };

        let label = format!("{}{}{}", indent, icon, entry.name);
        let label = if label.len() > inner.width as usize {
            format!("{}…", &label[..inner.width.saturating_sub(1) as usize])
        } else {
            format!("{:<width$}", label, width = inner.width as usize)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{}{}", indent, icon),
                Style::default().fg(icon_color).bg(bg),
            ),
            Span::styled(
                format!("{:<width$}", entry.name, width = inner.width.saturating_sub((indent.len() + icon.len()) as u16) as usize),
                Style::default().fg(fg).bg(bg),
            ),
        ]));
        let _ = label; // suppress unused warning
    }

    let para = Paragraph::new(Text::from(lines))
        .scroll((scroll as u16, 0))
        .style(Style::default().bg(TREE_BG));
    f.render_widget(para, inner);

    // Tab hint at bottom of tree
    if inner.height > 2 {
        let hint_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        let hint = if focused {
            " tab → editor"
        } else {
            " tab → tree "
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(DIM))),
            hint_area,
        );
    }
}

// ─── Editor content ───────────────────────────────────────────────────────────

fn draw_editor_content(f: &mut Frame, ed: &crate::editor::EditorState, area: Rect) {
    let visible_h = area.height as usize;
    let gutter_w = (ed.lines.len().to_string().len() as u16 + 2).max(4);

    let text_area = Rect {
        x: area.x + gutter_w,
        y: area.y,
        width: area.width.saturating_sub(gutter_w),
        height: area.height,
    };
    let gutter_area = Rect {
        x: area.x,
        y: area.y,
        width: gutter_w,
        height: area.height,
    };

    let scroll = {
        let mut sr = ed.scroll_row;
        if ed.cursor_row < sr {
            sr = ed.cursor_row;
        } else if ed.cursor_row >= sr + visible_h && visible_h > 0 {
            sr = ed.cursor_row + 1 - visible_h;
        }
        sr
    };

    // Gutter
    let mut gutter_lines: Vec<Line> = Vec::new();
    for i in 0..visible_h {
        let li = scroll + i;
        if li < ed.lines.len() {
            let is_cur = li == ed.cursor_row;
            let style = if is_cur {
                Style::default().fg(LINE_NUM_CUR).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(LINE_NUM)
            };
            gutter_lines.push(Line::from(Span::styled(
                format!("{:>width$} ", li + 1, width = (gutter_w - 1) as usize),
                style,
            )));
        } else {
            gutter_lines.push(Line::from(Span::styled(
                format!("{:>width$} ", "~", width = (gutter_w - 1) as usize),
                Style::default().fg(DIM),
            )));
        }
    }
    f.render_widget(
        Paragraph::new(Text::from(gutter_lines)).style(Style::default().bg(BG)),
        gutter_area,
    );

    // Content — use syntax-highlighted cache when available, fallback to plain
    let tw = text_area.width as usize;
    let mut content_lines: Vec<Line> = Vec::new();
    for i in 0..visible_h {
        let li = scroll + i;
        if li < ed.lines.len() {
            let bg = if li == ed.cursor_row { CURSOR_LINE } else { BG };
            let line = if li < ed.highlight_cache.len() {
                // re-apply bg color to every span so cursor-line bg shows
                let spans: Vec<Span<'static>> = ed.highlight_cache[li]
                    .spans
                    .iter()
                    .map(|s| {
                        Span::styled(
                            s.content.clone(),
                            s.style.bg(bg),
                        )
                    })
                    .collect();
                // pad to full width so bg fills the row
                let used: usize = spans.iter().map(|s| s.content.len()).sum();
                let mut padded = spans;
                if used < tw {
                    padded.push(Span::styled(
                        " ".repeat(tw - used),
                        Style::default().bg(bg),
                    ));
                }
                Line::from(padded)
            } else {
                Line::from(Span::styled(
                    format!("{:<width$}", ed.lines[li], width = tw),
                    Style::default().fg(FG).bg(bg),
                ))
            };
            content_lines.push(line);
        } else {
            content_lines.push(Line::from(""));
        }
    }
    f.render_widget(
        Paragraph::new(Text::from(content_lines)).style(Style::default().bg(BG)),
        text_area,
    );

    // Cursor
    let vis_col = ed.cursor_col.min(text_area.width.saturating_sub(1) as usize);
    let vis_row = ed.cursor_row.saturating_sub(scroll);
    if vis_row < visible_h {
        match ed.mode {
            EditorMode::Normal | EditorMode::Insert => {
                f.set_cursor_position((
                    text_area.x + vis_col as u16,
                    text_area.y + vis_row as u16,
                ));
            }
            _ => {}
        }
    }
}

// ─── Editor status bar ────────────────────────────────────────────────────────

fn draw_editor_statusbar(
    f: &mut Frame,
    app: &App,
    ed: &crate::editor::EditorState,
    area: Rect,
) {
    let fname = ed.file_path.as_deref().unwrap_or("[no name]");
    let dirty = if ed.dirty { " [+]" } else { "" };
    let mode_label = match ed.mode {
        EditorMode::Normal => " NORMAL ",
        EditorMode::Insert => " INSERT ",
        EditorMode::Command => " COMMAND",
    };
    let mode_color = match ed.mode {
        EditorMode::Normal => ACCENT,
        EditorMode::Insert => GREEN,
        EditorMode::Command => AMBER,
    };
    let pos = format!("{}:{} ", ed.cursor_row + 1, ed.cursor_col + 1);
    let status_msg = ed.status_msg.as_deref().unwrap_or("");
    let tree_hint = if app.tree.is_some() { "  ctrl+e tree" } else { "  ctrl+e tree" };

    let line = Line::from(vec![
        Span::styled(mode_label, Style::default().fg(BG).bg(mode_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}{}", fname, dirty), Style::default().fg(MUTED)),
        Span::styled(format!("  {}", status_msg), Style::default().fg(DIM).add_modifier(Modifier::ITALIC)),
        Span::styled(tree_hint, Style::default().fg(DIM)),
        Span::styled(format!("  {}", pos), Style::default().fg(MUTED)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(OVERLAY_BG)),
        area,
    );
}

// ─── Editor command line ──────────────────────────────────────────────────────

fn draw_editor_cmdline(f: &mut Frame, ed: &crate::editor::EditorState, area: Rect) {
    let content = match ed.mode {
        EditorMode::Command => Line::from(vec![
            Span::styled(":", Style::default().fg(FG)),
            Span::styled(&ed.command_buf, Style::default().fg(FG)),
        ]),
        _ => Line::from(Span::styled(
            "  :w  save    :wq  save & quit    :q  quit    :q!  discard",
            Style::default().fg(DIM),
        )),
    };
    f.render_widget(
        Paragraph::new(content).style(Style::default().bg(BG)),
        area,
    );

    if ed.mode == EditorMode::Command {
        f.set_cursor_position((area.x + 1 + ed.command_buf.len() as u16, area.y));
    }
}

// ─── File-open modal ─────────────────────────────────────────────────────────

fn draw_file_open(f: &mut Frame, app: &App, area: Rect) {
    let modal_w = 62u16.min(area.width.saturating_sub(4));
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
        height: 2,
    };

    let hint = Line::from(Span::styled(
        "file path  (esc to cancel, \"new\" for empty buffer)",
        Style::default().fg(DIM),
    ));
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(ACCENT)),
        Span::styled(&app.file_open_buf, Style::default().fg(FG)),
        Span::styled("█", Style::default().fg(ACCENT)),
    ]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(hint), chunks[0]);
    f.render_widget(Paragraph::new(input_line), chunks[1]);
}

// ─── File picker popup ───────────────────────────────────────────────────────

fn draw_file_picker(f: &mut Frame, app: &App, area: Rect) {
    let picker = match app.picker.as_ref() {
        Some(p) => p,
        None => return,
    };

    let max_items = picker.filtered.len().min(12);
    let popup_h = (max_items as u16 + 2).max(4);
    let popup_w = 66u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    // sit just above the input box (which is ~3 rows from the bottom)
    let popup_y = area.y + area.height.saturating_sub(popup_h + 3);

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_w,
        height: popup_h,
    };

    f.render_widget(Clear, popup_area);

    let title = if picker.query.is_empty() {
        " @ files  type to filter  esc cancel ".to_string()
    } else {
        format!(" @{}  esc cancel ", picker.query)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(title, Style::default().fg(ACCENT)))
        .style(Style::default().bg(OVERLAY_BG));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    if picker.filtered.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  no files match",
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            )),
            inner,
        );
        return;
    }

    let h = inner.height as usize;
    let sel = picker.selected;
    let scroll_off = if sel >= h { sel + 1 - h } else { 0 };

    let mut lines: Vec<Line> = Vec::new();
    for (i, name) in picker.filtered.iter().enumerate() {
        let is_sel = i == sel;
        let (fg, bg) = if is_sel { (BG, ACCENT) } else { (FG, OVERLAY_BG) };
        let label = format!("  {}", name);
        let padded = format!("{:<width$}", label, width = inner.width as usize);
        lines.push(Line::from(Span::styled(
            padded,
            Style::default().fg(fg).bg(bg),
        )));
    }

    f.render_widget(
        Paragraph::new(Text::from(lines)).scroll((scroll_off as u16, 0)),
        inner,
    );
}

// ─── Welcome ──────────────────────────────────────────────────────────────────

fn draw_welcome(f: &mut Frame, app: &App, area: Rect) {
    let center_y = area.height / 2;
    let input_y = center_y.saturating_add(4).min(area.height.saturating_sub(6));

    let title_area = Rect {
        x: area.x,
        y: area.y + center_y.saturating_sub(5),
        width: area.width,
        height: 3,
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "otask",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        title_area,
    );

    let (provider_text, pcolor) = if let Some(ref name) = app.provider_name {
        (name.clone(), GREEN)
    } else {
        ("no provider connected".to_string(), MUTED)
    };
    let mode_color = match app.mode { Mode::Plan => ACCENT, Mode::Edit => GREEN };
    let mode_switch_hint = match app.mode { Mode::Plan => "  [e] edit", Mode::Edit => "  [p] plan" };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {} ", app.mode), Style::default().fg(BG).bg(mode_color).add_modifier(Modifier::BOLD)),
            Span::styled(mode_switch_hint, Style::default().fg(DIM)),
            Span::styled("  ·  ", Style::default().fg(DIM)),
            Span::styled(provider_text, Style::default().fg(pcolor)),
        ]))
        .alignment(Alignment::Center),
        Rect { x: area.x, y: area.y + center_y.saturating_sub(2), width: area.width, height: 1 },
    );

    draw_input(
        f,
        app,
        Rect {
            x: area.x + area.width / 5,
            y: area.y + input_y,
            width: area.width * 3 / 5,
            height: 3,
        },
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("ctrl+k", Style::default().fg(MUTED)),
            Span::styled("  commands  ", Style::default().fg(DIM)),
            Span::styled("ctrl+v", Style::default().fg(MUTED)),
            Span::styled("  editor  ", Style::default().fg(DIM)),
            Span::styled("@", Style::default().fg(MUTED)),
            Span::styled(" / ", Style::default().fg(DIM)),
            Span::styled("ctrl+f", Style::default().fg(MUTED)),
            Span::styled("  files", Style::default().fg(DIM)),
        ]))
        .alignment(Alignment::Center),
        Rect { x: area.x, y: area.y + input_y + 4, width: area.width, height: 1 },
    );

    if !app.status.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("· {}", app.status),
                Style::default().fg(status_color(&app.status)),
            ))
            .alignment(Alignment::Center),
            Rect { x: area.x, y: area.y + input_y + 6, width: area.width, height: 1 },
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

    let mut lines: Vec<Line> = vec![Line::from("")];
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
                let lc = if is_focused { ACCENT } else { MUTED };
                let label = if is_focused { "ai ◀" } else { "ai" };
                lines.push(Line::from(Span::styled(format!("{:<5}", label), Style::default().fg(lc))));
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
    let bottom = total.saturating_sub(visible);

    // Persist bottom position so scroll_up can move smoothly from the bottom
    app.bottom_hint.set(bottom);

    let scroll = if app.scroll == usize::MAX {
        bottom
    } else {
        app.scroll.min(bottom)
    };

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .scroll((scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        content_area,
    );

    draw_footer(f, app, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    if area.height < 4 { return; }
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
    let mc = match app.mode { Mode::Plan => ACCENT, Mode::Edit => GREEN };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {}  ", app.mode), Style::default().fg(mc)),
            Span::styled(status_text, Style::default().fg(sc)),
            Span::styled("ctrl+k commands  ctrl+v editor  @ / ctrl+f files  ", Style::default().fg(DIM)),
        ])),
        footer_area,
    );
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
        Line::from(Span::styled(placeholder, Style::default().fg(DIM).add_modifier(Modifier::ITALIC)))
    } else {
        Line::from(Span::styled(app.input.as_str(), Style::default().fg(FG)))
    };

    let mc = match app.mode { Mode::Plan => ACCENT, Mode::Edit => GREEN };
    let pinfo = app.provider_name.as_deref().map(|n| format!(" {} ", n)).unwrap_or_else(|| " no provider ".to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(display), chunks[0]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {} ", app.mode), Style::default().fg(mc).add_modifier(Modifier::BOLD)),
            Span::styled("·", Style::default().fg(DIM)),
            Span::styled(&pinfo, Style::default().fg(MUTED)),
            Span::styled("·", Style::default().fg(DIM)),
            Span::styled(" max", Style::default().fg(AMBER)),
        ])),
        chunks[1],
    );

    if is_typing {
        f.set_cursor_position((inner.x + app.cursor_pos as u16, inner.y));
    }
}

// ─── Command palette ─────────────────────────────────────────────────────────

fn draw_command_palette(f: &mut Frame, app: &App, area: Rect) {
    let modal_w = (area.width * 2 / 3).max(52).min(area.width.saturating_sub(4));
    let modal_h = 32u16.min(area.height.saturating_sub(4));
    let modal_area = Rect {
        x: area.x + (area.width.saturating_sub(modal_w)) / 2,
        y: area.y + (area.height.saturating_sub(modal_h)) / 2,
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

    let entries: &[(&str, &str, &str)] = &[
        ("i",                  "start typing",        "focus the input box"),
        ("/",                  "command mode",         "prefix for slash commands"),
        ("enter",              "send message",         "send your message to the AI"),
        ("esc",                "normal / close",       "exit typing or close this panel"),
        ("p",                  "plan mode",            "switch to plan mode (web search on)"),
        ("e",                  "edit mode",            "switch to edit mode (code, no search)"),
        ("j / ↓",              "scroll down",          "scroll chat"),
        ("k / ↑",              "scroll up",            "scroll chat"),
        ("y",                  "copy response",        "copy last AI response to clipboard"),
        ("s",                  "save response",        "save last AI response to file"),
        ("ctrl+v",             "open editor",          "open built-in code editor + file tree"),
        ("ctrl+n / /new",      "new session",          "clear chat, keep provider"),
        ("ctrl+k",             "command palette",      "open / close this panel"),
        ("ctrl+c / q",         "quit",                 "exit the application"),
        ("", "", ""),
        ("── plan mode ────────────────────────────────────────────────", "", ""),
        ("",                   "",                     "web search via exa ai is automatic"),
        ("",                   "",                     "cerebras decides when to search"),
        ("",                   "",                     "up to 3 searches per response (10 qps)"),
        ("",                   "",                     "search type: fast · results: 5 · highlights"),
        ("", "", ""),
        ("── editor ───────────────────────────────────────────────────", "", ""),
        ("h j k l",            "navigate",             "move cursor"),
        ("i",                  "insert mode",          "start typing at cursor"),
        ("a",                  "append",               "insert after cursor"),
        ("o / O",              "new line",             "open line below / above"),
        ("dd",                 "delete line",          ""),
        ("x",                  "delete char",          ""),
        ("0 / $",              "line start / end",     ""),
        ("g / G",              "file start / end",     ""),
        (":w",                 "save",                 "write file to disk"),
        (":q",                 "quit editor",          "return to AI agent"),
        (":wq",                "save & quit",          "write then return to AI agent"),
        (":q!",                "discard & quit",       "force quit without saving"),
        ("── file tree ────────────────────────────────────────────────", "", ""),
        ("ctrl+e",             "toggle tree",          "show / hide the file explorer"),
        ("tab",                "switch focus",         "toggle focus between tree and editor"),
        ("j / k",              "navigate",             "move selection up / down"),
        ("l / enter",          "open / expand",        "open file or expand directory"),
        ("h",                  "collapse",             "collapse directory"),
        ("r",                  "refresh",              "reload the file tree from disk"),
        ("", "", ""),
        ("── commands ─────────────────────────────────────────────────", "", ""),
        ("/connect cerebras <key>",              "", "connect cerebras · default: gpt-oss-120b"),
        ("/connect cerebras <key> llama3.1-8b", "", "connect cerebras · llama3.1-8b"),
        ("/exa <key>",                           "", "update exa api key at runtime"),
        ("/edit <path>",                         "", "open file directly in editor"),
        ("/new",                                 "", "clear session, keep provider connection"),
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

    for (key, label, desc) in entries {
        if key.is_empty() {
            lines.push(Line::from(""));
        } else if label.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<40}", key), Style::default().fg(MUTED)),
                Span::styled(*desc, Style::default().fg(DIM)),
            ]));
        } else if key.starts_with("──") {
            lines.push(Line::from(Span::styled(
                format!("  {}", key),
                Style::default().fg(Color::Rgb(45, 45, 45)).add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<14}", key), Style::default().fg(ACCENT)),
                Span::styled(format!("{:<22}", label), Style::default().fg(FG)),
                Span::styled(*desc, Style::default().fg(DIM)),
            ]));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .scroll((scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        inner,
    );

    // scrollbar
    if entries.len() > visible_h {
        let sb_h = modal_area.height.saturating_sub(2);
        let ratio = scroll as f32 / entries.len().saturating_sub(visible_h).max(1) as f32;
        let thumb = (ratio * sb_h as f32) as u16;
        for dy in 0..sb_h {
            f.render_widget(
                Paragraph::new(Span::styled(
                    if dy == thumb { "█" } else { "░" },
                    Style::default().fg(DIM),
                )),
                Rect {
                    x: modal_area.x + modal_area.width.saturating_sub(2),
                    y: modal_area.y + 1 + dy,
                    width: 1,
                    height: 1,
                },
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
