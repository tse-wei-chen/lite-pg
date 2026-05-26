use crate::db::statistics::{ActiveQuery, DbStatEntry, ServerOverview, TableStatEntry};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap};
use ratatui::Frame;

const TAB_TITLES: &[&str] = &[" Overview ", " Databases ", " Active Queries ", " Tables "];

pub fn render(
    f: &mut Frame,
    area: Rect,
    tab: usize,
    overview: &Option<ServerOverview>,
    db_stats: &[DbStatEntry],
    active_queries: &[ActiveQuery],
    table_stats: &[TableStatEntry],
    list_state: &mut ListState,
    error: &Option<String>,
) {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(area);

    let tabs = Tabs::new(
        TAB_TITLES
            .iter()
            .enumerate()
            .map(|(i, title)| {
                let style = if i == tab {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                Line::from(Span::styled(*title, style))
            })
            .collect::<Vec<_>>(),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Dashboard (F2: close) ")
            .title_alignment(Alignment::Left)
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(tabs, chunks[0]);

    if let Some(ref err) = error {
        let err_para = Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red)))
        .wrap(Wrap { trim: false });
        let err_area = ratatui::layout::Rect {
            x: chunks[1].x,
            y: chunks[1].y,
            width: chunks[1].width,
            height: chunks[1].height.min(3),
        };
        f.render_widget(err_para, err_area);
        return;
    }

    match tab {
        0 => render_overview(f, chunks[1], overview),
        1 => render_db_stats(f, chunks[1], db_stats, list_state),
        2 => render_active_queries(f, chunks[1], active_queries, list_state),
        3 => render_table_stats(f, chunks[1], table_stats, list_state),
        _ => {}
    }
}

fn render_overview(f: &mut Frame, area: Rect, overview: &Option<ServerOverview>) {
    let mut items = Vec::new();

    match overview {
        Some(ov) => {
            items.push(create_info_line("Version", &ov.version, Color::Green));
            items.push(create_info_line("Uptime", &ov.uptime, Color::Green));
            items.push(create_info_line(
                "Active Connections",
                &format!("{} / {}", ov.active_connections, ov.max_connections),
                if ov.active_connections as f64 / ov.max_connections as f64 > 0.8 {
                    Color::Red
                } else {
                    Color::Green
                },
            ));
            items.push(create_info_line(
                "Total DB Size",
                &ov.total_db_size,
                Color::Green,
            ));
            items.push(create_info_line(
                "Databases",
                &ov.num_databases.to_string(),
                Color::Green,
            ));
            items.push(create_info_line(
                "Server Time",
                &ov.server_time,
                Color::Gray,
            ));
        }
        None => {
            items.push(ListItem::new(Line::from(Span::styled(
                " Loading...",
                Style::default().fg(Color::Gray),
            ))));
        }
    }

    let list = List::new(items).block(
        Block::default()
            .title(" Server Overview ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(list, area);
}

fn create_info_line(label: &str, value: &str, value_color: Color) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(
            format!(" {}: ", label),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value.to_string(), Style::default().fg(value_color)),
    ]))
}

fn render_db_stats(
    f: &mut Frame,
    area: Rect,
    stats: &[DbStatEntry],
    state: &mut ListState,
) {
    let mut items: Vec<ListItem> = Vec::new();
    if stats.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No database statistics available",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header_line = Line::from(vec![
            Span::styled(
                format!(" {:<20}", "Database"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>12}", "Size"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "Conns"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>14}", "Transactions"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "Cache%"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        items.push(ListItem::new(header_line));

        for stat in stats {
            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<20}", stat.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>12}", stat.size),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("{:>8}", stat.connections),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:>14}", stat.transactions),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>7}%", stat.cache_hit_ratio),
                    if stat.cache_hit_ratio > 99.0 {
                        Style::default().fg(Color::Green)
                    } else if stat.cache_hit_ratio > 95.0 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Database Statistics ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}

fn render_active_queries(
    f: &mut Frame,
    area: Rect,
    queries: &[ActiveQuery],
    state: &mut ListState,
) {
    let mut items: Vec<ListItem> = Vec::new();
    if queries.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No active queries",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header_line = Line::from(vec![
            Span::styled(
                format!(" {:<8}", "PID"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<14}", "User"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<14}", "Database"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<10}", "State"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<8}", "Duration"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Query",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        items.push(ListItem::new(header_line));

        for q in queries {
            let truncated_query = if q.query.chars().count() > 60 {
                format!("{}...", q.query.chars().take(60).collect::<String>())
            } else {
                q.query.clone()
            };

            let state_color = match q.state.as_str() {
                "active" => Color::Green,
                "idle in transaction" => Color::Yellow,
                "waiting" | "blocked" => Color::Red,
                _ => Color::White,
            };

            let wait = q
                .wait_event
                .as_ref()
                .map(|w| format!(" [{}]", w))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<8}", q.pid),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:<14}", q.user),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:<14}", q.database),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:<10}", q.state),
                    Style::default().fg(state_color),
                ),
                Span::styled(
                    format!("{:<8}", q.duration),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("{}{}", truncated_query, wait),
                    Style::default().fg(Color::White),
                ),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Active Queries ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}

fn render_table_stats(
    f: &mut Frame,
    area: Rect,
    stats: &[TableStatEntry],
    state: &mut ListState,
) {
    let mut items: Vec<ListItem> = Vec::new();
    if stats.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No table statistics available",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header_line = Line::from(vec![
            Span::styled(
                format!(" {:<24}", "Table"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "SeqScan"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "IdxScan"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "Inserts"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "Updates"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "Deletes"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", "DeadTup"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        items.push(ListItem::new(header_line));

        for stat in stats {
            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<24}", format!("{}.{}", stat.schema, stat.table)),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>8}", stat.seq_scan),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:>8}", stat.idx_scan),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("{:>8}", stat.n_tup_ins),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("{:>8}", stat.n_tup_upd),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:>8}", stat.n_tup_del),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(
                    format!("{:>8}", stat.n_dead_tup),
                    if stat.n_dead_tup > 1000 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Table Statistics ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}


