use crate::bookmarks::BookmarkStorage;
use crate::connections::ConnectionManager;
use crate::db::cache::TimedCache;
use crate::db::{
    execute_query, fetch_active_queries, fetch_databases, fetch_db_stats, fetch_ddl,
    fetch_extensions, fetch_functions, fetch_object_detail, fetch_publications, fetch_roles,
    fetch_schemas, fetch_server_overview, fetch_settings, fetch_subscriptions, fetch_table_data,
    fetch_table_stats, generate_create_role_sql, generate_drop_role_sql, prefetch_all_details,
    search_objects, ActiveQuery, DatabaseInfo, DbObject, DbStatEntry, ExtensionInfo, FunctionInfo,
    PublicationInfo, QueryResult, RoleInfo, SchemaInfo, SearchResult, ServerOverview,
    SettingInfo, SubscriptionInfo, TableStatEntry,
};

use crate::export::{csv, json, markdown, sql_insert};
use crate::history::{HistoryEntry, HistoryStorage};
use crate::tui::role_form::RoleFormState;
use ratatui::widgets::ListState;
use sqlx::PgPool;
use std::time::Duration;
use tui_textarea::{TextArea, CursorMove};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Schema,
    QueryInput,
    Results,
    ConnectionList,
    ConnectionForm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    PropertyEditor,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Home,
    Dashboard,
    Bookmarks,
    Search,
    Roles,
    Databases,
    Functions,
    Extensions,
    Settings,
    Replication,
    ConnectionManager,
    Help,
}

pub struct MenuState {
    pub show: bool,
    pub level: usize,
    pub selection: usize,
    pub parent: usize,
}

impl MenuState {
    pub fn new() -> Self {
        MenuState { show: false, level: 0, selection: 0, parent: 0 }
    }
}

pub enum Action {
    None,
    ExecuteQuery(String),
    ExecuteExplain(String),
    Connect(usize),
    RefreshSchema,
    ShowDetail(usize, usize),
    NextPage,
    PrevPage,
    ShowDdl(usize, usize),
    ExecuteConfirm,
    CancelConfirm,
    GenerateSelect(usize, usize),
    GenerateInsert(usize, usize),
    GenerateUpdate(usize, usize),
    RefreshDashboard,
    RunSearch(String),
    SwitchDatabase(String),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaTreeItem {
    SchemaHeader(usize),
    ObjectRow(usize, usize),
    ColumnSection(usize, usize),
    IndexSection(usize, usize),
    TriggerSection(usize, usize),
    ConstraintSection(usize, usize),
    SequenceSection(usize, usize),
    ColumnRow(usize, usize, usize),
    IndexRow(usize, usize, usize),
    TriggerRow(usize, usize, usize),
    ConstraintRow(usize, usize, usize),
    SequenceRow(usize, usize),
}

pub struct SchemaTreeState {
    pub flat_items: Vec<SchemaTreeItem>,
    pub schema_expanded: Vec<bool>,
    pub object_expanded: Vec<Vec<bool>>,
    pub section_expanded: Vec<Vec<[bool; 5]>>,
    pub list_state: ListState,
}

impl SchemaTreeState {
    pub fn new() -> Self {
        SchemaTreeState {
            flat_items: Vec::new(),
            schema_expanded: Vec::new(),
            object_expanded: Vec::new(),
            section_expanded: Vec::new(),
            list_state: ListState::default(),
        }
    }

    pub fn toggle_object(&mut self, schema_idx: usize, object_idx: usize) {
        if schema_idx < self.object_expanded.len() && object_idx < self.object_expanded[schema_idx].len() {
            self.object_expanded[schema_idx][object_idx] =
                !self.object_expanded[schema_idx][object_idx];
        }
    }

    pub fn is_object_expanded(&self, schema_idx: usize, object_idx: usize) -> bool {
        self.object_expanded
            .get(schema_idx)
            .and_then(|v| v.get(object_idx))
            .copied()
            .unwrap_or(false)
    }

    pub fn toggle_section(&mut self, schema_idx: usize, object_idx: usize, section: usize) {
        if schema_idx < self.section_expanded.len()
            && object_idx < self.section_expanded[schema_idx].len()
            && section < 5
        {
            self.section_expanded[schema_idx][object_idx][section] =
                !self.section_expanded[schema_idx][object_idx][section];
        }
    }

    pub fn is_section_expanded(&self, schema_idx: usize, object_idx: usize, section: usize) -> bool {
        self.section_expanded
            .get(schema_idx)
            .and_then(|v| v.get(object_idx))
            .map(|s| s[section])
            .unwrap_or(false)
    }
}

pub struct CompletionState {
    pub visible: bool,
    pub items: Vec<String>,
    pub selected: usize,
    pub replacement_start: usize,
    pub word: String,
}

impl CompletionState {
    pub fn new() -> Self {
        CompletionState {
            visible: false,
            items: Vec::new(),
            selected: 0,
            replacement_start: 0,
            word: String::new(),
        }
    }

    pub fn update(&mut self, line: &str, cursor_col: usize, schemas: &[crate::db::SchemaInfo]) {
        let before = &line[..cursor_col.min(line.len())];
        let word_start = before
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace() || *c == '(' || *c == ',' || *c == ')' || *c == '.')
            .map(|(i, _)| i + before[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(0))
            .unwrap_or(0);

        let word: String = before[word_start..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        if word.len() < 2 {
            self.visible = false;
            return;
        }

        let lower = word.to_lowercase();
        let mut items: Vec<String> = Vec::new();

        // SQL keywords
        for kw in SQL_KEYWORDS {
            if kw.to_lowercase().starts_with(&lower) {
                items.push(kw.to_string());
            }
        }

        // Schema / object / column names
        for schema in schemas {
            if schema.name.to_lowercase().starts_with(&lower) {
                items.push(schema.name.clone());
            }
            for obj in &schema.objects {
                if obj.name.to_lowercase().starts_with(&lower) {
                    items.push(obj.name.clone());
                }
                for col in &obj.columns {
                    if col.name.to_lowercase().starts_with(&lower) {
                        items.push(col.name.clone());
                    }
                }
            }
        }

        let mut seen = std::collections::HashSet::new();
        items.retain(|item| seen.insert(item.clone()));

        if items.is_empty() {
            self.visible = false;
            return;
        }

        self.word = word;
        self.replacement_start = word_start;
        self.items = items;
        self.selected = 0;
        self.visible = true;
    }

    pub fn accept(&self, line: &str) -> Option<(String, usize)> {
        if !self.visible || self.items.is_empty() {
            return None;
        }
        let raw = &self.items[self.selected];
        let is_keyword = SQL_KEYWORDS.iter().any(|kw| kw.eq_ignore_ascii_case(raw));
        let replacement = if is_keyword {
            raw.clone()
        } else {
            crate::util::quote_ident(raw)
        };
        let before_replace = &line[..self.replacement_start];
        let after_replace = &line[self.replacement_start + self.word.len()..];
        let new_line = format!("{}{}{}", before_replace, replacement, after_replace);
        Some((new_line, replacement.len()))
    }
}

const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE",
    "SET", "DELETE", "CREATE", "TABLE", "ALTER", "DROP", "INDEX",
    "VIEW", "TRIGGER", "SEQUENCE", "FOREIGN", "KEY", "PRIMARY",
    "REFERENCES", "CONSTRAINT", "NOT", "NULL", "DEFAULT", "UNIQUE",
    "CHECK", "INNER", "LEFT", "RIGHT", "JOIN", "ON", "AND", "OR",
    "IN", "EXISTS", "BETWEEN", "LIKE", "ORDER", "BY", "GROUP",
    "HAVING", "LIMIT", "OFFSET", "AS", "DISTINCT", "COUNT", "SUM",
    "AVG", "MIN", "MAX", "CASE", "WHEN", "THEN", "ELSE", "END",
    "BEGIN", "COMMIT", "ROLLBACK", "GRANT", "REVOKE", "SCHEMA",
    "DATABASE", "OWNER", "CASCADE", "RESTRICT", "EXPLAIN", "ANALYZE",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PropSection {
    General,
    Columns,
    Indexes,
    Constraints,
    Triggers,
}

pub static PG_TYPES: &[&str] = &[
    "bigint", "bigserial", "bit", "boolean", "box", "bytea", "character varying",
    "char", "cidr", "circle", "date", "double precision", "inet", "integer",
    "interval", "json", "jsonb", "line", "lseg", "macaddr", "money", "numeric",
    "path", "pg_lsn", "pg_snapshot", "point", "polygon", "real", "smallint",
    "smallserial", "serial", "text", "time", "timestamp", "timestamptz",
    "tsquery", "tsvector", "txid_snapshot", "uuid", "xml",
];

pub struct ColumnForm {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: String,
    pub focus: usize,
    pub edit_index: Option<usize>,
    pub type_filtered: Vec<usize>,
    pub type_selected: usize,
}

impl ColumnForm {
    pub fn new() -> Self {
        let data_type = String::from("text");
        let type_filtered: Vec<usize> = PG_TYPES.iter().enumerate()
            .filter(|(_, t)| t.starts_with(&data_type))
            .map(|(i, _)| i)
            .collect();
        Self { name: String::new(), data_type, is_nullable: true, default_value: String::new(), focus: 0, edit_index: None, type_filtered, type_selected: 0 }
    }
    pub fn for_edit(col: &crate::db::schema::ColumnDetail, col_index: usize) -> Self {
        let data_type = col.data_type.clone();
        let type_filtered: Vec<usize> = PG_TYPES.iter().enumerate()
            .filter(|(_, t)| t.starts_with(&data_type))
            .map(|(i, _)| i)
            .collect();
        Self {
            name: col.name.clone(),
            data_type,
            is_nullable: col.is_nullable,
            default_value: col.default_value.clone().unwrap_or_default(),
            focus: 0,
            edit_index: Some(col_index),
            type_filtered,
            type_selected: 0,
        }
    }
    pub fn refilter_types(&mut self) {
        self.type_filtered = PG_TYPES.iter().enumerate()
            .filter(|(_, t)| t.starts_with(&self.data_type))
            .map(|(i, _)| i)
            .collect();
        if self.type_selected >= self.type_filtered.len() {
            self.type_selected = self.type_filtered.len().saturating_sub(1);
        }
    }
}

pub struct PropertyEditor {
    pub schema_idx: usize,
    pub object_idx: usize,
    pub section: PropSection,
    pub selected: usize,
    pub column_form: Option<ColumnForm>,
}

impl PropertyEditor {
    pub fn new(schema_idx: usize, object_idx: usize) -> Self {
        Self { schema_idx, object_idx, section: PropSection::General, selected: 0, column_form: None }
    }
}

pub struct App {
    // Connection
    pub connections: ConnectionManager,
    pub show_connection_panel: bool,
    pub connection_state: ListState,

