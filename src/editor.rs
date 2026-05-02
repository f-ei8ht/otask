use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub enum EditorMode {
    Normal,
    Insert,
    Command,
}

pub struct EditorState {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub file_path: Option<String>,
    pub dirty: bool,
    pub mode: EditorMode,
    pub command_buf: String,
    pub status_msg: Option<String>,
    pub pending_d: bool,
    pub highlight_cache: Vec<ratatui::text::Line<'static>>,
    pub cache_dirty: bool,
}

impl EditorState {
    pub fn open(file_path: Option<String>) -> Self {
        let lines = file_path
            .as_deref()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|s| {
                let mut v: Vec<String> = s.lines().map(String::from).collect();
                if v.is_empty() {
                    v.push(String::new());
                }
                v
            })
            .unwrap_or_else(|| vec![String::new()]);

        Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            file_path,
            dirty: false,
            mode: EditorMode::Normal,
            command_buf: String::new(),
            status_msg: None,
            pending_d: false,
            highlight_cache: Vec::new(),
            cache_dirty: true,
        }
    }

    pub fn save(&mut self) -> Result<(), String> {
        match &self.file_path {
            None => Err("no file name".to_string()),
            Some(path) => {
                let content = self.lines.join("\n");
                fs::write(path, content).map_err(|e| e.to_string())?;
                self.dirty = false;
                self.status_msg = Some(format!("\"{}\" written", path));
                Ok(())
            }
        }
    }

    fn clamp_col(&mut self) {
        let max = self.lines[self.cursor_row].len();
        if self.cursor_col > max {
            self.cursor_col = max;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_col();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_col();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let max = self.lines[self.cursor_row].len();
        if self.cursor_col < max {
            self.cursor_col += 1;
        }
    }

    pub fn goto_line_start(&mut self) {
        self.cursor_col = 0;
    }

    pub fn goto_line_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    pub fn goto_first_line(&mut self) {
        self.cursor_row = 0;
        self.clamp_col();
    }

    pub fn goto_last_line(&mut self) {
        self.cursor_row = self.lines.len().saturating_sub(1);
        self.clamp_col();
    }

    pub fn insert_char(&mut self, c: char) {
        let col = self.cursor_col;
        self.lines[self.cursor_row].insert(col, c);
        self.cursor_col += 1;
        self.dirty = true;
        self.cache_dirty = true;
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.lines[self.cursor_row].remove(self.cursor_col);
            self.dirty = true;
            self.cache_dirty = true;
        } else if self.cursor_row > 0 {
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current);
            self.dirty = true;
            self.cache_dirty = true;
        }
    }

    pub fn delete_char(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_row].remove(self.cursor_col);
            self.dirty = true;
            self.cache_dirty = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
            self.dirty = true;
            self.cache_dirty = true;
        }
    }

    pub fn enter(&mut self) {
        let rest = self.lines[self.cursor_row].split_off(self.cursor_col);
        self.lines.insert(self.cursor_row + 1, rest);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.dirty = true;
        self.cache_dirty = true;
    }

    pub fn delete_line(&mut self) {
        if self.lines.len() > 1 {
            self.lines.remove(self.cursor_row);
            if self.cursor_row >= self.lines.len() {
                self.cursor_row = self.lines.len().saturating_sub(1);
            }
        } else {
            self.lines[0].clear();
            self.cursor_col = 0;
        }
        self.clamp_col();
        self.dirty = true;
        self.cache_dirty = true;
    }

    pub fn open_line_below(&mut self) {
        self.lines.insert(self.cursor_row + 1, String::new());
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.mode = EditorMode::Insert;
        self.dirty = true;
        self.cache_dirty = true;
    }

    pub fn open_line_above(&mut self) {
        self.lines.insert(self.cursor_row, String::new());
        self.cursor_col = 0;
        self.mode = EditorMode::Insert;
        self.dirty = true;
        self.cache_dirty = true;
    }

    #[allow(dead_code)]
    pub fn scroll_to_cursor(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        } else if self.cursor_row >= self.scroll_row + visible_height {
            self.scroll_row = self.cursor_row + 1 - visible_height;
        }
    }

    pub fn execute_command(&mut self) -> EditorAction {
        let cmd = self.command_buf.trim().to_string();
        self.command_buf.clear();
        self.mode = EditorMode::Normal;

        match cmd.as_str() {
            "w" => match self.save() {
                Ok(_) => EditorAction::None,
                Err(e) => {
                    self.status_msg = Some(format!("E: {}", e));
                    EditorAction::None
                }
            },
            "wq" => match self.save() {
                Ok(_) => EditorAction::Quit,
                Err(e) => {
                    self.status_msg = Some(format!("E: {}", e));
                    EditorAction::None
                }
            },
            "q" => {
                if self.dirty {
                    self.status_msg = Some(
                        "unsaved changes — use :w to save, :q! to discard".to_string(),
                    );
                    EditorAction::None
                } else {
                    EditorAction::Quit
                }
            }
            "q!" => EditorAction::Quit,
            _ => {
                self.status_msg = Some(format!("unknown command: :{}", cmd));
                EditorAction::None
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EditorAction {
    None,
    Quit,
}
