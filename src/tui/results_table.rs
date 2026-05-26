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
    scroll_h: u16,
    focus: bool,
    visual_selection: &[usize],
) {
    let border_color = if focus { Color::Cyan } else { Color::DarkGray };

    if let Some(ref err) = result.error {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )))
        .block(Block::default().title(" Error ").borders(Borders::ALL).border_style(Style::default().fg(border_color)))
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

    // Compute column widths (using char count for display width)
    let col_count = result.columns.len();
    let mut col_widths: Vec<usize> = result.columns.iter().map(|c| c.chars().count()).collect();
    for row in &result.rows {
        for (i, val) in row.iter().enumerate() {
            if i < col_count {
                col_widths[i] = col_widths[i].max(val.chars().count());
            }
        }
    }

    // Build a formatted row string with padding
    fn format_row(values: &[String], widths: &[usize], header: bool) -> String {
        let mut parts = Vec::new();
        for (i, val) in values.iter().enumerate() {
            if i < widths.len() {
                let w = widths[i];
                let ccount = val.chars().count();
                if ccount >= w {
                    parts.push(val.clone());
                } else {
                    let mut s = val.clone();
                    s.push_str(&" ".repeat(w - ccount));
                    parts.push(s);
                }
            }
        }
        if header {
            parts.join(" │ ")
        } else {
            parts.join(" │ ")
        }
    }

    let header_str = format_row(
        &result.columns.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
        &col_widths,
        true,
    );
    let sep_str: String = col_widths
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let prefix = if i == 0 { "" } else { "─┼─" };
            format!("{}{}", prefix, "─".repeat(*w))
        })
        .collect();

    let visible_len = area.width.saturating_sub(2) as usize; // minus borders
    let soff = scroll_h as usize;

    fn clip(s: &str, offset: usize, max: usize) -> String {
        let chars: Vec<char> = s.chars().collect();
        if offset >= chars.len() || max == 0 {
            return String::new();
        }
        let from = offset;
        let prefix = if offset > 0 { "…" } else { "" };
        let prefix_len = prefix.chars().count();
        let actual_max = max.saturating_sub(prefix_len);
        let to = (from + actual_max).min(chars.len());
        let mut result = String::with_capacity(max);
        result.push_str(prefix);
        result.extend(chars[from..to].iter());
        if to < chars.len() {
            result.push('…');
        }
        result
    }

    let mut items: Vec<ListItem> = Vec::new();

    items.push(ListItem::new(Line::from(Span::styled(
        clip(&header_str, soff, visible_len),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))));

    items.push(ListItem::new(Line::from(Span::styled(
        clip(&sep_str, soff, visible_len),
        Style::default().fg(Color::Gray),
    ))));

    for (i, row) in result.rows.iter().enumerate() {
        let row_str = format_row(row, &col_widths, false);
        let is_selected = visual_selection.contains(&i);
        let style = if is_selected {
            Style::default().bg(Color::DarkGray).fg(Color::Yellow)
        } else {
            Style::default()
        };
        items.push(ListItem::new(Line::from(Span::styled(
            clip(&row_str, soff, visible_len),
            style,
        ))));
    }

    let list = List::new(items)
        .block(Block::default().title(" Results ").borders(Borders::ALL).border_style(Style::default().fg(border_color)))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("");

    f.render_stateful_widget(list, area, state);
}
