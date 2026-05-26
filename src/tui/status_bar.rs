use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{Focus, Mode};

pub fn render(
    f: &mut Frame,
    area: Rect,
    mode: &Mode,
    focus: &Focus,
    elapsed: &std::time::Duration,
    connected: bool,
    connection_name: Option<&String>,
    _show_connection_panel: bool,
) {
    let mode_str = match mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Visual => "VISUAL",
        Mode::PropertyEditor => "PROPERTY",
    };

    let focus_str = match focus {
        Focus::Schema => "Schema",
        Focus::QueryInput => "SQL",
        Focus::Results => "Results",
        Focus::ConnectionList => "Connections",
        Focus::ConnectionForm => "ConnForm",
    };

    let conn_symbol = if connected {
        "●"
    } else {
        "○"
    };
    let conn_color = if connected { Color::Green } else { Color::Red };

    let conn_name = connection_name
        .map(|s| s.as_str())
        .unwrap_or("Disconnected");

    let mode_span = Span::styled(
        format!(" {mode_str} "),
        Style::default()
            .fg(Color::Black)
            .bg(match mode {
                Mode::Normal => Color::Blue,
                Mode::Insert => Color::Green,
                Mode::Visual => Color::Yellow,
                Mode::PropertyEditor => Color::Magenta,
            })
            .add_modifier(Modifier::BOLD),
    );

    let focus_span = Span::styled(
        format!(" [{focus_str}] "),
        Style::default().fg(Color::White),
    );

    let elapsed_str = format!("[{:.2}ms]", elapsed.as_secs_f64() * 1000.0);
    let elapsed_span = Span::styled(
        elapsed_str,
        Style::default().fg(if elapsed.as_secs_f64() < 0.1 {
            Color::Green
        } else if elapsed.as_secs_f64() < 1.0 {
            Color::Yellow
        } else {
            Color::Red
        }),
    );

    let line = Line::from(vec![
        Span::styled(
            format!(" {conn_symbol} {conn_name} "),
            Style::default().fg(conn_color),
        ),
        Span::raw(" │"),
        mode_span,
        focus_span,
        Span::raw("│ "),
        elapsed_span,
    ]);

    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}
