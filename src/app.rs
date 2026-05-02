use crate::providers::{
    anthropic::AnthropicProvider, cerebras::CerebrasProvider, codex::CodexProvider, Message,
    Provider, ProviderType,
};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Plan,
    Edit,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Plan => write!(f, "PLAN"),
            Mode::Edit => write!(f, "EDIT"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Typing,
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
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub provider: Option<Arc<dyn Provider>>,
    pub provider_name: Option<String>,
    pub status: String,
    pub scroll: usize,
    pub is_loading: bool,
    pub should_quit: bool,
    pub cursor_pos: usize,
}

impl App {
    pub fn new() -> Self {
        let messages = vec![ChatMessage {
            role: "system".to_string(),
            content: "Welcome! Press [i] to start typing. Use [p] for Plan mode, [b] for Edit mode.\nType /connect <provider> <api-key> to connect a provider.\nAvailable providers: cerebras, anthropic, codex\n\nExample: /connect cerebras your-api-key-here\n\nType /help for all commands.".to_string(),
        }];

        Self {
            mode: Mode::Plan,
            input_mode: InputMode::Normal,
            input: String::new(),
            messages,
            provider: None,
            provider_name: None,
            status: "No provider connected. Use /connect to get started.".to_string(),
            scroll: 0,
            is_loading: false,
            should_quit: false,
            cursor_pos: 0,
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()>
    where
        <B as Backend>::Error: Send + Sync + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<AppEvent>(32);

        loop {
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

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break;
                    }

                    match self.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('p') => {
                                self.mode = Mode::Plan;
                                self.status = "Switched to Plan mode.".to_string();
                            }
                            KeyCode::Char('b') => {
                                self.mode = Mode::Edit;
                                self.status = "Switched to Edit mode.".to_string();
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
                            KeyCode::Up => {
                                if self.scroll > 0 {
                                    self.scroll = self.scroll.saturating_sub(1);
                                }
                            }
                            KeyCode::Down => {
                                self.scroll = self.scroll.saturating_add(1);
                            }
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
                            KeyCode::Home => {
                                self.cursor_pos = 0;
                            }
                            KeyCode::End => {
                                self.cursor_pos = self.input.len();
                            }
                            KeyCode::Char(c) => {
                                self.input.insert(self.cursor_pos, c);
                                self.cursor_pos += 1;
                            }
                            _ => {}
                        },
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
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
                        let model = model_override
                            .unwrap_or_else(|| "gpt-oss-120b".to_string());
                        self.provider = Some(Arc::new(CerebrasProvider::new(
                            api_key,
                            Some(model.clone()),
                        )));
                        self.provider_name =
                            Some(format!("Cerebras ({})", model));
                        self.status = format!("Connected to Cerebras — {}", model);
                        self.messages.push(ChatMessage {
                            role: "system".to_string(),
                            content: format!(
                                "Connected to Cerebras! Model: {}. You can now start chatting!",
                                model
                            ),
                        });
                    }
                    "anthropic" => {
                        self.provider = Some(Arc::new(AnthropicProvider::new(api_key)));
                        self.provider_name = Some(ProviderType::Anthropic.to_string());
                        self.status = "Connected to Anthropic (claude-opus-4-5)".to_string();
                        self.messages.push(ChatMessage {
                            role: "system".to_string(),
                            content: "Connected to Anthropic! Model: claude-opus-4-5. You can now start chatting!".to_string(),
                        });
                    }
                    "codex" => {
                        self.provider = Some(Arc::new(CodexProvider::new(api_key)));
                        self.provider_name = Some(ProviderType::Codex.to_string());
                        self.status = "Connected to Codex/OpenAI (gpt-4o)".to_string();
                        self.messages.push(ChatMessage {
                            role: "system".to_string(),
                            content: "Connected to Codex! Model: gpt-4o. You can now start chatting!".to_string(),
                        });
                    }
                    other => {
                        self.messages.push(ChatMessage {
                            role: "error".to_string(),
                            content: format!(
                                "Unknown provider: '{}'. Available: cerebras, anthropic, codex",
                                other
                            ),
                        });
                    }
                }
                self.scroll_to_bottom();
            }
            ["/help"] => {
                self.messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: "Commands:\n  /connect <provider> <api-key> [model]  Connect a provider\n  /help                                  Show this help\n\nProviders:\n  cerebras   gpt-oss-120b (default), llama-3.3-70b, llama3.1-8b\n  anthropic  claude-opus-4-5\n  codex      gpt-4o (OpenAI)\n\nExamples:\n  /connect cerebras <key>\n  /connect cerebras <key> gpt-oss-120b\n  /connect cerebras <key> llama-3.3-70b\n\nKeybindings:\n  [p]    Switch to Plan mode\n  [b]    Switch to Edit/Build mode\n  [i]    Start typing a message\n  [/]    Start a slash command\n  [↑↓]   Scroll messages\n  [Esc]  Stop typing\n  [q]    Quit".to_string(),
                });
                self.scroll_to_bottom();
            }
            _ => {
                self.messages.push(ChatMessage {
                    role: "error".to_string(),
                    content: format!(
                        "Unknown command: '{}'. Type /help for available commands.",
                        input
                    ),
                });
                self.scroll_to_bottom();
            }
        }
    }

    async fn send_message(&mut self, content: String, tx: mpsc::Sender<AppEvent>) {
        if self.provider.is_none() {
            self.messages.push(ChatMessage {
                role: "error".to_string(),
                content: "No provider connected. Use /connect <provider> <api-key> first."
                    .to_string(),
            });
            self.scroll_to_bottom();
            return;
        }

        let mode_ctx = match self.mode {
            Mode::Plan => "You are an AI coding assistant in PLAN mode. Help the user think through architecture, design decisions, and planning. Be structured and thorough. Use numbered lists and clear sections.",
            Mode::Edit => "You are an AI coding assistant in EDIT mode. Help the user write, modify, and debug code. Be precise and always provide complete, working code.",
        };

        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: content.clone(),
        });
        self.scroll_to_bottom();
        self.is_loading = true;

        let history: Vec<Message> = self
            .messages
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .map(|m| Message {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let mut messages = vec![Message {
            role: "system".to_string(),
            content: mode_ctx.to_string(),
        }];
        messages.extend(history);

        let provider = Arc::clone(self.provider.as_ref().unwrap());
        tokio::spawn(async move {
            let fut = provider.chat(messages);
            match fut.await {
                Ok(response) => {
                    let _ = tx.send(AppEvent::Response(response)).await;
                }
                Err(err) => {
                    let _ = tx.send(AppEvent::Error(err.to_string())).await;
                }
            }
        });
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll = usize::MAX;
    }
}
