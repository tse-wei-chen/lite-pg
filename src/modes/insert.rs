use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(app: &mut App, key: KeyEvent) {
    // Completion is active — intercept Tab/Enter/Esc/n/p
    if app.completion.visible {
        match key.code {
            KeyCode::Esc => {
                app.completion.visible = false;
                return;
            }
            KeyCode::Tab | KeyCode::Down => {
                let max = app.completion.items.len().saturating_sub(1);
                app.completion.selected = (app.completion.selected + 1).min(max);
                return;
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let max = app.completion.items.len().saturating_sub(1);
                app.completion.selected = (app.completion.selected + 1).min(max);
                return;
            }
            KeyCode::BackTab | KeyCode::Up => {
                app.completion.selected = app.completion.selected.saturating_sub(1);
                return;
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.completion.selected = app.completion.selected.saturating_sub(1);
                return;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.accept_completion();
                return;
            }
            _ => {
                // Any other key: forward to textarea then update completions
                app.current_tab_mut().input(key);
                app.update_completions();
                return;
            }
        }
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.completion.visible = false;
        }
        KeyCode::Tab => {
            // Tab triggers completion if cursor is on a word
            app.current_tab_mut().input(key);
            app.update_completions();
            // If nothing found, move cursor right (textarea default)
        }
        _ => {
            app.current_tab_mut().input(key);
            app.update_completions();
        }
    }
}
