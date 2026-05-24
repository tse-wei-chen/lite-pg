use crate::db::TableInfo;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, tables: &[TableInfo], state: &mut ListState, expanded: &[bool]) {
    let items: Vec<ListItem> = tables
        .iter()
        .enumerate()
        .flat_map(|(ti, table)| {
            let mut items = vec![ListItem::new(Line::from(Span::styled(
                if *expanded.get(ti).unwrap_or(&false) {
                    format!("  ▼ {}", table.name)
                } else {
                    format!("  ▶ {}", table.name)
                },
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )))];

            if *expanded.get(ti).unwrap_or(&false) {
                for col in &table.columns {
                    let nullable = if col.is_nullable { "?" } else { "" };
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("      {} : {}{}", col.name, col.data_type, nullable),
                        Style::default().fg(Color::White),
                    ))));
                }
            }

            items
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Schema ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, area, state);
}
