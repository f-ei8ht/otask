use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

pub struct FileTree {
    pub root: PathBuf,
    pub expanded: HashSet<PathBuf>,
    pub visible: Vec<FileEntry>,
    pub selected: usize,
}

impl FileTree {
    pub fn new(root: &Path) -> Self {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        let visible = compute_visible(&root, &expanded);
        Self { root, expanded, visible, selected: 0 }
    }

    pub fn refresh(&mut self) {
        let sel_path = self.selected_path().map(|p| p.to_path_buf());
        self.visible = compute_visible(&self.root, &self.expanded);
        if let Some(prev) = sel_path {
            if let Some(pos) = self.visible.iter().position(|e| e.path == prev) {
                self.selected = pos;
                return;
            }
        }
        self.selected = self.selected.min(self.visible.len().saturating_sub(1));
    }

    pub fn key_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn key_down(&mut self) {
        if self.selected + 1 < self.visible.len() {
            self.selected += 1;
        }
    }

    pub fn toggle_expand(&mut self) {
        if let Some(entry) = self.visible.get(self.selected) {
            if entry.is_dir {
                let path = entry.path.clone();
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
                self.refresh();
            }
        }
    }

    pub fn collapse_or_parent(&mut self) {
        if let Some(entry) = self.visible.get(self.selected) {
            let path = entry.path.clone();
            if entry.is_dir && self.expanded.contains(&path) {
                self.expanded.remove(&path);
                self.refresh();
                return;
            }
            // move to parent
            if let Some(parent) = path.parent() {
                if let Some(pos) = self.visible.iter().position(|e| e.path == parent) {
                    self.selected = pos;
                }
            }
        }
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.visible.get(self.selected).map(|e| e.path.as_path())
    }

    pub fn selected_is_file(&self) -> bool {
        self.visible
            .get(self.selected)
            .map(|e| !e.is_dir)
            .unwrap_or(false)
    }
}

fn compute_visible(root: &Path, expanded: &HashSet<PathBuf>) -> Vec<FileEntry> {
    let mut result = Vec::new();
    walk(root, 0, expanded, &mut result);
    result
}

fn walk(path: &Path, depth: usize, expanded: &HashSet<PathBuf>, out: &mut Vec<FileEntry>) {
    let Ok(read) = fs::read_dir(path) else { return };
    let mut entries: Vec<_> = read.flatten().collect();
    entries.sort_by_key(|e| {
        let is_dir = e.path().is_dir();
        (!is_dir, e.file_name().to_string_lossy().to_lowercase())
    });

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        let ep = entry.path();
        let is_dir = ep.is_dir();
        let is_expanded = is_dir && expanded.contains(&ep);
        out.push(FileEntry {
            path: ep.clone(),
            name,
            depth,
            is_dir,
            expanded: is_expanded,
        });
        if is_expanded {
            walk(&ep, depth + 1, expanded, out);
        }
    }
}

pub fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "r ",
        "toml" | "yaml" | "yml" => "c ",
        "md" => "m ",
        "json" => "j ",
        "js" | "ts" | "jsx" | "tsx" => "j ",
        "py" => "p ",
        "html" | "htm" => "h ",
        "css" | "scss" => "s ",
        "sh" | "bash" => "$ ",
        "lock" => "l ",
        _ => "  ",
    }
}
