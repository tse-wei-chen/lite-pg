use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub const MAIN_ITEMS: &[(&str, &str)] = &[
    ("Home", "Schema tree + SQL editor + results"),
    ("Dashboard", "Overview, active queries, table stats"),
    ("Bookmarks", "Saved SQL queries"),
    ("Search", "Global object search"),
    ("Management >", "Roles, Databases, Functions, Extensions..."),
    ("Connection >", "Connection manager, connect, disconnect"),
    ("Export >", "CSV, JSON, SQL INSERT, Markdown"),
    ("Help", "Keyboard shortcuts reference"),
    ("Quit", "Exit lite-pg"),
];

pub const MANAGEMENT_SUB: &[&str] = &[
    "Roles",
    "Databases",
    "Functions",
    "Extensions",
    "Settings",
    "Replication",
];

pub const CONNECTION_SUB: &[&str] = &[
    "Connection Manager",
];

pub const EXPORT_SUB: &[&str] = &[
    "CSV",
    "JSON",
    "SQL INSERT",
    "Markdown file",
    "Markdown clipboard",
];

pub fn render(f: &mut Frame, area: Rect, level: usize, selection: usize, parent: usize) {
    // Clear background to hide content noise
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Menu (j/k, Enter, Esc) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    if level == 0 {
        for (i, &(name, desc)) in MAIN_ITEMS.iter().enumerate() {
            let selected = i == selection;
            let prefix = if selected { " >" } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    if selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!(" {:<15}", name),
                    if selected {
                        Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                Span::styled(
                    format!("  {}", desc),
                    if selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]));
        }
    } else if parent == 4 {
        for (i, &name) in MANAGEMENT_SUB.iter().enumerate() {
            let selected = i == selection;
            let prefix = if selected { " >" } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    if selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!(" {}", name),
                    if selected {
                        Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]));
        }
    } else if parent == 5 {
        for (i, &name) in CONNECTION_SUB.iter().enumerate() {
            let selected = i == selection;
            let prefix = if selected { " >" } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    if selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!(" {}", name),
                    if selected {
                        Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]));
        }
    } else {
        for (i, &name) in EXPORT_SUB.iter().enumerate() {
            let selected = i == selection;
            let prefix = if selected { " >" } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    if selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled(
                    format!(" {}", name),
                    if selected {
                        Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]));
        }
    }

    lines.push(Line::from(Span::styled("", Style::default())));
    lines.push(Line::from(Span::styled(
        " Esc:close  j/k:nav  Enter:select",
        Style::default().fg(Color::Gray),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let render_area = Rect {
        x: inner.x + 2,
        y: inner.y + 1,
        width: inner.width.saturating_sub(4),
        height: inner.height.saturating_sub(2),
    };
    f.render_widget(paragraph, render_area);
}
