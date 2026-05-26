use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .connections
        .profiles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_active = app.connections.active_index == Some(i);
            let icon = if is_active { "●" } else { "○" };
            let style = if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(
                format!(" {} {}@{}:{}/{}", icon, p.user, p.host, p.port, p.dbname),
                style,
            )))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Connections ([n]ew [e]dit [d]elete Enter=connect) ")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, area, &mut app.connection_state);
}
