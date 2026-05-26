use crate::db::extensions::ExtensionInfo;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, exts: &[ExtensionInfo], state: &mut ListState) {
    let mut items: Vec<ListItem> = Vec::new();
    if exts.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No extensions installed",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header = Line::from(vec![
            Span::styled(
                format!(" {:<24}", "Extension"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<12}", "Version"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<14}", "Schema"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Description",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        items.push(ListItem::new(header));

        for ext in exts {
            let desc = ext
                .description
                .as_deref()
                .unwrap_or("")
                .to_string();
            let truncated = if desc.chars().count() > 40 {
                format!("{}...", desc.chars().take(37).collect::<String>())
            } else {
                desc
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<24}", ext.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:<12}", ext.version),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("{:<14}", ext.schema),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(truncated, Style::default().fg(Color::Gray)),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Extensions (Esc: close) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}
