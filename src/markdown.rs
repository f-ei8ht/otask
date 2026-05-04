use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

const FG: Color = Color::Rgb(250, 250, 252);
const HEADING1: Color = Color::Rgb(150, 180, 255);
const HEADING2: Color = Color::Rgb(120, 160, 240);
const HEADING3: Color = Color::Rgb(100, 140, 220);
const CODE_FG: Color = Color::Rgb(255, 200, 100);
const CODE_BG: Color = Color::Rgb(35, 36, 50);
const BLOCK_QUOTE: Color = Color::Rgb(168, 170, 185);
const RULE_COLOR: Color = Color::Rgb(50, 52, 68);
const LIST_BULLET: Color = Color::Rgb(80, 130, 230);
const LINK_COLOR: Color = Color::Rgb(100, 180, 255);
const TABLE_BORDER: Color = Color::Rgb(60, 65, 90);
const TABLE_HEADER: Color = Color::Rgb(140, 180, 255);
const TABLE_FG: Color = Color::Rgb(200, 205, 220);

pub fn md_to_text(md: &str, wrap_width: usize) -> Text<'static> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, options);

    let mut lines: Vec<Line<'static>> = vec![];
    let mut current: Vec<Span<'static>> = vec![];
    let mut style = Style::default().fg(FG);
    let mut list_depth: usize = 0;
    let mut ordered_counters: Vec<u64> = vec![];
    let mut in_code_block = false;
    let mut indent_prefix = String::new();

    // Table state
    let mut in_table = false;
    let mut _in_table_head = false;
    let mut in_table_cell = false;
    let mut table_rows: Vec<Vec<String>> = vec![];
    let mut current_row: Vec<String> = vec![];
    let mut current_cell = String::new();
    let mut head_row_count: usize = 0;

    for event in parser {
        match event {
            // ── Text ─────────────────────────────────────────────────────────
            Event::Text(t) => {
                let text = t.into_string();
                if in_table_cell {
                    current_cell.push_str(&text);
                } else if in_code_block {
                    for line in text.lines() {
                        let wrapped = wrap_code_line(line, wrap_width);
                        lines.push(Line::from(vec![Span::styled(
                            format!("  {}", wrapped),
                            Style::default().fg(CODE_FG).bg(CODE_BG),
                        )]));
                    }
                } else {
                    current.push(Span::styled(text, style));
                }
            }

            Event::Code(t) => {
                let s = t.into_string();
                if in_table_cell {
                    current_cell.push_str(&s);
                } else {
                    current.push(Span::styled(
                        format!("`{}`", s),
                        Style::default().fg(CODE_FG).bg(CODE_BG),
                    ));
                }
            }

            // ── Tables ───────────────────────────────────────────────────────
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_rows.clear();
                head_row_count = 0;
                if !current.is_empty() {
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                }
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                in_table_cell = false;
                lines.extend(render_table(&table_rows, head_row_count, wrap_width));
                lines.push(Line::raw(""));
            }

            Event::Start(Tag::TableHead) => {
                _in_table_head = true;
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                _in_table_head = false;
                if !current_row.is_empty() {
                    head_row_count = table_rows.len() + 1;
                    table_rows.push(current_row.drain(..).collect());
                }
            }

            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !current_row.is_empty() || in_table {
                    table_rows.push(current_row.drain(..).collect());
                }
            }

            Event::Start(Tag::TableCell) => {
                in_table_cell = true;
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_table_cell = false;
                current_row.push(current_cell.trim().to_string());
                current_cell.clear();
            }

            // ── Headings ─────────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                style = match level {
                    HeadingLevel::H1 => Style::default()
                        .fg(HEADING1)
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED),
                    HeadingLevel::H2 => Style::default()
                        .fg(HEADING2)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(HEADING3)
                        .add_modifier(Modifier::BOLD),
                };
            }
            Event::End(TagEnd::Heading(_)) => {
                if !current.is_empty() {
                    let raw: String = current.iter().map(|s| s.content.as_ref()).collect();
                    lines.extend(wrap_line(&raw, style, wrap_width));
                    current.clear();
                }
                lines.push(Line::raw(""));
                style = Style::default().fg(FG);
            }

            // ── Inline styles ─────────────────────────────────────────────────
            Event::Start(Tag::Strong) => {
                style = style.add_modifier(Modifier::BOLD);
            }
            Event::End(TagEnd::Strong) => {
                style = style.remove_modifier(Modifier::BOLD);
            }
            Event::Start(Tag::Emphasis) => {
                style = style.add_modifier(Modifier::ITALIC);
            }
            Event::End(TagEnd::Emphasis) => {
                style = style.remove_modifier(Modifier::ITALIC);
            }
            Event::Start(Tag::Strikethrough) => {
                style = style.add_modifier(Modifier::CROSSED_OUT);
            }
            Event::End(TagEnd::Strikethrough) => {
                style = style.remove_modifier(Modifier::CROSSED_OUT);
            }

            // ── Code blocks ───────────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                lines.push(Line::from(vec![Span::styled(
                    " ┌─ code ".to_string(),
                    Style::default().fg(RULE_COLOR),
                )]));
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                lines.push(Line::from(vec![Span::styled(
                    " └────────".to_string(),
                    Style::default().fg(RULE_COLOR),
                )]));
                lines.push(Line::raw(""));
            }

            // ── Lists ─────────────────────────────────────────────────────────
            Event::Start(Tag::List(start)) => {
                list_depth += 1;
                ordered_counters.push(start.unwrap_or(0));
                indent_prefix = "  ".repeat(list_depth - 1);
            }
            Event::End(TagEnd::List(_)) => {
                if list_depth > 0 {
                    list_depth -= 1;
                }
                ordered_counters.pop();
                if list_depth == 0 {
                    lines.push(Line::raw(""));
                }
                indent_prefix = "  ".repeat(list_depth.saturating_sub(1));
            }
            Event::Start(Tag::Item) => {
                let bullet = if let Some(n) = ordered_counters.last_mut() {
                    if *n == 0 {
                        format!("{}• ", indent_prefix)
                    } else {
                        let s = format!("{}{}. ", indent_prefix, n);
                        *n += 1;
                        s
                    }
                } else {
                    format!("{}• ", indent_prefix)
                };
                current.push(Span::styled(bullet, Style::default().fg(LIST_BULLET)));
            }
            Event::End(TagEnd::Item) => {
                if !current.is_empty() {
                    let raw: String = current.iter().map(|s| s.content.as_ref()).collect();
                    lines.extend(wrap_line(&raw, style, wrap_width));
                    current.clear();
                }
            }

            // ── Blockquote ────────────────────────────────────────────────────
            Event::Start(Tag::BlockQuote(_)) => {
                style = Style::default()
                    .fg(BLOCK_QUOTE)
                    .add_modifier(Modifier::ITALIC);
                current.push(Span::styled(
                    "▌ ".to_string(),
                    Style::default().fg(LIST_BULLET),
                ));
            }
            Event::End(TagEnd::BlockQuote) => {
                if !current.is_empty() {
                    let raw: String = current.iter().map(|s| s.content.as_ref()).collect();
                    lines.extend(wrap_line(&raw, style, wrap_width));
                    current.clear();
                }
                lines.push(Line::raw(""));
                style = Style::default().fg(FG);
            }

            // ── Links ─────────────────────────────────────────────────────────
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                let label = if !title.is_empty() {
                    title.to_string()
                } else {
                    dest_url.to_string()
                };
                current.push(Span::styled(
                    format!("[{}]", label),
                    Style::default()
                        .fg(LINK_COLOR)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            }
            Event::End(TagEnd::Link) => {}

            // ── Paragraphs / breaks ───────────────────────────────────────────
            Event::End(TagEnd::Paragraph) => {
                if !current.is_empty() {
                    let raw: String = current.iter().map(|s| s.content.as_ref()).collect();
                    lines.extend(wrap_line(&raw, style, wrap_width));
                    current.clear();
                }
                lines.push(Line::raw(""));
            }
            Event::SoftBreak => {
                if !current.is_empty() {
                    let raw: String = current.iter().map(|s| s.content.as_ref()).collect();
                    lines.extend(wrap_line(&raw, style, wrap_width));
                    current.clear();
                }
            }
            Event::HardBreak => {
                if !current.is_empty() {
                    let raw: String = current.iter().map(|s| s.content.as_ref()).collect();
                    lines.extend(wrap_line(&raw, style, wrap_width));
                    current.clear();
                }
            }

            // ── Horizontal rule ───────────────────────────────────────────────
            Event::Rule => {
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(wrap_width.min(60)),
                    Style::default().fg(RULE_COLOR),
                )]));
                lines.push(Line::raw(""));
            }

            _ => {}
        }
    }

    if !current.is_empty() {
        let raw: String = current.iter().map(|s| s.content.as_ref()).collect();
        lines.extend(wrap_line(&raw, style, wrap_width));
    }

    Text::from(lines)
}

