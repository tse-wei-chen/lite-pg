use crate::app::CompletionState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, state: &CompletionState) {
    if !state.visible || state.items.is_empty() {
        return;
    }

    let max_h = 10usize.min(state.items.len());
    let popup_h = max_h as u16 + 2; // borders
    let popup_w = state
        .items
        .iter()
        .map(|s| s.len() as u16)
        .max()
        .unwrap_or(20)
        .max(25)
        + 4;

    let cursor_x = (area.x + state.replacement_start as u16).min(area.right().saturating_sub(5));
    let px = cursor_x.min(area.right().saturating_sub(popup_w));
    let py = if area.y >= popup_h + 1 {
        area.y.saturating_sub(popup_h + 1)
    } else {
        area.y + area.height.saturating_sub(popup_h + 1)
    };

    let popup = Rect {
        x: px,
        y: py,
        width: popup_w.min(area.width),
        height: popup_h,
    };

    let items: Vec<ListItem> = state
        .items
        .iter()
        .take(max_h)
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(item, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Completions")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(ratatui::widgets::Clear, popup);
    f.render_widget(list, popup);
}
