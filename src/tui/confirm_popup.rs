use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, message: &str) {
    let line_count = message.lines().count().max(1) as u16;
    let popup_height = (line_count + 4).min(10);
    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    let popup = Rect {
        x: area.x + area.width / 4,
        y: area.y + area.height / 2 - popup_height / 2,
        width: area.width / 2,
        height: popup_height,
    };

    let popup_inner = block.inner(popup);

    f.render_widget(Clear, popup);
    f.render_widget(block, popup);

    let msg = Line::from(vec![
        Span::styled(message, Style::default().fg(Color::White)),
        Span::raw(" "),
        Span::styled(
            "(y/n)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let para = Paragraph::new(msg).alignment(Alignment::Center).wrap(Wrap { trim: false });
    f.render_widget(para, popup_inner);
}
