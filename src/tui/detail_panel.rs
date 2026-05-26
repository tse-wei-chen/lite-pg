use crate::db::DbObject;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    obj: &DbObject,
    state: &mut ListState,
) {
    let mut items: Vec<ListItem> = Vec::new();

    // Header
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" {} {}", obj.obj_type.icon(), obj.name),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))));
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Schema: {}", obj.schema_name),
        Style::default().fg(Color::Gray),
    ))));
    if let Some(ref owner) = obj.owner {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Owner: {}", owner),
            Style::default().fg(Color::Gray),
        ))));
    }

    if let Some(ref desc) = obj.description {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Comment: {}", desc),
            Style::default().fg(Color::Gray),
        ))));
    }
    if let Some(cnt) = obj.row_count {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Rows: ~{}", cnt),
            Style::default().fg(Color::Gray),
        ))));
    }
    if let Some(sz) = obj.size_bytes {
        let human = if sz > 1_073_741_824 {
            format!("{:.1} GB", sz as f64 / 1_073_741_824.0)
        } else if sz > 1_048_576 {
            format!("{:.1} MB", sz as f64 / 1_048_576.0)
        } else if sz > 1024 {
            format!("{:.1} KB", sz as f64 / 1024.0)
        } else {
            format!("{} B", sz)
        };
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Size: {}", human),
            Style::default().fg(Color::Gray),
        ))));
    }

    // Columns
    if !obj.columns.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " Columns:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))));
        for col in &obj.columns {
            let pk = if col.is_primary { " PK" } else { "" };
            let nullable = if col.is_nullable { "" } else { " NOT NULL" };
            let def = col
                .default_value
                .as_ref()
                .map(|d| format!(" DEFAULT {}", d))
                .unwrap_or_default();
            items.push(ListItem::new(Line::from(Span::styled(
                format!(
                    "   {} : {}{}{}{}",
                    col.name, col.data_type, nullable, def, pk
                ),
                Style::default().fg(Color::White),
            ))));
        }
    }

    // Indexes
    if !obj.indexes.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " Indexes:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))));
        for idx in &obj.indexes {
            let uniq = if idx.is_unique { " UNIQUE" } else { "" };
            items.push(ListItem::new(Line::from(Span::styled(
                format!("   {} [{}{}]", idx.name, idx.index_type, uniq),
                Style::default().fg(Color::White),
            ))));
        }
    }

    // Triggers
    if !obj.triggers.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " Triggers:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))));
        for trg in &obj.triggers {
            items.push(ListItem::new(Line::from(Span::styled(
                format!(
                    "   {} ({} {} {})",
                    trg.name, trg.timing, trg.event, trg.level
                ),
                Style::default().fg(Color::White),
            ))));
        }
    }

    // Constraints
    if !obj.constraints.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " Constraints:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))));
        for con in &obj.constraints {
            let ref_info = if con.constraint_type == "FOREIGN KEY" {
                con.referenced_table
                    .as_ref()
                    .map(|t| format!(" → {}", t))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            items.push(ListItem::new(Line::from(Span::styled(
                format!("   {} ({}){}", con.name, con.constraint_type, ref_info),
                Style::default().fg(Color::White),
            ))));
        }
    }

    // DDL hint
    items.push(ListItem::new(Line::from(Span::styled(
        " [Shift+D] View DDL",
        Style::default().fg(Color::Gray),
    ))));

    let list = List::new(items)
        .block(Block::default().title(" Object Detail ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, area, state);
}

use crate::db::replication::{PublicationInfo, SubscriptionInfo};

pub fn render_replication_pub(
    f: &mut Frame,
    area: Rect,
    pubs: &[PublicationInfo],
    state: &mut ListState,
) {
    let mut items: Vec<ListItem> = Vec::new();
    if pubs.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No publications",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header = Line::from(vec![
            Span::styled(
                format!(" {:<20}", "Publication"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<14}", "Owner"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "All Tables",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " I/U/D/T",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]);
        items.push(ListItem::new(header));

        for p in pubs {
            let line = Line::from(vec![
                Span::styled(format!(" {:<20}", p.name), Style::default().fg(Color::White)),
                Span::styled(format!("{:<14}", p.owner), Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:>5}", if p.all_tables { "ALL" } else { "sel" }),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!(" {}/{}/{}/{}",
                        if p.publish_insert { "I" } else { "-" },
                        if p.publish_update { "U" } else { "-" },
                        if p.publish_delete { "D" } else { "-" },
                        if p.publish_truncate { "T" } else { "-" },
                    ),
                    Style::default().fg(Color::Magenta),
                ),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(Block::default().title(" Publications (h/l: tabs, Esc: close) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}

pub fn render_replication_sub(
    f: &mut Frame,
    area: Rect,
    subs: &[SubscriptionInfo],
    state: &mut ListState,
) {
    let mut items: Vec<ListItem> = Vec::new();
    if subs.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No subscriptions",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header = Line::from(vec![
            Span::styled(
                format!(" {:<20}", "Subscription"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<14}", "Owner"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<6}", "Active"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<15}", "Publication"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]);
        items.push(ListItem::new(header));

        for s in subs {
            let line = Line::from(vec![
                Span::styled(format!(" {:<20}", s.name), Style::default().fg(Color::White)),
                Span::styled(format!("{:<14}", s.owner), Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:<6}", if s.enabled { "YES" } else { "NO" }),
                    if s.enabled {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
                Span::styled(
                    format!("{:<15}", s.publication),
                    Style::default().fg(Color::Magenta),
                ),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(Block::default().title(" Subscriptions (h/l: tabs, Esc: close) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}
