use crate::db::settings::SettingInfo;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    settings: &[SettingInfo],
    state: &mut ListState,
    filter_category: &Option<String>,
) {
    let filtered: Vec<&SettingInfo> = if let Some(ref cat) = filter_category {
        settings
            .iter()
            .filter(|s| s.category == *cat)
            .collect()
    } else {
        settings.iter().collect()
    };

    let mut items: Vec<ListItem> = Vec::new();
    if filtered.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No settings found",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header = Line::from(vec![
            Span::styled(
                format!(" {:<36}", "Name"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<16}", "Value"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<16}", "Reset"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Context", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]);
        items.push(ListItem::new(header));

        for s in &filtered {
            let line = Line::from(vec![
                Span::styled(format!(" {:<36}", s.name), Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:<16}", s.value),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("{:<16}", s.reset_value.as_deref().unwrap_or("")),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!(" {}", s.context),
                    Style::default().fg(Color::Yellow),
                ),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let title = match filter_category {
        Some(ref cat) => format!(" Settings: {} (Esc: close) ", cat),
        None => " Settings (Esc: close) ".to_string(),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}