    // Connection form
    pub show_connection_form: bool,
    pub form_focus: usize,
    pub form_edit_index: Option<usize>,
    pub form_name: String,
    pub form_host: String,
    pub form_port: String,
    pub form_user: String,
    pub form_password: String,
    pub form_dbname: String,

    // Database
    pub db: Option<PgPool>,
    pub connected: bool,
    pub current_connection_name: Option<String>,

    // Schema
    pub schemas: Vec<SchemaInfo>,
    pub schema_tree: SchemaTreeState,

    // Query tabs
    pub query_tabs: Vec<TextArea<'static>>,
    pub active_tab: usize,
    pub tab_counter: u32,

    // Results
    pub results: QueryResult,
    pub results_state: ListState,

    // Explain result (separate from browse/query results)
    pub explain_output: Option<QueryResult>,

    // Object detail
    pub detail_object: Option<crate::db::DbObject>,
    pub detail_state: ListState,

    // Visual mode
    pub visual_selection: Vec<usize>,
    pub visual_anchor: Option<usize>,

    // Table browsing
    pub completion: CompletionState,
    pub browse_offset: u64,
    pub browse_limit: u64,
    pub browse_schema_idx: Option<usize>,
    pub browse_object_idx: Option<usize>,

    // Background prefetch complete flag
    pub prefetched: bool,

    // Property editor
    pub property_editor: Option<PropertyEditor>,

    // Confirmation dialog
    pub confirm_message: Option<String>,
    pub confirm_sql: Option<String>,

    // Current page
    pub current_page: Page,

    // Main menu overlay
    pub menu: MenuState,

    // Dashboard
    pub dashboard_tab: usize,
    pub server_overview: Option<ServerOverview>,
    pub db_stats: Vec<DbStatEntry>,
    pub active_queries: Vec<ActiveQuery>,
    pub table_stats: Vec<TableStatEntry>,
    pub dashboard_list_state: ListState,
    pub dashboard_cache: TimedCache<ServerOverview>,
    pub dashboard_db_cache: TimedCache<Vec<DbStatEntry>>,
    pub dashboard_aq_cache: TimedCache<Vec<ActiveQuery>>,
    pub dashboard_ts_cache: TimedCache<Vec<TableStatEntry>>,
    pub dashboard_error: Option<String>,

    // Roles
    pub roles: Vec<RoleInfo>,
    pub role_list_state: ListState,
    pub role_confirm: Option<String>,
    pub show_role_form: bool,
    pub role_form: RoleFormState,

    // Databases
    pub databases: Vec<DatabaseInfo>,
    pub database_list_state: ListState,

    // Functions
    pub functions: Vec<FunctionInfo>,
    pub function_list_state: ListState,

    // Extensions
    pub extensions: Vec<ExtensionInfo>,
    pub extension_list_state: ListState,

    // Settings
    pub settings: Vec<SettingInfo>,
    pub settings_list_state: ListState,
    pub settings_filter_category: Option<String>,

    // Replication
    pub publications: Vec<PublicationInfo>,
    pub subscriptions: Vec<SubscriptionInfo>,
    pub replication_tab: usize,
    pub replication_list_state: ListState,

    // Search
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_list_state: ListState,
    pub last_search_time: Option<std::time::Instant>,
    pub search_pending: bool,

    // Bookmarks
    pub bookmark_storage: BookmarkStorage,
    pub bookmark_list_state: ListState,

    pub mode: Mode,
    pub focus: Focus,
    pub elapsed: Duration,
    pub show_history: bool,
    pub history_search: String,
    pub history: HistoryStorage,
    pub scroll_h: u16,
    pub notification: Option<String>,
    pub quit: bool,
}

impl App {
    pub fn current_tab(&self) -> &TextArea<'static> {
        &self.query_tabs[self.active_tab]
    }

    pub fn current_tab_mut(&mut self) -> &mut TextArea<'static> {
        &mut self.query_tabs[self.active_tab]
    }

    pub fn new_tab(&mut self) {
        let mut tab = TextArea::default();
        tab.set_style(ratatui::style::Style::default());
        tab.set_placeholder_text("Enter SQL query...");
        self.tab_counter += 1;
        self.query_tabs.push(tab);
        self.active_tab = self.query_tabs.len() - 1;
    }

    pub fn new_sql_tab(&mut self, sql: &str) {
        let mut tab = TextArea::default();
        tab.set_style(ratatui::style::Style::default());
        tab.set_placeholder_text("Enter SQL query...");
        let lines: Vec<String> = sql.lines().map(|l| l.to_string()).collect();
        tab = tui_textarea::TextArea::from(lines);
        tab.set_style(ratatui::style::Style::default());
        self.tab_counter += 1;
        self.query_tabs.push(tab);
        self.active_tab = self.query_tabs.len() - 1;
    }

    pub fn close_tab(&mut self) {
        if self.query_tabs.len() <= 1 {
            // Clear content of the last tab
            let empty = TextArea::default();
            self.query_tabs[self.active_tab] = empty;
            self.query_tabs[self.active_tab].set_style(ratatui::style::Style::default());
            self.query_tabs[self.active_tab]
                .set_placeholder_text("Enter SQL query...");
            return;
        }
        self.query_tabs.remove(self.active_tab);
        if self.active_tab >= self.query_tabs.len() {
            self.active_tab = self.query_tabs.len() - 1;
        }
    }

