use std::fs;

#[derive(Clone, PartialEq)]
pub enum FileKind {
    Dir,
    Image,
    File,
}

pub fn file_kind(name: &str) -> FileKind {
    if name.ends_with('/') {
        return FileKind::Dir;
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" | "tiff" | "avif" => {
            FileKind::Image
        }
        _ => FileKind::File,
    }
}

pub struct FilePicker {
    pub files: Vec<String>,
    pub filtered: Vec<String>,
    pub selected: usize,
    pub query: String,
    /// If the picker was opened by typing `@`, this holds the index of `@` in app.input.
    /// None means it was opened via Ctrl+F.
    pub at_pos: Option<usize>,
}

impl FilePicker {
    pub fn new(cwd: &str) -> Self {
        let files = scan_files(cwd);
        let filtered = files.clone();
        Self {
            files,
            filtered,
            selected: 0,
            query: String::new(),
            at_pos: None,
        }
    }

    pub fn update_filter(&mut self, query: &str) {
        self.query = query.to_string();
        self.filtered = self
            .files
            .iter()
            .filter(|f| f.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn current(&self) -> Option<&String> {
        self.filtered.get(self.selected)
    }
}

pub fn scan_files(cwd: &str) -> Vec<String> {
    let mut files = Vec::new();
    scan_dir(cwd, cwd, &mut files);
    files.sort();
    files
}

fn scan_dir(base: &str, current: &str, acc: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with('.') || fname == "target" || fname == "node_modules" {
            continue;
        }
        let name = path.to_string_lossy().to_string();
        let relative = name
            .strip_prefix(base)
            .unwrap_or(&name)
            .trim_start_matches('/')
            .to_string();

        if path.is_dir() {
            acc.push(format!("{}/", relative));
            scan_dir(base, &name, acc);
        } else {
            acc.push(relative);
        }
    }
}
