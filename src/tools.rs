use std::fs;
use std::path::Path;

pub fn read_file(path: &str, cwd: &str) -> String {
    let full_path = Path::new(cwd).join(path);
    match fs::read_to_string(&full_path) {
        Ok(content) => {
            if content.is_empty() {
                "(empty file)".to_string()
            } else {
                content
            }
        }
        Err(e) => format!("Error reading {}: {}", full_path.display(), e),
    }
}

pub fn create_file(path: &str, content: &str, cwd: &str) -> String {
    let full_path = Path::new(cwd).join(path);
    if let Some(parent) = full_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return format!("Error creating directories: {}", e);
        }
    }
    match fs::write(&full_path, content) {
        Ok(_) => format!("Created: {}", full_path.display()),
        Err(e) => format!("Error writing {}: {}", full_path.display(), e),
    }
}

pub fn edit_file(path: &str, old_str: &str, new_str: &str, cwd: &str) -> String {
    let full_path = Path::new(cwd).join(path);
    let content = match fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => return format!("Error reading {}: {}", full_path.display(), e),
    };
    let count = content.matches(old_str).count();
    if count == 0 {
        return format!(
            "Error: old_str not found in {}. Read the file first to see its current content.",
            path
        );
    }
    if count > 1 {
        return format!(
            "Error: old_str found {} times in {}. Make old_str more unique.",
            count, path
        );
    }
    let updated = content.replacen(old_str, new_str, 1);
    match fs::write(&full_path, updated) {
        Ok(_) => format!("Edited: {}", full_path.display()),
        Err(e) => format!("Error writing {}: {}", full_path.display(), e),
    }
}
