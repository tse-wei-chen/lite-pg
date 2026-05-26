use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Focus, Mode, Page, PropSection};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < 20 {
        return;
    }

    let hints = match app.mode {
        Mode::PropertyEditor => {
            let mut hints = vec![
                hint("[d] Drop", ""),
                hint(" [Tab/l] Section", ""),
                hint(" [j/k] Move", ""),
            ];
            if app.current_page == Page::Home && app.property_editor.as_ref().map_or(false, |e| e.section == PropSection::Columns) {
                hints.push(hint(" [a] Add col", ""));
                hints.push(hint(" [e] Edit col", ""));
            }
            hints.push(hint(" [Esc] Close", ""));
            Line::from(hints)
        }
        _ => match app.current_page {
        Page::Home => match app.focus {
            Focus::Schema => Line::from(vec![
                hint("[j/k] Move", ""),
                hint(" [Enter] Expand", ""),
                hint(" [o]SELECT", ""),
                hint(" [u]UPDATE", ""),
                hint(" [D]DDL", ""),
                hint(" [d]Drop", ""),
                hint(" [t]Trunc", ""),
                hint(" [v]Vacuum", ""),
                hint(" [r]Refresh", ""),
            ]),
            Focus::QueryInput if app.mode == Mode::Insert => Line::from(vec![
                hint("[Esc] Normal", ""),
                hint(" [Alt+Enter] Run", ""),
            ]),
            Focus::QueryInput => Line::from(vec![
                hint("[i] Insert", ""),
                hint(" [[]/[]] Tab", ""),
                hint(" [Alt+Enter] Run", ""),
                hint(" [F5] Explain", ""),
            ]),
            Focus::Results => Line::from(vec![
                hint("[j/k] Move", ""),
                hint(" [h/l] Scroll", ""),
                hint(" [v] Visual", ""),
            ]),
            Focus::ConnectionList | Focus::ConnectionForm => Line::from(vec![
                hint("[j/k] Move", ""),
                hint(" [Enter] Connect", ""),
                hint(" [n] New", ""),
                hint(" [e] Edit", ""),
                hint(" [d] Delete", ""),
            ]),
        },
        Page::Dashboard => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [r] Refresh", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Roles => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [n] Create", ""),
            hint(" [d] Drop", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Databases => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [Enter] Switch", ""),
            hint(" [n] Create", ""),
            hint(" [d] Drop", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Functions => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [D] DDL", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Extensions => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Settings => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Replication => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [h/l] Tab", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Search => Line::from(vec![
            hint("[Enter] Search", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::Bookmarks => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [Enter] Load", ""),
            hint(" [d] Delete", ""),
            hint(" [Esc] Menu", ""),
        ]),
        Page::ConnectionManager => Line::from(vec![
            hint("[j/k] Move", ""),
            hint(" [Enter] Connect", ""),
            hint(" [n] New", ""),
            hint(" [e] Edit", ""),
            hint(" [d] Delete", ""),
        ]),
        Page::Help => Line::from(vec![hint("[Esc] Menu", "")]),
        }
    };

    let paragraph = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray).bg(Color::Reset));
    f.render_widget(paragraph, area);
}

fn hint<'a>(key: &'a str, desc: &'a str) -> Span<'a> {
    Span::styled(
        format!("{}{}", key, desc),
        Style::default().fg(Color::DarkGray),
    )
}
