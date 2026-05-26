use crate::db::functions::FunctionInfo;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, funcs: &[FunctionInfo], state: &mut ListState) {
    let mut items: Vec<ListItem> = Vec::new();
    if funcs.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " No functions found",
            Style::default().fg(Color::Gray),
        ))));
    } else {
        let header = Line::from(vec![
            col_span("Name", 24, true),
            col_span("Args", 30, true),
            col_span("Return", 20, true),
            col_span("Lang", 8, true),
            col_span("Vol", 10, true),
        ]);
        items.push(ListItem::new(header));

        for f in funcs {
            let line = Line::from(vec![
                col_span(&f.name, 24, false),
                col_span(&f.args, 30, false),
                col_span(&f.return_type, 20, false),
                col_span(&f.language, 8, false),
                col_span(&f.volatility, 10, false),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Functions (Esc: close, D: DDL) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, area, state);
}

fn col_span(text: &str, width: usize, bold: bool) -> Span<'static> {
    let style = if bold {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let formatted = if text.chars().count() > width as usize {
        format!("{}…", text.chars().take(width.saturating_sub(1)).collect::<String>())
    } else {
        format!("{:width$}", text, width = width)
    };
    Span::styled(formatted, style)
}
