use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

pub struct Highlighter {
    ps: SyntaxSet,
    ts: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight_file(&self, content: &str, ext: &str) -> Vec<Line<'static>> {
        let syntax = self
            .ps
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());
        let theme = &self.ts.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let mut lines = Vec::new();
        for raw in content.lines() {
            let ranges = h.highlight_line(raw, &self.ps).unwrap_or_default();
            let spans: Vec<Span<'static>> = ranges
                .into_iter()
                .filter(|(_, text)| !text.is_empty())
                .map(|(style, text)| {
                    let c = style.foreground;
                    Span::styled(
                        text.to_string(),
                        Style::default().fg(Color::Rgb(c.r, c.g, c.b)),
                    )
                })
                .collect();
            lines.push(if spans.is_empty() {
                Line::from(String::new())
            } else {
                Line::from(spans)
            });
        }

        // always have at least one line
        if lines.is_empty() {
            lines.push(Line::from(String::new()));
        }
        lines
    }
}
