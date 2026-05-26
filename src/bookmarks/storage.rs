use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub sql: String,
    pub created_at: String,
    pub connection_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkStorage {
    pub bookmarks: Vec<Bookmark>,
}

impl BookmarkStorage {
    pub fn new() -> Self {
        let path = get_bookmarks_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(storage) = serde_json::from_str::<BookmarkStorage>(&content) {
                return storage;
            }
        }
        BookmarkStorage {
            bookmarks: Vec::new(),
        }
    }

    pub fn save(&self) {
        let path = get_bookmarks_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if let Err(e) = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new("."))) {
                eprintln!("Warning: failed to create bookmarks directory: {e}");
            }
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("Warning: failed to save bookmarks: {e}");
            }
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.bookmarks.len() {
            self.bookmarks.remove(index);
            self.save();
        }
    }

    pub fn search(&self, query: &str) -> Vec<&Bookmark> {
        let lower = query.to_lowercase();
        if lower.is_empty() {
            return self.bookmarks.iter().collect();
        }
        self.bookmarks
            .iter()
            .filter(|b| {
                b.name.to_lowercase().contains(&lower)
                    || b.sql.to_lowercase().contains(&lower)
            })
            .collect()
    }
}

fn get_bookmarks_path() -> std::path::PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("lite-pg");
    path.push("bookmarks.json");
    path
}
