use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        _ => {
            app.query_input.input(key);
        }
    }
}
