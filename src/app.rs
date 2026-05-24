use crate::db::{execute_query, fetch_schema, QueryResult, TableInfo};
use crate::export::markdown::{copy_to_clipboard, rows_to_markdown};
use crate::history::{HistoryEntry, HistoryStorage};
use ratatui::widgets::ListState;
use sqlx::PgPool;
use std::time::Duration;
use tui_textarea::TextArea;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Schema,
    QueryInput,
    Results,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
}

pub enum Action {
    None,
    ExecuteQuery(String),
    ExecuteExplain(String),
    Quit,
}

pub struct App {
    pub db: Option<PgPool>,
    pub connected: bool,
    pub tables: Vec<TableInfo>,
    pub schema_expanded: Vec<bool>,
    pub schema_state: ListState,
    pub query_input: TextArea<'static>,
    pub results: QueryResult,
    pub results_state: ListState,
    pub visual_selection: Vec<usize>,
    pub visual_anchor: Option<usize>,
    pub mode: Mode,
    pub focus: Focus,
    pub elapsed: Duration,
    pub show_history: bool,
    pub history_search: String,
    pub history: HistoryStorage,
    pub scroll_h: u16,
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_style(ratatui::style::Style::default());
        textarea.set_placeholder_text("Enter SQL query...");

        let mut schema_state = ListState::default();
        schema_state.select(Some(0));

        App {
            db: None,
            connected: false,
            tables: Vec::new(),
            schema_expanded: Vec::new(),
            schema_state,
            query_input: textarea,
            results: QueryResult::default(),
            results_state: ListState::default(),
            visual_selection: Vec::new(),
            visual_anchor: None,
            mode: Mode::Normal,
            focus: Focus::Schema,
            elapsed: Duration::default(),
            show_history: false,
            history_search: String::new(),
            history: HistoryStorage::new(),
            scroll_h: 0,
            quit: false,
        }
    }

    pub fn set_db(&mut self, pool: PgPool) {
        self.db = Some(pool);
        self.connected = true;
    }

    pub async fn refresh_schema(&mut self) {
        if let Some(ref pool) = self.db {
            match fetch_schema(pool).await {
                Ok(tables) => {
                    self.tables = tables;
                    self.schema_expanded = vec![false; self.tables.len()];
                    self.schema_state.select(if self.tables.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                }
                Err(e) => {
                    self.results.error = Some(format!("Schema fetch failed: {e}"));
                }
            }
        }
    }

    pub async fn run_query(&mut self, sql: &str) {
        if let Some(ref pool) = self.db {
            let result = execute_query(pool, sql).await;
            self.results = result;
            self.elapsed = self.results.elapsed;
            self.results_state.select(Some(0));
            self.visual_selection.clear();
            self.visual_anchor = None;
            self.focus = Focus::Results;

            if self.results.error.is_none() {
                self.history.append(HistoryEntry {
                    sql: sql.to_string(),
                    timestamp: chrono::Local::now().to_rfc3339(),
                    elapsed_ms: self.results.elapsed.as_secs_f64() * 1000.0,
                });
            }
        } else {
            self.results = QueryResult {
                error: Some("Not connected to database".to_string()),
                ..Default::default()
            };
        }
    }

    pub async fn run_explain(&mut self, sql: &str) {
        if !sql.trim().is_empty() {
            let explain_sql = format!("EXPLAIN ANALYZE {}", sql.trim());
            self.run_query(&explain_sql).await;
        }
    }

    pub fn update_visual_selection(&mut self, start: usize, end: usize) {
        let (lo, hi) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        self.visual_selection = (lo..=hi).collect();
    }

    pub fn copy_markdown(&mut self) {
        let selected: Vec<usize> = if self.visual_selection.is_empty() {
            (0..self.results.rows.len()).collect()
        } else {
            self.visual_selection.clone()
        };

        let markdown = rows_to_markdown(&self.results.columns, &self.results.rows, &selected);
        match copy_to_clipboard(&markdown) {
            Ok(()) => {}
            Err(e) => {
                self.results.error = Some(format!("Clipboard error: {e}"));
            }
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.mode != Mode::Insert {
                    return Action::Quit;
                }
            }
            (KeyCode::Enter, mods) if mods.contains(KeyModifiers::CONTROL) => {
                let sql = self.query_input.lines().join("\n");
                if !sql.trim().is_empty() {
                    return Action::ExecuteQuery(sql);
                }
                return Action::None;
            }
            (KeyCode::F(5), _) => {
                let sql = self.query_input.lines().join("\n");
                if !sql.trim().is_empty() {
                    return Action::ExecuteExplain(sql);
                }
                return Action::None;
            }
            (KeyCode::Char('m'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                self.copy_markdown();
                return Action::None;
            }
            (KeyCode::Char('j'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                let sql = self.query_input.lines().join("\n");
                if !sql.trim().is_empty() {
                    return Action::ExecuteQuery(sql);
                }
                return Action::None;
            }
            _ => {}
        }

        match self.mode {
            Mode::Normal => {
                crate::modes::normal::handle(self, key);
            }
            Mode::Insert => {
                crate::modes::insert::handle(self, key);
            }
            Mode::Visual => {
                crate::modes::visual::handle(self, key);
            }
        }

        if self.quit {
            return Action::Quit;
        }
        Action::None
    }
}
