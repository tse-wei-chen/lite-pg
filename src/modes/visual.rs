use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('v') => {
            app.mode = Mode::Normal;
            app.visual_anchor = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.results.rows.is_empty() {
                let sel = app.results_state.selected().unwrap_or(0);
                let next = (sel + 1).min(app.results.rows.len().saturating_sub(1));
                app.results_state.select(Some(next));
                let anchor = app.visual_anchor.unwrap_or(sel);
                app.visual_anchor = Some(anchor);
                app.update_visual_selection(anchor, next);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.results.rows.is_empty() {
                let sel = app.results_state.selected().unwrap_or(0);
                let prev = sel.saturating_sub(1);
                app.results_state.select(Some(prev));
                let anchor = app.visual_anchor.unwrap_or(sel);
                app.visual_anchor = Some(anchor);
                app.update_visual_selection(anchor, prev);
            }
        }
        KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.copy_markdown();
        }
        _ => {}
    }
}
