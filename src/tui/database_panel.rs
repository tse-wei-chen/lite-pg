use crate::db::databases::DatabaseInfo;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    databases: &[DatabaseInfo],
    state: &mut ListState,
) {
    let items: Vec<ListItem> = if databases.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            " No databases found",
            Style::default().fg(Color::Gray),
        )))]
    } else {
        let mut rows = Vec::new();
        let header = Line::from(vec![
            Span::styled(
                format!(" {:<20}", "Database"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<14}", "Owner"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>10}", "Size"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {:<8}", "Encoding"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {:<12}", "Tablespace"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " ConnLimit",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        rows.push(ListItem::new(header));

        for db in databases {
            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<20}", db.name),
                    if db.is_template {
                        Style::default().fg(Color::Gray)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                Span::styled(
                    format!("{:<14}", db.owner),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:>10}", db.size),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!(" {:<8}", db.encoding),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(" {:<12}", db.tablespace),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!(" {}", db.connection_limit),
                    Style::default().fg(Color::White),
                ),
            ]);
            rows.push(ListItem::new(line));
        }
        rows
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Databases (Esc: close, Enter: connect, n: create, d: drop) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}
