use crate::app::{App, Focus, Mode};
use crate::db::TableInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn schema_flat_count(tables: &[TableInfo], expanded: &[bool]) -> usize {
    tables
        .iter()
        .enumerate()
        .map(|(i, t)| {
            1 + if *expanded.get(i).unwrap_or(&false) {
                t.columns.len()
            } else {
                0
            }
        })
        .sum()
}

fn flat_to_table(flat: usize, tables: &[TableInfo], expanded: &[bool]) -> Option<(usize, bool)> {
    let mut pos = 0;
    for (ti, table) in tables.iter().enumerate() {
        if pos == flat {
            return Some((ti, true));
        }
        pos += 1;
        if *expanded.get(ti).unwrap_or(&false) {
            let col_count = table.columns.len();
            if flat < pos + col_count {
                return Some((ti, false));
            }
            pos += col_count;
        }
    }
    None
}

pub fn handle(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('i') => {
            app.mode = Mode::Insert;
        }
        KeyCode::Char('v') => {
            if app.focus == Focus::Results {
                if !app.results.rows.is_empty() {
                    app.mode = Mode::Visual;
                    app.visual_anchor = app.results_state.selected();
                }
            }
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Schema => Focus::QueryInput,
                Focus::QueryInput => Focus::Results,
                Focus::Results => Focus::Schema,
            };
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.focus = match app.focus {
                Focus::Schema => Focus::QueryInput,
                Focus::QueryInput => Focus::Results,
                Focus::Results => Focus::Schema,
            };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            match app.focus {
                Focus::Schema => {
                    let max = schema_flat_count(&app.tables, &app.schema_expanded)
                        .saturating_sub(1);
                    match app.schema_state.selected() {
                        None => {
                            if !app.tables.is_empty() {
                                app.schema_state.select(Some(0));
                            }
                        }
                        Some(sel) => {
                            app.schema_state
                                .select(Some((sel + 1).min(max)));
                        }
                    }
                }
                Focus::Results => {
                    if !app.results.rows.is_empty() {
                        match app.results_state.selected() {
                            None => app.results_state.select(Some(0)),
                            Some(sel) => {
                                let max = app.results.rows.len().saturating_sub(1);
                                app.results_state
                                    .select(Some((sel + 1).min(max)));
                            }
                        }
                    }
                }
                Focus::QueryInput => {}
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            match app.focus {
                Focus::Schema => {
                    match app.schema_state.selected() {
                        None => {
                            if !app.tables.is_empty() {
                                app.schema_state.select(Some(0));
                            }
                        }
                        Some(sel) => {
                            app.schema_state
                                .select(Some(sel.saturating_sub(1)));
                        }
                    }
                }
                Focus::Results => {
                    if !app.results.rows.is_empty() {
                        match app.results_state.selected() {
                            None => app.results_state.select(Some(0)),
                            Some(sel) => {
                                app.results_state
                                    .select(Some(sel.saturating_sub(1)));
                            }
                        }
                    }
                }
                Focus::QueryInput => {}
            }
        }
        KeyCode::Char('h') | KeyCode::Left => {
            if app.focus == Focus::Results {
                app.scroll_h = app.scroll_h.saturating_sub(5);
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if app.focus == Focus::Results {
                app.scroll_h = app.scroll_h.saturating_add(5);
            }
        }
        KeyCode::Enter => {
            if app.focus == Focus::Schema {
                if let Some(sel) = app.schema_state.selected() {
                    if let Some((ti, is_header)) =
                        flat_to_table(sel, &app.tables, &app.schema_expanded)
                    {
                        if is_header {
                            if ti < app.schema_expanded.len() {
                                app.schema_expanded[ti] = !app.schema_expanded[ti];
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('/') => {
            app.show_history = !app.show_history;
            if app.show_history {
                app.history_search.clear();
            }
        }
        KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.copy_markdown();
        }
        _ => {}
    }
}
