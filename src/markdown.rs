use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
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

pub fn md_to_text(md: &str) -> Text<'static> {
    let parser = Parser::new(md);

    let mut lines: Vec<Line<'static>> = vec![];
    let mut current: Vec<Span<'static>> = vec![];
    let mut style = Style::default().fg(FG);
    let mut list_depth: usize = 0;
    let mut ordered_counters: Vec<u64> = vec![];
    let mut in_code_block = false;
    let mut indent_prefix = String::new();

    for event in parser {
        match event {
            Event::Text(t) => {
                let text = t.into_string();
                if in_code_block {
                    for line in text.lines() {
                        let prefix = format!("  {}", line);
                        lines.push(Line::from(vec![Span::styled(
                            prefix,
                            Style::default().fg(CODE_FG).bg(CODE_BG),
                        )]));
                    }
                } else {
                    current.push(Span::styled(text, style));
                }
            }

            Event::Code(t) => {
                current.push(Span::styled(
                    format!("`{}`", t.into_string()),
                    Style::default().fg(CODE_FG).bg(CODE_BG),
                ));
            }

            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = Some(level);
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
                let prefix = match level {
                    HeadingLevel::H1 => "# ",
                    HeadingLevel::H2 => "## ",
                    HeadingLevel::H3 => "### ",
                    _ => "#### ",
                };
                current.push(Span::styled(prefix.to_string(), style));
            }

            Event::End(TagEnd::Heading(_)) => {
                if !current.is_empty() {
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                }
                lines.push(Line::raw(""));
                heading_level = None;
                style = Style::default().fg(FG);
            }

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

            Event::Start(Tag::List(start)) => {
                list_depth += 1;
                if let Some(n) = start {
                    ordered_counters.push(n);
                } else {
                    ordered_counters.push(0);
                }
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
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                }
            }

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
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                }
                lines.push(Line::raw(""));
                style = Style::default().fg(FG);
            }

            Event::Start(Tag::Link { dest_url, title, .. }) => {
                let label = if title.is_empty() {
                    dest_url.to_string()
                } else {
                    title.to_string()
                };
                current.push(Span::styled(
                    format!("[{}]", label),
                    Style::default()
                        .fg(LINK_COLOR)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            }
            Event::End(TagEnd::Link) => {}

            Event::End(TagEnd::Paragraph) => {
                if !current.is_empty() {
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                }
                lines.push(Line::raw(""));
            }

            Event::SoftBreak => {
                if !current.is_empty() {
                    lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
                }
            }
            Event::HardBreak => {
                lines.push(Line::from(current.drain(..).collect::<Vec<_>>()));
            }

            Event::Rule => {
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(60),
                    Style::default().fg(RULE_COLOR),
                )]));
                lines.push(Line::raw(""));
            }

            _ => {}
        }
    }

    if !current.is_empty() {
        lines.push(Line::from(current));
    }

    Text::from(lines)
}