/// Word-wrap a styled line into multiple ratatui Lines.
/// Preserves the given style across all wrapped lines.
fn wrap_line(text: &str, style: Style, wrap_width: usize) -> Vec<Line<'static>> {
    if text.is_empty() {
        return vec![Line::raw("")];
    }

    let mut result = vec![];
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let w = word.chars().count();
        if current_width > 0 && current_width + 1 + w > wrap_width {
            result.push(Line::from(Span::styled(current_line.clone(), style)));
            current_line.clear();
            current_width = 0;
        }
        if current_width > 0 {
            current_line.push(' ');
            current_width += 1;
        }
        current_line.push_str(word);
        current_width += w;
    }
    if !current_line.is_empty() {
        result.push(Line::from(Span::styled(current_line, style)));
    }
    if result.is_empty() {
        result.push(Line::raw(""));
    }
    result
}

/// Truncate or wrap a single code line to fit within wrap_width.
/// Code can't be word-wrapped without breaking it, so we hard-trim.
fn wrap_code_line(line: &str, wrap_width: usize) -> String {
    let max = wrap_width.saturating_sub(2); // reserve for "  " prefix
    truncate_str(line, max)
}

/// Truncate a string to max_width chars, adding … if truncated.
fn truncate_str(s: &str, max_width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_width {
        s.to_string()
    } else if max_width == 0 {
        String::new()
    } else {
        let mut result: String = chars[..max_width.saturating_sub(1)].iter().collect();
        result.push('…');
        result
    }
}

