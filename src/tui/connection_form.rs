use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let fields = [
        ("Name", &app.form_name),
        ("Host", &app.form_host),
        ("Port", &app.form_port),
        ("User", &app.form_user),
        ("Password", &app.form_password),
        ("Database", &app.form_dbname),
    ];

    let mut lines = Vec::new();
    for (i, (label, value)) in fields.iter().enumerate() {
        let prefix = if app.form_focus == i { "▸ " } else { "  " };
        let masked = if i == 4 {
            "********".to_string()
        } else {
            value.to_string()
        };
        lines.push(Line::from(Span::styled(
            format!("{}{}: {}", prefix, label, masked),
            if app.form_focus == i {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            },
        )));
    }
    lines.push(Line::from(Span::styled(
        "  [Tab] next  [Enter] save  [Esc] cancel",
        Style::default().fg(Color::Gray),
    )));

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Connection ")
                .borders(Borders::ALL),
        );
    f.render_widget(para, area);
}
