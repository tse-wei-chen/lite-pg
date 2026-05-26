use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct RoleFormState {
    pub focus: usize,
    pub name: String,
    pub login: bool,
    pub superuser: bool,
    pub createdb: bool,
    pub createrole: bool,
    pub replication: bool,
    pub password: String,
    pub conn_limit: String,
    pub edit_mode: bool,
}

impl RoleFormState {
    pub fn new() -> Self {
        RoleFormState {
            focus: 0,
            name: String::new(),
            login: true,
            superuser: false,
            createdb: false,
            createrole: false,
            replication: false,
            password: String::new(),
            conn_limit: String::from("-1"),
            edit_mode: false,
        }
    }

    pub fn reset(&mut self) {
        self.focus = 0;
        self.name.clear();
        self.login = true;
        self.superuser = false;
        self.createdb = false;
        self.createrole = false;
        self.replication = false;
        self.password.clear();
        self.conn_limit = String::from("-1");
        self.edit_mode = false;
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &RoleFormState) {
    let block = Block::default()
        .title(if state.edit_mode {
            " Edit Role "
        } else {
            " Create Role "
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled(
                " Name:       ",
                Style::default().fg(if state.focus == 0 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                state.name.clone(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Login:      ",
                Style::default().fg(if state.focus == 1 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                if state.login { "YES" } else { "NO" },
                Style::default().fg(Color::White),
            ),
            Span::styled(
                "  (Space to toggle)",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Superuser:  ",
                Style::default().fg(if state.focus == 2 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                if state.superuser { "YES" } else { "NO" },
                Style::default().fg(Color::White),
            ),
            Span::styled(
                "  (Space to toggle)",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Create DB:  ",
                Style::default().fg(if state.focus == 3 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                if state.createdb { "YES" } else { "NO" },
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Create Role:",
                Style::default().fg(if state.focus == 4 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                if state.createrole { "YES" } else { "NO" },
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Replication:",
                Style::default().fg(if state.focus == 5 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                if state.replication { "YES" } else { "NO" },
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Password:   ",
                Style::default().fg(if state.focus == 6 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                if state.password.is_empty() {
                    String::new()
                } else {
                    "********".to_string()
                },
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Conn Limit: ",
                Style::default().fg(if state.focus == 7 {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::styled(
                state.conn_limit.clone(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![]),
        Line::from(Span::styled(
            " Tab: next  Enter: save  Esc: cancel  Space: toggle boolean",
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    let inner_area = Rect {
        x: inner.x + 2,
        y: inner.y + 1,
        width: inner.width.saturating_sub(4),
        height: inner.height.saturating_sub(2),
    };
    f.render_widget(paragraph, inner_area);
}
