use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;
use tui_textarea::TextArea;

pub fn render(f: &mut Frame, area: Rect, textarea: &mut TextArea, focus: bool) {
    let border_color = if focus { Color::Cyan } else { Color::DarkGray };
    let block = Block::default()
        .title(" SQL (Alt+Enter to execute) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    textarea.set_block(block);
    f.render_widget(&*textarea, area);
}
