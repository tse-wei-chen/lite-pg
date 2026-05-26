use crate::bookmarks::Bookmark;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    bookmarks: &[&Bookmark],
    state: &mut ListState,
) {
    let mut items: Vec<ListItem> = Vec::new();
    if bookmarks.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No bookmarks. Press Ctrl+Shift+B to save current query.",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        for bm in bookmarks {
            let first_line = bm.sql.lines().next().unwrap_or("");
            let preview = if first_line.chars().count() > 50 {
                format!("{}...", first_line.chars().take(50).collect::<String>())
            } else {
                first_line.to_string()
            };

            let conn = bm
                .connection_name
                .as_deref()
                .map(|c| format!(" [{}]", c))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<24}", bm.name),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(preview, Style::default().fg(Color::White)),
                Span::styled(conn, Style::default().fg(Color::Gray)),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Bookmarks (Esc: close, Enter: load, d: delete, Ctrl+Shift+B: save) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}
