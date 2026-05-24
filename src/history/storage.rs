use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub sql: String,
    pub timestamp: String,
    pub elapsed_ms: f64,
}

pub struct HistoryStorage {
    entries: Vec<HistoryEntry>,
    path: PathBuf,
    matcher: SkimMatcherV2,
    max_entries: usize,
}

impl HistoryStorage {
    pub fn new() -> Self {
        let path = history_path();
        let mut storage = HistoryStorage {
            entries: Vec::new(),
            path,
            matcher: SkimMatcherV2::default(),
            max_entries: 1000,
        };
        storage.load();
        storage
    }

    pub fn append(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.save();
    }

    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        if query.is_empty() {
            return self.entries.iter().rev().take(50).collect();
        }
        let mut scored: Vec<(i64, &HistoryEntry)> = self
            .entries
            .iter()
            .filter_map(|e| {
                self.matcher
                    .fuzzy_match(&e.sql, query)
                    .map(|score| (score, e))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(50).map(|(_, e)| e).collect()
    }

    #[allow(dead_code)]
    pub fn all(&self) -> &[HistoryEntry] {
        &self.entries
    }

    fn load(&mut self) {
        if !self.path.exists() {
            return;
        }
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(_) => return,
        };
        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(line) {
                self.entries.push(entry);
            }
        }
    }

    fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content: Vec<String> = self
            .entries
            .iter()
            .map(|e| serde_json::to_string(e).unwrap_or_default())
            .collect();
        let _ = std::fs::write(&self.path, content.join("\n"));
    }
}

fn history_path() -> PathBuf {
    if let Some(data_dir) = dirs::data_dir() {
        return data_dir.join("lite-pg").join("history.jsonl");
    }
    PathBuf::from(".").join("history.jsonl")
}

impl Default for HistoryStorage {
    fn default() -> Self {
        Self::new()
    }
}
