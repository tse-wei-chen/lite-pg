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
) {
    let mode_str = match mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Visual => "VISUAL",
    };

    let focus_str = match focus {
        Focus::Schema => "Schema",
        Focus::QueryInput => "SQL",
        Focus::Results => "Results",
    };

    let conn_str = if connected {
        Span::styled("● Connected", Style::default().fg(Color::Green))
    } else {
        Span::styled("○ Disconnected", Style::default().fg(Color::Red))
    };

    let mode_span = Span::styled(
        format!(" {mode_str} "),
        Style::default()
            .fg(Color::Black)
            .bg(match mode {
                Mode::Normal => Color::Blue,
                Mode::Insert => Color::Green,
                Mode::Visual => Color::Yellow,
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
        conn_str,
        Span::raw(" │"),
        mode_span,
        focus_span,
        Span::raw("│ "),
        elapsed_span,
    ]);

    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}
