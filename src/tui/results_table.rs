use crate::db::QueryResult;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    result: &QueryResult,
    state: &mut ListState,
    _scroll_h: u16,
) {
    if let Some(ref err) = result.error {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )))
        .block(Block::default().title(" Error ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
        return;
    }

    if result.columns.is_empty() && result.rows.is_empty() {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            "No results",
            Style::default().fg(Color::Gray),
        )))
        .block(Block::default().title(" Results ").borders(Borders::ALL));
        f.render_widget(paragraph, area);
        return;
    }

    if result.columns.len() == 1 && result.columns[0] == "RESULT" {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            &result.rows[0][0],
            Style::default().fg(Color::Yellow),
        )))
        .block(Block::default().title(" Result ").borders(Borders::ALL));
        f.render_widget(paragraph, area);
        return;
    }

    let mut items: Vec<ListItem> = Vec::new();

    let header: Vec<Span> = result
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let prefix = if i == 0 { "" } else { " │ " };
            Span::styled(
                format!("{prefix}{col}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    items.push(ListItem::new(Line::from(header)));

    let sep_spans: Vec<Span> = result
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let prefix = if i == 0 { "" } else { "─┼─" };
            Span::styled(
                format!("{prefix}{}", "─".repeat(col.len())),
                Style::default().fg(Color::Gray),
            )
        })
        .collect();
    items.push(ListItem::new(Line::from(sep_spans)));

    for row in &result.rows {
        let spans: Vec<Span> = row
            .iter()
            .enumerate()
            .map(|(i, val)| {
                let prefix = if i == 0 { "" } else { " │ " };
                Span::raw(format!("{prefix}{val}"))
            })
            .collect();
        items.push(ListItem::new(Line::from(spans)));
    }

    let list = List::new(items)
        .block(Block::default().title(" Results ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("");

    f.render_stateful_widget(list, area, state);
}
