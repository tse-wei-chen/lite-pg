use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;
use tui_textarea::TextArea;

pub fn render(f: &mut Frame, area: Rect, textarea: &mut TextArea) {
    let block = Block::default()
        .title(" SQL (Ctrl+Enter to execute) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    textarea.set_block(block);
    f.render_widget(&*textarea, area);
}
