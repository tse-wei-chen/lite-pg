use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(
    f: &mut Frame,
    area: Rect,
    active_tab: usize,
    tabs: &[tui_textarea::TextArea],
    focus: bool,
) {
    if tabs.is_empty() {
        return;
    }

    let tab_color = if focus { Color::Cyan } else { Color::DarkGray };

    // Build tab labels
    let mut spans = Vec::new();
    let max_w = area.width.saturating_sub(6) as usize / tabs.len().max(1);

    for (i, tab) in tabs.iter().enumerate() {
        let preview = tab.lines().first().map(|l| l.as_str()).unwrap_or("");
        let short = if preview.chars().count() > max_w.saturating_sub(5) {
            format!("{}…", preview.chars().take(max_w.saturating_sub(6)).collect::<String>())
        } else {
            preview.to_string()
        };
        let label = if short.is_empty() {
            format!(" {} ", i + 1)
        } else {
            format!(" {}:{} ", i + 1, short)
        };

        if i == active_tab {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                label,
                Style::default().fg(Color::DarkGray),
            ));
        }
        spans.push(Span::raw(" "));
    }

    // [+] new tab button
    if focus {
        spans.push(Span::styled(
            "[+]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Render: underline for active tab only
    let line = Line::from(spans);
    let para = Paragraph::new(line);

    // Bottom border acts as tab underline
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(tab_color));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(para, inner);
}
