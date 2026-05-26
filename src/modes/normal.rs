use crate::app::{Action, App, Focus, Mode, SchemaTreeItem};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn handle_schema_enter(app: &mut App) -> Option<Action> {
    if let Some(sel) = app.schema_tree.list_state.selected() {
        if let Some(item) = app.schema_tree.flat_items.get(sel).cloned() {
            match item {
                SchemaTreeItem::SchemaHeader(si) => {
                    if si < app.schema_tree.schema_expanded.len() {
                        app.schema_tree.schema_expanded[si] =
                            !app.schema_tree.schema_expanded[si];
                    }
                    None
                }
                SchemaTreeItem::ObjectRow(si, oi) => {
                    if si < app.schemas.len() && oi < app.schemas[si].objects.len() {
                        let obj = &app.schemas[si].objects[oi];
                        if !obj.detail_loaded {
                            Some(Action::ShowDetail(si, oi))
                        } else {
                            app.schema_tree.toggle_object(si, oi);
                            None
                        }
                    } else {
                        None
                    }
                }
                SchemaTreeItem::ColumnSection(si, oi) => {
                    app.schema_tree.toggle_section(si, oi, 0);
                    None
                }
                SchemaTreeItem::IndexSection(si, oi) => {
                    app.schema_tree.toggle_section(si, oi, 1);
                    None
                }
                SchemaTreeItem::TriggerSection(si, oi) => {
                    app.schema_tree.toggle_section(si, oi, 2);
                    None
                }
                SchemaTreeItem::ConstraintSection(si, oi) => {
                    app.schema_tree.toggle_section(si, oi, 3);
                    None
                }
                SchemaTreeItem::SequenceSection(si, oi) => {
                    app.schema_tree.toggle_section(si, oi, 4);
                    None
                }
                SchemaTreeItem::ColumnRow(..)
                | SchemaTreeItem::IndexRow(..)
                | SchemaTreeItem::TriggerRow(..)
                | SchemaTreeItem::ConstraintRow(..)
                | SchemaTreeItem::SequenceRow(..) => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('i') => {
            match app.focus {
                Focus::QueryInput => app.mode = Mode::Insert,
                Focus::Schema => {
                    if let Some(sel) = app.schema_tree.list_state.selected() {
                        if let Some(item) = app.schema_tree.flat_items.get(sel) {
                            if let SchemaTreeItem::ObjectRow(si, oi) = item {
                                if *si < app.schemas.len() && *oi < app.schemas[*si].objects.len() {
                                    return Some(Action::GenerateInsert(*si, *oi));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            None
        }
        KeyCode::Char('v') => {
            if app.focus == Focus::Results && !app.results.rows.is_empty() {
                app.mode = Mode::Visual;
                app.visual_anchor = app.results_state.selected();
            }
            None
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Schema => Focus::QueryInput,
                Focus::QueryInput => Focus::Results,
                Focus::Results => Focus::Schema,
                other => other,
            };
            None
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.focus = match app.focus {
                Focus::Schema => Focus::QueryInput,
                Focus::QueryInput => Focus::Results,
                Focus::Results => Focus::Schema,
                other => other,
            };
            None
        }
        KeyCode::Char('j') | KeyCode::Down => {
            match app.focus {
                Focus::Schema => {
                    let max = app.schema_tree.flat_items.len().saturating_sub(1);
                    if !app.schema_tree.flat_items.is_empty() {
                        let sel = app.schema_tree.list_state.selected().unwrap_or(0);
                        app.schema_tree.list_state.select(Some((sel + 1).min(max)));
                    }
                }
                Focus::Results => {
                    if !app.results.rows.is_empty() {
                        let sel = app.results_state.selected().unwrap_or(0);
                        let max = app.results.rows.len() + 1;
                        app.results_state.select(Some((sel + 1).min(max)));
                    }
                }
                _ => {}
            }
            None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            match app.focus {
                Focus::Schema => {
                    if !app.schema_tree.flat_items.is_empty() {
                        let sel = app.schema_tree.list_state.selected().unwrap_or(0);
                        if sel > 0 {
                            app.schema_tree.list_state.select(Some(sel.saturating_sub(1)));
                        }
                    }
                }
                Focus::Results => {
                    if !app.results.rows.is_empty() {
                        let sel = app.results_state.selected().unwrap_or(0);
                        if sel > 0 {
                            app.results_state.select(Some(sel.saturating_sub(1)));
                        }
                    }
                }
                _ => {}
            }
            None
        }
        KeyCode::Char('h') | KeyCode::Left => {
            if app.focus == Focus::Schema {
                return handle_schema_enter(app);
            }
            if app.focus == Focus::Results {
                app.scroll_h = app.scroll_h.saturating_sub(5);
            }
            None
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if app.focus == Focus::Schema {
                return handle_schema_enter(app);
            }
            if app.focus == Focus::Results {
                app.scroll_h = app.scroll_h.saturating_add(5);
            }
            None
        }
        KeyCode::Enter => {
            if app.focus == Focus::Schema {
                return handle_schema_enter(app);
            }
            None
        }
        KeyCode::Char('o') => {
            if app.focus == Focus::Schema {
                if let Some(sel) = app.schema_tree.list_state.selected() {
                    if let Some(item) = app.schema_tree.flat_items.get(sel) {
                        if let SchemaTreeItem::ObjectRow(si, oi) = item {
                            if *si < app.schemas.len() && *oi < app.schemas[*si].objects.len() {
                                return Some(Action::GenerateSelect(*si, *oi));
                            }
                        }
                    }
                }
            }
            None
        }
        KeyCode::Char('u') => {
            if app.focus == Focus::Schema {
                if let Some(sel) = app.schema_tree.list_state.selected() {
                    if let Some(item) = app.schema_tree.flat_items.get(sel) {
                        if let SchemaTreeItem::ObjectRow(si, oi) = item {
                            if *si < app.schemas.len() && *oi < app.schemas[*si].objects.len() {
                                return Some(Action::GenerateUpdate(*si, *oi));
                            }
                        }
                    }
                }
            }
            None
        }
        KeyCode::Char('/') => {
            app.show_history = !app.show_history;
            if app.show_history {
                app.history_search.clear();
            }
            None
        }
        _ => None,
    }
}