// ── Table renderer ────────────────────────────────────────────────────────────

fn render_table(rows: &[Vec<String>], head_rows: usize, wrap_width: usize) -> Vec<Line<'static>> {
    if rows.is_empty() {
        return vec![];
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return vec![];
    }

    // Compute per-column max widths, capped so total table fits
    let mut col_widths = vec![1usize; col_count];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                col_widths[i] = col_widths[i].max(cell.chars().count());
            }
        }
    }
    // Total table width = 1 (left │) + sum(w+2) + (col_count-1) separators + 1 (right │)
    // = 3*col_count + sum(w) + (col_count-1) = 4*col_count - 1 + sum(w)
    // Cap each column proportionally if too wide
    let total_w: usize = col_widths.iter().sum();
    let overhead = 4 * col_count + 1;
    if total_w + overhead > wrap_width {
        let budget = wrap_width.saturating_sub(overhead);
        for w in &mut col_widths {
            *w = (*w * budget / total_w).max(3);
        }
    }
    // Truncate cell contents to fit column widths
    let mut capped_rows: Vec<Vec<String>> = vec![];
    for row in rows {
        let mut capped_row = vec![];
        for (i, cell) in row.iter().enumerate() {
            let max_w = col_widths.get(i).copied().unwrap_or(3);
            let truncated = truncate_str(cell, max_w);
            capped_row.push(truncated);
        }
        capped_rows.push(capped_row);
    }
    let rows = &capped_rows;

    // Helper: build a horizontal border line given left/mid/right/fill chars
    let h_line = |l: &str, m: &str, r: &str| -> String {
        let mut s = l.to_string();
        for (i, &w) in col_widths.iter().enumerate() {
            s.push_str(&"─".repeat(w + 2));
            if i + 1 < col_count {
                s.push_str(m);
            }
        }
        s.push_str(r);
        s
    };

    let mut out: Vec<Line<'static>> = vec![];

    // Top border ┌───┬───┐
    out.push(Line::from(Span::styled(
        h_line("┌", "┬", "┐"),
        Style::default().fg(TABLE_BORDER),
    )));

    for (row_idx, row) in rows.iter().enumerate() {
        // Data row: │ cell │ cell │
        let mut spans: Vec<Span<'static>> = vec![];
        spans.push(Span::styled("│".to_string(), Style::default().fg(TABLE_BORDER)));
        for col_idx in 0..col_count {
            let cell = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            let padded = format!(" {:<width$} ", cell, width = col_widths[col_idx]);
            let cell_style = if row_idx < head_rows {
                Style::default()
                    .fg(TABLE_HEADER)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TABLE_FG)
            };
            spans.push(Span::styled(padded, cell_style));
            spans.push(Span::styled("│".to_string(), Style::default().fg(TABLE_BORDER)));
        }
        out.push(Line::from(spans));

        // Header separator ├───┼───┤ after last header row
        if row_idx + 1 == head_rows && row_idx + 1 < rows.len() {
            out.push(Line::from(Span::styled(
                h_line("├", "┼", "┤"),
                Style::default().fg(TABLE_BORDER),
            )));
        }
    }

    // Bottom border └───┴───┘
    out.push(Line::from(Span::styled(
        h_line("└", "┴", "┘"),
        Style::default().fg(TABLE_BORDER),
    )));

    out
}
