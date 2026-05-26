use crate::db::roles::RoleInfo;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    roles: &[RoleInfo],
    state: &mut ListState,
    show_confirm: Option<&str>,
) {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(if show_confirm.is_some() { 3 } else { 0 }),
        ])
        .split(area);

    let items: Vec<ListItem> = if roles.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            " No roles found",
            Style::default().fg(Color::Gray),
        )))]
    } else {
        let mut rows = Vec::new();
        let header = Line::from(vec![
            Span::styled(
                format!(" {:<24}", "Role"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>4}", "S"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>4}", "L"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>4}", "D"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>4}", "R"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>6}", "Conn"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>6}", "Active"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  Member Of",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        rows.push(ListItem::new(header));

        for role in roles {
            let member_str = if role.member_of.is_empty() {
                String::new()
            } else {
                role.member_of.join(", ")
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<24}", role.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>4}", if role.superuser { "Y" } else { "" }),
                    if role.superuser {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!("{:>4}", if role.login { "Y" } else { "" }),
                    if role.login {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!("{:>4}", if role.create_db { "Y" } else { "" }),
                    if role.create_db {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!("{:>4}", if role.replication { "Y" } else { "" }),
                    if role.replication {
                        Style::default().fg(Color::Magenta)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!("{:>6}", role.conn_limit),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>6}", role.use_count),
                    if role.use_count > 0 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!("  {}", member_str),
                    Style::default().fg(Color::Gray),
                ),
            ]);
            rows.push(ListItem::new(line));
        }
        rows
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Roles (Esc: close, n: new, d: drop, e: alter) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, chunks[0], state);

    if let Some(msg) = show_confirm {
        let confirm = Paragraph::new(Line::from(Span::styled(
            format!(" {} (y/n): ", msg),
            Style::default().fg(Color::Yellow),
        )))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        f.render_widget(confirm, chunks[1]);
    }
}
