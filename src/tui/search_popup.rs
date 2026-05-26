use crate::db::search::SearchResult;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    query: &str,
    results: &[SearchResult],
    state: &mut ListState,
) {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(area);

    let input = Paragraph::new(Line::from(vec![
        Span::styled(" Search: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(query, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]))
    .block(
        Block::default()
            .title(" Global Object Search (Ctrl+F) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(input, chunks[0]);

    let mut items: Vec<ListItem> = Vec::new();
    if results.is_empty() && !query.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No results found",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header = Line::from(vec![
            Span::styled(
                format!(" {:<12}", "Type"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Object", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]);
        items.push(ListItem::new(header));

        for r in results {
            let type_color = match r.object_type.as_str() {
                "TABLE" => Color::Green,
                "VIEW" => Color::Yellow,
                "COLUMN" => Color::Magenta,
                "FUNCTION" => Color::Blue,
                _ => Color::White,
            };
            let desc = r
                .description
                .as_deref()
                .map(|d| format!(" ({})", d))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<12}", r.object_type),
                    Style::default().fg(type_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}.{}", r.schema, r.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(desc, Style::default().fg(Color::Gray)),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, chunks[1], state);
}