    pub fn next_tab(&mut self) {
        if self.query_tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.query_tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if self.query_tabs.len() > 1 {
            self.active_tab = if self.active_tab == 0 {
                self.query_tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    pub fn new() -> Self {
        let mut tab = TextArea::default();
        tab.set_style(ratatui::style::Style::default());
        tab.set_placeholder_text("Enter SQL query...");

        let connections = ConnectionManager::new();
        let show_panel = connections.profiles.is_empty() || connections.active_index.is_none();

        let mut connection_state = ListState::default();
        if !connections.profiles.is_empty() {
            connection_state.select(Some(0));
        }

        App {
            current_page: if show_panel { Page::ConnectionManager } else { Page::Home },
            connections,
            show_connection_panel: show_panel,
            connection_state,

            show_connection_form: false,
            form_focus: 0,
            form_edit_index: None,
            form_name: String::new(),
            form_host: String::from("localhost"),
            form_port: String::from("5432"),
            form_user: String::from("postgres"),
            form_password: String::new(),
            form_dbname: String::new(),

            db: None,
            connected: false,
            current_connection_name: None,

            schemas: Vec::new(),
            schema_tree: SchemaTreeState::new(),

            query_tabs: vec![tab],
            active_tab: 0,
            tab_counter: 1,

            results: QueryResult::default(),
            results_state: ListState::default(),
            explain_output: None,

            detail_object: None,
            detail_state: ListState::default(),

            visual_selection: Vec::new(),
            visual_anchor: None,

            completion: CompletionState::new(),
            browse_offset: 0,
            browse_limit: 100,
            browse_schema_idx: None,
            browse_object_idx: None,
            prefetched: false,

            property_editor: None,

            confirm_message: None,
            confirm_sql: None,

            menu: MenuState::new(),

            dashboard_tab: 0,
            server_overview: None,
            db_stats: Vec::new(),
            active_queries: Vec::new(),
            table_stats: Vec::new(),
            dashboard_list_state: ListState::default(),
            dashboard_cache: TimedCache::new(30),
            dashboard_db_cache: TimedCache::new(30),
            dashboard_aq_cache: TimedCache::new(10),
            dashboard_ts_cache: TimedCache::new(30),
            dashboard_error: None,

            roles: Vec::new(),
            role_list_state: ListState::default(),
            role_confirm: None,
            show_role_form: false,
            role_form: RoleFormState::new(),

            databases: Vec::new(),
            database_list_state: ListState::default(),

            functions: Vec::new(),
            function_list_state: ListState::default(),

            extensions: Vec::new(),
            extension_list_state: ListState::default(),

            settings: Vec::new(),
            settings_list_state: ListState::default(),
            settings_filter_category: None,

            publications: Vec::new(),
            subscriptions: Vec::new(),
            replication_tab: 0,
            replication_list_state: ListState::default(),

            search_query: String::new(),
            search_results: Vec::new(),
            search_list_state: ListState::default(),
            last_search_time: None,
            search_pending: false,

            bookmark_storage: BookmarkStorage::new(),
            bookmark_list_state: ListState::default(),

            mode: Mode::Normal,
            focus: Focus::Schema,
            elapsed: Duration::default(),
            show_history: false,
            history_search: String::new(),
            history: HistoryStorage::new(),
            scroll_h: 0,
            notification: None,
            quit: false,
        }
    }

    pub fn set_db(&mut self, pool: PgPool, name: String) {
        self.db = Some(pool);
        self.connected = true;
        self.current_connection_name = Some(name);
    }

    // ── Dashboard ───────────────────────────────────────────────

    pub async fn refresh_dashboard(&mut self) {
        if let Some(ref pool) = self.db {
            self.dashboard_error = None;

            if self.dashboard_cache.get().is_none() {
                match fetch_server_overview(pool).await {
                    Ok(ov) => {
                        self.dashboard_cache.set(ov.clone());
                        self.server_overview = Some(ov);
                    }
                    Err(e) => {
                        self.server_overview = None;
                        self.dashboard_error = Some(format!("Dashboard: {e}"));
                    }
                }
            } else {
                self.server_overview = self.dashboard_cache.get().cloned();
            }

            if self.dashboard_db_cache.get().is_none() {
                match fetch_db_stats(pool).await {
                    Ok(stats) => {
                        self.dashboard_db_cache.set(stats.clone());
                        self.db_stats = stats;
                    }
                    Err(e) => {
                        self.dashboard_error = Some(format!("DB stats: {e}"));
                    }
                }
            } else {
                self.db_stats = self.dashboard_db_cache.get().cloned().unwrap_or_default();
            }

            if self.dashboard_aq_cache.get().is_none() {
                match fetch_active_queries(pool).await {
                    Ok(queries) => {
                        self.dashboard_aq_cache.set(queries.clone());
                        self.active_queries = queries;
                    }
                    Err(e) => {
                        self.dashboard_error = Some(format!("Active queries: {e}"));
                    }
                }
            } else {
                self.active_queries = self
                    .dashboard_aq_cache
                    .get()
                    .cloned()
                    .unwrap_or_default();
            }

            if self.dashboard_ts_cache.get().is_none() {
                match fetch_table_stats(pool, None).await {
                    Ok(stats) => {
                        self.dashboard_ts_cache.set(stats.clone());
                        self.table_stats = stats;
                    }
                    Err(e) => {
                        self.dashboard_error = Some(format!("Table stats: {e}"));
                    }
                }
            } else {
                self.table_stats = self
                    .dashboard_ts_cache
                    .get()
                    .cloned()
                    .unwrap_or_default();
            }
        }
    }

    // ── Roles ───────────────────────────────────────────────────

    pub async fn refresh_roles(&mut self) {
        if let Some(ref pool) = self.db {
            match fetch_roles(pool).await {
                Ok(roles) => {
                    self.roles = roles;
                    self.role_list_state.select(if self.roles.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                }
                Err(e) => {
                    self.results = QueryResult {
                        error: Some(format!("Role fetch failed: {e}")),
                        ..Default::default()
                    };
                }
            }
        }
    }

    pub fn open_role_form(&mut self) {
        self.role_form.reset();
        self.show_role_form = true;
    }

    pub fn save_role_form(&mut self) {
        let conn_limit: i32 = self.role_form.conn_limit.parse().unwrap_or(-1);
        let sql = generate_create_role_sql(
            &self.role_form.name,
            self.role_form.login,
            self.role_form.superuser,
            self.role_form.createdb,
            self.role_form.createrole,
            self.role_form.replication,
            &self.role_form.password,
            conn_limit,
        );
        self.show_role_form = false;
        // The caller (main.rs event loop) will handle executing this SQL
        // Store it in confirm_sql to be executed
        self.confirm_sql = Some(sql);
        self.confirm_message = Some("Execute CREATE ROLE?".to_string());
    }

    pub fn confirm_drop_role(&mut self, idx: usize) {
        if idx < self.roles.len() {
            let name = &self.roles[idx].name;
            self.role_confirm = Some(name.clone());
        }
    }

    // ── Databases ──────────────────────────────────────────────

    pub async fn refresh_databases(&mut self) {
        if let Some(ref pool) = self.db {
            match fetch_databases(pool).await {
                Ok(dbs) => {
                    self.databases = dbs;
                    self.database_list_state.select(if self.databases.is_empty() { None } else { Some(1) });
                }
                Err(e) => self.results.error = Some(format!("DB fetch: {e}")),
            }
        }
    }

    // ── Functions ──────────────────────────────────────────────

    pub async fn refresh_functions(&mut self) {
        if let Some(ref pool) = self.db {
            let mut all_funcs = Vec::new();
            for schema in &self.schemas {
                match fetch_functions(pool, &schema.name).await {
                    Ok(funcs) => all_funcs.extend(funcs),
                    Err(e) => {
                        self.results.error = Some(format!("Function fetch ({}): {e}", schema.name));
                        return;
                    }
                }
            }
            self.functions = all_funcs;
            self.function_list_state.select(if self.functions.is_empty() { None } else { Some(0) });
        }
    }

    // ── Extensions ─────────────────────────────────────────────

    pub async fn refresh_extensions(&mut self) {
        if let Some(ref pool) = self.db {
            match fetch_extensions(pool).await {
                Ok(exts) => {
                    self.extensions = exts;
                    self.extension_list_state.select(Some(0));
                }
                Err(e) => self.results.error = Some(format!("Extension fetch: {e}")),
            }
        }
    }

    // ── Settings ───────────────────────────────────────────────

    pub async fn refresh_settings(&mut self) {
        if let Some(ref pool) = self.db {
            match fetch_settings(pool).await {
                Ok(s) => {
                    self.settings = s;
                    self.settings_list_state.select(Some(0));
                }
                Err(e) => self.results.error = Some(format!("Settings fetch: {e}")),
            }
        }
    }

    // ── Replication ────────────────────────────────────────────

    pub async fn refresh_replication(&mut self) {
        if let Some(ref pool) = self.db {
            match fetch_publications(pool).await {
                Ok(pubs) => self.publications = pubs,
                Err(e) => self.results.error = Some(format!("Publications fetch: {e}")),
            }
            match fetch_subscriptions(pool).await {
                Ok(subs) => self.subscriptions = subs,
                Err(e) => self.results.error = Some(format!("Subscriptions fetch: {e}")),
            }
        }
    }

    // ── Search ─────────────────────────────────────────────────

    pub async fn run_search(&mut self, query: &str) {
        if let Some(ref pool) = self.db {
            if !query.trim().is_empty() {
                match search_objects(pool, query).await {
                    Ok(results) => {
                        self.search_results = results;
                        self.search_list_state.select(Some(0));
                    }
                    Err(e) => self.results.error = Some(format!("Search: {e}")),
                }
            } else {
                self.search_results.clear();
            }
        }
    }

    // ── Schema ──────────────────────────────────────────────────

    pub async fn refresh_schema(&mut self) {
        if let Some(ref pool) = self.db {
            match fetch_schemas(pool).await {
                Ok(schemas) => {
                    self.schemas = schemas;
                    self.schema_tree.schema_expanded = vec![false; self.schemas.len()];
                    self.schema_tree.object_expanded = vec![Vec::new(); self.schemas.len()];
                    self.detail_object = None;
                    self.browse_schema_idx = None;
                    self.browse_object_idx = None;
                    self.prefetched = false;
                    self.schema_tree.list_state.select(if self.schemas.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                }
                Err(e) => {
                    self.results = QueryResult {
                        error: Some(format!("Schema fetch failed: {e}")),
                        ..Default::default()
                    };
                }
            }
        }
    }

    pub async fn run_prefetch(&mut self) {
        if self.prefetched || self.db.is_none() {
            return;
        }
        let objects: Vec<(String, String, crate::db::DbObjectType)> = self
            .schemas
            .iter()
            .flat_map(|s| {
                s.objects.iter().filter_map(|o| {
                    matches!(
                        o.obj_type,
                        crate::db::DbObjectType::Table | crate::db::DbObjectType::View
                    )
                    .then(|| (o.schema_name.clone(), o.name.clone(), o.obj_type.clone()))
                })
            })
            .collect();

        if objects.is_empty() {
            self.prefetched = true;
            return;
        }

        if let Some(ref pool) = self.db.clone() {
            if let Ok(results) = prefetch_all_details(pool, &objects).await {
                for (schema_name, obj_name, cols, indexes, triggers, constraints, row_count, size_bytes) in results {
                    for schema in &mut self.schemas {
                        for obj in &mut schema.objects {
                            if obj.schema_name == schema_name && obj.name == obj_name && obj.columns.is_empty() {
                                obj.columns = cols.clone();
                                obj.indexes = indexes.clone();
                                obj.triggers = triggers.clone();
                                obj.constraints = constraints.clone();
                                if let Some(rc) = row_count { obj.row_count = Some(rc); }
                                if let Some(sz) = size_bytes { obj.size_bytes = Some(sz); }
                                obj.detail_loaded = true;
                            }
                        }
                    }
                }
            }
        }
        self.prefetched = true;
    }

    pub async fn show_object_detail(&mut self, schema_idx: usize, object_idx: usize) {
        if schema_idx >= self.schemas.len() || object_idx >= self.schemas[schema_idx].objects.len() {
            return;
        }

        // If already loaded, just toggle expansion in tree
        if self.schemas[schema_idx].objects[object_idx].detail_loaded {
            self.schema_tree.toggle_object(schema_idx, object_idx);
            return;
        }

        // Fetch detail from database and populate inline in the tree
        let obj = &self.schemas[schema_idx].objects[object_idx];
        if let Some(ref pool) = self.db {
            match fetch_object_detail(pool, &obj.schema_name, &obj.name, &obj.obj_type).await {
                Ok(detail) => {
                    let obj_mut = &mut self.schemas[schema_idx].objects[object_idx];
                    obj_mut.columns = detail.columns;
                    obj_mut.indexes = detail.indexes;
                    obj_mut.triggers = detail.triggers;
                    obj_mut.constraints = detail.constraints;
                    obj_mut.row_count = detail.row_count;
                    obj_mut.size_bytes = detail.size_bytes;
                    obj_mut.description = detail.description;
                    obj_mut.detail_loaded = true;

                    self.schema_tree.toggle_object(schema_idx, object_idx);
                }
                Err(e) => {
                    self.results = QueryResult {
                        error: Some(format!("Detail fetch failed: {e}")),
                        ..Default::default()
                    };
                }
            }
        }
    }

    pub fn generate_select(&mut self, schema_idx: usize, object_idx: usize) {
        if schema_idx >= self.schemas.len() || object_idx >= self.schemas[schema_idx].objects.len() {
            return;
        }
        let obj = &self.schemas[schema_idx].objects[object_idx];
        let sql = format!(
            r#"SELECT * FROM "{}"."{}"
LIMIT 100;"#,
            obj.schema_name, obj.name
        );
        self.new_sql_tab(&sql);
        self.focus = Focus::QueryInput;
    }

    pub fn generate_insert(&mut self, schema_idx: usize, object_idx: usize) {
        if schema_idx >= self.schemas.len() || object_idx >= self.schemas[schema_idx].objects.len() {
            return;
        }
        let obj = &self.schemas[schema_idx].objects[object_idx];
        let cols: Vec<&str> = obj.columns.iter().map(|c| c.name.as_str()).collect();
        let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("${}", i)).collect();
        let sql = format!(
            r#"INSERT INTO "{}"."{}" ({})
VALUES ({});"#,
            obj.schema_name,
            obj.name,
            cols.iter().map(|c| crate::util::quote_ident(c)).collect::<Vec<_>>().join(", "),
            placeholders.join(", "),
        );
        self.new_sql_tab(&sql);
        self.focus = Focus::QueryInput;
    }

    pub fn generate_update(&mut self, schema_idx: usize, object_idx: usize) {
        if schema_idx >= self.schemas.len() || object_idx >= self.schemas[schema_idx].objects.len() {
            return;
        }
        let obj = &self.schemas[schema_idx].objects[object_idx];
        let set_clause: Vec<String> = obj.columns.iter().enumerate().map(|(i, c)| format!(r#"{} = ${}"#, crate::util::quote_ident(&c.name), i + 1)).collect();
        let sql = format!(
            r#"UPDATE "{}"."{}"
                SET {}
                WHERE 1=1 -- add conditions;"#,
            obj.schema_name, obj.name,
            set_clause.join(", "),
        );
        self.new_sql_tab(&sql);
        self.focus = Focus::QueryInput;
    }

    pub async fn load_browse_page(&mut self) {
        let (si, oi) = match (self.browse_schema_idx, self.browse_object_idx) {
            (Some(si), Some(oi)) => (si, oi),
            _ => return,
        };
        if si >= self.schemas.len() || oi >= self.schemas[si].objects.len() {
            return;
        }
        let obj = &self.schemas[si].objects[oi];
        if let Some(ref pool) = self.db {
            let result = fetch_table_data(pool, &obj.schema_name, &obj.name, self.browse_limit, self.browse_offset).await;
            self.results = result;
            self.explain_output = None;
            self.elapsed = self.results.elapsed;
            self.results_state.select(Some(0));
            self.visual_selection.clear();
            self.visual_anchor = None;
            self.focus = Focus::Results;
        }
    }

    pub fn next_page(&mut self) {
        if self.browse_schema_idx.is_some() {
            self.browse_offset = self.browse_offset.saturating_add(self.browse_limit);
        }
    }

    pub fn prev_page(&mut self) {
        if self.browse_schema_idx.is_some() {
            self.browse_offset = self.browse_offset.saturating_sub(self.browse_limit);
        }
    }

    // ── Query ────────────────────────────────────────────────────

    pub async fn run_query(&mut self, sql: &str) {
        if let Some(ref pool) = self.db {
            let result = execute_query(pool, sql).await;
            self.results = result;
            self.elapsed = self.results.elapsed;
            self.results_state.select(Some(0));
            self.visual_selection.clear();
            self.visual_anchor = None;
            self.detail_object = None;
            self.explain_output = None;
            self.mode = Mode::Normal;
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
            if let Some(ref pool) = self.db {
                let result = execute_query(pool, &explain_sql).await;
                self.explain_output = Some(result);
                self.detail_object = None;
                self.mode = Mode::Normal;
                self.focus = Focus::Results;
            }
        }
    }

    // ── Visual ────────────────────────────────────────────────────

    pub fn update_visual_selection(&mut self, start: usize, end: usize) {
        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
        self.visual_selection = (lo..=hi).collect();
    }

    pub fn copy_markdown(&mut self) {
        let selected: Vec<usize> = if self.visual_selection.is_empty() {
            (0..self.results.rows.len()).collect()
        } else {
            self.visual_selection.clone()
        };
        let markdown = markdown::rows_to_markdown(&self.results.columns, &self.results.rows, &selected);
        if let Err(e) = markdown::copy_to_clipboard(&markdown) {
            self.results.error = Some(format!("Clipboard error: {e}"));
        }
    }

    // ── Completion ──────────────────────────────────────────────

    pub fn update_completions(&mut self) {
        let (line, col) = {
            let lines = self.current_tab().lines();
            let (line_idx, col) = self.current_tab().cursor();
            if line_idx < lines.len() {
                (lines[line_idx].clone(), col)
            } else {
                return;
            }
        };
        self.completion
            .update(&line, col, &self.schemas);
    }

    pub fn accept_completion(&mut self) {
        let lines: Vec<String> = self.current_tab().lines().to_vec();
        let (line_idx, _col) = self.current_tab().cursor();
        if line_idx >= lines.len() {
            return;
        }
        if let Some((new_line, replacement_len)) = self.completion.accept(&lines[line_idx]) {
            let new_cursor = self.completion.replacement_start + replacement_len;
            let mut new_lines = lines;
            new_lines[line_idx] = new_line;
            let refs: Vec<&str> = new_lines.iter().map(|s| s.as_str()).collect();
            self.query_tabs[self.active_tab] = tui_textarea::TextArea::from(refs);
            self.query_tabs[self.active_tab].move_cursor(CursorMove::Jump(line_idx as u16, new_cursor as u16));
            self.completion.visible = false;
        }
    }

    pub async fn show_ddl(&mut self, schema_idx: usize, object_idx: usize) {
        if schema_idx >= self.schemas.len() || object_idx >= self.schemas[schema_idx].objects.len() {
            return;
        }
        let obj = &self.schemas[schema_idx].objects[object_idx];
        if let Some(ref pool) = self.db {
            match fetch_ddl(pool, &obj.schema_name, &obj.name, &obj.obj_type).await {
                Ok(ddl) => {
                    let lines: Vec<&str> = ddl.lines().collect();
                    self.results = QueryResult {
                        columns: vec![format!("DDL: {}.{}", obj.schema_name, obj.name)],
                        rows: lines.iter().map(|l| vec![l.to_string()]).collect(),
                        ..Default::default()
                    };
                    self.elapsed = Duration::default();
                    self.results_state.select(Some(0));
                    self.detail_object = None;
                    self.focus = Focus::Results;
                }
                Err(e) => {
                    self.results = QueryResult {
                        error: Some(format!("DDL fetch failed: {e}")),
                        ..Default::default()
                    };
                }
            }
        }
        }

    pub fn save_markdown_to_file(&mut self) {
        let selected: Vec<usize> = if self.visual_selection.is_empty() {
            (0..self.results.rows.len()).collect()
        } else {
            self.visual_selection.clone()
        };
        let content = markdown::rows_to_markdown(&self.results.columns, &self.results.rows, &selected);

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("lite-pg_export_{}.md", timestamp);
        let path = std::path::Path::new(&filename);

        match markdown::save_to_file(&content, path) {
            Ok(()) => {
                self.notification = Some(format!("Saved to: {}", path.display()));
            }
            Err(e) => {
                self.notification = Some(format!("Save failed: {e}"));
            }
        }
    }

    pub fn export_csv(&mut self) {
        self.export_file("csv", |cols, rows, sel| csv::rows_to_csv(cols, rows, sel));
    }

    pub fn export_json(&mut self) {
        self.export_file("json", |cols, rows, sel| json::rows_to_json(cols, rows, sel));
    }

    pub fn export_sql_insert(&mut self) {
        let table_name = self
            .browse_schema_idx
            .and_then(|si| {
                self.browse_object_idx
                    .map(|oi| self.schemas[si].objects[oi].name.clone())
            })
            .unwrap_or_else(|| "query_result".to_string());

        let selected: Vec<usize> = if self.visual_selection.is_empty() {
            (0..self.results.rows.len()).collect()
        } else {
            self.visual_selection.clone()
        };
        let content = sql_insert::rows_to_sql_insert(
            &self.results.columns,
            &self.results.rows,
            &selected,
            &table_name,
        );

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("lite-pg_export_{}.sql", timestamp);
        let path = std::path::Path::new(&filename);

        match sql_insert::save_to_file(&content, path) {
            Ok(()) => {
                self.notification = Some(format!("Saved to: {}", path.display()));
            }
            Err(e) => {
                self.notification = Some(format!("Save failed: {e}"));
            }
        }
    }

    fn export_file(&mut self, ext: &str, formatter: fn(&[String], &[Vec<String>], &[usize]) -> String) {
        let selected: Vec<usize> = if self.visual_selection.is_empty() {
            (0..self.results.rows.len()).collect()
        } else {
            self.visual_selection.clone()
        };
        let content = formatter(&self.results.columns, &self.results.rows, &selected);

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("lite-pg_export_{}.{}", timestamp, ext);
        let path = std::path::Path::new(&filename);

        let saver = match ext {
            "csv" => csv::save_to_file,
            "json" => json::save_to_file,
            _ => return,
        };
        match saver(&content, path) {
            Ok(()) => {
                self.notification = Some(format!("Saved to: {}", path.display()));
            }
            Err(e) => {
                self.notification = Some(format!("Save failed: {e}"));
            }
        }
    }

    // ── Connection form ───────────────────────────────────────────

    pub fn open_new_connection_form(&mut self) {
        self.show_connection_form = true;
        self.form_edit_index = None;
        self.form_focus = 0;
        self.form_name.clear();
        self.form_host = String::from("localhost");
        self.form_port = String::from("5432");
        self.form_user = String::from("postgres");
        self.form_password.clear();
        self.form_dbname.clear();
        self.focus = Focus::ConnectionForm;
    }

    pub fn open_edit_connection_form(&mut self, index: usize) {
        if let Some(profile) = self.connections.profiles.get(index) {
            self.show_connection_form = true;
            self.form_edit_index = Some(index);
            self.form_focus = 0;
            self.form_name = profile.name.clone();
            self.form_host = profile.host.clone();
            self.form_port = profile.port.to_string();
            self.form_user = profile.user.clone();
            self.form_password.clear();
            self.form_dbname = profile.dbname.clone();
            self.focus = Focus::ConnectionForm;
        }
    }

    pub fn save_connection_form(&mut self) {
        let port: u16 = self.form_port.parse().unwrap_or(5432);
        let password = if self.form_password.is_empty() {
            None
        } else {
            Some(self.form_password.as_str())
        };

        if let Some(idx) = self.form_edit_index {
            self.connections.update_profile(
                idx, &self.form_name, &self.form_host, port,
                &self.form_user, password, &self.form_dbname,
            );
        } else {
            self.connections.add_profile(
                &self.form_name, &self.form_host, port,
                &self.form_user, password, &self.form_dbname,
            );
            let last = self.connections.profiles.len().saturating_sub(1);
            self.connection_state.select(Some(last));
        }

        self.show_connection_form = false;
        self.form_password.clear();
        self.focus = Focus::ConnectionList;
    }

    // ── Key handling ──────────────────────────────────────────────

    // ── Page-specific key handlers ───────────────────────────────

    fn handle_roles_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.roles.len().saturating_sub(1);
                let sel = self.role_list_state.selected().unwrap_or(0);
                self.role_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.role_list_state.selected().unwrap_or(0);
                self.role_list_state.select(Some(sel.saturating_sub(1)));
            }
            KeyCode::Char('n') if self.role_confirm.is_none() => {
                self.open_role_form();
            }
            KeyCode::Char('d') if self.role_confirm.is_none() => {
                if let Some(sel) = self.role_list_state.selected() {
                    if sel < self.roles.len() {
                        self.confirm_drop_role(sel);
                    }
                }
            }
            KeyCode::Char('y') | KeyCode::Enter if self.role_confirm.is_some() => {
                if let Some(ref name) = self.role_confirm.clone() {
                    let sql = generate_drop_role_sql(&name);
                    self.role_confirm = None;
                    self.confirm_sql = Some(sql);
                    self.confirm_message = Some(format!("DROP ROLE {}?", name));
                }
            }
            KeyCode::Char('n') if self.role_confirm.is_some() => {
                self.role_confirm = None;
            }
            _ => {}
        }
        Action::None
    }

    fn handle_databases_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;

        let db_index = |sel: usize| -> Option<usize> {
            if sel > 0 { Some(sel - 1) } else { None }
        };

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.databases.len();
                let sel = self.database_list_state.selected().unwrap_or(0);
                self.database_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.database_list_state.selected().unwrap_or(0);
                self.database_list_state.select(Some(if sel > 1 { sel - 1 } else { 1 }));
            }
            KeyCode::Char('n') => {
                if let Some(sel) = self.database_list_state.selected() {
                    if let Some(idx) = db_index(sel) {
                        if idx < self.databases.len() {
                            let db = &self.databases[idx];
                            let sql = crate::db::generate_create_database_sql(&db.name, &db.owner, &db.encoding);
                            self.confirm_sql = Some(sql);
                            self.confirm_message = Some("Execute CREATE DATABASE?".to_string());
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(sel) = self.database_list_state.selected() {
                    if let Some(idx) = db_index(sel) {
                        if idx < self.databases.len() {
                            let name = &self.databases[idx].name;
                            let sql = crate::db::generate_drop_database_sql(name);
                            self.confirm_message = Some(format!("DROP DATABASE {}?", name));
                            self.confirm_sql = Some(sql);
                        }
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(sel) = self.database_list_state.selected() {
                    if let Some(idx) = db_index(sel) {
                        if idx < self.databases.len() {
                            let name = self.databases[idx].name.clone();
                            return Action::SwitchDatabase(name);
                        }
                    }
                }
            }
            _ => {}
        }
        Action::None
    }

    fn handle_functions_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.functions.len().saturating_sub(1);
                let sel = self.function_list_state.selected().unwrap_or(0);
                self.function_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.function_list_state.selected().unwrap_or(0);
                self.function_list_state.select(Some(sel.saturating_sub(1)));
            }
            KeyCode::Char('D') => {
                if let Some(sel) = self.function_list_state.selected() {
                    if sel < self.functions.len() {
                        let f = &self.functions[sel];
                        if let Some(ref def) = f.definition {
                            let lines: Vec<&str> = def.lines().collect();
                            self.results = QueryResult {
                                columns: vec![format!("DDL: {}.{}", f.schema, f.name)],
                                rows: lines.iter().map(|l| vec![l.to_string()]).collect(),
                                ..Default::default()
                            };
                        }
                    }
                }
            }
            _ => {}
        }
        Action::None
    }

    fn handle_extensions_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.extensions.len().saturating_sub(1);
                let sel = self.extension_list_state.selected().unwrap_or(0);
                self.extension_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.extension_list_state.selected().unwrap_or(0);
                self.extension_list_state.select(Some(sel.saturating_sub(1)));
            }
            _ => {}
        }
        Action::None
    }

    fn handle_settings_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.settings.len().saturating_sub(1);
                let sel = self.settings_list_state.selected().unwrap_or(0);
                self.settings_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.settings_list_state.selected().unwrap_or(0);
                self.settings_list_state.select(Some(sel.saturating_sub(1)));
            }
            _ => {}
        }
        Action::None
    }

    fn handle_replication_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                self.replication_tab = self.replication_tab.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.replication_tab = (self.replication_tab + 1).min(1);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = if self.replication_tab == 0 {
                    self.publications.len().saturating_sub(1)
                } else {
                    self.subscriptions.len().saturating_sub(1)
                };
                let sel = self.replication_list_state.selected().unwrap_or(0);
                self.replication_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.replication_list_state.selected().unwrap_or(0);
                self.replication_list_state.select(Some(sel.saturating_sub(1)));
            }
            _ => {}
        }
        Action::None
    }

    fn handle_dashboard_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                self.dashboard_tab = self.dashboard_tab.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.dashboard_tab = (self.dashboard_tab + 1).min(3);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = match self.dashboard_tab {
                    1 => self.db_stats.len().saturating_sub(1),
                    2 => self.active_queries.len().saturating_sub(1),
                    3 => self.table_stats.len().saturating_sub(1),
                    _ => 0,
                };
                let sel = self.dashboard_list_state.selected().unwrap_or(0);
                self.dashboard_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.dashboard_list_state.selected().unwrap_or(0);
                if sel > 0 {
                    self.dashboard_list_state.select(Some(sel.saturating_sub(1)));
                }
            }
            KeyCode::Char('r') => {
                // handled async in main.rs
                return Action::RefreshDashboard;
            }
            _ => {}
        }
        Action::None
    }

    fn handle_search_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Enter if !self.search_query.is_empty() => {
                return Action::RunSearch(self.search_query.clone());
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.search_results.len().saturating_sub(1);
                let sel = self.search_list_state.selected().unwrap_or(0);
                self.search_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.search_list_state.selected().unwrap_or(0);
                self.search_list_state.select(Some(sel.saturating_sub(1)));
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                if self.search_query.len() >= 2 {
                    self.search_pending = true;
                    self.last_search_time = Some(std::time::Instant::now());
                } else {
                    self.search_results.clear();
                    self.search_pending = false;
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                if self.search_query.len() >= 2 {
                    self.search_pending = true;
                    self.last_search_time = Some(std::time::Instant::now());
                }
            }
            _ => {}
        }
        Action::None
    }

    fn handle_bookmarks_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let bm = self.bookmark_storage.search("");
                let max = bm.len().saturating_sub(1);
                let sel = self.bookmark_list_state.selected().unwrap_or(0);
                self.bookmark_list_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.bookmark_list_state.selected().unwrap_or(0);
                self.bookmark_list_state.select(Some(sel.saturating_sub(1)));
            }
            KeyCode::Enter => {
                let sel = self.bookmark_list_state.selected();
                let sql = self.bookmark_storage.search("").get(sel.unwrap_or(999)).map(|b| b.sql.clone());
                if let Some(s) = sql {
                    self.new_sql_tab(&s);
                    self.current_page = Page::Home;
                }
            }
            KeyCode::Char('d') => {
                if let Some(sel) = self.bookmark_list_state.selected() {
                    self.bookmark_storage.remove(sel);
                }
            }
            _ => {}
        }
        Action::None
    }

    fn handle_connection_manager_page_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key.code {
            KeyCode::Enter => {
                if let Some(sel) = self.connection_state.selected() {
                    if sel < self.connections.profiles.len() {
                        self.show_connection_panel = false;
                        self.current_page = Page::Home;
                        return Action::Connect(sel);
                    }
                }
            }
            KeyCode::Char('q') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.connections.profiles.len().saturating_sub(1);
                let sel = self.connection_state.selected().unwrap_or(0);
                self.connection_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.connection_state.selected().unwrap_or(0);
                self.connection_state.select(Some(sel.saturating_sub(1)));
            }
            KeyCode::Char('n') => self.open_new_connection_form(),
            KeyCode::Char('e') => {
                if let Some(sel) = self.connection_state.selected() {
                    if sel < self.connections.profiles.len() {
                        self.open_edit_connection_form(sel);
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(sel) = self.connection_state.selected() {
                    if sel < self.connections.profiles.len() {
                        self.connections.remove_profile(sel);
                        let max = self.connections.profiles.len().saturating_sub(1);
                        self.connection_state.select(
                            if self.connections.profiles.is_empty() { None } else { Some(sel.min(max)) }
                        );
                        self.connections.save_profiles();
                    }
                }
            }
            _ => {}
        }
        Action::None
    }

    pub fn open_property_editor(&mut self, schema_idx: usize, object_idx: usize) {
        if schema_idx < self.schemas.len() && object_idx < self.schemas[schema_idx].objects.len() {
            self.property_editor = Some(PropertyEditor::new(schema_idx, object_idx));
            self.mode = Mode::PropertyEditor;
        }
    }

    fn handle_property_editor_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let editor = match self.property_editor.as_ref() {
            Some(e) => e,
            None => return,
        };

        // Column form takes priority
        if editor.column_form.is_some() {
            self.handle_column_form_key(key);
            return;
        }

        let schema_idx = editor.schema_idx;
        let object_idx = editor.object_idx;

        match key.code {
            KeyCode::Esc => {
                self.property_editor = None;
                self.mode = Mode::Normal;
            }
            KeyCode::Tab | KeyCode::Char('l') => {
                if let Some(ref mut e) = self.property_editor {
                    e.section = match e.section {
                        PropSection::General => PropSection::Columns,
                        PropSection::Columns => PropSection::Indexes,
                        PropSection::Indexes => PropSection::Constraints,
                        PropSection::Constraints => PropSection::Triggers,
                        PropSection::Triggers => PropSection::General,
                    };
                    e.selected = 0;
                }
            }
            KeyCode::BackTab | KeyCode::Char('h') => {
                if let Some(ref mut e) = self.property_editor {
                    e.section = match e.section {
                        PropSection::General => PropSection::Triggers,
                        PropSection::Columns => PropSection::General,
                        PropSection::Indexes => PropSection::Columns,
                        PropSection::Constraints => PropSection::Indexes,
                        PropSection::Triggers => PropSection::Constraints,
                    };
                    e.selected = 0;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(ref mut e) = self.property_editor {
                    let count = if schema_idx < self.schemas.len() && object_idx < self.schemas[schema_idx].objects.len() {
                        let obj = &self.schemas[schema_idx].objects[object_idx];
                        match e.section {
                            PropSection::General => 1,
                            PropSection::Columns => obj.columns.len().max(1),
                            PropSection::Indexes => obj.indexes.len().max(1),
                            PropSection::Constraints => obj.constraints.len().max(1),
                            PropSection::Triggers => obj.triggers.len().max(1),
                        }
                    } else { 1 };
                    e.selected = (e.selected + 1).min(count.saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(ref mut e) = self.property_editor {
                    e.selected = e.selected.saturating_sub(1);
                }
            }
            KeyCode::Char('d') => {
                self.drop_property_item(schema_idx, object_idx);
            }
            KeyCode::Char('a') => {
                if let Some(ref mut e) = self.property_editor {
                    if e.section == PropSection::Columns {
                        e.column_form = Some(ColumnForm::new());
                        e.column_form.as_mut().unwrap().focus = 0;
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(ref mut e) = self.property_editor {
                    if e.section == PropSection::Columns && e.selected < self.schemas[schema_idx].objects[object_idx].columns.len() {
                        let col = &self.schemas[schema_idx].objects[object_idx].columns[e.selected];
                        e.column_form = Some(ColumnForm::for_edit(col, e.selected));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_column_form_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let (schema_idx, object_idx, _section, _selected) = match self.property_editor.as_ref() {
            Some(e) => (e.schema_idx, e.object_idx, e.section, e.selected),
            None => return,
        };

        let is_type_focused = self.property_editor.as_ref()
            .and_then(|e| e.column_form.as_ref())
            .map_or(false, |f| f.focus == 1);

        match key.code {
            KeyCode::Esc => {
                if let Some(ref mut e) = self.property_editor {
                    e.column_form = None;
                }
            }
            KeyCode::Tab => {
                if let Some(ref mut e) = self.property_editor {
                    if let Some(ref mut f) = e.column_form {
                        f.focus = (f.focus + 1) % 4;
                        if f.focus == 1 { f.refilter_types(); }
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(ref mut e) = self.property_editor {
                    if let Some(ref mut f) = e.column_form {
                        if f.focus == 1 && !f.type_filtered.is_empty() {
                            // Select the highlighted type from dropdown
                            f.data_type = PG_TYPES[f.type_filtered[f.type_selected]].to_string();
                            f.focus += 1;
                        } else if f.focus >= 3 {
                            let is_edit = f.edit_index.is_some();
                            let sql = if is_edit {
                                Self::generate_alter_column_sql(&self.schemas[schema_idx].objects[object_idx], &f)
                            } else {
                                Self::generate_add_column_sql(&self.schemas[schema_idx].objects[object_idx], &f)
                            };
                            let msg = if is_edit { "Alter column?" } else { "Add column?" };
                            e.column_form = None;
                            self.confirm_message = Some(msg.to_string());
                            self.confirm_sql = Some(sql);
                        } else {
                            f.focus += 1;
                            if f.focus == 1 { f.refilter_types(); }
                        }
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down if is_type_focused => {
                if let Some(ref mut e) = self.property_editor {
                    if let Some(ref mut f) = e.column_form {
                        if !f.type_filtered.is_empty() {
                            f.type_selected = (f.type_selected + 1).min(f.type_filtered.len().saturating_sub(1));
                        }
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up if is_type_focused => {
                if let Some(ref mut e) = self.property_editor {
                    if let Some(ref mut f) = e.column_form {
                        f.type_selected = f.type_selected.saturating_sub(1);
                    }
                }
            }
            KeyCode::Char(' ') => {
                if let Some(ref mut e) = self.property_editor {
                    if let Some(ref mut f) = e.column_form {
                        if f.focus == 2 {
                            f.is_nullable = !f.is_nullable;
                        } else if f.focus != 1 {
                            match f.focus {
                                0 => f.name.push(' '),
                                3 => f.default_value.push(' '),
                                _ => {}
                            }
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut e) = self.property_editor {
                    if let Some(ref mut f) = e.column_form {
                        match f.focus {
                            0 => { f.name.pop(); }
                            1 => { f.data_type.pop(); f.refilter_types(); }
                            3 => { f.default_value.pop(); }
                            _ => {}
                        }
                    }
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut e) = self.property_editor {
                    if let Some(ref mut f) = e.column_form {
                        match f.focus {
                            0 => f.name.push(c),
                            1 => { f.data_type.push(c); f.refilter_types(); },
                            3 => f.default_value.push(c),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn drop_property_item(&mut self, schema_idx: usize, object_idx: usize) {
        let (section, selected) = match self.property_editor.as_ref() {
            Some(e) => (e.section, e.selected),
            None => return,
        };
        let obj = &self.schemas[schema_idx].objects[object_idx];
        let (sql, msg) = match section {
            PropSection::Columns => {
                if selected < obj.columns.len() {
                    let col = &obj.columns[selected];
                    (format!("ALTER TABLE \"{}\".\"{}\" DROP COLUMN \"{}\";", obj.schema_name, obj.name, col.name),
                     format!("Drop column '{}'?", col.name))
                } else { return; }
            }
            PropSection::Indexes => {
                if selected < obj.indexes.len() {
                    let idx = &obj.indexes[selected];
                    (format!("DROP INDEX \"{}\".\"{}\";", obj.schema_name, idx.name),
                     format!("Drop index '{}'?", idx.name))
                } else { return; }
            }
            PropSection::Constraints => {
                if selected < obj.constraints.len() {
                    let con = &obj.constraints[selected];
                    (format!("ALTER TABLE \"{}\".\"{}\" DROP CONSTRAINT \"{}\";", obj.schema_name, obj.name, con.name),
                     format!("Drop constraint '{}'?", con.name))
                } else { return; }
            }
            _ => return,
        };
        self.confirm_message = Some(msg);
        self.confirm_sql = Some(sql);
    }

    fn resolve_type(raw: &str) -> &str {
        match raw {
            "serial" => "integer",
            "bigserial" => "bigint",
            "smallserial" => "smallint",
            other => other,
        }
    }

    pub fn generate_add_column_sql(obj: &DbObject, form: &ColumnForm) -> String {
        let data_type = Self::resolve_type(&form.data_type);
        let nullable = if form.is_nullable { "" } else { " NOT NULL" };
        let default = if form.default_value.is_empty() {
            String::new()
        } else {
            format!(" DEFAULT {}", form.default_value)
        };
        format!(
            "ALTER TABLE \"{}\".\"{}\" ADD COLUMN \"{}\" {}{}{};",
            obj.schema_name, obj.name, form.name, data_type, default, nullable
        )
    }

    pub fn generate_alter_column_sql(obj: &DbObject, form: &ColumnForm) -> String {
        let data_type = Self::resolve_type(&form.data_type);
        format!(
            "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN \"{}\" TYPE {};",
            obj.schema_name, obj.name, form.name, data_type
        )
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::{KeyCode, KeyModifiers};

        self.notification = None;

        // Connection form takes priority
        if self.show_connection_form {
            self.handle_form_key(key);
            return Action::None;
        }

        // Role form
        if self.show_role_form {
            self.handle_role_form_key(key);
            return Action::None;
        }

        // Confirmation dialog
        if self.confirm_message.is_some() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    return Action::ExecuteConfirm;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    return Action::CancelConfirm;
                }
                _ => return Action::None,
            }
        }

        // Connection panel
        if self.show_connection_panel {
            if matches!(key.code, KeyCode::Enter) {
                if let Some(sel) = self.connection_state.selected() {
                    if sel < self.connections.profiles.len() {
                        self.show_connection_panel = false;
                        self.current_page = Page::Home;
                        return Action::Connect(sel);
                    }
                }
            }
            self.handle_connection_list_key(key);
            return Action::None;
        }

        // Page-specific key handling
        match self.current_page {
            Page::Roles => { return self.handle_roles_page_key(key); }
            Page::Databases => { return self.handle_databases_page_key(key); }
            Page::Functions => { return self.handle_functions_page_key(key); }
            Page::Extensions => { return self.handle_extensions_page_key(key); }
            Page::Settings => { return self.handle_settings_page_key(key); }
            Page::Replication => { return self.handle_replication_page_key(key); }
            Page::Dashboard => { return self.handle_dashboard_page_key(key); }
            Page::Search => { return self.handle_search_page_key(key); }
            Page::Bookmarks => { return self.handle_bookmarks_page_key(key); }
            Page::ConnectionManager => { return self.handle_connection_manager_page_key(key); }
            Page::Help => { return Action::None; }
            Page::Home => {} // falls through to global keys + mode dispatch
        }

        // Global keys
        match (key.code, key.modifiers) {
            (KeyCode::Enter, mods) if mods.contains(KeyModifiers::ALT) => {
                let sql = self.current_tab().lines().join("\n");
                if !sql.trim().is_empty() {
                    return Action::ExecuteQuery(sql);
                }
                return Action::None;
            }
            (KeyCode::F(5), _) => {
                let sql = self.current_tab().lines().join("\n");
                if !sql.trim().is_empty() {
                    return Action::ExecuteExplain(sql);
                }
                return Action::None;
            }
            (KeyCode::Char('j'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                let sql = self.current_tab().lines().join("\n");
                if !sql.trim().is_empty() {
                    return Action::ExecuteQuery(sql);
                }
                return Action::None;
            }
            (KeyCode::Char('r'), _) if self.mode == Mode::Normal && self.db.is_some() => {
                return Action::RefreshSchema;
            }
            (KeyCode::Char('E'), _) if self.mode == Mode::Normal && self.focus == Focus::Schema => {
                let sel = self.schema_tree.list_state.selected();
                if let Some(item) = sel.and_then(|s| self.schema_tree.flat_items.get(s)) {
                    match item {
                        SchemaTreeItem::ObjectRow(si, oi) => self.open_property_editor(*si, *oi),
                        SchemaTreeItem::SequenceRow(si, oi) => self.open_property_editor(*si, *oi),
                        _ => {}
                    }
                }
                return Action::None;
            }
            (KeyCode::Char('D'), _) if self.mode == Mode::Normal && self.focus == Focus::Schema => {
                let sel = self.schema_tree.list_state.selected();
                if let Some(item) = sel.and_then(|s| self.schema_tree.flat_items.get(s)) {
                    match item {
                        SchemaTreeItem::ObjectRow(si, oi) => return Action::ShowDdl(*si, *oi),
                        SchemaTreeItem::SequenceRow(si, oi) => return Action::ShowDdl(*si, *oi),
                        _ => {}
                    }
                }
                return Action::None;
            }
            (KeyCode::Char('d'), _) if self.mode == Mode::Normal && self.focus == Focus::Schema => {
                let sel = self.schema_tree.list_state.selected();
                if let Some(item) = sel.and_then(|s| self.schema_tree.flat_items.get(s)) {
                    let (si, oi) = match item {
                        SchemaTreeItem::ObjectRow(si, oi) => (*si, *oi),
                        SchemaTreeItem::SequenceRow(si, oi) => (*si, *oi),
                        _ => return Action::None,
                    };
                    if si < self.schemas.len() && oi < self.schemas[si].objects.len() {
                        let obj = &self.schemas[si].objects[oi];
                        let name = format!("\"{}\".\"{}\"", self.schemas[si].name, obj.name);
                        let obj_type = match obj.obj_type {
                            crate::db::DbObjectType::Table => "TABLE",
                            crate::db::DbObjectType::View => "VIEW",
                            crate::db::DbObjectType::Sequence => "SEQUENCE",
                            _ => "TABLE",
                        };
                        self.confirm_message = Some(format!("DROP {} {}?", obj_type, name));
                        self.confirm_sql = Some(format!("DROP {} {};", obj_type, name));
                    }
                }
                return Action::None;
            }
            (KeyCode::Char('t'), _) if self.mode == Mode::Normal && self.focus == Focus::Schema => {
                let sel = self.schema_tree.list_state.selected();
                if let Some(item) = sel.and_then(|s| self.schema_tree.flat_items.get(s)) {
                    if let SchemaTreeItem::ObjectRow(si, oi) = item {
                        if *si < self.schemas.len() && *oi < self.schemas[*si].objects.len() {
                            let obj = &self.schemas[*si].objects[*oi];
                            if obj.obj_type == crate::db::DbObjectType::Table {
                                let name = format!("\"{}\".\"{}\"", self.schemas[*si].name, obj.name);
                                self.confirm_message = Some(format!("TRUNCATE {}?", name));
                                self.confirm_sql = Some(format!("TRUNCATE TABLE {};", name));
                            }
                        }
                    }
                }
                return Action::None;
            }
            (KeyCode::Char('v'), _) if self.mode == Mode::Normal && self.focus == Focus::Schema => {
                let sel = self.schema_tree.list_state.selected();
                if let Some(item) = sel.and_then(|s| self.schema_tree.flat_items.get(s)) {
                    if let SchemaTreeItem::ObjectRow(si, oi) = item {
                        if *si < self.schemas.len() && *oi < self.schemas[*si].objects.len() {
                            let obj = &self.schemas[*si].objects[*oi];
                            if obj.obj_type == crate::db::DbObjectType::Table {
                                let name = format!("\"{}\".\"{}\"", self.schemas[*si].name, obj.name);
                                self.confirm_message = Some(format!("VACUUM {}?", name));
                                self.confirm_sql = Some(format!("VACUUM {};", name));
                            }
                        }
                    }
                }
                return Action::None;
            }
            (KeyCode::Char('n'), _) if self.mode == Mode::Normal && self.browse_schema_idx.is_some() => {
                return Action::NextPage;
            }
            (KeyCode::Char('p'), _) if self.mode == Mode::Normal && self.browse_schema_idx.is_some() => {
                return Action::PrevPage;
            }
            (KeyCode::Char('c'), _) if self.mode == Mode::Normal => {
                self.show_connection_panel = true;
                self.focus = Focus::ConnectionList;
                return Action::None;
            }
            (KeyCode::Char('t'), mods) if mods == KeyModifiers::CONTROL => {
                self.new_tab();
                if self.focus != Focus::QueryInput {
                    self.focus = Focus::QueryInput;
                }
                return Action::None;
            }
            (KeyCode::Char('w'), mods) if mods == KeyModifiers::CONTROL => {
                self.close_tab();
                return Action::None;
            }
            (KeyCode::Char(']'), _) if self.mode == Mode::Normal && self.focus == Focus::QueryInput => {
                self.next_tab();
                return Action::None;
            }
            (KeyCode::Char('['), _) if self.mode == Mode::Normal && self.focus == Focus::QueryInput => {
                self.prev_tab();
                return Action::None;
            }
            _ => {}
        }

        // Mode dispatch — modes can now return an Action
        let mode_action = match self.mode {
            Mode::Normal => crate::modes::normal::handle(self, key),
            Mode::Insert => { crate::modes::insert::handle(self, key); None }
            Mode::Visual => { crate::modes::visual::handle(self, key); None }
            Mode::PropertyEditor => { self.handle_property_editor_key(key); None }
        };

        mode_action.unwrap_or_else(|| {
            if self.quit { Action::Quit } else { Action::None }
        })
    }

    fn handle_connection_list_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                if self.connected {
                    self.show_connection_panel = false;
                    self.focus = Focus::Schema;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.connections.profiles.len().saturating_sub(1);
                let sel = self.connection_state.selected().unwrap_or(0);
                self.connection_state.select(Some((sel + 1).min(max)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let sel = self.connection_state.selected().unwrap_or(0);
                self.connection_state.select(Some(sel.saturating_sub(1)));
            }
            KeyCode::Enter => {
                // handled in handle_key above
            }
            KeyCode::Char('n') => self.open_new_connection_form(),
            KeyCode::Char('e') => {
                if let Some(sel) = self.connection_state.selected() {
                    if sel < self.connections.profiles.len() {
                        self.open_edit_connection_form(sel);
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(sel) = self.connection_state.selected() {
                    if sel < self.connections.profiles.len() {
                        self.connections.remove_profile(sel);
                        let max = self.connections.profiles.len().saturating_sub(1);
                        self.connection_state.select(
                            if self.connections.profiles.is_empty() { None } else { Some(sel.min(max)) }
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_role_form_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.show_role_form = false;
            }
            KeyCode::Enter => {
                if self.role_form.focus == 7 {
                    self.save_role_form();
                } else {
                    self.role_form.focus = (self.role_form.focus + 1).min(7);
                }
            }
            KeyCode::Tab => {
                self.role_form.focus = (self.role_form.focus + 1) % 8;
            }
            KeyCode::Char(' ') => {
                match self.role_form.focus {
                    1 => self.role_form.login = !self.role_form.login,
                    2 => self.role_form.superuser = !self.role_form.superuser,
                    3 => self.role_form.createdb = !self.role_form.createdb,
                    4 => self.role_form.createrole = !self.role_form.createrole,
                    5 => self.role_form.replication = !self.role_form.replication,
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                match self.role_form.focus {
                    0 => self.role_form.name.push(c),
                    6 => self.role_form.password.push(c),
                    7 => self.role_form.conn_limit.push(c),
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                match self.role_form.focus {
                    0 => { self.role_form.name.pop(); }
                    6 => { self.role_form.password.pop(); }
                    7 => { self.role_form.conn_limit.pop(); }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_form_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => {
                self.show_connection_form = false;
                self.form_password.clear();
                self.focus = Focus::ConnectionList;
            }
            KeyCode::Enter => {
                if self.form_focus == 5 {
                    self.save_connection_form();
                } else {
                    self.form_focus = (self.form_focus + 1).min(5);
                }
            }
            KeyCode::Tab => {
                self.form_focus = (self.form_focus + 1) % 6;
            }
            KeyCode::Insert if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.paste_to_form_field();
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.paste_to_form_field();
            }
            KeyCode::Char(c) => {
                let field = match self.form_focus {
                    0 => &mut self.form_name,
                    1 => &mut self.form_host,
                    2 => &mut self.form_port,
                    3 => &mut self.form_user,
                    4 => &mut self.form_password,
                    5 => &mut self.form_dbname,
                    _ => return,
                };
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = match self.form_focus {
                    0 => &mut self.form_name,
                    1 => &mut self.form_host,
                    2 => &mut self.form_port,
                    3 => &mut self.form_user,
                    4 => &mut self.form_password,
                    5 => &mut self.form_dbname,
                    _ => return,
                };
                field.pop();
            }
            _ => {}
        }
    }

    fn paste_to_form_field(&mut self) {
        if let Ok(mut ctx) = arboard::Clipboard::new() {
            if let Ok(text) = ctx.get_text() {
                let field = match self.form_focus {
                    0 => &mut self.form_name,
                    1 => &mut self.form_host,
                    2 => &mut self.form_port,
                    3 => &mut self.form_user,
                    4 => &mut self.form_password,
                    5 => &mut self.form_dbname,
                    _ => return,
                };
                field.push_str(&text);
            }
        }
    }
}
