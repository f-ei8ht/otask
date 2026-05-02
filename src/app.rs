use crate::editor::{EditorAction, EditorMode, EditorState};
use crate::filetree::FileTree;
use crate::providers::{
    anthropic::AnthropicProvider, cerebras::CerebrasProvider, codex::CodexProvider, Message,
    Provider, ProviderType,
};
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::{backend::Backend, Terminal};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Plan,
    Edit,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Plan => write!(f, "plan"),
            Mode::Edit => write!(f, "edit"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Typing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    CommandPalette,
    FileOpen,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub enum AppEvent {
    Response(String),
    Error(String),
}

pub struct App {
    pub mode: Mode,
    pub input_mode: InputMode,
    pub overlay: Overlay,
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub provider: Option<Arc<dyn Provider>>,
    pub provider_name: Option<String>,
    pub status: String,
    pub status_flash: Option<Instant>,
    pub scroll: usize,
    pub is_loading: bool,
    pub should_quit: bool,
    pub cursor_pos: usize,
    pub save_count: usize,
    pub focused_msg: Option<usize>,
    pub palette_scroll: usize,
    pub editor: Option<EditorState>,
    pub file_open_buf: String,
    pub tree: Option<FileTree>,
    pub tree_focused: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Edit,
            input_mode: InputMode::Normal,
            overlay: Overlay::None,
            input: String::new(),
            messages: vec![],
            provider: None,
            provider_name: None,
            status: String::new(),
            status_flash: None,
            scroll: usize::MAX,
            is_loading: false,
            should_quit: false,
            cursor_pos: 0,
            save_count: 0,
            focused_msg: None,
            palette_scroll: 0,
            editor: None,
            file_open_buf: String::new(),
            tree: None,
            tree_focused: false,
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()>
    where
        <B as Backend>::Error: Send + Sync + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<AppEvent>(32);

        loop {
            self.tick_flash();
            terminal.draw(|f| crate::ui::draw(f, self))?;

            while let Ok(evt) = rx.try_recv() {
                self.is_loading = false;
                match evt {
                    AppEvent::Response(content) => {
                        self.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content,
                        });
                        self.scroll_to_bottom();
                    }
                    AppEvent::Error(err) => {
                        self.messages.push(ChatMessage {
                            role: "error".to_string(),
                            content: format!("Error: {}", err),
                        });
                        self.scroll_to_bottom();
                    }
                }
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Mouse(mouse) => {
                        if self.editor.is_none() {
                            match mouse.kind {
                                MouseEventKind::ScrollDown => {
                                    if self.overlay == Overlay::CommandPalette {
                                        self.palette_scroll = self.palette_scroll.saturating_add(1);
                                    } else {
                                        self.scroll_down(3);
                                    }
                                }
                                MouseEventKind::ScrollUp => {
                                    if self.overlay == Overlay::CommandPalette {
                                        self.palette_scroll = self.palette_scroll.saturating_sub(1);
                                    } else {
                                        self.scroll_up(3);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Event::Key(key) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            break;
                        }

                        // ── Editor / tree active ─────────────────────────────
                        if self.editor.is_some() {
                            // Ctrl+E toggles tree
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && key.code == KeyCode::Char('e')
                            {
                                if self.tree.is_some() {
                                    self.tree = None;
                                    self.tree_focused = false;
                                } else {
                                    self.tree = Some(FileTree::new(std::path::Path::new(".")));
                                    self.tree_focused = true;
                                }
                                continue;
                            }

                            // Tab switches focus between tree and editor
                            if key.code == KeyCode::Tab {
                                if self.tree.is_some() {
                                    self.tree_focused = !self.tree_focused;
                                }
                                continue;
                            }

                            // Tree focus: tree navigation keys
                            if self.tree_focused {
                                if let Some(ref mut tree) = self.tree {
                                    match key.code {
                                        KeyCode::Char('j') | KeyCode::Down => tree.key_down(),
                                        KeyCode::Char('k') | KeyCode::Up => tree.key_up(),
                                        KeyCode::Char('l') | KeyCode::Right => {
                                            if tree.selected_is_file() {
                                                let path_s = tree
                                                    .selected_path()
                                                    .map(|p| p.to_string_lossy().to_string());
                                                if let Some(p) = path_s {
                                                    self.editor = Some(EditorState::open(Some(p)));
                                                    self.tree_focused = false;
                                                }
                                            } else {
                                                tree.toggle_expand();
                                            }
                                        }
                                        KeyCode::Char('h') | KeyCode::Left => {
                                            tree.collapse_or_parent();
                                        }
                                        KeyCode::Enter => {
                                            if tree.selected_is_file() {
                                                let path_s = tree
                                                    .selected_path()
                                                    .map(|p| p.to_string_lossy().to_string());
                                                if let Some(p) = path_s {
                                                    self.editor = Some(EditorState::open(Some(p)));
                                                    self.tree_focused = false;
                                                }
                                            } else {
                                                tree.toggle_expand();
                                            }
                                        }
                                        KeyCode::Char('r') => tree.refresh(),
                                        KeyCode::Char('q') | KeyCode::Esc => {
                                            self.tree_focused = false;
                                        }
                                        _ => {}
                                    }
                                }
                                continue;
                            }

                            // Editor focus: vim key handling
                            let action = self.handle_editor_key(key);
                            if action == EditorAction::Quit {
                                self.editor = None;
                                self.tree = None;
                                self.tree_focused = false;
                            }
                            continue;
                        }

                        // ── File-open prompt ─────────────────────────────────
                        if self.overlay == Overlay::FileOpen {
                            match key.code {
                                KeyCode::Esc => {
                                    self.overlay = Overlay::None;
                                    self.file_open_buf.clear();
                                }
                                KeyCode::Enter => {
                                    let path = self.file_open_buf.trim().to_string();
                                    self.file_open_buf.clear();
                                    self.overlay = Overlay::None;
                                    if !path.is_empty() {
                                        let fp = if path == "new" { None } else { Some(path) };
                                        self.editor = Some(EditorState::open(fp));
                                        self.tree = Some(FileTree::new(std::path::Path::new(".")));
                                        self.tree_focused = false;
                                    }
                                }
                                KeyCode::Backspace => {
                                    self.file_open_buf.pop();
                                }
                                KeyCode::Char(c) => {
                                    self.file_open_buf.push(c);
                                }
                                _ => {}
                            }
                            continue;
                        }

                        // ── Global shortcuts ─────────────────────────────────
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('k')
                        {
                            self.overlay = if self.overlay == Overlay::CommandPalette {
                                Overlay::None
                            } else {
                                self.palette_scroll = 0;
                                Overlay::CommandPalette
                            };
                            continue;
                        }

                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('n')
                        {
                            self.new_session();
                            continue;
                        }

                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('v')
                        {
                            self.overlay = Overlay::FileOpen;
                            self.file_open_buf.clear();
                            continue;
                        }

                        // ── Command palette keys ─────────────────────────────
                        if self.overlay == Overlay::CommandPalette {
                            match key.code {
                                KeyCode::Esc => self.overlay = Overlay::None,
                                KeyCode::Down | KeyCode::Char('j') => {
                                    self.palette_scroll = self.palette_scroll.saturating_add(1);
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    self.palette_scroll = self.palette_scroll.saturating_sub(1);
                                }
                                _ => {}
                            }
                            continue;
                        }

                        // ── Normal AI agent keys ─────────────────────────────
                        match self.input_mode {
                            InputMode::Normal => match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('p') => {
                                    self.mode = Mode::Plan;
                                    self.flash_status("plan mode".to_string());
                                }
                                KeyCode::Char('e') => {
                                    self.mode = Mode::Edit;
                                    self.flash_status("edit mode".to_string());
                                }
                                KeyCode::Char('i') => {
                                    self.input_mode = InputMode::Typing;
                                    self.cursor_pos = self.input.len();
                                }
                                KeyCode::Char('/') => {
                                    self.input_mode = InputMode::Typing;
                                    self.input = "/".to_string();
                                    self.cursor_pos = 1;
                                }
                                KeyCode::Char('s') => self.save_last_response(),
                                KeyCode::Char('y') => self.yank_response(),
                                KeyCode::Char('j') | KeyCode::Down => self.scroll_down(3),
                                KeyCode::Char('k') | KeyCode::Up => self.scroll_up(3),
                                _ => {}
                            },
                            InputMode::Typing => match key.code {
                                KeyCode::Esc => {
                                    self.input_mode = InputMode::Normal;
                                }
                                KeyCode::Enter => {
                                    let input = self.input.trim().to_string();
                                    if !input.is_empty() {
                                        self.handle_input(input, tx.clone()).await;
                                    }
                                    self.input.clear();
                                    self.cursor_pos = 0;
                                    self.input_mode = InputMode::Normal;
                                }
                                KeyCode::Backspace => {
                                    if self.cursor_pos > 0 {
                                        self.cursor_pos -= 1;
                                        self.input.remove(self.cursor_pos);
                                    }
                                }
                                KeyCode::Delete => {
                                    if self.cursor_pos < self.input.len() {
                                        self.input.remove(self.cursor_pos);
                                    }
                                }
                                KeyCode::Left => {
                                    if self.cursor_pos > 0 {
                                        self.cursor_pos -= 1;
                                    }
                                }
                                KeyCode::Right => {
                                    if self.cursor_pos < self.input.len() {
                                        self.cursor_pos += 1;
                                    }
                                }
                                KeyCode::Home => self.cursor_pos = 0,
                                KeyCode::End => self.cursor_pos = self.input.len(),
                                KeyCode::Char(c) => {
                                    self.input.insert(self.cursor_pos, c);
                                    self.cursor_pos += 1;
                                }
                                _ => {}
                            },
                        }
                    }
                    _ => {}
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn handle_editor_key(&mut self, key: crossterm::event::KeyEvent) -> EditorAction {
        let ed = match self.editor.as_mut() {
            Some(e) => e,
            None => return EditorAction::None,
        };

        match ed.mode {
            EditorMode::Normal => {
                if ed.pending_d {
                    ed.pending_d = false;
                    if key.code == KeyCode::Char('d') {
                        ed.delete_line();
                    }
                    return EditorAction::None;
                }

                match key.code {
                    KeyCode::Char('h') | KeyCode::Left => ed.move_left(),
                    KeyCode::Char('j') | KeyCode::Down => ed.move_down(),
                    KeyCode::Char('k') | KeyCode::Up => ed.move_up(),
                    KeyCode::Char('l') | KeyCode::Right => ed.move_right(),
                    KeyCode::Char('0') => ed.goto_line_start(),
                    KeyCode::Char('$') => ed.goto_line_end(),
                    KeyCode::Char('g') => ed.goto_first_line(),
                    KeyCode::Char('G') => ed.goto_last_line(),
                    KeyCode::Char('i') => {
                        ed.mode = EditorMode::Insert;
                        ed.status_msg = None;
                    }
                    KeyCode::Char('a') => {
                        ed.move_right();
                        ed.mode = EditorMode::Insert;
                        ed.status_msg = None;
                    }
                    KeyCode::Char('o') => ed.open_line_below(),
                    KeyCode::Char('O') => ed.open_line_above(),
                    KeyCode::Char('d') => ed.pending_d = true,
                    KeyCode::Char('x') => ed.delete_char(),
                    KeyCode::Char(':') => {
                        ed.mode = EditorMode::Command;
                        ed.command_buf.clear();
                        ed.status_msg = None;
                    }
                    _ => {}
                }
                EditorAction::None
            }

            EditorMode::Insert => {
                match key.code {
                    KeyCode::Esc => {
                        ed.mode = EditorMode::Normal;
                        if ed.cursor_col > 0 {
                            ed.cursor_col -= 1;
                        }
                    }
                    KeyCode::Enter => ed.enter(),
                    KeyCode::Backspace => ed.backspace(),
                    KeyCode::Delete => ed.delete_char(),
                    KeyCode::Left => ed.move_left(),
                    KeyCode::Right => ed.move_right(),
                    KeyCode::Up => ed.move_up(),
                    KeyCode::Down => ed.move_down(),
                    KeyCode::Home => ed.goto_line_start(),
                    KeyCode::End => ed.goto_line_end(),
                    KeyCode::Char(c) => ed.insert_char(c),
                    _ => {}
                }
                EditorAction::None
            }

            EditorMode::Command => {
                match key.code {
                    KeyCode::Esc => {
                        ed.mode = EditorMode::Normal;
                        ed.command_buf.clear();
                    }
                    KeyCode::Enter => return ed.execute_command(),
                    KeyCode::Backspace => {
                        if ed.command_buf.is_empty() {
                            ed.mode = EditorMode::Normal;
                        } else {
                            ed.command_buf.pop();
                        }
                    }
                    KeyCode::Char(c) => ed.command_buf.push(c),
                    _ => {}
                }
                EditorAction::None
            }
        }
    }

    fn scroll_down(&mut self, amount: usize) {
        if self.scroll != usize::MAX {
            self.scroll = self.scroll.saturating_add(amount);
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        if self.scroll == usize::MAX {
            self.scroll = 0;
        } else {
            self.scroll = self.scroll.saturating_sub(amount);
        }
    }

    fn save_last_response(&mut self) {
        let last = self.messages.iter().rev().find(|m| m.role == "assistant");
        match last {
            None => self.flash_status("no response to save yet".to_string()),
            Some(msg) => {
                self.save_count += 1;
                let filename = format!("response_{}.md", self.save_count);
                let content = msg.content.clone();
                match fs::write(&filename, &content) {
                    Ok(_) => self.flash_status(format!("saved → {}", filename)),
                    Err(e) => self.flash_status(format!("save failed: {}", e)),
                }
            }
        }
    }

    fn yank_response(&mut self) {
        let idx = self.focused_msg.or_else(|| {
            self.messages
                .iter()
                .enumerate()
                .rev()
                .find(|(_, m)| m.role == "assistant")
                .map(|(i, _)| i)
        });
        let content = match idx {
            None => {
                self.flash_status("no response to copy yet".to_string());
                return;
            }
            Some(i) => self.messages[i].content.clone(),
        };
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(content.clone())) {
            Ok(_) => self.flash_status("copied to clipboard".to_string()),
            Err(_) => {
                self.save_count += 1;
                let filename = format!("response_{}.md", self.save_count);
                match fs::write(&filename, &content) {
                    Ok(_) => self.flash_status(format!("clipboard unavailable → saved to {}", filename)),
                    Err(e) => self.flash_status(format!("copy failed: {}", e)),
                }
            }
        }
    }

    pub fn flash_status(&mut self, msg: String) {
        self.status = msg;
        self.status_flash = Some(Instant::now());
    }

    pub fn tick_flash(&mut self) {
        if let Some(t) = self.status_flash {
            if t.elapsed() > Duration::from_secs(3) {
                self.status_flash = None;
                self.status = String::new();
            }
        }
    }

    async fn handle_input(&mut self, input: String, tx: mpsc::Sender<AppEvent>) {
        if input.starts_with('/') {
            self.handle_command(input);
        } else {
            self.send_message(input, tx).await;
        }
    }

    fn handle_command(&mut self, input: String) {
        let parts: Vec<&str> = input.splitn(4, ' ').collect();
        match parts.as_slice() {
            ["/connect", provider, api_key] | ["/connect", provider, api_key, _] => {
                let api_key = api_key.to_string();
                let model_override: Option<String> = parts.get(3).map(|s| s.to_string());
                match provider.to_lowercase().as_str() {
                    "cerebras" => {
                        let model = model_override.unwrap_or_else(|| "gpt-oss-120b".to_string());
                        self.provider = Some(Arc::new(CerebrasProvider::new(api_key, Some(model.clone()))));
                        self.provider_name = Some(format!("cerebras · {}", model));
                        self.flash_status(format!("connected to cerebras ({})", model));
                    }
                    "anthropic" => {
                        self.provider = Some(Arc::new(AnthropicProvider::new(api_key)));
                        self.provider_name = Some(ProviderType::Anthropic.to_string());
                        self.flash_status("connected to anthropic (claude-opus-4-5)".to_string());
                    }
                    "codex" => {
                        self.provider = Some(Arc::new(CodexProvider::new(api_key)));
                        self.provider_name = Some(ProviderType::Codex.to_string());
                        self.flash_status("connected to codex (gpt-4o)".to_string());
                    }
                    other => self.flash_status(format!("unknown provider: '{}'", other)),
                }
                self.scroll_to_bottom();
            }
            ["/new"] => self.new_session(),
            ["/edit"] => {
                self.overlay = Overlay::FileOpen;
                self.file_open_buf.clear();
            }
            ["/edit", path] => {
                let path = path.to_string();
                self.editor = Some(EditorState::open(Some(path)));
                self.tree = Some(FileTree::new(std::path::Path::new(".")));
                self.tree_focused = false;
            }
            ["/help"] => self.flash_status("press ctrl+k to see all commands".to_string()),
            _ => self.flash_status(format!("unknown command: '{}'", input)),
        }
    }

    async fn send_message(&mut self, content: String, tx: mpsc::Sender<AppEvent>) {
        if self.provider.is_none() {
            self.flash_status("no provider — use /connect <provider> <key>".to_string());
            return;
        }
        let mode_ctx = match self.mode {
            Mode::Plan => "You are an AI coding assistant in PLAN mode. Help the user think through architecture, design decisions, and planning. Be structured and thorough.",
            Mode::Edit => "You are an AI coding assistant in EDIT mode. Help the user write, modify, and debug code. Be precise and always provide complete, working code.",
        };
        self.messages.push(ChatMessage { role: "user".to_string(), content: content.clone() });
        self.scroll_to_bottom();
        self.is_loading = true;

        let history: Vec<Message> = self
            .messages
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .map(|m| Message { role: m.role.clone(), content: m.content.clone() })
            .collect();
        let mut messages = vec![Message { role: "system".to_string(), content: mode_ctx.to_string() }];
        messages.extend(history);

        let provider = Arc::clone(self.provider.as_ref().unwrap());
        tokio::spawn(async move {
            match provider.chat(messages).await {
                Ok(response) => { let _ = tx.send(AppEvent::Response(response)).await; }
                Err(err) => { let _ = tx.send(AppEvent::Error(err.to_string())).await; }
            }
        });
    }

    fn new_session(&mut self) {
        self.messages.clear();
        self.input.clear();
        self.cursor_pos = 0;
        self.scroll = usize::MAX;
        self.focused_msg = None;
        self.is_loading = false;
        self.input_mode = InputMode::Normal;
        self.overlay = Overlay::None;
        self.flash_status("new session started".to_string());
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll = usize::MAX;
    }
}
